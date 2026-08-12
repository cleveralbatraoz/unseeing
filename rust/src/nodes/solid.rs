//! What every solid thing in the world has in common — the abstraction the
//! walls and the three prop shapes all reach the level through.
//!
//! A solid is anything a designer places that the waves can strike: a wall
//! segment, a box, a column, a wedge. They differ in geometry and in
//! nothing else. Each one needs exactly two things from the level, and the
//! level needs exactly one thing back:
//!
//! - the WORLD skin, so the data pass draws it at all (a solid never
//!   reaches for a material; it is handed one — the level is the single
//!   injection point);
//! - a flat object id, handed down by the graph colouring in
//!   [`crate::oid_palette`] so every seam between two touching solids draws;
//! - and, read back, the id it actually carries.
//!
//! That is [`WaveSolid`], published to the engine with `#[godot_dyn]`. The
//! level walks its children once and collects every solid there is without
//! naming a concrete class, so a fourth shape is a new file and no edit to
//! the level at all. It is the same mechanism [`super::source`] uses for
//! sound sources, applied to the other half of the world.
//!
//! The id is deliberately NOT mirrored into a field: it is read straight
//! back off the mesh instance, so there is one source of truth — exactly
//! what the data pass will write into the G channel.

use godot::classes::mesh::ArrayType;
use godot::classes::{
    ArrayMesh, BoxShape3D, CollisionShape3D, Material, Mesh, MeshInstance3D, Shape3D,
};
use godot::obj::WithBaseField;
use godot::prelude::*;

use crate::oid_palette;
use crate::render;

/// The name the engine writes on every mesh limb it builds, and on every
/// collider beside one. A designer never types these — they exist so that a
/// builder entering the tree can recognise the limbs an EARLIER build left
/// behind and take them over.
///
/// It has to be the name. `Node.duplicate()` — Ctrl+D, the way
/// `game/README.md` tells a designer to author a level — copies the children
/// a node built for itself, and the copy reaches `_ready` as a fresh Rust
/// object whose own handles are all `None`: the ghosts exist only in the
/// scene tree. A name survives the copy exactly, so it is the one handle
/// left on them.
pub(crate) const SKIN_NAME: &str = "WaveSkin";

/// The collider's half of [`SKIN_NAME`].
pub(crate) const COLLIDER_NAME: &str = "WaveCollider";

/// The pair [`build_body`] makes — what a solid's `_ready` hands
/// [`clear_limbs`] on the way in.
pub(crate) const LIMBS: [&str; 2] = [SKIN_NAME, COLLIDER_NAME];

/// Free whatever limbs an earlier build left under `node`, so that a builder
/// may run twice — a duplicated node, a re-entered scene — and own exactly
/// one set when it is done. Total: a freshly placed node carries none of
/// these names and nothing happens.
///
/// The ghosts are FREED rather than adopted, and that is the load-bearing
/// half. `duplicate()` copies a mesh by REFERENCE, so an adopted ghost would
/// hold the original's own `BoxMesh` and resize with it every time the
/// original's knob moved; a rebuilt limb holds a mesh of its own. They are
/// freed immediately rather than queued, too: a limb still standing when the
/// new one is added would be a second shape for a frame, and a test bench
/// counts a queued node as an orphan.
///
/// The walk runs under a [`WithBaseField::base_mut`] guard for its whole
/// length, and that is not tidiness: `remove_child` notifies the PARENT, and
/// a shape that listens for its own transform ([`super::props::WaveColumn`],
/// [`super::props::WaveWedge`]) is re-entered by that notification while its
/// `_ready` still holds the borrow. Only the guard makes the re-entry legal
/// — without it the engine aborts the process on a double borrow.
pub(crate) fn clear_limbs<C>(node: &mut C, names: &[&str])
where
    C: WithBaseField,
    C::Base: Inherits<Node>,
{
    let base = node.base_mut();
    let mut owner: Gd<Node> = Gd::clone(&base).upcast();
    let stale: Vec<Gd<Node>> = owner
        .get_children()
        .iter_shared()
        .filter(|child| names.iter().any(|name| child.get_name() == *name))
        .collect();
    for limb in stale {
        owner.remove_child(&limb);
        limb.free();
    }
    drop(base);
}

