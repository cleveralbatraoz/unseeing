# The live debugging loop — godot-mcp driving WaveObserver

*Written for a session that has never seen this before. If you are about to
debug something visual in this game, read this instead of rendering a frame
and looking at it.*

## What this is for

Debugging Unseeing used to mean rendering frames and staring at them. That is
slow, expensive in context, and mute about causes: a picture can show that a
seam did not draw, but it cannot say whether the participating faces were
intentionally joined into one superface or carried labels without the required
separation.

`WaveObserver` answers those questions as structured data. **godot-mcp** is
what lets you ask them from inside a conversation, against a running game,
without writing a scene or a test first.

The two halves are independent. The observer works in gdUnit4 suites with no
MCP at all; godot-mcp is a convenience for the interactive loop.

## Status: pinned client plus an ignored, worktree-local addon

The two halves install differently because only the client declaration is safe
to commit.

- **The MCP client declaration ships in-tree.** `.mcp.json` launches exact
  `@satelliteoflove/godot-mcp@4.1.0` through `npx -y`. That MCP stdio process is
  a WebSocket client of the editor addon; it does not listen on the addon's
  port.
- **The editor addon installs in every task worktree that uses it.**
  `tools/setup-mcp.sh` checks Node 20+ and installs exact 4.1.0 under that
  worktree's `game/addons/godot_mcp/`. Ignored files and compiled GDExtension
  output do not propagate between Git worktrees, so each worktree must run its
  own checked-in bootstrap and addon installer.
- **The addon stays ignored and untracked.** It is developer-only,
  export-excluded and forbidden from Git by `test/repo_hygiene.sh`. It is never
  a game, build, test or deployment dependency.

## Prerequisites

- **Node.js 20+.** The game, tests and GitHub Pages pipeline do not otherwise
  depend on Node.
- **The repository-pinned graphical Godot editor.** A headless import is not a
  substitute for the editor session to which the addon attaches.
- **One client per addon endpoint.** A second client is rejected; it may retry
  after the owner disconnects, but it is not queued and cannot take over.

## Install in an isolated task worktree

Never enable the addon in the durable primary clone. Create or select an
isolated task worktree, require its tracked tree clean, and bootstrap that
worktree's own release library and class census. From anywhere inside that
worktree, the resource-bounded Debian form is:

```sh
set -eu
REPO="$(git rev-parse --show-toplevel)"
cd "$REPO"
. "$HOME/.cargo/env"
CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 \
  tools/bootstrap.sh
```

Both `.mcp.json` and `tools/setup-mcp.sh` own the same 4.1.0 literal. Remove an
inherited override, verify the registry integrity, install, and validate the
worktree-local result:

```sh
set -eu
REPO="$(git rev-parse --show-toplevel)"
cd "$REPO"
test "$(npm view @satelliteoflove/godot-mcp@4.1.0 dist.integrity)" = \
  'sha512-uq3Gh5n7fos8vIoXpr32/K7r9tL9eYLbERr+Tolksg3Y+FC5coYEkRkbJ1JktMMhoH/BnGWsWhE5E+XJ/nMEPg=='
/usr/bin/env -u GODOT_MCP_VERSION ./tools/setup-mcp.sh
test "$(sed -n 's/^version="\([^"]*\)"$/\1/p' \
  game/addons/godot_mcp/plugin.cfg)" = 4.1.0
git check-ignore -q game/addons/godot_mcp/plugin.cfg
test -z "$(git status --short)"
```

This install may populate npm's cache; inventory it instead of deleting shared
cache state broadly. Do not use `GODOT_MCP_VERSION` to drift one half. Updating
means reviewing a new package and integrity, changing both checked-in literals
in an isolated branch, reinstalling in a fresh worktree, and repeating the
complete editor and structured proof before integration.

## Enabling is temporary tracked project state

Before enabling, require `game/project.godot` byte-identical to tracked `HEAD`
and capture its bytes, SHA-256, owner, mode, device and inode. Use the normal
local editor route: **Project → Project Settings → Plugins → Godot MCP →
Enable**. A remotely supervised first boot must add only that exact plugin row
before launch.

Version 4.1.0 also creates the `MCPGameBridge` autoload and four settings:
`godot_mcp/bind_mode`, `godot_mcp/custom_bind_ip`,
`godot_mcp/port_override_enabled`, and `godot_mcp/port_override`, then saves
`project.godot`. Disabling removes none of that persistent state. Never stage
the session diff or hide it with `assume-unchanged` or `skip-worktree`. The
closing procedure must accept only this complete MCP boundary and restore the
captured exact preimage.

