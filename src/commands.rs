//! yt-dlp and ffmpeg argument lists. Copy is the normal path.

use std::path::Path;

use crate::model::{AudioCodec, AudioQuality, Container, Options, Resolution, VideoCodec};

/// `download:` is yt-dlp's progress *type*, not part of the printed line.
/// The template itself must start with `download:` so `parse_progress` can find it.
pub const PROGRESS_TEMPLATE: &str =
    "download:download:%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s";

/// Printed once per video before the download starts. For a video with separate
/// video and audio parts, yt-dlp gives the size of both parts together here.
pub const EXPECTED_PRINT: &str = "before_dl:expected_bytes:%(filesize,filesize_approx)s";

/// yt-dlp reads the first `after_move:` as when to print and leaves it out of the
/// output, so it is written twice to keep the prefix `parse_after_move` looks for.
pub const SAVED_PATH_PRINT: &str = "after_move:after_move:%(filepath)s";

pub const SPONSORBLOCK_REMOVE: &str = "sponsor,selfpromo,interaction";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tags {
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub track: Option<u32>,
    pub track_total: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub speed: Option<f64>,
    /// 0 to 1 when the line gives a percent or a byte count.
    pub fraction: Option<f64>,
    /// The current part is complete.
    pub finished: bool,
}

/// Progress for one video across all of its parts, for one download attempt.
/// yt-dlp fetches the video and audio of a video as separate parts, and each
/// part counts from 0 again. This keeps a running total so the bar only moves forward.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemProgress {
    /// Bytes of the parts that have finished.
    banked: u64,
    downloaded: u64,
    part_total: Option<u64>,
    /// Size of all parts together, when yt-dlp knows it.
    expected: Option<u64>,
    fraction: Option<f64>,
}

impl ItemProgress {
    pub fn expect(&mut self, bytes: u64) {
        if bytes > 0 {
            self.expected = Some(bytes);
        }
    }

    pub fn update(&mut self, progress: &Progress) {
        if progress.downloaded == 0 && progress.total.is_none() {
            // A line with only a percent. It is only right while there is one part.
            if self.banked == 0 && self.expected.is_none() {
                if let Some(fraction) = progress.fraction {
                    self.raise(fraction);
                }
            }
            return;
        }
        if progress.downloaded < self.downloaded {
            // The count went back down, so a new part started without a finished line.
            self.bank();
        }
        self.downloaded = progress.downloaded;
        if progress.total.is_some() {
            self.part_total = progress.total;
        }
        let sized = self.part_total.is_some() || self.expected.is_some();
        if progress.finished {
            self.bank();
        }
        if sized {
            let size = self.size();
            if size > 0 {
                self.raise(self.copied() as f64 / size as f64);
            }
        }
    }

    /// Bytes downloaded so far, across all parts.
    pub fn copied(&self) -> u64 {
        self.banked.saturating_add(self.downloaded)
    }

    /// Expected size of all parts, or 0 when nothing is known yet.
    pub fn size(&self) -> u64 {
        let known = match self.part_total {
            Some(total) => self.banked.saturating_add(total.max(self.downloaded)),
            None if self.banked > 0 => self.copied(),
            None => 0,
        };
        known.max(self.expected.unwrap_or(0))
    }

    pub fn fraction(&self) -> Option<f64> {
        self.fraction
    }

    fn bank(&mut self) {
        let part = self.part_total.unwrap_or(0).max(self.downloaded);
        self.banked = self.banked.saturating_add(part);
        self.downloaded = 0;
        self.part_total = None;
    }

    fn raise(&mut self, fraction: f64) {
        let fraction = fraction.clamp(0.0, 1.0);
        self.fraction = Some(self.fraction.map_or(fraction, |old| old.max(fraction)));
    }
}

pub fn tool_args(node: &Path, ffmpeg_dir: &Path) -> Vec<String> {
    // The runtime is given by file name only, and callers run yt-dlp in node's
    // folder (tools::node_dir), so no drive-letter colon ever reaches the
    // `RUNTIME:PATH` value.
    let node_name = node.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_else(|| "node.exe".into());
    vec![
        "--ignore-config".into(),
        "--no-colors".into(),
        "--js-runtimes".into(),
        format!("node:{node_name}"),
        "--ffmpeg-location".into(),
        ffmpeg_dir.display().to_string(),
    ]
}

