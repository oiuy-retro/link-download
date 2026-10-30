//! The same readouts as Folder Difference Copy: speed, items per second, files left, time left, elapsed.

pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        if bytes == 1 {
            return "1 byte".into();
        }
        return format!("{bytes} bytes");
    }
    format_scaled(bytes as f64)
}

pub fn format_rate(bytes_per_sec: f64) -> String {
    let bytes_per_sec = if bytes_per_sec.is_finite() && bytes_per_sec > 0.0 { bytes_per_sec } else { 0.0 };
    if bytes_per_sec < 1024.0 {
        return format!("{} B/s", bytes_per_sec.round() as u64);
    }
    format!("{}/s", format_scaled(bytes_per_sec))
}

fn format_scaled(bytes: f64) -> String {
    let units = ["KB", "MB", "GB", "TB"];
    let mut value = bytes;
    let mut unit = 0;
    // The input is still in bytes. The first division lands on KB.
    if value >= 1024.0 {
        value /= 1024.0;
    }
    while value >= 1024.0 && unit + 1 < units.len() {
        value /= 1024.0;
        unit += 1;
    }
    let number = if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    };
    format!("{number} {}", units[unit])
}

pub fn format_items(per_second: f64) -> String {
    let per_second = if per_second.is_finite() && per_second > 0.0 { per_second } else { 0.0 };
    if per_second >= 100.0 {
        format!("{per_second:.0}")
    } else if per_second >= 10.0 {
        format!("{per_second:.1}")
    } else {
        format!("{per_second:.2}")
    }
}

