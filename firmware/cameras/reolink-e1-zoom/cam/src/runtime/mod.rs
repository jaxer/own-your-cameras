//! Production watchdog policy. All timestamps share the watchdog owner's monotonic clock.
mod health_file;
pub mod health_watch;
pub mod production;
pub mod s00;
pub mod workers;
pub use health_file::{HealthFile, HealthObserver, valid_attempt};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

/// Includes graph creation, motor homing and service startup; never extended by progress.
pub const STARTUP_GRACE: Duration = Duration::from_secs(90);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    MainEncoder,
    SubEncoder,
    Network,
    Rtsp,
    Mqtt,
    Hardware,
}

impl Progress {
    pub const ALL: [Self; 6] = [
        Self::MainEncoder,
        Self::SubEncoder,
        Self::Network,
        Self::Rtsp,
        Self::Mqtt,
        Self::Hardware,
    ];

    pub fn max_age(self) -> Duration {
        Duration::from_secs(match self {
            Self::MainEncoder | Self::SubEncoder | Self::Rtsp => 5,
            Self::Mqtt => 10,
            Self::Network => 30,
            Self::Hardware => 60,
        })
    }
}

/// Fixed-size, lock-free observations: the feeder cannot block behind a stuck hardware worker.
#[derive(Clone)]
pub struct LifecycleHealth(Arc<Observations>);

struct Observations {
    last: [AtomicU64; 6],
    failed: AtomicBool,
}

/// Keep one inside each required worker, including nested workers and the feeder.
/// Any exit (success, error, unwind or failed spawn) permanently cancels feeding.
pub struct WorkerLease(LifecycleHealth);

impl Drop for WorkerLease {
    fn drop(&mut self) {
        self.0.stop();
    }
}

impl Default for LifecycleHealth {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleHealth {
    pub fn new() -> Self {
        Self(Arc::new(Observations {
            last: std::array::from_fn(|_| AtomicU64::new(u64::MAX)),
            failed: AtomicBool::new(false),
        }))
    }

    pub fn worker(&self) -> WorkerLease {
        WorkerLease(self.clone())
    }

    /// Call before joining any service or beginning any media teardown, even on normal exit.
    pub fn stop(&self) {
        self.0.failed.store(true, Ordering::Release);
    }

    pub fn stopped(&self) -> bool {
        self.0.failed.load(Ordering::Acquire)
    }

    pub fn beat(&self, source: Progress, now: Duration) {
        self.0.last[source as usize].store(millis(now), Ordering::Release);
    }

    pub fn progressing(&self, now: Duration) -> bool {
        !self.stopped()
            && Progress::ALL.into_iter().all(|source| {
                let last = self.0.last[source as usize].load(Ordering::Acquire);
                // A concurrent beat may be newer than the observer's `now` snapshot.
                last != u64::MAX && millis(now).saturating_sub(last) < millis(source.max_age())
            })
    }

    pub fn may_feed(&self, now: Duration) -> bool {
        !self.stopped() && (now < STARTUP_GRACE || self.progressing(now))
    }
}

impl LifecycleHealth {
    /// Time of the latest beat from `source`, if it ever beat. For tests and diagnostics.
    pub fn last_beat(&self, source: Progress) -> Option<Duration> {
        match self.0.last[source as usize].load(Ordering::Acquire) {
            u64::MAX => None,
            ms => Some(Duration::from_millis(ms)),
        }
    }
}

/// What a service worker gets to report progress: the shared health and the attempt's shared clock.
/// A beat means one loop iteration or unit of work completed; nothing else may call it for a worker.
#[derive(Clone)]
pub struct Beats {
    health: LifecycleHealth,
    now: Arc<dyn Fn() -> Duration + Send + Sync>,
}

impl Beats {
    pub fn new<C: Clock + Clone + Send + Sync + 'static>(
        health: LifecycleHealth,
        clock: C,
    ) -> Self {
        Self {
            health,
            now: Arc::new(move || clock.now()),
        }
    }

    pub fn now(&self) -> Duration {
        (self.now)()
    }

