//! Day/night: the IR-cut filter (MS32006 auxiliary output), the IR LEDs (GPIO 134), the ISP colour/B&W switch
//! and the light sensor (ADC channel 1), plus the status LED (GPIO 226). Evidence and offsets: `docs/hardware/ir.md`.
//!
//! Every hardware access goes through [`Hw`], so the sequencing and the auto day/night state machine are
//! unit-tested on the host against [`fake::FakeHw`]; the real one is [`linux::LinuxHw`].

use std::time::Duration;

pub mod fake;
pub(crate) mod linux;
pub mod sysfs;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("{0}")]
    Hw(String),
}

/// Day: filter in, colour, IR LEDs off. Night: filter out, B&W, IR LEDs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Day,
    Night,
}

/// Which way the IR-cut coil is driven: the filter swings in front of the sensor, or out of the way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    In,
    Out,
}

/// The day/night hardware.
pub trait Hw {
    /// Drives the IR-cut coil towards `f` (energised, not yet released).
    fn ircut_drive(&mut self, f: Filter) -> Result<(), Error>;
    /// De-energises the IR-cut coil; the filter stays where it was driven.
    fn ircut_release(&mut self) -> Result<(), Error>;
    fn ir_leds(&mut self, on: bool) -> Result<(), Error>;
    /// ISP night mode: true renders B&W.
    fn isp_night(&mut self, night: bool) -> Result<(), Error>;
    fn status_led(&mut self, on: bool) -> Result<(), Error>;
    /// Reads back the IR LED pin.
    fn ir_leds_state(&mut self) -> Result<bool, Error>;
    /// Reads back the status LED pin.
    fn status_led_state(&mut self) -> Result<bool, Error>;
    /// Reads back the ISP night flag: true renders B&W.
    fn isp_night_state(&mut self) -> Result<bool, Error>;
    /// Raw light-sensor reading: higher is darker.
    fn light(&mut self) -> Result<u32, Error>;
    fn sleep(&mut self, d: Duration);
}

/// How long the vendor energises the IR-cut coil (its caller passes 300 to the pulse state machine @ 0xf9750).
pub const IRCUT_PULSE: Duration = Duration::from_millis(300);

/// What `cam ir status` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    pub light: u32,
    pub ir_leds: bool,
    pub status_led: bool,
    pub isp_night: bool,
}

pub struct Ir<H: Hw> {
    hw: H,
    release_failed: bool,
    /// Night lights the IR LEDs; off renders B&W without the IR flood.
    leds_at_night: bool,
    mode: Option<Mode>,
}

impl<H: Hw> Ir<H> {
    pub fn new(hw: H) -> Self {
        Self {
            hw,
            release_failed: false,
            leds_at_night: true,
            mode: None,
        }
    }

    /// Whether night mode lights the IR LEDs; applied at once when already at night.
    pub fn set_leds_at_night(&mut self, on: bool) -> Result<(), Error> {
        self.leds_at_night = on;
        if self.mode == Some(Mode::Night) {
            self.hw.ir_leds(on)?;
        }
        Ok(())
    }

    pub fn failed(&self) -> bool {
        self.release_failed
    }

    pub fn light(&mut self) -> Result<u32, Error> {
        self.hw.light()
    }

    /// Reads the light sensor and every output's current state.
    pub fn status(&mut self) -> Result<Status, Error> {
        Ok(Status {
            light: self.hw.light()?,
            ir_leds: self.hw.ir_leds_state()?,
            status_led: self.hw.status_led_state()?,
            isp_night: self.hw.isp_night_state()?,
        })
    }

    pub fn hw(&self) -> &H {
        &self.hw
    }

    /// Switches everything to `mode`. LEDs go off before the filter comes back in and on only after it is out,
    /// so the sensor never sees the LEDs through the filter.
    pub fn set(&mut self, mode: Mode) -> Result<(), Error> {
        if self.release_failed {
            return Err(Error::Hw("coil release failed; reboot required".into()));
        }
        self.mode = None;
        match mode {
            Mode::Night => {
                self.pulse(Filter::Out)?;
                self.hw.isp_night(true)?;
                self.hw.ir_leds(self.leds_at_night)?;
            }
            Mode::Day => {
                self.hw.ir_leds(false)?;
                self.pulse(Filter::In)?;
                self.hw.isp_night(false)?;
            }
        }
        self.mode = Some(mode);
        Ok(())
    }

