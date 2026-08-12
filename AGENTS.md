# Unseeing agent instructions

## Project and non-negotiable design

Unseeing is a mystic/horror game about a blind hero. Act as a principal game
developer. The technical bar is ideal: prove decisions with code, tests, and
traces. Physics and sound-wave propagation must be exact.

- Render black and white, with thin outlines only: no textures, fills,
  materials, or visual noise. The world is revealed only by sound, touch, and
  wind waves.
- Draw one outline per object and every seam between objects. Anything that
  can touch another object needs an object id at least 0.08 clear of it. The
  id budget and graph colouring live in `rust/src/oid_palette.rs`; never cycle
  through a list of ids.
- Keep UI and UX simple and minimal.
- `game/` is the sole Godot 4.7 project and source of truth. Export that same
  project to web, macOS, and Windows; never make platform implementations.
- Everything must work on x86_64 and arm64. macOS is universal, Windows has
  x86_64 and arm64 exports, and Rust targets both desktop architectures plus
  wasm32.
- The approved stack is Godot, typed GDScript, GDExtension Rust, wasm, and the
  tooling documented here. Ask before introducing another technology.

## Documentation

Read the project wiki before implementation or research, starting with
Mechanics Overview. Its linked pages cover rendering, waves, sound sources,
levels and objects, and build/test/deploy. Code wins when it disagrees with
the wiki. Before declaring work complete, rewrite or create the relevant wiki
page so it describes current behaviour and names the file owning each quoted
constant. Research is not complete until recorded there.

Specs and plans are separate, tracked artifacts under
`docs/superpowers/specs/` and `docs/superpowers/plans/`. A spec freezes what
was decided and why; the wiki describes what ships now. Update both when both
are affected.

## Workflow authority

`AGENTS.md` owns project policy. The repository-pinned Superpowers plugin owns
generic procedure. Apply stricter compatible requirements from either. If
they genuinely conflict, stop and ask the user; neither silently overrides
the other. Do not use an unpinned or competing Superpowers installation.

Use the Superpowers workflow: brainstorming before creative work (design must
be approved before implementation), writing-plans, subagent-driven-development
or executing-plans, and finishing-a-development-branch. Always use
systematic-debugging for bugs or unexpected results, TDD for changes,
verification-before-completion before success claims, requesting-code-review
after every task and before merge, and using-git-worktrees at task start.

Plans must carry these global constraints into every implementer brief: the
perception laws and object-id clearance above, supported platforms, the two
code layers below, commit rules, and attribution ban. Commit steps state that
a commit is made but do not prescribe a literal commit message.

Autonomy ends at integration and deployment. Present the finish-branch choice;
never merge, push, or deploy without the user's choice. Deploy only after an
approved merge, from clean `main`, because `deploy.sh` pushes `main` and its
compiled cores must match that exact tree.

Write specs to `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md` and plans
to `docs/superpowers/plans/YYYY-MM-DD-<feature>.md`. Keep durable project
knowledge in tracked documentation, never host-specific memory.

## Isolation and parallel work

Every task uses its own worktree; never develop in the durable primary
checkout. Detect existing isolation first and do not nest worktrees. Use a
host's native isolation when available. Otherwise create
`.worktrees/<branch>` from the primary checkout after confirming that path is
ignored. Clean up with the same host/tool that created the worktree; manually
created `.worktrees/` entries may use `git worktree remove` after integration.

The only exception is an explicitly selected merge into `main`, performed in
the clean primary checkout. Stop if that checkout is dirty or on an unexpected
branch. Multiple sessions may be live; reviewers are read-only and must not
move HEAD in the checkout under review.

## Strict TDD and debugging

- For every behaviour change: write the test, observe the correct failure,
  add minimal code, observe the pass, then refactor. Delete production code
  written before its test. There are no exceptions for prototypes, generated
  code, or configuration.
- Every test names the break it catches. Do not use mirror assertions or
  constant-change detectors: hand-derive literals or use checked fixtures and
  test externally visible behaviour.
- Before finishing, mutation-check realistic constants, branches, side
  effects, and early returns; each mutation must fail a test.
- Use gdUnit4 under `game/tests/`, plain `cargo test` for engine-free Rust,
  browser tests under `test/`, and movie-maker frames for visual verification.
- Debug root causes, one hypothesis at a time, with a failing regression test
  before the fix. After three failed fixes, stop and question the architecture.
