#!/bin/sh
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
# The pin under test comes from the lock the update path rewrites, so this
# suite cannot drift away from what it is meant to be verifying.
SP_PIN="$(sed -n 's/^pin=//p' "$ROOT/ci/superpowers.lock" | head -1)"
[ -n "$SP_PIN" ] || { echo "verify-superpowers-test: ci/superpowers.lock has no pin" >&2; exit 1; }
VERIFY="$ROOT/ci/verify-superpowers.sh"

# Every fixture below is built by copying the repo's real `.gitmodules`, and
# `.gitattributes` marks that file `export-ignore` — so it is absent by design
# from the tar extract the droplet's post-receive hook deploys, and the copy
# died there under `set -e`, failing the whole pipeline. The verifier this
# script tests already handles that tree: with no repository metadata it
# asserts the developer tooling is ABSENT and passes. There is nothing left
# for the self-test to prove once the tooling it guards cannot be present, so
# it steps aside in the same voice `test/repo_hygiene.sh` uses for the same
# reason. (The stronger shape is to synthesise the fixture's `.gitmodules`
# from a hand-written literal instead of copying the real one, which would
# keep these cases running everywhere; that is a redesign of what the fixture
# IS, and this is a deploy-blocking hotfix.)
[ -f "$ROOT/.gitmodules" ] || {
  echo "verify-superpowers-test: SKIP no .gitmodules — deploy work tree is a tar extract"
  exit 0
}

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
  # The lock is what the verifier reads its pin from, so it is part of the
  # fixture. Copied rather than written, so this suite exercises the real file.
  mkdir -p "$repo/ci"
  cp "$ROOT/ci/superpowers.lock" "$repo/ci/superpowers.lock"
  git -C "$repo" add .gitmodules verify.sh ci/superpowers.lock
  git -C "$repo" update-index --add --cacheinfo 160000,$SP_PIN,tools/superpowers
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
git -C "$TMP/extra" update-index --add --cacheinfo 160000,$SP_PIN,vendor/embedded
expect 1 "rejects a second gitlink" env SUPERPOWERS_ROOT="$TMP/extra" "$TMP/extra/verify.sh" metadata

mkdir -p "$TMP/archive"
cp "$VERIFY" "$TMP/archive/verify.sh"
expect 0 "accepts a developer-tool-free archive" env SUPERPOWERS_ROOT="$TMP/archive" "$TMP/archive/verify.sh" metadata
mkdir -p "$TMP/archive/tools/superpowers"
expect 1 "rejects leaked payload in an archive" env SUPERPOWERS_ROOT="$TMP/archive" "$TMP/archive/verify.sh" metadata

exit "$FAIL"
