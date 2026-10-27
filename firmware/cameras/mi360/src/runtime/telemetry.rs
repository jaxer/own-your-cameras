//! Read-only process and system telemetry.
use super::*;

#[derive(Default)]
pub(super) struct Cpu {
    pub(super) last: Option<(u64, u64)>,
}
impl Cpu {
    pub(super) fn sample(&mut self, text: &str) -> Option<f64> {
        let mut lines = text.lines();
        let mut values = lines.next()?.split_whitespace();
        if values.next() != Some("cpu") {
            return None;
        }
        let numbers = values
            .take(8)
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        if numbers.len() < 4 {
            return None;
        }
        let total = numbers.iter().try_fold(0_u64, |a, n| a.checked_add(*n))?;
        let idle = numbers[3].checked_add(*numbers.get(4).unwrap_or(&0))?;
        let previous = self.last.replace((total, idle))?;
        let delta = total.checked_sub(previous.0)?;
        let idle = idle.checked_sub(previous.1)?;
        (delta > 0 && idle <= delta).then(|| 100.0 * (delta - idle) as f64 / delta as f64)
    }
}
pub(super) fn proc_kib(text: &str, key: &str) -> Option<u64> {
    let line = text.lines().find_map(|line| line.strip_prefix(key))?;
    let mut fields = line.split_whitespace();
    let value = fields.next()?.parse().ok()?;
    (fields.next() == Some("kB") && fields.next().is_none()).then_some(value)
}
pub(super) fn wifi_signal(text: &str) -> Option<f64> {
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("wlan0:"))?;
    let value = line
        .split_whitespace()
        .nth(3)?
        .trim_end_matches('.')
        .parse::<f64>()
        .ok()?;
    (value.is_finite() && (-127.0..0.0).contains(&value)).then_some(value)
}
pub(super) fn telemetry(publisher: &mqtt::Publisher, cpu: &mut Cpu) {
    if let Ok(text) = std::fs::read_to_string("/proc/uptime")
        && let Some(value) = text
            .split_whitespace()
            .next()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v >= 0.0)
    {
        let _ = publisher.state("uptime", format!("{value:.1}"));
    }
    if let Ok(text) = std::fs::read_to_string("/proc/stat")
        && let Some(value) = cpu.sample(&text)
    {
        let _ = publisher.state("cpu_busy", format!("{value:.1}"));
    }
    if let Ok(text) = std::fs::read_to_string("/proc/meminfo")
        && let Some(value) = proc_kib(&text, "MemAvailable:")
    {
        let _ = publisher.state("memory_available", format!("{:.2}", value as f64 / 1024.0));
    }
    if let Ok(text) = std::fs::read_to_string("/proc/self/status")
        && let Some(value) = proc_kib(&text, "VmRSS:")
    {
        let _ = publisher.state("daemon_memory", format!("{:.2}", value as f64 / 1024.0));
    }
    if let Ok(text) = std::fs::read_to_string("/proc/net/wireless")
        && let Some(value) = wifi_signal(&text)
    {
        let _ = publisher.state("wifi_signal", format!("{value:.1}"));
    }
}
