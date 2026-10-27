#!/usr/bin/env python3
"""Put PT_TLS last for the exact legacy uClibc loader's early-stop scanner.

This is a permutation of existing program-header records only. No segment bytes,
addresses, permissions, sizes, relocations or dependency records are changed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct


def reorder(blob):
    if blob[:7] != b'\x7fELF\x01\x01\x01':
        raise ValueError('Requires ELF32 little endian')
    fields = struct.unpack_from('<HHIIIIIHHHHHH', blob, 16)
    etype, machine = fields[:2]
    phoff, ehsize, phentsize, phnum = fields[4], fields[7], fields[8], fields[9]
    if etype != 2 or machine != 8 or ehsize != 52 or phentsize != 32 or not phnum:
        raise ValueError('Requires MIPS ET_EXEC with canonical ELF32 headers')
    end = phoff + phentsize * phnum
    if phoff < ehsize or end > len(blob):
        raise ValueError('Invalid program-header extent')
    records = [blob[phoff+i*32:phoff+(i+1)*32] for i in range(phnum)]
    kinds = [struct.unpack_from('<I', record)[0] for record in records]
    if kinds.count(7) != 1 or kinds.count(2) != 1:
        raise ValueError('Requires exactly one PT_TLS and PT_DYNAMIC')
    tls = kinds.index(7)
    order = [i for i in range(phnum) if i != tls] + [tls]
    patched = blob[:phoff] + b''.join(records[i] for i in order) + blob[end:]
    # Independent invariants around the transformation.
    after = [patched[phoff+i*32:phoff+(i+1)*32] for i in range(phnum)]
    if sorted(after) != sorted(records):
        raise AssertionError('Program-header records changed')
    if [r for r in after if struct.unpack_from('<I',r)[0] == 1] != [r for r in records if struct.unpack_from('<I',r)[0] == 1]:
        raise AssertionError('PT_LOAD order changed')
    if patched[:phoff] != blob[:phoff] or patched[end:] != blob[end:]:
        raise AssertionError('Bytes outside program-header table changed')
    return patched, {'before_order': kinds, 'after_order': [kinds[i] for i in order], 'program_headers_offset':phoff, 'program_headers_bytes':phnum*32}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary',type=Path)
    parser.add_argument('--report',type=Path,required=True)
    args=parser.parse_args()
    original=args.binary.read_bytes()
    patched,report=reorder(original)
    assert reorder(patched)[0] == patched
    report.update(before_sha256=hashlib.sha256(original).hexdigest(),after_sha256=hashlib.sha256(patched).hexdigest(),changed=original!=patched)
    if patched != original:args.binary.write_bytes(patched)
    args.report.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))

if __name__ == '__main__':main()
