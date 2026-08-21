# Scene Authoring Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn plain grouping nodes, nested `PackedScene` instances, and true
Godot inherited scenes into a measured, mutation-checked level-authoring
contract without changing the game laws that already compose them correctly.

**Architecture:** Five tracked test scenes express one room twice: once through
a translated, quarter-turned plain group containing an inherited room and a
nested prop scene, and once as an independently hand-flattened oracle. A new
gdUnit suite compares the live Rust-derived wall, collision, source, creature,
spawn, superface, and real mesh-label results by explicit semantic keys. The
existing editor-prefab probe adds Godot 4.7.1 `SceneState`, duplication,
disk-round-trip, generated-owner, and nested-warning checks. Unchanged
production Rust is expected to pass; a regression failure triggers systematic
debugging and a design/plan amendment before any production fix.

**Tech Stack:** Godot 4.7.1, typed tests/probes-only GDScript, GDExtension Rust
(`gdext` 0.5.4), gdUnit4, POSIX shell, repository-pinned Superpowers.

**Spec:** `docs/superpowers/specs/2026-08-21-scene-authoring-contract-design.md`

## Global Constraints

- **Scope is regression and documentation first.** Add no room/location class,
  custom `Resource`, registry, autoload, canonical snapshot API, shipped
  GDScript, or production abstraction. Run the complete new regression against
  unchanged Rust. If it is green, production stays unchanged. If it is red,
  keep the failing test, use `superpowers:systematic-debugging`, isolate one
  root cause, and amend both this plan and the approved spec before editing the
  owning production component.
- **Perception laws do not move.** Do not change propagation, visible-air
  distance cuts, geometry-based occlusion admission, source-through/detail-knee
  scope, wave speed, reveal composition, or any authored perception constant.
  Occlusion remains geometry-based, never node-class-based.
- **Labels and superfaces do not move.** `MIN_SEP = 0.08` remains owned by
  `rust/src/render/labels.rs`; the sRGB-safe label band remains `[0.15, 0.96]`
  with only the existing radio preview exception; `COPLANAR_EPS` and
  `PATCH_EPS` remain owned by `rust/src/render/superface.rs`. Same-facing,
  coplanar overlaps merge bit-for-bit; separate touching solids keep the
  required separation. Numeric palettes are not compared across independently
  coloured fixtures.
- **The ring cut remains a distance.** Nothing in this plan may turn the
  `sight::visible_air` `min()` composition into a boolean or alter its scope.
- **The two code layers remain exact.** Designer-facing content is composed
  from existing registered Rust tool nodes and plain `.tscn` scenes. Pure
  engine laws stay in engine-free Rust; registered nodes remain thin boundary
  adapters. No global state, ambient collaborator, architecture conditional,
  unsafe code, or side effect hidden in domain logic is introduced.
- **Totality and injection remain explicit.** Runtime fixtures inject materials
  and pulses before tree entry. Tests reject unknown paths, duplicate semantic
  keys, malformed mesh arrays, missing resources, and absent physics hits
  instead of indexing blindly or accepting defaults.
- **One project and every target.** `game/` remains the sole Godot project and
  the same sources continue to support macOS universal, Windows x86_64 and
  arm64, native x86_64/arm64 development, and wasm32. This plan adds no
  platform-specific behavior or technology.
- **Test integrity.** Import before gdUnit. Trust exact executed suite/case
  counts, not gdUnit's exit status alone. Poll editor conditions with bounded
  frame budgets; add no sleep. Read actual `Mesh.ARRAY_CUSTOM0` values for the
  label proof; `WaveObserver.faults` alone is not GPU evidence. Do not mirror a
  production transform or palette to manufacture an oracle.
- **TDD and mutations.** Write each behavioral check before any production
  change. A newly added characterization may correctly be green against
  unchanged production; record that honestly rather than fabricating a red.
  The deliberate mutation matrix supplies the required failure evidence.
  Mutation patches are never committed and are reversed with explicit patches,
  followed by a clean-status and green rerun.
- **Isolation.** Execute in the existing isolated worktree on branch
  `issue-65-scene-authoring-contract`; do not nest another worktree. Reviewers
  are read-only and never move its `HEAD`.
- **Commits and attribution.** Make small, self-contained, green commits, one
  behavior per commit, with an evocative narrative subject and a body stating
  the precise what/why. Repository identity is `Dmitrii Galchenko
  <dggrus@gmail.com>`. Never add `Co-Authored-By`, assistant/AI attribution,
  generated-with text, or tool credit to commits, code, tests, docs, or PRs.
  Never commit build output, exports, reports, rendered frames, `target/`,
  `.wasm`, `.pck`, or temporary `user://` files.
- **Review after every task.** After each task is green and committed, invoke
  `superpowers:requesting-code-review` with the task's base/head SHAs and the
  approved spec. Verify every finding against the code, fix accepted findings
  in a new green commit, and request a fresh review when the fix is material.
  A subagent report is not evidence; inspect the diff and rerun the gates.
- **Documentation and integration.** The tracked spec records why; the wiki
  describes what ships. Prepare the exact wiki write-back on this branch, but
  publish it only after integration so it never claims an unmerged contract is
  on `main`. Finish with `superpowers:finishing-a-development-branch`. Never
  merge or push without the user's explicit choice. A merge to `main` is the
  automatic web-deployment gate; there is no manual deploy step.

Baseline measured at the approved-spec commit is 568 Cargo tests, 32 gdUnit
suites / 354 cases, and 16 editor-prefab checks. The implementation adds no
Cargo tests, six gdUnit cases (33 / 360), and 20 editor-prefab checks (36).

---

## File Structure

- `game/tests/fixtures/scene_composition/nested_prop.tscn` (new) — plain-root
  nested prop scene carrying a named merge pair and a named separate seam.
- `game/tests/fixtures/scene_composition/base_room.tscn` (new) — plain-root
  room with a run, cross wall, fan, cat, spawn, and nested prop instance.
- `game/tests/fixtures/scene_composition/inherited_room_variant.tscn` (new) —
  true inherited scene overriding `Fan.volume` and adding `SoundRadio`.
- `game/tests/fixtures/scene_composition/composed_level.tscn` (new) —
  `WaveLevel` with a nonidentity plain ancestor and the inherited instance.
- `game/tests/fixtures/scene_composition/flat_level.tscn` (new) — independent
  hand-flattened oracle, with no grouping, nesting, or inheritance.
- `game/tests/scene_composition_test.gd` (new) — six-case runtime contract over
  retained outputs, absolute anchors, collision, superfaces, actual mesh bytes,
  and fault-free derivation.
- `game/tests/scene_composition_test.gd.uid` (new, Godot-generated) — tracked
  stable UID sidecar for the new suite; keep it after import.
- `game/tests/probe/editor_prefab_probe.gd` (modify) — inheritance state,
  owner inventory, duplicate rebuild, disk persistence, and warning-watch
  phases, while retaining all 16 existing prefab checks.
- `tools/probe_editor_prefabs.sh` (modify) — require the expanded 36-check
  editor probe; no new pipeline stage.
- `docs/opening-in-godot.md` (modify) — designer workflow for inherited room
  variants and the authored/generated ownership boundary.
