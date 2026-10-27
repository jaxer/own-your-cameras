//! Durable, once-per-boot video-profile trials. No SDK, reboot or transport work.
//! The runtime supplies monotonic time, actual SPS metadata and real health.
//! Only a qualified trial can replace the last confirmed graph. Factory defaults
//! are used only when the passed store is absent; malformed stores fail closed.
use crate::media::{RustVideoSpec, VideoMetadata};
use serde::{Deserialize, Serialize};
use std::ffi::{CString, OsString};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

pub const MAIN_RESOLUTIONS: &[&str] = &["1920x1080", "1280x720", "640x360"];
pub const SUB_RESOLUTIONS: &[&str] = &["640x360", "320x180"];
pub const PROFILES: &[&str] = &["baseline", "main", "high"];
pub const PROBATION: Duration = Duration::from_secs(10);
pub const MAX_HEALTH_GAP: Duration = Duration::from_secs(2);
const MAX_BYTES: u64 = 4096;
const LOCK_MAGIC: &[u8] = b"mi360-video-settings-lock-v1\n";

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn input(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

// Persistent DTO for the existing media type, without changing its FFI owner.
#[derive(Serialize, Deserialize)]
#[serde(remote = "RustVideoSpec", deny_unknown_fields)]
struct SpecRecord {
    main_width: u32,
    main_height: u32,
    sub_width: u32,
    sub_height: u32,
    main_profile: u32,
    sub_profile: u32,
    main_fps: u32,
    sub_fps: u32,
    main_bitrate: u32,
    sub_bitrate: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Staged,
    Armed,
    Attempted,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    phase: Phase,
    #[serde(with = "SpecRecord")]
    target: RustVideoSpec,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema: u32,
    #[serde(with = "SpecRecord")]
    confirmed: RustVideoSpec,
    pending: Option<Pending>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            schema: 1,
            confirmed: RustVideoSpec::default(),
            pending: None,
        }
    }
}
fn valid_spec(spec: RustVideoSpec) -> bool {
    matches!(
        (spec.main_width, spec.main_height),
        (1920, 1080) | (1280, 720) | (640, 360)
    ) && matches!((spec.sub_width, spec.sub_height), (640, 360) | (320, 180))
        && matches!(spec.main_profile, 66 | 77 | 100)
        && matches!(spec.sub_profile, 66 | 77 | 100)
        && (1..=20).contains(&spec.main_fps)
        && (1..=20).contains(&spec.sub_fps)
        && (32..=2048).contains(&spec.main_bitrate)
        && (32..=512).contains(&spec.sub_bitrate)
}
impl State {
    fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() as u64 > MAX_BYTES {
            return Err(invalid("video settings exceed4KiB"));
        }
        let state: Self =
            serde_json::from_slice(bytes).map_err(|_| invalid("invalid video settings JSON"))?;
        if state.schema != 1
            || !valid_spec(state.confirmed)
            || state.pending.as_ref().is_some_and(|pending| {
                !valid_spec(pending.target) || pending.target == state.confirmed
            })
        {
            return Err(invalid("invalid video settings schema or graph"));
        }
        Ok(state)
    }
    fn values(&self) -> RustVideoSpec {
        self.pending.as_ref().map_or(self.confirmed, |p| p.target)
    }
    fn stage(&self, field: &str, value: &str) -> io::Result<Self> {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.phase != Phase::Staged)
        {
            return Err(input(
                "discard or finish the armed video trial before editing",
            ));
        }
        let mut target = self.values();
        match field {
            "main_resolution" => {
                let (width, height) = match value {
                    "1920x1080" => (1920, 1080),
                    "1280x720" => (1280, 720),
                    "640x360" => (640, 360),
                    _ => return Err(input("unsupported main resolution")),
                };
                target.main_width = width;
                target.main_height = height;
            }
            "sub_resolution" => {
                let (width, height) = match value {
                    "640x360" => (640, 360),
                    "320x180" => (320, 180),
                    _ => return Err(input("unsupported sub resolution")),
                };
                target.sub_width = width;
                target.sub_height = height;
            }
            "main_profile" | "sub_profile" => {
                let profile = match value {
                    "baseline" => 66,
                    "main" => 77,
                    "high" => 100,
                    _ => return Err(input("unsupported AVC profile")),
                };
                if field == "main_profile" {
                    target.main_profile = profile;
                } else {
                    target.sub_profile = profile;
                }
            }
            _ => return Err(input("unknown cold-video setting")),
        }
        let mut next = self.clone();
        next.pending = (target != self.confirmed).then_some(Pending {
            phase: Phase::Staged,
            target,
        });
        Ok(next)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Confirmed,
    Staged,
    Armed,
    Attempted,
    RolledBack,
    StorageError,
}

#[derive(Default)]
struct Qualification {
    since: Option<Duration>,
    last: Option<Duration>,
    complete: bool,
}
impl Qualification {
    fn observe(&mut self, valid: bool, now: Duration) -> io::Result<bool> {
        if self.last.is_some_and(|last| now < last) {
            *self = Self::default();
            return Err(input("video qualification clock moved backwards"));
        }
        let gap = self.last.is_some_and(|last| now - last > MAX_HEALTH_GAP);
        self.last = Some(now);
        if !valid {
            self.since = None;
            self.complete = false;
            return Ok(false);
        }
        if gap {
            self.since = None;
            self.complete = false;
        }
        let since = *self.since.get_or_insert(now);
        self.complete = now - since >= PROBATION;
        Ok(self.complete)
    }
}

