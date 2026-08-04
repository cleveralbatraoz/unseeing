# CLAUDE.md — Unseeing

## What this game is

Unseeing is a mystic/horror game about a **blind hero**. The whole point is to
visualize how a blind person *feels* the world — and the hero is blind, so the
visualization is the central design dilemma: we render a world its protagonist
cannot see. A blind person can't see, but they feel *more* than a sighted one.

Perception laws (non-negotiable):
- Black & white, **thin outlines only**. No textures, no fills, no materials,
  no bloat, no modern-game visual noise. Contours and vibe.
- The world is revealed by **waves** (sound, touch, wind). If nothing emits,
  nothing is visible.
- **One outline per object, and every seam between two objects draws.** The
  hearing pass draws two kinds of line: silhouettes, from a Laplacian of
  packed distance, and creases, from differences in a flat object id. Where
  two things interpenetrate there is no depth step, so the id difference is
  the *only* thing that can draw their seam — two touching objects sharing
  an id have no line between them and melt into one shape. So anything new
  that can touch something else needs an id at least 0.08 clear of it. The
  id budget and the graph colouring that hands them out live in
  `rust/src/oid_palette.rs`; never assign ids by cycling a list.
- UI/UX is simple and minimalistic.
- Inspiration: modern codebases and games in the blind-protagonist /
  echolocation genre — *Perception*, *Dark Echo*, *Stifled*.

## Platforms and stack

**One source of truth: `game/`, the Godot 4.7 project.** Supported platforms
are **Windows, macOS, and web** — all produced by *exporting* that one
project. Never write a separate implementation per platform.

- **Web** ships continuously — the wasm export, live at
  https://206.223.241.165, deployed through the test-gated `deploy.sh`
  pipeline: headless test runner → strict web export → browser smoke gate
  (`test/`, headless-Chrome DevTools probes) → atomic deploy → HTTPS
  byte-verify.
- **Windows and macOS** are exported on demand (when the user asks), not on
  every push.
- **Architecture independence: the game must work on both x86_64 and
  arm64.** Never rely on a particular architecture — no arch-specific code
  paths, intrinsics, or assumptions. macOS ships a universal binary; Windows
  ships both x86_64 and arm64 exports; the Rust core must build for
  x86_64 + aarch64 on every desktop platform, plus wasm32 for web.

Keep the technology stack deliberately small. Approved: Godot, typed
GDScript, GDExtension Rust (the wave/physics core), wasm, and the tooling
listed below. Before introducing any other technology, ask the user.

## Your role

Act as a **principal game developer engineer**. Follow modern industry
conventions and technologies. Don't hesitate to write complex, smart code —
apply difficult concepts when the problem calls for them. Don't overload the
code, but don't dumb it down either. Avoid global state.

The technical bar: **ideal**. Every decision must be proved, not assumed.
Physics and sound-wave propagation are the main technical challenge here and
must work perfectly.

## Workflow: research → questions → full autonomy

1. **Research first.** Before planning or asking anything, investigate the
   actual code. Never suppose — acknowledge by running code, tests, and
   tracing. Most problems are already solved by other projects: search GitHub/
   GitLab/the internet for existing solutions, and research the physics/math/
   mechanics itself when needed. Copy whatever works — license risk is
   accepted on this project.
2. **Ask every question upfront.** Surface all open questions *before*
   implementation starts, and keep asking until none remain.
3. **Then full autonomy.** Once questions are answered, proceed without
   further confirmation: install anything, run anything, use the server,
   spawn agents, consume whatever resources the task needs — including
   deploys.
4. **Keep memory and this file current.** When something new and *crucial*
   is figured out — a decision, constraint, or gotcha that changes future
   work — record it in persistent memory (crucial facts only, not
   everything) and update CLAUDE.md itself when the rules or stack evolve.

## Parallel sessions: one worktree each

