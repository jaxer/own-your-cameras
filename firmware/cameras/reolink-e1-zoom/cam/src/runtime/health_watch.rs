//! `cam health-watch`: the launcher-facing consumer of the health record. It wraps
//! [`HealthObserver`] with the independent child-liveness check and a launcher-owned monotonic clock,
//! and prints one line per state change so a shell launcher can `read` it:
//!
//! ```text
//! unqualified      printed first, and whenever qualification is revoked
//! qualified        180 s of continuous, advancing, current-attempt health
//! child-exited     the child is gone or a zombie; the watcher then exits 1
//! ```

use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::runtime::HealthObserver;

pub const POLL: Duration = Duration::from_secs(1);

/// False when `/proc/PID/stat` is missing or the process is a zombie (exited but not reaped).
pub fn process_alive(proc_root: &Path, pid: u32) -> bool {
    let Ok(stat) = fs::read_to_string(proc_root.join(pid.to_string()).join("stat")) else {
        return false;
    };
    stat.rsplit_once(')')
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .is_some_and(|state| state != "Z" && state != "X")
}

/// The attempt's health record, at most 1 KiB.
pub fn read_record(dir: &Path, attempt: &str) -> Option<String> {
    if !crate::runtime::valid_attempt(attempt) {
        return None;
    }
    let path = dir.join(format!("cam-health-{attempt}.json"));
    String::from_utf8(crate::files::read_regular(&path, 1024).ok()?).ok()
}

pub struct Watch {
    observer: HealthObserver,
    qualified: bool,
}

impl Watch {
    pub fn new(attempt: &str, pid: u32) -> Self {
        Self {
            observer: HealthObserver::new(attempt, pid),
            qualified: false,
        }
    }

    pub fn progressing(&self, now: Duration) -> bool {
        self.observer.progressing(now)
    }

    /// One poll at launcher-monotonic `now`; returns the line to print when the state changed.
    pub fn step(
        &mut self,
        record: Option<&str>,
        child_alive: bool,
        now: Duration,
    ) -> Option<&'static str> {
        if !child_alive {
            self.qualified = false;
            return Some("child-exited");
        }
        let qualified = self.observer.observe(record, now);
        (std::mem::replace(&mut self.qualified, qualified) != qualified).then_some(if qualified {
            "qualified"
        } else {
            "unqualified"
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zombies_and_missing_processes_are_dead() {
        let dir = std::env::temp_dir().join(format!("cam-hw-{}", std::process::id()));
        for (pid, stat) in [
            (1, "1 (cam) S 0 0"),
            (2, "2 (c) a (m) Z 1 1"),
            (3, "3 (cam) X 1"),
        ] {
            fs::create_dir_all(dir.join(pid.to_string())).unwrap();
            fs::write(dir.join(pid.to_string()).join("stat"), stat).unwrap();
        }
        assert!(process_alive(&dir, 1));
        assert!(!process_alive(&dir, 2), "zombie");
        assert!(!process_alive(&dir, 3));
        assert!(!process_alive(&dir, 4), "missing");
        fs::remove_dir_all(&dir).unwrap();
    }

    fn record(attempt: &str, elapsed_ms: u64, healthy_ms: u64) -> String {
        format!(
            "{{\"version\":1,\"attempt\":\"{attempt}\",\"pid\":7,\"status\":{{\"elapsed_ms\":{elapsed_ms},\"healthy_for_ms\":{healthy_ms},\"qualifying\":{}}}}}",
            healthy_ms >= 180_000
        )
    }

    #[test]
    fn prints_only_transitions_and_reports_a_dead_child() {
        let mut watch = Watch::new("a1", 7);
        let at = |s| Duration::from_secs(s);
        assert_eq!(watch.step(None, true, at(0)), None);
        assert_eq!(
            watch.step(Some(&record("a1", 179_000, 179_000)), true, at(1)),
            None
        );
        assert_eq!(
            watch.step(Some(&record("a1", 180_000, 180_000)), true, at(2)),
            Some("qualified"),
            "second advancing observation at 180 s of health"
        );
        assert_eq!(
            watch.step(Some(&record("a1", 181_000, 181_000)), true, at(3)),
            None
        );
        assert_eq!(
            watch.step(Some(&record("a1", 182_000, 182_000)), true, at(4)),
            None
        );
        // A stale (repeated) record revokes.
        assert_eq!(
            watch.step(Some(&record("a1", 182_000, 182_000)), true, at(8)),
            Some("unqualified")
        );
        // Another attempt's record never qualifies.
        assert_eq!(
            watch.step(Some(&record("other", 400_000, 400_000)), true, at(9)),
            None
        );
        assert_eq!(
            watch.step(Some(&record("a1", 183_000, 183_000)), false, at(10)),
            Some("child-exited")
        );
    }
}
