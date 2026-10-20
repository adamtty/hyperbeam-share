# Third-party notices

Hyperbeam Share is MIT-licensed (see [LICENSE](LICENSE)). It bundles, and sends to viewers alongside
the player page, the following third-party files.

## JASSUB 2.5.15

`src-tauri/assets/jassub/` holds the [JASSUB](https://github.com/ThaUnknown/jassub) subtitle renderer:
`jassub-worker.wasm`, `jassub-worker-modern.wasm` and `default.woff2` are byte-identical to the
`jassub@2.5.15` npm release, and `jassub.js` / `jassub-worker.js` are JASSUB's JavaScript.

- **JASSUB**: MIT License, © 2021–2022 JASSUB contributors and © 2017–2021 JavascriptSubtitlesOctopus
  contributors. Full text in [`src-tauri/assets/jassub/LICENSE`](src-tauri/assets/jassub/LICENSE).

The WebAssembly files are JASSUB's build of these libraries, each under its own license (source and
full license texts at the links):

| Library | License |
|---|---|
| [libass](https://github.com/libass/libass) | ISC |
| [FriBidi](https://github.com/fribidi/fribidi) | LGPL-2.1-or-later |
| [FreeType](https://gitlab.freedesktop.org/freetype/freetype) | FreeType License (FTL), or GPL-2.0 |
| [HarfBuzz](https://github.com/harfbuzz/harfbuzz) | "Old MIT" (MIT-style) |
| [Brotli](https://github.com/google/brotli) | MIT |

FriBidi is LGPL: its source is at the link above, and JASSUB's build scripts (in its repository) show
how the WebAssembly is produced, so it can be rebuilt with a modified FriBidi.

## Liberation Sans 2.00.5 (`default.woff2`)

The default subtitle font, © 2010 Google Corporation and © 2012 Red Hat, Inc., with Reserved Font
Names Arimo, Tinos, Cousine and Liberation. Licensed under the SIL Open Font License 1.1; full text in
[`src-tauri/assets/jassub/default.woff2.OFL.txt`](src-tauri/assets/jassub/default.woff2.OFL.txt).
"Liberation" is a trademark of Red Hat, Inc.

## Bundled in releases, not stored in this repository

`ffmpeg`, `ffprobe` and `cloudflared` live in `src-tauri/binaries/` (git-ignored) and are bundled into
the built app:

- **FFmpeg 9.0.1** (`ffmpeg`, `ffprobe`): built by the release workflow from FFmpeg's signed source
  tarball with `scripts/build-ffmpeg.sh`, with no GPL or non-free parts and no third-party libraries,
  so it's licensed **LGPL-2.1-or-later**. Every release attaches the exact source tarball it was built
  from, and the build script shows how, so you can rebuild or modify it. https://ffmpeg.org
- **cloudflared** (Cloudflare Tunnel client): Cloudflare's own signed release binary, **Apache-2.0**.
  https://github.com/cloudflare/cloudflared

If you build and distribute the app with different binaries (a GPL ffmpeg, for example), their license
terms apply to what you distribute.
