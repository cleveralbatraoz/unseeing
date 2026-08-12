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

/// An axis-aligned box, 24 vertices and 12 triangles, carrying one constant
/// label per face in `ARRAY_CUSTOM0` — the spike's proof object. `size` is
/// the box's full extent; `lift` translates every vertex, so several boxes
/// can later be baked into one shared `ArrayMesh` without each needing its
/// own node transform. `face_labels` is read in [`FACE_ORDER`]: −X, +X,
/// −Y, +Y, −Z, +Z.
pub fn labelled_box(size: Vector3, lift: Vector3, face_labels: [f32; 6]) -> Gd<ArrayMesh> {
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

    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays_ex(PrimitiveType::TRIANGLES, &arrays)
        .flags(custom0_format())
        .done();
    mesh
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
