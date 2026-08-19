#!/bin/sh
# Measure the WEB target's platform facts, end to end, and put the project
# back exactly as it was.
#
# WHY THIS IS A SCRIPT AND NOT A CI GATE. The probe is a test scene, and the
# Web preset excludes `tests/*` on purpose — this repository has a standing
# rule, and a commit named for it, that test scripts must never reach the
# shipped scene tree. Measuring the web platform therefore needs the export
# to be built differently from the one that ships: the probe as its main
# scene, and the test exclusion lifted. Doing that by hand is how a stray
# `run/main_scene` or a relaxed `exclude_filter` gets committed. So the swap,
# the export, the measurement and the revert all live here, and the revert
# runs on every exit path including a crash.
#
# WHAT IT ANSWERS, and why the answers are load-bearing:
#   - how far apart two values must be to survive one data channel, in
#     units of a nominal 10-bit code. rust/src/render/channel.rs pins that
#     as WORST_STEP_CODES and the B-channel reconstruction guard turns on
#     it: the desktop needs 1.25 codes, so sight::RECT_SHRINK has to clear
#     24.4 mm rather than the 19.6 a clean code would imply. A browser that
#     needs MORE than the pinned figure fails this run, which is the whole
#     point of taking the measurement on the target rather than assuming it.
#   - whether hint_depth_texture is live. game/shaders/hearing_post.gdshader
#     ORs its exact depth-based layer test with an older, incomplete
#     wall-table inference purely because this was unknown on the web.
#
# Usage: tools/measure_web_platform.sh
# Env knobs: GODOT (binary), CHROME (browser), PROBE_PORT.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

# shellcheck source=tools/lib/engine.sh
. "$DIR/tools/lib/engine.sh"
GODOT="$(unseeing_engine_select "$DIR" "${GODOT:-}")" || {
  echo "platform-web: no Godot matching .godot-version; set GODOT=/path/to/godot"
  exit 2
}

PROJECT="$DIR/game/project.godot"
PRESETS="$DIR/game/export_presets.cfg"
BUILD="$DIR/game/build/web"
STASH="$(mktemp -d "${TMPDIR:-/tmp}/unseeing-web-measure.XXXXXX")"

restore() {
  # Unconditional and idempotent: the whole reason this script exists is
  # that a half-reverted project.godot is a committable accident.
  [ -f "$STASH/project.godot" ] && cp "$STASH/project.godot" "$PROJECT"
  [ -f "$STASH/export_presets.cfg" ] && cp "$STASH/export_presets.cfg" "$PRESETS"
  rm -rf "$STASH"
}
trap restore EXIT INT TERM HUP

cp "$PROJECT" "$STASH/project.godot"
cp "$PRESETS" "$STASH/export_presets.cfg"

# the probe becomes the main scene, and the test exclusion is lifted so the
# scene is actually in the pack.
#
# Written through a temporary file rather than `sed -i`: BSD sed wants an
# argument after -i and GNU sed refuses one, so the in-place form works on
# exactly one of the two hosts that have a GPU worth measuring. This script
# failed on macOS for that reason alone.
edit_in_place() {
  sed "$1" "$2" > "$2.probe-tmp" && mv "$2.probe-tmp" "$2"
}
edit_in_place 's|^run/main_scene=.*|run/main_scene="res://tests/probe/platform_probe.tscn"|' \
  "$PROJECT"
edit_in_place 's|^exclude_filter="tests/\*,|exclude_filter="|' "$PRESETS"
grep -q 'platform_probe.tscn' "$PROJECT" || {
  echo "platform-web: FAILED could not point the project at the probe scene"
  exit 1
}

echo "platform-web: building the throwaway probe export"
rm -rf "$BUILD"
mkdir -p "$BUILD"
touch "$DIR/game/build/.gdignore"
EXPORT_LOG="$(mktemp "${TMPDIR:-/tmp}/unseeing-web-measure-export.XXXXXX")"
if ! "$GODOT" --headless --path "$DIR/game" --export-release "Web" build/web/index.html \
  >"$EXPORT_LOG" 2>&1; then
  tail -15 "$EXPORT_LOG"
  rm -f "$EXPORT_LOG"
  echo "platform-web: FAILED the export did not build"
  exit 1
fi
rm -f "$EXPORT_LOG"

echo "platform-web: measuring under the browser"
"$DIR/tools/platform_probe_web.sh" "$BUILD"
STATUS=$?

# The export in game/build/web is now a PROBE build, not the game. Leaving it
# there would let a later step serve or deploy a test scene as if it were the
# product; the directory is git-ignored, so nothing would object.
rm -rf "$BUILD"

exit "$STATUS"
