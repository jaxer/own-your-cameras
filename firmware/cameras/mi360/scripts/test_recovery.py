"""Isolated recovery identity and AP command checks; never changes host networking."""
import argparse
import base64
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location('mi360_recovery', HERE / 'recover-network.py')
recovery = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = recovery
SPEC.loader.exec_module(recovery)


def radio(seed):
    # Synthetic locally-administered identities, used only in isolated host tests.
    return ':'.join(f'{value:02x}' for value in [2, 0, 0, seed, seed + 1, seed + 2])


def identity(seed):
    raw = b'isolated public-key fixture' + bytes([seed])
    fingerprint = 'SHA256:' + base64.b64encode(hashlib.sha256(raw).digest()).decode().rstrip('=')
    args = argparse.Namespace(host=f'camera_{seed}', host_key_alias=None,
                              host_key_sha256=fingerprint, mac=radio(seed), ap_ssid=None)
    return recovery.Identity.from_args(args), raw


class RecoveryIdentity(unittest.TestCase):
    def test_distinct_owners_keep_distinct_host_keys_and_radio_identity(self):
        first, first_key = identity(10)
        second, second_key = identity(20)
        self.assertNotEqual(first.ap_ssid, second.ap_ssid)
        for selected, raw, wrong in [(first, first_key, second), (second, second_key, first)]:
            record = selected.key_alias + ' ssh-ed25519 ' + base64.b64encode(raw).decode()
            self.assertTrue(recovery.fingerprint_ok(record, selected.fingerprint))
            self.assertFalse(recovery.fingerprint_ok(record, wrong.fingerprint))
            with patch.object(recovery, 'checked', return_value=record):
                camera = recovery.Camera('test0', True, selected)
            good = subprocess.CompletedProcess([], 0, selected.mac, '')
            bad = subprocess.CompletedProcess([], 0, wrong.mac, '')
            with patch.object(camera, 'call', return_value=good):
                self.assertTrue(camera.hotspot_identity())
            with patch.object(camera, 'call', return_value=bad):
                self.assertFalse(camera.hotspot_identity())
            self.assertIn('-oStrictHostKeyChecking=yes', camera.args)
            self.assertIn('-oHostKeyAlias=' + selected.key_alias, camera.args)

    def test_wrong_key_is_refused_before_any_ssh_request(self):
        owner, _ = identity(10)
        other, raw = identity(20)
        record = other.key_alias + ' ssh-ed25519 ' + base64.b64encode(raw).decode()
        with patch.object(recovery, 'checked', return_value=record):
            with self.assertRaises(recovery.RecoveryError):
                recovery.Camera('test0', True, owner)

    def test_invalid_identity_is_rejected_before_network_inspection(self):
        owner, _ = identity(10)
        args = argparse.Namespace(host=owner.host, host_key_alias=None,
                                  host_key_sha256=owner.fingerprint, mac=owner.mac, ap_ssid=None)
        for name, value in [('host', '-oProxyCommand=bad'), ('host', 'bad host'),
                            ('host_key_sha256', 'unknown'), ('mac', 'not-a-radio'),
                            ('ap_ssid', 'bad\nssid')]:
            changed = argparse.Namespace(**{**vars(args), name: value})
            with self.subTest(name=name, value=value), self.assertRaises(recovery.RecoveryError):
                recovery.Identity.from_args(changed)

    def test_both_rooted_and_native_boots_are_recognized(self):
        self.assertTrue(recovery.prepared_boot({'boot_mode': '/mnt/data/mi360-network/bin'}))
        owner, raw = identity(10)
        record = owner.key_alias + ' ssh-ed25519 ' + base64.b64encode(raw).decode()
        with patch.object(recovery, 'checked', return_value=record):
            camera = recovery.Camera('test0', True, owner)
        result = subprocess.CompletedProcess([], 0, 'rescue=ready\nnative_network=ready\n', '')
        with patch.object(camera, 'call', return_value=result):
            self.assertTrue(recovery.prepared_boot(camera.inspect()))
        self.assertFalse(recovery.prepared_boot({}))
        self.assertFalse(recovery.prepared_boot({'boot_mode': '/mnt/data/other'}))

    def test_ap_fallback_uses_actual_radio_without_data_configuration(self):
        for seed in [10, 20]:
            owner, _ = identity(seed)
            with tempfile.TemporaryDirectory(prefix='mi360-ap-test-') as temporary:
                root = Path(temporary)
                (root / 'run').mkdir()
                (root / 'address').write_text(owner.mac + '\n')
                (root / 'hostapd.conf').write_text('interface=wlan0\n')
                source = (HERE / 'network-ap.sh').read_text()
                source = source.replace('PATH=/mnt/usr/bin:/bin:/sbin:/usr/bin:/usr/sbin', '')
                source = source.replace('/sys/class/net/wlan0/address', str(root / 'address'))
                source = source.replace('/tmp/mi360-network', str(root / 'run'))
                source = source.replace('/etc/hostapd.conf', str(root / 'hostapd.conf'))
                source = source.replace('/var/run/hostapd', str(root / 'hostapd'))
                source = source.replace('/bin/busybox', 'test_busybox')
                script = root / 'ap.sh'
                script.write_text('test_busybox() { return 1; }\nifconfig() { :; }\nsleep() { :; }\nhostapd() { :; }\nudhcpd() { :; }\n' + source)
                result = subprocess.run(['/bin/sh', str(script)], capture_output=True, text=True, timeout=3)
                self.assertEqual(result.returncode, 0, result.stderr)
                config = (root / 'run/hostapd.conf').read_text()
                self.assertIn('ssid=' + owner.ap_ssid, config)
                self.assertFalse((root / 'mi360-network').exists())


if __name__ == '__main__':
    unittest.main()