    pub fn beat(&self, source: Progress) {
        self.health.beat(source, self.now());
    }

    pub fn lease(&self) -> WorkerLease {
        self.health.worker()
    }

    /// Terminal cancellation shared by services that have not finished starting yet.
    pub fn stopped(&self) -> bool {
        self.health.stopped()
    }
}

fn millis(time: Duration) -> u64 {
    u64::try_from(time.as_millis())
        .unwrap_or(u64::MAX - 1)
        .min(u64::MAX - 1)
}

use crate::wdt::{FEED_EVERY, TIMEOUT_SECS, WatchdogDevice};
use std::io;

#[derive(Debug, thiserror::Error)]
#[error("runtime stopped")]
struct Cancelled;

fn cancelled() -> io::Error {
    io::Error::new(io::ErrorKind::Interrupted, Cancelled)
}

pub(crate) fn is_cancelled(error: &io::Error) -> bool {
    error.get_ref().is_some_and(|cause| cause.is::<Cancelled>())
}

/// Independent watchdog/health observation cadence.
pub const POLL_EVERY: Duration = Duration::from_secs(1);
pub const MAX_POLL_GAP: Duration = Duration::from_secs(3);
/// Strictly longer than STARTUP_GRACE + the 60-second hardware timeout.
pub const SUSTAINED_HEALTH: Duration = Duration::from_secs(180);

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthStatus {
    pub elapsed_ms: u64,
    pub healthy_for_ms: u64,
    pub qualifying: bool,
}

/// Production's watchdog, with no way to disarm it: dropping the device closes it without the magic `V`, so
/// the timer stays armed, also on panic or process exit.
pub struct WatchdogSession<D> {
    dev: D,
    lease: WorkerLease,
    next_feed: Duration,
    last_poll: Duration,
    healthy_since: Option<Duration>,
}

impl<D: WatchdogDevice> WatchdogSession<D> {
    pub fn arm(dev: D, health: LifecycleHealth, now: Duration) -> io::Result<Self> {
        Self::arm_at(dev, health, || Ok(now))
    }

    fn arm_at(
        mut dev: D,
        health: LifecycleHealth,
        mut checked_now: impl FnMut() -> io::Result<Duration>,
    ) -> io::Result<Self> {
        let lease = health.worker();
        let now = checked_now()?;
        if !health.may_feed(now) {
            return Err(io::Error::other("runtime stopped or startup grace expired"));
        }
        if dev.set_timeout(TIMEOUT_SECS)? != TIMEOUT_SECS {
            return Err(io::Error::other(
                "watchdog driver adjusted requested timeout",
            ));
        }
        // SETTIMEOUT can block. Observe cancellation/deadline again before a separate feed.
        let now = checked_now()?;
        if !health.may_feed(now) {
            return Err(io::Error::other("runtime stopped while arming watchdog"));
        }
        dev.keepalive()?;
        Ok(Self {
            dev,
            lease,
            next_feed: now.saturating_add(FEED_EVERY),
            last_poll: now,
            healthy_since: None,
        })
    }

    /// Poll on the independent feeder thread, never inside a network/media/hardware loop.
    pub fn poll(&mut self, now: Duration) -> io::Result<HealthStatus> {
        let health = &self.lease.0;
        if health.stopped() {
            return Err(cancelled());
        }
        if now
            .checked_sub(self.last_poll)
            .is_none_or(|gap| gap > MAX_POLL_GAP)
            || !health.may_feed(now)
        {
            health.stop();
            return Err(io::Error::other("runtime stopped or progress stale"));
        }
        self.last_poll = now;
        let healthy_for = if health.progressing(now) {
            now.saturating_sub(*self.healthy_since.get_or_insert(now))
        } else {
            self.healthy_since = None;
            Duration::ZERO
        };
        if now >= self.next_feed {
            if let Err(error) = self.dev.keepalive() {
                health.stop();
                return Err(error);
            }
            self.next_feed = now.saturating_add(FEED_EVERY);
        }
        Ok(HealthStatus {
            elapsed_ms: millis(now),
            healthy_for_ms: millis(healthy_for),
            qualifying: healthy_for >= SUSTAINED_HEALTH,
        })
    }
}

