# Editor Authoring SP2 — Any Edit Ships — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A designer's legitimate content edit never reddens the deploy gate; two sound sources pushed together — even two of the same class — draw their seam; the nesting-inflation and courtyard blind spots are closed.

**Architecture:** The ~29 shipped-census pins retire into level-agnostic laws with non-vacuity guards, on the fixture pattern the issue campaign established. Sources stop being Fixed palette anchors and become colourable nodes — two role nodes per source sharing its swept union box — colouring from a `WORLD_OIDS` grown to six slots by the 0.05 the retired radio-case constant frees. The seam law (`explain_oids` violations empty + zero starvation) becomes a red gate only after the recolouring lands (a design-review sequencing requirement). `mesh_world_box` stops unioning censused children; the pack-range budget measures the slab-inclusive extent.

**Design authority:** the campaign spec §Sub-project 2, refined by a three-lens adversarial design review (colouring-correctness, renderer-perception, palette-pressure — all verdicts SOUND WITH CHANGES; every required change appears as a task item below). The decisive structural fact: two touching same-class sources defeat any fixed-constants re-plan — only per-instance colouring satisfies free placement.

## Global Constraints

Every task's requirements implicitly include this section.

- **Perception laws:** one outline per object, every seam draws; the crease is `smoothstep(0.04, 0.08, |Δ|)` of the G channel (`hearing_post.gdshader:73-74`) — full strength at Δ ≥ 0.08, and the palette rides 0.09 steps to clear 8-bit quantization (23/255 vs the 20.4/255 knee). No shader branches on any id value (verified exhaustively) — but never introduce one.
- **Free placement is law** (campaign spec): touching sources draw their seam; the engine makes arrangements correct rather than forbidding them. The honest ceiling: within one mutually-touching cluster, `2×sources + world solids ≤ 6` slots — beyond it, starvation is LOUD (per-node warning + red gate), and the fault text must say a source costs TWO ids.
- **Determinism:** `assign()` stays pure with integer-only ordering (degree desc, input index asc); same scene colours identically on desktop and wasm. Scene order is the tie-break; never float keys.
- **Platforms:** arch-independent; `gl_compatibility`/RGBA8 is the norm everywhere — quantization arithmetic above is binding.
- **Strict TDD** (fail-first, hand-derived literals at f32 precision where Vector components narrow — the no-mirror-assertion rule names propagation maths as the bug's home; mutation check per task). Count discipline: retiring assertions must predict the exact suite/case delta and match it (`--import` first; baselines at b920f07: **315 cargo / 269 gdUnit cases / 30 suites**).
- **Boot-gate contract:** new `godot_error!`/`godot_warn!` class-style openings are literals with pattern entries in the same commit (`ci/boot_error_pattern.sh:38`; `ERROR: WaveLevel` already covered — prefer composing new level faults in `level_plan.rs` relayed by WaveLevel). Severity is load-bearing: `Warn` does NOT redden the boot gate; the silence gates (`level_test.gd:641,:730,:771`) mean every always-on message reddens the suite — new budgets stay silent on a healthy level.
- **The say/fault dual-channel** (SP1): every fault is stored in `level_faults`/`node_faults` in both modes and printed only at runtime; editor faults reach `get_configuration_warnings`.
- **Commits:** small, green, narrative house style; **no attribution of any kind**. Formatters/analyzers before every commit. Full `SKIP_EXPORT=1 ci/pipeline.sh` green before claiming any task done.
- **assign_oids ↔ oid_census symmetry is law** (`level.rs:1147-1153` pins it): any change to how sources take ids edits BOTH the colouring input and the census in lockstep.
- **Scope guards:** cat 0.7 / hero 0.82/0.96 stay fixed non-anchored (unchanged); prefabs/doorways/level knob are SP3; `pulse_pool.rs` `REFUSAL_MESSAGE` ("Pulses.emit:") may be renamed ONLY in a task that already touches its pins.

## File Structure

- `rust/src/oid_palette.rs` — 6-slot palette facts + spacing-law test (T1); source-role colouring tests (T4).
- `game/tests/map_test.gd`, `wiring_test.gd`, `level_test.gd`, `observer_test.gd`, `data_skins_test.gd`, `restore_test.gd`, `game_root_test.gd` — census retirement (T2, T3).
- `rust/src/nodes/source.rs`, `fan.rs`, `radio.rs`, `level.rs`, `level_plan.rs` — source colouring + two-domain starved mapping + warnings forwarders (T4).
- `game/tests/source_seam_test.gd` (new) — the seam law red-capable (T5).
- `rust/src/level_plan.rs` + `rust/src/nodes/level.rs` + `game/tests/level_test.gd` — buried-in-wall runtime heir (T6).
- `rust/src/nodes/level.rs` (`mesh_world_box`) + fixtures (T7).
- `rust/src/level_plan.rs` (`map_diagonal`/`pack_range_budget`) + `game/tests/shader_contract_test.gd` (T8).
- Docs: wiki-debt file, `oid_palette.rs` budget prose, campaign spec check (T9).

---

### Task 1: The palette grows a sixth slot and pins its spacing law

**Files:** Modify `rust/src/oid_palette.rs` (WORLD_OIDS home is `level.rs:89` — move or extend there; the budget prose at `oid_palette.rs:41-79`), `rust/src/nodes/level.rs:89`.
**Interfaces:** `WORLD_OIDS` becomes `[0.05, 0.25, 0.34, 0.43, 0.52, 0.61]` (0.05 freed by the radio-case retirement in T4; safe: 0.10 from floor 0.15, 0.20 from 0.25, far from cat/hero/ceiling — panel-verified). Produces the spacing-law test T4 relies on.

- [ ] Cargo tests first (red): (a) adjacent sorted palette entries ≥ 0.09 apart (hand-derived: the quantization-margin law — 23/255 vs the 20.4/255 knee; assert on the sorted array with literals, naming the break: "a palette edit that spends the 8-bit margin dims seams on every shipped platform"); (b) every entry clears floor 0.15 and cat 0.70 by ≥ MIN_SEP through `separated()` (the existing `the_shipped_palette_separates_its_own_entries` at `oid_palette.rs:471-480` stays in force — extend, don't replace). Watch (a) fail against the 5-slot palette? No — it passes trivially; the RED here is the new 0.05 entry's absence from the palette-clears-floor test (0.05 vs 0.15 = 0.10 ✓ — write the test to enumerate all SIX expected entries so it fails on the 5-entry palette). Implement the sixth slot; green.
- [ ] Update the budget prose (`oid_palette.rs:41-79`): 0.05 moves from "radio case" to the world palette; state the capacity law verbatim: "within one mutually-touching cluster, two ids per source plus one per world solid must fit in six slots; the seventh starves, loudly". Note the two historic sub-MIN_SEP fixed pairs (0.63/0.70, 0.90/0.96) die with T4's constant retirement / stay documented respectively.
- [ ] Shipped map still colours 0-starved (`observer_test.gd:307` and `map_test` seam law stay green — the palette only grew). Mutation: shrink the palette back to 5 — the six-entry test fails. Full pipeline. Commit.

### Task 2: Census retirement, wave A — map_test.gd

**Files:** Modify `game/tests/map_test.gd` only.
**Interfaces:** none new. The research inventory (each item verified at b920f07) is the work order:

- [ ] RETIRE outright: `test_shipped_level_matches_validated_design` (:143-158, 9 pins, pure census) and `test_shipped_prop_census` (:339-343, 72/27/7). Predict the case delta.
- [ ] DE-PIN, keep the law: seam law :389 `is_equal(125)` → non-vacuity (`is_greater(100)`? NO — any magic number is census; use `is_not_empty()` plus the existing distinct-ids-fewer-than-boxes counter-law at :415-421 as the anti-vacuity pair — the pattern `observer_test.gd:313` pins); buried-prop :561 same treatment; wall-table :468 drop the ==19, keep the fault-walk; silence-gate rider `level_test.gd:646` is wave B.
- [ ] LAW-SHAPE the tap test (:116-126): replace the 6.25/6.4 literals with the law — derive the expected face from `level.demo_tap()`'s own contract (tap sits ON a wall face between spawn and nearest source: assert tap lies on SOME wall segment's face plane within half-thickness, normal points spawn-ward) — careful: derive expectations from level_plan GEOMETRY (`wall_segments()`), never from `demo_tap()` itself (mirror-assertion trap).
- [ ] KEEP UNCHANGED: :103-108, :129-138, :171-193, :278-308, :415-421, :491-513, :620-656, :666-687 (already level-agnostic). Cat pins :317/:322 → keep the injection law (:319-320), drop count+position literals; muffle-literal riders :206-233/:255-259 → the one-wall muffle law already lives at :171-193 on fixtures; keep the intruder/impostor mechanics with `is_greater_equal(3)`-style bounds instead of ==3, and `_source_named` everywhere a positional index remains.
- [ ] Predict counts before running (state the arithmetic in the report), `--import`, full suite, exact match. Mutation: re-add one census literal? No — the meaningful mutation: empty `_painted_boxes` input (comment the recursion) — the seam law's non-vacuity guard must fail. Full pipeline. Commit.

### Task 3: Census retirement, wave B — the other six suites

**Files:** Modify `wiring_test.gd` (:52, :69 → drop ==19/==2, keep read-back laws), `level_test.gd` (:646 drop the ==19 rider, keep the silence gate), `observer_test.gd` (positional `sources()[0]`+name at :289-291 → `_source_named` pattern; knob-census FAN_* mirrors :29-42 → read the knobs off the live node; :319/:339/:383/:401/:444/:455-464/:506 → law-shape or fixture-move per the research inventory; the explain_ray shipped-coordinate cases :455-464 → move onto a code-built one-wall fixture preserving the crossings law), `data_skins_test.gd` (:178-204 five pinned sight lines → one-wall fixture laws for GRAZE_EPS and birth-wall asymmetry; keep the suite's Rust-not-GPU trap comment), `restore_test.gd` (:12 `cats()[0]` → guard emptiness with a named skip/fail sentence, keep the law), `game_root_test.gd` (:245-246 positional+name → `_source_named`-equivalent against `game.level`; :283-289 u_count==2 → law-shape: count sources whose `next_emit()` precedes now and assert u_count equals THAT — derived from live source clockwork, not a literal; preserve the single-`process_frame` await shape its own warning mandates).
**Interfaces:** none new.

- [ ] Suite-by-suite: predict delta, edit, single-suite green, then full run + pipeline. The observer naming pins ("Fan @0.33") get their final form in T4 — here mark them clearly if they must temporarily stay (they pin CURRENT constants, which are still live until T4; retiring them here would blind T4's transition — keep them with a `# retired by the source-colouring task` note and a TODO the T4 brief owns). Mutation: one law per suite flipped (e.g. wiring read-back count made unequal) must fail. Commit per suite or as one wave — implementer's judgment under the one-behaviour rule.

### Task 4: Sources enter the colouring

**Files:** Modify `rust/src/nodes/source.rs` (trait + SourceRig roles), `fan.rs`, `radio.rs` (constants retired → role defaults; warnings forwarders), `rust/src/nodes/level.rs` (assign_oids two-domain, census symmetry), `rust/src/level_plan.rs` (two-domain starved mapping), `rust/src/oid_palette.rs` (tests), `game/tests/observer_test.gd` (naming pins law-shaped).
**Interfaces (panel-mandated shape):**
- `SoundSource` trait: `fn oids(&self) -> &'static [f64]` becomes `fn role_count(&self) -> usize` + `fn set_role_oids(&mut self, oids: &[f64])` (mirroring `WaveSolid::set_oid`); `SourceRig::limb` gains a `role: usize` parameter recorded per limb so `set_role_oids` retags `u_oid` per role (`set_instance_shader_parameter`, the exact call `limb()` already makes).
- Out-of-level defaults (prefab preview): roles default to the first `role_count` palette slots (`WORLD_OIDS[0..n]`) at build — two DISTINCT ids in any preview; the colouring overwrites them in a level. Document at the trait.
- `assign_oids`: sources contribute `role_count` colourable nodes each, all sharing the swept union box (`mesh_world_box` + `grown_flat(sweep_margin)` — unchanged box law), appended AFTER solids in census order; slabs remain the only Fixed anchors. The positional output maps through a TWO-DOMAIN index (`enum ColouredNode { Solid(usize), SourceRole { source: usize, role: usize } }` in `level_plan.rs`, pure, cargo-tested — replacing raw `painted[slot]`): starved faults name the right node in both domains (a starved source role's fault text says the source costs TWO ids and moving IT is the fix — panel-measured misattribution otherwise). A box-less source (rig unbuilt) is skipped WITHOUT desynchronising later indices (the map handles the gap — panel-required).
- `SoundFan`/`SoundRadio` gain the `get_configuration_warnings` + inherent `#[func]` forwarder pair (the SP1 solid pattern — `warnings_from_level`), so starved sources wear the triangle.
- Constants `FAN_OID`/`FAN_BLADE_OID`/`RADIO_CASE_OID`/`RADIO_FACE_OID` and their `OIDS` arrays are deleted; `oid_census` reads ids BACK OFF limbs (symmetry law) and keeps one entry per role with the `"{name} @{oid}"` naming now placement-dependent — `observer_test` pins become law-shaped (names CONTAIN "Fan @", ids pairwise separated) in the same commit.
- **Sequencing (panel-mandated):** this task does NOT make the seam law a red gate — T5 does, after this lands. The editor watch note (source aabb folded as None — correct while source geometry is class-constant) goes into the code doc here.

- [ ] Cargo TDD first in `oid_palette.rs`/`level_plan.rs`: identical-box role pairs get distinct slots (hand-walked greedy); the K6 corner case (fan+radio+two walls) colours 0-starved on the six-slot palette; K7 starves deterministically with the right two-domain indices; the two-domain map's gap-skipping (a missing source shifts nothing). Then the node wiring; then the gdUnit updates.
- [ ] Runtime regression: full suite green (fan/radio limb tests read ids off limbs, not constants — `map_test._source_oids` already does; `_limb_box`'s hardcoded 0.45 fan margin is pre-existing, leave unless touched). Shipped map: 0 starved, seam law green, silence gates hold.
- [ ] Mutations: (a) make both roles share one slot (skip the role node append) — the role-separation cargo test fails; (b) break the gap-skip — the two-domain test fails. Full pipeline. Commit (this is the campaign's physics-review-gated engine change — the commit body carries the design's one-paragraph rationale incl. the same-class-sources argument).

### Task 5: The seam law goes red-capable

**Files:** Create `game/tests/source_seam_test.gd`; modify `ci/boot_error_pattern.sh` only if a new opening appears (prefer none — WaveLevel relays).
**Interfaces:** consumes T4. The law, in the observer's census-free instrument (`explain_oids`): pairs non-empty, violations EMPTY, starved 0 — on ANY level.

- [ ] Fixture tests first (red only until T4 landed — this task runs after): (a) two touching SoundFans (same class — the structural case) draw their seam: build a code-built level with two fans 0.3 m apart + spawn, derive, `explain_oids` violations empty AND the two fans' role ids pairwise separated (read off limbs); (b) fan touching radio: same; (c) THE STARVING PILE CONTAINING A SOURCE (panel-required): a K7 cluster (e.g. fan+radio+three mutually-touching walls in a corner) → starved > 0, the gate red (level faults non-empty), the starved fault names a node and says a source costs two ids; assert the own-roles melt is CAUGHT (violations non-empty when starved) — the fallback hands both roles one slot and the law must see it; (d) the shipped map stays silent (the existing `observer_test.gd:307` case is the carrier — verify it still holds and reference it, don't duplicate).
- [ ] Wire nothing new into ci — the suite runs in the gdUnit stage; the boot gate hears starvation via the existing `ERROR: WaveLevel` relay (verify with a planted starving level in the test, not by trusting).
- [ ] Mutation: revert T4's role-append (both roles one node) — (a) must fail. Full pipeline. Commit. Issues #16 and #36 close at campaign merge with this task's evidence.

### Task 6: The buried-prop law gets a runtime heir

**Files:** Modify `rust/src/level_plan.rs` (pure `buried_in_wall(walls, solids) -> Vec<PlacementFault>`), `rust/src/nodes/level.rs` (report in derive, both channels), `game/tests/level_test.gd` (fixture case + silence-gate coverage), `game/tests/map_test.gd` (the CI case keeps its de-pinned walk as belt-and-braces).
**Interfaces:** mirrors `unfloored`/`sunken` exactly (PLACEMENT_EPS 0.001, path+text faults, per-node warnings via the existing `faults_for`). Research fact driving this: buried-in-wall has NO runtime heir — the de-pinned CI walk was its only guard.

- [ ] Cargo TDD (fixture: prop 0.3 deep in a wall → one fault naming the prop's path; touching-at-epsilon → silent); relay + editor fault; gdUnit fixture case pinning the message text once (two-file text law — keep cargo and gdUnit literals byte-identical); shipped-map silence holds. Mutation: flip the depth comparison — both cargo and gdUnit fail. Full pipeline. Commit.

### Task 7: mesh_world_box stops at censused children

**Files:** Modify `rust/src/nodes/level.rs` (`mesh_world_box` ~:1339 + doc, `placed_solids` doc :748-752), `game/tests/level_test.gd` (new fixture cases).
**Interfaces:** recursion stops at any CHILD recognisable by `collect()`'s vocabulary (try_dynify WaveSolid / SoundSource, try_cast WaveCat) — children only, NEVER the root argument (sources/cats are measured by direct calls on their own nodes — refusing the root drops them from colouring and census, panel-verified trap); plain nodes (limbs, grouping Node3Ds, markers) still recurse.

- [ ] gdUnit fixture first (red): prop nested under a crate — the crate's colouring box no longer unions the child (assert via `explain_oids` boxes or a starved-pressure differential: the panel's measured probe — nested prop forced the parent off 0.25 pre-fix; post-fix a control pair confirms no over-separation), and the nested prop keeps its own box/id/placement fault (blame moves to the child — update `placed_solids`' doc promise in the same commit). Cargo can't reach mesh_world_box (Godot-tree-bound) — the gdUnit fixture is the instrument; say so in the report.
- [ ] All six callers audited by test: floor_box (slab child meshes are plain — unchanged), source/cat direct calls (roots — unchanged), census symmetry. Mutation: apply the stop to the root too — source-measuring tests fail. Full pipeline. Commit.

### Task 8: The pack range measures the slab

**Files:** Modify `rust/src/level_plan.rs` (`map_diagonal` → slab-inclusive; message text + cargo literals :1868-1995), `rust/src/nodes/level.rs` (`report_pack_range` :886-892 feeds the slab boxes — `floor_box()` + the lid, world-space, panel/coordinate-trap: measure off slab world boxes, never the raw extents knob), `game/tests/shader_contract_test.gd` (:152-162 moves to the same measure in the SAME commit, keeping its independent re-derivation — it must not become a mirror of WaveLevel's arithmetic: re-derive from `level.extents` + slab geometry law, not from a new #[func]).
**Interfaces:** new measure = diagonal of the union of the two slab world boxes (hand-derived shipped value: √(28²+28²+3.2²) ≈ 39.727 m — 0.27 m headroom, still silent). The equality-refusal doc rewrites (the understatement rationale evaporates; the vd==range-packs-1.0 half stands).

- [ ] Cargo TDD: courtyard fixture (extents-sized slabs 80×80, one 6×6 walled room) → Error under the new measure (was silent — the RED this fix exists for); shipped-shape fixture stays silent at 39.73; empty-wall level now MEASURES (slabs exist without walls — the "no walls, no footprint" return dies); message literals updated (state old→new text in the report). gdUnit: shader_contract case re-derived; silence gates hold. Fix the stale `pulse_pool.gdshaderinc:12-14` "room is 20 m" comment in passing (it's the file being contract-tested). Mutation: revert to wall-centerline measure — the courtyard cargo test fails. Full pipeline. Commit.

### Task 9: The paper trail

**Files:** Modify the wiki-debt file (SP2 section: palette table with six slots + capacity law, source colouring, the two closed blind spots, the census retirement — every claim file:line), `oid_palette.rs` prose final check, campaign spec check (no errata expected — the design followed the spec's own option (b)), memory note is the controller's job not this task's.

- [ ] Write, verify every citation, commit. Full `SKIP_EXPORT=1 ci/pipeline.sh` as the sub-project's closing certification; report final counts vs predictions across T2/T3/T5 (the ledger holds each task's arithmetic).

---

## Self-Review

1. **Spec coverage:** census retirement (T2/T3 — #22), source seams with the law landing red-capable in sequence (T1/T4/T5 — #16/#36), nesting (T7 — #35), courtyard (T8 — #45), buried-prop heir (T6 — the inventory's one unguarded law). All eleven panel-required changes are task items: sixth slot + capacity doc (T1/T9), sequencing (T4/T5 split), spacing-law test (T1), two-domain mapping + gap-skip (T4), source warnings forwarders (T4), out-of-level defaults (T4), seam walk feeds sources (T5 via explain_oids), starving-pile-with-source test (T5), density-ceiling prose + fault text (T4/T9), editor-watch geometry note (T4), palette pairwise test kept (T1).
2. **Placeholders:** none — each task cites the measured inventory lines it acts on.
3. **Type consistency:** `ColouredNode` two-domain enum (T4) consumed by starved faults and tests; `role_count`/`set_role_oids` (T4) consumed by T5 fixtures; six-slot `WORLD_OIDS` (T1) consumed by T4's K6 case.
