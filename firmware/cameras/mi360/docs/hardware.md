# Qualified hardware boundary

Only MJSXJ10CM / T31 / SC2336 on the pinned 4.3.4_0376 stock kernel and SDK is supported. Other Mi 360
models, sensors and kernels need separate qualification even if device nodes have the same names.
The [stock component lock](../firmware/stock.lock.json) identifies the accepted ROOTFS, kernel, SDK,
modules and tuning files. The image recipe keeps those components unchanged.

## Ownership and shutdown

`src/media.rs` owns one IMP system and serializes its C adapters in `sdk/`. The C interfaces assert
actual O32 structure sizes/offsets; host sanitizer fixtures test lifecycle and bounds without loading
proprietary libraries. Failed acquisition/cleanup retains uncertain ownership and requests a clean
boot. It never hands a partially initialized SDK to a second owner.

`src/board.rs` owns motor/GPIO policy; `src/board/linux.rs` owns device/sysfs/ioctl I/O. A separate
hardware worker prevents ordinary board operations from blocking the media schedule. SDK calls and
kernel ioctls can still block; the independent watchdog is the final escalation boundary.

The motor uses qualified normal travel, pan 0–2280 and tilt 0–650 raw steps. Those counters are not
encoders. Startup and manual Home perform a finite physical reference seek and validated center
return. Moving the head by hand while powered requires Home again. Rotating the camera body changes
the world view and cannot be corrected by head homing. Gear play means raw counter equality alone
does not guarantee an identical view; presets retain each axis's last approach direction.

The timer repair and atomic retarget path in `src/motor_clock.rs` accesses TCU channel 2 through
`/dev/mem`. Its qualified assumptions include one CPU, hard IRQ delivery and the exact stock driver.
It preserves watchdog/channel 0 state. Do not run it against an unqualified kernel; matching an SDK
version string or sensor name is insufficient.

The qualified day/night bridge pulses GPIO 51 or 54 for 500 ms, then lowers both; illumination uses
GPIO 49. GPIO readback cannot reveal the mechanical filter position after the pulse. The speaker
amplifier has its own `/dev/speaker` lifetime, separate from audio buffer delivery.

## Image pointing

A valid optional root-owned 0600 `/mnt/data/mi360/position3d.json` enables image-point requests. Missing
calibration disables them; there are no guessed lens coefficients. The immediate directory must be
root-owned 0700. Symlinks, hard links, duplicate/unknown fields and files over 4096 bytes are refused.

The flat JSON schema contains `version: 1`, `pan_a`, `pan_b`, `tilt_a`, `tilt_b`,
`pan_right_grows`, `tilt_up_grows` and optional `display_delay_ms`. Each model is `a*d + b*d^3`
in raw steps, where offsets are frame widths. The direction fields are booleans describing increasing
counters with logical Mirror/Flip off. Coefficients must be finite, bounded and strictly increasing
on the image domain. See `Calibration` in `src/pointing.rs` for the exact schema and validation.

Measure your own unit using recorded real frames and same-direction motor movements, fit pan and tilt
separately, and validate against independent image pairs before enabling this feature. Do not infer
coefficients from the Reolink lens or reuse household recordings as public fixtures. Display delay
also depends on the viewing client. Open-loop pointing is approximate, not angular/encoder feedback.

## Audio and recovery limits

Microphone capture and negotiated PCMU talkback transport have been exercised on the qualified
hardware. Driver delivery does not establish speaker loudness or acoustic echo cancellation; those
remain attended acceptance checks. Optional SDK AEC interfaces do not imply production acoustic
qualification. Host tests never open a microphone or produce sound.

The camera has a single shared boot environment. A userspace boot-attempt budget can recover from a
daemon failure only while the kernel and SSH/network recovery can boot. It cannot repair a corrupt
boot environment or guarantee survival of an interrupted flash write. Keep the original full dump
and established external-programmer access.
