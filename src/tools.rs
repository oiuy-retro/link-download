//! Finds yt-dlp, ffmpeg, ffprobe, and node using the `[tools]` section of
//! configuration.conf. See `conf.rs` for the file itself.

use std::path::{Path, PathBuf};

use crate::conf::{self, Conf};

#[derive(Clone, Debug)]
pub struct Bundle {
    pub dir: PathBuf,
    pub ytdlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub node: PathBuf,
}

pub fn exe_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|err| err.to_string())?;
    exe.parent().map(Path::to_path_buf).ok_or_else(|| "The program folder could not be found.".into())
}

/// The `[tools]` keys Link Download reads, with the file each one looks for.
pub const TOOLS: [(&str, &str); 4] = [("yt-dlp", "yt-dlp.exe"), ("ffmpeg", "ffmpeg.exe"), ("ffprobe", "ffprobe.exe"), ("node", "node.exe")];

/// Everything the search needs, so tests can run it without the real program folder or PATH.
pub struct Lookup<'a> {
    pub exe_dir: &'a Path,
    pub path_dirs: Vec<PathBuf>,
    pub env: &'a dyn Fn(&str) -> Option<String>,
    pub conf_name: String,
}

fn file_in(dir: &Path, name: &str) -> Option<PathBuf> {
    let candidate = dir.join(name);
    candidate.is_file().then_some(candidate)
}

/// Folders directly inside `dir`, sorted by name ignoring case.
fn subfolders(dir: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).collect())
        .unwrap_or_default();
    dirs.sort_by_key(|path| path.file_name().map(|name| name.to_string_lossy().to_lowercase()).unwrap_or_default());
    dirs
}

/// The `auto` order: next to the program, each direct subfolder (A to Z),
/// then `bin` inside each subfolder (A to Z), then PATH.
pub fn search_auto(name: &str, look: &Lookup, first: Option<&Path>) -> Option<PathBuf> {
    if let Some(found) = first.and_then(|dir| file_in(dir, name)) {
        return Some(found);
    }
    if let Some(found) = file_in(look.exe_dir, name) {
        return Some(found);
    }
    let subs = subfolders(look.exe_dir);
    if let Some(found) = subs.iter().find_map(|dir| file_in(dir, name)) {
        return Some(found);
    }
    if let Some(found) = subs.iter().find_map(|dir| file_in(&dir.join("bin"), name)) {
        return Some(found);
    }
    search_path(name, look)
}

fn search_path(name: &str, look: &Lookup) -> Option<PathBuf> {
    look.path_dirs.iter().find_map(|dir| file_in(dir, name))
}

/// Resolves one `[tools]` key. `first` is a folder checked before the usual
/// `auto` order (ffprobe uses ffmpeg's folder).
pub fn resolve(conf: &Conf, key: &str, name: &str, look: &Lookup, first: Option<&Path>) -> Result<PathBuf, String> {
    let entry = conf.get("tools", key);
    let value = entry.as_ref().map(|entry| entry.value.trim().to_string()).unwrap_or_default();
    if value.is_empty() || value.eq_ignore_ascii_case("auto") {
        return search_auto(name, look, first).ok_or_else(|| {
            format!(
                "{name} was not found ({key} = auto in {}). Link Download looked next to Link Download.exe, in the folders beside it and their bin folders, and on PATH. Put {name} in one of those places, or set {key} to its full path in {}.",
                look.conf_name, look.conf_name
            )
        });
    }
    let line = entry.as_ref().map(|entry| entry.line).unwrap_or(0);
    if value.eq_ignore_ascii_case("path") {
        return search_path(name, look)
            .ok_or_else(|| format!("{name} was not found on PATH ({key} = path, line {line} of {}).", look.conf_name));
    }
    let expanded = conf::expand_vars(&value, look.env);
    let mut path = PathBuf::from(&expanded);
    if path.is_relative() {
        path = look.exe_dir.join(path);
    }
    if path.is_dir() {
        if let Some(found) = file_in(&path, name) {
            return Ok(found);
        }
    } else if path.is_file() {
        return Ok(path);
    }
    Err(format!(
        "{key} = {value} (line {line} of {}): no such file. Tried {}.",
        look.conf_name,
        path.display()
    ))
}

