# Nested and Inherited Scene Authoring Contract — Design

**Status:** approved by the user on 2026-08-21. Implements the design phase
of GitHub issue #65, “Make nested and inherited Godot scenes a tested
level-authoring contract.”

## Goal

Make three existing Godot authoring mechanisms a durable project contract:

1. plain `Node3D` grouping roots;
2. nested `PackedScene` room and prop instances; and
3. Godot inherited scenes that override authored properties and add authored
   children.

The contract is proved by a tracked composed/inherited fixture and an
independently authored flat equivalent. If the new regression passes against
unchanged production code, this task changes tests and documentation only.
Production Rust changes are authorized only after the regression exposes a
real defect.

## Existing behaviour and the uncovered boundary

`WaveLevel::collect` in `rust/src/nodes/level.rs` walks every live descendant
in depth-first scene order. It recognizes designer-facing Rust nodes by their
dynamic contracts or concrete type, then recurses into the child regardless
of whether that child was recognized. A plain `Node3D` therefore contributes
no gameplay entity while remaining a transparent grouping boundary.

Derivation already reads world-space shapes and transforms. `WaveWall`
canonicalizes its world pose before paint, collision, and occlusion consume
it; solids expose `WaveSolid::world_shape`; sources, creatures, and
`WaveSpawn` are retained from the same recursive census. Nothing in those
paths asks whether a node came from a flat scene, a nested instance, or an
inherited scene.

Generated data follows the opposite ownership rule. `WaveRun` creates
metadata-marked `RunSeg*` walls without assigning an owner. Walls, solids,
sources, and creatures likewise build private preview, mesh, collider, and
physics limbs without assigning scene ownership. Those nodes exist in the
live tree when the engine needs them, but are not authored scene content and
must not survive packing or saving.

Existing tests prove ordinary prefab recursion, rotated ancestors, ownerless
preview limbs, in-memory packing, warning forwarding, wall collision, and
real `CUSTOM0` label writes. They do not contain a true inherited `.tscn`, do
not use `SceneState::get_base_scene_state` to prove inheritance, and do not
round-trip a generated editor preview through a saved scene file. That is the
gap this design closes.

## Decision

Add a small fixture family under
`game/tests/fixtures/scene_composition/`. It is test content, not shipped
level content.

### Fixture topology

```text
composed_level.tscn                         WaveLevel
└── PlainGroup                              plain Node3D, translated and quarter-turned
    └── InheritedRoomVariant                instance of the inherited variant
        ├── inherited base_room.tscn        plain Node3D root
        │   ├── BoundaryRun                 authored WaveRun
        │   ├── CrossWall                   authored WaveWall
        │   ├── Fan                         authored SoundFan
        │   ├── Cat                         authored WaveCat
        │   ├── Spawn                       authored WaveSpawn
        │   └── NestedProp                  nested_prop.tscn PackedScene instance
        ├── Fan authored-property override
        └── Radio                           authored child added by the inherited scene
```

`nested_prop.tscn` has a plain `Node3D` root and four `WaveProp` children:

- two known same-facing, coplanar, positive-area overlaps, using the existing
  shelf/crate geometry already proven by `observer_test.gd`; and
- two known face-to-face touching boxes whose seam must retain at least
  `MIN_SEP = 0.08` label separation.

`BoundaryRun` and `CrossWall` use the existing hand-checked T-junction shape
from `observer_test.gd`: the generated run wall and authored wall share one
genuine superface at the junction while their bend remains distinct.

`inherited_room_variant.tscn` is a real inherited scene, not a plain scene
that happens to instance `base_room.tscn`. Its root inherits the base scene,
it overrides one exported authored property on `Fan`, and it adds `Radio` as
an owned authored child. It never edits, overrides, reparents, or assigns an
owner to a generated node.

`flat_level.tscn` is an independently written `WaveLevel` fixture containing
the same typed authored gameplay nodes without grouping, nested instances, or
inheritance. Its values are hand-transformed into the equivalent level-space
coordinates. The flat fixture is an oracle, not generated from the composed
fixture and not flattened by a helper.

All introduced folders, filenames, roots, node names, tests, and documentation
headings are English.

### Authored and derived identity

The tests keep the two populations explicit:

- **Authored fixture nodes** are the known typed nodes represented by the
  fixture's `SceneState` ownership and instance graph. In a live instance,
  every non-root authored node has a non-null owner inherited from its scene
  artifact.
- **Generated nodes** are the existing ownerless `RunSeg*`, private wall
  body/skin/collider, solid skins and shapes, source blueprint limbs, and cat
  limbs, plus `WaveLevel`'s ownerless `WaveFloor` and `WaveCeiling` bodies and
  their mesh/collider limbs. They may be present and consumed at runtime, but
  they are absent from the saved `SceneState`.

A generated `RunSeg*` remains a real derived wall in `WaveLevel`'s wall,
paint, collision, and occlusion output. The test does not call it authored
merely because the live census correctly consumes it.

