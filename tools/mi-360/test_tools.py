#!/usr/bin/env python3
"""Unit checks; optional real-dump rebuild and reference-image comparison, offline only."""
import argparse
import base64
import ctypes
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
import zlib

from build import build
from firmware import (ENV_START, FLASH_SIZE, KERNEL_START, MARKER, USR_SIZE, USR_START,
                      check_dump, check_preserved, check_static_mips, digest,
                      pseudo_manifest, public_key)

ARTIFACTS = None


class FormatTests(unittest.TestCase):
    def test_public_key_strips_comment_and_rejects_options_or_multiple_keys(self):
        blob = b'\0\0\0\x0bssh-ed25519\0\0\0\x20' + bytes(range(32))
        line = 'ssh-ed25519 ' + base64.b64encode(blob).decode()
        self.assertEqual(public_key(line + ' comment\n'), line + '\n')
        for invalid in ('', line + '\n' + line, 'restrict ' + line, 'ssh-ed25519 !!!',
                        'ssh-ed25519 ' + base64.b64encode(blob[:-1]).decode()):
            with self.subTest(input_kind=invalid[:12]), self.assertRaises(ValueError):
                public_key(invalid)

    def test_elf_requires_static_mips32r2_o32_with_bounded_headers(self):
        data = bytearray(84)
        data[:7] = b'\x7fELF\x01\x01\x01'
        struct.pack_into('<HH', data, 16, 2, 8)
        struct.pack_into('<I', data, 28, 52)
        struct.pack_into('<I', data, 36, 0x70001000)
        struct.pack_into('<HH', data, 42, 32, 1)
        struct.pack_into('<I', data, 52, 1)
        check_static_mips(data)
        for offset, fmt, value in [(18, '<H', 40), (36, '<I', 0x50001000),
                                   (52, '<I', 2), (52, '<I', 3), (28, '<I', 80),
                                   (42, '<H', 0), (44, '<H', 0)]:
            bad = bytearray(data)
            struct.pack_into(fmt, bad, offset, value)
            with self.subTest(offset=offset, value=value), self.assertRaises(ValueError):
                check_static_mips(bad)
        with self.assertRaises(ValueError):
            check_static_mips(data[:51])

    def test_pseudo_payload_cannot_escape_its_bounds(self):
        def pseudo(size, offset, sparse=0):
            return f'file R 1 644 0 0 {size} {offset} {sparse}\n'.encode() + MARKER + b'abc'
        entry = pseudo_manifest(pseudo(3, 0))['file']
        self.assertEqual(entry['sha256'], digest(b'abc'))
        for size, offset, sparse in [(4, 0, 0), (1, -1, 0), (-1, 0, 0), (3, 0, 1)]:
            with self.subTest(size=size, offset=offset, sparse=sparse), self.assertRaises(ValueError):
                pseudo_manifest(pseudo(size, offset, sparse))
        with self.assertRaises(ValueError):
            pseudo_manifest(b'file D 1 755 0 0\nfile D 1 755 0 0\n' + MARKER)

    def test_protected_regions_and_length(self):
        source = bytes(FLASH_SIZE)
        for offset in (USR_START, USR_START + USR_SIZE - 1):
            candidate = bytearray(source)
            candidate[offset] = 1
            check_preserved(source, candidate)
        for offset in (0, ENV_START, USR_START - 1, USR_START + USR_SIZE, FLASH_SIZE - 1):
            candidate = bytearray(source)
            candidate[offset] = 1
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                check_preserved(source, candidate)
        with self.assertRaises(ValueError):
            check_preserved(source, source[:-1])


class RealArtifactTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if ARTIFACTS is None:
            raise unittest.SkipTest('real input arguments not supplied')
        cls.stock = ARTIFACTS.dump.read_bytes()
        cls.source_hash = digest(cls.stock)

    def test_original_dump_and_corruption_refusal(self):
        check_dump(self.stock, self.source_hash)
        for offset in (ENV_START, ENV_START + 4, KERNEL_START, KERNEL_START + 64,
                       USR_START, USR_START + 56):
            bad = bytearray(self.stock)
            bad[offset] ^= 1
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                check_dump(bad, digest(bad))
        with self.assertRaises(ValueError):
            check_dump(self.stock, '0' * 64)
        with self.assertRaises(ValueError):
            check_dump(self.stock[:-1], digest(self.stock[:-1]))

    def test_changed_boot_selection_and_map_even_with_valid_crc(self):
        for original, replacement in [(b'boot_flag=1', b'boot_flag=0'),
                                      (b'imi_version=4.3.4_0376', b'imi_version=4.3.4_0377'),
                                      (b'4544k(B_USRFS)', b'4543k(B_USRFS)')]:
            bad = bytearray(self.stock)
            env = bytes(bad[ENV_START + 4:ENV_START + 0x8000])
            self.assertIn(original, env)
            env = env.replace(original, replacement, 1)
            bad[ENV_START + 4:ENV_START + 0x8000] = env
            struct.pack_into('<I', bad, ENV_START, zlib.crc32(env))
            with self.subTest(setting=original.split(b'=')[0]), self.assertRaises(ValueError):
                check_dump(bad, digest(bad))

    def test_c_validator_against_real_images_and_corruptions(self):
        # Compile just the pure validator, excluding every MTD/device operation.
        with tempfile.TemporaryDirectory(prefix='mi360-validator-') as temporary:
            directory = Path(temporary)
            source = directory / 'validator.c'
            runtime = Path(__file__).with_name('camera-mtd-write.c').resolve()
            source.write_text('#define CAMERA_VALIDATION_ONLY\n#include "' + str(runtime) + '"\n'
                              'int check_image(const unsigned char *b, size_t n, int k) '
                              '{ return validate(b, n, k); }\n')
            library = directory / 'validator.so'
            subprocess.run(['cc', '-shared', '-fPIC', str(source), '-o', str(library)], check=True)
            compiled = ctypes.CDLL(str(library))
            compiled.check_image.argtypes = [ctypes.c_char_p, ctypes.c_size_t, ctypes.c_int]
            compiled.check_image.restype = ctypes.c_int

            def verify(data, kernel, expected):
                self.assertEqual(bool(compiled.check_image(bytes(data), len(data), kernel)), expected)

            kernel = self.stock[KERNEL_START:USR_START]
            payload_end = struct.unpack_from('>I', kernel, 12)[0] + 64
            verify(kernel, True, True)
            verify(kernel[:payload_end], True, True)
            for offset in (0, 4, 12, 24, 29, 31, 64, payload_end - 1, payload_end):
                bad = bytearray(kernel)
                bad[offset] ^= 1
                verify(bad, True, False)
            for bad in (kernel[:63], kernel[:payload_end - 1], kernel + b'\xff'):
                verify(bad, True, False)
            for offset, value in ((12, 0xffffffff), (12, 0), (28, 0x05040203)):
                bad = bytearray(kernel)
                struct.pack_into('>I', bad, offset, value)
                bad[4:8] = bytes(4)
                struct.pack_into('>I', bad, 4, zlib.crc32(bad[:64]))
                verify(bad, True, False)
            application = self.stock[USR_START:USR_START + USR_SIZE]
            verify(application, False, True)
            verify(application[:95], False, False)
            verify(application + b'\xff', False, False)
            for offset in (0, 12, 20, 28, 30):
                bad = bytearray(application)
                bad[offset] ^= 1
                verify(bad, False, False)
            for used in (0, 95, len(application) + 1, 0xffffffffffffffff):
                bad = bytearray(application)
                struct.pack_into('<Q', bad, 40, used)
                verify(bad, False, False)

    def test_rebuild_preserves_dump_and_matches_optional_reference(self):
        if not ARTIFACTS.ssh_binary:
            self.skipTest('supply worker, writer and public key to rebuild')
        with tempfile.TemporaryDirectory(prefix='mi360-test-') as temporary:
            output = Path(temporary) / 'image'
            report = build(ARTIFACTS.dump, self.source_hash, ARTIFACTS.ssh_binary,
                           ARTIFACTS.writer, ARTIFACTS.authorized_key, output)
            candidate = (output / 'camera-ssh-16MiB.bin').read_bytes()
            check_preserved(self.stock, candidate)
            self.assertNotEqual(self.stock[USR_START:USR_START + USR_SIZE],
                                candidate[USR_START:USR_START + USR_SIZE])
            self.assertGreater(report['stock_paths_preserved'], 0)
            self.assertEqual(report, json.loads((output / 'manifest.json').read_text()))
            if ARTIFACTS.reference_image:
                self.assertEqual(candidate, ARTIFACTS.reference_image.read_bytes())
            # Never overwrite a prior candidate directory, even for identical inputs.
            with self.assertRaises(FileExistsError):
                build(ARTIFACTS.dump, self.source_hash, ARTIFACTS.ssh_binary,
                      ARTIFACTS.writer, ARTIFACTS.authorized_key, output)
            self.assertEqual(self.source_hash, digest(ARTIFACTS.dump.read_bytes()))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--dump', type=Path)
    parser.add_argument('--ssh-binary', type=Path)
    parser.add_argument('--writer', type=Path)
    parser.add_argument('--authorized-key', type=Path)
    parser.add_argument('--reference-image', type=Path)
    args = parser.parse_args()
    workers = (args.ssh_binary, args.writer, args.authorized_key)
    if any(workers) and (not all(workers) or args.dump is None):
        parser.error('Rebuild requires --dump, --ssh-binary, --writer and --authorized-key together')
    if args.reference_image and not all(workers):
        parser.error('--reference-image requires all rebuild inputs')
    ARTIFACTS = args if args.dump else None
    unittest.main(argv=[__file__], verbosity=2)
