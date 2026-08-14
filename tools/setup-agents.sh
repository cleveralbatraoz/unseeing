#!/bin/sh
set -eu

# Nothing above main() may have an effect or read a file. Sourcing this script
# defines its functions and stops, which is how test/setup_agents_test.sh drives
# the classifier — and $0 belongs to the SOURCING script then, so resolving the
# checkout root here would resolve it somewhere else entirely.
# One home for the pin, the tag and the version: ci/superpowers.lock, which
# tools/update-superpowers.sh rewrites. They used to be spelled out in four
# files the update path never touched.
sp_lock_get() { sed -n "s/^$1=//p" "$2" | head -1; }

# Which competing Superpowers installations actually compete HERE.
#
# Scope decides. A `user` plugin is global and loads in every project, so it
# competes. A `local` or `project` plugin belongs to ONE checkout: if that is
# this repository it competes, and if it is somebody else's it cannot load here
# at all — blocking setup over it would be wrong, and removing it with
# --migrate would delete a plugin out of an unrelated project.
#
# An unreported scope is treated as `user`, because a plugin this script cannot
# reason about is exactly the one it must not wave through.
#
# Prints one tab-separated line per competitor: verdict, id, scope, project.
plugin_conflicts() { # plugin_conflicts <this-repo-path>   (JSON on stdin)
  python3 -c 'import json,sys
here = sys.argv[1].rstrip("/")
raw = sys.stdin.read()
try:
    d = json.loads(raw)
except ValueError:
    sys.stderr.write("setup-agents: could not read the plugin list as JSON\n")
    sys.exit(2)
xs = d if isinstance(d, list) else d.get("installed", [])
for x in xs:
    pid = x.get("id", x.get("pluginId", ""))
    if not pid.startswith("superpowers@") or pid == "superpowers@superpowers-dev":
        continue
    scope = x.get("scope") or "user"
    project = (x.get("projectPath") or "").rstrip("/")
    if scope in ("local", "project") and project and project != here:
        verdict = "elsewhere"
    else:
        verdict = "block"
    print("%s\t%s\t%s\t%s" % (verdict, pid, scope, project))' "$1"
}

market_root() {
  python3 -c 'import json,sys
d=json.load(sys.stdin); xs=d if isinstance(d,list) else d.get("marketplaces",[])
for x in xs:
 if x.get("name")=="superpowers-dev": print(x.get("path",x.get("root","")))'
}
installed_record() {
  python3 -c 'import json,sys
d=json.load(sys.stdin); xs=d if isinstance(d,list) else d.get("installed",[])
for x in xs:
 i=x.get("id",x.get("pluginId",""))
 if i=="superpowers@superpowers-dev": print("%s|%s|%s"%(x.get("version",""),str(x.get("enabled",False)).lower(),x.get("installPath",x.get("installedPath",""))))'
}
# Was bare `shasum` over a prefix-stripped path list, compared inside a `[ ... ]`
# as two command substitutions. shasum is a Perl script and simply absent from
# minimal and container images — and command substitution discards exit status,
# so with it missing BOTH sides came back empty, compared equal, and a tampered
# skill cache passed this gate silently. Moving the hashing into the library was
# only half the repair: the COMPARISON has to be where the status still exists,
# which is why it is a three-answer call and not a string test.
check_skills() { # check_skills <host label> <pinned tree> <installed tree>
  unseeing_digest_trees_match "$2" "$3"
  case "$?" in
    0) return 0 ;;
    1)
      echo "setup-agents: $1 skill cache differs from repository pin" >&2
      return 1
      ;;
    *)
      echo "setup-agents: $1 skill cache cannot be verified — no working content hasher" >&2
      echo "setup-agents: fix: install coreutils (sha256sum) or perl (shasum), then re-run" >&2
      return 1
      ;;
  esac
}