pub fn dump_args(node: &Path, ffmpeg_dir: &Path, url: &str) -> Vec<String> {
    let mut args = tool_args(node, ffmpeg_dir);
    args.extend([
        "--encoding".into(),
        "utf-8".into(),
        // Titles and URLs only. A full extract of every playlist video dumps
        // formats and subtitles, often exits 1, and the Names page never sees it.
        "--flat-playlist".into(),
        "--dump-single-json".into(),
        "--skip-download".into(),
        "--ignore-errors".into(),
        "--".into(),
        url.into(),
    ]);
    args
}

pub fn update_args(node: &Path, ffmpeg_dir: &Path) -> Vec<String> {
    let mut args = tool_args(node, ffmpeg_dir);
    args.extend(["--encoding".into(), "utf-8".into(), "-U".into()]);
    args
}

pub fn download_args(opts: &Options, url: &str, playlist_item: Option<u32>, output_template: &str, node: &Path, ffmpeg_dir: &Path) -> Vec<String> {
    let mut args = tool_args(node, ffmpeg_dir);
    args.extend([
        "--newline".into(),
        "--encoding".into(),
        "utf-8".into(),
        "--no-overwrites".into(),
        "--retries".into(),
        "10".into(),
        "--fragment-retries".into(),
        "10".into(),
        "--no-write-thumbnail".into(),
        "--no-embed-thumbnail".into(),
        "--no-write-description".into(),
        "-f".into(),
        format_selector(opts),
        "-o".into(),
        output_template.into(),
        "--progress-template".into(),
        PROGRESS_TEMPLATE.into(),
        "--print".into(),
        EXPECTED_PRINT.into(),
        "--print".into(),
        SAVED_PATH_PRINT.into(),
        // `--print` implies quiet, and quiet hides the progress bar unless this is set.
        "--progress".into(),
    ]);
    if !opts.audio_only {
        args.push("--merge-output-format".into());
        args.push("mkv".into());
    }
    if let Some(index) = playlist_item {
        args.push("--playlist-items".into());
        args.push(index.to_string());
    } else {
        args.push("--no-playlist".into());
    }
    if opts.strip_genre {
        args.push("--no-embed-metadata".into());
    } else {
        // Genre and the video description (description/synopsis tags) come from this pass.
        args.push("--embed-metadata".into());
        args.push("--no-embed-info-json".into());
    }
    if opts.sponsorblock {
        args.push("--sponsorblock-remove".into());
        args.push(SPONSORBLOCK_REMOVE.into());
        // Cutting at keyframes with a copy. Re-encoding around cuts stays off.
        args.push("--no-force-keyframes-at-cuts".into());
    }
    args.push("--".into());
    args.push(url.into());
    args
}

pub fn format_selector(opts: &Options) -> String {
    if opts.reencode {
        return if opts.audio_only {
            "ba/b".into()
        } else if let Some(height) = opts.resolution.height() {
            format!("bv*[height<={height}]+ba/b[height<={height}]/bv*+ba/b")
        } else {
            "bv*+ba/b".into()
        };
    }
    if opts.audio_only {
        return audio_selector(opts.audio_codec, opts.audio_quality);
    }
    video_selector(opts.resolution, opts.video_codec, opts.audio_codec, opts.audio_quality)
}

fn audio_selector(codec: AudioCodec, quality: AudioQuality) -> String {
    let Some(filter) = audio_filter(codec) else {
        return match quality.kbps() {
            Some(kbps) => format!("ba[abr<={kbps}]/ba/b"),
            None => "ba/b".into(),
        };
    };
    match quality.kbps() {
        Some(kbps) => format!("ba{filter}[abr<={kbps}]/ba{filter}"),
        None => format!("ba{filter}"),
    }
}

fn video_selector(resolution: Resolution, video: VideoCodec, audio: AudioCodec, quality: AudioQuality) -> String {
    let height = resolution.height().map(|h| format!("[height<={h}]")).unwrap_or_default();
    let audio = audio_filter(audio).unwrap_or_default();
    let abr = quality.kbps().map(|kbps| format!("[abr<={kbps}]")).unwrap_or_default();
    let videos = video_filters(video);
    if videos.is_empty() {
        return format!("bv*{height}+ba{audio}{abr}/b{height}{audio}{abr}");
    }
    let mut parts = Vec::new();
    for filter in &videos {
        parts.push(format!("bv*{height}{filter}+ba{audio}{abr}"));
    }
    for filter in &videos {
        parts.push(format!("b{height}{filter}{audio}{abr}"));
    }
    parts.join("/")
}

fn audio_filter(codec: AudioCodec) -> Option<&'static str> {
    match codec {
        AudioCodec::Aac => Some("[acodec^=mp4a]"),
        AudioCodec::Opus => Some("[acodec^=opus]"),
        AudioCodec::Mp3 => Some("[acodec^=mp3]"),
        AudioCodec::Vorbis => Some("[acodec^=vorbis]"),
        AudioCodec::Flac => Some("[acodec^=flac]"),
        AudioCodec::Alac => Some("[acodec^=alac]"),
        AudioCodec::Pcm => Some("[acodec^=pcm]"),
        AudioCodec::Any => None,
    }
}

