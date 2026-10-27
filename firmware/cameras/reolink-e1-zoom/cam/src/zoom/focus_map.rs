//! The learned focus map: where every successful search found its sharp focus, by pan, tilt and zoom.
//!
//! The ZFTable in the `sp` partition is calibrated for a far scene; a room is nearer, so its sharp focus lies
//! off that table, at mid and tele zoom even outside the ±64 windows the vendor gives the driver. The map learns
//! the real scene instead: every `Focused` search adds a point, the park point after a zoom or a pan/tilt is
//! predicted from the nearest known points (the calibrated table is the fallback), and the driver's focus-curve
//! rows are derived from the table widened around the learned points. Nothing here writes the `sp` partition.
//!
//! Persistence goes through the same [`Disk`] seam as the settings: one file, written at most once per
//! [`SAVE_WITHIN`] after the first unsaved change, never in a command's path.

use std::io;
use std::time::Duration;

use super::zf_table::{self, ZfTable};
use super::{FOCUS_MAX, Row};
use crate::settings::Disk;

/// Points farther than this (pan-equivalent steps) do not predict a park point.
pub const RADIUS: u32 = 350;
/// Candidates within this of the nearest one form the site the prediction interpolates in.
const SITE_SPREAD: u32 = 100;
/// A new point replaces one within this angle and this zoom of it.
const MERGE_ANGLE: u32 = 100;
const MERGE_ZOOM: u32 = 32;
/// Two site points this far apart in zoom still interpolate; one alone predicts within half of it.
const INTERPOLATE_ZOOM: u32 = 512;
const SINGLE_ZOOM: u32 = 256;
/// The map keeps this many points; the oldest learned goes first.
pub const MAX_POINTS: usize = 256;
/// The driver's window below and above each calibrated focus, before any point is learned.
pub const WIDE_BELOW: u32 = 300;
pub const WIDE_ABOVE: u32 = 300;
/// A learned point within this zoom of a row widens the row by this margin around it.
const ROW_REACH: u32 = 64;
const LEARNED_MARGIN: u32 = 32;
/// One pan step and this many hundredths of a tilt step move the picture the same amount.
const TILT_PER_PAN_PCT: u32 = 91;
/// The file is written this long after the first unsaved change, at most.
pub const SAVE_WITHIN: Duration = Duration::from_secs(60);
/// Larger files are refused as corrupt.
const MAX_FILE: usize = 16 * 1024;

/// Where the lens points and how far it is zoomed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pose {
    pub pan: u32,
    pub tilt: u32,
    pub zoom: u32,
}

/// One learned point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub pan: u32,
    pub tilt: u32,
    pub zoom: u32,
    pub focus: u32,
    pub va: u64,
    /// Learning order; the camera clock is not set, so no wall time.
    pub seq: u64,
}

/// Where a park point came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Factory,
    Learned { distance: u32 },
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Factory => "factory",
            Self::Learned { .. } => "learned",
        }
    }
}

/// The focus to park at for a pose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Park {
    pub focus: u32,
    pub source: Source,
}

/// Angular distance in pan-equivalent steps (the pan has end stops, so no wrap).
pub fn distance(a: (u32, u32), b: (u32, u32)) -> u32 {
    let pan = a.0.abs_diff(b.0);
    let tilt = a.1.abs_diff(b.1) * 100 / TILT_PER_PAN_PCT;
    pan.max(tilt)
}

pub struct FocusMap {
    disk: Box<dyn Disk>,
    points: Vec<Point>,
    seq: u64,
    dirty_since: Option<Duration>,
    retry_at: Option<Duration>,
}

impl FocusMap {
    /// An empty map on `disk`.
    pub fn new(disk: Box<dyn Disk>) -> Self {
        Self {
            disk,
            points: Vec::new(),
            seq: 0,
            dirty_since: None,
            retry_at: None,
        }
    }

    /// Reads the map from `disk`; an absent, unreadable or invalid file is an empty map and one problem.
    pub fn load(mut disk: Box<dyn Disk>) -> (Self, Option<String>) {
        let (points, problem) = match disk.read() {
            Ok(None) => (Vec::new(), None),
            Ok(Some(text)) => match parse(&text) {
                Ok(points) => (points, None),
                Err(e) => (Vec::new(), Some(format!("focus map ignored: {e}"))),
            },
            Err(e) => (Vec::new(), Some(format!("focus map unreadable: {e}"))),
        };
        let seq = points.iter().map(|p| p.seq).max().unwrap_or(0);
        (
            Self {
                disk,
                points,
                seq,
                dirty_since: None,
                retry_at: None,
            },
            problem,
        )
    }

