# Coding standards

## Architecture

- Separate pure domain rules, adapters that perform I/O, and wiring that supplies dependencies.
  Pass time and external dependencies into policy.
- Prefer deep modules: small, clear interfaces that hide substantial implementation details.
  Put boundaries around real responsibilities.
- Give each rule one owner. Share portable behavior; keep hardware, resource limits, image formats
  and recovery specific to each camera model.

## Review checks

- Keep Rust `unsafe` and C/FFI boundaries narrow. Explain unsafe assumptions with `SAFETY` comments;
  check buffer bounds, resource ownership and cleanup on failure.
- Use the documented target builds. Preserve CPU, ABI and SDK pins and compatibility guards.
- For firmware changes, run `make fast` while editing and `make gate` before merging. Host tests use
  isolated files and ports, require no camera, and never open a microphone or play sound.
- Keep READMEs short: supported hardware, features and next steps. Put procedures and reference
  details in linked guides, with one home per fact.
- Use placeholders in examples. Keep credentials, device identities, backups, recordings and generated
  firmware outside Git; fetch vendor headers and libraries through the documented build steps.
