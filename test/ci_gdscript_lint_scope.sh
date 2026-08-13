#!/bin/sh
# CI gate self-test: the GDScript lint set and tests/probes-only placement law.
#
# ci/pipeline.sh:97 used to hand gdformat/gdlint only
# `find game/scripts game/tests -name '*.gd'` — any script added anywhere
# else (a game/tools/ helper, say) escaped both checks silently. This
# tests both functions in ci/gdscript_files.sh directly against the real game/
# tree. Lint must widen to every authored script, while the permanent
# engine/content law must reject any first-party GDScript outside game/tests/.
# All lint and policy scans exclude the known gdUnit4 and godot_mcp addon
# trees (third-party code; gdUnit4 alone is vendored and lock-pinned) and
# game/.godot/ (import cache, never authored). An unknown addon remains
# first-party and illegal.
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
FALLBACK_WORKTREE_PROBE="$DIR/.worktrees/ci_policy_probe_$$/game/scripts/foreign.gd"
WORKTREE_RESOURCE_PROBE="$DIR/.claude/worktrees/ci_policy_probe_$$/game/scenes/foreign.tscn"
FALLBACK_WORKTREE_RESOURCE_PROBE="$DIR/.worktrees/ci_policy_probe_$$/game/scenes/foreign.tscn"
UNKNOWN_ADDON="$DIR/game/addons/ci_policy_probe_$$/runtime.gd"
SPACE_PROBE="$DIR/game/tests/ci policy probe $$.gd"
BUILTIN_PROBE="$DIR/game/scenes/ci_policy_builtin_$$.tscn"
OPAQUE_PROBE="$DIR/game/scenes/ci_policy_opaque_$$.res"
LEGAL_BUILTIN="$DIR/game/tests/ci policy built in $$.tscn"
REFERENCE_FIXTURE="$(mktemp -d)"
REFERENCE_SCRIPT="$REFERENCE_FIXTURE/game/tests/helper script.gd"
EXTERNAL_SCENE="$REFERENCE_FIXTURE/game/scenes/external test script.tscn"
EXTERNAL_RESOURCE="$REFERENCE_FIXTURE/game/scenes/external test script.tres"
LEGAL_EXTERNAL="$REFERENCE_FIXTURE/game/tests/legal external script.tscn"
REFERENCE_PROJECT="$REFERENCE_FIXTURE/game/project.godot"
NON_AUTOLOAD_FIXTURE="$(mktemp -d)"
NON_AUTOLOAD_PROJECT="$NON_AUTOLOAD_FIXTURE/game/project.godot"
TRANSITIVE_FIXTURE="$(mktemp -d)"
SCRIPTED_TEST_SCENE="$TRANSITIVE_FIXTURE/game/tests/scripted fixture.tscn"
SCRIPTED_TEST_RESOURCE="$TRANSITIVE_FIXTURE/game/tests/scripted fixture.tres"
TRANSITIVE_SCENE="$TRANSITIVE_FIXTURE/game/scenes/transitive test scene.tscn"
TRANSITIVE_RESOURCE="$TRANSITIVE_FIXTURE/game/scenes/transitive test resource.tres"
WHITESPACE_BUILTIN="$TRANSITIVE_FIXTURE/game/scenes/whitespace built in.tscn"
TRANSITIVE_PROJECT="$TRANSITIVE_FIXTURE/game/project.godot"
POLICY_OUT="$(mktemp)"
cleanup() {
  rm -rf "$PROBE_DIR" "$DIR/game/.godot/ci_probe_$$.gd" "$TEST_PROBE" \
    "$OUTSIDE_PROBE" "$SPACE_PROBE" "$(dirname "$UNKNOWN_ADDON")" \
    "$DIR/.claude/worktrees/ci_policy_probe_$$" \
    "$DIR/.worktrees/ci_policy_probe_$$" "$BUILTIN_PROBE" "$OPAQUE_PROBE" \
    "$LEGAL_BUILTIN" "$REFERENCE_FIXTURE" "$NON_AUTOLOAD_FIXTURE" \
    "$TRANSITIVE_FIXTURE" "$POLICY_OUT"
}
trap cleanup EXIT INT TERM HUP
mkdir -p "$PROBE_DIR"
printf 'extends Node\n' >"$PROBE_FILE"
printf 'extends Node\n' >"$TEST_PROBE"
printf 'extends Node\n' >"$OUTSIDE_PROBE"
mkdir -p "$(dirname "$WORKTREE_PROBE")"
printf 'extends Node\n' >"$WORKTREE_PROBE"
mkdir -p "$(dirname "$FALLBACK_WORKTREE_PROBE")"
printf 'extends Node\n' >"$FALLBACK_WORKTREE_PROBE"
mkdir -p "$(dirname "$WORKTREE_RESOURCE_PROBE")" \
  "$(dirname "$FALLBACK_WORKTREE_RESOURCE_PROBE")"
