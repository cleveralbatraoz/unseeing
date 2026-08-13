# Unseeing — Godot project

The Godot 4 project — the single source of truth. Every shipped platform is
an export of this one project.

## Architecture

Two layers: the Rust crate (`../rust/`) is the hidden engine, Godot is
the visible game.

- `scenes/main.tscn` — one root node, `UnseeingGame`: the composition
  root, straight from the registered Rust class
  (`../rust/src/nodes/game.rs`). It instances the level scene, injects
  the material and wave pool into it, wires the engine nodes together,
  and owns the fullscreen hearing quad and the per-frame globals (clock,
  flicker). `scripts/` carries nothing now — the wave-pool shim, the
  flicker envelope and the dev demo tap that used to live there are all
  Rust (`WaveCore` in `../rust/src/ffi.rs`, `../rust/src/flicker.rs`,
  `../rust/src/demo_tap.rs`); the only GDScript left in the project is
  test- and probe-facing, under `tests/`.
- `scenes/level_01.tscn` and `scenes/level_02.tscn` — levels authored in the
  editor from typed walls, runs, solid pieces, sources, creatures, spawns,
  and reusable plain-root prefabs. Their `WaveLevel` root derives wall
  centerlines, the occluder table sound is muffled through, spawn, demo tap,
  and the per-face superface labels from what the designer placed.
