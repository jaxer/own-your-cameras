//! Automatic gain for talkback: a phone or browser microphone arrives far below the level the speaker
//! needs to fill a room, so speech is raised towards a loud target and a peak limiter keeps it clean.

/// Speech level the gain aims for, in linear full-scale units (-12 dBFS).
const TARGET_RMS: f32 = 0.251;
/// Largest boost (+24 dB); quieter input stays quiet rather than turning noise into speech level.
const MAX_GAIN: f32 = 16.0;
/// Gain for the first packet of a session (+12 dB), so the first word is not lost while adapting.
const START_GAIN: f32 = 4.0;
/// Below this packet RMS (-50 dBFS) the input is a pause or room noise: level and gain are held.
const GATE_RMS: f32 = 0.003_16;
/// Weight of each speech packet in the smoothed speech level (about 130 ms at 20 ms packets).
const LEVEL_WEIGHT: f32 = 0.15;
/// Per-packet gain fall while speech is too loud (about -1.4 dB per packet).
const FALL: f32 = 0.85;
/// Per-packet gain rise while speech is too quiet (about +6 dB per second at 20 ms packets).
const RISE: f32 = 1.028;
/// Peak ceiling after gain (-1 dBFS).
const CEILING: f32 = 0.891;
/// Limiter release per 16 kHz sample (50 ms time constant); its attack is instant.
const RELEASE: f32 = 0.998_75;

/// One talk session's gain state; a new session starts from `START_GAIN`.
pub struct TalkGain {
    gain: f32,
    level: Option<f32>,
    envelope: f32,
}

impl Default for TalkGain {
    fn default() -> Self {
        Self {
            gain: START_GAIN,
            level: None,
            envelope: 0.0,
        }
    }
}

impl TalkGain {
    /// Scales one decoded packet in place. The gain follows the smoothed speech level, ramping across
    /// the packet so a change never clicks; the limiter then holds every sample under the ceiling.
    pub fn process(&mut self, pcm: &mut [i16]) {
        if pcm.is_empty() {
            return;
        }
        let sum: f32 = pcm.iter().map(|&s| (f32::from(s) / 32768.0).powi(2)).sum();
        let rms = (sum / pcm.len() as f32).sqrt();
        let mut want = self.gain;
        if rms >= GATE_RMS {
            let level = self.level.map_or(rms, |l| l + (rms - l) * LEVEL_WEIGHT);
            self.level = Some(level);
            let ideal = (TARGET_RMS / level).clamp(1.0, MAX_GAIN);
            want = if ideal < want {
                ideal.max(want * FALL)
            } else {
                (want * RISE).min(ideal)
            };
        }
        let step = (want - self.gain) / pcm.len() as f32;
        for (i, s) in pcm.iter_mut().enumerate() {
            let y = f32::from(*s) / 32768.0 * (self.gain + step * (i + 1) as f32);
            self.envelope = y.abs().max(self.envelope * RELEASE);
            let limited = if self.envelope > CEILING {
                y * CEILING / self.envelope
            } else {
                y
            };
            *s = (limited * 32768.0) as i16;
        }
        self.gain = want;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 20 ms, 16 kHz packet of a 440 Hz tone at `dbfs` RMS.
    fn packet(dbfs: f32, phase: &mut usize) -> Vec<i16> {
        let amp = 10f32.powf(dbfs / 20.0) * std::f32::consts::SQRT_2 * 32768.0;
        (0..320)
            .map(|_| {
                *phase += 1;
                let t = *phase as f32 / 16_000.0;
                (amp * (2.0 * std::f32::consts::PI * 440.0 * t).sin()) as i16
            })
            .collect()
    }

    fn rms_dbfs(pcm: &[i16]) -> f32 {
        let sum: f32 = pcm.iter().map(|&s| (f32::from(s) / 32768.0).powi(2)).sum();
        20.0 * (sum / pcm.len() as f32).sqrt().log10()
    }

    fn peak(pcm: &[i16]) -> f32 {
        pcm.iter().map(|s| f32::from(*s).abs()).fold(0.0, f32::max) / 32768.0
    }

    #[test]
    fn quiet_speech_is_raised_to_the_target_within_a_few_seconds() {
        let (mut agc, mut phase) = (TalkGain::default(), 0);
        let mut last = Vec::new();
        for _ in 0..200 {
            last = packet(-30.0, &mut phase);
            agc.process(&mut last);
        }
        let level = rms_dbfs(&last);
        assert!((-13.0..=-11.0).contains(&level), "{level}");
    }

    #[test]
    fn the_boost_stops_at_24_db() {
        let (mut agc, mut phase) = (TalkGain::default(), 0);
        let mut last = Vec::new();
        for _ in 0..400 {
            last = packet(-46.0, &mut phase);
            agc.process(&mut last);
        }
        let level = rms_dbfs(&last);
        assert!((-22.5..=-21.5).contains(&level), "{level}");
    }

    #[test]
    fn loud_input_is_never_amplified_and_never_passes_the_ceiling() {
        let (mut agc, mut phase) = (TalkGain::default(), 0);
        for _ in 0..100 {
            let mut p = packet(-3.0, &mut phase);
            agc.process(&mut p);
            assert!(peak(&p) <= CEILING + 1e-4, "{}", peak(&p));
        }
        let mut p = packet(-3.0, &mut phase);
        agc.process(&mut p);
        assert!(rms_dbfs(&p) <= -3.0, "{}", rms_dbfs(&p));
    }

    #[test]
    fn the_first_packet_starts_boosted() {
        let (mut agc, mut phase) = (TalkGain::default(), 0);
        let mut p = packet(-30.0, &mut phase);
        agc.process(&mut p);
        let tail = rms_dbfs(&p[280..]);
        assert!((-19.0..=-17.0).contains(&tail), "{tail}");
    }

    #[test]
    fn pauses_hold_the_gain_instead_of_pumping_up_room_noise() {
        let (mut agc, mut phase) = (TalkGain::default(), 0);
        for _ in 0..200 {
            agc.process(&mut packet(-60.0, &mut phase));
        }
        assert_eq!(agc.gain, START_GAIN);
    }

    #[test]
    fn a_shout_after_quiet_speech_is_limited_at_once_and_the_gain_falls() {
        let (mut agc, mut phase) = (TalkGain::default(), 0);
        for _ in 0..200 {
            agc.process(&mut packet(-40.0, &mut phase));
        }
        let before = agc.gain;
        let mut shout = packet(-6.0, &mut phase);
        agc.process(&mut shout);
        assert!(peak(&shout) <= CEILING + 1e-4, "{}", peak(&shout));
        assert!(agc.gain < before);
        for _ in 0..50 {
            agc.process(&mut packet(-6.0, &mut phase));
        }
        assert_eq!(agc.gain, 1.0);
    }
}