### Semantic comparison

One new gdUnit suite, `game/tests/scene_composition_test.gd`, loads and injects
the composed and flat fixtures before adding either to the tree. It compares
externally relevant results through explicit fixture-path-to-semantic-key
tables. It does not infer identity from leaf names and does not compare
instance IDs across the two independently instantiated levels.

“Discovered exactly once” means that each expected semantic entity occurs
once in the exposed result. It does not mean `WaveLevel` must execute exactly
one recursive walk or derive pass; editor condition watching may legitimately
walk more than once.

The suite proves:

- the explicit authored wall, generated run segment, solids, sources,
  creature, and spawn each appear exactly once in the relevant live or
  retained output;
- the plain grouping root contributes no gameplay entity;
- wall names and centerlines, wall occluder rectangles and vertical spans,
  source and creature global transforms, authored solid global shapes, and
  spawn position/yaw agree after semantic normalization;
- the authored wall's private body, mesh, collider transform, collider shape,
  and a physics ray verdict agree with the flat fixture;
- the healthy fixtures have no placement, paint, occlusion, label, or
  configuration-warning faults; and
- the inherited property override and added authored child reach the live
  instance.

The comparison includes independent absolute anchors so two identically wrong
fixtures cannot pass:

- a hand-derived expected wall path and centerline set for each fixture;
- a hand-derived expected spawn position and yaw after the composed
  translation and quarter turn;
- expected source positions and the overridden property value;
- one named genuine superface merge; and
- one named face-to-face seam that must remain separate.

The implementation plan records the literal coordinates after the fixture
values are frozen. Tests never derive the flat expectation by applying the
composed fixture's transform at runtime.

The plan also carries one explicit inventory table for every fixture path. A
row names its semantic key, authored or generated classification, owning scene
or ownerless builder, expected live multiplicity, expected retained-output
multiplicity, and whether it is forbidden from saved `SceneState`. This table
is the oracle for completeness; no helper discovers its own expected census.

### Superface and label proof

Numeric palette assignments may differ when scene order differs. The suite
therefore compares the semantic merge/separation graph, never the complete raw
label vector.

`WaveObserver::explain_oids()["superfaces"]` supplies the class membership
that the last real derive produced. It is normalized through the explicit
semantic path map. That structured graph is not sufficient proof by itself:
`faults` is a postcondition over the same class graph.

The load-bearing assertions read the actual mesh `Mesh.ARRAY_CUSTOM0` bytes:

- the named same-facing overlapping faces carry bit-identical non-placeholder
  labels within each fixture, while the independently coloured fixtures may
  choose different numeric values; and
- the named opposite-facing touching faces carry labels at least `0.08` apart
  in both fixtures.

This preserves the existing ownership of `MIN_SEP` in
`rust/src/render/labels.rs` and the merge predicate in
`rust/src/render/superface.rs`. The task does not change either law or assign
new labels.

### Editor inheritance, duplication, and persistence proof

Extend the existing
`game/tests/probe/editor_prefab_probe.gd` and its existing shell wrapper. Do
not add a new pipeline stage.

The editor-mode extension proves:

1. `PackedScene.get_state().get_base_scene_state()` is non-null for
   `inherited_room_variant.tscn`, while the nested prop is represented as an
   ordinary child scene instance.
2. `SceneState` contains the authored property override and the added owned
   child.
3. The already inherited variant and the composed top-level fixture are each
   instantiated as the main edited scene with
   `PackedScene.GEN_EDIT_STATE_MAIN`, then added to the tree before any global
   transform is read. `GEN_EDIT_STATE_MAIN_INHERITED` is not used: Godot
   reserves it for instantiating a base scene while creating another inherited
   scene, not for opening an already inherited artifact.
4. After preview generation, every known private/generated limb remains
   ownerless, including `WaveFloor`, `WaveCeiling`, and both slabs' mesh and
   collider limbs.
5. `Node.duplicate(Node.DUPLICATE_USE_INSTANTIATION)` followed by tree entry
   settles to one generated limb set, not zero and not a doubled ghost set.
   In particular, exactly one `WaveFloor` and one `WaveCeiling` remain.
6. Packing the live duplicate, saving it to a unique `user://` `.tscn`, and
   reloading with `ResourceLoader.CACHE_MODE_IGNORE_DEEP` preserves every
   authored node and the scene instance/inheritance semantics while omitting
   every generated limb, including both slabs and all of their descendants.
   The probe removes its temporary file before exit.
7. Moving a nested authored prop into a known invalid placement makes the
   editor condition watch rederive; the warning names the full nested authored
   path. Repairing the authored transform clears that warning.
8. The derive count is used only as a change-and-settle witness: it rises
   after an edit, then remains stable after the scene signature settles. No
   exact number of derive passes is part of the contract.

The probe polls concrete conditions with a bounded frame budget. It adds no
arbitrary sleep.

### No new production observability by default

