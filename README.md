# Hyperbeam Share

[![CI](https://github.com/adamtty/hyperbeam-share/actions/workflows/ci.yml/badge.svg)](https://github.com/adamtty/hyperbeam-share/actions/workflows/ci.yml)
[![CodeQL](https://github.com/adamtty/hyperbeam-share/actions/workflows/codeql.yml/badge.svg)](https://github.com/adamtty/hyperbeam-share/actions/workflows/codeql.yml)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/adamtty/hyperbeam-share/badge)](https://scorecard.dev/viewer/?uri=github.com/adamtty/hyperbeam-share)
[![Release](https://img.shields.io/github/v/release/adamtty/hyperbeam-share)](https://github.com/adamtty/hyperbeam-share/releases/latest)

Remux a local video, extract subtitles, serve it locally, tunnel it out, hand you a URL for Hyperbeam.

Pick a video and (optionally) a subtitle track. The app remuxes it to a browser-friendly MP4
(video copied, audio converted to AAC if needed), extracts the subs (text tracks → WebVTT,
ASS/SSA kept and rendered with JASSUB), serves the folder on `127.0.0.1` and opens a Cloudflare
quick tunnel. Paste the `trycloudflare.com` URL into Hyperbeam. **Stop Sharing** closes the
tunnel and deletes the temp files (so does quitting the app).

## Install

Download the latest `Hyperbeam-Share-…-macos-arm64.zip` from
[Releases](https://github.com/adamtty/hyperbeam-share/releases/latest) (Apple Silicon, macOS 11+),
unzip it and move **Hyperbeam Share** to Applications.

Releases aren't signed with an Apple Developer ID yet, so macOS blocks the app the first time you open
it. Try to open it once, then go to **System Settings → Privacy & Security**, scroll down and click
**Open Anyway**. (Or run `xattr -dr com.apple.quarantine "/Applications/Hyperbeam Share.app"`.) The
checks below confirm it's the build this repository published.

To check a download: `shasum -a 256 -c SHA256SUMS` in the folder with the release files, and
`gh attestation verify Hyperbeam-Share-…-macos-arm64.zip --repo adamtty/hyperbeam-share`, which proves
it was built by this repository's release workflow from the tagged commit.

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

- **ffmpeg / ffprobe:** build them the way releases do, from FFmpeg's signed source, with
  `scripts/build-ffmpeg.sh` (about a minute on Apple Silicon, LGPL only, no extra libraries), then
  copy them in:

  ```sh
  scripts/build-ffmpeg.sh /tmp/ffmpeg-build
  cp /tmp/ffmpeg-build/ffmpeg src-tauri/binaries/ffmpeg-aarch64-apple-darwin
  cp /tmp/ffmpeg-build/ffprobe src-tauri/binaries/ffprobe-aarch64-apple-darwin
  ```

  Any other ffmpeg works too, but a GPL build changes the license terms of an app you distribute.
- **cloudflared:** Cloudflare's own release from https://github.com/cloudflare/cloudflared/releases
  (`cloudflared-darwin-arm64.tgz`), or `brew install cloudflared`.

## Build

```sh
scripts/build-app.sh   # → src-tauri/target/release/bundle/macos/Hyperbeam Share.app
```

That's `cargo tauri build --bundles app -- --locked` with the build machine's paths remapped (see
[Reproducible builds](#reproducible-builds)). Plain `cargo tauri build` in `src-tauri` works too.

## Releases

Pushing a tag such as `v0.2.0` (matching the version in `tauri.conf.json` and `Cargo.toml`) runs the
release workflow, which:

1. builds ffmpeg/ffprobe from FFmpeg's source tarball, checking its hash and FFmpeg's signature, and
   refuses anything but an LGPL build;
2. downloads Cloudflare's `cloudflared`, checking its hash and that Cloudflare signed it;
3. builds the app with a pinned Rust toolchain and Tauri CLI, signs it, and notarises it when the Apple
   secrets are configured (see below), otherwise ad-hoc signs it;
4. publishes a GitHub release with the zipped app, `SHA256SUMS`, a build-provenance attestation, the
   FFmpeg source it was built from (for the LGPL), and `THIRD_PARTY_NOTICES.md`.

Each release also attaches `BUILDINFO`: the commit, and the Xcode, macOS SDK, Rust, Tauri CLI,
cloudflared and runner image it was built with.

Signing secrets live in the repository's `release` environment, which only `v*` tags can use:
`APPLE_CERTIFICATE` (base64 `.p12` of a Developer ID Application certificate),
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, and for notarisation an App Store Connect API
key: `APPLE_API_ISSUER`, `APPLE_API_KEY` (the key ID) and `APPLE_API_KEY_P8` (the key file's contents).

## Reproducible builds

The provenance attestation proves a release was built by this repository's workflow. A rebuild proves
what's in it: anyone can build the tagged source themselves and check that every byte matches the
published app, without trusting GitHub or this repository's workflow.

```sh
scripts/verify-release.sh v0.2.0
```

It downloads the release, checks its hashes and provenance, rebuilds ffmpeg from FFmpeg's signed
source, fetches Cloudflare's `cloudflared`, builds the app with the Rust and Tauri CLI versions the
release workflow pins, and compares the two bundles file by file. It needs an Apple Silicon Mac with
the Xcode version named in the release's `BUILDINFO`, because Xcode's linker and SDK are recorded in
the binary; older versions are at [developer.apple.com/download](https://developer.apple.com/download/all/).
It takes about 15 minutes.

What makes this work:

- **No build paths in the binary:** `scripts/build-app.sh` remaps the checkout to `/build` and the
  cargo home to `/cargo`, so the folder you build in and your user name don't matter.
- **Pinned inputs:** `Cargo.lock`, the Rust toolchain, the Tauri CLI, FFmpeg's source tarball (by
  hash and signature), cloudflared's version and Cloudflare's signature, and the Xcode version.
- **Signatures set aside:** a code signature can't be reproduced (a Developer ID signature records a
  certificate and a time; even an ad-hoc one depends on the `codesign` version), so executables are
  compared without theirs (`scripts/unsigned-hash.py`) and `_CodeSignature/CodeResources` is skipped.
  So is each executable's build UUID, which Apple's linker doesn't always produce the same for the same
  output. Everything else must match exactly.

Releases up to v0.1.1 predate this (no `BUILDINFO`, no path remapping) and can't be checked this way.

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
- Every binary in the release is signed with the hardened runtime. CI runs `rustfmt`, `clippy` (warnings
  are errors), the tests and `zizmor`; CodeQL scans the Rust, the JavaScript and the workflows. Actions
  are pinned to commit SHAs on StepSecurity Harden-Runner, and Renovate keeps them and the dependencies
  current. Found a problem? See [SECURITY.md](SECURITY.md).

## History

The original source was lost; this project was rebuilt in Oct 2026 from the 0.1.0 app bundle.
The UI, player page and JASSUB assets were extracted byte-for-byte; `main.rs` was rewritten
from the commands, ffmpeg/cloudflared arguments and messages left in the binary.

## License

[MIT](LICENSE). Bundled third-party files (JASSUB and the libraries in its WebAssembly, and the
Liberation Sans font) keep their own licenses: see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
