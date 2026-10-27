//! Every file the daemon reads or creates goes through here: never through a symlink, never blocking on a
//! FIFO, bounded in size, and with error text that never contains bytes from the file.

use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::Path;

fn open_read(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
}

fn read_limited(file: File, limit: u64) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(io::Error::other(format!("larger than {limit} bytes")));
    }
    Ok(bytes)
}

/// Owned by this user, mode 0600, one link, regular.
fn private(file: &File) -> io::Result<()> {
    let meta = file.metadata()?;
    // SAFETY: geteuid has no arguments or memory effects.
    let uid = unsafe { libc::geteuid() };
    if !meta.is_file() || meta.nlink() != 1 || meta.uid() != uid || meta.mode() & 0o777 != 0o600 {
        return Err(io::Error::other(
            "expected an owned mode-0600 regular file without links",
        ));
    }
    Ok(())
}

/// A file holding secrets.
pub fn read_private(path: &Path, limit: u64) -> io::Result<String> {
    let file = open_read(path)?;
    private(&file)?;
    String::from_utf8(read_limited(file, limit)?)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not UTF-8"))
}

/// A regular file anyone may have written, such as a `/proc` record or the vendor's Wi-Fi profile.
pub fn read_regular(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let file = open_read(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("expected a regular file"));
    }
    read_limited(file, limit)
}

/// A regular file or a read-only device node, such as the calibration partition.
pub fn read_device_or_file(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let file = open_read(path)?;
    let kind = file.metadata()?.file_type();
    if !kind.is_file() && !kind.is_char_device() {
        return Err(io::Error::other(
            "expected a regular file or character device",
        ));
    }
    read_limited(file, limit)
}

/// Opens `path` for writing, empty, mode 0600. An existing file is checked before it is truncated, so a planted
/// link never truncates someone else's file.
pub fn create_private(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    private(&file)?;
    file.set_len(0)?;
    Ok(file)
}

/// Opens (creating it 0600 if absent) a lock file that only this user can have planted: an owned, unshared
/// regular file without group or other write permission.
pub fn open_lock(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    // SAFETY: geteuid has no arguments or memory effects.
    let uid = unsafe { libc::geteuid() };
    if !meta.is_file() || meta.nlink() != 1 || meta.uid() != uid || meta.mode() & 0o022 != 0 {
        return Err(io::Error::other(
            "runtime lock must be an owned, unshared regular file without group/other write permission",
        ));
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;

    struct Dir(PathBuf);
    impl Dir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("cam-files-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn file(&self, name: &str, bytes: &str, mode: u32) -> PathBuf {
            let path = self.0.join(name);
            std::fs::write(&path, bytes).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            path
        }
        fn fifo(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
            // SAFETY: a valid NUL-terminated path and a mode.
            assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
            path
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn private_files_must_be_owned_0600_single_link_and_within_the_limit() {
        let d = Dir::new("private");
        let ok = d.file("ok", "user:secret", 0o600);
        assert_eq!(read_private(&ok, 11).unwrap(), "user:secret");
        let err = read_private(&ok, 10).unwrap_err().to_string();
        assert!(!err.contains("secret"), "{err}");
        let open = d.file("open", "user:secret", 0o644);
        assert!(read_private(&open, 64).is_err());
        std::fs::hard_link(&ok, d.0.join("link")).unwrap();
        assert!(read_private(&ok, 64).is_err(), "two links");
        symlink(&open, d.0.join("sym")).unwrap();
        assert!(read_private(&d.0.join("sym"), 64).is_err());
        assert!(read_private(&d.fifo("fifo"), 64).is_err());
    }

    #[test]
    fn regular_reads_refuse_symlinks_fifos_and_devices() {
        let d = Dir::new("regular");
        let f = d.file("f", "12345", 0o644);
        assert_eq!(read_regular(&f, 5).unwrap(), b"12345");
        assert!(read_regular(&f, 4).is_err());
        symlink(&f, d.0.join("sym")).unwrap();
        assert!(read_regular(&d.0.join("sym"), 64).is_err());
        assert!(read_regular(&d.fifo("fifo"), 64).is_err());
        assert!(read_regular(Path::new("/dev/null"), 64).is_err());
    }

    #[test]
    fn device_or_file_reads_take_character_devices_but_not_fifos() {
        let d = Dir::new("device");
        let f = d.file("f", "12345", 0o644);
        assert!(read_device_or_file(&f, 4).is_err());
        assert_eq!(read_device_or_file(&f, 5).unwrap(), b"12345");
        assert_eq!(
            read_device_or_file(Path::new("/dev/null"), 64).unwrap(),
            b""
        );
        assert!(read_device_or_file(&d.fifo("fifo"), 64).is_err());
    }

    #[test]
    fn private_creation_checks_before_truncating() {
        let d = Dir::new("create");
        let new = d.0.join("new");
        create_private(&new).unwrap();
        assert_eq!(
            std::fs::metadata(&new).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let old = d.file("old", "keep", 0o600);
        create_private(&old).unwrap();
        assert_eq!(std::fs::read(&old).unwrap(), b"");
        let shared = d.file("shared", "keep", 0o644);
        assert!(create_private(&shared).is_err());
        assert_eq!(std::fs::read(&shared).unwrap(), b"keep");
        let target = d.file("target", "keep", 0o600);
        std::fs::hard_link(&target, d.0.join("hard")).unwrap();
        assert!(create_private(&d.0.join("hard")).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"keep");
        symlink(&target, d.0.join("sym")).unwrap();
        assert!(create_private(&d.0.join("sym")).is_err());
    }

    #[test]
    fn locks_refuse_group_writable_linked_or_symlinked_files() {
        let d = Dir::new("lock");
        assert!(open_lock(&d.0.join("fresh")).is_ok());
        let readable = d.file("readable", "", 0o644);
        assert!(open_lock(&readable).is_ok());
        let writable = d.file("writable", "", 0o664);
        assert!(open_lock(&writable).is_err());
        std::fs::hard_link(&readable, d.0.join("hard")).unwrap();
        assert!(open_lock(&readable).is_err());
        symlink(d.0.join("fresh"), d.0.join("sym")).unwrap();
        assert!(open_lock(&d.0.join("sym")).is_err());
    }
}