- `docs/superpowers/handoffs/2026-08-21-scene-authoring-wiki-writeback.md`
  (new) — exact post-integration updates for the two affected wiki pages.
- `rust/src/nodes/level.rs`, `rust/src/nodes/run.rs`,
  `rust/src/nodes/wall.rs` — unchanged production owners used only for
  temporary, uncommitted mutation evidence. If unchanged regression is green,
  these files must have no final diff.

---

### Task 1: Land the independently authored fixture graph and census contract

**Files:**
- Create: `game/tests/fixtures/scene_composition/nested_prop.tscn`
- Create: `game/tests/fixtures/scene_composition/base_room.tscn`
- Create: `game/tests/fixtures/scene_composition/inherited_room_variant.tscn`
- Create: `game/tests/fixtures/scene_composition/composed_level.tscn`
- Create: `game/tests/fixtures/scene_composition/flat_level.tscn`
- Create: `game/tests/scene_composition_test.gd` (first two cases and shared
  inventory constants/helpers)
- Create and track after import: `game/tests/scene_composition_test.gd.uid`

**Implementer brief — mandatory:** Read `AGENTS.md`, the approved spec, this
plan's Global Constraints, and the relevant wiki pages before editing. This is
tests/content only: preserve every perception, visible-air, geometry-occlusion,
label-clearance, and superface law. Use existing Rust tool nodes under plain
Godot roots; add no production class or shipped GDScript. Keep all targets
platform-neutral. Work only in the isolated branch, make one green narrative
commit with the repository identity and no attribution, then request a fresh
read-only review.

**Interfaces:**
- Consumes: existing `WaveLevel` recursive census, world-space derivation,
  `WaveRun` generation, source/cat/spawn retention, and ownerless builders.
- Produces: one explicit fixture graph and two initial tests proving typed
  authored nodes remain present exactly once through grouping, nesting, and
  inheritance; Task 2 consumes the same semantic maps for deeper equivalence.

#### Frozen scene content

- [ ] **Step 0: Resolve the repository-pinned Godot in this task's shell.**

```sh
. "$PWD/tools/lib/engine.sh"
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
```

Require the selector to succeed and print the pinned 4.7.1 binary. Do not carry
an engine variable over from another independently dispatched task.

- [ ] **Step 1: Start with a missing-fixture regression and observe the honest red.**

Create `game/tests/scene_composition_test.gd` with the gdUnit suite header, the
two resource path strings, and the first two named cases:

```gdscript
extends GdUnitTestSuite

const COMPOSED_PATH := "res://tests/fixtures/scene_composition/composed_level.tscn"
const FLAT_PATH := "res://tests/fixtures/scene_composition/flat_level.tscn"


func test_plain_groups_do_not_hide_or_duplicate_nested_gameplay() -> void:
	assert_object(load(COMPOSED_PATH)).is_not_null()
	assert_object(load(FLAT_PATH)).is_not_null()


func test_inherited_override_and_added_radio_reach_retained_sources_once() -> void:
	assert_object(load(COMPOSED_PATH)).is_not_null()
```

Import and run only this suite:

```sh
"$GODOT_BIN" --headless --path "$PWD/game" --import >/dev/null 2>&1 || true
"$GODOT_BIN" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a res://tests/scene_composition_test.gd
```

Expected: two failures naming the absent fixture resources. This red checks the
test is discovered; do not interpret it as a production defect. Retain the
newly generated `game/tests/scene_composition_test.gd.uid` for the green commit.

- [ ] **Step 2a: Create the nested prop artifact.**

Create `nested_prop.tscn`:

```tscn
[gd_scene format=3]

[node name="NestedProp" type="Node3D"]

[node name="MergeShelf" type="WaveProp" parent="."]
size = Vector3(2, 1, 1)
position = Vector3(3, 0.5, 3)

[node name="MergeCrate" type="WaveProp" parent="."]
size = Vector3(1, 1, 0.8)
position = Vector3(3.4, 0.5, 3.1)

[node name="SeamLeft" type="WaveProp" parent="."]
size = Vector3(1, 1, 1)
position = Vector3(6, 0.5, 3)

[node name="SeamRight" type="WaveProp" parent="."]
size = Vector3(1, 1, 1)
position = Vector3(7, 0.5, 3)
```

- [ ] **Step 2b: Create the base-room artifact.**

Create `base_room.tscn`:

```tscn
[gd_scene format=3]

[ext_resource type="PackedScene" path="res://tests/fixtures/scene_composition/nested_prop.tscn" id="1_prop"]

[node name="BaseRoom" type="Node3D"]

[node name="BoundaryRun" type="WaveRun" parent="."]
from = Vector2(2, 4)
to = Vector2(8, 4)

[node name="CrossWall" type="WaveWall" parent="."]
length = 3.0
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 5, 0, 5.5)

[node name="Fan" type="SoundFan" parent="."]
position = Vector3(3, 0, 6)

[node name="Cat" type="WaveCat" parent="."]
position = Vector3(7, 0, 2)
seed = 11
roam_size = Vector2(1.5, 1.5)

[node name="Spawn" type="WaveSpawn" parent="."]
position = Vector3(3, 0, 2)

[node name="NestedProp" parent="." instance=ExtResource("1_prop")]
position = Vector3(0, 0, 6)
```

- [ ] **Step 2c: Create the true inherited variant.**

Create `inherited_room_variant.tscn`:

```tscn
[gd_scene format=3]

[ext_resource type="PackedScene" path="res://tests/fixtures/scene_composition/base_room.tscn" id="1_base"]

[node name="InheritedRoomVariant" instance=ExtResource("1_base")]

[node name="Fan" parent="." index="2"]
volume = 0.6

[node name="Radio" type="SoundRadio" parent="."]
position = Vector3(7, 0, 7)
```

- [ ] **Step 2d: Create the composed level.**

Create `composed_level.tscn`:

```tscn
[gd_scene format=3]

[ext_resource type="PackedScene" path="res://tests/fixtures/scene_composition/inherited_room_variant.tscn" id="1_room"]

[node name="ComposedLevel" type="WaveLevel"]
extents = Vector2(14, 14)

[node name="PlainGroup" type="Node3D" parent="."]
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 2, 0, 12)

[node name="InheritedRoomVariant" parent="PlainGroup" instance=ExtResource("1_room")]
```

- [ ] **Step 2e: Hand-author the independent flat oracle.**

Create `flat_level.tscn`; do not generate it from or
load values out of the composed scene:

```tscn
[gd_scene format=3]

[node name="FlatLevel" type="WaveLevel"]
extents = Vector2(14, 14)

[node name="BoundaryRun" type="WaveRun" parent="."]
from = Vector2(6, 4)
to = Vector2(6, 10)

[node name="CrossWall" type="WaveWall" parent="."]
length = 3.0
transform = Transform3D(-1, 0, 0, 0, 1, 0, 0, 0, -1, 7.5, 0, 7)

[node name="Fan" type="SoundFan" parent="."]
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 8, 0, 9)
volume = 0.6

[node name="Cat" type="WaveCat" parent="."]
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 4, 0, 5)
seed = 11
roam_size = Vector2(1.5, 1.5)

[node name="Spawn" type="WaveSpawn" parent="."]
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 4, 0, 9)

[node name="MergeShelf" type="WaveProp" parent="."]
size = Vector3(2, 1, 1)
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 11, 0.5, 9)

[node name="MergeCrate" type="WaveProp" parent="."]
size = Vector3(1, 1, 0.8)
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 11.1, 0.5, 8.6)

[node name="SeamLeft" type="WaveProp" parent="."]
size = Vector3(1, 1, 1)
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 11, 0.5, 6)

[node name="SeamRight" type="WaveProp" parent="."]
size = Vector3(1, 1, 1)
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 11, 0.5, 5)

[node name="Radio" type="SoundRadio" parent="."]
transform = Transform3D(0, 0, 1, 0, 1, 0, -1, 0, 0, 9, 0, 5)
```

