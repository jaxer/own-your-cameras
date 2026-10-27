//! The Linux watchdog device and its feeder, which feeds only while the pipeline reports healthy, so a hung
//! pipeline lets the SoC reset.

use std::io;
use std::time::Duration;

/// Timeout the vendor stack uses (`/sys/class/watchdog/watchdog0/timeout`).
pub const TIMEOUT_SECS: u32 = 60;
/// Feed cadence; six feeds fit in one timeout.
pub const FEED_EVERY: Duration = Duration::from_secs(10);

/// The watchdog hardware; the real one is `LinuxWatchdog`, tests use a fake.
pub trait WatchdogDevice {
    /// Return the timeout actually accepted by the driver, which may adjust the request.
    fn set_timeout(&mut self, secs: u32) -> io::Result<u32>;
    fn keepalive(&mut self) -> io::Result<()>;
    /// Orderly stop (magic close): the device will not reset the SoC afterwards.
    fn disarm(&mut self) -> io::Result<()>;
}

/// Is the pipeline alive at `now` (monotonic time since process start)?
pub trait Health {
    fn healthy(&self, now: Duration) -> bool;
}

/// Health for a process with no pipeline yet (`cam wdt` alone): always healthy.
pub struct AlwaysHealthy;

impl Health for AlwaysHealthy {
    fn healthy(&self, _now: Duration) -> bool {
        true
    }
}

/// Health from a heartbeat the pipeline stamps; stale for more than `max_age` means hung.
#[derive(Clone)]
pub struct Heartbeat {
    last_ms: std::sync::Arc<std::sync::atomic::AtomicU64>,
    max_age: Duration,
}

impl Heartbeat {
    pub fn new(max_age: Duration) -> Self {
        Self {
            last_ms: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            max_age,
        }
    }

