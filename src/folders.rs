//! Each Browse button remembers the last folder it picked, in the `[folders]`
//! section of configuration.conf. Link Download has one: the Location
//! button, saved as `output`.

use std::path::{Path, PathBuf};

use crate::conf::{self, Conf};

/// The `[folders]` key for the Location Browse button.
pub const OUTPUT: &str = "output";

/// The folder itself when it exists, else the closest parent that does.
pub fn nearest_existing(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        if !dir.as_os_str().is_empty() && dir.is_dir() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

/// Where a picker opens: its remembered folder (or the closest parent that
/// still exists), else what is in its text box, else wherever Windows chooses.
pub fn start_folder(conf: &Conf, key: &str, current: &str) -> Option<PathBuf> {
    let remembered = conf
        .get("folders", key)
        .map(|entry| conf::expand_vars(entry.value.trim(), conf::env_lookup))
        .filter(|value| !value.is_empty());
    for text in remembered.iter().map(String::as_str).chain([current.trim()]) {
        if text.is_empty() {
            continue;
        }
        let path = Path::new(text);
        if !path.is_absolute() {
            continue;
        }
        if let Some(found) = nearest_existing(path) {
            return Some(found);
        }
    }
    None
}

pub fn start(key: &str, current: &str) -> Option<PathBuf> {
    start_folder(&conf::load(), key, current)
}

/// Saves the picked folder; only that one line of the file changes.
pub fn remember(key: &str, folder: &str) {
    let _ = conf::save_key("folders", key, folder);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ld-folders-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn nearest_existing_walks_up() {
        let dir = scratch("near");
        fs::create_dir_all(dir.join("a")).unwrap();
        assert_eq!(nearest_existing(&dir.join("a")), Some(dir.join("a")));
        assert_eq!(nearest_existing(&dir.join("a").join("gone").join("deeper")), Some(dir.join("a")));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn remembered_folder_wins_then_text_box() {
        let dir = scratch("start");
        fs::create_dir_all(dir.join("kept")).unwrap();
        fs::create_dir_all(dir.join("box")).unwrap();
        let mut conf = Conf::parse("[folders]\n");
        conf.set("folders", OUTPUT, &dir.join("kept").display().to_string());
        let boxed = dir.join("box").display().to_string();
        assert_eq!(start_folder(&conf, OUTPUT, &boxed), Some(dir.join("kept")));
        // The remembered folder was deleted: open at its parent, quietly.
        fs::remove_dir_all(dir.join("kept")).unwrap();
        assert_eq!(start_folder(&conf, OUTPUT, &boxed), Some(dir.clone()));
        // Nothing remembered: the text box.
        assert_eq!(start_folder(&Conf::parse(""), OUTPUT, &boxed), Some(dir.join("box")));
        // Nothing usable at all.
        assert_eq!(start_folder(&Conf::parse("[folders]\noutput = relative\\x\n"), OUTPUT, ""), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn remembering_rewrites_only_its_line() {
        let dir = scratch("save");
        let file = dir.join(conf::FILE_NAME);
        fs::write(&file, "# mine\n[folders]\n# Location button\noutput = D:\\old\ninput = D:\\in\n[tools]\nffmpeg = auto\n").unwrap();
        assert!(conf::save_key_in(&dir, None, "folders", OUTPUT, "E:\\new"));
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "# mine\n[folders]\n# Location button\noutput = E:\\new\ninput = D:\\in\n[tools]\nffmpeg = auto\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