trait Storage {
    fn load(&mut self) -> io::Result<Option<Vec<u8>>>;
    fn commit(&mut self, expected: Option<&[u8]>, bytes: &[u8]) -> io::Result<()>;
}

/// One settings owner. A derived `<filename>.lock` sidecar in the passed parent
/// is private0600, magic-validated, nofollow/one-link and flocked for this lifetime.
/// Existing locks are never unlinked. Parent must already be owned by `uid` with
/// mode0700; production passes0, host tests inject their actual UID.
///
/// All file access stays within the opened parent. Exclusive temporaries are
/// fsynced before install; new targets use atomic no-overwrite hard-link install,
/// existing validated targets use rename, followed by directory fsync. A stale
/// target refuses replacement. Any commit error poisons this instance because a
/// failed directory sync can follow an installed record: reload before proceeding.
pub struct GraphSettings {
    state: State,
    storage: Box<dyn Storage>,
    bytes: Option<Vec<u8>>,
    selected: Option<RustVideoSpec>,
    running_candidate: bool,
    rolled_back: bool,
    failed: bool,
    qualification: Qualification,
}
impl GraphSettings {
    pub fn load(path: impl AsRef<Path>, uid: u32) -> io::Result<Self> {
        Self::from_storage(Box::new(PrivateStore::open(path.as_ref(), uid)?))
    }
    fn from_storage(mut storage: Box<dyn Storage>) -> io::Result<Self> {
        let bytes = storage.load()?;
        let state = bytes
            .as_deref()
            .map_or_else(|| Ok(State::default()), State::decode)?;
        Ok(Self {
            state,
            storage,
            bytes,
            selected: None,
            running_candidate: false,
            rolled_back: false,
            failed: false,
            qualification: Qualification::default(),
        })
    }
    fn ready(&self) -> io::Result<()> {
        if self.failed {
            Err(io::Error::other(
                "video settings durability uncertain; reload required",
            ))
        } else {
            Ok(())
        }
    }
    fn save(&mut self, state: State) -> io::Result<()> {
        self.ready()?;
        if state == self.state {
            return Ok(());
        }
        let mut bytes =
            serde_json::to_vec(&state).map_err(|_| invalid("cannot encode video settings"))?;
        bytes.push(b'\n');
        if bytes.len() as u64 > MAX_BYTES {
            return Err(invalid("video settings exceed4KiB"));
        }
        if let Err(error) = self.storage.commit(self.bytes.as_deref(), &bytes) {
            self.failed = true;
            return Err(error);
        }
        self.state = state;
        self.bytes = Some(bytes);
        Ok(())
    }
    pub fn confirmed(&self) -> RustVideoSpec {
        self.state.confirmed
    }
    pub fn staged(&self) -> Option<RustVideoSpec> {
        self.state.pending.as_ref().map(|p| p.target)
    }
    /// Editable/staged values for presentation; never use as the boot selection.
    pub fn values(&self) -> RustVideoSpec {
        self.state.values()
    }
    /// Selected graph after select_for_boot, otherwise the confirmed fallback.
    pub fn spec(&self) -> RustVideoSpec {
        self.selected.unwrap_or(self.state.confirmed)
    }
    pub fn status(&self) -> Status {
        if self.failed {
            return Status::StorageError;
        }
        if self.rolled_back {
            return Status::RolledBack;
        }
        match self.state.pending.as_ref().map(|p| p.phase) {
            None => Status::Confirmed,
            Some(Phase::Staged) => Status::Staged,
            Some(Phase::Armed) => Status::Armed,
            Some(Phase::Attempted) => Status::Attempted,
        }
    }
    pub fn stage(&mut self, field: &str, value: &str) -> io::Result<()> {
        self.ready()?;
        let next = self.state.stage(field, value)?;
        self.save(next)?;
        self.rolled_back = false;
        Ok(())
    }
    pub fn discard(&mut self) -> io::Result<()> {
        self.ready()?;
        if self
            .state
            .pending
            .as_ref()
            .is_some_and(|p| p.phase == Phase::Attempted)
        {
            return Err(input(
                "running video trial must qualify or roll back on next boot",
            ));
        }
        let mut next = self.state.clone();
        next.pending = None;
        self.save(next)?;
        self.rolled_back = false;
        Ok(())
    }
    /// Persist intent before the caller requests a reboot. Does not select it in
    /// an already selected/running session; repeated arm is idempotent.
    pub fn arm(&mut self) -> io::Result<()> {
        self.ready()?;
        let mut next = self.state.clone();
        let pending = next
            .pending
            .as_mut()
            .ok_or_else(|| input("no staged video settings"))?;
        match pending.phase {
            Phase::Staged => pending.phase = Phase::Armed,
            Phase::Armed => return Ok(()),
            Phase::Attempted => return Err(input("video trial was already attempted")),
        }
        self.save(next)
    }
    /// Call once before initializing media. Armed becomes Attempted durably
    /// before the target is returned. Attempted loaded by a later process is a
    /// failed trial: clear it durably and use the previous confirmed graph.
    /// Repeated calls on this same instance return its cached selection.
    pub fn select_for_boot(&mut self) -> io::Result<RustVideoSpec> {
        self.ready()?;
        if let Some(spec) = self.selected {
            return Ok(spec);
        }
        let mut next = self.state.clone();
        let spec = match next.pending.as_mut() {
            Some(pending) if pending.phase == Phase::Armed => {
                pending.phase = Phase::Attempted;
                let spec = pending.target;
                self.save(next)?;
                self.running_candidate = true;
                spec
            }
            Some(pending) if pending.phase == Phase::Attempted => {
                next.pending = None;
                self.save(next)?;
                self.rolled_back = true;
                self.state.confirmed
            }
            _ => self.state.confirmed,
        };
        self.selected = Some(spec);
        Ok(spec)
    }
    pub fn needs_confirmation(&self) -> bool {
        self.running_candidate && !self.failed
    }
    /// Called at least every2s with actual independently parsed SPS metadata and
    /// current real runtime health. Missing/mismatched SPS or unhealthy intervals
    /// reset probation. No wall clock is read. Call confirm_healthy immediately
    /// after true; that method cannot independently measure time since this call.
    pub fn observe_health(
        &mut self,
        spec: RustVideoSpec,
        actual: [Option<VideoMetadata>; 2],
        healthy: bool,
        now: Duration,
    ) -> io::Result<bool> {
        self.ready()?;
        if !self.running_candidate || self.selected != Some(spec) {
            self.qualification = Qualification::default();
            return Err(input("health does not belong to the selected video trial"));
        }
        let expected = [
            VideoMetadata {
                width: spec.main_width,
                height: spec.main_height,
                profile_idc: spec.main_profile,
            },
            VideoMetadata {
                width: spec.sub_width,
                height: spec.sub_height,
                profile_idc: spec.sub_profile,
            },
        ];
        self.qualification
            .observe(healthy && actual == expected.map(Some), now)
    }
    /// Commit only the candidate selected by this instance after real qualified
    /// observations. A previous process's Attempted record cannot be confirmed.
    /// Repeating confirmation of the already confirmed same graph writes nothing.
    pub fn confirm_healthy(&mut self, spec: RustVideoSpec) -> io::Result<()> {
        self.ready()?;
        if self.state.pending.is_none() && self.state.confirmed == spec {
            return Ok(());
        }
        if !self.running_candidate
            || self.selected != Some(spec)
            || !self.qualification.complete
            || !self
                .state
                .pending
                .as_ref()
                .is_some_and(|p| p.phase == Phase::Attempted && p.target == spec)
        {
            return Err(input("video trial has not completed SPS/healthy probation"));
        }
        let mut next = self.state.clone();
        next.confirmed = spec;
        next.pending = None;
        self.save(next)?;
        self.running_candidate = false;
        Ok(())
    }
}

