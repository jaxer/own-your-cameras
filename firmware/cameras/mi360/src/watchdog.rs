//! Exact B-kernel JZ watchdog ownership. See docs/hardware.md for qualification.
//!
//! Opening arms the hardware. Feed only when the application's health policy
//! permits it. `disarm` is an explicit orderly-shutdown operation; Drop never
//! disarms, so an abort or unhandled failure still results in a watchdog reset.
//! No magic-close byte is ever written. A failed open with recovery_required
//! must terminate/reboot: the module retains uncertain ownership until exit.

use std::fmt;
use std::io;

pub const DEFAULT_TIMEOUT_SECONDS: u32 = 40;
pub const DEFAULT_FEED_SECONDS: u32 = 10;
pub const QUALIFIED_B_KERNEL_SHA256: &str =
    "a06bc71c6d94b59ef2bea947e152428efa00e8953bb578edbaf1865adc45b902";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Timeout {
    pub requested_seconds: u32,
    pub counter: u16,
    pub prescale: u32,
}
impl Timeout {
    /// A deliberately narrow range inside the driver-advertised 1..2048 seconds.
    /// GETTIMEOUT returns the request, not this driver's quantized timer period.
    pub fn new(seconds: u32) -> io::Result<Self> {
        if !(10..=120).contains(&seconds) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "watchdog timeout must be10..120 seconds",
            ));
        }
        let mut count = seconds * 46875;
        let mut prescale = 1;
        while count >= 65536 {
            count >>= 2;
            prescale *= 4;
        }
        Ok(Self {
            requested_seconds: seconds,
            counter: count as u16,
            prescale,
        })
    }
    /// Static driver arithmetic, not measured oscillator accuracy.
    pub fn nominal_period_micros(self) -> u64 {
        u64::from(self.counter) * u64::from(self.prescale) * 1_000_000 / 46875
    }
    pub fn maximum_feed_interval_seconds(self) -> u32 {
        self.requested_seconds / 4
    }
}

#[derive(Debug)]
pub struct OpenError {
    /// Explicit disarm failed after open armed the hardware. Exit/reboot needed.
    pub recovery_required: bool,
    source: io::Error,
}
impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.recovery_required {
            write!(f, "POISON watchdog may remain armed; reboot required: ")?;
        }
        self.source.fmt(f)
    }
}
impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
impl From<io::Error> for OpenError {
    fn from(source: io::Error) -> Self {
        Self {
            recovery_required: false,
            source,
        }
    }
}

#[repr(C)]
#[derive(Default)]
struct Support {
    options: u32,
    firmware: u32,
    identity: [u8; 32],
}
const _: () = assert!(std::mem::size_of::<Support>() == 40);
const _: () = assert!(std::mem::align_of::<Support>() == 4);

trait Driver {
    fn support(&mut self) -> io::Result<Support>;
    fn set_timeout(&mut self, seconds: i32) -> io::Result<i32>;
    fn get_timeout(&mut self) -> io::Result<i32>;
    fn feed(&mut self) -> io::Result<()>;
    fn disable(&mut self) -> io::Result<()>;
    fn close(&mut self);
}
struct Session<D: Driver> {
    driver: D,
    timeout: Timeout,
    armed: bool,
}
impl<D: Driver> Session<D> {
    // Driver has already been opened and armed. All further failure paths disarm.
    fn initialize(mut driver: D, timeout: Timeout) -> Result<Self, OpenError> {
        let result = (|| {
            let support = driver.support()?;
            let identity = support
                .identity
                .split(|b| *b == 0)
                .next()
                .unwrap_or_default();
            if support.options != 0x8180 || support.firmware != 0 || identity != b"jz Watchdog" {
                return Err(io::Error::other("unqualified watchdog support record"));
            }
            if driver.set_timeout(timeout.requested_seconds as i32)?
                != timeout.requested_seconds as i32
                || driver.get_timeout()? != timeout.requested_seconds as i32
            {
                return Err(io::Error::other("watchdog timeout readback differs"));
            }
            driver.feed()
        })();
        if let Err(first) = result {
            if let Err(cleanup) = driver.disable() {
                // Keep fd and flock together; never suggest an uncertain armed
                // device is available to a second owner in this process.
                let _ = Box::leak(Box::new(driver));
                return Err(OpenError {
                    recovery_required: true,
                    source: io::Error::other(format!("{first}; explicit disarm failed: {cleanup}")),
                });
            }
            driver.close();
            return Err(first.into());
        }
        Ok(Self {
            driver,
            timeout,
            armed: true,
        })
    }
    fn feed(&mut self) -> io::Result<()> {
        if !self.armed {
            return Err(io::Error::other("watchdog is disarmed"));
        }
        self.driver.feed()
    }
    fn disarm(&mut self) -> io::Result<()> {
        if self.armed {
            self.driver.disable()?;
            self.armed = false;
            // Do not restore timeout: this kernel's setter re-enables hardware.
            self.driver.close();
        }
        Ok(())
    }
}
impl<D: Driver> Drop for Session<D> {
    fn drop(&mut self) {
        self.driver.close();
    }
}

