#!/bin/sh
# Run developer-agent behavior only in a complete checkout. `.gitmodules` and
# the agent entry points are exported out together, so its absence is the
# explicit deployment-archive marker rather than an ambient git command.
set -eu

ROOT="${1:-}"
[ -n "$ROOT" ] && [ -d "$ROOT" ] || {
  echo "ci: agent-plugin scope self-test FAILED — repository root is missing"
  exit 1
}

SETUP="$ROOT/tools/setup-agents.sh"
TEST="$ROOT/test/setup_agents_test.sh"

if [ ! -e "$ROOT/.gitmodules" ]; then
  if [ -e "$SETUP" ] || [ -e "$TEST" ]; then
    echo "ci: agent-plugin scope self-test FAILED — developer tooling leaked into the deployment archive"
    exit 1
  fi
  echo "ci: agent-plugin scope self-test SKIP (developer tooling absent from deployment archive)"
  exit 0
fi

[ -x "$SETUP" ] || {
  echo "ci: agent-plugin scope self-test FAILED — $SETUP is missing or not executable"
  exit 1
}
[ -x "$TEST" ] || {
  echo "ci: agent-plugin scope self-test FAILED — $TEST is missing or not executable"
  exit 1
}

SETUP_AGENTS_SUBJECT="$SETUP" "$TEST"
