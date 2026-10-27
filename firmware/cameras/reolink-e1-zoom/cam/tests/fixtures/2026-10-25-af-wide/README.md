# Autofocus recordings with wide driver windows

VA (the ISP's sharpness statistic, ISP item 30, the sum of 256 cells) recorded on one E1 Zoom through the diagnostic
pipeline (`cam serve --diagnostic` without MQTT, then `cam zoom rows 400 200`, `cam zoom <z>` and `cam focus` from
the command line), so the lens could reach focus positions the factory rows refuse. Indoor room, afternoon, lights
on. `cam/tests/af_replay.rs` replays them.

| Files | View |
|---|---|
| `sweep-pos2-zoom*.json` | a room about 3 m deep (door, plant, wall switches): contrast at every zoom |
| `sweep-pos1-zoom*.json` | a plain white wall closer than 0.5 m: almost no contrast, VA under 1 M |

`window` is the driver's focus window at that zoom after the rows were widened to −400/+200 around each calibrated
point (`MTR_GET_FOCUS_RANGE`); `focus` is where the search settled; `samples` is every (focus, VA) it measured,
in order. The absolute VA scale differs from the 2026-10-24 recordings (a different ISP tuning in the diagnostic
pipeline); only the shape matters.
