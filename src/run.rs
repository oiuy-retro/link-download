//! Update yt-dlp, read link information, download, then remux without re-encoding
//! unless the user turned that on.

use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::classify::{self, Hit};
use crate::commands::{self, Tags};
use crate::model::{Block, Group, Options};
use crate::net;
use crate::paths;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    Queued,
    Active,
    Waiting,
    Pinned,
    Failed,
}

#[derive(Clone, Debug)]
pub struct WorkRow {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub copied: u64,
    pub fraction: f64,
    pub convert: f64,
    pub kind: RowKind,
    pub note: String,
}

#[derive(Clone, Debug)]
pub struct Live {
    pub status: String,
    pub rows: Vec<WorkRow>,
    pub banked_bytes: u64,
    pub current_bytes: u64,
    pub ok: u32,
    pub bad: u32,
    pub total: u32,
    pub done: bool,
    pub error: String,
}

impl Live {
    pub fn new() -> Self {
        Self {
            status: String::new(),
            rows: Vec::new(),
            banked_bytes: 0,
            current_bytes: 0,
            ok: 0,
            bad: 0,
            total: 0,
            done: false,
            error: String::new(),
        }
    }
}

pub struct Shared {
    pub live: Mutex<Live>,
    pub groups: Mutex<Option<Vec<Group>>>,
    pub child: Mutex<Option<Child>>,
    pub cancel: AtomicBool,
}

impl Shared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            live: Mutex::new(Live::new()),
            groups: Mutex::new(None),
            child: Mutex::new(None),
            cancel: AtomicBool::new(false),
        })
    }
}

struct RunOut {
    code: i32,
    log: String,
    stdout: String,
    filepath: String,
}

pub fn fetch(shared: Arc<Shared>, ytdlp: PathBuf, node: PathBuf, ffmpeg_dir: PathBuf, urls: Vec<String>) {
    reset(&shared);
    let mut groups = Vec::new();
    set_status(&shared, "Updating yt-dlp…");
    match run_command(&shared, &ytdlp, &commands::update_args(&node, &ffmpeg_dir), Some(&ffmpeg_dir), false, |_| {}) {
        Ok(output) if output.code != 0 => {
            shared.live.lock().expect("live").error = "yt-dlp did not update. Using the current copy.".into();
        }
        Err(err) => {
            shared.live.lock().expect("live").error = format!("yt-dlp did not update ({err}). Using the current copy.");
        }
        _ => {}
    }
    if cancelled(&shared) {
        finish(&shared, groups, "");
        return;
    }
    for (index, url) in urls.iter().enumerate() {
        if cancelled(&shared) {
            break;
        }
        set_status(&shared, &format!("Reading {} of {}", index + 1, urls.len()));
        {
            let mut live = shared.live.lock().expect("live");
            live.total = urls.len() as u32;
            live.ok = index as u32;
        }
        match run_command(&shared, &ytdlp, &commands::dump_args(&node, &ffmpeg_dir, url), Some(&ffmpeg_dir), true, |_| {}) {
            Ok(output) => groups.push(group_from_dump(url, &output)),
            Err(err) => groups.push(error_group(url, &err)),
        }
        shared.live.lock().expect("live").ok = (index as u32) + 1;
    }
    finish(&shared, groups, "");
}

