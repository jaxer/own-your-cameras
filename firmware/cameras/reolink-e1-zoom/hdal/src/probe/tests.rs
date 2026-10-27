//! The order and clean-up of the videocap probe, against `fake::Fake`.
use super::*;
use crate::fake::Fake;

fn cfg() -> ProbeConfig {
    ProbeConfig::os05a10()
}

#[test]
fn happy_path_runs_every_step_in_order_and_cleans_up_in_reverse() {
    let mut hw = Fake::default();
    let out = probe_videocap(&mut hw, &cfg()).expect("probe");
    assert_eq!(
        hw.calls,
        [
            "common_init",
            "common_mem_init",
            "videocap_init",
            "videocap_open",
            "videocap_set_sensor",
            "videocap_open_io",
            "videocap_set_in",
            "videocap_set_out",
            "videocap_start",
            "videocap_get_sysinfo",
            "videocap_get_syscaps",
            "videocap_stop",
            "videocap_close",
            "videocap_close",
            "videocap_uninit",
            "common_mem_uninit",
            "common_uninit",
        ]
    );
    assert_eq!(out.info.cur_dim, Dim { w: 2592, h: 1944 });
    assert_eq!(out.info.cur_fps[0], 3000 << 16 | 100);
}

#[test]
fn frame_rate_decodes_numerator_over_denominator() {
    let info = VideoCapSysInfo {
        cur_fps: [0x0BB8_0064, 0, 0, 0, 0, 0, 0, 0],
        ..Default::default()
    };
    assert_eq!(info.fps(0), 30.0);
}

#[test]
fn the_sensor_name_and_pool_come_from_the_config() {
    let mut hw = Fake::default();
    probe_videocap(&mut hw, &cfg()).expect("probe");
    assert_eq!(hw.sensor.as_deref(), Some("nvt_sen_os05a10"));
    let pool = hw.pool.expect("pool");
    assert_eq!((pool.blk_size, pool.blk_cnt), (2592 * 1944 * 3 / 2, 3));
}

#[test]
fn a_failing_step_unwinds_only_what_was_done() {
    for (step, done) in [
        (Step::CommonInit, &[][..]),
        (Step::CommonMemInit, &["common_uninit"][..]),
        (
            Step::VideocapInit,
            &["common_mem_uninit", "common_uninit"][..],
        ),
        (
            Step::VideocapOpen,
            &["videocap_uninit", "common_mem_uninit", "common_uninit"][..],
        ),
        (
            Step::VideocapSetSensor,
            &[
                "videocap_close",
                "videocap_uninit",
                "common_mem_uninit",
                "common_uninit",
            ][..],
        ),
        (
            Step::VideocapOpenIo,
            &[
                "videocap_close",
                "videocap_uninit",
                "common_mem_uninit",
                "common_uninit",
            ][..],
        ),
        (
            Step::VideocapSetIn,
            &[
                "videocap_close",
                "videocap_close",
                "videocap_uninit",
                "common_mem_uninit",
                "common_uninit",
            ][..],
        ),
        (
            Step::VideocapSetOut,
            &[
                "videocap_close",
                "videocap_close",
                "videocap_uninit",
                "common_mem_uninit",
                "common_uninit",
            ][..],
        ),
        (
            Step::VideocapStart,
            &[
                "videocap_close",
                "videocap_close",
                "videocap_uninit",
                "common_mem_uninit",
                "common_uninit",
            ][..],
        ),
        (
            Step::VideocapGetSysinfo,
            &[
                "videocap_stop",
                "videocap_close",
                "videocap_close",
                "videocap_uninit",
                "common_mem_uninit",
                "common_uninit",
            ][..],
        ),
    ] {
        let mut hw = Fake::failing(step, -7);
        let err = probe_videocap(&mut hw, &cfg()).expect_err("must fail");
        assert_eq!(err.primary, Error { step, code: -7 });
        assert!(err.cleanup.is_empty());
        let n = hw.calls.len() - done.len();
        assert_eq!(&hw.calls[n..], done, "unwind after {step:?}");
        assert_eq!(
            hw.calls[n - 1],
            step.name(),
            "failed call is the last forward call"
        );
    }
}

