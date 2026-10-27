"""Execute the shipped shell and helper with isolated OS/process fixtures, never hardware commands."""
import atexit
import importlib.util
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest

REPO = Path(__file__).resolve().parents[1]
# A read-only directory is how these tests make a write fail; root writes through it anyway.
needs_unprivileged = unittest.skipIf(os.geteuid() == 0, "root ignores the read-only fixture directory")


def assembled_start_app():
    """image/start_app as the image builder assembles it: with the real stock blocks when STOCK_PAK and
    OWN_YOUR_CAMERAS are set, else with stand-ins (--fixture-root runs fixture stand-ins either way)."""
    sys.path.insert(0, str(REPO / "scripts"))
    spec = importlib.util.spec_from_file_location("start_app", REPO / "scripts/start_app.py")
    start_app = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(start_app)
    if os.environ.get("STOCK_PAK") and os.environ.get("OWN_YOUR_CAMERAS"):
        stock = start_app.stock_start_app(Path(os.environ["STOCK_PAK"]), Path(os.environ["OWN_YOUR_CAMERAS"]))
        blocks = start_app.stock_blocks(stock)
    else:
        blocks = (b"echo stock init stand-in\n", b"echo stock vendor stand-in\n")
    out = Path(tempfile.mkdtemp(prefix="cam-start-app-")) / "start_app"
    atexit.register(shutil.rmtree, out.parent, True)
    out.write_bytes(start_app.assemble((REPO / "image/start_app").read_bytes(), *blocks))
    return out


SCRIPT = assembled_start_app()