printf '%s\n' '[gd_scene format=3]' \
  '[ext_resource path="res://tests/foreign.gd" type="Script" id="1"]' \
  '[node name="Foreign" type="Node"]' >"$WORKTREE_RESOURCE_PROBE"
printf '%s\n' '[gd_scene format=3]' \
  '[ext_resource path="res://tests/foreign.gd" type="Script" id="1"]' \
  '[node name="Foreign" type="Node"]' >"$FALLBACK_WORKTREE_RESOURCE_PROBE"
mkdir -p "$(dirname "$UNKNOWN_ADDON")"
printf 'extends Node\n' >"$UNKNOWN_ADDON"
printf 'extends Node\n' >"$SPACE_PROBE"
printf '%s\n' '[gd_scene load_steps=2 format=3]' \
  '[sub_resource type="GDScript" id="GDScript_probe"]' \
  'script/source = "extends Node"' \
  '[node name="Probe" type="Node"]' \
  'script = SubResource("GDScript_probe")' >"$BUILTIN_PROBE"
# Binary .scn/.res resources are opaque to this cheap source gate and can hide
# the same built-in script. First-party authoring therefore uses diffable
# .tscn/.tres outside tests; the extension itself is the refusal witness here.
printf 'opaque resource probe\n' >"$OPAQUE_PROBE"
cp "$BUILTIN_PROBE" "$LEGAL_BUILTIN"

# An allowlisted test script is legal only as test code. Production scenes and
# resources must not smuggle it back into the shipped scene tree through an
# external Script resource, and project.godot must not turn it into an autoload.
# Keep these fixtures outside the real project so the autoload case never edits
# the user's tracked project settings even for an instant.
mkdir -p "$(dirname "$REFERENCE_SCRIPT")" "$(dirname "$EXTERNAL_SCENE")"
printf 'extends Node\n' >"$REFERENCE_SCRIPT"
printf '%s\n' '[gd_scene load_steps=2 format=3]' \
  '[ext_resource type="Script" path="res://tests/helper script.gd" id="1_script"]' \
  '[node name="Production" type="Node"]' \
  'script = ExtResource("1_script")' >"$EXTERNAL_SCENE"
printf '%s\n' '[gd_resource load_steps=2 format=3]' \
  '[ext_resource path="res://tests/helper script.gd" type="Script" id="1_script"]' \
  '[resource]' \
  'script = ExtResource("1_script")' >"$EXTERNAL_RESOURCE"
cp "$EXTERNAL_SCENE" "$LEGAL_EXTERNAL"
printf '%s\n' '[application]' \
  'config/name="Reference fixture"' \
  '' \
  '[autoload]' \
  'TestStarred="*res://tests/helper script.gd"' \
  'TestPlain="res://tests/helper script.gd"' >"$REFERENCE_PROJECT"

# A path-shaped string outside [autoload] is data, not executable wiring. The
# policy must not reject project.godot merely because ordinary metadata happens
# to mention a test resource.
mkdir -p "$(dirname "$NON_AUTOLOAD_PROJECT")"
printf '%s\n' '[application]' \
  'config/name="res://tests/helper script.gd"' >"$NON_AUTOLOAD_PROJECT"

