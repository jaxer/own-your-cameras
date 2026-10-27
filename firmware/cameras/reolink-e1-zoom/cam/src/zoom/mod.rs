//! Zoom and focus of the YT3005 lens (AN41908 motor driver) through the `vendor_isp` items on `/dev/nvt_isp`,
//! reproduced from the vendor `device` (`Na_af_*`, item table and payloads in `docs/hardware/zoom-focus.md`).
//!
//! Every ioctl goes through [`Isp`], so sequencing (moves, waits, the autofocus search in [`af`]) is unit-tested on the host
//! against [`fake::FakeIsp`]; the real device is [`isp::NvtIsp`].

use std::time::Duration;

use crate::zoom::zf_table::ZfTable;

pub mod af;
pub mod fake;
pub mod focus_map;
pub mod isp;
pub mod zf_table;

/// Words in every payload buffer handed to the kernel. The vendor's largest struct here is `VA_DATA`'s 2052 bytes;
/// the real adapter allows only item/direction pairs whose copies fit this buffer.
pub const WORDS: usize = 1024;
/// VA (sharpness statistic) cells in `ISPT_ITEM_VA_DATA`: four 8×8 grids after the ISP id word.
pub const VA_CELLS: usize = 256;

/// `ISPT_ITEM_*` numbers (the ioctl `nr`) of `vendor_isp_{get,set}_common`.
pub mod item {
    /// get: `[0]` = ISP id (0), `[1..=256]` = VA cells (`Na_af_get_cur_va` sums them all).
    pub const VA: u32 = 30;
    pub const FOCUS: u32 = 41;
    pub const ZOOM: u32 = 42;
    pub const MISC: u32 = 43;
}

/// First payload word of the motor items: the motor command.
pub mod cmd {
    /// get on the focus item: `[3]..=[4]` = the focus window of the driver row for the current zoom
    /// (`MTR_GET_FOCUS_RANGE`); on `MOTOR_MISC` with the row index in `[1]`: that row in `[18..=20]`.
    pub const RANGE: u32 = 3;
    /// set: initialise (home) the motor; the driver refuses moves until this ran (`Na_af_motor_init`).
    pub const INIT: u32 = 1;
    /// get: `[3]` low 16 bits = position, high 16 bits = direction flag.
    pub const POSITION: u32 = 4;
    /// get on the focus item: `[3]` non-zero while either motor moves.
    pub const STATUS: u32 = 6;
    /// set on the focus item: `[1]` = target position.
    pub const FOCUS_GO: u32 = 6;
    /// set on the zoom item: `[1]` = target position (what every vendor caller uses).
    pub const ZOOM_GO: u32 = 5;
    /// set on the zoom item: `[1]` = speed 20/40/60/80/100 (`an41908_set_zoom_speed`, synchronous).
    pub const ZOOM_SPEED: u32 = 2;
    /// set on the zoom item: run toward the long end (`[1]` ≠ 0) or 0 (`an41908_press_zoom_move`,
    /// stock `Na_af_continue_move` @ 0xa5d28).
    pub const ZOOM_PRESS: u32 = 6;
    /// set on the zoom item: halt the running move at the current step (`an41908_release_zoom_move`,
    /// stock `Na_af_stop_move` @ 0xa5ea8).
    pub const ZOOM_RELEASE: u32 = 7;
}

/// Speeds the AN41908 driver distinguishes; anything else becomes 20 in the driver.
pub const ZOOM_SPEEDS: [u32; 5] = [20, 40, 60, 80, 100];

/// Mechanical travel the AN41908 driver reports at load (`an41908_init_zoom max_distance:1148`, dmesg). It only
/// accepts targets up to the last row of its focus curve (1024 on the E1 Zoom, `set out of range pos!` beyond), so this
/// is the fallback when the rows cannot be read.
pub const ZOOM_MAX: u32 = 1148;
/// `an41908_init_focus max_distance:1705`.
pub const FOCUS_MAX: u32 = 1705;

const POLL: Duration = Duration::from_millis(10);
/// How long the vendor waits for a motor before it gives up (`Na_af_wait_motor_free(…, 60)`).
pub const WAIT: Duration = Duration::from_secs(60);
/// How many VA reads one sample tries before the reading counts as unstable (`Na_af_get_cur_stable_va`).
const VA_TRIES: u32 = 300;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("nvt_isp: {0}")]
    Ioctl(String),
    #[error("{what} {pos} is beyond the driver's {max}")]
    OutOfRange {
        what: &'static str,
        pos: u32,
        max: u32,
    },
    #[error("invalid driver focus window {lo}..={hi}")]
    InvalidWindow { lo: u32, hi: u32 },
    #[error("motor still busy after {WAIT:?}")]
    MotorTimeout,
    #[error("VA never settled ({VA_TRIES} reads)")]
    VaUnstable,
}

