#!/bin/sh
# Enforce the permanent engine/content split: shipped behavior is Rust;
# first-party GDScript exists only under game/tests/ for suites, fixtures,
# probes, and compatibility shims. Third-party addons and the import cache are
# outside this ownership rule.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
. "$DIR/ci/gdscript_files.sh"

VIOLATIONS="$(gdscript_policy_violations "$DIR")"
RESOURCE_VIOLATIONS="$(gdscript_resource_policy_violations "$DIR")"
SHADER_VIOLATIONS="$(shader_placement_violations "$DIR")"
if [ -n "$SHADER_VIOLATIONS" ]; then
  echo "ci: shader placement FAILED — a probe-only shader is in the shipped tree:" >&2
  printf '%s\n' "$SHADER_VIOLATIONS" | sed 's/^/ci:   /' >&2
  echo "ci: export_presets.cfg exports all_resources and excludes only tests/," >&2
  echo "ci: addons/ and reports/, so these would be packed into every build." >&2
  echo "ci: move them under game/tests/probe/shaders/ with the scenes that use them" >&2
  exit 1
fi
if [ -n "$VIOLATIONS" ] || [ -n "$RESOURCE_VIOLATIONS" ]; then
  echo "ci: GDScript placement FAILED — first-party code is tests/probes only:" >&2
  [ -z "$VIOLATIONS" ] || printf '%s\n' "$VIOLATIONS" | sed 's/^/ci:   /' >&2
  [ -z "$RESOURCE_VIOLATIONS" ] \
    || printf '%s\n' "$RESOURCE_VIOLATIONS" | sed 's/^/ci:   /' >&2
  echo "ci: move test code under game/tests/ or implement shipped behavior in Rust" >&2
  echo "ci: production Godot resources must be text (.tscn/.tres), inspectable, and code-free" >&2
  exit 1
fi

echo "ci: GDScript placement OK (first-party scripts and embedded code are tests/probes only)"
echo "ci: shader placement OK (no probe-only shader in the shipped resource tree)"