fn video_filters(codec: VideoCodec) -> Vec<&'static str> {
    match codec {
        VideoCodec::H264 => vec!["[vcodec^=avc1]"],
        VideoCodec::H265 => vec!["[vcodec^=hev1]", "[vcodec^=hvc1]"],
        VideoCodec::Vp9 => vec!["[vcodec^=vp9]", "[vcodec^=vp09]"],
        VideoCodec::Av1 => vec!["[vcodec^=av01]"],
        VideoCodec::Vp8 => vec!["[vcodec^=vp8]", "[vcodec^=vp08]"],
        VideoCodec::Mpeg4 => vec!["[vcodec^=mp4v]", "[vcodec^=mpeg4]"],
        VideoCodec::Any => Vec::new(),
    }
}

pub fn metadata_args(tags: &Tags) -> Vec<String> {
    let mut args = Vec::new();
    let title = tags.title.trim();
    if !title.is_empty() {
        args.push("-metadata".into());
        args.push(format!("title={title}"));
    }
    let artists: Vec<&str> = tags.artists.iter().map(|name| name.trim()).filter(|name| !name.is_empty()).collect();
    if !artists.is_empty() {
        args.push("-metadata".into());
        args.push(format!("artist={}", artists.join("; ")));
    }
    let album = tags.album.trim();
    if !album.is_empty() {
        args.push("-metadata".into());
        args.push(format!("album={album}"));
    }
    if let Some(track) = tags.track {
        args.push("-metadata".into());
        let value = match tags.track_total {
            Some(total) if total > 0 => format!("{track}/{total}"),
            _ => track.to_string(),
        };
        args.push(format!("track={value}"));
    }
    args
}

pub fn remux_args(input: &Path, output: &Path, tags: &Tags, audio_only: bool, container: Container, realign: bool, strip_genre: bool) -> Vec<String> {
    let mut args = vec!["-y".into(), "-hide_banner".into()];
    if realign {
        args.extend(["-fflags".into(), "+genpts".into()]);
    }
    args.extend(["-i".into(), input.display().to_string()]);
    push_map_metadata(&mut args, strip_genre);
    if audio_only {
        args.extend(["-vn".into(), "-map".into(), "0:a".into()]);
    } else {
        args.extend(["-map".into(), "0:v:0?".into(), "-map".into(), "0:a".into()]);
    }
    args.extend(["-c".into(), "copy".into()]);
    if realign {
        args.extend(["-avoid_negative_ts".into(), "make_zero".into()]);
    }
    args.extend(metadata_args(tags));
    push_faststart(&mut args, container);
    push_container_format(&mut args, container);
    args.push(output.display().to_string());
    args
}

pub fn transcode_args(input: &Path, output: &Path, tags: &Tags, opts: &Options) -> Vec<String> {
    let mut args = vec!["-y".into(), "-hide_banner".into(), "-i".into(), input.display().to_string()];
    push_map_metadata(&mut args, opts.strip_genre);
    if opts.audio_only {
        args.extend(["-vn".into(), "-map".into(), "0:a".into()]);
        push_audio_encoder(&mut args, opts.audio_codec, opts.audio_quality);
    } else {
        args.extend(["-map".into(), "0:v:0?".into(), "-map".into(), "0:a?".into()]);
        if let Some(height) = opts.resolution.height() {
            args.push("-vf".into());
            args.push(format!("scale=-2:'min({height},ih)'"));
        }
        push_video_encoder(&mut args, opts.video_codec);
        push_audio_encoder(&mut args, opts.audio_codec, opts.audio_quality);
    }
    args.extend(metadata_args(tags));
    push_faststart(&mut args, opts.container);
    push_container_format(&mut args, opts.container);
    args.push(output.display().to_string());
    args
}