For deterministic windowed use, a reviewed lifecycle owner may create only
this exact ignored `game/override.cfg` and must always remove it:

```text
[display]

window/size/mode=0
window/size/viewport_width=1280
window/size/viewport_height=720
```

Refuse a pre-existing override. If any unrelated `project.godot` bytes appear,
preserve a blocking recovery artifact and do not overwrite the live file.

### The addon is gitignored, deliberately

`game/addons/godot_mcp/` is in `.gitignore`, and `test/repo_hygiene.sh` pins
that from both directions: nothing under the path may be tracked, and the rule
must continue to cover it. `AGENTS.md` is authoritative.

1. Tracking the editor-only Node bridge would pollute every source clone and
   Git archive for no runtime purpose.
2. Every export preset excludes `addons/*`; the addon is deliberately not a
   game or export dependency.
3. A tracked plugin row would make worktrees without the ignored addon open a
   broken plugin and let Godot rewrite `project.godot` according to local state.

The version is pinned twice in-tree—`.mcp.json` and `tools/setup-mcp.sh`—and
both literals move together. This is not a lock-file regime like gdUnit4's
`ci/gdunit4.lock`; reviewed registry integrity and the installed manifest are
the additional per-install evidence.

## Local editor versus a remote editor

When client and editor run on the same host, use the addon's default loopback
`127.0.0.1:6550`; no SSH forward is needed. Do not connect a second client to
that endpoint.

A remote controller must select one unused loopback port and configure both
ends. The dated hp-local proof reserves 16550 so unrelated default-port clients
remain untouched: its supervised project state sets
`godot_mcp/port_override_enabled=true` and
`godot_mcp/port_override=16550`, while its owned stdio process sets
`GODOT_HOST=127.0.0.1` and `GODOT_PORT=16550`.

Before starting SSH, inspect base `ssh -G hp-local` and refuse inherited local,
remote or dynamic forwards; control path/master/persistence or fork-after-
authentication; proxy command/jump; or local command. Inspect the full proposed
command through `ssh -G` too and require exactly the sole local forward below
plus `ForkAfterAuthentication=no`. `ClearAllForwardings=yes` is not a guard
here: it would also discard the required command-line `-L`. Both base and full
effective configurations reject `ClearAllForwardings=yes`.

The reviewed, PID-owning, signal-cleaned, maximum-lease wrapper uses exactly:

```text
ssh -N -T \
  -o BatchMode=yes \
  -o ExitOnForwardFailure=yes \
  -o ForkAfterAuthentication=no \
  -o ControlMaster=no \
  -o ControlPath=none \
  -o ControlPersist=no \
  -o PermitLocalCommand=no \
  -o UpdateHostKeys=no \
  -o StrictHostKeyChecking=yes \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=3 \
  -L 127.0.0.1:16550:127.0.0.1:16550 \
  hp-local
```

Prove the exact SSH PID owns the controller listener and the owned Godot child
owns the target listener. Never use `-g`, a wildcard/non-loopback bind, a
background tunnel with no cleanup owner, or a shared control socket.

The dated proof has one absolute monotonic 1200-second deadline with a
1170-second mutation-capable work cutoff and a final fixed 30-second cleanup
reserve. No mutation-capable editor, tunnel, controller, or MCP work may start
after the cutoff. The deadline begins before the first supervisor/editor,
tunnel, or controller startup and includes the structured proof, every
`finally` path, child termination, project restore, override removal, tunnel
close, and listener-absence proof. No component may mint, restart, or extend
either endpoint; reaching the deadline without completed cleanup is a failed
proof, not permission for a later unowned cleanup phase.