pub fn download(shared: Arc<Shared>, ytdlp: PathBuf, node: PathBuf, ffmpeg: PathBuf, ffmpeg_dir: PathBuf, opts: Options, mut groups: Vec<Group>) {
    reset(&shared);
    journal(&format!("download into {}", opts.location));
    let jobs = job_count(&groups);
    let (rows, already_bad) = rows_for(&groups);
    {
        let mut live = shared.live.lock().expect("live");
        live.total = jobs;
        live.bad = already_bad;
        live.rows = rows;
    }
    let mut rng = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1);
    let mut seen = 0u32;
    for group in groups.iter_mut() {
        let source_url = group.source_url.clone();
        let dirs: Vec<PathBuf> = (0..group.items.len())
            .map(|index| paths::output_dir(Path::new(&opts.location), group, index))
            .collect();
        let track_total = if group.is_playlist {
            Some(group.items.iter().filter(|item| item.blocked.is_none()).count() as u32)
        } else {
            None
        };
        let album = group.album.clone();
        let playlist_thumb = group.playlist_thumbnail.clone();
        if group.is_playlist {
            if let Some(dir) = dirs.first() {
                let _ = fs::create_dir_all(dir);
                let _ = write_folder_cover(&shared, &ffmpeg, group, dir);
            }
        }
        for (index, item) in group.items.iter_mut().enumerate() {
            seen += 1;
            if cancelled(&shared) {
                finish(&shared, groups, "");
                return;
            }
            let row_id = row_id_url(&source_url, index);
            if item.blocked.is_some() || (item.download_url.is_empty() && item.playlist_item.is_none()) {
                continue;
            }
            if let Some(path) = item.temp_path.clone() {
                if Path::new(&path).is_file() {
                    set_fraction(&shared, row_id, 1.0);
                    store_item(&shared, &ffmpeg, &opts, row_id, Path::new(&path), item, &dirs[index], track_total, &album, &playlist_thumb);
                    continue;
                }
            }
            set_kind(&shared, row_id, RowKind::Active, "");
            set_status(&shared, &item.title);
            let mut fetched_now = false;
            let dir = &dirs[index];
            if let Err(err) = fs::create_dir_all(dir) {
                fail(&shared, row_id, &err.to_string());
                journal(&format!("could not create {}: {err}", dir.display()));
                continue;
            }
            let stem = paths::download_stem(dir, &item.title);
            let template = dir.join(format!("{stem}.%(ext)s")).display().to_string();
            let mut saved = String::new();
            let mut outcome = DownloadOutcome::Retry;
            while outcome == DownloadOutcome::Retry {
                outcome = DownloadOutcome::Failed;
                if cancelled(&shared) {
                    finish(&shared, groups, "");
                    return;
                }
                let args = commands::download_args(&opts, &item.download_url, item.playlist_item, &template, &node, &ffmpeg_dir);
                let shared_progress = shared.clone();
                let result = run_command(&shared, &ytdlp, &args, Some(&ffmpeg_dir), false, move |line| {
                    if let Some(progress) = commands::parse_progress(line) {
                        let mut live = shared_progress.live.lock().expect("live");
                        if progress.downloaded > 0 {
                            live.current_bytes = progress.downloaded;
                        }
                        if let Some(row) = live.rows.iter_mut().find(|row| row.id == row_id) {
                            if progress.downloaded > 0 {
                                row.copied = progress.downloaded;
                            }
                            if let Some(total) = progress.total {
                                row.size = total;
                            }
                            if let Some(fraction) = progress.fraction {
                                row.fraction = fraction;
                            }
                        }
                    }
                });
                match result {
                    Ok(output) if output.code == 0 => {
                        saved = output.filepath;
                        outcome = DownloadOutcome::Ok;
                    }
                    Ok(output) => {
                        let hit = classify::classify(&format!("{}\n{}", output.log, output.stdout));
                        match hit {
                            Hit::RateLimit => {
                                outcome = DownloadOutcome::Retry;
                                if !countdown(&shared, row_id, 30, "Waiting") {
                                    finish(&shared, groups, "");
                                    return;
                                }
                            }
                            Hit::Age => {
                                item.blocked = Some(Block::Age);
                                pin(&shared, row_id, "Age restricted", true);
                            }
                            Hit::Missing => {
                                item.blocked = Some(Block::Missing);
                                pin(&shared, row_id, "Unavailable", true);
                            }
                            Hit::Other(message) => fail(&shared, row_id, &message),
                        }
                    }
                    Err(err) => fail(&shared, row_id, &err),
                }
            }
            if outcome == DownloadOutcome::Ok {
                if let Some(path) = resolve_download(dir, &stem, &saved) {
                    item.temp_path = Some(path.display().to_string());
                    set_fraction(&shared, row_id, 1.0);
                    store_item(&shared, &ffmpeg, &opts, row_id, &path, item, dir, track_total, &album, &playlist_thumb);
                    fetched_now = true;
                } else {
                    fail(&shared, row_id, "The download finished without a file.");
                }
            }
            if fetched_now && seen < jobs {
                let seconds = commands::wait_seconds(opts.min_wait, opts.max_wait, &mut rng);
                if seconds > 0 && !countdown(&shared, next_queued(&shared), seconds, "Waiting") {
                    finish(&shared, groups, "");
                    return;
                }
            }
        }
    }
    finish(&shared, groups, "");
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DownloadOutcome {
    Retry,
    Ok,
    Failed,
}

