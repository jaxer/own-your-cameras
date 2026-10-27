#!/usr/bin/env python3
"""Fetch or reuse the exact vendor SDK headers; keep their bytes outside source history."""
import argparse
import hashlib
import json
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--from-directory', type=Path)
    args = parser.parse_args()
    lock = json.loads((ROOT/'sdk/headers.lock.json').read_text())
    dest = args.out/'imp'
    dest.mkdir(parents=True, exist_ok=True)
    for source in lock['sources']:
        name = Path(source['path']).name
        target = dest/name
        data = target.read_bytes() if target.exists() else b''
        if hashlib.sha256(data).hexdigest() != source['sha256']:
            if args.from_directory:
                data = (args.from_directory/'imp'/name).read_bytes()
            else:
                with urllib.request.urlopen(source['url'], timeout=20) as response:
                    data = response.read()
            if len(data) != source['bytes'] or hashlib.sha256(data).hexdigest() != source['sha256']:
                raise ValueError('Header does not match the pin: '+name)
            target.write_bytes(data)
    print(f"Verified {len(lock['sources'])} SDK headers from {lock['commit']}")


if __name__ == '__main__':
    main()
