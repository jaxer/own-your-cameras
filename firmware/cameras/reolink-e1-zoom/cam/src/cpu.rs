//! System CPU load between two `/proc/stat` readings.

/// Remembers the previous reading, so each new one yields the busy share since then.
#[derive(Debug, Default)]
pub struct Sampler {
    last: Option<(u64, u64)>,
}

impl Sampler {
    /// Starts from the system's `/proc/stat` now.
    pub fn system() -> Self {
        let mut sampler = Self::default();
        sampler.sample_system();
        sampler
    }

    pub fn sample_system(&mut self) -> Option<f64> {
        self.sample(std::fs::read_to_string("/proc/stat").ok().as_deref())
    }

    /// Busy percent since the previous parsable reading; an unparsable one is skipped.
    pub fn sample(&mut self, stat: Option<&str>) -> Option<f64> {
        let now = parse_total_idle(stat?)?;
        let busy = self.last.map(|before| busy_percent(before, now));
        self.last = Some(now);
        busy
    }
}

/// `(total, idle)` jiffies from the aggregate `cpu` line; idle counts `iowait` too.
fn parse_total_idle(stat: &str) -> Option<(u64, u64)> {
    let line = stat.lines().find(|l| l.starts_with("cpu "))?;
    let f: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .map(|v| v.parse().ok())
        .collect::<Option<_>>()?;
    if f.len() < 5 {
        return None;
    }
    // user nice system idle iowait irq softirq steal [guest guest_nice]: guest time is already in user.
    let total = f.iter().take(8).sum();
    Some((total, f[3] + f[4]))
}

fn busy_percent(before: (u64, u64), after: (u64, u64)) -> f64 {
    let total = after.0.saturating_sub(before.0);
    let idle = after.1.saturating_sub(before.1);
    if total == 0 {
        return 0.0;
    }
    (total - idle.min(total)) as f64 * 100.0 / total as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_reading_only_sets_the_baseline() {
        let mut s = Sampler::default();
        assert_eq!(s.sample(Some("cpu  10 0 10 80 0 0 0 0\n")), None);
        assert_eq!(s.sample(Some("cpu  20 0 20 160 0 0 0 0\n")), Some(20.0));
    }

    #[test]
    fn unparsable_readings_keep_the_previous_baseline() {
        let mut s = Sampler::default();
        s.sample(Some("cpu  0 0 0 100 0 0 0 0\n"));
        assert_eq!(s.sample(Some("cpu0 1 2 3")), None);
        assert_eq!(s.sample(None), None);
        assert_eq!(s.sample(Some("cpu  50 0 0 150 0 0 0 0\n")), Some(50.0));
    }

    #[test]
    fn busy_share_comes_from_the_aggregate_line() {
        let mut s = Sampler::default();
        s.sample(Some(
            "cpu  100 0 50 800 50 0 0 0 0 0\ncpu0 100 0 50 800 50 0 0 0 0 0\n",
        ));
        // total +250, idle (idle + iowait) +100: 60 % busy.
        assert_eq!(
            s.sample(Some("cpu  200 0 100 900 50 0 0 0 0 0\n")),
            Some(60.0)
        );
        assert!(parse_total_idle("garbage").is_none());
    }

    #[test]
    fn iowait_is_idle_and_a_still_clock_is_zero() {
        assert_eq!(parse_total_idle("cpu  1 2 3 4 5 6 7 8 9 10"), Some((36, 9)));
        assert_eq!(parse_total_idle("cpu  1 2 3"), None);
        assert_eq!(busy_percent((10, 5), (10, 5)), 0.0);
    }
}
