//! Contrast autofocus: the one search and the passive blur watchdog.
//!
//! A search climbs from the current focus in the vendor's coarse step until a sample on each side of the best one
//! reads clearly below it (the bracket), sweeps the fine step around the best, and settles there approaching from
//! below so gear backlash lands the same way every time. A climb that runs into the driver window's edge before it
//! is bracketed (no contrast, or the peak beyond the edge) falls back to the full coarse sweep of the window, whose
//! flat-scene test chooses the calibrated (`sp`) focus. A pan/tilt move first checks VA at the focus the last search
//! found and keeps it when the scene is still as sharp. It runs in steps, at most one focus move or one VA read per
//! [`Autofocus::poll`], so the hardware thread keeps serving commands meanwhile.
//!
//! The watchdog only reads VA. When it stays far below the value the last search found, and steady, for the hold
//! time, it asks for one search; it never moves the focus otherwise.

use std::collections::VecDeque;
use std::time::Duration;

use super::{Error, FOCUS_MAX, Isp, Lens, Positions, VA_TRIES};
use crate::zoom::zf_table::ZfTable;

/// A sweep whose best VA is less than this much above its worst saw no contrast (dark or blank scene).
const FLAT_PERCENT: u64 = 10;
/// Delay between two VA reads of one sample; two reads within 1/30 count as stable (`Na_af_get_cur_stable_va`).
const VA_POLL: Duration = Duration::from_millis(20);
/// Motor status poll while a focus move runs.
const MOVE_POLL: Duration = Duration::from_millis(10);
/// How long a focus move may take before the search fails (`Na_af_wait_motor_free(…, 60)`).
const MOVE_TIMEOUT: Duration = super::WAIT;
/// A pipeline that delivers no VA yet (startup) is asked again after this long.
const NO_VA_RETRY: Duration = Duration::from_secs(1);
/// After a day/night switch the search waits this long, so exposure caught up and VA reflects the new image.
pub const DAY_NIGHT_SETTLE: Duration = Duration::from_secs(1);
/// Watchdog sampling period.
const WATCH_PERIOD: Duration = Duration::from_secs(1);
/// Samples the watchdog keeps for the sensor attributes.
const HISTORY: usize = 60;
/// A window counts as still when its samples spread no more than this percent of their mean.
const STILL_PERCENT: u64 = 10;
/// At most one watchdog search this often, so a permanently changed scene cannot make it hunt.
pub const WATCHDOG_RATE: Duration = Duration::from_secs(600);

/// What asked for a search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    Zoom,
    Move,
    DayNight,
    Button,
    Watchdog,
    /// One point of the calibration walk: always a full search.
    Calibration,
}

impl Trigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Zoom => "zoom",
            Self::Move => "move",
            Self::DayNight => "day_night",
            Self::Button => "button",
            Self::Watchdog => "watchdog",
            Self::Calibration => "calibration",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AfState {
    Idle,
    Searching,
    /// The search found contrast and settled on this focus.
    Focused(u32),
    /// A pan/tilt move found the scene still sharp at the focus the last search found, and kept it.
    Kept(u32),
    /// The scene was flat; focus is the calibrated point.
    Calibrated(u32),
    /// The search failed; focus is the calibrated point.
    Failed(String),
}

impl std::fmt::Display for AfState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Searching => write!(f, "searching"),
            Self::Focused(p) => write!(f, "focused {p}"),
            Self::Kept(p) => write!(f, "kept {p}"),
            Self::Calibrated(p) => write!(f, "calibrated {p}"),
            Self::Failed(e) => write!(f, "failed: {e}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AfConfig {
    /// Off: triggers other than the button do nothing and the watchdog sleeps.
    pub enabled: bool,
    /// How long VA must stay soft and still before the watchdog searches.
    pub watch_hold: Duration,
    /// Soft means VA under this percent of the reference.
    pub watch_drop_pct: u32,
    /// After a pan/tilt, the focus is kept while VA is at least this percent of the reference.
    pub move_keep_pct: u32,
    /// A climb sample this percent below the best brackets the peak on its side.
    pub climb_drop_pct: u32,
}

impl Default for AfConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            watch_hold: Duration::from_secs(30),
            watch_drop_pct: 60,
            move_keep_pct: 80,
            climb_drop_pct: 2,
        }
    }
}

/// What the `autofocus` sensor shows besides the state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AfReport {
    pub trigger: Option<Trigger>,
    /// How long the last finished search took.
    pub duration: Option<Duration>,
    /// VA at the focus the last successful search found; the watchdog compares against it.
    pub reference: Option<u64>,
    /// The latest VA read.
    pub va: Option<u64>,
    /// Every (focus, VA) the last search measured, in order.
    pub samples: Vec<(u32, u64)>,
    /// Searches started since the daemon started (checks included).
    pub searches: u64,
    /// The watchdog's latest once-a-second VA samples, oldest first.
    pub history: Vec<u64>,
}

/// The positions one search visits at one zoom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sweep {
    pub lo: u32,
    pub hi: u32,
    pub coarse: u32,
    pub fine: u32,
}

impl Sweep {
    /// The sweep of the driver window at `zoom`. `Na_af_start_af`: steps 8/4 in the tele third of the calibrated
    /// range, 12/6 below it.
    pub fn new(zoom: u32, (lo, hi): (u32, u32), zf: &ZfTable) -> Result<Self, Error> {
        let hi = hi.min(FOCUS_MAX);
        if lo > hi {
            return Err(Error::InvalidWindow { lo, hi });
        }
        let tele = zf.points.last().map_or(0, |p| p.zoom);
        let coarse = if 2 * tele / 3 <= zoom { 8 } else { 12 };
        Ok(Self {
            lo,
            hi,
            coarse,
            fine: coarse / 2,
        })
    }

    pub fn coarse_points(&self) -> Vec<u32> {
        (self.lo..=self.hi).step_by(self.coarse as usize).collect()
    }

    /// One coarse step either side of the coarse best, in the fine step.
    pub fn fine_points(&self, best: u32) -> Vec<u32> {
        let from = best.saturating_sub(self.coarse).max(self.lo);
        let to = (best + self.coarse).min(self.hi);
        (from..=to).step_by(self.fine as usize).collect()
    }

    /// [`Self::fine_points`] without the positions `sampled` already holds.
    pub fn fine_points_beyond(&self, best: u32, sampled: &[(u32, u64)]) -> Vec<u32> {
        self.fine_points(best)
            .into_iter()
            .filter(|f| !sampled.iter().any(|s| s.0 == *f))
            .collect()
    }