pub fn resolve_all(conf: &Conf, look: &Lookup) -> Result<[PathBuf; 4], String> {
    let mut found: Vec<Option<PathBuf>> = Vec::new();
    let mut errors = Vec::new();
    for (key, name) in TOOLS {
        let first = if key == "ffprobe" {
            found.get(1).cloned().flatten().and_then(|ffmpeg| ffmpeg.parent().map(Path::to_path_buf))
        } else {
            None
        };
        match resolve(conf, key, name, look, first.as_deref()) {
            Ok(path) => found.push(Some(path)),
            Err(err) => {
                errors.push(err);
                found.push(None);
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("\n\n"));
    }
    let mut paths = found.into_iter().map(Option::unwrap);
    Ok([paths.next().unwrap(), paths.next().unwrap(), paths.next().unwrap(), paths.next().unwrap()])
}

pub fn load_bundle() -> Result<Bundle, String> {
    let dir = exe_dir()?;
    let conf = conf::load();
    let conf_name = conf.path.as_ref().map(|path| path.display().to_string()).unwrap_or_else(|| conf::FILE_NAME.into());
    let path_dirs = std::env::var_os("PATH").map(|path| std::env::split_paths(&path).collect()).unwrap_or_default();
    let look = Lookup { exe_dir: &dir, path_dirs, env: &conf::env_lookup, conf_name };
    let [ytdlp, ffmpeg, ffprobe, node] = resolve_all(&conf, &look)?;
    Ok(Bundle { dir, ytdlp, ffmpeg, ffprobe, node })
}

pub fn ffmpeg_dir(ffmpeg: &Path) -> PathBuf {
    ffmpeg.parent().unwrap_or(Path::new(".")).to_path_buf()
}

/// The folder yt-dlp runs in. `--js-runtimes node:<file name>` is resolved from it.
pub fn node_dir(node: &Path) -> PathBuf {
    ffmpeg_dir(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ld-tools-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    fn no_env(_: &str) -> Option<String> {
        None
    }

    fn look<'a>(dir: &'a Path, path_dirs: Vec<PathBuf>, env: &'a dyn Fn(&str) -> Option<String>) -> Lookup<'a> {
        Lookup { exe_dir: dir, path_dirs, env, conf_name: "configuration.conf".into() }
    }

    #[test]
    fn auto_order_exe_then_sub_then_bin_then_path() {
        let dir = scratch("order");
        let app = dir.join("app");
        let path = dir.join("pathdir");
        let conf = Conf::parse("[tools]\nffmpeg = auto\n");
        touch(&path.join("ffmpeg.exe"));
        let lk = look(&app, vec![path.clone()], &no_env);
        fs::create_dir_all(&app).unwrap();
        assert_eq!(resolve(&conf, "ffmpeg", "ffmpeg.exe", &lk, None).unwrap(), path.join("ffmpeg.exe"));
        touch(&app.join("b").join("bin").join("ffmpeg.exe"));
        assert_eq!(resolve(&conf, "ffmpeg", "ffmpeg.exe", &lk, None).unwrap(), app.join("b").join("bin").join("ffmpeg.exe"));
        touch(&app.join("Zed").join("ffmpeg.exe"));
        assert_eq!(resolve(&conf, "ffmpeg", "ffmpeg.exe", &lk, None).unwrap(), app.join("Zed").join("ffmpeg.exe"));
        touch(&app.join("alpha").join("ffmpeg.exe"));
        assert_eq!(resolve(&conf, "ffmpeg", "ffmpeg.exe", &lk, None).unwrap(), app.join("alpha").join("ffmpeg.exe"));
        touch(&app.join("ffmpeg.exe"));
        assert_eq!(resolve(&conf, "ffmpeg", "ffmpeg.exe", &lk, None).unwrap(), app.join("ffmpeg.exe"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_key_means_auto_and_path_means_only_path() {
        let dir = scratch("path");
        let path = dir.join("p");
        touch(&dir.join("node.exe"));
        touch(&path.join("node.exe"));
        let lk = look(&dir, vec![path.clone()], &no_env);
        assert_eq!(resolve(&Conf::parse(""), "node", "node.exe", &lk, None).unwrap(), dir.join("node.exe"));
        assert_eq!(resolve(&Conf::parse("[tools]\nNODE = Path\n"), "node", "node.exe", &lk, None).unwrap(), path.join("node.exe"));
        let lk = look(&dir, vec![], &no_env);
        let err = resolve(&Conf::parse("[tools]\nnode = path\n"), "node", "node.exe", &lk, None).unwrap_err();
        assert!(err.contains("node = path") && err.contains("line 2"), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn relative_paths_and_vars() {
        let dir = scratch("rel");
        touch(&dir.join("tools").join("yt.exe"));
        touch(&dir.join("home").join("n.exe"));
        let home = dir.join("home").display().to_string();
        let env = move |name: &str| (name == "USERPROFILE").then(|| home.clone());
        let lk = look(&dir, vec![], &env);
        let conf = Conf::parse("[tools]\nyt-dlp = tools/yt.exe\nnode = %USERPROFILE%/n.exe\nffmpeg = tools\n");
        assert_eq!(resolve(&conf, "yt-dlp", "yt-dlp.exe", &lk, None).unwrap(), dir.join("tools/yt.exe"));
        assert_eq!(resolve(&conf, "node", "node.exe", &lk, None).unwrap(), dir.join("home").join("n.exe"));
        // A folder works when the usual file name is in it.
        touch(&dir.join("tools").join("ffmpeg.exe"));
        assert_eq!(resolve(&conf, "ffmpeg", "ffmpeg.exe", &lk, None).unwrap(), dir.join("tools").join("ffmpeg.exe"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn explicit_missing_path_names_key_line_and_path() {
        let dir = scratch("missing");
        touch(&dir.join("ffmpeg.exe"));
        let lk = look(&dir, vec![], &no_env);
        let conf = Conf::parse("# x\n[tools]\nffmpeg = nope\\ffmpeg.exe\n");
        let err = resolve(&conf, "ffmpeg", "ffmpeg.exe", &lk, None).unwrap_err();
        assert!(err.starts_with("ffmpeg = nope\\ffmpeg.exe (line 3 of configuration.conf)"), "{err}");
        assert!(err.contains(&dir.join("nope\\ffmpeg.exe").display().to_string()), "{err}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn ffprobe_prefers_ffmpeg_folder() {
        let dir = scratch("probe");
        for name in ["yt-dlp.exe", "node.exe", "ffprobe.exe"] {
            touch(&dir.join(name));
        }
        touch(&dir.join("ff").join("bin").join("ffmpeg.exe"));
        touch(&dir.join("ff").join("bin").join("ffprobe.exe"));
        let lk = look(&dir, vec![], &no_env);
        let conf = Conf::parse("[tools]\nffmpeg = ff\\bin\\ffmpeg.exe\n".replace('\\', std::path::MAIN_SEPARATOR_STR).as_str());
        let [_, ffmpeg, ffprobe, _] = resolve_all(&conf, &lk).unwrap();
        assert_eq!(ffmpeg.parent(), ffprobe.parent());
        assert_eq!(ffprobe, dir.join("ff").join("bin").join("ffprobe.exe"));
        // Without one beside ffmpeg, the normal order is used.
        fs::remove_file(dir.join("ff").join("bin").join("ffprobe.exe")).unwrap();
        let [_, _, ffprobe, _] = resolve_all(&conf, &lk).unwrap();
        assert_eq!(ffprobe, dir.join("ffprobe.exe"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_missing_tool_is_listed() {
        let dir = scratch("none");
        let lk = look(&dir, vec![], &no_env);
        let err = resolve_all(&Conf::parse(""), &lk).unwrap_err();
        for key in ["yt-dlp = auto", "ffmpeg = auto", "ffprobe = auto", "node = auto"] {
            assert!(err.contains(key), "{err}");
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
