//! Captures the errno of a failing ioctl inside libhdal, which HDAL itself reports only as a generic error.
//! Capture is per thread and only around our own calls; other threads pass straight through. The result is
//! `-65536 - errno`, outside HDAL's own status values.

use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
pub struct Failure {
    pub rc: i32,
    pub errno: i32,
    pub command: usize,
    pub item: Option<u32>,
}
impl Failure {
    pub fn code(self) -> i32 {
        eprintln!(
            "HDAL ioctl failed: rc={} errno={} command={:#x} item={:?}",
            self.rc, self.errno, self.command, self.item
        );
        -65536 - self.errno.clamp(1, 4095)
    }
}
#[derive(Clone, Copy, Default)]
struct Scope {
    first: Option<Failure>,
}
thread_local! {
    static CAPTURE: Cell<Option<Scope>> = const { Cell::new(None) };
}
struct Guard(Option<Scope>);
impl Drop for Guard {
    fn drop(&mut self) {
        CAPTURE.with(|slot| {
            let current = slot.replace(self.0);
            if let (Some(mut parent), Some(child)) = (self.0, current) {
                parent.first = parent.first.or(child.first);
                slot.set(Some(parent));
            }
        });
    }
}
pub fn record(rc: i32, errno: i32, command: usize, item: Option<u32>) {
    if rc >= 0 {
        return;
    }
    let _ = CAPTURE.try_with(|slot| {
        if let Some(mut scope) = slot.get() {
            scope.first.get_or_insert(Failure {
                rc,
                errno,
                command,
                item,
            });
            slot.set(Some(scope));
        }
    });
}
// ARM EABI passes ioctl's optional third machine word in r2. It is forwarded
// without interpreting it, including scalar values and commands ignoring r2.
// Exported only on the Linux ARM target, where it interposes the process's ioctl.
#[cfg(target_arch = "arm")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioctl(fd: libc::c_int, command: libc::c_ulong, arg: usize) -> libc::c_int {
    // SAFETY: forward the original three ABI words to the kernel unchanged.
    let rc = unsafe { libc::syscall(libc::SYS_ioctl, fd, command, arg) } as libc::c_int;
    // SAFETY: glibc exposes the calling thread's live errno cell.
    let errno = unsafe { libc::__errno_location() };
    // SAFETY: errno is valid for this thread throughout this function.
    let saved = unsafe { *errno };
    record(rc, saved, command as usize, None);
    // SAFETY: restore errno even if TLS initialization changed it.
    unsafe { *errno = saved };
    rc
}
pub fn run<T>(call: impl FnOnce() -> T) -> (T, Option<Failure>) {
    let guard = Guard(CAPTURE.with(|slot| slot.replace(Some(Scope::default()))));
    let value = call();
    let failure = CAPTURE.with(|slot| slot.get().and_then(|scope| scope.first));
    drop(guard);
    (value, failure)
}
pub fn result(call: impl FnOnce() -> i32) -> i32 {
    let (rc, failure) = run(call);
    match failure {
        None => rc,
        Some(_) if rc < 0 => {
            // The pinned kernel returns -EFAULT for a nonzero ISF status after
            // copying that status into the envelope (including empty/full queues).
            // A negative HD_RESULT already reports failure; preserve its meaning.
            // Unreported syscall failures leave the initialized result word at zero
            // and still take the distinct captured-error branch below.
            rc
        }
        Some(failure) => failure.code(),
    }
}
