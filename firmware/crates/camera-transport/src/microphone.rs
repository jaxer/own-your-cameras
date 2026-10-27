//! Hardware-neutral microphone formats. PCMU uses RFC3551 static PT0/8000 Hz;
//! AAC remains available for the canonical Reolink transport's regression suite.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
    #[default]
    Aac16k,
    Pcmu8k,
}

pub fn sdp(format: Format) -> String {
    match format {
        Format::Aac16k => "m=audio 0 RTP/AVP 97\r\na=rtpmap:97 MPEG4-GENERIC/16000/1\r\n\
            a=fmtp:97 streamtype=5;profile-level-id=1;mode=AAC-hbr;config=1408;sizeLength=13;indexLength=3;indexDeltaLength=3\r\n\
            a=control:trackID=1\r\na=recvonly\r\n".into(),
        Format::Pcmu8k => "m=audio 0 RTP/AVP 0\r\na=rtpmap:0 PCMU/8000/1\r\n\
            a=control:trackID=1\r\na=recvonly\r\n".into(),
    }
}

pub struct Packetizer {
    aac: crate::rtsp::audio::AacPacketizer,
    format: Format,
    ssrc: u32,
    sequence: u16,
    first: bool,
}

impl Packetizer {
    pub fn new(ssrc: u32, format: Format) -> Self {
        Self {
            aac: crate::rtsp::audio::AacPacketizer::new(ssrc),
            format,
            ssrc,
            sequence: 0,
            first: true,
        }
    }

    pub fn packets(&mut self, pts_us: u64, bytes: &[u8]) -> Option<Vec<Vec<u8>>> {
        if self.format == Format::Aac16k {
            return self.aac.packets(pts_us, bytes);
        }
        if bytes.is_empty() || bytes.len() > 8191 {
            return None;
        }
        let timestamp = (pts_us.wrapping_mul(8000) / 1_000_000) as u32;
        // RFC3551's20ms interval at8kHz. The sender paces these packets;
        // SDK40ms blocks must not become a pair sent in one burst.
        const SAMPLES: usize = 160;
        let mut packets = Vec::with_capacity(bytes.len().div_ceil(SAMPLES));
        for (index, payload) in bytes.chunks(SAMPLES).enumerate() {
            let mut packet = Vec::with_capacity(12 + payload.len());
            packet.extend_from_slice(&[0x80, if self.first { 0x80 } else { 0 }]);
            packet.extend_from_slice(&self.sequence.to_be_bytes());
            packet.extend_from_slice(
                &timestamp
                    .wrapping_add((index * SAMPLES) as u32)
                    .to_be_bytes(),
            );
            packet.extend_from_slice(&self.ssrc.to_be_bytes());
            packet.extend_from_slice(payload);
            packets.push(packet);
            self.sequence = self.sequence.wrapping_add(1);
            self.first = false;
        }
        Some(packets)
    }
}

/// Pure pacing policy. The caller supplies monotonic microseconds and waits
/// outside this module. Small scheduling jitter preserves the original cadence;
/// deadlines more than40ms late reset instead of accumulating catch-up work.
#[derive(Default)]
pub struct Pacer {
    next: Option<u64>,
}
impl Pacer {
    pub fn deadline(&mut self, now_us: u64, samples: usize) -> Option<u64> {
        if !(1..=160).contains(&samples) {
            return None;
        }
        let due = match self.next {
            Some(next) if now_us.saturating_sub(next) <= 40_000 => next,
            _ => now_us,
        };
        self.next = Some(due.checked_add((samples as u64).checked_mul(125)?)?);
        Some(due.max(now_us))
    }
}

/// Convert a bounded, whole even-sized 16 kHz mono block to PCMU at 8 kHz.
/// Pair averaging provides the explicit 2:1 rate conversion; no hardware is used.
pub fn encode_pcm16k_mono(samples: &[i16]) -> Option<Vec<u8>> {
    if samples.is_empty() || samples.len() > 3200 || !samples.len().is_multiple_of(2) {
        return None;
    }
    Some(
        samples
            .chunks_exact(2)
            .map(|pair| {
                let average = (i32::from(pair[0]) + i32::from(pair[1])) / 2;
                linear_to_mulaw(average as i16)
            })
            .collect(),
    )
}