    pub fn status_led(&mut self, on: bool) -> Result<(), Error> {
        self.hw.status_led(on)
    }

    /// Energises the coil for [`IRCUT_PULSE`] and always releases it, even when driving failed half-way.
    fn pulse(&mut self, f: Filter) -> Result<(), Error> {
        let driven = self.hw.ircut_drive(f);
        if driven.is_ok() {
            self.hw.sleep(IRCUT_PULSE);
        }
        let released = self.hw.ircut_release();
        self.release_failed |= released.is_err();
        match (driven, released) {
            (Err(primary), Err(release)) => Err(Error::Hw(format!(
                "{primary}; coil release also failed: {release}"
            ))),
            (driven, released) => driven.and(released),
        }
    }
}

/// Light-sensor thresholds of the auto mode. The reading rises as the room darkens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    /// In day mode, a reading at or above this (for [`HOLD`]) switches to night (`color2bw_threshold`).
    pub to_night: u32,
    /// In night mode, a reading at or below this (for [`HOLD`]) switches to day (`bw2color_threshold`).
    pub to_day: u32,
}

/// `color2bw_threshold=2300`, `bw2color_threshold=1900` from the vendor's `0_isp.cfg` / `dvr.xml`.
pub const VENDOR_THRESHOLDS: Thresholds = Thresholds {
    to_night: 2300,
    to_day: 1900,
};

/// How long a reading must stay past a threshold before the mode switches (the vendor's `daynight_cds` thread
/// waits 4 s, read from its memory at runtime).
pub const HOLD: Duration = Duration::from_secs(4);

/// How often the auto mode reads the sensor (the vendor polls every 100 ms; the hold makes that needless).
pub const POLL: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("to_night ({to_night}) must be above to_day ({to_day})")]
pub struct BadThresholds {
    pub to_night: u32,
    pub to_day: u32,
}

/// Auto day/night with hysteresis and a hold time, fed one sensor reading at a time (the vendor `daynight_cds`
/// thread @ 0xed430).
#[derive(Debug, Clone)]
pub struct Auto {
    t: Thresholds,
    hold: Duration,
    mode: Option<Mode>,
    /// When the reading first crossed the threshold that leads out of the current mode.
    since: Option<Duration>,
}

impl Auto {
    pub fn new(t: Thresholds, hold: Duration) -> Result<Self, BadThresholds> {
        if t.to_night <= t.to_day {
            return Err(BadThresholds {
                to_night: t.to_night,
                to_day: t.to_day,
            });
        }
        Ok(Self {
            t,
            hold,
            mode: None,
            since: None,
        })
    }

    pub fn mode(&self) -> Option<Mode> {
        self.mode
    }

    /// New thresholds and hold time, keeping the current mode; a pending hold starts over.
    pub fn retune(&mut self, t: Thresholds, hold: Duration) -> Result<(), BadThresholds> {
        let next = Self::new(t, hold)?;
        *self = Self {
            mode: self.mode,
            ..next
        };
        Ok(())
    }

    /// Takes the reading at time `now` (any monotonic origin) and returns the mode to switch to, if any. The first
    /// reading decides at once: night at or above `to_night`, day otherwise.
    pub fn feed(&mut self, reading: u32, now: Duration) -> Option<Mode> {
        let (leaving, next) = match self.mode {
            None => {
                let m = if reading >= self.t.to_night {
                    Mode::Night
                } else {
                    Mode::Day
                };
                self.mode = Some(m);
                return Some(m);
            }
            Some(Mode::Day) => (reading >= self.t.to_night, Mode::Night),
            Some(Mode::Night) => (reading <= self.t.to_day, Mode::Day),
        };
        if !leaving {
            self.since = None;
            return None;
        }
        let since = *self.since.get_or_insert(now);
        if now.saturating_sub(since) < self.hold {
            return None;
        }
        self.since = None;
        self.mode = Some(next);
        Some(next)
    }
}

