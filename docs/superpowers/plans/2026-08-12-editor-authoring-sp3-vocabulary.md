# Editor Authoring SP3 — The Vocabulary — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The "models" a designer reaches for: a chair or a table as one draggable prefab; walls and doorways authored by the numbers a designer thinks in (ends and openings); a typed spawn; and a second map chosen by a knob — closing #39, #41, #42.

**Architecture:** Prefabs need zero engine code (recursion is proven — this session's research ran live probes: tool blueprint limbs build per instance, zero leaked limbs, faults name nested paths); the work is the library, the re-nested shipped map, and permanent fixtures. `WaveRun` is a tool composite emitting real `WaveWall` children (the zero-new-budget pattern: each child colours and occludes exactly like a hand-placed wall), authored by `from`/`to` + an openings list — one node subsumes both the endpoint story and the doorway story. `WaveSpawn` (base=Marker3D, tool) replaces the magic string with collection-by-type, fixing the yaw local-vs-global hazard the prefab library would otherwise trip. The `level_scene` knob on `UnseeingGame` falls back to level_01 when empty (mandated by 14 suite cases and the two-line main.tscn) and refuses loudly when set wrong; a small second level proves it.

## Global Constraints

### Post-rebase errata

- The original plan counted 16 registered classes, but `WaveRestorer` already
  existed and was missing from both hand-written rosters. The corrected SP3
  baseline is 17 classes; `WaveSpawn` raises it to 18 and `WaveRun` to 19.
- Openings are absolute parent-local coordinates on the selected X/Z axis,
  paired with a width; they are not offsets from `from`. Negative widths use
  their magnitude. The dominant axis wins (X on ties), endpoints normalize,
  and non-finite or zero-length runs are rejected safely.
- Reusable prefabs live under `game/scenes/props/`; SP3 includes a configured
  doorway plus `room_16x16.tscn`, rather than deferring the room library.

Every task's requirements implicitly include this section.

- **The two-layer standard (ratified, CLAUDE.md):** Law 1 — every designer-met entity is a registered Rust node class that just works when dragged in; the **new-object checklist in CLAUDE.md is the acceptance test for `WaveSpawn` and `WaveRun`** (tool, named-child idempotence, censused, warnings pair, knob hints + /// docs, icon + manifest count, BOTH rosters, boot-pattern entry for any class-style opening, injected-never-self-wired, blob coverage if stateful, wasm-safe). Law 2 — logic in Rust; GDScript tests-only.
- **Perception laws** unchanged: axis-aligned walls; one outline per object; id clearance via the six-slot colouring; the capacity law (2×sources + solids ≤ 6 per touching cluster).
- **Injection order is law:** inject BEFORE add_child everywhere; emitted/instanced children are ownerless (never `set_owner`); Ctrl+D ghosts cleared BY NAME and `.free()`d immediately.
- **Strict TDD** (fail-first, hand-derived literals, mutation checks); count discipline (baseline **334 cargo / 276 gdUnit cases / 31 suites**; predict-and-match every delta; `--import` first).
- **Boot-gate contract:** class-style openings are literals + pattern entries in the same commit; prefer composing complaints in `level_plan.rs` relayed by WaveLevel ("WaveLevel: " is already covered). The dual-channel law (store always, print at runtime only) applies to every new fault.
- **Commits:** small, green, narrative, NO attribution. Formatters/analyzers before every commit. Full `SKIP_EXPORT=1 ci/pipeline.sh` green per task. **Do not touch or push the wiki remote** — the wiki-debt file is the canvas.
- **Registration ripple for each new class:** BOTH hand-written rosters (engine_binary_test.gd + engine_census_probe.gd, 17 names at the corrected baseline), `[icons]` + SVG + sidecar + icon_manifest_test's **exactly-N** function (name carries the count — rename it), knob_hint_test for new hinted knobs, `nodes/mod.rs` alphabetical.
- **Scene-file hygiene:** committed `.tscn` files ship in every export (`export_filter="all_resources"`) — keep prefabs lean; Godot 4.7 mints random `unique_id=` per node on editor save (expect diff noise; never build on those ids).

## Decisions Locked by Research

1. **WaveRun subsumes endpoint walls** — no `from`/`to` knobs on WaveWall itself (avoids the knob-vs-transform reconciliation on a class seven tests pin; a run with zero openings IS an endpoint wall). WaveWall's contract is untouched.
2. **WaveRun knobs are parent-local XZ** (`from: Vector2`, `to: Vector2` — prefab instances compose; issue #42's own type choice) + `openings: PackedVector2Array` (each entry = offset-along-run, width). A non-axis-aligned from→to folds to the nearest axis with a named warning (the SignFold/drop_scale doctrine: knob wins, geometry normalized, warn by name). Emitted children are real `WaveWall`s named `"RunSeg1"…"RunSegN"`, cleared by name on every ready (the Ctrl+D law — duplicates carry ghost children referencing the ORIGINAL's meshes; free, never adopt).
3. **WaveSpawn retires the name law completely** (`SPAWN_NAME`/`spawn_name`/`SpawnName` deleted; "replacing" per spec): collect by `try_cast::<WaveSpawn>`; `choose_spawn` becomes candidate-count law (0 → complaint + origin fallback KEPT — the hero must wake somewhere; ≥2 → complaint naming losers by path; first-in-walk-order wins). The Numbered arm dies (a duplicated node is a real second candidate now). **Yaw derives from the GLOBAL basis** — fixing the nested-prefab wrong-facing hazard in the same stroke, pinned by a spawn-under-rotated-prefab test.
4. **Level knob:** `#[export] level_scene: Option<Gd<PackedScene>>` (auto RESOURCE_TYPE hint); empty → level_01 fallback (zero suite churn); set-but-not-a-WaveLevel → loud `"UnseeingGame: "` refusal naming the designer's path; inject-before-add preserved. The "launch from the editor" answer is the recipe (duplicate main.tscn → point the knob → Run Custom Scene), documented, plus a shipped small second level as living proof.
5. **level_02.tscn is small by law:** 16×16 extents (diagonal ≈ 22.9 m, far under the 40 m ceiling whose shipped headroom is 0.27 m at 28×28) — one room, border walls via WaveRun, a WaveSpawn, one source, built FROM the prefab library (dogfood).

## File Structure

- `rust/src/nodes/spawn.rs` (WaveSpawn), `rust/src/nodes/run.rs` (WaveRun), `nodes/mod.rs`, `level_plan.rs` (spawn law rewrite; run maths), `level.rs` (census arm swap).
- `game/scenes/props/chair.tscn`, `props/table.tscn`; `game/scenes/level_01.tscn` (re-nested + WaveSpawn + WaveRun doorways); `game/scenes/level_02.tscn`; `game/scenes/main.tscn` (untouched — knob stays empty).
- `game/icons/wave_spawn.svg`, `wave_run.svg` + sidecars; `game/unseeing.gdextension`.
- Tests: `game/tests/level_test.gd` (spawn law rewrite + prefab census case + run cases), `game_root_test.gd` (knob case), the ~10 spawn-fixture migrations, `icon_manifest_test.gd`, `knob_hint_test.gd`, both rosters; `game/tests/probe/editor_prefab_probe.gd` + `tools/probe_editor_prefabs.sh` + `ci/pipeline.sh`.
- Docs: `game/README.md`, wiki-debt file, `docs/superpowers/plans/2026-08-12-campaign-close-checklist.md`.

---

### Task 1: WaveSpawn — the spawn becomes a class

**Files:** create `rust/src/nodes/spawn.rs`; modify `nodes/mod.rs`, `level_plan.rs` (law rewrite, name-law deletion), `level.rs` (census arm, `derive_spawn` global-basis yaw), `game/scenes/level_01.tscn` (node type swap), both rosters, `[icons]` + `wave_spawn.svg` + `icon_manifest_test.gd` (rename to nine + entry), the ~10 spawn fixture sites (helpers `_spawn_marker` across level/map/restore/observer/data_skins/props/source/source_seam suites + the two editor probes incl. their `"SpawnPoint"` warning-needle greps), `game/README.md`'s two SpawnPoint mentions, `level.rs` module doc.
**Interfaces:** `WaveSpawn`: `#[class(tool, init, base=Marker3D)]`, no knobs beyond the transform (the marker IS the datum), no limbs (draws nothing; deliberately absent from `is_censused_child` — keep it off). `Census.spawns: Vec<Gd<WaveSpawn>>`. `choose_spawn(candidates: &[SpawnCandidate], fallback) -> SpawnVerdict` keeps its shape; `SpawnCandidate` drops `kind`. New complaint texts composed in `level_plan.rs` opening with the relayed "WaveLevel: " (zero gate edits) and naming the class (probes' warning needles update from "SpawnPoint" to the new phrasing).

- [ ] Cargo TDD first: rewrite the spawn-law tests (hand-derive the NEW complaint texts — never read them back): zero candidates → complaint + fallback; two → loser named by path; three-in-order → first wins; the name-classification tests DELETE (predict the cargo delta). Then the node/census wiring; **yaw from global basis** with a cargo-reachable pure check if any (the basis→heading extraction can be pure) plus the gdUnit nested-prefab-yaw test in Task 2's fixture (note the cross-task hand-off).
- [ ] Migrate every fixture in ONE commit with the law change (a split leaves suites spawnless and asserting fallback complaints). gdUnit: level_test's three spawn cases re-pinned to the new texts; probes' needles updated.
- [ ] Mutations: (a) collect arm reverted to Marker3D → the typed-collection test fails; (b) yaw back to local → Task 2's nested test fails (defer evidence to T2, state it); (c) delete the duplicate complaint → two-candidate test fails. Full pipeline; counts predicted/matched. Commit.

### Task 2: The prefab library

**Files:** create `game/scenes/props/chair.tscn`, `props/table.tscn`; modify `game/scenes/level_01.tscn` (Chair + RadioChair → two chair.tscn instances keeping their node names; the five-prop table → table.tscn instance; keep Fan/Radio/Cat names); create `game/tests/probe/editor_prefab_probe.gd` + `tools/probe_editor_prefabs.sh`; modify `ci/pipeline.sh` (wire after the level probe), `game/tests/level_test.gd` (census-through-instance case + spawn-under-rotated-prefab yaw case).
**Interfaces:** prefab roots are plain `Node3D` (not censused — recursion passes through). The research's measured probes (scratchpad templates, NOT committed — rewrite as repo fixtures) prove: per-instance tool limbs, zero leaked limbs on re-pack, path-named faults.

- [ ] gdUnit first: census-through-instance (a code-built level instancing chair.tscn twice: censused solids counted through instances, ids separated, zero faults; hand-derive the chair's box arithmetic) — RED before the prefab files exist (load fails), GREEN after. The spawn-yaw case: WaveSpawn under a π/2-rotated Node3D → `spawn_yaw()` reflects the GLOBAL heading (hand-derive: local 0 + π/2 parent = π/2) — this is Task 1's mutation (b) evidence.
- [ ] The editor probe (fourth in the family): instanced prefab in `-e` mode builds tool blueprint limbs per instance; re-pack leaks nothing; exact check-count assertion in the runner (the SP1 hardening pattern). Wire into the pipeline before SKIP_EXPORT.
- [ ] Re-nest level_01 (editor-save diff noise from `unique_id=` minting is expected — commit message names it); full suite green (census retirement means nothing pins the census — the muffle/tap laws still hold because geometry is unchanged).
- [ ] Mutation: break the chair.tscn root type to a censused class → the census-through-instance count assertion fails (the prefab-root-is-plain law). Full pipeline. Commit.

### Task 3: WaveRun — ends and openings

**Files:** create `rust/src/nodes/run.rs`; modify `nodes/mod.rs`, `level_plan.rs` (pure run→segments maths: `run_segments(from: Vector2, to: Vector2, openings: &[(f64, f64)]) -> Vec<RunSeg { center, length, vertical }>` + axis-fold with warning text), `game/scenes/level_01.tscn` (DividerNorth/South → one WaveRun with the 4.4 m opening; PartyEast pair → one WaveRun with the 3.0 m opening — the issue's own example; hand-derive both from the research's design numbers: run x=6.4 z 0.6→19.4 opening at 8.0 width 4.4; run x=19.4 z 0.6→19.4 opening at 10.0 width 3.0), both rosters (→18), `[icons]` + `wave_run.svg` + manifest test (→ten), `knob_hint_test.gd` (from/to/openings hints), `level_test.gd` (run cases).
**Interfaces:** `WaveRun`: `#[class(tool, init, base=Node3D)]`; knobs `from: Vector2`, `to: Vector2` (parent-local XZ, `#[export]` with setters that live re-emit), `openings: PackedVector2Array`; ready() clears children named `RunSeg*` then emits `WaveWall`s (each snaps itself; the run pre-folds a diagonal from→to to the nearest axis with a literal-opening warning — decide the opening word and its gate entry: prefer a `godot_warn!` whose text is composed in level_plan and opens "WaveLevel: "? No — the run is not the level; give WaveRun its own literal "WaveRun: " opening + `|ERROR: WaveRun` pattern entry + gate must_catch line, in the same commit). Emitted walls flow through census→segments→wall_budget exactly like hand-placed ones (K+1 segments per K openings — the budget story unchanged).

- [ ] Cargo TDD first on the pure maths (hand-derived: the Divider pair's design numbers → two segments with centres 4.3/15.9 lengths 7.4/7.0 — the EXACT derived numbers the shipped map hand-computed, now derived by code; opening clamped to the run; zero-opening run = one segment; diagonal fold warning). Then the node: emit/clear idempotence (Ctrl+D law), live re-emit on knob change.
- [ ] gdUnit: a run-built room occludes what it draws (reuse the drawn-centerline law idiom); the doorway gap admits sound (muffle through the opening < through the wall — hand-derive on a fixture); Ctrl+D duplicate does not double geometry.
- [ ] level_01 conversion: the two shipped doorway pairs become runs; wall_segments count UNCHANGED (19 — two pairs become 2 runs emitting the same 4 segments); the whole suite green.
- [ ] Mutations: (a) skip the clear on ready → duplicate test fails; (b) off-by-one in opening arithmetic → the Divider-derivation cargo test fails. Full pipeline. Commit.

### Task 4: The level knob and the second map

**Files:** modify `rust/src/nodes/game.rs` (the knob + fallback + refusal), `game/tests/game_root_test.gd` (knob fixture case), create `game/scenes/level_02.tscn` (16×16: WaveRun border with one doorway, a WaveSpawn, one SoundFan, a chair instance — every SP3 vocabulary item dogfooded), `game/README.md` (the second-map recipe: duplicate main.tscn → point the knob → Run Custom Scene; plus the updated authoring steps for prefabs/runs/spawn).
**Interfaces:** `#[export] level_scene: Option<Gd<PackedScene>>` on UnseeingGame; `None` → the existing level_01 hard-load path verbatim; `Some(scene)` → `try_instantiate_as::<WaveLevel>` or `godot_error!("UnseeingGame: {path} did not instantiate as a WaveLevel — check the scene's root type")` + return (the literal opening is already patterned). Blob/restore compose automatically (`get_scene_file_path` — designed refusal on cross-level blobs stands).

- [ ] gdUnit first: knob case in game_root_test (set `level_scene = load("res://scenes/level_02.tscn")` between `new()` and `add_child` — exports settable pre-tree; assert the booted level's extents == (16,16) and wall_segments non-empty and observer snapshot healthy); a wrong-scene case (point at a plain-Node3D packed scene → the refusal fires via push_error monitor). RED (no knob property) → implement → GREEN.
- [ ] level_02.tscn authored (headless-built + editor-saved or hand-written — hand-derive every law it must satisfy: diagonal 22.9 < 40, spawn unique, seams colour, budget silent); boot it once headless via a throwaway harness invocation to prove silence (the boot-check pattern with the knob-set scene — state the command).
- [ ] Mutation: make the fallback path skip inject → the existing game_root wiring tests fail (proves the knob refactor didn't fork the injection order). Full pipeline. Commit.

### Task 5: The paper trail and the campaign-close checklist

**Files:** wiki-debt file (SP3 section: prefab doctrine, WaveRun, WaveSpawn, the knob, level_02 — file:line verified; plus the "Mechanics — Level and Objects" page updates for the new vocabulary); `game/README.md` final coherence read; create `docs/superpowers/plans/2026-08-12-campaign-close-checklist.md`: the merge-time sequence (finishing-a-development-branch menu → wiki push incl. revived 9778a00 draft + the Adding-an-Object page + all four SP sections → deploy per CLAUDE.md's merge-then-deploy law → issue closures with per-issue evidence links: #16 #22 #30 #31 #32 #33 #34 #35 #36 #38-scoped #39 #41 #42 #44 #45) and the **single consolidated human editor session checklist** (SP1: triangle appears/clears on drag, blueprint fan/radio/cat, icons in Create Node; SP2: a starved pile shows the source's triangle; SP3: drag a chair prefab in, stretch a WaveRun opening, point the knob at level_02 and Run Custom Scene).

- [ ] Write all three; verify citations; full `SKIP_EXPORT=1 ci/pipeline.sh` as SP3's closing certification with the final count reconciliation. Commit.

---

## Self-Review

1. **Spec coverage:** prefabs (#41 — T2), endpoint walls/doorways (#42 — T3), typed spawn (T1), level knob + editor launch (#39 — T4), write-back (T5). The spec's rooms/*.tscn stretch is deliberately folded into level_02's dogfood rather than a separate room library — the vocabulary is proven; the library grows by use (recorded as a scope note, not silent).
2. **Placeholders:** none; open micro-decisions (WaveRun's warning opening word, level_02 authoring route) are named with their constraint and decided in-task.
3. **Type consistency:** `Census.spawns: Vec<Gd<WaveSpawn>>` (T1) consumed by T2's yaw fixture; `run_segments`/`RunSeg` (T3) consumed by its own node; the knob (T4) consumed by game_root's new case; roster/icon counts ripple 17→18→19 and eight→nine→ten across T1/T3 in order.

## Post-rebase supersession (2026-08-13)

SP3 is implemented; this plan remains a frozen pre-implementation brief. The
following corrections control any later review:

- `openings` entries are `(absolute start coordinate on the selected
  parent-local axis, width)`, never offsets from `from`. In Godot's displayed
  `Vector2`, the second component named `y` maps to the **parent's local Z
  coordinate**, not world Z.
- Generated `RunSeg1…N` names are readable diagnostic paths, not deletion
  authority. Rebuild cleanup requires a `WaveWall` with the private generated
  metadata under a typed `WaveRun` parent.
- The shipped library includes separate plain-root chair, table, doorway, and
  `room_16x16` scenes; it was not merely folded into level 02. A raw
  `WaveLevel` is not a playable F6 target. Use **Run Current Scene** from a
  configured `UnseeingGame` runner; never call that action “Run Custom Scene.”
- The six-slot/source-colouring mechanics and their mutations were superseded
  by the `dfbb69a` superface architecture. World solids receive per-face
  labels; sources keep semantic roles but take per-instance numeric labels from
  the shared separation graph; creatures keep fixed numeric roles. `AGENTS.md`
  owns the current new-object checklist.
- The body's promise that “WaveWall's contract is untouched” did not survive
  live-editor acceptance. `WaveWall` is now a designer-facing `Node3D` datum
  with explicit collision properties/signals and ownerless private physics
  limbs. That narrow contract lets paint, occlusion, mesh, and collision share
  one exact canonical frame under oblique prefab ancestors without exposing a
  dummy `StaticBody3D` RID or inherited body state the datum cannot honor.
- WaveRun's equivalent setters may replace a RunSeg with byte-identical
  geometry. The editor scene signature therefore folds each censused Godot
  instance identity: a new generation forces one repaint and refreshes retained
  level-relative wall paths before the following idle frame. WaveLevel stages
  live walls before derivation, while runtime walls take one immutable snapshot
  at ready so retained paint and occlusion cannot drift.
- Historical task deltas are not closeout expectations. The source-role
  live-wall checkpoint is 419 Cargo tests and 327 gdUnit cases in 31 suites,
  with 19 classes and ten icons. The real editor-level probe passes 29 live
  checks plus its runtime check; closeout must recompute final totals.
