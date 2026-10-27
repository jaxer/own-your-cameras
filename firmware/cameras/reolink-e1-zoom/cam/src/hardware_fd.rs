//! Atomic descriptor ownership boundary shared by the I2C and ISP adapters.
use std::ffi::CString;
use std::io;
use std::os::fd::{FromRawFd, OwnedFd};

pub(crate) fn open_rw(path: &str) -> io::Result<OwnedFd> {
    let path = CString::new(path)?;
    // SAFETY: valid NUL-terminated path; ownership is transferred only on success.
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: freshly opened descriptor, owned exclusively by the returned value.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsRawFd;

    #[test]
    fn hardware_descriptor_is_close_on_exec_at_the_os_boundary() {
        let fd = open_rw("/dev/null").unwrap();
        // SAFETY: F_GETFD takes an open descriptor, no pointer argument.
        let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) };
        assert!(flags >= 0);
        assert_ne!(flags & libc::FD_CLOEXEC, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_exec_cannot_inherit_the_hardware_open() {
        let path = std::env::temp_dir().join(format!("cam-fd-{}", std::process::id()));
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let fd = open_rw(path.to_str().unwrap()).unwrap();
        let output = std::process::Command::new("/bin/sh")
            .args([
                "-c",
                "test -d /proc/self/fd && command -v readlink >/dev/null || exit 1; for fd in /proc/self/fd/*; do readlink \"$fd\"; done; exit 0",
            ])
            .output()
            .unwrap();
        drop(fd);
        drop(file);
        std::fs::remove_file(&path).unwrap();
        assert!(output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains(path.to_str().unwrap()));
    }
}