fn store_item(shared: &Arc<Shared>, ffmpeg: &Path, opts: &Options, row_id: u64, source: &Path, item: &crate::model::Item, dir: &Path, track_total: Option<u32>, album: &str, playlist_thumb: &str) {
    if cancelled(shared) {
        return;
    }
    let title = item.title.clone();
    let tag_title = item.meta_title.clone();
    let artists = item.artists.clone();
    let track = item.track_number;
    let duration = item.duration;
    let own_thumb = if item.thumbnail.is_empty() { playlist_thumb.to_string() } else { item.thumbnail.clone() };
    set_kind(shared, row_id, RowKind::Active, "");
    set_status(shared, &title);
    set_convert(shared, row_id, 0.02);
    if let Err(err) = fs::create_dir_all(dir) {
        fail(shared, row_id, &err.to_string());
        return;
    }
    let tags = Tags {
        title: tag_title,
        artists,
        album: album.to_string(),
        track,
        track_total,
    };
    let stem = format!("part-{row_id}");
    let tagged = dir.join(format!("{stem}.{}", opts.container.extension()));
    let covered = dir.join(format!("{stem}.covered.{}", opts.container.extension()));
    let media_args = if opts.reencode {
        commands::transcode_args(source, &tagged, &tags, opts)
    } else {
        commands::remux_args(source, &tagged, &tags, opts.audio_only, opts.container, opts.sponsorblock && !opts.audio_only, opts.strip_genre)
    };
    let shared_progress = Arc::clone(shared);
    let encoded = run_command(shared, ffmpeg, &with_progress(media_args), Some(dir), false, move |line| {
        if let Some(micros) = commands::parse_out_time_ms(line) {
            if duration > 0.0 {
                let ratio = (micros as f64 / (duration * 1_000_000.0)).clamp(0.0, 1.0);
                set_convert(&shared_progress, row_id, 0.05 + ratio * 0.7);
            }
        }
    });
    match encoded {
        Err(err) => {
            let _ = fs::remove_file(&tagged);
            fail(shared, row_id, &err);
            return;
        }
        Ok(output) if output.code != 0 || file_len(&tagged) == 0 => {
            let _ = fs::remove_file(&tagged);
            fail(shared, row_id, &tail(&output.log));
            return;
        }
        Ok(_) => {}
    }
    set_convert(shared, row_id, 0.8);
    let thumb = thumbnail_jpeg(shared, ffmpeg, dir, &own_thumb, row_id);
    let mut ready = tagged.clone();
    if let Some(ref thumb) = thumb {
        let cover = commands::cover_args(&tagged, thumb, &covered, opts.container, opts.audio_only);
        let covered_ok = run_command(shared, ffmpeg, &cover, Some(dir), false, |_| {}).map(|output| output.code == 0 && file_len(&covered) > 0).unwrap_or(false);
        if covered_ok && covered.is_file() {
            ready = covered;
        } else {
            let _ = fs::remove_file(&covered);
            journal(&format!("saved without a cover: {title}"));
        }
    }
    if let Some(thumb) = &thumb {
        let _ = fs::remove_file(thumb);
    }
    let destination = final_path(dir, &title, opts.container.extension(), source);
    if destination.as_path() == source {
        let _ = fs::remove_file(source);
    }
    if let Err(err) = move_file(&ready, &destination) {
        fail(shared, row_id, &err);
        journal(&format!("kept download {}: {err}", source.display()));
        return;
    }
    if source != destination.as_path() {
        let _ = fs::remove_file(source);
    }
    if tagged.as_path() != destination.as_path() {
        let _ = fs::remove_file(&tagged);
    }
    if ready.as_path() != destination.as_path() && ready != tagged {
        let _ = fs::remove_file(&ready);
    }
    journal(&format!("saved {}", destination.display()));
    set_convert(shared, row_id, 1.0);
    let size = fs::metadata(&destination).map(|meta| meta.len()).unwrap_or(0);
    remove_done(shared, row_id, size);
}

