//! Qualified Mi360 board adapter: raw driver steps and GPIO levels, never angles/lux.
//!
//! Only the Linux/MIPS backend touches hardware. Opening retains the same flock
//! inode as camctl and does not actuate anything. Stock owners must be stopped by
//! the application; this module never stops a service, exports a GPIO, opens the
//! watchdog, calibrates a motor, or invokes a factory-reset operation. GPIOs must
//! already have their qualified directions. See docs/hardware.md for evidence.
//!
//! Call shutdown() and handle its error before handing hardware to another owner.
//! Drop attempts cleanup; unresolved cleanup retains descriptors/lock until process
//! exit. SIGKILL, abort, power loss and a wedged kernel cannot run Rust cleanup.

use crate::motion::{Command as MotionCommand, Planner, Profile};
use std::io;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

pub const MAX_RELATIVE_STEPS: i32 = 256;
pub const DEFAULT_MOTOR_SPEED: i32 = 200;
pub const MAX_MOTOR_SPEED: i32 = 900;
// Qualified normal travel excludes end-stop overtravel; only homing uses it.
pub const PAN_TRAVEL: i32 = 2280;
pub const TILT_TRAVEL: i32 = 650;
pub const IR_CUT_PULSE: Duration = Duration::from_millis(500);
pub const QUALIFIED_B_KERNEL_SHA256: &str =
    "a06bc71c6d94b59ef2bea947e152428efa00e8953bb578edbaf1865adc45b902";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Led {
    Yellow,
    Blue,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IrCut {
    Day,
    Night,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MotorStatus {
    pub position: Position,
    pub moving: bool,
    pub speed: i32,
    pub owner_open: bool,
    pub max_x: i32,
    pub max_y: i32,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RelativeMove {
    pub x: i32,
    pub y: i32,
    pub speed: i32,
}

impl RelativeMove {
    pub fn new(x: i32, y: i32, speed: i32) -> io::Result<Self> {
        if !(-MAX_RELATIVE_STEPS..=MAX_RELATIVE_STEPS).contains(&x)
            || !(-MAX_RELATIVE_STEPS..=MAX_RELATIVE_STEPS).contains(&y)
            || (x == 0 && y == 0)
            || !(100..=MAX_MOTOR_SPEED).contains(&speed)
        {
            return Err(invalid(
                "move needs nonzero +/-256 raw steps and speed100..900",
            ));
        }
        Ok(Self { x, y, speed })
    }
    pub fn at_default_speed(x: i32, y: i32) -> io::Result<Self> {
        Self::new(x, y, DEFAULT_MOTOR_SPEED)
    }
    fn endpoint(self, status: MotorStatus) -> io::Result<Position> {
        // Revalidate public fields at every boundary, including struct literals.
        Self::new(self.x, self.y, self.speed)?;
        motor_endpoint(
            Position {
                x: self.x,
                y: self.y,
            },
            status,
        )
    }
}

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub enum Observation<T> {
    Known(T),
    Unavailable(String),
}
impl<T> Observation<T> {
    #[cfg(all(target_os = "linux", target_arch = "mips"))]
    fn from_result(result: io::Result<T>) -> Self {
        match result {
            Ok(v) => Self::Known(v),
            Err(e) => Self::Unavailable(e.to_string()),
        }
    }
}
#[derive(Debug)]
pub struct BoardSnapshot {
    pub motor: Observation<MotorStatus>,
    pub yellow_led_high: Observation<bool>,
    pub blue_led_high: Observation<bool>,
    pub ir_illumination_high: Observation<bool>,
    /// Electrical level only. Press polarity and debounce are not certified.
    pub reset_gpio57_high: Observation<bool>,
    pub watchdog_owner_pids: Observation<Vec<u32>>,
}

fn invalid(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, text)
}
fn busy(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, text)
}
fn unsupported(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, text)
}
fn interrupted() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, "board operation cancelled")
}
fn combined<T>(result: io::Result<T>, cleanup: io::Result<()>) -> io::Result<T> {
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e),
        (Err(first), Err(last)) => Err(io::Error::other(format!("{first}; cleanup: {last}"))),
    }
}
fn validate_motor(m: MotorStatus) -> io::Result<()> {
    if m.max_x != PAN_TRAVEL
        || m.max_y != TILT_TRAVEL
        || !(0..=m.max_x).contains(&m.position.x)
        || !(0..=m.max_y).contains(&m.position.y)
        || !(100..=900).contains(&m.speed)
    {
        return Err(invalid(
            "unsupported motor limits, coordinate or driver speed",
        ));
    }
    Ok(())
}

fn motor_endpoint(delta: Position, status: MotorStatus) -> io::Result<Position> {
    validate_motor(status)?;
    if status.moving {
        return Err(busy("motor is already moving"));
    }
    if delta.x == 0 && delta.y == 0 {
        return Err(invalid(
            "motor target must differ from its current position",
        ));
    }
    let x = status
        .position
        .x
        .checked_add(delta.x)
        .ok_or_else(|| invalid("motor x overflow"))?;
    let y = status
        .position
        .y
        .checked_add(delta.y)
        .ok_or_else(|| invalid("motor y overflow"))?;
    if !(0..=status.max_x).contains(&x) || !(0..=status.max_y).contains(&y) {
        return Err(invalid("requested endpoint exceeds qualified motor limits"));
    }
    Ok(Position { x, y })
}

/// Parse the qualified proc diagnostic text; absent/conflicting fields fail closed.
pub fn parse_motor_diagnostics(text: &str) -> io::Result<MotorStatus> {
    parse_motor_diagnostics_mode(text, false)
}
fn parse_motor_diagnostics_mode(text: &str, unhomed: bool) -> io::Result<MotorStatus> {
    fn one<'a>(text: &'a str, prefix: &str) -> io::Result<&'a str> {
        let mut rows = text.lines().filter_map(|line| line.strip_prefix(prefix));
        let row = rows
            .next()
            .ok_or_else(|| invalid("missing motor diagnostic field"))?;
        if rows.next().is_some() {
            return Err(invalid("duplicate motor diagnostic field"));
        }
        Ok(row)
    }
    if one(text, "The version of Motor driver is ")? != "H20171204a" {
        return Err(unsupported("unsupported motor driver version"));
    }
    let owner_open = match one(text, "Motor driver is ")? {
        "opened" => true,
        "closed" => false,
        _ => return Err(invalid("invalid motor owner field")),
    };
    let moving = match one(text, "The status of motor is ")? {
        "running" => true,
        "stop" => false,
        _ => return Err(invalid("invalid motor status field")),
    };
    let position = one(text, "The pos of motor is ")?;
    let position = position
        .strip_prefix('(')
        .and_then(|p| p.strip_suffix(')'))
        .ok_or_else(|| invalid("invalid motor coordinate syntax"))?;
    let (x, y) = position
        .split_once(',')
        .ok_or_else(|| invalid("missing motor coordinate"))?;
    let integer = |v: &str| {
        v.trim()
            .parse::<i32>()
            .map_err(|_| invalid("invalid motor integer"))
    };
    fn axis(text: &str, label: &str) -> io::Result<i32> {
        let mut matches = text.match_indices(label);
        let (start, _) = matches
            .next()
            .ok_or_else(|| invalid("missing motor axis"))?;
        if matches.next().is_some() {
            return Err(invalid("duplicate motor axis"));
        }
        text[start + label.len()..]
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .parse()
            .map_err(|_| invalid("invalid motor axis limit"))
    }
    let status = MotorStatus {
        position: Position {
            x: integer(x)?,
            y: integer(y)?,
        },
        moving,
        owner_open,
        speed: integer(one(text, "The speed of motor is ")?)?,
        max_x: axis(text, "## motor is Horizontal motor ##\nmax steps ")?,
        max_y: axis(text, "## motor is Vertical motor ##\nmax steps ")?,
    };
    if status.max_x != 2404 || status.max_y != 713 || !(100..=900).contains(&status.speed) {
        return Err(invalid("unqualified driver motor geometry"));
    }
    let status = MotorStatus {
        max_x: PAN_TRAVEL,
        max_y: TILT_TRAVEL,
        ..status
    };
    if !unhomed {
        validate_motor(status)?;
    }
    Ok(status)
}

