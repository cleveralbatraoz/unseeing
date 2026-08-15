#!/bin/sh
# Choose an ordinary production/main update or a transient retry trigger when
# main already names the requested commit. The server hook builds both through
# the identical archive and pipeline path.
set -eu

ROOT="${1:-}"
HEAD_SHA="${2:-}"
[ -n "$ROOT" ] && [ -d "$ROOT" ] || {
  echo "deploy: FAILED production push needs the repository root"
  exit 2
}
[ "${#HEAD_SHA}" -eq 40 ] || {
  echo "deploy: FAILED production push needs a full 40-digit commit"
  exit 2
}
case "$HEAD_SHA" in
  *[!0-9a-f]*)
    echo "deploy: FAILED production push received a malformed commit"
    exit 2
    ;;
esac

REMOTE_LINE="$(git -C "$ROOT" ls-remote production refs/heads/main)"
# ls-remote's first field is the object name. Word splitting is deliberate and
# bounded here: an absent main yields no words; a normal answer yields SHA/ref.
set -- $REMOTE_LINE
REMOTE_SHA="${1:-}"

if [ "$REMOTE_SHA" != "$HEAD_SHA" ]; then
  echo "deploy: updating production/main"
  git -C "$ROOT" push production main
  exit 0
fi

ATTEMPT="${DEPLOY_ATTEMPT_ID:-$(date -u +%Y%m%dT%H%M%SZ)-$$}"
case "$ATTEMPT" in
  ''|.*|*..*|*.|*.lock|*[!A-Za-z0-9._-]*)
    echo "deploy: FAILED unsafe retry-attempt identifier '$ATTEMPT'"
    exit 2
    ;;
esac
SHORT="$(printf %.9s "$HEAD_SHA")"
RETRY_REF="refs/heads/deploy-retry/$SHORT-$ATTEMPT"
echo "deploy: production/main already names $SHORT; sending retry trigger $RETRY_REF"
git -C "$ROOT" push production "main:$RETRY_REF"
