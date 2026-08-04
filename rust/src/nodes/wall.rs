//! The world's boxes as engine nodes a designer can hold. A level is
//! authored by dragging these around the editor: [`WaveWall`] is one
//! wall segment — place it, rotate it, stretch its `length` knob — and
//! [`WaveProp`] is a free box obstacle (a tabletop, a chair leg) sized by
//! its `size` knob. Each builds its own outline mesh and collider in
//! `_ready` from nothing but its transform and knobs, so what the editor
//! shows is exactly what the waves will strike.
//!
//! Axis law, enforced here so a designer cannot break physics by
//! accident: walls snap to the nearest quarter turn on entering the tree
//! — the hum-room derivation and the shader's room rect both read wall
//! centerlines as axis-aligned lines. The snapped basis comes from
//! [`level_plan::quadrant_basis`]'s exact 0/±1 columns, and a wall's
//! scale is discarded with the same stroke: `length` is the one size
//! knob. Props stay free — they carry no room contract, and the waves
//! outline them from any angle.
//!
//! The data-pass material arrives from the level root ([`super::level`])
//! — the single injection point — not per-node; a bare node in the
//! editor simply shows its plain box.

use godot::classes::{
    BoxMesh, BoxShape3D, CollisionShape3D, IStaticBody3D, Material, MeshInstance3D, StaticBody3D,
};
use godot::prelude::*;

use crate::level_plan;
use crate::oid_palette;

/// One wall segment: an axis-snapped box, `length` meters of centerline
/// padded by a wall half-thickness each way, floor to ceiling. The node
/// stands on the floor at the centerline's midpoint; the box rises from
/// it.
#[derive(GodotClass)]
#[class(tool, init, base=StaticBody3D)]
pub struct WaveWall {
    /// Centerline length in meters — the designer's one size knob.
    #[export]
    #[var(get = get_length, set = set_length)]
    #[init(val = 4.0)]
    length: f64,
    /// The snapped quarter turn, derived on entering the tree: even runs
    /// along world X, odd along world Z.
    quadrant: u8,
    data_mat: Option<Gd<Material>>,
    skin: Option<Gd<MeshInstance3D>>,
    mesh: Option<Gd<BoxMesh>>,
    shape: Option<Gd<BoxShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveWall {
    fn ready(&mut self) {
        self.snap_to_axis();
        let size = level_plan::wall_box(self.length);
        let lift = Vector3::new(0.0, (level_plan::WALL_H * 0.5) as f32, 0.0);
        let built = build_box(size, lift, self.data_mat.as_ref());
        let mut base = self.base_mut();
        base.add_child(&built.skin);
        base.add_child(&built.collider);
        drop(base);
        self.skin = Some(built.skin);
        self.mesh = Some(built.mesh);
        self.shape = Some(built.shape);
    }
}

#[godot_api]
impl WaveWall {
    /// The length knob reshapes the wall live — in the editor and at
    /// runtime alike, mesh and collider together.
    #[func]
    fn set_length(&mut self, length: f64) {
        self.length = length;
        let size = level_plan::wall_box(length);
        if let Some(mesh) = self.mesh.as_mut() {
            mesh.set_size(size);
        }
        if let Some(shape) = self.shape.as_mut() {
            shape.set_size(size);
        }
    }

    #[func]
    fn get_length(&self) -> f64 {
        self.length
    }

    /// The level root hands every wall the one data-pass material — the
    /// world is outline-only, and only that pass makes anything visible.
    pub(crate) fn set_material(&mut self, mat: &Gd<Material>) {
        self.data_mat = Some(mat.clone());
        if let Some(skin) = self.skin.as_mut() {
            skin.set_material_override(mat);
        }
    }

    /// The level root assigns this wall its flat object id — one silhouette
    /// per wall, no interior box-corner clutter; neighbours get different
    /// ids so the seam between two walls still draws.
    pub(crate) fn set_oid(&mut self, oid: f64) {
        if let Some(skin) = self.skin.as_mut() {
            skin.set_instance_shader_parameter("u_oid", &oid.to_variant());
        }
    }

    /// The id this wall currently carries, read back off the skin rather
    /// than mirrored in a field, so there is one source of truth: exactly
    /// what the data pass will write to G. [`oid_palette::NO_OID`] before a
    /// level has painted it.
    #[func]
    fn oid(&self) -> f64 {
        read_oid(self.skin.as_ref())
    }

    /// This wall's centerline as the classic (x1, z1, x2, z2) segment —
    /// the level root derives every room contract from these. Tree-only:
    /// the segment reads the global position `_ready` snapped.
    pub(crate) fn segment(&self) -> Vector4 {
        let at = self.base().get_global_position();
        level_plan::wall_segment(at, self.length, self.quadrant)
    }

