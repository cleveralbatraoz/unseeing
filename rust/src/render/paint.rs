//! The mesh paint/submission boundary: turn geometry into `ArrayMesh`
//! surfaces that carry labels as per-vertex attributes rather than
//! per-instance shader parameters. `WaveLevel` uses it during derivation to
//! repaint world faces; source, creature, and viewmodel builders use the same
//! boundary when they submit their semantic-role meshes. The pure geometry,
//! graph, and label laws stay in the modules these adapters call.
//!
//! A shared vertex cannot hold two labels, so every face gets its own four
//! corners even where two faces meet: 24 vertices for a box, not the 8 an
//! engine `BoxMesh` would share. That is the same "two touching solids
//! sharing an id have no line between them" law the old per-instance
//! object-id uniform this replaces relied on — only now it is enforced per
//! VERTEX rather than per solid.

use godot::classes::ArrayMesh;
use godot::classes::mesh::{ArrayCustomFormat, ArrayFormat, ArrayType, PrimitiveType};
use godot::prelude::*;

/// The face order every per-face label array is read in, and the order
/// [`labelled_box`] emits its four-vertex quads in: −X, +X, −Y, +Y, −Z, +Z.
pub const FACE_ORDER: [Vector3; 6] = [
    Vector3::new(-1.0, 0.0, 0.0),
    Vector3::new(1.0, 0.0, 0.0),
    Vector3::new(0.0, -1.0, 0.0),
    Vector3::new(0.0, 1.0, 0.0),
    Vector3::new(0.0, 0.0, -1.0),
    Vector3::new(0.0, 0.0, 1.0),
];

/// Each face's four corners, as ±1 multiples of the half-extent, in the
/// mathematical counter-clockwise order where `(v1−v0) × (v2−v0)` points
/// along that face's own outward normal. [`labelled_box_arrays`] reverses
/// each submitted triangle at the Godot boundary because the engine calls
/// CLOCKWISE front-facing; keeping that renderer convention out of this
/// table lets the pure face geometry retain the usual outward-cross law.
/// Order matches [`FACE_ORDER`].
const FACE_CORNERS: [[Vector3; 4]; 6] = [
    // -X: (v1-v0)=(0,0,2hz), (v2-v0)=(0,2hy,2hz) -> cross = (-4hy*hz,0,0)
    [
        Vector3::new(-1.0, -1.0, -1.0),
        Vector3::new(-1.0, -1.0, 1.0),
        Vector3::new(-1.0, 1.0, 1.0),
        Vector3::new(-1.0, 1.0, -1.0),
    ],
    // +X: (v1-v0)=(0,2hy,0), (v2-v0)=(0,2hy,2hz) -> cross = (4hy*hz,0,0)
    [
        Vector3::new(1.0, -1.0, -1.0),
        Vector3::new(1.0, 1.0, -1.0),
        Vector3::new(1.0, 1.0, 1.0),
        Vector3::new(1.0, -1.0, 1.0),
    ],
    // -Y: (v1-v0)=(2hx,0,0), (v2-v0)=(2hx,0,2hz) -> cross = (0,-4hx*hz,0)
    [
        Vector3::new(-1.0, -1.0, -1.0),
        Vector3::new(1.0, -1.0, -1.0),
        Vector3::new(1.0, -1.0, 1.0),
        Vector3::new(-1.0, -1.0, 1.0),
    ],
    // +Y: (v1-v0)=(0,0,2hz), (v2-v0)=(2hx,0,2hz) -> cross = (0,4hx*hz,0)
    [
        Vector3::new(-1.0, 1.0, -1.0),
        Vector3::new(-1.0, 1.0, 1.0),
        Vector3::new(1.0, 1.0, 1.0),
        Vector3::new(1.0, 1.0, -1.0),
    ],
    // -Z: (v1-v0)=(0,2hy,0), (v2-v0)=(2hx,2hy,0) -> cross = (0,0,-4hy*hx)
    [
        Vector3::new(-1.0, -1.0, -1.0),
        Vector3::new(-1.0, 1.0, -1.0),
        Vector3::new(1.0, 1.0, -1.0),
        Vector3::new(1.0, -1.0, -1.0),
    ],
    // +Z: (v1-v0)=(2hx,0,0), (v2-v0)=(2hx,2hy,0) -> cross = (0,0,4hx*hy)
    [
        Vector3::new(-1.0, -1.0, 1.0),
        Vector3::new(1.0, -1.0, 1.0),
        Vector3::new(1.0, 1.0, 1.0),
        Vector3::new(-1.0, 1.0, 1.0),
    ],
];

