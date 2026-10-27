/* Isolated adapter faults: these functions never touch real GPIOs. */
#define _POSIX_C_SOURCE 200809L
#define write probe_write
#define lseek probe_lseek
#define nanosleep probe_nanosleep
#define main camctl_main
#include "camctl.c"
#undef main
#undef write
#undef lseek
#undef nanosleep
#include <assert.h>

static int levels[2], writes, inject_eintr, fail_high, cancel_sleep, fail_low;
off_t probe_lseek(int fd, off_t offset, int whence) {
    assert(fd == 51 || fd == 54); assert(offset == 0 && whence == SEEK_SET);
    return 0;
}
ssize_t probe_write(int fd, const void *buf, size_t size) {
    assert(fd == 51 || fd == 54); assert(size == 1);
    int high = *(const char *)buf == '1';
    writes++;
    if (inject_eintr) { inject_eintr = 0; errno = EINTR; return -1; }
    if ((fail_high && high) || (fail_low && !high && fd == 51)) {
        errno = EIO; return -1;
    }
    levels[fd == 54] = high;
    assert(!(levels[0] && levels[1]));
    return 1;
}
int probe_nanosleep(const struct timespec *req, struct timespec *remaining) {
    assert(req->tv_sec == 0 && req->tv_nsec == 500000000L);
    if (cancel_sleep) {
        on_signal(SIGTERM); inject_eintr = 1;
        *remaining = *req; errno = EINTR; return -1;
    }
    return 0;
}
static void reset(void) {
    levels[0] = levels[1] = writes = interrupted = inject_eintr = fail_high = cancel_sleep = fail_low = 0;
}
int main(void) {
    reset(); assert(!pulse_fds(51, 54, 0));
    assert(writes == 3 && !levels[0] && !levels[1]);
    reset(); assert(!pulse_fds(51, 54, 1)); assert(!levels[0] && !levels[1]);
    reset(); interrupted = SIGTERM; assert(pulse_fds(51, 54, 0)); assert(!writes);
    reset(); inject_eintr = 1; assert(!pulse_fds(51, 54, 0)); assert(writes == 4);
    reset(); fail_high = 1; assert(pulse_fds(51, 54, 0)); assert(!levels[0] && !levels[1]);
    reset(); cancel_sleep = 1; assert(pulse_fds(51, 54, 1));
    assert(!levels[0] && !levels[1] && writes == 4);
    reset(); fail_low = 1; assert(pulse_fds(51, 54, 1));
    assert(!levels[1] && writes == 3); /* A failed first low must not skip the other. */
    puts("GPIO pulse cancellation, EINTR and failure cleanup checks passed");
    return 0;
}
