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
