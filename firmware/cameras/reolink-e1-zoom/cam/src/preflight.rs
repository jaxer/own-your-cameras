//! `cam preflight`: the read-only check the launcher runs before any watchdog handoff, socket, network change
//! or HDAL access. Errors name files and rules but never echo a credential; nothing is written.

use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::mqtt;
use crate::rtsp::proto::Credentials;

pub const WIFI_CONF: &str = "/mnt/para/cam-wifi.conf";
pub const RTSP_AUTH: &str = "/mnt/para/cam-rtsp.auth";
pub const CONFIG: &str = "/mnt/para/cam.toml";

/// OS and vendor files the production runtime executes or loads. Device nodes are not listed: they
/// exist only once the vendor kernel modules are up and fail the runtime itself when absent.
pub const ARTIFACTS: [&str; 16] = [
    "/usr/lib/libhdal.so",
    "/dev/mtd7ro",
    "/sbin/insmod",
    "/sbin/rmmod",
    "/sbin/modprobe",
    "/sbin/ifconfig",
    "/sbin/wpa_supplicant",
    "/sbin/udhcpc",
    "/sbin/reboot",
    "/lib/modules/usb_ko/usb-common.ko",
    "/lib/modules/usb_ko/usbcore.ko",
    "/lib/modules/usb_ko/ehci-hcd.ko",
    "/lib/modules/wifi/firmware_class.ko",
    "/lib/modules/wifi/rfkill.ko",
    "/lib/modules/wifi/wifi_8812ko/cfg80211.ko",
    "/lib/modules/wifi/wifi_8812ko/wlan.ko",
];

const MAX_FILE: u64 = 16 * 1024;

pub struct Ready {
    pub rtsp: Credentials,
    pub mqtt: mqtt::Config,
}

fn under(root: &Path, path: &str) -> PathBuf {
    root.join(path.trim_start_matches('/'))
}

/// Validates every production input under `root` (`/` on the camera). All problems are collected, so the
/// launcher log shows the complete list.
pub fn check(root: &Path) -> Result<Ready, Vec<String>> {
    let mut problems = Vec::new();
    match crate::files::read_private(&under(root, WIFI_CONF), 8192) {
        Ok(text) if crate::net::wifi_conf::usable(&text) => {}
        Ok(_) => problems.push(format!(
            "{WIFI_CONF}: expected one supported WPA-PSK network; see docs/hardware/network.md"
        )),
        Err(e) => problems.push(format!("{WIFI_CONF}: {e}")),
    }
    let rtsp = match crate::files::read_private(&under(root, RTSP_AUTH), MAX_FILE) {
        Err(e) => {
            problems.push(format!("{RTSP_AUTH}: {e}"));
            None
        }
        Ok(text) => {
            let creds = Credentials::parse(&text);
            if creds.is_none() {
                problems.push(format!(
                    "{RTSP_AUTH}: no usable user:password on its first line"
                ));
            }
            creds
        }
    };
    let mqtt = match mqtt::Config::read(&under(root, CONFIG)) {
        Err(e) => {
            problems.push(format!("{CONFIG}: {e}"));
            None
        }
        Ok(cfg) if cfg.mqtt.is_none() => {
            problems.push(format!(
                "{CONFIG}: the [mqtt] section is required in production"
            ));
            None
        }
        Ok(cfg) => Some(cfg),
    };
    for artifact in ARTIFACTS {
        if let Err(e) = artifact_check(&under(root, artifact), artifact) {
            problems.push(format!("{artifact}: {e}"));
        }
    }
    match (rtsp, mqtt, problems.is_empty()) {
        (Some(rtsp), Some(mqtt), true) => Ok(Ready { rtsp, mqtt }),
        _ => Err(problems),
    }
}

fn artifact_check(path: &Path, artifact: &str) -> io::Result<()> {
    // BusyBox applets and library sonames may legitimately be symlinks. Inspect the
    // opened target, with nonblocking open so a FIFO cannot hang the startup gate.
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    if artifact == "/dev/mtd7ro" {
        if !meta.file_type().is_char_device() {
            return Err(io::Error::other("expected a readable character device"));
        }
    } else if !meta.is_file() || meta.len() == 0 || meta.permissions().mode() & 0o444 == 0 {
        return Err(io::Error::other(
            "expected a nonempty readable regular file",
        ));
    } else if artifact.starts_with("/sbin/") && meta.permissions().mode() & 0o111 == 0 {
        return Err(io::Error::other("helper is not executable"));
    }
    Ok(())
}

/// `cam preflight` body: prints one line per problem to stderr; 0 ready, 1 invalid.
pub fn run(root: &Path) -> u8 {
    match check(root) {
        Ok(_) => {
            println!("preflight: ready");
            0
        }
        Err(problems) => {
            for problem in problems {
                eprintln!("preflight: {problem}");
            }
            1
        }
    }
}
