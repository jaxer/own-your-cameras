#!/usr/bin/env python3
"""Build the pinned key-only Cortex-A9 SSH worker locally; no network calls."""
import argparse
import hashlib
from pathlib import Path
import shutil
import subprocess
import tarfile

SOURCE_SHA256 = 'e098034a843699200c8c977a991fff73159735bf795d5f72ef672c41a6b1ae81'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source_archive', type=Path)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    if hashlib.sha256(args.source_archive.read_bytes()).hexdigest() != SOURCE_SHA256:
        raise SystemExit('Expected the exact Dropbear 2026.94 source archive')
    if subprocess.check_output(['zig', 'version'], text=True).strip() != '0.17.0':
        raise SystemExit('This recipe was checked with Zig 0.17.0; match the compiler')
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=False)
    with tarfile.open(args.source_archive) as archive:
        archive.extractall(output, filter='data')
    source = output / 'dropbear-2026.94'
    shutil.copyfile(Path(__file__).with_name('localoptions.h'), source / 'localoptions.h')
    configure = ['./configure', '--host=arm-linux-musleabihf',
        'CC=zig cc -target arm-linux-musleabihf -mcpu=cortex_a9',
        'AR=zig ar', 'RANLIB=zig ranlib', 'CFLAGS=-Os', 'LDFLAGS=-static -s',
        '--disable-zlib', '--disable-syslog', '--disable-utmp', '--disable-utmpx',
        '--disable-wtmp', '--disable-wtmpx', '--disable-lastlog', '--disable-loginfunc']
    for name, command in [('configure', configure), ('make',
            ['make', '-j4', 'PROGRAMS=dropbear dropbearkey', 'MULTI=1'])]:
        with (output / f'{name}.log').open('wb') as log:
            subprocess.run(command, cwd=source, check=True, stdout=log, stderr=subprocess.STDOUT)
    binary = output / 'dropbearmulti-cortex-a9'
    shutil.copyfile(source / 'dropbearmulti', binary)
    binary.chmod(0o755)
    print(f'{hashlib.sha256(binary.read_bytes()).hexdigest()}  {binary}')


if __name__ == '__main__':
    main()
