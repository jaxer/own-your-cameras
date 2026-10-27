//! Accepts exactly one WPA-PSK network and refuses everything else. No credential text enters errors.
use std::{collections::HashSet, io, path::Path};

pub(crate) fn read_private(path: &Path) -> io::Result<String> {
    let result = crate::files::read_private(path, 8192).and_then(|text| {
        if usable(&text) {
            Ok(text)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected one supported WPA-PSK network; see docs/hardware/network.md",
            ))
        }
    });
    result.map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))
}

pub(crate) fn read(path: &Path) -> io::Result<String> {
    let result = (|| {
        let text = String::from_utf8(crate::files::read_regular(path, 8192)?)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "not UTF-8"))?;
        if !usable(&text) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected one enabled WPA-PSK network with a 1..32-byte SSID and 8..63-byte passphrase or 64 hex digits; see docs/hardware/network.md",
            ));
        }
        Ok(text)
    })();
    result.map_err(|e: io::Error| io::Error::new(e.kind(), format!("{}: {e}", path.display())))
}

fn quoted(value: &str) -> Option<&str> {
    value.strip_prefix('"')?.strip_suffix('"').filter(|v| {
        !v.bytes()
            .any(|b| b < 32 || b == 127 || b == b'"' || b == b'\\')
    })
}
fn hex(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len())
        && value.len().is_multiple_of(2)
        && value.bytes().all(|b| b.is_ascii_hexdigit())
}
pub(crate) fn usable(text: &str) -> bool {
    let (mut inside, mut done, mut ssid, mut psk) = (false, false, false, false);
    let mut keys = HashSet::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if line == "network={" && !inside && !done {
            inside = true;
            continue;
        }
        if line == "}" && inside {
            inside = false;
            done = true;
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return false;
        };
        let (key, value) = (key.trim(), value.trim());
        if !keys.insert((inside, key)) {
            return false;
        }
        let valid = if inside {
            match key {
                "ssid" => {
                    ssid = quoted(value).is_some_and(|s| (1..=32).contains(&s.len()))
                        || hex(value, 2, 64);
                    ssid
                }
                "psk" => {
                    psk = quoted(value)
                        .is_some_and(|s| (8..=63).contains(&s.len()) && s.is_ascii())
                        || hex(value, 64, 64);
                    psk
                }
                "key_mgmt" => value == "WPA-PSK",
                "scan_ssid" => matches!(value, "0" | "1"),
                "disabled" => value == "0",
                "priority" => value.parse::<u32>().is_ok(),
                "proto" => matches!(value, "RSN" | "WPA" | "WPA RSN" | "RSN WPA"),
                "pairwise" | "group" => {
                    matches!(value, "CCMP" | "TKIP" | "CCMP TKIP" | "TKIP CCMP")
                }
                "auth_alg" => value == "OPEN",
                _ => false,
            }
        } else {
            match key {
                "ctrl_interface" => value == "/mnt/tmp/wpa_supplicant",
                "update_config" => value == "0",
                "ap_scan" => value == "1",
                "country" => value == "EE",
                _ => false,
            }
        };
        if !valid {
            return false;
        }
    }
    done && !inside && ssid && psk
}
