# Install from an already rooted stock camera

This procedure preserves live DATA, Wi-Fi settings, calibration and SSH host keys. It stages only the
inactive kernel/application pair. The first installation requires supported stock **B** selected,
as established by the root-access guide; staging replaces the old stock A recovery pair.
The consolidated source/image has host and offline build checks, not new on-camera acceptance.

Use stable power, the original full-chip backup and available external-programmer recovery. Keep the
stock watchdog feeding throughout staging/activation. Do not stop it or try to run the new daemon beside
stock media. Stop on any failed check; do not improvise environment writes or reboot after failed
activation verification. Activation changes one nonredundant environment and is not power-loss atomic.

## Preserve the working access path

With `WORK` set to a private directory outside the checkout and `umask 077`:

```sh
umask 077
ssh root@CAMERA_HOST 'id; fw_printenv -n boot_flag; fw_printenv -n bootcmd; cat /proc/mtd; mount; pidof imi_watchdog'
ssh root@CAMERA_HOST 'tar -C /mnt/data -cf - .' > "$WORK/data-before.tar"
ssh root@CAMERA_HOST 'cat /dev/mtd0' > "$WORK/boot-before.bin"
```

Require root, `boot_flag=1`, B's kernel command (`sf read … 0x8f0000 0x1b0000`),
`/dev/mtdblock5` mounted at `/mnt/usr`, the documented nine partitions and one stock watchdog owner.
The DATA archive contains credentials/keys: keep it private, verify it with `tar -tf`, and protect
or encrypt it with your normal backup tools. Never upload these files.

Record recovery identity while this trusted SSH path works:

```sh
ssh root@CAMERA_HOST 'cat /sys/class/net/wlan0/address' > "$WORK/radio-mac.txt"
ssh-keygen -F CAMERA_HOST -f "$HOME/.ssh/known_hosts" > "$WORK/trusted-host-entry"
ssh-keygen -lf "$WORK/trusted-host-entry" -E sha256
```

Save the Ed25519 fingerprint with the radio address privately. The known_hosts lookup must use the
actual name under which the key was already trusted (the resolved host or configured HostKeyAlias).
Do not replace a missing/mismatched pin using `ssh-keyscan`. When recovery uses another SSH alias,
pass `--host-key-alias TRUSTED_KNOWN_HOSTS_NAME` as well as `--host CAMERA_SSH_ALIAS`.

Prepare the [private config and working WPA settings](configuration.md), with the native `enabled`
flag absent. Staging below leaves DATA untouched; those settings and the existing SSH host key survive.

## Upload the exact reviewed files

Before uploading, check `cat /proc/meminfo` and `df -k /tmp` over SSH. RAM must hold both files plus
the writer's full-image buffer and normal processes. If insufficient, use the updater's supported
mounted-microSD input path instead. Do not fill RAM or terminate unrelated owners to make room.

