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

## Status: half-installed by this repository, by design

The two halves install differently, on purpose, because only one of them is
safe to commit.

- **The MCP client half ships in-tree.** `.mcp.json` at the repo root
  declares the `godot-mcp` server (`npx -y
  @satelliteoflove/godot-mcp@<pinned version>`, no serve flags needed — the
  plain invocation speaks MCP over stdio). Any MCP client that reads
  `.mcp.json` picks this up with no setup step at all.
- **The addon half installs on demand, per machine.** Run
  `tools/setup-mcp.sh` from the repo root: it checks for Node 20+, then runs
  the pinned `npx @satelliteoflove/godot-mcp@<version> --install-addon
  game`. The one step it cannot script is enabling the plugin in the editor
  (below) — that stays a manual, per-machine click.
- **The addon stays untracked, always, by policy** — not an installation gap
  to close, a decision to keep (see below). `tools/setup-mcp.sh` writes it to
  disk; nothing ever adds it to git.

## Prerequisites

- **Node.js 20+.** Not currently a dependency of anything else here — the game,
  the tests and the deploy pipeline all run without it.
- **Godot 4.7 editor, running, with the project open.** The server has no
  headless mode; it talks to the editor over a WebSocket and to the running
  game over the debugger protocol.
- **One MCP client at a time.** A second client queues rather than taking over.

## Install

```sh
tools/setup-mcp.sh
```

Then enable the plugin in the editor: **Project → Project Settings → Plugins →
Godot MCP → Enable**. That step is a GUI click; there is no scripted equivalent
— see `tools/setup-mcp.sh`'s own comment for why it deliberately does not
touch `game/project.godot` to do this for you.

### The addon is gitignored, deliberately

`game/addons/godot_mcp/` is in `.gitignore`, and `test/repo_hygiene.sh` pins
that from both directions — nothing under the path may be tracked, and the
ignore rule must still cover it. The reasons are load-bearing project hygiene;
`AGENTS.md` is authoritative and `CLAUDE.md` is only its adapter:

1. `deploy.sh` ships the tree by `git archive` into a bare repo whose
   post-receive hook untars it. Anything committed under `game/addons/`
   reaches the **droplet checkout** — an editor-only Node tool occupying the
   server tree for no runtime purpose.
2. Every export preset excludes `addons/*`, so the addon is deliberately not
   a game/export dependency. Tracking an editor-only Node tool would still
   pollute the repository and server checkout for no shipped benefit.
3. The enabled-plugin list is stored in tracked `game/project.godot`, but the
   addon is per-machine. Committing that entry would make fresh clones open a
   broken plugin row and let Godot rewrite the tracked file according to local
   install state. Setup therefore installs the ignored addon and leaves the
   one enable click local.

The cost of this choice: the version is not pinned in-tree by any lock file
(unlike gdUnit4's `ci/gdunit4.lock`) — `.mcp.json` and `tools/setup-mcp.sh`
each carry their own version literal, so two machines that update at
different times can still end up running different addon builds against the
same server. Accepted — it is a debugging aid, not a dependency of the build.

## The loop

Five steps. The order is not decoration: **freeze first**, or the state moves
between the question and the answer.

```
godot_game_time   freeze                     the clock stops
godot_input       tap / walk                 the hero makes a sound
godot_game_time   step 30 frames             advance exactly, deterministically
godot_exec        observer.snapshot(now)     the state vector, as JSON
godot_exec        observer.explain_*(...)    why, when the snapshot is not enough
```

`godot_exec` returns the value of the GDScript it runs, so every question is a
one-liner. The observer is reachable from the scene root:

```gdscript
JSON.stringify(get_tree().root.get_node("Main").observer.snapshot(now))
```

`WaveObserver` runs with `ProcessMode::ALWAYS` precisely so step 1 does not
break steps 4 and 5 — a frozen tree would otherwise stop `_physics_process` and
leave `take_explanation` answering `{"pending": true}` forever.

## Reading the answers

- **A refusal carries exactly one key**, `unavailable`, with a reason. It is
  never a zero and never an empty array. If you got a refusal, the observer
  could not see — do not interpret it as "nothing there".
- **An unobservable field is omitted**, and its name appears in the `unknown`
  array. `snap["flick"]` on such a field raises a GDScript invalid-key error.
  That is deliberate: a plausible zero is worse than a loud absence.
- **`take_explanation` answers exactly once.** A second collect gets a refusal.

## Validate the meshes as structured data too

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
   populated, then require 144/144. This covers level 01's wedges and columns,
   fan, radio, cat, hero/cane, and the world box path together.

`godot_validate_meshes` walks the **running** `SceneTree.current_scene`, not
the editor's edited-scene root. Editor-only fan/radio/cat blueprint presence is
therefore proved by `tools/probe_editor_sources.sh`; the production mesh routes
are checked through the configured runners above. Record the actual structured
counts whenever topology changes; a screenshot cannot substitute for them.

## The rule about screenshots

**A screenshot is the last resort, not the first.** Take one only when a
structured answer contradicts itself, or when you have established that no
structured answer exists for the question.

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
