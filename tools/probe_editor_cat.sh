#!/bin/sh
# Editor-mode cat probe — the gate on the one branch a headless test suite
# cannot reach: WaveCat's ancestor-placement warning tracking a LIVE
# ancestor edit (raise and clear) with no scene reload.
#
# Godot exposes is_editor_hint() but NO setter — not to GDScript, not in the
# gdext bindings — so every gdUnit4 run is in run mode, and a regression in
# the editor poll would leave the whole suite green. `-e` is the way in: it
# puts the engine itself into editor mode, and it works headlessly with the
# stock pinned binary.
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

PROBE="res://tests/probe/editor_cat_probe.gd"

# The probe reaches WaveCat through the Rust extension, and the engine loads
# that only when .godot/extension_list.cfg names it — a generated file git
# deliberately does not track. Import first; the probe still refuses loudly
# if the class is missing afterwards.
"$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true

# Both modes must pass, and each run must PROVE it was in the mode it was
# asked for. `count` pins the exact number of checks a healthy run reports,
# not just that SOME check passed — a probe that aborted mid-phase after one
# check would still print `probe: PASS` for that one check.
run_mode() {
  want="$1"
  count="$2"
  shift 2
  echo "probe: cat — asking for mode=$want"
  if ! out="$("$GODOT" --headless --path "$DIR/game" "$@" -s "$PROBE" 2>&1)"; then
    printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || printf '%s\n' "$out" | tail -15
    echo "probe: FAILED (mode=$want — the engine exited non-zero)"
    exit 1
  fi
  printf '%s\n' "$out" | grep -aE '^(#|ok|not ok|1\.\.|probe:)' || true
  if ! printf '%s' "$out" | grep -q "^# cat: mode=$want\$"; then
    echo "probe: FAILED — asked for mode=$want and the engine did not report it"
    exit 1
  fi
  if ! printf '%s' "$out" | grep -q "^probe: PASS ($count checks)\$"; then
    echo "probe: FAILED (mode=$want — expected probe: PASS ($count checks))"
    exit 1
  fi
  if [ "$want" = editor ] && printf '%s' "$out" | grep -Fq "WaveCat 'LiveCat':"; then
    echo "probe: FAILED (mode=editor — the ancestor warning leaked into editor output)"
    exit 1
  fi
}

run_mode editor 3 -e
run_mode run 2

echo "probe: cat OK — the ancestor-placement warning tracks a live ancestor edit only in the editor"
