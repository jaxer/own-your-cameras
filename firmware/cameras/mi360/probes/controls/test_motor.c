#include "motor.h"
#include <assert.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>

int main(void) {
    /* Retained real diagnostic fields, not injected into a deployed system. */
    const char *real = "The version of Motor driver is H20171204a\n"
        "Motor driver is opened\nThe max speed is 900 and the min speed is 100\n"
        "The status of motor is stop\nThe pos of motor is (1059, 301)\n"
        "The speed of motor is 200\n## motor is Horizontal motor ##\nmax steps 2404\n"
        "## motor is Vertical motor ##\nmax steps 713\n";
    struct motor_info m;
    assert(!motor_parse(real, &m));
    assert(m.opened == 1 && !m.running && m.x == 1059 && m.y == 301 && m.speed == 200);
    assert(!motor_plan(&m, 32, 0, 200));
    assert(!motor_plan(&m, -256, 256, 300));
    assert(motor_plan(&m, 257, 0, 200));
    assert(motor_plan(&m, 0, -257, 200));
    assert(motor_plan(&m, INT_MIN, INT_MAX, 200));
    assert(motor_plan(&m, 0, 0, 200));
    assert(motor_plan(&m, 1, 0, 99));
    assert(motor_plan(&m, 1, 0, 301));
    m.running = 1; assert(motor_plan(&m, 1, 0, 200)); m.running = 0;
    m.x = 0; assert(motor_plan(&m, -1, 0, 200)); assert(!motor_plan(&m, 1, 0, 200));
    m.x = 2404; assert(motor_plan(&m, 1, 0, 200)); assert(!motor_plan(&m, -1, 0, 200));
    m.y = 0; assert(motor_plan(&m, 0, -1, 200));
    m.y = 713; assert(motor_plan(&m, 0, 1, 200));
    m.max_x = 2405; assert(motor_plan(&m, -1, 0, 200));
    assert(motor_parse("", &m));
    char truncated[512]; snprintf(truncated, sizeof(truncated), "%s", real);
    *strstr(truncated, "## motor is Vertical") = 0;
    assert(motor_parse(truncated, &m));
    char changed[512]; snprintf(changed, sizeof(changed), "%s", real);
    memcpy(strstr(changed, "H20171204a"), "H20171204b", 10);
    assert(motor_parse(changed, &m));
    int v = 999;
    assert(!parse_int("-256", -256, 256, &v) && v == -256);
    const char *invalid[] = {"", " ", "1x", "1\n", "0x10", "999999999999999999999", "257", "-257", "+1", " 1"};
    for (unsigned i = 0; i < sizeof(invalid)/sizeof(*invalid); i++)
        assert(parse_int(invalid[i], -256, 256, &v));
    puts("Motor parsing and movement policy checks passed");
    return 0;
}