/// The two `vendor_isp_*_common` ioctl directions plus the poll delay.
pub trait Isp {
    fn get(&mut self, item: u32, words: &mut [u32; WORDS]) -> Result<(), Error>;
    fn set(&mut self, item: u32, words: &[u32; WORDS]) -> Result<(), Error>;
    /// `vendor_isp_get_iq`: image-quality items; `words[0]` is the ISP id on the way in.
    fn get_iq(&mut self, item: u32, _words: &mut [u32; WORDS]) -> Result<(), Error> {
        Err(Error::Ioctl(format!("IQ item {item} not supported here")))
    }
    /// `vendor_isp_set_iq`.
    fn set_iq(&mut self, item: u32, _words: &[u32; WORDS]) -> Result<(), Error> {
        Err(Error::Ioctl(format!("IQ item {item} not supported here")))
    }
    /// `vendor_isp_get_ae`: auto-exposure items; `words[0]` is the ISP id on the way in.
    fn get_ae(&mut self, item: u32, _words: &mut [u32; WORDS]) -> Result<(), Error> {
        Err(Error::Ioctl(format!("AE item {item} not supported here")))
    }
    /// `vendor_isp_set_ae`.
    fn set_ae(&mut self, item: u32, _words: &[u32; WORDS]) -> Result<(), Error> {
        Err(Error::Ioctl(format!("AE item {item} not supported here")))
    }
    fn sleep(&mut self, d: Duration);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Positions {
    pub zoom: u32,
    pub focus: u32,
}

/// How a one-shot autofocus ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AfResult {
    /// Focus position it settled on.
    pub focus: u32,
    /// True when the scene had no contrast and the focus came from the `sp` calibration instead.
    pub calibrated: bool,
    /// Every (focus, VA) the search measured, in order.
    pub samples: Vec<(u32, u64)>,
}

/// One row of the focus curve the driver holds (loaded by the vendor `Na_af_load_zftable` at boot).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub zoom: u32,
    pub focus_min: u32,
    pub focus_max: u32,
}

/// Rows in the driver table (`fz_tab0..49`).
pub const ROWS: u32 = 50;

#[derive(Debug, Clone, Copy)]
struct ZoomHold {
    toward_max: bool,
    timeout: Duration,
    renewed: Duration,
    /// The halt was sent; waiting for the motor to stop.
    released: bool,
}

pub struct Lens<I: Isp> {
    isp: I,
    hold: Option<ZoomHold>,
    /// The zoom motor's speed setting, for the time a recorded zoom move takes.
    zoom_speed: u32,
    /// Recent zoom moves (start, from, end, to), linear in time, for taps made on an older picture.
    history: std::collections::VecDeque<(Duration, u32, Duration, u32)>,
}

/// Full-range (1024 steps) zoom travel per speed setting, measured (`docs/hardware/zoom-focus.md`).
const ZOOM_FULL_RANGE_MS: [(u32, u64); 5] = [
    (20, 24_200),
    (40, 13_700),
    (60, 7_200),
    (80, 5_500),
    (100, 5_300),
];
/// How much of the recorded zoom motion a tap may look back into.
const ZOOM_HISTORY: Duration = Duration::from_secs(10);

impl<I: Isp> Lens<I> {
    pub fn new(isp: I) -> Self {
        Self {
            isp,
            hold: None,
            zoom_speed: 80,
            history: std::collections::VecDeque::new(),
        }
    }

    /// Records a zoom move from `from` to `to` starting at `start`, at the current speed setting.
    fn remember_zoom(&mut self, start: Duration, from: u32, to: u32) {
        let full = ZOOM_FULL_RANGE_MS
            .iter()
            .find(|(s, _)| *s == self.zoom_speed)
            .map_or(5_500, |(_, ms)| *ms);
        let took = Duration::from_millis(u64::from(from.abs_diff(to)) * full / 1024);
        let end = start + took;
        while self
            .history
            .front()
            .is_some_and(|h| h.2 < end.saturating_sub(ZOOM_HISTORY))
        {
            self.history.pop_front();
        }
        self.history.push_back((start, from, end, to));
    }

