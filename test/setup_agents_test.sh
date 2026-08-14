#!/bin/sh
# Behavioral contract for how tools/setup-agents.sh classifies a competing
# Superpowers installation.
#
# Two defects this pins. The migrate path uninstalled with a hardcoded
# `--scope user`, so a plugin installed in local scope could not be removed at
# all — the CLI refuses and names the scope it actually wants. And a local- or
# project-scoped plugin belongs to ONE project: treating another repository's
# plugin as this repository's conflict would either block setup over something
# that cannot load here, or, with --migrate, quietly uninstall a plugin from
# somebody's unrelated checkout.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUBJECT="${SETUP_AGENTS_SUBJECT:-$ROOT/tools/setup-agents.sh}"
FAIL=0

ok() { echo "setup-agents: OK   $1"; }
bad() { echo "setup-agents: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}
refute() {
  label="$1"
  shift
  if "$@"; then bad "$label"; else ok "$label"; fi
}

[ -f "$SUBJECT" ] || { echo "setup-agents: FAIL $SUBJECT does not exist"; exit 1; }

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP
HERE="$T/this repo"
OTHER="$T/some other repo"
mkdir -p "$HERE" "$OTHER"

# Sourced for its functions only — the same seam tools/bootstrap.ps1 offers
# through -NoRun. Without it the worktree check and the submodule fetch run
# before a single assertion could.
UNSEEING_SETUP_AGENTS_NORUN=1
export UNSEEING_SETUP_AGENTS_NORUN
# shellcheck source=/dev/null
. "$SUBJECT"
# main() sources this before dispatching, so a test driving setup_claude
# directly has to supply it too — check_skills exits the shell without it.
# shellcheck source=/dev/null
. "$ROOT/tools/lib/digest.sh"

classify() { printf '%s' "$1" | plugin_conflicts "$HERE"; }

PINNED='[{"id":"superpowers@superpowers-dev","version":"6.3.0","scope":"user","enabled":true}]'
require "the pinned installation is never its own conflict" \
  test -z "$(classify "$PINNED")"

USER_SCOPED='[{"id":"superpowers@claude-plugins-official","version":"6.2.0","scope":"user","enabled":true}]'
require "a user-scoped competitor blocks" \
  test "$(classify "$USER_SCOPED")" = "block	superpowers@claude-plugins-official	user	"

# The case that produced the report: installed in local scope, and belonging to
# a different checkout entirely. It cannot load here, so it is not this
# repository's business to refuse over it — and certainly not to delete it.
ELSEWHERE="[{\"id\":\"superpowers@claude-plugins-official\",\"version\":\"6.2.0\",\"scope\":\"local\",\"enabled\":false,\"projectPath\":\"$OTHER\"}]"
require "a local-scoped competitor in ANOTHER project does not block" \
  test "$(classify "$ELSEWHERE")" = "elsewhere	superpowers@claude-plugins-official	local	$OTHER"

MINE="[{\"id\":\"superpowers@claude-plugins-official\",\"version\":\"6.2.0\",\"scope\":\"local\",\"enabled\":true,\"projectPath\":\"$HERE\"}]"
require "a local-scoped competitor in THIS project blocks" \
  test "$(classify "$MINE")" = "block	superpowers@claude-plugins-official	local	$HERE"

# An unknown or absent scope is treated as competing: assuming otherwise would
# let a plugin this script cannot reason about load beside the pinned one.
NOSCOPE='[{"id":"superpowers@somewhere-else","version":"1.0.0","enabled":true}]'
require "a competitor with no scope reported still blocks" \
  test "$(classify "$NOSCOPE")" = "block	superpowers@somewhere-else	user	"

# Plugins that are not Superpowers at all are none of this script's business.
UNRELATED='[{"id":"something-else@a-marketplace","version":"1.0.0","scope":"user","enabled":true}]'
require "an unrelated plugin is not a Superpowers conflict" \
  test -z "$(classify "$UNRELATED")"

MIXED="[{\"id\":\"superpowers@superpowers-dev\",\"scope\":\"user\",\"enabled\":true},
{\"id\":\"superpowers@a\",\"scope\":\"user\",\"enabled\":true},
{\"id\":\"superpowers@b\",\"scope\":\"local\",\"enabled\":false,\"projectPath\":\"$OTHER\"}]"
require "a mixed list separates the blocker from the bystander" \
  test "$(classify "$MIXED" | cut -f1 | tr '\n' ' ')" = "block elsewhere "

# Malformed input must not be read as "no conflicts" — that is the same silent
# agreement tools/lib/digest.sh exists to refuse, one layer up.
status=0
printf '%s' 'not json at all' | plugin_conflicts "$HERE" >/dev/null 2>&1 || status=$?
refute "unreadable plugin output is refused, not read as 'nothing competing'" \
  test "$status" -eq 0

# --- and the command it actually issues ------------------------------------
# The classifier knowing the scope is only half of it. `claude plugin uninstall
# --scope user` against a local install is refused by the CLI with "installed in
# local scope, not user", which is exactly what a real --migrate run hit. The
# fake below records its argv so the scope on the wire can be asserted.
FAKEBIN="$T/fake bin"
CALLS="$T/claude-calls.log"
mkdir -p "$FAKEBIN" "$HERE/tools/superpowers/skills" "$T/cache/skills"
printf 'pinned skill\n' >"$HERE/tools/superpowers/skills/a.md"
printf 'pinned skill\n' >"$T/cache/skills/a.md"

cat >"$FAKEBIN/claude" <<'FAKE'
#!/bin/sh
printf 'claude %s\n' "$*" >>"$CLAUDE_TEST_CALLS"
case "$*" in
  "plugin list --json")
    # Before the uninstall, the competitor is present; afterwards only the pin.
    if [ -f "$CLAUDE_TEST_REMOVED" ]; then
      cat "$CLAUDE_TEST_AFTER"
    else
      cat "$CLAUDE_TEST_BEFORE"
    fi
    ;;
  "plugin marketplace list --json") printf '[]\n' ;;
  # Either verb advances the fake's state to "the pin is what is installed":
  # a run with nothing to remove still installs, and the checks that follow
  # read the list again.
  "plugin uninstall "*|"plugin install "*) : >"$CLAUDE_TEST_REMOVED" ;;
  *) : ;;
