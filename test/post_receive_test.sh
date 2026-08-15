#!/bin/sh
# End-to-end behavioral contract for the versioned bare-repository hook.
# Ordinary main updates deploy once; retry refs deploy the identical commit
# again and are always removed; unrelated refs remain inert.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOK="${POST_RECEIVE_SUBJECT:-$ROOT/infra/post-receive}"
FAIL=0

ok() { echo "post-receive: OK   $1"; }
bad() { echo "post-receive: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}

[ -f "$HOOK" ] || {
  echo "post-receive: FAIL $HOOK does not exist"
  exit 1
}

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP
FIX_HOME="$T/home"
BARE="$FIX_HOME/git/unseeing.git"
SOURCE="$T/source"
LOG="$FIX_HOME/pipeline.log"
mkdir -p "$FIX_HOME/git" "$SOURCE/ci" "$SOURCE/rust"
git init --bare -q "$BARE"
git init -q "$SOURCE"
git -C "$SOURCE" config user.name 'Hook Fixture'
git -C "$SOURCE" config user.email 'hook@example.invalid'

cat >"$SOURCE/ci/pipeline.sh" <<'EOF'
#!/bin/sh
set -eu
[ "${PREBUILT_RUST:-}" = 1 ]
[ -n "${BUILD_SHA:-}" ]
[ -L rust/target ]
[ -z "${GIT_DIR+x}" ]
printf '%s\n' "$BUILD_SHA" >>"$HOME/pipeline.log"
[ "${HOOK_PIPELINE_FAIL:-0}" = 0 ]
EOF
chmod +x "$SOURCE/ci/pipeline.sh"
printf '%s\n' fixture >"$SOURCE/rust/Cargo.toml"
git -C "$SOURCE" add ci/pipeline.sh rust/Cargo.toml
git -C "$SOURCE" commit -qm 'fixture tree'
git -C "$SOURCE" branch -M main
FULL_SHA="$(git -C "$SOURCE" rev-parse HEAD)"
SHORT_SHA="$(printf %.9s "$FULL_SHA")"

cp "$HOOK" "$BARE/hooks/post-receive"
chmod +x "$BARE/hooks/post-receive"

HOME="$FIX_HOME" git -C "$SOURCE" push -q "$BARE" main
require "an ordinary main update runs the pipeline once" \
  test "$(wc -l <"$LOG" | tr -d ' ')" -eq 1
require "the ordinary update hands the exact short commit to the pipeline" \
  test "$(sed -n '1p' "$LOG")" = "$SHORT_SHA"

HOME="$FIX_HOME" git -C "$SOURCE" push -q "$BARE" \
  "main:refs/heads/deploy-retry/fixture-success"
require "a retry ref rebuilds the identical commit" \
  test "$(wc -l <"$LOG" | tr -d ' ')" -eq 2
require "the retry ref is deleted after success" \
  test -z "$(git --git-dir="$BARE" show-ref refs/heads/deploy-retry/fixture-success || true)"

HOME="$FIX_HOME" git -C "$SOURCE" push -q "$BARE" \
  "main:refs/heads/unrelated"
require "an unrelated ref never runs the deployment pipeline" \
  test "$(wc -l <"$LOG" | tr -d ' ')" -eq 2
require "an unrelated ref is not deleted" \
  test -n "$(git --git-dir="$BARE" show-ref refs/heads/unrelated)"

HOME="$FIX_HOME" HOOK_PIPELINE_FAIL=1 git -C "$SOURCE" push -q "$BARE" \
  "main:refs/heads/deploy-retry/fixture-failure"
require "a failing retry still reaches the pipeline" \
  test "$(wc -l <"$LOG" | tr -d ' ')" -eq 3
require "the retry ref is deleted after failure" \
  test -z "$(git --git-dir="$BARE" show-ref refs/heads/deploy-retry/fixture-failure || true)"

exit "$FAIL"