pub fn format_eta(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "Calculating...".into();
    }
    let total = seconds.round() as u64;
    let days = total / 86_400;
    let hours = (total % 86_400) / 3_600;
    let minutes = (total % 3_600) / 60;
    let secs = total % 60;
    if days > 0 {
        format!("{days:02}:{hours:02}:{minutes:02}:{secs:02}")
    } else if hours > 0 {
        format!("{hours:02}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

/// What Time left is worked out from. All times are seconds.
#[derive(Clone, Copy, Default)]
pub struct Job {
    /// Since the run started.
    pub elapsed: f64,
    /// Items finished so far, and when the latest of them ended.
    pub finished: u32,
    pub last_finish: f64,
    /// Items not finished yet, the active or waiting one included.
    pub left: u32,
    /// The active item's download still needs this long at the smoothed speed.
    pub download: Option<f64>,
    /// The active item is converting, and this much of the usual convert time is left.
    pub convert_left: Option<f64>,
    /// The pause before the next item is running and has this long to go.
    pub waiting: Option<f64>,
    /// Average convert time of the items that have converted so far.
    pub convert_avg: Option<f64>,
    /// Average download size of the items that have downloaded so far, in bytes.
    pub bytes_per_item: Option<f64>,
    /// Smoothed download speed, bytes per second.
    pub speed: Option<f64>,
    /// Average pause the app inserts before each next item (the middle of min..max).
    pub wait: f64,
}

/// Seconds left for the whole run. None means there is not enough to go on yet.
///
/// The current item: its download left at the smoothed speed plus the usual convert
/// time, or what is left of its convert, or of the pause before it. Each item after it:
/// the pause, then the average download size at the smoothed speed plus the usual
/// convert time. Before anything has downloaded, sizes are unknown, so the per-item
/// time is the plain average of finished items (pauses included), or for the very
/// first item, how long it has taken so far plus its download left.
pub fn time_left(job: &Job) -> Option<f64> {
    if job.left == 0 {
        return Some(0.0);
    }
    let ok = |v: Option<f64>| v.filter(|x| x.is_finite() && *x >= 0.0);
    let wait = job.wait.max(0.0);
    let convert = ok(job.convert_avg).unwrap_or(0.0);
    let speed = ok(job.speed).filter(|s| *s > 0.0);
    let download = ok(job.download);
    let avg_per = if job.finished > 0 { Some(job.last_finish.max(0.0) / job.finished as f64) } else { None };
    // One item's time without the pause before it.
    let mut item = match (ok(job.bytes_per_item), speed) {
        (Some(bytes), Some(speed)) => Some(bytes / speed + convert),
        _ => avg_per.map(|per| (per - wait).max(0.0)),
    };
    if item.is_none() {
        // First item: what it took so far plus what its download still needs.
        let download = download?;
        if job.elapsed < 3.0 {
            return None;
        }
        item = Some(job.elapsed + download);
    }
    let item = item?;
    let since = (job.elapsed - job.last_finish).max(0.0);
    let current = if let Some(pause) = ok(job.waiting) {
        pause + item
    } else if let Some(download) = download {
        download + convert
    } else if let Some(left) = ok(job.convert_left) {
        left
    } else if let Some(per) = avg_per {
        (per - since).max(0.0)
    } else {
        item
    };
    Some(current + (job.left - 1) as f64 * (wait + item))
}

/// Download speed as an exponentially weighted average with a time constant of `tau`
/// seconds, weighted by the real time between samples. Older samples fade out
/// gradually, so a slow patch does not drop out of the average all at once.
pub struct SmoothRate {
    tau: f64,
    rate: Option<f64>,
    last: Option<(f64, u64)>,
    warm: Option<(f64, u64)>,
}

impl SmoothRate {
    pub fn new(tau: f64) -> Self {
        Self { tau, rate: None, last: None, warm: None }
    }

    pub fn clear(&mut self) {
        *self = Self::new(self.tau);
    }

    /// `paused`: the app is in its pause between items. Nothing downloads then, so the
    /// time is left out instead of pulling the speed toward zero.
    pub fn push(&mut self, at: f64, bytes: u64, paused: bool) {
        if paused {
            self.last = None;
            self.warm = None;
            return;
        }
        let Some((last_at, last_bytes)) = self.last else {
            self.last = Some((at, bytes));
            self.warm = Some((at, bytes));
            return;
        };
        if bytes < last_bytes {
            // The total went down (a failed item's bytes were dropped). Measure from here.
            self.last = Some((at, bytes));
            self.warm = Some((at, bytes));
            return;
        }
        let dt = at - last_at;
        if dt <= 0.0 {
            return;
        }
        self.last = Some((at, bytes));
        match self.rate {
            None => {
                // The first value is a plain average over at least a second.
                if let Some((warm_at, warm_bytes)) = self.warm {
                    if at - warm_at >= 1.0 {
                        self.rate = Some(bytes.saturating_sub(warm_bytes) as f64 / (at - warm_at));
                    }
                }
            }
            Some(rate) => {
                let now = (bytes - last_bytes) as f64 / dt;
                let alpha = 1.0 - (-dt / self.tau).exp();
                self.rate = Some(rate + alpha * (now - rate));
            }
        }
    }

    pub fn rate(&self) -> Option<f64> {
        self.rate
    }
}

/// Time left as shown. The estimate is worked out every `EVERY` seconds. In between the
/// shown value counts down by the clock, and it eases toward the newest estimate rather
/// than jumping: each tick closes part of the gap. When the estimate grew, the shown
/// value only counts down more slowly; it never counts up. It jumps straight to the
/// estimate the first time, when an item finishes, and when the gap is over half.
pub struct Countdown {
    shown: Option<f64>,
    target: Option<(f64, f64)>,
    tick_at: f64,
    math_at: Option<f64>,
    left: u32,
}

impl Countdown {
    pub const EVERY: f64 = 5.0;
    /// Seconds for most of a gap to close.
    pub const EASE: f64 = 3.0;

    pub fn new() -> Self {
        Self { shown: None, target: None, tick_at: 0.0, math_at: None, left: 0 }
    }

    pub fn due(&self, now: f64, left: u32) -> bool {
        match self.math_at {
            None => true,
            Some(at) => now - at >= Self::EVERY || left != self.left,
        }
    }

    /// New math. None (not enough to go on) keeps counting down what is shown.
    pub fn sync(&mut self, now: f64, left: u32, estimate: Option<f64>) {
        self.tick(now);
        let item_done = self.math_at.is_some() && left != self.left;
        self.math_at = Some(now);
        self.left = left;
        let Some(estimate) = estimate.filter(|s| s.is_finite() && *s >= 0.0) else {
            return;
        };
        self.target = Some((now, estimate));
        let snap = match self.shown {
            None => true,
            Some(shown) => item_done || (estimate - shown).abs() > 0.5 * shown.max(1.0),
        };
        if snap {
            self.shown = Some(estimate);
        }
    }

    /// Move the shown value on to `now`.
    pub fn tick(&mut self, now: f64) {
        let dt = (now - self.tick_at).max(0.0);
        self.tick_at = now;
        let (Some(shown), Some((at, value))) = (self.shown, self.target) else {
            return;
        };
        let target = (value - (now - at)).max(0.0);
        let mut next = shown - dt;
        let gap = target - next;
        let mut step = gap * (1.0 - (-dt / Self::EASE).exp());
        if step > 0.0 {
            // Slow the countdown down, but keep it going down.
            step = step.min(dt * 0.9);
        }
        next += step;
        self.shown = Some(next.max(0.0));
    }

    pub fn show(&self) -> Option<f64> {
        self.shown
    }
}

/// "Waiting 12 seconds" from the pause between items, as 12.
pub fn waiting_seconds(note: &str) -> Option<f64> {
    let rest = note.strip_prefix("Waiting ")?;
    let number = rest.split_whitespace().next()?;
    number.parse::<u32>().ok().map(f64::from)
}

pub fn format_elapsed(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours >= 1 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

pub fn percent(ratio: f64) -> u32 {
    let ratio = if ratio.is_finite() { ratio.clamp(0.0, 1.0) } else { 0.0 };
    if ratio >= 1.0 {
        return 100;
    }
    let pct = (ratio * 100.0).round() as u32;
    if pct >= 100 { 99 } else { pct }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readouts_match_the_copy_program() {
        assert_eq!(format_bytes(1), "1 byte");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_rate(2048.0), "2.00 KB/s");
        assert_eq!(format_items(1.2), "1.20");
        assert_eq!(format_eta(2.0), "00:02");
        assert_eq!(format_eta(90.0), "01:30");
        assert_eq!(format_eta(5.0 * 3600.0 + 10.0 * 60.0), "05:10:00");
        assert_eq!(format_eta(86_400.0 + 5.0 * 3600.0 + 10.0 * 60.0), "01:05:10:00");
        assert_eq!(format_elapsed(75), "1:15");
        assert_eq!(format_elapsed(3661), "1:01:01");
        assert_eq!(percent(0.999), 99);
        assert_eq!(percent(1.0), 100);
    }

    fn job(elapsed: f64, finished: u32, last_finish: f64, left: u32, download: Option<f64>, wait: f64) -> Job {
        Job { elapsed, finished, last_finish, left, download, wait, ..Job::default() }
    }

    #[test]
    fn time_left_waits_for_something_to_go_on() {
        assert_eq!(time_left(&job(0.0, 0, 0.0, 3, None, 6.0)), None);
        assert_eq!(time_left(&job(10.0, 0, 0.0, 3, None, 6.0)), None);
        assert_eq!(time_left(&job(1.0, 0, 0.0, 3, Some(10.0), 6.0)), None);
    }

    #[test]
    fn time_left_guesses_from_the_first_item() {
        // 10 s in, 10 s of download left: 10 s more, then a 6 s pause and 20 s for each of 2.
        assert_eq!(time_left(&job(10.0, 0, 0.0, 3, Some(10.0), 6.0)), Some(10.0 + 26.0 * 2.0));
    }

    #[test]
    fn time_left_after_one_item_without_sizes() {
        // One item took 30 s, pause included. Two left, nothing downloading yet.
        assert_eq!(time_left(&job(30.0, 1, 30.0, 2, None, 6.0)), Some(60.0));
        assert_eq!(time_left(&job(40.0, 1, 30.0, 2, None, 6.0)), Some(50.0));
    }

    #[test]
    fn time_left_counts_sizes_convert_and_pauses() {
        // 10 MB items at 1 MB/s, 4 s to convert, 6 s pause. Active item: 3 s of
        // download and its 4 s convert. Then two items of pause + 10 s + 4 s.
        let j = Job {
            elapsed: 50.0, finished: 1, last_finish: 30.0, left: 3, download: Some(3.0),
            convert_avg: Some(4.0), bytes_per_item: Some(10e6), speed: Some(1e6), wait: 6.0,
            ..Job::default()
        };
        assert_eq!(time_left(&j), Some(7.0 + 2.0 * 20.0));
        // Converting, 1 s of its usual convert left.
        let j2 = Job { download: None, convert_left: Some(1.0), ..j };
        assert_eq!(time_left(&j2), Some(1.0 + 2.0 * 20.0));
        // In the pause before the next item, 5 s to go: those 5 s, that item, then one more.
        let j3 = Job { download: None, waiting: Some(5.0), left: 2, ..j };
        assert_eq!(time_left(&j3), Some(5.0 + 14.0 + 20.0));
    }

    #[test]
    fn time_left_is_zero_when_done() {
        assert_eq!(time_left(&job(90.0, 3, 90.0, 0, None, 6.0)), Some(0.0));
    }

    #[test]
    fn smooth_rate_starts_with_a_plain_average_then_fades_gradually() {
        let mut rate = SmoothRate::new(5.0);
        rate.push(0.0, 0, false);
        rate.push(0.5, 500, false);
        assert_eq!(rate.rate(), None);
        rate.push(1.0, 1_000, false);
        assert_eq!(rate.rate(), Some(1_000.0));
        // The speed drops to zero: the average falls a little each second, not all at once.
        let mut seen = vec![];
        for t in 2..=6 {
            rate.push(t as f64, 1_000, false);
            seen.push(rate.rate().unwrap());
        }
        assert!(seen.windows(2).all(|w| w[1] < w[0]));
        // After one time constant (5 s) about e^-1 of it is left.
        assert!((seen[4] - 1_000.0 * (-1.0f64).exp()).abs() < 1.0, "{seen:?}");
    }

    #[test]
    fn smooth_rate_weights_by_real_time() {
        // Ten 0.1 s samples move it as much as one 1 s sample.
        let mut a = SmoothRate::new(5.0);
        let mut b = SmoothRate::new(5.0);
        for r in [&mut a, &mut b] {
            r.push(0.0, 0, false);
            r.push(1.0, 1_000, false);
        }
        a.push(2.0, 4_000, false);
        for step in 1..=10 {
            b.push(1.0 + step as f64 * 0.1, 1_000 + step * 300, false);
        }
        assert!((a.rate().unwrap() - b.rate().unwrap()).abs() < 5.0);
    }

    #[test]
    fn smooth_rate_leaves_out_the_pause() {
        let mut rate = SmoothRate::new(5.0);
        rate.push(0.0, 0, false);
        rate.push(2.0, 2_000, false);
        for t in 3..=20 {
            rate.push(t as f64, 2_000, true);
        }
        assert_eq!(rate.rate(), Some(1_000.0));
        rate.push(21.0, 2_000, false);
        rate.push(22.0, 3_000, false);
        assert_eq!(rate.rate(), Some(1_000.0));
        // A drop in the total starts measuring again without losing the speed.
        rate.push(23.0, 10, false);
        assert_eq!(rate.rate(), Some(1_000.0));
    }

    fn tick_through(eta: &mut Countdown, from: f64, to: f64) -> Vec<f64> {
        let mut out = vec![];
        let mut t = from;
        while t < to - 1e-9 {
            t += 0.1;
            eta.tick(t);
            out.push(eta.show().unwrap());
        }
        out
    }

    #[test]
    fn countdown_ticks_and_eases_down() {
        let mut eta = Countdown::new();
        assert_eq!(eta.show(), None);
        assert!(eta.due(0.0, 3));
        eta.sync(0.0, 3, Some(100.0));
        eta.tick(1.0);
        assert!((eta.show().unwrap() - 99.0).abs() < 1e-9);
        // New math says 90 at 5 s, where the countdown says 95: no jump, it closes the gap.
        tick_through(&mut eta, 1.0, 5.0);
        assert!(eta.due(5.0, 3));
        eta.sync(5.0, 3, Some(90.0));
        assert!((eta.show().unwrap() - 95.0).abs() < 1e-6);
        let seen = tick_through(&mut eta, 5.0, 15.0);
        // Always going down, faster than the clock at first, landing on the estimate.
        assert!(seen.windows(2).all(|w| w[1] < w[0]));
        assert!(seen[9] < 95.0 - 1.0 - 0.5);
        assert!((seen.last().unwrap() - 80.0).abs() < 0.3, "{}", seen.last().unwrap());
    }

    #[test]
    fn countdown_never_counts_up_when_the_estimate_grows() {
        let mut eta = Countdown::new();
        eta.sync(0.0, 3, Some(100.0));
        tick_through(&mut eta, 0.0, 5.0);
        eta.sync(5.0, 3, Some(120.0));
        let seen = tick_through(&mut eta, 5.0, 25.0);
        assert!(seen.windows(2).all(|w| w[1] < w[0]));
        // It slowed down: after 20 s it is well above 95 - 20.
        assert!(*seen.last().unwrap() > 85.0, "{}", seen.last().unwrap());
    }

    #[test]
    fn countdown_jumps_on_a_huge_gap_or_an_item_finishing() {
        let mut eta = Countdown::new();
        eta.sync(0.0, 3, Some(100.0));
        eta.sync(5.0, 3, Some(300.0));
        assert_eq!(eta.show(), Some(300.0));
        assert!(eta.due(5.5, 2));
        eta.sync(5.5, 2, Some(250.0));
        assert_eq!(eta.show(), Some(250.0));
        // No estimate: keeps counting down.
        eta.sync(6.0, 2, None);
        assert!((eta.show().unwrap() - 249.5).abs() < 1e-6);
        eta.tick(10_000.0);
        assert_eq!(eta.show(), Some(0.0));
    }

    #[test]
    fn reads_the_pause_countdown() {
        assert_eq!(waiting_seconds("Waiting 12 seconds"), Some(12.0));
        assert_eq!(waiting_seconds("Waiting 1 seconds"), Some(1.0));
        assert_eq!(waiting_seconds("Unavailable"), None);
    }
}
