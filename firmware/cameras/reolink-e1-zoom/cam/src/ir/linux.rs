//! The real day/night hardware: the IR-cut coil on the MS32006 (`/dev/i2c-2`, register 0x09), the ISP night mode
//! (`vendor_isp_set_iq` item 15 on `/dev/nvt_isp`), GPIOs and the light sensor through sysfs. Evidence: `docs/hardware/ir.md`.

use std::time::Duration;

use crate::ir::sysfs::{Gpio, IR_LED_GPIO, LIGHT, STATUS_LED_GPIO, read_adc};
use crate::ir::{Error, Filter, Hw};
use crate::ptz::Bus;
use crate::ptz::i2c::I2cBus;
use crate::zoom::WORDS;
use crate::zoom::isp::NvtIsp;

/// MS32006 register whose bits 3/2 energise the IR-cut coil one way or the other (`@ 0xf96f0`). The vendor writes
/// the whole register; its other bits are the self-clearing pan/tilt kick bits.
const REG_IRCUT: u8 = 0x09;
/// Pulse value per direction: the vendor's `ircut(1)` writes 0x08 (filter in), `ircut(0)` 0x04 (filter out),
/// confirmed on the camera by frames and by lighting the IR LEDs behind each.
const IRCUT_IN: u8 = 0x08;
const IRCUT_OUT: u8 = 0x04;
/// `vendor_isp_set_iq` item the vendor's day/night handler (@ 0xc6aac) sets to `{isp id, night}`.
pub const IQ_NIGHT: u32 = 15;

pub struct LinuxHw {
    bus: I2cBus,
    isp: NvtIsp,
    gpio: Gpio,
}

fn bus_err(e: crate::ptz::Error) -> Error {
    Error::Hw(format!("ms32006: {e}"))
}

fn isp_err(e: crate::zoom::Error) -> Error {
    Error::Hw(e.to_string())
}

impl LinuxHw {
    pub fn open(i2c: &str, isp: &str, gpio_root: &str) -> Result<Self, Error> {
        Ok(Self {
            bus: I2cBus::open(i2c).map_err(bus_err)?,
            isp: NvtIsp::open(isp).map_err(isp_err)?,
            gpio: Gpio::new(gpio_root),
        })
    }
}

impl Hw for LinuxHw {
    fn ircut_drive(&mut self, f: Filter) -> Result<(), Error> {
        let v = match f {
            Filter::In => IRCUT_IN,
            Filter::Out => IRCUT_OUT,
        };
        self.bus.write(REG_IRCUT, &[v]).map_err(bus_err)
    }

    fn ircut_release(&mut self) -> Result<(), Error> {
        self.bus.write(REG_IRCUT, &[0]).map_err(bus_err)
    }

    fn ir_leds(&mut self, on: bool) -> Result<(), Error> {
        self.gpio.set(IR_LED_GPIO, on)
    }

    fn isp_night(&mut self, night: bool) -> Result<(), Error> {
        let mut w = Box::new([0u32; WORDS]);
        w[1] = u32::from(night);
        self.isp.set_iq(IQ_NIGHT, &w).map_err(isp_err)
    }

    fn status_led(&mut self, on: bool) -> Result<(), Error> {
        self.gpio.set(STATUS_LED_GPIO, on)
    }

    fn ir_leds_state(&mut self) -> Result<bool, Error> {
        self.gpio.get(IR_LED_GPIO)
    }

    fn status_led_state(&mut self) -> Result<bool, Error> {
        self.gpio.get(STATUS_LED_GPIO)
    }

    /// The ISP's night flag as `vendor_isp_get_iq` reports it.
    fn isp_night_state(&mut self) -> Result<bool, Error> {
        let mut w = Box::new([0u32; WORDS]);
        self.isp.get_iq(IQ_NIGHT, &mut w).map_err(isp_err)?;
        Ok(w[1] != 0)
    }

    fn light(&mut self) -> Result<u32, Error> {
        read_adc(LIGHT)
    }

    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}
