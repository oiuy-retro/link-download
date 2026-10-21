# Third-party software

Link Download links against or is meant to run next to the following projects. This repository ships the Link Download sources and a built `dist/Link Download.exe`. It does not ship yt-dlp, FFmpeg, or Node.js.

Tested with yt-dlp `2026.10.10.233141`, FFmpeg `2026-09-10-git-fd7c73d01e-full_build` (Gyan), and Node `v24.16.0`.

## Rust crates (linked into the exe)

| Project | License | Use |
| --- | --- | --- |
| [windows-rs](https://github.com/microsoft/windows-rs) | MIT OR Apache-2.0 | Win32 UI, WinHTTP |
| [serde_json](https://github.com/serde-rs/json) | MIT OR Apache-2.0 | JSON profiles and yt-dlp dumps |
| [winres](https://github.com/mxre/winres) | MIT | Icon and manifest at build time |
| [Rust](https://www.rust-lang.org/) / LLVM | MIT / Apache-2.0 / NCSA | Compiler |

## Required beside the exe

| Project | License | Notes |
| --- | --- | --- |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp) | Unlicense | Download, playlist dump, SponsorBlock cuts, metadata. Tested with `2026.10.10.233141`. |
| [FFmpeg](https://ffmpeg.org/) | LGPL / GPL, depending on the build | Remux, transcode, covers. A Gyan full build is GPL-3.0. Tested with `ffmpeg version 2026-09-10-git-fd7c73d01e-full_build` from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/). |
| [Node.js](https://nodejs.org/) | MIT | yt-dlp runs YouTube player JS with `node.exe`. Tested with `v24.16.0`. |

FFmpeg is a trademark of Fabrice Bellard / the FFmpeg project. Node.js is a trademark of the OpenJS Foundation.

## SponsorBlock

When that option is on, yt-dlp calls [SponsorBlock](https://sponsor.ajay.app/) (`sponsor`, `selfpromo`, `interaction` categories). SponsorBlock data is contributed by its users. See [SponsorBlock on GitHub](https://github.com/ajayyy/SponsorBlock).

## Windows

Common Controls and the Explorer folder picker are part of Windows. This program does not redistribute Windows.
