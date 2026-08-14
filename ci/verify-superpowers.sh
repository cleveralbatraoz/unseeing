#!/bin/sh
set -eu

MODE="${1:-}"
case "$MODE" in metadata|full) ;; *) echo "usage: $0 metadata|full" >&2; exit 2 ;; esac
ROOT="${SUPERPOWERS_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
# One home for the pin, the tag and the version: ci/superpowers.lock, which
# tools/update-superpowers.sh rewrites. They used to be spelled out in four
# files the update path never touched.
sp_lock_get() { sed -n "s/^$1=//p" "$2" | head -1; }
LOCK="$ROOT/ci/superpowers.lock"
PATH_PIN=tools/superpowers
URL=https://github.com/obra/superpowers.git

fail() { echo "superpowers: FAILED: $*" >&2; exit 1; }

if ! git -C "$ROOT" rev-parse --git-dir >/dev/null 2>&1; then
  [ ! -e "$ROOT/.gitmodules" ] || fail ".gitmodules exists without repository metadata"
  [ ! -e "$ROOT/$PATH_PIN" ] || fail "$PATH_PIN leaked into an exported tree"
  echo "superpowers: metadata OK (developer tooling absent from archive)"
  exit 0
fi

# Read after the archive check, never before: an exported tree carries no
# submodule to pin, so demanding the lock there would fail the one case that
# proves developer tooling stayed out of the deploy.
[ -f "$LOCK" ] || fail "no lock at $LOCK"
PIN="$(sp_lock_get pin "$LOCK")"
TAG="$(sp_lock_get tag "$LOCK")"
SP_VERSION="$(sp_lock_get version "$LOCK")"
[ -n "$PIN" ] && [ -n "$TAG" ] && [ -n "$SP_VERSION" ] \
  || fail "$LOCK is missing pin, tag or version"

[ -f "$ROOT/.gitmodules" ] || fail ".gitmodules is missing"
paths="$(git -C "$ROOT" config -f .gitmodules --get-regexp '^submodule\..*\.path$' 2>/dev/null | awk '{print $2}')"
[ "$paths" = "$PATH_PIN" ] || fail "$PATH_PIN must be the sole submodule path (found: ${paths:-none})"
[ "$(git -C "$ROOT" config -f .gitmodules --get submodule.tools/superpowers.url)" = "$URL" ] || fail "unexpected submodule URL"
if git -C "$ROOT" config -f .gitmodules --get-regexp '^submodule\..*\.(branch|update)$' >/dev/null 2>&1; then
  fail "branch and update overrides are forbidden"
fi
links="$(git -C "$ROOT" ls-files -s | awk '$1 == "160000" {print $4}')"
[ "$links" = "$PATH_PIN" ] || fail "$PATH_PIN must be the sole mode-160000 entry (found: ${links:-none})"
[ "$(git -C "$ROOT" ls-files -s "$PATH_PIN" | awk '{print $2}')" = "$PIN" ] || fail "gitlink is not pinned to $PIN"
echo "superpowers: metadata OK"
[ "$MODE" = metadata ] && exit 0

[ -e "$ROOT/$PATH_PIN/.git" ] || fail "submodule is not initialized"
[ -z "$(git -C "$ROOT/$PATH_PIN" status --porcelain)" ] || fail "submodule is dirty"
[ "$(git -C "$ROOT/$PATH_PIN" rev-parse HEAD)" = "$PIN" ] || fail "initialized HEAD does not match gitlink"
[ -z "$(git -C "$ROOT/$PATH_PIN" ls-files -s | awk '$1 == "160000" {print $4}')" ] || fail "nested submodules are forbidden"
[ "$(git -C "$ROOT/$PATH_PIN" describe --tags --exact-match HEAD 2>/dev/null)" = "$TAG" ] || fail "$TAG does not name HEAD"

versions="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
  "$ROOT/$PATH_PIN/.claude-plugin/plugin.json" \
  "$ROOT/$PATH_PIN/.claude-plugin/marketplace.json" \
  "$ROOT/$PATH_PIN/.codex-plugin/plugin.json" \
  "$ROOT/$PATH_PIN/package.json" | sort -u)"
[ "$versions" = "$SP_VERSION" ] || fail "upstream manifests disagree with version $SP_VERSION (found: $versions)"

for skill in brainstorming dispatching-parallel-agents executing-plans \
  finishing-a-development-branch receiving-code-review requesting-code-review \
  subagent-driven-development systematic-debugging test-driven-development \
  using-git-worktrees using-superpowers verification-before-completion \
  writing-plans writing-skills; do
  [ -s "$ROOT/$PATH_PIN/skills/$skill/SKILL.md" ] || fail "missing shared skill: $skill"
done
for file in .claude-plugin/plugin.json .claude-plugin/marketplace.json hooks/hooks.json \
  hooks/session-start .codex-plugin/plugin.json .agents/plugins/marketplace.json; do
  [ -s "$ROOT/$PATH_PIN/$file" ] || fail "missing host integration file: $file"
done
echo "superpowers: full verification OK ($TAG at $PIN)"
