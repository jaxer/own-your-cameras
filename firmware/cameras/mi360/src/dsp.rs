//! Loader qualification for the shipped optional audio DSP provider.
//!
//! This module never creates an AEC/NS processor or opens an audio device. The
//! lock records exact ELF imports and relocated MIPS GOT slots, not an ABI claim
//! from a version string. Loading C++ also runs its pinned startup constructors;
//! target load/TLS and acoustic acceptance remain separate runtime gates.
use std::io;

#[cfg(any(test, all(target_os = "linux", target_arch = "mips")))]
mod qualification {
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Lock {
        pub version: u32,
        pub libraries: Vec<Library>,
        pub profile: Profile,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Library {
        pub name: String,
        pub path: String,
        pub bytes: u64,
        pub sha256: String,
        pub anchor: String,
        pub anchor_value: u32,
        pub imports: Vec<Import>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Import {
        pub name: String,
        pub weak: bool,
        pub got: u32,
        pub providers: Vec<Provider>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Provider {
        pub library: usize,
        pub value: u32,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Profile {
        pub path: String,
        pub bytes: u64,
        pub sha256: String,
    }

    pub(super) fn lock() -> std::io::Result<Lock> {
        let lock: Lock = serde_json::from_str(include_str!("../sdk/dsp.lock.json"))?;
        if lock.version != 1 || lock.libraries.len() != 9 {
            return Err(std::io::Error::other("unsupported DSP qualification lock"));
        }
        Ok(lock)
    }

    pub(super) fn binding_is_qualified(actual: usize, providers: &[usize], weak: bool) -> bool {
        (weak && actual == 0) || (actual != 0 && providers.contains(&actual))
    }
}

#[cfg(all(target_os = "linux", target_arch = "mips"))]
mod target {
    use super::qualification::{self, Import, Library, Lock};
    use super::*;
    use sha2::{Digest, Sha256};
    use std::ffi::{CString, c_char, c_int, c_void};
    use std::fs::{File, Metadata};
    use std::io::Read;
    use std::marker::PhantomData;
    use std::os::unix::fs::MetadataExt;
    use std::path::PathBuf;
    use std::rc::Rc;

    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(path: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
        fn dlerror() -> *const c_char;
        fn dlclose(handle: *mut c_void) -> c_int;
    }

    struct Identity {
        canonical: PathBuf,
        metadata: Metadata,
    }
    struct Loaded {
        handle: *mut c_void,
        base: usize,
    }
    struct Mapping {
        begin: usize,
        end: usize,
        readable: bool,
        inode: u64,
        path: PathBuf,
    }

    fn checked_file(path: &str, bytes: u64, expected: &str) -> io::Result<Identity> {
        let mut file = File::open(path)?;
        let before = file.metadata()?;
        if !before.is_file() || before.len() != bytes {
            return Err(io::Error::other(format!(
                "{path} has unqualified size/type"
            )));
        }
        let mut hash = Sha256::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hash.update(&buffer[..n]);
        }
        let after = file.metadata()?;
        if before.dev() != after.dev()
            || before.ino() != after.ino()
            || before.len() != after.len()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
            || format!("{:x}", hash.finalize()) != expected
        {
            return Err(io::Error::other(format!(
                "{path} differs from the qualified build"
            )));
        }
        Ok(Identity {
            canonical: std::fs::canonicalize(path)?,
            metadata: after,
        })
    }

    fn mappings() -> io::Result<Vec<Mapping>> {
        let text = std::fs::read_to_string("/proc/self/maps")?;
        text.lines()
            .filter_map(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                // Qualified camera library paths contain no whitespace.
                if fields.len() != 6 || !fields[5].starts_with('/') {
                    return None;
                }
                Some((|| {
                    let (begin, end) = fields[0]
                        .split_once('-')
                        .ok_or_else(|| io::Error::other("invalid mapping range"))?;
                    let number = |value| usize::from_str_radix(value, 16).map_err(io::Error::other);
                    Ok(Mapping {
                        begin: number(begin)?,
                        end: number(end)?,
                        readable: fields[1].starts_with('r'),
                        inode: fields[4].parse().map_err(io::Error::other)?,
                        path: PathBuf::from(fields[5]),
                    })
                })())
            })
            .collect()
    }

    fn owns_mapping(identity: &Identity, address: usize, bytes: usize, maps: &[Mapping]) -> bool {
        let Some(end) = address.checked_add(bytes) else {
            return false;
        };
        maps.iter().any(|map| {
            map.readable
                && map.begin <= address
                && end <= map.end
                && map.inode == identity.metadata.ino()
                && std::fs::canonicalize(&map.path).is_ok_and(|path| path == identity.canonical)
        })
    }

    fn symbol(handle: *mut c_void, name: &str) -> io::Result<usize> {
        let c_name = CString::new(name).map_err(io::Error::other)?;
        // SAFETY: caller supplies a live dlopen handle or RTLD_DEFAULT (null).
        unsafe {
            let _ = dlerror();
            let value = dlsym(handle, c_name.as_ptr());
            if !dlerror().is_null() {
                return Ok(0);
            }
            Ok(value as usize)
        }
    }

    fn provider_addresses(import: &Import, loaded: &[Loaded]) -> io::Result<Vec<usize>> {
        import
            .providers
            .iter()
            .map(|provider| {
                let library = loaded.get(provider.library).ok_or_else(|| {
                    io::Error::other("DSP lock provider is not loaded before its consumer")
                })?;
                library
                    .base
                    .checked_add(provider.value as usize)
                    .ok_or_else(|| io::Error::other("DSP provider address overflow"))
            })
            .collect()
    }

    fn preflight(library: &Library, loaded: &[Loaded]) -> io::Result<()> {
        for import in &library.imports {
            let providers = provider_addresses(import, loaded)?;
            let global = symbol(std::ptr::null_mut(), &import.name)?;
            if global != 0 && !providers.contains(&global) {
                return Err(io::Error::other(format!(
                    "{} import {} has an unqualified global provider",
                    library.name, import.name
                )));
            }
            // A local C++ dependency can satisfy DSP even when RTLD_DEFAULT
            // does not expose it. Check that exact pinned handle and ELF VA.
            let mut available = false;
            for (provider, expected) in import.providers.iter().zip(&providers) {
                if symbol(loaded[provider.library].handle, &import.name)? == *expected {
                    available = true;
                }
            }
            if !available && !import.weak {
                return Err(io::Error::other(format!(
                    "{} import {} has no qualified provider",
                    library.name, import.name
                )));
            }
        }
        Ok(())
    }

    fn verify_got(
        library: &Library,
        index: usize,
        loaded: &[Loaded],
        identity: &Identity,
    ) -> io::Result<()> {
        let maps = mappings()?;
        for import in &library.imports {
            let slot = loaded[index]
                .base
                .checked_add(import.got as usize)
                .ok_or_else(|| io::Error::other("DSP GOT address overflow"))?;
            if slot & 3 != 0 || !owns_mapping(identity, slot, 4, &maps) {
                return Err(io::Error::other(
                    "DSP GOT is outside its pinned readable mapping",
                ));
            }
            // SAFETY: exact-file ELF slot, O32-aligned and fully inside the
            // retained library's readable file mapping verified above. All
            // loader operations are serialized on this owner thread.
            let actual = unsafe { std::ptr::read_volatile(slot as *const u32) } as usize;
            let providers = provider_addresses(import, loaded)?;
            if !qualification::binding_is_qualified(actual, &providers, import.weak) {
                return Err(io::Error::other(format!(
                    "{} import {} has an unqualified relocated GOT slot",
                    library.name, import.name
                )));
            }
        }
        Ok(())
    }

    /// Keep on the media owner thread. Unclosed handles intentionally stay
    /// resident until process exit, including every partial or poisoned path.
    pub struct Dsp {
        loaded: Vec<Loaded>,
        _main_thread: PhantomData<Rc<()>>,
    }

    impl Dsp {
        /// Pin every file before loading providers. A failure after dlopen
        /// deliberately retains acquired references; restart the process before
        /// another attempt instead of accumulating repeated partial owners.
        pub fn open() -> io::Result<Self> {
            let Lock {
                libraries, profile, ..
            } = qualification::lock()?;
            let identities: Vec<_> = libraries
                .iter()
                .map(|lib| checked_file(&lib.path, lib.bytes, &lib.sha256))
                .collect::<io::Result<_>>()?;
            checked_file(&profile.path, profile.bytes, &profile.sha256)?;
            match std::fs::symlink_metadata("/tmp/app") {
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
                Ok(_) => {
                    return Err(io::Error::other(
                        "AEC PCM dump sentinel /tmp/app is present",
                    ));
                }
            }
            let mut dsp = Self {
                loaded: Vec::new(),
                _main_thread: PhantomData,
            };
            for (index, library) in libraries.iter().enumerate() {
                preflight(library, &dsp.loaded)?;
                let path = CString::new(library.path.as_str()).map_err(io::Error::other)?;
                // MIPS uClibc RTLD_NOW=2, RTLD_GLOBAL=4. Math is also a
                // qualified global provider for the core SDK; C++/DSP remain
                // local and are borrowed through the explicit DSP handle.
                let flags = if library.name == "math" { 6 } else { 2 };
                let handle = unsafe { dlopen(path.as_ptr(), flags) };
                if handle.is_null() {
                    return Err(io::Error::other(format!(
                        "cannot load pinned {}",
                        library.name
                    )));
                }
                // Retain immediately: even failed constructor/import checks
                // must not accidentally dlclose code/state with unknown users.
                dsp.loaded.push(Loaded { handle, base: 0 });
                let anchor = symbol(handle, &library.anchor)?;
                let base = anchor
                    .checked_sub(library.anchor_value as usize)
                    .filter(|base| *base != 0 && base & 0xfff == 0)
                    .ok_or_else(|| io::Error::other("DSP anchor has unqualified ELF layout"))?;
                if !owns_mapping(&identities[index], anchor, 4, &mappings()?) {
                    return Err(io::Error::other(
                        "DSP anchor resolves outside the pinned library",
                    ));
                }
                dsp.loaded[index].base = base;
                verify_got(library, index, &dsp.loaded, &identities[index])?;
            }
            Ok(dsp)
        }

        /// Borrow for t31_audio_open with effect4 (AEC capable) or1 (NS).
        ///
        /// # Safety
        /// Caller must retain this owner through every SDK/DSP user and prevent
        /// all calls after successful close_after_media. No ownership transfer.
        pub unsafe fn borrow_handle(&self) -> *mut c_void {
            self.loaded
                .last()
                .map_or(std::ptr::null_mut(), |lib| lib.handle)
        }

        /// Optional unload after a fully successful media session shutdown.
        /// Default Rust destruction deliberately never calls dlclose.
        ///
        /// # Safety
        /// All audio effects/channels/workers and the shared video/System owner
        /// must have closed successfully; no borrowed pointer may remain. Never
        /// call after an uncertain acquisition or any poisoned SDK operation.
        pub unsafe fn close_after_media(mut self) -> io::Result<()> {
            while let Some(library) = self.loaded.last() {
                if unsafe { dlclose(library.handle) } != 0 {
                    return Err(io::Error::other(
                        "DSP unload failed; remaining providers retained",
                    ));
                }
                self.loaded.pop();
            }
            Ok(())
        }
    }
}

#[cfg(all(target_os = "linux", target_arch = "mips"))]
pub use target::Dsp;

#[cfg(not(all(target_os = "linux", target_arch = "mips")))]
pub struct Dsp;

#[cfg(not(all(target_os = "linux", target_arch = "mips")))]
impl Dsp {
    pub fn open() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "DSP is available only on the target camera",
        ))
    }

    /// # Safety
    /// Host adapter is unsupported and returns no target provider.
    pub unsafe fn borrow_handle(&self) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }

    /// # Safety
    /// Mirrors the target owner's successful-media-shutdown precondition.
    pub unsafe fn close_after_media(self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "DSP is available only on the target camera",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::qualification::{binding_is_qualified, lock};

    #[test]
    fn exact_import_lock_has_only_earlier_pinned_providers() {
        let lock = lock().unwrap();
        assert_eq!(lock.libraries[7].name, "stdcpp");
        assert_eq!(lock.libraries[8].name, "dsp");
        for (index, library) in lock.libraries.iter().enumerate() {
            assert!(library.path.starts_with('/'));
            assert!(library.bytes > 0 && library.sha256.len() == 64);
            assert!(!library.anchor.is_empty() && library.anchor_value > 0);
            for import in &library.imports {
                assert!(!import.name.is_empty() && import.got & 3 == 0);
                assert!(import.weak || !import.providers.is_empty());
                assert!(
                    import
                        .providers
                        .iter()
                        .all(|p| p.library < index && p.value > 0)
                );
            }
        }
        assert_eq!(
            lock.libraries[7].imports.iter().filter(|i| !i.weak).count(),
            119
        );
        assert_eq!(
            lock.libraries[8].imports.iter().filter(|i| !i.weak).count(),
            165
        );
        assert_eq!(lock.profile.path, "/etc/webrtc_profile.ini");
        assert_eq!(lock.profile.bytes, 863);
        assert_eq!(lock.profile.sha256.len(), 64);
    }

    #[test]
    fn missing_strong_and_wrong_or_stub_got_are_rejected() {
        let providers = [0x77001000, 0x77102000];
        assert!(!binding_is_qualified(0, &providers, false));
        assert!(!binding_is_qualified(0x77001004, &providers, false));
        assert!(!binding_is_qualified(0x6a1a0, &providers, false));
        assert!(binding_is_qualified(providers[0], &providers, false));
        assert!(binding_is_qualified(providers[1], &providers, false));
        assert!(binding_is_qualified(0, &[], true));
        assert!(!binding_is_qualified(0x77001000, &[], true));
    }

    #[cfg(not(all(target_os = "linux", target_arch = "mips")))]
    #[test]
    fn host_adapter_refuses_without_loading_a_provider() {
        let error = super::Dsp::open().err().unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    }
}
