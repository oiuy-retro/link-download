//! Tools live in the same folder as Link Download.exe.

use std::path::{Path, PathBuf};

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

/// ffmpeg, ffprobe, node, and yt-dlp are the copies sitting next to this program.
pub fn load_bundle() -> Result<Bundle, String> {
    let dir = exe_dir()?;
    let ytdlp = dir.join("yt-dlp.exe");
    let ffmpeg = dir.join("ffmpeg.exe");
    let ffprobe = dir.join("ffprobe.exe");
    let node = dir.join("node.exe");
    let mut missing = Vec::new();
    for (name, path) in [("yt-dlp.exe", &ytdlp), ("ffmpeg.exe", &ffmpeg), ("ffprobe.exe", &ffprobe), ("node.exe", &node)] {
        if !path.is_file() {
            missing.push(name);
        }
    }
    if !missing.is_empty() {
        return Err(format!(
            "Put these files in the same folder as Link Download.exe: {}.",
            missing.join(", ")
        ));
    }
    Ok(Bundle { dir, ytdlp, ffmpeg, ffprobe, node })
}

pub fn ffmpeg_dir(ffmpeg: &Path) -> PathBuf {
    ffmpeg.parent().unwrap_or(Path::new(".")).to_path_buf()
}