The transform sign is load-bearing: the textual quarter-turn maps room
`(x, y, z)` to world `(2 + z, y, 12 - x)` and loads with yaw `PI / 2`.

- [ ] **Step 2f: Import once and reject every fixture load error.**

Run the pinned Godot import and inspect its output for missing custom classes,
bad inherited indices, or resource cycles. Continue only when all five scenes
load.

- [ ] **Step 3a: Replace path strings with the explicit semantic maps.**

Use these exact maps in the suite:

```gdscript
const COMPOSED_SCENE := preload(
	"res://tests/fixtures/scene_composition/composed_level.tscn"
)
const FLAT_SCENE := preload(
	"res://tests/fixtures/scene_composition/flat_level.tscn"
)

const COMPOSED_PATHS := {
	"group": "PlainGroup",
	"room": "PlainGroup/InheritedRoomVariant",
	"run": "PlainGroup/InheritedRoomVariant/BoundaryRun",
	"run_wall": "PlainGroup/InheritedRoomVariant/BoundaryRun/RunSeg1",
	"cross_wall": "PlainGroup/InheritedRoomVariant/CrossWall",
	"fan": "PlainGroup/InheritedRoomVariant/Fan",
	"cat": "PlainGroup/InheritedRoomVariant/Cat",
	"spawn": "PlainGroup/InheritedRoomVariant/Spawn",
	"prop_root": "PlainGroup/InheritedRoomVariant/NestedProp",
	"merge_shelf": "PlainGroup/InheritedRoomVariant/NestedProp/MergeShelf",
	"merge_crate": "PlainGroup/InheritedRoomVariant/NestedProp/MergeCrate",
	"seam_left": "PlainGroup/InheritedRoomVariant/NestedProp/SeamLeft",
	"seam_right": "PlainGroup/InheritedRoomVariant/NestedProp/SeamRight",
	"radio": "PlainGroup/InheritedRoomVariant/Radio",
}

const FLAT_PATHS := {
	"run": "BoundaryRun",
	"run_wall": "BoundaryRun/RunSeg1",
	"cross_wall": "CrossWall",
	"fan": "Fan",
	"cat": "Cat",
	"spawn": "Spawn",
	"merge_shelf": "MergeShelf",
	"merge_crate": "MergeCrate",
	"seam_left": "SeamLeft",
	"seam_right": "SeamRight",
	"radio": "Radio",
}
```

Implement `_enter_fixture(scene)` with inject-before-add.

- [ ] **Step 3b: Implement total retained-path normalization helpers.**

Implement `_wall_rows`, `_retained_transforms`, and `_assert_live_inventory` as
total helpers: every
unknown retained path and every duplicate semantic key calls `fail()` and
returns an empty/invalid result that makes the caller fail too.

The inventory oracle is:

| Semantic key | Composed path | Flat path | Classification / owner | Live | Retained | Saved state |
|---|---|---|---|---:|---:|---|
| level | `.` | `.` | authored fixture root | 1 | root | allowed |
| group | `PlainGroup` | — | authored plain `Node3D` | 1 | 0 | allowed |
| room | `PlainGroup/InheritedRoomVariant` | — | authored inherited instance | 1 | 0 | allowed as instance |
| run | `…/BoundaryRun` | `BoundaryRun` | authored `WaveRun` | 1 | 0 directly | allowed |
| run wall | `…/BoundaryRun/RunSeg1` | `BoundaryRun/RunSeg1` | generated by `WaveRun`, ownerless | 1 | wall/solid/paint: 1 | forbidden |
| cross wall | `…/CrossWall` | `CrossWall` | authored `WaveWall` | 1 | wall/solid/paint: 1 | allowed |
| fan | `…/Fan` | `Fan` | authored source, base + inherited override | 1 | source: 1 | allowed |
| cat | `…/Cat` | `Cat` | authored creature | 1 | cat: 1 | allowed |
| spawn | `…/Spawn` | `Spawn` | authored drawless datum | 1 | selected spawn: 1 | allowed |
| nested root | `…/NestedProp` | — | authored nested instance | 1 | 0 | allowed as instance |
| merge shelf | `…/NestedProp/MergeShelf` | `MergeShelf` | authored `WaveProp` | 1 | solid/paint/clarity: 1 | allowed |
| merge crate | `…/NestedProp/MergeCrate` | `MergeCrate` | authored `WaveProp` | 1 | solid/paint/clarity: 1 | allowed |
| seam left | `…/NestedProp/SeamLeft` | `SeamLeft` | authored `WaveProp` | 1 | solid/paint/clarity: 1 | allowed |
| seam right | `…/NestedProp/SeamRight` | `SeamRight` | authored `WaveProp` | 1 | solid/paint/clarity: 1 | allowed |
| radio | `…/Radio` | `Radio` | authored inherited addition | 1 | source: 1 | allowed |
| floor | `WaveFloor` | `WaveFloor` | generated by `WaveLevel`, ownerless | 1 | slab/paint: 1 | forbidden |
| ceiling | `WaveCeiling` | `WaveCeiling` | generated by `WaveLevel`, ownerless | 1 | slab/paint: 1 | forbidden |

- [ ] **Step 3c: Replace the first smoke case with the once-only census.**

For both fixtures assert exact recursive class counts: one `WaveRun`, two
`WaveWall` nodes including `RunSeg1`, four `WaveProp`, one `SoundFan`, one
`SoundRadio`, one `WaveCat`, and one `WaveSpawn`. Assert every authored
non-root path has a non-null owner, `RunSeg1.owner == null`, every explicit
path resolves to its expected class, and the grouping roots never enter walls,
sources, cats, spawn output, or observer membership. Assert the retained wall,
source, cat, and spawn semantic keys each occur once.

For observer membership extend the exact reverse-path map with
`"Floor": "floor"` and `"Ceiling": "ceiling"`; require the union of class
members to contain exactly those two slab keys plus the six gameplay-solid
keys. A plain group, room root, nested root, source, creature, or drawless spawn
must never appear as a painted world solid.

- [ ] **Step 3d: Replace the second smoke case with inherited-edit evidence.**

In the second case assert the composed `Fan.volume == 0.6`, `Radio is
SoundRadio`, each occurs once in `sources()`, and each has generated mesh limbs
after inject-before-entry. Compare their semantic world transforms with the
flat fixture, not their instance IDs.

- [ ] **Step 4: Verify the fixture task against unchanged production.**

