# Build and install root SSH

[E1 Zoom overview](README.md) · [Format and recovery limits](firmware-format.md)

Supported input: **`IPC_566SD664M5MP`, v3.1.0.4417_2412122178 only**.
Use a terminal at the repository root. Replace uppercase path/host placeholders
with your own values; all generated files and backups must stay private.

## 1. Prepare inputs

- Obtain the matching stock PAK from [Reolink](https://reolink.com/download-center/)
  or request this exact historical build from support. A working direct download
  for this historical PAK is not supplied. A newer PAK is not a substitute.
- Record hardware/firmware identity and important settings before changing anything.
  On the tested build `ExportCfg` and ONVIF system-backup/log calls were unsupported;
  ordinary configuration API reads supplied the pre-flash backup. Raw NAND required
  root/UART and was captured only after flashing. Save settings before proceeding;
  this repository does not include an automated pre-root settings exporter.
- Keep the exact stock PAK for recovery and decide how you will recover if the
  network updater becomes unavailable. Serial/U-Boot recovery requires opening
  the camera and was not tested in this procedure.
- Install Python 3.12 or newer, Make, Zig **0.17.0**, SquashFS tools **4.7.5**,
  and OpenSSH. The export was checked with Python 3.14 on macOS. Do not use Python
  `-O`: the builder refuses it because validation uses assertions.

Required stock filename:

```text
IPC_566SD664M5MP.4417_2412122178.E1-Zoom.5MP.WIFI7.PTZ.REOLINK.pak
```

Required SHA-256:

```text
ae1227a89eae4d801381490c95b3fbc38e5e0040d7c839d90f1993ec0b771d5c
```

Generate a dedicated key, or supply an existing dedicated Ed25519 public key.
The command prompts before overwriting an existing identity; choose a fresh path.

```sh
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519_camera
```

Keep the private key outside the build. The PAK embeds the public key and therefore
also stays private. Use new output directories for every attempt, even after failure.

## 2. Build the Cortex-A9 worker

Download the pinned source archive from the
[Dropbear release directory](https://matt.ucc.asn.au/dropbear/releases/):

```sh
mkdir -p build/reolink-inputs
curl --fail --location \
  https://matt.ucc.asn.au/dropbear/releases/dropbear-2026.94.tar.bz2 \
  --output build/reolink-inputs/dropbear-2026.94.tar.bz2
python3 tools/reolink-e1-zoom/build_dropbear.py \
  build/reolink-inputs/dropbear-2026.94.tar.bz2 \
  --output-dir build/reolink-worker
```

The helper checks the archive hash and compiler version, uses a fresh source tree,
configures `arm-linux-musleabihf -mcpu=cortex_a9` with static musl, and builds
`dropbear` + `dropbearkey` as one executable. Build logs stay in the output directory.
The source archive SHA-256 is
`e098034a843699200c8c977a991fff73159735bf795d5f72ef672c41a6b1ae81`.
Upstream release signatures were not independently authenticated in this procedure.

Password/PAM, forwarding and SFTP are compiled out in
[`localoptions.h`](../../../tools/reolink-e1-zoom/localoptions.h).
The static worker avoids dependence on the vendor glibc. A different compiler,
source release or options require renewed CPU/runtime validation. The PAK builder
refuses workers whose hash differs from the tested build:

```text
8610bab73426103d9e046de8a47ede7fd4a4b04c0927cda13da37a1822be6334
```

The exported recipe reproduced this hash on the tested toolchain. If your build
differs, stop and investigate; changing the hash guard requires a new validation
of the binary and startup behavior.

## 3. Build the PAK offline

```sh
python3 tools/reolink-e1-zoom/build.py STOCK_PAK \
  --ssh-binary build/reolink-worker/dropbearmulti-cortex-a9 \
  --authorized-key ~/.ssh/id_ed25519_camera.pub \
  --output-dir build/reolink-root-ssh
```

Expected: **922 stock entries verified**, five added paths, only `/root` ownership
and mode changed among existing entries. The manifest must report unchanged
loader, fdt, uboot, kernel and app payloads and an identical MTD table.
The worker must be a static ARM32 little-endian ELF. This check alone cannot
prove Cortex-A9 compatibility; use the exact worker recipe above.

| Output | Purpose |
|---|---|
| `IPC_566SD664M5MP.4419_2412122178.E1-Zoom.5MP.WIFI7.PTZ.REOLINK_patched.pak` | Candidate for review and upload |
| `manifest.json` | Input/output hashes, changed paths, preserved sections and capacity checks |
| `stock.squashfs`, `rootfs.squashfs`, `rootfs.ubi` | Offline inspection |
| `stock.pseudo`, `candidate.pseudo` | Filesystem content and metadata comparison |

`4419` is the output filename build, not a new vendor software release. Internal
version files remain unchanged, so Device Info still reports **4417** after install.
Your PAK hash depends on your public key and inputs; record your own manifest hash.

## 4. Verify independently

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r tools/reolink-e1-zoom/requirements.txt
.venv/bin/ubireader_extract_images \
  -o build/reolink-root-ssh/independent-ubi-extract \
  build/reolink-root-ssh/rootfs.ubi
.venv/bin/python tools/reolink-e1-zoom/verify.py STOCK_PAK build/reolink-root-ssh
```

Dependency installation uses the network; filesystem verification is offline.
Expected: `passed: true`, final PAK hash/CRC, preserved sections and MTD table
checked against the exact original PAK; all stock files decompressed/compared,
metadata and symlinks checked, and added SSH paths/authorized key verified.
The independent extraction is bound to the actual rootfs bytes in that final PAK.
Run the isolated checks against the real stock artifact and this candidate:

```sh
.venv/bin/python tools/reolink-e1-zoom/test_tools.py \
  --stock-pak STOCK_PAK --build-dir build/reolink-root-ssh
```

Expected: 16 tests pass, including write-failure handling and detection of a
replaced final PAK or wrong authorized-key hash. No camera connection is made.
Review the manifest, hook and keys locally. Do not upload until all checks pass.

For the original installation, the actual candidate filesystem and stock
BusyBox/glibc were also tested in a Cortex-A9 VM with Linux 4.19 and a real key
login. The VM harness is not included here. Filesystem checks and a generic ARM
VM do not replace that CPU/runtime test for changed binaries or startup code.

## 5. Upload once — changes the camera

**An upload writes flash and triggers installation/reboot; it is not a dry run.**
Backups and a recovery plan must already be in place. Keep power stable. The
included uploader follows the tested `UpgradePrepare`/chunked `Upgrade` sequence
with configuration reset disabled. It verifies the live model, hardware and
firmware first. Its generalized implementation is offline-tested only.

Copy the exact `candidate.sha256` from your reviewed manifest into `IMAGE_SHA256`:

```sh
python3 tools/reolink-e1-zoom/upload.py \
  build/reolink-root-ssh/IPC_566SD664M5MP.4419_2412122178.E1-Zoom.5MP.WIFI7.PTZ.REOLINK_patched.pak \
  --sha256 IMAGE_SHA256
```

This first command validates/describes the file offline. To perform the actual
upload, add `--host CAMERA_HOST --flash`. It prompts for the administrator password;
no password goes in the command line. If the device uses a self-signed HTTPS
certificate, `--insecure` explicitly disables certificate verification: use that
only on a network you trust after checking the target yourself.

The uploader sends 40,960-byte chunks and never retries a write or sends a separate
reboot. If it fails or disconnects, stop and inspect availability/version before
considering another attempt. An upload acknowledgement does not prove boot success.

[Reolink Client manual update](https://support.reolink.com/articles/34817575507481-How-to-Manually-Update-Firmware-for-Reolink-Devices/)
provides a vendor alternative: System → Maintenance → Firmware Update; leave
**Reset Configuration unchecked**. That GUI route was not used to validate this
custom image; use the API procedure above when reproducing the tested route.

## 6. Verify SSH and retained settings

First create a fresh private `OWNER_DATA_DIR` outside the repository if you do
not already have one. Collect the public host key from the physically identified
camera on a trusted local network:

```sh
umask 077
mkdir -m 700 OWNER_DATA_DIR
mkdir -p ~/.ssh
chmod 700 ~/.ssh
ssh-keyscan -T 10 -t ed25519 CAMERA_HOST > OWNER_DATA_DIR/known_hosts.candidate
ssh-keygen -lf OWNER_DATA_DIR/known_hosts.candidate
```

`ssh-keyscan` does not authenticate the key. Compare the displayed fingerprint
with an independently obtained trusted value if you have one. Otherwise this
is **trust on first use**, based on your verified device/network, not independent
cryptographic verification; that was the initial installation's trust basis.
UART-based independent verification was not tested. Once you accept that trust
basis, install the candidate in your client host database:

```sh
cat OWNER_DATA_DIR/known_hosts.candidate >> ~/.ssh/known_hosts
chmod 600 ~/.ssh/known_hosts
```

For an existing pin, compare it first and investigate a changed key instead of
appending a replacement blindly. Then:

```sh
ssh -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes \
  -i ~/.ssh/id_ed25519_camera root@CAMERA_HOST 'id -u; uname -r'
ssh -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes \
  -i ~/.ssh/id_ed25519_camera root@CAMERA_HOST \
  'sha256sum /usr/sbin/dropbear; cat /proc/cpuinfo; cat /mnt/tmp/root-ssh.log'
```

Expected UID **0**, kernel **4.19.91**, Cortex-A9 CPU, and a worker hash matching
your build. Verify camera video, audio, PTZ and selected settings separately.
The observed installation preserved eleven selected responses: local network,
Wi-Fi, ports, encoding, DDNS, NTP, recording, push, PTZ presets, PTZ guard and
system configuration. That is not a complete functional test.

The hook generates the host key once under `/mnt/para/root-ssh/`. It was started
by the real firmware-install boot; a further independent reboot test was not
performed. Back up the host key with the configuration. Replacing firmware or
factory-resetting the camera can remove access or change identity.

SFTP/SCP is not supplied; use SSH shell/pipe transfers. Keep vendor processes
running; see [backup commands and recovery limits](firmware-format.md#back-up-after-root-access).
