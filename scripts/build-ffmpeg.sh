#!/bin/bash
# Build the ffmpeg and ffprobe that releases bundle: LGPL-2.1-or-later only, from FFmpeg's signed
# source tarball, with no third-party libraries and no network code (the app only reads local files;
# cloudflared does the networking). Everything the app asks of ffmpeg is native FFmpeg code: reading
# any container, copying video, encoding AAC, converting subtitles to WebVTT and writing MP4.
#
# Usage: scripts/build-ffmpeg.sh OUT_DIR
# Writes OUT_DIR/ffmpeg, OUT_DIR/ffprobe, and OUT_DIR/source/ (the exact source tarball and its
# signature, which releases attach to meet the LGPL).
set -euo pipefail

FFMPEG_VERSION=9.0.1
FFMPEG_SHA256=cf38e0e28c7e5605942c4a77755349b0145804a397af37eb1fb4c77cb237f635
# "FFmpeg release signing key <ffmpeg-devel@ffmpeg.org>"
FFMPEG_KEY=FCF986EA15E6E293A5644F10B4322F04D67658D8
MACOS_MIN=11.0

OUT=$(mkdir -p "$1" && cd "$1" && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

tarball=ffmpeg-$FFMPEG_VERSION.tar.xz
curl -fsSLO "https://ffmpeg.org/releases/$tarball"
curl -fsSLO "https://ffmpeg.org/releases/$tarball.asc"
echo "$FFMPEG_SHA256  $tarball" | shasum -a 256 -c -

if command -v gpg >/dev/null; then
  export GNUPGHOME="$WORK/gnupg"
  mkdir -m 700 "$GNUPGHOME"
  curl -fsSL https://ffmpeg.org/ffmpeg-devel.asc | gpg -q --import
  gpg --status-fd 1 --verify "$tarball.asc" "$tarball" 2>/dev/null | grep -q "VALIDSIG $FFMPEG_KEY " \
    || { echo "ffmpeg source signature check failed" >&2; exit 1; }
  echo "signature OK ($FFMPEG_KEY)"
fi

tar -xJf "$tarball"
cd "ffmpeg-$FFMPEG_VERSION"
# --disable-autodetect: nothing is linked just because it happens to be installed on the build machine
./configure \
  --disable-autodetect --enable-zlib --enable-bzlib --enable-iconv \
  --disable-network --disable-avdevice --disable-ffplay --disable-doc --disable-debug \
  --enable-pthreads --extra-libs=-liconv \
  --extra-cflags="-mmacosx-version-min=$MACOS_MIN" --extra-ldflags="-mmacosx-version-min=$MACOS_MIN"
make -j"$(sysctl -n hw.ncpu)" ffmpeg ffprobe

# refuse to ship anything but an LGPL build
# (the license text wraps, so join its lines before matching)
./ffmpeg -hide_banner -L | tr '\n' ' ' | grep -q "GNU Lesser General Public License" \
  || { echo "not an LGPL build" >&2; ./ffmpeg -hide_banner -L >&2; exit 1; }
if ./ffmpeg -hide_banner -buildconf | grep -qE -- "--enable-(gpl|nonfree)"; then
  echo "GPL or non-free components enabled" >&2; exit 1
fi

cp ffmpeg ffprobe "$OUT/"
mkdir -p "$OUT/source"
cp "$WORK/$tarball" "$WORK/$tarball.asc" "$OUT/source/"
"$OUT/ffmpeg" -hide_banner -version | head -1
