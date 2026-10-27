//! The sysfs side of day/night: GPIO outputs (`/sys/class/gpio`) and the light sensor (`nvt_adc` IIO). Plain file
//! I/O under a configurable root, so it is tested on the host against a temporary directory.

use std::fs;
use std::path::PathBuf;

use crate::ir::Error;

/// IR LED enable: 1 lights the IR LEDs.
pub const IR_LED_GPIO: u32 = 134;
/// Power/status LED (`S00_PreReady` sets it to 1 at boot).
pub const STATUS_LED_GPIO: u32 = 226;
/// E1 Zoom external speaker PA; stock talkback drives it high, then low when finished.
pub const SPEAKER_ENABLE_GPIO: u32 = 234;
/// The light sensor is ADC channel 1 (`f8864(1, &v)` in the vendor `daynight_cds` thread).
pub const LIGHT: &str = "/sys/bus/iio/devices/iio:device0/in_voltage1_raw";
pub const GPIO_ROOT: &str = "/sys/class/gpio";

fn io(what: &str, e: std::io::Error) -> Error {
    Error::Hw(format!("{what}: {e}"))
}

pub struct Gpio {
    root: PathBuf,
}

impl Gpio {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Drives `pin` as an output. Exports it if needed and sets the direction only when it is not already an
    /// output (writing `out` resets the level to 0, which would blink a lit LED).
    pub fn set(&self, pin: u32, on: bool) -> Result<(), Error> {
        let dir = self.root.join(format!("gpio{pin}"));
        if !dir.exists() {
            let export = self.root.join("export");
            fs::write(&export, pin.to_string()).map_err(|e| io(&export.to_string_lossy(), e))?;
        }
        let direction = dir.join("direction");
        let cur =
            fs::read_to_string(&direction).map_err(|e| io(&direction.to_string_lossy(), e))?;
        if cur.trim() != "out" {
            fs::write(&direction, "out").map_err(|e| io(&direction.to_string_lossy(), e))?;
        }
        let value = dir.join("value");
        fs::write(&value, if on { "1" } else { "0" }).map_err(|e| io(&value.to_string_lossy(), e))
    }

    /// Current level of an exported pin.
    pub fn get(&self, pin: u32) -> Result<bool, Error> {
        let value = self.root.join(format!("gpio{pin}/value"));
        let v = fs::read_to_string(&value).map_err(|e| io(&value.to_string_lossy(), e))?;
        Ok(v.trim() == "1")
    }
}

/// One raw ADC reading (12 bits) from an IIO `in_voltageN_raw` file.
pub fn read_adc(path: &str) -> Result<u32, Error> {
    let t = fs::read_to_string(path).map_err(|e| io(path, e))?;
    t.trim()
        .parse()
        .map_err(|_| Error::Hw(format!("{path}: not a number: {t:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cam-ir-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn pin(root: &Path, n: u32, direction: &str, value: &str) {
        let d = root.join(format!("gpio{n}"));
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("direction"), direction).unwrap();
        fs::write(d.join("value"), value).unwrap();
    }

    fn read(p: PathBuf) -> String {
        fs::read_to_string(p).unwrap()
    }

    #[test]
    fn an_exported_output_only_gets_its_value_written() {
        let root = tmp("out");
        pin(&root, 134, "out\n", "0\n");
        let g = Gpio::new(&root);
        g.set(134, true).unwrap();
        assert_eq!(read(root.join("gpio134/value")), "1");
        assert_eq!(read(root.join("gpio134/direction")), "out\n");
        assert!(g.get(134).unwrap());
        g.set(134, false).unwrap();
        assert!(!g.get(134).unwrap());
    }

    #[test]
    fn an_input_is_turned_into_an_output() {
        let root = tmp("in");
        pin(&root, 226, "in\n", "1\n");
        Gpio::new(&root).set(226, false).unwrap();
        assert_eq!(read(root.join("gpio226/direction")), "out");
        assert_eq!(read(root.join("gpio226/value")), "0");
    }

    #[test]
    fn a_missing_pin_is_exported_first() {
        let root = tmp("export");
        // a plain file stands in for the kernel's export node; the pin directory never appears, so set fails after
        // the export write, which is what this checks
        fs::write(root.join("export"), "").unwrap();
        assert!(Gpio::new(&root).set(134, true).is_err());
        assert_eq!(read(root.join("export")), "134");
    }

    #[test]
    fn adc_reads_the_raw_value() {
        let root = tmp("adc");
        let p = root.join("in_voltage1_raw");
        fs::write(&p, "1413\n").unwrap();
        assert_eq!(read_adc(&p.to_string_lossy()), Ok(1413));
        fs::write(&p, "junk").unwrap();
        assert!(read_adc(&p.to_string_lossy()).is_err());
        assert!(read_adc("/nonexistent/in_voltage1_raw").is_err());
    }
}