    /// The axis law: whatever free-hand rotation (or scale) the node
    /// carries collapses onto the nearest exact quarter turn. Loud when
    /// it actually moved something — the designer should learn the law,
    /// not fight ghosts.
    fn snap_to_axis(&mut self) {
        let mut transform = self.base().get_transform();
        self.quadrant = level_plan::yaw_quadrant(f64::from(self.base().get_rotation().y));
        let snapped = level_plan::quadrant_basis(self.quadrant);
        if !basis_close(transform.basis, snapped) {
            godot_warn!(
                "WaveWall '{}': snapped to the nearest quarter turn — walls are \
                 axis-aligned boxes by law (use the length knob, never scale)",
                self.base().get_name(),
            );
        }
        transform.basis = snapped;
        self.base_mut().set_transform(transform);
    }
}

/// A free box obstacle — table top, chair leg, crate. The node sits at
/// the box's center; `size` is its full extent. Unlike walls it may
/// rotate freely: props carry no room contract, and waves outline them
/// from any angle.
#[derive(GodotClass)]
#[class(tool, init, base=StaticBody3D)]
pub struct WaveProp {
    /// Full box extent in meters.
    #[export]
    #[var(get = get_size, set = set_size)]
    #[init(val = Vector3::new(0.5, 0.5, 0.5))]
    size: Vector3,
    data_mat: Option<Gd<Material>>,
    skin: Option<Gd<MeshInstance3D>>,
    mesh: Option<Gd<BoxMesh>>,
    shape: Option<Gd<BoxShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveProp {
    fn ready(&mut self) {
        let built = build_box(self.size, Vector3::ZERO, self.data_mat.as_ref());
        let mut base = self.base_mut();
        base.add_child(&built.skin);
        base.add_child(&built.collider);
        drop(base);
        self.skin = Some(built.skin);
        self.mesh = Some(built.mesh);
        self.shape = Some(built.shape);
    }
}

#[godot_api]
impl WaveProp {
    /// The size knob reshapes the prop live, mesh and collider together.
    #[func]
    fn set_size(&mut self, size: Vector3) {
        self.size = size;
        if let Some(mesh) = self.mesh.as_mut() {
            mesh.set_size(size);
        }
        if let Some(shape) = self.shape.as_mut() {
            shape.set_size(size);
        }
    }

    #[func]
    fn get_size(&self) -> Vector3 {
        self.size
    }

    /// The level root's material injection — same door as the walls'.
    pub(crate) fn set_material(&mut self, mat: &Gd<Material>) {
        self.data_mat = Some(mat.clone());
        if let Some(skin) = self.skin.as_mut() {
            skin.set_material_override(mat);
        }
    }

    /// The level root assigns this prop its flat object id — same door as
    /// the walls'.
    pub(crate) fn set_oid(&mut self, oid: f64) {
        if let Some(skin) = self.skin.as_mut() {
            skin.set_instance_shader_parameter("u_oid", &oid.to_variant());
        }
    }

    /// The id this prop currently carries — same door as the walls'.
    #[func]
    fn oid(&self) -> f64 {
        read_oid(self.skin.as_ref())
    }
}

/// Read a skin's flat object id back, answering [`oid_palette::NO_OID`] for
/// a limb that was never painted or never built.
fn read_oid(skin: Option<&Gd<MeshInstance3D>>) -> f64 {
    skin.map(|skin| skin.get_instance_shader_parameter("u_oid"))
        .and_then(|value| value.try_to::<f64>().ok())
        .unwrap_or(oid_palette::NO_OID)
}

/// One built box: the mesh limb for the data pass and the collider for
/// cane rays, echo rays and the walking body — the retired map builder's
/// `_add_box`, as parts.
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
/// above the owner's origin, drawn through the data-pass material when
/// one has been injected (a bare editor node shows a plain box instead).
pub(crate) fn build_box(size: Vector3, lift: Vector3, mat: Option<&Gd<Material>>) -> BuiltBox {
    let mut mesh = BoxMesh::new_gd();
    mesh.set_size(size);
    let mut skin = MeshInstance3D::new_alloc();
    skin.set_mesh(&mesh);
    skin.set_position(lift);
    if let Some(mat) = mat {
        skin.set_material_override(mat);
    }
    let mut shape = BoxShape3D::new_gd();
    shape.set_size(size);
    let mut collider = CollisionShape3D::new_alloc();
    collider.set_shape(&shape);
    collider.set_position(lift);
    BuiltBox {
        skin,
        mesh,
        collider,
        shape,
    }
}

/// Whether two bases agree within a designer-invisible epsilon — decides
/// only whether the snap WARNS; the snap itself always writes the exact
/// basis.
fn basis_close(a: Basis, b: Basis) -> bool {
    let eps = 1e-4;
    (a.col_a() - b.col_a()).length() < eps
        && (a.col_b() - b.col_b()).length() < eps
        && (a.col_c() - b.col_c()).length() < eps
}