    /// The zoom at `t` from the recorded moves: linear inside a move, the last end between moves, the first kept
    /// move's start before it; `None` with nothing recorded (the caller then uses the lens's own position).
    pub fn zoom_at(&self, t: Duration) -> Option<u32> {
        let mut last = self.history.front();
        for h in &self.history {
            if t < h.0 {
                break;
            }
            last = Some(h);
        }
        let &(start, from, end, to) = last?;
        if t >= end {
            return Some(to);
        }
        if t <= start {
            return Some(from);
        }
        let span = (end - start).as_micros().max(1);
        let done = (u128::from(from.abs_diff(to)) * (t - start).as_micros() / span) as u32;
        Some(if to >= from { from + done } else { from - done })
    }

    /// Starts (or renews, or reverses) a continuous zoom toward the long end or toward 0 with the
    /// driver's press move; it stops by itself at the end of travel. Measured (`docs/hardware/zoom-focus.md`): the halt lands on the next 64-step section, about 0.2 s after release.
    pub fn begin_hold(
        &mut self,
        toward_max: bool,
        timeout: Duration,
        now: Duration,
    ) -> Result<(), Error> {
        if let Some(h) = self.hold.as_mut()
            && h.toward_max == toward_max
            && !h.released
        {
            h.renewed = now;
            h.timeout = timeout;
            return Ok(());
        }
        if self.hold.is_some() {
            self.zoom_release()?;
        }
        self.zoom_press(toward_max)?;
        self.hold = Some(ZoomHold {
            toward_max,
            timeout,
            renewed: now,
            released: false,
        });
        Ok(())
    }

    /// Halts the running press move; `poll_hold` reports the end once the motor stopped.
    pub fn release_hold(&mut self) {
        if let Some(h) = self.hold.as_mut()
            && !h.released
        {
            h.released = true;
            if let Err(e) = self.zoom_release() {
                eprintln!("zoom: release failed: {e}");
            }
        }
    }

    pub fn holding(&self) -> bool {
        self.hold.is_some()
    }

    /// Ends the hold on release, the travel end (the motor stopped by itself) or timeout. Never waits.
    /// After `Ended` the caller settles focus for the reached zoom.
    pub fn poll_hold(&mut self, now: Duration) -> Result<crate::ptz::HoldState, Error> {
        use crate::ptz::{HoldEnd, HoldState};
        let Some(h) = self.hold else {
            return Ok(HoldState::Idle);
        };
        let poll = Duration::from_millis(20);
        if !h.released && now.saturating_sub(h.renewed) >= h.timeout {
            self.release_hold();
            if let Some(h) = self.hold.as_mut() {
                h.renewed = Duration::MAX;
            }
            return Ok(HoldState::Moving(poll));
        }
        if self.busy()? {
            return Ok(HoldState::Moving(poll));
        }
        self.hold = None;
        Ok(HoldState::Ended(match (h.released, h.renewed) {
            (true, Duration::MAX) => HoldEnd::Timeout,
            (true, _) => HoldEnd::Released,
            (false, _) => HoldEnd::Limit,
        }))
    }

    pub fn isp(&self) -> &I {
        &self.isp
    }

    /// The one `/dev/nvt_isp` owner, for the orientation and picture-control items.
    pub fn isp_mut(&mut self) -> &mut I {
        &mut self.isp
    }

    fn get(&mut self, item: u32, head: &[u32]) -> Result<Box<[u32; WORDS]>, Error> {
        let mut w = Box::new([0u32; WORDS]);
        w[..head.len()].copy_from_slice(head);
        self.isp.get(item, &mut w)?;
        Ok(w)
    }

    fn set(&mut self, item: u32, head: &[u32]) -> Result<(), Error> {
        let mut w = Box::new([0u32; WORDS]);
        w[..head.len()].copy_from_slice(head);
        self.isp.set(item, &w)
    }

    fn position(&mut self, motor: u32) -> Result<u32, Error> {
        Ok(self.get(motor, &[cmd::POSITION])?[3] & 0xffff)
    }

    pub fn positions(&mut self) -> Result<Positions, Error> {
        Ok(Positions {
            zoom: self.position(item::ZOOM)?,
            focus: self.position(item::FOCUS)?,
        })
    }

    /// Where focus can be sharp at the current zoom: the driver row's window (`Na_af_get_cur_focus_range`).
    pub fn focus_window(&mut self) -> Result<(u32, u32), Error> {
        let w = self.get(item::FOCUS, &[cmd::RANGE])?;
        Ok((w[3], w[4]))
    }

