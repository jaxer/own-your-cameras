//! AAC RTP and PCMU talkback at the RTSP boundary.

use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex, Weak};

pub struct AudioFrame {
    pub pts_us: u64,
    pub data: Vec<u8>,
}

/// One microphone capture shared by both RTSP paths; at most 16 AAC frames queued per viewer.
#[derive(Default)]
pub struct AudioHub {
    format: crate::microphone::Format,
    subs: Mutex<Vec<AudioSub>>,
}

struct AudioSub {
    tx: SyncSender<Arc<AudioFrame>>,
    viewer: Weak<()>,
}

pub struct AudioViewer {
    pub format: crate::microphone::Format,
    rx: Receiver<Arc<AudioFrame>>,
    _alive: Arc<()>,
}

impl AudioViewer {
    pub fn next(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Arc<AudioFrame>, std::sync::mpsc::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }
}

impl AudioHub {
    pub fn with_format(format: crate::microphone::Format) -> Arc<Self> {
        Arc::new(Self {
            format,
            ..Self::default()
        })
    }
    pub fn format(&self) -> crate::microphone::Format {
        self.format
    }
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    /// Current microphone viewers, including ones waiting for their first frame.
    pub fn viewers(&self) -> usize {
        self.subs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .filter(|s| s.viewer.strong_count() != 0)
            .count()
    }
    pub fn subscribe(&self) -> AudioViewer {
        let (tx, rx) = sync_channel(16);
        let alive = Arc::new(());
        let mut subs = self.subs.lock().unwrap_or_else(|e| e.into_inner());
        subs.retain(|s| s.viewer.strong_count() != 0);
        subs.push(AudioSub {
            tx,
            viewer: Arc::downgrade(&alive),
        });
        AudioViewer {
            rx,
            _alive: alive,
            format: self.format,
        }
    }
    pub fn publish(&self, pts_us: u64, data: Vec<u8>) {
        if data.is_empty() || data.len() > 8191 {
            return;
        }
        let data = data.into_boxed_slice().into_vec();
        let frame = Arc::new(AudioFrame { pts_us, data });
        self.subs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|s| {
                s.viewer.strong_count() != 0
                    && !matches!(
                        s.tx.try_send(Arc::clone(&frame)),
                        Err(TrySendError::Disconnected(_))
                    )
            });
    }
}

use super::agc::TalkGain;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Speaker {
    claimed: bool,
    queue: VecDeque<(Instant, Vec<i16>)>,
}

/// Exclusive speaker input, shared by both RTSP paths. At most five packets queued, and samples older
/// than 100 ms are discarded rather than played late. Disconnect clears outstanding samples.
#[derive(Default)]
pub struct Talkback {
    inner: Mutex<Speaker>,
}

pub struct TalkLease {
    talk: Arc<Talkback>,
    decoder: Pcmu,
    gain: TalkGain,
}

impl Talkback {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    pub fn claim(self: &Arc<Self>) -> Option<TalkLease> {
        let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if state.claimed {
            return None;
        }
        state.claimed = true;
        Some(TalkLease {
            talk: Arc::clone(self),
            decoder: Pcmu::default(),
            gain: TalkGain::default(),
        })
    }
    /// Called by the audio hardware loop; it must not wait for a network client.
    pub fn next_pcm(&self, now: Instant) -> Option<Vec<i16>> {
        let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        while let Some((at, pcm)) = state.queue.pop_front() {
            if now.saturating_duration_since(at) <= Duration::from_millis(100) {
                return Some(pcm);
            }
        }
        None
    }
}

impl TalkLease {
    pub fn push_rtp(&mut self, packet: &[u8]) {
        if let Some(mut pcm) = self.decoder.decode(packet) {
            self.gain.process(&mut pcm);
            let mut state = self.talk.inner.lock().unwrap_or_else(|e| e.into_inner());
            if state.queue.len() < 5 {
                state.queue.push_back((Instant::now(), pcm));
            }
        }
    }
}

impl Drop for TalkLease {
    fn drop(&mut self) {
        let mut state = self.talk.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.queue.clear();
        state.claimed = false;
    }
}