fn write_folder_cover(shared: &Shared, ffmpeg: &Path, group: &Group, dir: &Path) -> Result<(), String> {
    if group.playlist_thumbnail.is_empty() {
        return Ok(());
    }
    let bytes = net::get(&group.playlist_thumbnail)?;
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let jpeg = dir.join("cover.jpg");
    write_jpeg(ffmpeg, shared, &bytes, &jpeg)?;
    Ok(())
}

fn thumbnail_jpeg(shared: &Shared, ffmpeg: &Path, dir: &Path, url: &str, id: u64) -> Option<PathBuf> {
    if url.is_empty() {
        return None;
    }
    let bytes = net::get(&url).ok()?;
    let jpeg = dir.join(format!(".cover-{id}.jpg"));
    write_jpeg(ffmpeg, shared, &bytes, &jpeg).ok()?;
    Some(jpeg)
}

fn write_jpeg(ffmpeg: &Path, shared: &Shared, bytes: &[u8], dest: &Path) -> Result<(), String> {
    if bytes.starts_with(&[0xFF, 0xD8]) {
        fs::write(dest, bytes).map_err(|err| err.to_string())?;
        return Ok(());
    }
    let raw = dest.with_extension("bin");
    fs::write(&raw, bytes).map_err(|err| err.to_string())?;
    let output = run_command(shared, ffmpeg, &commands::jpeg_args(&raw, dest), None, false, |_| {})?;
    let _ = fs::remove_file(&raw);
    if output.code != 0 || !dest.is_file() {
        return Err(tail(&output.log));
    }
    Ok(())
}

fn with_progress(mut args: Vec<String>) -> Vec<String> {
    let output = args.pop().unwrap_or_default();
    args.push("-progress".into());
    args.push("pipe:1".into());
    args.push("-nostats".into());
    args.push(output);
    args
}

/// `--ignore-errors` still exits 1 when a playlist entry fails. The JSON on
/// stdout is the Names table; only fall back to the log when it is unusable.
fn group_from_dump(url: &str, output: &RunOut) -> Group {
    if let Ok(group) = crate::parse::parse_info(&output.stdout, url) {
        if !group.items.is_empty() {
            return group;
        }
        if output.code == 0 {
            return error_group(url, "That link has no videos.");
        }
    }
    error_group(url, &best_error(&output.log, ""))
}

fn error_group(url: &str, message: &str) -> Group {
    let mut group = crate::parse::parse_info("{}", url).unwrap_or_else(|_| Group {
        source_url: url.to_string(),
        is_playlist: false,
        playlist_title: String::new(),
        album: String::new(),
        original_album: String::new(),
        playlist_thumbnail: String::new(),
        error: String::new(),
        items: Vec::new(),
    });
    group.source_url = url.to_string();
    group.error = message.to_string();
    group.items.clear();
    group
}

fn best_error(log: &str, stdout: &str) -> String {
    match classify::classify(&format!("{log}\n{stdout}")) {
        Hit::Other(message) => message,
        Hit::Age => "Age restricted".into(),
        Hit::Missing => "Unavailable".into(),
        Hit::RateLimit => "Too many requests".into(),
    }
}

fn rows_for(groups: &[Group]) -> (Vec<WorkRow>, u32) {
    let mut rows = Vec::new();
    let mut bad = 0u32;
    for group in groups {
        for (index, item) in group.items.iter().enumerate() {
            let blocked = item.blocked.is_some();
            if blocked {
                bad += 1;
            }
            rows.push(WorkRow {
                id: row_id_for(group, index),
                name: item.title.clone(),
                size: 0,
                copied: 0,
                fraction: 0.0,
                convert: 0.0,
                kind: if blocked { RowKind::Pinned } else { RowKind::Queued },
                note: if blocked { "Unavailable".into() } else { String::new() },
            });
        }
    }
    rows.sort_by_key(|row| if row.kind == RowKind::Pinned { 0 } else { 1 });
    (rows, bad)
}

fn row_id_for(group: &Group, index: usize) -> u64 {
    row_id_url(&group.source_url, index)
}

