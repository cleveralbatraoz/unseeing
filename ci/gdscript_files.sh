# shellcheck shell=sh
# The GDScript file set every lint gate must agree on (#28): every *.gd
# under game/, except third-party addons (game/addons/, deliberately
# skipped by the pre-commit hook too) and the import cache (game/.godot/,
# regenerated, never authored). gdUnit4 alone is vendored and lock-pinned;
# the ignored godot_mcp addon is a per-machine developer tool.
# -prune keeps a script written to a new directory covered by default,
# rather than by remembering to add it to a list.
#
# Sourced by ci/pipeline.sh and test/ci_gdscript_lint_scope.sh so the gate
# and its test can never drift apart. Takes the repo root as $1.
gdscript_files() {
  find "$1/game" \
    \( -path "$1/game/addons/gdUnit4" -o -path "$1/game/addons/godot_mcp" \
       -o -path "$1/game/.godot" \) \
    -prune -o -name '*.gd' -print
}

# First-party GDScript is permanently a test/probe boundary. Return every
# authored script outside game/tests/ so callers can refuse it loudly. Keep
# this separate from gdscript_files(): an illegal file remains visible to the
# independent lint census, never hidden merely because placement rejects it.
gdscript_policy_violations() {
  find "$1" \
    \( -path "$1/.git" -o -path "$1/.claude" -o -path "$1/.worktrees" \
       -o -path "$1/.superpowers" -o -path "$1/game/addons/gdUnit4" \
       -o -path "$1/game/addons/godot_mcp" \
       -o -path "$1/game/.godot" -o -path "$1/game/build" \
       -o -path "$1/game/reports" -o -path "$1/game/tests" \
       -o -path "$1/rust/target" -o -path "$1/tools/superpowers" \) \
    -prune -o -name '*.gd' -print
}
