#!/usr/bin/env python3
"""Link PIC Rust to the pinned stock uClibc ABI using a minimal independent CRT."""
from pathlib import Path
import os
import sys

ROOT = Path(__file__).resolve().parents[1]
SYSLIB = Path(os.environ['MI360_SYSROOT']).resolve() / 'lib'
LIBS = {'c': 'libuClibc-0.9.33.2.so', 'pthread': 'libpthread-0.9.33.2.so',
        'dl': 'libdl-0.9.33.2.so', 'rt': 'librt-0.9.33.2.so', 'm': 'libm-0.9.33.2.so',
        'util': 'libutil-0.9.33.2.so', 'gcc_s': 'libgcc_s.so.1'}
args = []
for arg in sys.argv[1:]:
    if arg == '-pie':
        continue
    if arg.startswith('-l') and arg[2:] in LIBS:
        lib = SYSLIB / LIBS[arg[2:]]
        if arg == '-lutil' and not lib.exists():
            continue
        args.append(str(lib))
    else:
        args.append(arg)
command = ['zig', 'cc', '-target', 'mipsel-linux-gnu', '-mcpu=mips32r2',
           '-nostdlib', '-nostartfiles', '-no-pie', '-Wl,-e,__start',
           '-Wl,--dynamic-linker=/lib/ld-uClibc.so.0',
           *args, str(SYSLIB/'ld-uClibc-0.9.33.2.so')]
os.execvp(command[0], command)