/// The vertex/normal/CUSTOM0/index arrays a labelled box is built from —
/// factored out of [`labelled_box`] so [`resize_box_surface`] can rewrite
/// an EXISTING mesh's surface with the identical geometry a fresh call to
/// [`labelled_box`] would build, rather than the two risking drift apart.
fn labelled_box_arrays(size: Vector3, lift: Vector3, face_labels: [f32; 6]) -> Array<Variant> {
    let half = Vector3::new(size.x * 0.5, size.y * 0.5, size.z * 0.5);

    let mut verts = PackedVector3Array::new();
    let mut normals = PackedVector3Array::new();
    let mut custom = PackedFloat32Array::new();
    let mut indices = PackedInt32Array::new();

    for face in 0..FACE_ORDER.len() {
        let normal = FACE_ORDER[face];
        let label = face_labels[face];
        let base = verts.len() as i32;
        for corner in &FACE_CORNERS[face] {
            let pos = Vector3::new(corner.x * half.x, corner.y * half.y, corner.z * half.z) + lift;
            verts.push(pos);
            normals.push(normal);
            custom.push(label);
        }
        // Two triangles from the quad's four corners, no vertex shared
        // with any other face. FACE_CORNERS is mathematically CCW/outward;
        // Godot calls CLOCKWISE front-facing, so reverse each triangle only
        // in the submitted index order. The vertex blocks themselves stay
        // in FACE_ORDER for paint/read-back stability.
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 1);
        indices.push(base);
        indices.push(base + 3);
        indices.push(base + 2);
    }

    let mut arrays = Array::<Variant>::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &verts.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::CUSTOM0.ord() as usize, &custom.to_variant());
    arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
    arrays
}

/// An axis-aligned box, 24 vertices and 12 triangles, carrying one constant
/// label per face in `ARRAY_CUSTOM0` — the spike's proof object. `size` is
/// the box's full extent; `lift` translates every vertex, so several boxes
/// can later be baked into one shared `ArrayMesh` without each needing its
/// own node transform. `face_labels` is read in [`FACE_ORDER`]: −X, +X,
/// −Y, +Y, −Z, +Z.
pub fn labelled_box(size: Vector3, lift: Vector3, face_labels: [f32; 6]) -> Gd<ArrayMesh> {
    let arrays = labelled_box_arrays(size, lift, face_labels);
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays_ex(PrimitiveType::TRIANGLES, &arrays)
        .flags(custom0_format())
        .done();
    mesh
}

/// Rewrite `mesh`'s single surface IN PLACE to a box of `size` — the
/// resize half of every box-shaped static solid (a wall, a free prop, a
/// floor or ceiling slab). Mutating the existing resource rather than
/// handing back a fresh one is load-bearing, not a style choice:
/// `Node.duplicate()` shares a mesh by REFERENCE
/// (`nodes::solid::clear_limbs`'s own doc comment), so a knob drag has to
/// land on every reference that mesh has, not just the one the resizing
/// node happens to hold — replacing the resource outright would leave a
/// stale ghost limb frozen at its old size instead of resizing with the
/// original, which is the exact bug `clear_limbs` exists to guard against
/// after a duplicate re-enters the tree.
/// `face_labels` is what a mesh with NOTHING to carry over is built with —
/// the placeholder ordinals, on a first build. A mesh that already has a
/// surface keeps whatever CUSTOM0 it is wearing ([`carry_labels_over`]),
/// which after the level's derive-time bake is the solid's real per-face
/// labels: the label lives in the mesh now, and a knob drag must not
/// silently undo the paint pass.
pub fn resize_box_surface(mesh: &mut Gd<ArrayMesh>, size: Vector3, face_labels: [f32; 6]) {
    let mut arrays = labelled_box_arrays(size, Vector3::ZERO, face_labels);
    carry_labels_over(mesh, &mut arrays);
    mesh.clear_surfaces();
    mesh.add_surface_from_arrays_ex(PrimitiveType::TRIANGLES, &arrays)
        .flags(custom0_format())
        .done();
}

