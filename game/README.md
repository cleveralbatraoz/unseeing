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
  contracts (wall centerlines and the occluder table sound is muffled
  through, spawn, demo tap, and the object ids that keep every seam
  drawable) from what the designer placed.
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

Placing walls, furniture and sound sources is ordinary scene editing — no
programming. Getting the editor to recognise those engine node types in the
first place does take one terminal step, though: nothing ships as a
downloadable binary yet (that gap is issue #38), so building the Rust
library yourself is the only way in today.

1. Build the engine once so the editor knows the engine nodes: run
   `../tools/bootstrap.sh` (needed after a fresh clone or an engine
   change). It installs `rustup` if you don't already have one — the
   toolchain pinned in `../rust/rust-toolchain.toml` is picked up
   automatically — builds the Rust library, lets Godot import it, and
   checks that every engine class actually registered, ending in
   `bootstrap: OK`. macOS and Linux only: on Windows the script exits
   with the per-triple `cargo build --release --features editor-docs
   --target x86_64-pc-windows-msvc` command to run by hand instead. Skip
   this and the scene still opens, but every `WaveWall`, `WaveProp`,
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
   node means the level found a fault with it — hover the triangle to
   read why.
2. Open `game/project.godot` in Godot and double-click
   `scenes/level_01.tscn`.
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
4. Furniture: duplicate a `WaveProp` and set its **Size**.
5. Sound sources: `SoundFan` and `SoundRadio` both have their voice in
   the Inspector — **Volume** (0 to 1, how loud and how far the waves
   reach), **Cadence** (seconds between waves) and **Wave Speed** (how
   fast a wavefront travels, in m/s) on both. The fan alone also has
   **Beam Cos**, the cosine of its sweeping wash's half-angle (about 32°
   at the shipped default) — the radio has no such knob, because it
   radiates evenly in every direction instead of aiming one.
6. The hero wakes at the `SpawnPoint` marker — move it to move the start.
7. Press play. The `WaveLevel` root derives everything technical from what
   you placed — wall centerlines and the occluder table every source's
   silhouette is muffled through, the demo tap, and the flat object id
   each box carries so its outline stays separate from whatever it
   touches; the test suite gates the rest on every commit.

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