    /// True while either lens motor moves (the vendor polls the focus item for both).
    pub fn busy(&mut self) -> Result<bool, Error> {
        Ok(self.get(item::FOCUS, &[cmd::STATUS, 0, 0, 0xff])?[3] != 0)
    }

    fn wait_idle(&mut self) -> Result<(), Error> {
        self.wait_idle_with(&mut { WAIT }, &mut || Ok(()))
    }

    fn wait_idle_with(
        &mut self,
        remaining: &mut Duration,
        checkpoint: &mut dyn FnMut() -> Result<(), Error>,
    ) -> Result<(), Error> {
        loop {
            checkpoint()?;
            if !self.busy()? {
                return Ok(());
            }
            if *remaining < POLL {
                return Err(Error::MotorTimeout);
            }
            self.isp.sleep(POLL);
            *remaining -= POLL;
        }
    }

    fn checked(what: &'static str, pos: u32, max: u32) -> Result<(), Error> {
        if pos <= max {
            Ok(())
        } else {
            Err(Error::OutOfRange { what, pos, max })
        }
    }

    /// Initialises focus, then zoom, each waited for like `Na_af_motor_init`. The vendor `device` does this a while
    /// after boot; until it has, the driver drops zoom moves with `not initialized yet!` in dmesg.
    pub fn init(&mut self) -> Result<(), Error> {
        self.init_with(WAIT * 2, &mut || Ok(()))
    }

    /// One aggregate motor wait budget and an owner stop checkpoint throughout startup.
    pub fn init_with(
        &mut self,
        mut budget: Duration,
        checkpoint: &mut dyn FnMut() -> Result<(), Error>,
    ) -> Result<(), Error> {
        for motor in [item::FOCUS, item::ZOOM] {
            checkpoint()?;
            self.set(motor, &[cmd::INIT])?;
            self.wait_idle_with(&mut budget, checkpoint)?;
        }
        Ok(())
    }

    /// Install the factory lens-1 curve before homing, with exact driver readback.
    pub fn configure_factory(&mut self, zf: &ZfTable) -> Result<(), Error> {
        let rows = zf.factory_rows().map_err(|e| Error::Ioctl(e.to_string()))?;
        self.configure(&rows, zf.backlash)
    }

    /// Install `rows` as the driver's focus curve (one per ZFTable zoom, ascending, 2..49 of them) and the focus
    /// backlash, with exact driver readback. MOTOR_MISC cmd 1 writes a 16-byte row; cmd 4 writes backlash
    /// (docs/hardware/zoom-focus.md). Failure leaves startup unsuccessful; the owner must reboot, never resume
    /// partial state.
    pub fn configure(&mut self, rows: &[Row], backlash: u32) -> Result<(), Error> {
        if rows.len() < 2
            || rows.len() >= ROWS as usize
            || backlash > 255
            || rows[0].zoom != 0
            || rows
                .iter()
                .any(|r| r.zoom > ZOOM_MAX || r.focus_max > FOCUS_MAX || r.focus_min > r.focus_max)
            || rows.windows(2).any(|p| p[0].zoom >= p[1].zoom)
        {
            return Err(Error::Ioctl("unsupported lens curve".into()));
        }
        if self.busy()? {
            return Err(Error::Ioctl(
                "cannot replace calibration while motors move".into(),
            ));
        }
        let last = rows[rows.len() - 1];
        for i in 0..ROWS {
            let row = if i == ROWS - 1 {
                last
            } else {
                rows.get(i as usize).copied().unwrap_or(Row {
                    zoom: 0xffff,
                    focus_min: 0xffff,
                    focus_max: 0xffff,
                })
            };
            self.set(
                item::MISC,
                &[1, i, row.zoom, row.focus_min, row.focus_max, 0],
            )?;
            let got = self.get(item::MISC, &[cmd::RANGE, i])?;
            if (got[18], got[19], got[20]) != (row.zoom, row.focus_min, row.focus_max) {
                return Err(Error::Ioctl(format!("factory row {i} readback mismatch")));
            }
        }
        // Focus backlash from sp, zero zoom backlash, as the shipped vendor uses.
        self.set(item::MISC, &[4, backlash, 0])?;
        let got = self.get(item::MISC, &[6])?;
        if got[17] != backlash || got[18] != 0 {
            return Err(Error::Ioctl("factory backlash readback mismatch".into()));
        }
        Ok(())
    }