```sh
. "$PWD/tools/lib/engine.sh"
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
gdformat game/tests/scene_composition_test.gd
gdlint game/tests/scene_composition_test.gd
"$GODOT_BIN" --headless --path "$PWD/game" --import
test -s game/tests/scene_composition_test.gd.uid
"$GODOT_BIN" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a res://tests/scene_composition_test.gd
```

Expected: 1/1 suite and 2/2 cases green, all five resources parse, and no
production file is modified. The nonempty `.gd.uid` sidecar is required tracked
Godot source metadata, not disposable cache output. If the fixture does not
parse or inheritance is not real, fix the fixture/test. If live census behavior
is red, stop at the failure and follow the conditional debugging gate in Task 4
before any Rust edit.

- [ ] **Step 5: Inspect, commit, and review.**

Run `git diff --check`, inspect all five scene files, the suite, and its `.uid`,
and verify `git status --short` contains no generated import/cache/build output
apart from the required tracked `.gd.uid`. Stage that sidecar explicitly. Commit
this one green fixture behavior with a narrative subject and explanatory body,
then request code review using the task base/head SHAs. Verify and address all
accepted findings before Task 2.

---

### Task 2: Prove world equivalence, collision, superfaces, mesh bytes, and silence

**Files:**
- Modify: `game/tests/scene_composition_test.gd` (add four cases and the
  geometry/observer helpers)

**Implementer brief — mandatory:** Re-read `AGENTS.md`, the spec, Global
Constraints, and Task 1's committed fixture inventory. This task observes the
existing engine; it does not tune it. Preserve the distance cut, geometry-only
occlusion, perception constants, label band/`MIN_SEP`, and superface merge
predicate. Use actual retained tables, physics bodies, and `CUSTOM0` bytes;
never add a production observer or compare independent palette numbers. Keep
the test portable across every supported desktop architecture and wasm source
tree. Work in the isolated branch, make one green narrative commit with no
attribution, then request fresh read-only review.

**Interfaces:**
- Consumes: Task 1's exact paths and two live independently authored fixtures;
  current callable surfaces `wall_names`, `wall_segments`, `wall_rects`,
  `wall_spans`, `sources`, `cats`, `spawn_pos`, `spawn_yaw`, warning
  forwarders, private wall children, actual meshes, and
  `WaveObserver.explain_oids`.
- Produces: four more cases, completing the 33-suite / 360-case runtime gate.
  It does not expose `face_census`, typed handles, or any new Rust API.

- [ ] **Step 0: Resolve the repository-pinned Godot for this task.**

```sh
. "$PWD/tools/lib/engine.sh"
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
```

This is a fresh task shell; do not assume Task 1's variable exists.

- [ ] **Step 1: Add the four regression cases before any production edit.**

Add exactly these functions:

```gdscript
func test_composed_and_flat_fixtures_share_hand_anchored_world_outputs() -> void:
func test_inherited_cross_wall_keeps_the_flat_collision_and_physics_verdict() -> void:
func test_nested_merges_and_touching_seams_survive_semantic_normalization() -> void:
func test_composed_and_flat_fixtures_derive_without_faults() -> void:
```

Name the comparison tolerances in their own units:

```gdscript
const WORLD_EPS_M := 0.0001
const PHYSICS_EPS_M := 0.00001
const BASIS_EPS := 0.0001  # dimensionless basis-lane tolerance
```

Use `WORLD_EPS_M` for world positions, AABB positions/sizes, wall rows, and
spawn/source/cat transforms; `PHYSICS_EPS_M` for the ray hit; and `BASIS_EPS`
for basis lanes/normals. These tolerances cover Godot transform/physics float
round trips only. Keep the four `CUSTOM0` lanes on a selected face bit-exact.

Run each focused case as it is completed. Green against unchanged Rust is the
expected characterization result; record it. Any red must identify an actual
literal, fixture, observation, or engine discrepancy before edits continue.

- [ ] **Step 2a: Implement the four aligned retained-wall rows.**

The group basis columns are `Vector3.FORWARD`, `Vector3.UP`, and
`Vector3.RIGHT`, origin `Vector3(2, 0, 12)`. For each fixture independently,
assert the raw wall paths and then normalize them through the explicit map:

| Key | Segment | Occluder rectangle | Vertical span |
|---|---|---|---|
| `run_wall` | `Vector4(6, 4, 6, 10)` | `Vector4(5.9, 3.9, 6.1, 10.1)` | `Vector2(0, 3)` |
| `cross_wall` | `Vector4(6, 7, 9, 7)` | `Vector4(5.9, 6.9, 9.1, 7.1)` | `Vector2(0, 3)` |

Assert all four arrays have exactly two entries and reject unknown/duplicate
paths.

- [ ] **Step 2b: Add source, creature, spawn, and prop absolute anchors.**

Pin:

- spawn `Vector3(4, 0.9, 9)`, yaw `PI * 0.5`;
- Fan origin `Vector3(8, 0, 9)`, yaw `PI * 0.5`, volume `0.6`;
- Radio origin `Vector3(9, 0, 5)`, yaw `PI * 0.5`;
- Cat origin `Vector3(4, 0, 5)`, yaw `PI * 0.5`;
- prop world AABBs:

| Key | AABB position | AABB size |
|---|---|---|
| `merge_shelf` | `Vector3(10.5, 0, 8)` | `Vector3(1, 1, 2)` |
| `merge_crate` | `Vector3(10.7, 0, 8.1)` | `Vector3(0.8, 1, 1)` |
| `seam_left` | `Vector3(10.5, 0, 5.5)` | `Vector3.ONE` |
| `seam_right` | `Vector3(10.5, 0, 4.5)` | `Vector3.ONE` |

- [ ] **Step 2c: Compare normalized fixtures only after literal checks.**

Only after both fixtures pass the literals, compare their semantic wall rows,
source/cat transforms, prop AABBs, and spawn output with each other.

- [ ] **Step 3a: Implement and pin the private authored-wall snapshot.**

Probe `cross_wall`, not generated `RunSeg1`. Require exactly one direct
`StaticBody3D` carrying `_unseeing_wave_wall_body == true`; body, `WaveSkin`,
and `WaveCollider` owners are null. Pin body origin `Vector3(7.5, 0, 7)`, basis
columns `Vector3.LEFT`, `Vector3.UP`, `Vector3.FORWARD`; mesh/collider global
origin `Vector3(7.5, 1.5, 7)`; mesh and shape size
`Vector3(3.3, 3, 0.3)`. Require `wall.call("paint_frame")` to match that frame
and composed/flat snapshots to match field by field.

- [ ] **Step 3b: Cast the isolated real-physics ray.**

After two physics frames, cast exactly:

```gdscript
var query := PhysicsRayQueryParameters3D.create(
	Vector3(8, 1.5, 6),
	Vector3(8, 1.5, 8),
)
var hit := get_viewport().world_3d.direct_space_state.intersect_ray(query)
```

Require the hit collider is that private body, whose parent is the semantic
`CrossWall`; pin hit position `Vector3(8, 1.5, 6.85)` and normal
`Vector3.FORWARD`. Remove the composed fixture from the tree and await one
physics frame before entering the geometrically identical flat fixture, so the
two colliders never compete in one physics world. Leave `auto_free` responsible
for final cleanup.

- [ ] **Step 4a: Normalize the structural superface class multiset.**