# A test scene or resource can itself own test GDScript. Production content
# must not regain that behavior transitively by instancing or loading anything
# below res://tests/. Deliberately vary attribute order and whitespace so the
# source-only gate follows the path rather than one serializer layout.
mkdir -p "$(dirname "$SCRIPTED_TEST_SCENE")" "$(dirname "$TRANSITIVE_SCENE")"
printf '%s\n' '[gd_scene load_steps=2 format=3]' \
  '[ext_resource type="Script" path="res://tests/helper script.gd" id="1_script"]' \
  '[node name="ScriptedFixture" type="Node"]' \
  'script = ExtResource("1_script")' >"$SCRIPTED_TEST_SCENE"
printf '%s\n' '[gd_resource load_steps=2 format=3]' \
  '[ext_resource path="res://tests/helper script.gd" type="Script" id="1_script"]' \
  '[resource]' \
  'script = ExtResource("1_script")' >"$SCRIPTED_TEST_RESOURCE"
printf '%s\n' '[gd_scene load_steps=2 format=3]' \
  '[ext_resource   type = "PackedScene"   path = "res://tests/scripted fixture.tscn" id="1_fixture"]' \
  '[node name="Production" type="Node"]' \
  '[node name="Fixture" parent="." instance=ExtResource("1_fixture")]' >"$TRANSITIVE_SCENE"
printf '%s\n' '[gd_resource load_steps=2 format=3]' \
  '[ext_resource path = "res://tests/scripted fixture.tres"   id="1_fixture" type = "Resource"]' \
  '[resource]' \
  'metadata/fixture = ExtResource("1_fixture")' >"$TRANSITIVE_RESOURCE"
printf '%s\n' '[gd_scene load_steps=2 format=3]' \
  '[sub_resource id="GDScript_fixture" type = "GDScript"]' \
  'script/source = "extends Node"' \
  '[node name="Production" type="Node"]' \
  'script = SubResource("GDScript_fixture")' >"$WHITESPACE_BUILTIN"
printf '%s\n' '[autoload]' \
  'ScriptedFixture = "*res://tests/scripted fixture.tscn"' >"$TRANSITIVE_PROJECT"

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
if printf '%s\n' "$VIOLATIONS" | grep -qxF "$FALLBACK_WORKTREE_PROBE"; then
  bad "a fallback worktree's files were mistaken for this checkout"
else
  ok "fallback .worktrees are excluded from this checkout's policy scan"
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

if command -v gdscript_resource_policy_violations >/dev/null 2>&1; then
  RESOURCE_VIOLATIONS="$(gdscript_resource_policy_violations "$DIR")"
else
  bad "embedded/opaque GDScript resource predicate is absent"
  RESOURCE_VIOLATIONS=""
fi
if printf '%s\n' "$RESOURCE_VIOLATIONS" | grep -qxF "$BUILTIN_PROBE"; then
  ok "a built-in GDScript in an exportable scene is rejected"
else
  bad "a built-in GDScript escaped through an exportable scene"
fi
if printf '%s\n' "$RESOURCE_VIOLATIONS" | grep -qxF "$OPAQUE_PROBE"; then
  ok "an opaque binary Godot resource cannot hide shipped GDScript"
else
  bad "an opaque binary Godot resource escaped the inspectable-source policy"
fi
if printf '%s\n' "$RESOURCE_VIOLATIONS" | grep -qxF "$LEGAL_BUILTIN"; then
  bad "a built-in GDScript under game/tests/ was incorrectly rejected"
else
  ok "test fixtures may still carry built-in GDScript"
fi
if printf '%s\n' "$RESOURCE_VIOLATIONS" | grep -qxF "$WORKTREE_RESOURCE_PROBE"; then
  bad "an isolated worktree's resource was mistaken for this checkout"
else
  ok "resource policy prunes nested agent worktrees too"
fi
if printf '%s\n' "$RESOURCE_VIOLATIONS" | grep -qxF "$FALLBACK_WORKTREE_RESOURCE_PROBE"; then
  bad "a fallback worktree's resource was mistaken for this checkout"
else
  ok "resource policy prunes fallback .worktrees too"
fi

REFERENCE_VIOLATIONS="$(gdscript_resource_policy_violations "$REFERENCE_FIXTURE")"
if printf '%s\n' "$REFERENCE_VIOLATIONS" | grep -qxF "$EXTERNAL_SCENE"; then
  ok "a production .tscn cannot attach an allowlisted test script"
