# Wiki write-back — nested and inherited scene authoring

This handoff prepares the separate GitHub wiki update for the scene-authoring
contract. It does not publish that update. Apply it only after branch
`issue-65-scene-authoring-contract` has been integrated, so the wiki never
describes an unmerged contract as shipped.

The prose below is ready to paste under the named live headings. It records
measured Godot 4.7.1 behavior and the existing engine owners; it introduces no
room registry, flattening step, custom `Resource`, or other production
abstraction. The same `game/` scenes remain the source for macOS and Windows on
x86_64 and arm64, and for wasm32. Nothing here changes propagation,
perception, the distance-valued visible-air cut, geometry-only occlusion,
source-through/detail-knee scope, or the label and superface laws.

---

## Page: Mechanics — Level and Objects

### Target: append under §2, `What the level derives`

#### Scene composition is transparent to derivation

`WaveLevel` discovers the live scene recursively in deterministic depth-first
scene order. A plain `Node3D` contributes no gameplay entity of its own, but it
is a transparent grouping boundary: typed descendants beneath it are still
collected. An ordinary nested `PackedScene` instance and a true inherited scene
need no runtime registration or flattening step. Their authored ancestor
transforms compose in world space before wall centerlines, collision,
occlusion, source and creature poses, prop shapes, and the spawn are derived.

The recursion is owned by `rust/src/nodes/level.rs::collect`. It recognizes
solids and sources through their existing dynamic contracts and recognizes
walls, runs, cats, and `WaveSpawn` through their existing registered types,
then recurses beneath every child whether that child was recognized or not.
This is scene composition through the existing census, not a new inheritance-
aware runtime abstraction.

A `WaveRun` is authored, but the `RunSeg1` wall it builds is derived live data.
`rust/src/nodes/run.rs::WaveRun::rebuild` creates the segment without a scene
owner. The recursive census still consumes that live `WaveWall` in the wall,
solid, paint, collision, and occlusion results even though the saved
`SceneState` correctly contains no `RunSeg1`.

Occlusion remains a geometry verdict, never a node-class verdict:
`rust/src/level_plan.rs::spans_the_corridor` admits a solid only when it spans
floor to ceiling within `SPAN_EPS` and its minimum horizontal extent is at
least `2 * WALL_T`; refused props still contribute authored source-image
clarity through `level_plan::prop_through`. Scene nesting does not change that
law.

### Target: append under §5, `Map invariants that are tested, not eyeballed`

#### Nested and inherited scenes have a flat semantic oracle

`game/tests/fixtures/scene_composition/` carries five test-only scene
artifacts:

- `nested_prop.tscn` — a plain-root nested prop scene with a named coplanar
  overlap and a named face-to-face seam;
- `base_room.tscn` — a plain-root room containing `WaveRun`, `WaveWall`,
  `SoundFan`, `WaveCat`, `WaveSpawn`, and the nested prop instance;
- `inherited_room_variant.tscn` — a true inherited scene that overrides
  `Fan.volume` to `0.6` and adds an authored `SoundRadio`;
- `composed_level.tscn` — that inherited room under a translated,
  quarter-turned plain `Node3D` group; and
- `flat_level.tscn` — an independently authored, hand-transformed flat oracle
  with the same semantic gameplay content.

`game/tests/scene_composition_test.gd` compares the composed and flat scenes by
explicit semantic path maps, not instance IDs or matching leaf names. Its six
scene-behavior cases prove that the explicit wall, generated run wall, four
props, two sources, cat, and spawn reach their relevant live and retained
outputs exactly once while the grouping roots contribute none. They pin
world-space wall centerlines and occluder rectangles/spans, source and cat
transforms, prop AABBs, spawn pose, the inherited volume override and Radio
addition, the authored wall's private body/mesh/collider frame, and a real
physics-ray hit.
The seventh oracle-integrity case rejects malformed dynamic mesh-array slots
and non-finite real `ArrayMesh` faces, making this a seven-case final suite.

The suite compares semantic superface membership and then reads actual
`Mesh.ARRAY_CUSTOM0` bytes. The named run/cross-wall and shelf/crate
same-facing coplanar overlaps carry bit-identical labels within each fixture;
the named face-to-face prop seam remains at least `MIN_SEP = 0.08` apart.
Numeric palettes are deliberately not compared between the independently
coloured scenes. The healthy fixtures also report zero placement, warning,
paint, occlusion, and label faults.