fn row_id_url(source_url: &str, index: usize) -> u64 {
    // Stable for one run: hash the source and index without pulling in another crate.
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for byte in source_url.bytes() {
        value ^= byte as u64;
        value = value.wrapping_mul(0x100_0000_01b3);
    }
    value ^ (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

fn resolve_download(temp: &Path, stem: &str, printed: &str) -> Option<PathBuf> {
    let printed = printed.trim().trim_matches('"');
    if !printed.is_empty() {
        let path = PathBuf::from(printed);
        if path.is_file() {
            return Some(path);
        }
    }
    let mut best: Option<(u64, PathBuf)> = None;
    let entries = fs::read_dir(temp).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(stem) || name.ends_with(".part") {
            continue;
        }
        let len = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        if best.as_ref().map(|(size, _)| len >= *size).unwrap_or(true) {
            best = Some((len, entry.path()));
        }
    }
    best.map(|(_, path)| path)
}

fn run_command<F>(shared: &Shared, exe: &Path, args: &[String], cwd: Option<&Path>, keep_stdout: bool, on_line: F) -> Result<RunOut, String>
where
    F: FnMut(&str) + Send + 'static,
{
    if cancelled(shared) {
        return Err("Stopped.".into());
    }
    let mut command = Command::new(exe);
    command
        .args(args)
        .env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|err| format!("Could not start {}: {err}", exe.display()))?;
    let stdout = child.stdout.take().ok_or_else(|| "No output pipe.".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "No error pipe.".to_string())?;
    *shared.child.lock().expect("child") = Some(child);
    let log = Arc::new(Mutex::new(String::new()));
    let stdout_text = Arc::new(Mutex::new(String::new()));
    let filepath = Arc::new(Mutex::new(String::new()));
    let callback = Arc::new(Mutex::new(on_line));
    let log_err = log.clone();
    let callback_err = callback.clone();
    let stderr_thread = thread::spawn(move || {
        read_lines(stderr, move |line| {
            callback_err.lock().expect("progress")(&line);
            push_log(&log_err, &line);
        });
    });
    let log_out = log.clone();
    let stdout_keep = stdout_text.clone();
    let filepath_out = filepath.clone();
    let callback_out = callback.clone();
    let stdout_thread = thread::spawn(move || {
        read_lines(stdout, move |line| {
            if keep_stdout {
                let mut text = stdout_keep.lock().expect("stdout");
                if text.len() < 80 * 1024 * 1024 {
                    text.push_str(&line);
                    text.push('\n');
                }
                return;
            }
            if let Some(path) = commands::parse_after_move(&line) {
                *filepath_out.lock().expect("path") = path;
            }
            callback_out.lock().expect("progress")(&line);
            push_log(&log_out, &line);
        });
    });
    let status = shared.child.lock().expect("child").as_mut().and_then(|child| child.wait().ok());
    *shared.child.lock().expect("child") = None;
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();
    if cancelled(shared) {
        return Err("Stopped.".into());
    }
    let code = status.and_then(|status| status.code()).unwrap_or(1);
    let log_text = log.lock().expect("log").clone();
    let stdout = stdout_text.lock().expect("stdout").clone();
    let found = filepath.lock().expect("path").clone();
    journal_command(exe, args, code, &log_text);
    Ok(RunOut { code, log: log_text, stdout, filepath: found })
}

fn journal(text: &str) {
    use std::io::Write;
    let Some(base) = std::env::var_os("APPDATA") else {
        return;
    };
    let dir = PathBuf::from(base).join("Link Download");
    let _ = fs::create_dir_all(&dir);
    let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(dir.join("link-download.log")) else {
        return;
    };
    let _ = writeln!(file, "{text}");
}

fn journal_command(exe: &Path, args: &[String], code: i32, log: &str) {
    let tail: Vec<&str> = log.lines().rev().take(40).collect();
    let mut lines: Vec<&str> = tail.into_iter().rev().collect();
    lines.retain(|line| !line.starts_with("out_time") && !line.starts_with("progress=") && !line.starts_with("bitrate="));
    journal(&format!("\n---\n{} {}\nexit {code}\n{}", exe.display(), args.join(" "), lines.join("\n")));
}

fn file_len(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn final_path(dir: &Path, title: &str, extension: &str, source: &Path) -> PathBuf {
    let wanted = dir.join(format!("{}.{}", paths::sanitize(title), extension));
    if wanted.as_path() == source || !wanted.exists() {
        wanted
    } else {
        paths::unique_path(dir, title, extension)
    }
}

fn read_lines<R, F>(reader: R, mut on_line: F)
where
    R: Read,
    F: FnMut(String),
{
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let read = reader.read_until(b'\n', &mut buf).unwrap_or(0);
        if read == 0 {
            break;
        }
        if buf.last() == Some(&b'\n') {
            buf.pop();
        }
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }
        on_line(String::from_utf8_lossy(&buf).into_owned());
    }
}

