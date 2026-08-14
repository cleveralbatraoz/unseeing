#!/bin/sh
# Every tracked shell script must parse.
#
# Written because a real one did not. tools/export_macos.sh spent a commit with
# an orphaned `fi` left behind by an edit, and nothing noticed: its own uname
# guard exits before the shell ever reaches the bad line, so on Linux the file
# is never parsed past line 34 and the whole pipeline stayed green. The only
# machine that would have found it is a Mac, at the moment someone tried to cut
# a macOS release.
#
# A parse check costs milliseconds and needs nothing installed, so there is no
# reason for a syntax error to survive to the one platform that runs the file.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FAIL=0

ok() { echo "shell-syntax: OK   $1"; }
bad() { echo "shell-syntax: FAIL $1"; FAIL=1; }

# `git ls-files` when there is metadata, `find` when there is not: the droplet
# runs this from a tar extract with no git directory at all.
if git -C "$ROOT" rev-parse --git-dir >/dev/null 2>&1; then
  FILES="$(git -C "$ROOT" ls-files '*.sh' '.githooks/*')"
else
  FILES="$(cd "$ROOT" && find . -name '*.sh' -not -path './rust/target/*' \
    -not -path './game/addons/*' -not -path './tools/superpowers/*' | sed 's|^\./||')"
fi

COUNT=0
for rel in $FILES; do
  file="$ROOT/$rel"
  [ -f "$file" ] || continue
  # Only files that are actually shell. .githooks/ has no extension, so the
  # shebang decides rather than the name.
  case "$rel" in
    *.sh) : ;;
    *)
      head -1 "$file" | grep -q '^#!.*\(sh\|bash\)' || continue
      ;;
  esac
  COUNT=$((COUNT + 1))
  if err="$(sh -n "$file" 2>&1)"; then
    :
  else
    bad "$rel does not parse"
    printf 'shell-syntax:      %s\n' "$err"
  fi
done

[ "$COUNT" -gt 0 ] || bad "no shell scripts were found to check — the roster is broken, not empty"
[ "$FAIL" -eq 1 ] || ok "all $COUNT tracked shell scripts parse"

exit "$FAIL"
