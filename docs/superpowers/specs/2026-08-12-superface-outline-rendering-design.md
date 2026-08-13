# Superface outline rendering — design

**Date:** 2026-08-12
**Supersedes:** the cap-inset stopgap of
`2026-08-11-wall-junction-zfight-design.md` (issue #14). The mechanism
section and refuted-directions section of that spec remain valid and are
not restated here.

## Why the stopgap is replaced

The cap inset cures the z-fight by making geometry avoid exact coplanarity
— a hazard dodged, not a flaw removed. The user's direction: the renderer
itself must be robust to arbitrary overlap; overlapping objects must read
as one outlined form; and the change must simplify the code, not grow it.

## The decision trail (all three user calls recorded)

1. Overlap semantics: **"bends draw"** — where two objects merge, lines
   survive wherever the merged surface truly bends or steps (room corners,
   shelf edges, a crate's pierce line into a wall); flush and continuing
   surfaces melt seamlessly. ("Complete melt" was chosen first, then
   reversed once its cost was quantified: wall corners and all compound-
   furniture interior lines would vanish.)
2. Delivery: **fully per-vertex labels** — one mechanism for everything;
   the per-instance `u_oid` protocol dies.
3. Authoring voice: **warn when a prop's overlap joins it into a wall
   cluster** — walls are structural; joining them is more often an
   accident than intent. Prop–prop merges (bookcases) stay silent.

## The law

At level derive, every painted solid contributes its **faces**: a box 6, a
wedge 5, a column 3 (two rims and the curved side as one face), a slab 6.

- **Merge edges** (faces become one SUPERFACE): same axis, same outward
  facing (min-with-min or max-with-max), planes within `COPLANAR_EPS`
  (2e-3 — the derived depth-tie band at the pack-range ceiling), tangent
  rectangles overlapping by more than `PATCH_EPS`. This is exactly the
  proven z-fight predicate of `rust/src/observe/oids.rs`, promoted from
  diagnostic to production law. All faces of a superface carry ONE label,
  so the two writers of any fighting pixel hold **bit-identical G by
  construction** — the depth fight becomes invisible in all three channels
  (R and B already agree; the platform research confirmed identical
  constants survive the screen texture's sRGB round trip bit-for-bit).
- **Inequality edges** (labels at least `MIN_SEP` = 0.08 apart): faces
  that share a mesh edge within a multi-member cluster; visible face
  pairs of touching solids across members, excluding merged pairs and
  opposite-facing abutments; and — as today — anything touching across
  cluster boundaries. A box's face adjacency is 3-colourable and opposite
  faces are never co-visible, so per-solid label demand stays tiny; the
  17-wall network needs ~3 labels total.
- **Labels** are assigned by the existing graph-colouring machinery over
  the superface graph, reusing the world palette (values stay in the
  sRGB-safe band ≥ 0.25). The crease floors (0.04/0.08) and the hearing
  shader's arithmetic do not move. The shipped map has 100 clusters:
  the wall network (17), one two-wall L, two bookcases (6 each), and 96
  singletons.
- **Singletons keep today's exact look**: one label across the whole
  solid, one silhouette, no interior lines — the 1f196b1 law survives
  unchanged for everything that overlaps nothing.
- **Creatures and sources are exempt and unchanged**: the cat, hero, fan
  and radio keep their fixed bands (now owned by the rendering subsystem's
  label table, declared by ROLE — shell, moving part, case, creature) and
  never merge with the world. The fan's blades still draw against its
  housing; the cat never melts into a wall it brushes. Their geometry is
  curved/limb-built; no same-facing flat-face contact with the world
  exists, so no fight class reopens.
- **Slabs stay fixed labels** (floor 0.15, ceiling 0.90): walls ABUT the
  slabs (opposite-facing contact), so wall–floor and wall–ceiling seams
  keep drawing exactly as today.

## The rendering subsystem (decoupling, user-directed)

All of the above lives in a new `rust/src/render/` module family that owns
**how the world is seen**, separated from object logic:

- `faces` — a solid's world box/shape → its face set (pure).
- `superface` — the merge law over faces (pure; the promoted predicate).
- `labels` — the superface graph colouring against the role table (pure).
- `paint` — baking labels into mesh vertex attributes and dealing the
  skins/materials to entities (the one impure edge, called by `WaveLevel`
  after derive as a single pass).

Interface: rendering consumes a plain world description — face geometry,
entity roles, the touch/overlap relation — and returns painted meshes. It
never reads scene nodes directly and owns no physics. Object logic
(placement, census, collision, `sight` occlusion — which waves use, not
just pixels) knows nothing about labels: **`WaveSolid` loses `set_oid` and
`oid`; source classes lose their id constants** and declare roles instead.
The debug layer's `explain_oids` reads the rendering subsystem's own
census API instead of recomputing it, and reports superface classes.

Migration is staged: the label law and baking move first; material dealing
follows as its own task, so each commit stays small and green.

## Delivery: per-vertex labels

Every mesh builder writes the label as a vertex attribute (one constant
across each face; interpolation is therefore exact). Static solids are
baked in the paint pass after colouring; the cat, hero and sources bake
their role constants at build. This deletes, whole: the `u_oid` instance
uniform, every `set_instance_shader_parameter` call that carried it, and
the normal-derived fallback branch in `pack_data` — G becomes a single
attribute read. `shader_contract_test` pins the new attribute protocol,
and `filter_nearest` on the hearing pass is pinned as load-bearing (a
bilinear tap at unlucky phase halves a diff onto the dead floor).

## What is deleted or reverted (the simplification ledger)

- `CAP_INSET` and the wall end-cap inset — `wall_box` returns to full
  `WALL_T` padding; the junction tests flip to superface-merge tests.
- The bookcase tucks — `ShelfBack`/`RackBack` return to flush; flush is
  now the intended melt.
- The census's `EyeBand`, walk-bob coupling, `CREASE_FLOOR` gating and
  eye-visibility machinery — the standing invariant collapses to "no
  same-facing coplanar overlapping pair with unequal labels, on any
  plane, ever", a colouring postcondition, stricter and roughly half the
  code.
- The whole per-instance id protocol and the shader's fallback branch.
- The fixed-band constants scattered across node classes — one role table
  in `render/labels`.

## Alternatives considered and rejected

- **Complete melt** (flat per-cluster ids): smallest change, but erases
  wall corners and furniture interior lines; user reversed it on those
  grounds.
- **Cluster id + small normal offset in G**: arithmetically infeasible —
  six axis offsets at the 0.08 knee need a 0.40-wide band per cluster and
  the usable channel holds zero such bands; lowering the floors produces
  dashed seams under 10-bit quantization.
- **Geometric ribbon lines** (extracted union edges rendered after the
  post quad): feasible (~4k segments, single-digit ms at build) but adds a
  new rendering subsystem, near-plane and self-depth traps, and an
  ink-look-matching risk no gate can assert; the superface law makes it
  unnecessary.
- **CSG-unioning the level meshes**: heavy machinery, low marginal value —
  coplanar identity is the flaw, and labels fix it without touching
  geometry.

## Platform facts this design stands on (research, 2026-08-12)

- The screen texture is **RGB10_A2** (10-bit per channel), identical on
  desktop GL and WebGL2; the 3D backbuffer is blitted bit-exact and
  sampled nearest by allocation.
- The engine's sRGB round trip is deterministic: identical inputs store
  identically (the melt guarantee); id-band deltas survive (worst pair
  stretches to 0.0845); values below ≈0.027 crush to zero — labels stay
  ≥ 0.15.
- The depth buffer is 24-bit with reversed-Z semantics; `COPLANAR_EPS`'s
  2 mm derivation stands and becomes the merge tolerance.
- No MRT for user shaders on any renderer; `hint_screen_texture` is the
  only channel home — which this design does not outgrow.

## Testing

- Pure cargo TDD: face enumeration per shape; merge cases (a junction cap
  merges into the partner's flank plane; a bookcase back merges into its
  sides' rear plane); inequality cases (perpendicular faces ≥ 0.08); the
  colouring postcondition (zero unequal-label coplanar pairs); platform
  determinism (scene-order iteration, no hashing).
- The map suite's seam dichotomy: every touching pair either shares a
  superface plane (labels bit-equal) or stands ≥ 0.08 apart.
- The shipped-map zero-fights pin survives as the standing invariant, now
  enforced by construction.
- The wall-merge warning: a prop overlapped into a wall cluster names
  itself at derive.
- Post-implementation verification (not a decision gate): a rendered
  before/after probe at the spawn junction, extending the existing
  windowed probe harness.

## Risks, named

- The parallel editor-authoring branch touches the same mesh builders;
  merge order will conflict — coordinate before landing.
- The re-bake ordering (labels exist only after colouring, meshes before)
  adds one derive-time pass; its idempotence needs the same care
  `build_slabs` already documents.
- Attribute plumbing through gdext/`ImmediateMesh` vs `ArrayMesh` needs a
  spike task first: if `CUSTOM0` cannot ride the current mesh path on
  Compatibility/web, the fallback is baking labels into an unused UV
  channel — same law, different slot.

## Implementation precision erratum (2026-08-14)

Two implementation details close domains the original design left implicit
without changing its merge or seam decisions. `rust/src/render/superface.rs`
treats `Face::solid` as an opaque sparse identifier: only identifiers present
in the face slice receive cluster entries, so a huge or `usize::MAX` key cannot
become `max + 1` allocation state. Separation edges use logarithmic deterministic
membership but retain their first insertion order; the admitted dense K512
fixture contains exactly 130,816 pairs.

`rust/src/render/labels.rs` owns the renderer-number contract. Palette and
anchor separation is evaluated after narrowing both operands to the f32 lanes
written to `CUSTOM0` and performing the f32 subtraction used by the shader.
Assigned labels report that exact f32 value widened to f64 for pure diagnostics.
Thus a nominal f64 pair such as 0.31/0.39 is refused because its rendered gap is
below `MIN_SEP = 0.08`, while every accepted seam clears the actual shader knee.
