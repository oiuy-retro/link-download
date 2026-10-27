//! The hidden folder unfinished downloads are kept in.
//!
//! yt-dlp downloads into `<music folder>\.link-download-partial\<process id>\item-<n>`.
//! That folder is on the same drive as your music, so saving the finished song is
//! a quick rename, and nothing goes to the C: drive. The `.link-download-partial`
//! folder is marked hidden, so the .part, .ytdl and .f137 pieces of an unfinished
//! download never show up next to your music.
//!
//! Each run of the app only ever deletes its own `<process id>` folder. A folder
//! left by an earlier run that crashed is removed only when no program with that
//! process id is running any more. The hidden folder itself is removed when it is
//! empty. Nothing else is ever deleted.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::paths::PARTIAL_DIR;

/// Every run folder this app made, so they can be removed when it closes.
static MADE: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// This app's own folder inside the hidden folder of `location`.
pub fn run_dir(location: &Path) -> PathBuf {
    location.join(PARTIAL_DIR).join(std::process::id().to_string())
}

/// The folder yt-dlp downloads one item into.
pub fn item_dir(location: &Path, row_id: u64) -> PathBuf {
    run_dir(location).join(format!("item-{row_id}"))
}

/// Makes the hidden folder and this app's folder in it, and removes folders
/// left by earlier runs that are no longer running.
pub fn prepare(location: &Path) -> std::io::Result<PathBuf> {
    prepare_in(&mut MADE.lock().expect("partial"), location)
}

fn prepare_in(made: &mut Vec<PathBuf>, location: &Path) -> std::io::Result<PathBuf> {
    let parent = location.join(PARTIAL_DIR);
    fs::create_dir_all(&parent)?;
    hide(&parent);
    remove_stale(&parent);
    let run = run_dir(location);
    fs::create_dir_all(&run)?;
    if !made.contains(&run) {
        made.push(run.clone());
    }
    Ok(run)
}

/// After a run or a Cancel: removes everything in this app's folders except the
/// item folders holding a finished download that still waits to be converted
/// (`keep`; pressing Download again uses it). Empty folders are then removed,
/// the hidden folder too if nothing else is in it.
pub fn clean_up(keep: &[PathBuf]) {
    clean_up_in(&mut MADE.lock().expect("partial"), keep);
}

fn clean_up_in(made: &mut Vec<PathBuf>, keep: &[PathBuf]) {
    made.retain(|run| {
        if let Ok(entries) = fs::read_dir(run) {
            for entry in entries.flatten() {
                let path = entry.path();
                if keep.iter().any(|file| file.starts_with(&path)) {
                    continue;
                }
                if path.is_dir() {
                    let _ = fs::remove_dir_all(&path);
                } else {
                    let _ = fs::remove_file(&path);
                }
            }
        }
        // remove_dir only succeeds on an empty folder.
        let gone = fs::remove_dir(run).is_ok() || !run.exists();
        if let Some(parent) = run.parent() {
            let _ = fs::remove_dir(parent);
        }
        !gone
    });
}

/// When the app closes: removes this app's folders completely.
pub fn discard_all() {
    discard_all_in(&mut MADE.lock().expect("partial"));
}

fn discard_all_in(made: &mut Vec<PathBuf>) {
    for run in made.drain(..) {
        let _ = fs::remove_dir_all(&run);
        if let Some(parent) = run.parent() {
            let _ = fs::remove_dir(parent);
        }
    }
}

/// Removes `<process id>` folders whose program is no longer running. Folders
/// with other names, this app's own folder, and folders of programs that are
/// still running (another copy of the app) are left alone. If Windows has given
/// a crashed run's number to a new program, the folder is kept until a later
/// run, which errs on the safe side.
fn remove_stale(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    let own = std::process::id();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        if pid == own || name != pid.to_string() || !entry.path().is_dir() || process_running(pid) {
            continue;
        }
        let _ = fs::remove_dir_all(entry.path());
    }
}

#[cfg(windows)]
fn hide(path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES, INVALID_FILE_ATTRIBUTES};
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let now = GetFileAttributesW(PCWSTR(wide.as_ptr()));
        if now == INVALID_FILE_ATTRIBUTES || now & FILE_ATTRIBUTE_HIDDEN.0 != 0 {
            return;
        }
        let _ = SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_FLAGS_AND_ATTRIBUTES(now | FILE_ATTRIBUTE_HIDDEN.0));
    }
}

#[cfg(not(windows))]
fn hide(_path: &Path) {}