/// Replace the freshly built `arrays`' CUSTOM0 with the one `mesh` already
/// carries, whenever it has one of exactly the same length.
///
/// This is what makes a STATIC SOLID's resize keep the derive-time paint.
/// The label used to live in a per-instance shader uniform and survived a
/// resize for free; it lives in the mesh now, so a wall, a free prop, a
/// slab, a column or a wedge rebuilding its own surface has to carry it,
/// or a designer dragging a knob after `_ready` hands the shader raw
/// ordinals (1.0 .. 5.0) where a label in [0.15, 0.96] belongs — creases
/// at every internal face boundary, and a seam judged against a number no
/// colouring ever chose.
///
/// ONLY THOSE FIVE. The creatures, the viewmodel and the sound sources
/// rebuild triangle surfaces too — `nodes::hero`'s cane and body,
/// `nodes::cat`'s whole mesh, `nodes::fan`/`nodes::radio`'s limbs — and
/// they must NOT come through here, which is why the triangle path keeps
/// its label-carrying static door separate from both non-carry doors.
/// Two reasons, both load-bearing. Those builders choose their
/// own label every call (a fixed `render::role_label`, baked into every
/// vertex), so there is nothing to carry and a carry would silently freeze
/// the first build's labels forever — their tessellations are
/// fixed-resolution, so the length always matches and the branch always
/// fires. And they run EVERY FRAME: `WaveHero::update` is called
/// unconditionally from the composition root's `_process`, `WaveCat`
/// rebuilds on nearly every physics frame, and `surface_get_arrays` below
/// is an engine round trip that reconstructs the whole vertex/normal/CUSTOM0
/// set (a synchronous device read on the RD backends) for ~3.9k + ~4.9k
/// vertices, only to throw it away.
///
/// POSITIONAL, and sound because every resize on the paths that DO come
/// through here is shape-preserving in its vertex list: a box is always
/// [`FACE_ORDER`]'s six quads in order (24 vertices), a column always
/// `COLUMN_SEGMENTS * 12`, a wedge always its eight triangles — the knobs
/// move where vertices ARE, never how many there are or which face each
/// belongs to. The length check is the guard for that assumption rather
/// than a formality: a future shape whose tessellation follows its size
/// would fail it and fall back to the placeholders, which the level's next
/// derive repaints.
///
/// What it does NOT do, and must not be read as doing: re-derive. The
/// superface partition is a level-wide decision taken once, in
/// `WaveLevel::derive`, and a solid that changes size after that may
/// genuinely belong in a different merge cluster. Carrying the labels
/// keeps the solid in band and keeps its OLD separations; it does not
/// recompute them.
fn carry_labels_over(mesh: &Gd<ArrayMesh>, arrays: &mut Array<Variant>) {
    if mesh.get_surface_count() == 0 {
        return;
    }
    let slot = ArrayType::CUSTOM0.ord() as usize;
    let Some(Ok(kept)) = mesh
        .surface_get_arrays(0)
        .get(slot)
        .map(|v| v.try_to::<PackedFloat32Array>())
    else {
        return;
    };
    let Some(Ok(fresh)) = arrays.get(slot).map(|v| v.try_to::<PackedFloat32Array>()) else {
        return;
    };
    if kept.len() == fresh.len() {
        arrays.set(slot, &kept.to_variant());
    }
}

