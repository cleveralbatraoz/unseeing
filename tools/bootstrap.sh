#!/bin/sh
# One command from a fresh clone to a working editor: install rustup if it's
# missing, build the Rust engine, let Godot import it, and prove every
# engine class actually registered before calling it done.
#
# macOS and Linux only. The gdextension's Windows keys are per-triple
# (game/unseeing.gdextension: windows.*.x86_64 / windows.*.arm64), so a
# single host-arch `cargo build --release` can never satisfy them the way
# it satisfies the macOS/Linux keys, which both point at the one
# host-native rust/target/release/ artifact. Windows authoring is
# documented below, not scripted.
#
# Env knobs: GODOT (binary), same override every other tool/ script honours.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

case "$(uname)" in
  Darwin | Linux) : ;;
  *)
    echo "bootstrap: this script covers macOS and Linux only (uname says $(uname))"
    echo "bootstrap: on Windows, build the engine yourself: cd rust && cargo build --release --features editor-docs --target x86_64-pc-windows-msvc"
    exit 2
    ;;
esac

echo "bootstrap: checking for rustup/cargo"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
if ! command -v cargo >/dev/null 2>&1; then
  echo "bootstrap: cargo not found — installing rustup (non-interactive)"
  # `|| true`: under set -eu a nonzero exit here (curl network failure,
  # rustup's own installer refusing an unsupported platform, a conflicting
  # partial install) would kill the script on this line with rustup's raw
  # exit code, skipping the diagnostic two lines below entirely. Let it
  # fall through so that check is what actually reports the failure.
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || true
  [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
  command -v cargo >/dev/null 2>&1 || {
    echo "bootstrap: FAILED rustup install did not leave a usable cargo on PATH"
    exit 2
  }
fi
echo "bootstrap: cargo OK ($(cargo --version)) — rust/rust-toolchain.toml pins 1.97.1 and its targets automatically"

echo "bootstrap: checking for a C linker"
command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1 || {
  echo "bootstrap: FAILED no C linker found (Rust needs one even for pure-Rust crates)"
  if [ "$(uname)" = "Darwin" ]; then
    echo "bootstrap: fix: xcode-select --install"
  else
    echo "bootstrap: fix: install build-essential (or your distro's C toolchain package)"
  fi
  exit 2
}
echo "bootstrap: C linker OK"

echo "bootstrap: building the engine (cargo build --release --features editor-docs)"
# editor-docs embeds every #[export] knob's /// comment as Inspector docs —
# exactly what a designer wants and nothing a shipped export should carry
# (register-docs is non-default for that reason: see rust/Cargo.toml). A
# later tools/export_macos.sh rebuilds this same path as a universal dylib,
# and any plain `cargo build --release` after that clobbers universal back
# to thin — irrelevant here, this build is for authoring, not for shipping.
(cd "$DIR/rust" && cargo build --release --features editor-docs) || {
  echo "bootstrap: FAILED rust build (see errors above)"
  exit 1
}
echo "bootstrap: engine built"

GODOT="${GODOT:-}"
echo "bootstrap: locating Godot${GODOT:+ (GODOT=$GODOT)}"
if [ -z "$GODOT" ]; then
  for g in godot "$HOME/bin/godot" /opt/homebrew/bin/godot; do
    if command -v "$g" >/dev/null 2>&1 || [ -x "$g" ]; then GODOT="$g"; break; fi
  done
fi
[ -n "$GODOT" ] || {
  echo "bootstrap: FAILED godot not found"
  echo "bootstrap: fix: brew install godot (macOS) or download 4.7.1.stable.official from godotengine.org and put it on PATH; then re-run, or set GODOT=/path/to/godot"
  exit 2
}
HAVE="$("$GODOT" --version 2>/dev/null | head -1)"
if [ -f "$DIR/.godot-version" ]; then
  WANT="$(cat "$DIR/.godot-version")"
  case "$HAVE" in
    "$WANT"*) : ;;
    *)
      echo "bootstrap: FAILED godot version '$HAVE' != pinned '$WANT'"
      echo "bootstrap: fix: install Godot $WANT (brew install godot, or godotengine.org), or set GODOT=/path/to/matching/binary"
      exit 2
      ;;
  esac
fi
echo "bootstrap: godot OK ($HAVE)"

# After the build, never before: the engine records a failed extension load
# in .godot/extension_list.cfg at import time, and a running editor never
# retries that — only a fresh import after the dylib exists will do.
echo "bootstrap: importing the project"
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true

echo "bootstrap: verifying every engine class registered"
"$GODOT" --headless --path "$DIR/game" -s res://tests/probe/engine_census_probe.gd || {
  echo "bootstrap: FAILED the engine census probe did not pass — see its output above"
  exit 1
}

echo "bootstrap: OK — open game/project.godot in Godot 4.7.1 and double-click scenes/level_01.tscn"