class Launcher(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="cam-boot-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.processes = []
        self.addCleanup(self.stop)
        for d in ("mnt/para/root-ssh", "mnt/tmp", "usr/bin", "etc/init.d", "sbin",
                  "proc/sys/kernel/random"):
            (self.root / d).mkdir(parents=True)
        self.write("mnt/para/root-ssh/authorized_keys", "fixture-key-unchanged\n")
        self.write("mnt/para/root-ssh/dropbear_ed25519_host_key", "fixture-host-key\n")
        self.write("proc/sys/kernel/random/boot_id", "boot-001\n")
        self.executable("etc/init.d/cam-stock-init", 'echo init >> "$CAM_FIXTURE/events"')
        self.executable("etc/init.d/cam-stock-vendor", 'echo vendor >> "$CAM_FIXTURE/events"')
        self.executable("sbin/reboot", 'echo reboot >> "$CAM_FIXTURE/events"')
        self.executable("etc/init.d/S99_zzRootSSH", 'echo ssh >> "$CAM_FIXTURE/events"')

    def write(self, path, text):
        p = self.root / path
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(text)

    def executable(self, path, body):
        if path == "usr/bin/cam":
            body = ('if [ "$1" = preflight ]; then exec "$CAM_LAUNCH_BIN" --preflight '
                    '"$CAM_FIXTURE"; fi\n') + body
        self.write(path, "#!/bin/sh\n" + body + "\n")
        (self.root / path).chmod(0o755)

    def stop(self):
        for p in self.processes:
            try:
                os.killpg(p.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            p.wait(timeout=5)

    def boot(self):
        env = dict(os.environ, CAM_FIXTURE=str(self.root))
        # This is the rcS ordering: start_app must return before the SSH hook runs.
        before = self.events().count("ssh")
        p = subprocess.Popen(["/bin/sh", "-c",
                              'sh "$1" --fixture-root "$2"; sh "$2/etc/init.d/S99_zzRootSSH"; exec sleep 300',
                              "rcS", str(SCRIPT), str(self.root)], env=env,
                             start_new_session=True, stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL)
        self.processes.append(p)
        self.wait_for(lambda: self.events().count("ssh") > before, timeout=2)
        self.assertIsNone(p.poll())  # Keep the session leader owned until cleanup, avoiding PID reuse.
        self.assertEqual((self.root / "mnt/para/root-ssh/authorized_keys").read_text(),
                         "fixture-key-unchanged\n")
        self.assertEqual((self.root / "mnt/para/root-ssh/dropbear_ed25519_host_key").read_text(),
                         "fixture-host-key\n")

    def events(self):
        p = self.root / "events"
        return p.read_text().splitlines() if p.exists() else []

    def wait_for(self, predicate, timeout=5):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            if predicate():
                return
            time.sleep(0.02)
        self.fail(f"condition timed out; events={self.events()}")

    def test_missing_flag_preserves_vendor_and_ssh(self):
        self.boot()
        self.assertEqual(self.events(), ["init", "vendor", "ssh"])
        self.assertFalse((self.root / "mnt/para/cam.crashes").exists())

    def test_same_boot_reentry_never_starts_a_second_stack(self):
        self.boot()
        self.boot()
        self.assertEqual(self.events().count("vendor"), 1)
        self.assertNotIn("reboot", self.events())

    @needs_unprivileged
    def test_boot_marker_creation_failure_leaves_only_ssh(self):
        self.configured()
        tmp = self.root / "mnt/tmp"
        tmp.chmod(0o500)
        try:
            self.boot()
            self.assert_quiet_ssh()
            self.assertNotIn("init", self.events())
            self.assertNotIn("vendor", self.events())
            self.assertNotIn("custom", self.events())
        finally:
            tmp.chmod(0o700)

    def test_invalid_boot_markers_leave_only_ssh(self):
        self.configured()
        marker = self.root / "mnt/tmp/cam-boot-selected"
        for kind in ("file", "symlink"):
            with self.subTest(kind=kind):
                if kind == "file":
                    marker.write_text("not a selection directory")
                else:
                    marker.symlink_to(self.root / "mnt/para", target_is_directory=True)
                self.boot()
                self.assert_quiet_ssh()
                self.assertNotIn("init", self.events())
                self.assertNotIn("vendor", self.events())
                self.assertNotIn("custom", self.events())
                self.next_boot(2)

    def assert_quiet_ssh(self):
        end = time.monotonic() + 0.2
        self.wait_for(lambda: time.monotonic() >= end)
        self.assertEqual(self.events(), ["ssh"])

    def enable(self):
        self.write("mnt/para/cam.enabled", "")
        helper = Path(os.environ["CAM_LAUNCH_BIN"]).resolve()
        (self.root / "usr/bin/cam-launch").symlink_to(helper)
        self.executable("usr/bin/cam", 'echo custom >> "$CAM_FIXTURE/events"')

    def configured(self):
        self.enable()
        self.write("mnt/para/cam-wifi.conf", 'network={\nssid="fixture"\npsk="fixture-password"\n}\n')
        self.write("mnt/para/cam-rtsp.auth", "fixture:password\n")
        self.write("mnt/para/cam.toml", "[mqtt]\nhost='127.0.0.1'\n")
        for name in ("cam-wifi.conf", "cam-rtsp.auth", "cam.toml"):
            (self.root / "mnt/para" / name).chmod(0o600)
        for name in ("usr/lib/libhdal.so", "dev/mtd7ro", "sbin/insmod", "sbin/rmmod",
                     "sbin/modprobe", "sbin/ifconfig", "sbin/wpa_supplicant", "sbin/udhcpc",
                     "lib/modules/usb_ko/usb-common.ko", "lib/modules/usb_ko/usbcore.ko",
                     "lib/modules/usb_ko/ehci-hcd.ko", "lib/modules/wifi/firmware_class.ko",
                     "lib/modules/wifi/rfkill.ko", "lib/modules/wifi/wifi_8812ko/cfg80211.ko",
                     "lib/modules/wifi/wifi_8812ko/wlan.ko"):
            self.write(name, "fixture")
            if name.startswith("sbin/"):
                (self.root / name).chmod(0o755)
        (self.root / "dev/mtd7ro").unlink()
        (self.root / "dev/mtd7ro").symlink_to("/dev/null")
        self.executable("usr/bin/cam", '''
echo "attempt:$(cat "$CAM_FIXTURE/mnt/para/cam.crashes")" >> "$CAM_FIXTURE/events"
echo "args:$*" >> "$CAM_FIXTURE/events"
echo $$ > "$CAM_FIXTURE/daemon.pid"
echo $PPID > "$CAM_FIXTURE/supervisor.pid"
echo custom >> "$CAM_FIXTURE/events"
exec sleep 300
''')

    def test_attempt_is_durable_before_child_and_ssh_is_prompt(self):
        self.configured()
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        self.assertIn("ssh", self.events())
        self.assertIn("attempt:1", self.events())
        self.assertIn("args:serve --attempt boot-001-1", self.events())
        self.assertNotIn("vendor", self.events())

    def start_helper(self, phase):
        p = subprocess.Popen([os.environ["CAM_LAUNCH_BIN"], phase, str(self.root)],
                             env=dict(os.environ, CAM_FIXTURE=str(self.root)),
                             start_new_session=True, stdout=subprocess.DEVNULL,
                             stderr=subprocess.DEVNULL)
        self.processes.append(p)
        return p

    def helper(self, phase):
        return self.start_helper(phase).wait(timeout=12)

    def test_reset_escalates_even_when_normal_reboot_exits_successfully(self):
        self.executable("sbin/reboot", '''
if [ "$1" = -f ]; then echo reboot-f; else echo reboot; fi >> "$CAM_FIXTURE/events"
''')
        start = time.monotonic()
        self.assertEqual(self.helper("--reset"), 0)
        self.assertEqual(self.events(), ["reboot", "reboot-f"])
        self.assertGreaterEqual(time.monotonic() - start, 5)

    def test_reserve_counts_without_starting_a_graph_or_guard(self):
        self.configured()
        self.assertEqual(self.helper("--reserve"), 0)
        self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), "1\n")
        self.assertEqual(self.events(), [])
        self.assertEqual(list((self.root / "mnt/tmp").iterdir()), [])
        self.write("mnt/para/cam.crashes", "3\n")
        self.assertEqual(self.helper("--reserve"), 10)
        self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), "3\n")
        self.assertEqual(self.events(), [])

    def test_reset_escalates_after_failed_or_hung_normal_command(self):
        for action in ("exit 1", "exec sleep 300"):
            with self.subTest(action=action):
                self.executable("sbin/reboot", f'''
if [ "$1" = -f ]; then echo reboot-f >> "$CAM_FIXTURE/events"; exit 0; fi
echo reboot >> "$CAM_FIXTURE/events"
{action}
''')
                start = time.monotonic()
                self.assertEqual(self.helper("--reset"), 0)
                self.assertEqual(self.events(), ["reboot", "reboot-f"])
                self.assertGreaterEqual(time.monotonic() - start, 5)
                self.next_boot(2)

    def test_reset_bounds_a_hung_force_command(self):
        self.executable("sbin/reboot", '''
if [ "$1" = -f ]; then echo reboot-f; else echo reboot; fi >> "$CAM_FIXTURE/events"
exec sleep 300
''')
        start = time.monotonic()
        self.assertEqual(self.helper("--reset"), 0)
        self.assertEqual(self.events(), ["reboot", "reboot-f"])
        self.assertLess(time.monotonic() - start, 11)

    def test_missing_reboot_still_attempts_force_after_deadline(self):
        (self.root / "sbin/reboot").unlink()
        p = self.start_helper("--reset")
        # Keep the executable absent through the first spawn; restoring it then
        # exposes the forced invocation at the real executable boundary.
        start = time.monotonic()
        self.wait_for(lambda: time.monotonic() - start > 1)
        self.assertIsNone(p.poll())
        self.executable("sbin/reboot", 'echo "reboot$1" >> "$CAM_FIXTURE/events"')
        self.assertEqual(p.wait(timeout=6), 0)
        self.assertEqual(self.events(), ["reboot-f"])
        (self.root / "sbin/reboot").unlink()
        self.assertEqual(self.helper("--reset"), 0)

    def test_preflight_child_statuses_are_never_forwarded(self):
        self.configured()
        for status in (1, 10, 126, 127):
            with self.subTest(status=status):
                self.write("usr/bin/cam", f"#!/bin/sh\nexit {status}\n")
                self.assertEqual(self.helper("--reserve"), 10)
                self.assertFalse((self.root / "mnt/para/cam.crashes").exists())
                self.assertEqual(self.events(), [])

    def test_missing_configuration_chooses_vendor_before_any_attempt(self):
        self.enable()
        self.boot()
        self.wait_for(lambda: "vendor" in self.events())
        self.assertNotIn("custom", self.events())
        self.assertFalse((self.root / "mnt/para/cam.crashes").exists())

    def next_boot(self, number):
        self.stop()
        self.processes.clear()
        shutil.rmtree(self.root / "mnt/tmp")
        (self.root / "mnt/tmp").mkdir()
        self.write("proc/sys/kernel/random/boot_id", f"boot-{number:03d}\n")
        self.write("events", "")

    def test_three_immediate_failures_choose_vendor_only_on_fourth_boot(self):
        self.configured()
        self.executable("usr/bin/cam", 'echo custom >> "$CAM_FIXTURE/events"; exit 1')
        for n in range(1, 4):
            if n > 1:
                self.next_boot(n)
            self.boot()
            self.wait_for(lambda: "reboot" in self.events())
            self.assertNotIn("vendor", self.events())
            self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), f"{n}\n")
        self.next_boot(4)
        self.boot()
        self.wait_for(lambda: "vendor" in self.events())
        self.assertNotIn("custom", self.events())
        self.assertNotIn("reboot", self.events())

    def test_supervisor_abrupt_exit_requests_clean_reboot_without_vendor(self):
        self.configured()
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        pid = int((self.root / "supervisor.pid").read_text())
        os.kill(pid, signal.SIGKILL)
        self.wait_for(lambda: "reboot" in self.events())
        self.assertNotIn("vendor", self.events())
        self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), "1\n")

    def test_hung_preflight_is_bounded_and_never_consumes_an_attempt(self):
        self.configured()
        self.write("usr/bin/cam", "#!/bin/sh\nexec sleep 300\n")
        self.boot()
        self.wait_for(lambda: "vendor" in self.events(), timeout=4)
        self.assertFalse((self.root / "mnt/para/cam.crashes").exists())
        self.assertNotIn("reboot", self.events())

    def healthy_child(self):
        # A process-boundary fixture for the documented runtime health protocol, no device writes.
        self.write("fake_runtime.py", '''
import json, os, pathlib, sys, time
r = pathlib.Path(os.environ["CAM_FIXTURE"])
token = sys.argv[3]
(r / "daemon.pid").write_text(str(os.getpid()))
(r / "supervisor.pid").write_text(str(os.getppid()))
with (r / "events").open("a") as f: f.write("custom\\n")
start = time.monotonic()
while True:
    if (r / "freeze").exists():
        time.sleep(.02)
        continue
    ms = 200000 + int((time.monotonic() - start) * 1000)
    status = {"elapsed_ms": ms, "healthy_for_ms": ms, "qualifying": True}
    record = {"version": 1, "attempt": token, "pid": os.getpid(), "status": status}
    mode = (r / "health-mode").read_text() if (r / "health-mode").exists() else ""
    if mode == "old-attempt": record["attempt"] = "previous-boot"
    if mode == "wrong-pid": record["pid"] += 1
    if mode == "short-window": status["healthy_for_ms"] = 179999
    p = r / "mnt/tmp" / ("cam-health-" + token + ".json")
    tmp = p.with_suffix(".tmp")
    tmp.write_text(json.dumps(record))
    tmp.replace(p)
    time.sleep(.05)
''')
        self.executable("usr/bin/cam", f'exec "{sys.executable}" "$CAM_FIXTURE/fake_runtime.py" "$@"')

    def test_health_clears_old_history_but_keeps_an_active_attempt_for_later_reset(self):
        self.configured()
        self.write("mnt/para/cam.crashes", "2\n")
        self.healthy_child()
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        counter = self.root / "mnt/para/cam.crashes"
        self.assertEqual(counter.read_text(), "3\n")
        self.wait_for(lambda: counter.read_text() == "1\n")
        self.write("freeze", "")
        self.wait_for(lambda: "reboot" in self.events(), timeout=6)
        self.assertEqual(counter.read_text(), "1\n")
        self.assertNotIn("vendor", self.events())
        self.next_boot(2)
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        self.assertEqual(counter.read_text(), "2\n")

    def test_hung_child_before_watchdog_takeover_reboots_within_startup_budget(self):
        self.configured()
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        self.wait_for(lambda: "reboot" in self.events(), timeout=18)
        self.assertNotIn("vendor", self.events())
        self.assertFalse((self.root / "mnt/tmp/wdt_flag").exists())

    def assert_vendor(self):
        self.boot()
        self.wait_for(lambda: "vendor" in self.events())
        self.assertNotIn("custom", self.events())
        self.assertNotIn("reboot", self.events())

    def test_missing_or_nonexecutable_binary_and_missing_inputs(self):
        self.configured()
        for name in ("usr/bin/cam", "mnt/para/cam-wifi.conf", "mnt/para/cam-rtsp.auth",
                     "mnt/para/cam.toml", "usr/lib/libhdal.so", "sbin/reboot"):
            with self.subTest(name=name):
                p = self.root / name
                saved = p.read_bytes()
                mode = p.stat().st_mode
                p.unlink()
                self.assert_vendor()
                self.assertFalse((self.root / "mnt/para/cam.crashes").exists())
                p.write_bytes(saved)
                p.chmod(mode)
                self.next_boot(2)
        (self.root / "usr/bin/cam").chmod(0o600)
        self.assert_vendor()

    def test_missing_helper_retains_original_vendor_path(self):
        self.write("mnt/para/cam.enabled", "")
        self.assert_vendor()

    def replace_helper(self, body):
        (self.root / "usr/bin/cam-launch").unlink()
        self.executable("usr/bin/cam-launch", body)

    def fault_phase(self, phase, body):
        self.replace_helper(f'''if [ "$1" = "{phase}" ]; then
{body}
fi
exec "$CAM_LAUNCH_BIN" "$@"''')

    def test_killed_reservation_selects_vendor_on_every_clean_boot(self):
        self.configured()
        self.fault_phase("--reserve", 'echo reserve >> "$CAM_FIXTURE/events"; kill -KILL "$$"')
        for n in range(1, 5):
            if n > 1:
                self.next_boot(n)
            self.assert_vendor()
            self.assertIn("reserve", self.events())
            self.assertFalse((self.root / "mnt/para/cam.crashes").exists())

    def test_early_serve_failure_is_counted_and_never_selects_vendor(self):
        self.configured()
        self.fault_phase("--serve", 'echo serve >> "$CAM_FIXTURE/events"; kill -KILL "$$"')
        for n in range(1, 4):
            if n > 1:
                self.next_boot(n)
            self.boot()
            self.wait_for(lambda: "reboot" in self.events(), timeout=6)
            self.assertIn("serve", self.events())
            self.assertNotIn("vendor", self.events())
            self.assertNotIn("custom", self.events())
            self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), f"{n}\n")
        self.next_boot(4)
        self.assert_vendor()
        self.assertNotIn("serve", self.events())

    def test_early_abort_and_unexpected_serve_exit_request_reboot_not_vendor(self):
        self.configured()
        for body in ('ulimit -c 0; kill -ABRT "$$"', "exit 0", "exit 1", "exit 2",
                     "exit 10", "exit 126", "exit 127"):
            with self.subTest(body=body):
                self.fault_phase("--serve", body)
                self.boot()
                self.wait_for(lambda: "reboot" in self.events())
                self.assertNotIn("vendor", self.events())
                self.assertNotIn("custom", self.events())
                self.next_boot(2)
                (self.root / "mnt/para/cam.crashes").unlink()

    def test_stopped_reservation_times_out_to_vendor_without_reboot(self):
        self.configured()
        self.fault_phase("--reserve", 'echo reserve >> "$CAM_FIXTURE/events"; kill -STOP "$$"')
        start = time.monotonic()
        self.boot()
        self.wait_for(lambda: "vendor" in self.events(), timeout=11)
        self.assertLess(time.monotonic() - start, 11)
        self.assertNotIn("reboot", self.events())
        self.assertNotIn("custom", self.events())
        self.assertFalse((self.root / "mnt/para/cam.crashes").exists())

    def test_reservation_refusal_loader_failure_and_abort_select_vendor(self):
        self.configured()
        for body in ("exit 10", "exit 126", "exit 127", "exit 1", "exit 2",
                     'ulimit -c 0; kill -ABRT "$$"'):
            with self.subTest(body=body):
                self.fault_phase("--reserve", body)
                self.assert_vendor()
                self.assertFalse((self.root / "mnt/para/cam.crashes").exists())
                self.next_boot(2)

    def test_interrupted_reservation_refuses_until_explicit_counter_repair(self):
        self.configured()
        self.fault_phase("--reserve", '''
echo 1 > "$CAM_FIXTURE/mnt/para/cam.crashes.tmp"
echo persist >> "$CAM_FIXTURE/events"
kill -STOP "$$"
''')
        self.boot()
        self.wait_for(lambda: "vendor" in self.events(), timeout=11)
        self.assertNotIn("reboot", self.events())
        self.assertNotIn("custom", self.events())
        self.next_boot(2)
        self.replace_helper('exec "$CAM_LAUNCH_BIN" "$@"')
        self.assert_vendor()
        self.assertTrue((self.root / "mnt/para/cam.crashes.tmp").exists())
        # The documented SSH repair, applied only to this isolated fixture.
        for name in ("cam.crashes", "cam.crashes.tmp"):
            (self.root / "mnt/para" / name).unlink(missing_ok=True)
        self.next_boot(3)
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        self.assertIn("attempt:1", self.events())

    def test_stopped_serve_before_guard_is_reset_by_shell_sentinel(self):
        self.configured()
        self.fault_phase("--serve", 'echo serve >> "$CAM_FIXTURE/events"; kill -STOP "$$"')
        start = time.monotonic()
        self.boot()
        self.wait_for(lambda: "reboot" in self.events(), timeout=6)
        self.assertLess(time.monotonic() - start, 6)
        self.assertNotIn("vendor", self.events())
        self.assertNotIn("custom", self.events())
        self.assertFalse((self.root / "mnt/tmp/cam-guard-up").exists())
        self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), "1\n")

    def test_reset_exec_failure_uses_shell_fallback_after_reservation(self):
        self.configured()
        self.replace_helper('''
case "$1" in
    --serve) kill -KILL "$$" ;;
    --reset) exec "$CAM_FIXTURE/missing-reset-loader" ;;
esac
exec "$CAM_LAUNCH_BIN" "$@"
''')
        self.boot()
        self.wait_for(lambda: "reboot" in self.events())
        self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), "1\n")
        self.assertNotIn("vendor", self.events())
        self.assertNotIn("custom", self.events())

    def test_unexpected_reset_status_does_not_use_exec_failure_fallback(self):
        self.configured()
        self.replace_helper('''
case "$1" in
    --serve) kill -KILL "$$" ;;
    --reset) echo reset-failed >> "$CAM_FIXTURE/events"; exit 1 ;;
esac
exec "$CAM_LAUNCH_BIN" "$@"
''')
        self.boot()
        self.wait_for(lambda: self.events().count("reset-failed") == 2, timeout=6)
        self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), "1\n")
        self.assertNotIn("reboot", self.events())
        self.assertNotIn("vendor", self.events())

    def test_guard_spawn_failure_requests_reset_without_vendor(self):
        self.configured()
        # OS fault: the real helper can load, but its socket pair leaves no FD
        # for spawning a guard. The orchestrator retains its normal FD budget.
        self.fault_phase("--serve", 'ulimit -n 5; exec "$CAM_LAUNCH_BIN" "$@"')
        self.boot()
        self.wait_for(lambda: "reboot" in self.events(), timeout=6)
        self.assertFalse((self.root / "mnt/tmp/cam-guard-up").exists())
        self.assertNotIn("custom", self.events())
        self.assertNotIn("vendor", self.events())
        self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), "1\n")

    def test_serve_cannot_fall_back_on_invalid_active_reservation(self):
        self.configured()
        self.fault_phase("--serve", '''
echo corrupt > "$CAM_FIXTURE/mnt/para/cam.crashes"
exec "$CAM_LAUNCH_BIN" "$@"
''')
        self.boot()
        self.wait_for(lambda: "reboot" in self.events())
        self.assertNotIn("vendor", self.events())
        self.assertNotIn("custom", self.events())

    def test_killed_stock_init_shell_consumes_no_attempt(self):
        self.configured()
        self.executable("etc/init.d/cam-stock-init", '''
echo init >> "$CAM_FIXTURE/events"
kill -KILL "$PPID"
''')
        for n in range(1, 4):
            if n > 1:
                self.next_boot(n)
            self.boot()
            self.assertEqual(self.events(), ["init", "ssh"])
            self.assertFalse((self.root / "mnt/para/cam.crashes").exists())
        self.next_boot(4)
        self.executable("etc/init.d/cam-stock-init", 'echo init >> "$CAM_FIXTURE/events"')
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        self.assertIn("attempt:1", self.events())
        self.assertIn("args:serve --attempt boot-004-1", self.events())
        self.assertNotIn("vendor", self.events())

    def test_helper_with_missing_loader_retains_vendor_and_ssh(self):
        self.configured()
        helper = self.root / "usr/bin/cam-launch"
        helper.unlink()  # Never overwrite the symlink's real build artifact.
        helper.write_text(f"#!{self.root}/missing-loader\n")
        helper.chmod(0o755)
        self.assert_vendor()
        self.assertIn("ssh", self.events())
        self.assertFalse((self.root / "mnt/para/cam.crashes").exists())

    def test_invalid_helper_executable_retains_vendor_and_ssh(self):
        self.configured()
        helper = self.root / "usr/bin/cam-launch"
        helper.unlink()
        helper.write_bytes(b"\x7fELF" + b"\x00" * 60)
        helper.chmod(0o755)
        self.assert_vendor()
        self.assertIn("ssh", self.events())
        self.assertFalse((self.root / "mnt/para/cam.crashes").exists())

    def test_missing_or_invalid_boot_identity_refuses_custom_without_reboot_loop(self):
        self.configured()
        for value in (None, "", "../invalid", "x" * 65):
            with self.subTest(value=value):
                p = self.root / "proc/sys/kernel/random/boot_id"
                p.unlink() if value is None else p.write_text(value)
                self.assert_vendor()
                self.assertFalse((self.root / "mnt/para/cam.crashes").exists())
                self.next_boot(2)

    def test_invalid_credentials_and_configuration_never_bootstrap_from_vendor_ram(self):
        self.configured()
        for name, invalid in (
            ("mnt/para/cam-wifi.conf", 'network={\nssid="x"\npsk="short"\n}\n'),
            ("mnt/para/cam-rtsp.auth", "user:\n"),
            ("mnt/para/cam.toml", "[mqtt]\nhost="),
            ("mnt/para/cam.toml", ""),
        ):
            with self.subTest(name=name, invalid=invalid):
                p = self.root / name
                valid = p.read_text()
                self.write("mnt/tmp/atheros_config.conf", (self.root / "mnt/para/cam-wifi.conf").read_text())
                p.write_text(invalid)
                self.assert_vendor()
                self.assertFalse((self.root / "mnt/para/cam.crashes").exists())
                p.write_text(valid)
                self.next_boot(2)

    def test_corrupt_and_interrupted_counter_fail_closed(self):
        self.configured()
        for value in ("", "-1\n", "999\n", "1", "1\n2\n", "garbage", "0" * 4096):
            with self.subTest(value=value[:16]):
                self.write("mnt/para/cam.crashes", value)
                self.assert_vendor()
                self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), value)
                self.next_boot(2)
        (self.root / "mnt/para/cam.crashes").unlink()
        self.write("mnt/para/cam.crashes.tmp", "1")
        self.assert_vendor()
        self.assertEqual((self.root / "mnt/para/cam.crashes.tmp").read_text(), "1")

    @needs_unprivileged
    def test_counter_persistence_failure_never_starts_custom(self):
        self.configured()
        para = self.root / "mnt/para"
        para.chmod(0o500)
        try:
            self.assert_vendor()
            self.assertFalse((para / "cam.crashes").exists())
        finally:
            para.chmod(0o700)

    def test_interrupted_boot_counts_and_flag_removal_waits_for_clean_boot(self):
        self.configured()
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        self.next_boot(2)  # Abrupt whole-boot loss: no supervisor/daemon exit accounting can run.
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        self.assertIn("attempt:2", self.events())
        (self.root / "mnt/para/cam.enabled").unlink()
        self.boot()
        self.assertNotIn("vendor", self.events())
        self.assertEqual(self.events().count("custom"), 1)
        self.next_boot(3)
        self.assert_vendor()
        self.assertEqual((self.root / "mnt/para/cam.crashes").read_text(), "2\n")

    def test_stopped_supervisor_and_killed_child_request_reboot(self):
        self.configured()
        for name, sig in (("supervisor.pid", signal.SIGSTOP), ("daemon.pid", signal.SIGKILL)):
            with self.subTest(name=name):
                self.boot()
                self.wait_for(lambda: "custom" in self.events())
                os.kill(int((self.root / name).read_text()), sig)
                self.wait_for(lambda: "reboot" in self.events(), timeout=7)
                self.assertNotIn("vendor", self.events())
                self.next_boot(2)

    def test_healthy_run_abrupt_power_loss_is_counted_without_tick_writes(self):
        self.configured()
        self.write("mnt/para/cam.crashes", "2\n")
        self.healthy_child()
        self.boot()
        counter = self.root / "mnt/para/cam.crashes"
        self.wait_for(lambda: counter.read_text() == "1\n")
        stamp = counter.stat().st_mtime_ns
        # Wait for observed runtime progress, then prove no extra durable counter update.
        record = next((self.root / "mnt/tmp").glob("cam-health-*.json"))
        start = time.monotonic()
        self.wait_for(lambda: time.monotonic() - start > 2 and record.exists())
        self.assertEqual(counter.stat().st_mtime_ns, stamp)
        self.next_boot(2)
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        self.assertEqual(counter.read_text(), "2\n")

    def test_unqualified_or_foreign_health_cannot_clear_history(self):
        self.configured()
        self.healthy_child()
        for mode in ("old-attempt", "wrong-pid", "short-window"):
            with self.subTest(mode=mode):
                self.write("mnt/para/cam.crashes", "2\n")
                self.write("health-mode", mode)
                self.boot()
                self.wait_for(lambda: "custom" in self.events())
                counter = self.root / "mnt/para/cam.crashes"
                start = time.monotonic()
                self.wait_for(lambda: time.monotonic() - start > 2.2)
                self.assertEqual(counter.read_text(), "3\n")
                self.assertNotIn("vendor", self.events())
                self.next_boot(2)

    @needs_unprivileged
    def test_failed_health_reset_keeps_history_and_requests_reboot(self):
        self.configured()
        self.write("mnt/para/cam.crashes", "2\n")
        self.healthy_child()
        self.boot()
        self.wait_for(lambda: "custom" in self.events())
        para = self.root / "mnt/para"
        para.chmod(0o500)
        try:
            self.wait_for(lambda: "reboot" in self.events())
            self.assertEqual((para / "cam.crashes").read_text(), "3\n")
            self.assertNotIn("vendor", self.events())
        finally:
            para.chmod(0o700)


if __name__ == "__main__":
    unittest.main()
