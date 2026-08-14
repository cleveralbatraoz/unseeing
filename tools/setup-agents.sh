#!/bin/sh
set -eu

MIGRATE=0
if [ "${1:-}" = --migrate ]; then MIGRATE=1; shift; fi
HOST="${1:-all}"
[ "$#" -le 1 ] || { echo "usage: $0 [--migrate] [claude|codex|all]" >&2; exit 2; }
case "$HOST" in claude|codex|all) ;; *) echo "usage: $0 [--migrate] [claude|codex|all]" >&2; exit 2 ;; esac

ROOT="$(cd "$(dirname "$0")/.." && pwd -P)"
SUB="$ROOT/tools/superpowers"
# One home for the pin, the tag and the version: ci/superpowers.lock, which
# tools/update-superpowers.sh rewrites. They used to be spelled out in four
# files the update path never touched.
sp_lock_get() { sed -n "s/^$1=//p" "$2" | head -1; }
LOCK="$ROOT/ci/superpowers.lock"
[ -f "$LOCK" ] || { echo "setup-agents: no lock at $LOCK" >&2; exit 1; }
PIN="$(sp_lock_get pin "$LOCK")"
VERSION="$(sp_lock_get version "$LOCK")"
[ -n "$PIN" ] && [ -n "$VERSION" ] \
  || { echo "setup-agents: $LOCK is missing pin or version" >&2; exit 1; }
# Preflight, the way tools/setup-mcp.sh gates on node: every JSON reader below
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

json_ids() {
  python3 -c 'import json,sys
d=json.load(sys.stdin); xs=d if isinstance(d,list) else d.get("installed",[])
print("\n".join(x.get("id",x.get("pluginId","")) for x in xs if x.get("id",x.get("pluginId","")).startswith("superpowers@")))'
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
# shellcheck source=tools/lib/digest.sh
. "$ROOT/tools/lib/digest.sh"
# Was bare `shasum` over a prefix-stripped path list. shasum is a Perl script
# and is simply absent from minimal and container images — and with it missing
# BOTH sides of the comparison below came back empty, so a tampered skill cache
# passed this gate silently. The library refuses instead of agreeing.
hash_tree() { unseeing_digest_tree "$1"; }

setup_claude() {
  command -v claude >/dev/null 2>&1 || { echo "setup-agents: Claude Code is not installed; skipping"; return; }
  plugins="$(claude plugin list --json)"
  ids="$(printf '%s' "$plugins" | json_ids)"
  conflicts="$(printf '%s\n' "$ids" | awk 'NF && $0 != "superpowers@superpowers-dev"')"
  old_root="$(claude plugin marketplace list --json | market_root)"
  stale=0; [ -z "$old_root" ] || [ "$old_root" = "$SUB" ] || stale=1
  if { [ -n "$conflicts" ] || [ "$stale" -eq 1 ]; } && [ "$MIGRATE" -ne 1 ]; then
    echo "setup-agents: competing Claude Superpowers installation(s):" >&2
    printf '  %s\n' $conflicts >&2
    [ "$stale" -eq 0 ] || echo "  stale marketplace superpowers-dev -> $old_root" >&2
    echo "Remove with: claude plugin uninstall <selector> --scope user" >&2
    echo "and: claude plugin marketplace remove superpowers-dev --scope user" >&2
    echo "or rerun: tools/setup-agents.sh --migrate claude" >&2
    exit 1
  fi
  if [ "$MIGRATE" -eq 1 ]; then
    for id in $ids; do claude plugin uninstall "$id" --scope user >/dev/null; done
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
  [ "$(hash_tree "$SUB/skills")" = "$(hash_tree "$install_path/skills")" ] || { echo "setup-agents: Claude skill cache differs from repository pin" >&2; exit 1; }
  echo "setup-agents: Claude Code enabled superpowers@superpowers-dev v$VERSION"
}

setup_codex() {
  command -v codex >/dev/null 2>&1 || { echo "setup-agents: Codex App/CLI is not installed; skipping"; return; }
  plugins="$(codex plugin list --available --json)"
  ids="$(printf '%s' "$plugins" | json_ids)"
  conflicts="$(printf '%s\n' "$ids" | awk 'NF && $0 != "superpowers@superpowers-dev"')"
  old_root="$(codex plugin marketplace list --json | market_root)"
  stale=0; [ -z "$old_root" ] || [ "$old_root" = "$SUB" ] || stale=1
  if { [ -n "$conflicts" ] || [ "$stale" -eq 1 ]; } && [ "$MIGRATE" -ne 1 ]; then
    echo "setup-agents: competing Codex Superpowers installation(s):" >&2
    printf '  %s\n' $conflicts >&2
    [ "$stale" -eq 0 ] || echo "  stale marketplace superpowers-dev -> $old_root" >&2
    echo "Remove with: codex plugin remove <selector>" >&2
    echo "and: codex plugin marketplace remove superpowers-dev" >&2
    echo "or rerun: tools/setup-agents.sh --migrate codex" >&2
    exit 1
  fi
  if [ "$MIGRATE" -eq 1 ]; then
    for id in $ids; do codex plugin remove "$id" >/dev/null; done
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
  [ "$(hash_tree "$SUB/skills")" = "$(hash_tree "$install_path/skills")" ] || { echo "setup-agents: Codex skill cache differs from repository pin" >&2; exit 1; }
  echo "setup-agents: Codex App/CLI enabled superpowers@superpowers-dev v$VERSION"
}

case "$HOST" in claude) setup_claude ;; codex) setup_codex ;; all) setup_claude; setup_codex ;; esac
echo "Restart Claude Code or begin a new Codex session so the pinned skills load."
