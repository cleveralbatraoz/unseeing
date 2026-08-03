# Unseeing — Godot project

The Godot 4 project — the single source of truth. Every shipped platform is
an export of this one project.

## Architecture

Two layers: the Rust crate (`../rust/`) is the hidden engine, Godot is
the visible game.

- `scenes/main.tscn` — one root node carrying `main.gd`.
- `scenes/level_01.tscn` — the level, authored in the editor: `WaveWall`
  and `WaveProp` boxes, the `SoundFan`, and a `SpawnPoint` marker under a
  `WaveLevel` root that derives the technical contracts (wall
  centerlines, hum room, spawn, demo tap) from what the designer placed.
- `scripts/main.gd` — composition root: instances the level scene,
  injects the material and wave pool into it, wires the engine nodes
  together, owns the fullscreen hearing quad and the per-frame globals
  (clock, flicker).
- `scripts/pulses.gd` — thin shim over the Rust `WaveCore`: the 64-slot
  wave pool shared with both shaders.
- `../rust/src/` — the engine: pure wave/viewmodel/level-plan modules
  (cargo-tested) and the registered node classes the game places —
  `WaveLevel`/`WaveWall`/`WaveProp` (level authoring), `SoundFan`
  (designer knobs for the hum voice), `UnseeingPlayer` (movement, mouse
  look, cane tap modes), `HeroBody` (viewmodel meshes, head-bob,
  footsteps).
- `shaders/data_pass.gdshader` — world rendered as data (reveal/normals).
- `shaders/hearing_post.gdshader` — outlines + wave shells; the only pass
  the player ever sees.

Renderer is `gl_compatibility` — mandatory for the Web (wasm) export.

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
| gdUnit4 test framework migration | done (`tests/`, headless CLI in `ci/pipeline.sh`) |
