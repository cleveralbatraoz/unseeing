#!/bin/sh
# Compose the actual git-archive mechanism with the archive-aware pipeline
# gate. This catches either half drifting: a developer tool leaking into the
# deployment, or the executable gate/test disappearing from it.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TREEISH="${DEPLOY_ARCHIVE_TREEISH:-HEAD}"
FAIL=0

ok() { echo "deploy-archive: OK   $1"; }
bad() { echo "deploy-archive: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}
contains() { case "$2" in *"$1"*) return 0 ;; *) return 1 ;; esac; }

git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
  echo "deploy-archive: SKIP no git metadata — already inside a deployment archive"
  exit 0
}

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP
git -C "$ROOT" archive --worktree-attributes "$TREEISH" | tar -x -C "$T"

require "the deployment archive omits .gitmodules" test ! -e "$T/.gitmodules"
require "the deployment archive omits tools/setup-agents.sh" \
  test ! -e "$T/tools/setup-agents.sh"
require "the deployment archive omits test/setup_agents_test.sh" \
  test ! -e "$T/test/setup_agents_test.sh"
require "the archive retains the pipeline's agent-tooling gate" \
  test -x "$T/ci/run_agent_tooling_self_test.sh"
require "the archive retains the gate's behavioral test" \
  test -x "$T/test/ci_agent_tooling_gate_test.sh"

status=0
output="$("$T/ci/run_agent_tooling_self_test.sh" "$T" 2>&1)" || status=$?
require "the exact deployment archive passes the optional-tool gate" \
  test "$status" -eq 0
require "the exact deployment archive announces the skip" contains 'SKIP' "$output"

exit "$FAIL"