Create `_semantic_superfaces(level, path_to_key) -> Array[String]`: inject a
fresh `WaveObserver` with `(level, null)`, reject `unavailable`, translate every
exact member path (`Floor`/`Ceiling` included) through the explicit map, sort
members within each class, encode a class string, retain repeated class strings,
and sort the class multiset. Ignore numeric class IDs and labels. Require the
two fixture multisets agree and each contains a class with
`cross_wall+run_wall` and one with `merge_crate+merge_shelf`.

- [ ] **Step 4b: Select faces geometrically from the real mesh arrays.**

Create `_face_at_plane_and_normal`: require one ArrayMesh surface, 24 vertices,
and 24 float `ARRAY_CUSTOM0` lanes; select exactly one four-vertex face from its
world centroid and transformed geometric normal, never from `CUSTOM0`; require
all four label bits equal and the label in `[0.15, 0.96]`.

- [ ] **Step 4c: Assert the named merges and separate seam within each fixture.**

For each fixture pin:

| Pair | Plane / normals | Verdict |
|---|---|---|
| run wall + cross wall | `x = 5.85`, both `Vector3.LEFT` | labels bit-equal |
| merge shelf + crate top | `y = 1.0`, both `Vector3.UP` | labels bit-equal |
| merge shelf + crate side | `x = 11.5`, both `Vector3.RIGHT` | labels bit-equal |
| seam left / right | `z = 5.5`, `FORWARD` / `BACK` | absolute gap at least `WaveCore.new().min_label_separation()` |

Never compare a composed numeric label to a flat numeric label. Do not assert
legacy `explain_oids()["violations"]`; it is the documented first-face,
solid-granularity compatibility bridge, not this face-level oracle.

- [ ] **Step 5: Prove healthy silence and total helpers.**

For each fixture require `unfloored_solids() == 0`, `sunken_solids() == 0`,
empty level warnings, empty warning-forwarder arrays on run, both walls, four
props, fan, radio, and spawn, and an observer `faults` array present and empty.
`WaveCat` has no callable warning forwarder; prove it through exact retained
membership, injection/build, transform, and mesh existence. All helpers must
fail loudly on missing/duplicate keys, malformed arrays, or null nodes rather
than blind-indexing.

- [ ] **Step 6: Format, run focused/full gates, commit, and review.**

```sh
gdformat game/tests/scene_composition_test.gd
gdlint game/tests/scene_composition_test.gd
"$GODOT_BIN" --headless --path "$PWD/game" --import
"$GODOT_BIN" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a res://tests/scene_composition_test.gd
ci/run_gdunit.sh "$PWD/game" "$GODOT_BIN" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests
```

Expected focused: 1/1 suite, 6/6 cases. Expected full: 33/33 suites, 360/360
cases, zero errors/failures/skips. Inspect `git diff --check` and ensure no
production diff exists. Commit the completed runtime contract, then request
and resolve a fresh review before Task 3.

---

### Task 3: Extend the editor probe through inheritance, duplication, disk, and warnings

**Files:**
- Modify: `tools/probe_editor_prefabs.sh`
- Modify: `game/tests/probe/editor_prefab_probe.gd`

**Implementer brief — mandatory:** Read `AGENTS.md`, spec, Global Constraints,
and the committed fixture inventory. Preserve all 16 existing prefab checks.
Use only Godot 4.7.1's official `SceneState`, `PackedScene`, `ResourceSaver`,
`ResourceLoader`, and duplication APIs; add no production state or GDScript.
Use `GEN_EDIT_STATE_MAIN` only—never
`PackedScene.GEN_EDIT_STATE_MAIN_INHERITED`—and add each instance to
the tree before reading global transforms or duplicating/packing it. Generated
limbs remain ownerless; no label, superface, perception, or occlusion law may
change. Poll bounded conditions, keep all platforms source-compatible, commit
one green editor-contract behavior with no attribution, and request fresh
read-only review.

**Measured Godot 4.7.1 facts:** This exact fixture was exercised outside the
repository before the plan was frozen. The inherited local state paths are
`.`, `./Fan`, `./Radio`; the base nested instance is `./NestedProp`. A settled
composed level has 48 ownerless generated descendants, and duplication returns
to the same 48—not zero or 96. Packing the settled duplicate, saving, deep-cache
reloading, and entering it preserved both instance links, the inherited base,
the `0.6` override, and Radio while saving no generated name. These are the
expected assertions, not hypotheses.

- [ ] **Step 0: Resolve the repository-pinned Godot for this task.**

```sh
. "$PWD/tools/lib/engine.sh"
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
```

Keep this task's editor probe and runtime rerun on that exact binary.

- [ ] **Step 1: Make the shell wrapper demand the expanded contract first.**

Factor the script resource into:

```sh
PROBE=res://tests/probe/editor_prefab_probe.gd
```

Use it in the existing `-e -s` invocation, change the terminal assertion from
16 to 36 checks, and change the final success sentence to name inheritance,
duplication, disk persistence, and nested warning watching. Run:

```sh
GODOT="$GODOT_BIN" tools/probe_editor_prefabs.sh
```

Expected red: the unchanged probe prints `PASS (16 checks)` but the wrapper
rejects it because 36 are required. Any other failure must be resolved before
editing the probe.

- [ ] **Step 2a: Add constants and the bounded phase enum.**

Keep the old checks and add exactly 20. Use:

```gdscript
enum Phase {
	WAIT_LEGACY,
	WAIT_VARIANT_READY,
	WAIT_COMPOSED_READY,
	WAIT_DUPLICATE_READY,
	WAIT_ROUNDTRIP_READY,
	WAIT_INVALID_WARNING,
	WAIT_INVALID_SETTLE,
	WAIT_REPAIR,
	WAIT_REPAIR_SETTLE,
}

const READY_FRAMES := 30
const WATCH_FRAMES := 30
const SETTLE_FRAMES := 3
const INHERITED_PATH := (
	"res://tests/fixtures/scene_composition/inherited_room_variant.tscn"
)
const COMPOSED_PATH := "res://tests/fixtures/scene_composition/composed_level.tscn"
const FLAT_PATH := "res://tests/fixtures/scene_composition/flat_level.tscn"
```

- [ ] **Step 2b: Sequence tree-safe MAIN instances and duplicate settlement.**

Keep only one `GEN_EDIT_STATE_MAIN` artifact in the tree at a time. Sequence:

1. run the legacy judge unchanged;
2. instantiate the inherited variant with
   `inherited.instantiate(PackedScene.GEN_EDIT_STATE_MAIN)`, add it to `root`,
   poll ready, judge, then remove/free it;
3. instantiate composed the same way, add it, and poll its 48 generated nodes;
4. duplicate with `Node.DUPLICATE_USE_INSTANTIATION`, add the duplicate, and
   poll until its generated inventory settles to 48;
5. pack/save the settled live duplicate, remove original and duplicate, then
   deep-cache reload and add the reloaded MAIN instance;
6. move nested `SeamRight.position.y` from `0.5` to `0.0`, poll the warning and
   derive-count change/stability, restore `0.5`, and poll clear/stability.

Never free or pack a transient root inside `_initialize`; the measured
experiment showed that doing so can read global transforms before tree entry.

- [ ] **Step 3a: Add inherited-state and first-MAIN checks 1–5.**

