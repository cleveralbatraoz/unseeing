#!/bin/sh
# Behavioral contract for tools/lib/engine.sh — the single owner of "which
# Godot is the one this repository is pinned to".
#
# Discovery used to be twelve copy-pasted candidate loops that no test ever
# executed: every suite, and CI itself, handed the scripts an explicit engine.
# So the loop was a surviving mutation on every platform. These cases run
# discovery for real against fixture engines whose --version output is the only
# thing that distinguishes them, which is why the candidate list is injectable.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIB="${ENGINE_LIB:-$ROOT/tools/lib/engine.sh}"
FAIL=0

ok() { echo "engine-select: OK   $1"; }
bad() { echo "engine-select: FAIL $1"; FAIL=1; }
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
contains() { case "$2" in *"$1"*) return 0 ;; *) return 1 ;; esac; }

[ -f "$LIB" ] || { echo "engine-select: FAIL $LIB does not exist"; exit 1; }
# shellcheck source=/dev/null
. "$LIB"

# A checkout path with spaces is the cheapest way to catch an unquoted
# expansion, and this library is nothing but path handling.
T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM HUP
REPO="$T/repo with spaces"
BIN="$T/fake engines"
mkdir -p "$REPO" "$BIN/bin"
printf '%s\n' '4.7.1.stable.official' >"$REPO/.godot-version"

# Each fixture engine answers --version and nothing else. Selection must be
# decided by that answer alone, never by the order it happens to sit in.
fake_engine() {
  path="$1"
  version="$2"
  mkdir -p "$(dirname "$path")"
  cat >"$path" <<EOF
#!/bin/sh
[ "\$1" = "--version" ] || exit 0
printf '%s\n' '$version'
EOF
  chmod +x "$path"
}

# ---------------------------------------------------------------- accepts ---
# Hand-derived from .godot-version = 4.7.1.stable.official and the version
# strings the real editors print (checked against the 4.7.1 official build and
# the 4.7 mono snap on the audit machines).

require "the exact pinned build is accepted" \
  unseeing_engine_accepts '4.7.1.stable.official.a13da4feb' '4.7.1.stable.official'
require "a version string with no build hash is accepted" \
  unseeing_engine_accepts '4.7.1.stable.official' '4.7.1.stable.official'
# The defect this whole normalisation exists for: .mono. is a build-flavour
# field, and the pin does not constrain build flavour.
require "a Mono/.NET build of the pinned version is accepted" \
  unseeing_engine_accepts '4.7.1.stable.mono.official.a13da4feb' '4.7.1.stable.official'
require "a double-precision build of the pinned version is accepted" \
  unseeing_engine_accepts '4.7.1.stable.double.official.a13da4feb' '4.7.1.stable.official'
require "a double-precision Mono build is accepted" \
  unseeing_engine_accepts '4.7.1.stable.double.mono.official.abc' '4.7.1.stable.official'

refute "a nearby patch release is refused" \
  unseeing_engine_accepts '4.7.0.stable.official.a13da4feb' '4.7.1.stable.official'
# The 4.7 mono snap on the audit machine. Normalisation must not rescue it:
# it is genuinely the wrong version, quite apart from its flavour.
refute "a Mono build of the WRONG version is still refused" \
  unseeing_engine_accepts '4.7.stable.mono.official.5b4e0cb0f' '4.7.1.stable.official'
refute "a different release status is refused" \
  unseeing_engine_accepts '4.7.1.beta.official.a13da4feb' '4.7.1.stable.official'
refute "an empty version is refused" \
  unseeing_engine_accepts '' '4.7.1.stable.official'
# Boundary: without it, a bare numeric pin would swallow a ten-fold patch
# release, so 4.7.1 would accept 4.7.10.
refute "a longer numeric field is not a prefix match" \
  unseeing_engine_accepts '4.7.10.stable.official.abc' '4.7.1'
require "a field boundary is a real match" \
  unseeing_engine_accepts '4.7.1.stable.official.abc' '4.7.1'
# "mono" must be dropped as a whole FIELD, never as a substring, or an editor
# from a directory or flavour merely containing the letters would be mangled.
refute "the flavour field is dropped whole, not as a substring" \
  unseeing_engine_accepts '4.7.1.stable.monolithic.official.abc' '4.7.1.stable.official'

# -------------------------------------------------------------------- pin ---
PIN="$(unseeing_engine_pin "$REPO")" || PIN='<failed>'
require "the pin is read and trimmed" test "$PIN" = '4.7.1.stable.official'

printf '  4.7.1.stable.official  \n' >"$REPO/.godot-version"
PIN="$(unseeing_engine_pin "$REPO")" || PIN='<failed>'
require "surrounding whitespace is trimmed" test "$PIN" = '4.7.1.stable.official'
printf '%s\n' '4.7.1.stable.official' >"$REPO/.godot-version"

