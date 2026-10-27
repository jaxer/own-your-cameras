"""Cutover-image acceptance without external tools or hardware; the real build only when its inputs are named."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def load(name):
    sys.path.insert(0, str(ROOT / "scripts"))
    spec = importlib.util.spec_from_file_location(name, ROOT / f"scripts/{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


image = load("cutover_image")
MARKER = b"# START OF DATA - DO NOT MODIFY\n#\n"


class CutoverInventoryTests(unittest.TestCase):
    stock = {"/root": {"mode": 0o40777, "uid": 1050, "gid": 1050, "mtime": 1},
             "/etc/init.d/start_app": {"mode": 0o100755, "sha256": "stock"},
             "/bin/busybox": {"sha256": "busybox"}}
    ssh = image.ssh_entries(b"dropbear", b"ssh-ed25519 KEY\n", b"hook")
    approved = {"/etc/init.d/start_app": {"mode": 0o100755, "sha256": "launcher"},
                "/usr/bin/cam": {"mode": 0o100755, "sha256": "cam"},
                "/usr/bin/cam-launch": {"mode": 0o100755, "sha256": "helper"}}

    def after(self):
        return {**self.stock, "/root": {"mode": 0o40700, "uid": 0, "gid": 0, "mtime": 1},
                **self.ssh, **self.approved}

    def test_stock_plus_ssh_plus_exactly_three_artifacts_is_accepted(self):
        image.validate_inventory(self.stock, self.after(), self.ssh, self.approved)

    def test_any_other_stock_change_is_refused_by_path(self):
        after = self.after()
        after["/bin/busybox"] = {"sha256": "changed"}
        with self.assertRaisesRegex(ValueError, "/bin/busybox"):
            image.validate_inventory(self.stock, after, self.ssh, self.approved)

    def test_a_changed_ssh_addition_is_refused(self):
        after = self.after()
        after["/usr/sbin/dropbear"] = {**after["/usr/sbin/dropbear"], "mode": 0o104755}
        with self.assertRaisesRegex(ValueError, "dropbear"):
            image.validate_inventory(self.stock, after, self.ssh, self.approved)

    def test_added_or_missing_paths_are_refused(self):
        after = self.after()
        after["/etc/extra"] = {"sha256": "x"}
        del after["/usr/bin/dropbearkey"]
        with self.assertRaisesRegex(ValueError, "/etc/extra, /usr/bin/dropbearkey"):
            image.validate_inventory(self.stock, after, self.ssh, self.approved)

    def test_root_must_become_private(self):
        after = self.after()
        after["/root"] = self.stock["/root"]
        with self.assertRaisesRegex(ValueError, "/root"):
            image.validate_inventory(self.stock, after, self.ssh, self.approved)

    def test_pseudo_base_drops_stock_start_app_and_makes_root_private(self):
        header = (b"root D 1724642682 777 1050 1050\n"
                  b"etc/init.d/start_app R 1 755 0 0 10 0 0\n"
                  b"etc/init.d/rcS R 1 755 0 0 10 10 0\n")
        base = image.pseudo_base(header + MARKER + b"payload", MARKER)
        self.assertEqual(base, b"root D 1724642682 700 0 0\netc/init.d/rcS R 1 755 0 0 10 10 0\n"
                               + MARKER + b"payload")
        with self.assertRaisesRegex(ValueError, "start_app"):
            image.pseudo_base(b"root D 1724642682 777 1050 1050\n" + MARKER, MARKER)


class ExpectTests(unittest.TestCase):
    def test_matching_subset_passes(self):
        image.check_expectations({"candidate": "a", "/usr/bin/cam": "b"}, {"candidate": "a"})

    def test_any_difference_or_missing_key_is_refused_with_both_values(self):
        with self.assertRaisesRegex(ValueError, "candidate: built a, expected z"):
            image.check_expectations({"candidate": "a"}, {"candidate": "z"})
        with self.assertRaisesRegex(ValueError, "stock.spub: built None"):
            image.check_expectations({"candidate": "a"}, {"stock.spub": "s"})


REAL = ("STOCK_PAK", "OWN_YOUR_CAMERAS", "SSH_BINARY", "AUTHORIZED_KEY")


@unittest.skipUnless(all(os.environ.get(name) for name in REAL),
                     "set " + ", ".join(REAL) + " (and run after make build) for the real image")
class RealCutoverImageTests(unittest.TestCase):
    def test_build_verifies_and_the_flash_plan_refuses_changed_bytes(self):
        env = {name: os.environ[name] for name in REAL}
        (ROOT / "target").mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=ROOT / "target") as temp:
            out = Path(temp) / "image"
            result = subprocess.run(
                [sys.executable, "-I", str(ROOT / "scripts/cutover_image.py"),
                 "--stock-pak", env["STOCK_PAK"], "--own-your-cameras", env["OWN_YOUR_CAMERAS"],
                 "--ssh-binary", env["SSH_BINARY"], "--authorized-key", env["AUTHORIZED_KEY"],
                 "--output-dir", str(out)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            manifest = json.loads((out / "manifest.json").read_text())
            self.assertTrue(manifest["independent_filesystem_validation"])
            self.assertEqual(set(manifest["artifacts"]), image.ARTIFACT_PATHS)
            self.assertGreaterEqual(manifest["ubi"]["output"]["partition_spare_pebs"], 6)
            plan = [sys.executable, "-I", str(ROOT / "scripts/flash_cutover.py"),
                    "--own-your-cameras", env["OWN_YOUR_CAMERAS"], "--image-dir", str(out)]
            prepared = subprocess.run(plan, capture_output=True, text=True)
            self.assertEqual(prepared.returncode, 0, prepared.stdout + prepared.stderr)
            self.assertFalse(json.loads(prepared.stdout)["upload_started"])
            self.assertEqual(json.loads(prepared.stdout)["candidate_sha256"], manifest["hashes"]["candidate"])
            # A saved JSON verdict must never authorize different actual bytes.
            candidate = Path(manifest["candidate"]["path"])
            data = bytearray(candidate.read_bytes())
            data[-1] ^= 1
            candidate.write_bytes(data)
            rejected = subprocess.run(plan, capture_output=True, text=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("differs from", rejected.stderr)


if __name__ == "__main__":
    unittest.main()
