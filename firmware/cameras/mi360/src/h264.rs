//! Bounded SPS metadata reader. Annex B boundaries have one owner in the
//! qualified transport crate. SPS syntax follows ITU-T H.264 (08/2024),
//! sections 7.3.2.1.1, 7.3.2.1.1.1 and 7.4.2.1.1, including crop units.
//! https://www.itu.int/rec/dologin_pub.asp?id=T-REC-H.264-202408-I!!PDF-E&lang=e&type=items
use std::io;

const MAX_SPS: usize = 4096;
const MAX_DIMENSION: u32 = 8192;

type Result<T> = std::result::Result<T, &'static str>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VideoMetadata {
    pub width: u32,
    pub height: u32,
    pub profile_idc: u32,
}

struct Bits<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Bits<'_> {
    fn u(&mut self, count: usize) -> Result<u32> {
        if count > 32 || count > self.bytes.len() * 8 - self.at {
            return Err("truncated SPS");
        }
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u32::from((self.bytes[self.at / 8] >> (7 - self.at % 8)) & 1);
            self.at += 1;
        }
        Ok(value)
    }

    fn flag(&mut self) -> Result<bool> {
        Ok(self.u(1)? != 0)
    }

    fn ue(&mut self, max: u32) -> Result<u32> {
        let mut zeros = 0;
        while !self.flag()? {
            zeros += 1;
            if zeros > 31 {
                return Err("oversized SPS Exp-Golomb value");
            }
        }
        let value = ((1_u32 << zeros) - 1) + self.u(zeros)?;
        if value > max {
            return Err("SPS value out of range");
        }
        Ok(value)
    }

    fn se(&mut self) -> Result<i32> {
        let value = self.ue(u32::MAX)?;
        Ok(if value & 1 != 0 {
            (value / 2 + 1) as i32
        } else {
            -((value / 2) as i32)
        })
    }

    fn trailing(&mut self) -> Result<()> {
        if !self.flag()? {
            return Err("missing SPS RBSP stop bit");
        }
        while self.at < self.bytes.len() * 8 {
            if self.flag()? {
                return Err("invalid SPS RBSP padding");
            }
        }
        Ok(())
    }
}

fn rbsp(nal: &[u8]) -> Result<Vec<u8>> {
    if nal.len() < 2 || nal.len() > MAX_SPS || nal[0] & 0x9f != 7 || nal[0] & 0x60 == 0 {
        return Err("invalid or oversized SPS NAL");
    }
    // Annex B permits trailing_zero_8bits after the NAL's RBSP stop bit.
    let end = nal.iter().rposition(|byte| *byte != 0).unwrap_or(0) + 1;
    let mut out = Vec::with_capacity(end - 1);
    let mut zeros = 0;
    for (index, &byte) in nal[1..end].iter().enumerate() {
        if zeros == 2 {
            if byte == 3 {
                if nal.get(index + 2).is_none_or(|next| *next > 3) || index + 2 >= end {
                    return Err("invalid SPS emulation prevention");
                }
                zeros = 0;
                continue;
            }
            if byte < 3 {
                return Err("unescaped SPS start-code byte");
            }
        }
        out.push(byte);
        zeros = if byte == 0 { zeros + 1 } else { 0 };
    }
    Ok(out)
}

fn scaling_list(bits: &mut Bits<'_>, length: usize) -> Result<()> {
    let (mut last, mut next) = (8_i32, 8_i32);
    for _ in 0..length {
        if next != 0 {
            let delta = bits.se()?;
            if !(-128..=127).contains(&delta) {
                return Err("SPS scaling delta out of range");
            }
            next = (last + delta).rem_euclid(256);
        }
        if next != 0 {
            last = next;
        }
    }
    Ok(())
}

fn hrd(bits: &mut Bits<'_>) -> Result<()> {
    let count = bits.ue(31)? + 1;
    bits.u(8)?; // bit_rate_scale, cpb_size_scale
    for _ in 0..count {
        bits.ue(u32::MAX)?;
        bits.ue(u32::MAX)?;
        bits.flag()?;
    }
    bits.u(20)?; // Four five-bit delay/time-offset lengths.
    Ok(())
}

