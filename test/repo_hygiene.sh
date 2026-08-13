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

# Git exports GIT_DIR into every hook it runs, and it is RELATIVE ("."), so it
# outranks both `-C` and the cwd: inherited, every git call below addresses
# whatever repo the caller was in rather than the one it names, and the scratch
# `git init` and its `git add` land on a bare repo with no work tree. That is
# how this gate broke the production deploy.
#
# infra/post-receive now scrubs the environment before running the pipeline,
# which fixes the deploy path at its source. This stays anyway: a gate that
# spawns its own repos must be correct wherever it is invoked from, not only
# where someone remembered to clean up first. The self-checks below pin the
# property here, so it cannot regress silently if that hook is ever rewritten.
# (POSIX `unset` is silent for names that were never set.)
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_OBJECT_DIRECTORY \
  GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_QUARANTINE_PATH GIT_PREFIX \
  GIT_COMMON_DIR GIT_NAMESPACE

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
#
# game/override.cfg is the per-run project-setting override the rendered
# probe writes to boot windowed (the project defaults to full screen and the
# engine's --windowed flag cannot beat a project setting). It is deleted on
# the way out, but a killed run leaves it — and committed, it would silently
# un-fullscreen the shipped game.
#
# .superpowers/ is the scratch workspace the subagent-driven-development skill
# writes one directory per plan into: task briefs, implementer reports, review
# packages, and the progress ledger that survives compaction. The skill drops a
# self-ignoring `.gitignore` into `.superpowers/sdd/` — one level down, and only
# once its first script has run. So between entering a plan and that first call
# the ledger is untracked and unignored, exactly the window in which
# `git add -A` swallows it, and anything written outside `sdd/` is never
# covered at all. The standing rule belongs here, not in a directory the tool
# creates for itself.
#
# game/addons/godot_mcp/ is the godot-mcp editor addon: a developer tool, and
# the one tenant of game/addons/ that must never be committed. deploy.sh ships
# the tree by `git archive` into a bare repo, so a committed addon reaches the
# droplet checkout even though every game export preset excludes addons/*. Its
# project.godot enablement is deliberately local too: a tracked reference to an
# ignored per-machine addon makes fresh editors rewrite the project differently.
if [ "$HAVE_INDEX" = 0 ]; then
  skip "ignore rules (no git metadata — deploy work tree is a tar extract)"
else
  for p in .claude/settings.json .claude/worktrees/some-task/README.md \
           .worktrees/some-task/README.md \
           game/override.cfg .superpowers/sdd/a-plan/progress.md \
           game/addons/godot_mcp/plugin.cfg; do
    if git -C "$DIR" check-ignore -q "$p" 2>/dev/null; then
      ok "$p is ignored"
    else
      bad "$p is NOT ignored (no .gitignore rule covers it)"
    fi
  done
fi

# --- shared agent contract and the sole sanctioned gitlink -----------------
if [ "${HYGIENE_NESTED:-0}" = 1 ]; then
  skip "agent contract (nested self-check carries only test infrastructure)"
elif [ "$HAVE_INDEX" = 0 ]; then
  [ -s "$DIR/AGENTS.md" ] && ok "AGENTS.md is present in exported tree" || bad "AGENTS.md is missing or empty"
  [ ! -e "$DIR/.gitmodules" ] && [ ! -e "$DIR/tools/superpowers" ] \
    && ok "developer agent tooling is absent from exported tree" \
    || bad "developer agent tooling leaked into exported tree"
else
  if git -C "$DIR" ls-files --error-unmatch AGENTS.md >/dev/null 2>&1 \
     && [ -s "$DIR/AGENTS.md" ] && [ "$(wc -c < "$DIR/AGENTS.md" | tr -d ' ')" -le 24576 ]; then
    ok "AGENTS.md is tracked, nonempty, and at most 24 KiB"
  else
    bad "AGENTS.md must be tracked, nonempty, and at most 24 KiB"
  fi
  expected_adapter='# Claude Code instructions

@AGENTS.md'
  [ "$(cat "$DIR/CLAUDE.md" 2>/dev/null || true)" = "$expected_adapter" ] \
    && ok "CLAUDE.md is the approved import adapter" \
    || bad "CLAUDE.md contains policy instead of the approved @AGENTS.md adapter"
  "$DIR/ci/verify-superpowers.sh" metadata >/dev/null 2>&1 \
    && ok "Superpowers metadata and gitlink are pinned" \
    || bad "Superpowers metadata or gitlink violates the pin"
fi

# --- the one addon that must stay out --------------------------------------
# The ignore rule above is half the guard: it stops `git add -A` from sweeping
# the addon in. This is the other half, and it is the one that matters after
# the fact — `git add -f` and a rule deleted in a later edit both defeat the
# first check while leaving this one to catch the result.
#
# Checked as a TRACKED-PATH question rather than a working-tree one on purpose:
# the addon is expected to be present on a developer's disk and absent from
# every commit, so its existence proves nothing either way. Only the index does.
if [ "$HAVE_INDEX" = 0 ]; then
  skip "godot-mcp addon stays untracked (no git metadata)"
else
  mcp="$(git -C "$DIR" ls-files -- 'game/addons/godot_mcp' | head -20)"
  if [ -z "$mcp" ]; then
    ok "game/addons/godot_mcp/ is not tracked"
  else
    bad "godot-mcp addon is TRACKED — it would pollute the repository and droplet checkout:"
    echo "$mcp" | sed 's/^/hygiene:      /'
  fi
