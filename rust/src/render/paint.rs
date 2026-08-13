//! The derive-time paint pass: turn a shape's faces into geometry that
//! carries its label as a per-vertex attribute rather than a per-instance
//! shader parameter. This is the spike the whole superface campaign rides
//! on — proving `ARRAY_CUSTOM0` round-trips through gdext's `ArrayMesh`
//! at all, headless and provably, before anything downstream depends on
//! it.
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

use super::faces::Face;
use super::superface::Superfaces;

/// A class/separation graph's separation edges — [`Superfaces::separations`]'
/// own shape, named here so [`add_flank_classes`] and [`add_anchor_classes`]
/// don't each restate the same three-deep generic.
type Separations = Vec<(usize, usize)>;

/// The `(class, fixed label)` pairs [`super::labels::assign`]'s own
/// `anchors` parameter expects.
type Anchors = Vec<(usize, f64)>;

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
/// This is what makes a resize keep the derive-time paint. The label used
/// to live in a per-instance shader uniform and survived a resize for
/// free; it lives in the mesh now, so every path that rewrites the surface
/// has to carry it, or a designer dragging a knob after `_ready` hands the
/// shader raw ordinals (1.0 .. 5.0) where a label in [0.15, 0.96] belongs
/// — creases at every internal face boundary, and a seam judged against a
/// number no colouring ever chose.
///
/// POSITIONAL, and sound because every resize in this vocabulary is
/// shape-preserving in its vertex list: a box is always [`FACE_ORDER`]'s
/// six quads in order (24 vertices), a column always `COLUMN_SEGMENTS *
/// 12`, a wedge always its eight triangles — the knobs move where vertices
/// ARE, never how many there are or which face each belongs to. The length
/// check is the guard for that assumption rather than a formality: a
/// future shape whose tessellation follows its size would fail it and fall
/// back to the placeholders, which the level's next derive repaints.
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
///
/// The CUSTOM0 ordinals in `triangles` are the placeholders a FIRST build
/// needs; a rebuild carries over whatever the mesh already wears, the same
/// way [`resize_box_surface`] does and for the same reason — see
/// [`carry_labels_over`].
pub fn resize_triangle_surface(mesh: &mut Gd<ArrayMesh>, triangles: &[(Vector3, Vector3, f32)]) {
    let mut arrays = triangle_arrays(triangles);
    carry_labels_over(mesh, &mut arrays);
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
/// channel in place, replacing each vertex's PLACEHOLDER face ordinal — the
/// value a builder wrote at build time, task 5's ordinal contract — with
/// the real label `labels_by_ordinal[ordinal]` the paint pass chose for
/// that face. Every other array (VERTEX, NORMAL, INDEX) is read straight
/// back off the mesh and resubmitted byte for byte, so the geometry a
/// designer sees never moves — only the G channel changes underneath it.
///
/// An ordinal at or past `labels_by_ordinal.len()` — a mesh built from a
/// wider ordinal table than the caller's labels cover, or a mesh this pass
/// was never meant to touch — keeps its placeholder value rather than
/// panicking on an out-of-range read: a wrong colour is recoverable mid
/// derive, a panic is not. A mesh with no surface yet (a knob dragged
/// before `_ready`) is a no-op for the same reason.
pub fn relabel(mesh: &mut Gd<ArrayMesh>, labels_by_ordinal: &[f32]) {
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
    for label in custom.as_mut_slice() {
        let ordinal = *label as usize;
        if let Some(&real) = labels_by_ordinal.get(ordinal) {
            *label = real;
        }
    }
    let mut new_arrays = arrays.clone();
    new_arrays.set(ArrayType::CUSTOM0.ord() as usize, &custom.to_variant());
    mesh.surface_remove(0);
    mesh.add_surface_from_arrays_ex(PrimitiveType::TRIANGLES, &new_arrays)
        .flags(custom0_format())
        .done();
}

/// One warning line per non-wall solid sharing a MERGE CLUSTER with any
/// wall — the visible half of the merge law: a solid whose faces genuinely
/// coplanar-overlap a wall's own is no longer outlined as its own object at
/// all; it takes the wall's labels and its pierce lines draw as though it
/// were part of the wall structure. Whether that is a stray nudge or an
/// authored bump is a call only a person can make, so the level only names
/// it.
///
/// `cluster_of_solid`, `is_wall` and `names` are parallel, one entry per
/// combined census item (the same triple order [`Superfaces::cluster_of_solid`]
/// and the level's own census walk describe) — a length mismatch across the
/// three is a caller error, handled totally by walking only their common
/// prefix rather than panicking on the first out-of-range index.
///
/// Deterministic in CENSUS order — never a set's own iteration order, the
/// same discipline every other derived diagnostic in this crate holds to.
pub fn wall_merge_warnings(
    cluster_of_solid: &[usize],
    is_wall: &[bool],
    names: &[String],
) -> Vec<String> {
    let n = cluster_of_solid.len().min(is_wall.len()).min(names.len());
    let mut wall_clusters: Vec<usize> = Vec::new();
    for i in 0..n {
        if is_wall[i] && !wall_clusters.contains(&cluster_of_solid[i]) {
            wall_clusters.push(cluster_of_solid[i]);
        }
    }
    let mut warnings = Vec::new();
    for i in 0..n {
        if !is_wall[i] && wall_clusters.contains(&cluster_of_solid[i]) {
            warnings.push(format!(
                "WaveLevel: '{}' overlaps the wall structure and is drawn as part of it — its \
                 faces take the walls' labels and its pierce lines draw. Pull it clear of the \
                 wall if that was a nudge, or leave it if the bump is authored.",
                names[i]
            ));
        }
    }
    warnings
}

/// Record `(a, b)` as a separated class pair, normalized to `(min, max)`
/// and deduplicated — the identical rule [`superface`]'s own
/// `add_separation` enforces, duplicated here on purpose rather than
/// exposed from that module: the two callers solve different problems (one
/// builds separations FROM geometry, these build them FROM a shape that
/// has none), and this crate's pure-module doctrine keeps each
/// self-contained rather than reaching for a shared internal helper — the
/// same choice `superface.rs`'s own doc comment explains for its duplicated
/// vector helpers.
fn add_sep(seps: &mut Vec<(usize, usize)>, a: usize, b: usize) {
    if a == b {
        return;
    }
    let pair = if a < b { (a, b) } else { (b, a) };
    if !seps.contains(&pair) {
        seps.push(pair);
    }
}

/// Extend `sf` with one extra class per column flank — PERMANENTLY
/// SINGLETON, and thus separated, ONLY when the column's own solid
/// belongs to a MULTI-MEMBER cluster; a column alone in its own cluster
/// (its two rims never won a real merge — the ordinary case, since a
/// rim resting on anything presents an OPPOSITE-facing surface, a buried
/// abutment, never a same-direction coplanar overlap) instead JOINS its
/// solid's own singleton-collapsed class, exactly as
/// [`super::superface::superfaces`]'s own collapse now does for every
/// other kind of solid: today's look, one uniform label across rim and
/// flank alike.
///
/// THE GAP this closes: [`super::faces::column_faces`] emits only a
/// column's two flat rims — the curved flank has no plane at all, so it
/// never enters `faces` and can never win a real merge or separation edge
/// through the geometric law [`Superfaces`] implements. Left uncoloured, a
/// column standing flush on the floor would draw its rim's label onto its
/// own flank too (whatever placeholder ordinal the mesh still carried) and
/// — for a column whose rims genuinely belong to a multi-member cluster —
/// the rim/flank seam simply would not draw.
///
/// THE RULE for a MULTI-MEMBER column, and why it is sound: a flank never
/// merges with anything (it has no polygon a merge or a distance test
/// could run against), so it is always its own class, separated —
///   - from every OTHER class that solid's own REAL faces (its two rims)
///     belong to: the same "a box's own corner never disappears" law
///     [`superface`]'s rule (a) states for two faces sharing a polygon
///     edge, generalised to a face with no polygon to share one — a flank
///     meets both of its own rims at a genuine seam (the rim's own outer
///     circle) exactly as two box faces meet at a corner, so the two must
///     always separate;
///   - from every class ANY TOUCHING solid's real faces belong to,
///     BLANKET rather than fine-grained: with no polygon to test an
///     overlap or a distance against, there is no way to ask "does this
///     flank's curve actually meet that neighbour's face" the way two
///     planar faces can be asked, so the safe direction — matching rule
///     (c)'s own existing blanket law for solids that never merged at all
///     — is to always draw the line rather than risk a silent melt;
///   - from every OTHER column's flank it touches: two barrels standing
///     flush would otherwise be free to land on the identical label where
///     their curves meet, which is exactly the seam the whole-box touch
///     graph (the retired `oid_palette::assign`, superseded by this
///     module and [`super::labels::assign`]) already protected and this
///     campaign must not regress.
///
/// THE RULE for a SINGLETON column: the flank ALIASES the class its own
/// rims already collapsed to (`sf.class_of` of any of the solid's real
/// faces — they are all the identical class by construction) rather than
/// winning a fresh one. No separation is added for it at all: whatever
/// `sf.separations` already gave that collapsed class — rule (c)'s
/// blanket law against a touching neighbour in a DIFFERENT cluster, the
/// only kind a singleton column can have — the flank now inherits simply
/// by sharing the class number, exactly as its two rims already do.
///
/// `flank_solids` names which combined-census solid indices are columns —
/// solids that contributed exactly two real faces (their rims) to `faces`
/// — in the SAME order the caller reads the returned flank classes back in.
/// Total: an empty `flank_solids` returns `sf`'s own class count and
/// separations completely unchanged, so a level with no columns pays
/// nothing extra. A `flank_solids` entry naming a solid with no real face
/// in `faces` at all (never true of an actual column, which always
/// contributes its two rims) falls back to a fresh class — SAFE rather
/// than merely non-panicking: separated from every touching neighbour's
/// real classes the identical way a multi-member flank always is, so an
/// orphan flank cannot silently melt into whatever it stands beside.
fn separate_from_touching_neighbours(
    separations: &mut Vec<(usize, usize)>,
    this_class: usize,
    solid: usize,
    faces: &[Face],
    sf: &Superfaces,
    touching: &[(usize, usize)],
) {
    for &(a, b) in touching {
        let other = if a == solid {
            Some(b)
        } else if b == solid {
            Some(a)
        } else {
            None
        };
        let Some(other) = other else { continue };
        for (i, face) in faces.iter().enumerate() {
            if face.solid == other {
                add_sep(separations, this_class, sf.class_of[i]);
            }
        }
    }
}

pub fn add_flank_classes(
    sf: &Superfaces,
    faces: &[Face],
    touching: &[(usize, usize)],
    flank_solids: &[usize],
) -> (Vec<usize>, usize, Separations) {
    let mut classes = sf.classes;
    let mut separations = sf.separations.clone();
    let mut flank_class = Vec::with_capacity(flank_solids.len());

    // singleton detection: the identical law `superface::superfaces`'s
    // own collapse uses — a solid alone in its own cluster.
    let cluster_span = sf
        .cluster_of_solid
        .iter()
        .copied()
        .max()
        .map_or(0, |m| m + 1);
    let mut cluster_size = vec![0usize; cluster_span];
    for &c in &sf.cluster_of_solid {
        cluster_size[c] += 1;
    }
    let is_singleton = |solid: usize| -> bool {
        sf.cluster_of_solid
            .get(solid)
            .is_some_and(|&c| cluster_size[c] == 1)
    };

    for &solid in flank_solids {
        if is_singleton(solid) {
            // a lone column's flank JOINS its solid's single collapsed
            // class — today's look, one uniform label across rim and
            // flank alike. Whatever rule (c) already separated that
            // class from (a touching neighbour, in whatever DIFFERENT
            // cluster a singleton's neighbour always is) the flank now
            // inherits automatically, by aliasing rather than adding
            // anything new.
            let joined = faces.iter().position(|f| f.solid == solid);
            flank_class.push(match joined {
                Some(i) => sf.class_of[i],
                None => {
                    // total, and SAFE in the constraining direction: no
                    // real face at all for this solid — never true of an
                    // actual column (`render::faces::column_faces` always
                    // emits its two rims, degenerate radius/height and
                    // all), but a caller could still hand `flank_solids`
                    // an index nothing built faces for. Nothing to alias
                    // onto, so this wins a fresh class — but a FRESH,
                    // UNCONSTRAINED class would be free to land on
                    // whatever slot a touching neighbour already uses,
                    // silently melting into it. Give it the same blanket
                    // "separate from everything touching" rule the
                    // multi-member branch's own touching bullet applies
                    // below, erring toward drawing a line rather than
                    // risking that melt.
                    let this_class = classes;
                    classes += 1;
                    separate_from_touching_neighbours(
                        &mut separations,
                        this_class,
                        solid,
                        faces,
                        sf,
                        touching,
                    );
                    this_class
                }
            });
            continue;
        }

        let this_class = classes;
        classes += 1;
        flank_class.push(this_class);

        for (i, face) in faces.iter().enumerate() {
            if face.solid == solid {
                add_sep(&mut separations, this_class, sf.class_of[i]);
            }
        }

        separate_from_touching_neighbours(&mut separations, this_class, solid, faces, sf, touching);
    }

    for (a_idx, &solid_a) in flank_solids.iter().enumerate() {
        for (b_idx, &solid_b) in flank_solids.iter().enumerate().skip(a_idx + 1) {
            let touch = touching
                .iter()
                .any(|&(x, y)| (x == solid_a && y == solid_b) || (x == solid_b && y == solid_a));
            if touch {
                add_sep(&mut separations, flank_class[a_idx], flank_class[b_idx]);
            }
        }
    }

    (flank_class, classes, separations)
}

/// Extend a class/separation graph with one extra, FIXED-label class per
/// entry of `extra_anchors` — the mechanism that preserves the retired
/// `oid_palette::Fixed`'s old law (a sound source's own ids ban the world
/// palette entries near them for whatever touches it) now that a source
/// contributes no real face to the census at all (its limbs bake their
/// role labels directly into `CUSTOM0`, never through the census):
/// `render::labels::role_label` puts the world palette's 0.34 within a
/// centimetre of `Role::Shell`'s 0.33, and without a ban a wall or a crate
/// touching a source's swept envelope would be free to land there.
///
/// Each `(label, touching_classes)` pair becomes one phantom class fixed to
/// `label`, separated from every class named in `touching_classes` — the
/// caller has already resolved which REAL (and flank) classes belong to
/// whatever touches the source's own swept box; this function only wires
/// the separation edges and returns the `(class, label)` pairs
/// [`super::labels::assign`]'s own `anchors` parameter expects.
///
/// Total: an empty `extra_anchors` returns `classes`/`separations`
/// unchanged and no anchors — a level with no sound sources pays nothing
/// extra.
pub fn add_anchor_classes(
    classes: usize,
    separations: &[(usize, usize)],
    extra_anchors: &[(f64, Vec<usize>)],
) -> (usize, Separations, Anchors) {
    let mut classes = classes;
    let mut separations = separations.to_vec();
    let mut anchors = Vec::with_capacity(extra_anchors.len());
    for (label, touching_classes) in extra_anchors {
        let this_class = classes;
        classes += 1;
        anchors.push((this_class, *label));
        for &c in touching_classes {
            add_sep(&mut separations, this_class, c);
        }
    }
    (classes, separations, anchors)
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

    // ---------------------------------------------------------------
    // wall_merge_warnings
    // ---------------------------------------------------------------

    /// No wall touches anything: no warning, whatever else is in the
    /// census — the break this catches is a warning fired off cluster
    /// membership alone, ignoring `is_wall` entirely.
    #[test]
    fn no_warning_when_nothing_shares_a_wall_cluster() {
        let clusters = vec![0, 1, 2];
        let is_wall = vec![true, false, false];
        let names = vec![
            "WallA".to_string(),
            "Crate".to_string(),
            "Shelf".to_string(),
        ];
        assert!(wall_merge_warnings(&clusters, &is_wall, &names).is_empty());
    }

    /// A crate sharing a wall's cluster warns, naming the crate — never the
    /// wall, and never twice for one crate even though it shares the
    /// cluster with the wall by construction (only one wall in this
    /// fixture, but the dedup on `wall_clusters` is what stops a crate
    /// spanning several walls' worth of cluster from repeating).
    #[test]
    fn a_solid_sharing_a_wall_cluster_warns_naming_it() {
        let clusters = vec![0, 0, 1];
        let is_wall = vec![true, false, false];
        let names = vec![
            "WallA".to_string(),
            "WallCrate".to_string(),
            "FarProp".to_string(),
        ];
        let warnings = wall_merge_warnings(&clusters, &is_wall, &names);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("'WallCrate'"));
        assert!(!warnings[0].contains("FarProp"));
    }

    /// A WALL sharing a cluster with another wall — an ordinary junction,
    /// once one actually merges — never warns about itself: the law is
    /// about non-wall geometry drawn as part of the wall structure, not
    /// about walls meeting walls, which is the whole point of the merge
    /// law in the first place.
    #[test]
    fn two_merged_walls_never_warn_about_each_other() {
        let clusters = vec![0, 0];
        let is_wall = vec![true, true];
        let names = vec!["WallA".to_string(), "WallB".to_string()];
        assert!(wall_merge_warnings(&clusters, &is_wall, &names).is_empty());
    }

    /// The exact message text the brief pins, checked so a future edit to
    /// the wording is a deliberate, reviewed change rather than a drift
    /// gdUnit alone would have to catch.
    #[test]
    fn the_warning_names_the_solid_in_the_exact_wording() {
        let clusters = vec![0, 0];
        let is_wall = vec![true, false];
        let names = vec!["North".to_string(), "Shelf".to_string()];
        let warnings = wall_merge_warnings(&clusters, &is_wall, &names);
        assert_eq!(
            warnings,
            vec![
                "WaveLevel: 'Shelf' overlaps the wall structure and is drawn as part of it — its \
                 faces take the walls' labels and its pierce lines draw. Pull it clear of the \
                 wall if that was a nudge, or leave it if the bump is authored."
                    .to_string()
            ]
        );
    }

    /// Mismatched array lengths are a caller error handled totally: the
    /// walk stops at the shortest of the three rather than reading past
    /// the end of `names`.
    #[test]
    fn mismatched_lengths_do_not_panic() {
        let clusters = vec![0, 0, 0];
        let is_wall = vec![true, false];
        let names = vec!["North".to_string()];
        assert!(wall_merge_warnings(&clusters, &is_wall, &names).is_empty());
    }

    // ---------------------------------------------------------------
    // add_flank_classes
    // ---------------------------------------------------------------

    use super::super::faces::{Shape, faces};
    use super::super::labels;
    use super::super::superface::superfaces;

    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    const PALETTE: [f64; 5] = [0.25, 0.34, 0.43, 0.52, 0.61];

    fn separated(a: f64, b: f64) -> bool {
        (a - b).abs() >= labels::MIN_SEP - 1e-9
    }

    /// An empty `flank_solids` is a pure no-op: `sf`'s own class count and
    /// separations pass through untouched, so a level with no columns pays
    /// nothing for this pass.
    #[test]
    fn no_flanks_leaves_the_graph_unchanged() {
        let f = faces(
            0,
            &Shape::Column {
                center: [0.0; 3],
                radius: 0.3,
                half_height: 0.5,
            },
        );
        let sf = superfaces(&f, &[]);
        let (flank_class, classes, seps) = add_flank_classes(&sf, &f, &[], &[]);
        assert!(flank_class.is_empty());
        assert_eq!(classes, sf.classes);
        assert_eq!(seps, sf.separations);
    }

    /// Wave S: THE case the brief names, re-derived. A column standing
    /// flush on the floor is an ABUTMENT, not a merge — the column's
    /// bottom rim faces DOWN, the floor's top faces UP, opposite
    /// directions — so the column stays alone in its own cluster exactly
    /// as `superfaces`'s own singleton collapse defines one. The flank no
    /// longer wins a fresh class separated from its own rims: it JOINS
    /// the rim's already-collapsed class, and the whole column (rims and
    /// flank alike, now one class) still separates from the FLOOR's own
    /// class through rule (c)'s ordinary blanket law, since floor and
    /// column are different clusters and touching — end to end through
    /// `labels::assign`, exactly as `test_a_flank_separates_from_its_rims_and_a_touching_neighbour`
    /// (`game/tests/map_test.gd`) now checks on the real node → mesh
    /// pipeline.
    #[test]
    fn a_column_flush_on_the_floor_reads_as_one_uniform_class() {
        let floor = Shape::Box3d {
            center: [0.0, -0.05, 0.0],
            size: [10.0, 0.1, 10.0],
            basis: IDENTITY,
        };
        let column = Shape::Column {
            center: [0.0, 0.5, 0.0],
            radius: 0.3,
            half_height: 0.5,
        };
        let mut all = faces(0, &floor);
        all.extend(faces(1, &column));
        let touching = [(0, 1)];
        let sf = superfaces(&all, &touching);
        // the column's two rims (global 6, 7 — after the floor's six faces)
        assert_eq!(all[6].normal, [0.0, -1.0, 0.0]);
        assert_eq!(all[7].normal, [0.0, 1.0, 0.0]);
        // the abutment: never merged, so the rims collapsed to ONE class
        assert_eq!(sf.class_of[6], sf.class_of[7]);
        assert_ne!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);

        let (flank_class, classes, seps) = add_flank_classes(&sf, &all, &touching, &[1]);
        // no new class allocated: the flank ALIASES the rims' own
        assert_eq!(classes, sf.classes);
        let flank = flank_class[0];
        assert_eq!(flank, sf.class_of[6]);
        assert_eq!(flank, sf.class_of[7]);
        assert_eq!(seps, sf.separations);

        let augmented = Superfaces {
            class_of: sf.class_of.clone(),
            classes,
            separations: seps,
            cluster_of_solid: sf.cluster_of_solid.clone(),
        };
        let out = labels::assign(&augmented, &[], &PALETTE);
        assert_eq!(out.starved, 0);
        let flank_label = out.label_of_class[flank];
        // no internal seam: rim and flank read the identical label
        assert_eq!(flank_label, out.label_of_class[sf.class_of[6]]);
        assert_eq!(flank_label, out.label_of_class[sf.class_of[7]]);
        // the outer seam still draws: the column (rims+flank, one class)
        // differs from the floor's own class
        assert!(separated(flank_label, out.label_of_class[sf.class_of[0]]));
    }

    /// Two columns standing flush against each other must not let their
    /// flanks share a label where the curves meet — the same law the old
    /// whole-box touch graph already held, and this campaign must not
    /// regress it.
    #[test]
    fn two_touching_columns_separate_their_flanks() {
        // Centers far enough apart that neither rim's circle overlaps the
        // other's (distance 5.0 >> 2 * radius 0.3) — `touching` is supplied
        // directly, as the real level's AABB touch walk would, so this
        // isolates the flank-vs-flank rule from any incidental rim merge.
        let a = Shape::Column {
            center: [0.0, 0.5, 0.0],
            radius: 0.3,
            half_height: 0.5,
        };
        let b = Shape::Column {
            center: [5.0, 0.5, 0.0],
            radius: 0.3,
            half_height: 0.5,
        };
        let mut all = faces(0, &a);
        all.extend(faces(1, &b));
        let touching = [(0, 1)];
        let sf = superfaces(&all, &touching);
        let (flank_class, _classes, seps) = add_flank_classes(&sf, &all, &touching, &[0, 1]);
        assert!(seps.contains(&ordered(flank_class[0], flank_class[1])));
    }

    /// Wave S review finding (IMPORTANT): the MULTI-MEMBER branch (lines
    /// above, a column whose rim genuinely coplanar-MERGES with a
    /// partner rather than merely abutting one) was mutation-dead — every
    /// other flank fixture in this file ends up a SINGLETON, since a
    /// column resting ON anything presents an opposite-facing surface (a
    /// buried abutment, never a same-direction merge). This fixture
    /// forces a genuine merge instead: `post`'s TOP rim is flush with,
    /// and faces the SAME way as, `block`'s own top face, entirely
    /// inside its footprint — a coplanar overlap that MERGES them,
    /// putting `block` and `post` in the SAME multi-member cluster.
    /// `post`'s bottom rim stays clear (y=0, strictly inside `block`'s
    /// own y-range, matching none of its six faces), so exactly one
    /// merge edge exists — the minimal case that still makes the cluster
    /// multi-member.
    ///
    /// The two assertions are chosen to be independently diagnostic:
    /// `post`'s own bottom rim (global 6) is never one of `block`'s real
    /// faces, so that pair can ONLY come from the own-rim loop (476-480);
    /// `block`'s own -X face (global 0) is never one of `post`'s real
    /// faces, so that pair can ONLY come from the touching-neighbour loop
    /// (`separate_from_touching_neighbours`, formerly inlined at
    /// 482-496) — deleting either loop alone, or both together, must
    /// fail at least one.
    #[test]
    fn a_columns_flank_separates_from_a_genuinely_merged_rim_and_its_partner() {
        let block = Shape::Box3d {
            center: [0.0, 0.0, 0.0],
            size: [6.0, 2.0, 6.0],
            basis: IDENTITY,
        };
        let post = Shape::Column {
            center: [0.0, 0.5, 0.0],
            radius: 0.3,
            half_height: 0.5,
        };
        let mut all = faces(0, &block);
        all.extend(faces(1, &post));

        // the merge premise, hand-derived: post's top rim (global 7,
        // y=1) is flush with and faces the same way as block's own top
        // (global 3, y=1) — block spans y in [-1,1], post spans y in
        // [0,1], so post's bottom rim (global 6, y=0) matches neither
        // block's own -Y (y=-1) nor any other block face.
        assert_eq!(all[3].normal, [0.0, 1.0, 0.0]);
        assert_eq!(all[3].offset, 1.0);
        assert_eq!(all[7].normal, [0.0, 1.0, 0.0]);
        assert_eq!(all[7].offset, 1.0);
        assert_eq!(all[6].normal, [0.0, -1.0, 0.0]);
        assert_eq!(all[6].offset, 0.0);
        assert_eq!(all[0].normal, [-1.0, 0.0, 0.0]);

        let touching = [(0, 1)];
        let sf = superfaces(&all, &touching);
        // the merge actually happened: block and post share ONE
        // multi-member cluster now, neither is a singleton
        assert_eq!(sf.class_of[3], sf.class_of[7]);
        assert_eq!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);

        let (flank_class, classes, seps) = add_flank_classes(&sf, &all, &touching, &[1]);
        let flank = flank_class[0];
        // own-rim loop: post's bottom rim, never one of block's faces
        assert!(seps.contains(&ordered(flank, sf.class_of[6])));
        // touching-neighbour loop: block's own -X, never one of post's
        assert!(seps.contains(&ordered(flank, sf.class_of[0])));

        let augmented = Superfaces {
            class_of: sf.class_of.clone(),
            classes,
            separations: seps,
            cluster_of_solid: sf.cluster_of_solid.clone(),
        };
        let out = labels::assign(&augmented, &[], &PALETTE);
        assert_eq!(out.starved, 0);
        let flank_label = out.label_of_class[flank];
        assert!(separated(flank_label, out.label_of_class[sf.class_of[6]]));
        assert!(separated(flank_label, out.label_of_class[sf.class_of[0]]));
    }

    /// Wave S review finding (MINOR 1): a `flank_solids` entry naming a
    /// solid with no real face in `faces` at all — never true of an
    /// actual column today (`column_faces` has no degeneracy guard of
    /// its own and always emits its two rims), but a defensive property
    /// of THIS function's own contract, not a currently-reachable path —
    /// used to fall back to a fresh, wholly UNCONSTRAINED class: free to
    /// land on whatever slot a touching neighbour already uses, a silent
    /// melt. `far` (solid 2) exists only to push `solid_count` past
    /// index 1, so solid 1 still gets a valid (trivial, singleton)
    /// cluster entry rather than an out-of-range one — the exact
    /// shape `is_singleton`'s `.get(solid)` needs to read `true` and
    /// reach the orphan-fallback arm at all.
    #[test]
    fn a_flank_naming_a_faceless_solid_still_separates_from_its_touching_neighbours() {
        let block = Shape::Box3d {
            center: [0.0, 0.0, 0.0],
            size: [1.0, 1.0, 1.0],
            basis: IDENTITY,
        };
        let far = Shape::Box3d {
            center: [50.0, 0.0, 0.0],
            size: [1.0, 1.0, 1.0],
            basis: IDENTITY,
        };
        let mut all = faces(0, &block);
        all.extend(faces(2, &far)); // solid 1 deliberately absent
        let touching = [(0, 1)];
        let sf = superfaces(&all, &touching);
        assert_eq!(sf.cluster_of_solid.len(), 3);
        // solid 1 got a real, trivial singleton cluster despite
        // contributing no face at all — the premise the fallback branch
        // needs to even be reached
        assert_ne!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);

        let (flank_class, classes, seps) = add_flank_classes(&sf, &all, &touching, &[1]);
        let flank = flank_class[0];
        assert_eq!(classes, sf.classes + 1);
        // separated from EVERY class block's six real faces belong to —
        // the fix; the unfixed fallback added no separation at all
        for (i, face) in all.iter().enumerate() {
            if face.solid == 0 {
                assert!(seps.contains(&ordered(flank, sf.class_of[i])));
            }
        }

        let augmented = Superfaces {
            class_of: sf.class_of.clone(),
            classes,
            separations: seps,
            cluster_of_solid: sf.cluster_of_solid.clone(),
        };
        let out = labels::assign(&augmented, &[], &PALETTE);
        assert_eq!(out.starved, 0);
        let flank_label = out.label_of_class[flank];
        for &c in &sf.class_of[0..6] {
            assert!(separated(flank_label, out.label_of_class[c]));
        }
    }

    /// Wave S: a column with no neighbours at all is alone in its own
    /// cluster — its two rims already collapsed to ONE class by
    /// `superfaces`'s own singleton law before this function ever runs —
    /// and the flank JOINS that same class rather than winning a fresh,
    /// separated one: today's look, a lone barrel with no internal seam
    /// at all. No class allocated, no separation added.
    #[test]
    fn an_isolated_columns_flank_joins_its_solids_singleton_class() {
        let f = faces(
            0,
            &Shape::Column {
                center: [0.0; 3],
                radius: 0.3,
                half_height: 0.5,
            },
        );
        let sf = superfaces(&f, &[]);
        assert_eq!(sf.classes, 1);
        assert_eq!(sf.class_of[0], sf.class_of[1]);

        let (flank_class, classes, seps) = add_flank_classes(&sf, &f, &[], &[0]);
        assert_eq!(classes, sf.classes);
        assert_eq!(seps, sf.separations);
        assert!(seps.is_empty());
        assert_eq!(flank_class[0], sf.class_of[0]);
        assert_eq!(flank_class[0], sf.class_of[1]);
    }

    fn ordered(a: usize, b: usize) -> (usize, usize) {
        if a < b { (a, b) } else { (b, a) }
    }

    // ---------------------------------------------------------------
    // add_anchor_classes
    // ---------------------------------------------------------------

    /// An empty `extra_anchors` is a pure no-op.
    #[test]
    fn no_extra_anchors_leaves_the_graph_unchanged() {
        let (classes, seps, anchors) = add_anchor_classes(4, &[(0, 1)], &[]);
        assert_eq!(classes, 4);
        assert_eq!(seps, vec![(0, 1)]);
        assert!(anchors.is_empty());
    }

    /// One phantom anchor class per entry, separated from every class it
    /// names, and returned as an `(class, label)` pair `labels::assign`
    /// takes directly — the mechanism that preserves a source's old
    /// `Fixed`-anchor ban now that it contributes no real face at all.
    #[test]
    fn a_phantom_anchor_bans_its_named_neighbours() {
        let (classes, seps, anchors) = add_anchor_classes(3, &[], &[(0.33, vec![0, 2])]);
        assert_eq!(classes, 4);
        assert_eq!(anchors, vec![(3, 0.33)]);
        assert!(seps.contains(&(0, 3)));
        assert!(seps.contains(&(2, 3)));
        assert_eq!(seps.len(), 2);
    }

    /// End to end: a class touching a phantom anchor must not land within
    /// MIN_SEP of the anchor's own label once `labels::assign` runs.
    #[test]
    fn a_touching_class_avoids_the_phantom_anchors_label() {
        let sf = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![],
            cluster_of_solid: vec![0, 1],
        };
        let (classes, seps, anchors) =
            add_anchor_classes(sf.classes, &sf.separations, &[(0.33, vec![0])]);
        let augmented = Superfaces {
            class_of: sf.class_of,
            classes,
            separations: seps,
            cluster_of_solid: sf.cluster_of_solid,
        };
        let out = labels::assign(&augmented, &anchors, &PALETTE);
        assert_eq!(out.starved, 0);
        assert_eq!(out.label_of_class[2], 0.33); // the anchor's own class
        assert!(separated(out.label_of_class[0], 0.33));
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
