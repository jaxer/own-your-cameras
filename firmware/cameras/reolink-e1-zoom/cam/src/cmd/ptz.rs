//! `cam ptz`: pan/tilt through the MS32006.

use std::path::Path;
use std::process::ExitCode;

use super::CliError;
use super::args::{Opts, number};
use crate::control_stop;
use crate::ownership;
use crate::ptz::i2c::{self, I2cBus};
use crate::ptz::vendor_presets;
use crate::ptz::{Position, Ptz};

pub const USAGE: &str =
    "usage: cam ptz [--dev PATH] [--cfg PATH] home | move <pan> <tilt> | stop | preset <n>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Home,
    Move(Position),
    Stop,
    Preset(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub dev: String,
    pub cfg: String,
    pub action: Action,
}

pub fn parse(args: &[String]) -> Result<Args, CliError> {
    let opts = Opts::split(args, &["--dev", "--cfg"])?;
    let (dev, cfg) = (
        opts.get("--dev", i2c::DEVICE),
        opts.get("--cfg", vendor_presets::PATH),
    );
    let action = match opts.words.as_slice() {
        ["home"] => Action::Home,
        ["stop"] => Action::Stop,
        ["move", pan, tilt] => Action::Move(Position {
            pan: number(pan)?,
            tilt: number(tilt)?,
        }),
        ["preset", n] => Action::Preset(number(n)?),
        _ => return Err(CliError::Usage),
    };
    Ok(Args { dev, cfg, action })
}

/// A stop while another process owns the controls goes to that owner, without taking any lease: it must work
/// exactly when the controls are busy.
pub fn delegate_stop(runtime: &Path, args: &[String]) -> Option<Result<ExitCode, CliError>> {
    let endpoint = ownership::stop_endpoint(runtime);
    let stop = parse(args).is_ok_and(|a| a.action == Action::Stop);
    (stop && endpoint.exists()).then(|| {
        control_stop::request(&endpoint)?;
        Ok(ExitCode::SUCCESS)
    })
}

pub fn run(runtime: &Path, args: &[String]) -> Result<ExitCode, CliError> {
    let args = parse(args)?;
    let owner = control_stop::Owner::bind(&ownership::stop_endpoint(runtime))?;
    let mut ptz = Ptz::new(I2cBus::open(&args.dev)?).with_stop_owner(owner);
    let target = match args.action {
        Action::Stop => {
            ptz.stop()?;
            return Ok(ExitCode::SUCCESS);
        }
        Action::Home => None,
        Action::Move(to) => Some(to),
        Action::Preset(n) => {
            let cfg = crate::files::read_device_or_file(Path::new(&args.cfg), 64 * 1024)?;
            Some(vendor_presets::preset(&String::from_utf8(cfg)?, n)?)
        }
    };
    ptz.init()?;
    ptz.home()?;
    if let Some(to) = target {
        ptz.goto(to)?;
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::args::words as args;

    #[test]
    fn parses_each_subcommand() {
        let c = parse(&args("home")).unwrap();
        assert_eq!(c.action, Action::Home);
        assert_eq!(
            (c.dev.as_str(), c.cfg.as_str()),
            (i2c::DEVICE, vendor_presets::PATH)
        );
        assert_eq!(parse(&args("stop")).unwrap().action, Action::Stop);
        assert_eq!(
            parse(&args("move 6195 650")).unwrap().action,
            Action::Move(Position {
                pan: 6195,
                tilt: 650
            })
        );
        assert_eq!(parse(&args("preset 2")).unwrap().action, Action::Preset(2));
    }

    #[test]
    fn options_may_come_anywhere() {
        let c = parse(&args("--dev /dev/i2c-9 preset 1 --cfg /tmp/p.cfg")).unwrap();
        assert_eq!(c.dev, "/dev/i2c-9");
        assert_eq!(c.cfg, "/tmp/p.cfg");
        assert_eq!(c.action, Action::Preset(1));
    }

    #[test]
    fn bad_words_are_usage_errors_and_bad_values_are_failures() {
        for bad in ["", "fly", "move 1", "preset"] {
            assert_eq!(parse(&args(bad)), Err(CliError::Usage), "{bad:?}");
        }
        for bad in ["move a b", "preset -1", "--dev"] {
            assert!(
                matches!(parse(&args(bad)), Err(CliError::Failed(_))),
                "{bad:?}"
            );
        }
    }
}
