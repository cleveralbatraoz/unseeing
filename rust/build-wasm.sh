#!/bin/sh
# Build the wave core for the web export: wasm32-unknown-emscripten,
# SINGLE-THREADED (the export pins thread_support=false — no -pthread, no
# atomics, ever). Toolchain pins, and why they are exactly these:
#   nightly-2026-05-25 — last line with -Zemscripten-wasm-eh (removed ~June
#                        2026); -Cpanic=immediate-abort is the only alternative
#                        and turns every panic into an instant crash.
#   emsdk 4.0.20       — the version Godot 4.7's official web templates are
#                        built with (build-containers branch 4.7); a SIDE_MODULE
#                        must match the main module's emscripten.
set -eu
NIGHTLY="nightly-2026-05-25"
EMSDK="${EMSDK_DIR:-$HOME/emsdk}"

[ -f "$EMSDK/emsdk_env.sh" ] || {
  echo "build-wasm: emsdk not found at $EMSDK (git clone emscripten-core/emsdk; ./emsdk install 4.0.20 && ./emsdk activate 4.0.20)"
  exit 2
}
# Under plain sh (dash) emsdk_env.sh has no BASH_SOURCE to locate itself and
# ignores $EMSDK; its one portable path is being sourced from its own
# directory (it looks for ./emsdk.py). So stand there while sourcing.
_here="$PWD"
cd "$EMSDK"
EMSDK_QUIET=1 . ./emsdk_env.sh
cd "$_here"
command -v emcc >/dev/null 2>&1 || {
  echo "build-wasm: emcc still missing after sourcing emsdk_env.sh — is 4.0.20 installed AND activated?"
  exit 2
}

# Compute AFTER sourcing emsdk_env.sh — it clobbers common variable names
# (DIR among them) and set -u would trip on the wreckage.
CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"

rustup toolchain list | grep -q "$NIGHTLY" || {
  echo "build-wasm: $NIGHTLY missing (rustup toolchain install $NIGHTLY; rustup component add rust-src --toolchain $NIGHTLY; rustup target add wasm32-unknown-emscripten --toolchain $NIGHTLY)"
  exit 2
}

RUSTFLAGS="-C link-args=-sSIDE_MODULE=2 -C llvm-args=-enable-emscripten-cxx-exceptions=0 -Z default-visibility=hidden -Z link-native-libraries=no -Z emscripten-wasm-eh=false" \
  cargo "+$NIGHTLY" build --manifest-path "$CRATE_DIR/Cargo.toml" \
  --features nothreads -Zbuild-std \
  --target wasm32-unknown-emscripten --release
echo "build-wasm: OK -> $CRATE_DIR/target/wasm32-unknown-emscripten/release/unseeing_core.wasm"