    pub fn points(&self) -> usize {
        self.points.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Point> {
        self.points.iter()
    }

    /// A `Focused` search ended at `focus` with `va` at `at`: replaces the point within the merge distance, else
    /// adds one; beyond [`MAX_POINTS`] the oldest goes.
    pub fn learn(&mut self, at: Pose, focus: u32, va: u64, now: Duration) {
        self.seq += 1;
        let point = Point {
            pan: at.pan,
            tilt: at.tilt,
            zoom: at.zoom,
            focus,
            va,
            seq: self.seq,
        };
        let near = self.points.iter().position(|p| {
            distance((p.pan, p.tilt), (at.pan, at.tilt)) <= MERGE_ANGLE
                && p.zoom.abs_diff(at.zoom) <= MERGE_ZOOM
        });
        match near {
            Some(i) => self.points[i] = point,
            None => self.points.push(point),
        }
        while self.points.len() > MAX_POINTS {
            let oldest = self
                .points
                .iter()
                .enumerate()
                .min_by_key(|(_, p)| p.seq)
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.points.remove(oldest);
        }
        self.dirty_since.get_or_insert(now);
    }

    /// The learned offset from the calibrated focus at `zoom` for a view at `angles`, and the distance of the
    /// nearest point used; `None` when no point within [`RADIUS`] predicts this zoom.
    pub fn offset(&self, angles: (u32, u32), zoom: u32, zf: &ZfTable) -> Option<(i64, u32)> {
        let candidates: Vec<(&Point, u32)> = self
            .points
            .iter()
            .map(|p| (p, distance((p.pan, p.tilt), angles)))
            .filter(|(_, d)| *d <= RADIUS)
            .collect();
        let nearest = candidates.iter().map(|c| c.1).min()?;
        let site: Vec<(&Point, u32)> = candidates
            .iter()
            .filter(|c| c.1 <= nearest + SITE_SPREAD)
            .copied()
            .collect();
        let delta = |p: &Point| i64::from(p.focus) - i64::from(zf.focus_at(p.zoom));
        // the closest zoom on each side; at equal zoom the nearer view
        let below = site
            .iter()
            .filter(|(p, _)| p.zoom <= zoom)
            .min_by_key(|(p, d)| (zoom - p.zoom, *d))
            .map(|(p, _)| *p);
        let above = site
            .iter()
            .filter(|(p, _)| p.zoom >= zoom)
            .min_by_key(|(p, d)| (p.zoom - zoom, *d))
            .map(|(p, _)| *p);
        let d = match (below, above) {
            (Some(b), Some(a)) if a.zoom == b.zoom => delta(b),
            (Some(b), Some(a)) if a.zoom - b.zoom <= INTERPOLATE_ZOOM => {
                let span = i64::from(a.zoom - b.zoom);
                let t = i64::from(zoom - b.zoom);
                delta(b) + (delta(a) - delta(b)) * t / span
            }
            (Some(b), _) if zoom - b.zoom <= SINGLE_ZOOM => delta(b),
            (_, Some(a)) if a.zoom - zoom <= SINGLE_ZOOM => delta(a),
            _ => return None,
        };
        Some((d, nearest))
    }

    /// The focus to park at for `zoom` when the lens points at `angles` (unknown: the calibrated focus), inside
    /// the driver window at that zoom.
    pub fn park(&self, angles: Option<(u32, u32)>, zoom: u32, zf: &ZfTable) -> Park {
        let factory = zf.focus_at(zoom);
        let (lo, hi) = self.window_at(zoom, zf);
        match angles.and_then(|a| self.offset(a, zoom, zf)) {
            Some((delta, distance)) => Park {
                focus: (i64::from(factory) + delta).clamp(i64::from(lo), i64::from(hi)) as u32,
                source: Source::Learned { distance },
            },
            None => Park {
                focus: factory.clamp(lo, hi),
                source: Source::Factory,
            },
        }
    }

    /// The driver's focus-curve rows: the calibrated points with a wide window, widened further around learned
    /// points within [`ROW_REACH`] zoom steps, clamped to travel. Refuses a table the driver cannot take.
    pub fn rows(&self, zf: &ZfTable) -> Result<Vec<Row>, zf_table::Error> {
        zf.rows_with(|_, zoom| {
            let sharp = zf.focus_at(zoom);
            let mut below = WIDE_BELOW.min(sharp);
            let mut above = WIDE_ABOVE.min(FOCUS_MAX - sharp);
            for p in self
                .points
                .iter()
                .filter(|p| p.zoom.abs_diff(zoom) <= ROW_REACH)
            {
                let lo = p.focus.saturating_sub(LEARNED_MARGIN);
                let hi = p.focus.saturating_add(LEARNED_MARGIN).min(FOCUS_MAX);
                if lo < sharp {
                    below = below.max(sharp - lo);
                }
                if hi > sharp {
                    above = above.max(hi - sharp);
                }
            }
            (below, above)
        })
    }

    /// The window the installed rows give the driver at `zoom`: the row at or above it (where a zoom lands).
    pub fn window_at(&self, zoom: u32, zf: &ZfTable) -> (u32, u32) {
        let rows = self.rows(zf).unwrap_or_default();
        rows.iter()
            .find(|r| r.zoom >= zoom)
            .or(rows.last())
            .map_or((0, FOCUS_MAX), |r| (r.focus_min, r.focus_max))
    }

    /// Writes the file once the save window since the first unsaved change has passed; a failed write is
    /// retried after another window and reported once.
    pub fn tick(&mut self, now: Duration) -> io::Result<()> {
        let Some(since) = self.dirty_since else {
            return Ok(());
        };
        let due = self.retry_at.unwrap_or(since + SAVE_WITHIN);
        if now < due {
            return Ok(());
        }
        match self.write() {
            Ok(()) => {
                self.dirty_since = None;
                self.retry_at = None;
                Ok(())
            }
            Err(e) => {
                self.retry_at = Some(now + SAVE_WITHIN);
                Err(e)
            }
        }
    }

    /// Writes now if anything is unsaved.
    pub fn flush(&mut self) -> io::Result<()> {
        if self.dirty_since.is_none() {
            return Ok(());
        }
        self.write()?;
        self.dirty_since = None;
        self.retry_at = None;
        Ok(())
    }

    pub fn unsaved(&self) -> bool {
        self.dirty_since.is_some()
    }

    fn write(&mut self) -> io::Result<()> {
        let text = render(&self.points);
        self.disk.write_temp(text.as_bytes())?;
        self.disk.sync_temp()?;
        self.disk.rename_temp()?;
        self.disk.sync_dir()
    }
}

/// `points = [[pan, tilt, zoom, focus, va, seq], …]`, one point per line.
pub fn render(points: &[Point]) -> String {
    let mut s = String::from("# learned focus map: pan, tilt, zoom, focus, va, seq\npoints = [\n");
    for p in points {
        s.push_str(&format!(
            "  [{}, {}, {}, {}, {}, {}],\n",
            p.pan, p.tilt, p.zoom, p.focus, p.va, p.seq
        ));
    }
    s.push_str("]\n");
    s
}

/// Parses [`render`]'s format; points outside travel are dropped, anything else wrong is an error.
pub fn parse(text: &str) -> Result<Vec<Point>, String> {
    if text.len() > MAX_FILE {
        return Err(format!("{} bytes, over the {MAX_FILE} limit", text.len()));
    }
    let table: toml::Table = toml::from_str(text).map_err(|e| e.to_string())?;
    let rows = table
        .get("points")
        .and_then(|v| v.as_array())
        .ok_or("no points array")?;
    let mut points = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let v: Vec<i64> = row
            .as_array()
            .ok_or(format!("point {i} is not an array"))?
            .iter()
            .map(|x| x.as_integer().ok_or(format!("point {i} has a non-integer")))
            .collect::<Result<_, _>>()?;
        let [pan, tilt, zoom, focus, va, seq] = v[..] else {
            return Err(format!("point {i} does not have six values"));
        };
        let in_travel = (0..=i64::from(crate::ptz::PAN_MAX)).contains(&pan)
            && (0..=i64::from(crate::ptz::TILT_MAX)).contains(&tilt)
            && (0..=i64::from(super::ZOOM_MAX)).contains(&zoom)
            && (0..=i64::from(FOCUS_MAX)).contains(&focus)
            && va >= 0
            && seq >= 0;
        if !in_travel {
            continue;
        }
        points.push(Point {
            pan: pan as u32,
            tilt: tilt as u32,
            zoom: zoom as u32,
            focus: focus as u32,
            va: va as u64,
            seq: seq as u64,
        });
    }
    Ok(points)
}

