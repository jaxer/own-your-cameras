# Legacy loader header

`legacy-loader-header.hex` is the exact468-byte ELF header/program-header prefix of our original failed
Rust probe, whole executable SHA-256
`450f9537dad5349ed2e64a7752c1815009a1601a51f31e0227028686e7dfe668`.
It contains only our generated executable metadata; no vendor bytes, keys or device data.

Tests verify the program-header permutation against this real prefix and corrupted copies. This prefix
is not a complete executable. The full-binary and on-camera checks are separate evidence retained in the
private research archive.
