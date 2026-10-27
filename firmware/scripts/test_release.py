"""Release boundaries: link stubs and invalid models cannot become runtime assets."""
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

import bundle_image
import link_stubs
import release


class ReleaseBoundary(unittest.TestCase):
    def test_source_download_falls_back_without_relaxing_hash(self):
        data = b'isolated source fixture'
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / 'source.tar.bz2'
            with patch.object(release, 'DROPBEAR_SHA256', release.digest(data)), patch.object(
                    release.urllib.request, 'urlopen', side_effect=[
                        urllib.error.HTTPError(release.DROPBEAR_URLS[0], 403, 'Forbidden', {}, None),
                        io.BytesIO(data)]) as opener:
                release.download_dropbear(archive)
            self.assertEqual(archive.read_bytes(), data)
            self.assertEqual(opener.call_count, 2)

    def test_source_download_rejects_wrong_content_without_writing_it(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory) / 'source.tar.bz2'
            with patch.object(release.urllib.request, 'urlopen', return_value=io.BytesIO(b'wrong source')):
                with self.assertRaisesRegex(ValueError, 'digest mismatch'):
                    release.download_dropbear(archive)
            self.assertFalse(archive.exists())

    def test_generated_link_interface_cannot_be_packaged_as_a_runtime_binary(self):
        manifest = json.loads((release.ROOT / 'firmware/abi/reolink-e1-zoom/hdal.json').read_text())
        with self.assertRaisesRegex(ValueError, 'link stubs'):
            release.check_binary(link_stubs.generate(manifest), 40)

    def test_binary_architecture_and_bounds_are_checked(self):
        for data in [b'', b'\x7fELF' + bytes(100), b'not an executable']:
            with self.assertRaises(ValueError): release.check_binary(data, 8)

    def test_builder_sources_exclude_build_inputs_and_deployment_material(self):
        for model in release.MODELS:
            paths = release.source_paths(model)
            self.assertTrue(paths)
            for path in paths:
                self.assertTrue(path.is_file(), path)
                self.assertFalse(any(p in path.parts for p in ('.tools', 'target', 'vendor', 'private', 'backups')))
                self.assertNotIn(path.suffix, ('.so', '.pak', '.bin', '.key'))

    def test_image_wrapper_refuses_modified_or_outside_files_before_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            record = dict(schema=1, model='mi360', files={'payload': '0' * 64})
            (root / 'manifest.json').write_text(json.dumps(record))
            (root / 'payload').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'differs'): bundle_image.verify(root)
            record['files'] = {'../outside': '0' * 64}
            (root / 'manifest.json').write_text(json.dumps(record))
            with self.assertRaisesRegex(ValueError, 'Unsafe'): bundle_image.verify(root)


if __name__ == '__main__': unittest.main()
