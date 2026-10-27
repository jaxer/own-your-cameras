//! Bounded, exclusive video-only hardware probe. No network publishing or audio.
use std::io;

#[cfg(all(target_os = "linux", target_arch = "mips", feature = "imp-video"))]
mod target {
    use super::*;
    use std::ffi::{c_char, c_int, c_void};
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    static INTERRUPTED: AtomicBool = AtomicBool::new(false);
    extern "C" fn interrupt(_: c_int) {
        INTERRUPTED.store(true, Ordering::Relaxed);
    }

    unsafe extern "C" {
        fn t31_video_open(handle: *mut *mut c_void, error: *mut c_char, capacity: u32) -> c_int;
        fn t31_video_next(
            handle: *mut c_void,
            data: *mut u8,
            capacity: u32,
            length: *mut u32,
            timestamp: *mut i64,
            error: *mut c_char,
            error_capacity: u32,
        ) -> c_int;
        fn t31_video_close(handle: *mut c_void, error: *mut c_char, capacity: u32) -> c_int;
    }

    fn message(error: &[u8]) -> String {
        let n = error.iter().position(|b| *b == 0).unwrap_or(error.len());
        String::from_utf8_lossy(&error[..n]).into_owned()
    }

    struct Session {
        handle: *mut c_void,
        last_poll: Option<String>,
    }
    impl Session {
        fn open() -> io::Result<Self> {
            let mut pointer = std::ptr::null_mut();
            let mut error = [0_u8; 256];
            // SAFETY: shim owns all SDK types; these output buffers have the declared extents.
            let rc = unsafe {
                t31_video_open(&mut pointer, error.as_mut_ptr().cast(), error.len() as u32)
            };
            if rc != 0 || pointer.is_null() {
                return Err(io::Error::other(message(&error)));
            }
            Ok(Self {
                handle: pointer,
                last_poll: None,
            })
        }

        fn next(&mut self, data: &mut [u8]) -> io::Result<Option<(usize, i64)>> {
            let mut length = 0_u32;
            let mut timestamp = 0_i64;
            let mut error = [0_u8; 256];
            // SAFETY: self is the only owner; shim copies/relinquishes SDK buffers before returning.
            let rc = unsafe {
                t31_video_next(
                    self.handle,
                    data.as_mut_ptr(),
                    data.len() as u32,
                    &mut length,
                    &mut timestamp,
                    error.as_mut_ptr().cast(),
                    error.len() as u32,
                )
            };
            match rc {
                0 if length != 0 && (length as usize) <= data.len() => {
                    Ok(Some((length as usize, timestamp)))
                }
                1 => {
                    self.last_poll = Some(message(&error));
                    Ok(None)
                }
                _ => Err(io::Error::other(message(&error))),
            }
        }

        fn close(&mut self) -> io::Result<()> {
            if self.handle.is_null() {
                return Ok(());
            }
            let mut error = [0_u8; 256];
            // The shim consumes this handle once, including a poisoned failure outcome.
            let pointer = std::mem::replace(&mut self.handle, std::ptr::null_mut());
            let rc =
                unsafe { t31_video_close(pointer, error.as_mut_ptr().cast(), error.len() as u32) };
            if rc != 0 {
                return Err(io::Error::other(message(&error)));
            }
            Ok(())
        }
    }
    impl Drop for Session {
        fn drop(&mut self) {
            if let Err(e) = self.close() {
                eprintln!("SDK cleanup failed: {e}; reboot required before another SDK owner");
            }
        }
    }