fn linear_to_mulaw(sample: i16) -> u8 {
    let sign = if sample < 0 { 0x80 } else { 0 };
    let biased = i32::from(sample).abs().min(32635) + 132;
    let mut exponent = 7;
    let mut mask = 0x4000;
    while exponent > 0 && biased & mask == 0 {
        exponent -= 1;
        mask >>= 1;
    }
    let mantissa = (biased >> (exponent + 3)) & 15;
    !(sign | ((exponent << 4) | mantissa) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_mulaw_extrema_and_zero_match_the_existing_decoder() {
        assert_eq!(linear_to_mulaw(0), 0xff);
        assert_eq!(linear_to_mulaw(32124), 0x80);
        assert_eq!(linear_to_mulaw(-32124), 0x00);
        assert_eq!(linear_to_mulaw(i16::MAX), 0x80);
        assert_eq!(linear_to_mulaw(i16::MIN), 0x00);
        for code in 0_u8..=255 {
            let mut packet = vec![0x80, 0, 0, 1, 0, 0, 0, 0, 1, 2, 3, 4, code];
            let decoded = crate::rtsp::audio::Pcmu::default().decode(&packet).unwrap()[0];
            packet[12] = linear_to_mulaw(decoded);
            let roundtrip = crate::rtsp::audio::Pcmu::default().decode(&packet).unwrap()[0];
            assert_eq!(roundtrip, decoded);
        }
    }

    #[test]
    fn pcmu_rtp_has_eight_khz_clock_static_payload_and_bounded_fragments() {
        let mut packetizer = Packetizer::new(0x01020304, Format::Pcmu8k);
        let packets = packetizer.packets(1_000_000, &[0xff, 0x80]).unwrap();
        assert_eq!(
            packets,
            [vec![
                0x80, 0x80, 0, 0, 0, 0, 0x1f, 0x40, 1, 2, 3, 4, 0xff, 0x80
            ]]
        );
        let fragments = packetizer.packets(1_040_000, &vec![0xff; 3000]).unwrap();
        assert_eq!(fragments.len(), 19);
        for (i, packet) in fragments.iter().enumerate() {
            assert!(packet.len() <= 172);
            assert_eq!(packet[1], 0);
            assert_eq!(
                u32::from_be_bytes(packet[4..8].try_into().unwrap()),
                8320 + i as u32 * 160
            );
        }
    }

    #[test]
    fn paced_samples_preserve_twenty_ms_and_late_senders_never_catch_up() {
        let mut pacer = Pacer::default();
        assert_eq!(pacer.deadline(100_000, 160), Some(100_000));
        assert_eq!(pacer.deadline(100_000, 160), Some(120_000));
        assert_eq!(pacer.deadline(140_000, 160), Some(140_000));
        assert_eq!(pacer.deadline(140_000, 160), Some(160_000));
        assert_eq!(pacer.deadline(300_000, 160), Some(300_000));
        assert_eq!(pacer.deadline(300_000, 80), Some(320_000));
        assert_eq!(pacer.deadline(300_000, 160), Some(330_000));
        assert_eq!(pacer.deadline(300_000, 0), None);
        assert_eq!(pacer.deadline(300_000, 161), None);
        let mut overflow = Pacer::default();
        assert_eq!(overflow.deadline(u64::MAX - 1, 160), None);
        let mut jitter = Pacer::default();
        assert_eq!(jitter.deadline(100_000, 160), Some(100_000));
        assert_eq!(jitter.deadline(121_000, 160), Some(121_000));
        assert_eq!(jitter.deadline(121_000, 160), Some(140_000));
    }

    #[test]
    fn microphone_format_and_rate_conversion_are_explicit() {
        assert!(sdp(Format::Pcmu8k).contains("a=rtpmap:0 PCMU/8000/1"));
        assert!(!sdp(Format::Pcmu8k).contains("MPEG4"));
        assert_eq!(
            encode_pcm16k_mono(&[0, 0, 32124, 32124, -32124, -32124]),
            Some(vec![0xff, 0x80, 0x00])
        );
        assert!(encode_pcm16k_mono(&[]).is_none());
        assert!(encode_pcm16k_mono(&[0]).is_none());
        assert!(encode_pcm16k_mono(&[0; 3202]).is_none());
    }
}
