#!/bin/sh
# Deploy = git push to the droplet: its server-side pipeline runs the full
# gauntlet (boot check, unit tests, export, smoke test) and ships only on
# green. A local checks-only run fails fast first, and origin is pushed too
# so GitHub, its CI, and the tags always describe what is live.
set -eu
DIR="$(cd "$(dirname "$0")" && pwd)"

echo "== local checks =="
SKIP_EXPORT=1 "$DIR/ci/pipeline.sh"

echo "== cross-building the cores the droplet cannot build itself =="
# The 1.8 GB droplet cannot compile godot-core (OOM), so deploys carry the
# artifacts to it: linux .so via zigbuild, wasm via the pinned recipe. Its
# pipeline runs with PREBUILT_RUST=1 (set in the post-receive hook) and
# hard-fails if these are missing — freshness is structural, not hoped for.
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
cargo zigbuild --manifest-path "$DIR/rust/Cargo.toml" \
  --target x86_64-unknown-linux-gnu --release
"$DIR/rust/build-wasm.sh"
ssh vpn 'mkdir -p "$HOME/ci/cargo-target/release" "$HOME/ci/cargo-target/wasm32-unknown-emscripten/release"'
scp -q "$DIR/rust/target/x86_64-unknown-linux-gnu/release/libunseeing_core.so" \
  vpn:ci/cargo-target/release/libunseeing_core.so
scp -q "$DIR/rust/target/wasm32-unknown-emscripten/release/unseeing_core.wasm" \
  vpn:ci/cargo-target/wasm32-unknown-emscripten/release/unseeing_core.wasm

echo "== pushing to production (server-side CI takes over) =="
git -C "$DIR" push production main

echo "== pushing to origin =="
git -C "$DIR" push origin main --tags