Allocate them exactly so the wrapper count remains auditable:

1. inherited state has a non-null base state;
2. base `./NestedProp` has a non-null ordinary scene instance pointing to
   `nested_prop.tscn`;
3. inherited local `./Fan` carries `volume == 0.6`;
4. inherited local `./Radio` exists with owner path `.`;
5. inherited MAIN live instance has the override, Radio, and expected transform;

- [ ] **Step 3b: Add composed/duplicate inventory checks 6–10.**

6. composed MAIN has every authored inventory node with non-null owner;
7. original has the complete 48-node generated inventory, all recursively
   ownerless;
8. `duplicate(USE_INSTANTIATION)` returned non-null and entered the tree;
9. duplicate has 48 generated nodes and exactly one `WaveFloor`,
   `WaveCeiling`, and `RunSeg1`;
10. duplicate generated subtrees are recursively ownerless;

- [ ] **Step 3c: Add pack/save/reload checks 11–16.**

11. `PackedScene.pack(copy) == OK`;
12. `ResourceSaver.save(packed, unique_path) == OK`;
13. `ResourceLoader.load(unique_path, "PackedScene",
    ResourceLoader.CACHE_MODE_IGNORE_DEEP)` returns a `PackedScene`;
14. recursively loaded state preserves the inherited-room base and nested-prop
    instance links;
15. reloaded MAIN has every authored inventory node/owner, `volume == 0.6`,
    and Radio;
16. recursive saved `SceneState` graph contains no forbidden generated name;

- [ ] **Step 3d: Add warning-change/settle/repair checks 17–20.**

17. sinking `SeamRight` raises `derive_count` and adds its exact path warning;
18. invalid state holds one derive count for three observed frames;
19. repairing `SeamRight` raises the count and clears the warning;
20. repaired state stays stable for three frames and the temporary file is
    removed.

Within checks 1–6 and 14–16, also require the exact local record counts measured
for the fixture artifacts: nested prop 5 (root + four props), base room 7 (root
+ five typed direct nodes + one nested instance), inherited variant 3 (base
instance + override + Radio), composed level 3 (root + group + inherited
instance), and flat level 11 (root + ten authored nodes). These counts are part
of the explicit inventory oracle, not inferred from a fresh recursive walk.

Use explicit stable generated roots and recursive counts rather than guessing
auto-generated inner names:

| Builder | Count | Stable roots / pattern |
|---|---:|---|
| BoundaryRun | 4 | `RunSeg1/WaveBody/{WaveSkin,WaveCollider}` |
| CrossWall | 3 | `WaveBody/{WaveSkin,WaveCollider}` |
| four props | 8 | each `{WaveSkin,WaveCollider}` |
| Fan | 17 | `FanPedestal`, `FanPivot`, all descendants |
| Cat | 2 | `CatCollider`, `CatSkin` |
| Radio | 8 | six named roots, two children below `RadioCase` |
| slabs | 6 | `WaveFloor`, `WaveCeiling`, each two children |
| total | 48 | every generated node `owner == null` |

Forbidden names are `WaveFloor`, `WaveCeiling`, `RunSeg*`, `WaveBody`,
`WaveSkin`, `WaveCollider`, `FanPedestal`, `FanPivot`, `RadioCase`,
`RadioGrille`, `RadioTuner`, `RadioDialA`, `RadioDialB`, `RadioAntenna`,
`CatCollider`, and `CatSkin`.

Implement `RunSeg*` as `name.begins_with("RunSeg")`; compare every other name
against the literal forbidden-name set. A literal equality check for the text
`"RunSeg*"` is not a wildcard and is forbidden here because it would silently
retain `RunSeg1`.

- [ ] **Step 3e: Implement the total state, inventory, settle, and cleanup helpers.**

Implement total helpers with these signatures:

```gdscript
func _state_node_index(state: SceneState, path: NodePath) -> int:
func _state_property_equals(
	state: SceneState, path: NodePath, property: StringName, expected: Variant
) -> bool:
func _state_instance_path(state: SceneState, path: NodePath) -> String:
func _state_graph_has_forbidden(state: SceneState, seen: Dictionary) -> bool:
func _is_forbidden_generated_name(name: String) -> bool:
func _authored_nodes_are_owned(node: Node, paths: Array[NodePath]) -> bool:
func _generated_subtrees_are_ownerless(node: Node, paths: Array[NodePath]) -> bool:
func _generated_inventory_is_exact(node: Node) -> bool:
func _warning_has(node: Node, needle: String) -> bool:
func _begin_settle(level: WaveLevel, minimum_count: int) -> void:
func _settled(level: WaveLevel) -> bool:
func _remove_temp_scene() -> bool:
```

State paths/owners are exact: variant records `.`, `./Fan`, `./Radio`, with
Radio owner `.`, while base `./NestedProp` owner is `.`. Live composed owners
are: `PlainGroup` and the inherited instance owned by `ComposedLevel`; base
children, Radio, and `NestedProp` owned by `InheritedRoomVariant`; nested props
owned by `NestedProp`.

Build a unique path from process ID and ticks:

```gdscript
_saved_path = "user://editor-prefab-roundtrip-%d-%d.tscn" % [
	OS.get_process_id(), Time.get_ticks_usec()
]
```

Remove it with
`DirAccess.remove_absolute(ProjectSettings.globalize_path(_saved_path))` and
repeat that cleanup in `_finalize()` as a fail-safe.

- [ ] **Step 4: Pin the warning text and settle semantics.**

The exact healthy-to-invalid edit is local `SeamRight.y: 0.5 -> 0.0`. Require
the full warning, not only the leaf name:

```text
WaveLevel: 'PlainGroup/InheritedRoomVariant/NestedProp/SeamRight' is sunk through the floor — its box spans y -0.50..0.50, and the floor's top is at y 0.00. What is under the slab never draws, never sounds and cannot be walked into. A WaveProp is CENTRED on its node, so dropping one on the floor plane buries exactly half of it, while a wall, a column and a wedge STAND on theirs. Lift the node until the whole shape clears y 0.00.
```

Use `derive_count` only as a change-and-settle witness: require it to increase
after each authored edit, then remain unchanged for `SETTLE_FRAMES`. Do not pin
the absolute number of derive passes.

- [ ] **Step 5: Format, run the existing pipeline-owned probe, commit, and review.**

```sh
gdformat game/tests/probe/editor_prefab_probe.gd
gdlint game/tests/probe/editor_prefab_probe.gd
GODOT="$GODOT_BIN" tools/probe_editor_prefabs.sh
"$GODOT_BIN" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a res://tests/scene_composition_test.gd
```

Expected: editor mode proved and `PASS (36 checks)`. Run the six-case runtime
suite in the block to prove the probe did not perturb fixtures. Inspect for the
unique temp file, `game/override.cfg`, imports, reports, or generated content;
none may remain. Commit the editor persistence behavior, then request and
resolve a fresh review before mutations.

---

### Task 4: Demonstrate the regression kills each realistic mutation

**Files:**
- Temporarily modify and restore: `rust/src/nodes/level.rs`
- Temporarily modify and restore: `rust/src/nodes/wall.rs`
- Temporarily modify and restore: `rust/src/nodes/run.rs`
- Test only: `game/tests/scene_composition_test.gd`
- Probe only: `game/tests/probe/editor_prefab_probe.gd`