/// The calibration grid: cell centres over the pan travel, three tilt rows, five zooms: 120 points, about
/// 15 minutes on the camera. The camera is stationary, so a long walk is acceptable when the map is better for
/// it, as long as the duration is announced (the button's name and the sensor's remaining time do that).
pub const GRID_PANS: [u32; 8] = [460, 1380, 2300, 3220, 4140, 5060, 5980, 6900];
pub const GRID_TILTS: [u32; 3] = [250, 620, 990];
pub const GRID_ZOOMS: [u32; 5] = [0, 256, 512, 768, 1024];
/// What one point costs on the camera (goto, park, search), for the remaining-time estimate until the walk's own
/// pace is known: the first points are near the start and quick, so the nominal cost holds for [`PACE_AFTER`]
/// points.
pub const SECONDS_PER_POINT: u64 = 7;
pub const PACE_AFTER: usize = 12;

/// How the search at a calibration point ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Focused,
    Skipped,
}

/// What the walk asks the device to do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalStep {
    /// Move to this pan/tilt and park this zoom.
    Goto(Pose),
    /// Run one full search where the lens is.
    Search,
    /// Back to where the walk started, then a normal zoom search.
    Return(Pose),
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalState {
    Running,
    Done,
    Cancelled,
}

impl CalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalProgress {
    pub state: CalState,
    pub done: usize,
    pub total: usize,
    pub learned: usize,
    pub skipped: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Start,
    Moving(usize),
    Searching(usize),
    Returning,
    Finished,
}

