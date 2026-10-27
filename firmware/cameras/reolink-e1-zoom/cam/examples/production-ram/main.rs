//! Operator-run acceptance of the actual production owner from a private RAM root.
//! No image/flash writes. Vendor device must be absent; reboot is mandatory afterwards.
//! Root supplies real credentials/config and links to the unchanged OS artifacts.
#[cfg(target_arch = "arm")]
mod abi_guard;

#[cfg(not(target_arch = "arm"))]
fn main() {}

#[cfg(target_arch = "arm")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use cam::runtime::{Monotonic, production};
    use cam::signals;
    use std::{path::Path, sync::atomic::Ordering, time::Duration};
    let root = std::env::args().nth(1).ok_or("RAM root required")?;
    if !root.starts_with("/mnt/tmp/") {
        return Err("RAM root must be under /mnt/tmp".into());
    }
    if std::env::args().any(|arg| arg == "--abi-guard") {
        abi_guard::enable();
    }
    signals::install()?;
    if std::env::args().any(|arg| arg == "--probe") {
        let report =
            hdal::probe_videocap(&mut hdal::sys::Sys::new(), &hdal::ProbeConfig::os05a10())?;
        print!("{}", report.report());
        if !report.cleanup_errors.is_empty() {
            return Err("probe cleanup failed".into());
        }
        return Ok(());
    }
    let started = production::start(
        &production::Paths::rooted(Path::new(&root), 554),
        "gate59-ram",
        &signals::STOP,
        cam::cmd::camera_hdal(),
        Monotonic::default(),
        cam::mqtt::device::opener(),
    )
    .map_err(|e| e.to_string())?;
    let seconds = if std::env::args().any(|arg| arg == "--short") {
        80
    } else {
        240
    };
    let main = started.wiring.hubs.main.clone();
    let sub = started.wiring.hubs.sub.clone();
    let timer = std::thread::spawn(move || {
        for tick in 0..seconds {
            if signals::STOP.load(Ordering::Acquire) {
                return;
            }
            println!(
                "sample={tick} retained={} main_viewers={} sub_viewers={}",
                main.retained_bytes(),
                main.viewers(),
                sub.viewers()
            );
            let mem = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
            let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
            for line in mem
                .lines()
                .filter(|l| l.starts_with("MemAvailable:") || l.starts_with("MemFree:"))
            {
                println!("sample={tick} {line}");
            }
            for line in status
                .lines()
                .filter(|l| l.starts_with("VmRSS:") || l.starts_with("Threads:"))
            {
                println!("sample={tick} {line}");
            }
            std::thread::sleep(Duration::from_secs(1));
        }
        signals::STOP.store(true, Ordering::Release);
    });
    let network = started.network;
    let outcome = production::run(started.wiring);
    signals::STOP.store(true, Ordering::Release);
    let _ = timer.join();
    let code = match &outcome.result {
        Ok(report) => {
            println!("production_ram_ok {report:?}");
            0
        }
        Err(error) => {
            eprintln!("production_ram_failed {error}");
            1
        }
    };
    let host = network.lock().ok().and_then(|mut slot| slot.take());
    if let Some(host) = host {
        host.exit_preserving_network(code);
    }
    std::process::exit(code)
}
