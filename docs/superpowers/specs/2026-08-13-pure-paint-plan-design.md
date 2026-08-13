# Pure Paint Plan Design

The campaign's paint derivation is one atomic, pure operation. The Godot
boundary measures authored entries and sources, calls
`rust/src/render/paint_plan.rs`, and applies commands only after the complete
request returns `Ok`. `rust/src/render/paint.rs` remains the `ArrayMesh`
submission boundary.

`PaintRequest` carries world-space shapes and bounds, shape kinds, optional
level anchors, wall classification, source bounds/sweep margins/role counts,
and the candidate palette. `PaintPlan` returns one positional command for
every entry and source, the exact painted-face census, starvation owners, wall
merge owners, and indexed repairable faults. Rejected entries and sources use
`KeepExisting`; accepted ones use `Relabel`. Original census indices survive
rejection and drawless-source filtering.

Malformed entry bounds or an incorrect exact planar-face count are local,
repairable faults. A source with no area is intentionally drawless and silent.
Malformed source bounds or a NaN/infinite sweep margin are indexed source
faults; finite non-positive margins mean zero growth. Zero source roles is a
valid empty relabel command.

An empty or malformed palette, a palette pair closer than
`render::labels::MIN_SEP = 0.08`, an invalid level anchor, conflicting anchors
on a merged class, two anchored classes on a separation edge whose fixed
labels do not clear `MIN_SEP`, class-count overflow, or an invalid allocated
label rejects the whole request. Palette and level-anchor values must be finite
and inside the inclusive sRGB-safe band `[0.15, 0.96]`; the standalone
radio-preview `Role::Case = 0.05` exception never enters this level allocator.
Identical anchors on a merged class deduplicate; different bit patterns
conflict. The allocator entry point in `rust/src/render/labels.rs` is
crate-private and is no longer a public malformed-float bypass.

The merge, face, flank, source-role, label-separation, stable-order, and
starvation laws do not change. Planning remains deterministic and platform
independent: graph ties use census/class indices, never float sorting or hash
iteration. The planned algorithm retains the existing pairwise geometry and
touch complexity and performs checked capacity/class arithmetic before the
boundary mutates a mesh or source.
