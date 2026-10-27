# Configure and install Reolink firmware

Run from `firmware/cameras/reolink-e1-zoom` after [building](build.md).
Set `OWN_YOUR_CAMERAS` to this checkout's root if opening a new shell.

## Configure

Create these root-owned 0600 files in `/mnt/para` while vendor firmware runs:

| File | Contents |
|---|---|
| `cam-wifi.conf` | Your [WPA-PSK network](hardware/network.md#cam-wificonf) |
| `cam-rtsp.auth` | One line: `USER:PASSWORD` |
| `cam.toml` | Your [MQTT broker configuration](home-assistant.md#camtoml) |

Upload the binary to RAM for the configuration check:

```sh
ssh root@CAMERA_IP 'cat > /mnt/tmp/cam && chmod +x /mnt/tmp/cam' < target/armv7-unknown-linux-gnueabihf/release/cam
```

To reuse the camera's current Wi-Fi settings instead of writing `cam-wifi.conf`:

```sh
ssh root@CAMERA_IP /mnt/tmp/cam net --seed-vendor
```

Minimal `cam.toml`:

```toml
[mqtt]
host = "MQTT_HOST"
username = "MQTT_USERNAME"
password = "MQTT_PASSWORD"
```

Check all three files:

```sh
ssh root@CAMERA_IP /mnt/tmp/cam preflight
```

Require `preflight: ready` before continuing.

## Build the image

Replace `STOCK_PAK`, `DROPBEAR_BINARY` and `KEY.pub` with your retained input paths.
The Dropbear worker is `build/reolink-worker/dropbearmulti-cortex-a9` from root setup.

```sh
python3 -m venv .venv
.venv/bin/pip install -r "$OWN_YOUR_CAMERAS/tools/reolink-e1-zoom/requirements.txt"
.venv/bin/python -I scripts/cutover_image.py --stock-pak STOCK_PAK --own-your-cameras "$OWN_YOUR_CAMERAS" \
  --ssh-binary DROPBEAR_BINARY --authorized-key KEY.pub --output-dir target/image
```

## Flash and enable

The vendor firmware must be running with `/mnt/para/cam.enabled` absent.
Review the plan first:

```sh
python3 -I scripts/flash_cutover.py --own-your-cameras "$OWN_YOUR_CAMERAS" --image-dir target/image
```

Then upload once using a new operation directory. The camera reboots after the upload:

```sh
python3 -I scripts/flash_cutover.py --own-your-cameras "$OWN_YOUR_CAMERAS" --image-dir target/image \
  --execute-approved-flash --host CAMERA_IP --operation-dir target/flash-1
```

If the camera uses a self-signed HTTPS certificate, `--insecure` accepts it; use this only on a trusted network.
The first boot runs vendor firmware. Enable the new daemon:

```sh
ssh root@CAMERA_IP '/usr/bin/cam preflight && touch /mnt/para/cam.enabled && reboot'
```

After about three minutes, check the streams and Home Assistant controls, then:

```sh
ssh root@CAMERA_IP cat /mnt/para/cam.crashes
```

A healthy boot leaves the counter at `1`. Keep your stock PAK and NAND backup for recovery.
Never run `cam` alongside the vendor media process. See [recovery](recovery.md) and
[image details](../image/README.md).
