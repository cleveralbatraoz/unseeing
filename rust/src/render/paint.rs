//! The derive-time paint pass: turn a shape's faces into geometry that
//! carries its label as a per-vertex attribute rather than a per-instance
//! shader parameter. This is the spike the whole superface campaign rides
//! on — proving `ARRAY_CUSTOM0` round-trips through gdext's `ArrayMesh`
//! at all, headless and provably, before anything downstream depends on
//! it.
//!
//! A shared vertex cannot hold two labels, so every face gets its own four
//! corners even where two faces meet: 24 vertices for a box, not the 8 an
//! engine `BoxMesh` would share. That is the same law the doc-comment on
//! `OID_PARAM` states for the instance-uniform mechanism this replaces —
//! only now it is enforced per VERTEX rather than per solid.

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

/// Each face's four corners, as ±1 multiples of the half-extent, wound so
/// that `(v1−v0) × (v2−v0)` points along that face's own outward normal —
/// hand-derived from the cross product rather than read off any renderer,
/// so a winding mistake here cannot pass by agreeing with itself. Order
/// matches [`FACE_ORDER`].
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
        // two triangles from the quad's four corners, no vertex shared
        // with any other face
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 3);
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
pub fn resize_box_surface(mesh: &mut Gd<ArrayMesh>, size: Vector3, face_labels: [f32; 6]) {
    let arrays = labelled_box_arrays(size, Vector3::ZERO, face_labels);
    mesh.clear_surfaces();
    mesh.add_surface_from_arrays_ex(PrimitiveType::TRIANGLES, &arrays)
        .flags(custom0_format())
        .done();
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

/// The vertex/normal/CUSTOM0 arrays a triangle-list mesh (no index
/// buffer) is built from — the [`labelled_box_arrays`] of
/// [`resize_triangle_surface`].
fn triangle_arrays(triangles: &[(Vector3, Vector3, f32)]) -> Array<Variant> {
    let mut verts = PackedVector3Array::new();
    let mut normals = PackedVector3Array::new();
    let mut custom = PackedFloat32Array::new();
    for (pos, normal, ordinal) in triangles {
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
/// `(position, normal, CUSTOM0 ordinal)` triples: a shape whose triangles
/// are not already grouped into indexed quads the way [`labelled_box`]
/// builds them. Doubles as the FIRST build too, not only a later resize:
/// `clear_surfaces()` is a no-op on a mesh that has none yet, so the
/// column and wedge builders can call this once from `_ready` (on a mesh
/// built empty by `ArrayMesh::new_gd()`) and again on every knob drag,
/// with no separate "fresh mesh" path to risk drifting from this one.
/// Mutating the existing resource rather than handing back a fresh one is
/// load-bearing on every call, for the identical reason
/// [`resize_box_surface`]'s doc comment gives: `Node.duplicate()` shares a
/// mesh by reference, so a rebuild has to land on every reference that
/// mesh has.
pub fn resize_triangle_surface(mesh: &mut Gd<ArrayMesh>, triangles: &[(Vector3, Vector3, f32)]) {
    let arrays = triangle_arrays(triangles);
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

    // `resize_triangle_surface` cannot be cargo-tested past this either,
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
