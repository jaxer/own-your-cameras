//! Production start-up before the lifecycle owner runs: strict preflight, runtime lock, process
//! exclusion, S00 flag and the real filesystem adapters, all rooted in an isolated fixture tree.
use cam::exclusion;
use cam::preflight::{self, ARTIFACTS};
use cam::runtime::Monotonic;
use cam::runtime::production::{Paths, StartError, start};
use hdal::fake::Fake;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// A child briefly inherits open descriptors between fork and exec. Keep unrelated
// subprocess creation out of assertions that require immediate reacquisition after Drop.
static PROCESS_SPAWN: std::sync::RwLock<()> = std::sync::RwLock::new(());
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cam-launch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(path.join("mnt/tmp")).unwrap();
        fs::create_dir_all(path.join("mnt/para")).unwrap();
        fs::create_dir_all(path.join("proc")).unwrap();
        let root = Self(path);
        root.write(
            "mnt/para/cam-wifi.conf",
            "network={\n ssid=\"fixture-ap\"\n psk=\"fixture-password\"\n}\n",
        );
        root.write("mnt/para/cam-rtsp.auth", "go2rtc:fixture-secret\n");
        root.write(
            "mnt/para/cam.toml",
            "[mqtt]\nhost='127.0.0.1'\nusername='camera'\npassword='fixture-mqtt-secret'\n",
        );
        for artifact in ARTIFACTS {
            root.write(artifact.trim_start_matches('/'), "x");
            if artifact.starts_with("/sbin/") {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(
                    root.0.join(artifact.trim_start_matches('/')),
                    fs::Permissions::from_mode(0o755),
                )
                .unwrap();
            }
        }
        fs::remove_file(root.0.join("dev/mtd7ro")).unwrap();
        std::os::unix::fs::symlink("/dev/null", root.0.join("dev/mtd7ro")).unwrap();
        root.write("dev/watchdog", "");
        root
    }
    fn write(&self, rel: &str, data: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, data).unwrap();
        if rel.starts_with("mnt/para/") {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
    fn paths(&self) -> Paths {
        Paths::rooted(&self.0, 0)
    }
    fn process(&self, pid: u32, comm: &str, argv: &[&str]) {
        let dir = format!("proc/{pid}");
        self.write(&format!("{dir}/comm"), &format!("{comm}\n"));
        self.write(&format!("{dir}/cmdline"), &(argv.join("\0") + "\0"));
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn leak() -> &'static AtomicBool {
    Box::leak(Box::new(AtomicBool::new(false)))
}
fn hardware() -> cam::runtime::production::HardwareOpener {
    Box::new(|_| Err("no hardware in host tests".into()))
}

#[test]
fn preflight_accepts_a_complete_installation_and_prints_no_secret() {
    let root = Root::new();
    assert!(preflight::check(&root.0).is_ok());
    assert_eq!(preflight::run(&root.0), 0);
}

#[test]
fn preflight_hands_the_configured_identity_to_the_runtime() {
    let root = Root::new();
    let ready = preflight::check(&root.0).unwrap();
    assert_eq!(ready.mqtt.camera.id(), "e1zoom");
    root.write(
        "mnt/para/cam.toml",
        "[camera]\nid='hall'\nname='Hall camera'\n[mqtt]\nhost='h'\n",
    );
    let ready = preflight::check(&root.0).unwrap();
    assert_eq!(
        (ready.mqtt.camera.id(), ready.mqtt.camera.name()),
        ("hall", "Hall camera")
    );
}

#[test]
fn preflight_reports_every_problem_without_echoing_a_secret() {
    let root = Root::new();
    let secrets = ["fixture-password", "fixture-secret", "fixture-mqtt-secret"];
    for (name, breakage) in [
        (
            "wifi missing",
            Box::new(|r: &Root| fs::remove_file(r.0.join("mnt/para/cam-wifi.conf")).unwrap())
                as Box<dyn Fn(&Root)>,
        ),
        (
            "wifi short psk",
            Box::new(|r| {
                r.write(
                    "mnt/para/cam-wifi.conf",
                    "network={\nssid=\"a\"\npsk=\"short\"\n}\n",
                )
            }),
        ),
        (
            "auth missing",
            Box::new(|r| fs::remove_file(r.0.join("mnt/para/cam-rtsp.auth")).unwrap()),
        ),
        (
            "auth empty password",
            Box::new(|r| r.write("mnt/para/cam-rtsp.auth", "user:\n")),
        ),
        (
            "config missing",
            Box::new(|r| fs::remove_file(r.0.join("mnt/para/cam.toml")).unwrap()),
        ),
        (
            "config without mqtt",
            Box::new(|r| {
                r.write(
                    "mnt/para/cam.toml",
                    "[presets.home]\npan=1\ntilt=1\nzoom=1\n",
                )
            }),
        ),
        (
            "config garbage",
            Box::new(|r| {
                r.write(
                    "mnt/para/cam.toml",
                    "[mqtt]\nhost='h'\npassword='fixture-mqtt-secret\n",
                )
            }),
        ),
        (
            "config unknown key",
            Box::new(|r| r.write("mnt/para/cam.toml", "[mqtt]\nhost='h'\nbogus=1\n")),
        ),
        (
            "camera id with a slash",
            Box::new(|r| {
                r.write(
                    "mnt/para/cam.toml",
                    "[camera]\nid='a/b'\n[mqtt]\nhost='h'\npassword='fixture-mqtt-secret'\n",
                )
            }),
        ),
        (
            "camera name with a newline",
            Box::new(|r| {
                r.write(
                    "mnt/para/cam.toml",
                    "[camera]\nname=\"a\\nb\"\n[mqtt]\nhost='h'\npassword='fixture-mqtt-secret'\n",
                )
            }),
        ),
        (
            "libhdal missing",
            Box::new(|r| fs::remove_file(r.0.join("usr/lib/libhdal.so")).unwrap()),
        ),
        (
            "module missing",
            Box::new(|r| {
                fs::remove_file(r.0.join("lib/modules/wifi/wifi_8812ko/wlan.ko")).unwrap()
            }),
        ),
        (
            "artifact is a directory",
            Box::new(|r| {
                fs::remove_file(r.0.join("sbin/udhcpc")).unwrap();
                fs::create_dir(r.0.join("sbin/udhcpc")).unwrap();
            }),
        ),
    ] {
        let fresh = Root::new();
        breakage(&fresh);
        let Err(problems) = preflight::check(&fresh.0) else {
            panic!("{name}: must be rejected");
        };
        assert!(!problems.is_empty(), "{name}");
        let text = problems.join("\n");
        for secret in secrets {
            assert!(!text.contains(secret), "{name}: leaked a secret: {text}");
        }
        assert_eq!(preflight::run(&fresh.0), 1, "{name}");
    }
    drop(root);
}

#[test]
fn preflight_lists_all_problems_and_changes_nothing() {
    let root = Root::new();
    fs::remove_file(root.0.join("mnt/para/cam-rtsp.auth")).unwrap();
    fs::remove_file(root.0.join("mnt/para/cam.toml")).unwrap();
    let before: Vec<_> = fs::read_dir(root.0.join("mnt/tmp")).unwrap().collect();
    let Err(problems) = preflight::check(&root.0) else {
        panic!()
    };
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert_eq!(
        fs::read_dir(root.0.join("mnt/tmp")).unwrap().count(),
        before.len()
    );
    assert!(
        !root.0.join("mnt/tmp/atheros_config.conf").exists(),
        "never seeds from the vendor RAM file"
    );
}

#[test]
fn exclusion_finds_vendor_owners_and_other_cam_owners_only() {
    let root = Root::new();
    root.process(10, "init", &["/sbin/init"]);
    root.process(11, "sshd", &["/usr/sbin/sshd"]);
    root.process(12, "cam", &["/mnt/tmp/cam", "health-watch"]);
    assert_eq!(
        exclusion::conflicts(&root.0.join("proc"), 99).unwrap(),
        Vec::<String>::new()
    );
    root.process(20, "device", &["/mnt/app/device"]);
    root.process(21, "router", &["/mnt/app/router"]);
    root.process(22, "cam", &["/mnt/tmp/cam", "wdt"]);
    root.process(23, "cam", &["cam", "serve", "--diagnostic"]);
    root.process(24, "cam", &["/mnt/tmp/cam", "serve", "--attempt", "t"]);
    let found = exclusion::conflicts(&root.0.join("proc"), 24).unwrap();
    assert_eq!(found.len(), 4, "{found:?}");
    assert!(
        !found.iter().any(|f| f.starts_with("pid 24")),
        "ignores itself"
    );
}

#[test]
fn start_refuses_each_failed_gate_before_touching_the_watchdog_flag() {
    let flag = |r: &Root| r.0.join("mnt/tmp/wdt_flag");
    // Invalid preflight.
    let root = Root::new();
    fs::remove_file(root.0.join("mnt/para/cam.toml")).unwrap();
    let r = start(
        &root.paths(),
        "t1",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    );
    assert!(matches!(r, Err(StartError::Preflight(_))));
    assert!(!flag(&root).exists());
    // A vendor owner is running.
    let root = Root::new();
    root.process(20, "device", &["/mnt/app/device"]);
    let r = start(
        &root.paths(),
        "t1",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    );
    assert!(matches!(r, Err(StartError::Conflict(_))));
    assert!(!flag(&root).exists());
    // A bad attempt token.
    let root = Root::new();
    let r = start(
        &root.paths(),
        "bad token",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    );
    assert!(matches!(r, Err(StartError::Attempt)));
}

#[test]
fn start_refuses_an_uninspectable_process_before_releasing_s00() {
    for unreadable in ["comm", "cmdline"] {
        let root = Root::new();
        root.process(20, "cam", &["/mnt/tmp/cam", "serve", "--diagnostic"]);
        let path = root.0.join("proc/20").join(unreadable);
        fs::remove_file(&path).unwrap();
        // A directory gives a deterministic read error even when the test runs as root.
        fs::create_dir(&path).unwrap();
        let result = start(
            &root.paths(),
            "uninspectable",
            leak(),
            Fake::default(),
            Monotonic::default(),
            hardware(),
        );
        assert!(
            matches!(result, Err(StartError::Conflict(_))),
            "cannot prove single ownership with unreadable {unreadable}: {result:?}"
        );
        assert!(!root.paths().flag().exists(), "S00 must keep ownership");
    }
}

#[test]
fn a_second_runtime_fails_on_the_lock_and_an_existing_flag_blocks_takeover() {
    let _no_fork = PROCESS_SPAWN.read().unwrap();
    let root = Root::new();
    let first = start(
        &root.paths(),
        "t1",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    )
    .unwrap_or_else(|e| panic!("first runtime starts: {e}"));
    let second = start(
        &root.paths(),
        "t2",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    );
    assert!(
        matches!(second, Err(StartError::Lock(_))),
        "another runtime holds the lock"
    );
    assert!(
        !root.0.join("mnt/tmp/wdt_flag").exists(),
        "a refused runtime never touches the flag"
    );
    drop(first);
    // The lock inode persists; a stale flag from this boot blocks a same-boot restart.
    assert!(root.0.join("mnt/tmp/cam-runtime.lock").exists());
    fs::write(root.0.join("mnt/tmp/wdt_flag"), "").unwrap();
    let again = start(
        &root.paths(),
        "t3",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    )
    .unwrap_or_else(|e| panic!("lock is free again: {e}"));
    let outcome = cam::runtime::production::run(again.wiring);
    let message = outcome.result.expect_err("must fail closed").to_string();
    assert!(message.contains("watchdog"), "{message}");
    assert!(
        root.0.join("mnt/tmp/wdt_flag").exists(),
        "flag never removed"
    );
    assert!(
        root.0.join("mnt/tmp/cam-runtime.lock").exists(),
        "lock inode never removed"
    );
    let _ = Path::new("");
}

#[test]
fn start_binds_the_rtsp_port_only_after_every_check_passed() {
    let root = Root::new();
    let blocker = std::net::TcpListener::bind("0.0.0.0:0").unwrap();
    let port = blocker.local_addr().unwrap().port();
    let paths = Paths::rooted(&root.0, port);
    let r = start(
        &paths,
        "t1",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    );
    assert!(
        matches!(r, Err(StartError::Bind(_))),
        "port in use must fail before the handoff"
    );
    assert!(!root.0.join("mnt/tmp/wdt_flag").exists());
}

#[test]
fn diagnostics_and_production_exclude_each_other_for_the_whole_boot() {
    let _no_fork = PROCESS_SPAWN.read().unwrap();
    use cam::ownership::shared_diagnostic;
    use cam::runtime::Handoff;
    use cam::runtime::s00::S00Handoff;
    let root = Root::new();
    let paths = root.paths();
    let diagnostic = shared_diagnostic(&paths.lock()).unwrap();
    // The intended wdt + serve + peripheral workflow shares diagnostic ownership.
    let companion = shared_diagnostic(&paths.lock()).unwrap();
    let launch = || {
        start(
            &paths,
            "owned",
            leak(),
            Fake::default(),
            Monotonic::default(),
            hardware(),
        )
    };
    assert!(matches!(launch(), Err(StartError::Lock(_))));
    drop(diagnostic);
    assert!(matches!(launch(), Err(StartError::Lock(_))));
    drop(companion);
    let mut production = launch().unwrap();
    assert!(shared_diagnostic(&paths.lock()).is_err());
    production.wiring.handoff.request_release().unwrap();
    drop(production); // Includes feeder/process exit: recovery remains terminal for this boot.
    assert!(shared_diagnostic(&paths.lock()).is_err());
    let mut retry =
        S00Handoff::acquire(&paths.lock(), &paths.flag(), &root.0.join("dev/watchdog")).unwrap();
    fs::remove_file(paths.flag()).unwrap(); // Even manual deletion of S00's flag cannot revive it.
    assert!(retry.request_release().is_err());
    assert!(!paths.flag().exists());
}

#[test]
fn every_hardware_command_refuses_a_production_owner_before_dispatch() {
    let root = Root::new();
    let production = start(
        &root.paths(),
        "commands",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    )
    .unwrap();
    for args in [
        vec!["serve", "--diagnostic", "--no-auth"],
        vec!["wdt"],
        vec!["dump"],
        vec!["probe-videocap"],
        vec!["net"],
        vec!["net", "--seed-vendor"],
        vec!["ptz", "home"],
        vec!["zoom", "info"],
        vec!["focus"],
        vec!["ir", "status"],
        vec!["led", "on"],
    ] {
        let args: Vec<String> = args.into_iter().map(String::from).collect();
        assert!(
            cam::cmd::dispatch(&args, &root.paths().lock()).is_err(),
            "{args:?}"
        );
    }
    drop(production);
}

#[test]
fn ownership_refuses_symlinks_hardlinks_and_writable_lock_files() {
    use cam::ownership::shared_diagnostic;
    use cam::runtime::s00::S00Handoff;
    use std::os::unix::fs::{PermissionsExt, symlink};
    for kind in ["symlink", "hardlink", "writable"] {
        let root = Root::new();
        let lock = root.paths().lock();
        let other = root.0.join("untouched");
        fs::write(&other, "").unwrap();
        match kind {
            "symlink" => symlink(&other, &lock).unwrap(),
            "hardlink" => fs::hard_link(&other, &lock).unwrap(),
            _ => {
                fs::write(&lock, "").unwrap();
                fs::set_permissions(&lock, fs::Permissions::from_mode(0o666)).unwrap();
            }
        }
        assert!(
            S00Handoff::acquire(&lock, &root.paths().flag(), &root.0.join("dev/watchdog")).is_err(),
            "{kind}"
        );
        assert!(shared_diagnostic(&lock).is_err(), "{kind}");
        assert_eq!(fs::read(&other).unwrap(), b"");
        assert!(!root.paths().flag().exists());
    }
}

#[test]
fn production_refuses_renamed_diagnostics_and_peripheral_owners() {
    for command in [
        "serve",
        "wdt",
        "ptz",
        "zoom",
        "focus",
        "ir",
        "led",
        "dump",
        "probe-videocap",
        "net",
    ] {
        let root = Root::new();
        root.process(25, "cam.new", &["/mnt/tmp/cam.new", command]);
        let result = start(
            &root.paths(),
            "renamed",
            leak(),
            Fake::default(),
            Monotonic::default(),
            hardware(),
        );
        assert!(
            matches!(result, Err(StartError::Conflict(_))),
            "{command}: {result:?}"
        );
        assert!(!root.paths().flag().exists());
    }
}

#[test]
fn missing_identity_on_a_surviving_process_fails_closed_but_confirmed_exits_do_not() {
    for missing in ["comm", "cmdline"] {
        let root = Root::new();
        root.process(25, "cam.new", &["/mnt/tmp/cam.new", "wdt"]);
        root.write(&format!("proc/25/{missing}"), "");
        assert!(
            exclusion::conflicts(&root.0.join("proc"), 99).is_err(),
            "empty {missing}"
        );
        fs::remove_file(root.0.join("proc/25").join(missing)).unwrap();
        assert!(
            exclusion::conflicts(&root.0.join("proc"), 99).is_err(),
            "missing {missing}"
        );
    }
    let root = Root::new();
    // A readdir entry whose target has disappeared exercises the same ENOENT/confirmed-exit path.
    std::os::unix::fs::symlink(root.0.join("exited"), root.0.join("proc/26")).unwrap();
    root.process(27, "kworker", &[]);
    root.write("proc/27/cmdline", "");
    root.write("proc/27/stat", "27 (kworker) S 2 0 0 0 0 2097152 0");
    root.process(28, "cam", &[]);
    root.write("proc/28/cmdline", "");
    root.write("proc/28/stat", "28 (cam) Z 1 0 0 0 0 0 0");
    assert!(
        exclusion::conflicts(&root.0.join("proc"), 99)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn preflight_rejects_unexecutable_empty_or_wrong_kind_artifacts() {
    use std::os::unix::fs::PermissionsExt;
    for artifact in [
        "sbin/udhcpc",
        "usr/lib/libhdal.so",
        "lib/modules/wifi/wifi_8812ko/wlan.ko",
        "dev/mtd7ro",
    ] {
        let root = Root::new();
        let path = root.0.join(artifact);
        if artifact == "sbin/udhcpc" {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        } else {
            fs::remove_file(&path).unwrap();
            fs::write(&path, "").unwrap();
        }
        assert!(preflight::check(&root.0).is_err(), "{artifact}");
    }
}

#[test]
fn preflight_rejects_unsafe_credentials_without_printing_their_contents() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for name in ["cam-wifi.conf", "cam-rtsp.auth", "cam.toml"] {
        for kind in ["public", "symlink", "oversized", "directory"] {
            let root = Root::new();
            assert!(preflight::check(&root.0).is_ok());
            let path = root.0.join("mnt/para").join(name);
            match kind {
                "public" => fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap(),
                "symlink" => {
                    let target = path.with_extension("saved");
                    fs::rename(&path, &target).unwrap();
                    symlink(target, &path).unwrap();
                }
                "oversized" => fs::write(&path, "private-sentinel".repeat(2048)).unwrap(),
                _ => {
                    fs::remove_file(&path).unwrap();
                    fs::create_dir(&path).unwrap();
                }
            }
            let problems = preflight::check(&root.0)
                .err()
                .unwrap_or_else(|| panic!("accepted {name} {kind}"));
            assert!(problems.iter().any(|e| e.contains(name)));
            assert!(!problems.join(" ").contains("private-sentinel"));
        }
    }
}

fn finish_child(mut child: std::process::Child) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("command blocked past its deadline");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn preflight_does_not_block_on_fifo_inputs_or_artifacts() {
    let _fork = PROCESS_SPAWN.write().unwrap();
    for name in [
        "mnt/para/cam-wifi.conf",
        "mnt/para/cam-rtsp.auth",
        "mnt/para/cam.toml",
        "sbin/udhcpc",
    ] {
        let root = Root::new();
        let path = root.0.join(name);
        fs::remove_file(&path).unwrap();
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        let child = std::process::Command::new(env!("CARGO_BIN_EXE_cam-launch"))
            .arg("--preflight")
            .arg(&root.0)
            .spawn()
            .unwrap();
        assert_eq!(finish_child(child).code(), Some(1), "{name}");
    }
}

#[test]
fn shared_diagnostic_process_probe() {
    if let Some(path) = std::env::var_os("CAM_TEST_DIAGNOSTIC_LOCK") {
        let result = cam::ownership::shared_diagnostic(Path::new(&path));
        std::process::exit(i32::from(result.is_err()));
    }
}

#[test]
fn diagnostic_process_cannot_enter_a_live_or_exited_production_boot() {
    let _fork = PROCESS_SPAWN.write().unwrap();
    use cam::runtime::Handoff;
    let root = Root::new();
    let probe = || {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "shared_diagnostic_process_probe"])
            .env("CAM_TEST_DIAGNOSTIC_LOCK", root.paths().lock())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        finish_child(child).success()
    };
    let diagnostic = cam::ownership::shared_diagnostic(&root.paths().lock()).unwrap();
    assert!(probe(), "separate diagnostic companions must coexist");
    drop(diagnostic);
    let mut production = start(
        &root.paths(),
        "process",
        leak(),
        Fake::default(),
        Monotonic::default(),
        hardware(),
    )
    .unwrap();
    assert!(!probe(), "refuse a live owner");
    production.wiring.handoff.request_release().unwrap();
    drop(production);
    assert!(
        !probe(),
        "refuse this boot even after the process and feeder exit"
    );
}