else
  bad "a production .tscn smuggled game/tests GDScript into shipped content"
fi
if printf '%s\n' "$REFERENCE_VIOLATIONS" | grep -qxF "$EXTERNAL_RESOURCE"; then
  ok "a production .tres cannot attach an allowlisted test script"
else
  bad "a production .tres smuggled game/tests GDScript into shipped content"
fi
if printf '%s\n' "$REFERENCE_VIOLATIONS" | grep -qxF "$REFERENCE_PROJECT"; then
  ok "project.godot cannot autoload an allowlisted test script"
else
  bad "project.godot autoloaded game/tests GDScript into shipped behavior"
fi
if printf '%s\n' "$REFERENCE_VIOLATIONS" | grep -qxF "$LEGAL_EXTERNAL"; then
  bad "a test scene's external test script was incorrectly rejected"
else
  ok "test resources may still attach test GDScript"
fi
NON_AUTOLOAD_VIOLATIONS="$(gdscript_resource_policy_violations "$NON_AUTOLOAD_FIXTURE")"
if printf '%s\n' "$NON_AUTOLOAD_VIOLATIONS" | grep -qxF "$NON_AUTOLOAD_PROJECT"; then
  bad "non-executable project metadata mentioning a test path was rejected"
else
  ok "only executable project.godot wiring is policy-relevant"
fi

TRANSITIVE_VIOLATIONS="$(gdscript_resource_policy_violations "$TRANSITIVE_FIXTURE")"
if printf '%s\n' "$TRANSITIVE_VIOLATIONS" | grep -qxF "$TRANSITIVE_SCENE"; then
  ok "production scenes cannot instance scripted test scenes transitively"
else
  bad "a scripted test scene escaped through a whitespace-varied ext_resource"
fi
if printf '%s\n' "$TRANSITIVE_VIOLATIONS" | grep -qxF "$TRANSITIVE_RESOURCE"; then
  ok "production resources cannot load scripted test resources transitively"
else
  bad "a scripted test resource escaped through a whitespace-varied ext_resource"
fi
if printf '%s\n' "$TRANSITIVE_VIOLATIONS" | grep -qxF "$TRANSITIVE_PROJECT"; then
  ok "project.godot cannot autoload a scripted test scene transitively"
else
  bad "project.godot autoloaded a scripted scene from res://tests/"
fi
if printf '%s\n' "$TRANSITIVE_VIOLATIONS" | grep -qxF "$WHITESPACE_BUILTIN"; then
  ok "built-in GDScript type attributes are whitespace-tolerant"
else
  bad "a spaced GDScript type attribute escaped the resource policy"
fi
if printf '%s\n' "$TRANSITIVE_VIOLATIONS" \
  | grep -qxF "$SCRIPTED_TEST_SCENE"; then
  bad "the scripted test scene itself was incorrectly rejected"
else
  ok "scripted scenes remain legal inside game/tests/"
fi
if printf '%s\n' "$TRANSITIVE_VIOLATIONS" \
  | grep -qxF "$SCRIPTED_TEST_RESOURCE"; then
  bad "the scripted test resource itself was incorrectly rejected"
else
  ok "scripted resources remain legal inside game/tests/"
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
  bad "the production placement gate accepted embedded or opaque GDScript resources"
elif grep -qF "$BUILTIN_PROBE" "$POLICY_OUT" \
  && grep -qF "$OPAQUE_PROBE" "$POLICY_OUT"; then
  ok "the production placement gate refuses and names both resource escapes"
else
  bad "the production placement gate did not name both resource escapes"
fi
rm -f "$BUILTIN_PROBE" "$OPAQUE_PROBE"
if "$DIR/ci/check_gdscript_policy.sh" >"$POLICY_OUT" 2>&1; then
  ok "the production placement gate accepts a tests-only tree"
else
  bad "the production placement gate rejects the legal tests-only tree"
fi

# --- prove the lint census's known third-party exclusions: tracked gdUnit4 is
# in the tree, while the ignored godot_mcp addon may be installed locally.
# Unknown addons were deliberately proved illegal above. Split
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
