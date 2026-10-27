//! Speed profile of one pan or tilt leg: the pure planner that turns "how long have we been moving" and
//! "how far may we still go" into the next chunk's rate and length. No bus here, so the ramp is tested alone.

use std::time::Duration;

use super::{DEFAULT_PPS, MAX_CHUNK};

/// Lowest rate a profile may use; slower than this the divisor no longer fits its 14 bits.
pub const MIN_PPS: u32 = 100;
/// Travel one chunk carries while ramping or holding: short enough that a release stops within one chunk.
pub const CHUNK: Duration = Duration::from_millis(100);

/// Acceleration profile: start at `start_pps`, reach `max_pps` after `ramp`, and slow down symmetrically
/// before a known end so the motor never has to stop dead from full speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub start_pps: u32,
    pub max_pps: u32,
    pub ramp: Duration,
}

impl Profile {
    /// Homing drives into the mechanical end stop, so it always runs at the vendor rate with no ramp.
    pub const fn homing() -> Self {
        Self {
            start_pps: DEFAULT_PPS,
            max_pps: DEFAULT_PPS,
            ramp: Duration::ZERO,
        }
    }

    /// Clamps the rates into `MIN_PPS..=max` with `start <= max`.
    pub fn new(start_pps: u32, max_pps: u32, ramp: Duration) -> Self {
        let max_pps = max_pps.max(MIN_PPS);
        Self {
            start_pps: start_pps.clamp(MIN_PPS, max_pps),
            max_pps,
            ramp,
        }
    }

    /// Rate reachable `elapsed` after the start of the leg.
    fn ramp_up(&self, elapsed: Duration) -> u32 {
        let span = self.max_pps - self.start_pps;
        if span == 0 || elapsed >= self.ramp {
            return self.max_pps;
        }
        let gained = u128::from(span) * elapsed.as_micros() / self.ramp.as_micros();
        self.start_pps + gained as u32
    }

    /// Highest rate from which the ramp still gets down to `start_pps` within `remaining` steps
    /// (constant deceleration: v² = start² + 2·a·d with a = span / ramp).
    fn ramp_down(&self, remaining: u32) -> u32 {
        let span = self.max_pps - self.start_pps;
        if span == 0 || self.ramp.is_zero() {
            return self.max_pps;
        }
        let accel = f64::from(span) / self.ramp.as_secs_f64();
        let start = f64::from(self.start_pps);
        let v = (start * start + 2.0 * accel * f64::from(remaining)).sqrt();
        (v as u32).clamp(self.start_pps, self.max_pps)
    }

    /// Steps the deceleration from `pps` down to `start_pps` takes.
    fn braking(&self, pps: u32) -> u32 {
        let span = self.max_pps - self.start_pps;
        if span == 0 || self.ramp.is_zero() || pps <= self.start_pps {
            return 0;
        }
        let accel = f64::from(span) / self.ramp.as_secs_f64();
        let (v, s) = (f64::from(pps), f64::from(self.start_pps));
        ((v * v - s * s) / (2.0 * accel)).ceil() as u32
    }

    /// The next chunk of a leg: its rate and step count. `remaining` is how far the axis may still go
    /// (to the target, or to the travel limit while holding); it is never exceeded. While accelerating or
    /// braking a chunk carries [`CHUNK`] of travel; at cruise it runs on until braking must begin, unless
    /// `bounded` asks for [`CHUNK`]-sized chunks throughout (hold-to-move). Zero steps means arrived.
    pub fn next_chunk(&self, elapsed: Duration, remaining: u32, bounded: bool) -> (u32, u32) {
        if remaining == 0 {
            return (self.start_pps, 0);
        }
        let pps = self.ramp_up(elapsed).min(self.ramp_down(remaining));
        let short = (u64::from(pps) * CHUNK.as_millis() as u64 / 1000).max(1) as u32;
        let cruising = pps == self.max_pps && !bounded;
        let steps = if cruising {
            remaining.saturating_sub(self.braking(pps)).max(short)
        } else {
            short
        };
        (pps, steps.min(remaining).min(MAX_CHUNK))
    }

    /// Upper bound of the time a leg of `steps` takes, for timeouts (every chunk at the slowest rate).
    pub fn worst_case(&self, steps: u32) -> Duration {
        Duration::from_millis(u64::from(steps) * 1000 / u64::from(self.start_pps.max(1)))
    }
}

/// Travel time of `steps` at `pps`.
pub fn travel(steps: u32, pps: u32) -> Duration {
    Duration::from_micros(u64::from(steps) * 1_000_000 / u64::from(pps.max(1)))
}

/// One kicked chunk of an axis: `steps` from `begin` to `end`, up or down the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    pub begin: Duration,
    pub end: Duration,
    pub steps: u32,
    pub toward_max: bool,
}

