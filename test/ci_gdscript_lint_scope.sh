#!/bin/sh
# CI gate self-test: the GDScript lint set and tests/probes-only placement law.
#
# ci/pipeline.sh:97 used to hand gdformat/gdlint only
# `find game/scripts game/tests -name '*.gd'` — any script added anywhere
# else (a game/tools/ helper, say) escaped both checks silently. This
# tests both functions in ci/gdscript_files.sh directly against the real game/
# tree. Lint must widen to every authored script, while the permanent
# engine/content law must reject any first-party GDScript outside game/tests/.
# Both still exclude game/addons/ (third-party code; gdUnit4 alone is vendored
# and lock-pinned) and game/.godot/ (import cache, never authored).
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
TEST_PROBE="$DIR/game/tests/ci_policy_probe_$$.gd"
OUTSIDE_PROBE="$DIR/tools/ci_policy_probe_$$.gd"
WORKTREE_PROBE="$DIR/.claude/worktrees/ci_policy_probe_$$/game/scripts/foreign.gd"
UNKNOWN_ADDON="$DIR/game/addons/ci_policy_probe_$$/runtime.gd"
SPACE_PROBE="$DIR/game/tests/ci policy probe $$.gd"
POLICY_OUT="$(mktemp)"
cleanup() { rm -rf "$PROBE_DIR" "$DIR/game/.godot/ci_probe_$$.gd" "$TEST_PROBE" "$OUTSIDE_PROBE" "$SPACE_PROBE" "$(dirname "$UNKNOWN_ADDON")" "$(dirname "$(dirname "$(dirname "$WORKTREE_PROBE")")")" "$POLICY_OUT"; }
trap cleanup EXIT INT TERM HUP
mkdir -p "$PROBE_DIR"
printf 'extends Node\n' >"$PROBE_FILE"
printf 'extends Node\n' >"$TEST_PROBE"
printf 'extends Node\n' >"$OUTSIDE_PROBE"
mkdir -p "$(dirname "$WORKTREE_PROBE")"
printf 'extends Node\n' >"$WORKTREE_PROBE"
mkdir -p "$(dirname "$UNKNOWN_ADDON")"
printf 'extends Node\n' >"$UNKNOWN_ADDON"
printf 'extends Node\n' >"$SPACE_PROBE"

FOUND="$(gdscript_files "$DIR")"

if printf '%s\n' "$FOUND" | grep -qF "$PROBE_FILE"; then
  ok "a script under a brand-new directory (game/tools_ci_probe_$$/) is included"
else
  bad "a script under a brand-new directory is NOT included — a new script location would escape lint silently"
fi

# --- prove placement: linting an exportable script is not permission to ship
# it. The Rust/Godot split permits first-party GDScript only under game/tests/
# (suites, fixtures, probes and test-only shims). The brand-new tools script
# must be named as a violation, while the equally new test probe must remain
# legal. This calls the same production predicate as ci/pipeline.sh. ---
if command -v gdscript_policy_violations >/dev/null 2>&1; then
  VIOLATIONS="$(gdscript_policy_violations "$DIR")"
else
  bad "tests/probes-only placement predicate is absent"
  VIOLATIONS=""
fi
if printf '%s\n' "$VIOLATIONS" | grep -qxF "$PROBE_FILE"; then
  ok "a first-party script outside game/tests/ is rejected"
else
  bad "an exportable first-party script escaped the tests/probes-only policy"
fi
if printf '%s\n' "$VIOLATIONS" | grep -qxF "$TEST_PROBE"; then
  bad "a script under game/tests/ was incorrectly rejected"
else
  ok "game/tests/ remains the only legal first-party GDScript home"
fi
if printf '%s\n' "$VIOLATIONS" | grep -qxF "$OUTSIDE_PROBE"; then
  ok "a first-party script outside game/ is rejected too"
else
  bad "a repository script outside game/ escaped the tests/probes-only policy"
fi
if printf '%s\n' "$VIOLATIONS" | grep -qxF "$WORKTREE_PROBE"; then
  bad "an isolated worktree's files were mistaken for this checkout"
else
  ok "nested agent worktrees are excluded from this checkout's policy scan"
fi
if printf '%s\n' "$VIOLATIONS" | grep -qxF "$UNKNOWN_ADDON"; then
  ok "an unknown addon cannot masquerade as exempt third-party code"
else
  bad "an unknown addon escaped the first-party placement policy"
fi
if printf '%s\n' "$VIOLATIONS" | grep -qxF "$SPACE_PROBE"; then
  bad "a legal test path containing spaces was incorrectly rejected"
else
  ok "a legal test path containing spaces remains permitted"
fi

# Exercise the real executable boundary too: it must refuse the illegal file,
# name it, then accept the same tree once only the legal test probe remains.
if "$DIR/ci/check_gdscript_policy.sh" >"$POLICY_OUT" 2>&1; then
  bad "the production placement gate accepted an exportable GDScript file"
elif grep -qF "$PROBE_FILE" "$POLICY_OUT"; then
  ok "the production placement gate refuses and names the illegal script"
else
  bad "the production placement gate failed without naming the illegal script"
fi
rm -rf "$PROBE_DIR" "$OUTSIDE_PROBE" "$(dirname "$UNKNOWN_ADDON")"
if "$DIR/ci/check_gdscript_policy.sh" >"$POLICY_OUT" 2>&1; then
  ok "the production placement gate accepts a tests-only tree"
else
  bad "the production placement gate rejects the legal tests-only tree"
fi

# --- prove the known third-party allowlist: tracked gdUnit4 is in the tree,
# while the ignored godot_mcp addon may be installed locally. Unknown addons
# were deliberately proved illegal above. Split
# into an explicit if/else on directory presence, not `[ -d ... ] && grep`:
# the combined form falls to the else branch and prints a vacuous OK when
# the directory is simply absent, having asserted nothing — the exact
# anti-pattern test/repo_hygiene.sh's own comments warn against. ---
if [ -d "$DIR/game/addons" ]; then
  if printf '%s\n' "$FOUND" | grep -q '/game/addons/gdUnit4/'; then
    bad "game/addons/gdUnit4/ is NOT excluded — vendored third-party code would be linted"
  else
    ok "known gdUnit4 addon is excluded"
  fi
  if printf '%s\n' "$FOUND" | grep -q '/game/addons/godot_mcp/'; then
    bad "game/addons/godot_mcp/ is NOT excluded — ignored third-party code would be linted"
  else
    ok "known godot_mcp addon is excluded"
  fi
else
  echo "gdscript-lint-scope: SKIP known-addon exclusions (directory not present)"
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

# --- the legal scope must remain linted too. game/scripts/main.gd is gone now
# that main.tscn boots the Rust UnseeingGame node directly, and the placement
# gate above permanently forbids a first-party replacement there. What must
# still hold is game/tests/, proven here by two different files so a
# directory-level regression cannot hide behind one
# coincidentally-untouched name: pulses.gd (the wave pool's test-facing
# shim, relocated from game/scripts/ in the same change that retired
# main.gd) and wiring_test.gd (a suite that was always here). ---
if printf '%s\n' "$FOUND" | grep -q '/game/tests/pulses\.gd$'; then
  ok "game/tests/ is still covered (pulses.gd)"
else
  bad "game/tests/ dropped out of scope (pulses.gd)"
fi
if printf '%s\n' "$FOUND" | grep -q '/game/tests/wiring_test\.gd$'; then
  ok "game/tests/ is still covered (wiring_test.gd)"
else
  bad "game/tests/ dropped out of scope (wiring_test.gd)"
fi

exit "$FAIL"
