//! Generates the bindings and the table of header struct sizes, both for the camera's ABI
//! (armv7 Linux, ILP32) whatever the host is. System headers are replaced by `shim/`.

use std::env;
use std::fmt::Write as _;
use std::path::PathBuf;

mod layout;

const TARGET: &str = "--target=armv7-unknown-linux-gnueabihf";

fn builder() -> bindgen::Builder {
    bindgen::Builder::default()
        .header("wrapper.h")
        .clang_args([
            TARGET,
            "-mcpu=cortex-a9",
            "-nostdlibinc",
            "-isystem",
            "shim",
            "-I",
            "vendor/hdal-include",
        ])
        .allowlist_file(".*vendor/hdal-include/.*")
        .derive_default(true)
        .default_enum_style(bindgen::EnumVariation::Consts)
        .prepend_enum_name(false)
        .ctypes_prefix("crate::ctypes")
        .use_core()
}

fn main() {
    if !std::path::Path::new("vendor/hdal-include/hdal.h").exists() {
        eprintln!("HDAL headers missing: run `make headers`");
        std::process::exit(1);
    }
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=vendor/hdal-include");
    println!("cargo:rerun-if-changed=shim");
    if env::var_os("LIBCLANG_PATH").is_none() {
        let brew = "/opt/homebrew/opt/llvm/lib";
        if std::path::Path::new(brew).exists() {
            // SAFETY: build scripts are single-threaded at this point.
            unsafe { env::set_var("LIBCLANG_PATH", brew) };
        }
    }
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("arm") {
        builder()
            .layout_tests(false)
            .generate()
            .expect("host bindgen")
            .write_to_file(out.join("bindings.rs"))
            .expect("write bindings.rs");
    }

    // Sizes as clang computes them for the camera: bindgen's layout tests carry them as literals.
    let tests = builder()
        .layout_tests(true)
        .generate()
        .expect("bindgen layout")
        .to_string();
    let sizes = layout::extract_sizes(&tests).expect("complete ARM layout evidence");
    // These const assertions check Rust sizes, alignments AND offsets during every target build.
    std::fs::write(out.join("arm_bindings.rs"), &tests).expect("write arm_bindings.rs");
    let mut table = String::from("pub const HEADER_SIZES: &[(&str, usize)] = &[\n");
    for (name, size) in &sizes {
        writeln!(table, "    ({name:?}, {size}),").expect("write to String");
    }
    table.push_str("];\n");
    std::fs::write(out.join("header_sizes.rs"), table).expect("write header_sizes.rs");

    // The camera is the only place libhdal.so can run; host builds do not link it. The search path is
    // `-L vendor/sysroot/usr/lib` in .cargo/config.toml (scripts/fetch-sysroot).
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("arm") {
        let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
        println!(
            "cargo:rustc-link-search=native={}",
            root.join("../vendor/sysroot/usr/lib").display()
        );
        println!("cargo:rustc-link-lib=dylib=hdal");
    }
}
