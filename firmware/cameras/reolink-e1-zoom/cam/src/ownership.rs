//! Who may touch the hardware. The runtime lock is shared by diagnostics and exclusive to production; beside it
//! sit one lock per resource that only one diagnostic may hold, and the PTZ stop endpoint.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

pub const RUNTIME_LOCK: &str = "/mnt/tmp/cam-runtime.lock";

/// A resource one diagnostic owns on top of the shared runtime lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    /// HDAL init/uninit act on the whole kernel media graph.
    Media,
    /// Two network supervisors would race over process adoption and module recovery.
    Network,
    /// The MS32006 kick and IR bits and the lens state.
    Controls,
}

impl Resource {
    fn file_name(self) -> &'static str {
        match self {
            Self::Media => "cam-media.lock",
            Self::Network => "cam-network.lock",
            Self::Controls => "cam-controls.lock",
        }
    }
}

/// Shared with other diagnostics, refused once production has started this boot (even if it has since
/// exited). Hold it until the command exits.
pub fn shared_diagnostic(runtime: &Path) -> io::Result<File> {
    let file = lock(runtime, libc::LOCK_SH)?;
    if file.metadata()?.len() != 0 {
        return Err(io::Error::other(
            "production recovery pending; reboot before diagnostics",
        ));
    }
    Ok(file)
}

/// `resource`'s lock, beside `runtime`.
pub fn exclusive(runtime: &Path, resource: Resource) -> io::Result<File> {
    lock(&runtime.with_file_name(resource.file_name()), libc::LOCK_EX)
}

/// Production's hold on the runtime lock, kept until the boot ends.
pub fn production(runtime: &Path) -> io::Result<File> {
    lock(runtime, libc::LOCK_EX)
}

/// Where the owner of the controls listens for a stop from another process.
pub fn stop_endpoint(runtime: &Path) -> PathBuf {
    runtime.with_file_name("cam-controls.stop")
}

fn lock(path: &Path, mode: libc::c_int) -> io::Result<File> {
    use std::os::fd::AsRawFd;
    let file = crate::files::open_lock(path)?;
    // SAFETY: flock acts on this live owned fd; LOCK_NB never waits on another owner.
    if unsafe { libc::flock(file.as_raw_fd(), mode | libc::LOCK_NB) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}
