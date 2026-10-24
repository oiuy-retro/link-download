//! Options, downloaded items, and which codec can be copied into which container.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
    Best,
    P144,
    P240,
    P360,
    P480,
    P720,
    P1080,
    P1440,
    P2160,
    P4320,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    H265,
    Vp9,
    Av1,
    Vp8,
    Mpeg4,
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioQuality {
    Best,
    K320,
    K256,
    K192,
    K160,
    K128,
    K96,
    K64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioCodec {
    Aac,
    Opus,
    Mp3,
    Vorbis,
    Flac,
    Alac,
    Pcm,
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Container {
    Mp4,
    Mkv,
    Webm,
    Mov,
    Avi,
    M4a,
    Mp3,
    Opus,
    Ogg,
    Flac,
    Wav,
    Aac,
    Mka,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub resolution: Resolution,
    pub video_codec: VideoCodec,
    pub audio_quality: AudioQuality,
    pub audio_codec: AudioCodec,
    pub container: Container,
    pub audio_only: bool,
    pub reencode: bool,
    pub sponsorblock: bool,
    /// Drop yt-dlp's automatic genre tag and the video description.
    pub strip_genre: bool,
    pub min_wait: u32,
    pub max_wait: u32,
    pub location: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub message: String,
    /// Re-encoding can let the user past a codec/container mismatch.
    pub codec: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Age,
    Missing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: String,
    /// File name, without the extension.
    pub title: String,
    /// Title tag. Any character is allowed, including characters a file name cannot use.
    pub meta_title: String,
    pub original_title: String,
    pub original_artists: Vec<String>,
    pub artists: Vec<String>,
    pub download_url: String,
    /// Set when the video has no direct address and must be taken from the playlist.
    pub playlist_item: Option<u32>,
    pub track_number: Option<u32>,
    pub duration: f64,
    pub thumbnail: String,
    pub temp_path: Option<String>,
    pub blocked: Option<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub source_url: String,
    pub is_playlist: bool,
    pub playlist_title: String,
    pub album: String,
    pub original_album: String,
    pub playlist_thumbnail: String,
    pub error: String,
    pub items: Vec<Item>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            resolution: Resolution::P1080,
            video_codec: VideoCodec::H264,
            audio_quality: AudioQuality::Best,
            audio_codec: AudioCodec::Aac,
            container: Container::Mp4,
            audio_only: false,
            reencode: false,
            sponsorblock: false,
            strip_genre: true,
            min_wait: 4,
            max_wait: 8,
            location: String::new(),
        }
    }
}

impl Resolution {
    pub const ALL: [Resolution; 10] = [
        Resolution::Best,
        Resolution::P4320,
        Resolution::P2160,
        Resolution::P1440,
        Resolution::P1080,
        Resolution::P720,
        Resolution::P480,
        Resolution::P360,
        Resolution::P240,
        Resolution::P144,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Resolution::Best => "Best",
            Resolution::P144 => "144p",
            Resolution::P240 => "240p",
            Resolution::P360 => "360p",
            Resolution::P480 => "480p",
            Resolution::P720 => "720p",
            Resolution::P1080 => "1080p",
            Resolution::P1440 => "1440p",
            Resolution::P2160 => "2160p",
            Resolution::P4320 => "4320p",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Resolution::Best => "best",
            Resolution::P144 => "144",
            Resolution::P240 => "240",
            Resolution::P360 => "360",
            Resolution::P480 => "480",
            Resolution::P720 => "720",
            Resolution::P1080 => "1080",
            Resolution::P1440 => "1440",
            Resolution::P2160 => "2160",
            Resolution::P4320 => "4320",
        }
    }

    pub fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|v| v.key() == s).unwrap_or(Resolution::P1080)
    }

    pub fn height(self) -> Option<i32> {
        match self {
            Resolution::Best => None,
            Resolution::P144 => Some(144),
            Resolution::P240 => Some(240),
            Resolution::P360 => Some(360),
            Resolution::P480 => Some(480),
            Resolution::P720 => Some(720),
            Resolution::P1080 => Some(1080),
            Resolution::P1440 => Some(1440),
            Resolution::P2160 => Some(2160),
            Resolution::P4320 => Some(4320),
        }
    }
}