setup_claude() {
  command -v claude >/dev/null 2>&1 || { echo "setup-agents: Claude Code is not installed; skipping"; return; }
  plugins="$(claude plugin list --json)"
  all_conflicts="$(printf '%s' "$plugins" | plugin_conflicts "$ROOT")" || {
    echo "setup-agents: cannot read Claude's plugin list" >&2
    exit 1
  }
  blocking="$(printf '%s\n' "$all_conflicts" | awk -F'\t' '$1 == "block"')"
  elsewhere="$(printf '%s\n' "$all_conflicts" | awk -F'\t' '$1 == "elsewhere"')"
  old_root="$(claude plugin marketplace list --json | market_root)"
  stale=0; [ -z "$old_root" ] || [ "$old_root" = "$SUB" ] || stale=1
  # Reported, never acted on: these belong to other checkouts and cannot load
  # here, so they are neither a reason to refuse nor ours to remove.
  if [ -n "$elsewhere" ]; then
    printf '%s\n' "$elsewhere" | while IFS="$(printf '\t')" read -r _ id scope project; do
      [ -n "$id" ] || continue
      echo "setup-agents: note: $id is $scope-scoped to $project — left alone"
    done
  fi
  if { [ -n "$blocking" ] || [ "$stale" -eq 1 ]; } && [ "$MIGRATE" -ne 1 ]; then
    echo "setup-agents: competing Claude Superpowers installation(s):" >&2
    printf '%s\n' "$blocking" | while IFS="$(printf '\t')" read -r _ id scope _; do
      [ -n "$id" ] || continue
      echo "  $id ($scope scope)" >&2
      echo "    remove with: claude plugin uninstall $id --scope $scope" >&2
    done
    [ "$stale" -eq 0 ] || {
      echo "  stale marketplace superpowers-dev -> $old_root" >&2
      echo "    remove with: claude plugin marketplace remove superpowers-dev --scope user" >&2
    }
    echo "or rerun: tools/setup-agents.sh --migrate claude" >&2
    exit 1
  fi
  if [ "$MIGRATE" -eq 1 ] && [ -n "$blocking" ]; then
    # Each plugin is removed at the scope it reports. A hardcoded --scope user
    # simply fails against a local install, and the CLI says so: "installed in
    # local scope, not user".
    printf '%s\n' "$blocking" | while IFS="$(printf '\t')" read -r _ id scope _; do
      [ -n "$id" ] || continue
      claude plugin uninstall "$id" --scope "$scope" >/dev/null || {
        echo "setup-agents: could not uninstall $id from $scope scope" >&2
        exit 1
      }
    done || exit 1
  fi
  if [ "$MIGRATE" -eq 1 ]; then
    [ -z "$old_root" ] || claude plugin marketplace remove superpowers-dev --scope user >/dev/null
    old_root=""
  fi
  current="$(claude plugin list --json | installed_record)"
  if [ -z "$current" ]; then
    [ -n "$old_root" ] || claude plugin marketplace add "$SUB" --scope user >/dev/null
    claude plugin install superpowers@superpowers-dev --scope user >/dev/null
  fi
  record="$(claude plugin list --json | installed_record)"
  IFS='|' read -r version enabled install_path <<EOF
$record
EOF
  if [ "$enabled" != true ]; then
    claude plugin enable superpowers@superpowers-dev --scope user >/dev/null
    record="$(claude plugin list --json | installed_record)"
    IFS='|' read -r version enabled install_path <<EOF
$record
EOF
  fi
  [ "$version" = "$VERSION" ] && [ "$enabled" = true ] || { echo "setup-agents: Claude verification failed" >&2; exit 1; }
  check_skills "Claude" "$SUB/skills" "$install_path/skills" || exit 1
  echo "setup-agents: Claude Code enabled superpowers@superpowers-dev v$VERSION"
}