/// Where the head was at any time during one move: the chunk schedule a goto issues (deterministic from the
/// profile) or one hold chunk, with the position linear inside a chunk. Pure, so a tap made on a picture from
/// mid-move resolves against the pose the picture showed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trajectory {
    pub start: Duration,
    pub end: Duration,
    pub from: (u32, u32),
    pub to: (u32, u32),
    /// Per axis (pan, tilt), in order.
    pub legs: [Vec<Chunk>; 2],
}

impl Trajectory {
    /// The schedule a goto from `from` to `to` under `profile` issues from `start`: each axis follows the
    /// profile on its own, one chunk after another, as `run_plan` kicks them.
    pub fn plan(start: Duration, from: (u32, u32), to: (u32, u32), profile: &Profile) -> Self {
        let mut legs = [Vec::new(), Vec::new()];
        let mut end = start;
        for (i, (f, t)) in [(from.0, to.0), (from.1, to.1)].into_iter().enumerate() {
            let toward_max = t > f;
            let mut left = f.abs_diff(t);
            let mut at = start;
            while left > 0 {
                let (pps, steps) = profile.next_chunk(at - start, left, false);
                if steps == 0 {
                    break;
                }
                let chunk_end = at + travel(steps, pps);
                legs[i].push(Chunk {
                    begin: at,
                    end: chunk_end,
                    steps,
                    toward_max,
                });
                left -= steps;
                at = chunk_end;
            }
            end = end.max(at);
        }
        Self {
            start,
            end,
            from,
            to,
            legs,
        }
    }

    /// One hold chunk on one axis, kicked at `start` and ending at `end`.
    pub fn chunk(
        start: Duration,
        end: Duration,
        axis: usize,
        from: (u32, u32),
        steps: u32,
        toward_max: bool,
    ) -> Self {
        let mut to = from;
        let v = if axis == 0 { &mut to.0 } else { &mut to.1 };
        *v = if toward_max {
            v.saturating_add(steps)
        } else {
            v.saturating_sub(steps)
        };
        let mut legs = [Vec::new(), Vec::new()];
        legs[axis].push(Chunk {
            begin: start,
            end,
            steps,
            toward_max,
        });
        Self {
            start,
            end,
            from,
            to,
            legs,
        }
    }

