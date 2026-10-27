#!/usr/bin/env python3 -I
"""Prints the disassembly between two addresses: show.py libhdal.asm 2ce38 2cf24"""
import sys

lo, hi = int(sys.argv[2], 16), int(sys.argv[3], 16)
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    head = line.split(":")[0].strip()
    try:
        a = int(head, 16)
    except ValueError:
        continue
    if lo <= a < hi and "\t" in line:
        print(line.rstrip("\n"))
