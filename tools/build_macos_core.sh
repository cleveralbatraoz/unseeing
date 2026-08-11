#!/bin/sh
# Build the wave core as a UNIVERSAL macOS dylib — one file carrying both
# arm64 and x86_64 — at the single path game/unseeing.gdextension names for
# both macOS keys.
#
# Why this exists at all: `cargo build --release` builds for the host and
# nothing else, so on an Apple Silicon laptop it produces an arm64-only
# extension. game/export_presets.cfg declares the macOS preset
# binary_format/architecture="universal". Those two facts shipped together
# mean an Intel Mac downloads a bundle that promises to run and then cannot
# load the extension at all.
#
# The clobber trap, stated plainly: a later plain `cargo build --release`
# writes to target/release/ and silently replaces the universal file with a
# thin one. Nothing here can prevent that — so this script is cheap to re-run
# instead. The two per-slice builds land in target/<triple>/release/, which
# a host build never touches, so restoring the universal core after a clobber
# is one `lipo` over cached artifacts rather than a recompile. Everything that
# ships a macOS build runs this first, every time, and never trusts a file it
# finds already sitting at the path.
#
# Env knobs: none. Deliberately — an env switch here is a way to skip the
# thing this script is for.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

[ "$(uname)" = "Darwin" ] || {
  echo "build-macos-core: this builds Apple slices and needs macOS (uname says $(uname))"
  exit 2
}
command -v lipo >/dev/null 2>&1 || {
  echo "build-macos-core: lipo not found (install the Xcode command line tools)"
  exit 2
}
if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
command -v cargo >/dev/null 2>&1 || {
  echo "build-macos-core: cargo not found (install rustup; rust-toolchain.toml pins the version)"
  exit 2
}

# Both triples are already listed in rust/rust-toolchain.toml, so a rustup
# that honours the pin has them. Say so precisely when it does not, rather
# than letting cargo's own error explain a project rule it has never read.
for triple in aarch64-apple-darwin x86_64-apple-darwin; do
  if ! rustup target list --installed 2>/dev/null | grep -qx "$triple"; then
    echo "build-macos-core: FAILED rust target $triple is not installed"
    echo "build-macos-core:        rustup target add $triple (rust-toolchain.toml already lists it)"
    exit 2
  fi
done

SLICES=""
for triple in aarch64-apple-darwin x86_64-apple-darwin; do
  echo "build-macos-core: cargo build --release --target $triple"
  (cd "$DIR/rust" && cargo build --release --target "$triple") || {
    echo "build-macos-core: FAILED cargo build for $triple"
    exit 1
  }
  slice="$DIR/rust/target/$triple/release/libunseeing_core.dylib"
  [ -f "$slice" ] || {
    echo "build-macos-core: FAILED cargo reported success but $slice is not there"
    echo "build-macos-core:        (a CARGO_TARGET_DIR in the environment moves it out of reach)"
    exit 1
  }
  SLICES="${SLICES:+$SLICES }$slice"
done

# The path game/unseeing.gdextension names for macos.debug AND macos.release.
# One binary for the editor, the headless checks and the export — deliberately,
# so what the gate loads is what ships.
CORE="$DIR/rust/target/release/libunseeing_core.dylib"

# Written beside the target and moved into place, never straight over it: a
# killed run must not leave a truncated dylib at the one path every macOS key
# resolves to. Same directory, so the rename is atomic.
# shellcheck disable=SC2086
lipo -create -output "$CORE.new" $SLICES || {
  rm -f "$CORE.new"
  echo "build-macos-core: FAILED lipo could not fuse the slices"
  exit 1
}
mv -f "$CORE.new" "$CORE"

# Ask the bytes, not the build. cargo can succeed, lipo can succeed, and the
# file at this path still be the wrong one — this is the only statement about
# it that is made by reading it.
"$DIR/tools/check_universal.sh" "$CORE"
