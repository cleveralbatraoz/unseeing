# Which Godot is the one this repository is pinned to — the single owner.
#
# SOURCE this file, never execute it. Every function RETURNS a status and never
# calls exit, because each caller owns its own exit code and its own message
# prefix (`bootstrap:`, `ci:`, `probe:`, `vendor:`, `export-macos:`) and the
# self-test suites grep those prefixes.
#
# The law this replaces was two separable halves — find a binary, then check
# its version — and only the first half was ever copied. It ended up verbatim
# in ten POSIX files, absent from two more, and paired with a version gate in
# just three. Nothing tested it: every suite, and CI itself, handed the scripts
# an explicit engine, so the candidate list was a surviving mutation.
#
# The one decision that matters: DISCOVERY IS VERSION-AWARE. `select` returns
# the first candidate that SATISFIES THE PIN, never merely the first that
# exists. Widening the name list is only safe that way — a machine can hold
# several engines, and on the audited Debian host a 4.7 mono snap named
# `godot-4` shadows a correct 4.7.1 in ~/bin. See
# docs/superpowers/specs/2026-08-14-engine-selection-design.md.
#
# POSIX sh only, and deliberately so: this is what tools/bootstrap.sh uses to
# refuse a wrong engine BEFORE it installs Rust, and the droplet runs
# ci/pipeline.sh from a tar extract that can neither compile the core nor read
# git metadata. Internal variables carry `_ue*_` prefixes because POSIX sh has
# no `local`.

# Print the trimmed .godot-version of a checkout. Status 2 — never a silently
# skipped gate — when it is missing, unreadable, or blank.
unseeing_engine_pin() {
  _uep_root="${1:-}"
  if [ -z "$_uep_root" ]; then
    echo "engine: no repository root given to unseeing_engine_pin" >&2
    return 2
  fi
  _uep_file="$_uep_root/.godot-version"
  if [ ! -f "$_uep_file" ]; then
    echo "engine: no Godot pin at $_uep_file" >&2
    return 2
  fi
  # CR removal first: a Windows checkout with autocrlf true stores the pin with
  # CRLF, and a trailing CR would make every version comparison fail with two
  # strings that print identically.
  _uep_value="$(awk 'NR==1{gsub(/\r/,""); gsub(/^[ \t]+|[ \t]+$/,""); print; exit}' \
    "$_uep_file" 2>/dev/null)" || _uep_value=""
  if [ -z "$_uep_value" ]; then
    echo "engine: $_uep_file is blank; it must carry the pinned Godot version" >&2
    return 2
  fi
  printf '%s\n' "$_uep_value"
}

# The pure predicate: does a reported version satisfy the pin?
#
# Godot prints major.minor[.patch].status[.flavour].build[.hash]. `mono` and
# `double` are BUILD-FLAVOUR fields, and .godot-version does not constrain
# flavour — so a .NET editor of the pinned version is the pinned version. The
# old gate prefix-matched the raw string, which rejected every Mono build.
#
# Fields are dropped whole, never as substrings, so a `monolithic` field
# survives. The match must land on a field boundary, so a pin of `4.7.1` does
# not swallow `4.7.10`.
unseeing_engine_accepts() {
  _uea_have="${1:-}"
  _uea_want="${2:-}"
  [ -n "$_uea_have" ] || return 1
  [ -n "$_uea_want" ] || return 1
  _uea_norm="$(printf '%s' "$_uea_have" | awk -F'[.]' '{
    out = ""
    for (i = 1; i <= NF; i++) {
      if ($i == "mono" || $i == "double") continue
      out = (out == "" ? $i : out "." $i)
    }
    print out
  }')" || return 1
  case "$_uea_norm" in
    "$_uea_want") return 0 ;;
    "$_uea_want".*) return 0 ;;
    *) return 1 ;;
  esac
}