pub struct Watchdog {
    session: Session<backend::Target>,
}
impl Watchdog {
    /// Read-only qualification precedes exclusive open. Never stops stock owners.
    /// The caller must arrange stock watchdog shutdown and qualified kernel boot.
    pub fn open(timeout_seconds: u32) -> Result<Self, OpenError> {
        let timeout = Timeout::new(timeout_seconds)?;
        let driver = backend::Target::open()?;
        Ok(Self {
            session: Session::initialize(driver, timeout)?,
        })
    }
    pub fn timeout(&self) -> Timeout {
        self.session.timeout
    }
    pub fn feed(&mut self) -> io::Result<()> {
        self.session.feed()
    }
    /// Only call after independent worker health/shutdown decisions permit it.
    /// Failure retains ownership; dropping then deliberately leaves the timer armed.
    pub fn disarm(&mut self) -> io::Result<()> {
        self.session.disarm()
    }
}

#[cfg(all(target_os = "linux", target_arch = "mips"))]
mod backend {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::fs::{File, OpenOptions};
    use std::io::Read;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::Path;

    const GETSUPPORT: u32 = 0x40285700;
    const SETOPTIONS: u32 = 0x40045704;
    const KEEPALIVE: u32 = 0x40045705;
    const SETTIMEOUT: u32 = 0xc0045706;
    const GETTIMEOUT: u32 = 0x40045707;
    const _: () = assert!(std::mem::size_of::<libc::c_ulong>() == 4);
    pub(super) struct Target {
        device: Option<File>,
        lock: Option<File>,
    }
    fn read_small(path: &str, cap: usize) -> io::Result<String> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(cap as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > cap {
            return Err(io::Error::other(format!("{path} exceeds diagnostic cap")));
        }
        String::from_utf8(bytes).map_err(io::Error::other)
    }
    fn qualify() -> io::Result<()> {
        // The module parameter is boot-only (permission0), BSS defaultfalse.
        // Reject every override, even a false spelling, rather than parse aliases.
        if read_small("/proc/cmdline", 8192)?
            .split_whitespace()
            .any(|word| {
                word.split('=')
                    .next()
                    .is_some_and(|key| key == "nowayout" || key.ends_with(".nowayout"))
            })
        {
            return Err(io::Error::other(
                "watchdog nowayout boot override is unqualified",
            ));
        }
        let mtd = read_small("/proc/mtd", 4096)?;
        if mtd
            .lines()
            .filter(|line| *line == "mtd4: 001b0000 00008000 \"B_KERNEL\"")
            .count()
            != 1
        {
            return Err(io::Error::other("unqualified B kernel partition layout"));
        }
        let mut kernel = File::open("/dev/mtd4")?.take(0x1b0000);
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 8192];
        let mut size = 0;
        loop {
            let count = kernel.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            size += count;
            digest.update(&buffer[..count]);
        }
        if size != 0x1b0000 || format!("{:x}", digest.finalize()) != QUALIFIED_B_KERNEL_SHA256 {
            return Err(io::Error::other("unqualified B kernel bytes"));
        }
        // The independent /proc feeder uses the same physical timer and is not
        // protected by /dev/watchdog's exclusive-open bit. Require its OFF state.
        if read_small("/proc/jz/reset/wdt_control", 64)?.trim() != ">off<on" {
            return Err(io::Error::other(
                "independent kernel watchdog feeder is not off",
            ));
        }
        let symbols = read_small("/proc/kallsyms", 4 * 1024 * 1024)?;
        for wanted in [
            "80257030 t watchdog_open",
            "80256e68 t watchdog_stop",
            "80257874 t jz_wdt_set_timeout",
            "8052bc40 b nowayout",
        ] {
            if !symbols.lines().any(|line| line == wanted) {
                return Err(io::Error::other(format!(
                    "running kernel missing qualified symbol {wanted}"
                )));
            }
        }
        for entry in std::fs::read_dir("/proc")? {
            let entry = entry?;
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            if pid == std::process::id() {
                continue;
            }
            let command = match std::fs::read(entry.path().join("cmdline")) {
                Ok(command) => command,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            let name = command
                .split(|b| *b == 0)
                .next()
                .unwrap_or_default()
                .rsplit(|b| *b == b'/')
                .next();
            if name == Some(b"imi_watchdog".as_slice()) {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "stock imi_watchdog is running",
                ));
            }
            let fds = match std::fs::read_dir(entry.path().join("fd")) {
                Ok(fds) => fds,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            for fd in fds {
                match std::fs::read_link(fd?.path()) {
                    Ok(path)
                        if path == Path::new("/dev/watchdog")
                            || path == Path::new("/dev/watchdog0") =>
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::WouldBlock,
                            format!("watchdog owned by pid{pid}"),
                        ));
                    }
                    Ok(_) => (),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }
    impl Target {
        pub(super) fn open() -> io::Result<Self> {
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open("/tmp/mi360-watchdog.lock")?;
            // SAFETY: retained fd; advisory lock has no hardware side effects.
            if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(io::Error::other(format!(
                    "watchdog lock: {}",
                    io::Error::last_os_error()
                )));
            }
            qualify()?;
            let device = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open("/dev/watchdog")?;
            Ok(Self {
                device: Some(device),
                lock: Some(lock),
            })
        }
        fn call<T>(&self, operation: &str, request: u32, argument: &mut T) -> io::Result<()> {
            let file = self
                .device
                .as_ref()
                .ok_or_else(|| io::Error::other("watchdog closed"))?;
            // SAFETY: exact shipped MIPS request and repr(C)/fixed-width buffer;
            // the kernel copies at most its qualified structure size synchronously.
            if unsafe {
                libc::ioctl(
                    file.as_raw_fd(),
                    request as libc::c_ulong,
                    argument as *mut T,
                )
            } != 0
            {
                return Err(io::Error::other(format!(
                    "watchdog {operation}: {}",
                    io::Error::last_os_error()
                )));
            }
            Ok(())
        }
    }
    impl Driver for Target {
        fn support(&mut self) -> io::Result<Support> {
            let mut value = Support::default();
            self.call("GETSUPPORT", GETSUPPORT, &mut value)?;
            Ok(value)
        }
        fn set_timeout(&mut self, mut value: i32) -> io::Result<i32> {
            self.call("SETTIMEOUT", SETTIMEOUT, &mut value)?;
            Ok(value)
        }
        fn get_timeout(&mut self) -> io::Result<i32> {
            let mut value = 0_i32;
            self.call("GETTIMEOUT", GETTIMEOUT, &mut value)?;
            Ok(value)
        }
        fn feed(&mut self) -> io::Result<()> {
            self.call("KEEPALIVE", KEEPALIVE, &mut 0_i32)
        }
        fn disable(&mut self) -> io::Result<()> {
            self.call("SETOPTIONS(disable)", SETOPTIONS, &mut 1_i32)
        }
        fn close(&mut self) {
            self.device.take();
            self.lock.take();
        }
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "mips")))]
mod backend {
    use super::*;
    pub(super) struct Target;
    fn unavailable<T>() -> io::Result<T> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "watchdog needs qualified Mi360 MIPS hardware",
        ))
    }
    impl Target {
        pub(super) fn open() -> io::Result<Self> {
            unavailable()
        }
    }
    impl Driver for Target {
        fn support(&mut self) -> io::Result<Support> {
            unavailable()
        }
        fn set_timeout(&mut self, _: i32) -> io::Result<i32> {
            unavailable()
        }
        fn get_timeout(&mut self) -> io::Result<i32> {
            unavailable()
        }
        fn feed(&mut self) -> io::Result<()> {
            unavailable()
        }
        fn disable(&mut self) -> io::Result<()> {
            unavailable()
        }
        fn close(&mut self) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    struct Fake {
        events: Rc<RefCell<Vec<&'static str>>>,
        fail: Option<&'static str>,
        disable_fails: bool,
        seconds: i32,
        mismatch: bool,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                events: Rc::default(),
                fail: None,
                disable_fails: false,
                seconds: 5,
                mismatch: false,
            }
        }
        fn event(&mut self, name: &'static str) -> io::Result<()> {
            self.events.borrow_mut().push(name);
            if self.fail == Some(name) {
                Err(io::Error::other(name))
            } else {
                Ok(())
            }
        }
    }
    impl Driver for Fake {
        fn support(&mut self) -> io::Result<Support> {
            self.event("support")?;
            let mut support = Support {
                options: 0x8180,
                ..Support::default()
            };
            support.identity[..11].copy_from_slice(b"jz Watchdog");
            Ok(support)
        }
        fn set_timeout(&mut self, seconds: i32) -> io::Result<i32> {
            self.seconds = seconds;
            self.event("set")?;
            Ok(seconds)
        }
        fn get_timeout(&mut self) -> io::Result<i32> {
            self.event("get")?;
            Ok(self.seconds + i32::from(self.mismatch))
        }
        fn feed(&mut self) -> io::Result<()> {
            self.event("feed")
        }
        fn disable(&mut self) -> io::Result<()> {
            self.event("disable")?;
            if self.disable_fails {
                Err(io::Error::other("disable failed"))
            } else {
                Ok(())
            }
        }
        fn close(&mut self) {
            self.events.borrow_mut().push("close");
        }
    }
    #[test]
    fn timeout_quantization_is_bounded_and_feed_budget_has_margin() {
        for seconds in 10..=120 {
            let value = Timeout::new(seconds).unwrap();
            assert!(value.nominal_period_micros() <= u64::from(seconds) * 1_000_000);
            assert!(value.nominal_period_micros() > u64::from(seconds) * 999_000);
            assert!(value.maximum_feed_interval_seconds() * 3 < seconds);
        }
        assert_eq!(
            Timeout::new(40).unwrap(),
            Timeout {
                requested_seconds: 40,
                counter: 29296,
                prescale: 64
            }
        );
        for value in [0, 1, 9, 121, 2048, u32::MAX] {
            assert!(Timeout::new(value).is_err());
        }
    }
    #[test]
    fn every_initialization_failure_explicitly_disarms_before_close() {
        for operation in ["support", "set", "get", "feed"] {
            let mut driver = Fake::new();
            driver.fail = Some(operation);
            let events = driver.events.clone();
            let error = Session::initialize(driver, Timeout::new(40).unwrap())
                .err()
                .unwrap();
            assert!(!error.recovery_required);
            assert!(events.borrow().ends_with(&["disable", "close"]));
        }
    }
    #[test]
    fn timeout_mismatch_is_failure_and_disarm_does_not_restore_timeout() {
        let mut driver = Fake::new();
        driver.mismatch = true;
        let events = driver.events.clone();
        assert!(Session::initialize(driver, Timeout::new(40).unwrap()).is_err());
        assert_eq!(
            *events.borrow(),
            ["support", "set", "get", "disable", "close"]
        );
    }
    #[test]
    fn failed_initialization_disarm_retains_ownership_and_reports_poison() {
        let mut driver = Fake::new();
        driver.fail = Some("set");
        driver.disable_fails = true;
        let events = driver.events.clone();
        let error = Session::initialize(driver, Timeout::new(40).unwrap())
            .err()
            .unwrap();
        assert!(error.recovery_required && error.to_string().contains("POISON"));
        assert_eq!(*events.borrow(), ["support", "set", "disable"]);
    }
    #[test]
    fn orderly_disarm_is_idempotent_and_forbids_future_feeds() {
        let driver = Fake::new();
        let events = driver.events.clone();
        let mut session = Session::initialize(driver, Timeout::new(40).unwrap()).unwrap();
        session.disarm().unwrap();
        session.disarm().unwrap();
        assert!(session.feed().is_err());
        assert_eq!(
            *events.borrow(),
            ["support", "set", "get", "feed", "disable", "close"]
        );
    }
    #[test]
    fn normal_drop_does_not_disable_and_failed_feed_does_not_hide_health_failure() {
        let driver = Fake::new();
        let events = driver.events.clone();
        let mut session = Session::initialize(driver, Timeout::new(40).unwrap()).unwrap();
        session.driver.fail = Some("feed");
        assert!(session.feed().is_err());
        drop(session);
        assert_eq!(
            *events.borrow(),
            ["support", "set", "get", "feed", "feed", "close"]
        );
    }
    #[test]
    fn failed_orderly_disarm_keeps_handle_armed_and_can_be_retried() {
        let mut session = Session::initialize(Fake::new(), Timeout::new(40).unwrap()).unwrap();
        session.driver.disable_fails = true;
        assert!(session.disarm().is_err() && session.armed);
        session.driver.disable_fails = false;
        session.disarm().unwrap();
        assert!(!session.armed);
    }
}
