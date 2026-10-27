//! Fixed-lens image pointing. Domain time and confirmed motor positions are injected.
//! The optional loader is the only I/O seam; there are no optical defaults.
use crate::board::Position;
use camera_transport::position3d::{Operation, Request};
use serde::Deserialize;
use std::collections::VecDeque;
use std::io::{self, Read};
use std::time::Duration;

pub const CALIBRATION_PATH: &str = "/mnt/data/mi360/position3d.json";
pub const MAX_COMMAND_AGE: Duration = Duration::from_secs(2);
const MAX_HISTORY: usize = 512;
const ASPECT: f64 = 9.0 / 16.0;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Calibration {
    pub pan_a: f64,
    pub pan_b: f64,
    pub tilt_a: f64,
    pub tilt_b: f64,
    pub pan_right_grows: bool,
    pub tilt_up_grows: bool,
    #[serde(default = "default_delay")]
    pub display_delay_ms: u32,
}
fn default_delay() -> u32 {
    500
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Axes {
    /// Physical counter sign for screen right and up; opposite arrows negate it.
    pub right: i32,
    pub up: i32,
}
fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
impl Calibration {
    pub fn validate(&self) -> io::Result<()> {
        if self.display_delay_ms > 2000 {
            return Err(invalid("display delay exceeds 2000ms"));
        }
        for (a, b, r) in [
            (self.pan_a, self.pan_b, 0.5),
            (self.tilt_a, self.tilt_b, 0.5 * ASPECT),
        ] {
            // The derivative is a+3*b*x²; its extrema on [-r,r] are at 0 and r.
            let edge = a * r + b * r * r * r;
            if !a.is_finite()
                || !b.is_finite()
                || a <= 0.0
                || !(a + 3.0 * b * r * r).is_finite()
                || a + 3.0 * b * r * r <= 0.0
                || !edge.is_finite()
                || edge <= 0.0
                || edge > f64::from(i32::MAX)
            {
                return Err(invalid(
                    "calibration must be finite, bounded and strictly increasing",
                ));
            }
        }
        Ok(())
    }
    pub fn screen_axes(&self, logical_mask: u8) -> io::Result<Axes> {
        self.validate()?;
        if logical_mask & !3 != 0 {
            return Err(invalid("invalid logical orientation mask"));
        }
        Ok(Axes {
            right: if self.pan_right_grows ^ (logical_mask & 1 != 0) {
                1
            } else {
                -1
            },
            up: if self.tilt_up_grows ^ (logical_mask & 2 != 0) {
                1
            } else {
                -1
            },
        })
    }
}

pub struct Pointing {
    calibration: Calibration,
    history: VecDeque<(Duration, Position)>,
}
impl Pointing {
    pub fn new(calibration: Calibration) -> io::Result<Self> {
        calibration.validate()?;
        Ok(Self {
            calibration,
            history: VecDeque::new(),
        })
    }
    pub fn screen_axes(&self, logical_mask: u8) -> io::Result<Axes> {
        self.calibration.screen_axes(logical_mask)
    }
    pub fn observe(&mut self, time: Duration, position: Position) -> io::Result<()> {
        if position.x < 0 || position.y < 0 {
            return Err(invalid("negative motor counter"));
        }
        if let Some(&(last, _)) = self.history.back() {
            if time < last {
                return Err(invalid("motor observation time moved backwards"));
            }
            if time == last {
                self.history.pop_back();
            }
        }
        self.history.push_back((time, position));
        let window =
            MAX_COMMAND_AGE + Duration::from_millis(u64::from(self.calibration.display_delay_ms));
        let cutoff = time.saturating_sub(window);
        // Retain the immediately older anchor needed to interpolate at the cutoff.
        while self.history.len() > 2 && self.history[1].0 <= cutoff {
            self.history.pop_front();
        }
        // An excessive producer rate loses old coverage, never fabricates a position.
        while self.history.len() > MAX_HISTORY {
            self.history.pop_front();
        }
        Ok(())
    }
    pub fn resolve(
        &self,
        req: Request,
        received: Duration,
        now: Duration,
        logical_mask: u8,
        travel: Position,
    ) -> io::Result<Position> {
        if !req.is_point() || req.operation != Operation::ZoomTo {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "fixed lens accepts points only",
            ));
        }
        let age = now
            .checked_sub(received)
            .ok_or_else(|| invalid("request arrival is in the future"))?;
        if age > MAX_COMMAND_AGE {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "point request expired",
            ));
        }
        if travel.x <= 0 || travel.y <= 0 {
            return Err(invalid("unqualified motor travel"));
        }
        let at = received
            .checked_sub(Duration::from_millis(u64::from(
                self.calibration.display_delay_ms,
            )))
            .ok_or_else(|| invalid("display history is not ready"))?;
        let axes = self.screen_axes(logical_mask)?;
        if self.history.back().is_some_and(|(t, _)| *t > now) {
            return Err(invalid("motor history is in the future"));
        }
        let (x, y) = self.position_at(at, travel)?;
        let dx = f64::from(req.start.0) / 255.0 - 0.5;
        let dy = (f64::from(req.start.1) / 255.0 - 0.5) * ASPECT; // Canonical y grows upward.
        let c = self.calibration;
        let shift = |base: f64, offset: f64, sign: i32, maximum: i32| {
            (base + offset * f64::from(sign))
                .round()
                .clamp(0.0, f64::from(maximum)) as i32
        };
        Ok(Position {
            x: shift(
                x,
                c.pan_a * dx + c.pan_b * dx * dx * dx,
                axes.right,
                travel.x,
            ),
            y: shift(
                y,
                c.tilt_a * dy + c.tilt_b * dy * dy * dy,
                axes.up,
                travel.y,
            ),
        })
    }
    fn position_at(&self, at: Duration, travel: Position) -> io::Result<(f64, f64)> {
        let valid = |p: Position| p.x <= travel.x && p.y <= travel.y;
        let mut previous = None;
        for &(time, position) in &self.history {
            if time == at {
                if !valid(position) {
                    return Err(invalid("motor history exceeds travel"));
                }
                return Ok((f64::from(position.x), f64::from(position.y)));
            }
            if time > at {
                if let Some((before, p)) = previous {
                    if !valid(p) || !valid(position) {
                        return Err(invalid("motor history exceeds travel"));
                    }
                    let fraction = (at - before).as_secs_f64() / (time - before).as_secs_f64();
                    return Ok((
                        f64::from(p.x) + fraction * f64::from(position.x - p.x),
                        f64::from(p.y) + fraction * f64::from(position.y - p.y),
                    ));
                }
                break;
            }
            previous = Some((time, position));
        }
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "no motor history brackets displayed frame",
        ))
    }
}

