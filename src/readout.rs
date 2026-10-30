//! The same readouts as Folder Difference Copy: speed, items per second, files left, time left, elapsed.

use std::collections::VecDeque;

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

/// Seconds left, from the whole run so far. `elapsed` is seconds since the run started,
/// `last_finish` is when the latest of the `finished` items ended, `download` is how
/// many seconds the active item's download still needs at the average speed of the
/// last few seconds (None when it is not downloading or the size is not known yet),
/// and `wait` is the average pause between items. None means there is not enough to
/// go on yet.
pub fn time_left(elapsed: f64, finished: u32, last_finish: f64, left: u32, download: Option<f64>, wait: f64) -> Option<f64> {
    if left == 0 {
        return Some(0.0);
    }
    let download = download.filter(|d| d.is_finite() && *d >= 0.0);
    let rest = (left - 1) as f64;
    if finished == 0 {
        // Early guess from the first item alone: what it took so far plus what its
        // download still needs, for it and for each item after it.
        let download = download?;
        if elapsed < 3.0 {
            return None;
        }
        let per = elapsed + download;
        return Some(download + (per + wait.max(0.0)) * rest);
    }
    // Each item has taken this long on average, pauses included.
    let per = last_finish.max(0.0) / finished as f64;
    let since = (elapsed - last_finish).max(0.0);
    // While the active item downloads, its own bytes left at the recent speed say how
    // long it needs. Otherwise count down from the last finish.
    let current = download.unwrap_or((per - since).max(0.0));
    Some(current + per * rest)
}

/// Bytes per second over the last `window` seconds, from (seconds, total bytes) samples.
/// Averaging over a few seconds keeps one fast or slow moment from swinging Time left.
pub struct RollingRate {
    window: f64,
    samples: VecDeque<(f64, u64)>,
}

impl RollingRate {
    pub fn new(window: f64) -> Self {
        Self { window, samples: VecDeque::new() }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn push(&mut self, at: f64, bytes: u64) {
        if let Some(&(_, last)) = self.samples.back() {
            // The total went down (an item failed and its bytes were dropped): start over.
            if bytes < last {
                self.samples.clear();
            }
        }
        self.samples.push_back((at, bytes));
        // Keep the newest sample that is at least `window` old, so the span is the full window.
        while self.samples.len() > 2 && at - self.samples[1].0 >= self.window {
            self.samples.pop_front();
        }
    }

    /// None until there is at least a second to average over.
    pub fn rate(&self) -> Option<f64> {
        let (old_at, old_bytes) = *self.samples.front()?;
        let (new_at, new_bytes) = *self.samples.back()?;
        let span = new_at - old_at;
        if span < 1.0 {
            return None;
        }
        Some(new_bytes.saturating_sub(old_bytes) as f64 / span)
    }
}

/// Time left as shown: worked out every `EVERY` seconds, and counted down by the clock
/// in between, so the readout ticks down once a second instead of jumping around.
pub struct Countdown {
    /// When the last estimate was worked out, and what it was.
    synced: Option<(f64, f64)>,
    math_at: Option<f64>,
    left: u32,
}

impl Countdown {
    pub const EVERY: f64 = 5.0;

    pub fn new() -> Self {
        Self { synced: None, math_at: None, left: 0 }
    }

    /// Time for new math: the first time, every `EVERY` seconds, and whenever the
    /// number of items left changes.
    pub fn due(&self, now: f64, left: u32) -> bool {
        match self.math_at {
            None => true,
            Some(at) => now - at >= Self::EVERY || left != self.left,
        }
    }

    /// New math. None (not enough to go on) keeps counting down the last estimate.
    pub fn sync(&mut self, now: f64, left: u32, estimate: Option<f64>) {
        self.math_at = Some(now);
        self.left = left;
        if let Some(seconds) = estimate.filter(|s| s.is_finite() && *s >= 0.0) {
            self.synced = Some((now, seconds));
        }
    }

    pub fn show(&self, now: f64) -> Option<f64> {
        self.synced.map(|(at, seconds)| (seconds - (now - at).max(0.0)).max(0.0))
    }
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