/// The shape kinds a static solid's builder paints — one entry per shape
/// [`crate::render::faces::Shape`] describes, plus [`ShapeKind::Slab`],
/// which has no `Shape` variant of its own: a floor or ceiling is built
/// straight through [`crate::nodes::solid::build_box`], the same box path
/// a wall or a free prop takes, never through `render::faces`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Box,
    Wedge,
    Column,
    Slab,
}

/// How many CUSTOM0 ordinals a shape's mesh carries — the ONE contract
/// every builder ([`crate::nodes::solid`], [`crate::nodes::props`],
/// [`crate::nodes::level`]) and the later derive-time paint pass (Task 6's
/// `relabel`) must read the same way: an ordinal a builder writes at or
/// past this count, or a `labels_by_ordinal` slice shorter than it, is
/// exactly the bug this number exists to catch before it ships as a
/// silently wrong face.
///
/// A box or a slab matches [`FACE_ORDER`]'s six entries. A wedge matches
/// `render::faces::wedge_faces`'s five: floor, tall back wall, slope, and
/// the two triangular ends. A column's three is its two flat rims —
/// bottom then top, `render::faces::column_faces`'s own order — plus its
/// curved flank, which has no plane and so no entry in `faces()` at all;
/// the flank still needs its OWN ordinal so it never shares a vertex with
/// a rim it meets.
#[must_use]
pub fn face_count(kind: ShapeKind) -> usize {
    match kind {
        ShapeKind::Box | ShapeKind::Slab => 6,
        ShapeKind::Wedge => 5,
        ShapeKind::Column => 3,
    }
}

/// Recover the immutable face ordinal for one vertex from the builder's
/// layout, never from the mutable CUSTOM0 value the previous paint pass
/// already replaced with a real label. This is what makes painting total
/// and idempotent when the editor re-derives after a drag.
#[must_use]
pub fn vertex_ordinal(kind: ShapeKind, vertex: usize) -> Option<usize> {
    match kind {
        ShapeKind::Box | ShapeKind::Slab => (vertex < 24).then_some(vertex / 4),
        ShapeKind::Wedge => crate::prop_shape::WEDGE_TRIANGLE_ORDINALS
            .get(vertex / 3)
            .map(|ordinal| *ordinal as usize),
        ShapeKind::Column => {
            if vertex >= crate::prop_shape::COLUMN_SEGMENTS * 12 {
                return None;
            }
            Some(match vertex % 12 {
                0..=2 => 0,
                3..=5 => 1,
                _ => 2,
            })
        }
    }
}

/// Build the vertex/normal/CUSTOM0 arrays for a triangle-list mesh with no
/// index buffer. `reverse_for_godot` converts conventional outward triples;
/// already-clockwise animated limbs pass `false`.
fn triangle_arrays(
    triangles: &[(Vector3, Vector3, f32)],
    reverse_for_godot: bool,
) -> Array<Variant> {
    let mut verts = PackedVector3Array::new();
    let mut normals = PackedVector3Array::new();
    let mut custom = PackedFloat32Array::new();
    // Pure prop/source generators use the conventional CCW/outward law,
    // while animated limb builders already emit Godot-clockwise order. Keep
    // that distinction explicit at the direct and outward public doors.
    // All shipped callers provide complete triples; chunks_exact makes a malformed
    // internal tail harmless rather than indexing or panicking.
    let mut chunks = triangles.chunks_exact(3);
    for triangle in &mut chunks {
        let order = if reverse_for_godot {
            [0, 2, 1]
        } else {
            [0, 1, 2]
        };
        for corner in order {
            let (pos, normal, ordinal) = triangle[corner];
            verts.push(pos);
            normals.push(normal);
            custom.push(ordinal);
        }
    }
    for (pos, normal, ordinal) in chunks.remainder() {
        // Preserve malformed internal data for diagnostics/validator
        // visibility; never guess a triangle or panic at this total boundary.
        verts.push(*pos);
        normals.push(*normal);
        custom.push(*ordinal);
    }
    let mut arrays = Array::<Variant>::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &verts.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::CUSTOM0.ord() as usize, &custom.to_variant());
    arrays
}

