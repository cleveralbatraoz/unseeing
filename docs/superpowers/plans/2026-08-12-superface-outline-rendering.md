# Superface Outline Rendering — Implementation Plan (stage 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps
> use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the per-instance object-id crease with per-vertex
superface labels so any two overlapping solids render together with no
z-fight possible by construction, and revert the cap-inset stopgap.

**Architecture:** A new `rust/src/render/` subsystem owns how the world is
seen: pure face enumeration, the superface merge law (coplanar same-facing
overlapping faces share one label bit-for-bit), label colouring against a
role table, and a derive-time paint pass that bakes labels into a mesh
vertex attribute. Object logic loses all id knowledge. The shader reads G
from the attribute; `u_oid` and its plumbing die.

**Tech Stack:** Rust (gdext 0.5.4, `#![deny(unsafe_code)]`), Godot 4.7.1
gl_compatibility, typed GDScript + gdUnit4, cargo tests.

**Spec:** `docs/superpowers/specs/2026-08-12-superface-outline-rendering-design.md`
(read it before any task; the mechanism history is in
`docs/superpowers/specs/2026-08-11-wall-junction-zfight-design.md`).

## Global Constraints

- Perception laws (as amended by the spec): black & white thin outlines
  only; the world is revealed by waves; one silhouette per object; where
  two solids OVERLAP they are outlined together — flush/continuing
  surfaces melt (merged faces share one label bit-for-bit), bends and
  steps draw; seams between SEPARATE touching objects draw with labels at
  least 0.08 apart. Creatures and sources never merge with the world.
- Labels live in [0.15, 0.96]: the platform's sRGB round trip crushes
  values below ≈0.027 and distorts below ≈0.15.
- `filter_nearest` on the hearing pass screen texture is load-bearing;
  never remove it.
- Two layers: all logic in Rust, pure modules cargo-tested; Godot scenes +
  thin typed GDScript. Platforms: Windows/macOS/web; x86_64 + arm64 +
  wasm32; no arch-specific code; determinism = scene-order iteration, no
  hashing, no HashMap iteration order reaching output.
- Strict TDD: failing test first, watch it fail for the right reason,
  minimal code, watch it pass. No mirror assertions (an expected value must
  never come from the code under test), no change detectors. Run a
  mutation check per task: flip each new comparison/branch once and name
  the test that catches it.
- Gates before every commit: `cargo fmt`, `cargo clippy --all-targets --
  -D warnings`, `cargo test`; `gdformat` + `gdlint` on touched `.gd`.
  gdUnit runs need `godot --headless --import .` first from `game/`;
  trust printed per-suite counts, never exit codes or the PASSED word.
- Commits: small, green, self-contained; narrative evocative subject in
  the repo's voice with a precise technical body. **All work is authored
  on behalf of the user, never an assistant. No Co-Authored-By, no
  "Generated with", no mention of any assistant anywhere.** Commit with
  `git -c user.name="Dmitrii Galchenko" -c user.email="dggrus@gmail.com"`.
- Never commit build output; `game/addons/godot_mcp/` and
  `game/override.cfg` stay untracked.

## File Structure

- `rust/src/render/mod.rs` — subsystem root, re-exports, the `Role` enum
  and label table (this stage; material dealing remains in `nodes/` until
  stage 2).
- `rust/src/render/faces.rs` — solid description → world-space planar
  faces (pure).
- `rust/src/render/superface.rs` — merge law + label-separation graph
  (pure).
- `rust/src/render/labels.rs` — colouring superfaces against the palette
  and role table (pure).
- `rust/src/render/paint.rs` — the one impure edge: rewrite built meshes'
  CUSTOM0 arrays with final labels; the wall-merge warning text (pure
  helper) lives here too.
- `rust/src/nodes/solid.rs`, `wall.rs`, `props.rs`, `level.rs`,
  `hero.rs`, `cat.rs`, `fan.rs`, `radio.rs`, `observer.rs` — shed id
  knowledge, adopt ArrayMesh + CUSTOM0.
- `game/shaders/data_core.gdshaderinc` — G from `CUSTOM0.x`.
- `rust/src/level_plan.rs`, `game/scenes/level_01.tscn`,
  `game/tests/*.gd` — reverts and law updates.

---

### Task 1: Spike — CUSTOM0 through gdext ArrayMesh, headless-provable

