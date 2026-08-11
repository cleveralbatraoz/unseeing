#!/bin/sh
# Editor-mode source probe — the gate on the one branch a headless test suite
# cannot reach.
#
# A sound source's body (today: SoundFan) BUILDS its blueprint limbs when it
# is placed in the editor, so a designer sees the fan they are dragging
# instead of an empty node — but at run time, uninjected, it must still
# build NOTHING and refuse loudly (fan_test.gd pins that runtime guard
# exactly). Godot exposes is_editor_hint() but NO setter — not to GDScript,
# not in the gdext bindings — so every gdUnit4 run is in run mode, and a
# regression in the editor branch would leave the whole suite green.
#
# `-e` is the way in: it puts the engine itself into editor mode, and it
# works headlessly with the stock pinned binary. So unlike the WINDOWED
# probes next door (tools/probe_visibility.sh, tools/probe_display.sh),
# which need a real GPU and a human to run them, this one reads engine
# STATE, is headless and deterministic, and runs inside ci/pipeline.sh.
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

PROBE="res://tests/probe/editor_source_probe.gd"

# The probe reaches the fan through the Rust extension, and the engine loads
# that only when .godot/extension_list.cfg names it — a generated file git
# deliberately does not track. A fresh clone therefore has no engine classes
# at all, and a probe that found no SoundFan would be reporting on an empty
# engine rather than on the fan. Import first; the probe still refuses
# loudly if the class is missing afterwards.
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true

# Both modes must pass, and each run must PROVE it was in the mode it was
# asked for. An engine that quietly ignored `-e` would leave this gate
# asserting the run-mode law twice, with the editor branch uncovered again
# and nothing to show for it.
run_mode() {
  want="$1"
  shift
  echo "probe: sources — asking for mode=$want"
  # Teardown prints cosmetic RID-leak warnings after a perfectly good run,
  # so the verdict is read off the probe's own report and never off stderr
  # being empty.
  if ! out="$("$GODOT" --headless --path "$DIR/game" "$@" -s "$PROBE" 2>&1)"; then
    printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || printf '%s\n' "$out" | tail -15
    echo "probe: FAILED (mode=$want — the engine exited non-zero)"
    exit 1
  fi
  printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || true
  if ! printf '%s' "$out" | grep -q "^# sources: mode=$want\$"; then
    echo "probe: FAILED — asked for mode=$want and the engine did not report it"
    exit 1
  fi
  if ! printf '%s' "$out" | grep -q '^probe: PASS'; then
    echo "probe: FAILED (mode=$want)"
    exit 1
  fi
}

run_mode editor -e
run_mode run

echo "probe: sources OK — blueprint limbs in the editor, silence uninjected at run time"