#[test]
fn a_surviving_cam_with_missing_command_identity_blocks_production() {
    for argv in ["\0", "/mnt/tmp/cam.new\0"] {
        let root = Root::new();
        root.process(25, "cam.new", &["/mnt/tmp/cam.new", "serve"]);
        root.write("proc/25/cmdline", argv);
        assert!(exclusion::conflicts(&root.0.join("proc"), 99).is_err());
    }
}

#[test]
fn every_media_diagnostic_refuses_an_existing_hdal_owner() {
    let _fork = PROCESS_SPAWN.read().unwrap();
    let root = Root::new();
    let lock = root.paths().lock();
    let _companion = cam::ownership::shared_diagnostic(&lock).unwrap();
    let media = std::fs::File::create(lock.with_file_name("cam-media.lock")).unwrap();
    media.try_lock().unwrap();
    for args in [
        vec!["dump"],
        vec!["probe-videocap"],
        vec!["serve", "--diagnostic", "--no-auth"],
    ] {
        let args: Vec<String> = args.into_iter().map(String::from).collect();
        assert!(cam::cmd::dispatch(&args, &lock).is_err(), "{args:?}");
    }
    // A media lease must still permit the separate feeder/peripheral shared lease.
    assert!(cam::ownership::shared_diagnostic(&lock).is_ok());
    drop(media);
    // On the host this reaches the unsupported-hardware command and returns a status,
    // proving that the refused invocation did not leave an ownership lock behind.
    #[cfg(not(target_arch = "arm"))]
    assert!(cam::cmd::dispatch(&["probe-videocap".into()], &lock).is_ok());
}

#[test]
fn nginx_rewritten_process_title_is_not_a_missing_camera_identity() {
    let root = Root::new();
    root.process(
        791,
        "nginx",
        &["nginx: master process /mnt/app/nginx -p /mnt/app/"],
    );
    assert!(
        exclusion::conflicts(&root.0.join("proc"), 99)
            .unwrap()
            .is_empty()
    );
    // A camera cannot evade ownership checks by imitating nginx's argv title.
    root.process(
        791,
        "cam.new",
        &["nginx: master process /mnt/app/nginx -p /mnt/app/"],
    );
    assert!(exclusion::conflicts(&root.0.join("proc"), 99).is_err());
}