    #[test]
    fn time_left_waits_for_something_to_go_on() {
        assert_eq!(time_left(0.0, 0, 0.0, 3, None, 6.0), None);
        assert_eq!(time_left(10.0, 0, 0.0, 3, None, 6.0), None);
        assert_eq!(time_left(1.0, 0, 0.0, 3, Some(10.0), 6.0), None);
    }

    #[test]
    fn time_left_guesses_from_the_first_item() {
        // 10 s in, 10 s of download left at the recent speed: 10 s more for it,
        // then 20 s plus a 6 s pause for each of the 2 after it.
        assert_eq!(time_left(10.0, 0, 0.0, 3, Some(10.0), 6.0), Some(10.0 + 26.0 * 2.0));
    }

    #[test]
    fn time_left_after_one_item() {
        // One item took 30 s. Two left, nothing downloading yet.
        assert_eq!(time_left(30.0, 1, 30.0, 2, None, 6.0), Some(60.0));
        // Ten seconds later it has counted down by ten.
        assert_eq!(time_left(40.0, 1, 30.0, 2, None, 6.0), Some(50.0));
    }

    #[test]
    fn time_left_uses_the_download_speed_while_downloading() {
        // 50 s into an item that usually takes 30 s, with 4 s of download left: 4 s, then 30 s.
        assert_eq!(time_left(80.0, 1, 30.0, 2, Some(4.0), 6.0), Some(34.0));
    }

    #[test]
    fn time_left_is_zero_when_done() {
        assert_eq!(time_left(90.0, 3, 90.0, 0, None, 6.0), Some(0.0));
    }

    #[test]
    fn rolling_rate_averages_the_last_five_seconds() {
        let mut rate = RollingRate::new(5.0);
        rate.push(0.0, 0);
        assert_eq!(rate.rate(), None);
        rate.push(0.5, 500);
        assert_eq!(rate.rate(), None);
        // One fast second, then slow ones: the average is over the whole window.
        rate.push(1.0, 10_000);
        assert_eq!(rate.rate(), Some(10_000.0));
        for t in 2..=6 {
            rate.push(t as f64, 10_000 + (t as u64 - 1) * 1_000);
        }
        // Window now 1 s .. 6 s: 5,000 bytes in 5 s.
        assert_eq!(rate.rate(), Some(1_000.0));
        // A burst of 100 ms samples does not shrink the window.
        for step in 1..=10 {
            rate.push(6.0 + step as f64 * 0.1, 15_000 + step * 100);
        }
        let r = rate.rate().unwrap();
        assert!((r - (16_000.0 - 11_000.0) / 5.0).abs() < 1e-6, "{r}");
    }

    #[test]
    fn rolling_rate_starts_over_when_the_total_drops() {
        let mut rate = RollingRate::new(5.0);
        rate.push(0.0, 0);
        rate.push(2.0, 4_000);
        rate.push(3.0, 100);
        assert_eq!(rate.rate(), None);
        rate.push(4.0, 1_100);
        assert_eq!(rate.rate(), Some(1_000.0));
    }

    #[test]
    fn countdown_ticks_between_updates() {
        let mut eta = Countdown::new();
        assert_eq!(eta.show(0.0), None);
        assert!(eta.due(0.0, 3));
        eta.sync(0.0, 3, Some(100.0));
        assert!(!eta.due(1.0, 3));
        assert_eq!(eta.show(1.0), Some(99.0));
        assert_eq!(eta.show(4.5), Some(95.5));
        // New math comes every five seconds and re-syncs the readout.
        assert!(eta.due(5.0, 3));
        eta.sync(5.0, 3, Some(120.0));
        assert_eq!(eta.show(6.0), Some(119.0));
        // An item finishing is new math at once.
        assert!(eta.due(6.5, 2));
        // No estimate this time: keep counting down the last one.
        eta.sync(6.5, 2, None);
        assert_eq!(eta.show(7.0), Some(118.0));
        // Never below zero.
        assert_eq!(eta.show(1_000.0), Some(0.0));
    }
}
