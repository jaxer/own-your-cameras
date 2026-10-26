#!/usr/bin/env python3
"""Build a B_USRFS-only SSH image offline from your original 16 MiB SPI dump."""
import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess

from firmware import (KERNEL_SIZE, KERNEL_START, TIMESTAMP, USR_SIZE, USR_START,
                      check_dump, check_squashfs, check_static_mips, digest,
                      patch_dump, pseudo_manifest, public_key)

BASE = Path(__file__).resolve().parent


def run(*args):
    return subprocess.run(args, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout


def build(dump, expected_sha256, dropbear, writer, key_path, output):
    stock = dump.read_bytes()
    check_dump(stock, expected_sha256)
    key = public_key(key_path.read_text())
    for path in (dropbear, writer):
        check_static_mips(path.read_bytes())
    for program in ('mksquashfs', 'unsquashfs'):
        # unsquashfs 4.7.5 returns status 1 even for its version output.
        version = subprocess.run([program, '-version'], stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT, check=False)
        if version.returncode not in (0, 1) or b'version 4.7.5 ' not in version.stdout:
            raise ValueError('Match squashfs-tools 4.7.5 for the tested packing recipe')
    output = output.resolve()
    if output.is_relative_to(BASE.parents[1]):
        raise ValueError('Keep generated firmware and private inputs outside this repository')
    output.mkdir(mode=0o700)  # Existing output is deliberately never reused.
    old = output / 'stock-usrfs.bin'
    old.write_bytes(stock[USR_START:USR_START + USR_SIZE])
    release = run('unsquashfs', '-cat', str(old), 'etc/os-release')
    if b'CHUANGMI_VERSION=4.3.4_0376' not in release.splitlines():
        raise ValueError('B_USRFS does not report firmware 4.3.4_0376')
    pseudo = output / 'stock.pseudo'
    run('unsquashfs', '-no-progress', '-pf', str(pseudo), str(old))
    before = pseudo_manifest(pseudo.read_bytes())
    public = output / 'authorized_keys'
    public.write_text(key)
    files = {'bin/dropbear': (dropbear.resolve(), '755'),
             'bin/camera-mtd-write': (writer.resolve(), '755'),
             'bin/camera-update': (BASE / 'camera-update', '755'),
             'etc/root-ssh/authorized_keys': (public, '600'),
             'etc/init.d/S99zz_rootssh': (BASE / 'S99zz_rootssh', '755')}
    added = set(files) | {'etc/root-ssh', 'bin/dropbearkey'}
    if set(before) & added:
        raise ValueError('SSH paths already exist; use the original unmodified dump')
    overlay = [f'etc/root-ssh D {TIMESTAMP} 700 0 0',
               f'bin/dropbearkey S {TIMESTAMP} 777 0 0 dropbear']
    overlay += [f'{name} F {TIMESTAMP} {mode} 0 0 cat {shlex.quote(str(path))}'
                for name, (path, mode) in files.items()]
    overlay_path = output / 'overlay.pseudo'
    overlay_path.write_text('\n'.join(overlay) + '\n')
    empty = output / 'empty'
    empty.mkdir()
    image = output / 'usrfs-ssh.squashfs'
    root = before['/']
    log = run('mksquashfs', str(empty), str(image), '-pf', str(pseudo), '-pf',
              str(overlay_path), '-comp', 'xz', '-b', '131072', '-noappend',
              '-no-xattrs', '-no-progress', '-processors', '2', '-mem', '256M',
              '-mkfs-time', str(TIMESTAMP), '-root-time', str(root['mtime']),
              '-root-mode', root['mode'], '-root-uid', str(root['uid']), '-root-gid', str(root['gid']))
    (output / 'mksquashfs.log').write_bytes(log)
    candidate_pseudo = output / 'candidate.pseudo'
    run('unsquashfs', '-no-progress', '-pf', str(candidate_pseudo), str(image))
    after = pseudo_manifest(candidate_pseudo.read_bytes())
    if set(after) - set(before) != added or set(before) - set(after):
        raise ValueError('Unexpected added or removed filesystem paths')
    if any(after[name] != item for name, item in before.items()):
        raise ValueError('An original file, link or its metadata changed')
    for name, (path, mode) in files.items():
        expected = {'type': 'R', 'mtime': TIMESTAMP, 'mode': mode, 'uid': 0, 'gid': 0,
                    'size': path.stat().st_size, 'sha256': digest(path.read_bytes())}
        if after[name] != expected:
            raise ValueError('Added file contents or metadata differ from build inputs')
    if after['etc/root-ssh'] != {'type': 'D', 'mtime': TIMESTAMP, 'mode': '700',
                                'uid': 0, 'gid': 0, 'extra': []}:
        raise ValueError('SSH key directory metadata mismatch')
    if after['bin/dropbearkey'] != {'type': 'S', 'mtime': TIMESTAMP, 'mode': '777',
                                   'uid': 0, 'gid': 0, 'target': 'dropbear'}:
        raise ValueError('Dropbear key-generator symlink mismatch')
    application = image.read_bytes()
    patched = patch_dump(stock, application)
    (output / 'camera-ssh-16MiB.bin').write_bytes(patched)
    (output / 'kernel-for-update.bin').write_bytes(stock[KERNEL_START:KERNEL_START + KERNEL_SIZE])
    report = {'source_sha256': digest(stock), 'image_sha256': digest(patched),
              'image_bytes': len(patched), 'changed_partition': 'B_USRFS',
              'changed_interval': [USR_START, USR_START + USR_SIZE],
              'protected_bytes_identical': True, 'stock_paths_preserved': len(before),
              'added_paths': sorted(added), 'spare_bytes': USR_SIZE - len(application),
              'usrfs_bytes_used': check_squashfs(application),
              'usrfs_sha256': digest(application),
              'dropbear_sha256': digest(dropbear.read_bytes()),
              'writer_sha256': digest(writer.read_bytes()),
              'hardware_boot_verified': False, 'hardware_flash_verified': False}
    (output / 'manifest.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('dump', type=Path)
    parser.add_argument('--expected-sha256', required=True, help='Your recorded original dump SHA-256')
    parser.add_argument('--ssh-binary', type=Path, required=True)
    parser.add_argument('--writer', type=Path, required=True)
    parser.add_argument('--authorized-key', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True, help='New directory outside this repository')
    args = parser.parse_args()
    os.umask(0o077)
    report = build(args.dump, args.expected_sha256, args.ssh_binary, args.writer,
                   args.authorized_key, args.output_dir)
    print(f"Built camera-ssh-16MiB.bin; {report['stock_paths_preserved']} stock entries preserved; "
          f"{report['spare_bytes']} bytes spare in B_USRFS. No flash operations performed.")


if __name__ == '__main__':
    main()
