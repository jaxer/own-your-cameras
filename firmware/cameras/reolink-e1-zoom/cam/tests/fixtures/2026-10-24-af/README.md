# Autofocus recordings

VA (the ISP's sharpness statistic, ISP item 30, the sum of 256 cells) recorded on one E1 Zoom running this daemon
with its production pipeline, through the `autofocus_state` sensor's MQTT attributes: `samples` is every
(focus, VA) one search measured in order, `history` the watchdog's once-a-second VA. Indoor room, evening, room lights
on; the view is an indoor wall and curtain about three metres away. `cam/tests/af_replay.rs` replays them.

| File | What |
|---|---|
| `sweep-day-zoom0.json` | one search after a preset, zoom 0, day mode |
| `sweep-day-zoom0-refocus.json` | a Refocus press at the same view whose single best sample was 194 |
| `sweep-night-zoom0.json` | the search after switching to night mode (IR-cut filter out, floodlight on) |
| `sweep-day-zoom512.json`, `sweep-day-zoom1024.json` | zoom 512 and 1024: VA flat across the whole driver window (the scene is nearer than the window reaches), so the search falls back to the calibrated focus |
| `refocus-ten.json` | ten Refocus presses in a row at the zoom-0 view |
| `idle-still.json` | 60 s of watchdog VA, nobody in the room |
| `idle-flicker.json` | 60 s of watchdog VA in night mode with the IR floodlight switched off and on every 3 s |

`window` is the driver's focus window at that zoom (`MTR_GET_FOCUS_RANGE`).
