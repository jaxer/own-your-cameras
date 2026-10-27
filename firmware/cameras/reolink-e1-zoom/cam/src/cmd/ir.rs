//! `cam ir` and `cam led`: IR-cut, IR LEDs, the ISP's colour/B&W mode and the status LED. The ISP switch only
//! shows while frames flow (`cam serve` running).

use std::process::ExitCode;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use super::{CliError, failed};
use crate::ir::linux::LinuxHw;
use crate::ir::sysfs::{GPIO_ROOT, LIGHT, read_adc};
use crate::ir::{self, Auto, Ir, Mode, Thresholds, VENDOR_THRESHOLDS};
use crate::ptz::i2c as ptz_i2c;
use crate::signals;
use crate::zoom::isp as zoom_isp;

pub const USAGE: &str = "usage: cam ir day | night | status | auto [--to-night N] [--to-day N] [--seconds N]\n       \
                         cam led on | off";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Set(Mode),
    /// Print the light reading and the GPIO states.
    Status,
    /// `seconds` 0 runs until SIGTERM.
    Auto {
        t: Thresholds,
        seconds: u64,
    },
    StatusLed(bool),
}

pub fn parse_ir(args: &[String]) -> Result<Action, CliError> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["day"] => Ok(Action::Set(Mode::Day)),
        ["night"] => Ok(Action::Set(Mode::Night)),
        ["status"] => Ok(Action::Status),
        ["auto", opts @ ..] => auto(opts),
        _ => Err(CliError::Usage),
    }
}

pub fn parse_led(args: &[String]) -> Result<Action, CliError> {
    match args {
        [on] if on == "on" => Ok(Action::StatusLed(true)),
        [off] if off == "off" => Ok(Action::StatusLed(false)),
        _ => Err(CliError::Usage),
    }
}

fn auto(opts: &[&str]) -> Result<Action, CliError> {
    let mut t = VENDOR_THRESHOLDS;
    let mut seconds = 0;
    let mut it = opts.iter();
    while let Some(o) = it.next() {
        let v = it.next().ok_or(CliError::Usage)?;
        let n = v
            .parse::<u64>()
            .map_err(|_| failed(format!("not a number: {v}")))?;
        let reading = || u32::try_from(n).map_err(|_| failed(format!("too large: {v}")));
        match *o {
            "--to-night" => t.to_night = reading()?,
            "--to-day" => t.to_day = reading()?,
            "--seconds" => seconds = n,
            _ => return Err(CliError::Usage),
        }
    }
    Ok(Action::Auto { t, seconds })
}

pub fn run_ir(args: &[String]) -> Result<ExitCode, CliError> {
    run(parse_ir(args)?)
}

pub fn run_led(args: &[String]) -> Result<ExitCode, CliError> {
    run(parse_led(args)?)
}

fn run(action: Action) -> Result<ExitCode, CliError> {
    signals::install()?;
    let hw = LinuxHw::open(ptz_i2c::DEVICE, zoom_isp::DEVICE, GPIO_ROOT)?;
    match action {
        Action::Status => {
            let st = Ir::new(hw).status()?;
            println!("light={}", st.light);
            println!("ir_leds={}", u8::from(st.ir_leds));
            println!("status_led={}", u8::from(st.status_led));
            println!("isp_night={}", u8::from(st.isp_night));
        }
        Action::Set(mode) => Ir::new(hw).set(mode)?,
        Action::StatusLed(on) => Ir::new(hw).status_led(on)?,
        Action::Auto { t, seconds } => {
            let auto = Auto::new(t, ir::HOLD)?;
            println!(
                "ir auto: to_night>={} to_day<={} hold={:?}",
                t.to_night,
                t.to_day,
                ir::HOLD
            );
            let start = Instant::now();
            let mut ir = Ir::new(hw);
            let mut last = Instant::now();
            let switches = ir::run_auto(&mut ir, auto, ir::POLL, &mut || {
                if last.elapsed() >= Duration::from_secs(5) {
                    last = Instant::now();
                    if let Ok(v) = read_adc(LIGHT) {
                        println!("ir auto: {:>6.1}s light={v}", start.elapsed().as_secs_f32());
                    }
                }
                !signals::STOP.load(Ordering::Relaxed)
                    && (seconds == 0 || start.elapsed().as_secs() < seconds)
            })?;
            for (m, t) in switches {
                println!("ir auto: switched to {m:?} after {:.1}s", t.as_secs_f32());
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::args::words as args;

    #[test]
    fn parses_each_subcommand() {
        assert_eq!(parse_ir(&args("day")), Ok(Action::Set(Mode::Day)));
        assert_eq!(parse_ir(&args("night")), Ok(Action::Set(Mode::Night)));
        assert_eq!(parse_ir(&args("status")), Ok(Action::Status));
        assert_eq!(parse_led(&args("on")), Ok(Action::StatusLed(true)));
        assert_eq!(parse_led(&args("off")), Ok(Action::StatusLed(false)));
        assert_eq!(
            parse_ir(&args("auto")),
            Ok(Action::Auto {
                t: Thresholds {
                    to_night: 2300,
                    to_day: 1900
                },
                seconds: 0
            })
        );
    }

    #[test]
    fn auto_takes_thresholds_and_a_duration_in_any_order() {
        assert_eq!(
            parse_ir(&args("auto --seconds 60 --to-day 1300 --to-night 1450")),
            Ok(Action::Auto {
                t: Thresholds {
                    to_night: 1450,
                    to_day: 1300
                },
                seconds: 60
            })
        );
    }

    #[test]
    fn bad_words_are_usage_errors_and_bad_values_are_failures() {
        for bad in ["", "dusk", "day now", "auto --to-night", "auto --hold 3"] {
            assert_eq!(parse_ir(&args(bad)), Err(CliError::Usage), "{bad:?}");
        }
        for bad in ["", "blink", "on off"] {
            assert_eq!(parse_led(&args(bad)), Err(CliError::Usage), "{bad:?}");
        }
        for bad in ["auto --to-night x", "auto --to-night 99999999999"] {
            assert!(
                matches!(parse_ir(&args(bad)), Err(CliError::Failed(_))),
                "{bad:?}"
            );
        }
    }
}