/// Rewrite `mesh`'s single surface IN PLACE — no index buffer — from
/// `(position, normal, CUSTOM0 label)` triples: a shape whose triangles
/// are not already grouped into indexed quads the way [`labelled_box`]
/// builds them. Mutating the existing resource rather than handing back a
/// fresh one is load-bearing on every call, for the identical reason
/// [`resize_box_surface`]'s doc comment gives: `Node.duplicate()` shares a
/// mesh by reference, so a rebuild has to land on every reference that
/// mesh has.
///
/// THE WRITE-THROUGH DOOR, and it serves exactly one population: callers
/// that already know the label they want on every vertex, and choose it
/// afresh on every call. That is every per-frame builder in the game
/// (`nodes::hero`'s cane and body, `nodes::cat`'s mesh). Each already emits
/// Godot-clockwise geometry and bakes one fixed [`super::role_label`] value
/// straight into the triples. Conventional CCW/outward generators use
/// [`resize_outward_triangle_surface`] instead.
///
/// **A STATIC SOLID DOES NOT BELONG HERE.** A wall, prop, slab, column or
/// wedge wears the label the level's derive chose for it, which lives
/// nowhere but the mesh — routing a new one through this door would
/// silently overwrite that with whatever placeholder its builder happened
/// to pass. [`resize_outward_triangle_surface_preserving_labels`] is its
/// door; see [`carry_labels_over`] for why label carry is explicit.
pub fn resize_triangle_surface(mesh: &mut Gd<ArrayMesh>, triangles: &[(Vector3, Vector3, f32)]) {
    submit_triangle_arrays(mesh, triangle_arrays(triangles, false));
}

/// The conventional-geometry sibling of [`resize_triangle_surface`].
/// `triangles` wind counter-clockwise when seen from outside, so their
/// cross product follows the stored outward normal. Godot calls CLOCKWISE
/// front-facing; reverse each complete triple only as it enters ArrayMesh.
/// Pure prop/source generators use this door; animated limbs must not.
pub fn resize_outward_triangle_surface(
    mesh: &mut Gd<ArrayMesh>,
    triangles: &[(Vector3, Vector3, f32)],
) {
    submit_triangle_arrays(mesh, triangle_arrays(triangles, true));
}

/// [`resize_outward_triangle_surface`], but keeping whatever CUSTOM0 the
/// mesh already carries when the vertex count is unchanged — so a knob
/// dragged after derive does not undo the paint pass. Incoming triples use
/// conventional CCW/outward order; their ordinals are only placeholders on
/// a FIRST build. Columns and wedges are the current callers.
pub fn resize_outward_triangle_surface_preserving_labels(
    mesh: &mut Gd<ArrayMesh>,
    triangles: &[(Vector3, Vector3, f32)],
) {
    let mut arrays = triangle_arrays(triangles, true);
    carry_labels_over(mesh, &mut arrays);
    submit_triangle_arrays(mesh, arrays);
}

/// Replace `mesh`'s single surface with `arrays` — the submission half all
/// three entry points above share verbatim, kept in one place so they can
/// never drift apart on the surface flag.
fn submit_triangle_arrays(mesh: &mut Gd<ArrayMesh>, arrays: Array<Variant>) {
    mesh.clear_surfaces();
    mesh.add_surface_from_arrays_ex(PrimitiveType::TRIANGLES, &arrays)
        .flags(custom0_format())
        .done();
}

/// The surface flag that marks `ARRAY_CUSTOM0` as one 32-bit float per
/// vertex (`ARRAY_CUSTOM_R_FLOAT << ARRAY_FORMAT_CUSTOM0_SHIFT`) rather
/// than the compressed-byte default `add_surface_from_arrays` assumes when
/// no flag is given. A plain `const` cannot hold this: `ArrayCustomFormat`
/// and `ArrayFormat` read their ordinals through the `EngineEnum` /
/// `EngineBitfield` traits, and trait dispatch is not available in a
/// `const` context on this toolchain — so the value is composed once, by
/// name, on every call instead of ever being hand-copied as a bare
/// integer.
pub fn custom0_format() -> ArrayFormat {
    let r_float = ArrayCustomFormat::R_FLOAT.ord() as u64;
    let shift = ArrayFormat::CUSTOM0_SHIFT.ord();
    ArrayFormat::from_ord(r_float << shift)
}

