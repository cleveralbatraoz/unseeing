# Wall-junction z-fight — design (issue #14)

**Date:** 2026-08-11
**Issue:** #14 — wall intersections draw a jagged "hazard" crease when a
wave reveals them (regression from `1f196b1`).

## The proven mechanism

Established entirely from structured data — geometry census over the derived
wall boxes, plus `WaveObserver.explain_oids` against the running level — with
no rendered frame:

1. `level_plan::wall_box` pads a wall's centerline by `WALL_T = 0.15` at
   each END as well as each side. When a wall's centerline terminates ON a
   partner's centerline (every junction in the shipped map), the end cap
   lands **exactly in the partner's far flank plane**.
2. That produces, on the shipped map, **27 exactly-coplanar overlapping
   face pairs on vertical planes, all SAME-facing** (identical outward
   normal), one to two per junction, each 0.3 m wide and full height — plus
   36 horizontal ones (wall tops/bottoms) no standing eye can ever see, and
   12 from two flush-authored furniture assemblies (`ShelfBack`/`ShelfSide*`,
   `RackBack`/`RackSide*`).
3. The graph colouring **guarantees** touching walls carry ids at least
   0.08 apart (measured 0.09–0.18 on every junction pair). The two coplanar
   writers agree in R (same world point), B (same distance) and normal
   (same facing) and differ **only in G** — the object id.
4. `data_pass.gdshader` renders `cull_disabled` with a real depth test, so
   both faces rasterise; per-pixel depth-interpolation noise alternates the
   winner; G speckles between two ids ≥ 0.09 apart; the crease term
   (`smoothstep(0.04, 0.08, nrm)`) saturates per pixel; `reveal` gates the
   result — the jagged band that flares when a wave sweeps the wall.

## Directions refuted by the census (no GPU needed)

- **`cull_back`** (the issue's "cheapest probe"): every one of the 27
  vertical pairs is SAME-facing — both faces are front faces from the
  visible side. Culling back faces removes none of them.
- **Per-wall depth bias / render priority**: the `InnerEast`/`InnerSouth`
  L-corner needs `InnerEast` to win on plane `x = 14.15` and `InnerSouth`
  to win on plane `z = 15.75` — a cycle no per-instance ordering satisfies.
- **Same id for continuous surfaces**: reverses `0d3dcf3`'s law and leaves
  the depth fight in place (invisible only while ids agree — fragile).

## The chosen fix: deterministic winner by cap inset

`wall_box` run-axis padding becomes `WALL_T - CAP_INSET`
(`CAP_INSET = 0.005`): every end cap retreats 5 mm. At a junction the cap
now sits strictly INSIDE the partner's box, so the partner's flank wins the
depth test **deterministically at every pixel** — no coplanar pair, no
fight. The far-side flank reads as one uninterrupted surface (the
pre-`1f196b1` look); the seam between the two walls still draws, as the id
crease along the visible corner line where the flanks meet. This is the
issue's "deterministic depth" direction implemented in geometry, where it
is testable in pure Rust, rather than in render state, where it is not.

What it deliberately does NOT change:

- **Occlusion**: the occluder table derives from centerlines
  (`sight::wall_rect`), untouched. Waves and sight lines behave
  identically.
- **The touch graph / colouring**: junction boxes still interpenetrate by
  0.295 m ≫ `TOUCH_EPS`; every junction pair still touches and still takes
  distinct ids.
- **Doorways**: a free end retreats 5 mm; authored gaps widen by 1 cm —
  imperceptible.

Accepted residual: at an L-corner both caps retreat, leaving a
5 × 5 mm vertical notch at the convex corner — sub-pixel beyond ~1 m and
masked by the corner's own drawn crease line. One interior L-corner exists
(`InnerEast`/`InnerSouth`); the four border corners face the void.

Scene half: `ShelfBack` and `RackBack` are tucked 5 mm behind their side
panels' plane — same cure, authored geometry.

## The standing gate (and the observability payback)

The battle test of the debug layer found its hole: `explain_oids` answers
"will the seam draw" but not "will the surface fight". A new pure law
reports every same-facing coplanar overlapping face pair on a **vertical**
plane whose id delta clears the crease threshold (horizontal planes are
excluded on argument: this game's eye lives strictly between floor and
ceiling, and a horizontal fight is visible only from above or below its
plane). It is exposed through `explain_oids` and pinned to **zero** for the
shipped map in the suite, so the artifact class cannot land again silently.

## Amended after in-branch review (2026-08-11)

Three statements above did not survive the branch that implemented them.
The verdicts stand; the corrected reasoning is recorded here rather than
rewritten above, so the original decision stays legible as decided.

- **The standing gate is eye-BAND-aware, not a blanket exclusion.** The
  shipped law does not exclude horizontal planes "because the eye lives
  strictly between floor and ceiling" — that argument was false: a
  tabletop pair below the eye is in plain view. The law censuses X and Z
  planes always, and a horizontal pair by where the walking eye actually
  sweeps (`player::EYE ± viewmodel::BOB_AMP`, the band [1.572, 1.628]):
  an upward (max-max) pair fights iff its plane is below the bob crest,
  a downward (min-min) pair iff its plane is above the bob trough, both
  edges exclusive (at an edge the bob's extreme meets the plane edge-on,
  zero projected area). Floor-flush bottoms and wall tops still never
  census — for that reason, not the false one.
- **Five interior L-corners, not one.** The shipped map's interior
  L-corners are `DividerNorth`/`FanRoomSouth` (6.4, 8),
  `FanRoomSouth`/`InnerEast` (14, 8), `InnerEast`/`InnerSouth`
  (14, 15.6), `PartyEastSouth`/`PartySouthEast` (19.4, 19.4) and
  `StoreNorth`/`StoreEast` (21.4, 22.4) — all carrying the identical
  5 × 5 mm notch in walkable space. The border corners face the void.
- **The notch is invisible because nothing can DRAW in it, not because
  it is sub-pixel.** At the shipped 66° vertical FOV the notch subtends
  about 4 px at 1 m and drops sub-pixel only beyond ~4–6 m. What keeps
  it dark is the render maths: its depth steps (at most ~7 mm) sit two
  orders of magnitude under the 0.012 silhouette floor
  (`hearing_post.gdshader`), and each exposed cap strip carries its own
  wall's id, so the only id step present is the corner's own crease —
  the line the corner draws anyway.