For the dated proof, do not invoke npm/npx or the application's built-in MCP
process. A reviewed controller validates exact existing NPX tree
`/Users/dmgalchenko/.npm/_npx/e9af8ac9cd94a1c8`, including the package lock,
all package files, Godot MCP 4.1.0 integrity
`sha512-uq3Gh5n7fos8vIoXpr32/K7r9tL9eYLbERr+Tolksg3Y+FC5coYEkRkbJ1JktMMhoH/BnGWsWhE5E+XJ/nMEPg==`,
and resolved MCP SDK 1.30.0 integrity
`sha512-xKd8OIzlqNzcqcNumGAa6g+PW2kjD5vrpcKOnfldAUPP3j7lnqMPwlTXQm8gF+UwH72z0lqaRbjr9hqGz0eITA==`.
Ordinary addon installation still requires Node 20 or newer. This dated
controller instead requires exact Node `22.23.2` and validates the direct
executable identity. It copies descriptor-held reviewed source bytes into a
private execution capsule, then imports the parent SDK graph from a held
`Buffer` through `registerHooks()` without reopening reviewed pathnames. The
child's deterministic preload provides the same held-byte boundary. Sealed
parent and child resolution ledgers bind every admitted request, target and
format before `process.execPath` spawns the staged `dist/cli.js`.

The SDK transport inherits only present safe names `HOME`, `LOGNAME`, `PATH`,
`SHELL`, `TERM`, and `USER`; add fixed `GODOT_HOST=127.0.0.1`,
`GODOT_PORT=16550`, and `GODOT_MCP_USAGE_LOG=0`, and reject every other child-
environment name. Record the allowed environment and process identities, close
only the owned stdio child in `finally`, and require the pre-existing
`~/.godot-mcp/usage.log` path, device, inode, UID, mode, link count, size, line
count, SHA-256, and exact descriptor `mtime_ns` `1787340988551255243`
unchanged. The rounded `1787340988000000000` transcription is rejected. That unchanged-log
requirement belongs only to the dated 2026-08-25 owned-controller proof, which
captures a baseline and disables logging for its child; ordinary clients may
use the package's default logging and must not claim this boundary. List tools
before use. Package source plus fixed overrides prove the endpoint;
`addon_status` does not.

Before SDK import, consume a fresh supervisor contract binding the previously
absent named unit, journal cursor, unit/Godot start identities, exact project
and evidence roots, and the pending terminal gate. Controller success may
report only `controller_lane_status:"passed"` with
`integrated_proof_status:"pending_game_log_gate"`. After game stop, the
supervisor-owned game-process journal finalizer reads only that unit and cursor
interval, sanitizes and hashes it, requires zero game-process errors, and binds
the terminal result back to the contract. An MCP error-log result alone is not
integrated success.

## Dated hp-local editor-only result

Attempt 7 passed the separately scoped editor-only proof on 2026-08-26. The
owned connection progressed through the real readiness race from disconnected
and unknown fields to a complete server/addon/project/editor handshake. It
opened `res://scenes/level_02.tscn`, selected `/root/Level02/Room`, and returned
Godot UI identity `4.7.1-stable (official)` for the official binary whose CLI
identity is `4.7.1.stable.official.a13da4feb`. One transient 640-by-432 editor
capture was visually acknowledged, hashed in the evidence, and retired; no
image bytes are tracked.

Attempts 1--6 stopped cleanly on, in order: excessive host clock skew; an
incorrect `0644` project-mode assumption against the real `0600` worktree;
two stages of a launch `TypeError` investigation; the resulting version-probe
callback fix; and macOS incompatibility with executing the held
`/dev/fd/<ssh-fd>` lease followed by Godot's atomic project-inode replacement.
The successful retry used direct-owned Godot and SSH, with a reviewed
content/metadata restoration guard for the known save behavior. The full
chronology and exact hashes live in the
[hp-local development setup guide](../../hp-local-development-setup.md).

Cleanup left editor errors unchanged, the complete usage-log boundary
unchanged, the tracked project exactly restored, both tracked trees clean, and
all owned processes/listeners/override state absent. The isolated worktree and
its ignored addon/build caches are intentionally retained for development.
No runtime-game MCP claim is made by this result: no game was launched and the
six-call runtime/movement/mesh/journal sequence below remains a separate
protocol. Earlier native and Web run proof is likewise separate.

## The loop

First list the owned connection's tools and require exact server/addon version,
project path, Godot version, configured main scene and editor state. That is a
compatibility/identity preflight; the package source, fixed environment and
listener ownership remain the endpoint proof.

Then use six calls. The order is not decoration: **freeze first**, or the
state moves between the question and the answer. After the game starts, inspect
its runtime-generated bindings with `godot_input {action:"get_map"}`; editor
`ProjectSettings` do not contain that runtime map. Require `move_forward` to
have at least one event before stepping it.

