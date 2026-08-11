#!/bin/sh
# CI gate self-test: the GDScript file set the format/lint gate covers (#28).
#
# ci/pipeline.sh:97 used to hand gdformat/gdlint only
# `find game/scripts game/tests -name '*.gd'` — any script added anywhere
# else (a game/tools/ helper, say) escaped both checks silently. This
# tests gdscript_files() (ci/gdscript_files.sh — the same function
# ci/pipeline.sh calls to build GD_FILES, so the gate and this test can
# never drift apart) directly against the real game/ tree: it must widen
# to cover a brand-new directory by default, and it must still exclude the
# two directories that are not ours to lint — game/addons/ (vendored
# third-party, pinned by ci/vendor-gdunit4.sh, deliberately skipped by the
# pre-commit hook too) and game/.godot/ (import cache, never authored).
#
# Pure POSIX sh, no network, no Godot — runs anywhere ci/pipeline.sh does.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

. "$DIR/ci/gdscript_files.sh"

ok() { echo "gdscript-lint-scope: OK   $1"; }
bad() { echo "gdscript-lint-scope: FAIL $1"; FAIL=1; }

# --- prove inclusion: a script in a directory neither game/scripts nor
# game/tests, cleaned up on the way out whether this exits clean or not ---
PROBE_DIR="$DIR/game/tools_ci_probe_$$"
PROBE_FILE="$PROBE_DIR/probe.gd"
cleanup() { rm -rf "$PROBE_DIR" "$DIR/game/.godot/ci_probe_$$.gd"; }
trap cleanup EXIT
mkdir -p "$PROBE_DIR"
printf 'extends Node\n' >"$PROBE_FILE"

FOUND="$(gdscript_files "$DIR")"

if printf '%s\n' "$FOUND" | grep -qF "$PROBE_FILE"; then
  ok "a script under a brand-new directory (game/tools_ci_probe_$$/) is included"
else
  bad "a script under a brand-new directory is NOT included — a new script location would escape lint silently"
fi

# --- prove exclusion: the vendored addon really is in the tree (gdUnit4),
# so this is checked against real files, not a synthetic stand-in. Split
# into an explicit if/else on directory presence, not `[ -d ... ] && grep`:
# the combined form falls to the else branch and prints a vacuous OK when
# the directory is simply absent, having asserted nothing — the exact
# anti-pattern test/repo_hygiene.sh's own comments warn against. ---
if [ -d "$DIR/game/addons" ]; then
  if printf '%s\n' "$FOUND" | grep -q '/game/addons/'; then
    bad "game/addons/ is NOT excluded — the vendored addon would be linted"
  else
    ok "game/addons/ is excluded"
  fi
else
  echo "gdscript-lint-scope: SKIP game/addons/ exclusion (directory not present)"
fi

# --- prove exclusion of the import cache: create a real probe .gd there
# too, since .godot/ only exists after an --import has run ---
if [ -d "$DIR/game/.godot" ]; then
  printf 'extends Node\n' >"$DIR/game/.godot/ci_probe_$$.gd"
  FOUND="$(gdscript_files "$DIR")"
  if printf '%s\n' "$FOUND" | grep -q '/game/\.godot/'; then
    bad "game/.godot/ is NOT excluded — the import cache would be linted"
  else
    ok "game/.godot/ is excluded"
  fi
else
  echo "gdscript-lint-scope: SKIP game/.godot/ exclusion (no import cache present — run godot --import first)"
fi

# --- the scope this gate always had must survive the widening ---
if printf '%s\n' "$FOUND" | grep -q '/game/scripts/main\.gd$'; then
  ok "game/scripts/ is still covered"
else
  bad "game/scripts/ dropped out of scope"
fi
if printf '%s\n' "$FOUND" | grep -q '/game/tests/wiring_test\.gd$'; then
  ok "game/tests/ is still covered"
else
  bad "game/tests/ dropped out of scope"
fi

exit "$FAIL"
