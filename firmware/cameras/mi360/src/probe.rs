//! Explicit real-device, RAM-only media qualification; no broker or fabricated media.
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::time::{Duration, Instant};

pub fn run(prefix: &str) -> io::Result<()> {
    if !prefix
        .strip_prefix("/tmp/mi360-")
        .is_some_and(|s| !s.is_empty() && !s.contains('/'))
    {
        return Err(io::Error::other(
            "media-probe requires a /tmp/mi360- prefix",
        ));
    }
    let mut files = Vec::new();
    for suffix in ["main.h264", "sub.h264", "microphone.pcm"] {
        files.push(
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(format!("{prefix}-{suffix}"))?,
        );
    }
    let mut media = crate::media::Media::open(true)?;
    let start = Instant::now();
    let mut video = [0_u32; 2];
    let mut audio = 0_u32;
    let result = (|| -> io::Result<()> {
        while start.elapsed() < Duration::from_secs(3) {
            for channel in 0..2 {
                if let Some(frame) = media.next_video(channel)? {
                    files[channel].write_all(&frame.data)?;
                    video[channel] += 1;
                }
            }
            if let Some(frame) = media.next_audio()? {
                let mut pcm = [0_u8; 1280];
                for (i, sample) in frame.samples.iter().enumerate() {
                    pcm[i * 2..i * 2 + 2].copy_from_slice(&sample.to_le_bytes());
                }
                files[2].write_all(&pcm)?;
                audio += 1;
            }
        }
        if video.iter().any(|v| *v < 10) || audio < 10 {
            return Err(io::Error::other(format!(
                "insufficient real progress: video={video:?} audio={audio}"
            )));
        }
        Ok(())
    })();
    let close = media.close();
    eprintln!(
        "real_media video={video:?} audio_blocks={audio} elapsed_ms={}",
        start.elapsed().as_millis()
    );
    result.and(close)
}

fn drain(
    media: &mut crate::media::Media,
    duration: Duration,
    mut output: Option<&mut [std::fs::File; 2]>,
) -> io::Result<[u32; 3]> {
    let started = Instant::now();
    let mut counts = [0; 3];
    while started.elapsed() < duration {
        for channel in 0..2 {
            if let Some(frame) = media.next_video(channel)? {
                counts[channel] += 1;
                if let Some(files) = output.as_mut() {
                    files[channel].write_all(&frame.data)?;
                }
            }
        }
        if media.next_audio()?.is_some() {
            counts[2] += 1;
        }
    }
    Ok(counts)
}
fn wait_control(media: &mut crate::media::Media, id: u32, target: i32) -> io::Result<i32> {
    let start = Instant::now();
    loop {
        let actual = media.control_get(id)?;
        if actual == target {
            return Ok(actual);
        }
        if start.elapsed() > Duration::from_secs(5) {
            return Err(io::Error::other("active control did not settle"));
        }
        drain(media, Duration::from_millis(50), None)?;
    }
}
pub fn controls(prefix: &str) -> io::Result<()> {
    if !prefix
        .strip_prefix("/tmp/mi360-")
        .is_some_and(|s| !s.is_empty() && !s.contains('/'))
    {
        return Err(io::Error::other(
            "controls-probe requires a /tmp/mi360- prefix",
        ));
    }
    let file = |suffix: &str| {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(format!("{prefix}-{suffix}"))
    };
    let mut original = [file("original-main.h264")?, file("original-sub.h264")?];
    let mut upright = [file("upright-main.h264")?, file("upright-sub.h264")?];
    let mut media = crate::media::Media::open(true)?;
    let mut observations = Vec::new();
    let outcome = (|| -> io::Result<()> {
        let orientation_before = media.control_get(0)?;
        let before = drain(&mut media, Duration::from_secs(1), Some(&mut original))?;
        media.control_set(0, 3)?;
        let orientation_after = wait_control(&mut media, 0, 3)?;
        let after = drain(&mut media, Duration::from_millis(3300), Some(&mut upright))?;
        observations.push(serde_json::json!({"control":0,"before":orientation_before,"target":3,"after":orientation_after,"original_frames":before,"upright_frames":after}));
        for id in [
            1, 2, 3, 4, 5, 10, 11, 20, 21, 22, 23, 24, 25, 26, 27, 30, 31, 32, 33, 34, 35, 36, 37,
            38,
        ] {
            let before = media.control_get(id)?;
            let target = match id {
                5 | 36..=38 => {
                    if before == 0 {
                        1
                    } else {
                        0
                    }
                }
                20 | 21 => 10,
                26 | 27 => {
                    if before == 3 {
                        1
                    } else {
                        3
                    }
                }
                35 => {
                    if before == 0 {
                        2
                    } else {
                        0
                    }
                }
                22 | 23 => before / 2,
                24 | 25 => {
                    if before == 1 {
                        2
                    } else {
                        1
                    }
                }
                _ => {
                    if before < 120 {
                        before + 1
                    } else {
                        before - 1
                    }
                }
            };
            let applied = media
                .control_set(id, target)
                .and_then(|_| wait_control(&mut media, id, target));
            let progress = drain(&mut media, Duration::from_millis(1200), None)?;
            let restored = media
                .control_set(id, before)
                .and_then(|_| wait_control(&mut media, id, before));
            observations.push(serde_json::json!({"control":id,"before":before,"target":target,"actual":applied.as_ref().ok(),"apply_error":applied.as_ref().err().map(|e|e.to_string()),"real_progress_1200ms":progress,"restored":restored.as_ref().ok(),"restore_error":restored.as_ref().err().map(|e|e.to_string())}));
            restored?;
            applied?;
            if progress.iter().any(|v| *v < 3) {
                return Err(io::Error::other("real media stalled during control probe"));
            }
        }
        Ok(())
    })();
    let cleanup = media.close();
    let report = serde_json::json!({"real_hardware":true,"mqtt_published":false,"success":outcome.is_ok()&&cleanup.is_ok(),"observations":observations,"probe_error":outcome.as_ref().err().map(|e|e.to_string()),"cleanup_error":cleanup.as_ref().err().map(|e|e.to_string())});
    file("controls.json")?.write_all(&serde_json::to_vec(&report)?)?;
    outcome.and(cleanup)
}