pub const HANDOFF_TIMEOUT: Duration = Duration::from_secs(10);
const HANDOFF_RETRY: Duration = Duration::from_millis(200);

/// Clock origin is the beginning of this attempt, before S00 is asked to release the watchdog.
/// Share that origin with every progress producer; wait must advance monotonic time.
pub trait Clock {
    fn now(&self) -> Duration;
    fn wait(&mut self, duration: Duration);
}

#[derive(Clone)]
pub struct Monotonic(std::time::Instant);
impl Default for Monotonic {
    fn default() -> Self {
        Self(std::time::Instant::now())
    }
}
impl Clock for Monotonic {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
    fn wait(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// The implementation must retain exclusive runtime ownership for the whole watchdog session.
/// Requesting release must refuse an existing handoff; it must not kill another watchdog owner.
pub trait Handoff {
    type Device: WatchdogDevice;
    fn request_release(&mut self) -> io::Result<()>;
    fn open(&mut self) -> io::Result<Self::Device>;
}

pub fn take_over<H: Handoff, C: Clock>(
    handoff: &mut H,
    clock: &mut C,
    health: LifecycleHealth,
    stop: &AtomicBool,
) -> io::Result<WatchdogSession<H::Device>> {
    let result = (|| {
        let deadline = clock.now().saturating_add(HANDOFF_TIMEOUT);
        if health.stopped() || stop.load(Ordering::Acquire) {
            return Err(cancelled());
        }
        handoff.request_release()?;
        loop {
            if health.stopped() || stop.load(Ordering::Acquire) {
                return Err(cancelled());
            }
            if clock.now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "watchdog still owned after 10s",
                ));
            }
            match handoff.open() {
                Ok(device) => {
                    return WatchdogSession::arm_at(device, health.clone(), || {
                        if stop.load(Ordering::Acquire) {
                            return Err(cancelled());
                        }
                        let now = clock.now();
                        if now >= deadline {
                            return Err(io::Error::new(
                                io::ErrorKind::TimedOut,
                                "watchdog acquisition exceeded handoff deadline",
                            ));
                        }
                        Ok(now)
                    });
                }
                Err(e) if e.raw_os_error() == Some(libc::EBUSY) => {
                    clock.wait(HANDOFF_RETRY.min(deadline.saturating_sub(clock.now())));
                }
                Err(e) => return Err(e),
            }
        }
    })();
    if result.is_err() {
        health.stop();
    }
    result
}

/// The feeder, on its own thread: it never calls or joins a service, so a stuck service cannot keep it
/// feeding, and a blocked watchdog call or health write simply lets the armed timer reset the boot. `stop` is
/// checked directly, so SIGTERM ends feeding even if the lifecycle owner is stuck. The owner starts HDAL and
/// the services only after the first `Some` status. Every return stops feeding and invalidates the health.
pub fn supervise<H: Handoff, C: Clock>(
    handoff: &mut H,
    clock: &mut C,
    health: LifecycleHealth,
    stop: &AtomicBool,
    mut publish: impl FnMut(Option<HealthStatus>) -> io::Result<()>,
) -> io::Result<()> {
    let _lease = health.worker();
    let result = (|| {
        publish(None)?;
        if stop.load(Ordering::Acquire) {
            return Err(cancelled());
        }
        let mut watchdog = take_over(handoff, clock, health.clone(), stop)?;
        loop {
            if stop.load(Ordering::Acquire) {
                health.stop();
            }
            publish(Some(watchdog.poll(clock.now())?))?;
            clock.wait(POLL_EVERY);
        }
    })();
    health.stop();
    match (result, publish(None)) {
        (Err(primary), Err(cleanup)) => Err(io::Error::other(format!(
            "{primary}; health invalidation: {cleanup}"
        ))),
        (result, Ok(())) => result,
        (Ok(()), Err(cleanup)) => Err(cleanup),
    }
}
