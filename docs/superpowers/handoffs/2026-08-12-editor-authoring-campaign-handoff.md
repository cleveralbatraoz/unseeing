# Editor-Authoring Campaign — Active Handoff

**Current record:** 2026-08-13. This file replaces the original 2026-08-12
SP3-start snapshot. Do not append new facts around old contradictions; revise
this record when the active state changes.

## Read this branch in this order

1. `AGENTS.md` is the project-policy authority. Its perception laws,
   engine/content split, TDD rules, integration boundary, and in-flight section
   all apply.
2. `CLAUDE.md` is only a three-line adapter that includes `AGENTS.md`. It does
   not own competing policy or architecture.
3. This handoff records the campaign's current state and decisions.
4. `docs/superpowers/plans/2026-08-12-campaign-close-checklist.md` records the
   remaining gates and the separate user authorizations.

The campaign spec and SP1/SP2/SP3/SP4 plans are frozen decision/execution
artifacts. Read their dated post-rebase supersession notes before relying on
their bodies. Code and `AGENTS.md` win where a historical plan describes an
intermediate architecture.

## Repository state and boundary

- Branch: `worktree-editor-authoring-campaign` in
  `.claude/worktrees/editor-authoring-campaign`.
- The campaign was rebased onto fetched `origin/main` at `dfbb69a`. That commit
  is the current merge base.
- `e5f874c` is the strict gdUnit-runner milestone. Post-milestone contract
  cleanup includes `1ab7f82`, `e01469d`, `5e647da`, and `517b483`; the
  handoff/wiki closeout is committed at `5eb7263`. History was then cleaned
  without changing the final tree. Further closeout commits may still move
  HEAD, so recompute the range, inspect `git status`, and preserve concurrent
  or user-owned changes before acting.
- The feature implementation is complete. The branch is not integrated, the
  wiki is not published, nothing is deployed, and no issue is authorized for
  closure.
- Keep this handoff and the temporary `AGENTS.md` in-flight section until an
  explicitly authorized merge. At merge, remove both together while retaining
  all canonical `AGENTS.md` policy.

The user's goal remains the acceptance criterion: a non-programming level
designer composes and tunes the game in Godot with scenes, typed nodes,
prefabs, transforms, signals, and Inspector properties. They never edit Rust
or GDScript, generate intermediate authoring files, or copy derived engine
state into a scene.

## Current architecture after the superface rebase

- Godot is the visible content layer. A designer meets a registered Rust tool
  node or a plain `.tscn` prefab composed from those nodes. Shipped GDScript is
  forbidden; the remaining GDScript is tests and probes only.
- Rust is the hidden engine. Pure, total modules own geometry, labelling,
  movement, wave, restore, and scheduling laws. Registered Godot classes are
  thin adapters that validate Inspector/tree input and apply explicit results.
- World solids are painted per face through `rust/src/render/superface.rs`,
  `labels.rs`, and `paint.rs`. Same-facing coplanar overlapping faces merge
  into one superface and receive one bit-identical per-vertex label. Separate
  touching solids keep label separation of at least `MIN_SEP = 0.08`.
- Sources and creatures retain fixed role labels and never enter world-face
  colouring. The pre-rebase six-slot source-recolouring, K7 pile, and
  source-starvation story is retired. The older solid-level object-id output is
  compatibility observability, not the rendering or authoring model.
- World labels are carried in mesh `CUSTOM0`, not assigned by cycling a list.
  `WaveObserver`'s superface membership/faults and mesh read-back tests are the
  relevant structural witnesses.

## What the campaign delivers

### Editor feedback and designer objects

- `SoundFan`, `SoundRadio`, `WaveCat`, the solid classes, `WaveLevel`,
  `WaveSpawn`, and `WaveRun` participate in editor authoring. Sources/cat and
  solids build blueprint limbs; `WaveSpawn` is explicitly drawless.
- Warning-bearing nodes expose both Godot's configuration-warning virtual and
  a callable forwarder. Warnings are stored in editor mode, printed only at
  runtime, refreshed when authored data changes, and addressed by level-relative
  paths.
- Rust-built preview limbs are ownerless derived data and are never packed into
  a prefab. Named blueprint limbs are rebuilt idempotently. WaveRun segments
  use a stricter generated identity: `WaveWall` type + private metadata + a
  typed `WaveRun` parent. `RunSeg1…N` names are for readable paths, not cleanup
  authority.
- All designer knobs carry appropriate hints/docs, ten SVG class icons are
  registered, and both hand-written class rosters contain all 19 engine
  classes, including `WaveRestorer`, `WaveSpawn`, and `WaveRun`.

### Typed spawn