/// Stateful 8 → 16 kHz linear interpolation, retaining the last sample across packets.
#[derive(Default)]
pub struct Pcmu {
    previous: Option<i16>,
}

impl Pcmu {
    /// One RTP v2 PCMU packet. Malformed packets and non-PCMU payloads are ignored.
    pub fn decode(&mut self, packet: &[u8]) -> Option<Vec<i16>> {
        let payload = rtp_payload(packet)?;
        if payload.is_empty() || payload.len() > 1600 {
            return None;
        }
        let mut pcm = Vec::with_capacity(payload.len() * 2);
        for &byte in payload {
            let u = !byte;
            let magnitude = (((u & 15) as i32 * 8 + 132) << ((u >> 4) & 7)) - 132;
            let sample = if u & 128 != 0 { -magnitude } else { magnitude } as i16;
            let previous = self.previous.unwrap_or(sample);
            pcm.push(((i32::from(previous) + i32::from(sample)) / 2) as i16);
            pcm.push(sample);
            self.previous = Some(sample);
        }
        Some(pcm)
    }
}

fn rtp_payload(packet: &[u8]) -> Option<&[u8]> {
    if packet.len() < 12 || packet[0] >> 6 != 2 || packet[1] & 127 != 0 {
        return None;
    }
    let mut start = 12 + usize::from(packet[0] & 15) * 4;
    if packet[0] & 16 != 0 {
        let header = packet.get(start..start + 4)?;
        start += 4 + usize::from(u16::from_be_bytes([header[2], header[3]])) * 4;
    }
    let padding = if packet[0] & 32 != 0 {
        let n = usize::from(*packet.last()?);
        if n == 0 {
            return None;
        }
        n
    } else {
        0
    };
    packet.get(start..packet.len().checked_sub(padding)?)
}

/// AAC-LC, mono, 16 kHz, RFC 3640 AAC-hbr (13-bit size / 3-bit index).
/// The input is one raw AAC access unit, without ADTS.
pub struct AacPacketizer {
    seq: u16,
    ssrc: u32,
}

impl AacPacketizer {
    pub fn new(ssrc: u32) -> Self {
        Self { seq: 0, ssrc }
    }

