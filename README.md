# Link Download

Windows 98–style GUI for [yt-dlp](https://github.com/yt-dlp/yt-dlp). Download videos or audio, remux with FFmpeg, edit names and tags, then write files to a folder you pick.

A built copy is [`Link.Download.exe`](https://github.com/oiuy-retro/link-download/releases/latest/download/Link.Download.exe) on the [latest release](https://github.com/oiuy-retro/link-download/releases/latest). Put [`yt-dlp.exe`](https://github.com/yt-dlp/yt-dlp/releases), [`ffmpeg.exe`](https://www.gyan.dev/ffmpeg/builds/), [`ffprobe.exe`](https://www.gyan.dev/ffmpeg/builds/) (it ships in the same FFmpeg build as `ffmpeg.exe`), and [`node.exe`](https://nodejs.org/) in the same folder as the program.

## What it does

Wizard pages: Options, Links, Names, Titles, Metadata, Download.

- Profiles in `%AppData%\Link Download\profiles.json`.
- Audio-only Opus in `.m4a`, or video (default 1080p H.264 AAC MP4).
- **Strip genre and description** is on by default so YouTube categories and the video description are not stored as tags.
- **SponsorBlock** (off by default) cuts sponsor, self-promotion, and interaction segments through yt-dlp. Cuts are stream-copy; they land on keyframes.
- Cover art: Matroska attachment `cover.jpg`, or an `attached_pic` still on MP4/M4A.
- Finished rows leave the list. Speed, items per second, files left, time left, and elapsed match the other desktop tools.

The window uses owner-drawn Windows 98 chrome. The folder picker is the real Explorer dialog.

## Run

1. Copy `Link.Download.exe` from the [latest release](https://github.com/oiuy-retro/link-download/releases/latest) to a folder.
2. Place these files beside it:
   - [yt-dlp](https://github.com/yt-dlp/yt-dlp/releases) `yt-dlp.exe`
   - [FFmpeg](https://www.gyan.dev/ffmpeg/builds/) `ffmpeg.exe` and `ffprobe.exe` (both ship in the same build)
   - [Node.js](https://nodejs.org/) `node.exe` (yt-dlp uses it for YouTube player JS)
3. Start `Link Download.exe` from that folder.

Windows 10 or later.

Tested with yt-dlp `2026.10.10.233141`, FFmpeg `2026-09-10-git-fd7c73d01e-full_build` (Gyan), and Node `v24.16.0`.

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
