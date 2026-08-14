# Content digests for whole directory trees — the shared half of the check that
# a vendored or cached copy still matches its pin.
#
# SOURCE this file, never execute it. Functions RETURN status and never exit.
#
# The defect that made this a library: tools/setup-agents.sh compared the pinned
# skill tree against the installed one using bare `shasum` — a Perl script, and
# absent from minimal and container images. Missing, it produced an empty digest
# for BOTH sides, and empty equals empty, so a tampered cache passed the
# integrity gate silently. ci/vendor-gdunit4.sh already had this right: prefer
# sha256sum, fall back to shasum, and refuse outright when neither exists. That
# rule now lives in one place.
#
# Internal variables carry `_ud*_` prefixes because POSIX sh has no `local`.

# Echo the digest command (which may carry arguments, so callers must leave it
# unquoted to word-split into argv). Status 2 when the host has neither tool —
# never an empty string that a comparison would read as agreement.
unseeing_digest_cmd() {
  if command -v sha256sum >/dev/null 2>&1; then
    printf '%s\n' 'sha256sum'
    return 0
  fi
  if command -v shasum >/dev/null 2>&1; then
    printf '%s\n' 'shasum -a 256'
    return 0
  fi
  echo "digest: neither sha256sum nor shasum is available" >&2
  return 2
}

# One digest over every file in a tree: content, plus the relative path each
# byte was found at, so a rename with identical content is drift and is caught.
#
# Paths come from `cd`-ing into the tree and letting find print them relative,
# rather than stripping a prefix with an unquoted parameter expansion — that
# expansion treated the root path as a GLOB, so a checkout living under a
# directory containing [ or * hashed differently from an identical tree beside
# it.
unseeing_digest_tree() {
  _udt_root="${1:-}"
  if [ -z "$_udt_root" ]; then
    echo "digest: no tree given to unseeing_digest_tree" >&2
    return 2
  fi
  if [ ! -d "$_udt_root" ]; then
    echo "digest: $_udt_root is not a directory" >&2
    return 2
  fi
  _udt_cmd="$(unseeing_digest_cmd)" || return 2
  # shellcheck disable=SC2086
  _udt_out="$(
    cd "$_udt_root" || exit 2
    find . -type f -print | LC_ALL=C sort | tr '\n' '\0' | xargs -0 $_udt_cmd
  )" || return 2
  # xargs exits 0 with empty output when the tree is empty, which is a legitimate
  # digest of nothing — but it also does so when the digest command was never
  # found, and that is the case this whole file exists to refuse. The command
  # was resolved above, so reaching here with no digest at all means the tool
  # failed while running; treat that as a refusal rather than a value.
  if [ -z "$_udt_out" ] && [ -n "$(find "$_udt_root" -type f -print 2>/dev/null | head -1)" ]; then
    echo "digest: $_udt_cmd produced no output for $_udt_root" >&2
    return 2
  fi
  printf '%s\n' "$_udt_out" | $_udt_cmd | cut -d' ' -f1
}

# Do two trees hold the same content? 0 identical, 1 they differ, 2 CANNOT TELL.
#
# The third answer is why this function exists rather than being left to the
# caller. The obvious thing to write is
#
#     [ "$(unseeing_digest_tree "$a")" = "$(unseeing_digest_tree "$b")" ]
#
# and command substitution DISCARDS exit status — so on a host with no hasher
# both sides are the empty string, the test finds them equal, and a tampered
# tree is certified identical to its pin. That is the exact silent agreement
# this file was written to refuse, reintroduced one layer up. Keeping the
# comparison here is what makes the refusal survive it.
unseeing_digest_trees_match() {
  _udm_a="$(unseeing_digest_tree "${1:-}")" || return 2
  _udm_b="$(unseeing_digest_tree "${2:-}")" || return 2
  [ "$_udm_a" = "$_udm_b" ]
}