mkdir -p "$T/no-pin"
status=0
unseeing_engine_pin "$T/no-pin" >/dev/null 2>&1 || status=$?
# Not merely nonzero: a missing pin used to disable the version gate ENTIRELY
# and then kill bootstrap.sh on an unbound $WANT after a successful run.
require "a missing .godot-version is a refusal, not a skipped gate" test "$status" -eq 2

mkdir -p "$T/blank-pin"
printf '   \n' >"$T/blank-pin/.godot-version"
status=0
unseeing_engine_pin "$T/blank-pin" >/dev/null 2>&1 || status=$?
require "a blank .godot-version is a refusal" test "$status" -eq 2

# ----------------------------------------------------------------- select ---
# Three engines, only one of which satisfies the pin, and the two that do not
# sort EARLIER in the candidate list. A first-that-exists implementation picks
# godot-4 here; that is exactly the regression measured on the Debian host,
# where the 4.7 mono snap shadows a correct 4.7.1 in ~/bin.
fake_engine "$BIN/godot-4" '4.7.stable.mono.official.5b4e0cb0f'
fake_engine "$BIN/godot4" '4.3.stable.official.abcdef'
fake_engine "$BIN/bin/godot" '4.7.1.stable.official.a13da4feb'

CANDIDATES="$BIN/godot-4
$BIN/godot4
$BIN/bin/godot"

SEL="$(UNSEEING_ENGINE_CANDIDATES="$CANDIDATES" unseeing_engine_select "$REPO" '' 2>/dev/null)" || SEL='<failed>'
require "selection skips engines that fail the pin and takes the one that passes" \
  test "$SEL" = "$BIN/bin/godot"

# A candidate that cannot answer must not abort the walk: an unrelated binary
# named `godot` on PATH would otherwise make the whole toolchain unusable.
fake_engine "$T/broken/godot" 'ignored'
cat >"$T/broken/godot" <<'EOF'
#!/bin/sh
exit 3
EOF
chmod +x "$T/broken/godot"
printf '#!/bin/sh\nexit 0\n' >"$T/broken/godot-silent"
chmod +x "$T/broken/godot-silent"
# A candidate that drains stdin. The walk feeds itself from a heredoc, and a
# child inherits that stdin — so a binary which reads it swallows the remaining
# candidate lines and the loop sees EOF. The engine sitting further down the
# list is then never tried, and the refusal claims no engine exists at all.
printf '#!/bin/sh\ncat >/dev/null\nexit 1\n' >"$T/broken/godot-stdin"
chmod +x "$T/broken/godot-stdin"
CANDIDATES_BROKEN="$T/broken/godot
$T/broken/godot-silent
$T/broken/godot-stdin
$T/does-not-exist/godot
$BIN/bin/godot"
SEL="$(UNSEEING_ENGINE_CANDIDATES="$CANDIDATES_BROKEN" unseeing_engine_select "$REPO" '' 2>/dev/null)" || SEL='<failed>'
require "a candidate that exits nonzero, prints nothing, drains stdin, or is absent is skipped" \
  test "$SEL" = "$BIN/bin/godot"

status=0
SEL="$(UNSEEING_ENGINE_CANDIDATES="$BIN/godot-4
$BIN/godot4" unseeing_engine_select "$REPO" '' 2>/dev/null)" || status=$?
require "no candidate satisfying the pin is a refusal" test "$status" -eq 2

status=0
MSG="$(UNSEEING_ENGINE_CANDIDATES="$BIN/godot-4" unseeing_engine_select "$REPO" '' 2>&1 >/dev/null)" || status=$?
require "the refusal names the pin it could not satisfy" \
  contains '4.7.1.stable.official' "$MSG"
require "the refusal names the GODOT escape hatch" \
  contains 'GODOT' "$MSG"

# An explicit engine is USED, not merely preferred — ci/pipeline.sh hands
# GODOT down to six child scripts and the docs tell humans to set it.
SEL="$(unseeing_engine_select "$REPO" "$BIN/bin/godot" 2>/dev/null)" || SEL='<failed>'
require "an explicit engine satisfying the pin is used" test "$SEL" = "$BIN/bin/godot"

SEL="$(GODOT="$BIN/bin/godot" unseeing_engine_select "$REPO" '' 2>/dev/null)" || SEL='<failed>'
require "GODOT from the environment is honoured" test "$SEL" = "$BIN/bin/godot"

# The deliberate behaviour change spec'd in 2026-08-14-engine-selection-design:
# an explicit mismatched engine FAILS rather than silently falling through to a
# search, because a caller that named an engine meant that engine.
status=0
UNSEEING_ENGINE_CANDIDATES="$BIN/bin/godot" \
  unseeing_engine_select "$REPO" "$BIN/godot-4" >/dev/null 2>&1 || status=$?
