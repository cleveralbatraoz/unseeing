# Unseeing agent instructions

## Project and non-negotiable design

Unseeing is a mystic/horror game about a blind hero. Act as a principal game
developer. The technical bar is ideal: prove decisions with code, tests, and
traces. Physics and sound-wave propagation must be exact.

- Render black and white, with thin outlines only: no textures, fills,
  materials, or visual noise. The world is revealed only by sound, touch, and
  wind waves.
- Draw one silhouette per object. Where two solids overlap, outline them
  together: faces that are same-facing and coplanar merge into one superface
  and share one per-vertex label bit-for-bit, so the seam melts by
  construction. Bends and steps still draw — a room corner, a shelf edge, a
  crate's pierce line. Seams between *separate* touching solids draw too, and
  need labels at least `MIN_SEP` = 0.08 apart. Creatures and sound sources
  never merge with the world. Labels live in the sRGB-safe band
  `[0.15, 0.96]`, with one grandfathered exception (the radio chassis's
  `Role::Case` at 0.05); put new labels in the band. The merge law lives in
  `rust/src/render/superface.rs` (`COPLANAR_EPS`, `PATCH_EPS`) and the
  colouring and role table in `rust/src/render/labels.rs` (`MIN_SEP`); never
  assign labels by cycling a list. Get the merge predicate wrong and either a
  real corner melts invisible or two flush surfaces z-fight in G.
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
perception laws, the label clearance and superface merge law above, supported
platforms, the two
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

The Rust architecture below is mandatory, not aspirational. New code and
refactors must preserve all four laws; convenience, engine APIs, and a small
diff are not reasons to bypass them. Code review must identify which pure
component owns new logic, what its complete input domain is, and where its
dependencies enter.

- **Decouple components through explicit contracts.** Each component must own
  one coherent responsibility and depend only on values, traits, or narrow
  interfaces supplied by its caller. Use dependency injection for clocks,
  randomness, configuration, world queries, storage, and other collaborators.
  A component must not reach sideways into another component's internals or
  rely on call order, scene-tree location, initialization timing, or another
  undocumented ambient guarantee. Circular dependencies and knowledge of a
  concrete collaborator where a smaller contract suffices are forbidden.
  Decoupling is what lets the wave law, label allocator, or gait model be
  replaced and tested without constructing the rest of the game.
- **Every function must be total over its declared input domain.** For every
  value its type and public contract admit, it must return a defined result;
  it must not panic, index blindly, loop without a bound, emit NaN/Infinity,
  or depend on a supposedly impossible state. Represent absence and failure
  with `Option`, `Result`, or an explicit domain result. Narrow a domain with a
  validated type when invalid states must be unrepresentable. Engine callbacks
  are not an exception: validate untrusted Godot values at the boundary, then
  pass valid domain values inward. Tests must cover boundaries, degenerate
  values, and malformed external input, not only the shipped happy path.
- **Domain logic must be pure.** Given the same inputs it must return the same
  outputs without reading or changing the scene tree, clocks, random sources,
  files, environment variables, singletons, or mutable shared state. Put
  calculations and state transitions in engine-free modules as functions of
  immutable inputs and explicit prior state, and cargo-test them directly.
  Side effects are allowed only in thin boundary adapters: registered Godot
  nodes may read engine state, call a pure operation, then apply its returned
  commands or state. Do not hide an effect behind a helper and call it pure.
- **Global state is forbidden.** Do not add mutable statics, process-wide
  registries, service locators, implicit singletons/autoloads, or caches whose
  correctness depends on invisible shared lifetime. State belongs to an
  explicit owner and is passed, borrowed, or returned. If state must be shared,
  the owner, lifetime, synchronization, reset semantics, and test isolation
  must all be visible in its type and constructor. Constants and immutable
  compile-time tables are allowed because they carry no changing state.

Rust is the hidden engine: wave simulation, echo physics, kinematics,
animation math, and perception internals. Registered nodes are boundary
adapters, not a home for domain logic; expose only node classes, typed signals,
designer-facing `#[export]` knobs, and in-editor docs. The pure Rust modules are
the single behavioural source for native desktop and wasm builds.

The engine/content split is two enforceable laws:

- **Law 1 — everything a designer meets is a Godot object.** Every gameplay
  entity ships as a registered Rust tool node that works when dragged into the
  editor, or as a plain `.tscn` prefab composed from those nodes. A drawing
  node builds named blueprint limbs idempotently; an intentionally drawless
  datum says so explicitly. Each node is reached by recursive level census or
  has an explicit exclusion. World solids enter the per-face superface paint;
  creatures and sources use fixed role labels; drawless data consumes no
  label. Warnings are stored for editor triangles and printed only at runtime,
  with both the Godot virtual and callable forwarder. Designer-facing
  composition is scenes, nodes, signals, and Inspector properties, never
  custom Resources or generated intermediate files. Triggers and sequences
  are registered Rust nodes with typed signals. GDScript is tests and probes
  only, permanently.
- **Law 2 — everything else lives in Rust.** Pure laws belong in cargo-tested
  modules; registered nodes remain thin boundary adapters around that logic.

For every new designer-facing class, verify the complete new-object checklist:
tool registration; named-child clearing and ownerless rebuilding, or an
explicit drawless declaration; recursive census or an explicit exclusion;
per-face superface labels, fixed role labels, or an explicit label-free
classification; dual-channel warnings plus the callable forwarder and
boot-pattern coverage; ranged/suffixed Inspector hints and `editor-docs`
documentation; SVG icon and exact manifest count; both hand-written class
rosters; dependency injection before tree entry and no self-wiring; simulated
clocks and seeded randomness only; capture/restore coverage when stateful;
runtime portability checks rather than architecture conditional compilation;
cargo, gdUnit, editor-probe, and deliberate mutation evidence.

- Keep functions small enough that their domain, result, and dependencies are
  evident. Explicitly model inputs and outputs; do not conceal coupling merely
  to shorten a signature.
- Godot is the visible game: editor-authored scenes composed from registered
  Rust nodes. Author levels in the editor and derive technical contracts from
  scenes; shipped GDScript is forbidden.
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
`rust/src/observe/` expose pulse, eviction, superface class and coplanar label
fault, the older solid-granularity object-id, crossing, and reflection state.
A structured invariant is evidence only when something other than the code
under test produced the data it reads: `faults` is a postcondition over the
class graph, so what reached the GPU is pinned by the tests that read mesh
`CUSTOM0` back and by the web G-channel gate, never by `faults` alone. Use gdUnit4, the godot-mcp loop documented in
`docs/superpowers/mcp/godot-mcp-loop.md`, or a dump scene. Screenshots are a
last resort and reveal a missing observability surface. `explain_ray` reports
Rust's belief and cannot prove GLSL agrees. Keep `game/addons/godot_mcp/`
ignored and untracked because it must not reach the export.

## In-flight work

- **Editor-authoring campaign** (branch `worktree-editor-authoring-campaign`):
  state, remaining tasks, gates, and traps are in
  `docs/superpowers/handoffs/2026-08-12-editor-authoring-campaign-handoff.md`.
  Read it before touching that branch. Delete the handoff and this temporary
  section when the campaign merges.
