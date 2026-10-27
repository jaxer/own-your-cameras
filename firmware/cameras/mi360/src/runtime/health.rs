//! Worker freshness and watchdog supervision.
use super::*;

pub(super) struct Progress {
    pub(super) started: Instant,
    pub(super) rtsp: AtomicU32,
    pub(super) mqtt: AtomicU32,
    pub(super) hardware: AtomicU32,
    pub(super) main: AtomicU32,
    pub(super) main_video: AtomicU32,
    pub(super) sub_video: AtomicU32,
    pub(super) audio: AtomicU32,
    pub(super) qualified: AtomicBool,
}
impl Progress {
    pub(super) fn new() -> Self {
        Self {
            started: Instant::now(),
            rtsp: AtomicU32::new(0),
            mqtt: AtomicU32::new(0),
            hardware: AtomicU32::new(0),
            main: AtomicU32::new(0),
            main_video: AtomicU32::new(0),
            sub_video: AtomicU32::new(0),
            audio: AtomicU32::new(0),
            qualified: AtomicBool::new(false),
        }
    }
    pub(super) fn mark(&self, slot: &AtomicU32) {
        slot.store(
            (self.started.elapsed().as_millis() as u32).max(1),
            Ordering::Release,
        );
    }
    pub(super) fn healthy(&self) -> bool {
        let now = self.started.elapsed().as_millis() as u32;
        [
            (&self.rtsp, 5000),
            (&self.mqtt, 10000),
            (&self.hardware, 10000),
        ]
        .into_iter()
        .all(|(slot, budget)| {
            let stamp = slot.load(Ordering::Acquire);
            stamp != 0 && now.wrapping_sub(stamp) <= budget
        })
    }

    pub(super) fn media_fresh(&self, audio: bool) -> bool {
        let now = self.started.elapsed().as_millis() as u32;
        [&self.main, &self.main_video, &self.sub_video]
            .into_iter()
            .chain(audio.then_some(&self.audio))
            .all(|slot| {
                let stamp = slot.load(Ordering::Acquire);
                stamp != 0 && now.wrapping_sub(stamp) <= 3000
            })
    }
}

pub(super) fn feed_allowed(
    qualified: bool,
    startup_elapsed: Duration,
    workers_fresh: bool,
    media_fresh: bool,
) -> bool {
    if qualified {
        workers_fresh && media_fresh
    } else {
        startup_elapsed < Duration::from_secs(90)
    }
}

pub(super) struct Watcher {
    pub(super) stop: Arc<AtomicBool>,
    pub(super) thread: Option<JoinHandle<io::Result<()>>>,
}
impl Watcher {
    pub(super) fn start(
        watchdog: crate::watchdog::Watchdog,
        stop: Arc<AtomicBool>,
        progress: Arc<Progress>,
        audio: bool,
    ) -> io::Result<Self> {
        let worker_stop = stop.clone();
        let thread = std::thread::Builder::new()
            .name("watchdog".into())
            .stack_size(96 * 1024)
            .spawn(move || {
                let mut watchdog = watchdog;
                let mut next_feed = Instant::now();
                loop {
                    if worker_stop.load(Ordering::Acquire) || INTERRUPTED.load(Ordering::Relaxed) {
                        break;
                    }
                    let qualified = progress.qualified.load(Ordering::Acquire);
                    let allowed = feed_allowed(
                        qualified,
                        progress.started.elapsed(),
                        progress.healthy(),
                        progress.media_fresh(audio),
                    );
                    if !allowed {
                        worker_stop.store(true, Ordering::Release);
                        return Err(io::Error::other(
                            "watchdog qualification stopped; clean boot required",
                        ));
                    }
                    if Instant::now() >= next_feed {
                        if let Err(error) = watchdog.feed() {
                            worker_stop.store(true, Ordering::Release);
                            return Err(error);
                        }
                        next_feed = Instant::now() + Duration::from_secs(5);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                // Ordinary close preserves the armed reset on every daemon exit.
                Ok(())
            })?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
    pub(super) fn finish(&mut self) -> io::Result<()> {
        self.stop.store(true, Ordering::Release);
        self.thread.take().map_or(Ok(()), join_worker)
    }
}
impl Drop for Watcher {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("watchdog stopped: {error}");
        }
    }
}