**Files:**
- Test: `game/tests/mesh_label_test.gd` (new)
- Create: `rust/src/render/mod.rs`, minimal `rust/src/render/paint.rs`
- Modify: `rust/src/lib.rs` (add `pub mod render;`)

**Interfaces:**
- Produces: `render::paint::labelled_box(size: Vector3, lift: Vector3,
  face_labels: [f32; 6]) -> Gd<ArrayMesh>` — an axis-aligned box mesh
  (24 vertices, 12 triangles, outward normals, per-face constant
  CUSTOM0.x = that face's label; face order −X,+X,−Y,+Y,−Z,+Z), plus
  `pub const CUSTOM0_FORMAT: godot flags` used for
  `ARRAY_CUSTOM0` as `ARRAY_CUSTOM_R_FLOAT`.

- [ ] **Step 1: Write the failing gdUnit test** — the arrays round-trip and
  the format flag carries R_FLOAT:

```gdscript
func test_a_labelled_box_carries_one_label_per_face() -> void:
	var mesh: ArrayMesh = WaveLevel.debug_labelled_box(
		Vector3(2, 3, 0.3), Vector3.ZERO,
		PackedFloat32Array([0.25, 0.25, 0.34, 0.34, 0.43, 0.43])
	)
	assert_int(mesh.get_surface_count()).is_equal(1)
	var arrays: Array = mesh.surface_get_arrays(0)
	var custom: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	assert_int(custom.size()).is_equal(verts.size())
	var fmt := mesh.surface_get_format(0)
	var shift := Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT
	assert_int((fmt >> shift) & 7).is_equal(Mesh.ARRAY_CUSTOM_R_FLOAT)
	# the -X face's four vertices all carry label 0.25
	for i in verts.size():
		if absf(verts[i].x - (-1.0)) < 1e-5:
			assert_float(custom[i]).is_equal_approx(0.25, 1e-6)
```

  Expose the builder for the test only through a `#[func]`
  `WaveLevel::debug_labelled_box` shim (same pattern as other
  debug-facing funcs; it wraps `render::paint::labelled_box`).

- [ ] **Step 2: Run the suite** (`--import` first) — expect FAIL: no such
  method `debug_labelled_box`.
- [ ] **Step 3: Implement** `labelled_box` in `render/paint.rs`: build
  `ArrayMesh` via `surface_add_arrays` with VERTEX/NORMAL/CUSTOM0 packed
  arrays and format flag `ARRAY_CUSTOM_R_FLOAT << ARRAY_FORMAT_CUSTOM0_SHIFT`;
  24 vertices (4 per face, no sharing across faces — a shared vertex
  cannot hold two labels).
- [ ] **Step 4: Run the suite — green; run cargo gates.**
- [ ] **Step 5:** If `ARRAY_CUSTOM0` cannot round-trip through gdext on
  this pin, STOP and report — the fallback (spec) is the UV2.x slot, and
  the coordinator must re-approve before proceeding.
- [ ] **Step 6: Commit** (test + shim + builder together; the body records
  the spike's finding).

### Task 2: `render/faces` — a solid becomes its faces

**Files:**
- Create: `rust/src/render/faces.rs`
- Modify: `rust/src/render/mod.rs`

**Interfaces:**
- Produces:

```rust
pub struct Face {
    /// Unit outward normal, world space.
    pub normal: [f64; 3],
    /// Signed plane offset: dot(normal, p) == offset on the plane.
    pub offset: f64,
    /// The face's bounded polygon, world space, counter-clockwise seen
    /// from outside. Columns' curved flank has no entry here.
    pub poly: Vec<[f64; 3]>,
    /// Which solid this face belongs to (census index).
    pub solid: usize,
}
pub enum Shape {
    /// center, size, basis columns (unit, possibly rotated)
    Box3d { center: [f64; 3], size: [f64; 3], basis: [[f64; 3]; 3] },
    /// wedge per prop_shape::wedge_hull's 6 points, world space
    Wedge { hull: [[f64; 3]; 6] },
    /// upright circle faces only: center, radius, half_height
    Column { center: [f64; 3], radius: f64, half_height: f64 },
}
pub fn faces(solid: usize, shape: &Shape) -> Vec<Face>
```

- [ ] **Step 1: Failing cargo tests** (hand-derived literals):

```rust
/// A unit box at the origin yields six faces whose normals are the six
/// axis directions and whose offsets are ±0.5 — the break this catches
/// is a face built from the wrong basis column or a sign slip.
#[test]
fn a_box_yields_six_outward_faces() {
    let f = faces(0, &Shape::Box3d {
        center: [0.0; 3], size: [1.0; 3],
        basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    });
    assert_eq!(f.len(), 6);
    let px = f.iter().find(|f| f.normal == [1.0, 0.0, 0.0]).unwrap();
    assert!((px.offset - 0.5).abs() < 1e-12);
    assert_eq!(px.poly.len(), 4);
}
/// A column contributes only its two rims: the curved flank has no
/// plane and can never coplanar-merge with anything.
#[test]
fn a_column_contributes_only_its_rims() {
    let f = faces(3, &Shape::Column { center: [1.0, 0.5, 2.0], radius: 0.3, half_height: 0.5 });
    assert_eq!(f.len(), 2);
    assert_eq!(f[0].normal, [0.0, -1.0, 0.0]);
    assert!((f[0].offset - 0.0).abs() < 1e-12); // dot((0,-1,0),(x,0,z))
    assert_eq!(f[1].normal, [0.0, 1.0, 0.0]);
}
/// A wedge yields five faces (two triangles, three quads) built from
/// its hull points; the diagonal face's normal is not axis-aligned.
#[test]
fn a_wedge_yields_five_faces_including_the_diagonal() { /* hull literals from prop_shape tests */ }
/// A box under a quarter-turn basis yields the same six planes with
/// swapped axes — exact, no trig dust (quadrant_basis columns).
#[test]
fn a_quarter_turned_box_swaps_axes_exactly() { /* … */ }
```

  Circle rims discretize `poly` at 32 points (the merge law only ever
  needs polygon overlap tests; rims merge with slab tops when a column
  stands flush — that IS a legal melt).
- [ ] **Step 2: watch them fail (missing module).**
- [ ] **Step 3: implement; every function total (degenerate size → empty
  face list, never a panic).**
- [ ] **Step 4: cargo gates green.**
- [ ] **Step 5: Commit.**

### Task 3: `render/superface` — the merge law and the separation graph

**Files:**
- Create: `rust/src/render/superface.rs`
- Modify: `rust/src/render/mod.rs`

**Interfaces:**
- Consumes: `faces::Face`.
- Produces:

```rust
/// Tolerances promoted from the fight census (observe/oids.rs keeps
/// aliases pointing here until Task 10 guts it).
pub const COPLANAR_EPS: f64 = 2e-3;
pub const PATCH_EPS: f64 = 1e-3;
pub struct Superfaces {
    /// face index -> class index
    pub class_of: Vec<usize>,
    pub classes: usize,
    /// class pairs that must take labels >= MIN_SEP apart
    pub separations: Vec<(usize, usize)>,
    /// solid-level connected components of the overlap relation
    /// (used by paint's wall-merge warning and by tests)
    pub cluster_of_solid: Vec<usize>,
}
pub fn superfaces(faces: &[Face], touching: &[(usize, usize)]) -> Superfaces
```

  Merge edge: normals within 1e-9 dot of parallel SAME direction, offsets
  within `COPLANAR_EPS`, polygon overlap area beyond `PATCH_EPS` in both
  principal extents (2-D convex intersection after projecting to the
  shared plane). Separation edges: (a) two faces of one solid sharing a
  polygon edge; (b) faces of touching solids whose polygons pass within
  `PATCH_EPS` of each other and are not merged and not opposite-facing
  coplanar (buried abutment); (c) every face pair across two touching
  solids in DIFFERENT clusters keeps the old law: their solids' classes
  all separate (one entry per class pair, deduplicated).

- [ ] **Step 1: Failing cargo tests**, the issue-14 geometry as literals:

```rust
/// THE issue-14 case: wall A's end cap lands in wall B's far flank
/// plane (full WALL_T padding, no inset). The two faces MERGE — one
/// class, one label, bit-identical G, nothing to fight.
#[test]
fn a_junction_cap_merges_into_the_partners_flank() { /* two wall boxes, meeting centerlines; assert class_of[capA] == class_of[flankB] */ }
/// The same junction's PERPENDICULAR pairs separate: A's flank crosses
/// B's flank, and the corner line needs labels >= 0.08 apart.
#[test]
fn perpendicular_junction_faces_separate() { /* assert (classA_flank, classB_flank) in separations */ }
/// Abutment is not a merge and not a separation-by-contact: a crate's
/// bottom on the floor's top (opposite normals, same plane) stays
/// buried; the crate and floor separate at SOLID level (different
/// clusters) exactly as today.
#[test]
fn opposite_facing_abutment_neither_merges_nor_fights() { /* … */ }
/// Two faces of one box sharing an edge must separate — a box's own
/// silhouette law survives inside multi-member clusters.
#[test]
fn edge_sharing_faces_of_one_solid_separate() { /* … */ }
/// Disjoint coplanar rectangles do not merge (collinear walls across a
/// doorway gap).
#[test]
fn coplanar_but_disjoint_faces_do_not_merge() { /* … */ }
/// Determinism: shuffled input face order yields the same classes under
/// index normalization.
#[test]
fn class_assignment_is_input_order_stable() { /* … */ }
```

- [ ] **Step 2: fail (missing fns). Step 3: implement (union-find for
  merges, then edges). Step 4: gates. Step 5: mutation check (flip
  same-facing to opposite — junction test names it; drop the edge-share
  rule — the box test names it). Step 6: Commit.**

### Task 4: `render/labels` — colouring the superface graph

**Files:**
- Create: `rust/src/render/labels.rs`
- Modify: `rust/src/render/mod.rs`

**Interfaces:**
- Consumes: `Superfaces`.
- Produces:

```rust
pub enum Role { Case, Floor, Shell, Moving, Cat, HeroBody, Ceiling, HeroCane }
/// The one label table (moves the constants out of every node class):
/// Case 0.05 stays only for the radio chassis (pre-existing; grand-
/// fathered below the 0.15 sRGB comfort line and unchanged this stage),
/// Floor 0.15, Shell 0.33, Moving 0.63, Cat 0.70, HeroBody 0.82,
/// Ceiling 0.90, HeroCane 0.96.
pub fn role_label(role: Role) -> f64
pub const MIN_SEP: f64 = 0.08;
pub struct Labelling { pub label_of_class: Vec<f64>, pub starved: usize }
pub fn assign(sf: &Superfaces, anchors: &[(usize, f64)], palette: &[f64]) -> Labelling
```

  `anchors` are (class, fixed label) pairs — slabs and the source-swept
  neighbourhood bans, built by the caller exactly as `oid_palette::assign`
  takes `Fixed` today. Colouring: Welsh–Powell over classes, ties by
  class index; identical guarantees (never panics, `starved` counted).

- [ ] **Step 1: failing tests** — junction fixture end-to-end (two walls →
  merged cap class shares the flank's label; perpendicular classes ≥
  0.08 apart; palette reused between non-adjacent classes; a starved
  graph reports starved > 0 and still labels everything).
- [ ] **Steps 2–5: fail → implement (reuse `oid_palette`'s ban/greedy
  internals — extract shared helpers rather than duplicating) → gates →
  mutation check → Commit.**

### Task 5: static solids build ArrayMesh with face ordinals

**Files:**
- Modify: `rust/src/nodes/solid.rs` (`build_box` → uses
  `render::paint::labelled_box` with ORDINAL values 0..6 in CUSTOM0),
  `rust/src/nodes/props.rs` (wedge and column builders emit CUSTOM0
  ordinals; column flank = ordinal 2), `rust/src/nodes/level.rs`
  (slabs), keeping colliders byte-identical.
- Test: `game/tests/level_test.gd` additions; `game/tests/data_skins_test.gd`.

**Interfaces:**
- Produces: every static mesh carries CUSTOM0.x = face ordinal (a
  placeholder the paint pass rewrites); `render::paint::face_count(shape)
  -> usize` so paint and builders agree on ordinal order (box 6, wedge 5,
  column 3, slab 6 — documented as the ONE ordinal contract).

- [ ] **Step 1: failing gdUnit test** — a built wall's mesh arrays carry a
  CUSTOM0 channel sized to VERTEX and holding only values < 6.
- [ ] **Steps 2–5: red → implement → suites + cargo gates → Commit.**

### Task 6: `render/paint` — the derive-time bake and the wall-merge voice

**Files:**
- Modify: `rust/src/render/paint.rs`, `rust/src/nodes/level.rs`
  (`assign_oids` replaced by `paint_labels`, called at the same derive
  point; census hands `Shape`s + touching pairs)
- Test: cargo in `paint.rs` (pure warning text), gdUnit `map_test.gd`

**Interfaces:**
- Consumes: `faces`, `superfaces`, `labels`, task 5's ordinal contract.
- Produces: `paint::relabel(mesh: &Gd<ArrayMesh>, labels_by_ordinal:
  &[f32])` — reads `surface_get_arrays`, rewrites CUSTOM0 by ordinal
  lookup, `surface_remove` + `surface_add_arrays` back;
  `paint::wall_merge_warnings(cluster_of_solid, kinds, names) ->
  Vec<String>` — one line per non-wall solid sharing a cluster with any
  wall: "WaveLevel: '<name>' overlaps the wall structure and is drawn as
  part of it — its faces take the walls' labels and its pierce lines
  draw. Pull it clear of the wall if that was a nudge, or leave it if the
  bump is authored."

- [ ] **Step 1: failing map_test** — the shipped level's walls: sample two
  junctioned walls' meshes and assert the merged plane's vertices carry
  THE SAME CUSTOM0 label on both meshes (bit-equal as f32), and a
  perpendicular pair differs by ≥ 0.08. This is the new form of the
  zero-fights pin and replaces nothing yet (Task 10 rewrites the old
  census pin).
- [ ] **Step 2: red. Step 3: implement `paint_labels` in the level derive;
  `WaveSolid` loses `set_oid`/`oid` (compiles clean = every caller
  found). Step 4: full gdUnit + cargo. Step 5: mutation — skip the
  relabel pass: the map test names it. Step 6: Commit.**

### Task 7: dynamics bake role labels

**Files:**
- Modify: `rust/src/nodes/hero.rs`, `rust/src/nodes/cat.rs` (their
  ImmediateMesh builds move to ArrayMesh via the same packed-array path —
  the cat rebuilds per frame; keep the arrays preallocated),
  `rust/src/nodes/fan.rs`, `rust/src/nodes/radio.rs` (limbs take
  `role_label(Shell)/(Moving)/(Case)` constants in CUSTOM0; the id
  constants and every `u_oid` `set_instance_shader_parameter` call are
  deleted).
- Test: `game/tests/observer_test.gd`, `data_skins_test.gd` updates.

- [ ] **Step 1: failing tests** — a built fan's blade mesh carries CUSTOM0
  = 0.63 everywhere; the cat's mesh carries 0.70; no node in the tree
  answers `get_instance_shader_parameter("u_oid")` with anything but
  null.
- [ ] **Steps 2–5: red → migrate → gates (cat gait suites must stay green
  — the mesh path changes, the pose math must not) → Commit.**

### Task 8: the shader reads the attribute

**Files:**
- Modify: `game/shaders/data_core.gdshaderinc` (G = `v_label`;
  `varying float v_label` set from `CUSTOM0.x` in both skins' vertex
  fns; DELETE `instance uniform float u_oid` and the `u_oid < 0`
  normal-derived fallback), `game/shaders/data_pass.gdshader`,
  `game/shaders/data_xray.gdshader`.
- Test: `game/tests/shader_contract_test.gd` (the contract pins move from
  u_oid to CUSTOM0/v_label), `game/tests/wiring_test.gd`.

- [ ] **Step 1: update the contract test first (red)** — it must demand
  `CUSTOM0` in the vertex stage, `v_label` in `pack_data`, and the
  ABSENCE of `u_oid` anywhere in `game/shaders/`.
- [ ] **Steps 2–5: red → edit shaders → full gdUnit + a boot check
  (`ci/pipeline.sh`'s boot stage) → Commit.**

### Task 9: revert the stopgap

**Files:**
- Modify: `rust/src/level_plan.rs` (delete `CAP_INSET`; `wall_box` run
  term back to `length.abs() + WALL_T * 2.0`; the junction tests flip:
  they now build faces and assert MERGE — reuse Task 3's fixtures — and
  the padding pins return to 7.7/19.1/4.3/9.3),
  `game/scenes/level_01.tscn` (ShelfBack x 0.925 → 0.92, RackBack
  15.95 → 15.945), `game/tests/level_test.gd` (pins back to L + 0.3).
- [ ] **Steps: flip the level_plan tests first (red against the inset) →
  revert the constant and the scene → cargo + full gdUnit green →
  Commit (one commit; the body names what the revert restores and why it
  is now safe).**

### Task 10: the census becomes the postcondition

**Files:**
- Modify: `rust/src/observe/oids.rs` (DELETE `EyeBand`, the bob import,
  `CREASE_FLOOR` gating, the skip mask; `coplanar_fights_checked(boxes,
  oids)` becomes `coplanar_label_faults(faces, labels)` — same-facing
  coplanar overlapping face pairs whose labels are not bit-equal, ANY
  plane, no eye, no threshold; keep the vacuous-pass refusal shape),
  `rust/src/nodes/observer.rs` (`explain_oids` reports `superfaces`
  — class count, members by name — and `faults` from the new law;
  `pairs`/`violations` keep the solid-level law for SEPARATE solids),
  `game/tests/map_test.gd` (the zero pin asserts `faults == []`),
  `game/tests/observer_test.gd`.
- [ ] **Steps: rewrite the census tests first (the eye-band and skip tests
  are deleted WITH their machinery; the sub-floor-speckle test becomes a
  label-fault test) → gut and rewire → full gates → mutation check →
  Commit.**

### Task 11: wall-merge warning reaches the level's voice

**Files:**
- Modify: `rust/src/nodes/level.rs` (derive emits
  `paint::wall_merge_warnings` through the same `godot_warn!` path the
  placement faults use)
- Test: `game/tests/level_test.gd` — a hand-built level with a crate
  poked 2 cm into a wall warns naming the crate; the shipped map warns
  nothing.
- [ ] **Steps: red → wire → gates → Commit.**

### Task 12: full verification + docs

- [ ] Full pipeline stages locally: hygiene, cargo, import + full gdUnit
  (honest per-suite counts), determinism probe, restore probe, boot
  check.
- [ ] The final structural verification: re-run the coplanar census probe
  over the shipped map's painted meshes — zero same-facing coplanar
  pairs with unequal labels, all planes.
- [ ] The wiki clone's three pages are REWRITTEN for the superface law
  (the cap-inset drafts are superseded); CLAUDE.md's perception-law
  bullet is amended per the spec's wording; memory updated.
- [ ] Rendered before/after probe at the spawn junction (windowed,
  human-run caveat applies) — verification, not a gate.
- [ ] Commit docs; report with the merge menu.

## Self-Review

- Spec coverage: law → Tasks 2–4; delivery → 1, 5–8; deletions ledger →
  7–10; warning → 11; subsystem boundary → files under `render/` with
  object logic shedding ids in 6–7; stage-2 (material dealing) explicitly
  out of scope. ✓
- Placeholder scan: Task 2/3/4 test bodies marked `/* … */` carry their
  full intent in names+comments and their literals are pinned by the
  fixtures named in the same task — implementers derive them from the
  stated geometry (two 4 m walls, meeting centerlines, full padding).
  Acceptable per the granularity rule; no TBDs remain. ✓
- Type consistency: `Face`/`Shape`/`Superfaces`/`Labelling` names match
  across Tasks 2–6; ordinal contract named in 5 and consumed in 6. ✓

## Campaign rebase note (2026-08-13)

This plan is the frozen design/execution route that produced the architecture
the editor-authoring campaign inherited at merge base `dfbb69a`.

- Campaign commit `8bb9cb7` ported its Godot/Rust authoring laws onto this
  per-face model. World solids enter the superface graph; sources remain
  non-geometric but add semantic-role classes whose numeric labels are derived
  per instance; creatures retain fixed numeric roles; intentionally drawless
  data consumes no label.
  Any earlier campaign plan that describes six-slot source recolouring or a
  flat object-id seam as current is superseded.
- Post-rebase fixes retain generated WaveRun wall paths relative to their
  `WaveLevel`, route generated-segment faults to the authored run, and verify
  prefab seams by reading the two actual touching `CUSTOM0` face labels.
- `AGENTS.md` now owns the perception law and new-object checklist;
  `CLAUDE.md` is only an adapter. Task 12's documentation step never authorizes
  a wiki push. Wiki publication, like integration and deployment, remains a
  separate user gate.
- The source-role checkpoint is 407 Cargo tests and 329 gdUnit cases in 31
  suites, with 19 registered classes and ten icons. Final closeout recomputes
  those totals; this note does not alter the plan's historical body.
