#!/usr/bin/env python3
"""Verify/copy the maintained local IMP interface; never download vendor headers."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--from-directory', type=Path)
    args = parser.parse_args()
    source = args.from_directory or ROOT / 'sdk'
    lock = json.loads((ROOT / 'sdk/headers.lock.json').read_text())
    args.out.mkdir(parents=True, exist_ok=True)
    for name, digest in lock['headers'].items():
        data = (source / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != digest:
            raise ValueError('Local interface differs from its reviewed pin: ' + name)
        target = args.out / name
        if not target.is_file() or target.read_bytes() != data:
            target.write_bytes(data)
    print('Verified local IMP interface')


if __name__ == '__main__':
    main()
