#ifndef MI360_MOTOR_H
#define MI360_MOTOR_H
#include <stdint.h>

struct motor_info {
    int opened, running;
    int32_t x, y, max_x, max_y, speed;
};

/* Pure parsing/planning. Bounds are driver steps, not angles. */
int motor_parse(const char *text, struct motor_info *info);
int motor_plan(const struct motor_info *info, int dx, int dy, int speed);
int parse_int(const char *text, int low, int high, int *value);
#endif
