CC := cc
CFLAGS := -std=c11 -Os -Wall -Wextra -Werror

.PHONY: all fast area gate
all: build/camctl-mipsel

build:
	mkdir -p $@

build/camctl-mipsel: camctl.c motor.c motor.h | build
	zig cc -target mipsel-linux-musleabi -mcpu=mips32r2 $(CFLAGS) -static -s -fstack-protector-strong -ffunction-sections -fdata-sections -Wl,--gc-sections camctl.c motor.c -o $@

build/test-motor: test_motor.c motor.c motor.h | build
	$(CC) $(CFLAGS) -fsanitize=address,undefined -g test_motor.c motor.c -o $@

build/test-gpio: test_gpio.c camctl.c motor.c motor.h | build
	$(CC) $(CFLAGS) -fsanitize=address,undefined -g test_gpio.c motor.c -o $@

fast: build/test-motor build/test-gpio
	./build/test-motor
	./build/test-gpio

area: fast all
	python3 test_cli.py

gate: area