/// The grid walk that seeds the map: serpentine over pan within each tilt row, zoom up at one angle and down at
/// the next, so the lens never crosses its whole travel between points. Pure: it knows the order and the
/// bookkeeping, nothing about hardware.
pub struct Calibration {
    start: Pose,
    points: Vec<Pose>,
    phase: Phase,
    progress: CalProgress,
}

impl Calibration {
    pub fn new(start: Pose) -> Self {
        let points = grid();
        Self {
            start,
            phase: Phase::Start,
            progress: CalProgress {
                state: CalState::Running,
                done: 0,
                total: points.len(),
                learned: 0,
                skipped: 0,
            },
            points,
        }
    }

    /// The next step; `last` is how the search the previous `Search` step asked for ended.
    pub fn next(&mut self, last: Option<Outcome>) -> CalStep {
        match self.phase {
            Phase::Start => {
                self.phase = Phase::Moving(0);
                CalStep::Goto(self.points[0])
            }
            Phase::Moving(i) => {
                self.phase = Phase::Searching(i);
                CalStep::Search
            }
            Phase::Searching(i) => {
                self.progress.done += 1;
                match last {
                    Some(Outcome::Focused) => self.progress.learned += 1,
                    _ => self.progress.skipped += 1,
                }
                if i + 1 < self.points.len() {
                    self.phase = Phase::Moving(i + 1);
                    CalStep::Goto(self.points[i + 1])
                } else {
                    self.phase = Phase::Returning;
                    CalStep::Return(self.start)
                }
            }
            Phase::Returning => {
                self.phase = Phase::Finished;
                self.progress.state = CalState::Done;
                CalStep::Done
            }
            Phase::Finished => CalStep::Done,
        }
    }

    /// A search was asked for and has not reported yet.
    pub fn searching(&self) -> bool {
        matches!(self.phase, Phase::Searching(_))
    }

    pub fn finished(&self) -> bool {
        self.phase == Phase::Finished
    }

    pub fn cancel(&mut self) {
        if self.phase != Phase::Finished {
            self.phase = Phase::Finished;
            self.progress.state = CalState::Cancelled;
        }
    }

