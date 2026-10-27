"""device-session against a fake `ssh`: refusal, the order of the steps, the reboot after any outcome and the
bounded wait for the vendor stack."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("device-session")


class DeviceSessionTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.dir = Path(temp.name)
        self.calls = self.dir / "ssh-calls"
        self.stdin = self.dir / "ssh-stdin"
        bin_dir = self.dir / "bin"
        bin_dir.mkdir()
        ssh = bin_dir / "ssh"
        # The last argument is the remote command; `pidof cam` succeeds when a cam runs, `pidof device ...`
        # when the vendor stack is back.
        ssh.write_text(f"""#!/bin/sh
for a; do last=$a; done
echo "$last" >> {self.calls}
case "$last" in
  'pidof cam') [ -n "${{FAKE_CAM_RUNNING:-}}" ] ;;
  'pidof device rtsp router') [ -z "${{FAKE_VENDOR_GONE:-}}" ] ;;
  cat*) cat >> {self.stdin} ;;
esac
""")
        ssh.chmod(0o755)
        self.binary = self.dir / "cam"
        self.binary.write_bytes(b"BINARY")
        self.env = {**os.environ, "PATH": f"{bin_dir}:{os.environ['PATH']}", "CAMERA": "mycam"}

    def run_session(self, *command, args=(), **env):
        return subprocess.run(
            [str(SCRIPT), *args, str(self.binary), "--", *command],
            env={**self.env, **env}, capture_output=True, text=True)

    def remote(self):
        return self.calls.read_text().splitlines()

    def test_runs_the_command_between_taking_the_camera_and_the_reboot(self):
        marker = self.dir / "ran"
        result = self.run_session("sh", "-c", f'echo "$CAMERA" > {marker}; echo ran >> {self.calls}')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(marker.read_text(), "mycam\n")
        calls = self.remote()
        self.assertEqual(calls[0], "pidof cam")
        self.assertIn("mv /mnt/tmp/cam.new /mnt/tmp/cam", calls[1])
        self.assertTrue(calls[2].startswith("killall router device "), calls[2])
        self.assertIn("./cam wdt", calls[2])
        self.assertEqual(calls[3:], ["ran", "reboot", "pidof device rtsp router"])
        self.assertEqual(self.stdin.read_bytes(), b"BINARY")

    def test_copies_the_auth_file_when_given(self):
        auth = self.dir / "auth"
        auth.write_text("user:secret\n")
        result = self.run_session("true", args=("--auth", str(auth)))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("chmod 600 /mnt/tmp/cam-rtsp.auth", self.remote()[2])
        self.assertEqual(self.stdin.read_bytes(), b"BINARYuser:secret\n")

    def test_refuses_beside_another_cam_and_touches_nothing(self):
        result = self.run_session("true", FAKE_CAM_RUNNING="1")
        self.assertEqual(result.returncode, 1)
        self.assertIn("another cam process runs on mycam", result.stderr)
        self.assertEqual(self.remote(), ["pidof cam"])

    def test_a_failing_command_still_reboots_and_its_status_is_returned(self):
        result = self.run_session("sh", "-c", "exit 7")
        self.assertEqual(result.returncode, 7)
        self.assertEqual(self.remote()[-2:], ["reboot", "pidof device rtsp router"])

    def test_the_wait_for_the_vendor_stack_is_bounded(self):
        result = self.run_session("true", FAKE_VENDOR_GONE="1", WAIT="0")
        self.assertEqual(result.returncode, 1)
        self.assertIn("not back on mycam after 0s", result.stderr)

    def test_needs_camera_and_a_command(self):
        env = {k: v for k, v in self.env.items() if k != "CAMERA"}
        result = subprocess.run([str(SCRIPT), str(self.binary), "--", "true"], env=env,
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("CAMERA=<ssh host of your camera> is required", result.stderr)
        self.assertEqual(self.run_session().returncode, 2)
        self.assertFalse(self.calls.exists())


if __name__ == "__main__":
    unittest.main()
