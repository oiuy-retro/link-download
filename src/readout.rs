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

/// Time left with a unit on every field: 12D:05H:05M:04S. Leading units that are
/// zero are left out (01H:05M:04S, 05M:04S), but minutes and seconds always show.
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
        format!("{days:02}D:{hours:02}H:{minutes:02}M:{secs:02}S")
    } else if hours > 0 {
        format!("{hours:02}H:{minutes:02}M:{secs:02}S")
    } else {
        format!("{minutes:02}M:{secs:02}S")
    }
}

/// Seconds left, from the whole run so far. `elapsed` is seconds since the run started,
/// `last_finish` is when the latest of the `finished` items ended, `progress` is how far
/// the active item is (0 to 1), and `wait` is the average pause between items.
/// None means there is not enough to go on yet.
pub fn time_left(elapsed: f64, finished: u32, last_finish: f64, left: u32, progress: Option<f64>, wait: f64) -> Option<f64> {
    if left == 0 {
        return Some(0.0);
    }
    let progress = progress.filter(|p| p.is_finite()).unwrap_or(0.0).clamp(0.0, 1.0);
    let rest = (left - 1) as f64;
    if finished == 0 {
        // Early guess from the first item alone, once it is far enough along to trust.
        if progress < 0.2 || elapsed < 3.0 {
            return None;
        }
        let per = elapsed / progress;
        return Some(per * (1.0 - progress) + (per + wait.max(0.0)) * rest);
    }
    // Each item has taken this long on average, pauses included.
    let per = last_finish.max(0.0) / finished as f64;
    let since = (elapsed - last_finish).max(0.0);
    // Count down from the last finish. If this item runs long, fall back to its own progress
    // (the pause before it is already over, so only the download part is left).
    let floor = if progress > 0.0 { (per - wait).max(0.0) * (1.0 - progress) } else { 0.0 };
    let current = (per - since).max(floor);
    Some(current + per * rest)
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
        assert_eq!(format_elapsed(75), "1:15");
        assert_eq!(format_elapsed(3661), "1:01:01");
        assert_eq!(percent(0.999), 99);
        assert_eq!(percent(1.0), 100);
    }

    #[test]
    fn time_left_shows_units() {
        assert_eq!(format_eta(0.0), "00M:00S");
        assert_eq!(format_eta(2.0), "00M:02S");
        assert_eq!(format_eta(90.0), "01M:30S");
        assert_eq!(format_eta(59.6), "01M:00S");
        assert_eq!(format_eta(3599.0), "59M:59S");
        assert_eq!(format_eta(3600.0), "01H:00M:00S");
        assert_eq!(format_eta(5.0 * 3600.0 + 10.0 * 60.0), "05H:10M:00S");
        assert_eq!(format_eta(86_400.0), "01D:00H:00M:00S");
        assert_eq!(format_eta(12.0 * 86_400.0 + 5.0 * 3600.0 + 5.0 * 60.0 + 4.0), "12D:05H:05M:04S");
        assert_eq!(format_eta(120.0 * 86_400.0), "120D:00H:00M:00S");
        assert_eq!(format_eta(-1.0), "Calculating...");
        assert_eq!(format_eta(f64::NAN), "Calculating...");
    }

    #[test]
    fn time_left_waits_for_something_to_go_on() {
        assert_eq!(time_left(0.0, 0, 0.0, 3, None, 6.0), None);
        assert_eq!(time_left(10.0, 0, 0.0, 3, Some(0.1), 6.0), None);
        assert_eq!(time_left(1.0, 0, 0.0, 3, Some(0.5), 6.0), None);
    }

    #[test]
    fn time_left_guesses_from_the_first_item() {
        // Half done after 10 s: 10 s more for it, then 20 s plus a 6 s pause for each of the 2 after it.
        assert_eq!(time_left(10.0, 0, 0.0, 3, Some(0.5), 6.0), Some(10.0 + 26.0 * 2.0));
    }

    #[test]
    fn time_left_after_one_item() {
        // One item took 30 s. Two left, nothing started yet.
        assert_eq!(time_left(30.0, 1, 30.0, 2, None, 6.0), Some(60.0));
        // Ten seconds later it has counted down by ten.
        assert_eq!(time_left(40.0, 1, 30.0, 2, Some(0.0), 6.0), Some(50.0));
    }

    #[test]
    fn time_left_uses_the_active_item_when_it_runs_long() {
        // 50 s into an item that usually takes 30 s (6 s of it the pause), and 80% done:
        // a fifth of 24 s for it, then 30 s for the last one.
        let left = time_left(80.0, 1, 30.0, 2, Some(0.8), 6.0).unwrap();
        assert!((left - 34.8).abs() < 1e-9);
        // Still on schedule: the countdown wins.
        assert_eq!(time_left(40.0, 1, 30.0, 2, Some(0.5), 6.0), Some(50.0));
    }

    #[test]
    fn time_left_is_zero_when_done() {
        assert_eq!(time_left(90.0, 3, 90.0, 0, None, 6.0), Some(0.0));
    }
}
