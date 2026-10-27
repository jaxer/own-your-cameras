//! The process-wide stop flag: SIGTERM/SIGINT set it, long-running commands poll it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

pub fn install() -> std::io::Result<()> {
    for sig in [libc::SIGTERM, libc::SIGINT] {
        install_handler(sig)?;
    }
    Ok(())
}

/// OS boundary for installing the stop handler; failure must abort command startup.
pub fn install_handler(sig: libc::c_int) -> std::io::Result<()> {
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    let previous = unsafe {
        libc::signal(
            sig,
            on_signal as extern "C" fn(libc::c_int) as libc::sighandler_t,
        )
    };
    if previous == libc::SIG_ERR {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Sleeps up to `d` in short steps; false once a stop signal arrived.
pub fn nap(d: Duration) -> bool {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if STOP.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(200).min(d));
    }
    !STOP.load(Ordering::Relaxed)
}