```
godot_editor_edit run, frozen=true           start with the clock stopped
godot_input       action=get_map              inspect the running game's bindings
godot_game_time   step, frames=2              initialize exactly
godot_exec        action=run, snapshot source capture the before state as JSON
godot_game_time   step, frames=30, inputs=[{action_name:"move_forward",start_ms:0,duration_ms:500}]
godot_exec        action=run, snapshot source capture the after state as JSON
```

The fifth call requests `frames:30`; Godot MCP 4.1.0's accepted reply reports
`frames:31`, comprising the 30 requested frames plus one input-settle frame.
Require that exact distinction rather than rewriting either side as the other.
Input belongs inside the `godot_game_time` `step.inputs` request. A separate
`godot_input` injection while frozen misses the edge that the stepped frames
must consume.

`godot_exec` exposes injected `root` and `tree` variables and returns only an
explicit GDScript `return` value. Use the running current scene, never
`get_tree()` or an assumed root-child name:

```gdscript
var main := tree.current_scene
return JSON.stringify(main.observer.snapshot(main.now))
```

`WaveObserver` runs with `ProcessMode::ALWAYS` precisely so step 1 does not
break steps 4 and 6 — a frozen tree would otherwise stop `_physics_process` and
leave `take_explanation` answering `{"pending": true}` forever.

## Reading the answers

- **A refusal carries exactly one key**, `unavailable`, with a reason. It is
  never a zero and never an empty array. If you got a refusal, the observer
  could not see — do not interpret it as "nothing there".
- **An unobservable field is omitted**, and its name appears in the `unknown`
  array. `snap["flick"]` on such a field raises a GDScript invalid-key error.
  That is deliberate: a plausible zero is worse than a loud absence.
- **`take_explanation` answers exactly once.** A second collect gets a refusal.

## Close and uninstall without losing project state

The ordinary manual and dated automated sessions intentionally close
differently.

For an ordinary local manual session, the order is fixed:

1. stop the running game, then close the owning MCP client;
2. Disable Godot MCP in the editor;
3. close the editor;
4. require the post-disable `project.godot` diff to contain exactly the
   surviving `MCPGameBridge` autoload and four `godot_mcp/*` settings, with the
   enabled row absent; preserve recovery evidence and stop on any other byte;
5. remove only the exact reviewed `override.cfg`, restore the captured
   `project.godot` preimage, and verify exact bytes, SHA-256, UID, GID and mode.
   Record device plus original/replacement inode identities as facts without
   requiring equality; access and modification timestamps are outside this
   restoration contract; and
6. require clean tracked status.

The full dated automated runtime protocol does **not** create a post-disable
phase:

1. stop the game through the owned controller and close that controller plus
   only its stdio child;
2. stop the transient editor unit while the plugin is still enabled;
3. require the stopped editor's complete diff to contain exactly the enabled
   row, `MCPGameBridge` autoload and four settings;
4. have the supervisor remove only its override and restore the captured
   preimage directly under that same exact bytes/SHA-256/UID/GID/mode contract,
   with device/inodes recorded as facts and timestamps outside the contract;
5. close the owned SSH lease and prove both loopback ports released within the
   same 1200-second overall deadline; and
6. require both tracked trees clean and the dated controller usage-log boundary
   unchanged.

If the project diff contains unrelated bytes, preserve a recovery artifact and
stop; do not restore over them. Disabling the plugin or deleting the addon alone
is not an uninstall because the autoload and four settings survive disablement.

Keeping the ignored addon is normal for later sessions in the same worktree.
To remove it, first complete and verify the project restoration above, then
require its canonical worktree-relative path and verify every exact recorded
manifest entry's path, type, owner, mode and file hash before removing only
those entries beneath `game/addons/godot_mcp`. Retain shared npm and Godot
caches unless a separate complete ownership proof authorizes their removal.

Removing the whole worktree has an additional guard: stop every owned
editor/unit/client/tunnel; require exact branch/HEAD and clean tracked state;
then remove each manifest-authorized ignored output by verified canonical path,
type, owner, mode and file hash. Only with no residue may the durable checkout use non-forced
`git worktree remove`, verify both path and admin entry absent, and use safe
`git branch -d` for a no-unique-commit branch. Never use force or recursive
manual worktree deletion.

## Validate the meshes through the live tool too