    pub fn progress(&self) -> CalProgress {
        self.progress
    }
}

/// The grid in walking order.
pub fn grid() -> Vec<Pose> {
    let mut points = Vec::new();
    let mut angle = 0usize;
    for (row, &tilt) in GRID_TILTS.iter().enumerate() {
        let mut pans: Vec<u32> = GRID_PANS.to_vec();
        if row % 2 == 1 {
            pans.reverse();
        }
        for pan in pans {
            let mut zooms: Vec<u32> = GRID_ZOOMS.to_vec();
            if angle % 2 == 1 {
                zooms.reverse();
            }
            for zoom in zooms {
                points.push(Pose { pan, tilt, zoom });
            }
            angle += 1;
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::FakeDisk;

    /// (0, 230) .. (512, 800) .. (1024, 1022)
    fn zf() -> ZfTable {
        ZfTable::parse(b"<ZFTable>1000000,300e6,200,320,400,3fe</ZFTable>").unwrap()
    }

    fn map() -> FocusMap {
        FocusMap::new(Box::new(FakeDisk::default()))
    }

    fn pose(pan: u32, tilt: u32, zoom: u32) -> Pose {
        Pose { pan, tilt, zoom }
    }

    #[test]
    fn distance_is_the_larger_of_pan_and_scaled_tilt() {
        assert_eq!(distance((1000, 500), (1300, 500)), 300);
        assert_eq!(distance((1000, 500), (1000, 591)), 100);
        assert_eq!(distance((1000, 500), (1050, 591)), 100);
    }

    #[test]
    fn nothing_learned_parks_at_the_calibrated_focus() {
        let m = map();
        assert_eq!(
            m.park(Some((1000, 500)), 512, &zf()),
            Park {
                focus: 800,
                source: Source::Factory
            }
        );
        assert_eq!(m.park(None, 0, &zf()).focus, 230);
    }

    #[test]
    fn a_learned_point_predicts_its_own_zoom_and_offsets_nearby_zooms() {
        let mut m = map();
        m.learn(pose(1000, 500, 512), 935, 10, Duration::ZERO);
        let p = m.park(Some((1000, 500)), 512, &zf());
        assert_eq!(p.focus, 935);
        assert_eq!(p.source, Source::Learned { distance: 0 });
        // one point within 256 zoom steps carries its offset (+135)
        assert_eq!(
            m.park(Some((1000, 500)), 400, &zf()).focus,
            zf().focus_at(400) + 135
        );
        assert_eq!(
            m.park(Some((1000, 500)), 768, &zf()).focus,
            zf().focus_at(768) + 135
        );
        // beyond 256 zoom steps it predicts nothing
        assert_eq!(m.park(Some((1000, 500)), 0, &zf()).source, Source::Factory);
        assert_eq!(
            m.park(Some((1000, 500)), 1024, &zf()).source,
            Source::Factory
        );
    }

    #[test]
    fn two_site_points_interpolate_the_offset_in_zoom() {
        let mut m = map();
        m.learn(pose(1000, 500, 0), 330, 10, Duration::ZERO); // +100
        m.learn(pose(1000, 500, 512), 1000, 10, Duration::ZERO); // +200
        assert_eq!(
            m.park(Some((1000, 500)), 256, &zf()).focus,
            zf().focus_at(256) + 150
        );
        assert_eq!(m.park(Some((1000, 500)), 0, &zf()).focus, 330);
    }

    #[test]
    fn points_beyond_the_radius_do_not_count_and_the_nearest_site_wins() {
        let mut m = map();
        m.learn(pose(1000, 500, 512), 900, 10, Duration::ZERO);
        m.learn(pose(1500, 500, 512), 700, 10, Duration::ZERO);
        assert_eq!(m.park(Some((1400, 500)), 512, &zf()).focus, 700);
        assert_eq!(m.park(Some((1200, 500)), 512, &zf()).focus, 900);
        assert_eq!(
            m.park(Some((2000, 500)), 512, &zf()).source,
            Source::Factory
        );
        assert_eq!(
            m.park(Some((1000, 900)), 512, &zf()).source,
            Source::Factory
        );
    }

    #[test]
    fn a_point_in_the_site_but_at_a_far_zoom_does_not_predict_alone() {
        let mut m = map();
        m.learn(pose(1000, 500, 1024), 1100, 10, Duration::ZERO);
        assert_eq!(m.park(Some((1000, 500)), 0, &zf()).source, Source::Factory);
    }

    #[test]
    fn learning_merges_near_points_and_evicts_the_oldest() {
        let mut m = map();
        m.learn(pose(1000, 500, 512), 900, 10, Duration::ZERO);
        m.learn(pose(1050, 500, 530), 910, 11, Duration::from_secs(1));
        assert_eq!(m.points(), 1);
        assert_eq!(m.iter().next().unwrap().focus, 910);
        m.learn(pose(1000, 500, 600), 950, 12, Duration::from_secs(2));
        assert_eq!(m.points(), 2);
        for i in 0..MAX_POINTS as u32 {
            m.learn(pose(i % 60 * 120, i / 60 * 120, 0), 300, 1, Duration::ZERO);
        }
        assert_eq!(m.points(), MAX_POINTS);
        assert!(
            m.iter().all(|p| p.zoom == 0),
            "the two first points were the oldest"
        );
    }

    #[test]
    fn empty_map_rows_are_the_calibrated_points_with_the_wide_window() {
        let rows = map().rows(&zf()).unwrap();
        assert_eq!(
            rows,
            [
                Row {
                    zoom: 0,
                    focus_min: 0,
                    focus_max: 530
                },
                Row {
                    zoom: 512,
                    focus_min: 500,
                    focus_max: 1100
                },
                Row {
                    zoom: 1024,
                    focus_min: 722,
                    focus_max: 1322
                }
            ]
        );
        assert_eq!(map().window_at(300, &zf()), (500, 1100));
        assert_eq!(map().window_at(1100, &zf()), (722, 1322));
    }

    #[test]
    fn learned_points_widen_the_rows_near_their_zoom_and_clamp_to_travel() {
        let mut m = map();
        m.learn(pose(0, 0, 1000), 1400, 1, Duration::ZERO);
        m.learn(pose(0, 0, 40), 0, 1, Duration::ZERO);
        let rows = m.rows(&zf()).unwrap();
        assert_eq!((rows[2].focus_min, rows[2].focus_max), (722, 1432));
        assert_eq!((rows[0].focus_min, rows[0].focus_max), (0, 530));
        assert_eq!(
            (rows[1].focus_min, rows[1].focus_max),
            (500, 1100),
            "40 is beyond reach of 512"
        );
        m.learn(pose(0, 0, 1024), FOCUS_MAX, 1, Duration::ZERO);
        assert_eq!(m.rows(&zf()).unwrap()[2].focus_max, FOCUS_MAX);
    }

    #[test]
    fn the_file_round_trips_and_bad_files_are_ignored() {
        let mut m = map();
        m.learn(pose(1000, 500, 512), 935, 26_640_082, Duration::ZERO);
        m.learn(pose(6195, 650, 1024), 1124, 51_766_021, Duration::ZERO);
        let text = render(m.iter().copied().collect::<Vec<_>>().as_slice());
        assert_eq!(parse(&text).unwrap().len(), 2);
        assert!(parse("points = 5").is_err());
        assert!(parse("points = [[1, 2]]").is_err());
        assert!(parse("not toml at all [[").is_err());
        assert!(parse(&"x".repeat(MAX_FILE + 1)).is_err());
        // out of travel: dropped, the rest kept
        let kept = parse("points = [[99999, 0, 0, 100, 1, 1], [100, 100, 0, 100, 1, 2]]").unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].seq, 2);
        let (loaded, problem) = FocusMap::load(Box::new(FakeDisk {
            file: Some("points = [[1".into()),
            ..Default::default()
        }));
        assert_eq!(loaded.points(), 0);
        assert!(problem.unwrap().starts_with("focus map ignored"));
        let (loaded, problem) = FocusMap::load(Box::new(FakeDisk {
            file: Some(text),
            ..Default::default()
        }));
        assert_eq!((loaded.points(), problem), (2, None));
        assert_eq!(loaded.seq, 2);
    }

    #[test]
    fn the_file_is_written_once_per_save_window_and_on_flush() {
        let mut m = map();
        m.learn(pose(1000, 500, 512), 935, 1, Duration::from_secs(5));
        m.tick(Duration::from_secs(10)).unwrap();
        m.learn(pose(1000, 500, 0), 300, 1, Duration::from_secs(20));
        m.tick(Duration::from_secs(64)).unwrap();
        assert!(m.unsaved(), "the window counts from the first change");
        m.tick(Duration::from_secs(65)).unwrap();
        assert!(!m.unsaved());
        m.learn(pose(1000, 500, 0), 310, 1, Duration::from_secs(70));
        m.flush().unwrap();
        assert!(!m.unsaved());
        m.flush().unwrap();
    }

    #[test]
    fn the_grid_is_inside_travel_and_walks_a_serpentine_in_pan_and_zoom() {
        let g = grid();
        assert_eq!(g.len(), 120);
        assert!(g.iter().all(|p| p.pan <= crate::ptz::PAN_MAX
            && p.tilt <= crate::ptz::TILT_MAX
            && p.zoom <= 1024));
        // first angle zooms up, second down, pans ascend in the first row
        assert_eq!((g[0].pan, g[0].tilt, g[0].zoom), (460, 250, 0));
        assert_eq!(g[4].zoom, 1024);
        assert_eq!((g[5].pan, g[5].zoom), (1380, 1024));
        assert_eq!(g[9].zoom, 0);
        // the second row starts at the far pan
        assert_eq!((g[40].pan, g[40].tilt), (6900, 620));
        assert_eq!((g[80].pan, g[80].tilt), (460, 990));
        // consecutive points never differ by more than one pan cell and one zoom step
        for w in g.windows(2) {
            assert!(w[0].pan.abs_diff(w[1].pan) <= 920, "{:?}", w);
            assert!(w[0].zoom.abs_diff(w[1].zoom) <= 256, "{:?}", w);
        }
    }

    #[test]
    fn the_walk_alternates_goto_and_search_counts_outcomes_and_returns_to_the_start() {
        let start = pose(3000, 500, 512);
        let mut c = Calibration::new(start);
        assert_eq!(c.next(None), CalStep::Goto(grid()[0]));
        assert_eq!(c.next(None), CalStep::Search);
        assert!(c.searching());
        assert_eq!(c.next(Some(Outcome::Focused)), CalStep::Goto(grid()[1]));
        assert_eq!(c.next(None), CalStep::Search);
        assert_eq!(c.next(Some(Outcome::Skipped)), CalStep::Goto(grid()[2]));
        let p = c.progress();
        assert_eq!((p.done, p.total, p.learned, p.skipped), (2, 120, 1, 1));
        for _ in 2..120 {
            assert_eq!(c.next(None), CalStep::Search);
            c.next(Some(Outcome::Focused));
        }
        // the last outcome moved it to the return
        assert_eq!(c.progress().done, 120);
        assert!(!c.finished());
        assert_eq!(c.next(None), CalStep::Done);
        assert!(c.finished());
        assert_eq!(c.progress().state, CalState::Done);
        assert_eq!(c.next(None), CalStep::Done);
    }

    #[test]
    fn the_last_search_is_followed_by_the_return_to_the_start() {
        let start = pose(3000, 500, 512);
        let mut c = Calibration::new(start);
        let mut step = c.next(None);
        let mut returned = None;
        for _ in 0..1000 {
            step = match step {
                CalStep::Goto(_) => c.next(None),
                CalStep::Search => c.next(Some(Outcome::Focused)),
                CalStep::Return(p) => {
                    returned = Some(p);
                    c.next(None)
                }
                CalStep::Done => break,
            };
        }
        assert_eq!(returned, Some(start));
        assert_eq!(c.progress().learned, 120);
    }

    #[test]
    fn cancel_ends_the_walk_where_it_is() {
        let mut c = Calibration::new(pose(0, 0, 0));
        c.next(None);
        c.next(None);
        c.cancel();
        assert!(c.finished());
        assert_eq!(c.progress().state, CalState::Cancelled);
        assert_eq!(c.next(Some(Outcome::Focused)), CalStep::Done);
        assert_eq!(c.progress().done, 0);
        // a finished walk is not cancelled again
        let mut d = Calibration::new(pose(0, 0, 0));
        while d.next(Some(Outcome::Focused)) != CalStep::Done {}
        d.cancel();
        assert_eq!(d.progress().state, CalState::Done);
    }
}