    pub fn packets(&mut self, pts_us: u64, au: &[u8]) -> Option<Vec<Vec<u8>>> {
        if au.is_empty() || au.len() > 8191 {
            return None;
        }
        let ts = (pts_us.wrapping_mul(16_000) / 1_000_000) as u32;
        let chunks = au.len().div_ceil(1384);
        let mut packets = Vec::with_capacity(chunks);
        for (i, part) in au.chunks(1384).enumerate() {
            let mut packet = Vec::with_capacity(16 + part.len());
            packet.extend_from_slice(&[0x80, 97 | if i + 1 == chunks { 128 } else { 0 }]);
            packet.extend_from_slice(&self.seq.to_be_bytes());
            packet.extend_from_slice(&ts.to_be_bytes());
            packet.extend_from_slice(&self.ssrc.to_be_bytes());
            packet.extend_from_slice(&[0, 16]);
            packet.extend_from_slice(&((au.len() as u16) << 3).to_be_bytes());
            packet.extend_from_slice(part);
            packets.push(packet);
            self.seq = self.seq.wrapping_add(1);
        }
        Some(packets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaker_has_one_owner_bounded_latency_and_discards_audio_on_disconnect() {
        let talk = Talkback::new();
        let mut owner = talk.claim().unwrap();
        assert!(talk.claim().is_none());
        let packet = [0x80, 0, 0, 1, 0, 0, 0, 0, 1, 2, 3, 4, 0x80];
        for _ in 0..20 {
            owner.push_rtp(&packet);
        }
        let now = std::time::Instant::now();
        let mut count = 0;
        while talk.next_pcm(now).is_some() {
            count += 1;
        }
        assert_eq!(count, 5);
        owner.push_rtp(&packet);
        assert!(
            talk.next_pcm(now + std::time::Duration::from_secs(1))
                .is_none()
        );
        owner.push_rtp(&packet);
        drop(owner);
        assert!(talk.next_pcm(now).is_none());
        assert!(talk.claim().is_some());
    }

    #[test]
    fn audio_fanout_drops_lag_without_blocking_other_listeners() {
        let hub = AudioHub::new();
        let slow = hub.subscribe();
        let fast = hub.subscribe();
        for pts in 0..100 {
            hub.publish(pts, vec![7]);
            assert_eq!(fast.next(Duration::ZERO).unwrap().pts_us, pts);
        }
        let mut count = 0;
        while slow.next(Duration::ZERO).is_ok() {
            count += 1;
        }
        assert_eq!(count, 16);
    }

    #[test]
    fn aac_uses_sixteen_khz_clock_and_rfc3640_au_headers() {
        let mut pk = AacPacketizer::new(0x12345678);
        let packets = pk.packets(1_000_000, &[1, 2, 3, 4]).unwrap();
        assert_eq!(
            packets,
            vec![vec![
                0x80, 0xe1, 0, 0, 0, 0, 0x3e, 0x80, 0x12, 0x34, 0x56, 0x78, 0, 16, 0, 32, 1, 2, 3,
                4
            ]]
        );
        assert_eq!(
            &pk.packets(1_064_000, &[5]).unwrap()[0][2..8],
            &[0, 1, 0, 0, 0x42, 0x80]
        );
    }

    #[test]
    fn pcmu_rtp_decodes_to_sixteen_khz_pcm() {
        let packet = [
            0x80, 0x80, 0, 1, 0, 0, 0, 0, 1, 2, 3, 4, 0xff, 0x80, 0x00, 0x7f,
        ];
        let mut decoder = Pcmu::default();
        assert_eq!(
            decoder.decode(&packet).unwrap(),
            [0, 0, 16062, 32124, 0, -32124, -16062, 0]
        );
    }
}

#[cfg(test)]
mod framing_tests {
    use super::*;

    #[test]
    fn pcmu_skips_csrc_extension_and_padding_and_keeps_interpolation_between_packets() {
        let packet = [
            0xb1, 0, 0, 1, 0, 0, 0, 0, 1, 2, 3, 4, 9, 8, 7, 6, // CSRC
            0xbe, 0xde, 0, 1, 1, 2, 3, 4, // extension
            0x80, 0, 2,
        ]; // sample + padding
        let mut decoder = Pcmu::default();
        assert_eq!(decoder.decode(&packet).unwrap(), [32124, 32124]);
        let next = [0x80, 0, 0, 2, 0, 0, 0, 1, 1, 2, 3, 4, 0xff];
        assert_eq!(decoder.decode(&next).unwrap(), [16062, 0]);
    }

    #[test]
    fn malformed_rtp_never_produces_speaker_samples() {
        let mut decoder = Pcmu::default();
        for len in 0..32 {
            for first in 0..=255 {
                let mut packet = vec![0; len];
                if len > 0 {
                    packet[0] = first;
                }
                let pcm = decoder.decode(&packet);
                if len < 12 || first >> 6 != 2 {
                    assert!(pcm.is_none());
                }
                if let Some(pcm) = pcm {
                    assert!(pcm.len() <= 2 * len);
                }
            }
        }
        for packet in [
            vec![0x80, 97, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff], // AAC
            vec![0x90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff],  // short extension
            vec![0xa0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100],   // excessive padding
        ] {
            assert!(decoder.decode(&packet).is_none());
        }
    }

    #[test]
    fn aac_fragments_keep_au_size_timestamp_and_only_final_marker() {
        let mut pk = AacPacketizer::new(7);
        let au = vec![0x42; 3000];
        let packets = pk.packets(1_000_000, &au).unwrap();
        assert_eq!(packets.len(), 3);
        let mut assembled = Vec::new();
        for (index, p) in packets.iter().enumerate() {
            assert!(p.len() <= 1400);
            assert_eq!(p[1], if index == 2 { 225 } else { 97 });
            assert_eq!(&p[4..8], &[0, 0, 0x3e, 0x80]);
            assert_eq!(&p[12..16], &[0, 16, 0x5d, 0xc0]);
            assembled.extend_from_slice(&p[16..]);
        }
        assert_eq!(assembled, au);
        assert!(pk.packets(0, &[]).is_none());
        assert!(pk.packets(0, &vec![0; 8192]).is_none());
    }
}