`WaveObserver` explains the simulation and paint graph; it does not inspect an
`ArrayMesh`'s submitted index order. Use `godot_validate_meshes` for that
separate boundary. The official
[Godot 4.7 ArrayMesh reference](https://docs.godotengine.org/en/4.7/classes/class_arraymesh.html)
defines **clockwise** triangles as front-facing, so an actual submitted
triangle with an outward stored normal must satisfy:

```text
(vertex_1 - vertex_0).cross(vertex_2 - vertex_0).dot(outward_normal) < 0
```

The Rust geometry generators deliberately use the conventional mathematical
contract before submission: their counter-clockwise triangles have a positive
outward dot product. `rust/src/render/paint.rs` converts those complete triples
at the Godot boundary. Hero/cat sphere and tube buffers are the exception by
contract: `rust/src/nodes/limbs.rs` already emits Godot-clockwise triples and
uses the direct, non-converting door. A blanket reversal would turn those
limbs inside-out.

The validator is not cosmetic. The world skin in
`game/shaders/data_pass.gdshader` is `cull_disabled`, so backwards world
triangles are not currently dropped and its unshaded shader does not consume
their normals. Source limbs use `game/shaders/data_xray.gdshader`, which is
`cull_back`: backwards source geometry can lose the intended exterior/near
faces and expose farther or interior faces under the always-pass depth path.
That can corrupt the source's packed distance and self-overlap even when the
closed mesh does not disappear completely.

For campaign closeout, run the validator after the final release rebuild and
Godot import in all three states below, and require zero findings in each:

1. raw `level_02.tscn`, covering the six derived wall segments, two slabs,
   and six chair pieces. An uninjected raw WaveLevel intentionally does not
   build its runtime fan, so this state is expected to report 14 mesh
   resources / 14 triangle surfaces;
2. a code-free `UnseeingGame` runner selecting `level_02.tscn`. Step or poll
   until both hero ArrayMeshes have surfaces, then require 24/24; this covers
   the same level plus the injected fan's box, column, and torus paths and the
   hero/cane direct path;
3. the configured main scene, likewise stepped until both hero meshes are
   populated, then require 144/144. Godot MCP 4.1.0 returns this clean case as
   plain text, not `structuredContent`; require the complete value anchored to:

   ```text
   Checked 144 meshes (144 surfaces) — no integrity problems. This rules out winding, dropped triangles, degenerate UVs/tangents, and NaN data. If rendering still looks wrong, the cause is lighting or materials, not mesh data — note that SDFGI replaces constant ambient light, so shadow-side fill must come from a shadowless fill DirectionalLight rather than ambient_light_energy.
   ```

   This covers level 01's wedges and columns, fan, radio, cat, hero/cane, and
   the world box path together.

`godot_validate_meshes` walks the **running** `SceneTree.current_scene`, not
the editor's edited-scene root. Editor-only fan/radio/cat blueprint presence is
therefore proved by `tools/probe_editor_sources.sh`; the production mesh routes
are checked through the configured runners above. Record the actual tool result
whenever topology changes; a screenshot cannot substitute for it.

## The rule about screenshots

**A screenshot is the last resort, not the first.** Take one only when a
structured answer contradicts itself, or when you have established that no
structured answer exists for the question.

The one explicit setup-proof exception is a single 640-pixel-wide 3D editor
viewport capture requested to validate the actual GUI. Inspect it transiently;
retain canonical dimensions and the tool-result hash, not screenshot bytes, in
the evidence ledger. It does not replace any structured assertion.

**If you find yourself reaching for one, that is a signal, not a workflow.** It
means the observability layer has a hole where you are standing. Say so, and
what you needed — the hole is more valuable than the screenshot.

## The limit you must know before trusting any of this

**`explain_ray` is an oracle for what Rust believes, never for what the screen
draws.** It calls `rust/src/sight.rs` and never touches the GPU. No gate in this
repository asserts that the hand-transliterated GLSL in
`game/shaders/pulse_pool.gdshaderinc` still agrees with it.

So if the Rust and the picture disagree, this layer will side with the Rust and
tell you the shader is fine. When a rendering bug survives a clean
`explain_ray`, suspect the shader — do not conclude the geometry is innocent.

The wiki page *Engineering — Debugging and Observability* lists the six shader
constructs that no gate currently covers.

## When the editor is not available

Do not degrade to guessing from source. The same observables are reachable
through gdUnit4 suites in `game/tests/`, which run headless from
`ci/pipeline.sh`. Use those, and say which transport you used.
