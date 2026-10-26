# Build the Mi 360 SSH firmware again

[Model / verified results](README.md) · [Programmer wiring](spi-programmer.md)

**Scope:** MJSXJ10CM / T31 / SC2336, original 16 MiB SPI dump with B
`4.3.4_0376` selected. This makes a raw programmer image, **not a Xiaomi OTA file**.
The scripts build local files only; they never connect to or flash a camera.

## 1. Gather the inputs

- Your camera's **original, unmodified full-chip dump**, preferably two matching reads.
  A new dump of the already-rooted camera is not a substitute.
- The original dump's recorded SHA-256. Keep it alongside your backups.
- Your dedicated Ed25519 SSH public key; keep the matching private key on your computer.
- The [Dropbear 2026.94 release tarball](https://github.com/mkj/dropbear/releases/tag/DROPBEAR_2026.94),
  named `dropbear-2026.94.tar.bz2`. Use the linked release tarball, not GitHub's generated source archive.
- Python 3.12+, Zig **0.17.0**, `make`, and both `mksquashfs` and `unsquashfs`
  from squashfs-tools **4.7.5**. Tests also use the host C compiler (`cc`).
  The recipe was checked on macOS.

The source archive must have SHA-256
`e098034a843699200c8c977a991fff73159735bf795d5f72ef672c41a6b1ae81`;
the worker builder checks this automatically. Match the tool versions to repeat
the tested build. The Python scripts use only the standard library.

Run the commands below from this repository's root. Set these paths first; use
a new, private work directory **outside the repository**:

```sh
WORK="$HOME/mi360-rebuild"
DUMP="/path/to/your/original-dump.bin"
DROPBEAR_SOURCE="/path/to/dropbear-2026.94.tar.bz2"
SSH_PUBLIC_KEY="/path/to/your/dedicated-key.pub"
EXPECTED_DUMP_SHA256="REPLACE_WITH_YOUR_RECORDED_ORIGINAL_DUMP_SHA256"
mkdir -m 700 "$WORK"
```

When first recording the backup digest, compare your two original reads and
print the digest only if they match:

```sh
cmp "$DUMP" "/path/to/your/second-original-read.bin" && shasum -a 256 "$DUMP"
```

Save the printed digest with your backups and replace `EXPECTED_DUMP_SHA256`
above with it. For a later rebuild, use that saved digest rather than accepting
a new digest for an unexpectedly changed file.

If you need a new key, create it separately with `ssh-keygen -t ed25519 -f KEY_PATH`
and set `SSH_PUBLIC_KEY` to its `.pub` file. Existing installations need the private
key matching the public key used when their image was built.

## 2. Build the SSH server and update helper

```sh
python3 tools/mi-360/build_dropbear.py "$DROPBEAR_SOURCE" \
  --output-dir "$WORK/workers"
```

Expect `dropbearmulti-mipsel`, `camera-mtd-write`, `manifest.json` and build logs
under `$WORK/workers`. Both executables are static MIPS32r2 little-endian / O32;
the target is `mipsel-linux-musleabi`. Password authentication and forwarding are
disabled. Ed25519 key login is enabled.

## 3. Build the raw flash image

```sh
python3 tools/mi-360/build.py "$DUMP" \
  --expected-sha256 "$EXPECTED_DUMP_SHA256" \
  --ssh-binary "$WORK/workers/dropbearmulti-mipsel" \
  --writer "$WORK/workers/camera-mtd-write" \
  --authorized-key "$SSH_PUBLIC_KEY" \
  --output-dir "$WORK/image"
```

Expect `camera-ssh-16MiB.bin` (16,777,216 bytes), `usrfs-ssh.squashfs`,
`kernel-for-update.bin` (the unchanged B kernel) and `manifest.json`.
An existing output directory is refused. After a failure, keep its logs and
retry with a new output directory.

The builder checks the input digest, stored boot-environment CRC, exact partition
map, B selection, firmware version, both kernel CRCs and filesystem headers.
It then preserves every original B_USRFS entry, including bytes, links, owner,
mode and timestamps. Only these paths are added:

```text
bin/dropbear
bin/dropbearkey -> dropbear
bin/camera-mtd-write
bin/camera-update
etc/root-ssh/
etc/root-ssh/authorized_keys
etc/init.d/S99zz_rootssh
```

Every byte outside B_USRFS (`0xaa0000` through `0xf0ffff`) must remain identical.
That preserves the bootloader/environment, shared ROOTFS, both kernels, A_USRFS,
DATA, SOUND and FACTORY. A changed layout/version, existing SSH paths or an
oversized application image stops the build. The supplied digest identifies your
chosen backup; it is not a vendor signature or proof of hardware compatibility.
Inputs with extended attributes are refused because this packing recipe does
not preserve them; the inspected filesystems had none.

## 4. Check the result offline

```sh
python3 tools/mi-360/test_tools.py \
  --dump "$DUMP" \
  --ssh-binary "$WORK/workers/dropbearmulti-mipsel" \
  --writer "$WORK/workers/camera-mtd-write" \
  --authorized-key "$SSH_PUBLIC_KEY" \
  --reference-image "$WORK/image/camera-ssh-16MiB.bin"
```

This rebuilds in a temporary directory, compares the complete result, checks
protected flash regions and tests refusal of corrupted inputs and changed boot
settings. No camera or flash device is opened. For format-only unit tests,
run `python3 tools/mi-360/test_tools.py` without arguments.

Fresh exported worker builds matched the workers used in the original installation
byte for byte. The exported image recipe preserves the original stock entries and
was checked offline against the real backup. The startup hook uses a renamed
shell-local variable; exported image bytes therefore differ from the original
installed image. No new camera flash or cold-boot test was performed for this export.

## 5. Install separately, when ready

Use the [SPI programmer setup](spi-programmer.md) and a known-working programmer.
The ESP32 setup page records a required local firmware patch; that programmer
patch is not distributed here. The build scripts do not program the ESP32.

With the camera unpowered, the flash isolated as described in that guide, and
`PROGRAMMER_PORT` set to your programmer's serial port, these are the separate
**write and independent readback** commands:

```sh
flashrom -p "serprog:dev=$PROGRAMMER_PORT,spispeed=4M" \
  -w "$WORK/image/camera-ssh-16MiB.bin"
flashrom -p "serprog:dev=$PROGRAMMER_PORT,spispeed=4M" \
  -r "$WORK/after-write.bin"
cmp "$WORK/image/camera-ssh-16MiB.bin" "$WORK/after-write.bin"
```

Proceed to normal camera power only after successful full readback comparison.
Disconnect the programmer and resolder lifted VCC pin 8 to its PCB pad. Complete
normal Xiaomi Home Wi-Fi setup if needed, then connect using your matching key:

```sh
ssh -i SSH_PRIVATE_KEY root@CAMERA_HOST
```

Expect UID 0 from `id`. The startup hook stores its SSH home and persistent host
key in JFFS2 DATA and binds the home onto `/root`. It refuses startup without the
expected JFFS2 mount. Vendor services and OTA remain present; OTA can replace SSH.

`camera-update` is included to reproduce the original additions. Remote staging
and activation remain untested on hardware; do not treat them as the verified
installation path. See the [single-environment power-loss limits](README.md#boot-and-recovery).
Keep original dumps and generated files private: full-chip images retain your
device's factory data and configuration, and the candidate contains your public key.
