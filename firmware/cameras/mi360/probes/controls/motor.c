#include "motor.h"
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int parse_int(const char *text, int low, int high, int *value) {
    char *end;
    if (!text || !*text || (*text != '-' && (*text < '0' || *text > '9')))
        return -1;
    errno = 0;
    long v = strtol(text, &end, 10);
    if (errno || *end || v < low || v > high) return -1;
    *value = (int)v;
    return 0;
}

int motor_parse(const char *text, struct motor_info *info) {
    const char *p;
    struct motor_info m = {0};
    if (!strstr(text, "The version of Motor driver is H20171204a\n")) return -1;
    if (strstr(text, "Motor driver is opened\n")) m.opened = 1;
    else if (!strstr(text, "Motor driver is closed\n")) return -1;
    if (strstr(text, "The status of motor is stop\n")) m.running = 0;
    else if (strstr(text, "The status of motor is running\n")) m.running = 1;
    else return -1;
    p = strstr(text, "The pos of motor is (");
    if (!p || sscanf(p, "The pos of motor is (%d, %d)", &m.x, &m.y) != 2) return -1;
    p = strstr(text, "The speed of motor is ");
    if (!p || sscanf(p, "The speed of motor is %d", &m.speed) != 1) return -1;
    p = strstr(text, "## motor is Horizontal motor ##\nmax steps ");
    if (!p || sscanf(p, "## motor is Horizontal motor ##\nmax steps %d", &m.max_x) != 1)
        return -1;
    p = strstr(text, "## motor is Vertical motor ##\nmax steps ");
    if (!p || sscanf(p, "## motor is Vertical motor ##\nmax steps %d", &m.max_y) != 1)
        return -1;
    if (m.max_x != 2404 || m.max_y != 713 || m.x < 0 || m.x > m.max_x ||
        m.y < 0 || m.y > m.max_y || m.speed < 100 || m.speed > 900) return -1;
    *info = m;
    return 0;
}

int motor_plan(const struct motor_info *m, int dx, int dy, int speed) {
    if (m->running || m->max_x != 2404 || m->max_y != 713 ||
        m->x < 0 || m->x > m->max_x || m->y < 0 || m->y > m->max_y ||
        dx < -256 || dx > 256 || dy < -256 || dy > 256 || (!dx && !dy) ||
        speed < 100 || speed > 300) return -1;
    int64_t x = (int64_t)m->x + dx, y = (int64_t)m->y + dy;
    return x >= 0 && x <= m->max_x && y >= 0 && y <= m->max_y ? 0 : -1;
}
