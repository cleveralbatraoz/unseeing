# The live debugging loop — godot-mcp driving WaveObserver

*Written for a session that has never seen this before. If you are about to
debug something visual in this game, read this instead of rendering a frame
and looking at it.*

## What this is for

Debugging Unseeing used to mean rendering frames and staring at them. That is
slow, expensive in context, and mute about causes: a picture can show that a
seam did not draw, but it cannot say that two solids were handed the same
object id.

`WaveObserver` answers those questions as structured data. **godot-mcp** is
what lets you ask them from inside a conversation, against a running game,
without writing a scene or a test first.

The two halves are independent. The observer works in gdUnit4 suites with no
MCP at all; godot-mcp is a convenience for the interactive loop.

## Status: not installed by this repository

Nothing in this tree installs, pins, or vendors godot-mcp. It is a developer
tool you install on your own machine.

## Prerequisites

- **Node.js 20+.** Not currently a dependency of anything else here — the game,
  the tests and the deploy pipeline all run without it.
- **Godot 4.7 editor, running, with the project open.** The server has no
  headless mode; it talks to the editor over a WebSocket and to the running
  game over the debugger protocol.
- **One MCP client at a time.** A second client queues rather than taking over.

## Install

```sh
npx @satelliteoflove/godot-mcp --install-addon /path/to/unseeing/game
```

Then enable the plugin in the editor: **Project → Project Settings → Plugins →
godot-mcp → Enable**. That step is a GUI click; there is no scripted equivalent.

### The addon is gitignored, deliberately

`game/addons/godot_mcp/` is in `.gitignore`, and `test/repo_hygiene.sh` pins
that from both directions — nothing under the path may be tracked, and the
ignore rule must still cover it. Two reasons, both load-bearing:

1. `deploy.sh` ships the tree by `git archive` into a bare repo whose
   post-receive hook untars it. Anything committed under `game/addons/` reaches
   the droplet **and** the wasm export — a Node-backed debugging tool riding
   into the shipped game.
2. `ci/vendor-gdunit4.sh verify` fingerprints `game/addons/` believing gdUnit4
   is its only tenant. A second addon is drift it would have to be taught to
   forgive.

The cost of this choice: the version is not pinned in-tree, so two machines can
run different godot-mcp versions. Accepted — it is a debugging aid, not a
dependency of the build.

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
JSON.stringify(get_tree().root.get_node("UnseeingMain").observer.snapshot(now))
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