struct PrivateStore {
    dir: File,
    name: CString,
    uid: u32,
    _lock: File,
}
fn component_name(name: &std::ffi::OsStr) -> io::Result<CString> {
    CString::new(name.as_bytes()).map_err(|_| input("invalid video settings filename"))
}
fn open_at(dir: &File, name: &std::ffi::CStr, flags: i32, mode: libc::mode_t) -> io::Result<File> {
    // SAFETY: dir owns a live directory fd; name is NUL-terminated; no pointers
    // escape this call. On success the fresh fd is transferred exactly once.
    let fd = unsafe {
        // mode_t is u16 on the host; C varargs require integer promotion.
        libc::openat(dir.as_raw_fd(), name.as_ptr(), flags, mode as libc::c_uint)
    };
    if fd < 0 {
        Err(io::Error::last_os_error())
    } else {
        // SAFETY: openat returned a new owned fd above.
        Ok(unsafe { File::from_raw_fd(fd) })
    }
}
fn private_file(file: &File, uid: u32, maximum: u64) -> io::Result<()> {
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.uid() != uid
        || meta.mode() & 0o7777 != 0o600
        || meta.nlink() != 1
        || meta.len() > maximum
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "video settings files must be private0600, owned, one-link regular files",
        ));
    }
    Ok(())
}
fn open_parent(path: &Path, uid: u32) -> io::Result<(File, OsString)> {
    let name = path
        .file_name()
        .ok_or_else(|| input("video settings path needs a filename"))?;
    let parent = path
        .parent()
        .ok_or_else(|| input("video settings path needs a parent"))?;
    let mut dir = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(if path.is_absolute() { "/" } else { "." })?;
    for component in parent.components() {
        match component {
            Component::Normal(name) => {
                dir = open_at(
                    &dir,
                    &component_name(name)?,
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                    0,
                )?;
            }
            Component::RootDir => {}
            _ => {
                return Err(input(
                    "video settings parent cannot contain dot or parent components",
                ));
            }
        }
    }
    let meta = dir.metadata()?;
    if !meta.is_dir() || meta.uid() != uid || meta.mode() & 0o7777 != 0o700 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "video settings parent must be owned private0700 directory",
        ));
    }
    Ok((dir, name.to_owned()))
}
impl PrivateStore {
    fn check_parent(&self) -> io::Result<()> {
        let meta = self.dir.metadata()?;
        if !meta.is_dir() || meta.uid() != self.uid || meta.mode() & 0o7777 != 0o700 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "video settings parent is no longer private0700",
            ));
        }
        Ok(())
    }
    fn open(path: &Path, uid: u32) -> io::Result<Self> {
        let (dir, name) = open_parent(path, uid)?;
        let mut lock_name = name.clone();
        lock_name.push(".lock");
        let lock_name = component_name(&lock_name)?;
        let flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
        let (mut lock, created) = match open_at(
            &dir,
            &lock_name,
            flags | libc::O_CREAT | libc::O_EXCL,
            0o600,
        ) {
            Ok(file) => (file, true),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                (open_at(&dir, &lock_name, flags, 0)?, false)
            }
            Err(error) => return Err(error),
        };
        if created {
            lock.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        private_file(&lock, uid, 128)?;
        // SAFETY: lock owns a live regular file fd. flock has no pointer argument.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        if created {
            lock.write_all(LOCK_MAGIC)?;
            lock.sync_all()?;
            dir.sync_all()?;
        } else {
            let mut bytes = Vec::new();
            lock.rewind()?;
            (&mut lock).take(129).read_to_end(&mut bytes)?;
            if bytes != LOCK_MAGIC {
                return Err(invalid("unrecognized video settings lock file"));
            }
        }
        Ok(Self {
            dir,
            name: component_name(&name)?,
            uid,
            _lock: lock,
        })
    }
    fn read_target(&self) -> io::Result<Option<Vec<u8>>> {
        self.check_parent()?;
        let file = match open_at(
            &self.dir,
            &self.name,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0,
        ) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        private_file(&file, self.uid, MAX_BYTES)?;
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(invalid("video settings exceed4KiB"));
        }
        Ok(Some(bytes))
    }
}
struct Temporary<'a> {
    dir: &'a File,
    name: CString,
    active: bool,
}
impl Drop for Temporary<'_> {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        // SAFETY: name is a temporary this instance exclusively created within
        // the still-owned directory. No pre-existing entry is ever unlinked.
        let _ = unsafe { libc::unlinkat(self.dir.as_raw_fd(), self.name.as_ptr(), 0) };
    }
}
static TEMP_SEQUENCE: AtomicU32 = AtomicU32::new(0);
impl Storage for PrivateStore {
    fn load(&mut self) -> io::Result<Option<Vec<u8>>> {
        self.read_target()
    }
    fn commit(&mut self, expected: Option<&[u8]>, bytes: &[u8]) -> io::Result<()> {
        if self.read_target()?.as_deref() != expected {
            return Err(invalid("video settings changed outside this owner"));
        }
        let mut created = None;
        for _ in 0..8 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let name = CString::new(format!(
                ".mi360-video-{}-{sequence}.tmp",
                std::process::id()
            ))
            .map_err(|_| input("invalid video settings temporary"))?;
            match open_at(
                &self.dir,
                &name,
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            ) {
                Ok(file) => {
                    created = Some((
                        Temporary {
                            dir: &self.dir,
                            name,
                            active: true,
                        },
                        file,
                    ));
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
        let (mut temporary, mut file) = created.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "no exclusive video temporary available",
            )
        })?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        private_file(&file, self.uid, MAX_BYTES)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        if self.read_target()?.as_deref() != expected {
            return Err(invalid("video settings changed before commit"));
        }
        // SAFETY: both NUL-terminated names are relative to our held directory;
        // temporary is exclusively ours. Initial link never overwrites a racing
        // existing target. Existing targets were validated under the shared lock.
        let result = unsafe {
            if expected.is_none() {
                libc::linkat(
                    self.dir.as_raw_fd(),
                    temporary.name.as_ptr(),
                    self.dir.as_raw_fd(),
                    self.name.as_ptr(),
                    0,
                )
            } else {
                libc::renameat(
                    self.dir.as_raw_fd(),
                    temporary.name.as_ptr(),
                    self.dir.as_raw_fd(),
                    self.name.as_ptr(),
                )
            }
        };
        if result != 0 {
            return Err(io::Error::last_os_error());
        }
        // Initial install must remove its extra hard link before reporting success.
        if expected.is_none() {
            // SAFETY: this is the exclusive temporary linked above, in our held dir.
            if unsafe { libc::unlinkat(self.dir.as_raw_fd(), temporary.name.as_ptr(), 0) } != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        // After successful unlink/rename no cleanup may unlink a newly reused name.
        temporary.active = false;
        drop(temporary);
        self.dir.sync_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::os::unix::fs::{DirBuilderExt, symlink};
    use std::path::PathBuf;
    use std::rc::Rc;

    #[derive(Clone, Copy)]
    enum Fault {
        BeforeInstall,
        AfterInstall,
    }
    #[derive(Default)]
    struct Memory {
        bytes: Option<Vec<u8>>,
        commits: usize,
        fault: Option<Fault>,
    }
    #[derive(Clone, Default)]
    struct FakeStore(Rc<RefCell<Memory>>);
    impl Storage for FakeStore {
        fn load(&mut self) -> io::Result<Option<Vec<u8>>> {
            Ok(self.0.borrow().bytes.clone())
        }
        fn commit(&mut self, expected: Option<&[u8]>, bytes: &[u8]) -> io::Result<()> {
            let mut store = self.0.borrow_mut();
            assert_eq!(expected, store.bytes.as_deref());
            store.commits += 1;
            if matches!(store.fault, Some(Fault::BeforeInstall)) {
                return Err(io::Error::other("injected write/fsync failure"));
            }
            store.bytes = Some(bytes.to_vec());
            if matches!(store.fault, Some(Fault::AfterInstall)) {
                return Err(io::Error::other("injected directory fsync failure"));
            }
            Ok(())
        }
    }
    fn memory_graph() -> (GraphSettings, FakeStore) {
        let store = FakeStore::default();
        (
            GraphSettings::from_storage(Box::new(store.clone())).unwrap(),
            store,
        )
    }
    fn reload(store: &FakeStore) -> GraphSettings {
        GraphSettings::from_storage(Box::new(store.clone())).unwrap()
    }
    fn actual(spec: RustVideoSpec) -> [Option<VideoMetadata>; 2] {
        [
            Some(VideoMetadata {
                width: spec.main_width,
                height: spec.main_height,
                profile_idc: spec.main_profile,
            }),
            Some(VideoMetadata {
                width: spec.sub_width,
                height: spec.sub_height,
                profile_idc: spec.sub_profile,
            }),
        ]
    }
    fn trial(store: &FakeStore) -> (GraphSettings, RustVideoSpec) {
        let mut graph = reload(store);
        let spec = graph.select_for_boot().unwrap();
        assert!(graph.needs_confirmation());
        (graph, spec)
    }
    fn qualify(graph: &mut GraphSettings, spec: RustVideoSpec) {
        for second in 0..=10 {
            assert_eq!(
                graph
                    .observe_health(spec, actual(spec), true, Duration::from_secs(second))
                    .unwrap(),
                second == 10,
            );
        }
    }
    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            let parent = std::env::temp_dir().canonicalize().unwrap();
            for _ in 0..32 {
                let n = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
                let path = parent.join(format!("mi360-graph-test-{}-{n}", std::process::id()));
                match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                    Ok(()) => {
                        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                            .unwrap();
                        return Self(path);
                    }
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(error) => panic!("private fixture directory: {error}"),
                }
            }
            panic!("no exclusive private fixture directory");
        }
        fn path(&self) -> PathBuf {
            self.0.join("video-settings.json")
        }
        fn load(&self) -> io::Result<GraphSettings> {
            GraphSettings::load(self.path(), uid())
        }
        fn write(&self, bytes: &[u8]) {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(self.path())
                .unwrap();
            file.write_all(bytes).unwrap();
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn uid() -> u32 {
        // SAFETY: geteuid has no arguments or memory/lifetime contract.
        unsafe { libc::geteuid() }
    }

    #[test]
    fn factory_defaults_do_not_create_confirmed_json_or_arm_a_trial() {
        let scratch = Scratch::new();
        let mut graph = scratch.load().unwrap();
        assert_eq!(graph.confirmed(), RustVideoSpec::default());
        assert_eq!(graph.select_for_boot().unwrap(), RustVideoSpec::default());
        assert_eq!(graph.status(), Status::Confirmed);
        assert!(!graph.needs_confirmation());
        assert!(!scratch.path().exists());
        assert!(graph.arm().is_err());
        assert!(!scratch.path().exists());
    }

    #[test]
    fn every_finite_resolution_and_profile_choice_preserves_unrelated_fields() {
        for main in MAIN_RESOLUTIONS {
            for sub in SUB_RESOLUTIONS {
                for main_profile in PROFILES {
                    for sub_profile in PROFILES {
                        let mut state = State::default();
                        for (field, value) in [
                            ("main_resolution", *main),
                            ("sub_resolution", *sub),
                            ("main_profile", *main_profile),
                            ("sub_profile", *sub_profile),
                        ] {
                            state = state.stage(field, value).unwrap();
                        }
                        let spec = state.values();
                        assert!(valid_spec(spec));
                        assert_eq!(state.confirmed, RustVideoSpec::default());
                        assert_eq!((spec.main_fps, spec.sub_fps), (20, 20));
                        assert_eq!((spec.main_bitrate, spec.sub_bitrate), (800, 240));
                    }
                }
            }
        }
    }

    #[test]
    fn unsupported_inputs_do_not_change_or_write_any_state() {
        let (mut graph, store) = memory_graph();
        for (field, value) in [
            ("main_resolution", "1080p"),
            ("main_resolution", "320x180"),
            ("sub_resolution", "1920x1080"),
            ("main_profile", "100"),
            ("sub_profile", "NaN"),
            ("main_fps", "10"),
            ("", "high"),
        ] {
            assert!(graph.stage(field, value).is_err());
        }
        assert_eq!(graph.values(), RustVideoSpec::default());
        assert_eq!(store.0.borrow().commits, 0);
    }

    #[test]
    fn staging_never_selects_or_confirms_before_explicit_arm_and_next_session() {
        let (mut graph, store) = memory_graph();
        let good = graph.select_for_boot().unwrap();
        graph.stage("main_profile", "high").unwrap();
        let candidate = graph.values();
        assert_eq!(graph.confirmed(), good);
        assert_eq!(graph.spec(), good);
        assert_eq!(reload(&store).select_for_boot().unwrap(), good);
        graph.arm().unwrap();
        let writes = store.0.borrow().commits;
        graph.arm().unwrap();
        assert_eq!(store.0.borrow().commits, writes);
        assert_eq!(graph.select_for_boot().unwrap(), good);
        assert!(graph.confirm_healthy(candidate).is_err());
        assert!(graph.stage("main_profile", "main").is_err());
        let (mut next, selected) = trial(&store);
        assert_eq!(selected, candidate);
        assert_eq!(next.status(), Status::Attempted);
        let writes = store.0.borrow().commits;
        assert_eq!(next.select_for_boot().unwrap(), candidate);
        assert_eq!(store.0.borrow().commits, writes);
        assert_eq!(next.confirmed(), good);
        assert!(next.discard().is_err());
        assert!(next.arm().is_err());
    }

    #[test]
    fn unconfirmed_attempt_rolls_back_once_before_any_next_media_selection() {
        let (mut graph, store) = memory_graph();
        graph.stage("sub_resolution", "320x180").unwrap();
        graph.arm().unwrap();
        let (_, candidate) = trial(&store);
        let mut next = reload(&store);
        assert!(next.confirm_healthy(candidate).is_err());
        assert!(
            next.observe_health(candidate, actual(candidate), true, Duration::ZERO)
                .is_err()
        );
        assert_eq!(next.select_for_boot().unwrap(), RustVideoSpec::default());
        assert_eq!(next.status(), Status::RolledBack);
        assert_eq!(next.staged(), None);
        assert!(!next.needs_confirmation());
        let writes = store.0.borrow().commits;
        next.select_for_boot().unwrap();
        assert_eq!(store.0.borrow().commits, writes);
        assert_eq!(reload(&store).status(), Status::Confirmed);
    }

    #[test]
    fn only_both_matching_sps_and_continuous_ten_seconds_commit_the_candidate() {
        let (mut graph, store) = memory_graph();
        graph.stage("main_resolution", "1280x720").unwrap();
        graph.stage("sub_profile", "main").unwrap();
        graph.arm().unwrap();
        let (mut running, spec) = trial(&store);
        assert!(running.confirm_healthy(spec).is_err());
        qualify(&mut running, spec);
        assert_eq!(running.confirmed(), RustVideoSpec::default());
        running.confirm_healthy(spec).unwrap();
        assert_eq!(running.confirmed(), spec);
        assert_eq!(running.status(), Status::Confirmed);
        assert!(!running.needs_confirmation());
        let writes = store.0.borrow().commits;
        running.confirm_healthy(spec).unwrap();
        assert_eq!(store.0.borrow().commits, writes);
        assert_eq!(reload(&store).select_for_boot().unwrap(), spec);
    }

    #[test]
    fn missing_wrong_sps_unhealthy_gaps_and_time_reversal_reset_probation() {
        let mut q = Qualification::default();
        assert!(!q.observe(true, Duration::ZERO).unwrap());
        assert!(!q.observe(true, Duration::from_secs(10)).unwrap());
        for second in 11..20 {
            assert!(!q.observe(true, Duration::from_secs(second)).unwrap());
        }
        assert!(q.observe(true, Duration::from_secs(20)).unwrap());
        assert!(!q.observe(false, Duration::from_secs(21)).unwrap());
        assert!(!q.complete);
        assert!(q.observe(true, Duration::from_secs(20)).is_err());
        assert!(!q.complete);
        let (mut graph, store) = memory_graph();
        graph.stage("main_profile", "high").unwrap();
        graph.arm().unwrap();
        let (mut running, spec) = trial(&store);
        for second in 0..=9 {
            running
                .observe_health(spec, actual(spec), true, Duration::from_secs(second))
                .unwrap();
        }
        for (sample, healthy) in [(actual(spec).map(|_| None), true), (actual(spec), false)] {
            assert!(
                !running
                    .observe_health(spec, sample, healthy, Duration::from_secs(10))
                    .unwrap()
            );
            assert!(running.confirm_healthy(spec).is_err());
        }
        let mut wrong = actual(spec);
        wrong[1].as_mut().unwrap().profile_idc = 100;
        assert!(
            !running
                .observe_health(spec, wrong, true, Duration::from_secs(11))
                .unwrap()
        );
        for second in 12..22 {
            assert!(
                !running
                    .observe_health(spec, actual(spec), true, Duration::from_secs(second))
                    .unwrap()
            );
        }
        assert!(
            running
                .observe_health(spec, actual(spec), true, Duration::from_secs(22))
                .unwrap()
        );
        let wrong_spec = RustVideoSpec::default();
        assert!(
            running
                .observe_health(
                    wrong_spec,
                    actual(wrong_spec),
                    true,
                    Duration::from_secs(23)
                )
                .is_err()
        );
        assert!(running.confirm_healthy(spec).is_err());
    }

    #[test]
    fn discard_and_reverting_staged_fields_never_change_the_confirmed_graph() {
        let (mut graph, _) = memory_graph();
        graph.stage("main_profile", "high").unwrap();
        graph.stage("main_profile", "baseline").unwrap();
        assert_eq!(graph.staged(), None);
        graph.stage("main_profile", "main").unwrap();
        graph.arm().unwrap();
        graph.discard().unwrap();
        assert_eq!(graph.status(), Status::Confirmed);
        assert_eq!(graph.confirmed(), RustVideoSpec::default());
        graph.discard().unwrap();
    }

    #[test]
    fn failed_writes_and_post_install_sync_never_advance_memory_or_allow_retry() {
        for fault in [Fault::BeforeInstall, Fault::AfterInstall] {
            let (mut graph, store) = memory_graph();
            store.0.borrow_mut().fault = Some(fault);
            assert!(graph.stage("main_profile", "high").is_err());
            assert_eq!(graph.staged(), None);
            assert_eq!(graph.status(), Status::StorageError);
            assert!(graph.select_for_boot().is_err());
            assert!(graph.stage("main_profile", "main").is_err());
            assert!(graph.discard().is_err());
            assert_eq!(store.0.borrow().commits, 1);
            store.0.borrow_mut().fault = None;
            let restored = reload(&store);
            assert_eq!(restored.confirmed(), RustVideoSpec::default());
            assert_eq!(
                restored.staged().is_some(),
                matches!(fault, Fault::AfterInstall)
            );
        }
    }

    #[test]
    fn failed_consumption_or_rollback_never_returns_an_unsafe_boot_spec() {
        for fault in [Fault::BeforeInstall, Fault::AfterInstall] {
            let (mut graph, store) = memory_graph();
            graph.stage("main_profile", "high").unwrap();
            graph.arm().unwrap();
            let mut next = reload(&store);
            store.0.borrow_mut().fault = Some(fault);
            assert!(next.select_for_boot().is_err());
            assert!(!next.needs_confirmation());
            assert_eq!(next.spec(), RustVideoSpec::default());
            store.0.borrow_mut().fault = None;
            let mut restart = reload(&store);
            let chosen = restart.select_for_boot().unwrap();
            if matches!(fault, Fault::AfterInstall) {
                assert_eq!(chosen, RustVideoSpec::default());
            } else {
                assert_eq!(chosen.main_profile, 100);
            }
            // Ensure the persisted Attempted state exists, then fault rollback.
            if restart.status() != Status::Attempted {
                restart.stage("main_profile", "high").unwrap();
                restart.arm().unwrap();
                let mut consumed = reload(&store);
                consumed.select_for_boot().unwrap();
            }
            let mut rollback = reload(&store);
            store.0.borrow_mut().fault = Some(fault);
            assert!(rollback.select_for_boot().is_err());
            assert_eq!(rollback.status(), Status::StorageError);
        }
    }

    #[test]
    fn confirmation_fault_retains_previous_good_in_memory_and_requires_reload() {
        let (mut graph, store) = memory_graph();
        graph.stage("main_profile", "high").unwrap();
        graph.arm().unwrap();
        let (mut running, spec) = trial(&store);
        qualify(&mut running, spec);
        store.0.borrow_mut().fault = Some(Fault::AfterInstall);
        assert!(running.confirm_healthy(spec).is_err());
        assert_eq!(running.confirmed(), RustVideoSpec::default());
        assert_eq!(running.status(), Status::StorageError);
        assert!(!running.needs_confirmation());
        store.0.borrow_mut().fault = None;
        // A durable install before a failed dirsync is safe only because real
        // qualification preceded the confirmation attempt.
        assert_eq!(reload(&store).confirmed(), spec);
    }

    #[test]
    fn real_private_store_survives_trial_rollback_and_confirmed_reload() {
        let scratch = Scratch::new();
        let mut graph = scratch.load().unwrap();
        graph.stage("main_profile", "high").unwrap();
        graph.arm().unwrap();
        drop(graph);
        let mut running = scratch.load().unwrap();
        let spec = running.select_for_boot().unwrap();
        assert_eq!(spec.main_profile, 100);
        drop(running);
        let mut failed = scratch.load().unwrap();
        assert_eq!(failed.select_for_boot().unwrap(), RustVideoSpec::default());
        assert_eq!(failed.status(), Status::RolledBack);
        failed.stage("sub_profile", "main").unwrap();
        failed.arm().unwrap();
        drop(failed);
        let mut running = scratch.load().unwrap();
        let spec = running.select_for_boot().unwrap();
        qualify(&mut running, spec);
        running.confirm_healthy(spec).unwrap();
        drop(running);
        assert_eq!(scratch.load().unwrap().confirmed(), spec);
        let meta = std::fs::metadata(scratch.path()).unwrap();
        assert_eq!(meta.mode() & 0o7777, 0o600);
        assert_eq!(meta.nlink(), 1);
        assert_eq!(meta.uid(), uid());
        assert!(std::fs::read_dir(&scratch.0).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .as_bytes()
                .starts_with(b".mi360-video-")
        }));
    }

    #[test]
    fn lifetime_lock_rejects_another_owner_and_is_not_unlinked_on_drop() {
        let scratch = Scratch::new();
        let mut first = scratch.load().unwrap();
        let error = scratch.load().err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
        first.stage("main_profile", "high").unwrap();
        assert_eq!(
            scratch.load().err().unwrap().kind(),
            io::ErrorKind::WouldBlock
        );
        drop(first);
        let lock = scratch.0.join("video-settings.json.lock");
        assert_eq!(std::fs::read(&lock).unwrap(), LOCK_MAGIC);
        assert_eq!(std::fs::metadata(&lock).unwrap().mode() & 0o7777, 0o600);
        assert!(scratch.load().is_ok());
    }

    #[test]
    fn stale_external_target_is_never_overwritten_and_poison_is_explicit() {
        let scratch = Scratch::new();
        let mut graph = scratch.load().unwrap();
        graph.stage("main_profile", "high").unwrap();
        let state = State {
            confirmed: RustVideoSpec {
                main_profile: 77,
                ..RustVideoSpec::default()
            },
            ..State::default()
        };
        let outside = serde_json::to_vec(&state).unwrap();
        std::fs::write(scratch.path(), &outside).unwrap();
        assert!(graph.arm().is_err());
        assert_eq!(graph.status(), Status::StorageError);
        assert_eq!(std::fs::read(scratch.path()).unwrap(), outside);
        drop(graph);
        assert_eq!(scratch.load().unwrap().confirmed().main_profile, 77);
    }

    #[test]
    fn changing_parent_privacy_after_load_refuses_a_write() {
        let scratch = Scratch::new();
        let mut graph = scratch.load().unwrap();
        std::fs::set_permissions(&scratch.0, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(graph.stage("main_profile", "high").is_err());
        assert_eq!(graph.status(), Status::StorageError);
        assert!(!scratch.path().exists());
    }

    #[test]
    fn malformed_unsafe_or_future_records_are_not_reset_or_replaced() {
        let scratch = Scratch::new();
        let good = serde_json::to_vec(&State::default()).unwrap();
        scratch.write(&good);
        let mut bad_schema: serde_json::Value = serde_json::from_slice(&good).unwrap();
        bad_schema["schema"] = 2.into();
        let mut bad_spec: serde_json::Value = serde_json::from_slice(&good).unwrap();
        bad_spec["confirmed"]["main_fps"] = 21.into();
        let mut extra: serde_json::Value = serde_json::from_slice(&good).unwrap();
        extra["unreviewed"] = "private-test-value".into();
        for bytes in [
            b"private-test-value".to_vec(),
            serde_json::to_vec(&bad_schema).unwrap(),
            serde_json::to_vec(&bad_spec).unwrap(),
            serde_json::to_vec(&extra).unwrap(),
            vec![b'x'; MAX_BYTES as usize + 1],
        ] {
            std::fs::write(scratch.path(), &bytes).unwrap();
            let error = scratch.load().err().unwrap();
            assert!(!error.to_string().contains("private-test-value"));
            assert_eq!(std::fs::read(scratch.path()).unwrap(), bytes);
        }
    }

    #[test]
    fn symlinks_hardlinks_wrong_permissions_and_unknown_locks_fail_closed() {
        let scratch = Scratch::new();
        let outside = scratch.0.join("unrelated");
        std::fs::write(&outside, b"preserve-user-file").unwrap();
        symlink(&outside, scratch.path()).unwrap();
        assert!(scratch.load().is_err());
        std::fs::remove_file(scratch.path()).unwrap();
        std::fs::hard_link(&outside, scratch.path()).unwrap();
        assert!(scratch.load().is_err());
        std::fs::remove_file(scratch.path()).unwrap();
        assert_eq!(std::fs::read(&outside).unwrap(), b"preserve-user-file");
        scratch.write(&serde_json::to_vec(&State::default()).unwrap());
        std::fs::set_permissions(scratch.path(), std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(scratch.load().is_err());
        std::fs::set_permissions(scratch.path(), std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::set_permissions(&scratch.0, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(scratch.load().is_err());
        std::fs::set_permissions(&scratch.0, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(GraphSettings::load(scratch.path(), uid() + 1).is_err());
        let lock = scratch.0.join("video-settings.json.lock");
        std::fs::write(&lock, b"unrelated-lock-data").unwrap();
        assert!(scratch.load().is_err());
        assert_eq!(std::fs::read(&lock).unwrap(), b"unrelated-lock-data");
        std::fs::remove_file(&lock).unwrap();
        symlink(&outside, &lock).unwrap();
        assert!(scratch.load().is_err());
        assert_eq!(std::fs::read(&outside).unwrap(), b"preserve-user-file");
    }

    #[test]
    fn parent_symlink_and_parent_traversal_cannot_redirect_the_store() {
        let scratch = Scratch::new();
        let alias = scratch.0.join("alias");
        symlink(&scratch.0, &alias).unwrap();
        assert!(GraphSettings::load(alias.join("video-settings.json"), uid()).is_err());
        assert!(
            GraphSettings::load(scratch.0.join("alias/../video-settings.json"), uid()).is_err()
        );
        assert!(!scratch.path().exists());
        assert!(!scratch.0.join("video-settings.json.lock").exists());
    }
}
