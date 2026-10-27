//! Camera health for Home Assistant's diagnostic sensors: CPU, memory, load, temperature, Wi-Fi and the
//! encoders' delivered bitrate, frame rate and shortfall. Collected on its own thread every
//! [`INTERVAL`]; the media thread only bumps [`StreamCounters`]. A reading that is missing publishes
//! empty (unknown), never zero.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub const INTERVAL: Duration = Duration::from_secs(10);

/// One diagnostic sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sensor {
    pub key: &'static str,
    pub name: &'static str,
    pub unit: Option<&'static str>,
    pub device_class: Option<&'static str>,
    pub icon: Option<&'static str>,
}

const fn sensor(
    key: &'static str,
    name: &'static str,
    unit: Option<&'static str>,
    device_class: Option<&'static str>,
    icon: Option<&'static str>,
) -> Sensor {
    Sensor {
        key,
        name,
        unit,
        device_class,
        icon,
    }
}

/// Every telemetry sensor; discovery and the stale-state reset derive from it.
pub const SENSORS: &[Sensor] = &[
    sensor("cpu", "CPU busy", Some("%"), None, Some("mdi:cpu-32-bit")),
    sensor(
        "mem_available",
        "Memory available",
        Some("MiB"),
        Some("data_size"),
        None,
    ),
    sensor("rss", "Daemon memory", Some("MiB"), Some("data_size"), None),
    sensor("load", "Load average", None, None, Some("mdi:gauge")),
    sensor(
        "temperature",
        "Temperature",
        Some("°C"),
        Some("temperature"),
        None,
    ),
    sensor("wifi", "Wi-Fi signal", Some("%"), None, Some("mdi:wifi")),
    sensor(
        "wifi_rssi",
        "Wi-Fi RSSI",
        Some("dBm"),
        Some("signal_strength"),
        None,
    ),
    sensor(
        "main_bitrate_actual",
        "Main bitrate (actual)",
        Some("kbit/s"),
        Some("data_rate"),
        None,
    ),
    sensor(
        "sub_bitrate_actual",
        "Sub bitrate (actual)",
        Some("kbit/s"),
        Some("data_rate"),
        None,
    ),
    sensor(
        "main_fps_actual",
        "Main frame rate (actual)",
        Some("fps"),
        None,
        Some("mdi:filmstrip"),
    ),
    sensor(
        "sub_fps_actual",
        "Sub frame rate (actual)",
        Some("fps"),
        None,
        Some("mdi:filmstrip"),
    ),
    sensor(
        "main_dropped",
        "Main frames short",
        None,
        None,
        Some("mdi:filmstrip-off"),
    ),
    sensor(
        "sub_dropped",
        "Sub frames short",
        None,
        None,
        Some("mdi:filmstrip-off"),
    ),
    sensor("uptime", "Uptime", Some("s"), Some("duration"), None),
];

/// Frames and bytes the encoders delivered, per stream; bumped by the media thread, nothing else.
#[derive(Debug, Default)]
pub struct StreamCounters {
    frames: [AtomicU64; 2],
    bytes: [AtomicU64; 2],
}

impl StreamCounters {
    pub fn frame(&self, stream: usize, bytes: usize) {
        if let (Some(f), Some(b)) = (self.frames.get(stream), self.bytes.get(stream)) {
            f.fetch_add(1, Ordering::Relaxed);
            b.fetch_add(bytes as u64, Ordering::Relaxed);
        }
    }

    fn snapshot(&self) -> [(u64, u64); 2] {
        std::array::from_fn(|i| {
            (
                self.frames[i].load(Ordering::Relaxed),
                self.bytes[i].load(Ordering::Relaxed),
            )
        })
    }
}

/// `MemAvailable` in MiB.
pub fn mem_available_mib(meminfo: &str) -> Option<f64> {
    kib_field(meminfo, "MemAvailable:").map(|k| k / 1024.0)
}

/// `VmRSS` of a `/proc/<pid>/status` in MiB.
pub fn rss_mib(status: &str) -> Option<f64> {
    kib_field(status, "VmRSS:").map(|k| k / 1024.0)
}

fn kib_field(text: &str, name: &str) -> Option<f64> {
    let line = text.lines().find(|l| l.starts_with(name))?;
    let mut f = line[name.len()..].split_whitespace();
    let n: f64 = f.next()?.parse().ok()?;
    (f.next() == Some("kB")).then_some(n)
}

/// 1-minute load average.
pub fn load1(loadavg: &str) -> Option<f64> {
    loadavg.split_whitespace().next()?.parse().ok()
}

/// The Novatek thermal driver reports whole °C.
pub fn temperature(temp: &str) -> Option<i32> {
    temp.trim().parse().ok()
}

