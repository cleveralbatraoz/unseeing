#!/bin/sh
# Installs the godot-mcp editor addon, so a Claude session can drive a
# running Godot editor directly — freeze the clock, step frames, inject
# input, read WaveObserver as structured JSON — instead of rendering a
# screenshot and guessing. See docs/superpowers/mcp/godot-mcp-loop.md for
# the loop itself; this script only gets the addon onto disk.
#
# Two halves, one script covers the second. The MCP CLIENT half — the
# "godot-mcp" server entry an MCP client launches over stdio — ships in
# .mcp.json at the repo root and needs no setup of its own. This script
# installs the other half: the Godot-side bridge addon under
# game/addons/godot_mcp/, which is NOT a tracked project file (see below).
#
# Env knobs: GODOT_MCP_VERSION (pin override; default matches .mcp.json).
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
VERSION="${GODOT_MCP_VERSION:-4.1.0}"

echo "setup-mcp: checking for node >= 20"
if ! command -v node >/dev/null 2>&1; then
  echo "setup-mcp: FAILED node not found"
  echo "setup-mcp: fix: install Node.js 20+ from https://nodejs.org (or your package manager), then re-run tools/setup-mcp.sh"
  exit 2
fi
NODE_MAJOR="$(node --version | sed 's/^v//' | cut -d. -f1)"
if [ "$NODE_MAJOR" -lt 20 ]; then
  echo "setup-mcp: FAILED node $(node --version) is older than the required 20+"
  echo "setup-mcp: fix: upgrade Node.js (nvm install 20, or https://nodejs.org), then re-run tools/setup-mcp.sh"
  exit 2
fi
echo "setup-mcp: node OK ($(node --version))"

echo "setup-mcp: installing the godot-mcp editor addon into game/addons/godot_mcp (pinned @$VERSION)"
npx -y "@satelliteoflove/godot-mcp@$VERSION" --install-addon "$DIR/game" || {
  echo "setup-mcp: FAILED addon install (see npx output above)"
  exit 1
}

# Deliberately NOT touching game/project.godot's plugin-enabled list: that
# file is tracked, so enabling the plugin here would commit a reference to
# an addon that stays untracked by policy (game/addons/godot_mcp/ is
# gitignored — see AGENTS.md's godot-mcp policy; CLAUDE.md is only its adapter.
# A committed addon would ship to the droplet and the wasm export via
# deploy.sh's `git archive`, and would break ci/vendor-gdunit4.sh's assumption
# that gdUnit4 is the only tenant of game/addons/). That reference would NOT be
# caught by this project's CI boot gate: [editor_plugins] only loads in the
# EDITOR, so the headless `--quit-after` boot check (no `-e`) produces no
# output at all for a missing addon, and even an editor-mode run only WARNS
# ("Addon ... failed
# to load ... Removing from enabled plugins" — a WARNING line, and
# ci/boot_error_pattern.sh deliberately excludes those). The real cost is
# local instead: every clone that has not run this script would open the
# editor to a broken row in Project Settings > Plugins, and Godot's own
# load-failure handling REMOVES the dead entry from project.godot on that
# same launch — a tracked file rewritten differently by whatever a given
# machine happens to have installed. Enabling stays manual, per machine, so
# project.godot never carries a reference this repo cannot guarantee is on
# disk.
echo "setup-mcp: addon installed — one manual step left, once per machine:"
echo "setup-mcp:   open game/project.godot in the Godot editor, then"
echo "setup-mcp:   Project > Project Settings > Plugins > Godot MCP > Enable"
echo "setup-mcp: OK"
