"""The generated interfaces preserve symbols/versions while containing no runtime code."""
import copy
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest

import link_stubs

ROOT = Path(__file__).resolve().parents[1]


def sections(blob):
    header = struct.unpack_from('<HHIIIIIHHHHHH', blob, 16)
    offset, count, names_index = header[5], header[11], header[12]
    records = [struct.unpack_from('<10I', blob, offset + i * 40) for i in range(count)]
    names = records[names_index]
    names = blob[names[4]:names[4] + names[5]]
    return {names[r[0]:].split(b'\0', 1)[0].decode(): r for r in records}


class LinkInterfaces(unittest.TestCase):
    def test_all_interfaces_preserve_the_recorded_dynamic_symbols_and_versions(self):
        for path in (ROOT / 'abi').glob('*/*.json'):
            with self.subTest(path=path):
                manifest = json.loads(path.read_text())
                blob = link_stubs.generate(manifest)
                self.assertEqual(blob, link_stubs.generate(manifest))
                self.assertEqual(struct.unpack_from('<H', blob, 44)[0], 0)  # No program headers.
                self.assertEqual(struct.unpack_from('<I', blob, 36)[0], manifest['flags'])
                table = sections(blob)
                self.assertEqual(table['.text'][1], 8)  # NOBITS, no implementation.
                strings = table['.dynstr']
                strings = blob[strings[4]:strings[4] + strings[5]]
                symbols = table['.dynsym']
                self.assertEqual(symbols[5] // 16, len(manifest['symbols']) + 1)
                for index, expected in enumerate(manifest['symbols'], 1):
                    name, _, size, info, other, section = struct.unpack_from('<IIIBBH', blob, symbols[4] + index * 16)
                    self.assertEqual(strings[name:].split(b'\0', 1)[0].decode(), expected['name'])
                    self.assertEqual((size, info, other), (expected['size'], expected['info'], expected['other']))
                    self.assertEqual(section != 0, expected['defined'])
                    if manifest['versioned']:
                        actual = struct.unpack_from('<H', blob, table['.gnu.version'][4] + index * 2)[0]
                        self.assertEqual(actual, expected['version'])
                if manifest['mips_abiflags']:
                    abi = table['.MIPS.abiflags']
                    self.assertEqual(blob[abi[4]:abi[4] + abi[5]].hex(), manifest['mips_abiflags'])

    def test_unsupported_or_unsafe_interfaces_are_refused(self):
        sample = json.loads((ROOT / 'abi/reolink-e1-zoom/hdal.json').read_text())
        for edit in [lambda m: m.update(machine=0), lambda m: m.update(needed=['../lib.so']),
                     lambda m: m['symbols'][0].update(size=-1),
                     lambda m: m['symbols'][0].update(info=26)]:
            candidate = copy.deepcopy(sample)
            edit(candidate)
            with self.assertRaises(ValueError): link_stubs.generate(candidate)

    def test_generation_is_idempotent_and_separate_from_source(self):
        with tempfile.TemporaryDirectory() as directory:
            outputs = link_stubs.install('mi360', Path(directory))
            self.assertEqual(len(outputs), 7)
            before = [(p.stat().st_mtime_ns, hashlib.sha256(p.read_bytes()).hexdigest()) for p in outputs]
            link_stubs.install('mi360', Path(directory))
            self.assertEqual(before, [(p.stat().st_mtime_ns, hashlib.sha256(p.read_bytes()).hexdigest()) for p in outputs])


if __name__ == '__main__': unittest.main()
