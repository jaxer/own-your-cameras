//! Execute the actual image launcher and compiled helper across isolated process/OS faults.
#[test]
fn image_launcher_fault_matrix() {
    let status = std::process::Command::new("python3")
        .args(["-I", "scripts/launcher_tests.py"])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CAM_LAUNCH_BIN", env!("CARGO_BIN_EXE_cam-launch"))
        .status()
        .expect("run launcher matrix");
    assert!(status.success(), "launcher fault matrix failed");
}
