#!/usr/bin/env python3
"""Verify the loader fix against the real failed ELF header prefix, never a device."""
from pathlib import Path
import hashlib
import struct
from elf_compat import reorder

base=Path(__file__).resolve().parent
original=bytes.fromhex((base/'fixtures/legacy-loader-header.hex').read_text())
assert hashlib.sha256(original).hexdigest() == '4c94215ebac39d81846197b5a3da020e95838fe1aad1a3e81e62aa9c60aff44b'
fixed, report=reorder(original)
assert report['after_order'][-1] == 7
assert report['after_order'].index(2) < report['after_order'].index(7)
assert reorder(fixed)[0] == fixed
start=report['program_headers_offset'];end=start+report['program_headers_bytes']
assert original[:start] == fixed[:start] and original[end:] == fixed[end:]
assert sorted(original[i:i+32] for i in range(start,end,32)) == sorted(fixed[i:i+32] for i in range(start,end,32))
for broken in [b'', b'not-elf', original[:60]]:
    try: reorder(broken)
    except (ValueError, struct.error): pass
    else: raise AssertionError('Malformed ELF accepted')
wrong_type=bytearray(original);struct.pack_into('<H',wrong_type,16,3)
try: reorder(wrong_type)
except ValueError: pass
else: raise AssertionError('ET_DYN unexpectedly accepted')
no_tls=bytearray(original)
for i in range(start,end,32):
 if struct.unpack_from('<I',no_tls,i)[0] == 7: struct.pack_into('<I',no_tls,i,0)
try: reorder(no_tls)
except ValueError: pass
else: raise AssertionError('Missing TLS unexpectedly accepted')
print('Real ELF-header prefix permutation, immutability, idempotence and refusal checks passed')
