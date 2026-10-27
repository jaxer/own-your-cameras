//! Latest-wins hand-off between the settings owner (the `mqtt-hardware` thread) and the media thread,
//! which alone may call HDAL. The owner posts requests; the media loop takes at most one merged request
//! per iteration and reports what actually runs.

use std::sync::Mutex;

use crate::pipeline::{MainStream, MediaTuning, Tuned, Tuner};
use hdal::audio::Tuning;

#[derive(Debug, Default)]
struct State {
    initial: MediaTuning,
    pending: MediaTuning,
    audio: Option<Tuning>,
    main: Option<MainStream>,
    /// Refusals not yet seen by the owner.
    refusals: Vec<String>,
    restarts: Vec<u64>,
    /// Bumped on every report, so the owner republishes states the moment something was applied.
    reports: u64,
}

#[derive(Debug, Default)]
pub struct Handoff(Mutex<State>);

impl Handoff {
    /// `initial` is what the graph is built with (the persisted settings).
    pub fn new(initial: MediaTuning) -> Self {
        Self(Mutex::new(State {
            initial,
            ..State::default()
        }))
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn request_audio(&self, t: Tuning) {
        self.state().pending.audio = Some(t);
    }

    pub fn request_main(&self, m: MainStream) {
        self.state().pending.main = Some(m);
    }

    /// What runs now, once the media thread reported it.
    pub fn applied(&self) -> (Option<Tuning>, Option<MainStream>) {
        let s = self.state();
        (s.audio, s.main)
    }

    /// Requests the media thread refused (or replaced with stock at startup) since the last call.
    pub fn take_refusals(&self) -> Vec<String> {
        std::mem::take(&mut self.state().refusals)
    }

    /// Number of reports so far.
    pub fn reports(&self) -> u64 {
        self.state().reports
    }

    /// Main-path restart durations (ms) since the last call.
    pub fn take_restarts(&self) -> Vec<u64> {
        std::mem::take(&mut self.state().restarts)
    }
}

impl Tuner for &Handoff {
    fn initial(&mut self) -> MediaTuning {
        self.state().initial
    }

    fn next(&mut self) -> Option<MediaTuning> {
        let mut s = self.state();
        let p = std::mem::take(&mut s.pending);
        (p != MediaTuning::default()).then_some(p)
    }

    fn report(&mut self, tuned: Tuned) {
        let mut s = self.state();
        s.reports += 1;
        match tuned {
            Tuned::Audio { applied, refused } => {
                s.audio = Some(applied);
                if let Some(e) = refused {
                    s.refusals.push(format!("audio: {e}"));
                }
            }
            Tuned::Main {
                applied,
                refused,
                restart_ms,
            } => {
                s.main = Some(applied);
                if restart_ms > 0 {
                    s.restarts.push(restart_ms);
                }
                if let Some(e) = refused {
                    s.refusals.push(format!("main stream: {e}"));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_merge_latest_wins_and_are_taken_once() {
        let h = Handoff::new(MediaTuning::default());
        let mut t = &h;
        assert_eq!(t.next(), None);
        let mut a = Tuning::stock();
        h.request_audio(a);
        a.mic_volume = 10;
        h.request_audio(a);
        let m = MainStream {
            dim: hdal::Dim { w: 1280, h: 960 },
            bitrate_bps: 512_000,
            fps: 10,
        };
        h.request_main(m);
        assert_eq!(
            t.next(),
            Some(MediaTuning {
                audio: Some(a),
                main: Some(m)
            })
        );
        assert_eq!(t.next(), None);
    }

    #[test]
    fn reports_are_what_the_owner_reads_back() {
        let h = Handoff::new(MediaTuning::default());
        let mut t = &h;
        t.report(Tuned::Audio {
            applied: Tuning::stock(),
            refused: Some(hdal::Error {
                step: hdal::Step::AudioCapStart,
                code: -1,
            }),
        });
        assert_eq!(h.applied().0, Some(Tuning::stock()));
        assert_eq!(h.take_refusals().len(), 1);
        assert!(h.take_refusals().is_empty());
    }
}
