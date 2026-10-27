//! Host fake of the day/night hardware: records every call in order and answers `light` from a script.

use std::collections::VecDeque;
use std::time::Duration;

use super::{Error, Filter, Hw};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Drive(Filter),
    Release,
    IrLeds(bool),
    IspNight(bool),
    StatusLed(bool),
    Sleep(Duration),
}

pub struct FakeHw {
    /// Readings `light` returns in order; the last one repeats.
    readings: VecDeque<u32>,
    last: u32,
    events: Vec<Event>,
    fail_isp: bool,
    fail_release: bool,
    ir_leds: bool,
    status_led: bool,
    isp_night: bool,
}

impl FakeHw {
    pub fn new(light: u32) -> Self {
        Self {
            readings: VecDeque::new(),
            last: light,
            events: Vec::new(),
            fail_isp: false,
            fail_release: false,
            ir_leds: false,
            status_led: false,
            isp_night: false,
        }
    }

    /// `light` answers these in order, then keeps answering the last.
    pub fn with_readings(mut self, r: &[u32]) -> Self {
        self.readings = r.iter().copied().collect();
        self
    }

    /// The ISP night-mode call fails.
    pub fn failing_isp(mut self) -> Self {
        self.fail_isp = true;
        self
    }

    pub fn failing_release(mut self) -> Self {
        self.fail_release = true;
        self
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Hw for FakeHw {
    fn ircut_drive(&mut self, f: Filter) -> Result<(), Error> {
        self.events.push(Event::Drive(f));
        Ok(())
    }

    fn ircut_release(&mut self) -> Result<(), Error> {
        self.events.push(Event::Release);
        if self.fail_release {
            return Err(Error::Hw("fake: coil release".into()));
        }
        Ok(())
    }

    fn ir_leds(&mut self, on: bool) -> Result<(), Error> {
        self.ir_leds = on;
        self.events.push(Event::IrLeds(on));
        Ok(())
    }

    fn isp_night(&mut self, night: bool) -> Result<(), Error> {
        if self.fail_isp {
            return Err(Error::Hw("fake: isp".into()));
        }
        self.isp_night = night;
        self.events.push(Event::IspNight(night));
        Ok(())
    }

    fn status_led(&mut self, on: bool) -> Result<(), Error> {
        self.status_led = on;
        self.events.push(Event::StatusLed(on));
        Ok(())
    }

    fn ir_leds_state(&mut self) -> Result<bool, Error> {
        Ok(self.ir_leds)
    }

    fn status_led_state(&mut self) -> Result<bool, Error> {
        Ok(self.status_led)
    }

    fn isp_night_state(&mut self) -> Result<bool, Error> {
        Ok(self.isp_night)
    }

    fn light(&mut self) -> Result<u32, Error> {
        if let Some(v) = self.readings.pop_front() {
            self.last = v;
        }
        Ok(self.last)
    }

    fn sleep(&mut self, d: Duration) {
        self.events.push(Event::Sleep(d));
    }
}
