//! Diagnostic-only guard pages around the active fixed HDAL parameter buffers.
//! Uses the real production Sys calls and pinned library; never alters configuration values.
use std::sync::atomic::{AtomicBool, Ordering};
static ACTIVE: AtomicBool = AtomicBool::new(false);
pub fn enable() {
    ACTIVE.store(true, Ordering::Release);
}

unsafe fn invoke(
    name: &'static std::ffi::CStr,
    path: u32,
    id: u32,
    arg: *mut libc::c_void,
    size: Option<usize>,
) -> i32 {
    // SAFETY: the operator pins /usr/lib/libhdal.so; this handle names that loaded library.
    let handle = unsafe {
        libc::dlopen(
            c"/usr/lib/libhdal.so".as_ptr(),
            libc::RTLD_NOW | libc::RTLD_LOCAL,
        )
    };
    assert!(!handle.is_null());
    // SAFETY: all names below are three-argument HD_RESULT parameter functions in the pinned ELF.
    let symbol = unsafe { libc::dlsym(handle, name.as_ptr()) };
    assert!(!symbol.is_null());
    // SAFETY: exact ARM32 integer/pointer calling convention, independently inspected in the ELF.
    let original: unsafe extern "C" fn(u32, u32, *mut libc::c_void) -> i32 =
        unsafe { std::mem::transmute(symbol) };
    let result = if ACTIVE.load(Ordering::Acquire) {
        let size = size.expect("unexpected reachable parameter in ABI probe");
        assert!(size > 0 && size <= 3072 && !arg.is_null());
        // SAFETY: two anonymous private pages; the second is made unreadable. The first remains live.
        let allocation = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                8192,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(allocation, libc::MAP_FAILED);
        // SAFETY: page-aligned second half of our allocation.
        assert_eq!(
            unsafe { libc::mprotect(allocation.add(4096), 4096, libc::PROT_NONE) },
            0
        );
        // SAFETY: the typed production caller supplies exactly size valid bytes, copied into our page.
        let guarded = unsafe { allocation.add(4096 - size) };
        unsafe {
            allocation.cast::<u8>().write_bytes(0xa5, 4096 - size);
            std::ptr::copy_nonoverlapping(arg.cast::<u8>(), guarded.cast::<u8>(), size);
        }
        // SAFETY: same input bytes, alignment and ownership; an overrun faults in this disposable process.
        let rc = unsafe { original(path, id, guarded) };
        // SAFETY: bounded prefix within our live allocation. Also detect writes before the record.
        assert!(
            unsafe { std::slice::from_raw_parts(allocation.cast::<u8>(), 4096 - size) }
                .iter()
                .all(|b| *b == 0xa5)
        );
        // SAFETY: copy back only the original caller extent; no buffer is retained by these synchronous APIs.
        unsafe {
            std::ptr::copy_nonoverlapping(guarded.cast::<u8>(), arg.cast::<u8>(), size);
        }
        println!(
            "abi_guard name={} id={id} bytes={size} result={rc}",
            name.to_string_lossy()
        );
        // SAFETY: release exactly our two-page private mapping after the synchronous call completed.
        assert_eq!(unsafe { libc::munmap(allocation, 8192) }, 0);
        rc
    } else {
        // SAFETY: unchanged call to the actual library symbol with the original caller buffer.
        unsafe { original(path, id, arg) }
    };
    // SAFETY: drop only this additional reference; the executable retains its DT_NEEDED reference.
    unsafe {
        libc::dlclose(handle);
    }
    result
}

macro_rules! guarded {
    ($name:ident, $cname:literal, $($id:pat => $size:expr),+ $(,)?) => {
        #[unsafe(no_mangle)]
        unsafe extern "C" fn $name(path: u32, id: u32, arg: *mut libc::c_void) -> i32 {
            let size = match id { $($id => Some($size),)+ _ => None };
            // SAFETY: the wrapper preserves the real fixed three-word function signature.
            unsafe { invoke($cname, path, id, arg, size) }
        }
    };
}
guarded!(hd_videocap_set, c"hd_videocap_set", 3=>148, 11=>4, 12=>24, 8=>24);
guarded!(hd_videocap_get, c"hd_videocap_get", 1=>76, 2=>64);
guarded!(hd_videoproc_set, c"hd_videoproc_set", 3=>120, 4=>8, 9=>52);
guarded!(hd_videoenc_set, c"hd_videoenc_set", 2=>120, 4=>20, 5|27=>44, 8=>64, 28=>80);
guarded!(hd_videoenc_get, c"hd_videoenc_get", 3=>8);
guarded!(hd_audiocap_set, c"hd_audiocap_set", 3=>172, 4=>148, 5=>16, 6=>4, 7=>36, 8=>16);
guarded!(hd_audioenc_set, c"hd_audioenc_set", 3=>16, 4=>12, 5=>8);
guarded!(hd_audioenc_get, c"hd_audioenc_get", 2=>8);
guarded!(hd_audioout_set, c"hd_audioout_set", 3=>24, 4=>56, 7=>4, 5=>12, 6=>4);