// One control algorithm is used by the target and by isolated fault tests. The
// backend owns I/O, monotonic time and waiting; tests supply those dependencies.
trait Hardware {
    fn refuse_vendor(&self, names: &[&str]) -> io::Result<()>;
    fn motor_claim(&mut self) -> io::Result<MotorStatus>;
    fn motor_claim_home(&mut self) -> io::Result<MotorStatus> {
        self.motor_claim()
    }
    fn seed_home_start(&mut self) -> io::Result<()> {
        Err(unsupported("homing seed unavailable"))
    }
    fn seed_home_pending(&self) -> bool {
        false
    }
    fn seed_home_poll(&mut self) -> io::Result<Option<[i32; 4]>> {
        Err(unsupported("homing seed unavailable"))
    }
    fn motor_status(&mut self) -> io::Result<MotorStatus>;
    fn motor_speed(&mut self, speed: i32) -> io::Result<()>;
    fn motor_move(&mut self, movement: RelativeMove) -> io::Result<()>;
    fn motor_retarget(
        &mut self,
        _target: Position,
        _axes: [i32; 2],
    ) -> io::Result<(MotorStatus, Position)> {
        Err(unsupported("atomic live retarget unavailable"))
    }
    fn motor_stop(&mut self) -> io::Result<()>;
    fn motor_release(&mut self);
    fn gpio_claim(&mut self, pin: u32) -> io::Result<bool>;
    fn gpio_write(&mut self, pin: u32, high: bool) -> io::Result<()>;
    fn gpio_read(&mut self, pin: u32) -> io::Result<bool>;
    fn gpio_release(&mut self, pin: u32);
    fn speaker_claim(&mut self) -> io::Result<()>;
    fn speaker_gate(&mut self, on: bool) -> io::Result<()>;
    fn speaker_release(&mut self);
    fn now_ms(&self) -> u64;
    fn wait_ms(&mut self, milliseconds: u64);
}
#[cfg(any(test, all(target_os = "linux", target_arch = "mips")))]
fn retarget_at_snapshot<H: Hardware + ?Sized>(
    hw: &mut H,
    target: Position,
    axes: [i32; 2],
) -> io::Result<(MotorStatus, Position)> {
    let current = hw.motor_status()?;
    validate_motor(current)?;
    let target = Position {
        x: if axes[0] == 0 {
            current.position.x
        } else {
            target.x
        },
        y: if axes[1] == 0 {
            current.position.y
        } else {
            target.y
        },
    };
    if !(0..=current.max_x).contains(&target.x) || !(0..=current.max_y).contains(&target.y) {
        return Err(invalid("retarget outside qualified travel"));
    }
    let delta = Position {
        x: target.x - current.position.x,
        y: target.y - current.position.y,
    };
    if delta == (Position { x: 0, y: 0 }) {
        hw.motor_stop()?;
    } else {
        hw.motor_move(RelativeMove {
            x: delta.x,
            y: delta.y,
            speed: current.speed,
        })?;
    }
    Ok((current, target))
}
const PINS: [u32; 5] = [8, 9, 49, 51, 54];
struct ActiveMotion {
    planner: Planner,
    start: Position,
    target: Position,
    original_speed: i32,
    commanded_speed: i32,
    deadline_ms: u64,
}
struct Homing {
    original_speed: i32,
    seek_target: Position,
    centering: bool,
    deadline_ms: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HomeProgress {
    Running,
    Ready(MotorStatus),
}
struct Controller<H: Hardware> {
    io: H,
    original: [Option<bool>; 5],
    motor_owned: bool,
    motor_speed: Option<i32>,
    motion: Option<ActiveMotion>,
    home: Option<Homing>,
    last_motor_status: Option<MotorStatus>,
    speaker_owned: bool,
    poisoned: bool,
    last_filter_command: Option<IrCut>,
}
impl<H: Hardware> Controller<H> {
    fn new(io: H) -> Self {
        Self {
            io,
            original: [None; 5],
            motor_owned: false,
            motor_speed: None,
            motion: None,
            home: None,
            last_motor_status: None,
            speaker_owned: false,
            poisoned: false,
            last_filter_command: None,
        }
    }
    fn ready(&self) -> io::Result<()> {
        if self.poisoned {
            Err(io::Error::other(
                "board cleanup uncertain; shutdown/recovery required",
            ))
        } else {
            Ok(())
        }
    }
    fn claim(&mut self, pin: u32) -> io::Result<bool> {
        let index = PINS
            .iter()
            .position(|p| *p == pin)
            .ok_or_else(|| invalid("unsupported GPIO"))?;
        if self.original[index].is_none() {
            self.original[index] = Some(self.io.gpio_claim(pin)?);
        }
        self.io.gpio_read(pin)
    }
    fn wait(&mut self, duration: u64, cancel: &Cancellation) -> io::Result<()> {
        let deadline = self
            .io
            .now_ms()
            .checked_add(duration)
            .ok_or_else(|| invalid("timer overflow"))?;
        loop {
            if cancel.is_cancelled() {
                return Err(interrupted());
            }
            let now = self.io.now_ms();
            if now >= deadline {
                return Ok(());
            }
            self.io.wait_ms((deadline - now).min(10));
        }
    }
    fn finish_motor(&mut self) -> io::Result<()> {
        if self.home.is_some() {
            return self.finish_home();
        }
        self.motion = None;
        if !self.motor_owned {
            return Ok(());
        }
        if let Err(e) = self.io.motor_stop() {
            self.poisoned = true;
            return Err(io::Error::other(format!(
                "motor stop failed; ownership retained: {e}"
            )));
        }
        if let Some(speed) = self.motor_speed
            && let Err(e) = self.io.motor_speed(speed)
        {
            self.poisoned = true;
            return Err(io::Error::other(format!(
                "motor speed restore failed; ownership retained: {e}"
            )));
        }
        let after = self.io.motor_status().and_then(|status| {
            validate_motor(status)?;
            if status.moving {
                return Err(io::Error::other("motor remains moving after stop"));
            }
            if self.motor_speed.is_some_and(|speed| status.speed != speed) {
                return Err(io::Error::other("motor speed restore readback differs"));
            }
            Ok(status)
        });
        match after {
            Ok(mut status) => {
                status.owner_open = false;
                self.last_motor_status = Some(status);
            }
            Err(e) => {
                self.poisoned = true;
                return Err(io::Error::other(format!(
                    "motor stop verification failed; ownership retained: {e}"
                )));
            }
        }
        self.motor_speed = None;
        self.io.motor_release();
        self.motor_owned = false;
        Ok(())
    }
    fn begin_home(&mut self) -> io::Result<()> {
        self.ready()?;
        if self.motor_owned {
            return Err(busy("motor already active"));
        }
        let before = self.io.motor_claim_home()?;
        self.motor_owned = true;
        self.motor_speed = Some(before.speed);
        let result = (|| {
            let seek = Position {
                x: 2404
                    + before
                        .position
                        .x
                        .checked_sub(2404)
                        .ok_or_else(|| invalid("homing counter overflow"))?
                        .rem_euclid(8),
                y: 713
                    + before
                        .position
                        .y
                        .checked_sub(713)
                        .ok_or_else(|| invalid("homing counter overflow"))?
                        .rem_euclid(8),
            };
            // Finish at electrical phase zero while against each end stop.
            // Seeding counter zero then preserves phase instead of jumping it.
            let target = Position {
                x: before
                    .position
                    .x
                    .checked_sub(seek.x)
                    .ok_or_else(|| invalid("homing counter overflow"))?,
                y: before
                    .position
                    .y
                    .checked_sub(seek.y)
                    .ok_or_else(|| invalid("homing counter overflow"))?,
            };
            self.home = Some(Homing {
                original_speed: before.speed,
                seek_target: target,
                centering: false,
                deadline_ms: self.io.now_ms() + 11000,
            });
            self.io.motor_speed(300)?;
            // Dedicated finite physical-reference seek, never the stock driver's
            // unbounded zero-max reset branch. Negative counters are private to it.
            self.io.motor_move(RelativeMove {
                x: -seek.x,
                y: -seek.y,
                speed: 300,
            })
        })();
        match result {
            Ok(()) => Ok(()),
            Err(e) => combined(Err(e), self.finish_home()),
        }
    }
    fn finish_home(&mut self) -> io::Result<()> {
        let Some(home) = self.home.take() else {
            return Ok(());
        };
        let result = (|| {
            self.io.motor_stop()?;
            if home.centering {
                // Stop may race the kernel's seed->go-back transition. Keep
                // stopping until it joins, then Stop once more before release.
                let deadline = self.io.now_ms() + 1000;
                loop {
                    if !self.io.seed_home_pending() || self.io.seed_home_poll()?.is_some() {
                        break;
                    }
                    if self.io.now_ms() >= deadline {
                        return Err(io::Error::other("homing seed did not cancel"));
                    }
                    self.io.motor_stop()?;
                    self.io.wait_ms(5);
                }
                self.io.motor_stop()?;
            }
            self.io.motor_speed(home.original_speed)?;
            let mut status = self.io.motor_status()?;
            if status.moving || status.speed != home.original_speed {
                return Err(io::Error::other("homing cleanup readback differs"));
            }
            status.owner_open = false;
            self.last_motor_status = Some(status);
            self.motor_speed = None;
            self.io.motor_release();
            self.motor_owned = false;
            Ok(())
        })();
        if result.is_err() {
            self.poisoned = true;
            self.home = Some(home);
        }
        result
    }
    fn poll_home(&mut self, cancel: &Cancellation) -> io::Result<HomeProgress> {
        self.ready()?;
        if cancel.is_cancelled() || self.home.is_none() {
            return combined(Err(interrupted()), self.finish_home());
        }
        let result = (|| {
            let home = self.home.as_ref().unwrap();
            if self.io.now_ms() > home.deadline_ms {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "bounded homing deadline",
                ));
            }
            let status = self.io.motor_status()?;
            if !status.owner_open || status.speed != if home.centering { 100 } else { 300 } {
                return Err(io::Error::other("homing motor ownership/speed differs"));
            }
            if !home.centering {
                if status.moving {
                    return Ok(HomeProgress::Running);
                }
                if status.position != home.seek_target {
                    return Err(io::Error::other("homing finite seek endpoint differs"));
                }
                // Leave the physical stop at the qualified minimum rate; an
                // immediate 300 pps restart can miss steps while unloading it.
                self.io.motor_speed(100)?;
                self.io.seed_home_start()?;
                let home = self.home.as_mut().unwrap();
                home.centering = true;
                home.deadline_ms = self.io.now_ms() + 21000;
                return Ok(HomeProgress::Running);
            }
            let Some(payload) = self.io.seed_home_poll()? else {
                return Ok(HomeProgress::Running);
            };
            // The worker is joined. Avoid a second join in cleanup, including
            // errors and Stop races. A successful ioctl alone is insufficient.
            self.home.as_mut().unwrap().centering = false;
            let status = self.io.motor_status()?;
            if payload != [2404, 713, 1202, 356]
                || status.moving
                || status.position != (Position { x: 1202, y: 356 })
            {
                return Err(io::Error::other("homing center readback differs"));
            }
            validate_motor(status)?;
            self.finish_home()?;
            Ok(HomeProgress::Ready(self.last_motor_status.unwrap()))
        })();
        match result {
            Ok(p) => Ok(p),
            Err(e) => combined(Err(e), self.finish_home()),
        }
    }
    fn move_relative(
        &mut self,
        movement: RelativeMove,
        cancel: &Cancellation,
    ) -> io::Result<MotorStatus> {
        self.ready()?;
        if self.motor_owned {
            return Err(busy("motor already has an active operation"));
        }
        RelativeMove::new(movement.x, movement.y, movement.speed)?;
        if cancel.is_cancelled() {
            return Err(interrupted());
        }
        let before = self.io.motor_claim()?;
        self.motor_owned = true;
        let result = (|| {
            let expected = movement.endpoint(before)?;
            if cancel.is_cancelled() {
                return Err(interrupted());
            }
            if before.speed != movement.speed {
                // A failed write may already have changed speed; remember before calling.
                self.motor_speed = Some(before.speed);
                self.io.motor_speed(movement.speed)?;
            }
            let deadline = self
                .io
                .now_ms()
                .checked_add(5000)
                .ok_or_else(|| invalid("timer overflow"))?;
            if cancel.is_cancelled() {
                return Err(interrupted());
            }
            self.io.motor_move(movement)?;
            loop {
                if cancel.is_cancelled() {
                    return Err(interrupted());
                }
                let after = self.io.motor_status()?;
                validate_motor(after)?;
                if self.io.now_ms() > deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "motor completion deadline exceeded",
                    ));
                }
                if !after.moving {
                    if after.position != expected {
                        return Err(io::Error::other(
                            "motor endpoint differs from requested raw steps",
                        ));
                    }
                    return Ok(after);
                }
                self.io.wait_ms(10);
            }
        })();
        combined(result, self.finish_motor()).map(|mut status| {
            status.speed = before.speed;
            status.owner_open = false;
            status.moving = false;
            status
        })
    }
    fn begin_profiled_move(
        &mut self,
        delta: Position,
        profile: Profile,
    ) -> io::Result<MotorStatus> {
        self.ready()?;
        profile.validate()?;
        // Reject malformed targets before acquiring a device. Actual endpoint
        // bounds are checked again against status obtained after exclusive open.
        Planner::new(profile, [delta.x, delta.y], self.io.now_ms())?;
        if self.motor_owned {
            return Err(busy("motor already has an active operation"));
        }
        let before = self.io.motor_claim()?;
        self.motor_owned = true;
        self.last_motor_status = None;
        let result = (|| {
            let target = motor_endpoint(delta, before)?;
            let now = self.io.now_ms();
            let mut planner = Planner::new(profile, [delta.x, delta.y], now)?;
            let MotionCommand::Speed(speed) = planner.sample(now, [0, 0], false)? else {
                return Err(invalid("nonzero motion unexpectedly stopped before start"));
            };
            let speed = speed as i32;
            if speed != before.speed {
                // A failure can follow a partially completed hardware write.
                self.motor_speed = Some(before.speed);
                self.io.motor_speed(speed)?;
            }
            let travel_ms = u64::from(delta.x.unsigned_abs().max(delta.y.unsigned_abs())) * 10;
            let deadline_ms = now
                .checked_add(travel_ms + 2000)
                .ok_or_else(|| invalid("motor timer overflow"))?;
            // Exactly one kernel target. Updating speed never reissues Move.
            self.io.motor_move(RelativeMove {
                x: delta.x,
                y: delta.y,
                speed,
            })?;
            self.motion = Some(ActiveMotion {
                planner,
                start: before.position,
                target,
                original_speed: before.speed,
                commanded_speed: speed,
                deadline_ms,
            });
            Ok(before)
        })();
        match result {
            Ok(status) => Ok(status),
            Err(error) => combined(Err(error), self.finish_motor()),
        }
    }
    fn set_profiled_max_speed(&mut self, speed: u32) -> io::Result<()> {
        self.ready()?;
        self.motion
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "no active motor target"))?
            .planner
            .set_max_speed(speed)
    }
    fn retarget_profiled_move(&mut self, target: Position, axes: [i32; 2]) -> io::Result<bool> {
        self.ready()?;
        let mut active = self
            .motion
            .take()
            .ok_or_else(|| io::Error::other("no active target"))?;
        let result = (|| {
            let (before, target) = self.io.motor_retarget(target, axes)?;
            if before.speed != active.commanded_speed || !before.owner_open {
                return Err(io::Error::other("retarget ownership/speed changed"));
            }
            let delta = [target.x - before.position.x, target.y - before.position.y];
            if delta == [0, 0] {
                return Ok(false);
            }
            let now = self.io.now_ms();
            active.planner.retarget(delta, now)?;
            active.start = before.position;
            active.target = target;
            active.deadline_ms =
                now + u64::from(delta[0].unsigned_abs().max(delta[1].unsigned_abs())) * 10 + 2000;
            Ok(true)
        })();
        match result {
            Ok(true) => {
                self.motion = Some(active);
                Ok(true)
            }
            Ok(false) => {
                self.finish_motor()?;
                Ok(false)
            }
            Err(e) => combined(Err(e), self.finish_motor()),
        }
    }
    fn request_brake(&mut self) -> io::Result<()> {
        self.ready()?;
        if let Some(active) = &mut self.motion {
            active.planner.request_brake();
        }
        Ok(())
    }
    fn poll_profiled_move(&mut self, cancel: &Cancellation) -> io::Result<Option<MotorStatus>> {
        self.ready()?;
        let mut active = self
            .motion
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "no active motor target"))?;
        // Urgent cancellation always calls Stop before reading another status or
        // updating speed; normal release uses request_brake instead.
        if cancel.is_cancelled() {
            return combined(Err(interrupted()), self.finish_motor());
        }
        let result = (|| {
            let status = self.io.motor_status()?;
            validate_motor(status)?;
            if !status.owner_open || status.speed != active.commanded_speed {
                return Err(io::Error::other(
                    "active motor ownership or speed readback differs",
                ));
            }
            let now = self.io.now_ms();
            if now > active.deadline_ms {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "profiled motor completion deadline exceeded",
                ));
            }
            let travelled = [
                status.position.x - active.start.x,
                status.position.y - active.start.y,
            ];
            let command = active.planner.sample(now, travelled, false)?;
            if !status.moving {
                if !active.planner.is_braking() && status.position != active.target {
                    return Err(io::Error::other(
                        "profiled motor endpoint differs from requested raw steps",
                    ));
                }
                return Ok(true);
            }
            if cancel.is_cancelled() {
                return Err(interrupted());
            }
            match command {
                MotionCommand::Stop => Ok(true),
                MotionCommand::Speed(speed) => {
                    let speed = speed as i32;
                    if speed != active.commanded_speed {
                        self.motor_speed = Some(active.original_speed);
                        self.io.motor_speed(speed)?;
                        active.commanded_speed = speed;
                    }
                    Ok(false)
                }
            }
        })();
        match result {
            Ok(false) => {
                self.motion = Some(active);
                Ok(None)
            }
            Ok(true) => {
                self.finish_motor()?;
                self.last_motor_status
                    .map(Some)
                    .ok_or_else(|| io::Error::other("motor cleanup omitted final status"))
            }
            Err(error) => combined(Err(error), self.finish_motor()),
        }
    }
    fn set_led(&mut self, led: Led, high: bool) -> io::Result<()> {
        self.ready()?;
        self.io.refuse_vendor(&["miio_devicekit"])?;
        let pin = match led {
            Led::Yellow => 8,
            Led::Blue => 9,
        };
        self.claim(pin)?;
        self.io.gpio_write(pin, high)?;
        if self.io.gpio_read(pin)? != high {
            return Err(io::Error::other("LED readback differs"));
        }
        Ok(())
    }
    fn coils_off(&mut self) -> io::Result<()> {
        let mut errors = Vec::new();
        for pin in [51, 54] {
            let index = PINS.iter().position(|p| *p == pin).unwrap();
            if self.original[index].is_some()
                && let Err(e) = self.io.gpio_write(pin, false).and_then(|()| {
                    if self.io.gpio_read(pin)? {
                        Err(io::Error::other("coil still high"))
                    } else {
                        Ok(())
                    }
                })
            {
                errors.push(format!("GPIO{pin} off: {e}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            self.poisoned = true;
            Err(io::Error::other(errors.join("; ")))
        }
    }
    fn pulse_ircut(&mut self, mode: IrCut, cancel: &Cancellation) -> io::Result<()> {
        self.ready()?;
        self.io.refuse_vendor(&["ingenic_algo"])?;
        if cancel.is_cancelled() {
            return Err(interrupted());
        }
        // Both descriptors and low-state checks precede either high write.
        if self.io.gpio_read(51)? || self.io.gpio_read(54)? {
            return Err(busy("IR-cut bridge is already energized"));
        }
        self.claim(51)?;
        self.claim(54)?;
        self.last_filter_command = None;
        let high = match mode {
            IrCut::Day => 51,
            IrCut::Night => 54,
        };
        let result = if cancel.is_cancelled() {
            Err(interrupted())
        } else {
            self.io
                .gpio_write(high, true)
                .and_then(|()| self.wait(500, cancel))
        };
        let result = combined(result, self.coils_off());
        if result.is_ok() {
            self.last_filter_command = Some(mode);
        }
        result
    }
    fn illumination(&mut self, high: bool) -> io::Result<()> {
        self.ready()?;
        self.io.refuse_vendor(&["ingenic_algo"])?;
        self.claim(49)?;
        self.io.gpio_write(49, high)?;
        if self.io.gpio_read(49)? != high {
            return Err(io::Error::other("IR illumination readback differs"));
        }
        Ok(())
    }
    fn illumination_for(&mut self, duration: Duration, cancel: &Cancellation) -> io::Result<()> {
        let milliseconds = duration.as_millis();
        if !(1..=30_000).contains(&milliseconds) {
            return Err(invalid("IR light interval must be1..30000ms"));
        }
        if cancel.is_cancelled() {
            return Err(interrupted());
        }
        let result = self
            .illumination(true)
            .and_then(|()| self.wait(milliseconds as u64, cancel));
        // If claim succeeded, extinguish even when the enable write failed.
        let off = if self.original[2].is_some() {
            self.io.gpio_write(49, false).and_then(|()| {
                if self.io.gpio_read(49)? {
                    Err(io::Error::other(
                        "IR illumination remains high after timed interval",
                    ))
                } else {
                    Ok(())
                }
            })
        } else {
            Ok(())
        };
        if off.is_err() {
            self.poisoned = true;
        }
        combined(result, off)
    }
    fn speaker(&mut self, on: bool) -> io::Result<()> {
        self.ready()?;
        if on && !self.speaker_owned {
            self.io.refuse_vendor(&["fetch_av"])?;
            // Cleanup must disable even if configure/enable partially failed.
            self.speaker_owned = true;
            if let Err(e) = self.io.speaker_claim() {
                self.poisoned = true;
                return Err(e);
            }
        }
        if self.speaker_owned
            && let Err(e) = self.io.speaker_gate(on)
        {
            self.poisoned = true;
            return Err(e);
        }
        Ok(())
    }
    fn shutdown(&mut self) -> io::Result<()> {
        let mut errors = Vec::new();
        if let Err(e) = self.finish_motor() {
            errors.push(e.to_string());
        }
        if let Err(e) = self.coils_off() {
            errors.push(e.to_string());
        }
        for (index, pin) in PINS.iter().copied().enumerate() {
            if let Some(original) = self.original[index] {
                let value = if pin == 8 || pin == 9 {
                    original
                } else {
                    false
                };
                match self.io.gpio_write(pin, value).and_then(|()| {
                    if self.io.gpio_read(pin)? != value {
                        Err(io::Error::other("GPIO cleanup readback differs"))
                    } else {
                        Ok(())
                    }
                }) {
                    Ok(()) => {
                        self.io.gpio_release(pin);
                        self.original[index] = None;
                    }
                    Err(e) => errors.push(format!("GPIO{pin} cleanup: {e}")),
                }
            }
        }
        if self.speaker_owned {
            match self.io.speaker_gate(false) {
                Ok(()) => {
                    self.io.speaker_release();
                    self.speaker_owned = false;
                }
                Err(e) => errors.push(format!("speaker disable: {e}")),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            self.poisoned = true;
            Err(io::Error::other(format!(
                "board cleanup incomplete; ownership retained; recovery required: {}",
                errors.join("; ")
            )))
        }
    }
}

/// Owns the board control lock. Not an SDK/media owner or an automatic day/night policy.
pub struct Board {
    controller: Option<Controller<backend::Target>>,
}
impl Board {
    pub fn open() -> io::Result<Self> {
        Ok(Self {
            controller: Some(Controller::new(backend::Target::open()?)),
        })
    }
    fn controller(&mut self) -> io::Result<&mut Controller<backend::Target>> {
        self.controller
            .as_mut()
            .ok_or_else(|| io::Error::other("board is closed"))
    }
    pub fn motor_status(&self) -> io::Result<MotorStatus> {
        backend::motor_diagnostics()
    }
    pub fn begin_home(&mut self) -> io::Result<()> {
        self.controller()?.begin_home()
    }
    pub fn poll_home(&mut self, cancel: &Cancellation) -> io::Result<HomeProgress> {
        self.controller()?.poll_home(cancel)
    }
    pub fn move_relative(
        &mut self,
        movement: RelativeMove,
        cancel: &Cancellation,
    ) -> io::Result<MotorStatus> {
        self.controller()?.move_relative(movement, cancel)
    }
    /// Start one endpoint-bounded move while retaining the exclusive motor fd.
    /// Full qualified axis range is available; legacy move_relative stays +/-256.
    /// Returns the observed pre-move status. Poll at least every10ms while active;
    /// the caller must enforce its hold/deadman deadline with urgent cancellation.
    pub fn begin_profiled_move(
        &mut self,
        delta: Position,
        profile: Profile,
    ) -> io::Result<MotorStatus> {
        self.controller()?.begin_profiled_move(delta, profile)
    }
    /// None means active; Some is actual stopped/restored status after cleanup.
    /// Cancellation returns Interrupted after immediate Stop/cleanup. Errors must
    /// be handled; cleanup failure poisons ownership rather than releasing it.
    pub fn poll_profiled_move(&mut self, cancel: &Cancellation) -> io::Result<Option<MotorStatus>> {
        self.controller()?.poll_profiled_move(cancel)
    }
    /// Update an active target's cap without I/O. Lower caps apply at the next
    /// poll; raised caps retain acceleration slew. Inactive returns NotConnected.
    pub fn set_profiled_max_speed(&mut self, speed: u32) -> io::Result<()> {
        self.controller()?.set_profiled_max_speed(speed)
    }
    pub fn retarget_profiled_move(&mut self, target: Position, axes: [i32; 2]) -> io::Result<bool> {
        self.controller()?.retarget_profiled_move(target, axes)
    }
    /// Normal release ramps down and then stops; repeated release is harmless.
    pub fn request_brake(&mut self) -> io::Result<()> {
        self.controller()?.request_brake()
    }
    /// Stop only a motor this object currently owns. Never steals a stock-owned fd.
    pub fn stop(&mut self) -> io::Result<()> {
        self.controller()?.finish_motor()
    }
    pub fn set_led(&mut self, led: Led, on: bool) -> io::Result<()> {
        self.controller()?.set_led(led, on)
    }
    pub fn pulse_ircut(&mut self, mode: IrCut, cancel: &Cancellation) -> io::Result<()> {
        self.controller()?.pulse_ircut(mode, cancel)
    }
    /// Last completed pulse command; physical position is not observable from the GPIOs.
    pub fn last_filter_command(&self) -> Option<IrCut> {
        self.controller.as_ref().and_then(|c| c.last_filter_command)
    }
    pub fn set_ir_illumination(&mut self, on: bool) -> io::Result<()> {
        self.controller()?.illumination(on)
    }
    pub fn ir_illumination_for(
        &mut self,
        duration: Duration,
        cancel: &Cancellation,
    ) -> io::Result<()> {
        self.controller()?.illumination_for(duration, cancel)
    }
    /// Amplifier only. Media must enable after PCM setup and disable before teardown.
    /// No status getter/sample I/O is implied; do not use beside vendor playback.
    pub fn speaker_gate(&mut self, on: bool) -> io::Result<()> {
        self.controller()?.speaker(on)
    }
    pub fn shutdown(&mut self) -> io::Result<()> {
        self.controller()?.shutdown()
    }
}
impl Drop for Board {
    fn drop(&mut self) {
        if let Some(mut controller) = self.controller.take()
            && controller.shutdown().is_err()
        {
            // Retain lock and handles rather than make uncertain hardware available
            // to another owner inside this process. Explicit shutdown reports why.
            let _ = Box::leak(Box::new(controller));
        }
    }
}

pub fn snapshot() -> io::Result<BoardSnapshot> {
    backend::snapshot()
}
/// Auto night mode uses ISP exposure/gain statistics; no standalone lux ADC ABI is proven.
pub fn ambient_light() -> io::Result<f32> {
    Err(unsupported(
        "ambient light requires the qualified media exposure adapter; no board lux sensor mapped",
    ))
}

#[cfg(all(target_os = "linux", target_arch = "mips"))]
#[path = "board/linux.rs"]
mod backend;

#[cfg(not(all(target_os = "linux", target_arch = "mips")))]
mod backend {
    use super::*;
    pub(super) struct Target;
    fn unavailable<T>() -> io::Result<T> {
        Err(unsupported(
            "board hardware is available only on the qualified MIPS camera",
        ))
    }
    impl Target {
        pub(super) fn open() -> io::Result<Self> {
            unavailable()
        }
    }
    pub(super) fn motor_diagnostics() -> io::Result<MotorStatus> {
        unavailable()
    }
    pub(super) fn snapshot() -> io::Result<BoardSnapshot> {
        unavailable()
    }
    impl Hardware for Target {
        fn refuse_vendor(&self, _: &[&str]) -> io::Result<()> {
            unavailable()
        }
        fn motor_claim(&mut self) -> io::Result<MotorStatus> {
            unavailable()
        }
        fn motor_status(&mut self) -> io::Result<MotorStatus> {
            unavailable()
        }
        fn motor_speed(&mut self, _: i32) -> io::Result<()> {
            unavailable()
        }
        fn motor_move(&mut self, _: RelativeMove) -> io::Result<()> {
            unavailable()
        }
        fn motor_stop(&mut self) -> io::Result<()> {
            unavailable()
        }
        fn motor_release(&mut self) {}
        fn gpio_claim(&mut self, _: u32) -> io::Result<bool> {
            unavailable()
        }
        fn gpio_write(&mut self, _: u32, _: bool) -> io::Result<()> {
            unavailable()
        }
        fn gpio_read(&mut self, _: u32) -> io::Result<bool> {
            unavailable()
        }
        fn gpio_release(&mut self, _: u32) {}
        fn speaker_claim(&mut self) -> io::Result<()> {
            unavailable()
        }
        fn speaker_gate(&mut self, _: bool) -> io::Result<()> {
            unavailable()
        }
        fn speaker_release(&mut self) {}
        fn now_ms(&self) -> u64 {
            0
        }
        fn wait_ms(&mut self, _: u64) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // Isolated policy fixtures only; no host GPIO, driver, clock wait or network I/O.
    struct Fake {
        status: MotorStatus,
        levels: [bool; 5],
        claimed: BTreeSet<u32>,
        events: Vec<String>,
        failure: Option<String>,
        now: u64,
        stalled: bool,
        wrong_endpoint: bool,
        ignore_restore: bool,
        cancel_at: Option<(u64, Cancellation)>,
        vendor: bool,
        seed_pending: bool,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                status: MotorStatus {
                    position: Position { x: 1000, y: 300 },
                    moving: false,
                    speed: 200,
                    owner_open: false,
                    max_x: PAN_TRAVEL,
                    max_y: TILT_TRAVEL,
                },
                levels: [true, false, false, false, false],
                claimed: BTreeSet::new(),
                events: Vec::new(),
                failure: None,
                now: 0,
                stalled: false,
                wrong_endpoint: false,
                ignore_restore: false,
                cancel_at: None,
                vendor: false,
                seed_pending: false,
            }
        }
    }
    impl Fake {
        fn event(&mut self, value: String) -> io::Result<()> {
            self.events.push(value.clone());
            if self.failure.as_ref() == Some(&value) {
                self.failure = None;
                Err(io::Error::other(format!("injected {value}")))
            } else {
                Ok(())
            }
        }
        fn index(pin: u32) -> usize {
            PINS.iter().position(|p| *p == pin).unwrap()
        }
    }
    impl Hardware for Fake {
        fn seed_home_start(&mut self) -> io::Result<()> {
            self.event("seed 2404 713 0 0".into())?;
            self.seed_pending = true;
            self.status.position = Position {
                x: if self.wrong_endpoint { 1201 } else { 1202 },
                y: 356,
            };
            self.status.moving = false;
            Ok(())
        }
        fn seed_home_pending(&self) -> bool {
            self.seed_pending
        }
        fn seed_home_poll(&mut self) -> io::Result<Option<[i32; 4]>> {
            assert!(self.seed_pending, "second seed join");
            self.seed_pending = false;
            Ok(Some([
                2404,
                713,
                self.status.position.x,
                self.status.position.y,
            ]))
        }
        fn refuse_vendor(&self, _: &[&str]) -> io::Result<()> {
            if self.vendor {
                Err(busy("vendor fixture active"))
            } else {
                Ok(())
            }
        }
        fn motor_claim(&mut self) -> io::Result<MotorStatus> {
            if self.status.owner_open {
                return Err(busy("owned fixture"));
            }
            self.event("motor claim".into())?;
            self.status.owner_open = true;
            Ok(self.status)
        }
        fn motor_status(&mut self) -> io::Result<MotorStatus> {
            self.event("motor status".into())?;
            Ok(self.status)
        }
        fn motor_speed(&mut self, speed: i32) -> io::Result<()> {
            if !(self.ignore_restore && speed == 200) {
                self.status.speed = speed;
            }
            self.event(format!("speed {speed}"))
        }
        fn motor_move(&mut self, movement: RelativeMove) -> io::Result<()> {
            self.event("motor move".into())?;
            self.status.moving = self.stalled;
            if !self.wrong_endpoint && !self.stalled {
                self.status.position.x += movement.x;
                self.status.position.y += movement.y;
            }
            Ok(())
        }
        fn motor_retarget(
            &mut self,
            target: Position,
            axes: [i32; 2],
        ) -> io::Result<(MotorStatus, Position)> {
            self.event("motor retarget".into())?;
            retarget_at_snapshot(self, target, axes)
        }
        fn motor_stop(&mut self) -> io::Result<()> {
            self.event("motor stop".into())?;
            self.status.moving = false;
            Ok(())
        }
        fn motor_release(&mut self) {
            self.events.push("motor release".into());
            self.status.owner_open = false;
        }
        fn gpio_claim(&mut self, pin: u32) -> io::Result<bool> {
            self.event(format!("claim {pin}"))?;
            self.claimed.insert(pin);
            Ok(self.levels[Self::index(pin)])
        }
        fn gpio_write(&mut self, pin: u32, high: bool) -> io::Result<()> {
            assert!(
                self.claimed.contains(&pin),
                "write without descriptor preflight"
            );
            self.levels[Self::index(pin)] = high;
            assert!(
                !(self.levels[3] && self.levels[4]),
                "both H bridge outputs high"
            );
            // Deliberately model a write which changed hardware before returning error.
            self.event(format!("write {pin} {}", u8::from(high)))
        }
        fn gpio_read(&mut self, pin: u32) -> io::Result<bool> {
            self.event(format!("read {pin}"))?;
            Ok(self.levels[Self::index(pin)])
        }
        fn gpio_release(&mut self, pin: u32) {
            self.claimed.remove(&pin);
        }
        fn speaker_claim(&mut self) -> io::Result<()> {
            self.event("speaker claim".into())
        }
        fn speaker_gate(&mut self, on: bool) -> io::Result<()> {
            self.event(format!("speaker {}", u8::from(on)))
        }
        fn speaker_release(&mut self) {
            self.events.push("speaker release".into());
        }
        fn now_ms(&self) -> u64 {
            self.now
        }
        fn wait_ms(&mut self, milliseconds: u64) {
            self.now += milliseconds;
            if let Some((at, token)) = &self.cancel_at
                && self.now >= *at
            {
                token.cancel();
            }
        }
    }

    fn moving_controller() -> Controller<Fake> {
        Controller::new(Fake {
            stalled: true,
            ..Fake::default()
        })
    }
    fn profile() -> Profile {
        Profile {
            max_speed: 200,
            acceleration: 1000,
            deceleration: 1000,
            ..Profile::default()
        }
    }
    #[test]
    fn live_vector_update_retains_owner_speed_and_frozen_inactive_axis() {
        let mut c = moving_controller();
        c.begin_profiled_move(Position { x: 1000, y: 0 }, profile())
            .unwrap();
        c.io.now = 100;
        c.poll_profiled_move(&Cancellation::default()).unwrap();
        assert_eq!(c.io.status.speed, 200);
        c.io.status.position = Position { x: 1100, y: 300 };
        assert!(
            c.retarget_profiled_move(
                Position {
                    x: PAN_TRAVEL,
                    y: 0
                },
                [1, -1]
            )
            .unwrap()
        );
        assert_eq!(
            c.motion.as_ref().unwrap().start,
            Position { x: 1100, y: 300 }
        );
        assert_eq!(c.io.status.speed, 200);
        c.io.status.position = Position { x: 1200, y: 250 };
        c.retarget_profiled_move(
            Position {
                x: PAN_TRAVEL,
                y: 0,
            },
            [1, 0],
        )
        .unwrap();
        assert_eq!(
            c.motion.as_ref().unwrap().target,
            Position {
                x: PAN_TRAVEL,
                y: 250
            }
        );
        assert_eq!(
            c.io.events
                .iter()
                .filter(|e| e.as_str() == "motor claim")
                .count(),
            1
        );
        assert!(
            !c.io
                .events
                .iter()
                .any(|e| e == "motor stop" || e == "motor release")
        );
        c.request_brake().unwrap();
        c.io.now = 110;
        c.poll_profiled_move(&Cancellation::default()).unwrap();
        assert_eq!(c.io.status.speed, 190);
        c.retarget_profiled_move(
            Position {
                x: PAN_TRAVEL,
                y: 0,
            },
            [1, -1],
        )
        .unwrap();
        c.poll_profiled_move(&Cancellation::default()).unwrap();
        assert_eq!(c.io.status.speed, 190);
        assert!(
            c.retarget_profiled_move(
                Position {
                    x: PAN_TRAVEL + 1,
                    y: 0
                },
                [1, -1]
            )
            .is_err()
        );
        assert!(!c.motor_owned);
        assert!(!c.io.status.moving);
    }
    #[test]
    fn bounded_home_seeks_then_validates_seed_center_and_never_uses_full_reset() {
        let mut c = Controller::new(Fake::default());
        c.begin_home().unwrap();
        assert_eq!(c.io.status.position, Position { x: -1408, y: -416 });
        assert_eq!(
            c.poll_home(&Cancellation::default()).unwrap(),
            HomeProgress::Running
        );
        let HomeProgress::Ready(at) = c.poll_home(&Cancellation::default()).unwrap() else {
            panic!()
        };
        assert_eq!(at.position, Position { x: 1202, y: 356 });
        assert!(!at.moving && !at.owner_open);
        assert_eq!(at.speed, 200);
        assert_eq!(
            c.io.events
                .iter()
                .filter(|e| e.as_str() == "seed 2404 713 0 0")
                .count(),
            1
        );
        let mut c = moving_controller();
        c.begin_home().unwrap();
        let cancel = Cancellation::default();
        cancel.cancel();
        assert_eq!(
            c.poll_home(&cancel).unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert!(!c.motor_owned && !c.io.status.moving);
        let mut c = Controller::new(Fake::default());
        c.begin_home().unwrap();
        c.poll_home(&Cancellation::default()).unwrap();
        c.io.status.position.x = 1201;
        assert!(c.poll_home(&Cancellation::default()).is_err());
        assert!(!c.motor_owned);
        let mut c = moving_controller();
        c.begin_home().unwrap();
        c.io.now = 11001;
        assert!(c.poll_home(&Cancellation::default()).is_err());
        assert!(!c.motor_owned);
    }
    #[test]
    fn profile_holds_one_fd_and_one_target_through_speed_updates() {
        let mut controller = moving_controller();
        let before = controller
            .begin_profiled_move(Position { x: 1000, y: 0 }, profile())
            .unwrap();
        assert_eq!(before.position, Position { x: 1000, y: 300 });
        assert!(controller.motor_owned && controller.io.status.owner_open);
        assert_eq!(controller.io.status.speed, 100);
        assert!(
            controller
                .begin_profiled_move(Position { x: -10, y: 0 }, profile())
                .is_err()
        );
        controller.io.now = 100;
        controller.io.status.position.x = 1010;
        assert!(
            controller
                .poll_profiled_move(&Cancellation::default())
                .unwrap()
                .is_none()
        );
        assert_eq!(controller.io.status.speed, 200);
        controller.io.now = 200;
        controller.io.status.position.x = 1999;
        assert!(
            controller
                .poll_profiled_move(&Cancellation::default())
                .unwrap()
                .is_none()
        );
        assert!(controller.io.status.speed < 120);
        controller.io.status.position.x = 2000;
        controller.io.status.moving = false;
        let after = controller
            .poll_profiled_move(&Cancellation::default())
            .unwrap()
            .unwrap();
        assert_eq!(after.position.x, 2000);
        assert_eq!(after.speed, 200);
        assert!(!after.owner_open && !after.moving);
        assert!(!controller.motor_owned && controller.motion.is_none());
        assert_eq!(
            controller
                .io
                .events
                .iter()
                .filter(|e| *e == "motor claim")
                .count(),
            1
        );
        assert_eq!(
            controller
                .io
                .events
                .iter()
                .filter(|e| *e == "motor move")
                .count(),
            1
        );
        assert_eq!(
            controller
                .io
                .events
                .iter()
                .filter(|e| *e == "motor release")
                .count(),
            1
        );
    }
    #[test]
    fn profile_full_axis_bounds_do_not_expand_legacy_move_permission() {
        let mut controller = moving_controller();
        controller
            .begin_profiled_move(
                Position {
                    x: PAN_TRAVEL - 1000,
                    y: TILT_TRAVEL - 300,
                },
                profile(),
            )
            .unwrap();
        controller.finish_motor().unwrap();
        assert!(RelativeMove::new(1404, 413, 200).is_err());
        for delta in [
            Position {
                x: PAN_TRAVEL - 999,
                y: 0,
            },
            Position {
                x: 0,
                y: TILT_TRAVEL - 299,
            },
            Position { x: -1001, y: 0 },
        ] {
            let mut controller = moving_controller();
            assert!(controller.begin_profiled_move(delta, profile()).is_err());
            assert!(!controller.io.events.iter().any(|e| e == "motor move"));
            assert!(!controller.motor_owned && !controller.io.status.owner_open);
        }
    }
    #[test]
    fn profile_cap_updates_are_deferred_to_poll_and_preserve_target_owner() {
        let mut controller = moving_controller();
        assert_eq!(
            controller.set_profiled_max_speed(200).unwrap_err().kind(),
            io::ErrorKind::NotConnected
        );
        let profile = Profile {
            max_speed: 900,
            acceleration: 1000,
            ..profile()
        };
        controller
            .begin_profiled_move(Position { x: 1000, y: 0 }, profile)
            .unwrap();
        for now in (100..=800).step_by(100) {
            controller.io.now = now;
            controller
                .poll_profiled_move(&Cancellation::default())
                .unwrap();
        }
        assert_eq!(controller.io.status.speed, 900);
        let before = controller.io.events.len();
        controller.set_profiled_max_speed(200).unwrap();
        assert!(controller.set_profiled_max_speed(99).is_err());
        assert_eq!(controller.io.events.len(), before);
        controller
            .poll_profiled_move(&Cancellation::default())
            .unwrap();
        assert_eq!(controller.io.status.speed, 200);
        controller.set_profiled_max_speed(900).unwrap();
        controller.io.now = 810;
        controller
            .poll_profiled_move(&Cancellation::default())
            .unwrap();
        assert_eq!(controller.io.status.speed, 210);
        assert_eq!(
            controller
                .io
                .events
                .iter()
                .filter(|e| *e == "motor claim")
                .count(),
            1
        );
        assert_eq!(
            controller
                .io
                .events
                .iter()
                .filter(|e| *e == "motor move")
                .count(),
            1
        );
        controller.finish_motor().unwrap();
        assert_eq!(controller.io.status.speed, 200);
        assert!(!controller.motor_owned && !controller.io.status.owner_open);
    }
    #[test]
    fn profile_normal_release_brakes_and_returns_measured_stop_position() {
        let mut controller = moving_controller();
        controller
            .begin_profiled_move(Position { x: 1000, y: 0 }, profile())
            .unwrap();
        controller.io.now = 100;
        controller.io.status.position.x = 1010;
        controller
            .poll_profiled_move(&Cancellation::default())
            .unwrap();
        controller.request_brake().unwrap();
        controller.io.now = 120;
        assert!(
            controller
                .poll_profiled_move(&Cancellation::default())
                .unwrap()
                .is_none()
        );
        assert_eq!(controller.io.status.speed, 180);
        controller.io.now = 220;
        controller.io.status.position.x = 1020;
        let status = controller
            .poll_profiled_move(&Cancellation::default())
            .unwrap()
            .unwrap();
        assert_eq!(status.position.x, 1020);
        assert!(!status.moving && !status.owner_open);
        assert_eq!(status.speed, 200);
        controller.request_brake().unwrap();
    }
    #[test]
    fn profile_urgent_cancel_stops_before_status_or_speed_work() {
        let mut controller = moving_controller();
        controller
            .begin_profiled_move(Position { x: 1000, y: 0 }, profile())
            .unwrap();
        let before = controller.io.events.len();
        let cancellation = Cancellation::default();
        cancellation.cancel();
        assert_eq!(
            controller
                .poll_profiled_move(&cancellation)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(controller.io.events[before], "motor stop");
        assert!(!controller.motor_owned && !controller.io.status.owner_open);
    }
    #[test]
    fn profile_speed_or_status_failure_stops_restores_and_releases() {
        for failure in ["speed 100", "motor move", "speed 200", "motor status"] {
            let mut controller = moving_controller();
            if failure == "speed 100" || failure == "motor move" {
                controller.io.failure = Some(failure.into());
                assert!(
                    controller
                        .begin_profiled_move(Position { x: 1000, y: 0 }, profile())
                        .is_err()
                );
            } else {
                controller
                    .begin_profiled_move(Position { x: 1000, y: 0 }, profile())
                    .unwrap();
                controller.io.failure = Some(failure.into());
                controller.io.now = 100;
                assert!(
                    controller
                        .poll_profiled_move(&Cancellation::default())
                        .is_err()
                );
            }
            assert!(!controller.motor_owned && !controller.io.status.owner_open);
            assert_eq!(controller.io.status.speed, 200);
            assert!(controller.io.events.iter().any(|e| e == "motor stop"));
        }
    }
    #[test]
    fn profile_bad_progress_deadline_or_endpoint_stops_without_reissuing_move() {
        for case in [0, 1, 2] {
            let mut controller = moving_controller();
            controller
                .begin_profiled_move(Position { x: 1000, y: 0 }, profile())
                .unwrap();
            match case {
                0 => controller.io.status.position.x = 999,
                1 => controller.io.now = 12001,
                _ => controller.io.status.moving = false,
            }
            assert!(
                controller
                    .poll_profiled_move(&Cancellation::default())
                    .is_err()
            );
            assert!(!controller.motor_owned && !controller.io.status.owner_open);
            assert_eq!(
                controller
                    .io
                    .events
                    .iter()
                    .filter(|e| *e == "motor move")
                    .count(),
                1
            );
        }
    }
    #[test]
    fn profile_cleanup_failure_retains_ownership_and_poison_until_verified_shutdown() {
        for failed_stop in [true, false] {
            let mut controller = moving_controller();
            controller
                .begin_profiled_move(Position { x: 1000, y: 0 }, profile())
                .unwrap();
            if failed_stop {
                controller.io.failure = Some("motor stop".into());
            } else {
                controller.io.ignore_restore = true;
            }
            let cancellation = Cancellation::default();
            cancellation.cancel();
            assert!(controller.poll_profiled_move(&cancellation).is_err());
            assert!(
                controller.motor_owned && controller.io.status.owner_open && controller.poisoned
            );
            assert!(!controller.io.events.iter().any(|e| e == "motor release"));
            assert!(
                controller
                    .begin_profiled_move(Position { x: 10, y: 0 }, profile())
                    .is_err()
            );
            controller.io.ignore_restore = false;
            controller.shutdown().unwrap();
            assert!(!controller.motor_owned && !controller.io.status.owner_open);
        }
    }
    #[test]
    fn parses_identity_and_rejects_ambiguous_diagnostics() {
        let text = "The version of Motor driver is H20171204a\nMotor driver is closed\nThe status of motor is stop\nThe pos of motor is (1000, 300)\nThe speed of motor is 200\n## motor is Horizontal motor ##\nmax steps 2404\n## motor is Vertical motor ##\nmax steps 713\n";
        assert_eq!(
            parse_motor_diagnostics(text).unwrap(),
            Fake::default().status
        );
        for bad in [
            text.replace("H20171204a", "unknown"),
            text.replace("2404", "2403"),
            text.replace("1000, 300", "-1, 300"),
            text.to_owned() + "Motor driver is opened\n",
        ] {
            assert!(parse_motor_diagnostics(&bad).is_err());
        }
    }
    #[test]
    fn movement_limits_cover_edges_overflow_and_public_struct_literals() {
        let mut status = Fake::default().status;
        status.position = Position {
            x: 0,
            y: TILT_TRAVEL,
        };
        assert!(
            RelativeMove::new(-1, 0, 200)
                .unwrap()
                .endpoint(status)
                .is_err()
        );
        assert!(
            RelativeMove::new(0, 1, 200)
                .unwrap()
                .endpoint(status)
                .is_err()
        );
        assert_eq!(
            RelativeMove::new(1, -1, 300)
                .unwrap()
                .endpoint(status)
                .unwrap(),
            Position {
                x: 1,
                y: TILT_TRAVEL - 1
            }
        );
        for movement in [
            RelativeMove {
                x: i32::MAX,
                y: 1,
                speed: 200,
            },
            RelativeMove {
                x: 0,
                y: 0,
                speed: 200,
            },
            RelativeMove {
                x: 1,
                y: 0,
                speed: 901,
            },
        ] {
            assert!(movement.endpoint(status).is_err());
        }
    }
    #[test]
    fn move_reports_post_cleanup_state_and_restores_speed() {
        let mut controller = Controller::new(Fake::default());
        let status = controller
            .move_relative(
                RelativeMove::new(32, -16, 250).unwrap(),
                &Cancellation::default(),
            )
            .unwrap();
        assert_eq!(status.position, Position { x: 1032, y: 284 });
        assert!(!status.owner_open && !status.moving);
        assert_eq!(status.speed, 200);
        assert_eq!(controller.io.status.speed, 200);
        assert!(!controller.io.status.owner_open);
        let stop = controller
            .io
            .events
            .iter()
            .position(|s| s == "motor stop")
            .unwrap();
        let restore = controller
            .io
            .events
            .iter()
            .position(|s| s == "speed 200")
            .unwrap();
        assert!(stop < restore);
    }
    #[test]
    fn failed_move_or_wrong_endpoint_never_attempts_an_automatic_return() {
        for failure in [
            Some("motor move"),
            Some("motor status"),
            Some("speed 250"),
            None,
        ] {
            let fake = Fake {
                failure: failure.map(str::to_owned),
                wrong_endpoint: failure.is_none(),
                ..Fake::default()
            };
            let mut controller = Controller::new(fake);
            assert!(
                controller
                    .move_relative(
                        RelativeMove::new(32, 0, 250).unwrap(),
                        &Cancellation::default()
                    )
                    .is_err()
            );
            assert!(!controller.io.status.owner_open && !controller.io.status.moving);
            assert_eq!(controller.io.status.speed, 200);
            assert!(
                controller
                    .io
                    .events
                    .iter()
                    .filter(|e| e.as_str() == "motor move")
                    .count()
                    <= 1
            );
        }
    }
    #[test]
    fn ignored_speed_restore_retains_motor_instead_of_reporting_a_false_success() {
        let mut controller = Controller::new(Fake {
            ignore_restore: true,
            ..Fake::default()
        });
        assert!(
            controller
                .move_relative(
                    RelativeMove::new(32, 0, 250).unwrap(),
                    &Cancellation::default()
                )
                .is_err()
        );
        assert!(controller.motor_owned && controller.io.status.owner_open);
        assert_eq!(controller.motor_speed, Some(200));
        controller.io.ignore_restore = false;
        controller.shutdown().unwrap();
        assert_eq!(controller.io.status.speed, 200);
        assert!(!controller.io.status.owner_open);
    }
    #[test]
    fn cancellation_and_deadline_stop_and_release_motor() {
        for cancel_after in [Some(20), None] {
            let token = Cancellation::default();
            let mut fake = Fake {
                stalled: true,
                ..Fake::default()
            };
            fake.cancel_at = cancel_after.map(|at| (at, token.clone()));
            let mut controller = Controller::new(fake);
            let error = controller
                .move_relative(RelativeMove::new(32, 0, 200).unwrap(), &token)
                .unwrap_err();
            assert_eq!(
                error.kind(),
                if cancel_after.is_some() {
                    io::ErrorKind::Interrupted
                } else {
                    io::ErrorKind::TimedOut
                }
            );
            assert!(!controller.io.status.owner_open && !controller.io.status.moving);
        }
    }
    #[test]
    fn failed_stop_retains_ownership_and_blocks_further_actuation() {
        let fake = Fake {
            failure: Some("motor stop".into()),
            ..Fake::default()
        };
        let mut controller = Controller::new(fake);
        assert!(
            controller
                .move_relative(
                    RelativeMove::new(32, 0, 200).unwrap(),
                    &Cancellation::default()
                )
                .is_err()
        );
        assert!(controller.motor_owned && controller.io.status.owner_open);
        assert!(controller.set_led(Led::Blue, true).is_err());
        controller.shutdown().unwrap(); // Stop is idempotent and the one-shot fault cleared.
        assert!(!controller.io.status.owner_open);
    }
    #[test]
    fn filter_preflights_both_descriptors_then_pulses_only_one_for_500ms() {
        for mode in [IrCut::Day, IrCut::Night] {
            let mut controller = Controller::new(Fake::default());
            controller
                .pulse_ircut(mode, &Cancellation::default())
                .unwrap();
            assert_eq!(controller.io.now, 500);
            assert!(!controller.io.levels[3] && !controller.io.levels[4]);
            assert_eq!(controller.last_filter_command, Some(mode));
            let high_pin = if mode == IrCut::Day { 51 } else { 54 };
            let first_high = controller
                .io
                .events
                .iter()
                .position(|e| e == &format!("write {high_pin} 1"))
                .unwrap();
            for pin in [51, 54] {
                assert!(
                    controller
                        .io
                        .events
                        .iter()
                        .position(|e| e == &format!("claim {pin}"))
                        .unwrap()
                        < first_high
                );
            }
        }
    }
    #[test]
    fn already_energized_filter_is_refused_without_mutating_it_on_shutdown() {
        let mut fake = Fake::default();
        fake.levels[3] = true;
        let mut controller = Controller::new(fake);
        assert!(
            controller
                .pulse_ircut(IrCut::Day, &Cancellation::default())
                .is_err()
        );
        controller.shutdown().unwrap();
        assert!(controller.io.levels[3]);
        assert!(!controller.io.events.iter().any(|e| e.starts_with("write")));
    }
    #[test]
    fn partial_enable_cancel_or_preflight_failure_always_leaves_owned_coils_low() {
        for failure in ["claim 54", "write 54 1", "read 54"] {
            let fake = Fake {
                failure: Some(failure.into()),
                ..Fake::default()
            };
            let mut controller = Controller::new(fake);
            assert!(
                controller
                    .pulse_ircut(IrCut::Night, &Cancellation::default())
                    .is_err()
            );
            controller.shutdown().unwrap();
            assert!(!controller.io.levels[3] && !controller.io.levels[4]);
        }
        let token = Cancellation::default();
        let fake = Fake {
            cancel_at: Some((20, token.clone())),
            ..Fake::default()
        };
        let mut controller = Controller::new(fake);
        assert!(controller.pulse_ircut(IrCut::Night, &token).is_err());
        assert_eq!(controller.io.now, 20);
        assert!(!controller.io.levels[3] && !controller.io.levels[4]);
        assert_eq!(controller.last_filter_command, None);
    }
    #[test]
    fn both_coil_cleanup_writes_are_attempted_after_one_fails() {
        let fake = Fake {
            failure: Some("write 51 0".into()),
            ..Fake::default()
        };
        let mut controller = Controller::new(fake);
        assert!(
            controller
                .pulse_ircut(IrCut::Night, &Cancellation::default())
                .is_err()
        );
        assert!(controller.io.events.iter().any(|e| e == "write 54 0"));
        assert!(controller.poisoned);
        controller.shutdown().unwrap();
    }
    #[test]
    fn shutdown_restores_leds_extinguishes_ir_and_disables_speaker_before_close() {
        let mut controller = Controller::new(Fake::default());
        controller.set_led(Led::Yellow, false).unwrap();
        controller.set_led(Led::Blue, true).unwrap();
        controller.illumination(true).unwrap();
        controller.speaker(true).unwrap();
        controller.shutdown().unwrap();
        assert_eq!(controller.io.levels, [true, false, false, false, false]);
        let events = &controller.io.events;
        let disable = events.iter().rposition(|e| e == "speaker 0").unwrap();
        let close = events.iter().rposition(|e| e == "speaker release").unwrap();
        assert!(disable < close);
    }
    #[test]
    fn timed_ir_has_explicit_limit_and_cancel_extinguishes_it() {
        let token = Cancellation::default();
        let fake = Fake {
            cancel_at: Some((30, token.clone())),
            ..Fake::default()
        };
        let mut controller = Controller::new(fake);
        assert!(
            controller
                .illumination_for(Duration::from_secs(31), &token)
                .is_err()
        );
        assert!(controller.io.events.is_empty());
        assert!(
            controller
                .illumination_for(Duration::from_secs(1), &token)
                .is_err()
        );
        assert!(!controller.io.levels[2]);
    }
    #[test]
    fn vendor_presence_and_preexisting_cancellation_do_not_actuate() {
        let fake = Fake {
            vendor: true,
            ..Fake::default()
        };
        let mut controller = Controller::new(fake);
        assert!(controller.set_led(Led::Blue, true).is_err());
        assert!(
            controller
                .pulse_ircut(IrCut::Night, &Cancellation::default())
                .is_err()
        );
        assert!(controller.speaker(true).is_err());
        assert!(controller.io.events.is_empty());
        let token = Cancellation::default();
        token.cancel();
        assert!(
            controller
                .move_relative(RelativeMove::new(1, 0, 200).unwrap(), &token)
                .is_err()
        );
        assert!(controller.io.events.is_empty());
    }
    #[test]
    fn unsupported_controls_and_host_backends_never_open_hardware() {
        assert_eq!(
            ambient_light().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        #[cfg(not(all(target_os = "linux", target_arch = "mips")))]
        {
            assert!(Board::open().is_err());
            assert!(snapshot().is_err());
        }
    }
}
