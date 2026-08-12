# Editor-Authoring Campaign — Handoff (2026-08-12)

**Audience:** any agent or human picking up this branch cold (written for a
cross-tool handoff; assumes no access to session history, machine-local
scratch, or any prior assistant's memory). Reading order: this file →
`CLAUDE.md` → the SP3 plan. That is everything needed to continue.

## Where you are

- **Branch:** `worktree-editor-authoring-campaign`, worktree at
  `.claude/worktrees/editor-authoring-campaign`, branched from main @
  `3f376cf`, pushed to origin. HEAD at handoff: `0bc2491` plus this commit.
- **Campaign goal (the user's):** a game designer edits levels entirely in
  the Godot UI editor — picks a node/prefab, places it, it just works;
  drags it elsewhere, still works; everything saved as `.tscn` text; no
  designer-facing intermediate state. Spec:
  `docs/superpowers/specs/2026-08-11-editor-authoring-campaign-design.md`
  (read its **Errata** section too — two premises were later disproven and
  are recorded there).
- **Three anchor decisions, frozen by the user:** free placement is law
  (touching sources must draw their seam — palette surgery, never an
  error); no designer-facing intermediate state (`.tscn` is the only
  artifact); the designer-facing razor (Godot holds only what a designer
  needs — the composition root itself is Rust). Do not reopen these.
- **Four sub-projects, executed 1 → 4 → 2 → 3.** SP1, SP4, SP2 are
  **complete, reviewed, and committed** on this branch. SP3 is the last;
  its plan is committed and **zero SP3 implementation exists** — Task 1
  had not started when this handoff was written.

## What is already done (do not redo, do not re-review)

Each sub-project passed a per-task review and a fix wave; final states:

- **SP1 "Place it and see it"** — complete at `000c870` (plan:
  `docs/superpowers/plans/2026-08-11-editor-authoring-sp1-place-and-see.md`).
  Tool classes with named blueprint limbs, WaveLevel editor derive as
  fault-collecting planner (`level_faults`/`node_faults`/`rederive()`),
  scene-signature watch, `PlacementFault`, ranged+suffixed knobs, 8 SVG
  icons, `editor-docs` cargo feature, `tools/bootstrap.sh`, three headless
  editor probes in `ci/pipeline.sh` with exact check-count assertions.
- **Merge of origin/main** at `e0c0250` — 16 capture-restore commits landed
  on main mid-campaign; everything after is planned against the merged shape.
- **SP4 "The Rust composition root"** — complete at `b920f07` (plan:
  `…sp4-rust-root.md`). The game boots a registered Rust `UnseeingGame`
  node (`rust/src/nodes/game.rs`); `main.gd`/`flicker.gd`/`demo_tap.gd`
  deleted; the shipped artifact carries **zero GDScript** (`pulses.gd` is
  test-only under `game/tests/`, excluded from export). Flicker law ported
  bit-exactly; `capture_env`/`apply_env`/`restore_blob` ported with exact
  semantics including the post-write no-rollback asymmetry.
- **SP2 "Any edit ships"** — complete at `f9b76fc` (plan:
  `…sp2-laws-and-seams.md`). `WORLD_OIDS` is six slots
  `[0.05, 0.25, 0.34, 0.43, 0.52, 0.61]`; ~29 shipped-census test pins
  retired into level-agnostic laws with non-vacuity guards; sources are
  colourable role pairs (`role_count`/`set_role_oids`/`role_oid`, constants
  like `FAN_OID` deleted); seam law is red-capable
  (`game/tests/source_seam_test.gd`); buried-in-wall runtime heir;
  capacity law: 2×sources + solids ≤ 6 per mutually-touching cluster.
- **The two-layer standard ratified** at `faa759c` — CLAUDE.md's Law 1 /
  Law 2 and the **new-object checklist**, which is the acceptance test for
  every class SP3 adds.
- **godot-mcp** at `7c95242`+`34e4a34` — `.mcp.json` pins
  `@4.1.0` via npx; `tools/setup-mcp.sh` installs the editor addon and
  never touches `project.godot`; `game/addons/godot_mcp/` must stay
  untracked (deploy ships by `git archive`; the vendoring fingerprint
  assumes gdUnit4 is `game/addons/`' only tenant).
- **SP3 plan committed** at `0bc2491`:
  `docs/superpowers/plans/2026-08-12-editor-authoring-sp3-vocabulary.md`.

**Test baselines after the rebase correction: 334 cargo / 276 gdUnit cases /
31 suites; 17 registered engine classes.**
Every task must predict its count delta and match it.

## The active work: SP3 "The Vocabulary"

The plan file is the authoritative task spec — self-contained, bite-sized,
TDD-shaped, with exact values. Its five tasks, none started:

1. **Task 1: WaveSpawn — the spawn becomes a class.** Typed spawn node
   (`#[class(tool, init, base=Marker3D)]`, no knobs, no limbs), retires the
   `"SpawnPoint"` name law completely (`SPAWN_NAME`/`spawn_name` deleted;
   `choose_spawn` becomes a candidate-count law; origin fallback kept), and
   fixes yaw to derive from the **global** basis (a spawn nested in a
   rotated prefab currently wakes the hero facing the wrong way). ~10
   fixture sites migrate in ONE commit with the law change.
2. **Task 2: The prefab library.** `game/scenes/prefabs/` (chair, table…),
   level_01 re-nested from prefabs; includes the nested-prefab-yaw gdUnit
   test that provides Task 1's deferred mutation evidence.
3. **Task 3: WaveRun — ends and openings.** Walls authored by endpoints
   (`from`/`to` parent-local XZ `Vector2`) plus `openings`
   (`PackedVector2Array` of offset-along-run, width). Emits real `WaveWall`
   children `"RunSeg1"…N`, cleared by name every ready. A run with zero
   openings IS an endpoint wall — `WaveWall`'s own contract is untouched.
4. **Task 4: The level knob and the second map.**
   `#[export] level_scene: Option<Gd<PackedScene>>` on `UnseeingGame`
   (empty → level_01 fallback; wrong scene → loud `"UnseeingGame: "`
   refusal naming the path), plus `level_02.tscn` — 16×16, one room, built
   from the prefab library, border walls via WaveRun, proof a new level
   runs from the editor.
5. **Task 5: The paper trail and the campaign-close checklist.** README,
   wiki-debt additions, spec cross-references — the campaign's closing
   bookkeeping.

**Design decisions locked by research** (do not re-litigate; full text in
the plan's "Decisions Locked by Research" section): WaveRun subsumes
endpoint walls; WaveRun knobs are parent-local XZ with axis-fold-and-warn;
WaveSpawn retires the name law with global-basis yaw; the level knob's
empty→level_01 fallback and loud refusal; level_02 is 16×16 by law
(diagonal ≈ 22.9 m against the 40 m pack-range ceiling whose shipped
headroom is only 0.27 m at 28×28).

**Ledger state:** the machine-local ledger
(`.superpowers/sdd/2026-08-12-editor-authoring-sp3-vocabulary/progress.md`,
git-ignored) records only: no tasks complete; Task 1 BASE = `0bc2491`. If
you work on another machine, recreate a ledger from this file; if on the
same machine, the workspace also holds `task-1-brief.md` (the plan's Task 1
text) and `global-constraints.md` (the plan's constraints + locked
decisions).

## Process contract (translated from the plugin-based workflow)

CLAUDE.md delegates process to the "superpowers" plugin, which is
Claude-side tooling. If you don't have it, follow this faithful
translation — the obligations are the same:

- **Per task:** strict TDD (write the failing test, watch it fail for the
  right reason, minimal code, watch it pass; hand-derive expected literals,
  never read them back from the code under test) → full pipeline green
  (`SKIP_EXPORT=1 ci/pipeline.sh`) → **independent review of the task's
  diff** (spec compliance against the plan text AND code quality; generate
  the diff as `git diff BASE..HEAD` with the BASE recorded before the task
  started, never `HEAD~1`) → fix findings → record completion → next task.
- **Mutation checks before calling a task done:** flip each realistic
  constant/branch/side-effect the task added; each mutation must fail at
  least one test. The plan lists the specific mutations per task.
- **After Task 5:** one final whole-branch review (the diff from `3f376cf`
  merge-base to HEAD), one fix wave, then STOP — the campaign close below
  is the user's, not yours.
- **Known review hazard in this repo:** the costliest defect class is a
  **confidently false claim** — a doc comment, module doc, or report
  sentence describing behaviour that isn't there. Reviewers must re-derive
  claims, not re-read them. Nearly every prior task's first review failed
  on one of these.

## Gates, and the three ways they lie

- Full gate: `SKIP_EXPORT=1 ci/pipeline.sh` (runs repo hygiene, vendoring
  verify, cargo fmt/clippy/test, gdUnit4 headless, editor probes). Web
  export + browser smoke: `ci/pipeline.sh` without the skip; full deploy
  is `deploy.sh` (campaign close only — see below).
- **gdUnit4 lies three ways:** exit 0 on a parse failure; a green
  `PASSED` line that carries failures; and a fresh worktree with no
  `game/.godot` cache **runs zero tests while exiting 0**. Run
  `godot --headless --import` first; trust only suite+case counts
  (334 cargo / 276 cases / 31 suites at HEAD).
- Boot-gate: any new class-style complaint opening (e.g. `"WaveLevel: "`)
  must be a literal in `ci/boot_error_pattern.sh`'s pattern in the same
  commit. Prefer composing new complaints in `level_plan.rs` relayed
  through WaveLevel — `"WaveLevel: "` is already covered, so zero gate
  edits. Dual-channel law: every fault is always *stored* (editor fault
  list) and *printed* at runtime only.
- Probe runners assert exact `probe: PASS (N checks)` counts — a new check
  changes N in the runner too.
- Formatters before every commit: `cargo fmt`, `cargo clippy` (warnings
  are errors), `gdformat`, `gdlint`.

## Registration ripple for every new node class

Both hand-written rosters (`game/tests/engine_binary_test.gd` and
`game/tests/probe/engine_census_probe.gd` — 17 names at the corrected
baseline); `[icons]` in
`game/project.godot` + a new SVG + its `.import` sidecar +
`icon_manifest_test.gd`'s exactly-N function (the function name carries the
count — rename it); `knob_hint_test.gd` for new hinted knobs;
`rust/src/nodes/mod.rs` alphabetical; the CLAUDE.md new-object checklist is
the full acceptance list (tool class, named `LIMBS` + `clear_limbs`
idempotence, censused or an explicit collect() arm, warnings-forwarder
pair, ranged knobs + `///` docs behind `editor-docs`, injected-never-
self-wired, capture/restore blob if stateful, wasm-safe via runtime checks,
simulated clock + seeded randomness only).

## Traps this campaign already paid for (do not rediscover)

## SP3 execution ledger

- **WaveSpawn:** chose an intentionally drawless `Marker3D` datum so a
  designer places a transform and nothing else. Census is by Rust type in
  scene walk order, never by node name. With no datum the level origin remains
  the safe fallback and the level warns loudly; with duplicates the first wins
  deterministically and every losing datum carries the same warning naming all
  losers. Heading comes from the datum's global basis through a total pure
  helper, so nested rotated prefabs compose without leaking scene conventions
  into Rust. Plain markers remain valid grouping/editor aids and are ignored.
- **Verification:** RED began with missing `WaveSpawn`/typed-candidate APIs.
  Winner-selection and typed-census mutations failed as intended; the corrected
  release build passed 332 Cargo tests, 277 gdUnit cases in 31 suites, editor
  probes, and the 18-class/9-icon censuses. No independent agent was used:
  current orchestration policy forbids delegation unless explicitly requested,
  so specification and quality reviews were performed as separate local passes.
- **Prop prefabs:** chose plain `Node3D` roots and authored only typed Rust
  pieces below them. The reusable chair preserves the shipped seat, four legs,
  and back; the table preserves its top and four legs. Level 01 now instances
  the kitchen table and both chairs at their original global placements, while
  the deliberately different RadioTable remains bespoke. Runtime loading
  proves recursive census and distinct ids; the headless editor probe proves
  per-instance limbs, ownerless generated children, leak-free repacking,
  touching-pair colouring, and nested global spawn yaw. Changing the chair root
  to `WaveProp` produced three focused-suite failures, pinning the composition
  boundary. Counts at this gate are 332 Cargo and 279 gdUnit cases in 31 suites.
- **WaveRun and rooms:** an opening pair means `(absolute start coordinate
  on the selected parent-local axis, width)`, with width extending toward the
  increasing coordinate. This resolves the wording against the frozen shipped
  geometry: `(8, 4.4)` is exactly the gap 8.0..12.4 and reproduces divider
  centres 4.3/15.9 at lengths 7.4/7.0. Rust normalizes endpoint order,
  magnitudes, clamping, sorting, overlap and diagonal folding; X wins a
  dominant-axis tie. The tool node owns only that data and emits ownerless
  `RunSeg1…N` WaveWalls, clearing on every rebuild and retaining injected
  material. Its own planar transform is folded into endpoints and openings;
  ancestor prefab transforms remain ordinary composition. Y/tilt is projected
  with a warning because the vocabulary is deliberately planar. Level 01's two
  doorway pairs now derive the same four of its unchanged 19 segments. Added a
  configured doorway and a plain-root 16×16 room whose east border has a 3 m
  opening. Clear-removal and +1 m opening mutations failed their focused tests.
- **Level selection and level 02:** `UnseeingGame.level_scene` is an optional
  PackedScene resource picker. Empty retains the exact level-01 load; a selected
  scene is the only candidate and a wrong root is freed, reports its resource
  path, and returns before adding any world. Both choices converge on the same
  inject-before-add path. Level 02 is a 16×16 WaveLevel composed entirely in
  scenes: the reusable room, spawn `(4,0,8)`, fan `(12,0,8)`, chair `(4,0,11)`,
  and an interior run x=8 from z=2..14. Its split east border plus divider gives
  six wall segments and a nonzero demo crossing. Skipping injection broke the
  wiring fixture; removing the divider reduced the census to five and failed
  the selector case. The resource hint, exact fallback and wrong-root refusal
  are independently pinned.
- **Documentation close:** the in-repo authoring guide now teaches typed
  spawns, plain-root prop/room prefabs, absolute-start WaveRun openings, the
  level resource picker, and Godot's actual **Run Current Scene** / F6 loop.
  Wiki changes remain debt only and the close checklist separates the human
  editor, integration, wiki, deployment, and issue-closure authorizations.
  Final automated milestone: 339 Cargo tests; 289/289 gdUnit cases in 31/31
  suites; 19 registered classes; ten icons. The final full pipeline also built
  the wasm side module, exported the Web build, and passed browser smoke
  (`51730/184320` lit pixels) with `DEPLOY_DIR` set to the deliberately absent
  `/tmp/unseeing-campaign-nondeployable`; the pipeline confirmed build-only and
  copied nothing.
- **Godot MCP setup:** the pinned addon installer initially found no Node.js.
  Installed Homebrew `node@22` 22.23.2, reran `tools/setup-mcp.sh`, and installed
  ignored addon 4.1.0. Enabling the editor plugin and reconnecting the MCP
  transport remain the documented one-time interactive step; no shipped plugin
  setting was changed. Headless editor probes cover every automatable editor
  law meanwhile.

- **Rebase erratum:** `WaveRestorer` was registered in Rust but omitted from
  both independently maintained engine rosters. The post-rebase baseline adds
  it, so SP3 starts at 17 classes rather than the stale 16-class claim.

- **Injection order is law:** inject dependencies BEFORE `add_child`;
  engine-emitted/instanced children are ownerless (**never `set_owner`** on
  built limbs); Ctrl+D-duplicated nodes carry ghost children referencing
  the original's meshes — clear BY NAME and `.free()` immediately, never
  adopt.
- gdUnit asserts don't halt on failure — guard with `fail()` + `return`
  before indexing, or a forced-empty run crashes instead of failing.
- No mirror assertions (expected values computed by the code under test),
  no change detectors (assert behaviour, not constants).
- `mesh_world_box` stops at censused CHILDREN (root exempt) — a censused
  child's geometry never doubles into its parent's box.
- clippy does NOT flag dead fields on GodotClass structs — check field
  liveness by hand.
- gdext 0.5.4: tool classes run all virtuals in the editor;
  `#[export(range=(…, suffix=" m"))]` stacks with `#[var(get,set)]`; the
  `_get_configuration_warnings` GDVIRTUAL is never bound to ClassDB, so
  every warning-bearing class needs the inherent `#[func]` forwarder twin;
  `.gdextension` loads the RELEASE artifact — a debug-only rebuild shows
  stale behaviour.
- Godot 4.7 mints random `unique_id=` per node on editor save — expect
  `.tscn` diff noise, never build on those ids.
- Headless editor mode IS testable: `godot --headless -e -s probe.gd` sets
  `is_editor_hint`.

## Standing repo rules that bite

- **Commits:** small, green, narrative subject lines matching the existing
  history; body carries the technical what/why. Repo-local identity
  `Dmitrii Galchenko <dggrus@gmail.com>`. **No authorship attribution of
  any assistant or tool — no Co-Authored-By, no "Generated with", in
  commits, code, comments, docs, or PRs.**
- **Never write the site's DNS hostname anywhere** — use the raw IP
  `206.223.241.165` only (public repo; sole exception is the nginx cert
  paths on the droplet itself).
- **Do not touch or push the wiki remote until the campaign merges.** All
  wiki changes accumulate in
  `docs/superpowers/plans/2026-08-11-editor-authoring-wiki-debt.md`. A
  draft describing SP2 behaviour survives as wiki commit `9778a00`
  (reverted from the live wiki) — revive it at merge.
- Never bare `git stash` (the stash stack is shared across concurrent
  worktrees) — use a WIP commit, or `git stash push -u -m "<tag>"` +
  `apply` by SHA.
- Repo is source-only: no build output, no binaries beyond the audited
  PNGs; pre-commit rejects staged files over 5 MiB.
- Windowed runs need `game/override.cfg` (gitignored) — CLI flags cannot
  beat the fullscreen project setting.

## Campaign close (after SP3's final review — the user drives this)

1. Final whole-branch review + fix wave (see process contract).
2. The user owes ONE consolidated **human editor session** before merge:
   Scene-dock warning triangles appear/clear on drag (SP1), blueprint
   shapes + Create Node icons (SP1), starved-source triangle when a 7th
   node joins a cluster (SP2), prefab drag + WaveRun editing + the
   level_scene knob + Run Custom Scene (SP3).
3. Present the integration menu — merge to main / PR / keep the branch —
   **the user decides.** Merging happens in the shared checkout
   `/Users/dmgalchenko/unseeing` against a clean tree only.
4. At merge: push the accumulated wiki debt (including the revived
   `9778a00` draft and the new "Mechanics — Adding an Object" page).
5. Deploy AFTER merge, from main, in the shared checkout: `deploy.sh`
   (test-gated; it verifies `UNSEEING_BUILD` off the live page because
   `git push` success does not prove the post-receive hook ran). Droplet
   access: `ssh vpn`; sudo prompts for a password; only 22/80/443 open.
6. Close issues: #16 #22 #30 #31 #32 #33 #34 #35 #36 #38 (scoped) #39 #41
   #42 #44 #45.

## Backlog (noted, deliberately NOT in SP3)

- `DIST_PACK_RANGE` raise decision (shipped headroom 0.27 m at 28×28).
- Retag-loop slot-ranges refactor when palette-adjacent code is next
  touched.
- Windowed probes (`probe_display.sh`, `probe_visibility.sh`) still use
  the bare `probe: PASS` grep the headless runners were cured of.
- Doc sentence: standalone multi-source preview shares default ids (melt
  in preview only).
- `pulse_pool.rs` `REFUSAL_MESSAGE` still opens `"Pulses.emit:"` — rename
  when gate work next touches its pins.
- `game_root_test.gd` keeps 2 level_01-census-coupled pins (u_count==2,
  Fan/DividerNorth muffle fixture) — law-shape or exempt them when next
  touched.
