//! RAM-only measurements on the camera (no flash, no `/mnt/para` writes): PTZ step-rate
//! repeatability, divisor changes during a move, lens speed and halt, ISP picture items. Run next to
//! `cam serve --diagnostic` without MQTT (so no other process owns the motors), from `/mnt/tmp`,
//! with the vendor stack absent; reboot afterwards. Each step prints `key=value` lines.
#[cfg(not(target_arch = "arm"))]
fn main() {}

#[cfg(target_arch = "arm")]
fn main() -> Result<(), String> {
    use cam::ptz::{Axis, Dir, Position, Profile, Ptz};
    use cam::zoom::{Lens, WORDS};
    use std::io::BufRead;
    use std::time::{Duration, Instant};
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let text = |e: &dyn std::fmt::Display| e.to_string();
    let wait_line = |what: &str| {
        println!("ready={what}");
        let mut l = String::new();
        let _ = std::io::stdin().lock().read_line(&mut l);
    };
    let ptz = || -> Result<Ptz<cam::ptz::i2c::I2cBus>, String> {
        let mut p =
            Ptz::new(cam::ptz::i2c::I2cBus::open(cam::ptz::i2c::DEVICE).map_err(|e| text(&e))?);
        p.init().map_err(|e| text(&e))?;
        let t = Instant::now();
        p.home().map_err(|e| text(&e))?;
        println!("home_ms={}", t.elapsed().as_millis());
        Ok(p)
    };
    let lens = || -> Result<Lens<cam::zoom::isp::NvtIsp>, String> {
        Ok(Lens::new(
            cam::zoom::isp::NvtIsp::open(cam::zoom::isp::DEVICE).map_err(|e| text(&e))?,
        ))
    };
    let pos1 = Position {
        pan: 3262,
        tilt: 739,
    };
    let pos2 = Position {
        pan: 6195,
        tilt: 650,
    };
    match words.as_slice() {
        // Repeatability: frame at pos2, `n` round trips pos1<->pos2 at `rate`, frame at pos2 again.
        ["trips", rate, n, shape @ ..] => {
            let rate: u32 = rate.parse().map_err(|e| text(&e))?;
            let n: u32 = n.parse().map_err(|e| text(&e))?;
            let (start, ramp) = match shape {
                [s, r] => (
                    s.parse().map_err(|e| text(&e))?,
                    r.parse().map_err(|e| text(&e))?,
                ),
                _ => (200, 800),
            };
            let mut p = ptz()?;
            p.goto(pos2).map_err(|e| text(&e))?;
            wait_line("before");
            p.set_profile(Profile::new(start, rate, Duration::from_millis(ramp)));
            let t = Instant::now();
            for _ in 0..n {
                p.goto(pos1).map_err(|e| text(&e))?;
                p.goto(pos2).map_err(|e| text(&e))?;
            }
            println!("trips_ms={}", t.elapsed().as_millis());
            wait_line("after");
        }
        // Hold-to-move exactly as the hardware thread drives it (poll at the requested interval), then
        // revisit the counted position with absolute moves so a frame can be compared.
        ["hold", rate, start, ramp, secs] => {
            use cam::ptz::{HoldState, Toward};
            let p_ = |x: &str| x.parse::<u32>().map_err(|e| text(&e));
            let profile = Profile::new(p_(start)?, p_(rate)?, Duration::from_millis(u64::from(p_(ramp)?)));
            let mut p = ptz()?;
            p.goto(Position { pan: 3000, tilt: 650 }).map_err(|e| text(&e))?;
            let t0 = Instant::now();
            p.begin_hold(Axis::Pan, Toward::Max, profile, Duration::from_secs(20), t0.elapsed())
                .map_err(|e| text(&e))?;
            let release = Duration::from_secs(u64::from(p_(secs)?));
            let mut polls = 0u32;
            loop {
                let now = t0.elapsed();
                if now >= release {
                    p.release_hold();
                }
                match p.poll_hold(now).map_err(|e| text(&e))? {
                    HoldState::Moving(d) => std::thread::sleep(d),
                    HoldState::Ended(e) => {
                        println!("hold_end={e:?} at_ms={}", now.as_millis());
                        break;
                    }
                    HoldState::Idle => break,
                }
                polls += 1;
            }
            let pos = p.position().ok_or("position lost")?;
            println!("hold_polls={polls} counted_pan={} moves={}", pos.pan, 0);
            wait_line("after");
            p.goto(Position { pan: 2000, tilt: 650 }).map_err(|e| text(&e))?;
            p.goto(pos).map_err(|e| text(&e))?;
            wait_line("revisit");
        }
        // Full pan travel time at `rate` (ramped), after a home.
        ["travel", rate] => {
            let rate: u32 = rate.parse().map_err(|e| text(&e))?;
            let mut p = ptz()?;
            p.set_profile(Profile::new(200, rate, Duration::from_millis(800)));
            let t = Instant::now();
            p.goto(Position { pan: cam::ptz::PAN_MAX, tilt: 0 }).map_err(|e| text(&e))?;
            println!("travel_ms={}", t.elapsed().as_millis());
            p.goto(Position { pan: 3577, tilt: 0 }).map_err(|e| text(&e))?;
        }
        // Does a divisor written mid-chunk take effect? 1200 steps at 600 pps = 2 s; switch to 1200 pps at 0.5 s.
        ["divisor"] => {
            let mut p = ptz()?;
            p.goto(Position { pan: 1000, tilt: 0 }).map_err(|e| text(&e))?;
            for change in [false, true] {
                p.set_pps(Axis::Pan, 600).map_err(|e| text(&e))?;
                let t = Instant::now();
                p.start_move(Axis::Pan, Dir::Zero, 1200).map_err(|e| text(&e))?;
                let mut changed = false;
                while p.is_busy(Axis::Pan).map_err(|e| text(&e))? {
                    if change && !changed && t.elapsed() >= Duration::from_millis(500) {
                        p.set_pps(Axis::Pan, 1200).map_err(|e| text(&e))?;
                        changed = true;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                println!("divisor_change={change} busy_ms={}", t.elapsed().as_millis());
                // back, at 600
                p.set_pps(Axis::Pan, 600).map_err(|e| text(&e))?;
                p.start_move(Axis::Pan, Dir::One, 1200).map_err(|e| text(&e))?;
                while p.is_busy(Axis::Pan).map_err(|e| text(&e))? {
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
            // Inter-chunk gap: idle seen -> next kick, as the hold loop does at a 2 ms poll.
            p.set_pps(Axis::Pan, 1200).map_err(|e| text(&e))?;
            let mut gaps = Vec::new();
            for _ in 0..10 {
                p.start_move(Axis::Pan, Dir::Zero, 120).map_err(|e| text(&e))?;
                while p.is_busy(Axis::Pan).map_err(|e| text(&e))? {
                    std::thread::sleep(Duration::from_millis(2));
                }
                let t = Instant::now();
                p.set_pps(Axis::Pan, 1200).map_err(|e| text(&e))?;
                p.start_move(Axis::Pan, Dir::One, 120).map_err(|e| text(&e))?;
                gaps.push(t.elapsed().as_micros());
                while p.is_busy(Axis::Pan).map_err(|e| text(&e))? {
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            println!("kick_us={gaps:?}");
        }
        // Lens: full-range zoom time at each speed, then a press move halted after 600 ms.
        ["lens", "speed"] => {
            let mut l = lens()?;
            l.init().map_err(|e| text(&e))?;
            for spd in cam::zoom::ZOOM_SPEEDS {
                l.set_zoom_speed(spd).map_err(|e| text(&e))?;
                let t = Instant::now();
                l.zoom_to(1024).map_err(|e| text(&e))?;
                let up = t.elapsed().as_millis();
                let t = Instant::now();
                l.zoom_to(0).map_err(|e| text(&e))?;
                println!("zoom_speed={spd} up_ms={up} down_ms={}", t.elapsed().as_millis());
            }
            l.set_zoom_speed(80).map_err(|e| text(&e))?;
        }
        ["lens", "halt"] => {
            let mut l = lens()?;
            l.init().map_err(|e| text(&e))?;
            for (toward_max, hold_ms) in [(true, 600), (true, 300), (false, 400)] {
                let before = l.positions().map_err(|e| text(&e))?.zoom;
                let t = Instant::now();
                l.zoom_press(toward_max).map_err(|e| text(&e))?;
                std::thread::sleep(Duration::from_millis(hold_ms));
                l.zoom_release().map_err(|e| text(&e))?;
                let mut polls = 0;
                while l.busy().map_err(|e| text(&e))? && polls < 1000 {
                    polls += 1;
                    std::thread::sleep(Duration::from_millis(5));
                }
                let after = l.positions().map_err(|e| text(&e))?.zoom;
                println!(
                    "press toward_max={toward_max} hold_ms={hold_ms} from={before} to={after} stopped_ms={}",
                    t.elapsed().as_millis()
                );
                std::thread::sleep(Duration::from_millis(500));
            }
            l.zoom_to(0).map_err(|e| text(&e))?;
        }
        // ISP: `isp iq|ae ITEM` reads; `isp iq|ae ITEM VALUE` sets and reads back.
        ["isp", unit, item, rest @ ..] => {
            let item: u32 = item.parse().map_err(|e| text(&e))?;
            let mut isp = cam::zoom::isp::NvtIsp::open(cam::zoom::isp::DEVICE).map_err(|e| text(&e))?;
            let mut w = Box::new([0u32; WORDS]);
            if let [value] = rest {
                w[1] = value.parse().map_err(|e| text(&e))?;
                match *unit {
                    "iq" => isp.set_iq(item, &w),
                    _ => isp.set_ae(item, &w),
                }
                .map_err(|e| text(&e))?;
            }
            let mut back = Box::new([0u32; WORDS]);
            match *unit {
                "iq" => isp.get_iq(item, &mut back),
                _ => isp.get_ae(item, &mut back),
            }
            .map_err(|e| text(&e))?;
            println!("isp {unit} {item} = {}", back[1]);
        }
        _ => return Err("usage: probe90 trips RATE N [START RAMP_MS] | hold RATE START RAMP SECS | travel RATE | divisor | lens speed|halt | isp iq|ae ITEM [VALUE]".into()),
    }
    Ok(())
}
