#!/bin/sh
# Provenance and update path for the vendored gdUnit4 test framework.
#
# Godot resolves addons as project resources, so a framework has to live inside
# the tree at res://addons/. A submodule cannot: upstream's git tree carries no
# .uid/.import sidecars, and Godot writes 244 of them on import — inside a
# submodule they would be permanently dirty and uncommittable, and upstream's
# repo root is a whole test project, not the addon. So gdUnit4 is a copy — but
# a reproducible one. This script is the only sanctioned way to change it, and
# ci/gdunit4.lock fingerprints both upstream's source and our resulting tree.
#
# Modes:
#   ci/vendor-gdunit4.sh                 verify our tree matches the lock (offline; the CI gate)
#   ci/vendor-gdunit4.sh check-upstream  verify the lock still matches upstream (network)
#   ci/vendor-gdunit4.sh update <tag>    re-vendor at <tag> and rewrite the lock
#
# Env knobs: GODOT (binary, needed only by `update`).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
ADDON="$DIR/game/addons/gdUnit4"
LOCK="$DIR/ci/gdunit4.lock"
REPO="godot-gdunit-labs/gdUnit4"

if command -v sha256sum >/dev/null 2>&1; then
  SHACMD="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
  SHACMD="shasum -a 256"
else
  echo "vendor: FAILED neither sha256sum nor shasum found"
  exit 2
fi

# One value fingerprinting a whole directory: every file's sha256 paired with
# its relative path, then the roster of executable files, all ordered by path
# and hashed again. Independent of archive format, timestamps, and checkout
# order — and it works on the droplet, whose deploy work tree is a tar extract
# with no git metadata. The exec bits are in there because losing one is real
# drift: the hand-vendored copy shipped runtest.sh non-executable.
# $SHACMD is deliberately unquoted: "shasum -a 256" must word-split into argv.
fingerprint() {
  # shellcheck disable=SC2086
  (
    cd "$1" || exit 1
    find . -type f -print | LC_ALL=C sort | tr '\n' '\0' | xargs -0 $SHACMD
    find . -type f -perm -u+x -print | LC_ALL=C sort | sed 's/^/x /'
  ) | $SHACMD | cut -d' ' -f1
}

# lock_get runs inside $(...), so it cannot report its own errors — a missing
# lock would be captured as the value instead of printed. Callers gate on this.
require_lock() {
  [ -f "$LOCK" ] || { echo "vendor: FAILED no lock at $LOCK"; exit 2; }
}

lock_get() {
  sed -n "s/^$1=//p" "$LOCK"
}

# Upstream's source tarball, unpacked to a temp dir; echoes the addon path.
# GitHub applies the repo's .gitattributes export-ignore, which is why the
# tarball already omits upstream's own 412-file self-test suite.
fetch_upstream() {
  tag="$1"
  tmp="$2"
  command -v curl >/dev/null 2>&1 || { echo "vendor: FAILED curl not found"; exit 2; }
  curl -sfL -o "$tmp/src.tar.gz" \
    "https://codeload.github.com/$REPO/tar.gz/refs/tags/$tag" \
    || { echo "vendor: FAILED cannot fetch $REPO at $tag"; exit 1; }
  tar xzf "$tmp/src.tar.gz" -C "$tmp"
  found="$(find "$tmp" -type d -path '*/addons/gdUnit4' | head -1)"
  [ -n "$found" ] || { echo "vendor: FAILED no addons/gdUnit4 in the $tag tarball"; exit 1; }
  echo "$found"
}

upstream_commit() {
  curl -sfL "https://api.github.com/repos/$REPO/git/ref/tags/$1" 2>/dev/null \
    | tr ',{}' '\n' | sed -n 's/.*"sha" *: *"\([0-9a-f]\{40\}\)".*/\1/p' | head -1
}

