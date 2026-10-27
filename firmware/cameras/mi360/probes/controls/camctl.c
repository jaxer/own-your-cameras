#define _POSIX_C_SOURCE 200809L
#define _DARWIN_C_SOURCE 1
#define _DEFAULT_SOURCE 1
#include "motor.h"
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/file.h>
#include <time.h>
#include <unistd.h>

/* Exact B-kernel ABI: see docs/hardware.md. Never expose raw ioctls. */
enum { MOTOR_STOP = 1, MOTOR_MOVE = 3, MOTOR_STATUS = 4, MOTOR_SPEED = 5 };
struct driver_status { int32_t x, y, moving, speed; };
_Static_assert(sizeof(struct driver_status) == 16, "motor status ABI");
static volatile sig_atomic_t interrupted;
static void on_signal(int signo) { interrupted = signo; }

static int lock_controls(void) {
    int fd = open("/tmp/mi360-camctl.lock", O_RDWR | O_CREAT | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (fd < 0 || flock(fd, LOCK_EX | LOCK_NB)) {
        if (fd >= 0) close(fd);
        fprintf(stderr, "Another camctl owns controls or control lock is unavailable\n");
        return -1;
    }
    /* Keep this inode: unlinking it would break mutual exclusion. */
    return fd;
}

static int read_text(const char *path, char *out, size_t cap) {
    FILE *f = fopen(path, "r");
    if (!f) return -1;
    size_t n = fread(out, 1, cap - 1, f);
    int bad = ferror(f) || !feof(f);
    if (fclose(f)) bad = 1;
    out[n] = 0;
    return bad ? -1 : 0;
}

static int read_motor(struct motor_info *m) {
    char text[2048];
    if (read_text("/proc/jz/motor/motor_info", text, sizeof(text)) || motor_parse(text, m)) {
        fprintf(stderr, "Unsupported or unreadable Mi360 motor diagnostics\n");
        return -1;
    }
    return 0;
}

static int gpio_read(int pin) {
    char path[96], text[16];
    snprintf(path, sizeof(path), "/sys/class/gpio/gpio%d/value", pin);
    if (read_text(path, text, sizeof(text))) return -1;
    if (!strcmp(text, "0\n")) return 0;
    if (!strcmp(text, "1\n")) return 1;
    return -1;
}

static int gpio_output(int pin) {
    char path[96], direction[16];
    snprintf(path, sizeof(path), "/sys/class/gpio/gpio%d/direction", pin);
    if (read_text(path, direction, sizeof(direction)) || strcmp(direction, "out\n")) {
        fprintf(stderr, "GPIO %d must already be exported as output\n", pin);
        return -1;
    }
    return 0;
}

static int gpio_acquire(int pin) {
    char path[96];
    if (gpio_output(pin)) return -1;
    snprintf(path, sizeof(path), "/sys/class/gpio/gpio%d/value", pin);
    return open(path, O_RDWR | O_CLOEXEC);
}

static int gpio_set_fd(int fd, int value) {
    char b = value ? '1' : '0';
    off_t pos;
    do { pos = lseek(fd, 0, SEEK_SET); } while (pos < 0 && errno == EINTR);
    if (pos < 0) return -1;
    ssize_t n;
    do { n = write(fd, &b, 1); } while (n < 0 && errno == EINTR);
    return n == 1 ? 0 : -1;
}

static int vendor_ir_running(void) {
    DIR *dir = opendir("/proc");
    if (!dir) return 1;
    struct dirent *entry;
    int found = 0;
    while ((entry = readdir(dir))) {
        if (entry->d_name[0] < '0' || entry->d_name[0] > '9') continue;
        char path[320], comm[64];
        snprintf(path, sizeof(path), "/proc/%s/comm", entry->d_name);
        if (read_text(path, comm, sizeof(comm))) {
            if (errno == ENOENT || errno == ESRCH) continue;
            found = 1; break;
        }
        if (!strcmp(comm, "ingenic_algo\n")) { found = 1; break; }
    }
    if (closedir(dir)) found = 1;
    return found;
}

static long long monotonic_ms(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_MONOTONIC, &t)) return -1;
    return (long long)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}

