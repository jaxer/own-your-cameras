"""fetch-hdal-headers against a file:// stand-in for GitHub: no network, no real headers."""
import hashlib
from importlib.machinery import SourceFileLoader
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def load():
    loader = SourceFileLoader("fetch_hdal_headers", str(ROOT / "scripts/fetch-hdal-headers"))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


fetch = load()

UPSTREAM = {"a.h": b"int a;\nint b = 1;\nint c;\n", "vendor/v.h": b"#define V 1\n"}
PATCH = b"""our fix
--- a/a.h
+++ b/a.h
@@ -1,3 +1,3 @@
 int a;
-int b = 1;
+int b = 2;
 int c;
"""
PATCHED = {"a.h": b"int a;\nint b = 2;\nint c;\n", "vendor/v.h": UPSTREAM["vendor/v.h"]}


def sha(data):
    return hashlib.sha256(data).hexdigest()


class FetchHeadersTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.upstream = self.root / "upstream"
        for name, data in UPSTREAM.items():
            (self.upstream / name).parent.mkdir(parents=True, exist_ok=True)
            (self.upstream / name).write_bytes(data)
        self.patch = self.root / "hdal-include.patch"
        self.patch.write_bytes(PATCH)
        self.dest = self.root / "vendor/hdal-include"
        self.write_lock(PATCHED)

    def write_lock(self, patched, upstream=UPSTREAM, url=None):
        self.lock = self.root / "hdal-include.lock"
        lines = ["# test lock", f"url {url or self.upstream.as_uri() + '/'}"]
        lines += [f"{sha(upstream[name])} {sha(patched[name])} {name}" for name in sorted(upstream)]
        self.lock.write_text("\n".join(lines) + "\n")

    def run_fetch(self):
        return fetch.install(self.lock, self.patch, self.dest)

    def installed(self):
        return {name: (self.dest / name).read_bytes() for name in PATCHED if (self.dest / name).exists()}

    def test_fetches_patches_and_installs_the_locked_set(self):
        self.assertTrue(self.run_fetch())
        self.assertEqual(self.installed(), PATCHED)

    def test_upstream_hash_mismatch_is_refused_and_names_the_file(self):
        (self.upstream / "vendor/v.h").write_bytes(b"#define V 2\n")
        with self.assertRaisesRegex(SystemExit, r"vendor/v\.h.*upstream") as caught:
            self.run_fetch()
        self.assertIn(sha(UPSTREAM["vendor/v.h"]), str(caught.exception))
        self.assertFalse(self.dest.exists())

    def test_patch_failure_is_refused(self):
        self.patch.write_bytes(PATCH.replace(b" int c;", b" int d;"))
        with self.assertRaisesRegex(SystemExit, "hdal-include.patch"):
            self.run_fetch()
        self.assertFalse(self.dest.exists())

    def test_patched_hash_mismatch_is_refused(self):
        self.write_lock({**PATCHED, "a.h": b"something else\n"})
        with self.assertRaisesRegex(SystemExit, r"a\.h.*patched"):
            self.run_fetch()
        self.assertFalse(self.dest.exists())

    def test_present_set_is_a_no_op_without_network(self):
        self.run_fetch()
        self.write_lock(PATCHED, url="http://127.0.0.1:9/unreachable/")
        self.assertFalse(self.run_fetch())
        self.assertEqual(self.installed(), PATCHED)

    def test_partial_set_is_replaced_whole(self):
        self.dest.mkdir(parents=True)
        (self.dest / "a.h").write_bytes(PATCHED["a.h"])
        (self.dest / "stale.h").write_bytes(b"old\n")
        self.assertTrue(self.run_fetch())
        self.assertEqual(self.installed(), PATCHED)
        self.assertFalse((self.dest / "stale.h").exists())
        self.assertEqual(sorted(p.name for p in self.dest.parent.iterdir()), ["hdal-include"])

    def test_failed_fetch_leaves_a_partial_set_untouched(self):
        self.dest.mkdir(parents=True)
        (self.dest / "a.h").write_bytes(b"partial\n")
        (self.upstream / "a.h").unlink()
        with self.assertRaisesRegex(SystemExit, r"a\.h"):
            self.run_fetch()
        self.assertEqual(self.installed(), {"a.h": b"partial\n"})
        self.assertEqual(sorted(p.name for p in self.dest.parent.iterdir()), ["hdal-include"])

    def test_real_lock_pins_eighteen_headers_at_one_commit(self):
        url, entries = fetch.read_lock(ROOT / "hdal-sys/hdal-include.lock")
        self.assertEqual(url, "https://raw.githubusercontent.com/yanggungg/trunk_gtk4_dev/"
                              "69386cae2e39480b3969165bac3bd4a86425335a/src/include/novatek.nt9833x/")
        self.assertEqual(len(entries), 18)
        changed = sorted(name for name, upstream, patched in entries if upstream != patched)
        self.assertEqual(changed, ["hd_type.h", "hd_videoenc.h", "vendor/vendor_common.h"])


if __name__ == "__main__":
    unittest.main()
