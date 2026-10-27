use cam::runtime::{LifecycleHealth, Progress};
use std::time::Duration;

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

#[test]
fn startup_grace_is_bounded_and_both_encoders_must_progress() {
    let health = LifecycleHealth::new();
    assert!(health.may_feed(secs(89)));
    assert!(
        !health.may_feed(secs(90)),
        "never-started pipeline must expire"
    );
    for source in Progress::ALL {
        if source != Progress::SubEncoder {
            health.beat(source, secs(90));
        }
    }
    assert!(!health.may_feed(secs(90)), "main alone is insufficient");
    health.beat(Progress::SubEncoder, secs(90));
    assert!(health.may_feed(secs(94)));
    health.beat(Progress::MainEncoder, secs(95));
    assert!(
        !health.may_feed(secs(95)),
        "stalled sub encoder must expire"
    );
}

#[test]
fn service_exit_error_and_unwind_cancel_even_startup_grace() {
    for outcome in 0..3 {
        let health = LifecycleHealth::new();
        let lease = health.worker();
        let result = std::thread::spawn(move || {
            let _lease = lease;
            match outcome {
                0 => Ok(()),
                1 => Err(std::io::Error::other("hardware failed")),
                _ => panic!("hardware panic"),
            }
        })
        .join();
        assert_eq!(result.is_err(), outcome == 2);
        for source in Progress::ALL {
            health.beat(source, secs(1));
        }
        assert!(!health.may_feed(secs(1)), "worker exit must cancel grace");
        assert!(
            !health.progressing(secs(1)),
            "failure cannot be revived by frames"
        );
    }
}

use cam::runtime::WatchdogSession;
use cam::wdt::WatchdogDevice;
use std::io;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Device {
    events: Arc<Mutex<Vec<&'static str>>>,
    fail: Option<&'static str>,
    accepted_timeout: Option<u32>,
}
impl Device {
    fn event(&self, event: &'static str) -> io::Result<()> {
        self.events.lock().unwrap().push(event);
        if self.fail == Some(event) {
            Err(io::Error::other(event))
        } else {
            Ok(())
        }
    }
}
impl WatchdogDevice for Device {
    fn set_timeout(&mut self, secs: u32) -> io::Result<u32> {
        assert_eq!(secs, 60);
        self.event("arm")?;
        Ok(self.accepted_timeout.unwrap_or(secs))
    }
    fn keepalive(&mut self) -> io::Result<()> {
        self.event("feed")
    }
    fn disarm(&mut self) -> io::Result<()> {
        self.event("disarm")
    }
}

fn beat_all(health: &LifecycleHealth, at: u64) {
    for source in Progress::ALL {
        health.beat(source, secs(at));
    }
}

#[test]
fn feeder_latches_stall_and_never_disarms_on_exit() {
    let health = LifecycleHealth::new();
    let device = Device::default();
    let mut watchdog = WatchdogSession::arm(device.clone(), health.clone(), secs(0)).unwrap();
    for now in 1..90 {
        watchdog.poll(secs(now)).unwrap();
    }
    assert_eq!(
        device
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| **e == "feed")
            .count(),
        9
    );
    assert!(watchdog.poll(secs(90)).is_err());
    beat_all(&health, 91);
    assert!(
        watchdog.poll(secs(91)).is_err(),
        "late recovery must not revive feeding"
    );
    drop(watchdog);
    assert!(!device.events.lock().unwrap().contains(&"disarm"));
    assert_eq!(device.events.lock().unwrap().len(), 10, "no late feed");
}

#[test]
fn launcher_health_requires_180_seconds_of_all_sources_and_expires_on_stop() {
    let health = LifecycleHealth::new();
    let mut watchdog = WatchdogSession::arm(Device::default(), health.clone(), secs(0)).unwrap();
    assert!(!watchdog.poll(secs(0)).unwrap().qualifying);
    for now in 1..181 {
        beat_all(&health, now);
        let report = watchdog.poll(secs(now)).unwrap();
        assert!(
            !report.qualifying,
            "startup age alone is not sustained progress"
        );
    }
    beat_all(&health, 181);
    let report = watchdog.poll(secs(181)).unwrap();
    assert!(report.qualifying);
    assert_eq!(report.healthy_for_ms, 180_000);
    health.stop();
    assert!(watchdog.poll(secs(182)).is_err());
}

#[test]
fn arm_and_feed_errors_stop_the_owner_without_magic_close() {
    for fail in ["arm", "feed"] {
        let health = LifecycleHealth::new();
        let device = Device {
            fail: Some(fail),
            ..Device::default()
        };
        assert!(WatchdogSession::arm(device.clone(), health.clone(), secs(0)).is_err());
        assert!(health.stopped());
        assert!(!device.events.lock().unwrap().contains(&"disarm"));
    }
}