impl VideoCodec {
    pub const ALL: [VideoCodec; 7] = [
        VideoCodec::H264,
        VideoCodec::H265,
        VideoCodec::Vp9,
        VideoCodec::Av1,
        VideoCodec::Vp8,
        VideoCodec::Mpeg4,
        VideoCodec::Any,
    ];

    pub fn label(self) -> &'static str {
        match self {
            VideoCodec::H264 => "H.264",
            VideoCodec::H265 => "H.265",
            VideoCodec::Vp9 => "VP9",
            VideoCodec::Av1 => "AV1",
            VideoCodec::Vp8 => "VP8",
            VideoCodec::Mpeg4 => "MPEG-4",
            VideoCodec::Any => "Any",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            VideoCodec::H264 => "h264",
            VideoCodec::H265 => "h265",
            VideoCodec::Vp9 => "vp9",
            VideoCodec::Av1 => "av1",
            VideoCodec::Vp8 => "vp8",
            VideoCodec::Mpeg4 => "mpeg4",
            VideoCodec::Any => "any",
        }
    }

    pub fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|v| v.key() == s).unwrap_or(VideoCodec::H264)
    }
}

impl AudioQuality {
    pub const ALL: [AudioQuality; 8] = [
        AudioQuality::Best,
        AudioQuality::K320,
        AudioQuality::K256,
        AudioQuality::K192,
        AudioQuality::K160,
        AudioQuality::K128,
        AudioQuality::K96,
        AudioQuality::K64,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AudioQuality::Best => "Best",
            AudioQuality::K320 => "320 kbps",
            AudioQuality::K256 => "256 kbps",
            AudioQuality::K192 => "192 kbps",
            AudioQuality::K160 => "160 kbps",
            AudioQuality::K128 => "128 kbps",
            AudioQuality::K96 => "96 kbps",
            AudioQuality::K64 => "64 kbps",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            AudioQuality::Best => "best",
            AudioQuality::K320 => "320",
            AudioQuality::K256 => "256",
            AudioQuality::K192 => "192",
            AudioQuality::K160 => "160",
            AudioQuality::K128 => "128",
            AudioQuality::K96 => "96",
            AudioQuality::K64 => "64",
        }
    }

    pub fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|v| v.key() == s).unwrap_or(AudioQuality::Best)
    }

    pub fn kbps(self) -> Option<u32> {
        match self {
            AudioQuality::Best => None,
            AudioQuality::K320 => Some(320),
            AudioQuality::K256 => Some(256),
            AudioQuality::K192 => Some(192),
            AudioQuality::K160 => Some(160),
            AudioQuality::K128 => Some(128),
            AudioQuality::K96 => Some(96),
            AudioQuality::K64 => Some(64),
        }
    }
}

impl AudioCodec {
    pub const ALL: [AudioCodec; 8] = [
        AudioCodec::Aac,
        AudioCodec::Opus,
        AudioCodec::Mp3,
        AudioCodec::Vorbis,
        AudioCodec::Flac,
        AudioCodec::Alac,
        AudioCodec::Pcm,
        AudioCodec::Any,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AudioCodec::Aac => "AAC",
            AudioCodec::Opus => "Opus",
            AudioCodec::Mp3 => "MP3",
            AudioCodec::Vorbis => "Vorbis",
            AudioCodec::Flac => "FLAC",
            AudioCodec::Alac => "ALAC",
            AudioCodec::Pcm => "WAV",
            AudioCodec::Any => "Any",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            AudioCodec::Aac => "aac",
            AudioCodec::Opus => "opus",
            AudioCodec::Mp3 => "mp3",
            AudioCodec::Vorbis => "vorbis",
            AudioCodec::Flac => "flac",
            AudioCodec::Alac => "alac",
            AudioCodec::Pcm => "pcm",
            AudioCodec::Any => "any",
        }
    }

    pub fn parse(s: &str) -> Self {
        Self::ALL.into_iter().find(|v| v.key() == s).unwrap_or(AudioCodec::Aac)
    }

    pub fn lossless(self) -> bool {
        matches!(self, AudioCodec::Flac | AudioCodec::Alac | AudioCodec::Pcm)
    }
}

