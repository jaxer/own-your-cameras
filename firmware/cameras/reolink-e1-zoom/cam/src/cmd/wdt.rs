//! `cam wdt`: diagnostic-only, under the shared runtime lease held by command dispatch.

use std::io;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::runtime::{Clock, HANDOFF_TIMEOUT, Monotonic};
use crate::signals;
use crate::wdt::{self, AlwaysHealthy, Feeder, LinuxWatchdog, WatchdogDevice};

const WATCHDOG: &str = "/dev/watchdog";
const POLL: Duration = Duration::from_millis(200);

/// Wait at most ten seconds for the diagnostic device, feed until SIGTERM, and disarm only
/// on an orderly stop. Any device failure exits nonzero and leaves recovery armed.
pub fn run() -> ExitCode {
    if let Err(error) = signals::install() {
        eprintln!("signal setup failed: {error}");
        return ExitCode::FAILURE;
    }
    match run_with(
        || LinuxWatchdog::open(WATCHDOG),
        &mut Monotonic::default(),
        &signals::STOP,
    ) {
        Ok(()) => {
            println!("wdt: disarmed");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("wdt: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The real diagnostic command loop with the existing watchdog/time system boundaries injected.
/// Caller must retain diagnostic ownership for this entire call, including waits and cleanup.
pub fn run_with<D: WatchdogDevice, C: Clock>(
    mut open: impl FnMut() -> io::Result<D>,
    clock: &mut C,
    stop: &AtomicBool,
) -> io::Result<()> {
    let deadline = clock.now().saturating_add(HANDOFF_TIMEOUT);
    let device = loop {
        if stop.load(Ordering::Relaxed) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "watchdog acquisition cancelled",
            ));
        }
        if clock.now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "watchdog still owned after 10s",
            ));
        }
        match open() {
            Ok(device) => break device,
            Err(e) if e.raw_os_error() == Some(libc::EBUSY) => {
                clock.wait(POLL.min(deadline.saturating_sub(clock.now())))
            }
            Err(error) => return Err(error),
        }
    };
    if stop.load(Ordering::Relaxed) || clock.now() >= deadline {
        return Err(io::Error::other("watchdog acquisition cancelled or late"));
    }
    let mut feeder = Feeder::arm(device, AlwaysHealthy, wdt::TIMEOUT_SECS)?;
    let mut next = clock.now().saturating_add(wdt::FEED_EVERY);
    while !stop.load(Ordering::Relaxed) {
        let now = clock.now();
        if now >= next {
            feeder.tick(now)?;
            next = now.saturating_add(wdt::FEED_EVERY);
        }
        clock.wait(POLL);
    }
    feeder.disarm()
}
