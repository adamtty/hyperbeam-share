#!/usr/bin/env bash
# Builds the app bundle (src-tauri/target/release/bundle/macos/Hyperbeam Share.app). The release workflow runs this,
# and so should anyone rebuilding a release, so both use exactly the same steps.
#
# Paths are remapped so the binary doesn't record where it was built: the checkout becomes /build and the cargo home
# (where crate sources live) becomes /cargo. Without this, panic messages and debug info embed paths like
# /Users/<name>/.cargo/registry/..., and a rebuild by anyone else, in any other folder, would differ.
#
# Extra arguments go to `cargo tauri build`. APPLE_SIGNING_IDENTITY and the other Tauri signing variables pass through.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd -P)
cargo_home=${CARGO_HOME:-$HOME/.cargo}
mkdir -p "$cargo_home" # so the path below resolves on a first build too
flags=("--remap-path-prefix=$cargo_home=/cargo")
# also the path with symlinks resolved, in case that's the one cargo passes to rustc. Never an empty prefix: it would
# match every path, and as the later flag it would win.
real_home=$(cd "$cargo_home" && pwd -P)
if [ -n "$real_home" ] && [ "$real_home" != "$cargo_home" ]; then flags+=("--remap-path-prefix=$real_home=/cargo"); fi
flags+=("--remap-path-prefix=$root=/build")
# the encoded form (separated by 0x1f) so a path with spaces stays one argument
CARGO_ENCODED_RUSTFLAGS=$(IFS=$'\x1f'; printf '%s' "${flags[*]}")
export CARGO_ENCODED_RUSTFLAGS
unset RUSTFLAGS
cd "$root/src-tauri"
exec cargo tauri build --bundles app "$@" -- --locked
