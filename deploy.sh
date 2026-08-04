#!/bin/sh
# Deploy = git push to the droplet: its server-side pipeline runs the full
# gauntlet (boot check, unit tests, export, smoke test) and ships only on
# green. A local checks-only run fails fast first, and origin is pushed too
# so GitHub, its CI, and the tags always describe what is live.
set -eu
DIR="$(cd "$(dirname "$0")" && pwd)"

echo "== provenance =="
# The cores below are compiled from the WORKING TREE, but the deploy ships
# `git push production main`. Those are the same code only when main is what
# is checked out and nothing is uncommitted — otherwise the droplet runs a
# core built from source it never received, and no gate downstream can tell:
# its pipeline only sees a binary that exists. Checked before anything is
# built or pushed, so a wrong branch costs a second rather than a full build.
BRANCH="$(git -C "$DIR" rev-parse --abbrev-ref HEAD)"
[ "$BRANCH" = "main" ] || {
  echo "deploy: FAILED on branch '$BRANCH' — the deploy pushes main, so the cores would not match what ships"
  exit 2
}
DIRTY="$(git -C "$DIR" status --porcelain --untracked-files=no)"
[ -z "$DIRTY" ] || {
  echo "deploy: FAILED working tree is dirty — the cores would be built from code that is not in the commit being pushed:"
  printf '%s\n' "$DIRTY" | head -10
  exit 2
}
HEAD_SHA="$(git -C "$DIR" rev-parse HEAD)"
echo "deploy: shipping $(printf %.9s "$HEAD_SHA") from a clean main"

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
# ...and the commit they were built from, so the droplet's pipeline can
# REFUSE cores that belong to a different push instead of trusting that a
# file which exists is a file which is current
STAMP="$(mktemp)"
printf '%s\n' "$HEAD_SHA" > "$STAMP"
scp -q "$STAMP" vpn:ci/cargo-target/core.commit
rm -f "$STAMP"

echo "== pushing to production (server-side CI takes over) =="
git -C "$DIR" push production main

echo "== pushing to origin =="
git -C "$DIR" push origin main --tags
