//! Bounded production logging: stdout/stderr go into a pipe that a drain thread copies into a RAM file
//! up to a byte cap, then silently discards. A bounded nonblocking queue isolates the drain from
//! sink writes. Neither logging thread is a progress source; normal scheduler/OS stalls still apply.

use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::Path;

pub const CAP_BYTES: u64 = 256 * 1024;
pub const LOG_FILE: &str = "/mnt/tmp/cam-serve.log";
const MARKER: &[u8] = b"\n[cam: log cap reached; further output discarded]\n";

/// Best-effort bounded logging. A four-chunk queue isolates the input drain from a blocked
/// sink. Once the byte cap or queue budget is reached, discard the remaining input. The cap
/// marker is best-effort too. At EOF the sink is joined; production exits the whole process instead.
pub fn drain(mut from: impl Read, mut to: impl Write + Send, cap: u64) {
    std::thread::scope(|scope| {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(4);
        let sink = std::thread::Builder::new()
            .name("log-write".into())
            .spawn_scoped(scope, move || {
                for bytes in rx {
                    if to.write_all(&bytes).and_then(|()| to.flush()).is_err() {
                        break;
                    }
                }
            });
        // Even if the sink cannot start, keep consuming: logging cannot stop the daemon.
        let mut capped = sink.is_err();
        let mut queued = 0u64;
        let mut buf = [0u8; 4096];
        loop {
            let n = match from.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            };
            if capped {
                continue;
            }
            let room = usize::try_from(cap - queued).unwrap_or(usize::MAX).min(n);
            if tx.try_send(buf[..room].to_vec()).is_err() {
                capped = true;
                continue;
            }
            queued += room as u64;
            if room < n || queued >= cap {
                let _ = tx.try_send(MARKER.to_vec());
                capped = true;
            }
        }
        drop(tx);
        if let Ok(sink) = sink {
            let _ = sink.join();
        }
    });
}

/// Points each fd in `targets` (normally 1 and 2) at the capped log file at `path`.
pub fn redirect(path: &Path, cap: u64, targets: &[RawFd]) -> io::Result<()> {
    // Validate and save every target before opening the log or changing any output.
    let mut saved = Vec::new();
    for target in targets {
        // SAFETY: fcntl validates the caller's descriptor and duplicates it with CLOEXEC.
        let (fd, flags) = unsafe {
            (
                libc::fcntl(*target, libc::F_DUPFD_CLOEXEC, 0),
                libc::fcntl(*target, libc::F_GETFD),
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: successful fcntl returned a fresh owned descriptor.
        saved.push((*target, unsafe { OwnedFd::from_raw_fd(fd) }, flags));
    }
    let file = crate::files::create_private(path)?;
    let (reader, writer) = pipe()?;
    // A spawn failure leaves stdout/stderr untouched and reportable.
    std::thread::Builder::new()
        .name("log-drain".into())
        .spawn(move || drain(reader, file, cap))?;
    for target in targets {
        // SAFETY: the pipe writer and saved targets are open. Startup owns output redirection.
        if unsafe { libc::dup2(writer.as_raw_fd(), *target) } < 0 {
            let primary = io::Error::last_os_error();
            let mut restore_failed = false;
            for (target, backup, flags) in &saved {
                // SAFETY: restore the saved open descriptions and their original descriptor flags.
                if unsafe { libc::dup2(backup.as_raw_fd(), *target) } < 0
                    || unsafe { libc::fcntl(*target, libc::F_SETFD, *flags) } < 0
                {
                    restore_failed = true;
                }
            }
            return Err(io::Error::other(format!(
                "redirect: {primary}; restore_failed={restore_failed}"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_beyond_the_cap_is_discarded_after_one_marker() {
        let mut out = Vec::new();
        drain(io::Cursor::new(vec![b'x'; 10_000]), &mut out, 1000);
        assert_eq!(out.len(), 1000 + MARKER.len());
        assert!(out.ends_with(MARKER));
        let mut small = Vec::new();
        drain(io::Cursor::new(b"hello".to_vec()), &mut small, 1000);
        assert_eq!(small, b"hello");
    }

    #[test]
    fn exactly_the_cap_still_gets_the_marker_and_a_failing_sink_does_not_block() {
        let mut out = Vec::new();
        drain(io::Cursor::new(vec![b'y'; 100]), &mut out, 100);
        assert!(out.ends_with(MARKER));
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("disk full"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        drain(io::Cursor::new(vec![b'z'; 50_000]), Broken, 10);
    }

    #[test]
    fn redirect_captures_a_replaced_descriptor_into_the_capped_file() {
        let dir = std::env::temp_dir().join(format!("cam-logcap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("serve.log");
        // A stand-in for stdout, so the test never touches the harness's own descriptors.
        let stand_in = std::fs::File::open("/dev/null").unwrap();
        let target = {
            use std::os::fd::AsRawFd;
            // SAFETY: duplicating a live descriptor.
            unsafe { libc::dup(stand_in.as_raw_fd()) }
        };
        assert!(target >= 0);
        redirect(&log, 64, &[target]).unwrap();
        // SAFETY: `target` is a live descriptor we own and write to once.
        let mut writer = unsafe { std::fs::File::from_raw_fd(target) };
        writer.write_all(&[b'a'; 500]).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let text = std::fs::read(&log).unwrap();
            if text.ends_with(MARKER) {
                assert_eq!(text.len(), 64 + MARKER.len());
                break;
            }
            assert!(std::time::Instant::now() < deadline, "drain never capped");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

fn pipe() -> io::Result<(std::fs::File, std::fs::File)> {
    let mut fds = [0; 2];
    #[cfg(target_os = "linux")]
    // SAFETY: writable two-element array; CLOEXEC is set atomically on both private descriptors.
    let rc = unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) };
    #[cfg(not(target_os = "linux"))]
    // SAFETY: writable two-element array. Non-Linux host fallback sets CLOEXEC below.
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful pipe returned two new owned descriptors; RAII also closes on later errors.
    let pair = unsafe {
        (
            std::fs::File::from_raw_fd(fds[0]),
            std::fs::File::from_raw_fd(fds[1]),
        )
    };
    #[cfg(not(target_os = "linux"))]
    for fd in fds {
        // SAFETY: F_SETFD sets flags on an open private descriptor.
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(pair)
}

#[cfg(test)]
mod descriptor_tests {
    use super::*;
    #[test]
    fn private_log_pipe_descriptors_do_not_survive_exec() {
        let (reader, writer) = pipe().unwrap();
        for fd in [reader.as_raw_fd(), writer.as_raw_fd()] {
            // SAFETY: F_GETFD queries an open descriptor without a pointer.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            assert!(flags >= 0);
            assert_ne!(flags & libc::FD_CLOEXEC, 0);
        }
    }
}
