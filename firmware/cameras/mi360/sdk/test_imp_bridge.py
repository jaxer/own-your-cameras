#!/usr/bin/env python3
"""Offline bridge gate: real target ABI compilation plus isolated host fault checks.

The host cannot execute MIPS code. The unchanged production source is first
compiled as MIPS O32 with every static assertion enabled. A second host build
includes that source into test_imp_bridge.c: SDK/loader calls are local fakes,
O32 static assertions are disabled only in that test translation unit, and a
fresh temporary encoder header widens only stream.virAddr to uintptr_t so a
host allocation can hold ring data. No other vendor field is edited. Thus host
checks cover lifecycle/copy decisions, not O32 ABI, target pointer overflow,
real SDK behavior, driver cleanup, or hardware acceptance. The host fixture also
replaces SDK image-base/GOT reads with explicit local synthetic values; production
MIPS compilation keeps the actual relocated-GOT reads. Original header bytes
are verified against the repository lock before either compilation. No IMP
library is loaded, no camera/service is contacted, no audio is initialized.
"""
import argparse
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(command):
    subprocess.run([str(part) for part in command], cwd=ROOT, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--headers', type=Path)
    args = parser.parse_args()
    configured = os.environ.get('MI360_HEADERS') or os.environ.get('MI360_HEADER_SOURCE')
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))
    source = args.headers or (Path(configured) if configured else target / 'sdk-headers')
    with tempfile.TemporaryDirectory(prefix='mi360-imp-test-') as directory:
        temporary = Path(directory)
        headers = temporary / 'original'
        fetch = [sys.executable, ROOT / 'scripts/headers.py', '--out', headers]
        if source.is_dir():
            fetch += ['--from-directory', source]
        elif args.headers or configured:
            raise SystemExit(f'Specified SDK header directory is missing: {source}')
        # A missing optional cache fetches pinned public headers into this run's
        # private temporary directory; no shared cache or source file is changed.
        run(fetch)
        common = ['-std=c11', '-Wall', '-Wextra', '-Werror', '-Wno-comment']
        run(['zig', 'cc', '-target', 'mipsel-linux-gnueabihf', '-mcpu=mips32r2',
             *common, '-O2', '-fPIC', '-I', ROOT / 'sdk/include-compat',
             '-I', headers, '-c', ROOT / 'sdk/imp_bridge.c', '-o', temporary / 'bridge.o'])
        fixture = temporary / 'host-include/imp'
        fixture.mkdir(parents=True)
        original = (headers / 'imp/imp_encoder.h').read_text()
        modified, replacements = re.subn(r'uint32_t(\s+virAddr;)', r'uintptr_t\1', original)
        if replacements != 1:
            raise SystemExit('Pinned encoder address declaration differs; inspect host test seam')
        (fixture / 'imp_encoder.h').write_text(
            '/* HOST TEST ONLY: virAddr widened; original compiled separately for O32. */\n'
            + modified
        )
        executable = temporary / 'test-bridge'
        run([*shlex.split(os.environ.get('CC', 'clang')), *common,
             '-Wno-keyword-macro', '-fsanitize=address,undefined', '-fno-omit-frame-pointer', '-g',
             '-I', temporary / 'host-include',
             '-I', ROOT / 'sdk/include-compat', '-I', headers,
             ROOT / 'sdk/test_imp_bridge.c', '-o', executable])
        run([executable])
        print('PASS: production MIPS O32 ABI assertions compiled; host-only test seam completed')


if __name__ == '__main__':
    main()
