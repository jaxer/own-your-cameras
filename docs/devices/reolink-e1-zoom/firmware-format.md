# Firmware layout, backups and recovery limits

[E1 Zoom overview](README.md) · [Build/install procedure](root-access.md)

## Changes made by the SSH builder

| Path | Change |
|---|---|
| `/root` | Existing uid/gid 1050:1050, mode 0777 → uid/gid 0:0, mode 0700 |
| `/root/.ssh` | Add root-owned directory, mode 0700 |
| `/root/.ssh/authorized_keys` | Add your public key, root-owned mode 0600 |
| `/usr/sbin/dropbear` | Add the static Cortex-A9 multi-binary, mode 0755 |
| `/usr/bin/dropbearkey` | Add symlink `../sbin/dropbear` |
| `/etc/init.d/S99_zzRootSSH` | Add background startup hook, mode 0755 |

The stock passwd record and all existing file contents remain unchanged.
Root's ownership/permissions need correction for authorized-key validation.
SquashFS pseudo exports preserve symlinks, owners, modes and timestamps without
mounting or executing firmware; a regular-file-only extraction would lose them.

The [hook](../../../tools/reolink-e1-zoom/S99_zzRootSSH) sorts after `S99_Sysctl`.
It requires `/mnt/para` to be actually mounted as YAFFS2, takes a per-boot lock,
rejects symlink key locations, generates/reuses an Ed25519 host key and starts
Dropbear on TCP 22. Work runs in the background; failure does not block `rcS`.
Logs are in `/mnt/tmp/root-ssh.log`; host identity is stored in
`/mnt/para/root-ssh/dropbear_ed25519_host_key`, outside the read-only rootfs.
The hook neither repairs mounts nor stops vendor services.

## PAK and UBI

The exact stock PAK is 25,699,816 bytes, magic `0x32725913`, format word `0x4102`.
It has a `0x728`-byte header and six nonempty contiguous sections: loader, fdt,
uboot, kernel, rootfs, app. The builder changes only the rootfs section payload,
updates lengths/offsets and recomputes the legacy CRC.

CRC input order: bytes after `0x728`, little-endian integer 2, thirteen 64-byte
section records at `0x0c..0x34b`. With Python `zlib.crc32`, use initial state
`0xffffffff` and final XOR `0xffffffff`. Stock result: **`0xc77de7c5`**.
The MTD table at `0x34c..0x727` lies outside that CRC coverage; it is separately
checked for byte equality. CRC matching does not establish signature validity.

| Rootfs container property | Value |
|---|---|
| PEB / VID / data offsets | 131072 / 2048 / 4096 bytes |
| Logical eraseblock | 126976 bytes |
| Volume | Dynamic, ID 0, `rootfs`, two identical layout tables |
| Image sequence | 383159553, preserved from the exact stock template |
| Filesystem | SquashFS 4.0, XZ, 262144-byte blocks |
| Rootfs partition | 15 MiB (`0xf00000`) |

The narrow [UBI helper](../../../tools/reolink-e1-zoom/ubi.py) rejects unsupported
metadata, verifies EC/VID/volume-table CRCs and logical numbering, updates both
volume tables, preserves the stock headers and checks a byte-identical stock
roundtrip. At least six eraseblocks must remain outside the emitted image.
This capacity guard does not prove sufficient good blocks on another device.
It is not a general UBI filesystem editor.

Preserved package sections can still be erased/rewritten by the updater.
Static tracing found writes to the download MTD staging area; an upload is a
state change. No complete final-writer skip policy, atomic update, automatic
rollback or absence of signatures throughout the chain was established.

## Back up after root access

First inspect the actual unit. These are read-only commands:

```sh
ssh -i ~/.ssh/id_ed25519_camera root@CAMERA_HOST \
  'cat /proc/mtd; cat /proc/mounts; /usr/sbin/nanddump --help'
```

Match this layout before reading the all-flash device:

| MTD | Name | Offset | Size |
|---|---|---:|---:|
| 0 | loader | `0x0000000` | `0x0040000` |
| 1 | fdt | `0x0040000` | `0x0040000` |
| 2 | uboot | `0x0080000` | `0x0100000` |
| 3 | kernel | `0x0180000` | `0x0400000` |
| 4 | rootfs | `0x0580000` | `0x0f00000` |
| 5 | app | `0x1480000` | `0x0b00000` |
| 6 | para | `0x1f80000` | `0x0400000` |
| 7 | sp | `0x2380000` | `0x0080000` |
| 8 | ext_para | `0x2400000` | `0x0080000` |
| 9 | download | `0x2480000` | `0x1b80000` |
| 10 | all | `0x0000000` | `0x4000000` |

