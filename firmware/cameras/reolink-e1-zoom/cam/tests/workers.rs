use cam::runtime::workers::Workers;
use std::sync::{atomic::AtomicBool, mpsc};
use std::time::{Duration, Instant};

#[test]
fn unexpected_service_exit_reaches_the_owner() {
    let stop = AtomicBool::new(false);
    let mut workers = Workers::new(&stop);
    let (tx, rx) = mpsc::channel();
    workers
        .spawn("rtsp", move || {
            tx.send(()).unwrap();
            Ok(())
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let error = loop {
        if let Err(error) = workers.check() {
            break error;
        }
        assert!(
            Instant::now() < deadline,
            "owner did not notice worker exit"
        );
        std::thread::yield_now();
    };
    assert!(
        error.to_string().contains("rtsp exited unexpectedly"),
        "{error}"
    );
}

#[test]
fn shutdown_joins_every_worker_even_when_one_failed() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let stop = Arc::new(AtomicBool::new(false));
    let exited = Arc::new(AtomicUsize::new(0));
    let mut workers = Workers::new(&stop);
    workers
        .spawn("mqtt", || Err(std::io::Error::other("ISP open failed")))
        .unwrap();
    let stopped = Arc::clone(&stop);
    let count = Arc::clone(&exited);
    workers
        .spawn("rtsp", move || {
            let deadline = Instant::now() + Duration::from_secs(2);
            while !stopped.load(Ordering::Relaxed) {
                assert!(Instant::now() < deadline, "worker never got shutdown");
                std::thread::yield_now();
            }
            count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
        .unwrap();
    let error = workers.shutdown().unwrap_err();
    assert!(
        error.to_string().contains("mqtt: ISP open failed"),
        "{error}"
    );
    assert_eq!(exited.load(Ordering::Relaxed), 1);
    workers.shutdown().unwrap();
}

#[test]
fn dropping_partial_startup_stops_and_joins_existing_workers() {
    use std::sync::{Arc, atomic::Ordering};
    let stop = Arc::new(AtomicBool::new(false));
    let exited = Arc::new(AtomicBool::new(false));
    {
        let mut workers = Workers::new(&stop);
        let stopped = Arc::clone(&stop);
        let exited = Arc::clone(&exited);
        workers
            .spawn("mqtt", move || {
                let deadline = Instant::now() + Duration::from_secs(2);
                while !stopped.load(Ordering::Relaxed) {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                exited.store(true, Ordering::Relaxed);
                Ok(())
            })
            .unwrap();
        // e.g. return early because the next worker could not be started.
    }
    assert!(exited.load(Ordering::Relaxed), "worker outlived its owner");
}

#[test]
fn worker_errors_and_panics_are_not_silently_discarded() {
    for panic in [false, true] {
        let stop = AtomicBool::new(false);
        let mut workers = Workers::new(&stop);
        workers
            .spawn("mqtt", move || {
                assert!(!panic, "injected worker panic");
                Err(std::io::Error::other("ISP open failed"))
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let error = loop {
            if let Err(error) = workers.check() {
                break error;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        let expected = if panic {
            "mqtt panicked"
        } else {
            "mqtt: ISP open failed"
        };
        assert!(error.to_string().contains(expected), "{error}");
    }
}

#[test]
fn shutdown_reports_all_worker_failures_after_joining_every_worker() {
    let stop = AtomicBool::new(false);
    let mut workers = Workers::new(&stop);
    workers
        .spawn("network", || Err(std::io::Error::other("network fault")))
        .unwrap();
    workers
        .spawn("mqtt", || Err(std::io::Error::other("hardware fault")))
        .unwrap();
    workers.spawn("rtsp", || panic!("listener fault")).unwrap();
    let message = workers.shutdown().unwrap_err().to_string();
    for expected in [
        "network: network fault",
        "mqtt: hardware fault",
        "rtsp panicked",
    ] {
        assert!(message.contains(expected), "lost {expected}: {message}");
    }
    workers.shutdown().unwrap();
}

#[test]
fn observing_an_unexpected_exit_stops_peer_workers_before_returning() {
    use std::sync::atomic::Ordering;
    let stop = AtomicBool::new(false);
    let mut workers = Workers::new(&stop);
    workers.spawn("mqtt", || Ok(())).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while workers.check().is_ok() {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(
        stop.load(Ordering::Acquire),
        "peer workers never received stop on worker exit"
    );
}

#[test]
fn an_exit_is_named_before_the_held_guard_reports_it() {
    // The guard stands in for a `WorkerLease`: its drop is how the owner first learns a worker
    // ended, so by then `check` must already name that worker, even though its thread still runs.
    struct Guard(mpsc::Sender<()>, mpsc::Receiver<()>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.send(()).unwrap();
            self.1.recv().unwrap();
        }
    }
    let stop = AtomicBool::new(false);
    let mut workers = Workers::new(&stop);
    let (dropping, dropped) = mpsc::channel();
    let (release, released) = mpsc::channel();
    workers
        .spawn_holding("network", Guard(dropping, released), || Ok(()))
        .unwrap();
    dropped.recv_timeout(Duration::from_secs(2)).unwrap();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        release.send(()).unwrap();
    });
    let error = workers.check().unwrap_err();
    assert!(
        error.to_string().contains("network exited unexpectedly"),
        "{error}"
    );
    releaser.join().unwrap();
}
