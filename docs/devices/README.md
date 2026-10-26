# Find your model

The guides describe the versions tested in October 2026. Match the platform and
hardware revision as well as the model name; similar names can need different firmware.

| Model | What you can learn | Tested firmware |
|---|---|---|
| [DS-KD8003-IME1](ds-kd8003-ime1/README.md) | Root shell, firmware startup hook, hardware pinout, audio APIs | V2.2.62 build 230204, original non-B hardware |
| [DS-2CD2T87G2-L](ds-2cd2t87g2-l/README.md) | G3 firmware decoding, hardware, root access, managed upgrade, AI interfaces | V5.5.154 build 200928 → V5.5.820 build 231108, original non-C hardware |
| [DS-2CD2386G2-ISU/SL](ds-2cd2386g2-isu-sl/README.md) | G5 firmware decoding, root access and upgrade persistence | V5.5.113 → V5.7.23 build 260320, C variant |
| [DS-KH9510-WTE1](ds-kh9510-wte1/README.md) | Root SSH, helper build/install, hardware and Android application notes | V2.2.28 build 230613 |
| [Reolink E1 Zoom](reolink-e1-zoom/README.md) | Stock-derived key-only root SSH, rootfs builder, flash and NAND backup procedure | `IPC_566SD664M5MP`, v3.1.0.4417_2412122178 |
| [Mi 360° Camera (1080p)](mi-360/README.md) | Key-only root SSH image builder, SPI programmer setup and A/B boot limits | MJSXJ10CM, `4.3.4_0376`, T31 / SC2336 |

Each model directory contains its overview and device-specific guides.
G3/G5 share the [root-access](../shared/root-access.md) and
[firmware-decoding](../shared/firmware-decoding.md) guides.