The tested BusyBox `nanddump -o` **includes OOB**, unlike mtd-utils conventions.
Inspect the real help output; do not transfer flags between implementations.
For this exact layout/tool, stream encrypted captures to the computer. Install
`age` on the computer and replace `AGE_RECIPIENT` with your encryption recipient.
Use the private `OWNER_DATA_DIR` created during SSH setup (or create a new one
outside this repository). In Bash or zsh:

```sh
set -o pipefail
umask 077
ssh -i ~/.ssh/id_ed25519_camera root@CAMERA_HOST \
  '/usr/sbin/nanddump --bb padbad -f - /dev/mtd10' \
  | age -r AGE_RECIPIENT -o OWNER_DATA_DIR/nand-ecc-corrected.bin.age
ssh -i ~/.ssh/id_ed25519_camera root@CAMERA_HOST \
  '/usr/sbin/nanddump -n -o --bb padbad -f - /dev/mtd10' \
  | age -r AGE_RECIPIENT -o OWNER_DATA_DIR/nand-raw-with-oob.bin.age
ssh -i ~/.ssh/id_ed25519_camera root@CAMERA_HOST \
  'tar -C /mnt/para -cf - .' \
  | age -r AGE_RECIPIENT -o OWNER_DATA_DIR/para-files.tar.age
```

Check the exit status of **each** pipeline and retain stderr privately. Any failure
leaves an incomplete backup; preserve evidence and investigate before retrying.
The para archive contains credentials/configuration and the SSH host private key.
Keep all captures private, including encrypted copies.

Decrypt each capture into a pipe to verify its plaintext size and hash; no
plaintext file is needed. Replace `AGE_IDENTITY` with your local decryption key:

```sh
age -d -i AGE_IDENTITY OWNER_DATA_DIR/nand-ecc-corrected.bin.age | wc -c
age -d -i AGE_IDENTITY OWNER_DATA_DIR/nand-raw-with-oob.bin.age | wc -c
age -d -i AGE_IDENTITY OWNER_DATA_DIR/para-files.tar.age | tar -tf -
age -d -i AGE_IDENTITY OWNER_DATA_DIR/nand-ecc-corrected.bin.age | shasum -a 256
age -d -i AGE_IDENTITY OWNER_DATA_DIR/nand-raw-with-oob.bin.age | shasum -a 256
```

Expected NAND sizes: **67,108,864** corrected bytes and **69,206,016** raw/OOB bytes.
Record both plaintext and ciphertext hashes privately. Also capture `/proc/mtd`,
CPU/kernel/mount/boot information and relevant logs. Check the real NAND health:

```sh
ssh -i ~/.ssh/id_ed25519_camera root@CAMERA_HOST \
  'cat /sys/class/mtd/mtd10/bad_blocks; cat /sys/class/mtd/mtd10/ecc_failures; cat /sys/class/mtd/mtd10/corrected_bits'
```

`--bb padbad` pads bad blocks; a successful exit alone is not proof of a complete
physical read. Preserve geometry, bad-block information and OOB interpretation.
These are live **post-flash** captures, not untouched original or atomic snapshots.
The writable para partition can differ between reads; capture it logically too.
The original procedure verified decrypted sizes/hashes and found nine partition
data ranges identical between corrected and OOB-stripped dumps; only active para
differed. Restore from these captures was not tested.

## Recovery boundary and prior work

If the network updater remains reachable, retained exact stock firmware is the
software rollback material. Its ability to accept a downgrade is not guaranteed.
No automatic rollback is supplied. Do not erase/write MTD as a diagnostic step.

Serial/U-Boot recovery, UART electrical levels/pinout and physical NAND restore
were not verified in this procedure. An intact bootloader is not a recovery
guarantee. Do not use the community example assigning 8 MiB to para: the matching
stock package and live table use **4 MiB**, and the larger range overlaps later
partitions. Raw UBI containers and extracted volume files are different inputs;
never guess which a recovery command expects.

Useful external references, inspected as source rather than executed:

- [gabest11/reolink_firmware_patcher, pinned revision](https://github.com/gabest11/reolink_firmware_patcher/tree/eb7b6a06d2daf0ac643ce72cf4a30787e103f9a7):
  model-family patching and serial recovery precedent; not proof for another unit.
- [pakler legacy package implementation, pinned revision](https://github.com/vmallet/pakler/blob/62dc878ff06326c8a92febbbcf2c15ea0d7db62d/pakler/__init__.py):
  corroborates header/CRC conventions, not physical partition safety.
- [Linux UBI documentation](https://docs.kernel.org/filesystems/ubifs.html):
  distinguishes UBI's raw-flash volume management from a filesystem inside a volume.