    /// The next step of a climb from `start` given its `samples` so far: sample `start` first, then one coarse step
    /// past the farthest sample above the best until a sample on that side reads `drop_pct` percent below the best,
    /// then the same below it. Samples on either side count whichever order they came in, and a new best re-centres
    /// the sides. A step that would leave the window before its side is bracketed is an edge.
    pub fn climb(&self, start: u32, samples: &[(u32, u64)], drop_pct: u32) -> Climb {
        let Some((top, top_va)) = best(samples) else {
            return Climb::Probe(start.clamp(self.lo, self.hi));
        };
        let dropped = |s: &(u32, u64)| s.1 * 100 <= top_va * (100 - u64::from(drop_pct));
        let above = samples.iter().filter(|s| s.0 > top);
        if !above.clone().any(dropped) {
            let far = above.map(|s| s.0).max().unwrap_or(top);
            return match far.checked_add(self.coarse) {
                Some(next) if next <= self.hi => Climb::Probe(next),
                _ => Climb::Edge,
            };
        }
        let below = samples.iter().filter(|s| s.0 < top);
        if !below.clone().any(dropped) {
            let near = below.map(|s| s.0).min().unwrap_or(top);
            return match near.checked_sub(self.coarse) {
                Some(next) if next >= self.lo => Climb::Probe(next),
                _ => Climb::Edge,
            };
        }
        Climb::Bracketed(top)
    }

    /// The moves that end on `target` from `at`: from above, first one coarse step below it.
    pub fn settle(&self, at: u32, target: u32) -> Vec<u32> {
        if at > target {
            vec![target.saturating_sub(self.coarse).max(self.lo), target]
        } else {
            vec![target]
        }
    }
}

/// What a climb does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Climb {
    /// Move here and sample.
    Probe(u32),
    /// Both sides of this best sample read below it: run the fine stage around it.
    Bracketed(u32),
    /// The next step would leave the window: fall back to the full sweep.
    Edge,
}

/// Whether a pan/tilt keeps the focus: the lens is still where `reference` was measured and VA holds at least
/// `keep_pct` of it.
pub fn keep_focus(reference: &Reference, at: Positions, va: u64, keep_pct: u32) -> bool {
    at.zoom == reference.zoom
        && at.focus == reference.focus
        && va * 100 >= reference.va * u64::from(keep_pct)
}

/// Where and what the last successful search measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    pub zoom: u32,
    pub focus: u32,
    pub va: u64,
}

/// The first sample with the highest VA.
pub fn best(samples: &[(u32, u64)]) -> Option<(u32, u64)> {
    samples
        .iter()
        .copied()
        .fold(None, |b: Option<(u32, u64)>, s| match b {
            Some(b) if b.1 >= s.1 => Some(b),
            _ => Some(s),
        })
}

/// The focus of best contrast: the vertex of a least-squares parabola through the samples within two and a half coarse
/// steps of the highest one, kept within those samples. Near the peak VA is flat and noisy (±1 % on the camera), so the highest
/// single sample wanders by several fine steps between runs; the fitted vertex does not. The highest sample when the
/// samples do not curve downward.
pub fn peak(samples: &[(u32, u64)], coarse: u32) -> Option<u32> {
    let (top, _) = best(samples)?;
    let span = 5 * coarse / 2;
    let near: Vec<(f64, f64)> = samples
        .iter()
        .filter(|s| s.0.abs_diff(top) <= span)
        .map(|s| (f64::from(s.0) - f64::from(top), s.1 as f64))
        .collect();
    let floor = near.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    // normal equations of y = a·x² + b·x + c
    let mut m = [0f64; 7];
    for &(x, y) in &near {
        let y = y - floor;
        m[0] += 1.0;
        m[1] += x;
        m[2] += x * x;
        m[3] += x * x * x;
        m[4] += x * x * x * x;
        m[5] += y;
        m[6] += x * y;
    }
    let sx2y: f64 = near.iter().map(|&(x, y)| x * x * (y - floor)).sum();
    let det3 = |a: [[f64; 3]; 3]| {
        a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
            - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
            + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
    };
    let lhs = [[m[4], m[3], m[2]], [m[3], m[2], m[1]], [m[2], m[1], m[0]]];
    let rhs = [sx2y, m[6], m[5]];
    let det = det3(lhs);
    let column = |i: usize| {
        let mut a = lhs;
        for (row, r) in a.iter_mut().zip(rhs) {
            row[i] = r;
        }
        det3(a) / det
    };
    if near.len() < 3 || det.abs() < f64::EPSILON {
        return Some(top);
    }
    let (a, b) = (column(0), column(1));
    if a >= 0.0 {
        return Some(top);
    }
    let lo = near.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let hi = near.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let vertex = (-b / (2.0 * a)).clamp(lo, hi);
    Some((f64::from(top) + vertex).round().max(0.0) as u32)
}

/// True when the best VA is less than [`FLAT_PERCENT`] above the worst: no contrast to focus on.
pub fn flat(samples: &[(u32, u64)]) -> bool {
    let worst = samples.iter().map(|s| s.1).min().unwrap_or(0);
    let top = samples.iter().map(|s| s.1).max().unwrap_or(0);
    top * 100 < worst * (100 + FLAT_PERCENT)
}

/// True when the samples spread no more than [`STILL_PERCENT`] of their mean.
pub fn still(values: impl Iterator<Item = u64> + Clone) -> bool {
    let (n, sum) = values
        .clone()
        .fold((0u64, 0u64), |(n, s), v| (n + 1, s + v));
    let lo = values.clone().min().unwrap_or(0);
    let hi = values.max().unwrap_or(0);
    (hi - lo) * 100 * n <= sum * STILL_PERCENT
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// A pan/tilt: one VA read where the last search ended, no move.
    Check,
    /// The climb from the current focus, one [`Sweep::climb`] step at a time.
    Climb,
    /// The full coarse sweep of the window: the fallback when the climb ran into an edge.
    Coarse,
    Fine,
    Settle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// Read the zoom and the driver window, then plan.
    Plan,
    /// Send the next move (queued, or the climb's).
    Go,
    /// A move was sent at this time; wait for the motor.
    Moving(Duration),
    /// The lens is in place: take the first VA read.
    Sample,
    /// Waiting for two VA reads that agree.
    Read { last: u64, tries: u32 },
}

