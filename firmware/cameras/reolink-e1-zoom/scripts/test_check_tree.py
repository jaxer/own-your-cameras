"""check-tree on throwaway git repositories: vendor bytes are refused by path, untracked files never count."""
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("check-tree")
ROOT = Path(__file__).resolve().parents[1]


class CheckTreeTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.repo = Path(temp.name)
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        self.add("README.md", b"ours\n")

    def add(self, name, data):
        path = self.repo / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        subprocess.run(["git", "-C", str(self.repo), "add", "-f", name], check=True)

    def check(self):
        return subprocess.run(["python3", "-I", str(SCRIPT), str(self.repo)], capture_output=True, text=True)

    def assert_refused(self, name):
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(name, result.stderr)

    def test_clean_tree_passes(self):
        self.assertEqual(self.check().returncode, 0, self.check().stderr)

    def test_a_committed_shared_library_is_refused(self):
        self.add("lib/libhdal.so", b"not even an elf")
        subprocess.run(["git", "-C", str(self.repo), "-c", "user.name=t", "-c", "user.email=t@example.invalid",
                        "commit", "-qm", "add a library"], check=True)
        self.assert_refused("lib/libhdal.so")

    def test_versioned_library_names_and_elf_files_are_refused(self):
        self.add("libc.so.6", b"x")
        self.add("tools/helper", b"\x7fELF\x01\x01\x01")
        result = self.check()
        self.assertIn("libc.so.6", result.stderr)
        self.assertIn("tools/helper", result.stderr)

    def test_anything_under_hdal_sys_vendor_is_refused(self):
        self.add("hdal-sys/vendor/hdal-include/hd_type.h", b"/* header */\n")
        self.assert_refused("hdal-sys/vendor/hdal-include/hd_type.h")

    def test_novatek_copyright_is_allowed_only_in_the_patch(self):
        notice = b"Copyright " + b"Novatek Microelectronics Corp.\n"
        self.add("hdal-sys/hdal-include.patch", notice)
        self.assertEqual(self.check().returncode, 0)
        self.add("cam/src/copied.h", notice)
        self.assert_refused("cam/src/copied.h")

    def test_stock_start_app_text_is_refused(self):
        self.add("image/start_app", b"\t/mnt/app/spawn-" + b"fcgi -a 127.0.0.1 -p 9527 -f x\n")
        self.assert_refused("image/start_app")

    def test_large_files_are_refused(self):
        self.add("big.bin", bytes(1024 * 1024 + 1))
        self.assert_refused("big.bin")

    def test_untracked_vendor_files_are_refused(self):
        (self.repo / "vendor/sysroot/usr/lib").mkdir(parents=True)
        (self.repo / "vendor/sysroot/usr/lib/libhdal.so").write_bytes(b"\x7fELF")
        self.assertNotEqual(self.check().returncode, 0)
        self.add(".gitignore", b"vendor/sysroot/\n")
        self.assertEqual(self.check().returncode, 0)

    def test_this_repository_passes(self):
        result = subprocess.run(["python3", "-I", str(SCRIPT), str(ROOT)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
