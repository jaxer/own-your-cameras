//! Boot phases for image/start_app: reserve cannot start a graph; serve cannot select vendor.
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

fn quiet(command: &mut Command) -> &mut Command {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
}

fn bounded(child: &mut Child, limit: Duration) -> io::Result<bool> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.success());
        }
        if start.elapsed() >= limit {
            child.kill()?;
            // Do not block on an uninterruptible kernel syscall. init reaps it on exit.
            let _ = child.try_wait();
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn reboot(root: &Path) {
    let start = Instant::now();
    let limit = Duration::from_secs(5);
    if let Ok(mut child) = quiet(&mut Command::new(root.join("sbin/reboot"))).spawn() {
        let _ = bounded(&mut child, limit);
    }
    // A successful exit only acknowledges a request. Still running five seconds later means escalate,
    // even when the first command failed to spawn.
    std::thread::sleep(limit.saturating_sub(start.elapsed()));
    if let Ok(mut child) = quiet(Command::new(root.join("sbin/reboot")).arg("-f")).spawn() {
        let _ = bounded(&mut child, limit);
    }
}

/// No watchdog ownership: this independent process covers supervisor death/hang before S00 handoff.
/// EOF or five seconds without a pulse requests reboot, always after a reservation.
fn guard(root: &Path) {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut byte = [0];
        while io::stdin().read_exact(&mut byte).is_ok() {
            if tx.send(byte[0]).is_err() {
                break;
            }
        }
    });
    while let Ok(b'p') = rx.recv_timeout(Duration::from_secs(5)) {}
    reboot(root);
}

struct Guard {
    stream: UnixStream,
    child: Child,
}
impl Guard {
    fn start(root: &Path) -> io::Result<Self> {
        let (stream, input) = UnixStream::pair()?;
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;
        let child = quiet(
            Command::new(std::env::current_exe()?)
                .arg("--guard")
                .arg(root),
        )
        .stdin(Stdio::from(OwnedFd::from(input)))
        .spawn()?;
        Ok(Self { stream, child })
    }
    fn pulse(&mut self) -> io::Result<()> {
        if self.child.try_wait()?.is_some() {
            return Err(io::Error::other("guard exited"));
        }
        self.stream.write_all(b"p")
    }
}

fn small_file(path: &Path, limit: u64) -> io::Result<String> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.len() > limit {
        return Err(io::Error::other("not a bounded regular file"));
    }
    let mut text = String::new();
    File::open(path)?
        .take(limit + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > limit {
        return Err(io::Error::other("file grew beyond limit"));
    }
    Ok(text)
}

fn attempts(para: &Path) -> io::Result<u8> {
    if para.join("cam.crashes.tmp").symlink_metadata().is_ok() {
        return Err(io::Error::other("interrupted counter write"));
    }
    match small_file(&para.join("cam.crashes"), 2) {
        Ok(s) if matches!(s.as_str(), "0\n" | "1\n" | "2\n" | "3\n") => Ok(s.as_bytes()[0] - b'0'),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        _ => Err(io::Error::other("invalid counter")),
    }
}

fn persist(para: &Path, count: u8) -> io::Result<()> {
    let temp = para.join("cam.crashes.tmp");
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp)?;
    writeln!(f, "{count}")?;
    f.sync_all()?;
    fs::rename(temp, para.join("cam.crashes"))?;
    File::open(para)?.sync_all()
}

// Reservation may spawn only the read-only preflight. It cannot reach serve or a guard.
fn reserve(root: &Path) -> io::Result<u8> {
    let para = root.join("mnt/para");
    let binary = root.join("usr/bin/cam");
    let executable =
        fs::metadata(&binary).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0);
    if !executable || !para.join("cam.enabled").is_file() {
        return Ok(10);
    }
    let mut check = match quiet(Command::new(&binary).arg("preflight")).spawn() {
        Ok(child) => child,
        Err(_) => return Ok(10),
    };
    if !bounded(&mut check, Duration::from_secs(2))? {
        return Ok(10);
    }
    let count = match attempts(&para) {
        Ok(n) if n < 3 => n + 1,
        _ => return Ok(10),
    };
    let boot = match small_file(&root.join("proc/sys/kernel/random/boot_id"), 60) {
        Ok(boot) => boot,
        Err(_) => return Ok(10),
    };
    let token = format!("{}-{count}", boot.trim());
    if !cam::runtime::valid_attempt(boot.trim())
        || !cam::runtime::valid_attempt(&token)
        || persist(&para, count).is_err()
    {
        return Ok(10);
    }
    Ok(0)
}

fn serve(root: &Path) -> io::Result<u8> {
    let mut guard = Guard::start(root)?;
    guard.pulse()?;
    fs::create_dir(root.join("mnt/tmp/cam-guard-up"))?;
    let para = root.join("mnt/para");
    let count = attempts(&para)?;
    let boot = small_file(&root.join("proc/sys/kernel/random/boot_id"), 60)?;
    let token = format!("{}-{count}", boot.trim());
    if count == 0
        || !cam::runtime::valid_attempt(boot.trim())
        || !cam::runtime::valid_attempt(&token)
    {
        return Err(io::Error::other("invalid active reservation"));
    }
    let binary = root.join("usr/bin/cam");
    let mut child = Command::new(binary)
        .args(["serve", "--attempt", &token])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let start = Instant::now();
    let mut watch = cam::runtime::health_watch::Watch::new(&token, child.id());
    let health_dir = root.join("mnt/tmp");
    let (mut publishing, mut qualified) = (false, false);
    loop {
        guard.pulse()?;
        if child.try_wait()?.is_some() {
            break;
        }
        let record = cam::runtime::health_watch::read_record(&health_dir, &token);
        let now = start.elapsed();
        match watch.step(record.as_deref(), true, now) {
            Some("qualified") => {
                if count != 1 {
                    persist(&para, 1)?;
                }
                qualified = true;
            }
            Some("unqualified") => break,
            _ => (),
        }
        if watch.progressing(now) {
            publishing = true;
        } else if publishing || now >= Duration::from_secs(15) {
            break;
        }
        if !qualified && now >= Duration::from_secs(300) {
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    Ok(1)
}

// Reserve has only 0/10 outcomes. Serve has no vendor-authorizing status.
// Never return 126/127 or forward child statuses: those mean exec/loader failure.
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 || !Path::new(&args[2]).is_absolute() {
        return ExitCode::from(2);
    }
    let root = Path::new(&args[2]);
    match args[1].as_str() {
        "--reserve" => ExitCode::from(reserve(root).unwrap_or(10)),
        "--serve" => {
            let _ = serve(root);
            reboot(root);
            ExitCode::FAILURE
        }
        "--reset" => {
            reboot(root);
            ExitCode::SUCCESS
        }
        "--guard" => {
            guard(root);
            ExitCode::SUCCESS
        }
        "--preflight" => ExitCode::from(u8::from(cam::preflight::check(root).is_err())),
        _ => ExitCode::from(2),
    }
}
