//! Finds other owners of the hardware before production starts: the vendor media/watchdog daemons or a `cam`
//! diagnostic. It reports them and never kills.

use std::fs;
use std::io;
use std::path::Path;

/// Vendor daemons that own HDAL (`device`), the watchdog (`router`) or the RTSP port.
const VENDOR: [&str; 4] = ["device", "router", "rtsp", "recorder"];

fn is_cam(name: &str) -> bool {
    name == "cam" || name.starts_with("cam.") || name.starts_with("cam-")
}

/// Lists conflicting processes found under `proc_root` (`/proc`), excluding `own_pid`.
pub fn conflicts(proc_root: &Path, own_pid: u32) -> io::Result<Vec<String>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(proc_root)? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        if pid == own_pid {
            continue;
        }
        let process = entry.path();
        let identity = (|| {
            let comm = fs::read_to_string(process.join("comm"))?;
            let cmdline = fs::read(process.join("cmdline"))?;
            if comm.trim().is_empty() {
                return Err(io::Error::other("empty process name"));
            }
            if cmdline.is_empty() {
                let stat = fs::read_to_string(process.join("stat"))?;
                let fields: Vec<_> = stat
                    .rsplit_once(')')
                    .map(|(_, rest)| rest.split_whitespace().collect())
                    .unwrap_or_default();
                let exited = fields
                    .first()
                    .is_some_and(|state| matches!(*state, "Z" | "X"));
                let kernel = fields
                    .get(6)
                    .and_then(|flags| flags.parse::<u64>().ok())
                    .is_some_and(|flags| flags & 0x20_0000 != 0);
                if exited || kernel {
                    return Ok(None);
                }
                return Err(io::Error::other(
                    "empty command line on a surviving userspace process",
                ));
            }
            Ok::<_, io::Error>(Some((comm, cmdline)))
        })();
        let (comm, cmdline) = match identity {
            Ok(Some(identity)) => identity,
            Ok(None) => continue,
            // A process can exit between readdir and the reads. Only a confirmed exit is skipped; a surviving
            // PID whose identity cannot be read counts as a conflict.
            Err(error)
                if error.kind() == io::ErrorKind::NotFound
                    && matches!(fs::metadata(&process), Err(e) if e.kind() == io::ErrorKind::NotFound) =>
            {
                continue;
            }
            Err(error) => {
                return Err(io::Error::new(
                    error.kind(),
                    format!("cannot inspect pid {pid}: {error}"),
                ));
            }
        };
        let mut args = cmdline.split(|b| *b == 0).map(String::from_utf8_lossy);
        let program = args
            .next()
            .map(|a| {
                // nginx rewrites argv[0] into a display string that can end in its prefix directory, so
                // trust it only together with the kernel's process name, and never for cam itself.
                if comm.trim() == "nginx" && a.starts_with("nginx: ") {
                    "nginx".to_string()
                } else {
                    a.rsplit('/').next().unwrap_or_default().to_string()
                }
            })
            .unwrap_or_default();
        let subcommand = args.next().map(|a| a.into_owned()).unwrap_or_default();
        let comm = comm.trim();
        if program.is_empty() || ((is_cam(comm) || is_cam(&program)) && subcommand.is_empty()) {
            return Err(io::Error::other(format!(
                "cannot inspect pid {pid}: missing command identity"
            )));
        }
        if VENDOR.contains(&comm) || VENDOR.contains(&program.as_str()) {
            found.push(format!("pid {pid}: vendor {comm}"));
        } else if (is_cam(comm) || is_cam(&program)) && crate::cmd::owns_hardware(&subcommand) {
            found.push(format!("pid {pid}: cam {subcommand}"));
        }
    }
    found.sort();
    Ok(found)
}