    /// Called by the pipeline each time it makes progress.
    pub fn beat(&self, now: Duration) {
        let ms = u64::try_from(now.as_millis()).unwrap_or(u64::MAX);
        self.last_ms.store(ms, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Health for Heartbeat {
    fn healthy(&self, now: Duration) -> bool {
        let last = Duration::from_millis(self.last_ms.load(std::sync::atomic::Ordering::Relaxed));
        now.saturating_sub(last) <= self.max_age
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Tick {
    Fed,
    /// Pipeline unhealthy: no keepalive, the watchdog will reset the SoC.
    Withheld,
}

pub struct Feeder<D, H> {
    dev: D,
    health: H,
}

impl<D: WatchdogDevice, H: Health> Feeder<D, H> {
    /// Sets the timeout (which also arms and feeds the device).
    pub fn arm(mut dev: D, health: H, timeout_secs: u32) -> io::Result<Self> {
        if dev.set_timeout(timeout_secs)? != timeout_secs {
            return Err(io::Error::other(
                "watchdog driver adjusted requested timeout",
            ));
        }
        dev.keepalive()?;
        Ok(Self { dev, health })
    }

    pub fn tick(&mut self, now: Duration) -> io::Result<Tick> {
        if !self.health.healthy(now) {
            return Ok(Tick::Withheld);
        }
        self.dev.keepalive()?;
        Ok(Tick::Fed)
    }

    /// Orderly exit: the SoC stays up.
    pub fn disarm(mut self) -> io::Result<()> {
        self.dev.disarm()
    }
}

/// Open the device, waiting while another process (the vendor `router`) still holds it: Linux watchdog
/// devices are single-open, and once `router` is killed the timer keeps running, so the takeover must happen
/// within the timeout.
pub fn open_when_free<D>(
    mut open: impl FnMut() -> io::Result<D>,
    mut wait: impl FnMut() -> bool,
) -> io::Result<D> {
    loop {
        match open() {
            Err(e) if e.raw_os_error() == Some(libc::EBUSY) => {
                if !wait() {
                    return Err(e);
                }
            }
            other => return other,
        }
    }
}

/// `/dev/watchdog` through the Linux watchdog ioctls.
pub struct LinuxWatchdog {
    file: std::fs::File,
}

// _IOWR('W', 6, int) and _IOR('W', 5, int) from <linux/watchdog.h> on 32-bit ARM.
const WDIOC_SETTIMEOUT: libc::c_ulong = 0xC004_5706;
const WDIOC_KEEPALIVE: libc::c_ulong = 0x8004_5705;

impl LinuxWatchdog {
    pub fn open(path: &str) -> io::Result<Self> {
        let file = std::fs::OpenOptions::new().write(true).open(path)?;
        Ok(Self { file })
    }

    fn ioctl(&self, req: libc::c_ulong, mut arg: libc::c_int) -> io::Result<libc::c_int> {
        use std::os::fd::AsRawFd;
        // SAFETY: the fd is open for the lifetime of `self`, and both requests read or write one c_int
        // through the pointer, which points at a live local.
        let rc = unsafe {
            libc::ioctl(
                self.file.as_raw_fd(),
                req as _,
                &mut arg as *mut libc::c_int,
            )
        };
        if rc < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(arg)
        }
    }
}

impl WatchdogDevice for LinuxWatchdog {
    fn set_timeout(&mut self, secs: u32) -> io::Result<u32> {
        let request = libc::c_int::try_from(secs).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "watchdog timeout out of range")
        })?;
        let actual = self.ioctl(WDIOC_SETTIMEOUT, request)?;
        u32::try_from(actual).map_err(|_| io::Error::other("negative watchdog timeout"))
    }

    fn keepalive(&mut self) -> io::Result<()> {
        self.ioctl(WDIOC_KEEPALIVE, 0).map(|_| ())
    }

    fn disarm(&mut self) -> io::Result<()> {
        use std::io::Write;
        self.file.write_all(b"V")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct Log(Rc<RefCell<Vec<String>>>);

    struct FakeDev(Rc<RefCell<Vec<String>>>);

    impl WatchdogDevice for FakeDev {
        fn set_timeout(&mut self, secs: u32) -> io::Result<u32> {
            self.0.borrow_mut().push(format!("timeout {secs}"));
            Ok(secs)
        }
        fn keepalive(&mut self) -> io::Result<()> {
            self.0.borrow_mut().push("keepalive".into());
            Ok(())
        }
        fn disarm(&mut self) -> io::Result<()> {
            self.0.borrow_mut().push("disarm".into());
            Ok(())
        }
    }

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn arm_sets_timeout_then_feeds() {
        let log = Log::default();
        Feeder::arm(FakeDev(log.0.clone()), AlwaysHealthy, 60).unwrap();
        assert_eq!(*log.0.borrow(), ["timeout 60", "keepalive"]);
    }

    #[test]
    fn healthy_pipeline_is_fed() {
        let log = Log::default();
        let mut f = Feeder::arm(FakeDev(log.0.clone()), AlwaysHealthy, 60).unwrap();
        assert_eq!(f.tick(secs(10)).unwrap(), Tick::Fed);
        assert_eq!(log.0.borrow().last().unwrap(), "keepalive");
        assert_eq!(log.0.borrow().len(), 3);
    }

    #[test]
    fn stalled_pipeline_is_not_fed() {
        let log = Log::default();
        let hb = Heartbeat::new(secs(30));
        hb.beat(secs(5));
        let mut f = Feeder::arm(FakeDev(log.0.clone()), hb.clone(), 60).unwrap();
        assert_eq!(f.tick(secs(20)).unwrap(), Tick::Fed);
        assert_eq!(f.tick(secs(36)).unwrap(), Tick::Withheld);
        assert_eq!(log.0.borrow().len(), 3, "no keepalive after the stall");
        hb.beat(secs(40));
        assert_eq!(f.tick(secs(41)).unwrap(), Tick::Fed);
    }

    #[test]
    fn orderly_exit_disarms() {
        let log = Log::default();
        let f = Feeder::arm(FakeDev(log.0.clone()), AlwaysHealthy, 60).unwrap();
        f.disarm().unwrap();
        assert_eq!(log.0.borrow().last().unwrap(), "disarm");
    }

    #[test]
    fn open_waits_while_busy_then_succeeds() {
        let mut busy = 2;
        let mut waits = 0;
        let got = open_when_free(
            || {
                if busy > 0 {
                    busy -= 1;
                    Err(io::Error::from_raw_os_error(libc::EBUSY))
                } else {
                    Ok(7)
                }
            },
            || {
                waits += 1;
                true
            },
        );
        assert_eq!(got.unwrap(), 7);
        assert_eq!(waits, 2);
    }

    #[test]
    fn open_gives_up_when_told_and_passes_other_errors() {
        let busy =
            open_when_free::<()>(|| Err(io::Error::from_raw_os_error(libc::EBUSY)), || false);
        assert_eq!(busy.unwrap_err().raw_os_error(), Some(libc::EBUSY));
        let other =
            open_when_free::<()>(|| Err(io::Error::from_raw_os_error(libc::ENOENT)), || true);
        assert_eq!(other.unwrap_err().raw_os_error(), Some(libc::ENOENT));
    }
}