struct Search {
    trigger: Trigger,
    started: Duration,
    stage: Stage,
    step: Step,
    zoom: u32,
    sweep: Option<Sweep>,
    queue: VecDeque<u32>,
    /// Where the climb starts: the focus at planning time, inside the window.
    start: u32,
    /// Where the last move went.
    at: u32,
    /// The climb reached an edge and the full sweep ran: its flat-scene test applies.
    swept: bool,
    samples: Vec<(u32, u64)>,
    /// Settle target and whether the scene was flat.
    target: Option<(u32, bool)>,
}

#[derive(Debug, Clone, Copy)]
struct Pending {
    trigger: Trigger,
    not_before: Duration,
}

pub struct Autofocus {
    cfg: AfConfig,
    state: AfState,
    pending: Option<Pending>,
    search: Option<Search>,
    report: AfReport,
    /// The last successful search's end; `report.reference` carries its VA.
    reference: Option<Reference>,
    /// A `Focused` end not yet handed to the focus map: (zoom, focus, VA).
    learned: Option<(u32, u32, u64)>,
    /// Soft samples in a row, (time, VA).
    window: VecDeque<(Duration, u64)>,
    last_sample: Option<Duration>,
    last_watchdog: Option<Duration>,
    history: VecDeque<u64>,
}

impl Autofocus {
    pub fn new(cfg: AfConfig) -> Self {
        Self {
            cfg,
            state: AfState::Idle,
            pending: None,
            search: None,
            report: AfReport::default(),
            reference: None,
            learned: None,
            window: VecDeque::new(),
            last_sample: None,
            last_watchdog: None,
            history: VecDeque::new(),
        }
    }

    pub fn retune(&mut self, cfg: AfConfig) {
        self.cfg = cfg;
        self.window.clear();
    }

    /// A trigger fired: start (or restart) a search once the lens is idle, and not before `not_before`. At most one
    /// search is pending; a newer trigger replaces it. Off, only the button searches.
    pub fn request(&mut self, trigger: Trigger, not_before: Duration) {
        if !self.cfg.enabled && !matches!(trigger, Trigger::Button | Trigger::Calibration) {
            return;
        }
        if trigger == Trigger::DayNight {
            self.set_reference(None);
        }
        self.window.clear();
        // The first auto IR decision at startup is part of starting up; a pan/tilt never downgrades a pending
        // search of another kind to a check (a preset sends its move before its zoom).
        let trigger = match self.pending {
            Some(p) if p.trigger == Trigger::Startup && trigger == Trigger::DayNight => p.trigger,
            Some(p) if trigger == Trigger::Move => p.trigger,
            _ => trigger,
        };
        self.pending = Some(Pending {
            trigger,
            not_before: self
                .pending
                .map_or(not_before, |p| p.not_before.max(not_before)),
        });
    }

    /// Any motion command: abandon a running or pending search. Returns true when a search had moved the focus,
    /// so the caller parks the calibrated point.
    pub fn cancel(&mut self) -> bool {
        self.pending = None;
        self.window.clear();
        let moved = self.search.take().is_some_and(|s| s.step != Step::Plan);
        if self.state == AfState::Searching {
            self.state = AfState::Idle;
        }
        moved
    }

    pub fn searching(&self) -> bool {
        self.search.is_some() || self.pending.is_some()
    }

    pub fn state(&self) -> &AfState {
        &self.state
    }

    pub fn report(&self) -> AfReport {
        AfReport {
            history: self.history.iter().copied().collect(),
            ..self.report.clone()
        }
    }

    /// Advance one step: at most one focus move or one VA read through `lens`. `motion_idle` is false while a
    /// pan/tilt or zoom hold runs. Returns how soon it wants the next poll; `None` is the idle cadence. An error
    /// ends the search at the calibrated focus and is never fatal.
    pub fn poll<I: Isp>(
        &mut self,
        lens: &mut Lens<I>,
        zf: &ZfTable,
        park: &dyn Fn(u32) -> u32,
        now: Duration,
        motion_idle: bool,
    ) -> Result<Option<Duration>, Error> {
        if !motion_idle {
            self.window.clear();
            return Ok(None);
        }
        if let Some(p) = self.pending {
            if now < p.not_before {
                return Ok(Some(p.not_before - now));
            }
            if lens.busy()? {
                return Ok(Some(MOVE_POLL));
            }
            // Startup: wait until the pipeline delivers VA.
            let va = lens.va()?;
            self.report.va = Some(va);
            if va == 0 {
                self.pending = Some(Pending {
                    not_before: now + NO_VA_RETRY,
                    ..p
                });
                return Ok(Some(NO_VA_RETRY));
            }
            self.pending = None;
            if p.trigger == Trigger::Watchdog {
                self.last_watchdog = Some(now);
            }
            self.search = Some(Search {
                trigger: p.trigger,
                started: now,
                stage: Stage::Climb,
                step: Step::Plan,
                zoom: 0,
                sweep: None,
                queue: VecDeque::new(),
                start: 0,
                at: 0,
                swept: false,
                samples: Vec::new(),
                target: None,
            });
            self.state = AfState::Searching;
            self.report.trigger = Some(p.trigger);
            self.report.searches += 1;
            return Ok(Some(Duration::ZERO));
        }
        if self.search.is_some() {
            return match self.step(lens, zf, park, now) {
                Ok(wait) => Ok(wait),
                Err(e) => {
                    self.fail(lens, park, &e);
                    Err(e)
                }
            };
        }
        self.watch(lens, now)
    }