setup_codex() {
  command -v codex >/dev/null 2>&1 || { echo "setup-agents: Codex App/CLI is not installed; skipping"; return; }
  plugins="$(codex plugin list --available --json)"
  all_conflicts="$(printf '%s' "$plugins" | plugin_conflicts "$ROOT")" || {
    echo "setup-agents: cannot read Codex's plugin list" >&2
    exit 1
  }
  blocking="$(printf '%s\n' "$all_conflicts" | awk -F'\t' '$1 == "block"')"
  elsewhere="$(printf '%s\n' "$all_conflicts" | awk -F'\t' '$1 == "elsewhere"')"
  old_root="$(codex plugin marketplace list --json | market_root)"
  stale=0; [ -z "$old_root" ] || [ "$old_root" = "$SUB" ] || stale=1
  # Same rule as the Claude half: a plugin scoped to another checkout cannot
  # load here, so it is neither a reason to refuse nor ours to remove.
  if [ -n "$elsewhere" ]; then
    printf '%s\n' "$elsewhere" | while IFS="$(printf '\t')" read -r _ id scope project; do
      [ -n "$id" ] || continue
      echo "setup-agents: note: $id is $scope-scoped to $project — left alone"
    done
  fi
  if { [ -n "$blocking" ] || [ "$stale" -eq 1 ]; } && [ "$MIGRATE" -ne 1 ]; then
    echo "setup-agents: competing Codex Superpowers installation(s):" >&2
    printf '%s\n' "$blocking" | while IFS="$(printf '\t')" read -r _ id scope _; do
      [ -n "$id" ] || continue
      echo "  $id ($scope scope)" >&2
      echo "    remove with: codex plugin remove $id" >&2
    done
    [ "$stale" -eq 0 ] || echo "  stale marketplace superpowers-dev -> $old_root" >&2
    echo "or rerun: tools/setup-agents.sh --migrate codex" >&2
    exit 1
  fi
  if [ "$MIGRATE" -eq 1 ] && [ -n "$blocking" ]; then
    printf '%s\n' "$blocking" | while IFS="$(printf '\t')" read -r _ id _ _; do
      [ -n "$id" ] || continue
      codex plugin remove "$id" >/dev/null || {
        echo "setup-agents: could not remove $id" >&2
        exit 1
      }
    done || exit 1
  fi
  if [ "$MIGRATE" -eq 1 ]; then
    [ -z "$old_root" ] || codex plugin marketplace remove superpowers-dev >/dev/null
    old_root=""
  fi
  current="$(codex plugin list --available --json | installed_record)"
  if [ -z "$current" ]; then
    [ -n "$old_root" ] || codex plugin marketplace add "$SUB" >/dev/null
    codex plugin add superpowers@superpowers-dev >/dev/null
  fi
  record="$(codex plugin list --available --json | installed_record)"
  IFS='|' read -r version enabled install_path <<EOF
$record
EOF
  if [ -z "$install_path" ]; then
    install_path="${CODEX_HOME:-$HOME/.codex}/plugins/cache/superpowers-dev/superpowers/$VERSION"
  fi
  [ "$version" = "$VERSION" ] && [ "$enabled" = true ] || { echo "setup-agents: Codex verification failed" >&2; exit 1; }
  check_skills "Codex" "$SUB/skills" "$install_path/skills" || exit 1
  echo "setup-agents: Codex App/CLI enabled superpowers@superpowers-dev v$VERSION"
}

# Sourcing this file defines its functions and stops, which is what
# test/setup_agents_test.sh drives — the same seam tools/bootstrap.ps1 offers
# through -NoRun. Everything with a side effect lives below this line.
main() {
  MIGRATE=0
  if [ "${1:-}" = --migrate ]; then MIGRATE=1; shift; fi
  HOST="${1:-all}"
  [ "$#" -le 1 ] || { echo "usage: setup-agents.sh [--migrate] [claude|codex|all]" >&2; exit 2; }
  case "$HOST" in claude|codex|all) ;; *) echo "usage: setup-agents.sh [--migrate] [claude|codex|all]" >&2; exit 2 ;; esac

  ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
  SUB="$ROOT/tools/superpowers"
  # Sourced here, not at file scope: ROOT does not exist until main runs, and
  # sourcing this file must define functions without reading anything.
  # shellcheck source=tools/lib/digest.sh
  . "$ROOT/tools/lib/digest.sh"
  LOCK="$ROOT/ci/superpowers.lock"
  [ -f "$LOCK" ] || { echo "setup-agents: no lock at $LOCK" >&2; exit 1; }
  PIN="$(sp_lock_get pin "$LOCK")"
  VERSION="$(sp_lock_get version "$LOCK")"
  [ -n "$PIN" ] && [ -n "$VERSION" ] \
    || { echo "setup-agents: $LOCK is missing pin or version" >&2; exit 1; }
  # Preflight, the way tools/setup-mcp.sh gates on node: every JSON reader here
  # shells out to python3, and without it this script died mid-run with a bare
  # "python3: not found" after having already touched the plugin marketplace.
  command -v python3 >/dev/null 2>&1 || {
    echo "setup-agents: FAILED python3 not found" >&2
    echo "setup-agents: fix: install Python 3 (it reads the agent CLIs' JSON output), then re-run" >&2
    exit 2
  }

  gitdir="$(git -C "$ROOT" rev-parse --path-format=absolute --git-dir)"
  common="$(git -C "$ROOT" rev-parse --path-format=absolute --git-common-dir)"
  [ "$gitdir" = "$common" ] || {
    echo "setup-agents: run from the durable primary checkout, not a linked worktree" >&2
    exit 1
  }

  git -C "$ROOT" submodule update --init --depth 1 -- tools/superpowers
  "$ROOT/ci/verify-superpowers.sh" full

  case "$HOST" in claude) setup_claude ;; codex) setup_codex ;; all) setup_claude; setup_codex ;; esac
  echo "Restart Claude Code or begin a new Codex session so the pinned skills load."
}

[ -n "${UNSEEING_SETUP_AGENTS_NORUN:-}" ] || main "$@"
