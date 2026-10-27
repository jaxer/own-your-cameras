# Shared camera motion interface

Publish non-retained JSON to `camera/<id>/motion/set`. This is the preferred interface for
pan, tilt, zoom, image positioning, presets and homing. The parser and validation live in
[`motion_control.rs`](../../../crates/camera-control/src/motion_control.rs); other firmware adapters compile that
same module. Hardware scheduling, optical calibration and position storage stay in adapters.
Existing individual topics remain compatible.

Every request requires `v: 1` and an `op`. Unknown fields, duplicate fields, unsupported
versions and invalid values are refused. The maximum payload is 256 bytes.

## Continuous movement

```json
{"v":1,"op":"velocity","pan":-100,"tilt":100,"zoom":100}
```

Values are integer percentages from -100 to 100 of each axis's configured speed ceiling.
Positive pan is screen right, positive tilt is screen up, and positive zoom is in. The camera
applies the confirmed image orientation. An omitted axis is zero: each request replaces the
**whole desired vector**, rather than latching a partial axis update. No separate diagonal
commands exist. An all-zero vector is an ordinary release.

Send immediately on input changes and renew every 500 ms while held. The renewable lease
is two seconds from receipt; stale queued velocities do not start a new lease. Disconnect
cancels motion. The E1 adapter runs pan/tilt chunks independently and optical zoom alongside
them. An unchanged axis retains its trajectory while another joins or leaves. Hardware with
a shared axis clock may reject unequal nonzero percentages; unsupported axes are rejected
before any part of a vector executes.

```json
{"v":1,"op":"release"}
```

Release ends ordinary motion using the adapter's controlled stopping behavior. It differs
from urgent cancellation, which bypasses ordinary motion queues and ramps:

```json
{"v":1,"op":"stop"}
```

## Image positioning

A point centers the requested location without optical zoom:

```json
{"v":1,"op":"point","x":200,"y":150}
```

Coordinates are integers 0–255, **x from the left and y from the bottom**. A region preserves
the existing position3D contract, including reverse corner order for zoom-out:

```json
{"v":1,"op":"position3d","start_x":160,"start_y":90,"end_x":200,"end_y":130,"operation":"zoomto"}
```

`operation` is `zoomto` (default) or `zoomout`. Equal corners mean a point. A fixed-lens adapter
can support points while refusing boxes and zoom-out. The camera owns FOV mapping, travel
bounds and compensation for the configured displayed-frame delay. These are image-space
requests, not metric 3D coordinates.

## Presets and reference

```json
{"v":1,"op":"save_preset","name":"pos1"}
{"v":1,"op":"preset","name":"pos1"}
{"v":1,"op":"home"}
```

A save captures the real settled pose, including zoom where supported; recall uses the
camera's persisted position. E1 preset names must already exist in the configured table.
Names have at most 64 UTF-8 bytes and no control characters. An adapter must establish a
physical reference before trusting presets across startup or external head movement.

The shared HA card opts in with `motion_topic`. A button supplies an axis vector such as
`motion: { pan: -100 }`; keys, simultaneous pointers and temporary zoom pulses compose into
one complete request. Native pointer input starts immediately and releases on pointer-up or
cancellation. Preset buttons recall on click and save on long press or Shift-click. Other
camera protocols keep their existing service adapters.
