# Security policy

## Reporting a vulnerability

Please report security issues **privately** through GitHub:
[**Report a vulnerability**](https://github.com/adamtty/hyperbeam-share/security/advisories/new).
Don't open a public issue for them.

Include what you found, how to reproduce it, and what an attacker could do with it. You'll get a
reply within a week. Once it's fixed, you'll be credited in the advisory unless you'd rather not be.

## Supported versions

Only the latest release, and the latest commit on `main`, are supported.

## Scope

In scope: the app (`src/`, `src-tauri/`), the player page it serves, `scripts/build-ffmpeg.sh` and the
release workflow. Especially interesting: anything that exposes more than the one video you chose to
share, lets the page or a video's metadata run code in the app, keeps a share reachable after you
stop it, or lets someone tamper with a release.

Out of scope: vulnerabilities in FFmpeg, cloudflared, Cloudflare's tunnel service or Hyperbeam itself
(report those upstream), and the fact that anyone with a share's URL can watch it while it's running
(that's how sharing works; see the README).