    /// MQTT's compound zoom/calibrated-focus operation shares one wait budget.
    pub fn zoom_focus_with(
        &mut self,
        zoom: u32,
        focus: u32,
        budget: Duration,
        checkpoint: &mut dyn FnMut() -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.zoom_focus_at(zoom, focus, budget, checkpoint, None)
    }

    /// [`Lens::zoom_focus_with`] that also records the zoom move from `now` for [`Lens::zoom_at`].
    pub fn zoom_focus_at(
        &mut self,
        zoom: u32,
        focus: u32,
        mut budget: Duration,
        checkpoint: &mut dyn FnMut() -> Result<(), Error>,
        now: Option<Duration>,
    ) -> Result<(), Error> {
        checkpoint()?;
        Self::checked("zoom", zoom, self.zoom_max()?)?;
        Self::checked("focus", focus, FOCUS_MAX)?;
        if let Some(now) = now {
            let from = self.position(item::ZOOM)?;
            self.remember_zoom(now, from, zoom);
        }
        self.set(item::ZOOM, &[cmd::ZOOM_GO, zoom])?;
        self.wait_idle_with(&mut budget, checkpoint)?;
        checkpoint()?;
        self.set(item::FOCUS, &[cmd::FOCUS_GO, focus])?;
        self.wait_idle_with(&mut budget, checkpoint)
    }

    pub fn set_zoom_speed_if_changed(&mut self, speed: u32) -> Result<(), Error> {
        if self.zoom_speed == speed {
            Ok(())
        } else {
            self.set_zoom_speed(speed)
        }
    }

    /// Zoom motor speed for later moves (one of [`ZOOM_SPEEDS`]).
    pub fn set_zoom_speed(&mut self, speed: u32) -> Result<(), Error> {
        if !ZOOM_SPEEDS.contains(&speed) {
            return Err(Error::OutOfRange {
                what: "zoom speed",
                pos: speed,
                max: 100,
            });
        }
        self.zoom_speed = speed;
        self.set(item::ZOOM, &[cmd::ZOOM_SPEED, speed])
    }

    /// Starts a continuous zoom toward the long end or toward 0; returns at once.
    pub fn zoom_press(&mut self, toward_max: bool) -> Result<(), Error> {
        self.set(item::ZOOM, &[cmd::ZOOM_PRESS, u32::from(toward_max), 0])
    }

    /// Halts a running zoom move where it is; returns at once.
    pub fn zoom_release(&mut self) -> Result<(), Error> {
        self.set(item::ZOOM, &[cmd::ZOOM_RELEASE])
    }

    /// The highest zoom the driver accepts: the last row of its focus curve.
    pub fn zoom_max(&mut self) -> Result<u32, Error> {
        Ok(self
            .driver_rows()?
            .last()
            .map_or(ZOOM_MAX, |r| r.zoom)
            .min(ZOOM_MAX))
    }

    /// Drives the zoom motor to `pos` and waits until it stops (`Na_af_zoom_move`). The driver drags the focus
    /// along its curve and lands on the next row at or above `pos`.
    pub fn zoom_to(&mut self, pos: u32) -> Result<(), Error> {
        let max = self.zoom_max()?;
        Self::checked("zoom", pos, max)?;
        self.set(item::ZOOM, &[cmd::ZOOM_GO, pos])?;
        self.wait_idle()
    }

    /// Drives the focus motor to `pos` and waits until it stops (`Na_af_focus_move`, plain branch).
    pub fn focus_to(&mut self, pos: u32) -> Result<(), Error> {
        Self::checked("focus", pos, FOCUS_MAX)?;
        self.set(item::FOCUS, &[cmd::FOCUS_GO, pos])?;
        self.wait_idle()
    }

    /// Sends the focus motor toward `pos` and returns at once; [`Lens::busy`] tells when it arrived.
    pub fn focus_start(&mut self, pos: u32) -> Result<(), Error> {
        Self::checked("focus", pos, FOCUS_MAX)?;
        self.set(item::FOCUS, &[cmd::FOCUS_GO, pos])
    }

    /// The driver's focus-curve rows up to the first unused slot or a zoom that does not increase
    /// (`MOTOR_MISC` cmd 3, row index in `[1]`, row in `[18..=20]`).
    pub fn driver_rows(&mut self) -> Result<Vec<Row>, Error> {
        let mut rows: Vec<Row> = Vec::new();
        for i in 0..ROWS {
            let w = self.get(item::MISC, &[cmd::RANGE, i])?;
            let row = Row {
                zoom: w[18],
                focus_min: w[19],
                focus_max: w[20],
            };
            if row.zoom == 0xffff || rows.last().is_some_and(|r| r.zoom >= row.zoom) {
                break;
            }
            rows.push(row);
        }
        Ok(rows)
    }