#[test]
fn delayed_or_backward_watchdog_polls_cannot_claim_unobserved_health() {
    for next in [secs(9), secs(14)] {
        let health = LifecycleHealth::new();
        let mut watchdog =
            WatchdogSession::arm(Device::default(), health.clone(), secs(10)).unwrap();
        beat_all(&health, next.as_secs());
        assert!(watchdog.poll(next).is_err());
        assert!(health.stopped());
    }
}

use cam::runtime::{Clock, Handoff, take_over};

#[derive(Default)]
struct FakeClock(Duration);
impl Clock for FakeClock {
    fn now(&self) -> Duration {
        self.0
    }
    fn wait(&mut self, duration: Duration) {
        self.0 += duration;
    }
}
struct Transfer {
    busy: usize,
    fail: Option<i32>,
    released: bool,
    device: Device,
}
impl Handoff for Transfer {
    type Device = Device;
    fn request_release(&mut self) -> io::Result<()> {
        assert!(!self.released);
        self.released = true;
        Ok(())
    }
    fn open(&mut self) -> io::Result<Device> {
        assert!(self.released, "S00 must receive the flag first");
        if let Some(errno) = self.fail {
            return Err(io::Error::from_raw_os_error(errno));
        }
        if self.busy > 0 {
            self.busy -= 1;
            return Err(io::Error::from_raw_os_error(libc::EBUSY));
        }
        Ok(self.device.clone())
    }
}
fn transfer(busy: usize) -> Transfer {
    Transfer {
        busy,
        fail: None,
        released: false,
        device: Device::default(),
    }
}

#[test]
fn s00_transfer_is_bounded_and_never_feeds_someone_elses_watchdog() {
    let mut handoff = transfer(usize::MAX);
    let mut clock = FakeClock::default();
    let health = LifecycleHealth::new();
    assert_eq!(
        take_over(
            &mut handoff,
            &mut clock,
            health.clone(),
            &std::sync::atomic::AtomicBool::new(false)
        )
        .err()
        .unwrap()
        .kind(),
        io::ErrorKind::TimedOut
    );
    assert_eq!(clock.now(), secs(10));
    assert!(health.stopped());
    assert!(handoff.device.events.lock().unwrap().is_empty());
}