    /// The pose at `t`: `from` before the start, `to` after the end, linear inside a chunk.
    pub fn at(&self, t: Duration) -> (u32, u32) {
        if t <= self.start {
            return self.from;
        }
        if t >= self.end {
            return self.to;
        }
        let mut pose = [self.from.0, self.from.1];
        for (i, leg) in self.legs.iter().enumerate() {
            let mut moved: i64 = 0;
            for c in leg {
                let done = if t >= c.end {
                    c.steps
                } else if t <= c.begin {
                    0
                } else {
                    let span = (c.end - c.begin).as_micros().max(1);
                    (u128::from(c.steps) * (t - c.begin).as_micros() / span) as u32
                };
                moved += if c.toward_max {
                    i64::from(done)
                } else {
                    -i64::from(done)
                };
            }
            pose[i] = (i64::from(pose[i]) + moved).max(0) as u32;
        }
        (pose[0], pose[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast() -> Profile {
        Profile::new(200, 1200, Duration::from_millis(800))
    }

    /// Runs a whole leg through the planner, advancing time by each chunk's own travel.
    fn leg(p: Profile, distance: u32, bounded: bool) -> Vec<(u32, u32)> {
        let (mut left, mut t, mut out) = (distance, Duration::ZERO, Vec::new());
        loop {
            let (pps, steps) = p.next_chunk(t, left, bounded);
            if steps == 0 {
                return out;
            }
            out.push((pps, steps));
            left -= steps;
            t += Duration::from_micros(u64::from(steps) * 1_000_000 / u64::from(pps));
        }
    }

    #[test]
    fn homing_is_the_vendor_rate_without_a_ramp() {
        let h = Profile::homing();
        assert_eq!((h.start_pps, h.max_pps), (DEFAULT_PPS, DEFAULT_PPS));
        assert_eq!(h.next_chunk(Duration::ZERO, 7460, false), (600, MAX_CHUNK));
    }

    #[test]
    fn a_leg_starts_slow_ramps_to_cruise_and_brakes_before_the_target() {
        let chunks = leg(fast(), 6000, false);
        assert_eq!(
            chunks.iter().map(|c| c.1).sum::<u32>(),
            6000,
            "exact travel"
        );
        assert_eq!(chunks[0].0, 200, "first chunk at the start rate");
        assert!(chunks.iter().any(|c| c.0 == 1200), "reaches cruise");
        let peak = chunks.iter().position(|c| c.0 == 1200).unwrap();
        assert!(
            chunks[..peak].windows(2).all(|w| w[0].0 < w[1].0),
            "ramps up"
        );
        assert!(
            chunks[peak..].windows(2).all(|w| w[0].0 >= w[1].0),
            "never speeds up again after cruise"
        );
        let last = chunks.last().unwrap().0;
        assert!(last < 400, "brakes to near the start rate, got {last}");
        // cruise is one long chunk instead of many 100 ms ones
        assert!(chunks.len() < 40, "{} chunks", chunks.len());
    }

    #[test]
    fn a_short_leg_never_reaches_cruise() {
        let chunks = leg(fast(), 150, false);
        assert_eq!(chunks.iter().map(|c| c.1).sum::<u32>(), 150);
        assert!(chunks.iter().all(|c| c.0 < 1200));
    }

    #[test]
    fn bounded_chunks_stay_about_one_tenth_of_a_second() {
        for (pps, steps) in leg(fast(), 5000, true) {
            assert!(steps <= pps / 10 + 1, "{steps} steps at {pps} pps");
        }
    }

    #[test]
    fn no_ramp_runs_at_max_from_the_first_chunk() {
        let p = Profile::new(200, 1200, Duration::ZERO);
        assert_eq!(p.next_chunk(Duration::ZERO, 50, true).0, 1200);
    }

    #[test]
    fn rates_are_clamped_into_the_divisor_range() {
        let p = Profile::new(10, 50, Duration::from_millis(1));
        assert_eq!((p.start_pps, p.max_pps), (MIN_PPS, MIN_PPS));
        let p = Profile::new(2000, 1000, Duration::ZERO);
        assert_eq!(p.start_pps, 1000, "start never exceeds max");
    }

    #[test]
    fn worst_case_uses_the_start_rate() {
        assert_eq!(fast().worst_case(400), Duration::from_secs(2));
    }

    #[test]
    fn a_goto_trajectory_is_the_profiles_chunk_schedule_per_axis() {
        let p = fast();
        let t = Trajectory::plan(Duration::from_secs(1), (1000, 500), (3000, 300), &p);
        assert_eq!(t.start, Duration::from_secs(1));
        assert_eq!(t.at(Duration::ZERO), (1000, 500));
        assert_eq!(t.at(Duration::from_secs(1)), (1000, 500));
        assert_eq!(t.at(t.end), (3000, 300));
        assert_eq!(t.at(t.end + Duration::from_secs(5)), (3000, 300));
        // the pan leg covers 2000 steps, the tilt leg 200, both as the planner would issue them
        assert_eq!(t.legs[0].iter().map(|c| c.steps).sum::<u32>(), 2000);
        assert_eq!(t.legs[1].iter().map(|c| c.steps).sum::<u32>(), 200);
        assert!(t.legs[0].windows(2).all(|w| w[0].end == w[1].begin));
        // the tilt leg ends first; afterwards the tilt stays at 300 while the pan goes on
        let tilt_end = t.legs[1].last().unwrap().end;
        assert!(tilt_end < t.end);
        let mid = t.at(tilt_end + Duration::from_millis(1));
        assert_eq!(mid.1, 300);
        assert!(mid.0 > 1000 && mid.0 < 3000, "{mid:?}");
    }

    #[test]
    fn the_pose_is_linear_inside_a_chunk_and_monotonic_over_the_move() {
        let p = fast();
        let t = Trajectory::plan(Duration::ZERO, (0, 0), (1200, 0), &p);
        let c = t.legs[0][0];
        let quarter = c.begin + (c.end - c.begin) / 4;
        let half = c.begin + (c.end - c.begin) / 2;
        assert!(t.at(quarter).0 > 0);
        assert!(t.at(half).0 > t.at(quarter).0);
        assert_eq!(t.at(c.end).0, c.steps);
        let mut last = 0;
        let mut at = Duration::ZERO;
        while at < t.end {
            let now = t.at(at).0;
            assert!(now >= last);
            last = now;
            at += Duration::from_millis(10);
        }
    }

    #[test]
    fn a_hold_chunk_moves_one_axis_down_or_up() {
        let t = Trajectory::chunk(
            Duration::from_secs(2),
            Duration::from_secs(3),
            1,
            (100, 500),
            60,
            false,
        );
        assert_eq!(t.to, (100, 440));
        assert_eq!(t.at(Duration::from_millis(2500)), (100, 470));
        let up = Trajectory::chunk(
            Duration::ZERO,
            Duration::from_secs(1),
            0,
            (7340, 0),
            40,
            true,
        );
        assert_eq!(up.to, (7380, 0), "the caller clamps to travel");
    }
}
