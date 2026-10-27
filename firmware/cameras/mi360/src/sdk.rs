//! First narrow IMP adapter. This getter does not initialize or own media hardware.
//! The shipped getter at0x1cc80 calls sprintf(buffer,"IMP-%s","1.1.2").

#[cfg(all(target_os = "linux", target_arch = "mips"))]
pub fn verify_library() -> std::io::Result<()> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    for (path, expected) in [
        (
            "/mnt/usr/lib/libimp.so",
            "347b7a602b535541fa9594d100aee3bb87d6df35dec60fa9e16a94cabcffbca4",
        ),
        (
            "/mnt/usr/lib/libalog.so",
            "843dd5b5d521abd4c7b5582d1d46307eb5572c2246d242f8c7dbe0f1675bc8b6",
        ),
        (
            "/lib/libm.so.0",
            "be736dc62d9635f96b0c111c25e88aac6cca3481901f23b877f52688db5cb850",
        ),
    ] {
        let mut file = std::fs::File::open(path)?;
        let mut hash = Sha256::new();
        let mut data = [0_u8; 8192];
        loop {
            let n = file.read(&mut data)?;
            if n == 0 {
                break;
            }
            hash.update(&data[..n]);
        }
        if format!("{:x}", hash.finalize()) != expected {
            return Err(std::io::Error::other(format!(
                "{path} differs from the qualified ABI build"
            )));
        }
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "mips"))]
pub fn version() -> std::io::Result<String> {
    verify_library()?;
    use std::ffi::{c_char, c_int, c_void};
    use std::io;

    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(path: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(library: *mut c_void, symbol: *const c_char) -> *mut c_void;
        fn dlclose(library: *mut c_void) -> c_int;
    }
    struct Library(*mut c_void);
    impl Drop for Library {
        fn drop(&mut self) {
            // SAFETY: this is the exact nonnull handle returned by dlopen below.
            unsafe { dlclose(self.0) };
        }
    }
    // RTLD_NOW=2 for this Linux ABI. Absolute path, no arbitrary library selection.
    // The shipped libimp has no application entries in its constructor table.
    let handle = unsafe { dlopen(c"/mnt/usr/lib/libimp.so".as_ptr(), 2) };
    if handle.is_null() {
        return Err(io::Error::other("cannot load the camera IMP library"));
    }
    let library = Library(handle);
    let symbol = unsafe { dlsym(library.0, c"IMP_System_GetVersion".as_ptr()) };
    if symbol.is_null() {
        return Err(io::Error::other("missing IMP version getter"));
    }
    // SAFETY: exact shipped getter's one-pointer O32 ABI recovered from instructions.
    // It writes the static string IMP-1.1.2 (10 bytes including NUL), no SDK session calls.
    let getter: unsafe extern "C" fn(*mut u8) -> c_int = unsafe { std::mem::transmute(symbol) };
    let mut bytes = [0xa5_u8; 128];
    let result = unsafe { getter(bytes.as_mut_ptr()) };
    if result != 0 || bytes[64..].iter().any(|b| *b != 0xa5) {
        return Err(io::Error::other(
            "IMP version getter failed or exceeded its buffer",
        ));
    }
    let nul = bytes[..64]
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(|| io::Error::other("unterminated IMP version"))?;
    String::from_utf8(bytes[..nul].to_vec())
        .map_err(|_| io::Error::other("invalid IMP version text"))
}

#[cfg(not(all(target_os = "linux", target_arch = "mips")))]
pub fn version() -> std::io::Result<String> {
    Err(std::io::Error::other(
        "IMP is available only on the target camera",
    ))
}