**Every session/task works in its own git worktree.** Multiple sessions and
agents modify this repo in parallel; never work directly in the shared main
checkout. At the start of each session/task, create or enter a dedicated
worktree (`git worktree add` or the harness's worktree isolation), do all
work there, and land it back as the usual small green commits. Remove the
worktree when the task is done.

## Strict TDD

- Every behavior change starts with a test: **write the test → observe
  current behavior → change the code → test passes → done.**
- Trace every problem to its root. Never apply a fix naively; never touch
  code whose behavior you don't clearly understand.
- Test *everything* — features, physics, edge cases. Tests pin behavior down
  so anything can be changed fearlessly.
- Godot: **gdUnit4** is the test framework — suites in `game/tests/`, run
  headless from `ci/pipeline.sh` (the old custom runner is gone; that
  migration is done). Browser-level behavior: the headless-Chrome smoke
  suite in `test/`. Visual verification: movie-maker frame rendering
  (`godot --write-movie`, run under `caffeinate -dis` on macOS).

### Display defaults, and how to run windowed anyway

The game **boots full screen at the monitor's own resolution**
(`display/window/size/mode=3`), with the web exempted by feature tag
(`mode.web=0` — a browser grants the Fullscreen API only inside a user
gesture, and Godot's web display server swallows the rejected promise, so
a boot request fails *silently*). No content scale is set, so the viewport
IS the window and "native resolution" needs no machinery. Escape opens the
settings overlay (`SettingsMenu`), which freezes the world, frees the
mouse, and can toggle full screen or pick a resolution. Nothing is
persisted — every launch starts from these defaults, by design.

**`--windowed`, `-w` and `--resolution` cannot override this.** Measured,
and confirmed in `main/main.cpp`: the flags are parsed into `window_mode`,
then that variable is overwritten wholesale from
`display/window/size/mode` a thousand lines later — and the flags are
consumed on the way through, so `OS.get_cmdline_args()` never sees them
and no script can compensate.

To run windowed — rendered probes, `--write-movie`, any run whose frame
size must be identical on every machine — write a **`game/override.cfg`**,
the engine's documented escape hatch (merged over `project.godot` before
the window is created):

```
[display]

window/size/mode=0
window/size/viewport_width=1280
window/size/viewport_height=720
```

`tools/probe_visibility.sh` does exactly this and removes the file however
it exits. `game/override.cfg` is gitignored and `test/repo_hygiene.sh`
pins that: committed, it would silently un-fullscreen the shipped game.

## Commits

- Split every job into small, self-contained commits — one behavior per
  commit. Anyone (human or agent) reading `git log` must fully understand
  what changed and why.
- Each commit is **green**: the new test and the code that makes it pass land
  together.
