//! `cam self-test`: proves the binary runs on the camera and can reach `libhdal.so`.
//!
//! It never calls `hd_common_init`: HDAL init/uninit act on process-global kernel state and break the vendor
//! `device`, which keeps running beside the self-test. `hd_gettime_ms` works without init.

use std::ffi::{CStr, CString, c_char, c_void};

/// What the self-test needs from HDAL; the real one is `Libhdal`, tests use a fake.
pub trait Hdal {
    /// `hd_gettime_ms()`, the monotonic millisecond clock of the HDAL timer.
    fn gettime_ms(&self) -> Result<u32, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HDAL timer syscall failed: {code}")]
    Timer { code: i32 },
    #[error("dlopen {path}: {reason}")]
    Open { path: String, reason: String },
    #[error("dlsym {symbol}: {reason}")]
    Symbol {
        symbol: &'static str,
        reason: String,
    },
}

/// The report `cam self-test` prints, one `key=value` per line so the device test can grep it.
pub fn report(hdal: &dyn Hdal) -> Result<String, Error> {
    Ok(format!(
        "cam_version={}\narch={}\nhd_gettime_ms={}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        hdal.gettime_ms()?
    ))
}

/// `libhdal.so` loaded with `dlopen`.
pub struct Libhdal {
    gettime_ms: unsafe extern "C" fn() -> u32,
}

fn dl_error() -> String {
    // SAFETY: dlerror returns null or a NUL-terminated string valid until the next dl* call.
    let p = unsafe { libc::dlerror() };
    if p.is_null() {
        "unknown".into()
    } else {
        // SAFETY: non-null, NUL-terminated, see above.
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

impl Libhdal {
    pub fn open(path: &str) -> Result<Self, Error> {
        let cpath = CString::new(path).map_err(|e| Error::Open {
            path: path.into(),
            reason: e.to_string(),
        })?;
        // SAFETY: valid NUL-terminated path; the handle is intentionally never closed.
        let handle = unsafe { libc::dlopen(cpath.as_ptr(), libc::RTLD_NOW) };
        if handle.is_null() {
            return Err(Error::Open {
                path: path.into(),
                reason: dl_error(),
            });
        }
        let symbol = c"hd_gettime_ms";
        // SAFETY: handle is a live dlopen handle, symbol is NUL-terminated.
        let sym: *mut c_void = unsafe { libc::dlsym(handle, symbol.as_ptr() as *const c_char) };
        if sym.is_null() {
            return Err(Error::Symbol {
                symbol: "hd_gettime_ms",
                reason: dl_error(),
            });
        }
        // SAFETY: `hd_gettime_ms` has the C signature `unsigned int hd_gettime_ms(void)`.
        let gettime_ms =
            unsafe { std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> u32>(sym) };
        Ok(Self { gettime_ms })
    }
}

impl Hdal for Libhdal {
    fn gettime_ms(&self) -> Result<u32, Error> {
        #[cfg(target_arch = "arm")]
        {
            // SAFETY: dlsym resolved the pinned UINT32(void) timer export above.
            unsafe { hdal::sys::read_time_ms(self.gettime_ms) }
                .map_err(|code| Error::Timer { code })
        }
        #[cfg(not(target_arch = "arm"))]
        {
            // SAFETY: the same resolved timer signature; ARM HDAL cannot run on this host.
            Ok(unsafe { (self.gettime_ms)() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake(u32);
    impl Hdal for Fake {
        fn gettime_ms(&self) -> Result<u32, Error> {
            Ok(self.0)
        }
    }

    #[test]
    fn report_has_version_arch_and_clock() {
        let r = report(&Fake(3913710)).unwrap();
        assert!(r.contains(&format!("cam_version={}\n", env!("CARGO_PKG_VERSION"))));
        assert!(r.contains("arch="));
        assert!(r.contains("hd_gettime_ms=3913710\n"));
    }

    #[test]
    fn failed_timer_is_reported_as_failure() {
        struct Failed;
        impl Hdal for Failed {
            fn gettime_ms(&self) -> Result<u32, Error> {
                Err(Error::Timer { code: -65550 })
            }
        }
        assert!(matches!(
            report(&Failed),
            Err(Error::Timer { code: -65550 })
        ));
    }

    #[test]
    fn open_missing_library_is_an_error() {
        assert!(matches!(
            Libhdal::open("/nonexistent/libhdal.so"),
            Err(Error::Open { .. })
        ));
    }
}
