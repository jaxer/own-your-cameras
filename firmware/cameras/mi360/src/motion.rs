//! Pure requested-speed policy for one bounded motor target.
//!
//! The caller supplies monotonic milliseconds and measured raw-step progress. No
//! clock, device, thread, angles or image orientation live here. The board adapter
//! validates absolute endpoints against a frozen pulse-counter snapshot.
use std::io;

pub const MIN_SPEED: u32 = 100;
pub const MAX_SPEED: u32 = 900;
const MAX_TICK_MS: u64 = 100;
const MAX_RATE: u32 = (MAX_SPEED - MIN_SPEED) * 1000;

/// User timings describe the full start-to-ceiling speed span. Each move takes
/// one snapshot; changing a slider never changes an in-flight braking curve.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timings {
    pub start_speed: u32,
    pub ramp_up_ms: u32,
    pub ramp_down_ms: u32,
}
impl Default for Timings {
    fn default() -> Self {
        Self {
            start_speed: 100,
            ramp_up_ms: 250,
            ramp_down_ms: 150,
        }
    }
}
impl Timings {
    pub fn profile(self, maximum: u32) -> io::Result<Profile> {
        if !(MIN_SPEED..=MAX_SPEED).contains(&self.start_speed)
            || self.ramp_up_ms > 3000
            || self.ramp_down_ms > 3000
            || !(MIN_SPEED..=MAX_SPEED).contains(&maximum)
        {
            return Err(invalid("invalid PTZ ramp timings"));
        }
        let start = self.start_speed.min(maximum);
        let span = maximum - start;
        let rate = |ms: u32| (span * 1000).div_ceil(ms.max(1)).max(1);
        Profile {
            start_speed: start,
            max_speed: maximum,
            end_speed: start,
            acceleration: rate(self.ramp_up_ms),
            deceleration: rate(self.ramp_down_ms),
        }
        .validate()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Profile {
    pub start_speed: u32,
    pub max_speed: u32,
    pub end_speed: u32,
    /// Requested-speed increase per second, rather than measured acceleration.
    pub acceleration: u32,
    pub deceleration: u32,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            start_speed: 100,
            max_speed: 300,
            end_speed: 100,
            acceleration: 600,
            deceleration: 900,
        }
    }
}
impl Profile {
    pub fn validate(self) -> io::Result<Self> {
        if !(MIN_SPEED..=MAX_SPEED).contains(&self.start_speed)
            || !(self.start_speed..=MAX_SPEED).contains(&self.max_speed)
            || !(MIN_SPEED..=self.max_speed).contains(&self.end_speed)
            || !(1..=MAX_RATE).contains(&self.acceleration)
            || !(1..=MAX_RATE).contains(&self.deceleration)
        {
            return Err(invalid("invalid bounded motor speed profile"));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    Speed(u32),
    Stop,
}

#[derive(Debug)]
pub struct Planner {
    profile: Profile,
    delta: [i32; 2],
    travelled: [i32; 2],
    last_ms: u64,
    speed_milli: u64,
    braking: bool,
    stopped: bool,
}
impl Planner {
    pub fn new(profile: Profile, delta: [i32; 2], now_ms: u64) -> io::Result<Self> {
        let profile = profile.validate()?;
        if delta == [0, 0] || delta.iter().any(|d| d.unsigned_abs() > 2404) {
            return Err(invalid("motion needs a nonzero bounded raw-step target"));
        }
        Ok(Self {
            profile,
            delta,
            travelled: [0, 0],
            last_ms: now_ms,
            speed_milli: u64::from(profile.start_speed) * 1000,
            braking: false,
            stopped: false,
        })
    }
    /// A lower cap also clamps this trajectory's start/end speeds. Raising the
    /// cap does not jump the current speed; the next move takes fresh timings.
    pub fn set_max_speed(&mut self, max_speed: u32) -> io::Result<()> {
        self.profile = Profile {
            max_speed,
            start_speed: self.profile.start_speed.min(max_speed),
            end_speed: self.profile.end_speed.min(max_speed),
            ..self.profile
        }
        .validate()?;
        Ok(())
    }
    /// A compatible vector replacement keeps its speed/ramp and starts progress
    /// at the adapter's frozen, measured origin. True reversal brakes first.
    pub fn retarget(&mut self, delta: [i32; 2], now_ms: u64) -> io::Result<()> {
        if self.stopped || now_ms < self.last_ms {
            return Err(invalid("cannot retarget a braking/stopped trajectory"));
        }
        Self::new(self.profile, delta, now_ms)?;
        self.delta = delta;
        self.braking = false;
        self.travelled = [0, 0];
        self.last_ms = now_ms;
        Ok(())
    }
    /// Ordinary release: ramp down to the driver's minimum allowed endpoint
    /// speed, then stop. Urgent Stop/deadman must instead pass urgent_stop=true.
    pub fn request_brake(&mut self) {
        self.braking = true;
    }
    pub fn is_braking(&self) -> bool {
        self.braking
    }
    pub fn sample(
        &mut self,
        now_ms: u64,
        travelled: [i32; 2],
        urgent_stop: bool,
    ) -> io::Result<Command> {
        // Stop must not depend on clock/progress validity or wait for a ramp.
        if urgent_stop || self.stopped {
            self.stopped = true;
            return Ok(Command::Stop);
        }
        let dt = now_ms
            .checked_sub(self.last_ms)
            .ok_or_else(|| invalid("motion clock moved backward"))?
            .min(MAX_TICK_MS);
        let mut remaining = 0_u64;
        for (axis, &position) in travelled.iter().enumerate() {
            let direction = i64::from(self.delta[axis].signum());
            let total = i64::from(self.delta[axis]).abs();
            let progress = i64::from(position) * direction;
            let previous = i64::from(self.travelled[axis]) * direction;
            if (direction == 0 && position != 0) || progress < previous || progress > total {
                return Err(invalid(
                    "motor progress reversed, overshot or changed idle axis",
                ));
            }
            remaining = remaining.max((total - progress) as u64);
        }
        self.last_ms = now_ms;
        self.travelled = travelled;
        if remaining == 0 {
            self.stopped = true;
            return Ok(Command::Stop);
        }
        let end = u64::from(self.profile.end_speed) * 1000;
        // Conservative spatial cap. It can reduce speed faster than the time
        // slew when a delayed observation has already entered the braking zone.
        // Hardware endpoint bounds remain independent of this approximation.
        let cap = integer_sqrt(
            u64::from(self.profile.end_speed).pow(2)
                + 2 * u64::from(self.profile.deceleration) * remaining,
        )
        .min(u64::from(self.profile.max_speed))
            * 1000;
        if self.braking {
            self.speed_milli = self
                .speed_milli
                .saturating_sub(u64::from(self.profile.deceleration) * dt)
                .max(end)
                .min(cap);
            if self.speed_milli <= end {
                self.stopped = true;
                return Ok(Command::Stop);
            }
        } else {
            self.speed_milli =
                (self.speed_milli + u64::from(self.profile.acceleration) * dt).min(cap);
        }
        Ok(Command::Speed((self.speed_milli / 1000) as u32))
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut low = 1_u64;
    let mut high = value.min(u64::from(u32::MAX));
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configured_times_cover_full_speed_span_and_clamp_start_to_ceiling() {
        for (start, ceiling, up, down, acceleration, deceleration) in [
            (100, 900, 800, 800, 1000, 1000),
            (100, 900, 250, 150, 3200, 5334),
            (300, 900, 0, 3000, 600000, 200),
            (899, 900, 3000, 3000, 1, 1),
            (900, 100, 0, 0, 1, 1),
        ] {
            let p = Timings {
                start_speed: start,
                ramp_up_ms: up,
                ramp_down_ms: down,
            }
            .profile(ceiling)
            .unwrap();
            assert_eq!(
                (p.start_speed, p.end_speed),
                (start.min(ceiling), start.min(ceiling))
            );
            assert_eq!(
                (p.acceleration, p.deceleration),
                (acceleration, deceleration)
            );
        }
        for bad in [
            Timings {
                start_speed: 99,
                ..Timings::default()
            },
            Timings {
                start_speed: 901,
                ..Timings::default()
            },
            Timings {
                ramp_up_ms: 3001,
                ..Timings::default()
            },
            Timings {
                ramp_down_ms: u32::MAX,
                ..Timings::default()
            },
        ] {
            assert!(bad.profile(900).is_err());
        }
    }
    #[test]
    fn zero_and_slow_ramps_and_lower_live_cap_keep_stop_and_bounds() {
        let instant = Timings {
            ramp_up_ms: 0,
            ramp_down_ms: 0,
            ..Timings::default()
        };
        let mut p = Planner::new(instant.profile(900).unwrap(), [2000, 0], 0).unwrap();
        assert_eq!(speed(p.sample(0, [0, 0], false).unwrap()), 100);
        assert_eq!(speed(p.sample(10, [0, 0], false).unwrap()), 900);
        p.request_brake();
        assert_eq!(p.sample(20, [0, 0], false).unwrap(), Command::Stop);
        let slow = Timings {
            start_speed: 300,
            ramp_up_ms: 3000,
            ramp_down_ms: 3000,
        };
        let mut p = Planner::new(slow.profile(900).unwrap(), [2000, 0], 0).unwrap();
        for now in (100..=3000).step_by(100) {
            p.sample(now, [0, 0], false).unwrap();
        }
        assert_eq!(speed(p.sample(3000, [0, 0], false).unwrap()), 900);
        p.set_max_speed(100).unwrap();
        assert_eq!(speed(p.sample(3010, [0, 0], false).unwrap()), 100);
        p.request_brake();
        assert_eq!(p.sample(3020, [0, 0], false).unwrap(), Command::Stop);
        let mut p = Planner::new(slow.profile(900).unwrap(), [2000, 0], 0).unwrap();
        assert_eq!(p.sample(0, [0, 0], true).unwrap(), Command::Stop);
    }
    fn speed(command: Command) -> u32 {
        match command {
            Command::Speed(value) => value,
            Command::Stop => panic!("unexpected stop"),
        }
    }
    #[test]
    fn rejects_invalid_public_profiles_and_distances() {
        let p = Profile::default();
        for bad in [
            Profile {
                start_speed: 99,
                ..p
            },
            Profile {
                max_speed: 901,
                ..p
            },
            Profile {
                end_speed: 301,
                ..p
            },
            Profile {
                acceleration: 0,
                ..p
            },
            Profile {
                deceleration: MAX_RATE + 1,
                ..p
            },
        ] {
            assert!(Planner::new(bad, [100, 0], 0).is_err());
        }
        for bad in [[0, 0], [i32::MIN, 0], [2405, 0]] {
            assert!(Planner::new(p, bad, 0).is_err());
        }
    }
    #[test]
    fn rate_limit_fractional_ticks_cap_and_scheduler_delay() {
        let mut p = Planner::new(Profile::default(), [2404, 0], 0).unwrap();
        assert_eq!(speed(p.sample(0, [0, 0], false).unwrap()), 100);
        assert_eq!(speed(p.sample(1, [0, 0], false).unwrap()), 100);
        assert_eq!(speed(p.sample(2, [0, 0], false).unwrap()), 101);
        assert_eq!(speed(p.sample(1002, [0, 0], false).unwrap()), 161);
        for now in (1102..=2102).step_by(100) {
            assert!(speed(p.sample(now, [0, 0], false).unwrap()) <= 300);
        }
        assert_eq!(speed(p.sample(2102, [0, 0], false).unwrap()), 300);
    }
    #[test]
    fn changing_max_speed_lowers_immediately_but_raises_with_slew() {
        let profile = Profile {
            max_speed: 900,
            acceleration: 1000,
            deceleration: 1000,
            ..Profile::default()
        };
        let mut p = Planner::new(profile, [2404, 0], 0).unwrap();
        for now in (100..=800).step_by(100) {
            p.sample(now, [0, 0], false).unwrap();
        }
        assert_eq!(speed(p.sample(800, [0, 0], false).unwrap()), 900);
        p.set_max_speed(200).unwrap();
        assert_eq!(speed(p.sample(800, [0, 0], false).unwrap()), 200);
        p.set_max_speed(900).unwrap();
        assert_eq!(speed(p.sample(800, [0, 0], false).unwrap()), 200);
        assert_eq!(speed(p.sample(810, [0, 0], false).unwrap()), 210);
        p.request_brake();
        p.set_max_speed(150).unwrap();
        assert_eq!(speed(p.sample(820, [0, 0], false).unwrap()), 150);
        p.set_max_speed(900).unwrap();
        assert_eq!(speed(p.sample(830, [0, 0], false).unwrap()), 140);
    }
    #[test]
    fn invalid_max_update_preserves_previous_cap_and_start_end_contract() {
        let mut p = Planner::new(Profile::default(), [2404, 0], 0).unwrap();
        for invalid in [0, 99, 901, u32::MAX] {
            assert!(p.set_max_speed(invalid).is_err());
        }
        for now in (100..=500).step_by(100) {
            p.sample(now, [0, 0], false).unwrap();
        }
        assert_eq!(speed(p.sample(500, [0, 0], false).unwrap()), 300);
        let mut p = Planner::new(
            Profile {
                start_speed: 200,
                end_speed: 250,
                ..Profile::default()
            },
            [2404, 0],
            0,
        )
        .unwrap();
        assert!(p.set_max_speed(99).is_err());
        assert!(p.set_max_speed(100).is_ok());
        assert!(p.set_max_speed(250).is_ok());
    }
    #[test]
    fn short_moves_never_jump_to_max_and_last_step_stops() {
        let mut p = Planner::new(Profile::default(), [3, 0], 0).unwrap();
        assert!(speed(p.sample(100, [1, 0], false).unwrap()) < 120);
        assert!(speed(p.sample(200, [2, 0], false).unwrap()) < 110);
        assert_eq!(p.sample(210, [3, 0], false).unwrap(), Command::Stop);
    }
    #[test]
    fn approaching_endpoint_decelerates_using_actual_remaining_steps() {
        let mut p = Planner::new(Profile::default(), [2404, -300], 0).unwrap();
        for now in (100..=400).step_by(100) {
            p.sample(now, [0, 0], false).unwrap();
        }
        assert_eq!(speed(p.sample(500, [2000, -300], false).unwrap()), 300);
        assert!(speed(p.sample(600, [2400, -300], false).unwrap()) < 140);
        assert_eq!(p.sample(610, [2404, -300], false).unwrap(), Command::Stop);
    }
    #[test]
    fn ordinary_release_brakes_but_urgent_stop_is_immediate_and_latched() {
        let mut p = Planner::new(Profile::default(), [1000, 0], 0).unwrap();
        for now in (100..=400).step_by(100) {
            p.sample(now, [0, 0], false).unwrap();
        }
        p.request_brake();
        assert!(p.is_braking());
        assert_eq!(speed(p.sample(500, [20, 0], false).unwrap()), 210);
        assert_eq!(speed(p.sample(600, [30, 0], false).unwrap()), 120);
        assert_eq!(p.sample(700, [40, 0], false).unwrap(), Command::Stop);
        let mut p = Planner::new(Profile::default(), [1000, 0], 100).unwrap();
        assert_eq!(p.sample(0, [i32::MAX, 4], true).unwrap(), Command::Stop);
        assert_eq!(p.sample(0, [0, 0], false).unwrap(), Command::Stop);
    }
    #[test]
    fn opposite_direction_requires_new_stopped_plan() {
        let mut p = Planner::new(Profile::default(), [-20, 10], 0).unwrap();
        p.sample(10, [-2, 1], false).unwrap();
        for bad in [[-1, 1], [1, 1], [-21, 1], [-2, -1]] {
            assert!(p.sample(20, bad, false).is_err());
        }
        assert_eq!(p.sample(0, [0, 0], true).unwrap(), Command::Stop);
        let mut next = Planner::new(Profile::default(), [20, -10], 20).unwrap();
        assert!(next.sample(30, [2, -1], false).is_ok());
        let mut idle = Planner::new(Profile::default(), [20, 0], 0).unwrap();
        assert!(idle.sample(10, [1, 1], false).is_err());
    }
    #[test]
    fn clock_rollback_fails_without_advancing_plan() {
        let mut p = Planner::new(Profile::default(), [100, 0], 10).unwrap();
        assert!(p.sample(9, [1, 0], false).is_err());
        assert_eq!(speed(p.sample(10, [0, 0], false).unwrap()), 100);
    }
    #[test]
    fn square_root_is_floor_at_boundaries() {
        for value in [0_u64, 1, 2, 3, 4, 8, 9, 10, 25_000_000, u64::MAX] {
            let r = integer_sqrt(value);
            assert!(r == 0 || r <= value / r);
            assert!(r == u64::from(u32::MAX) || (r + 1) > value / (r + 1));
        }
    }
}