/// The derive-time bake: rewrite `mesh`'s existing (single-surface) CUSTOM0
/// channel in place with the real label the paint pass chose for each
/// face. The ordinal is recovered from [`vertex_ordinal`]'s immutable
/// builder layout, not read from CUSTOM0: after the first derive that
/// channel already carries labels, and treating a second pass's label as
/// an ordinal would collapse every value below 1.0 onto face zero. Every
/// other array (VERTEX, NORMAL, INDEX) is read straight back off the mesh
/// and resubmitted byte for byte, so the geometry a designer sees never
/// moves — only the G channel changes underneath it.
///
/// A vertex outside the shape's known layout, or an ordinal at or past
/// `labels_by_ordinal.len()`, keeps its current value rather than panicking:
/// a wrong colour is recoverable mid derive, a panic is not. A mesh with
/// no surface yet (a knob dragged before `_ready`) is a no-op for the same
/// reason.
pub fn relabel(mesh: &mut Gd<ArrayMesh>, kind: ShapeKind, labels_by_ordinal: &[f32]) {
    if mesh.get_surface_count() == 0 {
        return;
    }
    let arrays = mesh.surface_get_arrays(0);
    let Some(custom_variant) = arrays.get(ArrayType::CUSTOM0.ord() as usize) else {
        return;
    };
    let Ok(mut custom) = custom_variant.try_to::<PackedFloat32Array>() else {
        return;
    };
    for (vertex, label) in custom.as_mut_slice().iter_mut().enumerate() {
        if let Some(real) =
            vertex_ordinal(kind, vertex).and_then(|ordinal| labels_by_ordinal.get(ordinal))
        {
            *label = *real;
        }
    }
    let mut new_arrays = arrays.clone();
    new_arrays.set(ArrayType::CUSTOM0.ord() as usize, &custom.to_variant());
    mesh.surface_remove(0);
    mesh.add_surface_from_arrays_ex(PrimitiveType::TRIANGLES, &new_arrays)
        .flags(custom0_format())
        .done();
}

