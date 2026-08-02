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
DIR="$(cd "$(dirname "$0")" && pwd)"

NIGHTLY="nightly-2026-05-25"
EMSDK="${EMSDK_DIR:-$HOME/emsdk}"

[ -f "$EMSDK/emsdk_env.sh" ] || {
  echo "build-wasm: emsdk not found at $EMSDK (git clone emscripten-core/emsdk; ./emsdk install 4.0.20 && ./emsdk activate 4.0.20)"
  exit 2
}
EMSDK_QUIET=1 . "$EMSDK/emsdk_env.sh"

rustup toolchain list | grep -q "$NIGHTLY" || {
  echo "build-wasm: $NIGHTLY missing (rustup toolchain install $NIGHTLY; rustup component add rust-src --toolchain $NIGHTLY; rustup target add wasm32-unknown-emscripten --toolchain $NIGHTLY)"
  exit 2
}

RUSTFLAGS="-C link-args=-sSIDE_MODULE=2 -C llvm-args=-enable-emscripten-cxx-exceptions=0 -Z default-visibility=hidden -Z link-native-libraries=no -Z emscripten-wasm-eh=false" \
  cargo "+$NIGHTLY" build --manifest-path "$DIR/Cargo.toml" \
  --features nothreads -Zbuild-std \
  --target wasm32-unknown-emscripten --release
echo "build-wasm: OK -> $DIR/target/wasm32-unknown-emscripten/release/unseeing_core.wasm"