/// rtl88x2cu's `/proc/net/wireless` link field: quality percent (its level field is not dBm).
pub fn wifi_quality(wireless: &str) -> Option<u32> {
    wireless
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("wlan0:")
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.trim_end_matches('.').parse::<u32>().ok())
        })
        .filter(|n| *n <= 100)
}

/// `signal: -53 dBm` from `iw dev wlan0 link`, the only real dBm on this unit.
pub fn iw_signal_dbm(link: &str) -> Option<i32> {
    let v = link
        .lines()
        .find_map(|l| l.trim().strip_prefix("signal:"))?
        .trim()
        .strip_suffix("dBm")?
        .trim()
        .parse::<i32>()
        .ok()?;
    (-120..=0).contains(&v).then_some(v)
}

pub fn uptime_secs(uptime: &str) -> Option<u64> {
    uptime
        .split_whitespace()
        .next()?
        .split('.')
        .next()?
        .parse()
        .ok()
}

/// What the collector reads from the system; the real one reads files and runs `iw`.
pub trait Source: Send {
    /// Contents of a `/proc` or sysfs file, `None` if unreadable.
    fn file(&mut self, path: &str) -> Option<String>;
    /// `iw dev wlan0 link` output, bounded in time.
    fn iw_link(&mut self) -> Option<String>;
}

pub struct System;

impl Source for System {
    fn file(&mut self, path: &str) -> Option<String> {
        crate::files::read_device_or_file(std::path::Path::new(path), 64 * 1024)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
    }

    fn iw_link(&mut self) -> Option<String> {
        let mut child = std::process::Command::new("iw")
            .args(["dev", "wlan0", "link"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
            }
        }
        let mut out = String::new();
        use std::io::Read;
        child
            .stdout
            .take()?
            .take(4096)
            .read_to_string(&mut out)
            .ok()?;
        Some(out)
    }
}

/// When, and per-stream `(frames, bytes)`.
type Snapshot = (Duration, [(u64, u64); 2]);

/// Turns two samples [`INTERVAL`] apart into sensor states.
pub struct Collector<S: Source> {
    source: S,
    counters: std::sync::Arc<StreamCounters>,
    cpu: crate::cpu::Sampler,
    last: Option<Snapshot>,
}

impl<S: Source> Collector<S> {
    pub fn new(source: S, counters: std::sync::Arc<StreamCounters>) -> Self {
        Self {
            source,
            counters,
            cpu: crate::cpu::Sampler::default(),
            last: None,
        }
    }

    /// Every sensor in [`SENSORS`], empty where unknown. `expected_fps` is what each encoder is
    /// configured for; frames short of it over the window count as dropped (no HDAL drop counter is
    /// established, `docs/settings.md`).
    pub fn sample(
        &mut self,
        now: Duration,
        expected_fps: [Option<u32>; 2],
    ) -> BTreeMap<&'static str, String> {
        let mut s: BTreeMap<&'static str, String> =
            SENSORS.iter().map(|x| (x.key, String::new())).collect();
        let mut put = |k: &'static str, v: Option<String>| {
            if let Some(v) = v {
                s.insert(k, v);
            }
        };
        let stat = self.source.file("/proc/stat");
        put(
            "cpu",
            self.cpu.sample(stat.as_deref()).map(|b| format!("{b:.1}")),
        );
        let counts = self.counters.snapshot();
        put(
            "mem_available",
            self.source
                .file("/proc/meminfo")
                .and_then(|t| mem_available_mib(&t))
                .map(|v| format!("{v:.1}")),
        );
        put(
            "rss",
            self.source
                .file("/proc/self/status")
                .and_then(|t| rss_mib(&t))
                .map(|v| format!("{v:.1}")),
        );
        put(
            "load",
            self.source
                .file("/proc/loadavg")
                .and_then(|t| load1(&t))
                .map(|v| format!("{v:.2}")),
        );
        put(
            "temperature",
            self.source
                .file("/sys/class/thermal/thermal_zone0/temp")
                .and_then(|t| temperature(&t))
                .map(|v| v.to_string()),
        );
        put(
            "wifi",
            self.source
                .file("/proc/net/wireless")
                .and_then(|t| wifi_quality(&t))
                .map(|v| v.to_string()),
        );
        put(
            "wifi_rssi",
            self.source
                .iw_link()
                .and_then(|t| iw_signal_dbm(&t))
                .map(|v| v.to_string()),
        );
        put(
            "uptime",
            self.source
                .file("/proc/uptime")
                .and_then(|t| uptime_secs(&t))
                .map(|v| v.to_string()),
        );
        if let Some((then, counts_then)) = self.last {
            let secs = now.saturating_sub(then).as_secs_f64();
            if secs > 0.0 {
                for (i, name) in ["main", "sub"].into_iter().enumerate() {
                    let frames = counts[i].0.saturating_sub(counts_then[i].0);
                    let bytes = counts[i].1.saturating_sub(counts_then[i].1);
                    let (b, f, d) = match name {
                        "main" => ("main_bitrate_actual", "main_fps_actual", "main_dropped"),
                        _ => ("sub_bitrate_actual", "sub_fps_actual", "sub_dropped"),
                    };
                    put(
                        b,
                        Some(format!("{:.0}", bytes as f64 * 8.0 / 1000.0 / secs)),
                    );
                    put(f, Some(format!("{:.1}", frames as f64 / secs)));
                    put(
                        d,
                        expected_fps[i].map(|fps| {
                            let expected = (f64::from(fps) * secs).round() as u64;
                            // One frame of slack for the window edges.
                            expected.saturating_sub(frames + 1).to_string()
                        }),
                    );
                }
            }
        }
        self.last = Some((now, counts));
        s
    }
}