#[test]
fn transfer_success_and_open_arm_feed_failures_reach_the_owner() {
    let mut handoff = transfer(3);
    let mut clock = FakeClock::default();
    let health = LifecycleHealth::new();
    let watchdog = take_over(
        &mut handoff,
        &mut clock,
        health.clone(),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(clock.now(), Duration::from_millis(600));
    assert_eq!(*handoff.device.events.lock().unwrap(), ["arm", "feed"]);
    drop(watchdog);
    assert!(health.stopped());
    for fail in ["open", "arm", "feed"] {
        let mut handoff = transfer(0);
        if fail == "open" {
            handoff.fail = Some(libc::EACCES);
        } else {
            handoff.device.fail = Some(fail);
        }
        let health = LifecycleHealth::new();
        let mut clock = FakeClock::default();
        assert!(
            take_over(
                &mut handoff,
                &mut clock,
                health.clone(),
                &std::sync::atomic::AtomicBool::new(false)
            )
            .is_err()
        );
        assert_eq!(clock.now(), secs(0), "non-busy errors must not retry");
        assert!(health.stopped());
        assert!(!handoff.device.events.lock().unwrap().contains(&"disarm"));
    }
}

#[test]
fn every_required_service_and_either_or_both_encoders_can_expire() {
    for (stalled, budget) in [
        (Progress::MainEncoder, 5),
        (Progress::SubEncoder, 5),
        (Progress::Network, 30),
        (Progress::Rtsp, 5),
        (Progress::Mqtt, 10),
        (Progress::Hardware, 60),
    ] {
        let health = LifecycleHealth::new();
        beat_all(&health, 90);
        let deadline = 90 + budget;
        for source in Progress::ALL {
            if source != stalled {
                health.beat(source, secs(deadline));
            }
        }
        assert!(health.may_feed(secs(deadline) - Duration::from_millis(1)));
        assert!(!health.may_feed(secs(deadline)), "{stalled:?}");
    }
    let health = LifecycleHealth::new();
    beat_all(&health, 90);
    for source in [
        Progress::Network,
        Progress::Rtsp,
        Progress::Mqtt,
        Progress::Hardware,
    ] {
        health.beat(source, secs(95));
    }
    assert!(!health.may_feed(secs(95)), "both encoders stalled");
}

#[test]
fn s00_adapter_refuses_duplicate_runtime_and_existing_handoff_flag() {
    use cam::runtime::s00::S00Handoff;
    let dir = std::env::temp_dir().join(format!("cam-s00-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let lock = dir.join("runtime.lock");
    let flag = dir.join("wdt_flag");
    let device = dir.join("absent-device");
    let mut first = S00Handoff::acquire(&lock, &flag, &device).unwrap();
    assert!(!flag.exists(), "locking alone must not release S00");
    assert!(S00Handoff::acquire(&lock, &flag, &device).is_err());
    first.request_release().unwrap();
    assert!(flag.is_file());
    assert!(first.open().is_err());
    drop(first);
    let mut second = S00Handoff::acquire(&lock, &flag, &device).unwrap();
    assert_eq!(
        second.request_release().unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    drop(second);
    assert!(
        flag.exists(),
        "no same-boot retry may erase the transfer marker"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

use cam::runtime::{HealthFile, HealthObserver, HealthStatus};

#[test]
fn launcher_rejects_wrong_attempt_and_stale_health_files() {
    let dir = std::env::temp_dir().join(format!("cam-health-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut file = HealthFile::new(&dir, "boot-123-attempt-2", 123).unwrap();
    assert!(HealthFile::new(&dir, "../escape", 123).is_err());
    let mut observer = HealthObserver::new("boot-123-attempt-2", 123);
    let mut wrong = HealthObserver::new("boot-123-attempt-1", 123);
    let mut other_pid = HealthObserver::new("boot-123-attempt-2", 124);
    let status = HealthStatus {
        elapsed_ms: 200_000,
        healthy_for_ms: 180_000,
        qualifying: true,
    };
    file.publish(Some(status)).unwrap();
    let record = std::fs::read_to_string(file.path()).unwrap();
    assert!(
        !observer.observe(Some(&record), secs(0)),
        "must witness an update, not just a stale file"
    );
    assert!(!wrong.observe(Some(&record), secs(0)));
    assert!(!other_pid.observe(Some(&record), secs(0)));
    file.publish(Some(HealthStatus {
        elapsed_ms: 201_000,
        healthy_for_ms: 181_000,
        ..status
    }))
    .unwrap();
    let record = std::fs::read_to_string(file.path()).unwrap();
    assert!(observer.observe(Some(&record), secs(1)));
    assert!(!wrong.observe(Some(&record), secs(1)));
    assert!(!other_pid.observe(Some(&record), secs(1)));
    assert!(observer.observe(Some(&record), secs(3)));
    assert!(
        !observer.observe(Some(&record), secs(4)),
        "same bytes cannot renew health"
    );
    file.publish(None).unwrap();
    assert!(!file.path().exists());
    assert!(!observer.observe(None, secs(5)));
    drop(file);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn independent_supervisor_expires_never_started_media_and_revokes_health() {
    let health = LifecycleHealth::new();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let mut handoff = transfer(0);
    let mut clock = FakeClock::default();
    let mut last = None;
    let result =
        cam::runtime::supervise(&mut handoff, &mut clock, health.clone(), &stop, |status| {
            last = status;
            Ok(())
        });
    assert!(result.is_err());
    assert_eq!(clock.now(), secs(90));
    assert!(health.stopped());
    assert!(
        last.is_none(),
        "failure must revoke health before returning"
    );
    assert!(!handoff.device.events.lock().unwrap().contains(&"disarm"));
}

#[test]
fn production_feed_and_health_output_errors_are_terminal() {
    struct FailSecondFeed {
        feeds: usize,
        dev: Device,
    }
    impl WatchdogDevice for FailSecondFeed {
        fn set_timeout(&mut self, s: u32) -> io::Result<u32> {
            self.dev.set_timeout(s)
        }
        fn keepalive(&mut self) -> io::Result<()> {
            self.feeds += 1;
            if self.feeds == 2 {
                Err(io::Error::other("failed keepalive"))
            } else {
                self.dev.keepalive()
            }
        }
        fn disarm(&mut self) -> io::Result<()> {
            self.dev.disarm()
        }
    }
    let health = LifecycleHealth::new();
    let mut session = WatchdogSession::arm(
        FailSecondFeed {
            feeds: 0,
            dev: Device::default(),
        },
        health.clone(),
        secs(0),
    )
    .unwrap();
    for now in 1..10 {
        session.poll(secs(now)).unwrap();
    }
    assert!(session.poll(secs(10)).is_err());
    assert!(health.stopped());
    assert!(session.poll(secs(11)).is_err());

    let health = LifecycleHealth::new();
    let mut handoff = transfer(0);
    let mut invalidations = 0;
    assert!(
        cam::runtime::supervise(
            &mut handoff,
            &mut FakeClock::default(),
            health.clone(),
            &std::sync::atomic::AtomicBool::new(false),
            |status| {
                if status.is_some() {
                    Err(io::Error::other("health filesystem failed"))
                } else {
                    invalidations += 1;
                    Ok(())
                }
            }
        )
        .is_err()
    );
    assert!(health.stopped());
    assert_eq!(invalidations, 2);
    assert_eq!(*handoff.device.events.lock().unwrap(), ["arm", "feed"]);
}

#[test]
fn stopping_before_a_hung_join_withholds_feeds_without_freeing_hdal() {
    use cam::pipeline::{PipelineConfig, stream_with_audio};
    use cam::runtime::workers::Workers;
    use hdal::{Step, fake::Fake};
    use std::cell::RefCell;
    use std::sync::{atomic::AtomicBool, mpsc};
    let health = LifecycleHealth::new();
    let device = Device::default();
    let mut watchdog = WatchdogSession::arm(device.clone(), health.clone(), secs(0)).unwrap();
    let trace = Arc::new(Mutex::new(Vec::new()));
    let hw_trace = trace.clone();
    let (release, held) = mpsc::channel();
    let (stopping, requested) = mpsc::channel();
    let runtime = health.clone();
    let runner = std::thread::spawn(move || {
        let mut hw = Fake::failing(Step::VideoencRecv, -7);
        hw.trace = Some(hw_trace);
        let stop = AtomicBool::new(false);
        let workers = RefCell::new(Workers::new(&stop));
        let mut held = Some(held);
        stream_with_audio(
            &mut hw,
            &PipelineConfig::e1_zoom(),
            &mut || {
                let held = held.take().unwrap();
                let lease = runtime.worker();
                workers
                    .borrow_mut()
                    .spawn("mqtt-hardware", move || {
                        let _lease = lease;
                        held.recv_timeout(secs(5)).unwrap();
                        Ok(())
                    })
                    .unwrap();
            },
            &mut || {
                runtime.stop();
                stopping.send(()).unwrap();
                workers.borrow_mut().shutdown().unwrap();
            },
            &mut || true,
            &mut |_| {},
            &mut |_| {},
            &mut || None,
            &mut cam::pipeline::Untuned,
        )
    });
    requested.recv_timeout(secs(5)).unwrap();
    let result = watchdog.poll(secs(1));
    let held_graph = trace.lock().unwrap().clone();
    // Release/join before asserting, including failure cases.
    release.send(()).unwrap();
    runner.join().unwrap().unwrap_err();
    assert!(result.is_err());
    assert!(!held_graph.contains(&"common_uninit"));
    assert!(!held_graph.contains(&"audio_cap_stop"));
    assert_eq!(*device.events.lock().unwrap(), ["arm", "feed"]);
    assert!(trace.lock().unwrap().contains(&"common_uninit"));
}

#[test]
fn signal_stop_is_seen_without_waiting_for_the_media_owner() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let stop = AtomicBool::new(false);
    let health = LifecycleHealth::new();
    let mut handoff = transfer(0);
    let mut clock = FakeClock::default();
    assert!(
        cam::runtime::supervise(&mut handoff, &mut clock, health.clone(), &stop, |status| {
            if status.is_some() {
                stop.store(true, Ordering::Release);
            }
            Ok(())
        })
        .is_err()
    );
    assert_eq!(clock.now(), secs(1));
    assert!(health.stopped());
    assert_eq!(*handoff.device.events.lock().unwrap(), ["arm", "feed"]);
}

#[test]
fn incomplete_progress_during_grace_restarts_the_sustained_health_window() {
    let health = LifecycleHealth::new();
    let mut watchdog = WatchdogSession::arm(Device::default(), health.clone(), secs(0)).unwrap();
    for now in 0..=200 {
        // A startup pause at 20..30s is tolerated for feeding, but breaks continuous health.
        if !(20..30).contains(&now) {
            beat_all(&health, now);
        }
        let report = watchdog.poll(secs(now)).unwrap();
        assert!(!report.qualifying);
    }
    for now in 201..=210 {
        beat_all(&health, now);
        let report = watchdog.poll(secs(now)).unwrap();
        assert_eq!(report.qualifying, now == 210);
    }
}

#[test]
fn malformed_regressed_and_nonqualifying_records_revoke_launcher_health() {
    for bad in [
        None,
        Some("{}"),
        Some("not json"),
        Some(
            r#"{"version":1,"attempt":"a","pid":1,"status":{"elapsed_ms":1,"healthy_for_ms":180000,"qualifying":true}}"#,
        ),
        Some(
            r#"{"version":1,"attempt":"a","pid":1,"status":{"elapsed_ms":200000,"healthy_for_ms":180000,"qualifying":false}}"#,
        ),
    ] {
        let mut observer = HealthObserver::new("a", 1);
        let first = r#"{"version":1,"attempt":"a","pid":1,"status":{"elapsed_ms":200000,"healthy_for_ms":180000,"qualifying":true}}"#;
        let next = r#"{"version":1,"attempt":"a","pid":1,"status":{"elapsed_ms":201000,"healthy_for_ms":181000,"qualifying":true}}"#;
        assert!(!observer.observe(Some(first), secs(0)));
        assert!(observer.observe(Some(next), secs(1)));
        assert!(!observer.observe(bad, secs(2)));
    }
}

#[test]
fn signal_during_s00_wait_cancels_before_open_or_initial_feed() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct SignallingClock<'a> {
        time: FakeClock,
        stop: &'a AtomicBool,
    }
    impl Clock for SignallingClock<'_> {
        fn now(&self) -> Duration {
            self.time.now()
        }
        fn wait(&mut self, duration: Duration) {
            self.time.wait(duration);
            self.stop.store(true, Ordering::Release);
        }
    }
    let stop = AtomicBool::new(false);
    let mut clock = SignallingClock {
        time: FakeClock::default(),
        stop: &stop,
    };
    let mut handoff = transfer(1);
    assert!(
        cam::runtime::supervise(
            &mut handoff,
            &mut clock,
            LifecycleHealth::new(),
            &stop,
            |_| Ok(())
        )
        .is_err()
    );
    assert!(
        handoff.device.events.lock().unwrap().is_empty(),
        "cancellation during handoff must not arm/feed later"
    );
}

#[test]
fn driver_adjusted_timeout_cannot_silently_change_the_recovery_policy() {
    for accepted in [0, 30, 120, 300] {
        let device = Device {
            accepted_timeout: Some(accepted),
            ..Device::default()
        };
        let health = LifecycleHealth::new();
        assert!(WatchdogSession::arm(device.clone(), health.clone(), secs(0)).is_err());
        assert!(health.stopped());
        assert_eq!(*device.events.lock().unwrap(), ["arm"]);
    }
}

#[test]
fn a_stalled_publisher_cannot_feed_and_a_late_return_is_terminal() {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    };
    let health = LifecycleHealth::new();
    let device = Device::default();
    let events = Arc::clone(&device.events);
    let (published, seen) = mpsc::channel();
    let (resume, blocked) = mpsc::channel();
    let time = Arc::new(std::sync::atomic::AtomicU64::new(0));
    struct AdvancingClock(Arc<std::sync::atomic::AtomicU64>);
    impl Clock for AdvancingClock {
        fn now(&self) -> Duration {
            secs(self.0.load(Ordering::Acquire))
        }
        fn wait(&mut self, _: Duration) {}
    }
    let clock_time = Arc::clone(&time);
    let observed = health.clone();
    let runner = std::thread::spawn(move || {
        let mut handoff = transfer(0);
        handoff.device = device;
        cam::runtime::supervise(
            &mut handoff,
            &mut AdvancingClock(clock_time),
            health,
            &AtomicBool::new(false),
            |status| {
                if status.is_some() {
                    published.send(()).unwrap();
                    blocked.recv_timeout(secs(5)).unwrap();
                }
                Ok(())
            },
        )
    });
    seen.recv_timeout(secs(5)).unwrap();
    time.store(61, Ordering::Release);
    let while_blocked = events.lock().unwrap().clone();
    resume.send(()).unwrap();
    assert!(runner.join().unwrap().is_err());
    assert_eq!(while_blocked, ["arm", "feed"]);
    assert_eq!(*events.lock().unwrap(), ["arm", "feed"]);
    assert!(observed.stopped());
}

#[test]
fn failed_health_invalidation_is_reported_even_on_orderly_cancellation() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let stop = AtomicBool::new(false);
    let mut published = false;
    let result = cam::runtime::supervise(
        &mut transfer(0),
        &mut FakeClock::default(),
        LifecycleHealth::new(),
        &stop,
        |status| {
            if status.is_some() {
                published = true;
                stop.store(true, Ordering::Release);
            } else if published {
                return Err(io::Error::other("cannot remove health record"));
            }
            Ok(())
        },
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("cannot remove health record")
    );
}
