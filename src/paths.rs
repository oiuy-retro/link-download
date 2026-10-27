//! Finished file locations.
//!
//! A single video with one artist goes in Artist / file.
//! A playlist whose tracks all share that one artist goes in Artist / Album / file.
//! A playlist that does not share one artist, and a single video with several
//! artists and an album, goes in Album / file.

use std::path::{Path, PathBuf};

use crate::model::Group;

/// A character Windows does not allow anywhere in a file name.
pub fn is_invalid_char(ch: char) -> bool {
    ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
}

/// Shown in place of a control character, which has no picture of its own.
pub const CONTROL_MARK: char = '\u{a4}';
/// Shown in place of a trailing space, so the marked character can be seen.
pub const SPACE_MARK: char = '\u{b7}';

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Plain,
    /// A character Windows never allows.
    Bad,
    /// A space or period that is only wrong because it is last.
    End,
}

/// The name as it is drawn, one entry per character, with how each one is marked.
/// Reserved and too-long names are not about one character, so nothing is marked.
pub fn name_marks(name: &str) -> Vec<(char, Mark)> {
    let mut marks: Vec<(char, Mark)> = name
        .chars()
        .map(|ch| {
            if ch.is_control() {
                (CONTROL_MARK, Mark::Bad)
            } else if is_invalid_char(ch) {
                (ch, Mark::Bad)
            } else {
                (ch, Mark::Plain)
            }
        })
        .collect();
    if !name.trim().is_empty() {
        if let Some(last) = marks.last_mut() {
            if last.0 == ' ' {
                *last = (SPACE_MARK, Mark::End);
            } else if last.0 == '.' {
                last.1 = Mark::End;
            }
        }
    }
    marks
}

pub fn sanitize(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if is_invalid_char(ch) {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return "Untitled".into();
    }
    let upper = trimmed.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or("");
    let reserved = matches!(
        stem,
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    let mut text = if reserved { format!("_{trimmed}") } else { trimmed.to_string() };
    if text.chars().count() > 250 {
        text = text.chars().take(250).collect();
        text = text.trim().trim_end_matches(['.', ' ']).to_string();
    }
    if text.is_empty() { "Untitled".into() } else { text }
}

/// The first illegal file name in this list, or nothing when every saved name is legal.
pub fn name_problem_line(items: &[crate::model::Item]) -> Option<String> {
    first_name_problem(items).map(|(_, line)| line)
}

/// The first illegal file name and the whole problem line. The line starts with the name
/// and ": " unless the name is blank.
pub fn first_name_problem(items: &[crate::model::Item]) -> Option<(String, String)> {
    let mut count = 0usize;
    let mut first: Option<(&str, &str)> = None;
    for item in items.iter().filter(|item| item.blocked.is_none()) {
        if let Some(problem) = filename_problem(&item.title) {
            count += 1;
            if first.is_none() {
                first = Some((item.title.as_str(), problem));
            }
        }
    }
    let (name, problem) = first?;
    let shown = if name.trim().is_empty() { "" } else { name };
    let detail = if shown.is_empty() { problem.to_string() } else { format!("{name}: {problem}") };
    let line = if count > 1 { format!("{detail} {count} names need a change.") } else { detail };
    Some((shown.to_string(), line))
}

/// Why this text cannot be a Windows file name. `None` means it is allowed.
pub fn filename_problem(name: &str) -> Option<&'static str> {
    if name.trim().is_empty() {
        return Some("Type a file name.");
    }
    if name.chars().any(is_invalid_char) {
        return Some("A file name cannot contain \\ / : * ? \" < > |.");
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return Some("A file name cannot end with a space or a period.");
    }
    // Leave room for the dot and a 4-letter extension. Windows allows 255 for the whole name.
    if name.chars().count() > 250 {
        return Some("That file name is too long.");
    }
    let stem = name.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    if matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    ) {
        return Some("That file name is reserved by Windows.");
    }
    None
}

pub fn shared_single_artist(group: &Group) -> bool {
    let mut name: Option<&str> = None;
    let mut any = false;
    for item in group.items.iter().filter(|item| item.blocked.is_none()) {
        any = true;
        if item.artists.len() != 1 {
            return false;
        }
        let artist = item.artists[0].trim();
        if artist.is_empty() {
            return false;
        }
        match name {
            None => name = Some(artist),
            Some(have) if !have.eq_ignore_ascii_case(artist) => return false,
            _ => {}
        }
    }
    any
}

fn album_folder(group: &Group) -> Option<String> {
    let album = group.album.trim();
    if !album.is_empty() {
        return Some(sanitize(album));
    }
    if group.is_playlist {
        let title = group.playlist_title.trim();
        if !title.is_empty() {
            return Some(sanitize(title));
        }
        return Some("Playlist".into());
    }
    None
}