- Style: narrative, evocative subject line (matching the existing history,
  e.g. "The fan blows a real wind: a sweeping cone of waves, walled into its
  room"), with a body carrying the precise technical what/why.
- **All work is authored on behalf of the user, never the assistant.** No
  Co-Authored-By or "Generated with" trailers; no mention of Claude, AI, or
  any assistant anywhere in the repository — commits, code, comments, docs,
  or PRs. Repo-local git identity: `Dmitrii Galchenko <dggrus@gmail.com>`.

## Binary assets: keep them out, and know when that stops being free

**The repo is source-only, and the perception laws are what make that
possible.** Black and white, thin outlines only, no textures or materials,
geometry built in Rust as `ImmediateMesh`, sound synthesised rather than
sampled — the art direction *is* an asset budget of zero. As of the
2026-08-04 audit the entire tracked binary payload is 10 PNGs / 700 KB
(one README screenshot plus vendored gdUnit4 icons), every one at exactly
one revision, and none of it reaches a shipped Windows, macOS, or web
artifact.

- **Never commit build output.** Exports, `.pck`, `.wasm`, `target/`,
  `--write-movie` frames, reports. `.githooks/pre-commit` rejects any staged
  file over 5 MiB (`ALLOW_BIG=1` to override deliberately);
  `test/repo_hygiene.sh` holds the standing invariants and runs first in
  `ci/pipeline.sh`.
- **Do commit** `.import` and `.uid` sidecars — Godot's docs require it, and
  the project tracks one per script with zero orphans.
- **git-lfs is rejected, not deferred.** The cost of a binary in git is
  size × revisions, and this repo's churn is 1. LFS would break the deploy
  (`deploy.sh` pushes to a *bare* repo whose post-receive hook does
  `git archive | tar -x`; with no filter configured it would ship 130-byte
  pointer files, and the hook returns tar's status, so it would fail
  silently), break the README image on GitHub raw, add a required dependency
  to a droplet that cannot run `sudo` unattended, and cost a second history
  rewrite to adopt or leave. Same verdict for git-annex, DVC, and external
  object storage.
- **Reopen the question only on a real trigger**: a single file over ~50 MiB,
  `size-pack` over ~500 MB, or — the one that will actually happen first —
  **any binary reaching its 5th revision**. The likely path there is
  committing golden frames for visual regression; store perceptual hashes or
  downsampled baselines instead of full frames.

## Code style

- **Small, total functions**: every function handles any input it can
  receive; think in functional-programming terms — clear inputs, clear
  outputs, no hidden state.
- **Dependency injection / independent components**: split code into pieces
  that don't rely on unclear implicit guarantees of each other.
- **The two layers (the engine/content split — 2026-08-02 decision).**
  The game is built so non-technical collaborators can create and modify
  content in the Godot editor without ever meeting the machinery:
  - **Rust (`rust/`, godot-rust / `gdext`) = the engine, hidden**: wave
    simulation, echo physics, player kinematics, viewmodel animation math,
    perception/graphics internals. Exposed to Godot only as registered
    node classes with designer-meaningful `#[export]` knobs, typed
    signals, and in-editor docs (`register-docs`). All logic lives here,
    pure modules cargo-tested; native on desktop, wasm on web — one
    source of truth on every platform.
  - **Godot = the game, visible**: editor-authored `.tscn` scenes (levels,
    placements, sound sources) and thin statically-typed GDScript for
    game-facing scripting only — triggers, sequences, tuning — written
    against the Rust nodes' API. Levels are authored in the editor, never
    in code; technical contracts (hum rooms, level data) are derived from
    the scene by the engine layer.
  - Rust won for safety and testing culture — it directly serves the
    total-functions and fearless-refactoring doctrine. (C# was rejected:
    Godot 4.x .NET cannot export to web. C++ was chosen briefly, then
    replaced.) The wasm export allows exactly ONE Rust extension: every
    native system joins the single crate.
- **No unsafe Rust.** The crate is `#![deny(unsafe_code)]`; the single
  permitted exception is the `unsafe impl ExtensionLibrary` entry point
  that gdext's API mandates (a targeted `#[allow]` in `ffi.rs`). Never
  add another exception — if a problem seems to need `unsafe`, redesign
  or ask the user.
- **Rust web-export constraints** (as of 2026, gdext wasm rides the
  bleeding edge). Two toolchains, pinned in two places, and the split is
  deliberate:
  - `rust/rust-toolchain.toml` pins the **stable** channel every native
    build, test and clippy run uses, so laptop, droplet and CI compile
    with the identical compiler.
  - `rust/build-wasm.sh` pins the **nightly** used for wasm *only* —
    gdext's web build needs `-Zbuild-std` and `-Zemscripten-wasm-eh`,
    both nightly-only. Never promote that nightly into
    `rust-toolchain.toml`: it would put desktop and CI on a moving
    compiler and forfeit the reproducibility the stable pin exists for.

  Also pin the Emscripten version to match the Godot build, keep link
  flags in sync with Godot's web export settings, and remember only ONE
  Rust GDExtension can live in a wasm export. Verify the web build in CI
  on every core change — toolchain churn is the known risk we accepted.

## Tooling

Formatters and analyzers are mandatory, run before every commit:
- **GDScript**: `gdformat` + `gdlint` (godot-gdscript-toolkit).
- **Rust (GDExtension)**: `cargo fmt` + `cargo clippy` (warnings are
  errors) + `cargo test`; reach for Miri or `cargo-fuzz` on the trickiest
  wave-math when it earns its keep.
- Adopt further instruments (fuzzers, static analysis, profilers) whenever
  they earn their keep — but they must not grow the shipped stack.

### Vendored dependencies

Godot resolves addons as project resources, so a third-party framework has to
live in the tree (`game/addons/`) — **never a submodule**: upstream ships no
`.uid` sidecars, Godot mints hundreds of them on import, and inside a
submodule they are permanently dirty and uncommittable. The copy is pinned
instead, and is never hand-edited:

- `ci/gdunit4.lock` records the upstream repo, tag, commit, and two
  fingerprints — upstream's shipped source and our resulting tree (content
  plus executable bits).
- `ci/vendor-gdunit4.sh update <tag>` is the only sanctioned way to change
  the addon; `check-upstream` re-verifies the pin against GitHub; the bare
  `verify` runs inside `ci/pipeline.sh` on every build, so drift cannot land
  silently — the pre-commit hook deliberately skips `game/addons/`.
- In-editor self-updaters stay **off** (`game/project.godot`). gdUnit4's
  would poll GitHub on project open and, on one click, delete the addon and
  unpack an unreviewed release over it. Version bumps are reviewed commits.

Godot itself is not vendored: `.godot-version` pins it, CI downloads that
exact release, and `ci/pipeline.sh` refuses a mismatched binary.

## Agents, reviews, tooling

- Use subagents freely — most tasks want a mix of tester / critic / software
  architect / programmer / product / plain-gamer perspectives. Run as many as
  needed.
- **Review is part of every task, proportional to its size**: features and
  physics/wave work get a full multi-agent design review + performance review
  (redesign if it fails); trivial fixes get a lightweight self-review.
  Algorithmic complexity and performance are always part of the review.
- Connect any MCP server that helps (e.g. tools that can actually *play* the
  game for UI/UX testing), or write one if it doesn't exist.
