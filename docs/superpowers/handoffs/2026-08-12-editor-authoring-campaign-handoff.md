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
  without changing the final tree. The branch pushed through `fd048b8`, then
  the current adversarial session added the pure paint-plan, temporal-totality,
  fixed-anchor, and archive-boundary commits listed in the decision ledger
  below. Further closeout commits may still move HEAD, so recompute the range,
  inspect `git status`, and preserve concurrent or user-owned changes before
  acting.
- The planned feature implementation and the closeout `ArrayMesh` winding
  correction landed. Whole-branch review then found and repaired the
  same-class source seam, GDScript resource-policy gaps, WaveRun regeneration,
  live WaveWall transforms, residual shipped-content goldens, a non-atomic
  paint boundary, malformed temporal input, impossible fixed-label separation,
  and developer MCP files leaking into deployment archives. The last pushed
  checkpoint is `fd048b8`; the current local review series begins at `861cc45`
  and includes `b64e337`, `fcf6075`, `3164ba3`, `5beab1d`, `6e4cfc9`, and
  `f832826`. The temporal and final paint series have independent approval;
  complete whole-session/whole-branch review, export/MCP evidence, the last
  rebase, and all post-rebase gates remain. The branch is not integrated, the
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
- World solids are painted per face through `rust/src/render/faces.rs`,
  `superface.rs`, `labels.rs`, and `paint_plan.rs`; `paint.rs` is only the
  Godot `ArrayMesh` layout/submission boundary. Same-facing coplanar overlapping
  faces merge into one superface and receive one bit-identical per-vertex
  label. Separate touching solids keep label separation of at least
  `MIN_SEP = 0.08`.
- Sources never become world superfaces, but their semantic limb roles join the
  same separation graph as world face classes. The level derives numeric labels
  per source instance, so two touching copies retain their seam; creatures keep
  fixed numeric role labels. The pre-rebase six-slot/K7 mechanism is retired,
  while repairable source-role starvation is now a real shared-graph outcome.
  The older solid-level object-id output is compatibility observability, not the
  rendering or authoring model.
- World labels are carried in mesh `CUSTOM0`, not assigned by cycling a list.
  `WaveObserver`'s superface membership/faults and mesh read-back tests are the
  relevant structural witnesses.
- Pure box/wedge/column/torus generators use the conventional
  counter-clockwise/outward law. Godot 4.7 defines clockwise `ArrayMesh`
  triangles as front-facing, so `render::paint` converts complete conventional
  triples at submission. Hero/cat sphere and tube buffers already emit
  Godot-clockwise order and use the direct door. `render::faces` remains
  counter-clockwise analytic superface geometry and is not a render index
  buffer.

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
- An authored `WaveWall` is an honest `Node3D` datum with an explicit collision
  contract, not a dummy `StaticBody3D`. Its ownerless private top-level body,
  skin, and collider share one exact canonical world frame with paint and
  occlusion. The editor normalizes live ancestor edits before level derivation;
  runtime wall geometry is immutable after ready. Invalid length, priority, or
  transform input remains finite and produces one repairable warning owned by
  the wall.
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
- Construction setters are live before tree entry, and setters rebuild in-tree
  only in editor mode. Material injection is retained and propagated. An
  editor rebuild first removes exactly the generated segment set, then emits
  ownerless `WaveWall` children.
- The editor scene signature includes each censused node generation. An
  equivalent setter may recreate byte-identical RunSeg geometry, but the new
  instance identities still force one repaint and replace every retained wall
  handle before the next stable frame.
- A WaveRun's own representable planar transform is absorbed into endpoints
  and openings and reset to identity during construction or editor authoring.
  Y/tilt that cannot be represented warns; ancestor prefab transforms remain
  ordinary composition.
- Runtime ready freezes the generated generation exactly like WaveWall's
  geometry snapshot. Later endpoint, opening, or local-transform writes are
  ignored without freeing/rebuilding RunSeg walls; properties, identity,
  `CUSTOM0`, centerlines, and the level's retained wall names stay unchanged.
  `level_plan::authored_geometry_edit_is_live` owns the complete pure lifecycle
  table and both node boundaries supply only `inside_tree`/editor-mode values.
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
  enabled-plugin edit must never enter a campaign commit; every export preset
  excludes addon scripts.
