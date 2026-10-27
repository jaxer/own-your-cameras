//! Bounded RAM health records; no durability claim or persistent crash accounting here.
use super::{HealthStatus, MAX_POLL_GAP, SUSTAINED_HEALTH};
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u8,
    attempt: String,
    pid: u32,
    status: HealthStatus,
}

/// The launcher's attempt token: 1..=64 ASCII alphanumeric, `-` or `_` characters.
pub fn valid_attempt(attempt: &str) -> bool {
    !attempt.is_empty()
        && attempt.len() <= 64
        && attempt
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub struct HealthFile {
    path: PathBuf,
    temp: PathBuf,
    attempt: String,
    pid: u32,
}

impl HealthFile {
    /// Call only after acquiring exclusive runtime ownership. Attempt comes from the launcher,
    /// is unique across boots AND launches, and must not be reused even for a failed preflight.
    pub fn new(directory: &Path, attempt: &str, pid: u32) -> io::Result<Self> {
        if !valid_attempt(attempt) || pid == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid attempt or pid",
            ));
        }
        Ok(Self {
            path: directory.join(format!("cam-health-{attempt}.json")),
            temp: directory.join(format!("cam-health-{attempt}.tmp")),
            attempt: attempt.into(),
            pid,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Proves the directory accepts this attempt's record before the watchdog is touched. Leaves nothing behind.
    pub fn check_writable(&self) -> io::Result<()> {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&self.temp)?;
        remove(&self.temp)
    }

    /// None invalidates immediately. Each Some atomically replaces a single small file; no log.
    /// After an interrupted temp write, later writes fail rather than overwrite it.
    pub fn publish(&mut self, status: Option<HealthStatus>) -> io::Result<()> {
        let Some(status) = status else {
            return remove(&self.path);
        };
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&self.temp)?;
        let result = (|| {
            serde_json::to_writer(
                &mut file,
                &Record {
                    version: 1,
                    attempt: self.attempt.clone(),
                    pid: self.pid,
                    status,
                },
            )?;
            file.flush()?;
            fs::rename(&self.temp, &self.path)
        })();
        if result.is_err() {
            let _ = remove(&self.temp);
        }
        result
    }
}

impl Drop for HealthFile {
    fn drop(&mut self) {
        let _ = remove(&self.path);
        let _ = remove(&self.temp);
    }
}

fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// The launcher's view of the records. The launcher checks separately that its child is alive, and times
/// observations with its own monotonic clock, never against wall-clock time or file mtimes.
pub struct HealthObserver {
    attempt: String,
    pid: u32,
    last: Option<(u64, Duration)>,
    advancing: bool,
}

impl HealthObserver {
    pub fn new(attempt: &str, pid: u32) -> Self {
        Self {
            attempt: attempt.into(),
            pid,
            last: None,
            advancing: false,
        }
    }

    /// Current-attempt publisher is advancing, even while service health is still in startup grace.
    pub fn progressing(&self, now: Duration) -> bool {
        self.advancing
            && self.last.is_some_and(|(_, seen)| {
                now.checked_sub(seen).is_some_and(|age| age < MAX_POLL_GAP)
            })
    }

    /// Missing/malformed/mismatched or regressing records revoke qualification. Seeing the same
    /// elapsed_ms twice never extends freshness; at least two advancing observations are required.
    pub fn observe(&mut self, json: Option<&str>, now: Duration) -> bool {
        let record = json
            .filter(|s| s.len() <= 1024)
            .and_then(|s| serde_json::from_str::<Record>(s).ok());
        let Some(record) = record.filter(|r| {
            r.version == 1
                && r.attempt == self.attempt
                && r.pid == self.pid
                && r.status.healthy_for_ms <= r.status.elapsed_ms
        }) else {
            self.last = None;
            self.advancing = false;
            return false;
        };
        let status = record.status;
        let mut fresh = false;
        if let Some((elapsed, seen)) = self.last {
            if now.checked_sub(seen).is_some_and(|age| age < MAX_POLL_GAP)
                && status.elapsed_ms >= elapsed
            {
                fresh = true;
                if status.elapsed_ms > elapsed {
                    self.advancing = true;
                    self.last = Some((status.elapsed_ms, now));
                }
            } else {
                self.advancing = false;
                // A stale or regressed record becomes a new baseline, never an immediate success.
                self.last = Some((status.elapsed_ms, now));
            }
        } else {
            self.last = Some((status.elapsed_ms, now));
        }
        fresh
            && self.advancing
            && status.qualifying
            && Duration::from_millis(status.healthy_for_ms) >= SUSTAINED_HEALTH
    }
}