/// What the level needs of any solid, whatever shape it is.
///
/// Used to carry `oid` too — the per-instance flat object id the old
/// colouring painted every solid with. The derive-time superface paint pass
/// (`render::paint`) replaced that with a real per-face label baked into
/// each mesh's `CUSTOM0` channel, read back off the mesh itself (`Skin::oid`,
/// the shader's own G-channel source now that `pack_data` reads `CUSTOM0`
/// directly), so the trait no longer needs to speak for it at all — every
/// solid's own `#[func] oid()` reads its `Skin` directly instead of going
/// through this trait.
pub trait WaveSolid {
    /// Take the world skin — the data-writing material the level deals to
    /// everything that renders at real depth.
    fn set_material(&mut self, mat: &Gd<Material>);
}

/// The half of every solid that is identical: the one mesh limb the data
/// pass draws it through, and the material it was handed. Each class keeps
/// its own typed mesh and shape beside this, because only the class knows
/// how to reshape them when a designer drags its size knob.
#[derive(Default)]
pub(crate) struct Skin {
    limb: Option<Gd<MeshInstance3D>>,
    mat: Option<Gd<Material>>,
}

impl Skin {
    /// Remember the limb a class just built, and dress it if a material
    /// has already arrived.
    pub(crate) fn adopt(&mut self, limb: Gd<MeshInstance3D>) {
        self.limb = Some(limb);
        if let Some(mat) = self.mat.clone() {
            self.wear(&mat);
        }
    }

    /// The limb this solid draws itself with, for a class that must move it
    /// (a column and a wedge ride half their height above their node).
    pub(crate) fn limb(&mut self) -> Option<&mut Gd<MeshInstance3D>> {
        self.limb.as_mut()
    }

    /// The material this solid was handed, if any — what a class passes
    /// into its builder so a limb is born already dressed.
    pub(crate) fn material(&self) -> Option<&Gd<Material>> {
        self.mat.as_ref()
    }

    /// Take a material, dressing the limb if it is already built. Injection
    /// may land on either side of `_ready`, and both orders must work: the
    /// level injects before the tree, but a test may build first.
    pub(crate) fn set_material(&mut self, mat: &Gd<Material>) {
        self.mat = Some(mat.clone());
        self.wear(mat);
    }

    /// The id the limb carries right now — the one source of truth, read
    /// straight off the limb's own mesh (`mesh_first_label`) rather than
    /// mirrored or pushed through any per-instance uniform: `CUSTOM0` is
    /// what the shader itself reads for G, so this is exactly that value.
    pub(crate) fn oid(&self) -> f64 {
        self.limb
            .as_ref()
            .and_then(mesh_first_label)
            .unwrap_or(oid_palette::NO_OID)
    }

    fn wear(&mut self, mat: &Gd<Material>) {
        if let Some(limb) = self.limb.as_mut() {
            limb.set_material_override(mat);
        }
    }
}

/// The label a mesh limb's FIRST vertex carries in `CUSTOM0` — the
/// [`render::paint::FACE_ORDER`] −X face's real value once the derive-time
/// paint pass has rewritten it, read straight off the geometry itself: the
/// one source of truth the shader now reads too (`v_label`, off
/// `CUSTOM0.x`). `None` for a limb with no mesh, no surface, or no `CUSTOM0`
/// channel at all — a caller decides its own "nothing painted" answer
/// rather than this function folding one in.
///
/// This is still a SOLID-granularity read — the first face standing in for
/// a mesh that may carry several different labels across its own faces —
/// exactly the coarseness `WaveLevel::oid_census` (`super::level`) accepts
/// for the identical reason, until Task 10's real per-face law replaces
/// both call sites at once.
pub(crate) fn mesh_first_label(limb: &Gd<MeshInstance3D>) -> Option<f64> {
    let mesh = limb.get_mesh()?;
    if mesh.get_surface_count() == 0 {
        return None;
    }
    let arrays = mesh.surface_get_arrays(0);
    let custom = arrays.get(ArrayType::CUSTOM0.ord() as usize)?;
    let custom = custom.try_to::<PackedFloat32Array>().ok()?;
    custom.get(0).map(f64::from)
}

/// A minus sign typed into a size knob, folded away and reported ONCE,
/// naming the node — the other half of every solid that is identical.
///
/// The fold itself is not a nicety. A size is a magnitude to a mesh and a
/// magnitude to a collider, but they disagree about how to say so: the
/// generated box or column mesh takes a negative extent and draws its
/// mirror, while `BoxShape3D`/`CylinderShape3D` REFUSE it and silently keep
/// whatever they had — the default 1 m cube on a freshly built node. The
/// drawn shape and the struck shape stop being the same object, and the
/// only engine diagnostic ("BoxShape3D size cannot be negative") names no
/// node for a designer to click.
///
/// The warning is REMEMBERED rather than printed on the spot, because the
/// engine sets a scene's properties before the node is added to its parent
/// — at that moment it is still `@StaticBody3D@7`, and a warning naming
/// that helps nobody. A fold that lands off the tree waits for `_ready`; a
/// fold that lands on a knob dragged in the Inspector, where the name is
/// already final, is said immediately.
#[derive(Default)]
pub(crate) struct SignFold {
    pending: Option<String>,
}

