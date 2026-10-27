//! Taking the watchdog over from the stock `S00` feeder at cold boot.

use std::fs::File;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::wdt::LinuxWatchdog;

/// Holds the runtime lock for the rest of the boot. The lock and the flag are never removed, so a second
/// runtime or a same-boot restart is refused rather than taking over.
pub struct S00Handoff {
    lock: File,
    flag: PathBuf,
    device: PathBuf,
}

impl S00Handoff {
    /// Takes only the runtime lock; the flag and the watchdog are untouched until the handoff.
    pub fn acquire(lock: &Path, flag: &Path, device: &Path) -> io::Result<Self> {
        Ok(Self {
            lock: crate::ownership::production(lock)?,
            flag: flag.to_owned(),
            device: device.to_owned(),
        })
    }
}

impl super::Handoff for S00Handoff {
    type Device = LinuxWatchdog;

    fn request_release(&mut self) -> io::Result<()> {
        // The byte in the lock marks this boot as production's before S00 lets go, so no diagnostic can feed
        // the watchdog afterwards, whatever happens to the flag or to this process.
        if self.lock.metadata()?.len() != 0 {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "production already attempted this boot",
            ));
        }
        self.lock.write_all(b"P")?;
        // S00 stops feeding when the flag exists; create_new refuses a stale flag or a symlink.
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&self.flag)?;
        Ok(())
    }

    fn open(&mut self) -> io::Result<LinuxWatchdog> {
        LinuxWatchdog::open(&self.device.to_string_lossy())
    }
}