fn vui(bits: &mut Bits<'_>) -> Result<()> {
    if bits.flag()? && bits.u(8)? == 255 && (bits.u(16)? == 0 || bits.u(16)? == 0) {
        return Err("invalid SPS sample aspect ratio");
    }
    if bits.flag()? {
        bits.flag()?; // overscan_appropriate_flag
    }
    if bits.flag()? {
        bits.u(4)?; // video_format and video_full_range_flag
        if bits.flag()? {
            bits.u(24)?; // colour_primaries, transfer_characteristics, matrix_coefficients
        }
    }
    if bits.flag()? {
        bits.ue(5)?;
        bits.ue(5)?;
    }
    if bits.flag()? {
        if bits.u(32)? == 0 || bits.u(32)? == 0 {
            return Err("invalid SPS timing information");
        }
        bits.flag()?;
    }
    let nal_hrd = bits.flag()?;
    if nal_hrd {
        hrd(bits)?;
    }
    let vcl_hrd = bits.flag()?;
    if vcl_hrd {
        hrd(bits)?;
    }
    if nal_hrd || vcl_hrd {
        bits.flag()?;
    }
    bits.flag()?; // pic_struct_present_flag
    if bits.flag()? {
        bits.flag()?; // motion_vectors_over_pic_boundaries_flag
        for _ in 0..4 {
            bits.ue(16)?;
        }
        bits.ue(16)?;
        bits.ue(16)?;
    }
    Ok(())
}

/// Parse one SPS NAL, including its one-byte header but excluding its start code.
fn sps(nal: &[u8]) -> Result<VideoMetadata> {
    let bytes = rbsp(nal)?;
    let mut bits = Bits {
        bytes: &bytes,
        at: 0,
    };
    let profile_idc = bits.u(8)?;
    if bits.u(8)? & 3 != 0 {
        return Err("nonzero SPS reserved constraint bits");
    }
    bits.u(8)?; // level_idc
    bits.ue(31)?; // seq_parameter_set_id
    let mut chroma = 1;
    let mut separate_colour = false;
    match profile_idc {
        66 | 77 | 88 => {}
        100 | 110 | 122 | 244 | 44 | 83 | 86 | 118 | 128 | 138 | 139 | 134 | 135 => {
            chroma = bits.ue(3)?;
            if chroma == 3 {
                separate_colour = bits.flag()?;
            }
            bits.ue(6)?; // bit_depth_luma_minus8
            bits.ue(6)?; // bit_depth_chroma_minus8
            bits.flag()?; // qpprime_y_zero_transform_bypass_flag
            if bits.flag()? {
                for index in 0..if chroma == 3 { 12 } else { 8 } {
                    if bits.flag()? {
                        scaling_list(&mut bits, if index < 6 { 16 } else { 64 })?;
                    }
                }
            }
        }
        _ => return Err("unsupported SPS profile"),
    }
    bits.ue(12)?; // log2_max_frame_num_minus4
    match bits.ue(2)? {
        0 => {
            bits.ue(12)?;
        }
        1 => {
            bits.flag()?;
            bits.se()?;
            bits.se()?;
            for _ in 0..bits.ue(255)? {
                bits.se()?;
            }
        }
        _ => {}
    }
    bits.ue(16)?; // max_num_ref_frames
    bits.flag()?; // gaps_in_frame_num_value_allowed_flag
    let width = (bits.ue(MAX_DIMENSION / 16 - 1)? + 1) * 16;
    let map_height = (bits.ue(MAX_DIMENSION / 16 - 1)? + 1) * 16;
    let frame_only = bits.flag()?;
    let frame_factor = if frame_only { 1 } else { 2 };
    let height = map_height * frame_factor;
    if height > MAX_DIMENSION {
        return Err("SPS coded dimensions exceed budget");
    }
    if !frame_only {
        bits.flag()?; // mb_adaptive_frame_field_flag
    }
    bits.flag()?; // direct_8x8_inference_flag
    let mut crop = [0_u32; 4];
    if bits.flag()? {
        for offset in &mut crop {
            *offset = bits.ue(MAX_DIMENSION)?;
        }
    }
    let chroma_array = if separate_colour { 0 } else { chroma };
    let crop_x = if matches!(chroma_array, 1 | 2) { 2 } else { 1 };
    let crop_y = frame_factor * if chroma_array == 1 { 2 } else { 1 };
    let width = width
        .checked_sub((crop[0] + crop[1]) * crop_x)
        .filter(|n| *n != 0)
        .ok_or("invalid SPS horizontal crop")?;
    let height = height
        .checked_sub((crop[2] + crop[3]) * crop_y)
        .filter(|n| *n != 0)
        .ok_or("invalid SPS vertical crop")?;
    if bits.flag()? {
        vui(&mut bits)?;
    }
    bits.trailing()?;
    Ok(VideoMetadata {
        width,
        height,
        profile_idc,
    })
}

