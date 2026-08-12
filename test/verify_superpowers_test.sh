#!/bin/sh
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
VERIFY="$ROOT/ci/verify-superpowers.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
FAIL=0

make_repo() {
  repo="$1"
  mkdir -p "$repo/tools"
  git init -q "$repo"
  git -C "$repo" config user.name test
  git -C "$repo" config user.email test@example.invalid
  cp "$VERIFY" "$repo/verify.sh"
  cp "$ROOT/.gitmodules" "$repo/.gitmodules"
  git -C "$repo" add .gitmodules verify.sh
  git -C "$repo" update-index --add --cacheinfo 160000,b36e0829c6d0140e93cfef2ca599b1b07d4a7797,tools/superpowers
}
expect() {
  want="$1" label="$2"; shift 2
  got=0
  "$@" >"$TMP/out" 2>&1 || got=$?
  if [ "$got" -eq "$want" ]; then echo "verify-superpowers-test: OK   $label"; else
    echo "verify-superpowers-test: FAIL $label (wanted $want, got $got)"; sed 's/^/  /' "$TMP/out"; FAIL=1
  fi
}

make_repo "$TMP/good"
expect 0 "accepts exact metadata" env SUPERPOWERS_ROOT="$TMP/good" "$TMP/good/verify.sh" metadata

make_repo "$TMP/url"
git -C "$TMP/url" config -f .gitmodules submodule.tools/superpowers.url https://example.invalid/fork.git
expect 1 "rejects a fork URL" env SUPERPOWERS_ROOT="$TMP/url" "$TMP/url/verify.sh" metadata

make_repo "$TMP/branch"
git -C "$TMP/branch" config -f .gitmodules submodule.tools/superpowers.branch main
expect 1 "rejects a floating branch" env SUPERPOWERS_ROOT="$TMP/branch" "$TMP/branch/verify.sh" metadata

make_repo "$TMP/extra"
git -C "$TMP/extra" update-index --add --cacheinfo 160000,b36e0829c6d0140e93cfef2ca599b1b07d4a7797,vendor/embedded
expect 1 "rejects a second gitlink" env SUPERPOWERS_ROOT="$TMP/extra" "$TMP/extra/verify.sh" metadata

mkdir -p "$TMP/archive"
cp "$VERIFY" "$TMP/archive/verify.sh"
expect 0 "accepts a developer-tool-free archive" env SUPERPOWERS_ROOT="$TMP/archive" "$TMP/archive/verify.sh" metadata
mkdir -p "$TMP/archive/tools/superpowers"
expect 1 "rejects leaked payload in an archive" env SUPERPOWERS_ROOT="$TMP/archive" "$TMP/archive/verify.sh" metadata

exit "$FAIL"