**Implementer brief — mandatory:** Read `AGENTS.md`, spec, Global Constraints,
and `superpowers:verification-before-completion`. These edits are deliberate
fault injection, never implementation. Make one mutation at a time, build the
release GDExtension, observe the named test fail for the intended reason, apply
the exact inverse patch, rebuild, and observe green before proceeding. Do not
commit, stash, reset, or use checkout to restore. Never alter perception,
visible-air, geometry admission, label allocation, or superface constants as a
shortcut. All supported-platform source must be restored byte-for-byte; no
attribution or build output enters git.

- [ ] **Step 0: Resolve the repository-pinned Godot for the mutation shell.**

```sh
. "$PWD/tools/lib/engine.sh"
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
```

Define the narrow gate once:

```sh
run_contract() {
	(cd rust && cargo build --release) || return 1
	"$GODOT_BIN" --headless --path "$PWD/game" --import >/dev/null 2>&1 || true
	"$GODOT_BIN" --headless --path "$PWD/game" \
		-s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
		--ignoreHeadlessMode -c -a res://tests/scene_composition_test.gd
}
```

Do not commit the shell function; it is session-local evidence.

- [ ] **Mutation 1 — stop at the plain group.** In `collect`, immediately
  inside the child loop, temporarily add:

```rust
if child.get_name() == "PlainGroup" {
    continue;
}
```

Expected: `test_plain_groups_do_not_hide_or_duplicate_nested_gameplay` fails
because composed walls, props, sources, cat, and spawn disappear. Reverse the
patch, rebuild, and require 6/6 green.

- [ ] **Mutation 2 — ignore the ancestor transform at the wall boundary.** In
  `WaveWall::segment`, temporarily replace:

```rust
let placed = self.canonical_transform();
```

with:

```rust
let placed = self.base().get_transform();
```

Expected: the hand-anchored composed wall/occluder comparison fails while the
flat oracle remains correct. Reverse exactly, rebuild, and require 6/6 green.

- [ ] **Mutation 3 — serialize a generated run wall.** In
  `WaveRun::rebuild`, immediately after `self.base_mut().add_child(&wall);`,
  temporarily add:

```rust
if let Some(owner) = self.base().get_owner() {
    wall.set_owner(&owner);
}
```

Build release and run `GODOT="$GODOT_BIN" tools/probe_editor_prefabs.sh`.
Expected: original/duplicate owner checks and/or the saved-state forbidden-name
check fail. Reverse, rebuild, and require all 36 checks green.

- [ ] **Mutation 4a — skip the inherited variant.** In `collect`, before
  classification, temporarily add:

```rust
if child.get_name() == "InheritedRoomVariant" {
    continue;
}
```

Expected: the first two runtime cases fail on missing inherited base children,
Fan, and Radio. Reverse, rebuild, require 6/6 green.

- [ ] **Mutation 4b — double the inherited variant.** Immediately after the
  normal `collect(&child, census);`, temporarily add:

```rust
if child.get_name() == "InheritedRoomVariant" {
    collect(&child, census);
}
```

Expected: retained multiplicities, walls, sources, cats, or duplicate-spawn
warning assertions fail. Reverse, rebuild, require 6/6 and editor 36/36 green.

- [ ] **Step 6: Prove every mutation is gone.**

```sh
git diff --check
git status --short
git diff -- rust/src/nodes/level.rs rust/src/nodes/wall.rs rust/src/nodes/run.rs
```

Expected: no production diff and only already-authorized task changes, if any.
Run:

```sh
(cd rust && cargo fmt --check)
(cd rust && cargo clippy --all-targets -- -D warnings)
(cd rust && cargo test)
run_contract
GODOT="$GODOT_BIN" tools/probe_editor_prefabs.sh
```

Record the mutation/result matrix in the task handoff, not in a generated report file.
There is no mutation commit. Request a read-only review of the mutation targets,
test assertions, and clean production diff before documentation.

#### Conditional debugging gate

If unchanged production was red at any earlier point, stop here. Invoke
`superpowers:systematic-debugging`; reproduce one failure; trace the observed
value back through its owning boundary; form and test one hypothesis; keep the
failing regression; and after three failed fixes question the architecture.
Before any fix, amend the approved spec and this plan with the diagnosed owner,
complete input domain, injected dependencies, smallest pure change, and the
red/green test. A fix must start from its failing test, stay total/pure at the
domain layer, and receive its own task, green commit, mutation evidence, and
review. Do not silently turn this conditional paragraph into implementation.

---

### Task 5: Document the supported authoring workflow and prepare wiki write-back

**Files:**
- Modify: `docs/opening-in-godot.md`
- Create: `docs/superpowers/handoffs/2026-08-21-scene-authoring-wiki-writeback.md`

**Implementer brief — mandatory:** Read `AGENTS.md`, spec, Global Constraints,
the current wiki's Mechanics Overview, Mechanics — Level and Objects, and
Research — Editor Authoring. Documentation must describe the measured shipped
contract and name its owning files/constants without acoustic derivations.
Preserve the perception, visible-air, geometry-occlusion, label-clearance, and
superface laws verbatim in meaning; do not claim a new production abstraction.
The same Godot scenes serve all desktop architectures and wasm. Commit one
green documentation behavior with the repository identity/no attribution,
then request fresh read-only review. Do not publish or push the external wiki
before the user's integration choice.

- [ ] **Step 0: Resolve the repository-pinned Godot for documentation verification.**

```sh
. "$PWD/tools/lib/engine.sh"
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
```

- [ ] **Step 1: Add the designer workflow to `docs/opening-in-godot.md`.**

Immediately after the reusable-scene placement paragraph in section 6, add an
"Create an inherited room variant" subsection that tells a designer to:

1. select the base room scene and choose **New Inherited Scene**;
2. save the variant under `game/scenes/rooms/`;
3. override exported authored properties such as `SoundFan.Volume` on an
   inherited typed node;
4. add new typed authored children such as `SoundRadio` to the inherited root
   and save the scene;
5. instance the variant beneath any plain `Node3D` grouping transform in a
   `WaveLevel`;
6. edit only authored roots/nodes, never `RunSeg*`, `WaveBody`, `WaveSkin`,
   `WaveCollider`, source/cat blueprint limbs, `WaveFloor`, or `WaveCeiling`;
7. expect Rust to rebuild those ownerless generated nodes after duplication,
   reload, or play.

State explicitly that `rust/src/nodes/level.rs::collect` owns recursive live
discovery; `WaveRun`, wall/solid/source/cat builders, and
`WaveLevel::build_slabs` own generated limbs; and
`game/tests/scene_composition_test.gd` plus
`game/tests/probe/editor_prefab_probe.gd` own the regression contract.

- [ ] **Step 2: Prepare exact wiki prose in the tracked handoff.**

The handoff must contain paste-ready sections, target headings, and file owners
for:

- **Mechanics — Level and Objects §2:** plain groups are transparent to the
  recursive live census; nested and inherited scene instances need no runtime
  registration; authored transforms compose in world space; generated
  `RunSeg1` is consumed as a derived wall despite being absent from saved
  `SceneState`.