static int wait_ms(int milliseconds) {
    if (interrupted) return -1;
    struct timespec t = { milliseconds / 1000, (milliseconds % 1000) * 1000000L };
    while (nanosleep(&t, &t)) {
        if (errno != EINTR || interrupted) return -1;
    }
    return interrupted ? -1 : 0;
}

static int print_status(void) {
    struct motor_info m;
    if (read_motor(&m)) return 1;
    printf("{\"motor\":{\"owner_open\":%s,\"moving\":%s,\"x\":%d,\"y\":%d,"
           "\"max_x\":%d,\"max_y\":%d,\"speed\":%d},\"gpio\":{",
           m.opened ? "true" : "false", m.running ? "true" : "false",
           m.x, m.y, m.max_x, m.max_y, m.speed);
    const int pins[] = {8, 9, 49, 51, 54};
    for (size_t i = 0; i < sizeof(pins)/sizeof(pins[0]); i++) {
        int v = gpio_read(pins[i]);
        printf("%s\"%d\":", i ? "," : "", pins[i]);
        if (v < 0) printf("null"); else printf("%d", v);
    }
    puts("}}");
    return ferror(stdout) ? 1 : 0;
}

static int motor_leg(int fd, int dx, int dy, struct driver_status *s) {
    int32_t delta[2] = {dx, dy};
    if (interrupted || ioctl(fd, MOTOR_MOVE, delta)) return -1;
    long long now = monotonic_ms(), end = now + 5000;
    if (now < 0) return -1;
    do {
        if (ioctl(fd, MOTOR_STATUS, s)) return -1;
        if (!s->moving) return 0;
        now = monotonic_ms();
        if (now < 0 || now >= end || wait_ms(10)) return -1;
    } while (!interrupted);
    return -1;
}

static int jog(int dx, int dy, int speed, int roundtrip) {
    struct motor_info m;
    if (read_motor(&m) || motor_plan(&m, dx, dy, speed)) {
        fprintf(stderr, "Jog refused: moving, unsupported limits, bounds or speed\n");
        return 1;
    }
    if (m.opened) {
        fprintf(stderr, "Motor is owned by another process; no takeover attempted\n");
        return 1;
    }
    int fd = open("/dev/motor", O_RDWR | O_CLOEXEC);
    if (fd < 0) { perror("open motor"); return 1; }
    int bad = 1, changed_speed = 0;
    struct driver_status before = {0}, after = {0};
    if (ioctl(fd, MOTOR_STATUS, &before)) goto done;
    /* Recheck after gaining exclusive ownership; the proc snapshot may be stale. */
    m.x = before.x; m.y = before.y; m.running = before.moving;
    if (before.speed < 100 || before.speed > 900 || motor_plan(&m, dx, dy, speed)) goto done;
    if (before.speed != speed) {
        if (ioctl(fd, MOTOR_SPEED, &speed)) goto done;
        changed_speed = 1;
    }
    if (motor_leg(fd, dx, dy, &after)) goto done;
    printf("{\"phase\":\"out\",\"x\":%d,\"y\":%d,\"speed\":%d}\n",
           after.x, after.y, after.speed);
    if (after.x != before.x + dx || after.y != before.y + dy) goto done;
    if (roundtrip) {
        if (motor_leg(fd, -dx, -dy, &after)) goto done;
        printf("{\"phase\":\"back\",\"x\":%d,\"y\":%d,\"speed\":%d}\n",
               after.x, after.y, after.speed);
        if (after.x != before.x || after.y != before.y) goto done;
    }
    bad = 0;
done:
    if (ioctl(fd, MOTOR_STOP, 0)) { perror("stop motor"); bad = 1; }
    if (changed_speed && ioctl(fd, MOTOR_SPEED, &before.speed)) {
        perror("restore motor speed"); bad = 1;
    }
    if (close(fd)) bad = 1;
    if (bad) fprintf(stderr, "Motor operation incomplete; stopped, check status\n");
    return bad;
}

