#!/usr/bin/env python3
"""Verify private full-BOOT readbacks after one explicitly selected slot activation."""
import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bootstrap import load_tools

BOOT_SIZE = 0x50000
ENV_START, ENV_SIZE = 0x48000, 0x8000
COMMANDS = {
    b'0': b'sf probe;sf read 0x80600000 0x2d0000 0x1b0000;bootm 0x80600000',
    b'1': b'sf probe;sf read 0x80600000 0x8f0000 0x1b0000;bootm 0x80600000',
}


def verify(before, after, target):
    if len(before) != BOOT_SIZE or len(after) != BOOT_SIZE:
        raise ValueError('Both readbacks must contain the complete BOOT partition')
    if before[:ENV_START] != after[:ENV_START] or before[ENV_START + ENV_SIZE:] != after[ENV_START + ENV_SIZE:]:
        raise ValueError('BOOT bytes outside the environment changed')
    formats, _ = load_tools()
    old, new = formats.environment(before), formats.environment(after)
    flag = {"A": b"0", "B": b"1"}[target]
    old_flag = old.get(b'boot_flag')
    if old_flag not in COMMANDS or old_flag == flag or old.get(b'bootcmd') != COMMANDS[old_flag]:
        raise ValueError('Before image does not select the expected opposite kernel/slot')
    expected = {**old, b'boot_flag': flag, b'bootcmd': COMMANDS[flag], b'boot_retry': b'0'}
    if new != expected:
        raise ValueError('Environment differs from exactly the three intended boot settings')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--before', required=True, type=Path)
    parser.add_argument('--after', required=True, type=Path)
    parser.add_argument('--target', required=True, choices=['A', 'B'])
    args = parser.parse_args()
    verify(args.before.read_bytes(), args.after.read_bytes(), args.target)
    print('BOOT readbacks verified: valid CRCs, expected slot/settings, all other variables and BOOT bytes outside the environment preserved.')


if __name__ == '__main__':
    main()
