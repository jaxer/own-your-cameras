# Position3d: tap-to-centre and box zoom

Hikvision-style 3D positioning: a tap on the picture centres that point, a box is centred and zoomed to fill the
frame. `cam::position3d` is the pure map; the MQTT hardware owner moves only from a confirmed position, like every
other absolute move. Tags as in [README.md](README.md#evidence-tags).

## Command

`camera/<id>/position3d/set`, non-retained JSON, the payload a `position3d_service` of
[master-camera-card](../home-assistant.md#a-dashboard-card) sends:

```json
{"start_x": 66, "start_y": 189, "end_x": 66, "end_y": 189, "operation": "zoomto"}
```

- Coordinates are integers 0..=255 of the displayed picture (after `mirror`/`flip`), x from the left,
  **y from the bottom** (Hikvision's convention).
- `start == end`: centre that point, zoom unchanged.
- A box: centre its middle; `zoomto` zooms so the box's larger share of the frame fills it, `zoomout` so
  the current frame shrinks into the box. Zoom clamps to 0..=1024.
- Pan/tilt clamp to physical travel. An unknown position (after a stop) refuses, as for relative moves.
  Unknown fields, a missing coordinate, values outside 0..=255 and other operations are refused, and so are
  retained messages. There is no Home Assistant entity for it.

## The map [V]

The E1 Zoom lens is strongly barrel-distorted at wide, so a pinhole focal length does not fit (feature
shifts fall from 143 px at the centre to 125 px at the edge for the same 150-step pan). The map is
measured directly in motor steps: a feature `x` frame widths right of the centre is

    pan steps = a·x + b·x³          tilt steps = 0.91 · (a·y + b·y³)

with `y` the upward offset in frame widths (the frame is 4:3), and `(a, b)` per zoom, linear between rows:

| zoom | a (steps / frame width) | b | edge (x = 0.5) |
|---:|---:|---:|---:|
| 0 | 1960.7 | 553.4 | 1050 |
| 256 | 1803.0 | 385.2 | 950 |
| 512 | 1658.4 | 217.7 | 856 |
| 768 | 1547.6 | 127.3 | 790 |
| 1024 | 1462.6 | 149.9 | 750 |

Pan grows toward the right of the picture and tilt toward the top under stock orientation; `mirror`
reverses pan and `flip` reverses tilt. The whole optical zoom is only ≈1.34× at the centre (a 1960 → 1463),
and it straightens the lens (b falls), so box zoom saturates quickly.

How: `scripts/position3d/shoot.sh` moved the installed daemon over MQTT (pan sweeps of 100–800 steps at
zoom 0/256/512/768/1024, a tilt sweep of 100–400 steps at zoom 0) and grabbed go2rtc frames;
`fit_lens.py` template-matches 140 px patches across each frame pair and fits the local
steps-per-frame-width `a + 3b·x²` (blur tolerant; ORB matching on the sharp zoom-0 frames gave the same
edge, 1055 vs 1050 steps). The tilt ratio comes from centring real features: the sweep-only fit
(1.10) overshot every tilt by ≈15 %, and 0.91 lands them. The calibration frames are not published.

Pre-flash check (`validate.py`, the table applied off-camera, absolute moves): real features centred
within 1.6 % of the frame width and 2 % of its height at zoom 0 and zoom 512; matches below 0.85 are
reported as unreliable (repetitive floor, blur).

## Known limits

- Above zoom 0 the picture is soft on this unit at room distances (the zoom follows the factory ZFTable
  focus, which is not sharp here); it limits template matching, not the map.
- Tilt shares the pan map's shape (one radial lens) scaled by 0.91; residual ≈ 1–2 % of the frame.
- The map ignores pan/tilt coupling at steep tilt and the optical centre's offset from the frame centre.

## Checking it on a camera

`scripts/position3d/validate.py` taps real features and measures where they land. It drives the installed daemon
over MQTT and grabs frames through go2rtc (`scripts/position3d/cam.sh` lists its environment: `CAM`, `MQTT_HOST`,
`MQTT_USER`, `GO2RTC`, and the broker password from `cam.toml` over SSH):

```sh
CAM=e1zoom MQTT_HOST=broker.example MQTT_USER=USER GO2RTC=GO2RTC_URL \
  python3 scripts/position3d/validate.py --mqtt 5005 488 0  1660 570  1105 520
```

The first three numbers are the pose (pan, tilt, zoom), then pixel pairs of a 1920×1440 frame at that pose.
`--mqtt` publishes the card's payload and lets the daemon compute the move; every reliable match should land
within a few percent of the centre. On the tested unit taps landed 0.2–1.1 % of the frame from the centre, and a
box `zoomto` reached zoom 1024 (the lens limit).

## Taps while the head moves

A tap is made on the picture the dashboard showed, which runs behind the head by the stream's delay and, while a
move runs, behind the move itself. The daemon therefore stamps every command when it arrives (the hardware thread's
clock) and resolves a `position3d` request against the pose at `arrival − tap_delay`: the pan/tilt from the
recorded motion (every goto's chunk schedule and every hold chunk, kept for 10 s, linear inside a chunk) and the
zoom from the recorded zoom moves (linear at the speed setting's measured rate). Before the first recorded move,
or after a stop (the position is unknown), the confirmed position is used, as before. The `tap_delay` setting
(0…3000 ms, Pan & tilt) defaults to the delay measured on the camera's go2rtc stream (the dashboard's high stream pulled as MP4, frame differencing against a pan command published over MQTT, ten trials): median 519 ms, 500…701 ms, so 500 ms; the dashboard's WebRTC view adds its own rendering on top, so a slightly larger value may suit a slow client. A tap that is
still queued when a newer tap arrives is dropped, since each tap's target is absolute from its own picture.
Relative steps, absolute numbers, presets and holds resolve from the confirmed position, unchanged.
