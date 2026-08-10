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

use godot::classes::{
    BoxMesh, BoxShape3D, CollisionShape3D, Material, Mesh, MeshInstance3D, Shape3D,
};
use godot::prelude::*;

use crate::oid_palette;

/// The per-instance shader parameter carrying a solid's flat object id —
/// `data_core`'s `u_oid`, the G channel the outline pass diffs to find
/// creases. Two touching solids sharing an id have NO line between them.
pub(crate) const OID_PARAM: &str = "u_oid";

/// What the level needs of any solid, whatever shape it is.
pub trait WaveSolid {
    /// Take the world skin — the data-writing material the level deals to
    /// everything that renders at real depth.
    fn set_material(&mut self, mat: &Gd<Material>);

    /// Take the flat object id the colouring chose for this solid.
    fn set_oid(&mut self, oid: f64);

    /// The id this solid currently carries, read back off its skin.
    /// [`oid_palette::NO_OID`] before a level has painted it.
    fn oid(&self) -> f64;
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

    /// Paint the limb with a flat object id.
    pub(crate) fn set_oid(&mut self, oid: f64) {
        if let Some(limb) = self.limb.as_mut() {
            limb.set_instance_shader_parameter(OID_PARAM, &oid.to_variant());
        }
    }

    /// The id the limb carries right now — the one source of truth, read
    /// back rather than mirrored.
    pub(crate) fn oid(&self) -> f64 {
        self.limb
            .as_ref()
            .map(|limb| limb.get_instance_shader_parameter(OID_PARAM))
            .and_then(|value| value.try_to::<f64>().ok())
            .unwrap_or(oid_palette::NO_OID)
    }

    fn wear(&mut self, mat: &Gd<Material>) {
        if let Some(limb) = self.limb.as_mut() {
            limb.set_material_override(mat);
        }
    }
}

/// A minus sign typed into a size knob, folded away and reported ONCE,
/// naming the node — the other half of every solid that is identical.
///
/// The fold itself is not a nicety. A size is a magnitude to a mesh and a
/// magnitude to a collider, but they disagree about how to say so:
/// `BoxMesh`/`CylinderMesh` take a negative extent and draw its mirror,
/// while `BoxShape3D`/`CylinderShape3D` REFUSE it and silently keep
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
    skin.set_mesh(mesh);
    skin.set_position(lift);
    if let Some(mat) = mat {
        skin.set_material_override(mat);
    }
    let mut collider = CollisionShape3D::new_alloc();
    collider.set_shape(shape);
    collider.set_position(lift);
    BuiltBody { skin, collider }
}

/// One built box — the shape the walls, the slabs and the box prop all
/// share. Its mesh and shape come back typed, because their owners reshape
/// them live when a designer drags a size knob.
pub(crate) struct BuiltBox {
    /// The outline mesh limb.
    pub(crate) skin: Gd<MeshInstance3D>,
    /// The skin's box mesh, kept for live reshaping.
    pub(crate) mesh: Gd<BoxMesh>,
    /// The collider limb.
    pub(crate) collider: Gd<CollisionShape3D>,
    /// The collider's box shape, kept for live reshaping.
    pub(crate) shape: Gd<BoxShape3D>,
}

/// Build the mesh + collider pair for a box of `size` centered `lift`
/// above the owner's origin.
pub(crate) fn build_box(size: Vector3, lift: Vector3, mat: Option<&Gd<Material>>) -> BuiltBox {
    let mut mesh = BoxMesh::new_gd();
    mesh.set_size(size);
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
