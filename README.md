# Hyperbeam Share

Remux a local video, extract subtitles, serve it locally, tunnel it out, hand you a URL for Hyperbeam.

Pick a video and (optionally) a subtitle track. The app remuxes it to a browser-friendly MP4
(video copied, audio converted to AAC if needed), extracts the subs (text tracks → WebVTT,
ASS/SSA kept and rendered with JASSUB), serves the folder on `127.0.0.1` and opens a Cloudflare
quick tunnel. Paste the `trycloudflare.com` URL into Hyperbeam. **Stop Sharing** closes the
tunnel and deletes the temp files (so does quitting the app).

## Layout

- `src/` – app window UI (plain HTML/JS, uses `withGlobalTauri`)
- `src-tauri/src/main.rs` – commands: `probe_subtitles`, `prepare_and_share`, `stop_sharing`
- `src-tauri/assets/player.html` – page served to Hyperbeam (`__SUB_KIND__` is filled in at share time)
- `src-tauri/assets/jassub/` – JASSUB renderer files, embedded into the binary
- `src-tauri/binaries/` – sidecars (git-ignored, see below)

## Sidecars

Tauri needs `ffmpeg`, `ffprobe` and `cloudflared` in `src-tauri/binaries/` with the target
triple suffix, e.g. `ffmpeg-aarch64-apple-darwin`. Static macOS builds of ffmpeg/ffprobe:
https://www.martin-riedl.de; cloudflared: `brew install cloudflared` or its GitHub releases.

## Build

```sh
cd src-tauri
cargo tauri build   # → target/release/bundle/macos/Hyperbeam Share.app
```

## History

The original source was lost; this project was rebuilt in Oct 2026 from the 0.1.0 app bundle.
The UI, player page and JASSUB assets were extracted byte-for-byte; `main.rs` was rewritten
from the commands, ffmpeg/cloudflared arguments and messages left in the binary.
