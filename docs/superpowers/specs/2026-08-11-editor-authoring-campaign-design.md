# Editor authoring — the designer works in Godot, the code works in Rust

*Design frozen 2026-08-11. What we decided to build and why. How the shipped
thing works belongs on the wiki, not here.*

Grounded in the wiki page **Research — Editor Authoring** (researched
2026-08-10 at `b01632e`) and a six-way re-verification of every blocker
against `main` @ `3f376cf` on 2026-08-11, after the 15-issue campaign landed.
Where this spec states a fact about the code, it was re-derived at `3f376cf`,
not quoted from the research.

## The goal

A game designer works on game levels, never on code. They pick a node or a
prefab in the Godot editor, place it, and **it just works**. They drag a
sound source to another room — it still works. Everything they do is saved
as text in the `.tscn` (coordinates and knobs, diffable, committable). All
physics, wave wiring, object-id colouring, floor derivation, budgets — the
machinery — lives in the Rust framework and derives itself from what the
scene says, automatically, every time.

Three requirements sharpen that goal, each a decision made in this
brainstorm:

1. **Free placement is the law.** A designer may put any two things
   anywhere, including two sound sources touching. The engine must make the
   result correct — touching sources draw their seam — rather than forbid
   the arrangement. (Rejected: erroring or warning on touching sources.
   Consequence: sources need genuinely distinct object ids; this is palette
   surgery and gets the full physics review.)
2. **No intermediate state for the designer.** The `.tscn` is the single
   authored artifact. Nothing the designer does requires regenerating a
   golden, running a script, or maintaining a second file. Dev/CI-only
   artifacts may exist, but no gate reddens because a designer legitimately
   edited content. (Rejected: a designer-regenerated census golden.)
3. **The designer-facing razor, applied to the code itself.** Godot holds
   only what the designer needs to do their job: scenes, placements,
   exported knobs, thin trigger/tuning scripts if one is ever wired. Every
   line not required for that job lives in Rust. This tightens the existing
   two-layer doctrine's criterion from "game-facing" to "designer-facing",
   and this campaign enforces it by structure: the imperative GDScript
   composition root is migrated to Rust in full (sub-project 4).

The designer is a technical friend: a one-time bootstrap (rustup, pinned
toolchain, one build command, self-verifying) is acceptable plumbing. It is
plumbing, not a teaching document — after it runs once, the editor just
works. CI-published binaries are out of scope for this campaign.

## What is already true (verified at `3f376cf`)

The hard half shipped before this campaign started:

- The four solids and `WaveLevel` are `tool` classes; every knob reshapes
  mesh + collider + origin lift live in the editor.
- Object ids are graph-coloured automatically (`rust/src/oid_palette.rs`);
  a designer never meets the budget.
- `WaveLevel::collect` recurses, so grouping folders and instanced
  sub-scenes already compose correctly; editor save round-trips losslessly.
- The five authoring-correctness fixes are merged: global-space wall snap
  (room prefabs safe to nest), sign folding, world-space standing, scale
  folded into knobs, idempotent Ctrl+D-safe builders.
- The gate hears the engine: boot errors from `WaveLevel`/sources fail CI;
  starved ids, missing/duplicate spawn, sunken/unfloored/stray solids, wall
  and pack-range budgets are all computed as pure `level_plan` functions
  with tests.

## What still blocks the goal (verified at `3f376cf`)

