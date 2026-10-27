# Build Reolink E1 Zoom firmware

Complete the [root-access guide](../../../../docs/devices/reolink-e1-zoom/root-access.md) and
[host setup](../../../README.md#host-development). Keep the stock 4417 PAK, NAND backup,
Cortex-A9 Dropbear worker and your Ed25519 public key.

From the repository root:

```sh
export OWN_YOUR_CAMERAS="$PWD"
export PATH="$OWN_YOUR_CAMERAS/firmware/.tools/zig:$PATH"
cd firmware/cameras/reolink-e1-zoom
rustup target add armv7-unknown-linux-gnueabihf
```

On macOS, add your LLVM bin directory to PATH so `llvm-objdump` is available
(Homebrew on Apple Silicon: `/opt/homebrew/opt/llvm/bin`).

```sh
make tools
make headers
make sysroot CAMERA=CAMERA_HOST
make build
```

`CAMERA_HOST` must log in as root over SSH. `make sysroot` copies and verifies the camera's
`libhdal.so`; a matching cached copy is reused.

Outputs: `target/armv7-unknown-linux-gnueabihf/release/{cam,cam-launch}`.
The build checks Cortex-A9 instruction compatibility. Continue with [installation](install.md).
For host tests and device development, see [development](development.md).
