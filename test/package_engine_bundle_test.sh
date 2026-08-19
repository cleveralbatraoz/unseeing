#!/bin/sh
# Behavioral contract for the designer-bundle packaging script: given an
# already-built library and a git checkout, it must produce a zip whose
# single top-level folder contains the tracked game/ tree, the library at
# the exact rust/target/<dest> path .gdextension expects (a SIBLING of
# game/, not nested inside it -- the bug this test exists to catch), and an
# ENGINE_COMMIT file naming the real commit. Runs against a synthetic
# fixture repo, never the real 300+-file game/ tree, so it stays fast and
# is not a change detector on real content.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUBJECT="${PACKAGE_BUNDLE_SUBJECT:-$ROOT/tools/package_engine_bundle.sh}"
FAIL=0

ok() { echo "package-bundle: OK   $1"; }
bad() { echo "package-bundle: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}

command -v git >/dev/null 2>&1 || { echo "package-bundle: SKIP git not found"; exit 0; }
command -v python3 >/dev/null 2>&1 || { echo "package-bundle: SKIP python3 not found"; exit 0; }
command -v unzip >/dev/null 2>&1 || { echo "package-bundle: SKIP unzip not found"; exit 0; }

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM
REPO="$T/repo"
mkdir -p "$REPO/tools" "$REPO/game/scenes" "$REPO/rust"
cp "$SUBJECT" "$REPO/tools/package_engine_bundle.sh"
chmod +x "$REPO/tools/package_engine_bundle.sh"
printf 'config_version=5\n' >"$REPO/game/project.godot"
printf 'fixture level content\n' >"$REPO/game/scenes/level_01.tscn"
# Only game/ is ever archived, so a file outside it proves the script does
# not silently ship the whole checkout.
printf 'not shipped\n' >"$REPO/rust/should-not-appear.txt"

( cd "$REPO" \
  && git init -q \
  && git config user.email "fixture@example.invalid" \
  && git config user.name "fixture" \
  && git add game \
  && git commit -q -m "fixture game tree" )
COMMIT="$(cd "$REPO" && git rev-parse HEAD)"

printf 'fake engine library\n' >"$T/libunseeing_core.so"
OUT_ZIP="$T/out/unseeing-editor-linux-x86_64.zip"

STATUS=0
"$REPO/tools/package_engine_bundle.sh" linux-x86_64 "$T/libunseeing_core.so" \
  release/libunseeing_core.so "$OUT_ZIP" >"$T/run.log" 2>&1 || STATUS=$?
require "the script exits 0 on a valid invocation" test "$STATUS" -eq 0
[ "$STATUS" -eq 0 ] || sed 's/^/package-bundle:      /' "$T/run.log"

require "the zip was created" test -s "$OUT_ZIP"

EXTRACT="$T/extract"
mkdir -p "$EXTRACT"
unzip -q "$OUT_ZIP" -d "$EXTRACT"

require "the zip has exactly one top-level entry" \
  test "$(find "$EXTRACT" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')" = "1"
require "the top-level entry is named unseeing-editor-linux-x86_64" \
  test -d "$EXTRACT/unseeing-editor-linux-x86_64"

BUNDLE="$EXTRACT/unseeing-editor-linux-x86_64"
require "game/project.godot is present" test -f "$BUNDLE/game/project.godot"
require "game/scenes/level_01.tscn is present" \
  test -f "$BUNDLE/game/scenes/level_01.tscn"
require "the library sits at rust/target/release/libunseeing_core.so" \
  test -f "$BUNDLE/rust/target/release/libunseeing_core.so"
# THE bug this test exists to catch: unseeing.gdextension resolves
# res://../rust/target/..., one level ABOVE game/ -- so rust/ must be a
# SIBLING of game/, never nested inside it.
require "rust/ is a sibling of game/, not nested inside it" \
  test ! -e "$BUNDLE/game/rust"
require "the shipped library is the one that was given, byte for byte" \
  cmp -s "$BUNDLE/rust/target/release/libunseeing_core.so" "$T/libunseeing_core.so"
require "ENGINE_COMMIT names the real commit" \
  test "$(cat "$BUNDLE/ENGINE_COMMIT")" = "$COMMIT"
require "rust/should-not-appear.txt is not shipped (only game/ is archived)" \
  test ! -e "$BUNDLE/rust/should-not-appear.txt"
require "no .git reaches the bundle" test ! -e "$BUNDLE/.git"

# Preconditions: a missing source library must refuse before touching disk.
STATUS=0
"$REPO/tools/package_engine_bundle.sh" linux-x86_64 "$T/does-not-exist.so" \
  release/libunseeing_core.so "$T/out/should-not-exist.zip" >"$T/refuse.log" 2>&1 || STATUS=$?
require "a missing source library refuses with exit 2" test "$STATUS" -eq 2
require "the refusal names the missing file" \
  grep -q "does-not-exist.so" "$T/refuse.log"
require "no zip was written on refusal" test ! -e "$T/out/should-not-exist.zip"

exit "$FAIL"