/// Strict read-only configuration adapter. Missing calibration disables point commands.
pub fn load(path: &std::path::Path) -> io::Result<Option<Calibration>> {
    load_owned(path, 0)
}
fn load_owned(path: &std::path::Path, uid: u32) -> io::Result<Option<Calibration>> {
    use std::ffi::CString;
    use std::fs::{File, OpenOptions};
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::Component;
    fn openat(dir: &File, name: &std::ffi::OsStr, flags: i32) -> io::Result<File> {
        let name =
            CString::new(name.as_bytes()).map_err(|_| invalid("invalid calibration path"))?;
        // SAFETY: directory descriptor and NUL-terminated path remain valid; successful fd is owned once.
        let fd = unsafe { libc::openat(dir.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { File::from_raw_fd(fd) })
        }
    }
    let read = || -> io::Result<Calibration> {
        if !path.is_absolute() {
            return Err(invalid("calibration path must be absolute"));
        }
        let name = path
            .file_name()
            .ok_or_else(|| invalid("calibration needs a filename"))?;
        let mut dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/")?;
        for component in path
            .parent()
            .ok_or_else(|| invalid("calibration needs a parent"))?
            .components()
        {
            match component {
                Component::RootDir => {}
                Component::Normal(name) => {
                    dir = openat(
                        &dir,
                        name,
                        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    )?
                }
                _ => {
                    return Err(invalid(
                        "calibration path cannot contain dot/parent components",
                    ));
                }
            }
        }
        let parent = dir.metadata()?;
        if !parent.is_dir() || parent.uid() != uid || parent.mode() & 0o7777 != 0o700 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "calibration parent must be root-owned0700",
            ));
        }
        let file = openat(
            &dir,
            name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
        )?;
        let check = |m: &std::fs::Metadata| -> io::Result<()> {
            if !m.is_file()
                || m.uid() != uid
                || m.mode() & 0o7777 != 0o600
                || m.nlink() != 1
                || m.len() > 4096
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "calibration must be root-owned0600, regular, one-link and at most4KiB",
                ));
            }
            Ok(())
        };
        let before = file.metadata()?;
        check(&before)?;
        let mut bytes = Vec::new();
        (&file).take(4097).read_to_end(&mut bytes)?;
        let after = file.metadata()?;
        check(&after)?;
        if bytes.len() > 4096
            || before.len() != bytes.len() as u64
            || before.len() != after.len()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
            || before.ctime() != after.ctime()
            || before.ctime_nsec() != after.ctime_nsec()
        {
            return Err(invalid("calibration changed while reading"));
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Document {
            version: u32,
            pan_a: f64,
            pan_b: f64,
            tilt_a: f64,
            tilt_b: f64,
            pan_right_grows: bool,
            tilt_up_grows: bool,
            #[serde(default = "default_delay")]
            display_delay_ms: u32,
        }
        let d: Document = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("invalid calibration JSON/schema"))?;
        if d.version != 1 {
            return Err(invalid("unsupported calibration version"));
        }
        let calibration = Calibration {
            pan_a: d.pan_a,
            pan_b: d.pan_b,
            tilt_a: d.tilt_a,
            tilt_b: d.tilt_b,
            pan_right_grows: d.pan_right_grows,
            tilt_up_grows: d.tilt_up_grows,
            display_delay_ms: d.display_delay_ms,
        };
        calibration.validate()?;
        Ok(calibration)
    };
    match read() {
        Ok(c) => Ok(Some(c)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    // Synthetic coefficients/positions are confined to isolated domain tests.
    fn calibration() -> Calibration {
        Calibration {
            pan_a: 1000.0,
            pan_b: 0.0,
            tilt_a: 1000.0,
            tilt_b: 0.0,
            pan_right_grows: true,
            tilt_up_grows: true,
            display_delay_ms: 500,
        }
    }
    fn point(x: u8, y: u8) -> Request {
        Request {
            start: (x, y),
            end: (x, y),
            operation: Operation::ZoomTo,
        }
    }
    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }
    const TRAVEL: Position = Position { x: 2000, y: 2000 };
    fn moving() -> Pointing {
        let mut p = Pointing::new(calibration()).unwrap();
        p.observe(ms(0), Position { x: 500, y: 500 }).unwrap();
        p.observe(ms(1000), Position { x: 700, y: 600 }).unwrap();
        p.observe(ms(2000), Position { x: 900, y: 700 }).unwrap();
        p
    }
    #[test]
    fn targets_displayed_position_at_arrival_not_execution() {
        let p = moving();
        assert_eq!(
            p.resolve(point(255, 255), ms(1250), ms(2000), 0, TRAVEL)
                .unwrap(),
            Position { x: 1150, y: 856 }
        );
    }
    #[test]
    fn canonical_y_is_bottom_and_each_orientation_changes_only_its_axis() {
        let p = moving();
        let expected = [
            Position { x: 1200, y: 881 },
            Position { x: 200, y: 881 },
            Position { x: 1200, y: 319 },
            Position { x: 200, y: 319 },
        ];
        for (mask, want) in expected.into_iter().enumerate() {
            assert_eq!(
                p.resolve(point(255, 255), ms(1500), ms(2000), mask as u8, TRAVEL)
                    .unwrap(),
                want
            );
        }
        assert_eq!(
            p.resolve(point(0, 0), ms(1500), ms(2000), 0, TRAVEL)
                .unwrap(),
            Position { x: 200, y: 319 }
        );
    }
    #[test]
    fn calibrated_base_axis_directions_are_xored_with_logical_flags() {
        let mut c = calibration();
        c.pan_right_grows = false;
        assert_eq!(c.screen_axes(0).unwrap(), Axes { right: -1, up: 1 });
        assert_eq!(c.screen_axes(3).unwrap(), Axes { right: 1, up: -1 });
        assert!(c.screen_axes(4).is_err());
        c.tilt_up_grows = false;
        assert_eq!(c.screen_axes(0).unwrap(), Axes { right: -1, up: -1 });
    }
    #[test]
    fn nonlinear_model_uses_separate_pan_and_tilt_coefficients() {
        let mut c = calibration();
        c.pan_b = 400.0;
        c.tilt_a = 800.0;
        c.tilt_b = 300.0;
        let mut p = Pointing::new(c).unwrap();
        p.observe(ms(1000), Position { x: 1000, y: 1000 }).unwrap();
        assert_eq!(
            p.resolve(point(255, 255), ms(1500), ms(1500), 0, TRAVEL)
                .unwrap(),
            Position { x: 1550, y: 1232 }
        );
    }
    #[test]
    fn only_final_endpoint_clamps_to_qualified_travel() {
        let mut p = Pointing::new(calibration()).unwrap();
        p.observe(ms(1000), Position { x: 1900, y: 100 }).unwrap();
        assert_eq!(
            p.resolve(point(255, 0), ms(1500), ms(1500), 0, TRAVEL)
                .unwrap(),
            Position { x: 2000, y: 0 }
        );
        assert!(
            p.resolve(
                point(255, 0),
                ms(1500),
                ms(1500),
                0,
                Position { x: 1800, y: 2000 }
            )
            .is_err()
        );
        assert!(
            p.resolve(
                point(255, 0),
                ms(1500),
                ms(1500),
                0,
                Position { x: 0, y: 2000 }
            )
            .is_err()
        );
    }
    #[test]
    fn box_and_zoomout_never_resolve_even_with_valid_history() {
        let p = moving();
        let mut r = point(255, 255);
        r.end = (254, 255);
        assert_eq!(
            p.resolve(r, ms(1500), ms(2000), 0, TRAVEL)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
        r = point(255, 255);
        r.operation = Operation::ZoomOut;
        assert_eq!(
            p.resolve(r, ms(1500), ms(2000), 0, TRAVEL)
                .unwrap_err()
                .kind(),
            io::ErrorKind::Unsupported
        );
    }
    #[test]
    fn stale_future_and_unbracketed_requests_fail_closed() {
        let p = moving();
        assert!(
            p.resolve(point(255, 255), ms(1500), ms(3500), 0, TRAVEL)
                .is_ok()
        );
        assert_eq!(
            p.resolve(point(255, 255), ms(1500), ms(3501), 0, TRAVEL)
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        assert!(
            p.resolve(point(255, 255), ms(2500), ms(2400), 0, TRAVEL)
                .is_err()
        );
        assert!(
            p.resolve(point(255, 255), ms(1500), ms(1900), 0, TRAVEL)
                .is_err()
        );
        assert!(
            p.resolve(point(255, 255), ms(400), ms(2000), 0, TRAVEL)
                .is_err()
        );
        assert!(
            p.resolve(point(255, 255), ms(2600), ms(2600), 0, TRAVEL)
                .is_err()
        );
        assert!(
            Pointing::new(calibration())
                .unwrap()
                .resolve(point(255, 255), ms(1500), ms(1500), 0, TRAVEL)
                .is_err()
        );
    }
    #[test]
    fn observation_order_is_checked_and_equal_time_replaces_position() {
        let mut p = Pointing::new(calibration()).unwrap();
        p.observe(ms(1000), Position { x: 400, y: 400 }).unwrap();
        assert!(p.observe(ms(999), Position { x: 900, y: 900 }).is_err());
        assert!(p.observe(ms(1001), Position { x: -1, y: 0 }).is_err());
        p.observe(ms(1000), Position { x: 600, y: 600 }).unwrap();
        assert_eq!(p.history.len(), 1);
        assert_eq!(
            p.resolve(point(255, 255), ms(1500), ms(1500), 0, TRAVEL)
                .unwrap(),
            Position { x: 1100, y: 881 }
        );
    }
    #[test]
    fn pruning_keeps_the_anchor_before_the_oldest_valid_frame() {
        let mut p = Pointing::new(calibration()).unwrap();
        for t in 0..=7 {
            p.observe(ms(t * 700), Position { x: 500, y: 500 }).unwrap();
        }
        // Last observation4900; oldest accepted arrival2900 displays2400.
        assert_eq!(p.history.front().unwrap().0, ms(2100));
        assert!(
            p.resolve(point(255, 255), ms(2900), ms(4900), 0, TRAVEL)
                .is_ok()
        );
        assert!(p.history.len() < 8);
    }
    #[test]
    fn excessive_observation_rate_is_bounded_and_lost_coverage_is_refused() {
        let mut c = calibration();
        c.display_delay_ms = 0;
        let mut p = Pointing::new(c).unwrap();
        for t in 0..1024 {
            p.observe(Duration::from_micros(t), Position { x: 500, y: 500 })
                .unwrap();
        }
        assert_eq!(p.history.len(), MAX_HISTORY);
        assert!(
            p.resolve(point(255, 255), Duration::ZERO, ms(1), 0, TRAVEL)
                .is_err()
        );
    }
    #[test]
    fn invalid_and_nonmonotonic_models_are_rejected_before_history() {
        for (a, b) in [
            (0.0, 0.0),
            (-1.0, 0.0),
            (f64::NAN, 0.0),
            (1.0, f64::INFINITY),
            (1000.0, -2000.0),
            (1e30, 0.0),
        ] {
            let mut c = calibration();
            c.pan_a = a;
            c.pan_b = b;
            assert!(Pointing::new(c).is_err(), "{a} {b}");
        }
        let mut c = calibration();
        c.tilt_b = -10000.0;
        assert!(c.validate().is_err());
        c = calibration();
        c.display_delay_ms = 2000;
        assert!(c.validate().is_ok());
        c.display_delay_ms = 2001;
        assert!(c.validate().is_err());
        c = calibration();
        c.pan_b = -1000.0;
        assert!(c.validate().is_ok());
    }
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let p = std::env::temp_dir().join(format!(
                "mi360-pointing-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
            Self(fs::canonicalize(p).unwrap())
        }
        fn path(&self) -> PathBuf {
            self.0.join("position3d.json")
        }
        fn write(&self, body: &str) {
            fs::write(self.path(), body).unwrap();
            fs::set_permissions(self.path(), fs::Permissions::from_mode(0o600)).unwrap();
        }
        fn uid(&self) -> u32 {
            fs::metadata(&self.0).unwrap().uid()
        }
        fn load(&self) -> io::Result<Option<Calibration>> {
            load_owned(&self.path(), self.uid())
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    const DOCUMENT: &str = r#"{"version":1,"pan_a":1000,"pan_b":0,"tilt_a":1000,"tilt_b":0,"pan_right_grows":true,"tilt_up_grows":false}"#;
    #[test]
    fn loader_missing_disables_and_valid_document_defaults_only_delay() {
        let s = Scratch::new();
        assert_eq!(s.load().unwrap(), None);
        s.write(DOCUMENT);
        let c = s.load().unwrap().unwrap();
        assert_eq!(c.display_delay_ms, 500);
        assert!(!c.tilt_up_grows);
        assert_eq!(
            load_owned(&s.0.join("missing/file"), s.uid()).unwrap(),
            None
        );
    }
    #[test]
    fn loader_strict_schema_rejects_incomplete_unknown_duplicate_and_invalid_fields() {
        let s = Scratch::new();
        for body in [
            "{}",
            "[]",
            "null",
            &DOCUMENT.replace("\"version\":1", "\"version\":2"),
            &DOCUMENT.replace("\"pan_b\":0,", ""),
            &DOCUMENT.replace("\"pan_a\":1000", "\"pan_a\":null"),
            &DOCUMENT.replace("\"pan_a\":1000", "\"pan_a\":1000,\"pan_a\":1000"),
            &DOCUMENT.replace("\"pan_a\":1000", "\"pan_a\":1000,\"mystery\":0"),
            &DOCUMENT.replace("\"pan_a\":1000", "\"pan_a\":1000,\"display_delay_ms\":2001"),
        ] {
            s.write(body);
            assert!(s.load().is_err(), "{body}");
        }
    }
    #[test]
    fn loader_refuses_permissions_owner_hardlinks_and_size() {
        let s = Scratch::new();
        s.write(DOCUMENT);
        fs::set_permissions(s.path(), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(s.load().is_err());
        fs::set_permissions(s.path(), fs::Permissions::from_mode(0o600)).unwrap();
        assert!(load_owned(&s.path(), s.uid().wrapping_add(1)).is_err());
        fs::hard_link(s.path(), s.0.join("other")).unwrap();
        assert!(s.load().is_err());
        fs::remove_file(s.0.join("other")).unwrap();
        s.write(&" ".repeat(4097));
        assert!(s.load().is_err());
        s.write(DOCUMENT);
        fs::set_permissions(&s.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(s.load().is_err());
    }
    #[test]
    fn loader_never_follows_file_or_ancestor_symlinks() {
        let s = Scratch::new();
        s.write(DOCUMENT);
        let alias = s.0.join("alias");
        symlink(s.path(), &alias).unwrap();
        assert!(load_owned(&alias, s.uid()).is_err());
        let d = Scratch::new();
        symlink(&s.0, d.0.join("directory")).unwrap();
        assert!(load_owned(&d.0.join("directory/position3d.json"), s.uid()).is_err());
        assert!(load_owned(Path::new("position3d.json"), s.uid()).is_err());
        assert!(load_owned(&s.0.join("../position3d.json"), s.uid()).is_err());
    }
}