    pub(crate) fn exclude_stock() -> io::Result<File> {
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/tmp/mi360-media.lock")?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::other("another custom media owner is active"));
        }
        let myself = std::process::id().to_string();
        for process in std::fs::read_dir("/proc")? {
            let process = process?;
            let name = process.file_name();
            let name = name.to_string_lossy();
            if name == myself || !name.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let comm = match std::fs::read_to_string(process.path().join("comm")) {
                Ok(comm) => comm,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            if ["fetch_av", "ingenic_algo"].contains(&comm.trim()) {
                return Err(io::Error::other(format!(
                    "stock capture process {name} is still alive"
                )));
            }
            let fds = match std::fs::read_dir(process.path().join("fd")) {
                Ok(fds) => fds,
                Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e),
            };
            for fd in fds {
                let device = match std::fs::read_link(fd?.path()) {
                    Ok(path) => path,
                    Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
                    Err(e) => return Err(e),
                };
                let name_is_media = [
                    "/dev/tx-isp",
                    "/dev/avpu",
                    "/dev/tisp",
                    "/dev/ipu",
                    "/dev/dsp",
                ]
                .iter()
                .any(|p| device == Path::new(p));
                let device_name = device.to_string_lossy();
                let frame_channel =
                    device_name
                        .strip_prefix("/dev/framechan")
                        .is_some_and(|suffix| {
                            !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                        });
                if name_is_media || frame_channel || device_name.starts_with("/dev/isp-") {
                    return Err(io::Error::other(format!(
                        "process {name} still owns {}; stop the stock capture owner first",
                        device.display()
                    )));
                }
            }
        }
        Ok(lock)
    }

    pub fn run(output: &Path, frames: u32) -> io::Result<()> {
        let _lock = exclude_stock()?;
        crate::sdk::verify_library()?;
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            if unsafe { libc::signal(signal, interrupt as *const () as libc::sighandler_t) }
                == libc::SIG_ERR
            {
                return Err(io::Error::last_os_error());
            }
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(output)?;
        let mut data = Vec::new();
        data.try_reserve_exact(1024 * 1024)
            .map_err(io::Error::other)?;
        data.resize(1024 * 1024, 0_u8);
        let started = Instant::now();
        if INTERRUPTED.load(Ordering::Relaxed) {
            return Err(io::Error::other("interrupted before SDK initialization"));
        }
        let mut session = Session::open()?;
        let capture = (|| -> io::Result<()> {
            let mut count = 0;
            let mut bytes = 0;
            let mut last_timestamp = None;
            while count < frames {
                if INTERRUPTED.load(Ordering::Relaxed)
                    || started.elapsed() > Duration::from_secs(15)
                {
                    return Err(io::Error::other(format!(
                        "capture interrupted or completion deadline reached; last poll: {}",
                        session.last_poll.as_deref().unwrap_or("none")
                    )));
                }
                let Some((length, timestamp)) = session.next(&mut data)? else {
                    continue;
                };
                if timestamp < 0 || last_timestamp.is_some_and(|last| timestamp < last) {
                    return Err(io::Error::other("encoder timestamp regressed"));
                }
                last_timestamp = Some(timestamp);
                bytes += length;
                if bytes > 8 * 1024 * 1024 {
                    return Err(io::Error::other("capture byte limit exceeded"));
                }
                file.write_all(&data[..length])?;
                count += 1;
                eprintln!("frame={count} bytes={length} timestamp_us={timestamp}");
            }
            file.flush()
        })();
        let cleanup = session.close();
        if let Err(ref e) = cleanup {
            eprintln!("SDK cleanup failed: {e}; reboot required before another SDK owner");
        }
        capture.and(cleanup)
    }
}

#[cfg(all(target_os = "linux", target_arch = "mips", feature = "imp-video"))]
pub(crate) use target::exclude_stock;

pub fn run(output: &str, frames: u32) -> io::Result<()> {
    let ram_file = output
        .strip_prefix("/tmp/mi360-")
        .is_some_and(|name| !name.is_empty() && !name.contains('/'));
    if !(1..=100).contains(&frames) || !ram_file {
        return Err(io::Error::other(
            "capture needs 1..100 frames and a /tmp/mi360- RAM path",
        ));
    }
    #[cfg(all(target_os = "linux", target_arch = "mips", feature = "imp-video"))]
    return target::run(std::path::Path::new(output), frames);
    #[cfg(not(all(target_os = "linux", target_arch = "mips", feature = "imp-video")))]
    Err(io::Error::other(
        "video capture is available only on the target camera",
    ))
}