/// Actual SPS metadata in one bounded access unit. Absence is unknown, and
/// conflicting SPS records in the same AU are rejected rather than guessed.
pub fn metadata(data: &[u8]) -> io::Result<Option<VideoMetadata>> {
    let invalid = |message| io::Error::new(io::ErrorKind::InvalidData, message);
    if data.len() > camera_transport::VideoLimits::MI360.max_au_bytes()
        || data
            .windows(3)
            .filter(|bytes| *bytes == [0, 0, 1])
            .take(camera_transport::VideoLimits::MI360.max_nals() + 1)
            .count()
            > camera_transport::VideoLimits::MI360.max_nals()
    {
        return Err(invalid("H.264 access unit exceeds parser budget"));
    }
    let mut found = None;
    for nal in camera_transport::annexb::nal_slices(data) {
        if nal[0] & 31 != 7 {
            continue;
        }
        let value = sps(nal).map_err(invalid)?;
        if found.is_some_and(|previous| previous != value) {
            return Err(invalid("conflicting SPS metadata in access unit"));
        }
        found = Some(value);
    }
    Ok(found)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // Only SPS NAL bytes, no image/slice content. Extracted read-only from
    // Recorded main/sub SPS metadata from the qualified sensor stream; no picture data.
    // Both orientations have identical SPS; source dimensions independently
    // observed by ffprobe as1920x1080/640x360, Baseline(profile_idc66).
    pub const REAL_MAIN: &[u8] = &[
        0x27, 0x42, 0x00, 0x33, 0xe7, 0x40, 0x3c, 0x01, 0x13, 0xf2, 0xcd, 0x40, 0x40, 0x40, 0x7c,
        0x00, 0x00, 0x03, 0x00, 0x04, 0x00, 0x00, 0x03, 0x00, 0xa0, 0xd0, 0x00, 0x08, 0x22, 0x00,
        0x00, 0xc3, 0x30, 0xdf, 0xff, 0xc0, 0xa0,
    ];
    pub const REAL_SUB: &[u8] = &[
        0x27, 0x42, 0x00, 0x33, 0xe7, 0x40, 0x50, 0x17, 0xfc, 0xb3, 0x50, 0x10, 0x10, 0x1f, 0x00,
        0x00, 0x03, 0x00, 0x01, 0x00, 0x00, 0x03, 0x00, 0x28, 0x33, 0x50, 0x04, 0xe2, 0x00, 0x75,
        0x37, 0xff, 0xf0, 0x28,
    ];
    pub fn annexb(nal: &[u8]) -> Vec<u8> {
        [&[0, 0, 0, 1][..], nal].concat()
    }

    #[test]
    fn recorded_sps_metadata_uses_actual_chroma_crop_and_emulation_bytes() {
        for (bytes, width, height) in [(REAL_MAIN, 1920, 1080), (REAL_SUB, 640, 360)] {
            let expected = VideoMetadata {
                width,
                height,
                profile_idc: 66,
            };
            assert_eq!(sps(bytes), Ok(expected));
            assert_eq!(metadata(&annexb(bytes)).unwrap(), Some(expected));
            let repeated = [annexb(bytes), annexb(bytes)].concat();
            assert_eq!(metadata(&repeated).unwrap(), Some(expected));
        }
        assert!(metadata(&[annexb(REAL_MAIN), annexb(REAL_SUB)].concat()).is_err());
        assert_eq!(metadata(&[0, 0, 1, 0x65, 0x80]).unwrap(), None);
    }

    // Generated SPS are isolated syntax fixtures for variants not yet observed
    // on the camera. They are never published to a camera/HA/RTSP service.
    struct Writer(Vec<bool>);
    impl Writer {
        fn u(&mut self, value: u32, count: usize) {
            for shift in (0..count).rev() {
                self.0.push(value & (1 << shift) != 0);
            }
        }
        fn ue(&mut self, value: u32) {
            let value = value + 1;
            let count = (32 - value.leading_zeros()) as usize;
            self.u(0, count - 1);
            self.u(value, count);
        }
        fn finish(mut self) -> Vec<u8> {
            self.u(1, 1);
            while !self.0.len().is_multiple_of(8) {
                self.u(0, 1);
            }
            let bytes = self
                .0
                .chunks(8)
                .map(|chunk| chunk.iter().fold(0_u8, |n, bit| (n << 1) | u8::from(*bit)));
            let mut out = vec![0x67];
            let mut zeros = 0;
            for byte in bytes {
                if zeros == 2 && byte <= 3 {
                    out.push(3);
                    zeros = 0;
                }
                out.push(byte);
                zeros = if byte == 0 { zeros + 1 } else { 0 };
            }
            out
        }
    }
    fn generated(
        profile: u32,
        chroma: u32,
        separate: bool,
        frame_only: bool,
        crop: [u32; 4],
        scaling: bool,
    ) -> Vec<u8> {
        let mut w = Writer(Vec::new());
        w.u(profile, 8);
        w.u(0, 8);
        w.u(31, 8);
        w.ue(0);
        if profile == 100 {
            w.ue(chroma);
            if chroma == 3 {
                w.u(u32::from(separate), 1);
            }
            w.ue(0);
            w.ue(0);
            w.u(0, 1);
            w.u(u32::from(scaling), 1);
            if scaling {
                for index in 0..if chroma == 3 { 12 } else { 8 } {
                    w.u(1, 1);
                    for _ in 0..if index < 6 { 16 } else { 64 } {
                        w.ue(0);
                    }
                }
            }
        }
        w.ue(0); // log2_max_frame_num_minus4
        w.ue(1);
        w.u(0, 1);
        w.ue(0);
        w.ue(0);
        w.ue(2);
        w.ue(0);
        w.ue(0); // POC1
        w.ue(1);
        w.u(0, 1);
        w.ue(39);
        w.ue(22); // coded640x368(progressive)
        w.u(u32::from(frame_only), 1);
        if !frame_only {
            w.u(0, 1);
        }
        w.u(1, 1);
        w.u(1, 1);
        for value in crop {
            w.ue(value);
        }
        w.u(0, 1); // no VUI
        w.finish()
    }

    #[test]
    fn main_high_chroma_planes_interlace_and_scaling_lists() {
        assert_eq!(
            sps(&generated(77, 1, false, true, [0, 0, 0, 4], false)).unwrap(),
            VideoMetadata {
                width: 640,
                height: 360,
                profile_idc: 77
            }
        );
        for (chroma, separate, expected_width, expected_height) in [
            (0, false, 638, 366),
            (1, false, 636, 364),
            (2, false, 636, 366),
            (3, false, 638, 366),
            (3, true, 638, 366),
        ] {
            let got = sps(&generated(100, chroma, separate, true, [1, 1, 1, 1], true)).unwrap();
            assert_eq!(
                (got.width, got.height, got.profile_idc),
                (expected_width, expected_height, 100)
            );
        }
        let interlaced = sps(&generated(100, 1, false, false, [0, 0, 0, 4], false)).unwrap();
        assert_eq!(interlaced.height, 720);
        assert!(sps(&generated(100, 1, false, true, [200, 200, 0, 0], false)).is_err());
    }

    #[test]
    fn malformed_truncated_oversized_and_conflicting_sps_are_bounded() {
        for source in [REAL_MAIN, REAL_SUB] {
            for end in 0..source.len() {
                assert!(sps(&source[..end]).is_err(), "truncation{end}");
            }
            let mut invalid = source.to_vec();
            invalid[0] |= 0x80;
            assert!(sps(&invalid).is_err());
            let mut invalid = source.to_vec();
            invalid[0] &= 0x9f;
            assert!(sps(&invalid).is_err());
        }
        assert!(rbsp(&[0x67, 0, 0, 3]).is_err());
        assert!(rbsp(&[0x67, 0, 0, 3, 4]).is_err());
        assert!(rbsp(&[0x67, 0, 0, 2, 0x80]).is_err());
        assert!(sps(&vec![0x67; MAX_SPS + 1]).is_err());
        assert!(
            metadata(&vec![
                0;
                camera_transport::VideoLimits::MI360.max_au_bytes() + 1
            ])
            .is_err()
        );
        let many = [0, 0, 1, 0x67].repeat(camera_transport::VideoLimits::MI360.max_nals() + 1);
        assert!(metadata(&many).is_err());
        let mut bits = Bits {
            bytes: &[0; 8],
            at: 0,
        };
        assert!(bits.ue(u32::MAX).is_err());
        assert!(metadata(&[0, 0, 1, 0, 0, 1, 0x67]).is_err());
    }
}
