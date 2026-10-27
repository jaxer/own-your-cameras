fn main() {
    let paths = [
        "toolchain/start.S",
        "toolchain/compat.c",
        "toolchain/libraries.json",
        "toolchain/elf_compat.py",
        "scripts/mips-linker.py",
        "scripts/build-mips",
        "scripts/headers.py",
        "sdk/headers.lock.json",
        "sdk/imp_bridge.c",
        "sdk/imp_bridge.h",
        "sdk/imp_audio.c",
        "sdk/imp_audio.h",
        "sdk/imp_controls.c",
        "sdk/imp_controls.h",
        "sdk/imp_tuning.c",
        "sdk/imp_tuning.h",
        "sdk/include-compat/linux/videodev2.h",
    ];
    let mut fingerprint = 0xcbf29ce484222325_u64;
    for path in paths {
        println!("cargo:rerun-if-changed={path}");
        for byte in std::fs::read(path).expect("toolchain input") {
            fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
    }
    println!("cargo:rerun-if-env-changed=MI360_SYSROOT");
    println!("cargo:rerun-if-env-changed=MI360_HEADERS");
    println!("cargo:rerun-if-env-changed=MI360_ZIG_SHA256");
    if std::env::var("TARGET").as_deref() != Ok("mipsel-unknown-linux-uclibc") {
        return;
    }
    let compiler = std::env::var("MI360_ZIG_SHA256").expect("build using scripts/build-mips");
    for byte in compiler.bytes() {
        fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    // Content-addressed names change rustc's explicit link arguments when a recipe changes.
    // Recompiling an object hidden only in a linker wrapper is insufficient for Cargo.
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    for (name, source, extra) in [
        ("start", "toolchain/start.S", &[][..]),
        ("compat", "toolchain/compat.c", &["-O2", "-fPIC"][..]),
    ] {
        let object = out.join(format!("{name}-{fingerprint:016x}.o"));
        let status = std::process::Command::new("zig")
            .args(["cc", "-target", "mipsel-linux-gnu", "-mcpu=mips32r2"])
            .args(extra)
            .args(["-c", source, "-o"])
            .arg(&object)
            .status()
            .expect("Zig compiler");
        assert!(status.success(), "compile {source}");
        println!("cargo:rustc-link-arg={}", object.display());
    }
    if std::env::var_os("CARGO_FEATURE_IMP_VIDEO").is_some() {
        let headers =
            std::path::PathBuf::from(std::env::var_os("MI360_HEADERS").expect("MI360_HEADERS"));
        let status = std::process::Command::new("python3")
            .args(["scripts/headers.py", "--out"])
            .arg(&headers)
            .arg("--from-directory")
            .arg(&headers)
            .status()
            .expect("verify pinned headers");
        assert!(status.success(), "SDK header pins");
        let mut entries = std::fs::read_dir(headers.join("imp"))
            .expect("SDK header directory")
            .map(|e| e.expect("SDK header").path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            println!("cargo:rerun-if-changed={}", path.display());
            for byte in std::fs::read(path).expect("SDK header") {
                fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100000001b3);
            }
        }
        for (name, source) in [
            ("imp-bridge", "sdk/imp_bridge.c"),
            ("imp-audio", "sdk/imp_audio.c"),
            ("imp-controls", "sdk/imp_controls.c"),
            ("imp-tuning", "sdk/imp_tuning.c"),
        ] {
            let object = out.join(format!("{name}-{fingerprint:016x}.o"));
            let status = std::process::Command::new("zig")
                .args([
                    "cc",
                    "-target",
                    "mipsel-linux-gnueabihf",
                    "-mcpu=mips32r2",
                    "-O2",
                    "-fPIC",
                    "-ffunction-sections",
                    "-fdata-sections",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-Wno-comment",
                    "-I",
                    "sdk/include-compat",
                    "-I",
                ])
                .arg(&headers)
                .args(["-c", source, "-o"])
                .arg(&object)
                .status()
                .expect("compile IMP adapter");
            assert!(status.success(), "compile {source}");
            println!("cargo:rustc-link-arg={}", object.display());
        }
    }
}
