//! Annex B elementary-stream writer: starts at the first key frame, and every key frame carries SPS and PPS.

/// NAL unit types (`nal_unit_type`, low five bits) of every NAL in `data`, in order.
pub fn nal_types(data: &[u8]) -> Vec<u8> {
    nal_ranges(data).into_iter().map(|(_, t, _)| t).collect()
}

/// `(start of the start code, nal type, end)` of each NAL.
pub fn nal_ranges(data: &[u8]) -> Vec<(usize, u8, usize)> {
    let mut starts = Vec::new(); // (start-code offset, payload offset)
    let mut i = 0;
    while i + 3 <= data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            let sc = if i > 0 && data[i - 1] == 0 { i - 1 } else { i };
            starts.push((sc, i + 3));
            i += 3;
        } else {
            i += 1;
        }
    }
    let mut out = Vec::new();
    for (n, &(sc, payload)) in starts.iter().enumerate() {
        if payload >= data.len() {
            continue;
        }
        let end = starts.get(n + 1).map_or(data.len(), |s| s.0);
        if payload >= end {
            continue; // Adjacent start codes contain no NAL header or payload.
        }
        out.push((sc, data[payload] & 0x1f, end));
    }
    out
}

/// Each NAL of `data` without its start code (trailing zero bytes before the next start code trimmed).
pub fn nal_slices(data: &[u8]) -> Vec<&[u8]> {
    nal_ranges(data)
        .into_iter()
        .map(|(sc, _, end)| {
            let skip = if data[sc..].starts_with(&[0, 0, 0, 1]) {
                4
            } else {
                3
            };
            &data[sc + skip..end]
        })
        .collect()
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn adjacent_start_codes_skip_empty_nals_without_invalid_slices() {
        for data in [
            vec![0, 0, 1, 0, 0, 1, 0x65, 9],
            vec![0, 0, 1, 0, 0, 0, 1, 0x65, 9],
            vec![0, 0, 0, 1, 0, 0, 1, 0x65, 9],
        ] {
            assert_eq!(nal_slices(&data), [&[0x65, 9][..]]);
            assert_eq!(nal_types(&data), [5]);
        }
    }
}