- `WaveSpawn` is a tool `Marker3D` datum collected by Rust type, regardless of
  node name. Plain `Marker3D` nodes remain legal grouping aids and are ignored.
- No candidate keeps the level-origin fallback and reports a loud level
  diagnostic. One candidate is silent. With duplicates, the first depth-first
  walk candidate wins and every loser receives a warning naming the losers.
- Position is read from the datum's global transform. Yaw is derived from its
  global basis through a total pure helper, so a spawn nested in a rotated
  prefab composes correctly.

### Prefab library

- `game/scenes/props/chair.tscn` and `table.tscn` have plain `Node3D` roots and
  typed Rust pieces. The chair has a seat, four legs, and a back; the table has
  a top and four legs.
- `game/scenes/rooms/doorway_8m.tscn` and `room_16x16.tscn` provide configured
  reusable wall composition. The room has four border runs and a three-metre
  east opening.
- Level 01 instances the reusable furniture without changing its intended
  placements or its bespoke RadioTable. Runtime-load and editor probes cover
  plain roots, recursive census, independent preview limbs, repacking without
  leaked children, inherited transforms, and the actual touching face labels.

### WaveRun

- `from` and `to` are `Vector2` endpoints in the parent's local X/Z plane.
  Godot displays the second component as `y`; for this planar API it maps to
  the **parent's local Z axis**, not global/world Z.
- Each `openings` entry is `(absolute start coordinate on the selected
  parent-local axis, width)`. It is not an offset from `from`; negative widths
  use their magnitude and extend toward the increasing coordinate.
- The pure law normalizes reversed endpoints, selects the dominant axis with X
  winning ties, folds diagonals with a warning, rejects non-finite/zero runs,
  clamps/sorts/merges openings, and emits every positive residual segment.
- Setters rebuild in-tree. Material injection is retained and propagated.
  Rebuild first removes exactly the generated segment set, then emits ownerless
  `WaveWall` children.
- A WaveRun's own representable planar transform is absorbed into endpoints
  and openings and reset to identity. Y/tilt that cannot be represented warns;
  ancestor prefab transforms remain ordinary composition.
- Generated wall faults are surfaced on the authored WaveRun because its
  endpoints/openings are what a designer can repair. Level-relative paths keep
  repeated `RunSeg1` leaf names distinct.

### Level selection and playable authoring loop

- `UnseeingGame.level_scene` is an optional PackedScene picker. Empty uses the
  exact level-01 fallback. A selected resource is the only candidate; a root
  that is not `WaveLevel` is freed, reports its resource path, and returns
  before any world is added. Both valid paths share inject-before-add wiring.
- Level 02 is a 16×16 `WaveLevel` composed from the reusable room, typed spawn,
  fan, chair, and interior run. It derives six wall segments and a nonzero demo
  crossing without code changes.
- A raw `WaveLevel` tab is content, not a playable current scene: it lacks the
  composition root's player, hearing pass, materials, and wave pool. For F6,
  duplicate/open a code-free `UnseeingGame` runner, assign **Level Scene**, make
  that runner tab active, then choose **Run Current Scene**. F5 runs the
  project's configured main scene.

### Bootstrap and local editor tooling

- macOS/Linux use `tools/bootstrap.sh`; Windows uses
  `tools\bootstrap.cmd` backed by PowerShell. Both build the pinned Rust target
  with `editor-docs`, import after the native library exists, and demand the
  exact 19-class census. Python is deliberately not a fresh-Windows bootstrap
  dependency.
- Windows x86_64 is built and censused in CI. ARM64 selection/routing is
  boundary-tested and Cargo-checked; a real Windows ARM64 editor runner remains
  unavailable. Linux and macOS architecture routes are explicit.
- Godot MCP setup installs pinned addon 4.1.0 into ignored
  `game/addons/godot_mcp/`. The user enabled it and a live read previously
  confirmed the correct Godot/project/addon versions. The addon and its local
  project settings must never enter a commit or export. A final automated MCP
  acceptance pass is still required after the branch settles.

## Decision and verification ledger

The narrative commits carry the detailed why. These are the decisions most
likely to be lost by reading only the pre-rebase plans:

- `8bb9cb7` restored the designer-facing two-law boundary on top of main and
  translated the new-object checklist from flat object ids to per-face
  superfaces/fixed roles/drawless data.
- `347401e` retained WaveRun's full level-relative wall addresses and
  reconciled source coverage with the fixed-role `CUSTOM0` vocabulary. The two
  obsolete source-recolouring-only commits were removed during history cleanup
  rather than replayed and deleted.
- `07eaf0e` routed generated-wall diagnostics to the authored WaveRun and moved
  pose absorption into a total pure law with refusal for poisoned/overflowing
  input.
