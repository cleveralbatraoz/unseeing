#!/bin/sh
# Repository hygiene gate — the invariants that keep this a source-only repo.
#
# The 2026-08-04 binary audit found the tree already clean (one PNG type, zero
# churn, nothing shipped) and rejected git-lfs as a net loss. What it did find
# was that nothing PREVENTS the next big file: a 109 MB export walked straight
# through the pre-commit hook. These checks are that prevention, plus the
# ignore rules whose absence would let an agent worktree be committed.
#
# Pure POSIX sh, no network, no Godot — runs anywhere ci/pipeline.sh runs.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

ok() { echo "hygiene: OK   $1"; }
bad() { echo "hygiene: FAIL $1"; FAIL=1; }

# --- ignore rules -----------------------------------------------------------
# .claude/ holds per-session agent worktrees. Untracked AND unignored, a
# `git add -A` stages them as embedded-repo gitlinks with no .gitmodules:
# they clone as empty directories and `git submodule status` exits 128.
for p in .claude/settings.json .claude/worktrees/some-task/README.md; do
  if git -C "$DIR" check-ignore -q "$p" 2>/dev/null; then
    ok "$p is ignored"
  else
    bad "$p is NOT ignored (add .claude/ to .gitignore)"
  fi
done

exit "$FAIL"