impl Container {
    pub const VIDEO: [Container; 5] = [
        Container::Mp4,
        Container::Mkv,
        Container::Webm,
        Container::Mov,
        Container::Avi,
    ];

    pub const AUDIO: [Container; 8] = [
        Container::M4a,
        Container::Mp3,
        Container::Opus,
        Container::Ogg,
        Container::Flac,
        Container::Wav,
        Container::Aac,
        Container::Mka,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Container::Mp4 => "MP4",
            Container::Mkv => "MKV",
            Container::Webm => "WebM",
            Container::Mov => "MOV",
            Container::Avi => "AVI",
            Container::M4a => "M4A",
            Container::Mp3 => "MP3",
            Container::Opus => "Opus",
            Container::Ogg => "Ogg",
            Container::Flac => "FLAC",
            Container::Wav => "WAV",
            Container::Aac => "AAC",
            Container::Mka => "MKA",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mkv => "mkv",
            Container::Webm => "webm",
            Container::Mov => "mov",
            Container::Avi => "avi",
            Container::M4a => "m4a",
            Container::Mp3 => "mp3",
            Container::Opus => "opus",
            Container::Ogg => "ogg",
            Container::Flac => "flac",
            Container::Wav => "wav",
            Container::Aac => "aac",
            Container::Mka => "mka",
        }
    }

    pub fn extension(self) -> &'static str {
        self.key()
    }

    pub fn parse(s: &str) -> Self {
        Self::VIDEO
            .into_iter()
            .chain(Self::AUDIO)
            .find(|v| v.key() == s)
            .unwrap_or(Container::Mp4)
    }

    pub fn listed_for(audio_only: bool) -> &'static [Container] {
        if audio_only {
            &Self::AUDIO
        } else {
            &Self::VIDEO
        }
    }

    pub fn is_audio(self) -> bool {
        Self::AUDIO.contains(&self)
    }
}

pub fn can_copy_video(codec: VideoCodec, container: Container) -> bool {
    match container {
        // Matroska can store every video codec this program offers.
        Container::Mkv => true,
        Container::Mp4 | Container::Mov => {
            matches!(codec, VideoCodec::H264 | VideoCodec::H265 | VideoCodec::Av1 | VideoCodec::Mpeg4)
        }
        Container::Webm => matches!(codec, VideoCodec::Vp8 | VideoCodec::Vp9 | VideoCodec::Av1),
        Container::Avi => matches!(codec, VideoCodec::H264 | VideoCodec::Mpeg4),
        _ => false,
    }
}

pub fn can_copy_audio(codec: AudioCodec, container: Container) -> bool {
    match container {
        Container::Mkv | Container::Mka => true,
        Container::Mp4 | Container::M4a | Container::Mov => {
            matches!(
                codec,
                AudioCodec::Aac | AudioCodec::Opus | AudioCodec::Mp3 | AudioCodec::Alac | AudioCodec::Flac
            )
        }
        Container::Webm => matches!(codec, AudioCodec::Opus | AudioCodec::Vorbis),
        Container::Avi => matches!(codec, AudioCodec::Mp3 | AudioCodec::Pcm),
        Container::Mp3 => codec == AudioCodec::Mp3,
        Container::Opus => codec == AudioCodec::Opus,
        Container::Ogg => matches!(codec, AudioCodec::Opus | AudioCodec::Vorbis | AudioCodec::Flac),
        Container::Flac => codec == AudioCodec::Flac,
        Container::Wav => codec == AudioCodec::Pcm,
        Container::Aac => codec == AudioCodec::Aac,
    }
}