/// Polls the sensor every `poll` while `keep_going` says so and applies each switch `auto` decides. Time is counted
/// in polls. Returns the most recent 128 switches, with the time of each.
pub fn run_auto<H: Hw>(
    ir: &mut Ir<H>,
    mut auto: Auto,
    poll: Duration,
    keep_going: &mut dyn FnMut() -> bool,
) -> Result<Vec<(Mode, Duration)>, Error> {
    let mut now = Duration::ZERO;
    let mut switches = Vec::new();
    while keep_going() {
        let reading = ir.hw.light()?;
        if let Some(m) = auto.feed(reading, now) {
            ir.set(m)?;
            if switches.len() == 128 {
                switches.remove(0);
            }
            switches.push((m, now));
        }
        ir.hw.sleep(poll);
        now += poll;
    }
    Ok(switches)
}

#[cfg(test)]
mod tests {
    use super::fake::{Event, FakeHw};
    use super::*;

    #[test]
    fn status_reads_the_sensor_and_every_output_back() {
        let mut ir = Ir::new(FakeHw::new(1500));
        ir.set(Mode::Night).unwrap();
        ir.status_led(true).unwrap();
        assert_eq!(
            ir.status().unwrap(),
            Status {
                light: 1500,
                ir_leds: true,
                status_led: true,
                isp_night: true,
            }
        );
    }

    #[test]
    fn night_pulses_the_filter_out_then_goes_bw_and_lights_the_leds() {
        let mut ir = Ir::new(FakeHw::new(1500));
        ir.set(Mode::Night).unwrap();
        assert_eq!(
            ir.hw().events(),
            [
                Event::Drive(Filter::Out),
                Event::Sleep(IRCUT_PULSE),
                Event::Release,
                Event::IspNight(true),
                Event::IrLeds(true),
            ]
        );
    }

    #[test]
    fn night_without_leds_still_goes_bw_and_the_choice_applies_live() {
        let mut ir = Ir::new(FakeHw::new(1500));
        ir.set_leds_at_night(false).unwrap();
        assert!(
            ir.hw().events().is_empty(),
            "not at night: nothing to switch"
        );
        ir.set(Mode::Night).unwrap();
        assert_eq!(
            ir.hw().events()[3..],
            [Event::IspNight(true), Event::IrLeds(false)]
        );
        ir.set_leds_at_night(true).unwrap();
        assert_eq!(ir.hw().events().last(), Some(&Event::IrLeds(true)));
        ir.set(Mode::Day).unwrap();
        ir.set_leds_at_night(false).unwrap();
        assert_eq!(ir.hw().events().last(), Some(&Event::IspNight(false)));
    }

    #[test]
    fn day_turns_the_leds_off_before_the_filter_comes_back() {
        let mut ir = Ir::new(FakeHw::new(1500));
        ir.set(Mode::Day).unwrap();
        assert_eq!(
            ir.hw().events(),
            [
                Event::IrLeds(false),
                Event::Drive(Filter::In),
                Event::Sleep(IRCUT_PULSE),
                Event::Release,
                Event::IspNight(false),
            ]
        );
    }

    #[test]
    fn a_failing_isp_is_reported_after_the_coil_was_released() {
        let mut ir = Ir::new(FakeHw::new(1500).failing_isp());
        assert!(ir.set(Mode::Night).is_err());
        assert_eq!(
            ir.hw().events(),
            [
                Event::Drive(Filter::Out),
                Event::Sleep(IRCUT_PULSE),
                Event::Release
            ]
        );
    }

    fn auto() -> Auto {
        Auto::new(VENDOR_THRESHOLDS, HOLD).unwrap()
    }

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn first_reading_picks_the_mode_at_once() {
        assert_eq!(auto().feed(1413, secs(0)), Some(Mode::Day));
        assert_eq!(auto().feed(2400, secs(0)), Some(Mode::Night));
        // between the thresholds: day, like the vendor's colour default
        assert_eq!(auto().feed(2000, secs(0)), Some(Mode::Day));
    }