fn push_video_encoder(args: &mut Vec<String>, codec: VideoCodec) {
    let encoder = match codec {
        VideoCodec::H264 | VideoCodec::Any => "libx264",
        VideoCodec::H265 => "libx265",
        VideoCodec::Vp9 => "libvpx-vp9",
        VideoCodec::Av1 => "libsvtav1",
        VideoCodec::Vp8 => "libvpx",
        VideoCodec::Mpeg4 => "mpeg4",
    };
    args.extend(["-c:v".into(), encoder.into()]);
    match codec {
        VideoCodec::H264 | VideoCodec::Any | VideoCodec::Mpeg4 => {
            args.extend(["-preset".into(), "veryfast".into(), "-crf".into(), "20".into()]);
        }
        VideoCodec::H265 => {
            args.extend(["-preset".into(), "veryfast".into(), "-crf".into(), "24".into()]);
        }
        VideoCodec::Vp9 | VideoCodec::Vp8 => {
            args.extend(["-crf".into(), "32".into(), "-b:v".into(), "0".into()]);
        }
        VideoCodec::Av1 => {
            args.extend(["-crf".into(), "30".into()]);
        }
    }
}

fn push_audio_encoder(args: &mut Vec<String>, codec: AudioCodec, quality: AudioQuality) {
    let (encoder, bitrate) = match codec {
        AudioCodec::Aac | AudioCodec::Any => ("aac", true),
        AudioCodec::Opus => ("libopus", true),
        AudioCodec::Mp3 => ("libmp3lame", true),
        AudioCodec::Vorbis => ("libvorbis", true),
        AudioCodec::Flac => ("flac", false),
        AudioCodec::Alac => ("alac", false),
        AudioCodec::Pcm => ("pcm_s16le", false),
    };
    args.extend(["-c:a".into(), encoder.into()]);
    if bitrate {
        let kbps = quality.kbps().unwrap_or(192);
        args.extend(["-b:a".into(), format!("{kbps}k")]);
    }
}

fn push_map_metadata(args: &mut Vec<String>, strip_genre: bool) {
    if strip_genre {
        args.extend(["-map_metadata".into(), "-1".into()]);
        args.extend(["-map_metadata:s:a".into(), "-1".into()]);
        args.extend(["-map_metadata:s:v".into(), "-1".into()]);
    } else {
        args.extend(["-map_metadata".into(), "0".into()]);
    }
}

fn push_faststart(args: &mut Vec<String>, container: Container) {
    if matches!(container, Container::Mp4 | Container::M4a | Container::Mov) {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
}

/// `.m4a` makes ffmpeg pick the ipod muxer, which refuses Opus. Force the mp4 muxer.
fn push_container_format(args: &mut Vec<String>, container: Container) {
    if matches!(container, Container::Mp4 | Container::M4a | Container::Mov) {
        args.extend(["-f".into(), "mp4".into()]);
    }
}

pub fn cover_args(media: &Path, image: &Path, output: &Path, container: Container, audio_only: bool) -> Vec<String> {
    match container {
        Container::Mkv | Container::Mka => vec![
            "-y".into(),
            "-hide_banner".into(),
            "-i".into(),
            media.display().to_string(),
            "-map_metadata".into(),
            "0".into(),
            "-c".into(),
            "copy".into(),
            "-attach".into(),
            image.display().to_string(),
            "-metadata:s:t:0".into(),
            "mimetype=image/jpeg".into(),
            "-metadata:s:t:0".into(),
            "filename=cover.jpg".into(),
            output.display().to_string(),
        ],
        Container::Mp3 => vec![
            "-y".into(),
            "-hide_banner".into(),
            "-i".into(),
            media.display().to_string(),
            "-i".into(),
            image.display().to_string(),
            "-map".into(),
            "0:a".into(),
            "-map".into(),
            "1".into(),
            "-c".into(),
            "copy".into(),
            "-id3v2_version".into(),
            "3".into(),
            "-metadata:s:v".into(),
            "title=Cover".into(),
            "-metadata:s:v".into(),
            "comment=Cover (front)".into(),
            "-map_metadata".into(),
            "0".into(),
            output.display().to_string(),
        ],
        _ => {
            let mut args = vec!["-y".into(), "-hide_banner".into(), "-i".into(), media.display().to_string(), "-i".into(), image.display().to_string()];
            if audio_only {
                args.extend([
                    "-map".into(),
                    "0:a".into(),
                    "-map".into(),
                    "1".into(),
                    "-c:a".into(),
                    "copy".into(),
                    "-c:v".into(),
                    "mjpeg".into(),
                    "-disposition:v:0".into(),
                    "attached_pic".into(),
                ]);
            } else {
                args.extend([
                    "-map".into(),
                    "0".into(),
                    "-map".into(),
                    "1".into(),
                    "-c".into(),
                    "copy".into(),
                    "-c:v:1".into(),
                    "mjpeg".into(),
                    "-disposition:v:1".into(),
                    "attached_pic".into(),
                ]);
            }
            args.extend(["-map_metadata".into(), "0".into()]);
            push_faststart(&mut args, container);
            push_container_format(&mut args, container);
            args.push(output.display().to_string());
            args
        }
    }
}

pub fn jpeg_args(input: &Path, output: &Path) -> Vec<String> {
    vec![
        "-y".into(),
        "-hide_banner".into(),
        "-i".into(),
        input.display().to_string(),
        "-frames:v".into(),
        "1".into(),
        output.display().to_string(),
    ]
}

pub fn parse_progress(line: &str) -> Option<Progress> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("download:") {
        if let Some(progress) = pipe_progress(rest) {
            return Some(progress);
        }
    }
    if let Some(progress) = pipe_progress(line) {
        return Some(progress);
    }
    let rest = line.strip_prefix("[download]")?.trim();
    let percent: f64 = rest.split('%').next()?.trim().parse().ok()?;
    if !(0.0..=100.0).contains(&percent) {
        return None;
    }
    Some(Progress {
        downloaded: 0,
        total: None,
        speed: None,
        fraction: Some((percent / 100.0).clamp(0.0, 1.0)),
        finished: false,
    })
}