// One graph attempt per process. The real default case closed successfully,
// but a second same-process open failed System_Init/AL_Board_Create with
// uncertain ownership. Successful teardown does not certify SDK reinitialization.
// The SDK/kernel can block inside open/get/close; the external process runner
// must retain its own deadline. These deadlines bound successful-return polling.
const SPEC_STARTUP: Duration = Duration::from_secs(5);
const SPEC_MEASURE: Duration = Duration::from_secs(3);
const SPEC_MAX_GAP: Duration = Duration::from_secs(2);
const SPEC_MAX_CASES: usize = 1;
static SPEC_ATTEMPTED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

struct SpecCase {
    name: String,
    spec: crate::media::RustVideoSpec,
}

fn parse_spec_cases(selection: &str) -> io::Result<Vec<SpecCase>> {
    use crate::media::RustVideoSpec;
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "video-spec-probe requires exactly one case per process: default or MAINxHEIGHT-SUBxHEIGHT-MAINPROFILE-SUBPROFILE; profiles baseline/main/high; batches are unsafe",
        )
    };
    if selection.is_empty()
        || selection.len() > 1024
        || !selection.is_ascii()
        || selection.contains(',')
    {
        return Err(invalid());
    }
    let mut cases: Vec<SpecCase> = Vec::new();
    for name in selection.split(',') {
        if cases.len() >= SPEC_MAX_CASES || name.is_empty() {
            return Err(invalid());
        }
        let mut spec = RustVideoSpec::default();
        match name {
            "default" => {}
            "both-main" => {
                spec.main_profile = 77;
                spec.sub_profile = 77;
            }
            "both-high" => {
                spec.main_profile = 100;
                spec.sub_profile = 100;
            }
            "main720-sub360" => {
                spec.main_width = 1280;
                spec.main_height = 720;
            }
            "main360-sub180" => {
                spec.main_width = 640;
                spec.main_height = 360;
                spec.sub_width = 320;
                spec.sub_height = 180;
            }
            "sub180" => {
                spec.sub_width = 320;
                spec.sub_height = 180;
            }
            "mixed-main-high" => {
                spec.main_profile = 77;
                spec.sub_profile = 100;
            }
            _ => {
                let parts: Vec<_> = name.split('-').collect();
                if parts.len() != 4
                    || !crate::graph_settings::MAIN_RESOLUTIONS.contains(&parts[0])
                    || !crate::graph_settings::SUB_RESOLUTIONS.contains(&parts[1])
                {
                    return Err(invalid());
                }
                let dimensions = |text: &str| -> io::Result<(u32, u32)> {
                    let (width, height) = text.split_once('x').ok_or_else(invalid)?;
                    Ok((
                        width.parse().map_err(|_| invalid())?,
                        height.parse().map_err(|_| invalid())?,
                    ))
                };
                (spec.main_width, spec.main_height) = dimensions(parts[0])?;
                (spec.sub_width, spec.sub_height) = dimensions(parts[1])?;
                let profile = |name: &str| -> io::Result<u32> {
                    let index = crate::graph_settings::PROFILES
                        .iter()
                        .position(|p| *p == name)
                        .ok_or_else(invalid)?;
                    [66, 77, 100].get(index).copied().ok_or_else(invalid)
                };
                spec.main_profile = profile(parts[2])?;
                spec.sub_profile = profile(parts[3])?;
            }
        }
        if cases.iter().any(|case| case.spec == spec) {
            return Err(invalid());
        }
        cases.push(SpecCase {
            name: name.into(),
            spec,
        });
    }
    Ok(cases)
}