    fn step<I: Isp>(
        &mut self,
        lens: &mut Lens<I>,
        zf: &ZfTable,
        park: &dyn Fn(u32) -> u32,
        now: Duration,
    ) -> Result<Option<Duration>, Error> {
        let Some(s) = self.search.as_mut() else {
            return Ok(None);
        };
        match s.step {
            Step::Plan => {
                let p = lens.positions()?;
                let sweep = Sweep::new(p.zoom, lens.focus_window()?, zf)?;
                s.zoom = p.zoom;
                s.at = p.focus;
                s.start = p.focus.clamp(sweep.lo, sweep.hi);
                s.sweep = Some(sweep);
                let check = s.trigger == Trigger::Move
                    && self
                        .reference
                        .is_some_and(|r| r.zoom == p.zoom && r.focus == p.focus);
                if check {
                    s.stage = Stage::Check;
                    s.step = Step::Sample;
                } else {
                    s.step = Step::Go;
                }
                Ok(Some(Duration::ZERO))
            }
            Step::Go => {
                let next = match s.stage {
                    Stage::Check => {
                        let r = self.reference.unwrap_or(Reference {
                            zoom: s.zoom,
                            focus: s.at,
                            va: 0,
                        });
                        let at = Positions {
                            zoom: s.zoom,
                            focus: s.at,
                        };
                        let va = s.samples.last().map_or(0, |v| v.1);
                        let kept = keep_focus(&r, at, va, self.cfg.move_keep_pct).then_some(s.at);
                        if let Some(f) = kept {
                            return self.finish(now, AfState::Kept(f), None);
                        }
                        s.stage = Stage::Climb;
                        return Ok(Some(Duration::ZERO));
                    }
                    Stage::Climb => match s
                        .sweep
                        .map(|w| w.climb(s.start, &s.samples, self.cfg.climb_drop_pct))
                    {
                        Some(Climb::Probe(f)) => Some(f),
                        Some(Climb::Bracketed(_)) | Some(Climb::Edge) | None => None,
                    },
                    _ => s.queue.pop_front(),
                };
                match next {
                    Some(f) if f == s.at && s.stage == Stage::Climb && s.samples.is_empty() => {
                        // the climb starts where the lens already is
                        s.step = Step::Sample;
                        Ok(Some(Duration::ZERO))
                    }
                    Some(f) => {
                        lens.focus_start(f)?;
                        s.at = f;
                        s.step = Step::Moving(now);
                        Ok(Some(MOVE_POLL))
                    }
                    None => self.next_stage(lens, park, now),
                }
            }
            Step::Moving(since) => {
                if lens.busy()? {
                    if now.saturating_sub(since) >= MOVE_TIMEOUT {
                        return Err(Error::MotorTimeout);
                    }
                    return Ok(Some(MOVE_POLL));
                }
                s.step = if s.stage == Stage::Settle {
                    Step::Go
                } else {
                    Step::Sample
                };
                Ok(Some(Duration::ZERO))
            }
            Step::Sample => {
                let va = lens.va()?;
                self.report.va = Some(va);
                s.step = Step::Read { last: va, tries: 0 };
                Ok(Some(VA_POLL))
            }
            Step::Read { last, tries } => {
                let va = lens.va()?;
                self.report.va = Some(va);
                if va != 0 && last != 0 && last.abs_diff(va) * 30 < va {
                    s.samples.push((s.at, va));
                    s.step = Step::Go;
                    return Ok(Some(Duration::ZERO));
                }
                if tries + 1 >= VA_TRIES {
                    if s.stage == Stage::Check {
                        // an unsettled scene is a reason to search, not to give up
                        s.stage = Stage::Climb;
                        s.step = Step::Go;
                        return Ok(Some(Duration::ZERO));
                    }
                    return Err(Error::VaUnstable);
                }
                s.step = Step::Read {
                    last: va,
                    tries: tries + 1,
                };
                Ok(Some(VA_POLL))
            }
        }
    }

    /// Ends the search in `state`; `reference` replaces the stored one when given.
    fn finish(
        &mut self,
        now: Duration,
        state: AfState,
        reference: Option<Option<Reference>>,
    ) -> Result<Option<Duration>, Error> {
        let s = self.search.take().expect("a search runs");
        self.report.duration = Some(now.saturating_sub(s.started));
        if let Some(r) = reference {
            self.set_reference(r);
        }
        self.report.samples = s.samples;
        self.state = state;
        self.window.clear();
        self.last_sample = Some(now);
        Ok(None)
    }

    fn set_reference(&mut self, r: Option<Reference>) {
        self.reference = r;
        self.report.reference = r.map(|r| r.va);
    }

    /// The (zoom, focus, VA) a `Focused` search ended at since the last call, once.
    pub fn take_learned(&mut self) -> Option<(u32, u32, u64)> {
        self.learned.take()
    }

    /// The queue of a stage ran out (or the climb ended): plan the next one or finish.
    fn next_stage<I: Isp>(
        &mut self,
        lens: &mut Lens<I>,
        park: &dyn Fn(u32) -> u32,
        now: Duration,
    ) -> Result<Option<Duration>, Error> {
        let Some(s) = self.search.as_mut() else {
            return Ok(None);
        };
        let Some(sweep) = s.sweep else {
            return Ok(None);
        };
        match s.stage {
            Stage::Check => {
                s.stage = Stage::Climb;
            }
            Stage::Climb => match sweep.climb(s.start, &s.samples, self.cfg.climb_drop_pct) {
                Climb::Bracketed(b) => {
                    s.queue = sweep.fine_points_beyond(b, &s.samples).into();
                    s.stage = Stage::Fine;
                }
                Climb::Edge | Climb::Probe(_) => {
                    s.queue = sweep.coarse_points().into();
                    s.stage = Stage::Coarse;
                    s.swept = true;
                }
            },
            Stage::Coarse => {
                let b = best(&s.samples).map_or(sweep.lo, |b| b.0);
                s.queue = sweep.fine_points(b).into();
                s.stage = Stage::Fine;
            }
            Stage::Fine => {
                // a bracketed climb proved contrast; only the full sweep can find a flat scene
                let calibrated = s.swept && flat(&s.samples);
                let target = if calibrated {
                    park(s.zoom).clamp(sweep.lo, sweep.hi)
                } else {
                    peak(&s.samples, sweep.coarse).map_or(sweep.lo, |p| p.clamp(sweep.lo, sweep.hi))
                };
                s.queue = sweep.settle(s.at, target).into();
                s.target = Some((target, calibrated));
                s.stage = Stage::Settle;
            }
            Stage::Settle => {
                let (target, calibrated) = s.target.unwrap_or((s.at, true));
                let reference = match best(&s.samples) {
                    Some((_, va)) if !calibrated => {
                        let p = lens.positions()?;
                        self.learned = Some((p.zoom, p.focus, va));
                        Some(Reference {
                            zoom: p.zoom,
                            focus: p.focus,
                            va,
                        })
                    }
                    _ => None,
                };
                let state = if calibrated {
                    AfState::Calibrated(target)
                } else {
                    AfState::Focused(target)
                };
                return self.finish(now, state, Some(reference));
            }
        }
        s.step = Step::Go;
        Ok(Some(Duration::ZERO))
    }

    /// Ends the running search at the park focus for its zoom (best effort; the error is already reported).
    fn fail<I: Isp>(&mut self, lens: &mut Lens<I>, park: &dyn Fn(u32) -> u32, e: &Error) {
        if let Some(s) = self.search.take()
            && s.step != Step::Plan
        {
            let _ = lens.focus_start(park(s.zoom));
        }
        self.state = AfState::Failed(e.to_string());
        self.window.clear();
    }