The current public test surfaces are sufficient: `wall_names`,
`wall_segments`, wall rectangle/span accessors, `sources`, `cats`,
`spawn_pos`, `spawn_yaw`, configuration-warning forwarders, actual mesh
arrays, private wall children, and `WaveObserver::explain_oids`.

Do not add a canonical scene snapshot API, authoring registry, or
inheritance-aware runtime abstraction. If a failing test proves one required
result cannot be observed without mirroring production logic, stop and amend
this design before expanding the engine API.

## TDD and mutation evidence

The fixture and regression land before any production edit. Run the regression
against unchanged Rust and record whether it is red or green. A green result
is expected from the current code review and means production stays unchanged.

After the healthy regression passes, introduce each realistic mutation one at
a time, run the narrow regression to capture its failure, then restore the
original code and rerun green:

1. **Stop at a plain group:** suppress recursive descent below an untyped
   `Node3D`. Nested authored outputs disappear.
2. **Ignore an ancestor transform:** substitute a local transform at one
   world-space wall/placement boundary. Absolute wall, collision, source, or
   spawn anchors fail.
3. **Serialize generated data:** assign scene ownership to a generated
   `RunSeg*` or private limb. The round-trip `SceneState` contains a forbidden
   node.
4. **Skip or double the inherited variant:** make the recursive collector
   visit the named `InheritedRoomVariant` subtree zero or two times. Its
   inherited base descendants, `Fan` override, and added `Radio` must disappear
   or duplicate observably through semantic membership, wall tables,
   source/cat arrays, or spawn warnings. Mutating only `NestedProp` does not
   satisfy this evidence. The test does not assert an internal walk counter.

Mutation patches are never committed. Each restoration is verified before the
next mutation.

If the unchanged implementation is red, apply systematic debugging before a
fix: isolate one root-cause hypothesis, retain the failing regression, make
the smallest correction in the existing owning component, and rerun the
mutation matrix. A new Rust class, global state, scene registry, or custom
Resource is not an acceptable repair.

## Documentation

When the contract ships, update:

- the wiki's **Mechanics — Level and Objects** page with supported grouping,
  nesting, inheritance, ownership, and the files/tests owning the contract;
- the wiki's **Research — Editor Authoring** page with the measured Godot 4.7
  inheritance, duplication, and disk-round-trip result; and
- `docs/opening-in-godot.md` with the designer workflow for creating an
  inherited variant, overriding authored data, adding typed authored children,
  and never editing or taking ownership of generated limbs.

The wiki describes what ships after implementation. This spec freezes what
was approved and why; the implementation plan is a separate artifact.

## Rejected approaches

### Reorganize a shipped level as the fixture

Rejected. `level_02.tscn` is a useful smoke target, but it has no independent
flat oracle and tying exact authoring assertions to shipped content would make
ordinary level edits rewrite the regression. `level_01.tscn` is broader and
more brittle still.

### Parse or diff raw `.tscn` text

Rejected as an oracle. Godot may normalize whitespace, default-valued
properties, ownership, and inherited-node indices while saving. `SceneState`
is the official semantic surface. Text fixtures remain tracked and reviewable,
but assertions target loaded state and behaviour.

### Add a Rust room/location class or canonical snapshot service

Rejected. Composition belongs to Godot scenes and existing registered tool
nodes. A new class, custom Resource, autoload, registry, or broad snapshot API
would duplicate Godot's scene model, expand production code before a defect is
known, and violate the engine/content boundary.

### Compare complete numeric palettes

Rejected. Graph colouring may choose different safe numbers for differently
ordered but semantically equivalent trees. The visible contract is identical
superface membership and adequate separation, proved at actual mesh bytes.

## Non-goals and locked constraints

- No change to propagation, perception, visible-air cuts, occlusion admission,
  source-through/detail-knee scope, label allocation, `MIN_SEP`,
  `COPLANAR_EPS`, `PATCH_EPS`, or any acoustic/perceptual stylisation.
- No flattening, procedural room generation, runtime registry, service
  locator, mutable global state, custom Resource, autoload, or shipped
  GDScript.
- No new technology. The stack remains Godot 4.7, typed test/probe GDScript,
  GDExtension Rust, wasm, and existing repository tooling.
- `game/` remains the sole Godot project and the same source exports to web,
  macOS, and Windows.
- The result must remain platform-neutral across x86_64 and arm64 desktop
  exports and wasm32. No architecture conditional behaviour is introduced.
- Rust domain logic remains pure, total, engine-free where applicable, and
  globally stateless; registered nodes remain thin Godot boundary adapters.
- No production code is written before its failing test. If unchanged
  production passes, no speculative implementation is added.
- Every task uses the existing isolated worktree, produces small green commits,
  receives code review, and preserves repository identity.
- No assistant attribution, `Co-Authored-By`, generated-with footer, build
  output, exported artifact, rendered frame, or test report is committed.
- Integration stops at the finish-branch choice. Nothing merges, pushes, or
  deploys without the user's explicit choice; merging `main` is the automatic
  web-deployment gate.
