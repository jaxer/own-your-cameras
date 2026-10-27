//! Unresolved cross-process control ownership contract; isolated OS fixtures only.

#[test]
fn control_commands_refuse_an_existing_peripheral_owner_before_device_access() {
    use std::{fs::File, os::fd::AsRawFd};
    let root = std::env::temp_dir().join(format!("cam-controls-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let runtime = root.join("cam-runtime.lock");
    let owner = File::create(root.join("cam-controls.lock")).unwrap();
    // SAFETY: flock takes a live descriptor and integer flags.
    assert_eq!(
        unsafe { libc::flock(owner.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    for command in ["ptz", "zoom", "focus", "ir", "led"] {
        assert!(
            cam::cmd::dispatch(&[command.into()], &runtime).is_err(),
            "{command} must refuse before dispatch"
        );
    }
    drop(owner);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn emergency_stop_remains_reachable_while_a_control_owner_is_active() {
    use std::{fs::File, os::fd::AsRawFd};
    let root = std::env::temp_dir().join(format!("cam-emergency-stop-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let runtime = root.join("cam-runtime.lock");
    let owner = File::create(root.join("cam-controls.lock")).unwrap();
    // SAFETY: live descriptor and integer flock flags.
    assert_eq!(
        unsafe { libc::flock(owner.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let mut ptz = cam::ptz::Ptz::new(cam::ptz::fake::FakeBus::new(0, 0))
        .with_stop_owner(cam::control_stop::Owner::bind(&root.join("cam-controls.stop")).unwrap());
    let result = std::thread::scope(|scope| {
        let caller = scope.spawn(|| cam::cmd::dispatch(&["ptz".into(), "stop".into()], &runtime));
        let deadline = std::time::Instant::now() + cam::control_stop::WAIT;
        while ptz.checkpoint().is_ok() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        caller.join().unwrap()
    });
    assert_eq!(result.as_ref().unwrap(), &std::process::ExitCode::SUCCESS);
    assert_eq!(ptz.position(), None);
    assert_eq!(ptz.bus().regs()[0] & 6, 6);
    drop(ptz);
    drop(owner);
    std::fs::remove_dir_all(root).unwrap();
    // On this host device access may fail, but ownership may not reject a documented stop request.
    assert!(
        result.is_ok(),
        "documented cross-process stop was rejected: {result:?}"
    );
}
