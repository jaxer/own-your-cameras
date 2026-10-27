//! Generate link-only HDAL metadata; runtime code stays on the camera.
use std::{env, path::PathBuf, process::Command};

fn main() {
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("arm") {
        return;
    }
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let workspace = root.join("../../..");
    let generator = workspace.join("scripts/link_stubs.py");
    println!("cargo:rerun-if-changed={}", generator.display());
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("abi/reolink-e1-zoom").display()
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR")).join("link-interfaces");
    let status = Command::new("python3")
        .arg(generator)
        .arg("reolink-e1-zoom")
        .arg("--output")
        .arg(&output)
        .status()
        .expect("Python3 is required to generate link interfaces");
    assert!(status.success(), "generate HDAL link interface");
    println!("cargo:rustc-link-search=native={}", output.display());
    println!("cargo:rustc-link-lib=dylib=hdal");
}
