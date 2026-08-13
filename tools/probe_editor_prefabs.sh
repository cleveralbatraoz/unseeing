#!/bin/sh
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
GODOT="${GODOT:-}"
if [ -z "$GODOT" ]; then
  for g in godot "$HOME/bin/godot" /opt/homebrew/bin/godot; do
    if command -v "$g" >/dev/null 2>&1 || [ -x "$g" ]; then GODOT="$g"; break; fi
  done
fi
[ -n "$GODOT" ] || { echo "probe: godot not found; set GODOT=/path/to/godot"; exit 2; }
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true
echo "probe: prefabs — asking for mode=editor"
if ! out="$("$GODOT" --headless --path "$DIR/game" -e -s res://tests/probe/editor_prefab_probe.gd 2>&1)"; then
  printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || printf '%s\n' "$out" | tail -20
  echo "probe: FAILED (editor prefab probe exited non-zero)"
  exit 1
fi
printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || true
printf '%s' "$out" | grep -q '^# prefabs: mode=editor$' || { echo "probe: FAILED — not editor mode"; exit 1; }
printf '%s' "$out" | grep -q '^probe: PASS (16 checks)$' || { echo "probe: FAILED — expected 16 checks"; exit 1; }
echo "probe: prefabs OK — instances, repacking, census, face labels and nested transforms agree"
