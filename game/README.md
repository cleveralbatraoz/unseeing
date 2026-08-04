# Unseeing — Godot project

The Godot 4 project — the single source of truth. Every shipped platform is
an export of this one project.

## Architecture

Two layers: the Rust crate (`../rust/`) is the hidden engine, Godot is
the visible game.

- `scenes/main.tscn` — one root node carrying `main.gd`.
- `scenes/level_01.tscn` — the level, authored in the editor: `WaveWall`
  and `WaveProp` boxes, the `SoundFan`, the companion `WaveCat`, and a
  `SpawnPoint` marker under a `WaveLevel` root that derives the technical
  contracts (wall centerlines, hum room, spawn, demo tap, and the object
  ids that keep every seam drawable) from what the designer placed.
- `scripts/main.gd` — composition root: instances the level scene,
  injects the material and wave pool into it, wires the engine nodes
  together, owns the fullscreen hearing quad and the per-frame globals
  (clock, flicker).
- `scripts/pulses.gd` — thin shim over the Rust `WaveCore`: the 64-slot
  wave pool shared with both shaders.
- `scripts/flicker.gd` — the mood's envelope: a seeded, bounded dimming
  the hearing pass multiplies through.
- `scripts/demo_tap.gd` — the dev cane tap the level derives, for
  walking the world without a player.
- `../rust/src/` — the engine: pure wave / viewmodel / level-plan /
  object-id modules (cargo-tested) and the registered node classes the
  game places —
  `WaveLevel`/`WaveWall`/`WaveProp` (level authoring), `SoundFan`
  (designer knobs for the hum voice), `WaveCat` (the companion's gait,
  brain and paw voice), `UnseeingPlayer` (movement, mouse look, cane tap
  modes), `HeroBody` (viewmodel meshes, head-bob, footsteps).
- `shaders/data_pass.gdshader` — world rendered as data: reveal, flat
  object id, camera distance.
- `shaders/hearing_post.gdshader` — outlines + wave shells; the only pass
  the player ever sees.

Renderer is `gl_compatibility` — mandatory for the Web (wasm) export.

## Authoring levels

No programming needed — the level is an ordinary Godot scene:

1. Build the Rust library once so the editor knows the engine nodes:
   `cargo build --release` in `../rust/` (only needed after a fresh
   clone or an engine change — and never open scenes before this build,
   or the editor will strip the engine node types from them).
2. Open `game/project.godot` in Godot and double-click
   `scenes/level_01.tscn`.
3. Walls: duplicate any `WaveWall`, drag it where you want, stretch it
   with its **Length** property. Walls snap to right angles by
   themselves — the perception physics needs axis-aligned walls, and the
   engine enforces it quietly.
4. Furniture: duplicate a `WaveProp` and set its **Size**.
5. Sound sources: the `SoundFan` node has its voice in the Inspector —
   whoosh cadence, hum range/speed/gain, beam cone.
6. The hero wakes at the `SpawnPoint` marker — move it to move the start.
7. Press play. The `WaveLevel` root derives everything technical from what
   you placed — wall centerlines, the hum room, the demo tap, and the flat
   object id each box carries so its outline stays separate from whatever
   it touches; the test suite gates the rest on every commit.

Nothing about those object ids needs setting by hand, and it is worth
knowing why: the engine looks at which boxes actually MEET and gives
neighbours different ids, so boxes at opposite ends of the map share freely
and a level of any size needs only a handful. Push enough boxes into one
another that no arrangement separates them all and the level says so loudly
in the output — and still runs, with those few seams unlined.

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
| Wave/physics core as GDExtension Rust module | done (`rust/`: pure modules + WaveCore behind the Pulses shim) |
| Fan / player / hero body as Rust engine nodes | done (`rust/src/nodes/`: SoundFan, UnseeingPlayer, HeroBody replace their .gd scripts; demo movie frames byte-identical across the port) |
| Companion cat (gait, brain, paw voice) | done (`rust/src/cat_*.rs` + `WaveCat` in `scenes/level_01.tscn`) |
| One outline per object, every seam drawn | done (`rust/src/oid_palette.rs` colours the touch graph; the shipped scene is pinned by `tests/level_test.gd`) |
| gdUnit4 test framework migration | done (`tests/`, headless CLI in `ci/pipeline.sh`) |
| Vendored framework pinned + reproducible | done (`ci/gdunit4.lock`, `ci/vendor-gdunit4.sh`; self-updater off) |
| Editor-authored levels (the engine/content split) | done (`scenes/level_01.tscn` + WaveLevel-derived contracts; see Authoring levels) |
