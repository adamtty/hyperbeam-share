#!/usr/bin/env bash
# Rebuilds a published release from source and checks it matches, file by file.
#   scripts/verify-release.sh v0.2.0
#
# Works for releases that attach a BUILDINFO file and build through scripts/build-app.sh (those after v0.1.1).
#
# Run it from a clone of this repository on an Apple Silicon Mac, with the Xcode version the release was built with
# (BUILDINFO, attached to the release, says which; older Xcode versions are at developer.apple.com/download). It needs
# git, curl, gh, gpg (optional) and rustup, and takes about 15 minutes. With RELEASE_DIR=<folder> it uses release files
# you've already downloaded instead (set CHECK_PROVENANCE=1 to still check the attestation).
#
# Code signatures can't match: a Developer ID signature records a certificate and a time, and even an ad-hoc signature
# depends on the signing tool's version. So executables are compared without their signatures
# (scripts/unsigned-hash.py), and the bundle's signature file (_CodeSignature/CodeResources) is skipped. Every other
# byte has to match.
set -euo pipefail
tag=${1:?usage: scripts/verify-release.sh <tag, e.g. v0.2.0>}
repo=${REPO:-adamtty/hyperbeam-share}
here=$(cd "$(dirname "$0")/.." && pwd -P)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
say() { printf '\n== %s\n' "$*"; }

mkdir "$work/release"
if [ -n "${RELEASE_DIR:-}" ]; then
  # files already downloaded (the zip, SHA256SUMS and BUILDINFO)
  say "Release files from $RELEASE_DIR"
  cp "$RELEASE_DIR"/*.zip "$RELEASE_DIR/SHA256SUMS" "$RELEASE_DIR/BUILDINFO" "$work/release/"
else
  say "Downloading $tag"
  (cd "$work/release" && gh release download "$tag" --repo "$repo" --pattern '*.zip' --pattern SHA256SUMS --pattern BUILDINFO)
fi
(cd "$work/release" && shasum -a 256 -c SHA256SUMS --ignore-missing)
zip=$(cd "$work/release" && ls -- *.zip)
if [ -z "${RELEASE_DIR:-}" ] || [ -n "${CHECK_PROVENANCE:-}" ]; then
  gh attestation verify "$work/release/$zip" --repo "$repo" >/dev/null
  echo "provenance: built by $repo's release workflow"
fi

want_commit=$(sed -n 's/^commit: //p' "$work/release/BUILDINFO")
have_commit=$(git -C "$here" rev-parse "$tag^{commit}")
if [ "$want_commit" != "$have_commit" ]; then
  echo "BUILDINFO says $tag was built from $want_commit, but $tag here is $have_commit" >&2; exit 1
fi

say "Toolchain"
want_xcode=$(sed -n 's/^xcode: //p' "$work/release/BUILDINFO")
have_xcode=$(xcodebuild -version | tr '\n' ' ' | sed 's/ *$//')
echo "release: $want_xcode"
echo "this Mac: $have_xcode"
if [ "$want_xcode" != "$have_xcode" ]; then
  echo "Xcode differs, so the app's own binary won't match (its linker and SDK are recorded in it)." >&2
  echo "Select the release's version (sudo xcode-select -s /Applications/Xcode_<version>.app) and run again." >&2
  exit 1
fi

say "Source at $tag"
mkdir "$work/src"
git -C "$here" archive "$tag" | tar -x -C "$work/src"
src=$work/src
env_of() { sed -n "s/^  $1: //p" "$src/.github/workflows/release.yml" | head -1; }
rust=$(env_of RUST_VERSION); tauri=$(env_of TAURI_CLI_VERSION)
cf=$(env_of CLOUDFLARED_VERSION); team=$(env_of CLOUDFLARED_TEAM)
echo "rust $rust, tauri-cli $tauri, cloudflared $cf"
rustup toolchain install "$rust" --profile minimal >/dev/null
export RUSTUP_TOOLCHAIN=$rust
cargo install tauri-cli --version "$tauri" --locked --root "$work/tools" >/dev/null 2>&1
export PATH="$work/tools/bin:$PATH"

say "ffmpeg (from FFmpeg's signed source)"
"$src/scripts/build-ffmpeg.sh" "$work/ffmpeg" >"$work/ffmpeg.log" 2>&1 || { tail -20 "$work/ffmpeg.log"; exit 1; }
mkdir -p "$src/src-tauri/binaries"
cp "$work/ffmpeg/ffmpeg" "$src/src-tauri/binaries/ffmpeg-aarch64-apple-darwin"
cp "$work/ffmpeg/ffprobe" "$src/src-tauri/binaries/ffprobe-aarch64-apple-darwin"

say "cloudflared $cf (Cloudflare's release, Apple signature checked)"
(cd "$work" && curl -fsSLO "https://github.com/cloudflare/cloudflared/releases/download/$cf/cloudflared-darwin-arm64.tgz" \
  && tar -xzf cloudflared-darwin-arm64.tgz)
codesign --verify --strict \
  -R="anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] and certificate leaf[field.1.2.840.113635.100.6.1.13] and certificate leaf[subject.OU] = \"$team\"" \
  "$work/cloudflared"
cp "$work/cloudflared" "$src/src-tauri/binaries/cloudflared-aarch64-apple-darwin"

say "Building the app"
APPLE_SIGNING_IDENTITY=- "$src/scripts/build-app.sh" >"$work/build.log" 2>&1 || { tail -20 "$work/build.log"; exit 1; }

say "Comparing"
mkdir "$work/published"
ditto -x -k "$work/release/$zip" "$work/published"
mine="$src/src-tauri/target/release/bundle/macos/Hyperbeam Share.app"
theirs="$work/published/Hyperbeam Share.app"
list() { (cd "$1" && find . -type f ! -path './Contents/_CodeSignature/*' | LC_ALL=C sort); }
if ! diff <(list "$theirs") <(list "$mine") >/dev/null; then
  echo "different files in the bundle:"; diff <(list "$theirs") <(list "$mine") | sed 's/^/  /'; exit 1
fi
bad=0
while IFS= read -r f; do
  a=$(python3 -I "$here/scripts/unsigned-hash.py" "$theirs/$f" | cut -d' ' -f1)
  b=$(python3 -I "$here/scripts/unsigned-hash.py" "$mine/$f" | cut -d' ' -f1)
  if [ "$a" = "$b" ]; then echo "  same     $f"; else echo "  DIFFERS  $f"; bad=1; fi
done < <(list "$theirs")
if [ "$bad" = 0 ]; then
  say "$tag is reproducible: every file matches the source at $tag (signatures aside)"
else
  say "$tag does NOT match this rebuild"; exit 1
fi
