#!/usr/bin/env python3
"""Build a personalized image from a verified binary release. Never contacts a camera."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def verify(root):
    manifest = json.loads((root / 'manifest.json').read_text())
    if manifest.get('schema') != 1 or manifest.get('model') not in ('mi360', 'reolink-e1-zoom'):
        raise ValueError('Unsupported release manifest')
    for name, expected in manifest['files'].items():
        path = root / name
        if path.is_symlink() or not path.resolve().is_relative_to(root.resolve()):
            raise ValueError('Unsafe bundle path: ' + name)
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError('Release file differs from its manifest: ' + name)
    return manifest


def main():
    os.umask(0o077)
    root = Path(__file__).resolve().parent
    manifest = verify(root)
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--authorized-key', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    if manifest['model'] == 'mi360':
        p.add_argument('--original-dump', type=Path, required=True)
        p.add_argument('--expected-original-sha256', required=True)
    else:
        p.add_argument('--stock-pak', type=Path, required=True)
    args = p.parse_args()
    source = root / 'source'
    if manifest['model'] == 'mi360':
        command = [sys.executable, source / 'firmware/cameras/mi360/firmware/build_image.py',
                   '--original-dump', args.original_dump.resolve(),
                   '--expected-original-sha256', args.expected_original_sha256,
                   '--ssh-binary', root / 'bin/dropbearmulti-mipsel',
                   '--bootstrap-writer', root / 'bin/bootstrap-camera-mtd-write',
                   '--authorized-key', args.authorized_key.resolve(),
                   '--cam', root / 'bin/mi360-cam',
                   '--writer', root / 'writer/camera-mtd-write', '--output', args.output.resolve()]
    else:
        command = [sys.executable, '-I', source / 'firmware/cameras/reolink-e1-zoom/scripts/cutover_image.py',
                   '--stock-pak', args.stock_pak.resolve(), '--own-your-cameras', source,
                   '--ssh-binary', root / 'bin/dropbearmulti-cortex-a9',
                   '--authorized-key', args.authorized_key.resolve(),
                   '--cam', root / 'bin/cam', '--launcher', root / 'bin/cam-launch',
                   '--output-dir', args.output.resolve()]
    subprocess.run([str(x) for x in command], check=True)


if __name__ == '__main__':
    main()
