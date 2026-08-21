#!/bin/sh
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
PROBE=res://tests/probe/editor_prefab_probe.gd
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true
echo "probe: prefabs — asking for mode=editor"
if ! out="$("$GODOT" --headless --path "$DIR/game" -e -s "$PROBE" 2>&1)"; then
  printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || printf '%s\n' "$out" | tail -20
  echo "probe: FAILED (editor prefab probe exited non-zero)"
  exit 1
fi
printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || true
printf '%s' "$out" | grep -q '^# prefabs: mode=editor$' || { echo "probe: FAILED — not editor mode"; exit 1; }
printf '%s' "$out" | grep -q '^probe: PASS (36 checks)$' || { echo "probe: FAILED — expected 36 checks"; exit 1; }
echo "probe: prefabs OK — inheritance, duplication, disk persistence and nested warning watching agree"
