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

# Every git question below is about a specific directory — $DIR, or the scratch
# repo the size guard is probed in — and is asked with `git -C`. An inherited
# GIT_DIR silently overrides all of that, so a caller's environment could aim
# these checks at a repository they were never about. A git hook is exactly
# such a caller: it exports GIT_DIR=. to its children. Ask from a clean slate.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_PREFIX \
      GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_QUARANTINE_PATH

ok() { echo "hygiene: OK   $1"; }
bad() { echo "hygiene: FAIL $1"; FAIL=1; }
skip() { echo "hygiene: SKIP $1"; }

# Half these checks interrogate the index, and the droplet's deploy work tree
# is a `git archive | tar -x` extract with NO git metadata (infra/post-receive).
# Run there unguarded they do not merely fail — `git ls-files` fatals to an
# EMPTY list, and an empty list satisfies "no file is too large", so the
# invariant reports OK while checking nothing. A vacuous pass is worse than a
# failure, so ask once and skip loudly rather than let git answer from nothing.
HAVE_INDEX=0
if git -C "$DIR" rev-parse --git-dir >/dev/null 2>&1; then HAVE_INDEX=1; fi

# --- ignore rules -----------------------------------------------------------
# .claude/ holds per-session agent worktrees. Untracked AND unignored, a
# `git add -A` stages them as embedded-repo gitlinks with no .gitmodules:
# they clone as empty directories and `git submodule status` exits 128.
if [ "$HAVE_INDEX" = 0 ]; then
  skip "ignore rules (no git metadata — deploy work tree is a tar extract)"
else
  for p in .claude/settings.json .claude/worktrees/some-task/README.md; do
    if git -C "$DIR" check-ignore -q "$p" 2>/dev/null; then
      ok "$p is ignored"
    else
      bad "$p is NOT ignored (add .claude/ to .gitignore)"
    fi
  done
fi

# --- pre-commit size guard --------------------------------------------------
# Exercised in a scratch repo, never here: the guard's whole job is to reject
# a commit, and the only honest way to prove it does is to stage something it
# must reject. The hook is read from this tree (not core.hooksPath) so the
# file under review is the file under test.
HOOK="$DIR/.githooks/pre-commit"
if [ ! -x "$HOOK" ]; then
  bad ".githooks/pre-commit missing or not executable"
else
  TMP="$(mktemp -d)"
  trap 'rm -rf "$TMP"' EXIT
  git init -q "$TMP/repo"

  # 6 MiB of zeroes: over the limit, and it compresses to nothing, proving
  # the guard measures the blob as staged rather than its packed cost.
  dd if=/dev/zero of="$TMP/repo/huge.bin" bs=1024 count=6144 2>/dev/null
  # a space in the name — the guard must not word-split its file list
  printf 'small\n' > "$TMP/repo/tiny file.txt"

  probe() { # probe <expected-exit> <label> [ALLOW_BIG value]
    want="$1"
    label="$2"
    allow="${3:-}"
    # `|| got=$?` keeps errexit from killing the suite: a non-zero exit is
    # the expected result of half these cases, not a failure of the harness.
    got=0
    (
      cd "$TMP/repo" || exit 99
      ALLOW_BIG="$allow" sh "$HOOK" >"$TMP/out" 2>&1
    ) || got=$?
    if [ "$got" -eq "$want" ]; then
      ok "$label"
    else
      bad "$label (expected exit $want, got $got)"
      sed 's/^/hygiene:      /' "$TMP/out"
    fi
  }

  git -C "$TMP/repo" add "tiny file.txt"
  probe 0 "size guard passes a small file"

  git -C "$TMP/repo" add huge.bin
  probe 1 "size guard rejects a 6 MiB staged file"
  if grep -q 'huge.bin' "$TMP/out"; then
    ok "size guard names the offending file"
  else
    bad "size guard does not name the offending file"
  fi

  probe 0 "size guard yields to ALLOW_BIG=1" 1
fi

# The guard only stops NEW oversize files; this is the standing invariant it
# protects. Also the audit's revisit trigger — if this ever legitimately
# fails, the binary-asset question is worth reopening.
# One cat-file for the whole index rather than one per file: --batch-check
# reads object names on stdin, so 600+ blobs cost a single process.
if [ "$HAVE_INDEX" = 0 ]; then
  skip "tracked-file size invariant (no git metadata)"
else
  big="$(git -C "$DIR" ls-files -s \
    | awk '$1 != "160000" { print $2 }' \
    | sort -u \
    | git -C "$DIR" cat-file --batch-check='%(objectname) %(objectsize)' \
    | awk '$2 > 5242880 { print $1 }')"
  if [ -z "$big" ]; then
    ok "no tracked file exceeds 5 MiB"
  else
    bad "tracked files exceed 5 MiB:"
    for sha in $big; do
      git -C "$DIR" ls-files -s | grep -F "$sha" | sed 's/^/hygiene:      /'
    done
  fi
fi

# --- the tar-extract contract -----------------------------------------------
# This gate runs FIRST in ci/pipeline.sh, so if it cannot survive a tree with
# no git metadata it takes the whole production deploy down with it. Prove that
# here rather than discover it on the droplet: re-run self against an extract
# of HEAD, exactly as infra/post-receive builds its work tree.
# HYGIENE_NESTED stops the recursion at one level.
if [ "${HYGIENE_NESTED:-0}" = 1 ]; then
  :
elif [ "$HAVE_INDEX" = 0 ]; then
  skip "tar-extract self-check (already running without an index)"
else
  # Copied from the WORKING TREE, not `git archive HEAD`: the contract must be
  # provable for the code in hand, or a fix for this very bug could never go
  # green before it was committed. -p keeps the hook's exec bit, which the
  # nested run checks. What the droplet gets differs only by uncommitted work.
  X="$(mktemp -d)"
  mkdir -p "$X/test" "$X/.githooks"
  cp -p "$DIR/test/repo_hygiene.sh" "$X/test/repo_hygiene.sh"
  cp -p "$DIR/.githooks/pre-commit" "$X/.githooks/pre-commit"
  xgot=0
  HYGIENE_NESTED=1 sh "$X/test/repo_hygiene.sh" >"$X/.out" 2>&1 || xgot=$?
  if [ "$xgot" -eq 0 ]; then
    ok "survives a git-less tar extract (the droplet's work tree)"
  else
    bad "FAILS in a tar extract — this would break the production deploy:"
    sed 's/^/hygiene:      /' "$X/.out"
  fi
  # A vacuous pass is the failure mode that hides here, so demand the skips.
  if grep -q 'SKIP tracked-file size invariant' "$X/.out"; then
    ok "index checks skip loudly there rather than pass on an empty list"
  else
    bad "index checks did not announce themselves as skipped in the extract"
    sed 's/^/hygiene:      /' "$X/.out"
  fi
  rm -rf "$X"
fi

exit "$FAIL"