impl Source for Box<dyn Source> {
    fn file(&mut self, path: &str) -> Option<String> {
        (**self).file(path)
    }
    fn iw_link(&mut self) -> Option<String> {
        (**self).iw_link()
    }
}

/// What the telemetry thread runs: the collector and where it learns each encoder's configured rate.
pub struct Task {
    pub collector: Collector<Box<dyn Source>>,
    pub expected_fps: Box<dyn Fn() -> [Option<u32>; 2] + Send>,
}

impl Task {
    pub fn sample(&mut self, now: Duration) -> BTreeMap<&'static str, String> {
        let fps = (self.expected_fps)();
        self.collector.sample(now, fps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/telemetry/");

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{DIR}{name}")).unwrap()
    }

    #[test]
    fn parsers_read_the_real_camera_text() {
        assert_eq!(
            mem_available_mib(&fixture("meminfo")),
            Some(50456.0 / 1024.0)
        );
        assert_eq!(rss_mib(&fixture("status")), Some(3320.0 / 1024.0));
        assert_eq!(load1(&fixture("loadavg")).map(|l| l > 1.0), Some(true));
        assert_eq!(temperature(&fixture("thermal_zone0_temp")), Some(66));
        assert_eq!(wifi_quality(&fixture("wireless")), Some(77));
        assert_eq!(iw_signal_dbm(&fixture("iw-link")), Some(-53));
        assert!(uptime_secs(&fixture("uptime")).is_some_and(|u| u > 3000));
        let stat = fixture("stat-10s");
        let mut lines = stat.lines();
        let mut cpu = crate::cpu::Sampler::default();
        cpu.sample(lines.next());
        let busy = cpu.sample(lines.next()).unwrap();
        assert!((20.0..30.0).contains(&busy), "{busy}");
    }

    #[test]
    fn garbage_is_unknown_never_zero() {
        assert_eq!(mem_available_mib("MemAvailable: lots"), None);
        assert_eq!(rss_mib(""), None);
        assert_eq!(iw_signal_dbm("Not connected."), None);
        assert_eq!(
            iw_signal_dbm("signal: 47 dBm"),
            None,
            "positive is not a dBm reading"
        );
        assert_eq!(wifi_quality(" wlan0: 0000   167.   47."), None);
    }

    struct Fixed {
        stat: Vec<String>,
    }
    impl Source for Fixed {
        fn file(&mut self, path: &str) -> Option<String> {
            match path {
                "/proc/stat" => (!self.stat.is_empty()).then(|| self.stat.remove(0)),
                "/proc/meminfo" => Some(fixture("meminfo")),
                _ => None,
            }
        }
        fn iw_link(&mut self) -> Option<String> {
            None
        }
    }

    #[test]
    fn rates_come_from_the_counters_over_the_window_and_unknowns_stay_empty() {
        let stat = fixture("stat-10s");
        let counters = std::sync::Arc::new(StreamCounters::default());
        let mut c = Collector::new(
            Fixed {
                stat: stat.lines().map(str::to_owned).collect(),
            },
            std::sync::Arc::clone(&counters),
        );
        let first = c.sample(Duration::ZERO, [Some(15), Some(10)]);
        assert_eq!(first["cpu"], "", "needs two samples");
        assert_eq!(first["temperature"], "", "unreadable is unknown");
        assert_eq!(first["mem_available"], "49.3");
        for _ in 0..150 {
            counters.frame(0, 8_533);
        }
        for _ in 0..90 {
            counters.frame(1, 6_400);
        }
        let s = c.sample(INTERVAL, [Some(15), Some(10)]);
        assert_eq!(s["main_fps_actual"], "15.0");
        assert_eq!(s["main_bitrate_actual"], "1024");
        assert_eq!(s["main_dropped"], "0");
        assert_eq!(s["sub_fps_actual"], "9.0");
        assert_eq!(
            s["sub_dropped"], "9",
            "100 expected, 90 delivered, one frame slack"
        );
        assert!(!s["cpu"].is_empty());
        assert_eq!(s.len(), SENSORS.len());
    }
}
