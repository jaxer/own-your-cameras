# Build and use a door-station startup hook

[DS-KD8003-IME1 overview](README.md)

**Supported:** DS-KD8003-IME1, original non-B hardware, V2.2.62 build 230204.
The builder changes only encrypted `/start.sh`. No application or credentials are
bundled. The generic public candidate has passed offline checks; it has not been
flashed or boot-tested. [Format and evidence](firmware-format.md).

## 1. Prepare

- Install Python 3.14 (the tested version).
- You must already have the unmodified V2.2.62 build 230204 DAV. No verified
  download link is supplied here. Other builds are rejected.
- Open a terminal at this repository's root.

Required stock SHA-256:

```text
f5b080741e4b1360c13d0f46abd6af1b6df7e07cbf62c6a7c6435203bb6d107d
```

## 2. Build

Replace `STOCK_DAV` with your local firmware path:

```sh
python3.14 -m venv .venv
.venv/bin/python -m pip install -r tools/door-station-firmware/requirements.txt
.venv/bin/python tools/door-station-firmware/build.py STOCK_DAV \
  --output build/door-station-hook
```

Dependency installation needs network access. The builder itself is offline and
never contacts a camera. Use a new output directory for every attempt, including
retries after failure. Python `-O` is refused because it disables validation.

## 3. Inspect

Success prints hashes, `Changed files: ['/start.sh']`, and flash headroom.
For the unchanged public hook, headroom is **1,028,096 bytes**.

| Output in `build/door-station-hook/` | Check |
|---|---|
| `stock-roundtrip.dav` | Exactly matches the input DAV |
| `door-station-2.2.62-hook.dav` | Candidate; record its printed SHA-256 |
| `manifest.json` | `only_changed_file` is `["/start.sh"]`; roundtrip is `true` |
| `start-stock.sh`, `start-candidate.sh` | Review the appended hook |
| `stock.cramfs`, `candidate.cramfs` | For independent filesystem checks |

On Linux with `fsck.cramfs` installed, also run:

```sh
fsck.cramfs -v build/door-station-hook/stock.cramfs
fsck.cramfs -v build/door-station-hook/candidate.cramfs
```

Both must pass. Keep all generated files local; `build/` is ignored.

## 4. Install — separate device operation

**Flashing changes the device and may make it unbootable.** An upload is not a dry
run. Before proceeding, retain stock firmware and a device configuration backup,
and establish a recovery plan. Dual-bank, serial and power-loss recovery have
not been verified. Offline checks do not prove safe flashing.

The original image was installed through this ISAPI endpoint. Replace
`CAMERA_HOST` with your station's address; `curl` prompts for its admin password:

```sh
curl --fail --show-error --digest --user admin --request PUT \
  --header 'Content-Type: application/octet-stream' \
  --data-binary @build/door-station-hook/door-station-2.2.62-hook.dav \
  'http://CAMERA_HOST/ISAPI/System/updateFirmware'
```

If `curl` reports an error, stop and inspect the device; do not retry blindly.
An HTTP success alone does not prove installation. Allow the update/reboot to
finish, then check the marker below. Do not interrupt power. Reflashing the retained
stock DAV through the same endpoint was the rollback plan, if the updater remains
reachable; it is not a recovery guarantee.

## 5. Check the hook

Enable maintenance SSH and use the [non-PTY root-shell method](README.md#get-a-root-shell).
After installation and reboot:

```sh
ssh -T root@CAMERA_HOST <<'SH'
cat /home/app/custom/boot-marker
exit
SH
```

Expected: `door-station-custom-boot-v1`. Check video, audio and access control
separately. A marker alone does not prove those functions work.

## 6. Enable your script (optional)

This example installs a minimal script that writes one line to the RAM log.
Run only after the hook check succeeds. It refuses an existing startup script
or enable/disable flag:

```sh
ssh -T root@CAMERA_HOST <<'SH'
if [ -e /home/config/custom/start.sh ] ||
   [ -e /home/config/custom/enabled ] ||
   [ -e /home/config/custom/disabled ]; then
    echo 'Custom startup already configured; inspect it before editing.'
    exit 1
fi
mkdir -p /home/config/custom
chmod 700 /home/config/custom
cat > /home/config/custom/start.sh <<'SCRIPT'
#!/bin/sh
echo 'custom startup reached'
SCRIPT
chmod 600 /home/config/custom/start.sh
touch /home/config/custom/enabled
exit
SH
```

After your next planned reboot:

```sh
ssh -T root@CAMERA_HOST <<'SH'
cat /home/app/custom/start.log
exit
SH
```

Expected: `custom startup reached`.

Replace the example script with your own startup commands when ready. Programs
must match ARMv5TE soft float/uClibc. Handle vendor-service readiness yourself;
keep `hicore` running. The hook does not restart failed programs.

To prevent execution on future boots:

```sh
ssh -T root@CAMERA_HOST <<'SH'
touch /home/config/custom/disabled
exit
SH
```

This does not stop an already-running program. Marker and log files disappear
at reboot. [Hook source](../../../tools/door-station-firmware/boot-hook.sh) ·
[Sidecar design](sidecar.md) · [Technical reference](firmware-format.md)
