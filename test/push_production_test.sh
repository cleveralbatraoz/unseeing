#!/bin/sh
# Behavioral contract for choosing the production ref. A failed post-receive
# advances production/main before the hook reports failure, so an identical
# retry needs a fresh transient ref rather than another no-op main push.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUBJECT="${PUSH_PRODUCTION_SUBJECT:-$ROOT/ci/push_production.sh}"
FAIL=0

ok() { echo "push-production: OK   $1"; }
bad() { echo "push-production: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}
contains() { case "$2" in *"$1"*) return 0 ;; *) return 1 ;; esac; }

[ -f "$SUBJECT" ] || {
  echo "push-production: FAIL $SUBJECT does not exist"
  exit 1
}

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP
BIN="$T/bin"
REPO="$T/repo with spaces"
LOG="$T/git.log"
mkdir -p "$BIN" "$REPO"

cat >"$BIN/git" <<'EOF'
#!/bin/sh
if [ "$#" -eq 5 ] && [ "$1" = "-C" ] && [ "$3" = "ls-remote" ] \
  && [ "$4" = "production" ] && [ "$5" = "refs/heads/main" ]; then
  [ "${GIT_LS_REMOTE_FAIL:-0}" = 0 ] || exit 7
  [ -n "${REMOTE_SHA:-}" ] \
    && printf '%s\trefs/heads/main\n' "$REMOTE_SHA"
  exit 0
fi
if [ "$#" -eq 5 ] && [ "$1" = "-C" ] && [ "$3" = "push" ] \
  && [ "$4" = "production" ]; then
  printf '%s\n' "$5" >>"$PUSH_LOG"
  [ "${GIT_PUSH_FAIL:-0}" = 0 ] || exit 8
  exit 0
fi
exit 9
EOF
chmod +x "$BIN/git"

HEAD_SHA=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
OTHER_SHA=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb

: >"$LOG"
status=0
output="$(PATH="$BIN" PUSH_LOG="$LOG" REMOTE_SHA="$OTHER_SHA" \
  DEPLOY_ATTEMPT_ID=fixture-one "$SUBJECT" "$REPO" "$HEAD_SHA" 2>&1)" \
  || status=$?
require "a new commit uses the ordinary production main push" test "$status" -eq 0
require "the ordinary push carries only main" test "$(cat "$LOG")" = main
require "the ordinary branch is announced" contains 'production/main' "$output"

: >"$LOG"
status=0
output="$(PATH="$BIN" PUSH_LOG="$LOG" REMOTE_SHA="$HEAD_SHA" \
  DEPLOY_ATTEMPT_ID=fixture-two "$SUBJECT" "$REPO" "$HEAD_SHA" 2>&1)" \
  || status=$?
require "an identical production main uses a retry ref" test "$status" -eq 0
require "the retry ref carries the exact main commit" \
  test "$(cat "$LOG")" = \
    'main:refs/heads/deploy-retry/aaaaaaaaa-fixture-two'
require "the retry branch is announced" contains 'retry trigger' "$output"

: >"$LOG"
status=0
PATH="$BIN" PUSH_LOG="$LOG" REMOTE_SHA="$HEAD_SHA" \
  DEPLOY_ATTEMPT_ID='slash/is-not-a-ref-component' \
  "$SUBJECT" "$REPO" "$HEAD_SHA" >/dev/null 2>&1 || status=$?
require "an unsafe retry-attempt identifier is refused" test "$status" -eq 2
require "an unsafe identifier never reaches git push" test ! -s "$LOG"

: >"$LOG"
status=0
PATH="$BIN" PUSH_LOG="$LOG" REMOTE_SHA="$OTHER_SHA" \
  GIT_PUSH_FAIL=1 "$SUBJECT" "$REPO" "$HEAD_SHA" >/dev/null 2>&1 || status=$?
require "a production push failure propagates" test "$status" -eq 8

status=0
PATH="$BIN" PUSH_LOG="$LOG" REMOTE_SHA="$OTHER_SHA" \
  "$SUBJECT" "$REPO" not-a-commit >/dev/null 2>&1 || status=$?
require "a malformed commit is refused before remote inspection" test "$status" -eq 2

exit "$FAIL"
