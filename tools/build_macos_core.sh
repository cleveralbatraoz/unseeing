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
CARGO_DIR="${CARGO_HOME:-${HOME:-}/.cargo}"
if [ -f "$CARGO_DIR/env" ]; then . "$CARGO_DIR/env"; fi
command -v cargo >/dev/null 2>&1 || {
  echo "build-macos-core: cargo not found (install rustup; rust-toolchain.toml pins the version)"
  exit 2
}
# The target gate below is a rustup question, not a cargo one. A Rust installed
# by Homebrew or a distro package has cargo and no rustup at all, and without
# this the gate answered "rust target aarch64-apple-darwin is not installed"
# and prescribed `rustup target add` — a command that host cannot run.
command -v rustup >/dev/null 2>&1 || {
  echo "build-macos-core: rustup not found, and the Apple slices need it to select the pinned toolchain"
  echo "build-macos-core:        fix: install rustup from https://rustup.rs (a package-manager cargo cannot honour rust-toolchain.toml)"
  exit 2
}

# Exactly two, named rather than accumulated into a list. A universal macOS
# binary IS arm64 plus x86_64 — nothing here is variadic — and two names carry
# the paths to lipo fully quoted, so a repo living under a directory with a
# space in it fuses correctly instead of splitting into four arguments.
ARM64_TRIPLE=aarch64-apple-darwin
X86_64_TRIPLE=x86_64-apple-darwin

# Both triples are already listed in rust/rust-toolchain.toml, so a rustup
# that honours the pin has them. Say so precisely when it does not, rather
# than letting cargo's own error explain a project rule it has never read.
#
# Asked from inside rust/, because that is where the pin lives. rustup resolves
# a toolchain from the CURRENT DIRECTORY, so run from the repository root — the
# ordinary way anyone invokes this — the question went to the default toolchain
# instead of the pinned one, while the build below runs in rust/ and uses the
# pinned one. On a machine whose default toolchain lacks the Apple targets that
# reported a failure the build would not have had, and the remedy it printed
# added the target to the wrong toolchain, so re-running never converged.
INSTALLED_TARGETS="$(cd "$DIR/rust" && rustup target list --installed 2>/dev/null)" || INSTALLED_TARGETS=""
for triple in "$ARM64_TRIPLE" "$X86_64_TRIPLE"; do
  if ! printf '%s\n' "$INSTALLED_TARGETS" | grep -qx "$triple"; then
    echo "build-macos-core: FAILED rust target $triple is not installed for the pinned toolchain"
    echo "build-macos-core:        fix: (cd rust && rustup target add $triple)"
    echo "build-macos-core:        rust-toolchain.toml already lists it; running this from rust/ is what aims it at the pin"
    exit 2
  fi
done

# Builds one slice and prints its path — the only way a path reaches lipo.
#
# The slice is DELETED before the build, which is the whole trick: afterwards
# its existence proves this run produced it, rather than proving only that
# something is sitting at the conventional path. Those are different claims and
# the gap between them ships stale binaries. `cargo` exiting 0 does not mean
# cargo wrote here — a CARGO_TARGET_DIR or --target-dir redirect, a `[build]
# target-dir` in a config file this repo cannot see, or a dropped `--target`
# all send the output elsewhere while succeeding — and target/ keeps slices
# from earlier checkouts indefinitely, so the leftovers are usually valid
# Mach-O that fuse into a perfectly well-formed universal binary. Nothing
# downstream can catch that: the artifact is not malformed, it is merely not
# this commit's. Cargo re-emits a deleted output for free.
#
# Same clean-then-produce shape export_macos.sh uses for game/build/macos.
# Everything conversational goes to stderr; stdout is the return value.
build_slice() { # build_slice <triple>
  _slice="$DIR/rust/target/$1/release/libunseeing_core.dylib"
  rm -f "$_slice"
  echo "build-macos-core: cargo build --release --target $1" >&2
  (cd "$DIR/rust" && cargo build --release --target "$1" >&2) || {
    echo "build-macos-core: FAILED cargo build for $1" >&2
    return 1
  }
  [ -f "$_slice" ] || {
    echo "build-macos-core: FAILED cargo exited 0 but $_slice is not there" >&2
    echo "build-macos-core:        cargo built something, somewhere else — a CARGO_TARGET_DIR," >&2
    echo "build-macos-core:        --target-dir or [build] target-dir is redirecting the output" >&2
    return 1
  }
  printf '%s' "$_slice"
}

ARM64_SLICE="$(build_slice "$ARM64_TRIPLE")" || exit 1
X86_64_SLICE="$(build_slice "$X86_64_TRIPLE")" || exit 1

# The path game/unseeing.gdextension names for macos.debug AND macos.release.
# One binary for the editor, the headless checks and the export — deliberately,
# so what the gate loads is what ships.
CORE="$DIR/rust/target/release/libunseeing_core.dylib"

# Written beside the target and moved into place, never straight over it: a
# killed run must not leave a truncated dylib at the one path every macOS key
# resolves to. Same directory, so the rename is atomic.
lipo -create -output "$CORE.new" "$ARM64_SLICE" "$X86_64_SLICE" || {
  rm -f "$CORE.new"
  echo "build-macos-core: FAILED lipo could not fuse the slices"
  exit 1
}
mv -f "$CORE.new" "$CORE"

# Ask the bytes, not the build. cargo can succeed, lipo can succeed, and the
# file at this path still be the wrong one — this is the only statement about
# it that is made by reading it.
"$DIR/tools/check_universal.sh" "$CORE"