case "${1:-verify}" in
  verify)
    require_lock
    [ -d "$ADDON" ] || { echo "vendor: FAILED no vendored addon at $ADDON"; exit 1; }
    have="$(fingerprint "$ADDON")"
    want="$(lock_get tree_sha256)"
    if [ "$have" != "$want" ]; then
      echo "vendor: FAILED game/addons/gdUnit4 does not match ci/gdunit4.lock"
      echo "vendor:   locked $want"
      echo "vendor:   actual $have"
      echo "vendor: vendored code is never hand-edited — re-run ci/vendor-gdunit4.sh update $(lock_get tag)"
      exit 1
    fi
    echo "vendor: gdUnit4 $(lock_get tag) matches the lock ($(find "$ADDON" -type f | wc -l | tr -d ' ') files)"
    ;;

  check-upstream)
    require_lock
    tag="$(lock_get tag)"
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    src="$(fetch_upstream "$tag" "$tmp")"
    have="$(fingerprint "$src")"
    want="$(lock_get source_sha256)"
    if [ "$have" != "$want" ]; then
      echo "vendor: FAILED upstream $tag no longer matches the lock (retagged?)"
      echo "vendor:   locked $want"
      echo "vendor:   actual $have"
      exit 1
    fi
    echo "vendor: upstream $REPO $tag still matches the lock"
    ;;

  update)
    tag="${2:-}"
    [ -n "$tag" ] || { echo "vendor: usage: ci/vendor-gdunit4.sh update <tag>   (e.g. v6.2.0)"; exit 2; }

    GODOT="${GODOT:-}"
    if [ -z "$GODOT" ]; then
      for g in godot "$HOME/bin/godot" /opt/homebrew/bin/godot; do
        if command -v "$g" >/dev/null 2>&1 || [ -x "$g" ]; then GODOT="$g"; break; fi
      done
    fi
    [ -n "$GODOT" ] || { echo "vendor: FAILED godot not found; set GODOT=/path/to/godot"; exit 2; }

    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    src="$(fetch_upstream "$tag" "$tmp")"
    src_sha="$(fingerprint "$src")"
    echo "vendor: fetched $REPO $tag ($(find "$src" -type f | wc -l | tr -d ' ') source files)"

    # Keep the sidecars Godot already minted. Their uids are random, so
    # regenerating them wholesale would churn every .tscn ext_resource and bury
    # the real upstream diff. Surviving files keep their identity; only files
    # new in this release get fresh ones, below.
    if [ -d "$ADDON" ]; then
      (cd "$ADDON" && find . \( -name '*.uid' -o -name '*.import' \) -print) > "$tmp/sidecars.txt"
      while IFS= read -r s; do
        [ -n "$s" ] || continue
        mkdir -p "$tmp/keep/$(dirname "$s")"
        cp "$ADDON/$s" "$tmp/keep/$s"
      done < "$tmp/sidecars.txt"
      rm -rf "$ADDON"
    else
      : > "$tmp/sidecars.txt"
    fi

    mkdir -p "$ADDON"
    (cd "$src" && tar cf - .) | (cd "$ADDON" && tar xf -)

    kept=0
    while IFS= read -r s; do
      [ -n "$s" ] || continue
      base="${s%.uid}"
      base="${base%.import}"
      # a sidecar outlives the release only if its source file did
      if [ -f "$ADDON/$base" ]; then
        cp "$tmp/keep/$s" "$ADDON/$s"
        kept=$((kept + 1))
      fi
    done < "$tmp/sidecars.txt"
    echo "vendor: carried over $kept sidecars"

    "$GODOT" --headless --path "$DIR/game" --import >/dev/null 2>&1 || true

    commit="$(upstream_commit "$tag")"
    [ -n "$commit" ] || commit="unknown"
    cat > "$LOCK" <<EOF
# Provenance for the vendored copy at game/addons/gdUnit4.
# Never hand-edit this file or the addon — run: ci/vendor-gdunit4.sh update <tag>
repo=https://github.com/$REPO
tag=$tag
commit=$commit
# fingerprint of upstream's addons/gdUnit4 as shipped in the source tarball
source_sha256=$src_sha
source_files=$(find "$src" -type f | wc -l | tr -d ' ')
# fingerprint of our tree: the above plus the .uid/.import sidecars Godot mints
tree_sha256=$(fingerprint "$ADDON")
tree_files=$(find "$ADDON" -type f | wc -l | tr -d ' ')
EOF
    echo "vendor: wrote $LOCK"
    echo "vendor: review the diff, then commit both the addon and the lock"
    ;;

  *)
    echo "vendor: usage: ci/vendor-gdunit4.sh [verify | check-upstream | update <tag>]"
    exit 2
    ;;
esac