`MIN_SEP` and the label-role table are owned by
`rust/src/render/labels.rs`. `COPLANAR_EPS` and `PATCH_EPS`, and therefore the
same-facing coplanar merge predicate, are owned by
`rust/src/render/superface.rs`. `game/tests/probe/editor_prefab_probe.gd` owns
the complementary editor-mode inheritance, ownership, duplication, disk, and
warning contract.

### Target: append under §6, `Authoring recipe`

#### Authored scene data and generated live data stay separate

Authored nodes keep the owner assigned by their scene artifact: level grouping
and inherited-instance roots belong to the level scene, base-room children and
the inherited Radio addition belong to the inherited room root, and nested
props belong to the nested prop root. Those authored roots, typed gameplay
nodes, transforms, and exported properties are what a designer edits and what
Godot saves.

Generated walls, slabs, skins, colliders, and source/cat blueprint limbs have
no scene owner. `rust/src/nodes/run.rs` owns `RunSeg*` generation;
`rust/src/nodes/wall.rs` and `rust/src/nodes/solid.rs` own private wall and
solid limbs; `rust/src/nodes/fan.rs`, `radio.rs`, and `cat.rs` own their
blueprint limbs; and `rust/src/nodes/level.rs::build_slabs` owns `WaveFloor`
and `WaveCeiling`. Builders clear their named generated children and rebuild
them idempotently. Designers must not edit, reparent, or assign an owner to
those nodes.

Measured in Godot 4.7.1 editor mode, the composed fixture settles with exactly
48 ownerless generated descendants. Duplication with
`Node.DUPLICATE_USE_INSTANTIATION` settles back to exactly 48, not zero or 96,
with one `WaveFloor`, one `WaveCeiling`, and one `RunSeg1`. Packing the settled
duplicate and reloading it rebuilds the same live data from the saved authored
scene while the saved `SceneState` contains none of the generated names. The
editor proof is `game/tests/probe/editor_prefab_probe.gd`; its pipeline owner
is `tools/probe_editor_prefabs.sh`.

---

## Page: Research — Editor Authoring

### Target: append under §1, `What already works`

#### True inheritance and nested ownership are measured on Godot 4.7.1

The regression fixture now measures ordinary nested scenes and a true Godot
inherited scene through the official `PackedScene` and `SceneState` APIs. For
`inherited_room_variant.tscn`, `get_base_scene_state()` is non-null. Its local
`SceneState` records are exactly `.`, `./Fan`, and `./Radio`; `./Fan` carries
the authored `volume = 0.6` override and `./Radio` is owned by `.`. The base
room represents `./NestedProp` as an ordinary owned scene instance of
`nested_prop.tscn`.

The exact local `SceneState` record counts are 5 for `nested_prop.tscn`, 7 for
`base_room.tscn`, 3 for `inherited_room_variant.tscn`, 3 for
`composed_level.tscn`, and 11 for the independently flat `flat_level.tscn`.
The already inherited variant and composed top-level fixture are opened as
main edited scenes with `PackedScene.GEN_EDIT_STATE_MAIN` and added to the tree
before global transforms are read. `GEN_EDIT_STATE_MAIN_INHERITED` is not used:
that mode is for creating an inherited scene from its base, not for opening an
already inherited artifact.

The runtime half is held by `game/tests/scene_composition_test.gd`; the editor
and `SceneState` half is held by
`game/tests/probe/editor_prefab_probe.gd`. The five fixture files live under
`game/tests/fixtures/scene_composition/` and are test content, not shipped
levels.

### Target: append under §6, `The staged plan`

#### Duplication and disk round-trip preserve authored composition

The composed main-edited fixture settles with 48 generated descendants, all
ownerless. Calling `Node.duplicate(Node.DUPLICATE_USE_INSTANTIATION)`, adding
the duplicate to the tree, and polling its concrete inventory settles to the
same 48 ownerless descendants — exactly one `WaveFloor`, one `WaveCeiling`,
and one `RunSeg1`, with no missing set and no doubled ghost set.