require "an explicit engine failing the pin is refused, never replaced by a search hit" \
  test "$status" -eq 2

status=0
unseeing_engine_select "$REPO" "$T/does-not-exist/godot" >/dev/null 2>&1 || status=$?
require "an explicit engine that does not exist is refused" test "$status" -eq 2

# A GUI-subsystem Godot answers --version with silence rather than a version.
# Diagnosing that as a version MISMATCH sends the reader hunting for a version
# problem that does not exist; the fix is to point at the console binary.
status=0
MSG="$(unseeing_engine_select "$REPO" "$T/broken/godot-silent" 2>&1 >/dev/null)" || status=$?
require "an explicit engine that answers with silence is refused" test "$status" -eq 2
require "silence is diagnosed as a missing version, not a version mismatch" \
  contains 'reported no version' "$MSG"

# A repo with no pin must refuse before running any engine at all.
status=0
UNSEEING_ENGINE_CANDIDATES="$BIN/bin/godot" \
  unseeing_engine_select "$T/no-pin" '' >/dev/null 2>&1 || status=$?
require "selection refuses when the pin cannot be read" test "$status" -eq 2

# The reported defect itself, against the DEFAULT candidate list rather than an
# injected one: nobody renames the official archive, so an editor extracted and
# put on PATH under its shipped name has to be found. Both audited machines
# reported "godot not found" while holding exactly this.
FIX="$T/downloads"
mkdir -p "$FIX" "$T/empty-home"
fake_engine "$FIX/Godot_v4.7.1-stable_linux.$(uname -m)" '4.7.1.stable.official.a13da4feb'
SEL="$(HOME="$T/empty-home" PATH="$FIX:/usr/bin:/bin" \
  unseeing_engine_select "$REPO" '' 2>/dev/null)" || SEL='<failed>'
require "the official archive name on PATH is discovered by the default list" \
  test "$SEL" = "$FIX/Godot_v4.7.1-stable_linux.$(uname -m)"

# ...and the same list must not be fooled by an archive of the wrong version.
rm -f "$FIX/Godot_v4.7.1-stable_linux.$(uname -m)"
fake_engine "$FIX/Godot_v4.6.2-stable_linux.$(uname -m)" '4.6.2.stable.official.deadbee'
status=0
HOME="$T/empty-home" PATH="$FIX:/usr/bin:/bin" \
  unseeing_engine_select "$REPO" '' >/dev/null 2>&1 || status=$?
require "an official archive of the wrong version is refused by the default list" \
  test "$status" -eq 2

# The plainest install of all, and the one nothing covered: an editor called
# `godot`, on PATH, resolved by name rather than by an absolute path. Every
# other case here injects a full path, so the bare-name branch of
# unseeing_engine_resolve was a surviving mutation — replacing it with `return 1`
# left this suite entirely green while the most common install stopped being
# discoverable.
#
# Hermetic: PATH holds the fixture directory and nothing but the few utilities
# the library itself shells out to, so a real Godot on the host cannot decide
# the outcome either way.
HERM="$T/hermetic"
mkdir -p "$HERM" "$T/hermetic-home"
for u in uname awk tr head sed sort find; do
  up="$(command -v "$u" 2>/dev/null)" && ln -sf "$up" "$HERM/$u"
done
fake_engine "$HERM/godot" '4.7.1.stable.official.a13da4feb'
SEL="$(HOME="$T/hermetic-home" PATH="$HERM" \
  unseeing_engine_select "$REPO" '' 2>/dev/null)" || SEL='<failed>'
require "a bare 'godot' on PATH is resolved by name" test "$SEL" = "$HERM/godot"

# ...and a bare name that fails the pin must not shadow a correct engine further
# down the list. `godot` is the very first candidate; ~/bin/godot comes later.
fake_engine "$HERM/godot" '4.7.stable.mono.official.5b4e0cb0f'
mkdir -p "$T/hermetic-home/bin"
fake_engine "$T/hermetic-home/bin/godot" '4.7.1.stable.official.a13da4feb'
SEL="$(HOME="$T/hermetic-home" PATH="$HERM" \
  unseeing_engine_select "$REPO" '' 2>/dev/null)" || SEL='<failed>'
require "a wrong-version bare 'godot' does not shadow a correct one further down" \
  test "$SEL" = "$T/hermetic-home/bin/godot"

# Nothing in the library may exit the calling shell: every caller owns its own
# exit code and message prefix, and the suites grep those prefixes.
( set -eu; . "$LIB"
  UNSEEING_ENGINE_CANDIDATES="$BIN/godot-4" unseeing_engine_select "$REPO" '' >/dev/null 2>&1 || true
  printf 'survived\n' ) >"$T/survive" 2>&1 || true
require "a refusal returns to the caller instead of exiting its shell" \
  grep -q survived "$T/survive"

exit "$FAIL"
