//! RAM-only check of the ioctl capture against the pinned library. Stop the vendor `device` first;
//! reboot afterwards. Patches only this process's pinned libhdal ioctl GOT slot.
#[cfg(not(target_arch = "arm"))]
fn main() {}

#[cfg(target_arch = "arm")]
fn main() {
    use hdal::sys::ioctl_capture;
    use hdal_sys as h;
    use std::cell::Cell;
    thread_local! {
        static ACTIVE: Cell<bool> = const { Cell::new(false) };
        static COUNT: Cell<usize> = const { Cell::new(0) };
        static FAIL_AT: Cell<usize> = const { Cell::new(0) };
        static UNREADABLE: Cell<usize> = const { Cell::new(0) };
    }
    unsafe extern "C" fn inject(fd: i32, command: u32, arg: usize) -> i32 {
        let selected = ACTIVE.get() && command == 0xc0144905;
        let mut saved = None;
        if selected {
            COUNT.set(COUNT.get() + 1);
            if COUNT.get() == FAIL_AT.get() {
                // SAFETY: pinned libhdal's set-param call supplies a valid five-word
                // envelope; only its payload pointer is replaced.
                let words = unsafe { std::slice::from_raw_parts_mut(arg as *mut u32, 5) };
                assert_ne!(words[4], 0);
                saved = Some(words[3]);
                words[3] = UNREADABLE.get() as u32;
            }
        }
        // SAFETY: original argument words forwarded to the production interposer.
        let rc = unsafe { ioctl_capture::ioctl(fd, command, arg) };
        // SAFETY: calling thread's live errno pointer.
        let error = unsafe { *libc::__errno_location() };
        if let Some(value) = saved {
            // SAFETY: restore the same live envelope before the vendor resumes.
            unsafe { *(arg as *mut u32).add(3) = value };
        }
        // SAFETY: preserve the syscall's observed errno through test bookkeeping.
        unsafe { *libc::__errno_location() = error };
        rc
    }
    let fail_at: usize = std::env::args().nth(1).unwrap().parse().unwrap();
    assert!(matches!(fail_at, 0 | 1 | 5));
    FAIL_AT.set(fail_at);
    // SAFETY: private anonymous unreadable page, never accessed by this process.
    let unreadable = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            4096,
            libc::PROT_NONE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert_ne!(unreadable, libc::MAP_FAILED);
    UNREADABLE.set(unreadable as usize);
    // Resolve through an explicit library handle: an imported function address can
    // otherwise identify the executable's PLT rather than the library mapping.
    // SAFETY: existing pinned HDAL dependency, retained through the whole helper run.
    let library = unsafe { libc::dlopen(c"/usr/lib/libhdal.so".as_ptr(), libc::RTLD_NOW) };
    assert!(!library.is_null());
    // SAFETY: this export exists in the pinned artifact; it is used only for dladdr.
    let symbol = unsafe { libc::dlsym(library, c"hd_audiocap_set".as_ptr()) };
    assert!(!symbol.is_null());
    // SAFETY: valid writable Dl_info and a symbol owned by this loaded dependency.
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    assert_ne!(unsafe { libc::dladdr(symbol, &mut info) }, 0);
    // R_ARM_JUMP_SLOT ioctl@GLIBC_2.4 is 0x67140 in the pinned SHA-256.
    let slot = (info.dli_fbase as usize + 0x67140) as *mut usize;
    // SAFETY: the pinned library's GOT page belongs only to this disposable process.
    assert_eq!(
        unsafe {
            libc::mprotect(
                (slot as usize & !4095) as _,
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
            )
        },
        0
    );
    // SAFETY: replacement has the same ARM register ABI as this ioctl import.
    unsafe { slot.write(inject as *const () as usize) };
    // SAFETY: exclusive HDAL ownership is an operator precondition of this helper.
    assert_eq!(ioctl_capture::result(|| unsafe { h::hd_common_init(0) }), 0);
    // SAFETY: common init succeeded; no vendor device may be running.
    assert_eq!(
        ioctl_capture::result(|| unsafe { h::hd_audiocap_init() }),
        0
    );
    let mut ctrl = 0;
    // SAFETY: valid output path pointer and initialized unit.
    assert_eq!(
        ioctl_capture::result(|| unsafe {
            h::hd_audiocap_open(0, h::HD_AUDIOCAP_0_CTRL, &mut ctrl)
        }),
        0
    );
    let mut cfg = h::HD_AUDIOCAP_DEV_CONFIG::default();
    cfg.in_max = h::HD_AUDIOCAP_IN {
        sample_rate: 16000,
        sample_bit: 16,
        mode: 1,
        frame_sample: 1024,
    };
    cfg.out_max.sample_rate = 16000;
    cfg.frame_num_max = 10;
    cfg.aec_max = h::HD_AUDIOCAP_AEC {
        enabled: 1,
        leak_estimate_enabled: 0,
        leak_estimate_value: 99,
        noise_cancel_level: -60,
        echo_cancel_level: -60,
        filter_length: 1024,
        frame_size: 128,
        notch_radius: 992,
        lb_channel: h::HD_AUDIOCAP_LB_CH_LEFT,
    };
    cfg.anr_max = h::HD_AUDIOCAP_ANR {
        enabled: 1,
        suppress_level: 30,
        hpf_cut_off_freq: 160,
        bias_sensitive: 9,
    };
    ACTIVE.set(true);
    let mut raw = -999;
    // SAFETY: exact same control configuration and generated FFI as the adapter.
    let result = ioctl_capture::result(|| {
        raw = unsafe {
            h::hd_audiocap_set(
                ctrl,
                h::HD_AUDIOCAP_PARAM_DEV_CONFIG,
                (&mut cfg as *mut h::HD_AUDIOCAP_DEV_CONFIG).cast(),
            )
        };
        raw
    });
    ACTIVE.set(false);
    println!(
        "fail_at={fail_at} calls={} HD_RESULT={raw} adapter_result={result}",
        COUNT.get()
    );
    // SAFETY: release the control path and initialized units; no pipeline was started.
    assert_eq!(
        ioctl_capture::result(|| unsafe { h::hd_audiocap_close(ctrl) }),
        0
    );
    // SAFETY: control path closed.
    assert_eq!(
        ioctl_capture::result(|| unsafe { h::hd_audiocap_uninit() }),
        0
    );
    // SAFETY: all owned units closed; reboot remains mandatory.
    assert_eq!(
        ioctl_capture::result(|| unsafe { h::hd_common_uninit() }),
        0
    );
    // SAFETY: release this process's anonymous test page.
    assert_eq!(unsafe { libc::munmap(unreadable, 4096) }, 0);
    assert_eq!(raw, 0);
    assert_eq!(result, if fail_at == 0 { 0 } else { -65550 });
    assert_eq!(COUNT.get(), if fail_at == 0 { 7 } else { fail_at });
}