esac
exit 0
FAKE
chmod +x "$FAKEBIN/claude"

printf '[{"id":"superpowers@claude-plugins-official","scope":"local","enabled":true,"projectPath":"%s"}]\n' \
  "$HERE" >"$T/before.json"
printf '[{"id":"superpowers@superpowers-dev","version":"6.3.0","scope":"user","enabled":true,"installPath":"%s"}]\n' \
  "$T/cache" >"$T/after.json"

run_migrate() {
  : >"$CALLS"
  rm -f "$T/removed"
  MIGRATE=1
  ROOT="$HERE"
  SUB="$HERE/tools/superpowers"
  VERSION=6.3.0
  status=0
  CLAUDE_TEST_CALLS="$CALLS" CLAUDE_TEST_BEFORE="$T/before.json" \
    CLAUDE_TEST_AFTER="$T/after.json" CLAUDE_TEST_REMOVED="$T/removed" \
    PATH="$FAKEBIN:$PATH" setup_claude >"$T/migrate.out" 2>&1 || status=$?
}

run_migrate
require "a --migrate run completes against a local-scoped competitor" test "$status" -eq 0
require "the uninstall names the scope the plugin reports, not 'user'" \
  grep -qx 'claude plugin uninstall superpowers@claude-plugins-official --scope local' "$CALLS"
refute "the uninstall never guesses --scope user" \
  grep -q -- '--scope user' "$CALLS"

# A competitor belonging to another checkout must survive --migrate untouched:
# removing it would delete a plugin out of a project this repository knows
# nothing about.
printf '[{"id":"superpowers@claude-plugins-official","scope":"local","enabled":false,"projectPath":"%s"}]\n' \
  "$OTHER" >"$T/before.json"
run_migrate
require "another project's plugin survives --migrate" test "$status" -eq 0
refute "another project's plugin is never uninstalled" \
  grep -q 'plugin uninstall' "$CALLS"
require "another project's plugin is at least mentioned" \
  grep -q "left alone" "$T/migrate.out"

exit "$FAIL"
