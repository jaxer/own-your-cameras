//! `cam net`.

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use super::{CliError, Ownership};
use crate::identity::Identity;
use crate::mqtt::Config;
use crate::net::linux::{LinuxHost, seed_wpa_conf};
use crate::net::{Supervisor, Timing};
use crate::ownership::Resource;
use crate::{preflight, signals};

/// Our own Wi-Fi credentials, seeded once from the vendor-rendered config below.
const OUR_WPA_CONF: &str = "/mnt/para/cam-wifi.conf";
/// What the vendor `device` renders from `link.cfg` at boot.
const VENDOR_WPA_CONF: &str = "/mnt/tmp/atheros_config.conf";
const NET_TICK: Duration = Duration::from_secs(2);

pub const USAGE: &str = "usage: cam net [--check | --seed-vendor]";

/// Supervising takes the network; seeding credentials only needs to keep production away.
pub fn owns(args: &[String]) -> Ownership {
    match args {
        [] => Ownership::Diagnostic(Some(Resource::Network)),
        [arg] if arg == "--seed-vendor" => Ownership::Diagnostic(None),
        _ => Ownership::Nothing,
    }
}

/// `cam net`: supervise Wi-Fi and eth0 until SIGTERM. Children keep running after exit.
pub fn run(args: &[String]) -> Result<ExitCode, CliError> {
    if !args.is_empty() && !matches!(args, [arg] if arg == "--check" || arg == "--seed-vendor") {
        return Err(CliError::Usage);
    }
    Ok(supervise(args))
}

fn supervise(args: &[String]) -> ExitCode {
    if args.first().is_some_and(|arg| arg == "--seed-vendor") {
        return match seed_wpa_conf(Path::new(OUR_WPA_CONF), Path::new(VENDOR_WPA_CONF)) {
            Ok(_) => {
                println!("net: persistent credentials ready");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("net: credential seeding failed: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let identity = match Config::read(Path::new(preflight::CONFIG)) {
        Ok(cfg) => cfg.camera,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Identity::default(),
        Err(e) => {
            eprintln!("net: preflight failed: {}: {e}", preflight::CONFIG);
            return ExitCode::FAILURE;
        }
    };
    let mut host = match LinuxHost::production(&identity) {
        Ok(host) => host,
        Err(e) => {
            eprintln!("net: preflight failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    if !args.is_empty() {
        println!("net: persistent credentials ready");
        return ExitCode::SUCCESS;
    }
    if let Err(error) = signals::install() {
        eprintln!("signal setup failed: {error}");
        return ExitCode::FAILURE;
    }
    let mut sup = Supervisor::new(Timing::default());
    let start = Instant::now();
    loop {
        for ev in sup.tick(&mut host, start.elapsed()) {
            println!("net: {:>6.1}s {ev:?}", start.elapsed().as_secs_f32());
        }
        if !signals::nap(NET_TICK) {
            host.exit_preserving_network(0);
        }
    }
}
