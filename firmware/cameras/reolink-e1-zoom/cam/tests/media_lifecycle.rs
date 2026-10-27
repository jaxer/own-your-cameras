//! Public media lifecycle, observed at the HDAL boundary with real service threads.
use cam::pipeline::{PipelineConfig, stream_with_audio};
use cam::runtime::workers::Workers;
use hdal::{Step, fake::Fake};
use std::cell::RefCell;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

#[test]
fn media_exits_join_hardware_workers_before_any_hdal_teardown() {
    for failure in [
        None,
        Some(Step::CommonMemInit),
        Some(Step::VideocapStart),
        Some(Step::AudioOutStart),
        Some(Step::AudioCapStart),
        Some(Step::VideoencRecv),
        Some(Step::AudioCapRecv),
        Some(Step::AudioOutPush),
    ] {
        let trace = Arc::new(Mutex::new(Vec::new()));
        let mut hw = failure.map_or_else(Fake::default, |step| Fake::failing(step, -7));
        hw.trace = Some(Arc::clone(&trace));
        let stop = Arc::new(AtomicBool::new(false));
        let workers = RefCell::new(Workers::new(&stop));
        let ready = std::cell::Cell::new(false);
        let mut polls = 0;
        let result = stream_with_audio(
            &mut hw,
            &PipelineConfig::e1_zoom(),
            &mut || {
                ready.set(true);
                let stopped = Arc::clone(&stop);
                let trace = Arc::clone(&trace);
                let (tx, rx) = mpsc::channel();
                workers
                    .borrow_mut()
                    .spawn("mqtt-hardware", move || {
                        trace.lock().unwrap().push("hardware_worker_started");
                        tx.send(()).unwrap();
                        let deadline = Instant::now() + Duration::from_secs(2);
                        while !stopped.load(Ordering::Relaxed) {
                            assert!(Instant::now() < deadline, "worker never got shutdown");
                            std::thread::yield_now();
                        }
                        trace.lock().unwrap().push("hardware_worker_stopped");
                        Ok(())
                    })
                    .unwrap();
                rx.recv_timeout(Duration::from_secs(2)).unwrap();
            },
            &mut || {
                workers.borrow_mut().shutdown().unwrap();
                trace.lock().unwrap().push("services_joined");
            },
            &mut || {
                if !ready.get() {
                    return true;
                }
                polls += 1;
                polls <= 1
            },
            &mut |_| {},
            &mut |_| {},
            &mut || Some(vec![1; 320]),
            &mut cam::pipeline::Untuned,
        );
        assert_eq!(result.err().map(|e| e.primary.step), failure);
        let trace = trace.lock().unwrap();
        let at = |name| trace.iter().position(|event| *event == name).unwrap();
        let joined = at("services_joined");
        for teardown in [
            "audio_cap_stop",
            "audio_out_stop",
            "videocap_stop",
            "videoenc_close",
            "common_mem_uninit",
            "common_uninit",
        ] {
            if let Some(i) = trace.iter().position(|event| *event == teardown) {
                assert!(joined < i, "{failure:?}: {trace:?}");
            }
        }
        let started = !matches!(
            failure,
            Some(
                Step::CommonMemInit
                    | Step::VideocapStart
                    | Step::AudioOutStart
                    | Step::AudioCapStart
            )
        );
        assert_eq!(trace.contains(&"hardware_worker_started"), started);
        if started {
            assert!(at("audio_cap_start") < at("hardware_worker_started"));
            assert!(at("hardware_worker_stopped") < joined);
        }
    }
}

#[test]
fn a_worker_that_has_not_joined_keeps_hdal_alive() {
    let trace = Arc::new(Mutex::new(Vec::new()));
    let hw_trace = Arc::clone(&trace);
    let (release, held) = mpsc::channel();
    let (stopping, requested) = mpsc::channel();
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
                workers
                    .borrow_mut()
                    .spawn("mqtt-hardware", move || {
                        held.recv_timeout(Duration::from_secs(2)).unwrap();
                        Ok(())
                    })
                    .unwrap();
            },
            &mut || {
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
    requested.recv_timeout(Duration::from_secs(2)).unwrap();
    let before_release = trace.lock().unwrap().clone();
    // Always release and join, including when the assertion below fails.
    release.send(()).unwrap();
    assert_eq!(
        runner.join().unwrap().unwrap_err().primary.step,
        Step::VideoencRecv
    );
    assert!(
        !before_release.contains(&"audio_cap_stop"),
        "{before_release:?}"
    );
    assert!(
        !before_release.contains(&"common_uninit"),
        "{before_release:?}"
    );
    assert!(trace.lock().unwrap().contains(&"common_uninit"));
}

#[test]
fn media_failure_reports_each_cleanup_error_after_attempting_all_teardown() {
    let mut hw = Fake::failing(Step::VideoencRecv, -7)
        .also_failing(Step::AudioCapStop, -8)
        .also_failing(Step::AudioOutStop, -9)
        .also_failing(Step::VideocapStop, -10)
        .also_failing(Step::CommonMemUninit, -11);
    let message = stream_with_audio(
        &mut hw,
        &PipelineConfig::e1_zoom(),
        &mut || {},
        &mut || {},
        &mut || true,
        &mut |_| {},
        &mut |_| {},
        &mut || None,
        &mut cam::pipeline::Untuned,
    )
    .unwrap_err()
    .to_string();
    for expected in [
        "videoenc_recv",
        "audio_cap_stop",
        "audio_out_stop",
        "videocap_stop",
        "common_mem_uninit",
    ] {
        assert!(message.contains(expected), "lost {expected}: {message}");
    }
    assert_eq!(hw.calls.last(), Some(&"common_uninit"));
}

#[test]
fn cancellation_at_each_startup_boundary_never_advertises_or_receives_media() {
    for last_call in [
        None,
        Some("common_init"),
        Some("common_mem_init"),
        Some("videocap_init"),
        Some("videoproc_init"),
        Some("videoenc_init"),
        Some("videocap_open"),
        Some("videocap_set_sensor"),
        Some("videocap_open_io"),
        Some("videocap_set_in"),
        Some("videocap_set_out"),
        Some("videoproc_open"),
        Some("videoproc_set_dev_config"),
        Some("videoproc_set_out"),
        Some("videoenc_open"),
        Some("videoenc_configure"),
        Some("bind_cap_proc"),
        Some("bind_proc_enc"),
        Some("videoenc_start"),
        Some("videoproc_start"),
        Some("videocap_start"),
        Some("audio_out_start"),
        Some("audio_cap_start"),
    ] {
        let trace = Arc::new(Mutex::new(Vec::new()));
        let mut hw = Fake::default();
        hw.trace = Some(Arc::clone(&trace));
        let mut advertised = false;
        stream_with_audio(
            &mut hw,
            &PipelineConfig::e1_zoom(),
            &mut || advertised = true,
            &mut || trace.lock().unwrap().push("joined"),
            &mut || last_call.is_some_and(|name| !trace.lock().unwrap().contains(&name)),
            &mut |_| panic!("video after cancellation"),
            &mut |_| panic!("audio after cancellation"),
            &mut || None,
            &mut cam::pipeline::Untuned,
        )
        .unwrap();
        assert!(!advertised, "{last_call:?}");
        let trace = trace.lock().unwrap();
        let joined = trace.iter().position(|name| *name == "joined").unwrap();
        assert_eq!(trace[..joined].last().copied(), last_call, "{trace:?}");
        if last_call.is_some() {
            assert_eq!(trace.last(), Some(&"common_uninit"));
        }
    }
}