pub fn output_dir(location: &Path, group: &Group, item_index: usize) -> PathBuf {
    let item = &group.items[item_index];
    if !group.is_playlist {
        if item.artists.len() == 1 && !item.artists[0].trim().is_empty() {
            return location.join(sanitize(&item.artists[0]));
        }
        if item.artists.len() > 1 {
            if let Some(album) = album_folder(group) {
                return location.join(album);
            }
        }
        return location.to_path_buf();
    }
    let album = album_folder(group).unwrap_or_else(|| "Playlist".into());
    if shared_single_artist(group) {
        let artist = group
            .items
            .iter()
            .find(|item| item.blocked.is_none())
            .map(|item| sanitize(&item.artists[0]))
            .unwrap_or_else(|| "Artist".into());
        location.join(artist).join(album)
    } else {
        location.join(album)
    }
}

/// A file-name stem for a download in `dir`. The extension is chosen by yt-dlp.
pub fn download_stem(dir: &Path, title: &str) -> String {
    let base = sanitize(title).replace('%', "");
    let base = if base.is_empty() { "Untitled".to_string() } else { base };
    if !stem_used(dir, &base) {
        return base;
    }
    for number in 2..10_000 {
        let candidate = format!("{base} ({number})");
        if !stem_used(dir, &candidate) {
            return candidate;
        }
    }
    format!("{base} copy")
}

/// The hidden folder where unfinished downloads are kept (see fix/cancel). It is
/// never a song, so name checks and the download search skip it.
pub const PARTIAL_DIR: &str = ".link-download-partial";

fn stem_used(dir: &Path, stem: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == PARTIAL_DIR {
            return false;
        }
        match name.strip_prefix(stem) {
            Some(rest) => rest.is_empty() || (rest.starts_with('.') && !is_ytdlp_leftover(&rest[1..])),
            None => false,
        }
    })
}

/// True for the pieces yt-dlp leaves behind for a download that did not
/// finish, given the part of the name after `stem.`:
/// `mkv.part`, `f137.mp4`, `f137.mp4.part`, `temp.mkv`, `mkv.ytdl`,
/// `f137.mp4.part-Frag3`. These must not push the next try to "Song (2)":
/// keeping the same name lets yt-dlp carry on from the partial file.
pub fn is_ytdlp_leftover(rest: &str) -> bool {
    let lower = rest.to_ascii_lowercase();
    if lower.ends_with(".part") || lower.ends_with(".ytdl") || lower.contains(".part-frag") {
        return true;
    }
    if lower.starts_with("temp.") {
        return true;
    }
    let first = lower.split('.').next().unwrap_or("");
    lower.contains('.') && first.len() > 1 && first.starts_with('f') && first[1..].chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && first[1..].chars().any(|c| c.is_ascii_digit())
}

/// True when `name` is the finished file yt-dlp makes for `stem`:
/// exactly `stem.ext`, one extension, and not a leftover or a side file.
/// "Song (Live).mkv", "Song.f137.mp4" and "Song.mkv.part" are not a match for "Song".
pub fn is_download_of(name: &str, stem: &str) -> bool {
    let Some(rest) = name.strip_prefix(stem).and_then(|rest| rest.strip_prefix('.')) else {
        return false;
    };
    if rest.is_empty() || rest.contains('.') || is_ytdlp_leftover(rest) {
        return false;
    }
    !matches!(
        rest.to_ascii_lowercase().as_str(),
        "part" | "ytdl" | "jpg" | "jpeg" | "png" | "webp" | "json" | "description" | "srt" | "vtt" | "lrc" | "txt" | "log"
    )
}

#[cfg(test)]
pub fn unique_path(dir: &Path, title: &str, extension: &str) -> PathBuf {
    unique_path_except(dir, title, extension, None)
}

/// Like `unique_path`, but `mine` counts as free. That is the file being
/// converted, which is about to be replaced, so "Song (2).m4a" can stay
/// "Song (2)" instead of jumping to "Song (3)".
pub fn unique_path_except(dir: &Path, title: &str, extension: &str, mine: Option<&Path>) -> PathBuf {
    let base = sanitize(title);
    let free = |path: &PathBuf| mine == Some(path.as_path()) || !path.exists();
    let first = dir.join(format!("{base}.{extension}"));
    if free(&first) {
        return first;
    }
    for number in 2..10_000 {
        let candidate = dir.join(format!("{base} ({number}).{extension}"));
        if free(&candidate) {
            return candidate;
        }
    }
    dir.join(format!("{base} (copy).{extension}"))
}