    /// One watchdog sample a second while nothing else runs; asks for a search when VA stayed soft and still.
    fn watch<I: Isp>(
        &mut self,
        lens: &mut Lens<I>,
        now: Duration,
    ) -> Result<Option<Duration>, Error> {
        if !self.cfg.enabled
            || self
                .last_sample
                .is_some_and(|t| now.saturating_sub(t) < WATCH_PERIOD)
        {
            return Ok(None);
        }
        self.last_sample = Some(now);
        let va = lens.va()?;
        self.report.va = Some(va);
        self.history.push_back(va);
        while self.history.len() > HISTORY {
            self.history.pop_front();
        }
        let Some(reference) = self.report.reference else {
            self.window.clear();
            return Ok(None);
        };
        if va * 100 >= reference * u64::from(self.cfg.watch_drop_pct) {
            self.window.clear();
            return Ok(None);
        }
        self.window.push_back((now, va));
        while !still(self.window.iter().map(|w| w.1)) {
            self.window.pop_front();
        }
        let held = self
            .window
            .front()
            .is_some_and(|w| now.saturating_sub(w.0) >= self.cfg.watch_hold);
        let rested = self
            .last_watchdog
            .is_none_or(|t| now.saturating_sub(t) >= WATCHDOG_RATE);
        if held && rested {
            self.request(Trigger::Watchdog, now);
            return Ok(Some(Duration::ZERO));
        }
        Ok(None)
    }
}

/// Runs one search to the end, sleeping between steps through the ISP (the `cam focus` command); `park` is the
/// focus a flat or failed search ends at for a zoom.
pub fn run_blocking<I: Isp>(
    lens: &mut Lens<I>,
    zf: &ZfTable,
    park: &dyn Fn(u32) -> u32,
) -> Result<AfReport, Error> {
    let mut af = Autofocus::new(AfConfig::default());
    af.request(Trigger::Button, Duration::ZERO);
    let mut now = Duration::ZERO;
    while af.searching() {
        let wait = af.poll(lens, zf, park, now, true)?.unwrap_or(MOVE_POLL);
        if !wait.is_zero() {
            lens.isp_mut().sleep(wait);
            now += wait;
        }
    }
    Ok(af.report())
}

#[cfg(test)]
mod tests {
    use super::super::fake::FakeIsp;
    use super::*;
    use crate::zoom::item;

    /// (0, 230) .. (512, 800) .. (1024, 1022): a three-point curve shaped like a real camera's
    fn zf() -> ZfTable {
        ZfTable::parse(b"<ZFTable>1000000,300e6,200,320,400,3fe</ZFTable>").unwrap()
    }

    /// The calibrated focus as the park point.
    fn park(zoom: u32) -> u32 {
        zf().focus_at(zoom)
    }

    fn lens(focus_window: (u32, u32), sharp: Option<u32>) -> Lens<FakeIsp> {
        let mut isp = FakeIsp::new(512, 300);
        isp.focus_window = focus_window;
        isp.sharp = sharp;
        Lens::new(isp)
    }

    /// Polls until no search runs or is pending, advancing time by each wait. Returns the time it ended.
    fn finish(af: &mut Autofocus, l: &mut Lens<FakeIsp>, mut now: Duration) -> Duration {
        for _ in 0..100_000 {
            if !af.searching() {
                return now;
            }
            let _ = af.poll(l, &zf(), &park, now, true);
            now += Duration::from_millis(10);
        }
        panic!("search never ended")
    }

