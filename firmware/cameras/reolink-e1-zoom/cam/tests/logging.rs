use std::io::{self, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn a_blocked_log_sink_does_not_backpressure_the_daemon() {
    struct Blocked {
        entered: mpsc::SyncSender<()>,
        release: mpsc::Receiver<()>,
    }
    impl Write for Blocked {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let _ = self.entered.try_send(());
            self.release.recv().unwrap();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let (entered, waiting) = mpsc::sync_channel(1);
    let (release, resume) = mpsc::channel();
    let (input, mut output) = UnixStream::pair().unwrap();
    output
        .set_write_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let drain = std::thread::spawn(move || {
        cam::logcap::drain(
            input,
            Blocked {
                entered,
                release: resume,
            },
            cam::logcap::CAP_BYTES,
        )
    });
    output.write_all(b"first log record").unwrap();
    waiting.recv_timeout(Duration::from_secs(2)).unwrap();
    // More than the OS socket/pipe capacity, while the actual sink cannot write.
    let result = output.write_all(&vec![b'x'; 4 * 1024 * 1024]);
    drop(output);
    // Rescue the baseline too; every accepted chunk may call Write once more.
    for _ in 0..1024 {
        release.send(()).ok();
    }
    drain.join().unwrap();
    assert!(
        result.is_ok(),
        "blocked log sink stalled the producer: {result:?}"
    );
}

#[test]
fn redirect_refuses_linked_log_files_without_overwriting_them() {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("cam-log-links-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source");
    let link = root.join("log");
    let target = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .unwrap();
    let mut refused = Vec::new();
    for hard in [false, true] {
        std::fs::write(&source, b"must survive").unwrap();
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o600)).unwrap();
        if hard {
            std::fs::hard_link(&source, &link).unwrap();
        } else {
            std::os::unix::fs::symlink(&source, &link).unwrap();
        }
        let result = cam::logcap::redirect(&link, 64, &[target.as_raw_fd()]);
        refused.push(result.is_err() && std::fs::read(&source).unwrap() == b"must survive");
        std::fs::remove_file(&link).unwrap();
    }
    drop(target);
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(refused, [true, true]);
}

#[test]
fn failed_redirect_leaves_the_original_output_descriptors_usable() {
    use std::os::fd::AsRawFd;
    let path = std::env::temp_dir().join(format!("cam-log-failed-{}", std::process::id()));
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .unwrap();
    assert!(cam::logcap::redirect(&path, 64, &[output.as_raw_fd(), -1]).is_err());
    let result = output.write_all(b"startup error still reportable");
    let _ = std::fs::remove_file(path);
    assert!(
        result.is_ok(),
        "partial redirect broke the original sink: {result:?}"
    );
}
