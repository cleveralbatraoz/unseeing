#!/bin/sh
# One command from a fresh clone to a working editor: install rustup if it's
# missing, build the Rust engine, let Godot import it, and prove every
# engine class actually registered before calling it done.
#
# This is the native macOS/Linux entry point. Windows uses
# tools\bootstrap.cmd, which selects the target-specific DLL path declared by
# game/unseeing.gdextension and provides this same build/import/census contract.
#
# Env knobs: GODOT (binary), same override every other tool/ script honours.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

case "$(uname)" in
  Darwin | Linux) : ;;
  *)
    echo "bootstrap: this entry point covers macOS and Linux (uname says $(uname))"
    printf '%s\n' 'bootstrap: on Windows run tools\bootstrap.cmd'
    exit 2
    ;;
esac

# The engine gate runs FIRST. It costs milliseconds, needs nothing the build
# produces, and a wrong or missing editor is the single most likely reason a
# fresh machine cannot bootstrap — so paying for a full release build before
# saying so is pure waste. tools/bootstrap.ps1 has always checked Godot first;
# this is the POSIX side agreeing with it.
echo "bootstrap: locating Godot${GODOT:+ (GODOT=$GODOT)}"
# shellcheck source=tools/lib/engine.sh
. "$DIR/tools/lib/engine.sh"
WANT="$(unseeing_engine_pin "$DIR")" || {
  echo "bootstrap: FAILED .godot-version does not name a Godot release"
  echo "bootstrap: fix: restore it from the repository; it is what every tool here pins against"
  exit 2
}
GODOT="$(unseeing_engine_select "$DIR" "${GODOT:-}")" || {
  echo "bootstrap: FAILED no Godot $WANT found"
  echo "bootstrap: fix: install Godot $WANT (brew install godot, scoop install godot, or godotengine.org),"
  echo "bootstrap: fix: leave it on PATH under any of its usual names, or set GODOT=/path/to/godot"
  exit 2
}
echo "bootstrap: godot OK ($(unseeing_engine_version "$GODOT"))"

echo "bootstrap: checking for rustup/cargo"
# rustup-init installs into CARGO_HOME when it is set — Docker's rust images and
# many CI images set it away from $HOME. Looking only at $HOME/.cargo reported a
# perfectly successful install as a failed one, and advised reopening a terminal
# that could never have helped.
CARGO_DIR="${CARGO_HOME:-${HOME:-}/.cargo}"
[ -f "$CARGO_DIR/env" ] && . "$CARGO_DIR/env"
RUSTUP="${UNSEEING_BOOTSTRAP_RUSTUP:-}"
if [ -z "$RUSTUP" ] && command -v rustup >/dev/null 2>&1; then
  RUSTUP="$(command -v rustup)"
fi
if [ -z "$RUSTUP" ]; then
  echo "bootstrap: rustup not found — installing it non-interactively"
  # `|| true`: under set -eu a nonzero exit here (curl network failure,
  # rustup's own installer refusing an unsupported platform, a conflicting
  # partial install) would kill the script on this line with rustup's raw
  # exit code, skipping the diagnostic two lines below entirely. Let it
  # fall through so that check is what actually reports the failure.
  if [ -n "${UNSEEING_BOOTSTRAP_INSTALL_RUSTUP:-}" ]; then
    "$UNSEEING_BOOTSTRAP_INSTALL_RUSTUP" || true
  else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || true
  fi
  [ -f "$CARGO_DIR/env" ] && . "$CARGO_DIR/env"
  [ -x "$CARGO_DIR/bin/rustup" ] && PATH="$CARGO_DIR/bin:$PATH"
  command -v rustup >/dev/null 2>&1 && RUSTUP="$(command -v rustup)"
  [ -n "$RUSTUP" ] || {
    echo "bootstrap: FAILED the install did not leave a usable rustup on PATH"
    echo "bootstrap: check the rustup/curl output above for why, then either:"
    echo "bootstrap: fix: install rustup yourself from https://rustup.rs, run . \"$CARGO_DIR/env\" (or reopen your terminal), and re-run tools/bootstrap.sh"
    exit 2
  }
fi
PIN="$(sed -n 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$DIR/rust/rust-toolchain.toml" | head -1)"
[ -n "$PIN" ] || {
  echo "bootstrap: FAILED rust/rust-toolchain.toml carries no channel pin"
  exit 2
}
if RUSTC_HAVE="$(cd "$DIR/rust" && "$RUSTUP" run "$PIN" rustc --version 2>/dev/null)"; then
  :
else
  echo "bootstrap: installing pinned Rust $PIN toolchain"
  "$RUSTUP" toolchain install "$PIN" --profile minimal || {
    echo "bootstrap: FAILED pinned Rust $PIN toolchain install failed"
    exit 2
  }
  RUSTC_HAVE="$(cd "$DIR/rust" && "$RUSTUP" run "$PIN" rustc --version 2>/dev/null)" || {
    echo "bootstrap: FAILED rustup could not select the pinned Rust $PIN toolchain"
    exit 2
  }
fi
case "$RUSTC_HAVE" in
  "rustc $PIN "*) : ;;
  *)
    echo "bootstrap: FAILED rustc version '$RUSTC_HAVE' != pinned '$PIN'"
    exit 2
    ;;
esac
CARGO_HAVE="$(cd "$DIR/rust" && "$RUSTUP" run "$PIN" cargo --version 2>/dev/null)" || {
  echo "bootstrap: FAILED cargo is unavailable in the pinned Rust $PIN toolchain"
  exit 2
}
echo "bootstrap: rustup/cargo OK ($RUSTC_HAVE; $CARGO_HAVE)"

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
case "$(uname)" in
  Darwin) ARTIFACT="$DIR/rust/target/release/libunseeing_core.dylib" ;;
  Linux) ARTIFACT="$DIR/rust/target/release/libunseeing_core.so" ;;
esac
rm -f "$ARTIFACT" || {
  echo "bootstrap: FAILED cannot remove stale $ARTIFACT; close Godot and retry"
  exit 1
}
(cd "$DIR/rust" && "$RUSTUP" run "$PIN" cargo build --release \
  --features editor-docs --target-dir "$DIR/rust/target") || {
  echo "bootstrap: FAILED rust build (see errors above)"
  exit 1
}
[ -f "$ARTIFACT" ] || {
  echo "bootstrap: FAILED cargo exited 0 but did not recreate $ARTIFACT"
  echo "bootstrap: fix: inspect the Cargo output above, then retry"
  exit 1
}
echo "bootstrap: engine built ($ARTIFACT)"

# After the build, never before: the engine records a failed extension load
# in .godot/extension_list.cfg at import time, and a running editor never
# retries that — only a fresh import after the dylib exists will do.
echo "bootstrap: importing the project"
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true

echo "bootstrap: verifying every engine class registered"
if CENSUS="$("$GODOT" --headless --path "$DIR/game" \
  -s res://tests/probe/engine_census_probe.gd 2>&1)"; then
  :
else
  status=$?
  printf '%s\n' "$CENSUS"
  echo "bootstrap: FAILED the engine census probe did not pass (exit $status) — see its output above"
  exit 1
fi
printf '%s\n' "$CENSUS"
case "$CENSUS" in
  *"probe: PASS (19 checks)"*) : ;;
  *)
    echo "bootstrap: FAILED the engine census returned success without the exact 19-class verdict"
    exit 1
    ;;
esac

echo "bootstrap: OK — open game/project.godot in Godot $WANT and double-click scenes/level_01.tscn"
