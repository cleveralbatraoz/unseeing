# Unseeing

[![tests](https://github.com/cleveralbatraoz/unseeing/actions/workflows/test.yml/badge.svg)](https://github.com/cleveralbatraoz/unseeing/actions/workflows/test.yml)

A first-person echolocation game built in Godot 4. The hero is blind: the
world exists only as sound. Cane taps and footsteps send visible waves through
the dark; thin white outlines flare where a wave strikes geometry and fade
away; every surface answers back — echoes bloom from what the wavefront
touches.

![A cane tap revealing a rock-walled room and a wooden table in white outlines](docs/screenshot.png)

**Play:** https://206.223.241.165 — best in a Chromium browser.
(`?demo` makes the game tap by itself, if you just want to watch.)

## Controls

- `W A S D` — walk (physical key positions; works on any keyboard layout)
- mouse — look
- click — tap the cane: strikes what you aim at within reach, otherwise taps
  whatever the cane tip is resting on; sweeping empty air answers with nothing
- `Esc` — release the mouse

## How it works

Everything visible is sound. A **data pass** renders the world not as an
image but as data — how recently a wave swept each point, a per-vertex face
label, and camera distance — packed into color channels. At level derivation,
Rust joins same-facing coplanar overlaps into **superfaces** and bakes one
bit-identical class label into those faces' `CUSTOM0` vertices; faces that
must draw a crease receive labels at least `MIN_SEP` (0.08) apart. Creatures
use fixed numeric role labels; sound sources keep semantic limb roles while
the level derives separated numeric labels for each placed instance. The pure
`rust/src/render/paint_plan.rs` owns that complete, atomic decision;
`rust/src/render/paint.rs` is only the `ArrayMesh` read/write boundary. A
fullscreen **hearing pass** turns that data into everything you see: thin
white outlines, and only where waves have swept. Two kinds of line make them
— silhouettes, where packed distance steps, and creases, where the face label
changes. Flush overlaps therefore melt without a depth-fighting seam, while
bends, steps, and separate touching solids still draw. Expanding wave shells
are ray-traced in air and occluded by the world, so obstacles carve visible
bites out of the rings. **Echo reflections** sample the world with physics ray
fans from every sound — each struck surface point becomes a secondary emitter
firing exactly when the wavefront arrives, and anything in acoustic shadow
stays silent. No texture assets exist — the world is nothing but thin white
lines on black.

See `game/README.md` for the architecture and porting status.

## Platforms

One Godot project, exported everywhere — no per-platform code:

- **Web** — continuously deployed to https://206.223.241.165 (wasm).
- **macOS** — universal binary (x86_64 + arm64): `godot --headless --path
  game --export-release macOS build/macos/unseeing.zip`
- **Windows** — twin exports, `"Windows x86_64"` and `"Windows arm64"`
  presets. The game never relies on a particular architecture.

## Setup

Install four things, then run one command.

| Tool | Version | Why |
| --- | --- | --- |
| **Godot** | exactly `.godot-version` (`4.7.1.stable`) | the engine. Standard or .NET build, installed any way you like — the bootstrap searches `godot`, `godot4`, `godot-4`, Homebrew, Scoop, WinGet, `~/bin`, `/Applications`, and the official archive's own filename on `PATH`. |
| **rustup** | any | the framework is a Rust GDExtension. `rust/rust-toolchain.toml` pins the compiler; rustup installs it. The bootstrap installs rustup itself if it is missing. |
| **A C linker** | any | Rust needs one. `build-essential` on Linux, `xcode-select --install` on macOS, Visual Studio 2022 Build Tools with **Desktop development with C++** on Windows. |
| **gdtoolkit** | `4.*` | `pipx install "gdtoolkit==4.*"` — `gdformat` and `gdlint` gate every commit and every CI run. |

Then:

```sh
tools/bootstrap.sh                 # macOS and Linux
git config core.hooksPath .githooks
```

```powershell
.\tools\bootstrap.cmd              # Windows
git config core.hooksPath .githooks
```

The bootstrap ends with `bootstrap: OK` only after every engine class has
registered — the expected count lives in `ci/engine_class_count`, and a
mismatch names that file. If it cannot find your editor, pass it:
`GODOT=/path/to/godot tools/bootstrap.sh`, or
`.\tools\bootstrap.cmd -Godot C:\path\to\Godot_console.exe`.

**Play the game:** `tools/run_game.sh` (`.\tools\run_game.cmd` on Windows) —
builds the engine and launches the world. Add `--windowed` for a window instead
of full screen, `--skip-build` to play what is already built.

**Author levels:** open `game/project.godot` in Godot and press play. Renderer
is `gl_compatibility`, required for the Web export. The editor tour, the
correct-worktree check, and the code-free level workflow are in
[Opening and running Unseeing in Godot](docs/opening-in-godot.md).

**Check everything:** `ci/pipeline.sh` — the same script CI and the droplet run.
`SKIP_EXPORT=1` for checks only.

Needed only for particular jobs, not for setup: Node 20+ (`tools/setup-mcp.sh`,
the Godot editor bridge), Chrome or Chromium plus Python 3 (the web smoke test),
`cargo-zigbuild` and emsdk (`deploy.sh`).

Claude Code and Codex App/CLI contributors should follow the pinned,
repository-local setup and upgrade guide in [docs/agent-workflow.md](docs/agent-workflow.md).
The agent plugin is developer tooling and is excluded from game and deployment
artifacts.

- `rust/` — the wave/physics core as a GDExtension (godot-rust). Pure laws
  live in engine-free modules under plain `cargo test`; registered adapters
  under `rust/src/nodes/` and the narrow `ffi` boundary translate between
  those laws and Godot types. `rust/build-wasm.sh` builds the single-threaded
  wasm for the web export (toolchain pins and their reasons are documented in
  the script).
- `ci/pipeline.sh` — the full gauntlet: vendored-framework integrity check,
  cargo fmt/clippy/test + release build, format + lint gate, headless boot
  check, unit tests (`game/tests/`), the wasm core build, clean Web export,
  build stamping, precompression of every shipped artifact, and a browser
  smoke test that boots the exported wasm in headless Chrome and asserts it
  renders. The same POSIX script runs locally, on the droplet, and in cloud
  CI — and when it runs on prebuilt cores it refuses any whose recorded
  commit is not the one being deployed.
- `game/addons/gdUnit4/` — the test framework. Godot resolves addons as
  project resources, so it lives in the tree rather than as a submodule
  (upstream ships no `.uid` sidecars; Godot mints 244 of them on import,
  which inside a submodule would be permanently dirty and uncommittable).
  The copy is pinned by `ci/gdunit4.lock` and reproduced byte-for-byte by
  `ci/vendor-gdunit4.sh update <tag>` — the only sanctioned way to change
  it. The pipeline verifies its fingerprint on every run, and its in-editor
  self-updater is switched off so a version bump is always a reviewed commit.
- `deploy.sh` — refuses anything but a clean `main` first (the cores below
  are compiled from the working tree while the push ships the branch, so
  those have to be the same code), proves cargo-zigbuild and Zig separately,
  runs local checks, then cross-builds the linux and wasm cores the 1.8 GB
  droplet cannot compile itself and seeds them — with the commit they were
  built from — over ssh. The droplet's post-receive hook runs the full
  archive-mode pipeline and deploys only on green. If `production/main`
  already names the commit after an earlier refused hook, the deploy sends a
  one-shot retry ref through that same pipeline. Only a matching live build
  stamp permits the final `git push origin`.
- `infra/` — versioned copies of the droplet's hook and nginx config.

## License

MIT — see [LICENSE](LICENSE). The name "Unseeing" and any future art/audio
assets are not covered by the code license.