fn push_log(log: &Mutex<String>, line: &str) {
    let mut text = log.lock().expect("log");
    text.push_str(line);
    text.push('\n');
    if text.len() > 64_000 {
        let cut = text.len() - 48_000;
        *text = text[cut..].to_string();
    }
}

fn countdown(shared: &Shared, row_id: u64, seconds: u32, label: &str) -> bool {
    if row_id != 0 {
        set_kind(shared, row_id, RowKind::Waiting, "");
    }
    for left in (1..=seconds).rev() {
        if cancelled(shared) {
            return false;
        }
        let text = format!("{label} {left} seconds");
        set_status(shared, &text);
        if row_id != 0 {
            set_note(shared, row_id, &text);
        }
        thread::sleep(Duration::from_secs(1));
    }
    if row_id != 0 {
        set_kind(shared, row_id, RowKind::Active, "");
    }
    true
}

fn next_queued(shared: &Shared) -> u64 {
    shared
        .live
        .lock()
        .expect("live")
        .rows
        .iter()
        .find(|row| row.kind == RowKind::Queued)
        .map(|row| row.id)
        .unwrap_or(0)
}

fn reset(shared: &Shared) {
    let mut live = shared.live.lock().expect("live");
    *live = Live::new();
    *shared.groups.lock().expect("groups") = None;
}

fn finish(shared: &Shared, groups: Vec<Group>, error: &str) {
    *shared.groups.lock().expect("groups") = Some(groups);
    let mut live = shared.live.lock().expect("live");
    live.current_bytes = 0;
    if !error.is_empty() {
        live.error = error.to_string();
    }
    if live.ok == 0 && live.bad > 0 && !live.status.is_empty() && live.status != "Done." {
        // Keep the reason the download failed. "Done." hides it.
    } else if error.is_empty() {
        live.status = "Done.".into();
    }
    live.done = true;
}

fn set_status(shared: &Shared, text: &str) {
    shared.live.lock().expect("live").status = text.to_string();
}

fn set_kind(shared: &Shared, id: u64, kind: RowKind, note: &str) {
    if let Some(row) = shared.live.lock().expect("live").rows.iter_mut().find(|row| row.id == id) {
        row.kind = kind;
        if !note.is_empty() {
            row.note = note.to_string();
        }
        if kind == RowKind::Active {
            row.note.clear();
        }
    }
}

fn set_note(shared: &Shared, id: u64, note: &str) {
    if let Some(row) = shared.live.lock().expect("live").rows.iter_mut().find(|row| row.id == id) {
        row.note = note.to_string();
    }
}

fn set_fraction(shared: &Shared, id: u64, fraction: f64) {
    if let Some(row) = shared.live.lock().expect("live").rows.iter_mut().find(|row| row.id == id) {
        row.fraction = fraction.clamp(0.0, 1.0);
    }
}

fn set_convert(shared: &Shared, id: u64, fraction: f64) {
    if let Some(row) = shared.live.lock().expect("live").rows.iter_mut().find(|row| row.id == id) {
        row.convert = fraction.clamp(0.0, 1.0);
    }
}

fn pin(shared: &Shared, id: u64, note: &str, missing_or_age: bool) {
    let mut live = shared.live.lock().expect("live");
    if let Some(index) = live.rows.iter().position(|row| row.id == id) {
        let mut row = live.rows.remove(index);
        row.kind = if missing_or_age { RowKind::Pinned } else { RowKind::Failed };
        row.note = note.to_string();
        row.fraction = 0.0;
        let insert = live.rows.iter().position(|row| row.kind != RowKind::Pinned).unwrap_or(live.rows.len());
        live.rows.insert(insert, row);
        live.bad += 1;
    }
}

fn fail(shared: &Shared, id: u64, note: &str) {
    let mut live = shared.live.lock().expect("live");
    if let Some(row) = live.rows.iter_mut().find(|row| row.id == id) {
        row.kind = RowKind::Failed;
        row.note = note.to_string();
        live.bad += 1;
        live.status = note.to_string();
    }
}

