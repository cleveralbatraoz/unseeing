#!/bin/sh
# Display-defaults probe — the window pins that only a REAL window can
# prove. Boots the game with its SHIPPED settings (full screen, at the
# monitor's own resolution), opens the settings overlay, and toggles full
# screen off and back on, checking the window actually followed.
#
# Deliberately NOT part of ci/pipeline.sh: a headless display server reports
# zero screens and no window, so every check below would be vacuous there.
# Deliberately NOT run through probe_visibility.sh either — that one forces
# a windowed override.cfg, which is the one thing this probe must not have.
#
# The toggle fires ~20 frames after boot, ON PURPOSE: that is mid-transition
# on macOS, where a full-screen toggle is silently dropped and the window
# server writes the old mode back afterwards. This is the regression.
#
# Env knobs: GODOT (binary).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

# One owner decides which engine is the pinned one, and refuses anything
# else — including an explicitly supplied mismatch. tools/lib/engine.sh.
# shellcheck source=tools/lib/engine.sh
. "$DIR/tools/lib/engine.sh"
GODOT="$(unseeing_engine_select "$DIR" "${GODOT:-}")" || {
  echo "probe: no Godot matching .godot-version; set GODOT=/path/to/godot"
  exit 2
}

# A stale override.cfg from a killed probe_visibility.sh run would force this
# probe windowed and make every full-screen check fail for the wrong reason.
if [ -f "$DIR/game/override.cfg" ]; then
  echo "probe: FAILED game/override.cfg exists — it forces windowed mode."
  echo "probe: remove it (a killed probe_visibility.sh run leaves it behind)."
  exit 2
fi

KEEP_AWAKE=""
command -v caffeinate >/dev/null 2>&1 && KEEP_AWAKE="caffeinate -dis"

# shellcheck disable=SC2086
$KEEP_AWAKE "$GODOT" --path "$DIR/game" res://tests/probe/display_probe.tscn
