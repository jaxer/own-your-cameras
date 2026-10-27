"""fetch-sysroot against a fake `ssh`: the hash gate, the CAMERA requirement, no camera when present."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("fetch-sysroot")
LIBHDAL_SHA256 = "7e8b23c12e69271f3c81132e98ab9eacbf340d4d2f0c3e94cd4b8f44946539b3"


class FetchSysrootTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.repo = Path(temp.name) / "repo"
        (self.repo / "scripts").mkdir(parents=True)
        shutil.copy(SCRIPT, self.repo / "scripts")
        self.bin = Path(temp.name) / "bin"
        self.bin.mkdir()
        self.calls = Path(temp.name) / "ssh-calls"
        self.served = Path(temp.name) / "libhdal.so"
        ssh = self.bin / "ssh"
        ssh.write_text(f'#!/bin/sh\necho "$@" >> {self.calls}\ncat {self.served}\n')
        ssh.chmod(0o755)
        self.lib = self.repo / "vendor/sysroot/usr/lib/libhdal.so"

    def run_script(self, camera="mycam"):
        env = {**os.environ, "PATH": f"{self.bin}:{os.environ['PATH']}"}
        env.pop("CAMERA", None)
        if camera:
            env["CAMERA"] = camera
        return subprocess.run([str(self.repo / "scripts/fetch-sysroot")], env=env,
                              capture_output=True, text=True)

    def test_refuses_a_library_that_is_not_the_4417_one(self):
        self.served.write_bytes(b"other firmware")
        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("4417", result.stderr)
        self.assertIn(hashlib.sha256(b"other firmware").hexdigest(), result.stderr)
        self.assertFalse(self.lib.exists())
        self.assertEqual(list(self.lib.parent.iterdir()), [])

    def test_needs_camera_when_the_library_is_missing(self):
        result = self.run_script(camera=None)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("CAMERA=", result.stderr)
        self.assertFalse(self.calls.exists())

    def test_copies_only_libhdal_read_only(self):
        # The real library is not in the tree: fake its hash check by serving bytes and pinning them.
        script = self.repo / "scripts/fetch-sysroot"
        data = b"pretend libhdal"
        script.write_text(script.read_text().replace(LIBHDAL_SHA256, hashlib.sha256(data).hexdigest()))
        self.served.write_bytes(data)
        result = self.run_script()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.lib.read_bytes(), data)
        self.assertEqual(self.calls.read_text(), "mycam cat /usr/lib/libhdal.so\n")
        self.assertEqual(sorted(p.name for p in (self.repo / "vendor/sysroot").rglob("*")),
                         ["lib", "libhdal.so", "usr"])
        self.calls.unlink()
        again = self.run_script(camera=None)
        self.assertEqual(again.returncode, 0, again.stderr)
        self.assertFalse(self.calls.exists())


if __name__ == "__main__":
    unittest.main()
