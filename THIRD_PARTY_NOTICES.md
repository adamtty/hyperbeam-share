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

## Not in this repository

`ffmpeg`, `ffprobe` and `cloudflared` are downloaded separately into `src-tauri/binaries/` (git-ignored)
and bundled into the built app. They have their own licenses: FFmpeg is LGPL-2.1-or-later or
GPL-2.0-or-later depending on how it was built, and cloudflared is Apache-2.0. If you distribute a
build, include their license notices too.
