# Camera motion contract

One parser serves both cameras. It has no hardware access, network connection or clock.
Commands are non-retained MQTT publications to `camera/<id>/motion/set`, at most 256 bytes.

```json
{"v":1,"op":"velocity","pan":100,"tilt":0,"zoom":0}
```

Positive percentages mean screen right, screen up and zoom in after confirmed image orientation.
The vector replaces the complete previous vector. Omitted axes are zero. Renew a hold every 500 ms;
its lease expires two seconds after arrival. Release requests normal braking; Stop cancels urgently.
A stale/queued request cannot acquire a new lease merely because it was executed later.
See `motion_control.rs` for the exact accepted operation names and preset/image schemas.

Mi360 supports pan/tilt but no optical zoom; its shared motor timer refuses unequal nonzero axis
magnitudes. Reolink supports independent pan/tilt/zoom. Unsupported capability combinations are
rejected as a whole, rather than silently moving a subset of the requested axes.

Image coordinates use 0–255, x from the left and y from the bottom. A point has equal start/end
coordinates. Reolink also supports boxes and zoom-out; Mi360 accepts calibrated points only.
Each camera owns optics, physical bounds, reference/homing and compensation for displayed-image age.
