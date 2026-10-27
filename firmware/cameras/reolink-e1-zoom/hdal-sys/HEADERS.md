# Maintained HDAL interface

The build uses [Rust declarations](src/abi.rs), [frozen ARM layout assertions](src/arm_layout.rs)
and the [historical size snapshot](src/header_sizes.rs). It downloads no SDK headers and needs no bindgen/libclang.
Only declarations needed by production, diagnostics and retained ABI checks are maintained.
Implementations remain in the camera's HDAL2.00.000 library.

The reference translation used18 HDAL3.02.000 headers from `yanggungg/trunk_gtk4_dev`,
commit `69386cae2e39480b3969165bac3bd4a86425335a`, path `src/include/novatek.nt9833x/`.
Those vendor headers are not distributed. The retained interface includes the reviewed
`HD_VIDEOENC_OUT` and `HD_VIDEOENC_OUT2` unions and the corrected audio input20 constant.
[ABI method and limits](../docs/hdal-abi.md) explain the binary evidence.

ARM builds compile the frozen size/alignment/offset literals. Host builds preserve actual host-width
pointers while keeping camera scalar widths. These assertions detect declaration drift; they do not
establish new SDK behavior or qualify another camera/firmware revision.