impl SignFold {
    /// The magnitude of a vector knob, remembering the raw reading if a
    /// sign had to be folded out of it.
    pub(crate) fn vector(&mut self, knob: &str, raw: Vector3) -> Vector3 {
        if raw.x < 0.0 || raw.y < 0.0 || raw.z < 0.0 {
            self.remember(format!("{knob} {raw}"));
        }
        raw.abs()
    }

    /// The magnitude of a scalar knob, on the same terms.
    pub(crate) fn scalar(&mut self, knob: &str, raw: f64) -> f64 {
        if raw < 0.0 {
            self.remember(format!("{knob} {raw}"));
        }
        raw.abs()
    }

    /// Say the pending fold, if there is one and the node can be named.
    /// A nameless node keeps its fold pending for `_ready` to flush.
    pub(crate) fn say(&mut self, name: Option<StringName>) {
        let Some(name) = name else { return };
        let Some(what) = self.pending.take() else {
            return;
        };
        godot_warn!(
            "'{name}': folded a negative knob to its magnitude ({what}). A collider \
             refuses a negative extent where a mesh accepts one, so the shape \
             drawn and the shape struck would stop being the same object."
        );
    }

    /// Queue a reading to blame, joining it to whatever is already waiting:
    /// a scene load sets every knob before the node is in the tree, so a
    /// column with two negative knobs must not lose the first one.
    fn remember(&mut self, what: String) {
        self.pending = Some(match self.pending.take() {
            Some(already) => format!("{already}, {what}"),
            None => what,
        });
    }
}

/// One built body: the mesh limb for the data pass and the collider for
/// cane rays, echo rays and the walking body.
pub(crate) struct BuiltBody {
    /// The outline mesh limb.
    pub(crate) skin: Gd<MeshInstance3D>,
    /// The collider limb.
    pub(crate) collider: Gd<CollisionShape3D>,
}

/// Build the mesh + collider pair for any shape, centered `lift` above the
/// owner's origin, drawn through the world skin when one has been injected
/// (a bare editor node simply shows its plain shape instead).
pub(crate) fn build_body(
    mesh: &Gd<Mesh>,
    shape: &Gd<Shape3D>,
    lift: Vector3,
    mat: Option<&Gd<Material>>,
) -> BuiltBody {
    let mut skin = MeshInstance3D::new_alloc();
    skin.set_name(SKIN_NAME);
    skin.set_mesh(mesh);
    skin.set_position(lift);
    if let Some(mat) = mat {
        skin.set_material_override(mat);
    }
    let mut collider = CollisionShape3D::new_alloc();
    collider.set_name(COLLIDER_NAME);
    collider.set_shape(shape);
    collider.set_position(lift);
    BuiltBody { skin, collider }
}

/// The six ordinals (0..6) every box-shaped solid — a wall, a free prop,
/// a floor or ceiling slab — carries in CUSTOM0, one per face, read in
/// [`render::paint::FACE_ORDER`]'s own −X,+X,−Y,+Y,−Z,+Z order: a
/// placeholder the derive-time paint pass (Task 6) rewrites into a real
/// label. Its length is [`render::paint::face_count`]'s Box/Slab entry
/// made concrete, so a wrong-length edit here breaks a cargo test rather
/// than drifting silently.
pub(crate) const BOX_ORDINALS: [f32; 6] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];

/// One built box — the shape the walls, the slabs and the box prop all
/// share. Its mesh and shape come back typed, because their owners reshape
/// them live when a designer drags a size knob: the collider's `set_size`
/// mutates `BoxShape3D` in place as it always has, and the mesh's own
/// resize goes through [`render::paint::resize_box_surface`], which
/// mutates the SAME `ArrayMesh` resource for the same reason — see that
/// function's doc comment.
pub(crate) struct BuiltBox {
    /// The outline mesh limb.
    pub(crate) skin: Gd<MeshInstance3D>,
    /// The skin's box mesh, kept for live reshaping.
    pub(crate) mesh: Gd<ArrayMesh>,
    /// The collider limb.
    pub(crate) collider: Gd<CollisionShape3D>,
    /// The collider's box shape, kept for live reshaping.
    pub(crate) shape: Gd<BoxShape3D>,
}

