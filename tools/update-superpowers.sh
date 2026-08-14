#!/bin/sh
set -eu

TAG="${1:-}"
printf '%s\n' "$TAG" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$' \
  || { echo "usage: $0 <vX.Y.Z>" >&2; exit 2; }
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUB="$ROOT/tools/superpowers"

gitdir="$(git -C "$ROOT" rev-parse --path-format=absolute --git-dir)"
common="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir)"
[ "$gitdir" != "$common" ] || { echo "update-superpowers: use a clean isolated worktree" >&2; exit 1; }
[ -z "$(git -C "$ROOT" status --porcelain)" ] || { echo "update-superpowers: worktree must be clean" >&2; exit 1; }
[ -e "$SUB/.git" ] || { echo "update-superpowers: initialize the submodule first" >&2; exit 1; }
[ -z "$(git -C "$SUB" status --porcelain)" ] || { echo "update-superpowers: submodule must be clean" >&2; exit 1; }

old="$(git -C "$SUB" rev-parse HEAD)"
old_version="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$SUB/package.json" | head -1)"
git -C "$SUB" fetch --no-tags origin "refs/tags/$TAG:refs/tags/$TAG"
candidate="$(git -C "$SUB" rev-parse "$TAG^{commit}")"
[ -n "$candidate" ] || { echo "update-superpowers: tag does not peel to a commit" >&2; exit 1; }
git -C "$SUB" checkout --detach "$candidate"

version="${TAG#v}"
versions="$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
  "$SUB/.claude-plugin/plugin.json" "$SUB/.claude-plugin/marketplace.json" \
  "$SUB/.codex-plugin/plugin.json" "$SUB/package.json" | sort -u)"
if [ "$versions" != "$version" ]; then
  git -C "$SUB" checkout --detach "$old" >/dev/null
  echo "update-superpowers: manifests report '${versions:-none}', expected '$version'" >&2
  exit 1
fi
if [ "$version" = "$old_version" ] && [ "$candidate" != "$old" ]; then
  git -C "$SUB" checkout --detach "$old" >/dev/null
  echo "update-superpowers: refusing changed source with unchanged version $version" >&2
  exit 1
fi

# The documented upgrade path used to leave every pin behind, so the very next
# command it told you to run — ci/verify-superpowers.sh full — failed against
# constants three files away. The lock is the one place they live now, and this
# is what moves them.
cat >"$ROOT/ci/superpowers.lock" <<LOCK
# Provenance for the developer-agent plugin submodule at tools/superpowers.
# Never hand-edit — run: tools/update-superpowers.sh <vX.Y.Z> on a clean
# isolated worktree, which rewrites this file and then asks you to review it.
pin=$candidate
tag=$TAG
version=$version
LOCK
echo "Rewrote ci/superpowers.lock to $TAG at $candidate"

echo "Candidate $TAG"
git -C "$SUB" show --no-patch --format='  %H%n  %s%n  authored %aI' "$candidate"
echo "Upstream change from $old:"
git -C "$SUB" diff --stat "$old..$candidate"
echo "Candidate is detached and un-staged. Review it, run ci/verify-superpowers.sh full,"
echo "then stage tools/superpowers AND ci/superpowers.lock. After merge, rerun tools/setup-agents.sh."