- Poll for conditions instead of arbitrary waits. Existing debt is limited to
  the Chrome startup sleep in `test/web_smoke.sh` and `SMOKE_WAIT` in
  `test/web_probe.py`; do not add more.

The game boots fullscreen at native resolution, except web. Command-line
window flags cannot override the project setting. For deterministic windowed
runs, create and always remove `game/override.cfg` containing a `[display]`
section with mode 0 and explicit viewport dimensions. It is ignored and must
never ship.

## Commits

- Make small, self-contained, green commits: one behaviour each, with its test.
- Use an evocative narrative subject matching repository history and a body
  explaining the precise what and why. Do not paste literal commit messages
  from plans.
- Repository identity is `Dmitrii Galchenko <dggrus@gmail.com>`.
- Work is authored for the user. Never add `Co-Authored-By`, `Generated with`,
  or any assistant attribution in commits, code, comments, docs, or PRs. Tool
  names and agent-facing process documentation are allowed; authorship credit
  is not.

## Source and asset policy

Never commit build output, exports, `.pck`, `.wasm`, `target/`, rendered
frames, or reports. The pre-commit hook rejects staged files over 5 MiB unless
the user deliberately sets `ALLOW_BIG=1`. Commit Godot `.import` and `.uid`
sidecars. Do not use Git LFS, git-annex, DVC, or external object storage.
Revisit only if one file exceeds about 50 MiB, the pack exceeds about 500 MiB,
or a binary reaches its fifth revision; prefer hashes or downsampled visual
baselines over golden frames.

Developer-agent tooling, including `.gitmodules` and `tools/superpowers`, must
never enter deployment archives or become a game/build/deploy dependency.

## Architecture and style

- Prefer small total functions, explicit inputs and outputs, dependency
  injection, independent components, and no hidden global state.
- Rust is the hidden engine: wave simulation, echo physics, kinematics,
  animation math, and perception internals. Put logic in pure tested modules;
  expose only registered nodes, typed signals, and designer-facing exports.
- Godot is the visible game: editor-authored scenes and thin, statically typed
  GDScript for triggers, sequences, and tuning. Author levels in the editor;
  derive technical contracts from scenes.
- `#![deny(unsafe_code)]` applies. The sole exception is the targeted
  `unsafe impl ExtensionLibrary` required by gdext; add no other exception.
- Native Rust uses the stable pin in `rust/rust-toolchain.toml`. Web alone
  uses the nightly pinned in `rust/build-wasm.sh` for `-Zbuild-std` and
  `-Zemscripten-wasm-eh`. Keep Emscripten aligned with Godot. A web export may
  contain exactly one Rust GDExtension.
- Run gdformat and gdlint for GDScript; cargo fmt, clippy with warnings denied,
  tests, and release build for Rust. Add analysis tools when useful without
  expanding the shipped stack.

## Dependencies and tooling

Godot addons are vendored, never submodules. `ci/gdunit4.lock` pins gdUnit4;
`ci/vendor-gdunit4.sh update <tag>` is the only update path and `verify` must
pass. Never hand-edit it or enable its updater. Godot itself is pinned in
`.godot-version`.

`tools/superpowers` is the repository's sole submodule and is developer-only.
Use `tools/setup-agents.sh` from the durable primary checkout to install its
local `superpowers-dev` marketplace for Claude Code or Codex App/CLI. Use
`tools/update-superpowers.sh <vX.Y.Z>` on a clean isolated branch to inspect a
pinned release update. Never track upstream branches or arbitrary commits.
After setup or upgrades, restart Claude Code or begin a new Codex session.
Codex IDE and other agents are not supported by this integration.

Use subagents and dispatching-parallel-agents for independent domains with a
clear scope, constraint, and named result. Review every task and merge using
Superpowers' reviewer rubric and a diff, with additional multi-agent design
and performance review for physics and wave work. Verify feedback against the
codebase before accepting it. A subagent report is never proof; inspect the
diff and rerun the evidence.

Prefer structured state over screenshots when debugging. `WaveObserver` and
`rust/src/observe/` expose pulse, eviction, object-id, crossing, and reflection
state. Use gdUnit4, the godot-mcp loop documented in
`docs/superpowers/mcp/godot-mcp-loop.md`, or a dump scene. Screenshots are a
last resort and reveal a missing observability surface. `explain_ray` reports
Rust's belief and cannot prove GLSL agrees. Keep `game/addons/godot_mcp/`
ignored and untracked because it must not reach the export.