/// Keep the current container when it is still on the menu. Otherwise pick the
/// first container that can hold the selected codecs without re-encoding.
pub fn container_after_toggle(opts: &Options) -> Container {
    let listed = Container::listed_for(opts.audio_only);
    if listed.contains(&opts.container) {
        return opts.container;
    }
    if opts.audio_only && opts.container == Container::Mp4 {
        return Container::M4a;
    }
    for candidate in listed {
        let audio_ok = can_copy_audio(opts.audio_codec, *candidate);
        let video_ok = opts.audio_only || can_copy_video(opts.video_codec, *candidate);
        if audio_ok && video_ok {
            return *candidate;
        }
    }
    listed[0]
}

pub fn problems(opts: &Options) -> Vec<Problem> {
    let mut out = Vec::new();
    if opts.min_wait > opts.max_wait {
        out.push(Problem {
            message: "The maximum wait is shorter than the minimum wait.".into(),
            codec: false,
        });
    }
    if let Err(message) = crate::paths::clean_location(&opts.location) {
        out.push(Problem { message, codec: false });
    }
    if let Some(message) = codec_problem(opts) {
        out.push(Problem { message, codec: true });
    }
    out
}

pub fn can_advance(opts: &Options) -> bool {
    if opts.location.trim().is_empty() {
        return false;
    }
    problems(opts).iter().all(|problem| problem.codec && opts.reencode)
}

fn codec_problem(opts: &Options) -> Option<String> {
    if opts.audio_only {
        if can_copy_audio(opts.audio_codec, opts.container) {
            return None;
        }
        return Some(codec_message(None, Some(opts.audio_codec), opts.container));
    }
    let video_ok = can_copy_video(opts.video_codec, opts.container);
    let audio_ok = can_copy_audio(opts.audio_codec, opts.container);
    match (video_ok, audio_ok) {
        (true, true) => None,
        (false, true) => Some(codec_message(Some(opts.video_codec), None, opts.container)),
        (true, false) => Some(codec_message(None, Some(opts.audio_codec), opts.container)),
        (false, false) => Some(codec_message(Some(opts.video_codec), Some(opts.audio_codec), opts.container)),
    }
}

fn codec_message(video: Option<VideoCodec>, audio: Option<AudioCodec>, container: Container) -> String {
    let file = container.label();
    match (video, audio) {
        (Some(VideoCodec::Any), Some(AudioCodec::Any)) => format!(
            "Any video codec and any audio codec cannot be copied into a {file} file, because the download might use a codec that file cannot store. Turn on re-encoding, or choose a codec."
        ),
        (Some(VideoCodec::Any), None) => format!(
            "Any video codec cannot be copied into a {file} file, because the download might use a codec that file cannot store. Turn on re-encoding, or choose a codec."
        ),
        (None, Some(AudioCodec::Any)) => format!(
            "Any audio codec cannot be copied into a {file} file, because the download might use a codec that file cannot store. Turn on re-encoding, or choose a codec."
        ),
        (Some(video), Some(audio)) => format!(
            "{video} video and {audio} audio cannot be copied into a {file} file. Turn on re-encoding, or choose a different codec or container.",
            video = video.label(),
            audio = audio.label()
        ),
        (Some(video), None) => format!(
            "{video} video cannot be copied into a {file} file. Turn on re-encoding, or choose a different codec or container.",
            video = video.label()
        ),
        (None, Some(audio)) => format!(
            "{audio} audio cannot be copied into a {file} file. Turn on re-encoding, or choose a different codec or container.",
            audio = audio.label()
        ),
        (None, None) => format!("That selection cannot be copied into a {file} file."),
    }
}