fn remove_done(shared: &Shared, id: u64, size: u64) {
    let mut live = shared.live.lock().expect("live");
    if let Some(index) = live.rows.iter().position(|row| row.id == id) {
        live.rows.remove(index);
        live.banked_bytes = live.banked_bytes.saturating_add(size);
        live.current_bytes = 0;
        live.ok += 1;
    }
}

fn cancelled(shared: &Shared) -> bool {
    shared.cancel.load(Ordering::Relaxed)
}

fn job_count(groups: &[Group]) -> u32 {
    groups.iter().map(|group| group.items.len() as u32).sum()
}

fn move_file(from: &Path, to: &Path) -> Result<(), String> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to).map_err(|err| err.to_string())?;
    let _ = fs::remove_file(from);
    Ok(())
}

fn tail(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("out_time") && !line.starts_with("progress=") && !line.starts_with("bitrate="))
        .next_back()
        .unwrap_or("ffmpeg failed.")
        .chars()
        .take(180)
        .collect()
}

pub fn kill_child(shared: &Shared) {
    shared.cancel.store(true, Ordering::Relaxed);
    if let Some(child) = shared.child.lock().expect("child").as_mut() {
        let _ = child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines_of(bytes: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        read_lines(bytes, |line| lines.push(line));
        lines
    }

    #[test]
    fn saved_path_is_read_from_its_own_line() {
        // What yt-dlp writes with `--newline` and the saved-path print.
        let out = b"download:downloading|1024|185292|NA|NA\ndownload:finished|185292|185292|NA|2659031.1\nafter_move:C:\\Music\\Me at the zoo - (test).mkv\r\n";
        let lines = lines_of(out);
        assert_eq!(lines.len(), 3);
        assert!(commands::parse_progress(&lines[1]).is_some());
        let found: Vec<String> = lines.iter().filter_map(|line| commands::parse_after_move(line)).collect();
        assert_eq!(found, vec!["C:\\Music\\Me at the zoo - (test).mkv".to_string()]);
    }

    #[test]
    fn without_newline_the_saved_path_shares_a_line() {
        // What yt-dlp writes without `--newline`: one line, updates split by `\r` and an erase code.
        let out = b"\r\x1b[Kdownload:downloading|1024|185292|NA|NA\r\x1b[Kdownload:finished|185292|185292|NA|NA\r\x1b[Kafter_move:C:\\a.mkv\n";
        let lines = lines_of(out);
        assert_eq!(lines.len(), 1);
        assert_eq!(commands::parse_after_move(&lines[0]), None);
    }

    fn out(code: i32, stdout: &str, log: &str) -> RunOut {
        RunOut {
            code,
            log: log.into(),
            stdout: stdout.into(),
            filepath: String::new(),
        }
    }

    #[test]
    fn dump_keeps_titles_when_yt_dlp_exits_one() {
        let json = r#"{"_type":"playlist","title":"OST","entries":[{"id":"a","title":"Song","url":"https://www.youtube.com/watch?v=a"}]}"#;
        let group = group_from_dump(
            "https://www.youtube.com/playlist?list=PL",
            &out(1, json, "ERROR: [youtube] b: Video unavailable"),
        );
        assert_eq!(group.items.len(), 1);
        assert_eq!(group.items[0].title, "Song");
        assert!(group.error.is_empty());
    }

    #[test]
    fn dump_error_uses_stderr_not_json() {
        let group = group_from_dump(
            "https://example.com/a",
            &out(
                1,
                "{not json",
                "ERROR: [youtube] a: This video is unavailable\n{\"__yt_dlp_client\":\"tv_samsung\"}",
            ),
        );
        assert_eq!(group.error, "Unavailable");
        assert!(group.items.is_empty());
        let json_only = group_from_dump(
            "https://example.com/a",
            &out(1, "{not json", r#"_yt_dlp_client": "tv_samsung"}. {"ext": "srv2", "url": "https://www.youtube.com/api/timedtext?v=x"}"#),
        );
        assert!(!json_only.error.contains("timedtext"), "{}", json_only.error);
        assert!(!json_only.error.contains("yt_dlp_client"), "{}", json_only.error);
    }
}