fn spec_json(spec: crate::media::RustVideoSpec) -> serde_json::Value {
    serde_json::json!({
        "main_width":spec.main_width,"main_height":spec.main_height,
        "sub_width":spec.sub_width,"sub_height":spec.sub_height,
        "main_profile":spec.main_profile,"sub_profile":spec.sub_profile,
        "main_fps":spec.main_fps,"sub_fps":spec.sub_fps,
        "main_bitrate":spec.main_bitrate,"sub_bitrate":spec.sub_bitrate
    })
}

fn metadata_json(metadata: Option<crate::media::VideoMetadata>) -> serde_json::Value {
    metadata.map_or(serde_json::Value::Null, |m| {
        serde_json::json!({
            "width":m.width,"height":m.height,"profile_idc":m.profile_idc
        })
    })
}

fn check_spec_metadata(
    spec: crate::media::RustVideoSpec,
    actual: [Option<crate::media::VideoMetadata>; 2],
) -> io::Result<bool> {
    let expected = [
        (spec.main_width, spec.main_height, spec.main_profile),
        (spec.sub_width, spec.sub_height, spec.sub_profile),
    ];
    for (channel, metadata) in actual.iter().enumerate() {
        if let Some(m) = metadata
            && (m.width, m.height, m.profile_idc) != expected[channel]
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("channel{channel} real SPS differs from requested spec"),
            ));
        }
    }
    Ok(actual.iter().all(Option::is_some))
}

#[derive(Default)]
struct SpecProgress {
    measurement_started: bool,
    measured_ms: u128,
    video_frames: [u32; 2],
    video_bytes: [u64; 2],
    audio_blocks: u32,
    first_video_us: [Option<u64>; 2],
    last_video_us: [Option<u64>; 2],
    first_audio_us: Option<u64>,
    last_audio_us: Option<u64>,
}

fn sample_spec_media(
    media: &mut crate::media::Media,
    progress: &mut SpecProgress,
    last_progress: &mut [Instant; 3],
) -> io::Result<()> {
    for (channel, last_at) in last_progress[..2].iter_mut().enumerate() {
        if let Some(frame) = media.next_video(channel)? {
            progress.video_frames[channel] += 1;
            progress.video_bytes[channel] += frame.data.len() as u64;
            progress.first_video_us[channel].get_or_insert(frame.timestamp_us);
            progress.last_video_us[channel] = Some(frame.timestamp_us);
            *last_at = Instant::now();
        }
    }
    if let Some(frame) = media.next_audio()? {
        progress.audio_blocks += 1;
        progress.first_audio_us.get_or_insert(frame.timestamp_us);
        progress.last_audio_us = Some(frame.timestamp_us);
        last_progress[2] = Instant::now();
    }
    Ok(())
}