#[cfg(windows)]
fn process_running(pid: u32) -> bool {
    use windows::Win32::Foundation::{CloseHandle, ERROR_INVALID_PARAMETER, STILL_ACTIVE};
    use windows::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => {
                let mut code = 0u32;
                let read = GetExitCodeProcess(handle, &mut code).is_ok();
                let _ = CloseHandle(handle);
                // If the state can't be read, treat it as running and keep the folder.
                !read || code == STILL_ACTIVE.0 as u32
            }
            // "Invalid parameter" means there is no such process. Any other error
            // (for example access denied) means it exists, so keep its folder.
            Err(err) => err.code() != ERROR_INVALID_PARAMETER.to_hresult(),
        }
    }
}

#[cfg(not(windows))]
fn process_running(_pid: u32) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ld-partial-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(windows)]
    #[test]
    fn the_partial_folder_is_hidden() {
        use std::os::windows::fs::MetadataExt;
        let mut made = Vec::new();
        let music = scratch("hidden");
        let run = prepare_in(&mut made, &music).unwrap();
        assert_eq!(run, music.join(PARTIAL_DIR).join(std::process::id().to_string()));
        let attributes = fs::metadata(music.join(PARTIAL_DIR)).unwrap().file_attributes();
        assert_ne!(attributes & 0x2, 0, "FILE_ATTRIBUTE_HIDDEN");
        discard_all_in(&mut made);
        let _ = fs::remove_dir_all(&music);
    }

    #[cfg(windows)]
    #[test]
    fn only_folders_of_programs_that_ended_are_removed() {
        let mut made = Vec::new();
        let music = scratch("stale");
        let parent = music.join(PARTIAL_DIR);
        // A process that has ended: start one, wait for it, use its number.
        let mut child = std::process::Command::new("cmd.exe").args(["/c", "exit"]).spawn().unwrap();
        let dead = child.id();
        child.wait().unwrap();
        // A process that is still running.
        let mut alive = std::process::Command::new("cmd.exe").args(["/c", "ping -n 30 127.0.0.1 >nul"]).spawn().unwrap();
        for name in [dead.to_string(), alive.id().to_string(), "notes".to_string(), format!("0{dead}")] {
            fs::create_dir_all(parent.join(&name).join("item-1")).unwrap();
            fs::write(parent.join(&name).join("item-1").join("Song.mp4.part"), b"half").unwrap();
        }
        fs::write(parent.join("readme.txt"), b"not ours").unwrap();
        let run = prepare_in(&mut made, &music).unwrap();
        assert!(!parent.join(dead.to_string()).exists(), "the ended run's folder is removed");
        assert!(parent.join(alive.id().to_string()).exists(), "a running program's folder stays");
        assert!(parent.join("notes").exists(), "other names stay");
        assert!(parent.join(format!("0{dead}")).exists(), "only exact process numbers count");
        assert!(parent.join("readme.txt").exists());
        assert!(run.is_dir());
        let _ = alive.kill();
        let _ = alive.wait();
        discard_all_in(&mut made);
        let _ = fs::remove_dir_all(&music);
    }

    #[test]
    fn clean_up_removes_only_this_runs_folder_and_an_empty_hidden_folder() {
        let mut made = Vec::new();
        let music = scratch("cleanup");
        fs::write(music.join("Song.mp4"), b"music").unwrap();
        let other = music.join(PARTIAL_DIR).join("notes");
        fs::create_dir_all(&other).unwrap();
        prepare_in(&mut made, &music).unwrap();
        let waiting = item_dir(&music, 1).join("Song.mkv");
        let unfinished = item_dir(&music, 2).join("Other.f137.mp4.part");
        for file in [&waiting, &unfinished] {
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"x").unwrap();
        }
        clean_up_in(&mut made, &[waiting.clone()]);
        assert!(waiting.is_file(), "a finished download waiting to be converted stays");
        assert!(!item_dir(&music, 2).exists());
        clean_up_in(&mut made, &[]);
        assert!(!run_dir(&music).exists());
        assert!(other.exists(), "something else in the hidden folder stays, and so does the folder");
        fs::remove_dir(&other).unwrap();
        prepare_in(&mut made, &music).unwrap();
        clean_up_in(&mut made, &[]);
        assert!(!music.join(PARTIAL_DIR).exists(), "the empty hidden folder is removed");
        assert_eq!(fs::read(music.join("Song.mp4")).unwrap(), b"music");
        let _ = fs::remove_dir_all(&music);
    }
}