fi

# --- canonicalize mirror -----------------------------------------------------
# game/tests/probe/determinism_probe.gd and restore_probe.gd each carry their
# own copy of `canonicalize`/`_canonicalize_sequence` — the JSON-safe walk
# that decomposes every Vector2/3/4 and Basis lane into bare floats before
# JSON.stringify, which is what makes the state hash both probes print
# comparable at all. A gdUnit suite proving them byte-identical would need a
# live Godot runtime; this is the cheap POSIX half of that guarantee, run on
# every pipeline invocation. Marked with `# canonicalize-mirror: BEGIN/END`
# comments in both files (deliberately around the CODE only, not the doc
# comments above it, which are allowed to say different things).
DET_PROBE="$DIR/game/tests/probe/determinism_probe.gd"
RES_PROBE="$DIR/game/tests/probe/restore_probe.gd"
canon_block() { # canon_block <file>
  sed -n '/# canonicalize-mirror: BEGIN/,/# canonicalize-mirror: END/p' "$1" | sed '1d;$d'
}
# The tar-extract self-check further below copies only test/ and .githooks/
# into its scratch tree, never game/ — a real deploy work tree (and every
# other invocation of this script) has the full tree, so a missing probe
# file means something different in each case. HYGIENE_NESTED is the same
# signal the self-check section already uses to stop its own recursion.
if [ "${HYGIENE_NESTED:-0}" = 1 ]; then
  skip "canonicalize mirror (nested self-check scratch tree carries no game/)"
elif [ ! -f "$DET_PROBE" ] || [ ! -f "$RES_PROBE" ]; then
  bad "canonicalize mirror: probe file(s) missing ($DET_PROBE, $RES_PROBE)"
else
  CANON_TMP="$(mktemp -d)"
  canon_block "$DET_PROBE" >"$CANON_TMP/det.txt"
  canon_block "$RES_PROBE" >"$CANON_TMP/res.txt"
  if [ ! -s "$CANON_TMP/det.txt" ] || [ ! -s "$CANON_TMP/res.txt" ]; then
    bad "canonicalize mirror: BEGIN/END markers not found in $DET_PROBE and/or $RES_PROBE"
  elif diff -q "$CANON_TMP/det.txt" "$CANON_TMP/res.txt" >/dev/null; then
    ok "canonicalize mirror: $DET_PROBE and $RES_PROBE agree"
  else
    bad "canonicalize mirror DIVERGED between $DET_PROBE and $RES_PROBE:"
    diff "$CANON_TMP/det.txt" "$CANON_TMP/res.txt" | sed 's/^/hygiene:      /'
  fi
  rm -rf "$CANON_TMP"
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
  mkdir -p "$TMP/repo/game/tests" "$TMP/repo/game/scripts"
  printf 'extends Node\n' > "$TMP/repo/game/tests/legal test.gd"
  printf 'extends Node\n' > "$TMP/repo/game/scripts/illegal.gd"

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

  git -C "$TMP/repo" add "game/tests/legal test.gd"
  probe 0 "pre-commit accepts legal test GDScript whose path contains spaces"

  git -C "$TMP/repo" add game/scripts/illegal.gd
  probe 1 "pre-commit rejects staged first-party GDScript outside game/tests"
  if grep -q 'game/scripts/illegal.gd' "$TMP/out"; then
    ok "pre-commit names the illegal GDScript path"
  else
    bad "pre-commit does not name the illegal GDScript path"
  fi
  git -C "$TMP/repo" reset -q HEAD -- game/scripts/illegal.gd 2>/dev/null \
    || git -C "$TMP/repo" rm --cached -q game/scripts/illegal.gd

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

  # Two environments, because the first fix only covered the first one. A bare
  # extract is what the work tree LOOKS like; GIT_DIR=. is what the hook that
  # builds it actually exports. Only the second reproduces the deploy failure.
  extract_probe() { # extract_probe <label> [GIT_DIR value]
    lbl="$1"
    egot=0
    # GIT_DIR must be genuinely ABSENT for the clean case, not set to "" —
    # an empty value is still an override, and git resolves it to the cwd,
    # so passing "" would quietly make both probes test the same poisoned
    # environment and the clean one would never be exercised at all.
    if [ "$#" -ge 2 ]; then
      (
        cd "$X" || exit 99
        GIT_DIR="$2" HYGIENE_NESTED=1 sh "$X/test/repo_hygiene.sh"
      ) >"$X/.out" 2>&1 || egot=$?
    else
      (
        cd "$X" || exit 99
        HYGIENE_NESTED=1 sh "$X/test/repo_hygiene.sh"
      ) >"$X/.out" 2>&1 || egot=$?
    fi
    if [ "$egot" -eq 0 ]; then
      ok "$lbl"
    else
      bad "$lbl — this breaks the production deploy:"
      sed 's/^/hygiene:      /' "$X/.out"
    fi
    # A vacuous pass is the failure mode that hides here, so demand the skips.
    if grep -q 'SKIP tracked-file size invariant' "$X/.out"; then
      ok "$lbl: index checks skip loudly rather than pass on an empty list"
    else
      bad "$lbl: index checks did not announce themselves as skipped"
      sed 's/^/hygiene:      /' "$X/.out"
    fi
  }

  extract_probe "survives a git-less tar extract (the droplet's work tree)"
  extract_probe "survives GIT_DIR inherited from the post-receive hook" .
  rm -rf "$X"
fi

exit "$FAIL"
