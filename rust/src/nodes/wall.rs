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
//! Both the snap and the centerline it feeds are read in WORLD space, and
//! that is not a detail: a wall is authored inside whatever room prefab
//! carries it, so the transform it draws under is its ancestors' as much as
//! its own. Snapping the local basis under a turned room would leave the
//! box drawn down one axis and the occluder derived down the other — sound
//! passing through a wall the eye is shown, and stopping at air it is not.
//!
//! The world skin arrives from the level root ([`super::level`]) — the
//! single injection point — not per-node; a bare node in the editor simply
//! shows its plain box.

use godot::classes::{ArrayMesh, BoxShape3D, IStaticBody3D, Material, StaticBody3D};
use godot::prelude::*;

use super::solid::{
    self, BOX_ORDINALS, LIMBS, SignFold, Skin, WaveSolid, build_box, clear_limbs,
    warnings_from_level,
};
use crate::level_plan;
use crate::render;

/// One wall segment: an axis-snapped box, `length` meters of centerline
/// padded by a wall half-thickness each way, floor to ceiling. The node
/// stands on the floor at the centerline's midpoint; the box rises from
/// it.
#[derive(GodotClass)]
#[class(tool, init, base=StaticBody3D)]
pub struct WaveWall {
    /// Centerline length in meters — the designer's one size knob, and a
    /// magnitude: a negative reading folds at the knob ([`SignFold`]).
    #[export]
    #[var(get = get_length, set = set_length)]
    #[init(val = 4.0)]
    length: f64,
    skin: Skin,
    fold: SignFold,
    mesh: Option<Gd<ArrayMesh>>,
    shape: Option<Gd<BoxShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveWall {
    fn ready(&mut self) {
        self.snap_to_axis();
        // a duplicated wall arrives carrying the original's limbs; this
        // build owns the pair, so the ghosts go first
        clear_limbs(self, &LIMBS);
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
        let name = self.base().get_name();
        self.fold.say(Some(name));
    }

    /// The Scene dock's warning icon for this one wall — whatever its
    /// owning [`super::level::WaveLevel`] pinned to this node's path, via
    /// [`warnings_from_level`]. Empty outside any level, which is legal:
    /// a prefab edited on its own has committed no fault yet.
    fn get_configuration_warnings(&self) -> PackedStringArray {
        warnings_from_level(&self.base().clone().upcast::<Node>())
    }
}

#[godot_api]
impl WaveWall {
    /// The length knob reshapes the wall live — in the editor and at
    /// runtime alike, mesh and collider together, on the knob's magnitude.
    /// A wall is three things derived from this one number (a drawn box, a
    /// collider, an occluding centerline) and they answer a minus sign
    /// three different ways, so the sign never gets past here.
    #[func]
    fn set_length(&mut self, length: f64) {
        self.length = self.fold.scalar("length", length);
        let size = level_plan::wall_box(self.length);
        if let Some(mesh) = self.mesh.as_mut() {
            render::paint::resize_box_surface(mesh, size, BOX_ORDINALS);
        }
        if let Some(shape) = self.shape.as_mut() {
            shape.set_size(size);
        }
        let named = self.base().is_inside_tree().then(|| self.base().get_name());
        self.fold.say(named);
    }

    #[func]
    fn get_length(&self) -> f64 {
        self.length
    }

    /// The id this wall carries — the engine-facing read-back of its own
    /// [`Skin`], off the mesh's own `CUSTOM0`, so the suites can hold the
    /// seam law against a scene without binding Rust traits.
    #[func]
    fn oid(&self) -> f64 {
        self.skin.oid()
    }

    /// This wall's box as `render::Shape`, in world space — the geometry
    /// the derive-time paint pass folds into the superface graph. Mirrors
    /// exactly what `ready()` builds: [`level_plan::wall_box`], centered at
    /// the same lift the mesh is drawn at (`(0, WALL_H/2, 0)` local),
    /// carried into world space by the wall's own global transform —
    /// [`Self::snap_to_axis`] has already collapsed that transform's basis
    /// to an exact quadrant by the time `ready` calls this, so no further
    /// snapping happens here.
    pub(crate) fn world_shape(&self) -> render::Shape {
        let placed = self.base().get_global_transform();
        let size = level_plan::wall_box(self.length);
        let lift = Vector3::new(0.0, (level_plan::WALL_H * 0.5) as f32, 0.0);
        render::Shape::Box3d {
            center: solid::to_f64_3(placed * lift),
            size: solid::to_f64_3(size),
            basis: solid::basis_columns_f64(placed.basis),
        }
    }

    /// Bake the derive-time paint pass's labels onto this wall — see
    /// [`solid::paint_solid`].
    pub(crate) fn paint(&mut self, labels_by_ordinal: &[f32]) {
        solid::paint_solid(
            self.mesh.as_mut(),
            render::paint::ShapeKind::Box,
            labels_by_ordinal,
        );
    }

    /// The engine-facing read-back of
    /// [`IStaticBody3D::get_configuration_warnings`] — needed for the same
    /// reason [`super::level::WaveLevel`]'s own forwarder carries one: that
    /// override is a pure GDVIRTUAL Godot's editor calls directly through
    /// the C++ virtual table and never binds to `ClassDB`, so no script can
    /// reach it under that name. Same disambiguation as [`Self::oid`] above
    /// — an inherent `#[func]` of the same name, forwarded through UFCS so
    /// it calls the trait override instead of recursing into itself.
    #[func]
    fn get_configuration_warnings(&self) -> PackedStringArray {
        IStaticBody3D::get_configuration_warnings(self)
    }

    /// This wall's centerline as the classic (x1, z1, x2, z2) segment —
    /// the level root derives every contract from these. Tree-only, and
    /// read WHOLE from one global transform: the same placement that puts
    /// the box in front of the eye decides which axis the run goes down,
    /// so the occluder can never end up perpendicular to the wall it
    /// describes.
    pub(crate) fn segment(&self) -> Vector4 {
        let placed = self.base().get_global_transform();
        let quadrant = level_plan::basis_quadrant(placed.basis);
        level_plan::wall_segment(placed.origin, self.length, quadrant)
    }

    /// The axis law, enforced in WORLD space: whatever free-hand rotation
    /// (or scale) reaches this node — its own, or inherited from any
    /// ancestor above it — collapses onto the nearest exact quarter turn.
    /// World space is the whole point: the centerline is derived there, so
    /// snapping the LOCAL basis would leave a wall in a turned room drawing
    /// down one axis and occluding down the other. A quadrant basis has
    /// unit columns, so writing it globally discards inherited scale with
    /// the same stroke — `length` stays the one size knob however deep a
    /// room prefab nests the wall. Loud when it actually moved something —
    /// the designer should learn the law, not fight ghosts.
    fn snap_to_axis(&mut self) {
        let mut placed = self.base().get_global_transform();
        let snapped = level_plan::quadrant_basis(level_plan::basis_quadrant(placed.basis));
        if !basis_close(placed.basis, snapped) {
            godot_warn!(
                "WaveWall '{}': snapped to the nearest quarter turn — walls are \
                 axis-aligned boxes by law (use the length knob, never scale)",
                self.base().get_name(),
            );
        }
        placed.basis = snapped;
        self.base_mut().set_global_transform(placed);
    }
}

#[godot_dyn]
impl WaveSolid for WaveWall {
    fn set_material(&mut self, mat: &Gd<Material>) {
        self.skin.set_material(mat);
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
