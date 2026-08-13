#!/bin/sh
# Editor-mode level probe — the gate on the one branch a headless test suite
# cannot reach.
#
# WaveLevel derives its technical contracts (wall centerlines, the spawn,
# the demo tap, and now its own configuration warnings) in the editor too,
# so a designer sees what is wrong with their scene without pressing play —
# but at run time, even uninjected, it must still derive the same honest
# geometry it always has (level_test.gd pins that runtime behaviour
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

PROBE="res://tests/probe/editor_level_probe.gd"

# The probe reaches WaveLevel/WaveWall through the Rust extension, and the
# engine loads that only when .godot/extension_list.cfg names it — a
# generated file git deliberately does not track. A fresh clone therefore
# has no engine classes at all, and a probe that found no WaveLevel would be
# reporting on an empty engine rather than on the level. Import first; the
# probe still refuses loudly if the class is missing afterwards.
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true

# Both modes must pass, and each run must PROVE it was in the mode it was
# asked for. An engine that quietly ignored `-e` would leave this gate
# asserting the run-mode law twice, with the editor branch uncovered again
# and nothing to show for it.
#
# `count` pins the exact number of checks a healthy run reports, not just
# that SOME check passed: grepping only for `probe: PASS` would still match
# a probe that aborted mid-`_judge` after one check instead of eight —
# fewer checks looks exactly like a green run to a bare `probe: PASS` grep,
# and this repo has already been burned by an empty run wearing exit 0.
run_mode() {
  want="$1"
  count="$2"
  shift 2
  echo "probe: level — asking for mode=$want"
  # Teardown prints cosmetic RID-leak warnings after a perfectly good run,
  # so the verdict is read off the probe's own report and never off stderr
  # being empty.
  if ! out="$("$GODOT" --headless --path "$DIR/game" "$@" -s "$PROBE" 2>&1)"; then
    printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || printf '%s\n' "$out" | tail -15
    echo "probe: FAILED (mode=$want — the engine exited non-zero)"
    exit 1
  fi
  printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || true
  if ! printf '%s' "$out" | grep -q "^# level: mode=$want\$"; then
    echo "probe: FAILED — asked for mode=$want and the engine did not report it"
    exit 1
  fi
  if ! printf '%s' "$out" | grep -q "^probe: PASS ($count checks)\$"; then
    echo "probe: FAILED (mode=$want — expected probe: PASS ($count checks))"
    exit 1
  fi
  if [ "$want" = editor ] && printf '%s' "$out" | grep -Fq "WaveLevel: 'FlatCrate' built 2 planar face(s)"; then
    echo "probe: FAILED (mode=editor — degenerate diagnostic leaked into editor output)"
    exit 1
  fi
  if [ "$want" = editor ] && printf '%s' "$out" | grep -Fq "WaveLevel: 'WallCrate' overlaps the wall structure"; then
    echo "probe: FAILED (mode=editor — wall-merge diagnostic leaked into editor output)"
    exit 1
  fi
}

run_mode editor 12 -e
run_mode run 1

echo "probe: level OK — the level derives at edit time and keeps deriving honestly at run time"
