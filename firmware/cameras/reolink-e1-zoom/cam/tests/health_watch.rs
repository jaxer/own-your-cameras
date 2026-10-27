use cam::runtime::health_watch::read_record;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::Duration;

struct Root(PathBuf);
impl Root {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "cam-health-read-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn a_fifo_health_record_is_rejected_without_waiting_for_a_writer() {
    let root = Root::new();
    let path = root.0.join("cam-health-test.json");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let dir = root.0.clone();
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        tx.send(read_record(&dir, "test")).unwrap();
    });
    let result = rx.recv_timeout(Duration::from_secs(1));
    // Unblock the old implementation before asserting, so even a red run leaves no thread behind.
    if result.is_err() {
        let mut writer = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        writer.write_all(b"{}").unwrap();
        drop(writer);
    }
    reader.join().unwrap();
    assert_eq!(result.unwrap(), None);
}

#[test]
fn only_bounded_regular_health_records_are_read() {
    let root = Root::new();
    let path = root.0.join("cam-health-test.json");
    fs::write(&path, "{}").unwrap();
    assert_eq!(read_record(&root.0, "test").as_deref(), Some("{}"));
    fs::write(&path, "x".repeat(1025)).unwrap();
    assert_eq!(read_record(&root.0, "test"), None);
    fs::remove_file(&path).unwrap();
    let target = root.0.join("other");
    fs::write(&target, "{}").unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert_eq!(read_record(&root.0, "test"), None);
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert_eq!(read_record(&root.0, "test"), None);
    assert_eq!(read_record(&root.0, "../test"), None);
}
