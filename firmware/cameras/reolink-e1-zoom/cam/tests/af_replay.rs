//! The autofocus search and watchdog replayed against VA recorded on a camera (`fixtures/2026-10-24-af/README.md`).

use cam::zoom::Lens;
use cam::zoom::af::{AfConfig, AfState, Autofocus, Trigger};
use cam::zoom::fake::FakeIsp;
use cam::zoom::zf_table::ZfTable;
use serde_json::Value;
use std::time::Duration;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/2026-10-24-af");
const WIDE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/2026-10-25-af-wide"
);

fn fixture(name: &str) -> Value {
    let text = std::fs::read_to_string(format!("{DIR}/{name}.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

fn wide(name: &str) -> Value {
    let text = std::fs::read_to_string(format!("{WIDE}/{name}.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The camera's calibration ends: (0, 230) … (1024, 1022).
fn zf() -> ZfTable {
    ZfTable::parse(b"<ZFTable>1000000,300e6,200,320,400,3fe</ZFTable>").unwrap()
}

/// A lens whose VA follows a recorded sweep (repeated positions averaged).
fn replay_lens(sweep: &Value) -> Lens<FakeIsp> {
    let mut by_focus = std::collections::BTreeMap::<u32, (u64, u64)>::new();
    for s in sweep["samples"].as_array().unwrap() {
        let e = by_focus.entry(s[0].as_u64().unwrap() as u32).or_default();
        e.0 += s[1].as_u64().unwrap();
        e.1 += 1;
    }
    let window = &sweep["window"];
    let zoom = sweep["zoom"].as_u64().unwrap() as u32;
    let mut isp = FakeIsp::new(zoom, zf().focus_at(zoom));
    isp.focus_window = (
        window[0].as_u64().unwrap() as u32,
        window[1].as_u64().unwrap() as u32,
    );
    isp.curve = Some(by_focus.into_iter().map(|(f, (v, n))| (f, v / n)).collect());
    Lens::new(isp)
}

fn search(af: &mut Autofocus, lens: &mut Lens<FakeIsp>, now: Duration) -> Duration {
    run(af, lens, Trigger::Button, now)
}

/// The calibrated focus as the park point.
fn park(zoom: u32) -> u32 {
    zf().focus_at(zoom)
}

fn run(
    af: &mut Autofocus,
    lens: &mut Lens<FakeIsp>,
    trigger: Trigger,
    mut now: Duration,
) -> Duration {
    af.request(trigger, now);
    while af.searching() {
        af.poll(lens, &zf(), &park, now, true).unwrap();
        now += Duration::from_millis(10);
    }
    now
}

/// Focus positions whose recorded VA is within 1 % of the recording's best: the plateau a sharp search lands on.
fn plateau(sweep: &Value) -> (u32, u32) {
    let samples: Vec<(u32, u64)> = sweep["samples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s[0].as_u64().unwrap() as u32, s[1].as_u64().unwrap()))
        .collect();
    let top = samples.iter().map(|s| s.1).max().unwrap();
    let on: Vec<u32> = samples
        .iter()
        .filter(|s| s.1 * 100 >= top * 99)
        .map(|s| s.0)
        .collect();
    (*on.iter().min().unwrap(), *on.iter().max().unwrap())
}

/// The recorded sample with the highest VA.
fn recorded_best(sweep: &Value) -> u32 {
    sweep["samples"]
        .as_array()
        .unwrap()
        .iter()
        .max_by_key(|s| s[1].as_u64().unwrap())
        .unwrap()[0]
        .as_u64()
        .unwrap() as u32
}

/// Every zoom-0 recording, climbed from the calibrated park point and from its recorded peak, lands inside the
/// recorded plateau ±1 fine step within the move budget; the flat zoom 512 and 1024 scenes still end calibrated.
#[test]
fn recorded_sweeps_land_on_the_recorded_peak_or_fall_back_when_flat() {
    for (name, sharp) in [
        ("sweep-day-zoom0", true),
        ("sweep-day-zoom0-refocus", true),
        ("sweep-night-zoom0", true),
        ("sweep-day-zoom512", false),
        ("sweep-day-zoom1024", false),
    ] {
        let sweep = fixture(name);
        let park = zf().focus_at(sweep["zoom"].as_u64().unwrap() as u32);
        for (start, budget) in [(park, 14), (recorded_best(&sweep), 10)] {
            let mut lens = replay_lens(&sweep);
            lens.isp_mut().focus = start;
            let mut af = Autofocus::new(AfConfig::default());
            search(&mut af, &mut lens, Duration::ZERO);
            let fine = if sweep["zoom"].as_u64().unwrap() >= 682 {
                4
            } else {
                6
            };
            let moves = lens.isp().focus_moves().len();
            match (af.state(), sharp) {
                (AfState::Focused(f), true) => {
                    let (lo, hi) = plateau(&sweep);
                    assert!(
                        (lo.saturating_sub(fine)..=hi + fine).contains(f),
                        "{name} from {start}: {f} outside the recorded peak {lo}..={hi}"
                    );
                    assert!(moves <= budget, "{name} from {start}: {moves} moves");
                }
                (AfState::Calibrated(_), false) => {}
                (s, _) => panic!("{name} from {start}: {s:?}"),
            }
            eprintln!("{name} from {start}: {} after {moves} moves", af.state());
        }
    }
}

/// A pan/tilt over the recorded zoom-0 scene: the focus is kept while VA holds 90 % of the reference, and a
/// search runs at 70 %.
#[test]
fn a_recorded_move_keeps_the_focus_while_sharp_and_searches_when_soft() {
    for (scale, kept) in [(90, true), (70, false)] {
        let sweep = fixture("sweep-day-zoom0");
        let mut lens = replay_lens(&sweep);
        let mut af = Autofocus::new(AfConfig::default());
        let end = search(&mut af, &mut lens, Duration::ZERO);
        let focus = match af.state() {
            AfState::Focused(f) => *f,
            s => panic!("{s:?}"),
        };
        lens.isp_mut().clear_sets();
        lens.isp_mut().scale_pct = scale;
        run(
            &mut af,
            &mut lens,
            Trigger::Move,
            end + Duration::from_secs(1),
        );
        let moves = lens.isp().focus_moves();
        if kept {
            assert_eq!(af.state(), &AfState::Kept(focus));
            assert!(moves.is_empty(), "{moves:?}");
        } else {
            assert!(
                matches!(af.state(), AfState::Focused(_)),
                "{:?}",
                af.state()
            );
            assert!(!moves.is_empty());
        }
    }
}

/// Runs the watchdog over a recorded once-a-second idle trace after a search whose reference is twice the trace's
/// level, so every sample is soft; returns the second it asked for a search, if it did.
fn watch_trace(name: &str) -> Option<usize> {
    let sweep = fixture("sweep-day-zoom0");
    let mut lens = replay_lens(&sweep);
    let mut af = Autofocus::new(AfConfig::default());
    let trace: Vec<u64> = fixture(name)["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let mean = trace.iter().sum::<u64>() / trace.len() as u64;
    let peak = sweep["samples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s[1].as_u64().unwrap())
        .max()
        .unwrap();
    lens.isp_mut().scale_pct = 200 * mean / peak;
    let end = search(&mut af, &mut lens, Duration::ZERO);
    assert!(af.report().reference.unwrap() > mean * 19 / 10);
    for (t, va) in trace.iter().enumerate() {
        lens.isp_mut().va_script = [*va].into();
        af.poll(
            &mut lens,
            &zf(),
            &park,
            end + Duration::from_secs(1 + t as u64),
            true,
        )
        .unwrap();
        if af.searching() {
            return Some(t);
        }
    }
    None
}

#[test]
fn a_still_recorded_room_triggers_the_watchdog_after_the_hold() {
    let fired = watch_trace("idle-still").expect("a still soft scene refocuses");
    assert!((30..=31).contains(&fired), "fired at {fired} s");
}

#[test]
fn a_recorded_flickering_light_never_triggers_the_watchdog() {
    assert_eq!(watch_trace("idle-flicker"), None);
}

#[test]
fn ten_recorded_refocus_presses_land_within_one_fine_step() {
    let runs = fixture("refocus-ten");
    let mut ends = Vec::new();
    for samples in runs["runs"].as_array().unwrap() {
        let sweep = serde_json::json!({"zoom": 0, "window": [86, 330], "samples": samples});
        let mut lens = replay_lens(&sweep);
        let mut af = Autofocus::new(AfConfig::default());
        search(&mut af, &mut lens, Duration::ZERO);
        match af.state() {
            AfState::Focused(f) => ends.push(*f),
            s => panic!("{s:?}"),
        }
    }
    let (lo, hi) = (ends.iter().min().unwrap(), ends.iter().max().unwrap());
    assert!(hi - lo <= 6, "{ends:?}");
}

/// The room view recorded with the wide driver windows at zoom 256..1024: the sharp focus lies outside the
/// factory ±64 window at 512 and above, and a search over the wide window ends `focused` on the recorded peak.
#[test]
fn wide_window_recordings_focus_beyond_the_factory_window() {
    for (name, factory_hi) in [
        ("sweep-pos2-zoom256", 660),
        ("sweep-pos2-zoom512", 901),
        ("sweep-pos2-zoom768", 1030),
        ("sweep-pos2-zoom1024", 1086),
    ] {
        let sweep = wide(name);
        let mut lens = replay_lens(&sweep);
        lens.isp_mut().focus = zf().focus_at(sweep["zoom"].as_u64().unwrap() as u32);
        let mut af = Autofocus::new(AfConfig::default());
        search(&mut af, &mut lens, Duration::ZERO);
        let (lo, hi) = plateau(&sweep);
        match af.state() {
            AfState::Focused(f) => {
                assert!(
                    (lo.saturating_sub(6)..=hi + 6).contains(f),
                    "{name}: {f} outside the recorded peak {lo}..={hi}"
                );
                if name != "sweep-pos2-zoom256" {
                    assert!(*f > factory_hi, "{name}: {f} is inside the factory window");
                }
            }
            s => panic!("{name}: {s:?}"),
        }
        eprintln!(
            "{name}: {} after {} moves",
            af.state(),
            lens.isp().focus_moves().len()
        );
    }
}

/// A search from a learned park point (the previous peak) on the zoom-0 recording takes at most 8 focus moves
/// (12 from the calibrated park).
#[test]
fn a_learned_park_point_makes_the_next_search_short() {
    use cam::zoom::focus_map::{FocusMap, Pose};
    let sweep = fixture("sweep-day-zoom0");
    let mut lens = replay_lens(&sweep);
    let mut af = Autofocus::new(AfConfig::default());
    search(&mut af, &mut lens, Duration::ZERO);
    let AfState::Focused(found) = *af.state() else {
        panic!("{:?}", af.state())
    };
    let mut map = FocusMap::new(Box::new(cam::settings::FakeDisk::default()));
    map.learn(
        Pose {
            pan: 6195,
            tilt: 650,
            zoom: 0,
        },
        found,
        1,
        Duration::ZERO,
    );
    // a later zoom back to 0 parks where the map says, not at the calibrated 230
    let park = map.park(Some((6195, 650)), 0, &zf());
    assert_eq!(park.focus, found);
    let mut lens = replay_lens(&sweep);
    lens.isp_mut().focus = park.focus;
    let mut af = Autofocus::new(AfConfig::default());
    af.request(Trigger::Zoom, Duration::ZERO);
    let mut now = Duration::ZERO;
    while af.searching() {
        af.poll(
            &mut lens,
            &zf(),
            &|z| map.park(Some((6195, 650)), z, &zf()).focus,
            now,
            true,
        )
        .unwrap();
        now += Duration::from_millis(10);
    }
    let moves = lens.isp().focus_moves().len();
    assert!(
        matches!(af.state(), AfState::Focused(_)),
        "{:?}",
        af.state()
    );
    // on this flat plateau the bracket needs two probes a side: 2 + 2 + fine 2 + settle 2
    assert!(moves <= 8, "{moves} moves from the learned park point");
}