    /// Sum of the 256 VA cells for the frame the ISP last measured (`Na_af_get_cur_va`).
    pub fn va(&mut self) -> Result<u64, Error> {
        let w = self.get(item::VA, &[0])?;
        Ok(w[1..=VA_CELLS].iter().map(|&c| u64::from(c)).sum())
    }

    /// One-shot contrast autofocus to the end ([`af`] holds the search; `cam focus` runs it this way).
    pub fn autofocus(&mut self, zf: &ZfTable) -> Result<AfResult, Error> {
        let r = af::run_blocking(self, zf, &|z| zf.focus_at(z))?;
        let focus = self.position(item::FOCUS)?;
        Ok(AfResult {
            focus,
            calibrated: r.reference.is_none(),
            samples: r.samples,
        })
    }
}

#[cfg(test)]
mod hold_tests {
    use super::fake::FakeIsp;
    use super::*;
    use crate::ptz::{HoldEnd, HoldState};

    fn drive(lens: &mut Lens<FakeIsp>, mut now: Duration, release_at: Option<Duration>) -> HoldEnd {
        for _ in 0..10_000 {
            if release_at.is_some_and(|r| now >= r) {
                lens.release_hold();
            }
            match lens.poll_hold(now).unwrap() {
                HoldState::Ended(e) => return e,
                HoldState::Idle => panic!("idle before the end"),
                HoldState::Moving(_) => now += Duration::from_millis(20),
            }
        }
        panic!("never ended")
    }

    fn zoom_cmds(lens: &Lens<FakeIsp>) -> Vec<Vec<u32>> {
        lens.isp()
            .sets()
            .iter()
            .filter(|s| s.item == item::ZOOM)
            .map(|s| s.words[..2].to_vec())
            .collect()
    }

    #[test]
    fn a_held_zoom_is_one_press_move_halted_on_release() {
        let mut lens = Lens::new(FakeIsp::new(100, 0));
        lens.begin_hold(true, Duration::from_secs(20), Duration::ZERO)
            .unwrap();
        let end = drive(&mut lens, Duration::ZERO, Some(Duration::from_millis(100)));
        assert_eq!(end, HoldEnd::Released);
        assert_eq!(
            zoom_cmds(&lens),
            [vec![cmd::ZOOM_PRESS, 1], vec![cmd::ZOOM_RELEASE, 0]]
        );
        let z = lens.isp().zoom;
        assert!(z > 100 && z < 1024, "moved a little: {z}");
        assert!(!lens.holding());
    }

    #[test]
    fn the_travel_end_and_a_lost_release_both_end_the_hold() {
        let mut lens = Lens::new(FakeIsp::new(70, 0));
        lens.begin_hold(false, Duration::from_secs(20), Duration::ZERO)
            .unwrap();
        assert_eq!(drive(&mut lens, Duration::ZERO, None), HoldEnd::Limit);
        assert_eq!(lens.isp().zoom, 0);
        let mut lens = Lens::new(FakeIsp::new(0, 0));
        lens.begin_hold(true, Duration::from_millis(100), Duration::ZERO)
            .unwrap();
        assert_eq!(drive(&mut lens, Duration::ZERO, None), HoldEnd::Timeout);
        assert_eq!(zoom_cmds(&lens).last(), Some(&vec![cmd::ZOOM_RELEASE, 0]));
    }

    #[test]
    fn reversing_halts_first_then_presses_the_other_way() {
        let mut lens = Lens::new(FakeIsp::new(500, 0));
        lens.begin_hold(true, Duration::from_secs(20), Duration::ZERO)
            .unwrap();
        lens.begin_hold(false, Duration::from_secs(20), Duration::ZERO)
            .unwrap();
        assert_eq!(
            zoom_cmds(&lens),
            [
                vec![cmd::ZOOM_PRESS, 1],
                vec![cmd::ZOOM_RELEASE, 0],
                vec![cmd::ZOOM_PRESS, 0]
            ]
        );
    }

