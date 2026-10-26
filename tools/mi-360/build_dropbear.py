#!/usr/bin/env python3
"""Build the pinned static MIPS32r2 Dropbear and MTD helper locally, without downloads."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile

from firmware import check_static_mips, digest

BASE = Path(__file__).resolve().parent
SOURCE_SHA256 = 'e098034a843699200c8c977a991fff73159735bf795d5f72ef672c41a6b1ae81'
CFLAGS = '-Oz -fstack-protector-strong -ffunction-sections -fdata-sections -fno-unwind-tables -fno-asynchronous-unwind-tables'
LDFLAGS = '-static -s -Wl,--gc-sections -Wl,-z,relro -Wl,-z,now'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_archive', type=Path)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    if digest(args.source_archive.read_bytes()) != SOURCE_SHA256:
        raise ValueError('Use the exact Dropbear 2026.94 release tarball')
    if subprocess.check_output(['zig', 'version'], text=True).strip() != '0.17.0':
        raise ValueError('Match Zig 0.17.0 for the tested build recipe')
    output = args.output_dir.resolve()
    output.mkdir(mode=0o700)
    with tarfile.open(args.source_archive) as archive:
        archive.extractall(output, filter='data')
    source = output / 'dropbear-2026.94'
    shutil.copyfile(BASE / 'localoptions.h', source / 'localoptions.h')
    configure = ['./configure', '--host=mipsel-linux-musleabi',
                 'CC=zig cc -target mipsel-linux-musleabi -mcpu=mips32r2',
                 'AR=zig ar', 'RANLIB=zig ranlib', 'CFLAGS=' + CFLAGS, 'LDFLAGS=' + LDFLAGS,
                 '--disable-harden', '--disable-zlib', '--disable-syslog', '--disable-utmp',
                 '--disable-utmpx', '--disable-wtmp', '--disable-wtmpx', '--disable-lastlog',
                 '--disable-loginfunc']
    # Avoid configure's static PIE; keep explicit stack protection, RELRO and NOW.
    for name, command in [('configure', configure), ('make',
            ['make', '-j4', 'PROGRAMS=dropbear dropbearkey', 'MULTI=1'])]:
        with (output / (name + '.log')).open('wb') as log:
            subprocess.run(command, cwd=source, stdout=log, stderr=subprocess.STDOUT, check=True)
    worker = output / 'dropbearmulti-mipsel'
    shutil.copyfile(source / 'dropbearmulti', worker)
    worker.chmod(0o755)
    writer = output / 'camera-mtd-write'
    subprocess.run(['zig', 'cc', '-target', 'mipsel-linux-musleabi', '-mcpu=mips32r2',
                    *CFLAGS.split(), *LDFLAGS.split(), '-Wall', '-Wextra', '-Werror',
                    str(BASE / 'camera-mtd-write.c'), '-o', str(writer)], check=True)
    for binary in (worker, writer):
        check_static_mips(binary.read_bytes())
    report = {'dropbear_version': '2026.94', 'zig_version': '0.17.0',
              'source_sha256': SOURCE_SHA256, 'target': 'mipsel-linux-musleabi/mips32r2',
              'cflags': CFLAGS, 'ldflags': LDFLAGS,
              'binaries': {path.name: {'bytes': path.stat().st_size,
                                     'sha256': digest(path.read_bytes())}
                           for path in (worker, writer)}}
    (output / 'manifest.json').write_text(json.dumps(report, indent=2) + '\n')
    print('Built dropbearmulti-mipsel and camera-mtd-write; provenance in manifest.json.')


if __name__ == '__main__':
    main()