Packing that settled live duplicate, saving it to a unique `user://` `.tscn`,
and loading it with `ResourceLoader.CACHE_MODE_IGNORE_DEEP` preserves the
inherited-room base link, the ordinary nested-prop instance link, the
`Fan.volume = 0.6` override, the added `SoundRadio`, every authored node, and
the exact authored owner relationships. The recursive saved `SceneState`
contains no generated `RunSeg*`, slabs, wall bodies, skins, colliders, or
source/cat blueprint limb names. Entering the reloaded scene rebuilds those
ownerless nodes. The probe removes its unique temporary file before exit and
again from its finalizer as a fail-safe.

Editor warning watching also crosses the full composed path. Moving
`PlainGroup/InheritedRoomVariant/NestedProp/SeamRight` from local `y = 0.5` to
`y = 0.0` increases `WaveLevel.derive_count` and produces exactly:

```text
WaveLevel: 'PlainGroup/InheritedRoomVariant/NestedProp/SeamRight' is sunk through the floor — its box spans y -0.50..0.50, and the floor's top is at y 0.00. What is under the slab never draws, never sounds and cannot be walked into. A WaveProp is CENTRED on its node, so dropping one on the floor plane buries exactly half of it, while a wall, a column and a wedge STAND on theirs. Lift the node until the whole shape clears y 0.00.
```

The invalid state holds one derive count for three observed frames. Restoring
local `y = 0.5` raises the count again, clears the warning, and then holds the
repaired derive count for three observed frames. The count is a change-and-
settle witness only; no absolute number of derivations is part of the contract.
Both ready and warning polls are bounded and add no arbitrary sleep.

### Target: replace the complete §6 Stage 0 paragraph

Replace this complete current paragraph once:

> **Stage 0 — corrections that cost nothing (½ day).** Delete the false
> `game/README.md:46-48` warning ("never open scenes before this build, or the
> editor will strip the engine node types") — measured false, and it is actively
> keeping a designer out of the editor; rewrite step 3, which recommends Ctrl+D
> (the one gesture that doubles engine-built geometry: `original 2 → after add 4`,
> knob then drives only the newest pair) and describes a law that is invisible
> all session; skip the `lid` slab under `is_editor_hint()`; make the tool
> `ready()`s idempotent.

Paste this complete replacement paragraph in its place:

> **Stage 0 — corrections that cost nothing (½ day).** Delete the false
> `game/README.md:46-48` warning ("never open scenes before this build, or the
> editor will strip the engine node types") — measured false, and it is actively
> keeping a designer out of the editor; replace the old duplication instruction
> with the measured contract: in the tracked composed fixture on Godot 4.7.1,
> `Node.duplicate(Node.DUPLICATE_USE_INSTANTIATION)` settles from 48 ownerless
> generated descendants to 48, with exactly one `WaveFloor`, one `WaveCeiling`,
> and one `RunSeg1`. A doubled set is a regression, not the supported behavior.
> `game/tests/probe/editor_prefab_probe.gd` measures the complete inventory
> after tree entry rather than inferring it from a screenshot or an internal
> walk count. This regression does not exercise GUI Ctrl+D, so no Ctrl+D
> behavior is claimed; skip the `lid` slab under `is_editor_hint()`; make the
> tool `ready()`s idempotent.

### Target: delete the exact Ctrl+D bullet in §7

Delete this exact live bullet from §7; do not paste the §6 replacement a second
time and do not leave the old Ctrl+D claim behind:

> - **"Reparenting doubles geometry."** False — the trigger is **Ctrl+D**.

---

## Post-integration publication checklist

- Confirm the feature branch has been integrated and the integrated commit is
  the one whose seven-case runtime suite (six scene-behavior cases plus one
  oracle-integrity case) and 36-check editor probe passed.
- Open the separate wiki repository without modifying this game repository;
  apply only the paste-ready prose above to the named pages and headings.
- Diff the wiki clone and verify that the `Mechanics — Level and Objects` and
  `Research — Editor Authoring` page links and heading anchors still resolve
  from `Mechanics — Overview`.
- Recheck every path, mode, count, warning sentence, file owner, `MIN_SEP`,
  `COPLANAR_EPS`, and `PATCH_EPS` against integrated source and tests.
- Search the wiki diff for unsupported real-world sound rationales and
  prohibited authorship credit; do not publish either.
- Commit the wiki-only change as `Dmitrii Galchenko <dggrus@gmail.com>` with an
  explanatory narrative subject/body and no attribution footer.
- Ask the user before pushing the separate wiki repository. Do not combine
  wiki publication with the automatic `main` web deployment gate.