    #[test]
    fn darkness_must_last_the_hold_time_before_night() {
        let mut a = auto();
        a.feed(1400, secs(0));
        assert_eq!(a.feed(2300, secs(1)), None);
        assert_eq!(a.feed(2350, secs(4)), None);
        assert_eq!(a.feed(2310, secs(5)), Some(Mode::Night));
        assert_eq!(a.mode(), Some(Mode::Night));
        assert_eq!(a.feed(2400, secs(20)), None);
    }

    #[test]
    fn a_bright_blip_restarts_the_hold() {
        let mut a = auto();
        a.feed(1400, secs(0));
        a.feed(2400, secs(1));
        assert_eq!(a.feed(2299, secs(3)), None);
        assert_eq!(a.feed(2400, secs(4)), None);
        assert_eq!(a.feed(2400, secs(7)), None);
        assert_eq!(a.feed(2400, secs(8)), Some(Mode::Night));
    }

    #[test]
    fn readings_inside_the_hysteresis_band_keep_the_mode() {
        let mut a = auto();
        a.feed(2400, secs(0));
        // the IR LEDs' own light lowers the reading a little; it must not flip back to day
        for t in 1..60 {
            assert_eq!(a.feed(1901 + (t as u32 * 7) % 390, secs(t)), None);
        }
        assert_eq!(a.feed(1900, secs(60)), None);
        assert_eq!(a.feed(1850, secs(64)), Some(Mode::Day));
    }

    #[test]
    fn retuned_thresholds_keep_the_mode() {
        let mut a = auto();
        a.feed(3000, secs(0));
        a.retune(
            Thresholds {
                to_night: 3500,
                to_day: 3200,
            },
            secs(1),
        )
        .unwrap();
        assert_eq!(a.mode(), Some(Mode::Night));
        assert_eq!(a.feed(3100, secs(10)), None, "hold starts over");
        assert_eq!(a.feed(3100, secs(11)), Some(Mode::Day));
        assert!(a.retune(VENDOR_THRESHOLDS, secs(1)).is_ok());
        assert!(
            a.retune(
                Thresholds {
                    to_night: 1,
                    to_day: 2
                },
                secs(1)
            )
            .is_err()
        );
    }

    #[test]
    fn thresholds_must_leave_a_band() {
        let t = |to_night, to_day| Thresholds { to_night, to_day };
        assert!(Auto::new(t(1900, 2300), HOLD).is_err());
        assert!(Auto::new(t(2000, 2000), HOLD).is_err());
        assert!(Auto::new(t(2001, 2000), HOLD).is_ok());
    }

    #[test]
    fn run_auto_applies_each_switch_once_and_stops_when_asked() {
        // 1 s polls: day at once, dark from t=2 (night after the 4 s hold, at t=6), bright again from t=8
        let readings = [
            1400, 1400, 2500, 2500, 2500, 2500, 2500, 2500, 1500, 1500, 1500, 1500, 1500, 1500,
        ];
        let mut ir = Ir::new(FakeHw::new(0).with_readings(&readings));
        let mut polls = 0;
        let switches = run_auto(&mut ir, auto(), secs(1), &mut || {
            polls += 1;
            polls <= readings.len()
        })
        .unwrap();
        assert_eq!(
            switches,
            [
                (Mode::Day, secs(0)),
                (Mode::Night, secs(6)),
                (Mode::Day, secs(12))
            ]
        );
        let modes: Vec<_> = ir
            .hw()
            .events()
            .iter()
            .filter_map(|e| match e {
                Event::IspNight(n) => Some(*n),
                _ => None,
            })
            .collect();
        assert_eq!(modes, [false, true, false]);
    }

    #[test]
    fn status_led_is_its_own_switch() {
        let mut ir = Ir::new(FakeHw::new(0));
        ir.status_led(false).unwrap();
        ir.status_led(true).unwrap();
        assert_eq!(
            ir.hw().events(),
            [Event::StatusLed(false), Event::StatusLed(true)]
        );
    }
}
