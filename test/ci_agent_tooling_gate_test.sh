#!/bin/sh
# Behavioral contract for the pipeline boundary between a complete developer
# checkout and the git-exported deployment tree. Developer-agent tooling must
# be tested where it exists and must never become a production dependency.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUBJECT="${AGENT_TOOLING_GATE_SUBJECT:-$ROOT/ci/run_agent_tooling_self_test.sh}"
FAIL=0

ok() { echo "agent-tooling-gate: OK   $1"; }
bad() { echo "agent-tooling-gate: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}
contains() { case "$2" in *"$1"*) return 0 ;; *) return 1 ;; esac; }

[ -f "$SUBJECT" ] || {
  echo "agent-tooling-gate: FAIL $SUBJECT does not exist"
  exit 1
}

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP

fixture() {
  root="$1"
  mkdir -p "$root/tools" "$root/test"
  cat >"$root/tools/setup-agents.sh" <<'EOF'
#!/bin/sh
exit 0
EOF
  cat >"$root/test/setup_agents_test.sh" <<'EOF'
#!/bin/sh
printf '%s\n' ran >"$AGENT_TOOLING_TEST_MARKER"
EOF
  chmod +x "$root/tools/setup-agents.sh" "$root/test/setup_agents_test.sh"
}

CHECKOUT="$T/complete checkout"
fixture "$CHECKOUT"
printf '%s\n' '[submodule "tools/superpowers"]' >"$CHECKOUT/.gitmodules"
marker="$T/checkout-ran"
status=0
output="$(AGENT_TOOLING_TEST_MARKER="$marker" "$SUBJECT" "$CHECKOUT" 2>&1)" || status=$?
require "a complete checkout runs the developer-agent behavioral test" \
  test "$status" -eq 0
require "the checkout test crossed its real executable boundary" \
  test "$(cat "$marker" 2>/dev/null || true)" = ran

ARCHIVE="$T/deployment archive"
fixture "$ARCHIVE"
rm "$ARCHIVE/tools/setup-agents.sh" "$ARCHIVE/test/setup_agents_test.sh"
marker="$T/archive-did-not-run"
status=0
output="$(AGENT_TOOLING_TEST_MARKER="$marker" "$SUBJECT" "$ARCHIVE" 2>&1)" || status=$?
require "a deployment archive skips the checkout-only test" test "$status" -eq 0
require "the archive skip is explicit" contains 'SKIP' "$output"
require "the omitted subject is never executed indirectly" test ! -e "$marker"

BROKEN="$T/broken checkout"
fixture "$BROKEN"
printf '%s\n' '[submodule "tools/superpowers"]' >"$BROKEN/.gitmodules"
rm "$BROKEN/tools/setup-agents.sh"
status=0
output="$("$SUBJECT" "$BROKEN" 2>&1)" || status=$?
require "a checkout missing its tracked setup tool is refused" test "$status" -eq 1
require "the broken-checkout refusal names setup-agents" \
  contains 'tools/setup-agents.sh' "$output"

LEAK="$T/leaking archive"
fixture "$LEAK"
status=0
output="$("$SUBJECT" "$LEAK" 2>&1)" || status=$?
require "an archive leaking developer tooling is refused" test "$status" -eq 1
require "the leak refusal explains the export boundary" contains 'leaked' "$output"

TEST_LEAK="$T/archive leaking only the developer test"
fixture "$TEST_LEAK"
rm "$TEST_LEAK/tools/setup-agents.sh"
status=0
output="$("$SUBJECT" "$TEST_LEAK" 2>&1)" || status=$?
require "an archive leaking only the developer test is refused" test "$status" -eq 1

NO_TEST="$T/checkout without its test"
fixture "$NO_TEST"
printf '%s\n' '[submodule "tools/superpowers"]' >"$NO_TEST/.gitmodules"
rm "$NO_TEST/test/setup_agents_test.sh"
status=0
output="$("$SUBJECT" "$NO_TEST" 2>&1)" || status=$?
require "a checkout missing the behavioral test is refused" test "$status" -eq 1
require "the missing-test refusal names setup_agents_test" \
  contains 'test/setup_agents_test.sh' "$output"

exit "$FAIL"
