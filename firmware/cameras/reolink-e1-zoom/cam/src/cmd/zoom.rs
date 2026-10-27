//! `cam zoom` and `cam focus`: the lens motors through `/dev/nvt_isp`. A meaningful autofocus needs frames
//! flowing through the ISP (`cam serve` running).

use std::path::Path;
use std::process::ExitCode;

use super::args::{Opts, number};
use super::{CliError, failed};
use crate::zoom::Lens;
use crate::zoom::isp::{self, NvtIsp};
use crate::zoom::zf_table::{SP_DEVICE, ZfTable};

pub const USAGE: &str = "usage: cam zoom [--dev PATH] [--sp PATH] <pos> | info | init | rows <below> <above>\n       \
                         cam focus [--dev PATH] [--sp PATH] [<pos>]   (no position: one-shot autofocus)\n       \
                         rows: reinstall the focus-curve rows with that window around each calibrated point";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Info,
    Init,
    /// Reinstall the driver rows with this window below and above each calibrated focus.
    Rows(u32, u32),
    Zoom(u32),
    Focus(u32),
    Autofocus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub dev: String,
    pub sp: String,
    pub action: Action,
}

fn parse(
    args: &[String],
    action: impl Fn(&[&str]) -> Result<Action, CliError>,
) -> Result<Args, CliError> {
    let opts = Opts::split(args, &["--dev", "--sp"])?;
    Ok(Args {
        dev: opts.get("--dev", isp::DEVICE),
        sp: opts.get("--sp", SP_DEVICE),
        action: action(&opts.words)?,
    })
}

pub fn parse_zoom(args: &[String]) -> Result<Args, CliError> {
    parse(args, |words| match words {
        ["info"] => Ok(Action::Info),
        ["init"] => Ok(Action::Init),
        ["rows", below, above] => Ok(Action::Rows(number(below)?, number(above)?)),
        [pos] => Ok(Action::Zoom(number(pos)?)),
        _ => Err(CliError::Usage),
    })
}

pub fn parse_focus(args: &[String]) -> Result<Args, CliError> {
    parse(args, |words| match words {
        [] => Ok(Action::Autofocus),
        [pos] => Ok(Action::Focus(number(pos)?)),
        _ => Err(CliError::Usage),
    })
}

pub fn run_zoom(args: &[String]) -> Result<ExitCode, CliError> {
    run(parse_zoom(args)?)
}

pub fn run_focus(args: &[String]) -> Result<ExitCode, CliError> {
    run(parse_focus(args)?)
}

fn run(args: Args) -> Result<ExitCode, CliError> {
    let zf = || -> Result<ZfTable, String> {
        let sp = crate::files::read_device_or_file(Path::new(&args.sp), 512 * 1024)
            .map_err(|x| format!("{}: {x}", args.sp))?;
        ZfTable::parse(&sp).map_err(|x| format!("{}: {x}", args.sp))
    };
    let mut lens = Lens::new(NvtIsp::open(&args.dev)?);
    let report = |lens: &mut Lens<NvtIsp>| -> Result<ExitCode, CliError> {
        let p = lens.positions()?;
        println!("zoom={} focus={}", p.zoom, p.focus);
        Ok(ExitCode::SUCCESS)
    };
    match args.action {
        Action::Info => {
            report(&mut lens)?;
            let (fmin, fmax) = lens.focus_window()?;
            println!("focus_window={fmin}..{fmax}");
            println!("zoom_max={}", lens.zoom_max()?);
            println!("busy={}", lens.busy()?);
            println!("va={}", lens.va()?);
            for (i, r) in lens.driver_rows()?.iter().enumerate() {
                println!("driver_row{i}={} {}..{}", r.zoom, r.focus_min, r.focus_max);
            }
            match zf() {
                Ok(t) => println!("{t}"),
                Err(x) => println!("zftable: {x}"),
            }
            Ok(ExitCode::SUCCESS)
        }
        Action::Init => {
            lens.configure_factory(&zf().map_err(failed)?)?;
            lens.init()?;
            report(&mut lens)
        }
        Action::Rows(below, above) => {
            let zf = zf().map_err(failed)?;
            let rows = zf.rows_with(|_, _| (below, above)).map_err(failed)?;
            lens.configure(&rows, zf.backlash)?;
            for (i, r) in lens.driver_rows()?.iter().enumerate() {
                println!("driver_row{i}={} {}..{}", r.zoom, r.focus_min, r.focus_max);
            }
            let (fmin, fmax) = lens.focus_window()?;
            println!("focus_window={fmin}..{fmax}");
            report(&mut lens)
        }
        Action::Zoom(pos) => {
            lens.zoom_to(pos)?;
            report(&mut lens)
        }
        Action::Focus(pos) => {
            lens.focus_to(pos)?;
            report(&mut lens)
        }
        Action::Autofocus => {
            let r = lens.autofocus(&zf().map_err(failed)?)?;
            for (f, va) in &r.samples {
                println!("af_sample focus={f} va={va}");
            }
            println!("af_focus={} af_calibrated={}", r.focus, r.calibrated);
            report(&mut lens)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::args::words as args;

    #[test]
    fn parses_each_subcommand() {
        let c = parse_zoom(&args("1148")).unwrap();
        assert_eq!(c.action, Action::Zoom(1148));
        assert_eq!(
            (c.dev.as_str(), c.sp.as_str()),
            ("/dev/nvt_isp", "/dev/mtd7ro")
        );
        assert_eq!(parse_zoom(&args("info")).unwrap().action, Action::Info);
        assert_eq!(parse_zoom(&args("init")).unwrap().action, Action::Init);
        assert_eq!(parse_focus(&args("")).unwrap().action, Action::Autofocus);
        assert_eq!(
            parse_focus(&args("700")).unwrap().action,
            Action::Focus(700)
        );
    }

    #[test]
    fn options_may_come_anywhere() {
        let c = parse_focus(&args("--sp /tmp/sp.bin --dev /dev/x")).unwrap();
        assert_eq!((c.dev.as_str(), c.sp.as_str()), ("/dev/x", "/tmp/sp.bin"));
        assert_eq!(c.action, Action::Autofocus);
    }

    #[test]
    fn bad_words_are_usage_errors_and_bad_values_are_failures() {
        assert_eq!(parse_zoom(&args("")), Err(CliError::Usage));
        assert_eq!(parse_zoom(&args("1 2")), Err(CliError::Usage));
        assert_eq!(parse_focus(&args("1 2")), Err(CliError::Usage));
        for bad in ["-1", "far", "1 --dev"] {
            assert!(
                matches!(parse_zoom(&args(bad)), Err(CliError::Failed(_))),
                "{bad:?}"
            );
        }
    }
}