    #[test]
    fn zoom_speed_accepts_only_driver_speeds() {
        let mut lens = Lens::new(FakeIsp::new(0, 0));
        lens.set_zoom_speed(40).unwrap();
        assert_eq!(lens.isp().speed, 40);
        assert!(lens.set_zoom_speed(50).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{FakeIsp, Set};
    use super::*;

    fn lens(zoom: u32, focus: u32) -> Lens<FakeIsp> {
        Lens::new(FakeIsp::new(zoom, focus))
    }

    fn set(item: u32, words: &[u32]) -> Set {
        let mut w = words.to_vec();
        w.resize(6, 0);
        Set { item, words: w }
    }

    /// (0, 230) .. (512, 800) .. (1024, 1022): a three-point curve shaped like a real camera's
    fn zf() -> ZfTable {
        ZfTable::parse(b"<ZFTable>1000000,300e6,200,320,400,3fe</ZFTable>").unwrap()
    }

    #[test]
    fn factory_curve_replaces_sparse_defaults_before_homing() {
        let mut isp = FakeIsp::new(0, 230);
        isp.rows = vec![(160, 0, 780); 50];
        let mut l = Lens::new(isp);
        l.configure_factory(&zf()).unwrap();
        assert_eq!(
            l.driver_rows().unwrap(),
            vec![
                Row {
                    zoom: 0,
                    focus_min: 86,
                    focus_max: 330
                },
                Row {
                    zoom: 512,
                    focus_min: 736,
                    focus_max: 864
                },
                Row {
                    zoom: 1024,
                    focus_min: 958,
                    focus_max: 1086
                },
            ]
        );
        assert_eq!(l.isp().rows[49], (1024, 958, 1086));
        assert_eq!(l.zoom_max().unwrap(), 1024);
        assert!(l.isp().sets().iter().all(|s| s.item == item::MISC));
        l.init().unwrap();
    }

    #[test]
    fn unsupported_or_corrupt_calibration_never_writes_the_driver() {
        for case in 0..4 {
            let mut t = zf();
            match case {
                0 => t.lens = 0,
                1 => t.points.clear(),
                2 => t.points[1].zoom = 0,
                _ => t.points[0].focus = FOCUS_MAX + 1,
            }
            let mut l = lens(0, 230);
            assert!(l.configure_factory(&t).is_err());
            assert!(l.isp().sets().is_empty());
        }
    }

    #[test]
    fn reads_positions_and_the_focus_window() {
        let mut isp = FakeIsp::new(300, 512);
        isp.focus_window = (86, 330);
        let mut l = Lens::new(isp);
        assert_eq!(
            l.positions().unwrap(),
            Positions {
                zoom: 300,
                focus: 512
            }
        );
        assert_eq!(l.focus_window().unwrap(), (86, 330));
    }

    #[test]
    fn zoom_sends_the_vendor_command_and_waits_for_the_motor() {
        let mut l = lens(0, 230);
        l.zoom_to(1024).unwrap();
        assert_eq!(l.isp().sets(), [set(item::ZOOM, &[5, 1024])]);
        assert_eq!(l.positions().unwrap().zoom, 1024);
        assert!(l.isp().slept() >= POLL * fake::BUSY_POLLS);
    }

    #[test]
    fn focus_sends_the_vendor_command_and_waits_for_the_motor() {
        let mut l = lens(0, 230);
        l.focus_to(700).unwrap();
        assert_eq!(l.isp().sets(), [set(item::FOCUS, &[6, 700])]);
        assert_eq!(l.positions().unwrap().focus, 700);
    }

    #[test]
    fn zoom_stops_at_the_last_calibrated_row_the_driver_holds() {
        let mut isp = FakeIsp::new(0, 230);
        isp.rows = vec![(0, 86, 330), (512, 773, 901), (1024, 958, 1086)];
        let mut l = Lens::new(isp);
        assert_eq!(l.zoom_max().unwrap(), 1024);
        assert_eq!(
            l.zoom_to(1148),
            Err(Error::OutOfRange {
                what: "zoom",
                pos: 1148,
                max: 1024
            })
        );
        assert!(l.isp().sets().is_empty());
        l.zoom_to(1024).unwrap();
    }

    #[test]
    fn without_driver_rows_zoom_falls_back_to_the_mechanical_travel() {
        assert_eq!(lens(0, 230).zoom_max().unwrap(), ZOOM_MAX);
    }

    #[test]
    fn focus_beyond_the_driver_travel_is_refused_without_an_ioctl() {
        let mut l = lens(0, 230);
        assert!(matches!(l.focus_to(1706), Err(Error::OutOfRange { .. })));
        assert!(l.isp().sets().is_empty());
    }

    #[test]
    fn a_motor_that_never_settles_times_out() {
        let mut l = Lens::new(FakeIsp::new(0, 230).stuck());
        assert_eq!(l.zoom_to(10), Err(Error::MotorTimeout));
        assert!(l.isp().slept() >= WAIT);
    }

    #[test]
    fn zoom_at_is_linear_inside_a_recorded_move_and_its_start_before_it() {
        let mut l = lens(0, 230);
        l.zoom_focus_at(
            512,
            800,
            WAIT,
            &mut || Ok(()),
            Some(Duration::from_secs(10)),
        )
        .unwrap();
        // speed 80: 512 steps take 2.75 s
        assert_eq!(
            l.zoom_at(Duration::from_secs(9)),
            Some(0),
            "before the first record"
        );
        assert_eq!(l.zoom_at(Duration::from_secs(10)), Some(0));
        assert_eq!(l.zoom_at(Duration::from_millis(11_375)), Some(256));
        assert_eq!(l.zoom_at(Duration::from_secs(20)), Some(512));
        assert_eq!(
            Lens::new(FakeIsp::new(0, 230)).zoom_at(Duration::ZERO),
            None
        );
    }

    #[test]
    fn va_sums_all_256_cells() {
        let l = &mut lens(0, 600);
        // the fake's peak: 1 000 floor + 100 000
        assert_eq!(l.va().unwrap(), 101_000);
    }

    #[test]
    fn autofocus_finds_the_sharpest_focus_inside_the_window() {
        let mut isp = FakeIsp::new(512, 300);
        isp.focus_window = (500, 700);
        isp.sharp = Some(600);
        let mut l = Lens::new(isp);
        let r = l.autofocus(&zf()).unwrap();
        // the climb from the window's edge 500 in steps of 12 is bracketed at 608 and 584; fine 590 and 602;
        // the fitted vertex is 603
        assert_eq!(r.focus, 603);
        assert!(!r.calibrated);
        assert_eq!(l.positions().unwrap().focus, 603);
        let moves = l.isp().focus_moves();
        assert!(moves.iter().all(|f| (500..=700).contains(f)), "{moves:?}");
        assert_eq!(&moves[..3], [500, 512, 524]);
    }

    #[test]
    fn autofocus_settles_from_below() {
        let mut isp = FakeIsp::new(512, 300);
        isp.focus_window = (500, 700);
        isp.sharp = Some(594);
        let mut l = Lens::new(isp);
        l.autofocus(&zf()).unwrap();
        let moves = l.isp().focus_moves();
        // the fine stage ends at 602, above the vertex 593, so it backs off a coarse step before the last move
        assert_eq!(&moves[moves.len() - 3..], [602, 581, 593]);
    }

    #[test]
    fn autofocus_uses_the_fine_steps_in_the_tele_third() {
        let mut isp = FakeIsp::new(1024, 1000);
        isp.focus_window = (958, 1086);
        isp.sharp = Some(1000);
        let mut l = Lens::new(isp);
        let r = l.autofocus(&zf()).unwrap();
        // 1000 is sampled where the lens is; the climb brackets in steps of 8, the fine stage in steps of 4
        assert_eq!(l.isp().focus_moves(), [1008, 992, 996, 1004, 992, 1000]);
        assert_eq!(r.focus, 1000);
    }

    #[test]
    fn a_scene_without_contrast_gets_the_calibrated_focus() {
        let mut isp = FakeIsp::new(512, 300);
        isp.focus_window = (700, 900);
        isp.sharp = None;
        let mut l = Lens::new(isp);
        let r = l.autofocus(&zf()).unwrap();
        assert!(r.calibrated);
        assert_eq!(r.focus, 800);
        assert_eq!(l.positions().unwrap().focus, 800);
    }

    #[test]
    fn reads_the_focus_curve_rows_the_driver_holds() {
        let mut isp = FakeIsp::new(0, 230);
        isp.rows = vec![(0, 86, 330), (32, 179, 307), (1024, 958, 1086)];
        let rows = Lens::new(isp).driver_rows().unwrap();
        let row = |zoom, focus_min, focus_max| Row {
            zoom,
            focus_min,
            focus_max,
        };
        assert_eq!(
            rows,
            [row(0, 86, 330), row(32, 179, 307), row(1024, 958, 1086)]
        );
    }

    #[test]
    fn init_homes_both_motors_like_the_vendor_and_waits() {
        let mut l = lens(500, 500);
        l.init().unwrap();
        assert_eq!(
            l.isp().sets(),
            [set(item::FOCUS, &[1]), set(item::ZOOM, &[1])]
        );
        assert!(l.isp().slept() >= POLL * fake::BUSY_POLLS * 2);
    }
}
