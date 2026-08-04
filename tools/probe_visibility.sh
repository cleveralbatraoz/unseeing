#!/bin/sh
# Rendered visibility probe — the acoustic-image pixel pins. Boots the real
# game WINDOWED (real GPU frames; headless renders nothing), taps the
# divider the fan is behind, and asserts no reveal — not a shell wash, not
# a borrowed outline, not a tap's echo — leaks past the wall onto the
# always-on-top fan. Run on demand — deliberately NOT part of
# ci/pipeline.sh (headless CI cannot see shader-reveal leaks).
#
# Warm-boot law (memory, 2026-08-04): the first boot after a shader edit
# compiles subtly different GL programs than every boot after it — the
# probe therefore runs TWICE and both verdicts must agree; only a
# reproduced PASS counts. set -eu fails the script on the first FAIL.
#
# Env knobs: GODOT (binary).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

GODOT="${GODOT:-}"
if [ -z "$GODOT" ]; then
  for g in godot "$HOME/bin/godot" /opt/homebrew/bin/godot; do
    if command -v "$g" >/dev/null 2>&1 || [ -x "$g" ]; then GODOT="$g"; break; fi
  done
fi
[ -n "$GODOT" ] || { echo "probe: godot not found; set GODOT=/path/to/godot"; exit 2; }

KEEP_AWAKE=""
command -v caffeinate >/dev/null 2>&1 && KEEP_AWAKE="caffeinate -dis"

# shellcheck disable=SC2086
run_scene() {
  UNSEEING_DEMO=1 $KEEP_AWAKE "$GODOT" --path "$DIR/game" "$@"
}

for scene in res://tests/probe/occlusion_probe.tscn; do
  echo "probe: $scene — run 1 (cold cache legal; only agreement counts)"
  run_scene "$scene"
  echo "probe: $scene — run 2 (warm boot, the trusted one)"
  run_scene "$scene"
  echo "probe: $scene — PASS reproduced across two boots"
done