/// Format fields that change which media yt-dlp downloads.
pub fn same_download(a: &Options, b: &Options) -> bool {
    a.resolution == b.resolution
        && a.video_codec == b.video_codec
        && a.audio_quality == b.audio_quality
        && a.audio_codec == b.audio_codec
        && a.audio_only == b.audio_only
        && a.reencode == b.reencode
        && a.sponsorblock == b.sponsorblock
        && a.strip_genre == b.strip_genre
        && a.min_wait == b.min_wait
        && a.max_wait == b.max_wait
}

pub fn parse_links(text: &str) -> Result<Vec<String>, String> {
    let mut urls = Vec::new();
    let mut bad = None;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") {
            urls.push(line.to_string());
        } else if bad.is_none() {
            bad = Some(index + 1);
        }
    }
    if let Some(line) = bad {
        return Err(format!("Line {line} is not a link. Put one link on each line."));
    }
    if urls.is_empty() {
        return Err("Paste at least one link.".into());
    }
    Ok(urls)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Options {
        Options {
            location: "C:\\Music".into(),
            ..Options::default()
        }
    }

    #[test]
    fn default_video_selection_can_advance() {
        assert!(can_advance(&opts()));
        assert!(problems(&opts()).is_empty());
        assert!(Options::default().strip_genre);
    }

    #[test]
    fn empty_location_blocks_without_a_codec_problem() {
        let mut options = opts();
        options.location.clear();
        assert!(!can_advance(&options));
        assert!(problems(&options).is_empty());
    }

    #[test]
    fn unusable_location_blocks_next_and_says_why() {
        let mut options = opts();
        options.location = "C:\\Mu|sic".into();
        assert!(!can_advance(&options));
        let found = problems(&options);
        assert_eq!(found.len(), 1);
        assert!(found[0].message.contains("can't contain"), "{}", found[0].message);
        options.reencode = true;
        assert!(!can_advance(&options), "re-encoding must not hide a bad folder");
    }

    #[test]
    fn wait_order_is_not_fixed_by_reencoding() {
        let mut options = opts();
        options.min_wait = 9;
        options.max_wait = 2;
        options.reencode = true;
        assert!(!can_advance(&options));
        assert!(!problems(&options)[0].codec);
    }

    #[test]
    fn vp9_in_mp4_needs_reencoding() {
        let mut options = opts();
        options.video_codec = VideoCodec::Vp9;
        let found = problems(&options);
        assert!(found.iter().any(|problem| problem.codec));
        assert!(!can_advance(&options));
        options.reencode = true;
        assert!(can_advance(&options));
        assert!(found[0].message.contains("VP9"));
        assert!(found[0].message.contains("re-encoding"));
    }

    #[test]
    fn opus_in_mp3_needs_reencoding() {
        let mut options = opts();
        options.audio_only = true;
        options.audio_codec = AudioCodec::Opus;
        options.container = Container::Mp3;
        assert!(!can_advance(&options));
        options.reencode = true;
        assert!(can_advance(&options));
    }

    #[test]
    fn opus_in_m4a_is_a_copy() {
        let mut options = opts();
        options.audio_only = true;
        options.audio_codec = AudioCodec::Opus;
        options.container = Container::M4a;
        assert!(can_advance(&options));
    }

    #[test]
    fn any_codec_is_copied_only_into_matroska() {
        let mut options = opts();
        options.video_codec = VideoCodec::Any;
        options.audio_codec = AudioCodec::Any;
        options.container = Container::Mp4;
        assert!(!can_advance(&options));
        options.container = Container::Mkv;
        assert!(can_advance(&options));
    }

    #[test]
    fn audio_only_toggle_maps_mp4_to_m4a() {
        let mut options = opts();
        options.audio_only = true;
        assert_eq!(container_after_toggle(&options), Container::M4a);
    }

    #[test]
    fn links_skip_blank_lines_and_reject_other_text() {
        assert_eq!(parse_links("\nhttps://example.com/a\n\n").unwrap(), vec!["https://example.com/a".to_string()]);
        assert!(parse_links("hello").is_err());
        assert!(parse_links("  \n").is_err());
    }
}