# Resolve one candidate to an executable path, or fail. A bare name goes
# through PATH; anything containing a slash is taken as a path.
unseeing_engine_resolve() {
  _uer_candidate="${1:-}"
  [ -n "$_uer_candidate" ] || return 1
  case "$_uer_candidate" in
    */*)
      if [ -f "$_uer_candidate" ] && [ -x "$_uer_candidate" ]; then
        printf '%s\n' "$_uer_candidate"
        return 0
      fi
      return 1
      ;;
    *)
      _uer_found="$(command -v "$_uer_candidate" 2>/dev/null)" || return 1
      [ -n "$_uer_found" ] || return 1
      printf '%s\n' "$_uer_found"
      return 0
      ;;
  esac
}

# Ask an engine what it is. A candidate that cannot answer is NOT fatal: an
# unrelated binary called `godot` sitting on PATH must not make the whole
# toolchain unusable, it must simply lose the walk.
unseeing_engine_version() {
  _uev_bin="${1:-}"
  [ -n "$_uev_bin" ] || return 1
  # </dev/null matters: the candidate walk feeds itself from a heredoc, and a
  # child inherits that stdin. A binary that reads it — any wrapper, or an
  # unrelated program that happens to be called `godot` — swallows the rest of
  # the candidate list, the loop hits EOF, and the engine further down is never
  # tried. The refusal then says no engine exists while a correct one is sitting
  # right there. A candidate must lose the walk, not end it.
  _uev_out="$("$_uev_bin" --version </dev/null 2>/dev/null | tr -d '\r' | head -1)" || _uev_out=""
  [ -n "$_uev_out" ] || return 1
  printf '%s\n' "$_uev_out"
}

# The official archives are never renamed by the people who download them, so
# the naming convention has to be searched for rather than assumed away. This
# is the gap that made a correctly installed editor on PATH invisible on both
# audited machines.
unseeing_engine_archive_names() {
  _uea_root="${1:-}"
  _uea_arch="$(uname -m 2>/dev/null)" || _uea_arch=""
  _uea_dirs=""
  _uea_oldifs="$IFS"
  IFS=':'
  for _uea_d in ${PATH:-}; do
    [ -n "$_uea_d" ] || continue
    _uea_dirs="$_uea_dirs$_uea_d
"
  done
  IFS="$_uea_oldifs"
  if [ -n "$_uea_root" ]; then
    _uea_dirs="$_uea_dirs$_uea_root
$_uea_root/godot-bin
"
  fi
  printf '%s' "$_uea_dirs" | while IFS= read -r _uea_dir; do
    [ -n "$_uea_dir" ] || continue
    [ -d "$_uea_dir" ] || continue
    for _uea_glob in \
      "$_uea_dir"/Godot_v*-stable_linux."${_uea_arch:-x86_64}" \
      "$_uea_dir"/Godot_v*-stable_linux.x86_64 \
      "$_uea_dir"/Godot_v*_console.exe \
      "$_uea_dir"/Godot_v*.exe \
      "$_uea_dir"/Godot*.app/Contents/MacOS/Godot; do
      [ -f "$_uea_glob" ] || continue
      printf '%s\n' "$_uea_glob"
    done
  done
}

# The candidate list, most-likely-first. UNSEEING_ENGINE_CANDIDATES (newline
# separated) replaces it entirely — that injection is how the tests point
# discovery at fixture engines instead of the host, and it is the only reason
# discovery is testable at all.
unseeing_engine_candidates() {
  _uec_root="${1:-}"
  if [ -n "${UNSEEING_ENGINE_CANDIDATES:-}" ]; then
    printf '%s\n' "$UNSEEING_ENGINE_CANDIDATES"
    return 0
  fi
  printf '%s\n' godot godot4 godot-4 godot-editor Godot
  if [ -n "${HOME:-}" ]; then
    printf '%s\n' \
      "$HOME/bin/godot" \
      "$HOME/Applications/Godot.app/Contents/MacOS/Godot"
  fi
  printf '%s\n' \
    /opt/homebrew/bin/godot \
    /usr/local/bin/godot \
    /usr/bin/godot \
    /Applications/Godot.app/Contents/MacOS/Godot
  [ -z "$_uec_root" ] || printf '%s\n' "$_uec_root/godot-bin/godot"
  unseeing_engine_archive_names "$_uec_root"
}

# The whole law. Prints the selected engine on stdout; explains a refusal on
# stderr so the caller can print its own prefixed line and keep its exit code.
#
# An explicit engine (argument, else $GODOT) is USED AND GATED. It is never
# silently replaced by a search hit: a caller that named an engine meant that
# engine, and telling it "I ran a different one" is worse than refusing.
unseeing_engine_select() {
  _ues_root="${1:-}"
  _ues_explicit="${2:-}"
  [ -n "$_ues_explicit" ] || _ues_explicit="${GODOT:-}"

  _ues_want="$(unseeing_engine_pin "$_ues_root")" || return 2

  if [ -n "$_ues_explicit" ]; then
    if ! _ues_path="$(unseeing_engine_resolve "$_ues_explicit")"; then
      echo "engine: GODOT names '$_ues_explicit', which is not an executable" >&2
      return 2
    fi
    if ! _ues_have="$(unseeing_engine_version "$_ues_path")"; then
      echo "engine: $_ues_path reported no version; the pin is $_ues_want" >&2
      return 2
    fi
    if unseeing_engine_accepts "$_ues_have" "$_ues_want"; then
      printf '%s\n' "$_ues_path"
      return 0
    fi
    echo "engine: $_ues_path is $_ues_have, not the pinned $_ues_want" >&2
    echo "engine: set GODOT= to a matching binary, or install Godot $_ues_want" >&2
    return 2
  fi

  _ues_rejected=""
  # A heredoc, not a pipe: the loop must run in THIS shell so a match can
  # return from the function instead of from a subshell that no one reads.
  while IFS= read -r _ues_candidate; do
    [ -n "$_ues_candidate" ] || continue
    _ues_path="$(unseeing_engine_resolve "$_ues_candidate")" || continue
    _ues_have="$(unseeing_engine_version "$_ues_path")" || continue
    if unseeing_engine_accepts "$_ues_have" "$_ues_want"; then
      printf '%s\n' "$_ues_path"
      return 0
    fi
    _ues_rejected="$_ues_rejected  $_ues_path is $_ues_have
"
  done <<UNSEEING_ENGINE_CANDIDATE_LIST
$(unseeing_engine_candidates "$_ues_root")
UNSEEING_ENGINE_CANDIDATE_LIST

  echo "engine: no Godot $_ues_want found; set GODOT=/path/to/godot" >&2
  if [ -n "$_ues_rejected" ]; then
    echo "engine: engines found, but none match the pin:" >&2
    printf '%s' "$_ues_rejected" >&2
  fi
  return 2
}
