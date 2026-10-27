# Build Mi 360 firmware

Complete the [root-access guide](../../../../docs/devices/mi-360/root-access.md) and
[host setup](../../../README.md#host-development). Keep your original dump, recorded SHA-256,
SSH workers and Ed25519 public key. Install squashfs-tools 4.7.5.

Run from the repository root. Keep all private inputs and image outputs outside the checkout.

## Daemon

```sh
make -C firmware build-mi360
```

Output: `firmware/cameras/mi360/target/mipsel-unknown-linux-uclibc/release/mi360-cam`.
Compilation uses local IMP declarations and generated uClibc link interfaces; no camera/dump is needed.
For prebuilt binaries and the image builder, see [binary releases](../../../RELEASES.md).

## Image

Use the workers retained from root setup and your previously recorded original digest.
Each output directory must be new.

```sh
WORK="/path/to/private-mi360-work"
DUMP="/path/to/original-dump.bin"
mkdir -m 700 "$WORK"
python3 firmware/cameras/mi360/firmware/mtd_writer.py --output "$WORK/qualified-writer"
python3 firmware/cameras/mi360/firmware/build_image.py \
  --original-dump "$DUMP" \
  --expected-original-sha256 YOUR_RECORDED_ORIGINAL_SHA256 \
  --ssh-binary /path/to/workers/dropbearmulti-mipsel \
  --bootstrap-writer /path/to/workers/camera-mtd-write \
  --authorized-key /path/to/your/key.pub \
  --cam firmware/cameras/mi360/target/mipsel-unknown-linux-uclibc/release/mi360-cam \
  --writer "$WORK/qualified-writer/camera-mtd-write" \
  --output "$WORK/native-image"
```

For [installation](install.md), use `native-image/B_USRFS.bin`,
`native-image/bootstrap/kernel-for-update.bin` and the qualified writer/tools.
The builder checks the supported stock components and preserves the remaining flash regions.

The full-chip `camera-mi360-16MiB.bin` includes original DATA and your SSH public key.
Keep it private. Flashing it would overwrite later settings and host keys; use the inactive-slot
installer for an already rooted camera.
