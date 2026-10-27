# ISP picture controls

Image-quality and exposure items set through `/dev/nvt_isp`, exposed as settings
(`cam::isp_iq::ITEMS`, [../settings.md](../settings.md)). Tags as in [README.md](README.md#evidence-tags). Applied on the hardware thread through the one
`/dev/nvt_isp` owner (the lens's `NvtIsp`), after orientation at startup and on every change.

## Transport and safety

- `vendor_isp_{get,set}_iq` = ioctl type `'q'` (get `0xc00471nn`, set `0x400471nn`), `vendor_isp_*_ae` =
  type `'e'` (`0xc00465nn` / `0x400465nn`; stock `vendor_isp_set_ae` @ 0x154a10, get @ 0x153e80). The ioctl
  size field encodes a pointer, not the payload.
- Payload `{u32 isp id = 0, u32 value}`, **8 bytes both directions**, from the kernel's own size tables
  [D]: `nvt_iq.ko` (SHA256 `c435ceda…d25e`) `.data + 0x5a24 + item*8` = `{item, 8}`
  for items 10–15 and 20; `nvt_ae.ko` (`281a0f42…`) `.data + 0x2de8 + item*8` = `{item, 8}` for 12 and 14.
  Every item and direction is in the `NvtIsp` allowlist with that size ([../hdal-abi.md](../hdal-abi.md#isp-payloads)).
- **The kernel set path ignores a rejected value and still returns 0** (`iqt_api_set_cmd` @ .text
  0x1bcdc: the `blx` is followed by an unconditional return of 0; `aet_api_set_cmd` @ 0x14df4). Measured: every out-of-range set below
  left the readback at the previous value. So every set is followed by a get, and a mismatch is an error
  (the previous value stays, the `settings` sensor reports it).
- Accepted ranges are the kernel UI tables (`iq_ui_set_info` @ .text 0x21334, `.data + 0x10a88`, `max`
  exclusive; `ae_ui_set_info` @ 0x16970).

## Items

Stock evidence: `device` 4417 message handlers (VMA) that read the `0_isp.cfg` 0..255 values, scale and
issue the ioctl; the "apply all" path 0xc9bf0 re-applies them at start and day/night switches.
Live [V]: on `cam` with frames flowing (sub-stream JPEGs, the RAM probe `cam/examples/probe90.rs`).
Mean luma, luma spread (std), mean HSV saturation, edge energy (mean |∇Y|), per-channel means.

| Key | Item | Range | Stock (handler) | First-run default | Live: readback and frame effect |
|---|---|---|---|---|---|
| `isp_brightness` | IQ 14 | 0–200 | `brightness`, curve base 116 (0xc5cfc) | 90 | read 90 at start; 200 → luma 217, 0 → luma 8.7 (from 95); 201 refused |
| `isp_contrast` | IQ 13 | 0–200 | `contrast`, base 129 (0xc5dc0) | 101 | 200 → spread 75.9, 0 → spread 0.2 (from 53.3) |
| `isp_saturation` | IQ 12 | 0–200 | `saturation`, base 128 (0xc5f64; stock forces 0 at night) | 100 | 200 → saturation 0.53, 0 → 0.00 (from 0.33), luma unchanged |
| `isp_sharpness` | IQ 11 | 0–200 | `sharpen`, base 77 (0xc6034) | 60 | 200 → edge 12.5, 0 → 5.6 (from 8.6) |
| `isp_hue` | IQ 20 | 0–360° | `hue`, 128 = 0° (0xc5e84) | 0 | 180 → R/G/B 78.6/83.7/81.3 vs 80.9/82.1/82.2; 361 refused |
| `isp_noise_reduction` | IQ 10 (2D NR level) | 0–200 | kernel UI only (stock never sets it) | 100 | 0 → edge 10.9, 200 → edge 5.8 (from 8.7) |
| `isp_exposure` | AE 14 (EV) | 0–32, 16 = 0 EV | kernel UI only | 16 | 32 → luma 143, 0 → luma 4.0 (from 84) |
| `isp_anti_flicker` | AE 12 | 50 Hz / 60 Hz | `antiflick`, 1 = 60 Hz (0xc6680) | 50 Hz | readback 1/0; exposure re-converges (luma 72 → 80); 3 refused |

`0_isp.cfg` on the tested unit: brightness/contrast/saturation/sharpen/hue 128, antiflick 3 → levels
90/101/100/60, hue 0°, 50 Hz, through the per-sensor curve (table @ .data 0x356f30) with this unit's
bases fitted from the capture below.

## Not exposed, with reasons

| Item | Why |
|---|---|
| IQ 22 3D NR level (0–200, 8 B) | accepted and reads back, but no measurable effect on still frames (edge/luma within noise at 0 and 200); temporal NR needs a motion test — not shown to take effect |
| IQ 40 3D NR param (8040 B), IQ 45 WDR/DRC (744 B), AE 32 target Y (180 B), AE 47/48 exposure/gain bounds (12 B) | only a few fields of large structs are evidenced from stock writes (read-modify-write), field meanings unverified; IQ 40 exceeds the 4 KiB buffer. Not written without a pinned layout |
| AWB 11 R/B ratio (12 B) | needs AWB manual mode; stock never uses it |
| IQ 15 night/B&W | owned by the IR day/night switch ([ir.md](ir.md)) |

## Stock capture [V]

On the stock stack, a RAM-only ptrace observer printed every `/dev/nvt_isp` ioctl (item, direction, first
payload words; it never writes to the tracee) while the stock HTTPS API (`SetImage`) set each control to 200, 30
and back to 128. On every
change stock re-applied all five levels, each as one 8-byte set `{0, level}`:

| UI value | brightness IQ 14 | contrast IQ 13 | saturation IQ 12 | sharpness IQ 11 | hue IQ 20 |
|---|---|---|---|---|---|
| 128 (default) | 90 | 101 | 100 | 60 | 0 |
| 200 | 152 | 156 | 156 | — | — |
| 30 | 21 | 23 | — | — | — |

This confirms the items, the payload and the per-sensor curve on the message path; its bases for this
unit fit exactly: brightness 116, contrast 129, saturation 128, sharpness 77
(`v = ui ≤ 127 ? ui·base/128 : base + (256−base)·(ui−128)/128`, then `v·200/255`). The first-run
defaults are what stock applies for the unit's `0_isp.cfg`: brightness 90, contrast 101, saturation
100, sharpness 60, hue 0° (our daemon also reads brightness 90 at start). The stock API's anti-flicker
change did not reach AE 12 in this capture (the anti-flicker evidence stays the handler @ 0xc6680 plus
the live readback). Failed logins count toward the stock web API's own lockout.