    fn searched(sharp: u32) -> (Autofocus, Lens<FakeIsp>) {
        let mut l = lens((500, 700), Some(sharp));
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Button, Duration::ZERO);
        finish(&mut af, &mut l, Duration::ZERO);
        (af, l)
    }

    #[test]
    fn a_search_finds_the_sharp_focus() {
        let (af, l) = searched(600);
        // the climb from the window's edge 500 in steps of 12 is bracketed at 608 (above 596) and 584 (below);
        // the fine stage adds 590 and 602, the fitted vertex is 603
        assert_eq!(af.state(), &AfState::Focused(603));
        assert_eq!(l.isp().focus, 603);
        let r = af.report();
        assert_eq!(r.trigger, Some(Trigger::Button));
        assert_eq!(
            r.reference,
            Some(r.samples.iter().map(|s| s.1).max().unwrap())
        );
        assert!(r.duration.is_some());
        let moves = l.isp().focus_moves();
        assert_eq!(
            moves,
            [
                500, 512, 524, 536, 548, 560, 572, 584, 596, 608, 590, 602, 603
            ]
        );
    }

    #[test]
    fn a_climb_from_near_the_peak_takes_a_handful_of_moves() {
        let mut l = lens((500, 700), Some(600));
        l.isp_mut().focus = 596;
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Button, Duration::ZERO);
        finish(&mut af, &mut l, Duration::ZERO);
        // 596 is sampled without a move; 608 brackets above, 584 below; fine 590 and 602; the vertex 600 is
        // below 602, so the settle backs off first
        assert_eq!(l.isp().focus_moves(), [608, 584, 590, 602, 588, 600]);
        assert_eq!(af.state(), &AfState::Focused(600));
    }

    fn sweep() -> Sweep {
        Sweep::new(512, (500, 700), &zf()).unwrap()
    }

    #[test]
    fn the_climb_samples_the_start_then_brackets_above_then_below() {
        let w = sweep();
        assert_eq!(w.climb(548, &[], 2), Climb::Probe(548));
        let mut v = vec![(548, 1000)];
        assert_eq!(w.climb(548, &v, 2), Climb::Probe(560));
        v.push((560, 1100));
        assert_eq!(w.climb(548, &v, 2), Climb::Probe(572));
        v.push((572, 1090));
        // 1090 is only 0.9 % below 1100: not yet a drop
        assert_eq!(w.climb(548, &v, 2), Climb::Probe(584));
        v.push((584, 1000));
        // above is bracketed; 548 (9 % below) already brackets below
        assert_eq!(w.climb(548, &v, 2), Climb::Bracketed(560));
    }

    #[test]
    fn the_climb_goes_below_when_the_start_is_the_best_so_far() {
        let w = sweep();
        let v = [(600, 1000), (612, 900)];
        assert_eq!(w.climb(600, &v, 2), Climb::Probe(588));
        let v = [(600, 1000), (612, 900), (588, 950)];
        assert_eq!(w.climb(600, &v, 2), Climb::Bracketed(600));
    }

    #[test]
    fn a_new_best_recentres_the_climb() {
        let w = sweep();
        // the walk down finds a higher sample: the old best (600) now brackets above, the climb keeps going down
        let v = [(600, 1000), (612, 900), (588, 1100)];
        assert_eq!(w.climb(600, &v, 2), Climb::Probe(576));
        let v = [(600, 1000), (612, 900), (588, 1100), (576, 1050)];
        assert_eq!(w.climb(600, &v, 2), Climb::Bracketed(588));
    }

    #[test]
    fn the_drop_percent_is_the_bracket() {
        let w = sweep();
        let v = [(600, 1000), (612, 970), (588, 970)];
        assert_eq!(w.climb(600, &v, 2), Climb::Bracketed(600));
        assert_eq!(w.climb(600, &v, 5), Climb::Probe(624));
    }

    #[test]
    fn a_climb_that_reaches_an_edge_before_its_bracket_is_an_edge() {
        let w = sweep();
        // rising to the top of the window
        let up: Vec<(u32, u64)> = (0..).map(|i| (688 + i, 1000 + i as u64)).take(1).collect();
        assert_eq!(w.climb(688, &up, 2), Climb::Probe(700));
        let up = [(688, 1000), (700, 1010)];
        assert_eq!(w.climb(688, &up, 2), Climb::Edge);
        // falling to the bottom
        let down = [(512, 1000), (524, 900), (500, 1010)];
        assert_eq!(w.climb(512, &down, 2), Climb::Edge);
        // a start outside the window is clamped
        assert_eq!(w.climb(300, &[], 2), Climb::Probe(500));
    }

    #[test]
    fn a_flat_curve_climbs_to_an_edge_and_falls_back_to_the_full_sweep() {
        let mut l = lens((700, 900), None);
        l.isp_mut().focus = 800;
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Zoom, Duration::ZERO);
        finish(&mut af, &mut l, Duration::ZERO);
        let moves = l.isp().focus_moves();
        // the climb walks 812..900 without a drop (edge), then the full sweep from 700
        assert_eq!(&moves[..9], [812, 824, 836, 848, 860, 872, 884, 896, 700]);
        assert_eq!(af.state(), &AfState::Calibrated(800));
    }

    #[test]
    fn the_fine_stage_skips_sampled_points() {
        let w = sweep();
        assert_eq!(w.fine_points(596), [584, 590, 596, 602, 608]);
        assert_eq!(
            w.fine_points_beyond(596, &[(584, 1), (596, 2), (608, 3)]),
            [590, 602]
        );
    }

    /// A search that ended focused, then a pan/tilt at VA scaled to `scale_pct` of it.
    fn moved(scale_pct: u64) -> (Autofocus, Lens<FakeIsp>) {
        let (mut af, mut l) = searched(600);
        l.isp_mut().clear_sets();
        l.isp_mut().scale_pct = scale_pct;
        af.request(Trigger::Move, Duration::from_secs(5));
        finish(&mut af, &mut l, Duration::from_secs(5));
        (af, l)
    }

    #[test]
    fn a_move_keeps_the_focus_while_va_holds() {
        let (af, l) = moved(90);
        assert_eq!(af.state(), &AfState::Kept(603));
        assert!(l.isp().focus_moves().is_empty(), "the check makes no move");
        let r = af.report();
        assert_eq!(r.trigger, Some(Trigger::Move));
        assert_eq!(r.samples.len(), 1);
        assert_eq!(r.samples[0].0, 603);
        assert!(r.duration.is_some());
        assert_eq!(r.reference, searched(600).0.report().reference, "unchanged");
    }

    #[test]
    fn a_move_searches_when_va_fell_under_the_threshold() {
        let (af, l) = moved(70);
        assert_eq!(af.state(), &AfState::Focused(600));
        assert!(!l.isp().focus_moves().is_empty());
        assert_eq!(
            l.isp().focus_moves()[0],
            615,
            "the climb starts where the lens is"
        );
    }

    #[test]
    fn the_move_threshold_is_a_setting() {
        let (mut af, mut l) = searched(600);
        af.retune(AfConfig {
            move_keep_pct: 95,
            ..AfConfig::default()
        });
        l.isp_mut().clear_sets();
        l.isp_mut().scale_pct = 90;
        af.request(Trigger::Move, Duration::ZERO);
        finish(&mut af, &mut l, Duration::ZERO);
        assert!(matches!(af.state(), AfState::Focused(_)));
    }

    #[test]
    fn a_move_searches_when_the_lens_is_not_where_the_reference_was_taken() {
        for (zoom, focus) in [(512, 550), (600, 603)] {
            let (mut af, mut l) = searched(600);
            l.isp_mut().zoom = zoom;
            l.isp_mut().focus = focus;
            l.isp_mut().clear_sets();
            af.request(Trigger::Move, Duration::ZERO);
            finish(&mut af, &mut l, Duration::ZERO);
            assert!(matches!(af.state(), AfState::Focused(_)), "{zoom}/{focus}");
            assert!(!l.isp().focus_moves().is_empty());
        }
    }

    #[test]
    fn a_move_without_a_reference_searches() {
        let mut l = lens((500, 700), Some(600));
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Move, Duration::ZERO);
        finish(&mut af, &mut l, Duration::ZERO);
        assert_eq!(af.state(), &AfState::Focused(603));
    }

    #[test]
    fn a_move_with_unstable_va_searches_instead_of_failing() {
        let (mut af, mut l) = searched(600);
        l.isp_mut().clear_sets();
        l.isp_mut().va_script = (0..2 * VA_TRIES)
            .map(|i| 1_000 + u64::from(i % 2) * 50_000)
            .collect();
        af.request(Trigger::Move, Duration::ZERO);
        let mut now = Duration::ZERO;
        let mut failed = false;
        while af.searching() {
            failed |= af.poll(&mut l, &zf(), &park, now, true).is_err();
            now += Duration::from_millis(10);
        }
        assert!(!failed);
        assert_eq!(af.state(), &AfState::Focused(600));
    }

    #[test]
    fn a_move_never_downgrades_a_pending_search() {
        for t in [Trigger::Zoom, Trigger::Button] {
            let (mut af, mut l) = searched(600);
            l.isp_mut().clear_sets();
            af.request(t, Duration::ZERO);
            af.request(Trigger::Move, Duration::ZERO);
            finish(&mut af, &mut l, Duration::ZERO);
            assert_eq!(af.report().trigger, Some(t));
            assert!(!l.isp().focus_moves().is_empty(), "{t:?} searched");
        }
    }

    #[test]
    fn the_check_and_the_climb_move_or_read_at_most_once_per_poll() {
        let (mut af, mut l) = searched(600);
        l.isp_mut().scale_pct = 70;
        af.request(Trigger::Move, Duration::ZERO);
        let mut now = Duration::ZERO;
        while af.searching() {
            let (sets, reads) = (l.isp().sets().len(), l.isp().va_reads);
            af.poll(&mut l, &zf(), &park, now, true).unwrap();
            let moved = l.isp().sets().len() - sets;
            let read = (l.isp().va_reads - reads) as usize;
            assert!(
                moved + read <= 1,
                "{moved} moves and {read} reads in one poll"
            );
            now += Duration::from_millis(10);
        }
        assert_eq!(af.state(), &AfState::Focused(600));
    }

    #[test]
    fn the_peak_is_the_fitted_vertex_not_the_noisiest_sample() {
        // two Refocus presses on the camera at zoom 0 over the same still scene (VA in units of 100 000), whose
        // highest samples were 18 steps apart
        let a = [
            (86, 2464),
            (98, 2445),
            (110, 2451),
            (122, 2517),
            (134, 2554),
            (146, 2699),
            (158, 2878),
            (170, 2946),
            (182, 3017),
            (194, 2984),
            (206, 2912),
            (218, 2898),
            (230, 2790),
            (242, 2674),
            (254, 2635),
            (266, 2556),
            (278, 2506),
            (290, 2485),
            (302, 2451),
            (314, 2427),
            (326, 2403),
            (170, 3025),
            (176, 3042),
            (182, 3039),
            (188, 3030),
            (194, 3006),
        ];
        let b = [
            (86, 2464),
            (98, 2445),
            (110, 2457),
            (122, 2491),
            (134, 2589),
            (146, 2672),
            (158, 2794),
            (170, 2954),
            (182, 3013),
            (194, 3017),
            (206, 2928),
            (218, 2910),
            (230, 2839),
            (242, 2704),
            (254, 2607),
            (266, 2568),
            (278, 2516),
            (290, 2468),
            (302, 2457),
            (314, 2425),
            (326, 2413),
            (182, 2917),
            (188, 2970),
            (194, 2978),
            (200, 2964),
            (206, 2944),
        ];
        let (pa, pb) = (peak(&a, 12).unwrap(), peak(&b, 12).unwrap());
        assert_eq!((best(&a).unwrap().0, best(&b).unwrap().0), (176, 194));
        assert!(pa.abs_diff(pb) <= 6, "{pa} vs {pb}");
    }

    #[test]
    fn a_peak_that_does_not_curve_down_is_the_highest_sample() {
        assert_eq!(peak(&[(100, 1), (112, 2), (124, 3)], 12), Some(124));
        assert_eq!(peak(&[(100, 5)], 12), Some(100));
        assert_eq!(peak(&[], 12), None);
    }

    #[test]
    fn a_flat_scene_falls_back_to_the_calibrated_focus_and_leaves_no_reference() {
        let mut l = lens((700, 900), None);
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Zoom, Duration::ZERO);
        finish(&mut af, &mut l, Duration::ZERO);
        assert_eq!(af.state(), &AfState::Calibrated(800));
        assert_eq!(l.isp().focus, 800);
        assert_eq!(af.report().reference, None);
    }

    #[test]
    fn the_focus_is_approached_from_below() {
        let (af, l) = searched(594);
        let moves = l.isp().focus_moves();
        // the fine stage ends at 602, above the vertex 593, so it backs off a coarse step before the last move
        assert_eq!(af.state(), &AfState::Focused(593));
        assert_eq!(&moves[moves.len() - 3..], [602, 581, 593]);
    }

    #[test]
    fn each_poll_moves_the_focus_or_reads_va_at_most_once() {
        let mut l = lens((500, 700), Some(600));
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Button, Duration::ZERO);
        let mut now = Duration::ZERO;
        while af.searching() {
            let (sets, reads) = (l.isp().sets().len(), l.isp().va_reads);
            af.poll(&mut l, &zf(), &park, now, true).unwrap();
            let moved = l.isp().sets().len() - sets;
            let read = (l.isp().va_reads - reads) as usize;
            assert!(
                moved + read <= 1,
                "{moved} moves and {read} reads in one poll"
            );
            now += Duration::from_millis(10);
        }
    }

    #[test]
    fn cancel_mid_search_sends_no_further_moves() {
        let mut l = lens((500, 700), Some(600));
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Button, Duration::ZERO);
        let mut now = Duration::ZERO;
        for _ in 0..20 {
            af.poll(&mut l, &zf(), &park, now, true).unwrap();
            now += Duration::from_millis(10);
        }
        assert!(af.cancel(), "the search had moved the focus");
        let sets = l.isp().sets().len();
        for _ in 0..50 {
            af.poll(&mut l, &zf(), &park, now, true).unwrap();
            now += Duration::from_millis(10);
        }
        assert_eq!(l.isp().sets().len(), sets);
        assert!(!af.searching());
        assert_eq!(af.state(), &AfState::Idle);
    }

    #[test]
    fn a_new_request_restarts_the_search() {
        let mut l = lens((500, 700), Some(600));
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Zoom, Duration::ZERO);
        let mut now = Duration::ZERO;
        for _ in 0..20 {
            af.poll(&mut l, &zf(), &park, now, true).unwrap();
            now += Duration::from_millis(10);
        }
        let at = l.isp().focus;
        let done = l.isp().focus_moves().len();
        af.request(Trigger::Button, now);
        af.request(Trigger::Move, now);
        finish(&mut af, &mut l, now);
        let moves = l.isp().focus_moves();
        // the restarted climb starts where the lens is (sampled without a move), not from the window's edge
        assert_eq!(moves[done], at + 12, "{moves:?}");
        assert_eq!(moves.iter().filter(|f| **f == 500).count(), 1);
        assert_eq!(
            af.report().trigger,
            Some(Trigger::Button),
            "a move does not downgrade"
        );
        assert_eq!(af.state(), &AfState::Focused(603));
    }

    #[test]
    fn a_request_waits_for_its_time_and_for_motion_to_end() {
        let mut l = lens((500, 700), Some(600));
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::DayNight, Duration::from_secs(1));
        af.poll(&mut l, &zf(), &park, Duration::from_millis(500), true)
            .unwrap();
        af.poll(&mut l, &zf(), &park, Duration::from_secs(2), false)
            .unwrap();
        assert!(l.isp().sets().is_empty());
        af.poll(&mut l, &zf(), &park, Duration::from_secs(2), true)
            .unwrap();
        assert_eq!(af.state(), &AfState::Searching);
    }

    #[test]
    fn startup_waits_for_the_pipeline_to_deliver_va() {
        let mut l = lens((500, 700), Some(600));
        l.isp_mut().va_script = [0, 0].into();
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Startup, Duration::ZERO);
        assert_eq!(
            af.poll(&mut l, &zf(), &park, Duration::ZERO, true).unwrap(),
            Some(NO_VA_RETRY)
        );
        assert_eq!(af.state(), &AfState::Idle);
        finish(&mut af, &mut l, Duration::from_secs(1));
        assert_eq!(af.state(), &AfState::Focused(603));
    }

    #[test]
    fn unstable_va_fails_at_the_calibrated_focus() {
        let mut l = lens((500, 700), Some(600));
        // alternates far apart forever
        l.isp_mut().va_script = (0..10_000).map(|i| 1_000 + (i % 2) * 50_000).collect();
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Button, Duration::ZERO);
        let mut now = Duration::ZERO;
        let mut failed = None;
        while af.searching() {
            if let Err(e) = af.poll(&mut l, &zf(), &park, now, true) {
                failed = Some(e);
            }
            now += Duration::from_millis(10);
        }
        assert_eq!(failed, Some(Error::VaUnstable));
        assert!(matches!(af.state(), AfState::Failed(_)));
        assert_eq!(l.isp().focus_moves().last(), Some(&park(512)));
    }

    /// Feeds the watchdog `va` once a second from `from` for `secs` seconds; returns the second it asked for a
    /// search, if it did.
    fn watch(
        af: &mut Autofocus,
        l: &mut Lens<FakeIsp>,
        from: Duration,
        secs: u64,
        va: impl Fn(u64) -> u64,
    ) -> Option<u64> {
        for t in 0..secs {
            l.isp_mut().va_script = [va(t)].into();
            let now = from + Duration::from_secs(t);
            af.poll(l, &zf(), &park, now, true).unwrap();
            l.isp_mut().va_script.clear();
            if af.searching() {
                return Some(t);
            }
        }
        None
    }

    #[test]
    fn the_watchdog_searches_after_the_hold_of_soft_still_va() {
        let (mut af, mut l) = searched(600);
        let reference = af.report().reference.unwrap();
        let start = Duration::from_secs(10);
        let fired = watch(&mut af, &mut l, start, 60, |t| {
            reference * 40 / 100 + (t % 3) * 100
        });
        assert_eq!(fired, Some(30));
        assert_eq!(af.report().history.len(), 31);
        finish(&mut af, &mut l, start + Duration::from_secs(31));
        assert_eq!(af.report().trigger, Some(Trigger::Watchdog));
    }

    #[test]
    fn the_watchdog_ignores_soft_va_that_keeps_changing() {
        let (mut af, mut l) = searched(600);
        let reference = af.report().reference.unwrap();
        // someone walking: VA swings 30..50 % of the reference
        let swings = |t: u64| reference * [30, 50, 35, 45, 30, 50][(t % 6) as usize] / 100;
        assert_eq!(
            watch(&mut af, &mut l, Duration::from_secs(10), 300, swings),
            None
        );
    }

    #[test]
    fn the_watchdog_ignores_va_above_the_drop() {
        let (mut af, mut l) = searched(600);
        let reference = af.report().reference.unwrap();
        assert_eq!(
            watch(&mut af, &mut l, Duration::from_secs(10), 120, |_| {
                reference * 70 / 100
            }),
            None
        );
    }

    #[test]
    fn the_watchdog_searches_at_most_once_in_ten_minutes() {
        let (mut af, mut l) = searched(600);
        let reference = af.report().reference.unwrap();
        let soft = |_| reference * 40 / 100;
        let start = Duration::from_secs(10);
        assert_eq!(watch(&mut af, &mut l, start, 60, soft), Some(30));
        let first = start + Duration::from_secs(30);
        let after = finish(&mut af, &mut l, first);
        // the search found the same reference; the scene stays soft
        let later = watch(&mut af, &mut l, after + Duration::from_secs(1), 900, soft);
        let fired_at = after + Duration::from_secs(1 + later.expect("fires again eventually"));
        assert!(
            fired_at - first >= WATCHDOG_RATE,
            "{:?} after the first",
            fired_at - first
        );
    }

    #[test]
    fn no_watchdog_without_a_reference() {
        let mut l = lens((700, 900), None);
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Zoom, Duration::ZERO);
        let end = finish(&mut af, &mut l, Duration::ZERO);
        assert_eq!(
            watch(&mut af, &mut l, end + Duration::from_secs(1), 120, |_| 10),
            None
        );
    }

    #[test]
    fn the_first_day_night_decision_stays_part_of_startup() {
        let mut af = Autofocus::new(AfConfig::default());
        af.request(Trigger::Startup, Duration::ZERO);
        af.request(Trigger::DayNight, Duration::from_secs(1));
        let mut l = lens((500, 700), Some(600));
        finish(&mut af, &mut l, Duration::ZERO);
        assert_eq!(af.report().trigger, Some(Trigger::Startup));
    }

    #[test]
    fn a_day_night_switch_clears_the_reference() {
        let (mut af, _) = searched(600);
        af.request(Trigger::DayNight, Duration::from_secs(1));
        assert_eq!(af.report().reference, None);
    }

    #[test]
    fn disabled_only_the_button_searches() {
        let mut l = lens((500, 700), Some(600));
        let mut af = Autofocus::new(AfConfig {
            enabled: false,
            ..AfConfig::default()
        });
        for t in [
            Trigger::Startup,
            Trigger::Zoom,
            Trigger::Move,
            Trigger::DayNight,
            Trigger::Watchdog,
        ] {
            af.request(t, Duration::ZERO);
            assert!(!af.searching(), "{t:?}");
        }
        af.request(Trigger::Button, Duration::ZERO);
        let end = finish(&mut af, &mut l, Duration::ZERO);
        assert_eq!(af.state(), &AfState::Focused(603));
        let reference = af.report().reference.unwrap();
        let reads = l.isp().va_reads;
        assert_eq!(
            watch(&mut af, &mut l, end + Duration::from_secs(1), 120, |_| {
                reference / 10
            }),
            None
        );
        assert_eq!(l.isp().va_reads, reads, "no watchdog reads");
    }

    #[test]
    fn the_blocking_run_is_the_same_search() {
        let mut l = lens((500, 700), Some(600));
        let r = run_blocking(&mut l, &zf(), &park).unwrap();
        assert_eq!(l.isp().focus, 603);
        assert_eq!(best(&r.samples).unwrap().0, 602);
        assert!(l.isp().slept() > Duration::ZERO);
        assert!(l.isp().sets().iter().all(|s| s.item == item::FOCUS));
    }
}
