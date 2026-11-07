//! `configuration.conf`: the plain settings file shared by the oiuy-retro Windows apps.
//!
//! INI style, UTF-8. `#` starts a comment line, `[section]` starts a section,
//! and settings are `key = value`. Section and key names ignore case, and
//! unknown sections or keys are left alone. Saving one setting rewrites only
//! that one line, so comments and hand edits stay where the user put them.

use std::fs;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "configuration.conf";
pub const APP_FOLDER: &str = "link-download";

pub const TEMPLATE: &str = "\
# Link Download settings.
# Lines that start with # are notes and are ignored.
# This file uses the same layout as the other oiuy-retro Windows apps.

[tools]
# Where to find each helper program. Each line can be:
#   auto  = look next to Link Download.exe, then in each folder beside it
#           (in alphabetical order), then in a bin folder inside each of those,
#           and last on PATH.
#   path  = only look on PATH.
#   anything else = the program's file. A relative path starts from the folder
#           Link Download.exe is in. %USERPROFILE% and other %NAMES% are filled in.
# Example: ffmpeg = D:\\apps\\ffmpeg\\bin\\ffmpeg.exe
yt-dlp = auto
ffmpeg = auto
# On auto, ffprobe is looked for first in the folder ffmpeg was found in.
ffprobe = auto
node = auto

[folders]
# The last folder each Browse button opened. Link Download updates these lines.

[link-download]
# Settings only Link Download uses. Profiles are still kept in
# %APPDATA%\\Link Download\\profiles.json.
";

/// One setting found in the file, with its 1-based line number for error messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub value: String,
    pub line: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Conf {
    text: String,
    /// The file this was read from, or None when running on defaults.
    pub path: Option<PathBuf>,
}

enum Line<'a> {
    Section(&'a str),
    Pair(&'a str, &'a str),
    Other,
}

fn classify(line: &str) -> Line<'_> {
    let trimmed = line.trim().trim_start_matches('\u{feff}');
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
        return Line::Other;
    }
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(name) = rest.strip_suffix(']') {
            return Line::Section(name.trim());
        }
        return Line::Other;
    }
    match trimmed.split_once('=') {
        Some((key, value)) => Line::Pair(key.trim(), value.trim()),
        None => Line::Other,
    }
}

impl Conf {
    pub fn parse(text: &str) -> Self {
        Self { text: text.to_string(), path: None }
    }

    #[allow(dead_code)]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The last `key = value` for this key in this section. Surrounding quotes are dropped.
    pub fn get(&self, section: &str, key: &str) -> Option<Entry> {
        let mut current = String::new();
        let mut found = None;
        for (index, line) in self.text.lines().enumerate() {
            match classify(line) {
                Line::Section(name) => current = name.to_string(),
                Line::Pair(k, v) if current.eq_ignore_ascii_case(section) && k.eq_ignore_ascii_case(key) => {
                    found = Some(Entry { value: unquote(v).to_string(), line: index + 1 });
                }
                _ => {}
            }
        }
        found
    }