fn measure_spec(
    media: &mut crate::media::Media,
    spec: crate::media::RustVideoSpec,
    progress: &mut SpecProgress,
) -> io::Result<()> {
    let started = Instant::now();
    let mut last_progress = [started; 3];
    loop {
        sample_spec_media(media, progress, &mut last_progress)?;
        let metadata = [media.video_metadata(0), media.video_metadata(1)];
        if check_spec_metadata(spec, metadata)?
            && progress.video_frames.iter().all(|count| *count > 0)
            && progress.audio_blocks > 0
        {
            break;
        }
        if started.elapsed() >= SPEC_STARTUP {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "real video SPS/frames and microphone PCM did not start within5s",
            ));
        }
    }
    let measured = Instant::now();
    *progress = SpecProgress {
        measurement_started: true,
        ..SpecProgress::default()
    };
    last_progress.fill(measured);
    while measured.elapsed() < SPEC_MEASURE {
        sample_spec_media(media, progress, &mut last_progress)?;
        progress.measured_ms = measured.elapsed().as_millis();
        check_spec_metadata(spec, [media.video_metadata(0), media.video_metadata(1)])?;
        if last_progress.iter().any(|at| at.elapsed() > SPEC_MAX_GAP) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "real media stalled for2s",
            ));
        }
    }
    if progress.video_frames.iter().any(|count| *count < 10) || progress.audio_blocks < 10 {
        return Err(io::Error::other(
            "insufficient real frame/PCM progress in3s measurement",
        ));
    }
    Ok(())
}

/// Opens exactly one explicit named case, including PCM audio, and reports
/// observed SPS metadata and real progress. Writes no media/config files and
/// publishes no MQTT. Successful close does not prove same-process SDK reinit;
/// a second attempt is forbidden even after success. Use a fresh process for
/// another case, and recover a poisoned/retained session with a clean boot first.
pub fn video_specs(selection: &str) -> io::Result<()> {
    // Reject every batch before the first loader/hardware call.
    let cases = parse_spec_cases(selection)?;
    if SPEC_ATTEMPTED.swap(1, std::sync::atomic::Ordering::AcqRel) != 0 {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "video-spec-probe already attempted a graph; exit this process before another case",
        ));
    }
    let selected = cases.len();
    let mut reports = Vec::new();
    let mut failure = None;
    for case in cases {
        eprintln!("video-spec-probe opening {}", case.name);
        let started = Instant::now();
        let mut media = match crate::media::Media::open_spec(true, case.spec) {
            Ok(media) => media,
            Err(error) => {
                reports.push(serde_json::json!({"case":case.name,"requested":spec_json(case.spec),
                    "success":false,"open_error":error.to_string(),"cleanup":"owned by Media::open_spec/drop; no returned handle",
                    "elapsed_ms":started.elapsed().as_millis()}));
                failure = Some(error);
                break;
            }
        };
        let mut progress = SpecProgress::default();
        let result = measure_spec(&mut media, case.spec, &mut progress);
        let actual = [media.video_metadata(0), media.video_metadata(1)];
        // Always explicitly close this returned owner, even after frame/SPS failure.
        let cleanup = media.close();
        let success = result.is_ok() && cleanup.is_ok();
        reports.push(serde_json::json!({
            "case":case.name,"requested":spec_json(case.spec),"success":success,
            "actual_sps":[metadata_json(actual[0]),metadata_json(actual[1])],
            "video_frames":progress.video_frames,"video_bytes":progress.video_bytes,
            "audio_blocks":progress.audio_blocks,"audio_samples_per_block":640,
            "first_video_us":progress.first_video_us,"last_video_us":progress.last_video_us,
            "first_audio_us":progress.first_audio_us,"last_audio_us":progress.last_audio_us,
            "measurement_started":progress.measurement_started,"observed_measurement_ms":progress.measured_ms,
            "measurement_target_seconds":SPEC_MEASURE.as_secs(),"elapsed_ms":started.elapsed().as_millis(),
            "media_error":result.as_ref().err().map(ToString::to_string),
            "cleanup_error":cleanup.as_ref().err().map(ToString::to_string)
        }));
        eprintln!(
            "video-spec-probe {} success={success} explicit_close_ok={}",
            case.name,
            cleanup.is_ok()
        );
        if !success {
            failure = cleanup.err().or_else(|| result.err());
            break;
        }
    }
    let passed = reports
        .iter()
        .filter(|report| report["success"] == true)
        .count();
    let report = serde_json::json!({
        "real_hardware":cfg!(all(target_os="linux",target_arch="mips",feature="imp-video")),
        "mqtt_published":false,
        "success":failure.is_none(),"selected_cases":selected,"attempted_cases":reports.len(),"passed_cases":passed,
        "remaining_cases_refused":selected-reports.len(),"cases":reports,
        "sdk_reinitialization":"not qualified; one graph attempt per process; clean boot required after poison",
        "checks":["matching actual channel SPS","at least10 frames per channel and10 PCM blocks in3s","no2s media stall","explicit clean close"],
        "blocking_sdk_deadline":"external process supervisor required for SDK/kernel calls"});
    let mut output = io::stdout().lock();
    serde_json::to_writer(&mut output, &report)?;
    output.write_all(b"\n")?;
    failure.map_or(Ok(()), Err)
}