fn pipe_progress(rest: &str) -> Option<Progress> {
    let parts: Vec<&str> = rest.split('|').collect();
    if parts.len() < 5 {
        return None;
    }
    let status = parts[0].trim();
    if !matches!(status, "downloading" | "finished" | "error") {
        return None;
    }
    let downloaded = number_u64(parts[1]).unwrap_or(0);
    let total = number_u64(parts[2]).or_else(|| number_u64(parts[3]));
    let mut fraction = total.filter(|total| *total > 0).map(|total| (downloaded as f64 / total as f64).clamp(0.0, 1.0));
    if fraction.is_none() {
        if status == "finished" && downloaded > 0 {
            fraction = Some(1.0);
        } else if let Some(percent) = parts.get(5).and_then(|text| number_f64(text.trim().trim_end_matches('%'))) {
            if (0.0..=100.0).contains(&percent) {
                fraction = Some((percent / 100.0).clamp(0.0, 1.0));
            }
        }
    }
    Some(Progress { downloaded, total, speed: number_f64(parts[4]), fraction, finished: status == "finished" })
}

pub fn parse_expected_bytes(line: &str) -> Option<u64> {
    number_u64(line.trim().strip_prefix("expected_bytes:")?).filter(|bytes| *bytes > 0)
}

pub fn parse_after_move(line: &str) -> Option<String> {
    let path = line.trim().strip_prefix("after_move:")?.trim();
    if path.is_empty() { None } else { Some(path.to_string()) }
}

pub fn parse_out_time_ms(line: &str) -> Option<u64> {
    let rest = line.trim().strip_prefix("out_time_ms=")?;
    rest.trim().parse().ok()
}

fn number_u64(text: &str) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() || text.eq_ignore_ascii_case("NA") || text.eq_ignore_ascii_case("None") {
        return None;
    }
    text.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.0).map(|n| n as u64)
}

fn number_f64(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() || text.eq_ignore_ascii_case("NA") || text.eq_ignore_ascii_case("None") {
        return None;
    }
    text.parse::<f64>().ok().filter(|n| n.is_finite() && *n >= 0.0)
}

