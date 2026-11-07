# Link Download

Windows 98–style GUI for [yt-dlp](https://github.com/yt-dlp/yt-dlp). Download videos or audio, remux with FFmpeg, edit names and tags, then write files to a folder you pick.

A built copy is [`Link.Download.exe`](https://github.com/oiuy-retro/link-download/releases/latest/download/Link.Download.exe) on the [latest release](https://github.com/oiuy-retro/link-download/releases/latest).

## What it does

Wizard pages: Options, Links, Names, Titles, Metadata, Download.

- Profiles save your settings (format, quality, SponsorBlock, genre stripping, wait times, and download folder) so you can reuse them.
- Video or audio only, in the resolution, codecs, audio quality and container you pick (144p to 4320p; H.264, H.265, VP9, AV1 and more; MP4, MKV, WebM, M4A, MP3, Opus, FLAC and more).
- **Strip genre and description** is on by default so YouTube categories and the video description are not stored as tags.
- **SponsorBlock** (off by default) cuts sponsor, self-promotion, and interaction segments through yt-dlp. Cuts are stream-copy; they land on keyframes.
- Cover art: Matroska attachment `cover.jpg`, or an `attached_pic` still on MP4/M4A.

## Run

1. Copy `Link.Download.exe` from the [latest release](https://github.com/oiuy-retro/link-download/releases/latest) to a folder.
2. Place these files beside it, or in a folder beside it (see [configuration.conf](#configurationconf)):
   - [yt-dlp](https://github.com/yt-dlp/yt-dlp/releases) `yt-dlp.exe`
   - [FFmpeg](https://www.gyan.dev/ffmpeg/builds/) `ffmpeg.exe` and `ffprobe.exe`
   - [Node.js](https://nodejs.org/) `node.exe` (yt-dlp uses it for YouTube player JS)
3. Start `Link Download.exe` from that folder.

Windows 10 or later.

Tested with yt-dlp `2026.10.10.233141`, FFmpeg `2026-09-10-git-fd7c73d01e-full_build` (Gyan), and Node `v24.16.0`.

## configuration.conf

On first start Link Download writes `configuration.conf` next to `Link Download.exe`. It is a plain text file (UTF-8). Lines starting with `#` are notes, names in `[brackets]` start a section, and settings are `key = value`. Names ignore upper and lower case, and lines Link Download does not know are left alone. The other oiuy-retro Windows apps use the same layout.

If the program folder can't be written to (for example under Program Files), the file goes to `%APPDATA%\link-download\configuration.conf` instead. Link Download reads the one beside the program first. If neither can be written, it runs on the defaults.

### [tools]

```
[tools]
yt-dlp = auto
ffmpeg = auto
ffprobe = auto
node = auto
```

Each line can be:

- `auto` (also used when the line is missing): look next to `Link Download.exe`, then in each folder directly beside it (A to Z, first match wins), then in the `bin` folder inside each of those, then on PATH. `ffprobe` first looks in the folder `ffmpeg` was found in.
- `path`: only look on PATH.
- A file path, such as `D:\apps\ffmpeg\bin\ffmpeg.exe` or `tools\node.exe`. Relative paths start from the folder `Link Download.exe` is in, not from where it was started. `%USERPROFILE%` and other `%NAMES%` are filled in. A folder holding the usual file name (`ffmpeg.exe` and so on) also works.

A path that doesn't exist is not replaced by a search. Link Download shows a message with the setting, its line number, and the full path it tried.

yt-dlp is given the ffmpeg folder found here (`--ffmpeg-location`) and the node found here (`--js-runtimes`). yt-dlp looks for ffprobe in the same folder as ffmpeg, so keep the two together.

### [folders] and [link-download]

`[folders]` is for the last folder each Browse button used. `[link-download]` is for settings only this app has. Profiles are still kept in `%APPDATA%\Link Download\profiles.json`.

## Build

This crate targets `x86_64-pc-windows-gnu` (`rust-toolchain.toml`).

```bat
set RUSTFLAGS=-C target-feature=+crt-static
cargo build --release
copy target\release\link-download.exe dist\"Link Download.exe"
```

```bat
cargo test
```

## License

[MIT](LICENSE) for this program. Companion tools keep their own licenses; see [THIRD_PARTY.md](THIRD_PARTY.md).
