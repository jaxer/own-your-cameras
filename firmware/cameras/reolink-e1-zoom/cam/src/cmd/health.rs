//! `cam preflight` and `cam health-watch`: the launcher's read-only helpers.

use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

use super::CliError;
use crate::runtime::health_watch::{self as hw, Watch};
use crate::signals;

pub const USAGE: &str = "usage: cam health-watch --attempt TOKEN --pid PID [--dir DIR]";

/// Validates the production inputs without changing anything: exit 0 ready, 1 invalid.
pub fn preflight() -> ExitCode {
    ExitCode::from(crate::preflight::run(Path::new("/")))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub attempt: String,
    pub pid: u32,
    pub dir: String,
}

pub fn parse(args: &[String]) -> Result<Args, CliError> {
    let (mut attempt, mut pid, mut dir) = (None, None, "/mnt/tmp".to_string());
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or(CliError::Usage)?;
        match flag.as_str() {
            "--attempt" if crate::runtime::valid_attempt(value) => attempt = Some(value.clone()),
            "--pid" => pid = value.parse::<u32>().ok().filter(|p| *p > 0),
            "--dir" => dir = value.clone(),
            _ => return Err(CliError::Usage),
        }
    }
    match (attempt, pid) {
        (Some(attempt), Some(pid)) => Ok(Args { attempt, pid, dir }),
        _ => Err(CliError::Usage),
    }
}

/// Prints `unqualified`, `qualified` and `child-exited` transitions, one line each, for the launcher. Exits 1
/// on `child-exited`, 0 on SIGTERM.
pub fn watch(args: &[String]) -> Result<ExitCode, CliError> {
    Ok(watch_to(&parse(args)?, &mut std::io::stdout().lock()))
}

/// A consumer that stops reading ends the watcher with failure.
pub fn watch_to(args: &Args, out: &mut impl std::io::Write) -> ExitCode {
    if let Err(error) = signals::install() {
        eprintln!("signal setup failed: {error}");
        return ExitCode::FAILURE;
    }
    let (start, dir) = (Instant::now(), Path::new(&args.dir));
    let mut watch = Watch::new(&args.attempt, args.pid);
    let mut say = |line: &str| -> bool {
        if let Err(error) = writeln!(out, "{line}").and_then(|()| out.flush()) {
            eprintln!("health-watch: output failed: {error}");
            return false;
        }
        true
    };
    if !say("unqualified") {
        return ExitCode::FAILURE;
    }
    loop {
        let record = hw::read_record(dir, &args.attempt);
        let alive = hw::process_alive(Path::new("/proc"), args.pid);
        if let Some(line) = watch.step(record.as_deref(), alive, start.elapsed())
            && (!say(line) || line == "child-exited")
        {
            return ExitCode::FAILURE;
        }
        if !signals::nap(hw::POLL) {
            return ExitCode::SUCCESS;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_attempt_pid_and_optional_dir() {
        assert_eq!(
            parse(&args(&["--attempt", "b-1", "--pid", "42"])),
            Ok(Args {
                attempt: "b-1".into(),
                pid: 42,
                dir: "/mnt/tmp".into()
            })
        );
        for bad in [
            args(&[]),
            args(&["--attempt", "b-1"]),
            args(&["--pid", "42"]),
            args(&["--attempt", "b/1", "--pid", "42"]),
            args(&["--attempt", "b", "--pid", "0"]),
            args(&["--attempt", "b", "--pid", "x"]),
            args(&["--attempt", "b", "--pid", "1", "--bogus", "2"]),
        ] {
            assert_eq!(parse(&bad), Err(CliError::Usage), "{bad:?}");
        }
    }
}
