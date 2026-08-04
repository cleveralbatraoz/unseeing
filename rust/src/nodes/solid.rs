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