#[cfg(test)]
mod spec_tests {
    use super::*;

    #[test]
    fn explicit_matrix_contains_only_constructor_geometries_profiles_and20fps() {
        let mut specs = Vec::new();
        for main in crate::graph_settings::MAIN_RESOLUTIONS {
            for sub in crate::graph_settings::SUB_RESOLUTIONS {
                for main_profile in crate::graph_settings::PROFILES {
                    for sub_profile in crate::graph_settings::PROFILES {
                        let name = format!("{main}-{sub}-{main_profile}-{sub_profile}");
                        let cases = parse_spec_cases(&name).unwrap();
                        let spec = cases[0].spec;
                        assert_eq!((spec.main_fps, spec.sub_fps), (20, 20));
                        assert_eq!((spec.main_bitrate, spec.sub_bitrate), (800, 240));
                        assert!(!specs.contains(&spec));
                        specs.push(spec);
                    }
                }
            }
        }
        assert_eq!(specs.len(), 54);
        assert!(specs.contains(&crate::media::RustVideoSpec::default()));
        for shortcut in [
            "default",
            "both-main",
            "both-high",
            "main720-sub360",
            "main360-sub180",
            "sub180",
            "mixed-main-high",
        ] {
            assert!(specs.contains(&parse_spec_cases(shortcut).unwrap()[0].spec));
        }
    }
    #[test]
    fn every_case_is_validated_before_an_sdk_call_and_all_batches_are_refused() {
        for invalid in [
            "",
            "default,",
            "default,unknown",
            "default,default",
            "default,both-main",
            "both-main,both-high",
            "default,1920x1080-640x360-baseline-baseline",
            "1920x720-640x360-main-main",
            "1280x720-320x360-main-main",
            "1920x1080-640x360-hevc-main",
            " default",
            "défault",
        ] {
            assert_eq!(
                video_specs(invalid).unwrap_err().kind(),
                io::ErrorKind::InvalidInput
            );
        }
        let names = [
            "default",
            "both-main",
            "both-high",
            "main720-sub360",
            "main360-sub180",
            "sub180",
            "mixed-main-high",
            "1280x720-320x180-main-high",
            "640x360-640x360-high-main",
        ];
        for name in names {
            assert_eq!(parse_spec_cases(name).unwrap().len(), 1);
        }
        for count in 2..=names.len() {
            assert!(parse_spec_cases(&names[..count].join(",")).is_err());
        }
    }
    #[test]
    fn absent_or_wrong_sps_is_never_replaced_with_requested_values() {
        let spec = crate::media::RustVideoSpec::default();
        assert!(!check_spec_metadata(spec, [None, None]).unwrap());
        let main = crate::media::VideoMetadata {
            width: 1920,
            height: 1080,
            profile_idc: 66,
        };
        let sub = crate::media::VideoMetadata {
            width: 640,
            height: 360,
            profile_idc: 66,
        };
        assert!(!check_spec_metadata(spec, [Some(main), None]).unwrap());
        assert!(check_spec_metadata(spec, [Some(main), Some(sub)]).unwrap());
        assert!(
            check_spec_metadata(
                spec,
                [
                    Some(crate::media::VideoMetadata {
                        profile_idc: 100,
                        ..main
                    }),
                    Some(sub)
                ]
            )
            .is_err()
        );
        assert!(
            check_spec_metadata(
                spec,
                [
                    Some(main),
                    Some(crate::media::VideoMetadata { height: 352, ..sub })
                ]
            )
            .is_err()
        );
    }
}
