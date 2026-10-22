#!/usr/bin/env python3
"""Isolated tests; --stock-pak enables checks against real matching firmware."""
import argparse
import contextlib
import io
import json
from pathlib import Path
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch

import build
import ubi
import upload

STOCK_PAK = None
BUILD_DIR = None


class FakeSession:
    """In-memory unit fixture. Never opens a connection."""
    def __init__(self, fail_command=None, fail_chunk=None, identity=None):
        self.fail_command = fail_command
        self.fail_chunk = fail_chunk
        self.identity = identity if identity is not None else upload.EXPECTED.copy()
        self.calls = []
        self.requests = []

    def call(self, command, param=None):
        self.calls.append((command, param))
        if command == self.fail_command:
            raise RuntimeError('Unit fixture failure')
        return {'value': {'DevInfo': self.identity}}

    def request(self, command, body, content_type, extra=None):
        self.requests.append((command, body, content_type, extra))
        if len(self.requests) == self.fail_chunk:
            raise RuntimeError('Unit fixture transport failure')
        return {'cmd': command, 'code': 0}


class UploadSafetyTests(unittest.TestCase):
    def test_prepare_failure_sends_no_chunks(self):
        session = FakeSession(fail_command='UpgradePrepare')
        with self.assertRaises(RuntimeError):
            upload.upload(session, Path('candidate.pak'), b'test')
        self.assertEqual(session.requests, [])

    def test_identity_mismatch_precedes_prepare(self):
        session = FakeSession(identity={})
        with self.assertRaises(ValueError):
            upload.upload(session, Path('candidate.pak'), b'test')
        self.assertEqual([call[0] for call in session.calls], ['GetDevInfo'])
        self.assertEqual(session.requests, [])

    def test_transport_failure_stops_without_retry_or_reboot(self):
        session = FakeSession(fail_chunk=2)
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(RuntimeError):
            upload.upload(session, Path('candidate.pak'), bytes(3 * upload.CHUNK_BYTES))
        self.assertEqual(len(session.requests), 2)
        self.assertEqual([call[0] for call in session.calls], ['GetDevInfo', 'UpgradePrepare'])

    def test_retains_settings_and_chunk_order(self):
        session = FakeSession()
        data = b'a' * upload.CHUNK_BYTES + b'final-part'
        with contextlib.redirect_stdout(io.StringIO()):
            upload.upload(session, Path('candidate.pak'), data)
        self.assertEqual(session.calls[1], ('UpgradePrepare', {'restoreCfg': 0, 'fileName': 'candidate.pak'}))
        self.assertEqual(len(session.requests), 2)
        for index, (_, body, _, query) in enumerate(session.requests):
            self.assertEqual(query, {'file': 'upgrade-package', 'clearConfig': 0})
            self.assertIn(f'&{index}&{len(data)}"'.encode(), body)
            payload = body.split(b'\r\n\r\n', 1)[1].rsplit(b'\r\n--', 1)[0]
            self.assertEqual(payload, data[index * upload.CHUNK_BYTES:(index + 1) * upload.CHUNK_BYTES])
        self.assertEqual([call[0] for call in session.calls], ['GetDevInfo', 'UpgradePrepare'])


class RealStockTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if STOCK_PAK is None:
            raise unittest.SkipTest('Supply --stock-pak for real-artifact checks')
        cls.pak = STOCK_PAK
        cls.data = cls.pak.read_bytes()
        cls.template, _ = ubi.read_stock_pak(cls.pak)
        cls.squashfs = ubi.parse_ubi(cls.template)['payload']

    def test_stock_ubi_roundtrip_is_identical(self):
        result, manifest = ubi.repack(self.template, self.squashfs)
        self.assertEqual(result, self.template)
        self.assertTrue(manifest['validation']['preserves_stock_squashfs_roundtrip'])

    def test_package_roundtrip_preserves_entire_file(self):
        result, manifest = build.make_pak(self.data, self.template)
        self.assertEqual(result, self.data)
        self.assertTrue(manifest['mtd_table_byte_identical'])

    def test_corrupt_headers_and_redundant_table_rejected(self):
        for offset in [8, ubi.VID_OFFSET + 40, ubi.PEB_SIZE + ubi.DATA_OFFSET]:
            damaged = bytearray(self.template)
            damaged[offset] ^= 1
            with self.subTest(offset=offset), self.assertRaises(ValueError):
                ubi.parse_ubi(damaged)

    def test_duplicate_logical_block_rejected_with_valid_crc(self):
        damaged = bytearray(self.template)
        offset = 3 * ubi.PEB_SIZE + ubi.VID_OFFSET
        vid = bytearray(damaged[offset:offset + 64])
        struct.pack_into('>I', vid, 12, 0)
        ubi.refresh_crc(vid)
        damaged[offset:offset + 64] = vid
        with self.assertRaisesRegex(ValueError, 'numbering'):
            ubi.parse_ubi(damaged)

    def test_capacity_guard_rejects_oversize(self):
        # Invalid synthetic extension stays in this isolated memory test only.
        size = 112 * ubi.LEB_SIZE + 1
        extended = bytearray(self.squashfs + bytes(size - len(self.squashfs)))
        struct.pack_into('<Q', extended, 40, size)
        with self.assertRaisesRegex(ValueError, 'partition/spare-PEB bound'):
            ubi.repack(self.template, extended)

    def test_hash_mismatch_rejected(self):
        with self.assertRaisesRegex(ValueError, 'SHA-256'):
            upload.checked_image(self.pak, '0' * 64)

    def test_default_upload_mode_is_offline(self):
        args = ['upload.py', str(self.pak), '--sha256', ubi.STOCK_SHA256]
        with patch.object(sys, 'argv', args), patch.object(upload, 'Session') as session:
            with contextlib.redirect_stdout(io.StringIO()):
                upload.main()
        session.assert_not_called()

    def test_corrupt_package_crc_rejected_even_with_matching_hash(self):
        damaged = bytearray(self.data)
        damaged[-1] ^= 1
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'candidate.pak'
            path.write_bytes(damaged)
            with self.assertRaisesRegex(ValueError, 'CRC'):
                upload.checked_image(path, ubi.sha256(damaged))



class RealCandidateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if STOCK_PAK is None or BUILD_DIR is None:
            raise unittest.SkipTest('Supply --stock-pak and --build-dir for final-candidate checks')
        try:
            import verify
        except ImportError:
            raise unittest.SkipTest('Install validation requirements for candidate checks')
        cls.verifier = verify

    def test_independent_candidate_passes(self):
        self.assertTrue(self.verifier.verify(STOCK_PAK, BUILD_DIR)['passed'])

    def test_replaced_pak_rejected_even_with_sibling_exports_intact(self):
        manifest = json.loads((BUILD_DIR / 'manifest.json').read_text())
        candidate = Path(manifest['candidate']['path'])
        read_bytes = Path.read_bytes
        def altered(path):
            data = read_bytes(path)
            return data[:-1] + bytes([data[-1] ^ 1]) if path == candidate else data
        with patch.object(Path, 'read_bytes', altered), self.assertRaisesRegex(AssertionError, 'SHA-256'):
            self.verifier.verify(STOCK_PAK, BUILD_DIR)

    def test_wrong_authorized_key_manifest_rejected(self):
        manifest_path = BUILD_DIR / 'manifest.json'
        read_text = Path.read_text
        def altered(path, *args, **kwargs):
            text = read_text(path, *args, **kwargs)
            if path == manifest_path:
                data = json.loads(text)
                data['authorized_key_sha256'] = '0' * 64
                return json.dumps(data)
            return text
        with patch.object(Path, 'read_text', altered), self.assertRaises(AssertionError):
            self.verifier.verify(STOCK_PAK, BUILD_DIR)

    def test_valid_hash_does_not_hide_bad_pak_crc(self):
        manifest_path = BUILD_DIR / 'manifest.json'
        original_manifest = json.loads(manifest_path.read_text())
        candidate_path = Path(original_manifest['candidate']['path'])
        damaged = bytearray(candidate_path.read_bytes())
        damaged[4] ^= 1
        read_text, read_bytes = Path.read_text, Path.read_bytes
        def altered_bytes(path):
            return bytes(damaged) if path == candidate_path else read_bytes(path)
        def altered_text(path, *args, **kwargs):
            if path == manifest_path:
                updated = dict(original_manifest)
                updated['candidate'] = dict(updated['candidate'], sha256=ubi.sha256(damaged))
                return json.dumps(updated)
            return read_text(path, *args, **kwargs)
        with patch.object(Path, 'read_text', altered_text), patch.object(Path, 'read_bytes', altered_bytes):
            with self.assertRaisesRegex(AssertionError, 'CRC'):
                self.verifier.verify(STOCK_PAK, BUILD_DIR)

if __name__ == '__main__':
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument('--stock-pak', type=Path)
    parser.add_argument('--build-dir', type=Path)
    args, rest = parser.parse_known_args()
    STOCK_PAK = args.stock_pak
    BUILD_DIR = args.build_dir
    unittest.main(argv=[sys.argv[0], *rest])
