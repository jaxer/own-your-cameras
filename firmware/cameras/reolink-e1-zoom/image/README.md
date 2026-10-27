# The cutover image

The installable image is the reader's own stock 4417 PAK with two sets of additions and nothing removed:

- the five root-SSH paths [own-your-cameras](https://github.com/jaxer/own-your-cameras/blob/main/docs/devices/reolink-e1-zoom/root-access.md)
  adds (its Cortex-A9 Dropbear, the `S99_zzRootSSH` hook and your authorized key);
- three files from this repository:

| Source | Installed as | Mode |
| --- | --- | --- |
| `image/start_app`, with the stock blocks spliced in | `/etc/init.d/start_app` | 0755 |
| `target/armv7-unknown-linux-gnueabihf/release/cam-launch` (from `image/cam-launch.rs`) | `/usr/bin/cam-launch` | 0755 |
| `target/armv7-unknown-linux-gnueabihf/release/cam` | `/usr/bin/cam` | 0755 |

The stock kernel and modules, `/mnt/app`, the libraries, `S00_PreReady` and `S99_Sysctl` stay byte-identical. How
the boot then chooses between `cam` and the vendor stack: [../docs/architecture.md](../docs/architecture.md#boot).

## `start_app`

The installed `start_app` contains the stock 4417 initialisation (steps 1–7 through `./do_before_app`) and the stock
vendor launch block, including its serial-console handling. Those blocks are Reolink's and are not in this
repository: `image/start_app` has `@STOCK_INIT@` and `@STOCK_VENDOR@` placeholders, and `scripts/start_app.py` takes
both blocks from your stock PAK, refuses anything but their pinned SHA-256 digests and splices them in. Without the
flag the vendor block runs in the foreground exactly as before, five-second delay included.

## Building it

`scripts/cutover_image.py` assembles the image offline and never contacts a camera. It checks the exact path set,
every other stock entry, the partition table, the sections it leaves alone and the UDIV guard, then writes the PAK,
`stock.spub` ([../docs/recovery.md](../docs/recovery.md)), `inventory.json` and `manifest.json` (every input and
output hash). `--expect FILE` refuses any hash that differs from a previous build. The own-your-cameras commit and
file hashes it uses are pinned in `scripts/own_your_cameras.py`.

Follow the [image build and installation commands](../docs/install.md#build-the-image).

The PAK embeds your public key, so keep it private like the one own-your-cameras builds.

`scripts/flash_cutover.py` checks the built image against its manifest and prints the plan. With
`--execute-approved-flash --host CAMERA_IP --operation-dir DIR` it uploads the image once through own-your-cameras'
`upload.py` and the stock web API, configuration retained, without retry. The stock web API only answers while the
vendor stack runs, so `cam.enabled` must be absent when you flash.

## Tests

`make check` runs the launcher matrix twice, through `scripts/test_launcher.py` and `cam/tests/boot_launcher.rs`;
both execute `scripts/launcher_tests.py` against the real `start_app` and `cam-launch`. `start_app --fixture-root DIR`
replaces only the stock hardware blocks with fixture executables. Successive boots, faults and health records run
in temporary trees; no camera, broker or host hardware is touched. They do not prove a camera boot.

## Limits

- A hang inside the byte-identical stock initialisation blocks SSH, exactly as on stock firmware: `para` is mounted
  inside it and the SSH hook follows it. Only S00's watchdog helps then.
- The stock `watchdog_monitor_start` asks for a 30 s timeout and feeds 90 times, then stops feeding. Whether that
  resets the SoC in time on its own was not verified.
- If both reboot requests fail before the daemon owns the watchdog, recovery depends on S00. After the handoff, the
  daemon's 60 s hardware watchdog is the recovery boundary.
- BusyBox 1.31.1 on the camera has no `timeout` applet and there is no sysrq; the protocol relies on neither.