Use the outputs from [building the image](build.md#image):

```sh
KERNEL="$WORK/native-image/bootstrap/kernel-for-update.bin"
USRFS="$WORK/native-image/B_USRFS.bin"
KERNEL_SHA=$(shasum -a 256 "$KERNEL" | cut -d ' ' -f 1)
USRFS_SHA=$(shasum -a 256 "$USRFS" | cut -d ' ' -f 1)
ssh root@CAMERA_HOST 'set -eu; umask 077; test ! -e /tmp/mi360-update-tools; mkdir -m 700 /tmp/mi360-update-tools'
ssh root@CAMERA_HOST 'cat > /tmp/mi360-update-tools/kernel.bin' < "$KERNEL"
ssh root@CAMERA_HOST 'cat > /tmp/mi360-update-tools/usrfs.bin' < "$USRFS"
ssh root@CAMERA_HOST 'set -eu; cat > /tmp/mi360-update-tools/camera-mtd-write; chmod 700 /tmp/mi360-update-tools/camera-mtd-write' < "$WORK/qualified-writer/camera-mtd-write"
ssh root@CAMERA_HOST 'set -eu; cat > /tmp/mi360-update-tools/camera-update; chmod 700 /tmp/mi360-update-tools/camera-update' < "$WORK/qualified-writer/camera-update"
shasum -a 256 "$KERNEL" "$USRFS" "$WORK/qualified-writer/camera-mtd-write" "$WORK/qualified-writer/camera-update"
ssh root@CAMERA_HOST 'sha256sum /tmp/mi360-update-tools/kernel.bin /tmp/mi360-update-tools/usrfs.bin /tmp/mi360-update-tools/camera-mtd-write /tmp/mi360-update-tools/camera-update'
```

Review all four uploaded hashes against the local files before executing anything. The pair is fully
padded: kernel 0x1b0000 bytes, USRFS 0x470000 bytes.

## Plan and stage inactive A

These plans do not erase or write partitions:

```sh
ssh root@CAMERA_HOST 'set -eu; /tmp/mi360-update-tools/camera-mtd-write plan kernel /tmp/mi360-update-tools/kernel.bin; /tmp/mi360-update-tools/camera-mtd-write plan usrfs /tmp/mi360-update-tools/usrfs.bin'
```

Require both qualified geometry reports and the intended inactive partitions. The writer refuses
other NOR geometry. Stage separately only after reviewing those results:

```sh
ssh root@CAMERA_HOST "/tmp/mi360-update-tools/camera-update stage /tmp/mi360-update-tools/kernel.bin /tmp/mi360-update-tools/usrfs.bin $KERNEL_SHA $USRFS_SHA"
ssh root@CAMERA_HOST 'cat /dev/mtd2' | shasum -a 256
ssh root@CAMERA_HOST 'cat /dev/mtd3' | shasum -a 256
```

Require `Slot A verified. Slot B is still selected.` and independent partition hashes equal to
`KERNEL_SHA` and `USRFS_SHA` respectively. This comparison includes FF padding. Do not activate on
failure. Do not run the stock bootstrap writer, whose older NOR contract is different.

## Activate and independently verify BOOT

Activation does not reboot:

```sh
ssh root@CAMERA_HOST '/tmp/mi360-update-tools/camera-update activate'
ssh root@CAMERA_HOST 'cat /dev/mtd0' > "$WORK/boot-after.bin"
python3 firmware/cameras/mi360/firmware/verify_environment.py \
  --before "$WORK/boot-before.bin" --after "$WORK/boot-after.bin" --target A
```

The host verifier checks both environment CRCs, exact target boot command/flag/retry, every other
variable and every BOOT byte outside the environment. A failure is a reason to retain the running
access path and use the established recovery procedure, not to retry or reboot blindly.
Only after success, reboot explicitly:

```sh
ssh root@CAMERA_HOST sync
ssh root@CAMERA_HOST reboot
```

The first native boot should keep the same pinned SSH host key because DATA was preserved. Verify
independent Wi-Fi/SSH, sensor and current kernel/slot before enabling the daemon as described in the
[enable step](#enable-the-daemon). A failed network path can expose the radio-derived
recovery AP; use your recorded identity with the recovery helper.

## Enable the daemon

After the first native boot, check trusted SSH, working Wi-Fi and the expected SC2336 sensor/slot.
With [configuration](configuration.md) in place:

```sh
ssh root@CAMERA_HOST 'umask 077; printf "1\n" > /mnt/data/mi360/enabled; sync; reboot'
ssh root@CAMERA_HOST 'cat /tmp/mi360-runtime.health; cat /mnt/data/mi360/attempts'
```

Require `attempts` to settle at `1`, the current PID/boot ID and increasing video/audio counters.
Check both streams and Home Assistant controls. For failed starts or AP access, see [recovery](recovery.md).

## Full-chip programmer output

`camera-mi360-16MiB.bin` is a separate raw programmer artifact assembled from the original dump.
It carries **original** DATA, not a later live DATA snapshot. Flashing it would replace subsequently
saved Wi-Fi, native configuration and SSH host keys. The rooted procedure above never flashes it.
A full-chip recovery/installation needs its own reviewed DATA/provisioning and host-key recovery plan;
do not assume the currently prepared DATA or existing known_hosts pin survives that operation.
