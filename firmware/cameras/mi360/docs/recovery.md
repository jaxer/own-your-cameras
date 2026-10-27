# Mi 360 recovery

## Disable the daemon

```sh
ssh root@CAMERA_HOST 'rm -f /mnt/data/mi360/enabled; sync; reboot'
```

Networking and SSH stay available. Three failed starts also select this mode.
Inspect the fault before resetting `/mnt/data/mi360/attempts` to zero and enabling again.

## Recovery AP

If Wi-Fi cannot connect, the camera starts its recovery AP. SSH still requires your key.
Use the fingerprint and radio address [saved during installation](install.md#preserve-the-working-access-path).

From the repository root on macOS, with a working wired default route:

```sh
python3 firmware/cameras/mi360/scripts/recover-network.py inspect \
  --host CAMERA_SSH_ALIAS --host-key-sha256 RECORDED_ED25519_FINGERPRINT --mac RECORDED_RADIO_MAC
```

Replace `inspect` with `recover` to reconnect. For AP access, add
`--hotspot --restore-ssid YOUR_SAVED_MAC_NETWORK`. If the key is stored under another name in
known_hosts, add `--host-key-alias TRUSTED_KNOWN_HOSTS_NAME`.

The helper verifies the saved SSH key and radio, reboots into the installed Wi-Fi configuration,
and restores the Mac's saved Wi-Fi association. Prepare working
[Wi-Fi configuration](configuration.md#wi-fi-configuration) before using it.

## Flash recovery

Keep the original dump and an external programmer available. Slot activation writes one boot
environment; a power failure can lose boot selection. There is no automatic slot rollback.
The [full-chip recovery image](install.md#full-chip-programmer-output) restores original DATA,
including its settings and host keys.