- The required MCP mesh validator exposed the winding defect: the first
  configured level-02 runner found all 22 then-populated surfaces backwards.
  A readiness audit corrected the acceptance contract: raw level 02 is 14/14
  because uninjected sources deliberately stay absent; the configured level-02
  runner reaches 24/24 after its hero meshes populate; configured main reaches
  144/144. These exact zero-finding states must be repeated after the final
  pipeline/rebase. Editor-only blueprint presence remains covered by the
  editor-source probe because the MCP validator walks only a running scene.

## Decision and verification ledger

The narrative commits carry the detailed why. These are the decisions most
likely to be lost by reading only the pre-rebase plans:

- `8bb9cb7` restored the designer-facing two-law boundary on top of main and
  translated the new-object checklist from flat object ids to per-face
  superfaces/semantic roles/drawless data.
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
- `59efd5d` put semantic source roles into the shared separation graph. Two
  touching same-class radios now carry distinct real `CUSTOM0` labels; source
  assignments persist across limb rebuilds and starvation belongs to the
  source a designer can move.
- `0ba5ba0` closed Godot's other script doors: production resources cannot
  embed GDScript, load anything under `res://tests/`, or autoload test content.
- `97530a4` moved room-dependent tests onto code-built fixtures and retired the
  remaining shipped name/count/placement goldens without weakening the laws.
- `c8744de` made live walls one exact Node3D/private-body geometry and folded
  censused instance identity into the editor watch, so equivalent WaveRun
  rebuilds cannot leave placeholder paint or freed wall handles.
- `861cc45` moved the complete face/source paint decision into pure
  `render::paint_plan`. Request-wide planning and label-assignment failures now
  refuse before the Godot boundary mutates any mesh or source role, while
  repairable entry/source faults retain their original census owners and
  existing labels.
- `b64e337` made frame advancement, restored flicker state, demo appointments,
  and random samples total over their admitted input. The shared
  renderer-visible time horizon is a pure contract; the Godot root applies the
  returned transition and warns once when it repairs native temporal input.
- `fcf6075` rejects fixed-label separation conflicts inside the pure planner,
  before any paint command escapes, and closes malformed-bound/source-owner
  and palette-capacity mutation gaps without changing the valid merge law.
- `3164ba3` keeps `.mcp.json` and `tools/setup-mcp.sh` in developer checkouts
  while excluding both from the exact `git archive` deployment boundary. The
  repository-hygiene test proves both sides and fails if either exclusion is
  removed.
- `5beab1d` consolidated the renderer-visible horizon and clock transition in
  `rust/src/temporal.rs`; Flicker and DemoTap consume the same pure contract
  and the Godot root only applies it. Its focused run passed 11 DemoTap, ten
  Flicker, and five temporal tests. Moving the last-fire boundary past the
  horizon and moving the horizon itself each failed their intended tests.
- `6e4cfc9` made pure shapes the sole owner of conservative entry bounds,
  validates source bounds after sweep growth, and refuses oversized requests
  before quadratic graph work. Flank, source-role, starvation, and role-state
  laws moved into the atomic planner; `render::paint` remains only the Godot
  mesh-layout/submission boundary. Its first independent review found
  super-pairwise duplicate searches and overbroad fallible-allocation prose;
  those findings were fixed rather than accepted as debt.
- `f832826` closes that paint review with deterministic logarithmic membership
  checks that preserve edge insertion order, truthful request/allocation prose,
  shape-owned ordinal contracts, and direct conservative-bound fixtures. The
  final independent verdict is approved: 32/32 focused planner tests and
  454/454 all-target/all-feature Rust tests passed; the maximum admitted K512
  graph produced exactly 130,816 unique pairs in 0.09 seconds of test-body
  time (0.44 seconds for the Cargo process). Direct bounds, deduplication, and
  request-ceiling tests provide the final mutation sensitivity.

History references rewritten by the rebase must not be read as missing work.
The live equivalents are `93f4140` → `4897683`, `2ff5bdf` → `1e88abf`,
`c0ecba9` → `3f4f0eb`, and `6cc6c54` → `f8aeb2f`. The old merge checkpoint
`e0c0250` has no merge-commit descendant: `6a9e0e1` is the corresponding
linear pre-SP4 boundary on the rebased branch, whose actual base is `dfbb69a`.
The old final-review baseline `b920f07` was folded into the rewritten Rust-root
paper trail; use `9b3773e`, the live pre-SP2 boundary, for current ancestry.

Strict full-runner census at `c8744de` on 2026-08-13:

- 419 Cargo tests with all targets/features (417 in the default suite plus two
  focused `editor-docs` feature tests).
- 327 gdUnit cases in 31 suites. `ci/run_gdunit.sh` computes these totals from
  source and requires exact zero-error/zero-failure/zero-skip overall,
  executed-suite, and executed-case records.
- 19 registered engine classes in both rosters.
- 10 registered SVG icons.
- Editor probes passed 7/7 in both slab modes, 11/11 source-blueprint plus
  3/3 uninjected-source checks, 29/29 live-level plus 1/1 runtime-level checks,
  and 16/16 prefab checks. `SKIP_EXPORT=1 ci/pipeline.sh` completed green at
  this checkpoint.

Current local focused evidence after `f832826` (not a substitute for the final
post-rebase pipeline):

- 454/454 Cargo tests with all targets and features, with formatting and Clippy
  warnings denied.
- 328 authored gdUnit cases in 31 suites by the same source-census predicate as
  `ci/run_gdunit.sh`; final execution and exact terminal summaries remain a
  close gate.
- Repository hygiene passed with both developer MCP files present in the
  checkout and absent from `git archive HEAD`.

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
- WaveRun's equivalent-setter regression began red with fresh placeholder
  `CUSTOM0` labels and retained freed handles. Folding node instance identity
  into the scene signature makes the next editor pass repaint the new
  generation; the real editor probe pins safe labels, live paths, and an idle
  stable follow-up frame.
- The later runtime WaveRun regression began red with changed exported data,
  replaced RunSeg identities, `<freed wall …>` retained names, ordinal
  `CUSTOM0`, and changed centerlines after manual rederive. The shared pure
  four-state lifecycle rule and runtime boundary guards keep that whole
  ready-time generation exact while the existing editor probe keeps live
  rebuild coverage.
- Live WaveWall correction began red on oblique ancestors, singular/non-finite
  input, runtime/editor divergence, and repeated physics writes. Pure transform,
  length, and priority plans now feed one private top-level physics body and the
  same analytic paint/occlusion frame. Split gdUnit cases and 29 editor checks
  pin exact pose, signal/property forwarding, warning clearing, packing
  exclusion, length-only rederivation, and no-write stability.
- The winding correction began red when `godot_validate_meshes` reported all
  22 then-populated configured level-02 surfaces backwards under Godot's documented
  clockwise-front convention. Engine-bound box/outward-adapter cases and
  production prop/source/cat/viewmodel cases pin the two submission doors; a
  pure wedge case joins the existing box/column/torus outward proofs.
  Reverting box indices caused 12 focused failures; bypassing the outward
  conversion caused 2,441; misrouting the already-clockwise limb door caused
  2,384; and adding label carry to that direct door failed its write-through
  witness. The restored focused suites passed 64/64. A later readiness audit
  caught and rejected zero-finding passes taken before runtime-only meshes had
  populated; the close checklist now requires 14/14, 24/24, and 144/144.

Earlier pre-rebase export/browser-smoke results are historical evidence only;
they are not a substitute for the final post-rebase full pipeline.

## Remaining work before asking for integration

1. Finish independent review of every current-session commit and the complete
   `origin/main...HEAD` branch diff. Preserve the user's MCP/editor state and
   any concurrent changes; review reports are evidence only after the findings
   have been checked against the actual diff and focused tests.
2. Treat the green `SKIP_EXPORT=1 ci/pipeline.sh` run at `c8744de` (419 Cargo
   tests across default/editor-doc configurations, exact 327/327 gdUnit cases
   in 31/31 suites, 19 classes, ten icons, and every editor probe) as a
   checkpoint; rerun it after the final documentation/rebase state.
3. Run the full export/browser-smoke pipeline with an explicit non-deployable
   destination. Do not invoke deployment tooling.
4. Repeat the corrected three-state MCP mesh gate after the final build, then
   ask the user only for the remaining visual/interactive checks in one
   consolidated editor session.
5. Fetch and rebase the current branch onto the latest `main` only after the
   pre-rebase diff is reviewed and green. Re-review the rewritten commits and
   rerun every focused, pipeline, export/browser, and MCP gate afterward.
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
- Do not apply one blanket triangle reversal. Pure prop/source generators enter
  through the outward-converting paint door; already-Godot-clockwise hero/cat
  limb buffers enter through the direct door.
- Never commit `game/addons/godot_mcp/`, MCP-created project settings, build
  output, exports, reports, or the user's unrelated scene edits.
