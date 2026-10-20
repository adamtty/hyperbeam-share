# Hyperbeam Share

Remux a local video, extract subtitles, serve it locally, tunnel it out, hand you a URL for Hyperbeam.

Pick a video and (optionally) a subtitle track. The app remuxes it to a browser-friendly MP4
(video copied, audio converted to AAC if needed), extracts the subs (text tracks → WebVTT,
ASS/SSA kept and rendered with JASSUB), serves the folder on `127.0.0.1` and opens a Cloudflare
quick tunnel. Paste the `trycloudflare.com` URL into Hyperbeam. **Stop Sharing** closes the
tunnel and deletes the temp files (so does quitting the app).

## Who can see what you share

While a share is running, **anyone who has the `trycloudflare.com` URL can watch and download the
video and its subtitles**. There's no login or password: the URL is the only secret. Its subdomain is
random and unguessable, but it's visible to everyone in your Hyperbeam session (it's in the shared
browser's address bar), and to anyone they pass it to.

So share only what you'd be happy for the whole room to have a copy of, and press **Stop Sharing**
when you're done: the URL stops working at once and the temp copy is deleted.

**If the app crashes** instead of quitting normally, the `cloudflared` tunnel can keep running, so the
video stays reachable at that URL, and the temp files stay in your temp folder (`hb-share-…`). To end
it, stop the app's own `cloudflared` (it runs from inside the app, so this leaves any other tunnels
alone) or restart your Mac:

```sh
pkill -f 'Hyperbeam Share.app/Contents/MacOS/cloudflared'
```

## Layout

- `src/`: app window UI (plain HTML/JS, uses `withGlobalTauri`)
- `src-tauri/src/main.rs`: commands `probe_subtitles`, `prepare_and_share`, `stop_sharing`
- `src-tauri/assets/player.html`: page served to Hyperbeam (`__SUB_KIND__` is filled in at share time)
- `src-tauri/assets/jassub/`: JASSUB renderer files, embedded into the binary
- `src-tauri/binaries/`: sidecars (git-ignored, see below)

## Sidecars

Tauri needs `ffmpeg`, `ffprobe` and `cloudflared` in `src-tauri/binaries/` with the target triple
suffix, e.g. `ffmpeg-aarch64-apple-darwin`.

- **ffmpeg / ffprobe:** get them from the official download page, https://ffmpeg.org/download.html,
  which links the builds the FFmpeg project recommends for macOS. Whatever you use, check the build's
  checksum or signature where one is published, since these run on every file you share.
- **cloudflared:** `brew install cloudflared`, or the official releases at
  https://github.com/cloudflare/cloudflared/releases.

## Build

```sh
cd src-tauri
cargo tauri build   # → target/release/bundle/macos/Hyperbeam Share.app
```

## Security notes

- The local server listens on `127.0.0.1` only, on a random port, and serves just the share's own
  temp folder: no directory listings, and `..` paths are rejected.
- Each share gets a new temp folder with an unguessable name, readable only by you, and creation
  fails if the path already exists, so another user can't plant it on a shared `/tmp`.
- `ffmpeg`, `ffprobe` and `cloudflared` are run with argument lists, never through a shell. The video
  path only ever comes from the native file dialog.
- The app window has a Content Security Policy and only the default and dialog permissions; the
  shell plugin is used from Rust, so the page can't start processes.
- The player page is a fixed template, and nothing from the video's metadata is inserted into it.

## History

The original source was lost; this project was rebuilt in Oct 2026 from the 0.1.0 app bundle.
The UI, player page and JASSUB assets were extracted byte-for-byte; `main.rs` was rewritten
from the commands, ffmpeg/cloudflared arguments and messages left in the binary.

## License

[MIT](LICENSE). Bundled third-party files (JASSUB and the libraries in its WebAssembly, and the
Liberation Sans font) keep their own licenses: see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