- `e046484` replaced first-face/object-id prefab claims with reads of the real
  two touching `CUSTOM0` face labels.
- `e5f874c` made gdUnit success depend on a source census and three exact
  terminal records rather than process exit or a generic `PASSED` line.
- `1ab7f82` removed remaining live flat-object-id/source-colouring prose,
  `e01469d` aligned bootstrap diagnostics with repository pins, and `5e647da`
  made the pure core's documentation state the superface truth.
- `517b483` clarified that generated RunSeg walls keep real level-relative
  paint addresses while their authored WaveRun owns the repairable editor
  warning.

Current measured baseline at this handoff:

- 405 Cargo tests.
- 320 gdUnit cases in 31 suites. `ci/run_gdunit.sh` computes these totals from
  source and requires exact zero-error/zero-failure/zero-skip overall,
  executed-suite, and executed-case records.
- 19 registered engine classes in both rosters.
- 10 registered SVG icons.

RED/GREEN and mutation evidence retained for closeout:

- WaveSpawn began red on missing typed APIs; typed-vs-name census and
  winner-selection mutations failed before the corrected implementation
  passed. The later nested-prefab editor probe is the independent global-yaw
  regression witness.
- Prefab runtime/editor tests went red on the absent library. Mutating the
  chair root from plain `Node3D` to a solid failed three focused cases. The
  post-rebase seam case now reads the two actual touching face labels rather
  than a first-face compatibility value.
- WaveRun began red on the absent pure/node APIs. Segment-removal, material,
  setter rebuild, transform mapping, opening coordinate, and warning clearing
  have focused coverage; deliberate segment-removal and shifted-opening
  mutations failed before restoration.
- Level selection began red on the missing property/second scene. Skipping
  inject-before-add and removing level 02's divider failed their wiring/census
  cases.
- Cross-platform bootstrap began red on missing Windows entry points and on a
  POSIX script that accepted an incomplete class census. Target swaps,
  `editor-docs` removal, pin weakening, ignored Cargo failure, and import-order
  mutations failed the boundary suites.
- The strict gdUnit harness began red because its runner/call sites were
  absent. It now rejects parse-lost suites/cases, wrong ratios, errors,
  failures, skips, duplicate summaries, empty suites, ANSI tricks, and nonzero
  runners. Removing any of the overall/suite/case witnesses makes its named
  mutation case red.

Earlier pre-rebase export/browser-smoke results are historical evidence only;
they are not a substitute for the final post-rebase full pipeline.

## Remaining work before asking for integration

1. Finish and review the active live-prose/diagnostic cleanup without absorbing
   unrelated worktree changes.
2. Run focused gates, then `SKIP_EXPORT=1 ci/pipeline.sh`; reconcile the exact
   405/320/31/19/10 baseline rather than trusting a stale table.
3. Independently review `origin/main...HEAD` for spec compliance and code
   quality, fix findings, and rerun the complete gates.
4. Run the full export/browser-smoke pipeline with an explicit non-deployable
   destination. Do not invoke deployment tooling.
5. Use Godot MCP for every automatable editor acceptance check after the final
   build/import. Only then ask the user for the remaining visual/interactive
   checks in one consolidated editor session.
6. Present exactly one integration menu: merge locally, push/open a PR, or keep
   the local branch. Do nothing with that choice until the user explicitly
   selects it.

## Authorization boundaries

- Integration, wiki publication, deployment, and issue closure are four
  separate user decisions.
- A merge choice authorizes only the merge. It does not authorize a push, wiki
  publication, deployment, or issue closure.
- The reverted wiki commit `9778a00` is historical input only. Its six-slot
  source-colouring model is obsolete; never revive or cherry-pick it verbatim.
  Any authorized wiki pass must re-derive and rewrite it for superfaces.
- Deployment requires a separate instruction after an approved merge, from a
  clean shared `main`, through the repository's gated deployment workflow.
- Issue closure requires a separate instruction and per-issue evidence links.

## Traps worth keeping visible

- Inject every dependency before `add_child`; no node self-wires.
- Never assign owners to Rust-generated limbs/segments. Authored scene children
  and ownerless derived preview/runtime children are different contracts.
- gdUnit assertions do not halt a test; guard before indexing. Trust only the
  exact source/runner census gate.
- `.gdextension` loads the release library. A debug-only rebuild can make the
  editor appear stale.
- Godot 4.7 can rewrite `unique_id=` values on save. Do not treat those values
  as semantic identity.
- Headless editor probes (`godot --headless -e -s`) reach editor-hint branches;
  use them before asking a human to inspect a triangle or blueprint.
- Never commit `game/addons/godot_mcp/`, MCP-created project settings, build
  output, exports, reports, or the user's unrelated scene edits.