/// Rewrite every vertex of a source limb with one chosen role-class label.
/// Source builders emit one semantic role per limb, so unlike [`relabel`]
/// there is no face ordinal to recover: the whole existing CUSTOM0 channel
/// changes together. Geometry, normals and indices are resubmitted unchanged.
/// Non-finite labels and meshes with no readable surface/channel are no-ops.
pub fn relabel_constant(mesh: &mut Gd<ArrayMesh>, label: f32) {
    if !label.is_finite() || mesh.get_surface_count() == 0 {
        return;
    }
    let arrays = mesh.surface_get_arrays(0);
    let Some(custom_variant) = arrays.get(ArrayType::CUSTOM0.ord() as usize) else {
        return;
    };
    let Ok(mut custom) = custom_variant.try_to::<PackedFloat32Array>() else {
        return;
    };
    custom.as_mut_slice().fill(label);
    let mut new_arrays = arrays.clone();
    new_arrays.set(ArrayType::CUSTOM0.ord() as usize, &custom.to_variant());
    mesh.surface_remove(0);
    mesh.add_surface_from_arrays_ex(PrimitiveType::TRIANGLES, &new_arrays)
        .flags(custom0_format())
        .done();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every face's own winding is hand-derived in the comments beside
    /// [`FACE_CORNERS`]; this test is the independent half — it recomputes
    /// the same cross product from the emitted vertices themselves rather
    /// than trusting the comment, so a transcription slip in the table
    /// (not a slip in the derivation) still fails loudly.
    #[test]
    fn every_face_winds_outward() {
        for face in 0..FACE_ORDER.len() {
            let corners = &FACE_CORNERS[face];
            let edge1 = corners[1] - corners[0];
            let edge2 = corners[2] - corners[0];
            let computed = edge1.cross(edge2);
            let normal = FACE_ORDER[face];
            // the cross product's magnitude is the corner spacing (4, on
            // this ±1 cube), so compare direction: same sign on the one
            // nonzero axis the normal names
            let dot = computed.dot(normal);
            assert!(
                dot > 0.0,
                "face {face}: winding {computed:?} does not point along outward normal {normal:?}"
            );
        }
    }

    /// The ordinal contract's four counts, hand-derived from each shape's
    /// own vocabulary rather than read off `face_count` reproducing
    /// itself: a box's six faces, a wedge's five (`render::faces::wedge_faces`),
    /// a column's two rims plus its one curved flank, and a slab sharing
    /// the box's six because it is built through the same box path.
    #[test]
    fn face_count_matches_each_shapes_own_vocabulary() {
        assert_eq!(face_count(ShapeKind::Box), 6);
        assert_eq!(face_count(ShapeKind::Slab), 6);
        assert_eq!(face_count(ShapeKind::Wedge), 5);
        assert_eq!(face_count(ShapeKind::Column), 3);
    }

    /// Repainting is driven by immutable vertex layout, not by whatever
    /// real label CUSTOM0 already carries from the previous derive. These
    /// hand-derived sequences pin every static-solid layout so an editor
    /// watch can paint a second time without treating labels below 1.0 as
    /// ordinal zero and collapsing the mesh.
    #[test]
    fn every_static_shape_recovers_ordinals_from_vertex_layout() {
        assert_eq!(vertex_ordinal(ShapeKind::Box, 0), Some(0));
        assert_eq!(vertex_ordinal(ShapeKind::Box, 3), Some(0));
        assert_eq!(vertex_ordinal(ShapeKind::Box, 4), Some(1));
        assert_eq!(vertex_ordinal(ShapeKind::Box, 23), Some(5));
        assert_eq!(vertex_ordinal(ShapeKind::Slab, 23), Some(5));

        assert_eq!(vertex_ordinal(ShapeKind::Column, 0), Some(0));
        assert_eq!(vertex_ordinal(ShapeKind::Column, 2), Some(0));
        assert_eq!(vertex_ordinal(ShapeKind::Column, 3), Some(1));
        assert_eq!(vertex_ordinal(ShapeKind::Column, 5), Some(1));
        assert_eq!(vertex_ordinal(ShapeKind::Column, 6), Some(2));
        assert_eq!(vertex_ordinal(ShapeKind::Column, 383), Some(2));

        let wedge = [0, 0, 1, 1, 2, 2, 3, 4];
        for (triangle, expected) in wedge.into_iter().enumerate() {
            for corner in 0..3 {
                assert_eq!(
                    vertex_ordinal(ShapeKind::Wedge, triangle * 3 + corner),
                    Some(expected)
                );
            }
        }

        assert_eq!(vertex_ordinal(ShapeKind::Box, 24), None);
        assert_eq!(vertex_ordinal(ShapeKind::Column, 384), None);
        assert_eq!(vertex_ordinal(ShapeKind::Wedge, 24), None);
    }

    // The ArrayMesh resize doors cannot be cargo-tested past this either,
    // for the identical reason `labelled_box` below cannot: it too
    // reaches a live `Gd<ArrayMesh>`. The triples it merely copies are
    // where the real geometry is built and where it stays pure and
    // cargo-tested — `prop_shape::column_triangles`, and the ordinal
    // lookup beside `prop_shape::wedge_triangles`.

    // `labelled_box` itself is NOT cargo-tested beyond this: it constructs
    // a live `Gd<ArrayMesh>`, and gdext refuses engine calls outside a
    // running Godot process — confirmed empirically, not assumed:
    // `godot-ffi-0.5.4/src/lib.rs:605` panics "Godot engine not available;
    // make sure you are not calling it from unit/doc tests" the instant a
    // cargo test reaches `ArrayMesh::new_gd()`. That is exactly the split
    // this module's doc comment describes — [`labelled_box`] is the one
    // impure edge, proved headless through gdUnit
    // (`game/tests/mesh_label_test.gd`) instead.
}