1. **Binary delivery (#38):** every `.gdextension` key points into
   gitignored `rust/target/`; a fresh clone is `MissingNode` ×129. No
   bootstrap exists.
2. **Blind placement (#30–#34):** `SoundFan`/`SoundRadio`/`WaveCat` are
   non-`tool` placeholders drawing nothing; `derive()` early-returns under
   `is_editor_hint`, so every loud fault the engine can already compute is
   reported only at run time; zero configuration warnings, icons, shape-knob
   ranges, or in-editor docs (`register-docs` absent; CLAUDE.md claims it —
   #44).
3. **Census-pinned gate (#22):** ~29 assertions pin the shipped map's
   inventory; adding one crate reddens deploy by 3 tests.
4. **Hand arithmetic (#41/#42):** no prefab unit of "an object"
   (`level_01.tscn` is 129 flat siblings, zero `ext_resource`); walls are
   centre+length while designers think in endpoints; a doorway is four
   derived numbers recorded nowhere.
5. **Cannot run or ship it (#39):** the level is a parse-time `preload` in
   `main.gd:23`; F6 on a level scene has no camera or player.

Latent hazards that become designer-facing the moment placement is free:
fan and radio faces share oid 0.33 so touching sources melt (#16); the id
channel has no free band ≥ `MIN_SEP` from all 13 spent ids (#36); a solid
nested under a solid inflates the parent's oid box (#35); the pack-range
check measures the wall footprint, so a large-extents courtyard saturates
packed distance silently (#45).

## The decomposition — four sub-projects, one branch

All four land on the dedicated branch `worktree-editor-authoring-campaign`
(from `main` @ `3f376cf`), which lives until the whole campaign is done.
Each sub-project gets its own implementation plan, subagent-driven
execution, review, and wiki write-back. Order: **1 → 4 → 2 → 3.**

### Sub-project 1 — Place it and see it

The designer's first hour. Closes #30, #31, #32, #33, #34, #38 (as scoped),
#44. Detailed design below.

### Sub-project 4 — The Rust composition root (second in sequence)

A Rust `UnseeingGame` node absorbs `main.gd`: level instancing, player
construction, injection order, demo-tap scheduling, settings-menu wiring.
The settings overlay's *layout* stays a `.tscn`; its logic moves to Rust.
GDScript shrinks to designer-facing residue only, and the razor is recorded
in CLAUDE.md as law. Sequenced before 2 and 3 so the gate's law tests and
the level knob are built once against the final architecture. Injection
order is a documented trap (the reveal loop's bound gate, instance
uniforms); this sub-project gets the full multi-agent design review physics
work demands.

### Sub-project 2 — Any edit ships

The deploy gate keeps only **level-agnostic laws** — true of any level a
designer could author: every touching pair draws a seam, every solid stands
on floor inside the slab, spawn exists and is unique, budgets hold, walls
occlude what they draw. The ~29 shipped-census pins are retired; the
engine-regression half of what they caught (collection, derivation) moves
to code-built fixture levels, where the issue campaign already moved the
muffle and ladder laws. No golden gates content. Closes #22.

Plus the engine work free placement demands, with full physics review:

- **Source-vs-source seams (#16):** sources stop sharing oid 0.33. Either
  the fixed constants are re-planned or sources enter the graph colouring
  as colourable nodes (decided in this sub-project's plan, against the
  saturated channel — #36 constrains, the touch-graph doctrine permits).
  The new law lands with it: a fixture with two touching sources must draw
  their seam.
- **Nesting inflation (#35):** `mesh_world_box` stops unioning descendant
  solids into a parent's colouring box.
- **The courtyard blind spot (#45):** the pack-range budget measures the
  slab-inclusive extent, not the wall footprint.

### Sub-project 3 — The vocabulary

The "models" the designer reaches for. Closes #39, #41, #42.

- **Prefab library:** `game/scenes/props/*.tscn`, then
  `game/scenes/rooms/*.tscn` — a table, a doorway, a room as single
  draggable, rotatable things. Zero engine code (recursion already
  composes), but re-nesting churns oid tie-breaks map-wide, which is why
  this lands after sub-project 2's laws replaced the census.
- **Endpoint walls / doorways (#42):** walls authored by the numbers a
  designer thinks in (ends, openings), with the composite-emitting-children
  pattern that costs zero id budget.
- **Typed spawn:** a `WaveSpawn` class collected by type, loud on zero and
  on two — replacing the magic string.
- **Level switching (#39):** an exported `level_scene` knob on
  `UnseeingGame`; a second map becomes an editor act. A way to launch a
  chosen level from the editor session.

Out of scope for the campaign: gizmos and editor docks (the research's
Stage 4 — reach rings, wall end-handles), an in-viewport perception
preview (rejected: the editor shows blueprint mode; perception requires
pressing Play), CI-published binary artifacts, and Steam/desktop delivery
of the editor itself.

## Sub-project 1 — detailed design (approved)

**Editor-visible sources and cat** (`fan.rs`, `radio.rs`, `cat.rs`). All
three become `#[class(tool)]` with restructured `ready()`: limb geometry
(fan housing, radio box, cat body) builds always and idempotently (the
named-limb `clear_limbs` pattern the solids already use), while wave wiring
stays behind the injection gate exactly as now — in the editor nothing
injects, so nothing ticks, emits, or registers. The cat disables physics
processing under `is_editor_hint` so it cannot wander the viewport and
Ctrl+S its drift into the scene (measured hazard). What the designer sees
is **blueprint mode**: the same geometry the game outlines, default-lit;
the perception image stays runtime-only, and the docs say so.

**Live law-checking while dragging** (`level.rs`, `level_plan.rs`).
`WaveLevel` stops early-returning under `is_editor_hint` and runs
`derive()` in planning-only mode — no pulse pool, no injection, pure
analysis. It re-derives on child add/remove and on solid transform changes
(the notify-transform hook exists), debounced to once per frame. Every
fault the campaign already made loud — starved ids, missing/duplicate
spawn, sunken/unfloored/stray solids, wall and pack-range budgets — is
surfaced through `get_configuration_warnings()` on the node that caused
it; level-wide faults sit on the `WaveLevel` node. The designer sees the
yellow triangle in the Scene dock while the arrangement is still wrong,
not a console line after pressing Play.

**Knob polish and identity.** `#[export(range)]` with metre suffixes on
every shape knob (wall length, prop/wedge size, column radius/height,
level extents, cat roam/seed). An `[icons]` block in
`game/unseeing.gdextension` with committed SVG icons (text files; sidecars
committed per policy). `register-docs` behind a non-default cargo feature —
in-editor docs for whoever builds with the feature, zero bytes in shipped
wasm — and CLAUDE.md's claim corrected to match (#44).

**Bootstrap plumbing** (`tools/bootstrap.sh`). One command: rustup +
pinned stable toolchain + `cargo build --release` + a headless
class-census verification (the `engine_binary_test.gd` roster) so a
half-working install cannot masquerade as done. Not a teaching document.

**Testing.** The proven headless-editor instrument
(`godot --headless -e -s` reaches `is_editor_hint() == true`) gates it in
CI: sources render limbs in editor mode; derive-in-editor reports planted
faults; warnings name the right node; the cat holds still. Pure law logic
stays cargo-tested; TDD throughout; the mutation check applies.

## Success criteria

The campaign is done when, on this branch merged to main:

1. A fresh clone plus one bootstrap command opens in the Godot editor with
   every class visible — solids, sources, cat — and placing or dragging
   any of them renders its blueprint geometry immediately.
2. An arrangement that breaks a law shows a configuration warning on the
   offending node while editing; fixing the arrangement clears it.
3. Adding, moving, or deleting content in a level — including two sound
   sources pushed together — passes the full pipeline with no test edits,
   and the touching sources draw their seam in game.
4. A table, a room, and a doorway are single draggable prefabs; a wall is
   authored by its endpoints; the spawn is a typed node.
5. A second level is created, selected, and played without touching a
   line of code.
6. `main.gd` is gone; GDScript in the repo is designer-facing only; the
   razor is stated in CLAUDE.md.
7. The wiki describes all of it, and the stale claims in
   *Research — Editor Authoring* are marked resolved.

## Errata

Recorded after sub-project 4 landed, so a later reader does not have to
re-derive either fact from scratch.

1. **The settings sentence was written on a false premise.** Sub-project
   4's description above says "The settings overlay's *layout* stays a
   `.tscn`; its logic moves to Rust." Re-verified at `c0ecba9`: no settings
   `.gd` or `.tscn` ever existed on this branch or on `main` — logic AND
   layout were already fully Rust (`rust/src/nodes/settings.rs`, 643
   lines, `SettingsMenu`). There was nothing to extract. Resolved by
   decision, not by code: SP4's plan
   (`docs/superpowers/plans/2026-08-12-editor-authoring-sp4-rust-root.md`)
   locked this as decision 1 of its Global Constraints, and the entire
   settings-related work in SP4 is constructing `SettingsMenu` last inside
   `UnseeingGame::ready` (`rust/src/nodes/game.rs:302-304`), preserving the
   Escape-priority ordering the overlay always depended on.
2. **The sub-project ordering note is fine as written** — "1 → 4 → 2 → 3"
   held for the whole campaign; no correction needed there. What SP4 did
   surface, and is worth recording so it is not re-derived: Task 6's plan
   predicted the composition-root migration would let
   `UnseeingGame::ready` drop its call to `UnseeingPlayer::ensure_actions()`,
   reasoning that the player's own `ready()` already re-registers its
   input actions. That prediction was wrong. Both calls stand
   (`rust/src/nodes/game.rs:167` and `rust/src/nodes/player.rs:159`,
   inside `UnseeingPlayer::ready`) — the redundancy is a locked decision
   (decision 5, same Global Constraints file), not leftover duplication: a
   bare `UnseeingPlayer` dropped into a test scene with no `UnseeingGame`
   ancestor has only its own `ready()` to register its actions, so the
   second call is load-bearing for every such scene.
3. **The original bootstrap stopped one platform short.** The detailed design
   named only `tools/bootstrap.sh`; its implementation deliberately stopped on
   Windows and printed a manual per-target Cargo command. The user's
   2026-08-13 portability requirement supersedes that limit without changing
   the one-command success criterion: macOS/Linux keep the POSIX entry point,
   while Windows gains `tools\bootstrap.cmd` backed by PowerShell. The Windows
   path selects the DLL target from the Godot editor architecture, builds the
   same release `editor-docs` feature, imports, and requires the same 19-class
   census. The complete current design is frozen separately in
   `docs/superpowers/specs/2026-08-13-cross-platform-bootstrap-design.md`.
