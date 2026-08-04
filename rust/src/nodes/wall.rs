//! The wall — the one solid that is more than a shape. A level is authored
//! by dragging these around the editor: place one, rotate it, stretch its
//! `length` knob, and it builds its own outline mesh and collider in
//! `_ready` from nothing but transform and knob, so what the editor shows is
//! exactly what the waves will strike.
//!
//! A wall is special because it OCCLUDES. Its centerline is the level's
//! technical contract: the sight shaders count wall crossings to decide
//! what a source may light and what the hero may hear, so a wall's geometry
//! is physics, not decoration. That is why the axis law is enforced here —
//! a free-hand rotation snaps to the nearest quarter turn on entering the
//! tree, from [`level_plan::quadrant_basis`]'s exact 0/±1 columns, and the
//! node's scale is discarded with the same stroke: `length` is the one size
//! knob. The free shapes with no such contract live in [`super::props`].
//!
//! The world skin arrives from the level root ([`super::level`]) — the
//! single injection point — not per-node; a bare node in the editor simply
//! shows its plain box.

use godot::classes::{BoxMesh, BoxShape3D, IStaticBody3D, Material, StaticBody3D};
use godot::prelude::*;

use super::solid::{Skin, WaveSolid, build_box};
use crate::level_plan;

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
    skin: Skin,
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
        let built = build_box(size, lift, self.skin.material());
        let mut base = self.base_mut();
        base.add_child(&built.skin);
        base.add_child(&built.collider);
        drop(base);
        self.skin.adopt(built.skin);
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

    /// The id this wall carries — the engine-facing read-back of
    /// [`WaveSolid::oid`], so the suites can hold the seam law against a
    /// scene without binding Rust traits.
    #[func]
    fn oid(&self) -> f64 {
        WaveSolid::oid(self)
    }

    /// This wall's centerline as the classic (x1, z1, x2, z2) segment —
    /// the level root derives every contract from these. Tree-only: the
    /// segment reads the global position `_ready` snapped.
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

#[godot_dyn]
impl WaveSolid for WaveWall {
    fn set_material(&mut self, mat: &Gd<Material>) {
        self.skin.set_material(mat);
    }

    fn set_oid(&mut self, oid: f64) {
        self.skin.set_oid(oid);
    }

    fn oid(&self) -> f64 {
        self.skin.oid()
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
