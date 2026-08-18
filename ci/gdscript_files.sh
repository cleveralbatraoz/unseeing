# shellcheck shell=sh
# The GDScript file set every lint gate must agree on (#28): every *.gd
# under game/, except the two known third-party addon trees and the import
# cache (game/.godot/, regenerated, never authored). gdUnit4 is vendored and
# lock-pinned; the ignored godot_mcp addon is a per-machine developer tool.
# Any other addon path remains first-party and is linted and policy-checked.
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

# Godot can store a GDScript inside a text scene/resource instead of a .gd
# file, or attach a test resource back onto production as an external resource.
# Anything below res://tests/ may contain test-only behavior transitively, so
# production resources and project.godot [autoload] entries may not reference
# that subtree. Scan every first-party .tscn/.tres outside the test tree for
# both representations, and refuse opaque .scn/.res resources outright: their
# contents cannot be audited by this source-only gate. Designer-authored
# production resources therefore stay text, diffable, and code-free; test
# resources remain outside this rule.
gdscript_resource_policy_violations() {
  find "$1" \
    \( -path "$1/.git" -o -path "$1/.claude" -o -path "$1/.worktrees" \
       -o -path "$1/.superpowers" -o -path "$1/game/addons/gdUnit4" \
       -o -path "$1/game/addons/godot_mcp" \
       -o -path "$1/game/.godot" -o -path "$1/game/build" \
       -o -path "$1/game/reports" -o -path "$1/game/tests" \
       -o -path "$1/rust/target" -o -path "$1/tools/superpowers" \) \
    -prune -o \( -name '*.scn' -o -name '*.res' \) -print -o \
    \( -name '*.tscn' -o -name '*.tres' \) -exec grep -Il \
      -e '^\[sub_resource[^]]*[[:space:]]type[[:space:]]*=[[:space:]]*"GDScript"\([[:space:]]\|\]\)' \
      -e '^\[ext_resource[^]]*[[:space:]]path[[:space:]]*=[[:space:]]*"res://tests/[^"]*"' {} \;

  PROJECT_FILE="$1/game/project.godot"
  if [ -f "$PROJECT_FILE" ] && LC_ALL=C awk '
    /^[[:space:]]*\[[^]]+\][[:space:]]*$/ {
      in_autoload = ($0 ~ /^[[:space:]]*\[autoload\][[:space:]]*$/)
      next
    }
    in_autoload && /^[[:space:]]*[^;#][^=]*=[[:space:]]*"\*?res:\/\/tests\/[^\"]*"[[:space:]]*$/ {
      found = 1
    }
    END { exit(found ? 0 : 1) }
  ' "$PROJECT_FILE"; then
    printf '%s\n' "$PROJECT_FILE"
  fi
}

# A shader is a shipped resource. game/export_presets.cfg exports
# "all_resources" and its exclude filter names only tests/, addons/ and
# reports/, so every .gdshader under res:// that those do not cover is packed
# into the web, macOS and Windows builds whether or not the game references
# it. Nine probe-only shaders shipped to players that way, each one carrying
# a header saying it was never referenced by the game.
#
# This is a CONTENT test rather than a name test: it catches a shader that
# declares itself probe-only under any filename, and renaming cannot defeat
# it. Probe shaders belong under game/tests/, beside the scenes that preload
# them, where one exclusion rule already covers the whole corpus and where
# tools/measure_web_platform.sh's single sed still lifts it for the web
# measurement.
shader_placement_violations() {
  find "$1/game" \
    \( -path "$1/game/addons" -o -path "$1/game/.godot" \
       -o -path "$1/game/build" -o -path "$1/game/reports" \
       -o -path "$1/game/tests" \) \
    -prune -o \( -name '*.gdshader' -o -name '*.gdshaderinc' \) \
    -exec grep -Il -e 'PROBE ONLY' {} \;
}