/// Where the finished file for `title` goes when nothing is in the way.
pub fn final_file(dir: &Path, title: &str, extension: &str) -> PathBuf {
    dir.join(format!("{}.{}", sanitize(title), extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ld-paths-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn leftovers_from_a_stopped_download_keep_the_same_name() {
        let dir = scratch("leftover");
        for name in ["Song.f137.mp4.part", "Song.f140.m4a", "Song.mkv.ytdl", "Song.temp.mkv", "Song.mkv.part"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        std::fs::create_dir_all(dir.join(PARTIAL_DIR).join("1234")).unwrap();
        assert_eq!(download_stem(&dir, "Song"), "Song");
        assert_eq!(download_stem(&dir, ".link-download-partial"), ".link-download-partial");
        std::fs::write(dir.join("Song.mkv"), b"x").unwrap();
        assert_eq!(download_stem(&dir, "Song"), "Song (2)");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_the_exact_stem_counts_as_the_download() {
        assert!(is_download_of("Song.mkv", "Song"));
        assert!(is_download_of("Song.m4a", "Song"));
        assert!(is_download_of("Song (2).mkv", "Song (2)"));
        assert!(!is_download_of("Song (Live).mkv", "Song"));
        assert!(!is_download_of("Song (2).mkv", "Song"));
        assert!(!is_download_of("Songbird.mkv", "Song"));
        assert!(!is_download_of("Song.f137.mp4", "Song"));
        assert!(!is_download_of("Song.mkv.part", "Song"));
        assert!(!is_download_of("Song.webp", "Song"));
        assert!(!is_download_of("Song", "Song"));
        assert!(is_ytdlp_leftover("f137.mp4"));
        assert!(is_ytdlp_leftover("f251-drc.webm"));
        assert!(!is_ytdlp_leftover("flac"));
        assert!(!is_ytdlp_leftover("mkv"));
    }

    #[test]
    fn the_file_being_converted_does_not_push_the_number_up() {
        let dir = scratch("except");
        std::fs::write(dir.join("Song.m4a"), b"old").unwrap();
        std::fs::write(dir.join("Song (2).m4a"), b"new").unwrap();
        let mine = dir.join("Song (2).m4a");
        assert_eq!(unique_path_except(&dir, "Song", "m4a", Some(&mine)), mine);
        assert_eq!(unique_path(&dir, "Song", "m4a"), dir.join("Song (3).m4a"));
        let _ = std::fs::remove_dir_all(&dir);
    }
    use crate::model::{Group, Item};

    fn item(title: &str, artists: &[&str]) -> Item {
        Item {
            id: title.into(),
            title: title.into(),
            meta_title: title.into(),
            original_title: title.into(),
            original_artists: artists.iter().map(|s| (*s).to_string()).collect(),
            artists: artists.iter().map(|s| (*s).to_string()).collect(),
            download_url: String::new(),
            playlist_item: None,
            track_number: None,
            duration: 0.0,
            thumbnail: String::new(),
            temp_path: None,
            blocked: None,
        }
    }

    fn group(playlist: bool, album: &str, items: Vec<Item>) -> Group {
        Group {
            source_url: "https://example.test/list".into(),
            is_playlist: playlist,
            playlist_title: "Playlist Title".into(),
            album: album.into(),
            original_album: album.into(),
            playlist_thumbnail: String::new(),
            error: String::new(),
            items,
        }
    }

    #[test]
    fn folders_follow_artist_and_album() {
        let root = Path::new("C:\\Music");
        let single = group(false, "", vec![item("Song", &["Queen"])]);
        assert_eq!(output_dir(root, &single, 0), PathBuf::from("C:\\Music\\Queen"));

        let album = group(true, "News Of The World", vec![item("Song", &["Queen"]), item("Next", &["Queen"])]);
        assert_eq!(
            output_dir(root, &album, 1),
            PathBuf::from("C:\\Music\\Queen\\News Of The World")
        );

        let mixed = group(
            true,
            "News",
            vec![item("A", &["Queen"]), item("B", &["Queen", "David Bowie"])],
        );
        assert_eq!(output_dir(root, &mixed, 0), PathBuf::from("C:\\Music\\News"));
        assert_eq!(output_dir(root, &mixed, 1), PathBuf::from("C:\\Music\\News"));

        let compilation = group(true, "Sampler", vec![item("A", &["Queen"]), item("B", &["Bowie"])]);
        assert_eq!(output_dir(root, &compilation, 0), PathBuf::from("C:\\Music\\Sampler"));

        let several = group(false, "Duet", vec![item("Song", &["Queen", "Bowie"])]);
        assert_eq!(output_dir(root, &several, 0), PathBuf::from("C:\\Music\\Duet"));

        let loose = group(false, "", vec![item("Song", &["Queen", "Bowie"])]);
        assert_eq!(output_dir(root, &loose, 0), PathBuf::from("C:\\Music"));
    }

    #[test]
    fn blank_playlist_album_still_uses_the_playlist_title_as_the_folder() {
        let root = Path::new("C:\\Music");
        let group = group(true, "", vec![item("A", &["Queen"]), item("B", &["Bowie"])]);
        assert_eq!(output_dir(root, &group, 0), PathBuf::from("C:\\Music\\Playlist Title"));
    }

    #[test]
    fn sanitize_replaces_forbidden_characters_and_reserved_names() {
        assert_eq!(sanitize("A:B*C?"), "A_B_C_");
        assert_eq!(sanitize("  CON  "), "_CON");
        assert_eq!(sanitize("   "), "Untitled");
        assert_eq!(sanitize("Song."), "Song");
    }

    #[test]
    fn rejects_names_windows_will_not_store() {
        assert!(filename_problem("Song").is_none());
        assert!(filename_problem("Song: Live").is_some());
        assert!(filename_problem("Song.").is_some());
        assert!(filename_problem("CON").is_some());
        assert!(filename_problem(" ").is_some());
    }

    #[test]
    fn invalid_chars_are_the_ones_windows_rejects() {
        for ch in ['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\u{7}', '\t', '\n', '\u{0}'] {
            assert!(is_invalid_char(ch), "{ch:?}");
        }
        for ch in ['a', ' ', '.', '-', '(', ')', '\'', '\u{e9}', '\u{3042}'] {
            assert!(!is_invalid_char(ch), "{ch:?}");
        }
        assert_eq!(sanitize("a:b?c"), "a_b_c");
        assert!(filename_problem("a|b").is_some());
    }

    fn marked(name: &str) -> String {
        name_marks(name)
            .iter()
            .map(|(ch, mark)| match mark {
                Mark::Plain => ch.to_string(),
                Mark::Bad => format!("[{ch}]"),
                Mark::End => format!("{{{ch}}}"),
            })
            .collect()
    }

    #[test]
    fn marks_fall_on_the_characters_that_make_a_name_illegal() {
        assert_eq!(marked("Good name"), "Good name");
        assert_eq!(marked("Bad: name? <x>"), "Bad[:] name[?] [<]x[>]");
        assert_eq!(marked("a/b\\c|d*e\"f"), "a[/]b[\\]c[|]d[*]e[\"]f");
        assert_eq!(marked("Trailing period."), "Trailing period{.}");
        assert_eq!(marked("Trailing space "), "Trailing space{\u{b7}}");
        assert_eq!(marked("Ends in a star*"), "Ends in a star[*]");
        assert_eq!(marked("Ctrl\u{7}char"), "Ctrl[\u{a4}]char");
        assert_eq!(marked("1.5 inner dots. ok"), "1.5 inner dots. ok");
        // Reserved and long names are a whole-name problem.
        assert_eq!(marked("CON"), "CON");
        assert_eq!(marked(&"x".repeat(300)), "x".repeat(300));
        assert_eq!(marked("   "), "   ");
        assert_eq!(marked(""), "");
    }

    #[test]
    fn first_name_problem_gives_the_name_and_the_line() {
        let items = vec![item("Fine", &[]), item("Bad?", &[]), item("Also bad.", &[])];
        let (name, line) = first_name_problem(&items).unwrap();
        assert_eq!(name, "Bad?");
        assert!(line.starts_with("Bad?: "));
        assert!(line.ends_with("2 names need a change."));
        assert_eq!(name_problem_line(&items), Some(line));
        let blank = vec![item(" ", &[])];
        let (name, line) = first_name_problem(&blank).unwrap();
        assert_eq!(name, "");
        assert_eq!(line, "Type a file name.");
        assert_eq!(first_name_problem(&[item("Fine", &[])]), None);
    }

    #[test]
    fn name_problem_line_names_the_first_illegal_file() {
        let mut blocked = item("No: Way", &[]);
        blocked.blocked = Some(crate::model::Block::Missing);
        let line = name_problem_line(&[item("Fine", &[]), item("Song: Live", &[]), item("CON", &[])]).unwrap();
        assert!(line.contains("Song: Live"));
        assert!(line.contains("cannot contain"));
        assert!(line.contains("2 names"));
        assert!(name_problem_line(&[blocked]).is_none());
        assert!(name_problem_line(&[item("Fine", &[])]).is_none());
    }
}