static int led_test(int pin, int duration) {
    int fd = gpio_acquire(pin);
    if (fd < 0) return 1;
    int before = gpio_read(pin);
    if (before < 0 || interrupted) { close(fd); return 1; }
    int bad = gpio_set_fd(fd, !before);
    if (!bad && gpio_read(pin) != !before) bad = 1;
    if (!bad && wait_ms(duration)) bad = 1;
    if (gpio_set_fd(fd, before) || gpio_read(pin) != before) bad = 1;
    if (close(fd)) bad = 1;
    printf("{\"gpio\":%d,\"original\":%d,\"restored\":%s}\n",
           pin, before, bad ? "false" : "true");
    return bad;
}

static int pulse_fds(int fd51, int fd54, int night) {
    if (interrupted) return 1;
    int bad = gpio_set_fd(night ? fd54 : fd51, 1);
    if (!bad && wait_ms(500)) bad = 1;
    int a = gpio_set_fd(fd51, 0), b = gpio_set_fd(fd54, 0);
    return bad || a || b;
}

static int ircut(int night) {
    /* Match the vendor 500 ms pulse; refuse an already energized bridge. */
    if (vendor_ir_running() || gpio_output(51) || gpio_output(54) ||
        gpio_read(51) != 0 || gpio_read(54) != 0) {
        fprintf(stderr, "IR-cut refused: vendor controller active or GPIOs unavailable/energized\n");
        return 1;
    }
    int fd51 = gpio_acquire(51);
    if (fd51 < 0) return 1;
    int fd54 = gpio_acquire(54);
    if (fd54 < 0) { close(fd51); return 1; }
    int bad = pulse_fds(fd51, fd54, night);
    if (gpio_read(51) != 0 || gpio_read(54) != 0) bad = 1;
    if (close(fd51)) bad = 1;
    if (close(fd54)) bad = 1;
    printf("{\"ircut_pulse\":\"%s\",\"coils_off\":%s}\n",
           night ? "night" : "day", bad ? "false" : "true");
    return bad;
}

static void usage(void) {
    puts("camctl status\n"
         "camctl motor-jog DX DY [SPEED]        (steps, max +/-256; speed 100..300)\n"
         "camctl motor-roundtrip DX DY [SPEED]  (out then return)\n"
         "camctl led-test yellow|blue [MS]      (restore original, 20..1000 ms)\n"
         "camctl ircut day|night               (500 ms pulse, then coils off)");
}

int main(int argc, char **argv) {
    struct sigaction sa = {0};
    sa.sa_handler = on_signal;
    sigemptyset(&sa.sa_mask);
    if (sigaction(SIGINT, &sa, NULL) || sigaction(SIGTERM, &sa, NULL) ||
        sigaction(SIGHUP, &sa, NULL) || sigaction(SIGPIPE, &sa, NULL)) return 1;
    if (argc == 2 && !strcmp(argv[1], "--help")) { usage(); return 0; }
    if (argc == 2 && !strcmp(argv[1], "status")) return print_status();
    if ((argc == 4 || argc == 5) && (!strcmp(argv[1], "motor-jog") ||
        !strcmp(argv[1], "motor-roundtrip"))) {
        int x, y, speed = 200;
        if (parse_int(argv[2], -256, 256, &x) || parse_int(argv[3], -256, 256, &y) ||
            (argc == 5 && parse_int(argv[4], 100, 300, &speed))) return 2;
        int lock = lock_controls();
        if (lock < 0) return 1;
        int result = jog(x, y, speed, !strcmp(argv[1], "motor-roundtrip"));
        if (close(lock)) result = 1;
        return result;
    }
    if ((argc == 3 || argc == 4) && !strcmp(argv[1], "led-test")) {
        int pin = !strcmp(argv[2], "yellow") ? 8 : !strcmp(argv[2], "blue") ? 9 : -1;
        int duration = 200;
        if (pin < 0 || (argc == 4 && parse_int(argv[3], 20, 1000, &duration))) return 2;
        int lock = lock_controls();
        if (lock < 0) return 1;
        int result = led_test(pin, duration);
        if (close(lock)) result = 1;
        return result;
    }
    if (argc == 3 && !strcmp(argv[1], "ircut")) {
        if (strcmp(argv[2], "day") && strcmp(argv[2], "night")) return 2;
        int lock = lock_controls();
        if (lock < 0) return 1;
        int result = ircut(!strcmp(argv[2], "night"));
        if (close(lock)) result = 1;
        return result;
    }
    usage();
    return 2;
}