#[test]
fn a_failing_cleanup_does_not_hide_the_result_but_later_cleanup_still_runs() {
    let mut hw = Fake::failing(Step::VideocapClose, -3);
    let out = probe_videocap(&mut hw, &cfg());
    assert!(out.is_ok(), "probe data was already read");
    assert_eq!(hw.calls.last(), Some(&"common_uninit"));
    assert_eq!(
        out.expect("ok").cleanup_errors,
        [Error {
            step: Step::VideocapClose,
            code: -3
        }; 2],
        "both the i/o and the control path close fail"
    );
}

#[test]
fn syscaps_failing_is_reported_but_the_probe_still_has_sysinfo() {
    let mut hw = Fake::failing(Step::VideocapGetSyscaps, -1);
    let out = probe_videocap(&mut hw, &cfg()).expect("probe");
    assert_eq!(
        out.caps,
        Err(Error {
            step: Step::VideocapGetSyscaps,
            code: -1
        })
    );
    assert_eq!(out.info.cur_dim, Dim { w: 2592, h: 1944 });
    assert!(
        out.report()
            .contains("caps_error=hd_videocap_get_syscaps failed: HD_RESULT -1\n")
    );
    assert_eq!(hw.calls.last(), Some(&"common_uninit"));
}

#[test]
fn the_report_has_one_key_value_per_line() {
    let mut hw = Fake::default();
    let out = probe_videocap(&mut hw, &cfg()).expect("probe");
    let r = out.report();
    assert!(r.contains("sensor=nvt_sen_os05a10\n"), "{r}");
    assert!(r.contains("cur_dim=2592x1944\n"), "{r}");
    assert!(r.contains("cur_fps=30.00\n"), "{r}");
    assert!(r.contains("cleanup=ok\n"), "{r}");
}

#[test]
fn sysinfo_is_polled_until_the_output_reports_its_size() {
    let mut hw = Fake::default();
    hw.ready_after_polls = 3;
    let out = probe_videocap(&mut hw, &cfg()).expect("probe");
    let polls = hw
        .calls
        .iter()
        .filter(|c| **c == "videocap_get_sysinfo")
        .count();
    assert_eq!(polls, 3);
    assert_eq!(hw.slept_ms, 200, "two pauses of 100 ms between three polls");
    assert_eq!(out.info.cur_dim, Dim { w: 2592, h: 1944 });
}

#[test]
fn a_sensor_that_never_reports_a_size_yields_the_last_sysinfo() {
    let mut hw = Fake::default();
    hw.ready_after_polls = usize::MAX;
    let out = probe_videocap(&mut hw, &cfg()).expect("probe");
    assert_eq!(out.info.cur_dim, Dim { w: 0, h: 0 });
    assert_eq!(
        hw.slept_ms,
        100 * (hw
            .calls
            .iter()
            .filter(|c| **c == "videocap_get_sysinfo")
            .count() as u32
            - 1)
    );
}

#[test]
fn failed_probe_preserves_cleanup_failures_in_its_error() {
    let mut hw = Fake::failing(Step::VideocapGetSysinfo, -7)
        .also_failing(Step::VideocapStop, -8)
        .also_failing(Step::CommonMemUninit, -9);
    let error = probe_videocap(&mut hw, &cfg()).unwrap_err().to_string();
    for expected in [
        "hd_videocap_get_sysinfo failed: HD_RESULT -7",
        "hd_videocap_stop failed: HD_RESULT -8",
        "hd_common_mem_uninit failed: HD_RESULT -9",
    ] {
        assert!(error.contains(expected), "lost {expected}: {error}");
    }
    assert_eq!(hw.calls.last(), Some(&"common_uninit"));
}