/// A Godot vector's three f32 lanes, widened to the `[f64; 3]` triples
/// `render::faces` speaks — the one conversion boundary between the engine
/// layer's f32 geometry and the pure render subsystem's f64 vocabulary.
pub(crate) fn to_f64_3(v: Vector3) -> [f64; 3] {
    [f64::from(v.x), f64::from(v.y), f64::from(v.z)]
}

/// A basis's three columns, widened the same way — the world-space
/// `render::Shape::Box3d` basis every static solid's `world_shape` builds.
pub(crate) fn basis_columns_f64(b: Basis) -> [[f64; 3]; 3] {
    [
        to_f64_3(b.col_a()),
        to_f64_3(b.col_b()),
        to_f64_3(b.col_c()),
    ]
}

/// Bake the derive-time paint pass's chosen labels onto one solid: rewrite
/// its mesh's placeholder CUSTOM0 ordinals with the real per-face labels
/// ([`render::paint::relabel`]) — the shader reads `CUSTOM0` directly now,
/// so this is the whole of painting a solid; there is no instance uniform
/// left to keep in step. `mesh` is `None` for a solid whose knob was
/// dragged before `_ready` built one; a no-op, the same as every other
/// builder call in this module.
pub(crate) fn paint_solid(mesh: Option<&mut Gd<ArrayMesh>>, labels_by_ordinal: &[f32]) {
    if let Some(mesh) = mesh {
        render::paint::relabel(mesh, labels_by_ordinal);
    }
}

/// Build the mesh + collider pair for a box of `size` centered `lift`
/// above the owner's origin.
pub(crate) fn build_box(size: Vector3, lift: Vector3, mat: Option<&Gd<Material>>) -> BuiltBox {
    let mesh = render::paint::labelled_box(size, Vector3::ZERO, BOX_ORDINALS);
    let mut shape = BoxShape3D::new_gd();
    shape.set_size(size);
    let built = build_body(
        &mesh.clone().upcast::<Mesh>(),
        &shape.clone().upcast::<Shape3D>(),
        lift,
        mat,
    );
    BuiltBox {
        skin: built.skin,
        mesh,
        collider: built.collider,
        shape,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fold answers a magnitude on every sign, and remembers only when
    /// there was actually something to fold — a positive knob must never
    /// queue a warning for the next negative one to inherit.
    #[test]
    fn a_sign_folds_to_a_magnitude_and_is_remembered() {
        let mut fold = SignFold::default();
        assert_eq!(
            fold.vector("size", Vector3::new(-0.8, 0.4, -0.6)),
            Vector3::new(0.8, 0.4, 0.6)
        );
        assert!(fold.pending.is_some());
        fold.pending = None;

        assert_eq!(
            fold.vector("size", Vector3::new(0.8, 0.4, 0.6)),
            Vector3::new(0.8, 0.4, 0.6)
        );
        assert!(fold.pending.is_none(), "a clean knob queued a warning");

        assert_eq!(fold.scalar("radius", -0.3), 0.3);
        assert!(fold.pending.is_some());
        fold.pending = None;
        assert_eq!(fold.scalar("length", 4.0), 4.0);
        assert!(fold.pending.is_none(), "a clean knob queued a warning");
    }

    /// A scene load sets every knob before the node is in the tree and can
    /// be named, so the folds pile up before anything is said. All of them
    /// must reach the one warning — a column whose radius AND height were
    /// typed negative must not hear about the height alone.
    #[test]
    fn folds_waiting_for_a_name_all_reach_the_warning() {
        let mut fold = SignFold::default();
        fold.scalar("radius", -0.3);
        fold.scalar("height", -0.9);
        assert_eq!(fold.pending.as_deref(), Some("radius -0.3, height -0.9"));
    }

    /// Zero is not negative: a designer flattening a knob to nothing gets a
    /// degenerate shape, which is their business, and no warning about a
    /// sign that was never there.
    #[test]
    fn zero_is_not_a_folded_sign() {
        let mut fold = SignFold::default();
        assert_eq!(fold.vector("size", Vector3::ZERO), Vector3::ZERO);
        assert_eq!(fold.scalar("height", 0.0), 0.0);
        assert!(fold.pending.is_none());
    }
}
