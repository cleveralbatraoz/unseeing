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

# Everything this deploy needs from the machine is asked for before anything
# is built or sent. The component is behavioral-testable without weakening the
# clean-main provenance above.
"$DIR/ci/deploy_host_preflight.sh" "$DIR"

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

echo "== verifying the droplet really deployed =="
# `git push` succeeds even when post-receive FAILS: the ref is updated before
# the hook runs and the hook's exit status never reaches the client, so a
# refused build scrolls past as one `remote: ci: FAILED` line and the script
# exits 0. A deploy that cannot tell you whether it deployed is not a deploy.
# So do not take the hook's word for it — ask the site what it is serving.
SHORT="$(printf %.9s "$HEAD_SHA")"
LIVE="$(curl -skL --max-time 30 "${CHECK_URL:-https://206.223.241.165/}" \
  | grep -o "UNSEEING_BUILD='[^']*'" | head -1 | sed "s/.*='//;s/'//")"
[ "$LIVE" = "$SHORT" ] || {
  echo "deploy: FAILED the site serves build '${LIVE:-none}', not '$SHORT'."
  echo "deploy:        The droplet's pipeline refused this push — its 'ci: FAILED'"
  echo "deploy:        line is above, among the remote: output."
  echo "deploy:        NOTE production/main already points at this commit, so"
  echo "deploy:        re-running deploy.sh unchanged will not retry the build."
  echo "deploy:        Fix the cause and push a new commit."
  exit 1
}
echo "deploy: the site serves $LIVE"

echo "== pushing to origin =="
git -C "$DIR" push origin main --tags
