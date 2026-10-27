# Build Reolink E1 Zoom firmware

Complete the [host setup](../../../README.md#host-development), then run from the repository root:

```sh
rustup target add --toolchain 1.95.0 armv7-unknown-linux-gnueabihf
make -C firmware build-reolink
```

Outputs: `firmware/cameras/reolink-e1-zoom/target/armv7-unknown-linux-gnueabihf/release/{cam,cam-launch}`.
The build uses local HDAL declarations and link interfaces, then checks Cortex-A9 instruction compatibility.
No camera, stock PAK, proprietary library or SDK-header download is needed to compile.

For a prebuilt binary and image builder, see [binary releases](../../../RELEASES.md).
Personalized [image assembly and installation](install.md) still need the exact stock4417 PAK,
retained NAND backup, SSH worker and your Ed25519 public key.