- `../rust/src/` — the engine: cargo-tested pure wave, viewmodel, level-plan,
  and `render/` face/superface/label laws, plus the registered node adapters
  the game places —
  `UnseeingGame` (the composition root), `WaveLevel`/`WaveWall`/`WaveProp`
  (level authoring), `SoundFan` (designer knobs for the hum voice),
  `WaveCat` (the companion's gait, brain and paw voice), `UnseeingPlayer`
  (movement, mouse look, cane tap modes), `HeroBody` (viewmodel meshes,
  head-bob, footsteps).
- `shaders/data_pass.gdshader` — world rendered as data: reveal, the
  per-vertex `CUSTOM0` face or role label, and camera distance.
- `shaders/hearing_post.gdshader` — outlines + wave shells; the only pass
  the player ever sees.

Renderer is `gl_compatibility` — mandatory for the Web (wasm) export.

## Authoring levels

Placing walls, furniture and sound sources is ordinary scene editing — no
programming. Getting the editor to recognise those engine node types in the
first place does take one terminal step, though: nothing ships as a
downloadable binary yet (that gap is issue #38), so building the Rust
library yourself is the only way in today.

First time opening the repository? Follow the complete
[Godot editor tutorial](../docs/opening-in-godot.md), including the
correct-worktree check and the full-game runner scene.

1. Build the engine once so the editor knows the engine nodes: from the
   repository root, run `tools/bootstrap.sh` on macOS/Linux or
   `tools\bootstrap.cmd` on Windows (needed after a fresh clone or an engine
   change). It installs `rustup` if you don't already have one — the toolchain
   pinned in `rust/rust-toolchain.toml` is picked up
   automatically — builds the Rust library, lets Godot import it, and
   checks that every engine class actually registered, ending in
   `bootstrap: OK`. The Windows entry point reads the Godot executable's
   architecture and builds the matching x86_64 or ARM64 DLL automatically.
   Skip this and the scene still opens, but every `WaveWall`, `WaveProp`,
   `SoundFan` and so on loads as a `MissingNode` placeholder: no viewport
   geometry, and any method call on one fails (`Invalid call. Nonexistent
   function 'set_length (via call)' in base 'MissingNode'`). You cannot
   lay out a level that way, but you cannot damage one either — the
   placeholder keeps the original type and every property (a wall's
   `length` reads back off it intact). Reopening the scene is not how you
   get it back, though: a GDExtension that failed to load when the editor
   started is never retried while that editor process keeps running, so a
   build finished mid-session still shows `MissingNode` rows until you
   quit and relaunch Godot — only then does the scene come back exactly
   as it was. Once it has, sound sources and the cat show their blueprint
   shapes right there in the editor viewport, and a yellow triangle on a
   node means the engine found an authoring fault there — hover the triangle
   to read why.
2. Open `game/project.godot` in Godot and double-click a level scene. To
   make another level, create a `WaveLevel` scene; no code or GDScript belongs
   in it. Select `UnseeingGame` in a copy of `main.tscn` and assign that scene
   to its **Level Scene** resource picker. Leaving the picker empty is the
   exact level-01 fallback.
3. Walls: duplicate any `WaveWall` (Ctrl+D), drag it where you want,
   stretch it with its **Length** property. Walls must be axis-aligned:
   a wall's centerline is what the sight shaders count to decide what a
   source may light and the hero may hear, so its geometry is physics,
   not decoration — the engine holds that law itself. Rotate one and it
   snaps to the nearest quarter turn and drops any scale, warning in the
   Output panel when it does. That snap only runs when the wall
   (re-)enters the scene tree — placed, duplicated, or the scene
   reloaded — not while you're turning its gizmo in the viewport, so a
   wall you just rotated can sit at a free angle on screen for the rest
   of the session with no warning until the next reload.
4. Furniture: drag `scenes/props/chair.tscn` or `table.tscn` from the
   FileSystem dock, or compose another plain `Node3D` scene from typed props.
   A plain root is important: it groups content while Rust still recursively
   discovers every solid beneath it. The engine-generated preview limbs are
   deliberately ownerless and never become authored scene content.
5. Sound sources: `SoundFan` and `SoundRadio` both have their voice in
   the Inspector — **Volume** (0 to 1, how loud and how far the waves
   reach), **Cadence** (seconds between waves) and **Wave Speed** (how
   fast a wavefront travels, in m/s) on both. The fan alone also has
   **Beam Cos**, the cosine of its sweeping wash's half-angle (about 32°
   at the shipped default) — the radio has no such knob, because it
   radiates evenly in every direction instead of aiming one.
6. The hero wakes at the first `WaveSpawn` in scene order — move or rotate it
   to edit the start; delete duplicates when their warning appears.
7. For long walls and doorways, add `WaveRun`. **From** and **To** are X/Z
   coordinates in its parent's local space. Each **Openings** pair is
   `(absolute start coordinate on the selected axis, width)`; for example
   `(6.5, 3)` removes 6.5..9.5 m. The dominant axis is used (X on a tie), a
   diagonal warns and folds, and negative widths act as magnitudes. You can
   also drag `scenes/rooms/doorway_8m.tscn` or `room_16x16.tscn` and rotate the
   whole plain-root prefab normally.
8. A raw `WaveLevel` is content, not a playable composition: F6 from the level
   tab has no player, hearing pass, material injection or wave pool. For the
   useful authoring loop, duplicate `main.tscn`, assign the desired level to
   the copy's `UnseeingGame` **Level Scene** picker, and use **Run Current
   Scene** (F6) from that runner tab. This stays entirely in Godot and keeps the
   shipped default untouched. F5 always runs `main.tscn`; its empty picker is
   the exact level-01 fallback. The dedicated tutorial above gives every click.
9. The `WaveLevel` root derives everything technical from what
   you placed — wall centerlines and the occluder table every source's
   silhouette is muffled through, the demo tap, and how every solid face
   participates in the outline. Same-facing coplanar overlaps become one
   superface with a bit-identical label, so their depth fight disappears;
   bends, steps, and faces whose seam must draw receive separated labels.
   The test suite gates the rest on every commit.

Nothing about those labels needs setting by hand. The engine colours the
superface separation graph from five reusable world labels, while sources,
creatures, the floor, and the ceiling use fixed role labels. The palette is
not a limit on level size: distant face classes reuse it freely. If one local
arrangement demands more mutually separated labels than the palette can
provide, the affected solids and their `WaveLevel` show warnings and the game
still runs, with the named seams at risk of disappearing.

Optional tooling: `../tools/setup-mcp.sh` installs the godot-mcp editor
addon, which lets a connected MCP assistant drive this editor directly —
freeze the clock, step frames, inject input, and read the placement faults
above as structured data — instead of a screenshot and a guess. See
`../docs/superpowers/mcp/godot-mcp-loop.md` for the loop; it is a developer
convenience, never a build dependency.

## System status

| System | Status |
| --- | --- |
| Map, collision, movement, mouse look | done |
| Wave pool + data/hearing shaders | done |
| Cane tap modes (wall / floor / silent air) | done |
| Footstep ripples | done (from the animated shoes) |
| Hero body + cane viewmodel (bob/sway/strike) | done |
| Audio ticks (wall/floor/phantom) | TODO |
| Phantom sounds | TODO |
| Headless unit tests + browser smoke gate | done (`tests/`, `../test/`) |
| Web (wasm) export + droplet CI/CD deploy | done |
| Desktop exports (macOS universal, Windows x86_64 + arm64) | done |
| Wave/physics core as GDExtension Rust module | done (`rust/`: pure modules + `WaveCore`, driven directly by the `UnseeingGame` composition root; `Pulses` survives only as a `tests/` test shim) |
| Fan / player / hero body as Rust engine nodes | done (`rust/src/nodes/`: SoundFan, UnseeingPlayer, HeroBody replace their .gd scripts; demo movie frames byte-identical across the port) |
| Companion cat (gait, brain, paw voice) | done (`rust/src/cat_*.rs` + `WaveCat` in `scenes/level_01.tscn`) |
| One outline per object, intended overlaps melted, real bends and seams drawn | done (`../rust/src/render/` derives and paints the superface graph; mesh `CUSTOM0` read-backs are pinned by `tests/map_test.gd` and `tests/level_test.gd`) |
| gdUnit4 test framework migration | done (`tests/`, headless CLI in `ci/pipeline.sh`) |
| Vendored framework pinned + reproducible | done (`ci/gdunit4.lock`, `ci/vendor-gdunit4.sh`; self-updater off) |
| Editor-authored levels (the engine/content split) | done (two levels, reusable props/rooms, typed spawns/runs, and an Inspector level picker; see Authoring levels) |
