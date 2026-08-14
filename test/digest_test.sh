#!/bin/sh
# Behavioral contract for tools/lib/digest.sh.
#
# The defect this exists for: tools/setup-agents.sh hashed the pinned skill tree
# and the installed copy with bare `shasum`, a Perl script absent from minimal
# and container images. With it missing, BOTH digests came back empty — and
# empty equals empty, so the tamper check reported the trees identical. A
# missing tool must fail loudly, never agree silently.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIB="${DIGEST_LIB:-$ROOT/tools/lib/digest.sh}"
FAIL=0

ok() { echo "digest: OK   $1"; }
bad() { echo "digest: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}
refute() {
  label="$1"
  shift
  if "$@"; then bad "$label"; else ok "$label"; fi
}

[ -f "$LIB" ] || { echo "digest: FAIL $LIB does not exist"; exit 1; }
# shellcheck source=/dev/null
. "$LIB"

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP

# Names with a space and a glob metacharacter: the old implementation stripped
# the root prefix with an unquoted "${f#$1/}", so a checkout under a path
# containing [ or * had its relative names mangled and hashed differently on
# two machines holding identical trees.
A="$T/tree [one]"
B="$T/tree [two]"
mkdir -p "$A/skills/deep" "$B/skills/deep"
printf 'alpha\n' >"$A/skills/a file.md"
printf 'beta\n' >"$A/skills/deep/b.md"
printf 'alpha\n' >"$B/skills/a file.md"
printf 'beta\n' >"$B/skills/deep/b.md"

HA="$(unseeing_digest_tree "$A")" || HA=''
HB="$(unseeing_digest_tree "$B")" || HB=''
require "a digest is actually produced" test -n "$HA"
require "identical trees under differently-named roots agree" test "$HA" = "$HB"

printf 'alpha tampered\n' >"$B/skills/a file.md"
HB2="$(unseeing_digest_tree "$B")" || HB2=''
refute "a one-line content change is caught" test "$HA" = "$HB2"

printf 'alpha\n' >"$B/skills/a file.md"
mv "$B/skills/deep/b.md" "$B/skills/deep/renamed.md"
HB3="$(unseeing_digest_tree "$B")" || HB3=''
refute "a rename with identical content is caught" test "$HA" = "$HB3"
mv "$B/skills/deep/renamed.md" "$B/skills/deep/b.md"

mkdir -p "$T/extra"
HA2="$(unseeing_digest_tree "$A")" || HA2=''
require "the digest is stable across runs" test "$HA" = "$HA2"

status=0
unseeing_digest_tree "$T/does-not-exist" >/dev/null 2>&1 || status=$?
refute "a missing tree is refused rather than digested as nothing" test "$status" -eq 0

# The whole point. A stripped PATH stands in for the minimal image: with no
# digest tool at all, the call must FAIL. If it returns empty successfully,
# every caller comparing two digests concludes the trees match.
STRIP="$T/nodigest"
mkdir -p "$STRIP"
for b in sh dash find sort awk sed printf cat ls id; do
  p="$(command -v "$b" 2>/dev/null)" && ln -sf "$p" "$STRIP/$b"
done
status=0
out="$(PATH="$STRIP" unseeing_digest_tree "$A" 2>/dev/null)" || status=$?
refute "no digest tool on PATH is a failure, not a silent empty hash" \
  test "$status" -eq 0
require "no digest tool on PATH produces no digest to compare" test -z "$out"

# --- the comparison, because that is where the refusal was being thrown away --
# A caller reaching for `[ "$(digest a)" = "$(digest b)" ]` loses the status:
# command substitution discards it, so two failed digests compare as two equal
# empty strings and a tampered tree is reported identical. The comparison has to
# live where the status still exists, and it has to have a THIRD answer.
status=0
unseeing_digest_trees_match "$A" "$A" >/dev/null 2>&1 || status=$?
require "a tree matches itself" test "$status" -eq 0

printf 'alpha tampered\n' >"$B/skills/a file.md"
status=0
unseeing_digest_trees_match "$A" "$B" >/dev/null 2>&1 || status=$?
require "two different trees are reported as differing, not as unknowable" \
  test "$status" -eq 1

# The case the whole file exists for, asked of the comparison rather than the
# digest: with no hasher, "I cannot tell" must not be spelled "they match".
status=0
PATH="$STRIP" unseeing_digest_trees_match "$A" "$B" >/dev/null 2>&1 || status=$?
require "with no digest tool, the answer is 'cannot tell' (2)" test "$status" -eq 2
refute "with no digest tool, the answer is never 'identical'" test "$status" -eq 0

status=0
unseeing_digest_trees_match "$A" "$T/does-not-exist" >/dev/null 2>&1 || status=$?
require "an unreadable side is 'cannot tell', not a difference" test "$status" -eq 2

exit "$FAIL"