    /// Sets one value. Only the line holding that key changes; when the key is
    /// missing it is added at the end of its section, and a missing section is
    /// added at the end of the file.
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        let newline = if self.text.contains("\r\n") { "\r\n" } else { "\n" };
        let mut lines: Vec<String> = self.text.lines().map(str::to_string).collect();
        let mut current = String::new();
        let mut hit = None;
        let mut section_end = None;
        for (index, line) in lines.iter().enumerate() {
            match classify(line) {
                Line::Section(name) => current = name.to_string(),
                Line::Pair(k, _) if current.eq_ignore_ascii_case(section) => {
                    section_end = Some(index + 1);
                    if k.eq_ignore_ascii_case(key) {
                        hit = Some(index);
                    }
                }
                _ => {}
            }
            if current.eq_ignore_ascii_case(section) && section_end.is_none() {
                if let Line::Section(_) = classify(line) {
                    section_end = Some(index + 1);
                }
            }
        }
        if let Some(index) = hit {
            let old = &lines[index];
            let indent: String = old.chars().take_while(|c| c.is_whitespace()).collect();
            let name = old.trim_start().split('=').next().unwrap_or(key).trim_end().to_string();
            lines[index] = format!("{indent}{name} = {value}");
        } else if let Some(end) = section_end {
            lines.insert(end, format!("{key} = {value}"));
        } else {
            if lines.last().map(|l| !l.trim().is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            lines.push(format!("[{section}]"));
            lines.push(format!("{key} = {value}"));
        }
        let mut text = lines.join(newline);
        text.push_str(newline);
        self.text = text;
    }
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

/// Fills in `%NAME%` from the environment. Unknown names and a lone `%` stay as typed.
pub fn expand_vars(text: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match lookup(name) {
                    Some(value) => out.push_str(&value),
                    None => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn env_lookup(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// `%APPDATA%\link-download\configuration.conf`, used when the program folder can't be written to.
pub fn fallback_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join(APP_FOLDER).join(FILE_NAME))
}

fn write_file(path: &Path, text: &str) -> bool {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(path, text).is_ok()
}

/// Reads the file beside the program, else the AppData copy. When neither
/// exists, writes the commented template (beside the program when it can,
/// else in AppData). Never fails: without a file it runs on defaults.
pub fn load_from(exe_dir: &Path, fallback: Option<&Path>) -> Conf {
    let local = exe_dir.join(FILE_NAME);
    let mut candidates = vec![local.clone()];
    if let Some(path) = fallback {
        candidates.push(path.to_path_buf());
    }
    for path in &candidates {
        if let Ok(bytes) = fs::read(path) {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            return Conf { text, path: Some(path.clone()) };
        }
    }
    for path in &candidates {
        if write_file(path, TEMPLATE) {
            return Conf { text: TEMPLATE.into(), path: Some(path.clone()) };
        }
    }
    Conf { text: TEMPLATE.into(), path: None }
}

pub fn load() -> Conf {
    match crate::tools::exe_dir() {
        Ok(dir) => load_from(&dir, fallback_path().as_deref()),
        Err(_) => Conf::parse(TEMPLATE),
    }
}

/// Saves one setting. The file is read again first so edits made while the
/// app was open are kept. If the file can't be written, the whole text is
/// saved to the AppData copy instead; if that fails too, nothing is saved.
#[allow(dead_code)]
pub fn save_key_in(exe_dir: &Path, fallback: Option<&Path>, section: &str, key: &str, value: &str) -> bool {
    let mut conf = load_from(exe_dir, fallback);
    conf.set(section, key, value);
    if let Some(path) = &conf.path {
        if write_file(path, &conf.text) {
            return true;
        }
    }
    match fallback {
        Some(path) if conf.path.as_deref() != Some(path) => write_file(path, &conf.text),
        _ => false,
    }
}

#[allow(dead_code)]
pub fn save_key(section: &str, key: &str, value: &str) -> bool {
    match crate::tools::exe_dir() {
        Ok(dir) => save_key_in(&dir, fallback_path().as_deref(), section, key, value),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ld-conf-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn reads_sections_keys_and_comments() {
        let conf = Conf::parse("# note\n[Tools]\nFFmpeg = auto\n  # yt-dlp = x\nnode=\"C:\\n\\node.exe\"\n[other]\nffmpeg = no\n");
        assert_eq!(conf.get("tools", "ffmpeg").unwrap(), Entry { value: "auto".into(), line: 3 });
        assert_eq!(conf.get("TOOLS", "NODE").unwrap().value, "C:\\n\\node.exe");
        assert!(conf.get("tools", "yt-dlp").is_none());
        assert!(conf.get("missing", "ffmpeg").is_none());
    }

    #[test]
    fn template_parses_to_auto() {
        let conf = Conf::parse(TEMPLATE);
        for key in ["yt-dlp", "ffmpeg", "ffprobe", "node"] {
            assert_eq!(conf.get("tools", key).unwrap().value, "auto");
        }
    }

    #[test]
    fn expands_known_vars_only() {
        let lookup = |name: &str| (name.eq_ignore_ascii_case("USERPROFILE")).then(|| "C:\\Users\\me".to_string());
        assert_eq!(expand_vars("%USERPROFILE%\\bin\\x.exe", lookup), "C:\\Users\\me\\bin\\x.exe");
        assert_eq!(expand_vars("%NOPE%\\x", lookup), "%NOPE%\\x");
        assert_eq!(expand_vars("100% sure", lookup), "100% sure");
        assert_eq!(expand_vars("%%", lookup), "%%");
    }

    #[test]
    fn set_rewrites_only_that_line() {
        let text = "# top\n[folders]\n# keep me\n  OUTPUT = D:\\old\nother = 1\n\n[tools]\nffmpeg = auto\n";
        let mut conf = Conf::parse(text);
        conf.set("folders", "output", "E:\\new");
        assert_eq!(conf.text(), "# top\n[folders]\n# keep me\n  OUTPUT = E:\\new\nother = 1\n\n[tools]\nffmpeg = auto\n");
    }

    #[test]
    fn set_adds_missing_key_and_section() {
        let mut conf = Conf::parse("[folders]\n# note\n\n[tools]\nffmpeg = auto\n");
        conf.set("folders", "output", "D:\\x");
        assert_eq!(conf.text(), "[folders]\noutput = D:\\x\n# note\n\n[tools]\nffmpeg = auto\n");
        let mut conf = Conf::parse("[folders]\na = 1\n# note\n[tools]\n");
        conf.set("folders", "b", "2");
        assert_eq!(conf.text(), "[folders]\na = 1\nb = 2\n# note\n[tools]\n");
        let mut conf = Conf::parse("[tools]\nffmpeg = auto\n");
        conf.set("folders", "output", "D:\\x");
        assert_eq!(conf.text(), "[tools]\nffmpeg = auto\n\n[folders]\noutput = D:\\x\n");
    }

    #[test]
    fn set_keeps_crlf() {
        let mut conf = Conf::parse("[folders]\r\noutput = a\r\n");
        conf.set("folders", "output", "b");
        assert_eq!(conf.text(), "[folders]\r\noutput = b\r\n");
    }

    #[test]
    fn missing_file_gets_template_and_falls_back() {
        let dir = scratch("tpl");
        let conf = load_from(&dir, None);
        assert_eq!(conf.path.as_deref(), Some(dir.join(FILE_NAME).as_path()));
        assert_eq!(fs::read_to_string(dir.join(FILE_NAME)).unwrap(), TEMPLATE);

        // A program folder that can't hold the file: a file sits where the folder should be.
        let blocked = dir.join("blocked");
        fs::write(&blocked, "x").unwrap();
        let fallback = dir.join("appdata").join(APP_FOLDER).join(FILE_NAME);
        let conf = load_from(&blocked, Some(&fallback));
        assert_eq!(conf.path.as_deref(), Some(fallback.as_path()));
        assert!(fallback.is_file());
        assert!(save_key_in(&blocked, Some(&fallback), "folders", "output", "D:\\x"));
        assert_eq!(Conf::parse(&fs::read_to_string(&fallback).unwrap()).get("folders", "output").unwrap().value, "D:\\x");

        // Nowhere to write: defaults, no crash.
        let conf = load_from(&blocked, Some(&blocked.join("no").join(FILE_NAME)));
        assert!(conf.path.is_none());
        assert_eq!(conf.get("tools", "ffmpeg").unwrap().value, "auto");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_key_keeps_user_edits() {
        let dir = scratch("save");
        fs::write(dir.join(FILE_NAME), "# mine\n[tools]\nffmpeg = D:\\f.exe\n").unwrap();
        assert!(save_key_in(&dir, None, "folders", "output", "E:\\out"));
        assert_eq!(fs::read_to_string(dir.join(FILE_NAME)).unwrap(), "# mine\n[tools]\nffmpeg = D:\\f.exe\n\n[folders]\noutput = E:\\out\n");
        let _ = fs::remove_dir_all(&dir);
    }
}
