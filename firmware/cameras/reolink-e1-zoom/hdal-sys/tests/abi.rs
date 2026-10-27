//! Header-size drift checks against historical claims, NOT binary ABI verification.
//!
//! The legacy table was selected using the expected header sizes. Passing these tests
//! cannot establish copied bytes, ids, field meanings, directions or runtime coverage.
//! Keep these snapshot guards until every row has an independent binary reading (docs/hdal-abi.md).
//! Maintained declaration checks live in contracts.rs and the frozen ARM assertions.

use hdal_sys::header_sizes::HEADER_SIZES;

const TABLE: &str = include_str!("../abi/libhdal-2.00-param-sizes.tsv");

fn header_size(name: &str) -> Option<usize> {
    HEADER_SIZES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, s)| *s)
}

fn check_unit(unit: &str) {
    let mut rows = 0;
    let mut wrong = Vec::new();
    for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 8, "bad row: {line}");
        if f[0] != unit {
            continue;
        }
        rows += 1;
        let so: usize = f[5].parse().expect("bytes column");
        match header_size(f[4]) {
            Some(h) if h == so => {}
            Some(h) => wrong.push(format!(
                "{unit} {} id {} {}: historical claim is {so} bytes ({}), header struct {} is {h}",
                f[1], f[2], f[3], f[7], f[4]
            )),
            None => wrong.push(format!("{unit} {}: no header struct {}", f[3], f[4])),
        }
    }
    assert!(rows > 0, "no rows for {unit}");
    assert!(wrong.is_empty(), "ABI mismatches:\n{}", wrong.join("\n"));
}

#[test]
fn videocap_header_sizes_match_historical_claims() {
    check_unit("videocap");
}

#[test]
fn videoproc_header_sizes_match_historical_claims() {
    check_unit("videoproc");
}

#[test]
fn videoenc_header_sizes_match_historical_claims() {
    check_unit("videoenc");
}

#[test]
fn audiocap_header_sizes_match_historical_claims() {
    check_unit("audiocap");
}

#[test]
fn audioout_header_sizes_match_historical_claims() {
    check_unit("audioout");
}

#[test]
fn audioenc_header_sizes_match_historical_claims() {
    check_unit("audioenc");
}

#[test]
fn audio_startup_and_tuning_parameters_have_entries() {
    for (unit, ids) in [
        ("audiocap", &[3, 4, 5, 6, 7, 8, 9][..]),
        ("audioenc", &[3, 4, 5][..]),
        ("audioout", &[3, 4, 5, 6, 7][..]),
    ] {
        for id in ids {
            let prefix = format!("{unit}\tset\t{id}\t");
            assert!(
                TABLE.lines().any(|line| line.starts_with(&prefix)),
                "missing {prefix}"
            );
        }
    }
}

/// Every struct we patched says so in HEADERS.md, with its evidence.
#[test]
fn patched_structs_are_documented() {
    let source = include_str!("../HEADERS.md");
    for name in ["HD_VIDEOENC_OUT", "HD_VIDEOENC_OUT2"] {
        assert!(
            source.contains(name),
            "{name} is patched but not in HEADERS.md"
        );
    }
}

/// Historical heuristic rows must not masquerade as verified binary payload sizes.
#[test]
fn historical_rows_carry_their_evidence_limit() {
    for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 8, "bad row: {line}");
        assert!(
            matches!(
                fields[6],
                "legacy-ok" | "legacy-word" | "legacy-pin" | "retracted-word"
            ),
            "unqualified historical evidence: {line}"
        );
    }
}

/// On the camera the Rust structs themselves must have the clang sizes.
#[cfg(target_arch = "arm")]
#[test]
fn rust_structs_have_header_sizes() {
    use core::mem::size_of;
    assert_eq!(
        size_of::<hdal_sys::HD_VIDEOCAP_IN>(),
        header_size("HD_VIDEOCAP_IN").unwrap()
    );
    assert_eq!(
        size_of::<hdal_sys::HD_VIDEOENC_OUT>(),
        header_size("HD_VIDEOENC_OUT").unwrap()
    );
    assert_eq!(
        size_of::<hdal_sys::HD_AUDIOOUT_OUT>(),
        header_size("HD_AUDIOOUT_OUT").unwrap()
    );
}