- **Mechanics — Level and Objects §5:** the composed-vs-flat fixture pins once-
  only wall/source/cat/spawn results, collision, superface membership, actual
  mesh labels, and zero faults; name all five fixture files and both test/probe
  owners.
- **Mechanics — Level and Objects §6:** authored nodes keep scene ownership;
  generated walls, slabs, skins, colliders, and source/cat limbs remain
  ownerless and rebuild idempotently after `DUPLICATE_USE_INSTANTIATION`.
- **Research — Editor Authoring §1/§6/§7:** record the measured Godot 4.7.1
  paths `.`, `./Fan`, `./Radio`, base `./NestedProp`; `GEN_EDIT_STATE_MAIN`
  usage; 48 -> 48 duplicate settlement; pack/save/deep-cache reload preserving
  inheritance/nesting and excluding generated data; nested warning path and
  three-frame settle result. Correct any stale claim that Ctrl+D necessarily
  doubles generated limbs.

Include a publication checklist: apply the prose only after the feature branch
is integrated; diff the wiki clone; verify page links/headings; commit with the
repository identity and no attribution; ask before pushing the separate wiki
repository.

- [ ] **Step 3: Verify, commit, and review documentation.**

Run `git diff --check`; reread both documents against the live tests and spec;
search edited prose for unsupported frequency/energy/acoustic claims and for
assistant attribution. Then run:

```sh
"$GODOT_BIN" --headless --path "$PWD/game" --import
"$GODOT_BIN" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a res://tests/scene_composition_test.gd
GODOT="$GODOT_BIN" tools/probe_editor_prefabs.sh
```

Require 6/6 runtime cases and 36/36 editor checks so docs never land over a red
contract. Commit the documentation, then request and resolve a fresh review.

---

### Task 6: Run final evidence, obtain final review, and stop at integration

**Files:**
- Verify all task files above
- Do not create reports, exports, rendered frames, or production diffs

**Implementer brief — mandatory:** Read `AGENTS.md`, spec, Global Constraints,
`superpowers:verification-before-completion`,
`superpowers:requesting-code-review`, and
`superpowers:finishing-a-development-branch`. This task changes nothing unless
a failing check receives a test-first, reviewed repair within the authorized
scope. Reconfirm no perception/visible-air/occlusion/label/superface law moved,
no production API appeared, and all supported platform sources remain common.
Use the repository identity/no attribution. Never merge, push, publish the
wiki, or deploy without the user's explicit finish-branch choice.

- [ ] **Step 0: Resolve the repository-pinned Godot for final evidence.**

```sh
. "$PWD/tools/lib/engine.sh"
GODOT_BIN="$(unseeing_engine_select "$PWD" "${GODOT:-}")"
```

- [ ] **Step 1: Inspect the complete branch before claiming anything.**

```sh
git status --short --branch
git diff --check main...HEAD
git diff --stat main...HEAD
git diff main...HEAD -- rust/src
git log --oneline --decorate main..HEAD
git grep -n -E 'Co-Authored-By|Generated with|generated by (AI|an assistant)' -- \
  ':!tools/superpowers'
```

Expected: only specs/plans/tests/fixtures/probe/docs/handoff changes; no final
`rust/src` diff; no build output, temp file, or attribution.

- [ ] **Step 2: Run fresh format, Rust, Godot, runtime, and editor evidence.**

```sh
(cd rust && cargo fmt --check)
(cd rust && cargo clippy --all-targets -- -D warnings)
(cd rust && cargo test)
(cd rust && cargo check --features editor-docs)
(cd rust && cargo test --features editor-docs editor_docs)
(cd rust && cargo build --release)

gdformat --check game/tests/scene_composition_test.gd \
  game/tests/probe/editor_prefab_probe.gd
gdlint game/tests/scene_composition_test.gd \
  game/tests/probe/editor_prefab_probe.gd

"$GODOT_BIN" --headless --path "$PWD/game" --import
ci/run_gdunit.sh "$PWD/game" "$GODOT_BIN" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests
GODOT="$GODOT_BIN" tools/probe_editor_prefabs.sh
GODOT="$GODOT_BIN" tools/probe_editor_level.sh
GODOT="$GODOT_BIN" tools/probe_editor_sources.sh
GODOT="$GODOT_BIN" tools/probe_editor_slabs.sh
```

Require 568 Cargo tests or more without shrink, 33/33 gdUnit suites and
360/360 cases, 36/36 prefab checks, and every other probe's existing exact
PASS contract. Re-run any failure after diagnosing it; do not quote stale
earlier output as proof.

- [ ] **Step 3: Run the checks-only pipeline and classify the known clean-head failure exactly.**

```sh
GODOT="$GODOT_BIN" SKIP_EXPORT=1 ci/pipeline.sh
```

On the planning host, unchanged `main` currently fails in
`test/engine_select_test.sh`: a macOS `/bin/sh` prefix assignment persists on a
shell function and contaminates four later default-PATH cases. This issue is
outside #65. If the pipeline fails, compare it against a clean `main` worktree
and accept only that identical pre-existing failure with every #65-relevant
gate run directly above. Any new failure or different output blocks
completion. Do not fix the unrelated shell test on this branch without new
authority.

- [ ] **Step 4: Request final code, architecture, and performance review.**

Give a fresh reviewer the approved spec, this plan, `main...HEAD`, the exact
verification output, and mutation matrix. Require the reviewer to identify:

- which existing pure component owns every observed law and its complete input
  domain/dependencies;
- whether explicit maps and literals are independent or mirror production;
- whether actual `CUSTOM0` and collision evidence closes the observer gap;
- whether duplication/save tests prove ownership without depending on unstable
  auto-names;
- whether any test introduces unnecessary scene walks, per-frame work, global
  state, or platform coupling;
- whether docs/wiki handoff match only measured behavior.

Resolve all verified Critical/Important findings, rerun affected and full
evidence, commit any fixes separately, and obtain a fresh clean verdict.

- [ ] **Step 5: Use the finish-branch workflow and present the user's choices.**

Invoke `superpowers:finishing-a-development-branch`. Report the branch name,
commits, exact evidence and the one pre-existing pipeline caveat, production
Rust diff (expected none), and pending post-integration wiki publication.
Present the workflow's integration choices. Stop. Do not merge or push. If the
user later chooses integration, verify the durable primary checkout is clean
and on `main` before the merge; after integration, publish the prepared wiki
change only with explicit authority. The push of merged `main` automatically
deploys the web build through `.github/workflows/test.yml`.

---

## Plan self-check

- Approved-spec coverage: plain grouping, nested instances, true inheritance,
  flat oracle, exact once-only census, ancestor transforms, wall/occlusion/
  collision/source/cat/spawn equivalence, superface graph, real label bytes,
  generated ownership, duplicate settlement, disk reload, warning path/clear,
  mutations, docs, wiki, review, and integration gate are each assigned above.
- No production fix is assumed; unchanged Rust is the expected result.
- The complete authored/generated inventory is explicit, including slabs and
  all 48 generated descendants.
- Every numeric literal is a hand-derived fixture anchor or a named existing
  engine contract in its own units; none is justified from real acoustics.
- Every task repeats the perception/label/superface, architecture/platform,
  isolation/commit, and attribution constraints required in implementer briefs.
