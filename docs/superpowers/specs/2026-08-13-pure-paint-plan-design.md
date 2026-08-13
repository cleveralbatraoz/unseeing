# Pure Paint Plan Design

The campaign's paint derivation is one atomic, pure operation. The Godot
boundary measures authored entries and sources, calls
`rust/src/render/paint_plan.rs`, and applies commands only after the complete
request returns `Ok`. `rust/src/render/paint.rs` remains the `ArrayMesh`
submission boundary.

`PaintRequest` carries world-space shapes, optional level anchors,
wall classification, source bounds/sweep margins/role counts, and the candidate
palette. The pure shape vocabulary derives each entry's conservative world
bound; there is no independent entry AABB that can disagree with its geometry.
`PaintPlan` returns one positional command for
every entry and source, the exact painted-face census, starvation owners, wall
merge owners, and indexed repairable faults. Rejected entries and sources use
`KeepExisting`; accepted ones use `Relabel`. Original census indices survive
rejection and drawless-source filtering.

Malformed shape-derived entry bounds or an incorrect exact planar-face count are local,
repairable faults. A source with no area is intentionally drawless and silent.
Malformed source bounds, a NaN/infinite sweep margin, or a finite margin whose
post-growth sweep overflows are indexed source faults; finite non-positive
margins mean zero growth. Zero source roles is a valid empty relabel command.

An empty or malformed palette, a palette pair closer than
`render::labels::MIN_SEP = 0.08` after the exact f32 CUSTOM0 narrowing and
subtraction performed by the shader, an invalid level anchor, conflicting anchors
on a merged class, two anchored classes on a separation edge whose fixed
labels do not clear `MIN_SEP`, class-count overflow, or an invalid allocated
label rejects the whole request. Palette and level-anchor values must be finite
and inside the inclusive sRGB-safe band `[0.15, 0.96]`; the standalone
radio-preview `Role::Case = 0.05` exception never enters this level allocator.
Identical anchors on a merged class deduplicate; different bit patterns
conflict. The allocator entry point in `rust/src/render/labels.rs` is
crate-private and is no longer a public malformed-float bypass. Assigned
labels are the exact CUSTOM0 f32 values widened back to f64, so the pure plan's
commands and diagnostics cannot claim a clearance the renderer does not have.

The merge, face, flank, source-role, label-separation, stable-order, and
starvation laws do not change. Planning remains deterministic and platform
independent: graph ties use census/class indices, never float sorting or hash
iteration. The planned algorithm retains the existing pairwise geometry and
touch complexity and performs checked capacity/class arithmetic before the
boundary mutates a mesh or source. Public requests are bounded before
quadratic work by `paint_plan::{MAX_PAINT_ENTRIES, MAX_PAINT_SOURCES,
MAX_PALETTE_VALUES, MAX_SOURCE_ROLES}`; over-limit requests return an explicit
`PaintPlanError::RequestTooLarge` atomically. Inside that admitted domain,
class and pair capacities are checked before allocation. Separation dedup uses
a deterministic ordered set for logarithmic membership while preserving the
existing insertion order, so even the admitted 512-role clique stays bounded
by its 130,816 unique pairs rather than rescanning the accumulated edge list.
The underlying `superface` graph applies the same ordered-membership pattern.
Its solid-cluster census is a sparse deterministic map keyed only by identifiers
actually present in the face input; a huge or `usize::MAX` identifier therefore
does not define an allocation length or an arithmetic successor.