pub fn wait_seconds(min: u32, max: u32, rng: &mut u32) -> u32 {
    if max <= min {
        return min;
    }
    *rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
    min + (*rng % (max - min + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AudioCodec, AudioQuality, Resolution, VideoCodec};

    fn video_opts() -> Options {
        Options {
            location: "C:\\Music".into(),
            ..Options::default()
        }
    }

    #[test]
    fn dump_lists_playlist_titles_without_a_full_extract() {
        let args = dump_args(Path::new("node.exe"), Path::new("."), "https://example.com/x");
        assert!(args.iter().any(|arg| arg == "--flat-playlist"));
        assert!(args.iter().any(|arg| arg == "--dump-single-json"));
        assert!(args.iter().any(|arg| arg == "--skip-download"));
        assert!(args.iter().any(|arg| arg == "--ignore-errors"));
    }

    #[test]
    fn copy_format_asks_for_the_selected_codecs() {
        let spec = format_selector(&video_opts());
        assert!(spec.contains("vcodec^=avc1"));
        assert!(spec.contains("acodec^=mp4a"));
        assert!(spec.contains("height<=1080"));
        assert!(!spec.contains("libx264"));
    }

    #[test]
    fn reencode_format_does_not_lock_the_codec() {
        let mut opts = video_opts();
        opts.reencode = true;
        opts.video_codec = VideoCodec::Vp9;
        let spec = format_selector(&opts);
        assert!(!spec.contains("vcodec"));
        assert!(spec.starts_with("bv*"));
    }

    #[test]
    fn audio_copy_stays_on_the_audio_codec() {
        let mut opts = video_opts();
        opts.audio_only = true;
        opts.audio_codec = AudioCodec::Opus;
        opts.audio_quality = AudioQuality::K160;
        opts.container = crate::model::Container::M4a;
        assert_eq!(format_selector(&opts), "ba[acodec^=opus][abr<=160]/ba[acodec^=opus]");
    }

    #[test]
    fn blank_metadata_is_left_out_and_copy_drops_old_tags() {
        let tags = Tags {
            title: "Song".into(),
            artists: vec![" ".into(), "".into()],
            album: "  ".into(),
            track: None,
            track_total: None,
        };
        let args = remux_args(Path::new("in.mkv"), Path::new("out.m4a"), &tags, true, Container::M4a, false, true);
        assert!(has_pair(&args, "-map_metadata", "-1"));
        assert!(has_pair(&args, "-c", "copy"));
        assert!(args.iter().any(|arg| arg == "title=Song"));
        assert!(has_pair(&args, "-f", "mp4"));
        assert!(!args.iter().any(|arg| arg.starts_with("artist=")));
        assert!(!args.iter().any(|arg| arg.starts_with("album=")));
        assert!(!args.iter().any(|arg| arg.contains("libx264") || arg.contains("libopus")));
    }

    #[test]
    fn artists_are_joined_for_the_player_and_tracks_use_playlist_order() {
        let tags = Tags {
            title: "Under Pressure".into(),
            artists: vec!["Queen".into(), "David Bowie".into()],
            album: "Hot Space".into(),
            track: Some(2),
            track_total: Some(10),
        };
        let args = metadata_args(&tags);
        assert!(args.iter().any(|arg| arg == "artist=Queen; David Bowie"));
        assert!(args.iter().any(|arg| arg == "track=2/10"));
    }

    #[test]
    fn transcode_uses_an_encoder_instead_of_copy() {
        let mut opts = video_opts();
        opts.reencode = true;
        opts.video_codec = VideoCodec::H264;
        opts.audio_codec = AudioCodec::Aac;
        opts.resolution = Resolution::P720;
        let args = transcode_args(Path::new("in.mkv"), Path::new("out.mp4"), &Tags {
            title: "Song".into(),
            artists: vec!["Queen".into()],
            album: String::new(),
            track: None,
            track_total: None,
        }, &opts);
        assert!(args.iter().any(|arg| arg == "libx264"));
        assert!(args.iter().any(|arg| arg == "aac"));
        assert!(!has_pair(&args, "-c", "copy"));
    }

    fn has_pair(args: &[String], left: &str, right: &str) -> bool {
        args.windows(2).any(|pair| pair[0] == left && pair[1] == right)
    }

    #[test]
    fn progress_lines_parse() {
        let progress = parse_progress("download:downloading|1536|4096|NA|2048").unwrap();
        assert_eq!(progress.downloaded, 1536);
        assert_eq!(progress.total, Some(4096));
        assert_eq!(progress.speed, Some(2048.0));
        assert_eq!(progress.fraction, Some(0.375));
        assert!(!progress.finished);
        assert!(parse_progress("download:finished|4096|4096|NA|2048").unwrap().finished);
        let stripped = parse_progress("downloading|1536|4096|NA|2048").unwrap();
        assert_eq!(stripped.fraction, Some(0.375));
        assert_eq!(parse_progress("[download]  12.5% of 10.00MiB at 1.00MiB/s").unwrap().fraction, Some(0.125));
        assert_eq!(parse_after_move("after_move:C:\\Temp\\a.mkv").as_deref(), Some("C:\\Temp\\a.mkv"));
        assert_eq!(parse_after_move("after_move:C:\\Music\\Song - Live (2020).mkv").as_deref(), Some("C:\\Music\\Song - Live (2020).mkv"));
        assert_eq!(parse_after_move("C:\\Temp\\a.mkv"), None);
        assert_eq!(parse_after_move("after_move:"), None);
        assert_eq!(parse_out_time_ms("out_time_ms=1500000"), Some(1_500_000));
    }

    fn feed(tracker: &mut ItemProgress, line: &str) -> (u64, u64, f64) {
        if line.starts_with("expected_bytes:") {
            if let Some(bytes) = parse_expected_bytes(line) {
                tracker.expect(bytes);
            }
        } else {
            tracker.update(&parse_progress(line).unwrap());
        }
        (tracker.copied(), tracker.size(), tracker.fraction().unwrap_or(0.0))
    }

    #[test]
    fn video_and_audio_parts_add_up_without_going_back() {
        let mut tracker = ItemProgress::default();
        assert_eq!(feed(&mut tracker, "expected_bytes:1000"), (0, 1000, 0.0));
        assert_eq!(feed(&mut tracker, "download:downloading|400|800|NA|10"), (400, 1000, 0.4));
        assert_eq!(feed(&mut tracker, "download:finished|800|800|NA|10"), (800, 1000, 0.8));
        // The audio part starts counting from 0 again.
        assert_eq!(feed(&mut tracker, "download:downloading|0|200|NA|NA"), (800, 1000, 0.8));
        assert_eq!(feed(&mut tracker, "download:downloading|100|200|NA|10"), (900, 1000, 0.9));
        assert_eq!(feed(&mut tracker, "download:finished|200|200|NA|10"), (1000, 1000, 1.0));
    }

    #[test]
    fn parts_add_up_without_an_expected_size() {
        let mut tracker = ItemProgress::default();
        assert_eq!(feed(&mut tracker, "download:downloading|300|600|NA|10"), (300, 600, 0.5));
        assert_eq!(feed(&mut tracker, "download:finished|600|600|NA|10"), (600, 600, 1.0));
        // The second part's size was not known in advance, so the bar holds
        // instead of dropping, and the byte counts keep growing.
        assert_eq!(feed(&mut tracker, "download:downloading|100|400|NA|10"), (700, 1000, 1.0));
        assert_eq!(feed(&mut tracker, "download:finished|400|400|NA|10"), (1000, 1000, 1.0));
    }

    #[test]
    fn a_count_that_drops_starts_a_new_part() {
        let mut tracker = ItemProgress::default();
        tracker.expect(1000);
        assert_eq!(feed(&mut tracker, "download:downloading|700|700|NA|10"), (700, 1000, 0.7));
        assert_eq!(feed(&mut tracker, "download:downloading|50|300|NA|10"), (750, 1000, 0.75));
    }

    #[test]
    fn single_part_progress_is_unchanged() {
        let mut tracker = ItemProgress::default();
        assert_eq!(feed(&mut tracker, "expected_bytes:NA"), (0, 0, 0.0));
        assert_eq!(feed(&mut tracker, "download:downloading|1536|4096|NA|2048"), (1536, 4096, 0.375));
        assert_eq!(feed(&mut tracker, "download:finished|4096|4096|NA|2048"), (4096, 4096, 1.0));
        let mut percent = ItemProgress::default();
        assert_eq!(feed(&mut percent, "[download]  12.5% of 10.00MiB at 1.00MiB/s"), (0, 0, 0.125));
        assert_eq!(feed(&mut percent, "[download]  10.0% of 10.00MiB at 1.00MiB/s"), (0, 0, 0.125));
    }

    #[test]
    fn already_downloaded_part_counts_its_size() {
        let mut tracker = ItemProgress::default();
        tracker.expect(1000);
        assert_eq!(feed(&mut tracker, "download:finished|NA|800|NA|NA"), (800, 1000, 0.8));
        assert_eq!(feed(&mut tracker, "download:downloading|100|200|NA|10"), (900, 1000, 0.9));
    }

    #[test]
    fn expected_bytes_lines_parse() {
        assert_eq!(parse_expected_bytes("expected_bytes:437474"), Some(437474));
        assert_eq!(parse_expected_bytes("expected_bytes:123.7"), Some(123));
        assert_eq!(parse_expected_bytes("expected_bytes:NA"), None);
        assert_eq!(parse_expected_bytes("expected_bytes:0"), None);
        assert_eq!(parse_expected_bytes("download:downloading|1|2|NA|3"), None);
        assert!(parse_progress("expected_bytes:437474").is_none());
    }

    #[test]
    fn tool_args_pass_resolved_node_and_ffmpeg() {
        let args = tool_args(Path::new("D:\\apps\\node\\node22.exe"), Path::new("D:\\apps\\ffmpeg\\bin"));
        assert!(has_pair(&args, "--js-runtimes", "node:node22.exe"));
        assert!(has_pair(&args, "--ffmpeg-location", "D:\\apps\\ffmpeg\\bin"));
    }

    #[test]
    fn download_args_keep_progress_visible() {
        let args = download_args(
            &video_opts(),
            "https://example.com/a",
            None,
            "C:\\out\\song.%(ext)s",
            Path::new("C:\\tools\\node.exe"),
            Path::new("C:\\tools"),
        );
        assert!(args.windows(2).any(|pair| pair[0] == "--progress-template" && pair[1] == PROGRESS_TEMPLATE));
        assert!(args.windows(2).any(|pair| pair[0] == "--print" && pair[1] == EXPECTED_PRINT));
        assert!(PROGRESS_TEMPLATE.starts_with("download:download:"));
        assert!(args.windows(2).any(|pair| pair[0] == "--print" && pair[1] == SAVED_PATH_PRINT));
        assert!(SAVED_PATH_PRINT.starts_with("after_move:after_move:"));
        assert!(args.iter().any(|arg| arg == "--newline"));
        let print_at = args.iter().position(|arg| arg == "--print").unwrap();
        let progress_at = args.iter().position(|arg| arg == "--progress").unwrap();
        assert!(progress_at > print_at);
        assert!(!args.iter().any(|arg| arg == "--sponsorblock-remove"));
        assert!(args.iter().any(|arg| arg == "--no-embed-metadata"));
        assert!(args.iter().any(|arg| arg == "--no-write-description"));
        assert!(!args.iter().any(|arg| arg == "--embed-metadata"));
    }

    #[test]
    fn strip_genre_drops_auto_tags_and_keeping_them_embeds() {
        let tags = Tags {
            title: "Song".into(),
            artists: vec!["Queen".into()],
            album: String::new(),
            track: None,
            track_total: None,
        };
        let stripped = remux_args(Path::new("in.mkv"), Path::new("out.mp4"), &tags, false, Container::Mp4, false, true);
        assert!(has_pair(&stripped, "-map_metadata", "-1"));
        assert!(has_pair(&stripped, "-map_metadata:s:a", "-1"));
        assert!(has_pair(&stripped, "-map_metadata:s:v", "-1"));
        assert!(!stripped.iter().any(|arg| arg.starts_with("genre=") || arg.starts_with("description=") || arg.starts_with("synopsis=") || arg.starts_with("comment=")));
        let kept = remux_args(Path::new("in.mkv"), Path::new("out.mp4"), &tags, false, Container::Mp4, false, false);
        assert!(has_pair(&kept, "-map_metadata", "0"));
        let mut opts = video_opts();
        opts.strip_genre = false;
        let download = download_args(
            &opts,
            "https://example.com/a",
            None,
            "C:\\out\\song.%(ext)s",
            Path::new("C:\\tools\\node.exe"),
            Path::new("C:\\tools"),
        );
        assert!(download.iter().any(|arg| arg == "--embed-metadata"));
        assert!(download.iter().any(|arg| arg == "--no-embed-info-json"));
        assert!(download.iter().any(|arg| arg == "--no-write-description"));
        assert!(!download.iter().any(|arg| arg == "--no-embed-metadata"));
    }

    #[test]
    fn sponsorblock_cuts_without_a_reencode() {
        let mut opts = video_opts();
        opts.sponsorblock = true;
        let args = download_args(
            &opts,
            "https://example.com/a",
            None,
            "C:\\out\\song.%(ext)s",
            Path::new("C:\\tools\\node.exe"),
            Path::new("C:\\tools"),
        );
        assert!(has_pair(&args, "--sponsorblock-remove", SPONSORBLOCK_REMOVE));
        assert!(args.iter().any(|arg| arg == "--no-force-keyframes-at-cuts"));
        assert!(!args.iter().any(|arg| arg == "--force-keyframes-at-cuts"));
        let remux = remux_args(
            Path::new("in.mkv"),
            Path::new("out.mp4"),
            &Tags {
                title: "Song".into(),
                artists: vec!["Queen".into()],
                album: String::new(),
                track: None,
                track_total: None,
            },
            false,
            Container::Mp4,
            true,
            true,
        );
        assert!(has_pair(&remux, "-c", "copy"));
        assert!(has_pair(&remux, "-fflags", "+genpts"));
        assert!(has_pair(&remux, "-avoid_negative_ts", "make_zero"));
        assert!(!remux.iter().any(|arg| arg.contains("libx264") || arg.contains("libopus") || arg == "aac"));
        let audio = remux_args(Path::new("in.webm"), Path::new("out.m4a"), &Tags {
            title: "Song".into(),
            artists: Vec::new(),
            album: String::new(),
            track: None,
            track_total: None,
        }, true, Container::M4a, false, true);
        assert!(has_pair(&audio, "-c", "copy"));
        assert!(!audio.windows(2).any(|pair| pair[0] == "-fflags" && pair[1] == "+genpts"));
    }

    #[test]
    fn wait_stays_inside_the_range() {
        let mut rng = 1u32;
        for _ in 0..1000 {
            let wait = wait_seconds(4, 8, &mut rng);
            assert!((4..=8).contains(&wait));
        }
        assert_eq!(wait_seconds(5, 5, &mut rng), 5);
    }
}
