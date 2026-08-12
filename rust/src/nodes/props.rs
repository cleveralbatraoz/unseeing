//! The free shapes a designer fills a room with — everything that is not a
//! wall. A prop carries no technical contract: it does not occlude the
//! sight tests, it may be rotated to any angle, and the waves outline it
//! from wherever they find it. All three reach the level through the one
//! [`WaveSolid`] door, so the level never names them.
//!
//! Three shapes, because in a contours-only world the SILHOUETTE is the
//! whole of an object and three silhouettes is three vocabularies:
//! - [`WaveProp`] — a box. Crates, shelves, tabletops, doorframes.
//! - [`WaveColumn`] — a cylinder. Barrels, pipes, pillars, a stove flue.
//!   Its outline is a CURVE where every box in the world draws a corner.
//! - [`WaveWedge`] — a triangular prism. Ramps, leaning planks, a lean-to
//!   roof. Its outline is a DIAGONAL, the one line neither of the others
//!   can draw.
//!
//! ORIGIN LAW, and it differs on purpose. A box prop is CENTRED on its node,
//! because a box is as often floating (a shelf, a tabletop, a beam) as
//! standing. A column and a wedge STAND on their node — `y = 0` puts them on
//! the floor — because a barrel or a ramp that is not resting on something
//! is a mistake, and the common case should need no arithmetic from the
//! designer.
//!
//! Standing is a law about the FLOOR, so it is read in the space the floor
//! is in: the shape's LOWEST point in WORLD space rests at the node's own
//! y, whatever turn its own transform and its ancestors' put on it. Upright
//! that is half the height and existing content never moves; laid on its
//! side a barrel rests on its RADIUS. The arithmetic is
//! [`prop_shape::cylinder_lift`] / [`prop_shape::wedge_lift`], and it is
//! re-read on every transform change — a designer turning a barrel in the
//! viewport fires no knob setter at all, and would otherwise watch it sink.
//!
//! SIZE LAW: the knob is the one size a shape has, and it never lies. A
//! minus sign folds to its magnitude ([`SignFold`]), and the node's SCALE
//! is folded into the knob on entering the tree, the node coming back at 1
//! ([`drop_scale`]). Absorbed silently — which is what happened before —
//! the S key was a second size knob the Inspector could not see: a prop of
//! 0.5 under scale (4, 1, 2) drew, collided and coloured as 2.0 x 0.5 x 1.0
//! while the Inspector went on reporting 0.5. A wall discards its scale
//! instead of folding it, because its length feeds an occluding centerline;
//! a prop's extent is free, so it can hold what the scale meant.

use godot::classes::notify::Node3DNotification;
use godot::classes::{
    ArrayMesh, BoxShape3D, CollisionShape3D, ConvexPolygonShape3D, CylinderShape3D, IStaticBody3D,
    Material, Mesh, Shape3D, StaticBody3D,
};
use godot::prelude::*;

use godot::classes::MeshInstance3D;

use super::solid::{
    self, BOX_ORDINALS, LIMBS, SignFold, Skin, WaveSolid, build_body, build_box, clear_limbs,
};
use crate::prop_shape;
use crate::render;

/// A free box obstacle — table top, chair leg, crate, shelf. The node sits
/// at the box's CENTER; `size` is its full extent. Unlike a wall it may
/// rotate freely: props carry no room contract, and waves outline them from
/// any angle.
#[derive(GodotClass)]
#[class(tool, init, base=StaticBody3D)]
pub struct WaveProp {
    /// Full box extent in meters — a magnitude: a negative reading folds to
    /// its absolute value at the knob ([`SignFold`]).
    #[export]
    #[var(get = get_size, set = set_size)]
    #[init(val = Vector3::new(0.5, 0.5, 0.5))]
    size: Vector3,
    skin: Skin,
    fold: SignFold,
    mesh: Option<Gd<ArrayMesh>>,
    shape: Option<Gd<BoxShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveProp {
    fn ready(&mut self) {
        clear_limbs(self, &LIMBS);
        let scaled = scale_to_fold(&self.base());
        if let Some(scale) = scaled {
            self.size = prop_shape::fold_box_scale(self.size, scale);
            drop_scale(&mut self.base_mut(), scale, "");
        }
        let built = build_box(self.size, Vector3::ZERO, self.skin.material());
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
}

#[godot_api]
impl WaveProp {
    /// The size knob reshapes the prop live, mesh and collider together —
    /// on the knob's magnitude, so a minus sign cannot leave the mesh
    /// reshaped and the collider refusing to follow.
    #[func]
    fn set_size(&mut self, size: Vector3) {
        self.size = self.fold.vector("size", size);
        let size = self.size;
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
    fn get_size(&self) -> Vector3 {
        self.size
    }

    /// The id this prop carries — the engine-facing read-back of its own
    /// [`Skin`], the `u_oid` bridge the derive-time paint pass keeps alive.
    #[func]
    fn oid(&self) -> f64 {
        self.skin.oid()
    }

    /// This prop's box as `render::Shape`, in world space — a box prop is
    /// CENTRED on its node (`ready()` builds it at lift `Vector3::ZERO`),
    /// so the box's world center is simply the node's own global origin.
    pub(crate) fn world_shape(&self) -> render::Shape {
        let placed = self.base().get_global_transform();
        render::Shape::Box3d {
            center: solid::to_f64_3(placed.origin),
            size: solid::to_f64_3(self.size),
            basis: solid::basis_columns_f64(placed.basis),
        }
    }

    /// Bake the derive-time paint pass's labels onto this prop — see
    /// [`solid::paint_solid`].
    pub(crate) fn paint(&mut self, labels_by_ordinal: &[f32]) {
        solid::paint_solid(&mut self.skin, self.mesh.as_mut(), labels_by_ordinal);
    }
}

#[godot_dyn]
impl WaveSolid for WaveProp {
    fn set_material(&mut self, mat: &Gd<Material>) {
        self.skin.set_material(mat);
    }
}

/// A round obstacle — barrel, pipe, pillar, stove flue. The node stands at
/// the cylinder's BASE, so `y = 0` puts it on the floor. Its silhouette is
/// the one curve in a world of corners, which is the whole reason it
/// exists: with a flat object id across it, the outline pass draws no
/// interior seam and the shape reads purely as its rounded outline.
#[derive(GodotClass)]
#[class(tool, init, base=StaticBody3D)]
pub struct WaveColumn {
    /// Radius in meters — a magnitude ([`SignFold`]).
    #[export]
    #[var(get = get_radius, set = set_radius)]
    #[init(val = 0.28)]
    radius: f64,
    /// Height in meters, rising from the node — a magnitude ([`SignFold`]).
    #[export]
    #[var(get = get_height, set = set_height)]
    #[init(val = 0.9)]
    height: f64,
    skin: Skin,
    fold: SignFold,
    mesh: Option<Gd<ArrayMesh>>,
    shape: Option<Gd<CylinderShape3D>>,
    collider: Option<Gd<CollisionShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveColumn {
    fn ready(&mut self) {
        clear_limbs(self, &LIMBS);
        let scaled = scale_to_fold(&self.base());
        if let Some(scale) = scaled {
            let knobs = prop_shape::fold_column_scale(self.radius, self.height, scale);
            self.radius = knobs.radius;
            self.height = knobs.height;
            let lost = if knobs.round {
                ""
            } else {
                " The cross-section was pulled unevenly across X and Z, which asks for \
                 an elliptic cylinder this vocabulary does not own — the wider radius \
                 was taken, so the shape contains what was drawn."
            };
            drop_scale(&mut self.base_mut(), scale, lost);
        }
        // built empty here and filled by the `reshape()` call below, which
        // is the ONE place the real geometry is generated — so `ready` and
        // a dragged knob can never disagree about what a column is
        let mesh = ArrayMesh::new_gd();
        let shape = CylinderShape3D::new_gd();
        let built = build_body(
            &mesh.clone().upcast::<Mesh>(),
            &shape.clone().upcast::<Shape3D>(),
            Vector3::ZERO,
            self.skin.material(),
        );
        let mut base = self.base_mut();
        base.add_child(&built.skin);
        base.add_child(&built.collider);
        drop(base);
        self.skin.adopt(built.skin);
        self.collider = Some(built.collider);
        self.mesh = Some(mesh);
        self.shape = Some(shape);
        self.reshape();
        self.base_mut().set_notify_transform(true);
        let name = self.base().get_name();
        self.fold.say(Some(name));
    }

    /// A turned node is a re-lift: the standing law is read off the
    /// placement, and moving a node fires no knob setter. Cheap enough to
    /// run on every transform change — it is a support and an inverse, and
    /// a prop moves only when a designer drags it.
    fn on_notification(&mut self, what: Node3DNotification) {
        if matches!(what, Node3DNotification::TRANSFORM_CHANGED) {
            self.relift();
        }
    }
}

#[godot_api]
impl WaveColumn {
    #[func]
    fn set_radius(&mut self, radius: f64) {
        self.radius = self.fold.scalar("radius", radius);
        self.reshape();
        let named = self.base().is_inside_tree().then(|| self.base().get_name());
        self.fold.say(named);
    }

    #[func]
    fn get_radius(&self) -> f64 {
        self.radius
    }

    #[func]
    fn set_height(&mut self, height: f64) {
        self.height = self.fold.scalar("height", height);
        self.reshape();
        let named = self.base().is_inside_tree().then(|| self.base().get_name());
        self.fold.say(named);
    }

    #[func]
    fn get_height(&self) -> f64 {
        self.height
    }

    /// The id this column carries — the engine-facing read-back of its own
    /// [`Skin`], the `u_oid` bridge the derive-time paint pass keeps alive.
    #[func]
    fn oid(&self) -> f64 {
        self.skin.oid()
    }

    /// This column's rims as `render::Shape::Column`, in world space —
    /// upright only, matching the shape `render::faces::column_faces`
    /// itself models: the world CENTER is the node's own global origin
    /// lifted by [`prop_shape::cylinder_lift`]'s own `underhang`, exactly
    /// as [`Self::relift`] positions the drawn mesh, so a level-tilted or
    /// laid-down column's TRUE geometry is approximated by the nearest
    /// upright cylinder at its standing height rather than represented
    /// exactly — the same scope this render vocabulary's `Shape::Column`
    /// variant is written for (no basis of its own), and every shipped
    /// column and wedge stands upright in practice
    /// (`prop_shape::tests::an_upright_shape_lifts_exactly_half_its_height`'s
    /// own doc comment).
    pub(crate) fn world_shape(&self) -> render::Shape {
        let placed = self.base().get_global_transform();
        let radius = self.radius as f32;
        let height = self.height as f32;
        let lift = prop_shape::cylinder_lift(placed.basis, radius, height);
        render::Shape::Column {
            center: solid::to_f64_3(placed * lift),
            radius: self.radius,
            half_height: self.height * 0.5,
        }
    }

    /// Bake the derive-time paint pass's labels onto this column — see
    /// [`solid::paint_solid`].
    pub(crate) fn paint(&mut self, labels_by_ordinal: &[f32]) {
        solid::paint_solid(&mut self.skin, self.mesh.as_mut(), labels_by_ordinal);
    }

    /// Mesh, collider and lift together, so a knob dragged in the
    /// Inspector moves what the waves strike and not only what is drawn.
    /// The mesh is regenerated whole — [`prop_shape::column_triangles`] —
    /// rather than resized in place the way the `CylinderMesh` primitive
    /// this replaces was: a generated surface has no size knob of its own
    /// to mutate, only vertices to rebuild.
    ///
    /// Both knobs are magnitudes by the time they land in the fields (the
    /// setters fold, [`SignFold`]), so nothing here has a sign to defend
    /// against: `CylinderShape3D` would refuse a negative radius outright.
    fn reshape(&mut self) {
        let radius = self.radius as f32;
        let height = self.height as f32;
        if let Some(mesh) = self.mesh.as_mut() {
            let triangles = prop_shape::column_triangles(radius, height * 0.5);
            render::paint::resize_triangle_surface(mesh, &triangles);
        }
        if let Some(shape) = self.shape.as_mut() {
            shape.set_radius(radius);
            shape.set_height(height);
        }
        self.relift();
    }

    /// Ride the limbs up onto the lift this placement asks for, so the
    /// cylinder's lowest point rests on the node. Upright that is half the
    /// height — the engine's cylinder primitives are centred — and turned
    /// it is whatever [`prop_shape::cylinder_lift`] says.
    fn relift(&mut self) {
        let basis = placement_basis(&self.base());
        let lift = prop_shape::cylinder_lift(basis, self.radius as f32, self.height as f32);
        lift_limbs(self.skin.limb(), self.collider.as_mut(), lift);
    }
}

#[godot_dyn]
impl WaveSolid for WaveColumn {
    fn set_material(&mut self, mat: &Gd<Material>) {
        self.skin.set_material(mat);
    }
}

/// A sloped obstacle — a ramp, a leaning plank, a lean-to roof. A box whose
/// top has been cut away, rising from nothing at its −X end to the full
/// height at its +X end. The node stands at the wedge's BASE, so `y = 0`
/// puts it on the floor, and rotating the node aims the slope.
///
/// The geometry is generated ([`prop_shape`]) rather than taken from an
/// engine primitive, because a triangular prism is neither a mesh nor a
/// collider Godot ships. Its faces carry explicit normals: normals are the
/// crease id for anything the level has not painted, so a smeared normal
/// would draw a smeared edge.
#[derive(GodotClass)]
#[class(tool, init, base=StaticBody3D)]
pub struct WaveWedge {
    /// Full extent in meters: run along X, rise along Y, width along Z —
    /// a magnitude ([`SignFold`]).
    #[export]
    #[var(get = get_size, set = set_size)]
    #[init(val = Vector3::new(1.2, 0.5, 0.8))]
    size: Vector3,
    skin: Skin,
    fold: SignFold,
    mesh: Option<Gd<ArrayMesh>>,
    shape: Option<Gd<ConvexPolygonShape3D>>,
    collider: Option<Gd<CollisionShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveWedge {
    fn ready(&mut self) {
        clear_limbs(self, &LIMBS);
        let scaled = scale_to_fold(&self.base());
        if let Some(scale) = scaled {
            self.size = prop_shape::fold_box_scale(self.size, scale);
            let lost = if scale.x < 0.0 || scale.y < 0.0 || scale.z < 0.0 {
                " A mirrored axis is a reflection, and no size expresses one — the \
                 slope still rises toward +X; turn the node to aim it."
            } else {
                ""
            };
            drop_scale(&mut self.base_mut(), scale, lost);
        }
        // the geometry is generated BEFORE the shape is attached: a
        // ConvexPolygonShape3D with no points cannot build a hull, and the
        // engine says so loudly the instant a collider is given one
        let mut mesh = ArrayMesh::new_gd();
        let mut shape = ConvexPolygonShape3D::new_gd();
        cut_wedge(&mut mesh, &mut shape, self.size);
        let built = build_body(
            &mesh.clone().upcast::<Mesh>(),
            &shape.clone().upcast::<Shape3D>(),
            Vector3::ZERO,
            self.skin.material(),
        );
        let mut base = self.base_mut();
        base.add_child(&built.skin);
        base.add_child(&built.collider);
        drop(base);
        self.skin.adopt(built.skin);
        self.collider = Some(built.collider);
        self.mesh = Some(mesh);
        self.shape = Some(shape);
        self.relift();
        self.base_mut().set_notify_transform(true);
        let name = self.base().get_name();
        self.fold.say(Some(name));
    }

    /// A turned node is a re-lift — see [`WaveColumn`]. Only the LIFT is
    /// redone, never the geometry: a wedge regenerates 24 vertices and a
    /// hull from scratch, and a designer dragging one across the viewport
    /// sends a transform change per frame.
    fn on_notification(&mut self, what: Node3DNotification) {
        if matches!(what, Node3DNotification::TRANSFORM_CHANGED) {
            self.relift();
        }
    }
}

#[godot_api]
impl WaveWedge {
    #[func]
    fn set_size(&mut self, size: Vector3) {
        self.size = self.fold.vector("size", size);
        self.reshape();
        let named = self.base().is_inside_tree().then(|| self.base().get_name());
        self.fold.say(named);
    }

    #[func]
    fn get_size(&self) -> Vector3 {
        self.size
    }

    /// The id this wedge carries — the engine-facing read-back of its own
    /// [`Skin`], the `u_oid` bridge the derive-time paint pass keeps alive.
    #[func]
    fn oid(&self) -> f64 {
        self.skin.oid()
    }

    /// This wedge's hull as `render::Shape::Wedge`, in world space: the
    /// same six local points [`prop_shape::wedge_hull`] gives `ready()`,
    /// lifted by [`prop_shape::wedge_lift`] exactly as the drawn mesh is,
    /// then carried into world space by the node's own global transform.
    pub(crate) fn world_shape(&self) -> render::Shape {
        let placed = self.base().get_global_transform();
        let lift = prop_shape::wedge_lift(placed.basis, self.size);
        let hull = prop_shape::wedge_hull(self.size);
        render::Shape::Wedge {
            hull: hull.map(|p| solid::to_f64_3(placed * (p + lift))),
        }
    }

    /// Bake the derive-time paint pass's labels onto this wedge — see
    /// [`solid::paint_solid`].
    pub(crate) fn paint(&mut self, labels_by_ordinal: &[f32]) {
        solid::paint_solid(&mut self.skin, self.mesh.as_mut(), labels_by_ordinal);
    }

    /// Rebuild the surface and the hull from the size knob, and re-lift the
    /// limbs onto it. The mesh is generated whole rather than resized,
    /// because a triangular prism's vertices all move when any extent
    /// changes.
    fn reshape(&mut self) {
        let size = self.size;
        let (Some(mesh), Some(shape)) = (self.mesh.as_mut(), self.shape.as_mut()) else {
            return; // knob dragged before _ready: the build will read it
        };
        cut_wedge(mesh, shape, size);
        self.relift();
    }

    /// Ride the limbs up onto the lift this placement asks for, so the
    /// prism's lowest corner rests on the node.
    fn relift(&mut self) {
        let basis = placement_basis(&self.base());
        let lift = prop_shape::wedge_lift(basis, self.size);
        lift_limbs(self.skin.limb(), self.collider.as_mut(), lift);
    }
}

#[godot_dyn]
impl WaveSolid for WaveWedge {
    fn set_material(&mut self, mat: &Gd<Material>) {
        self.skin.set_material(mat);
    }
}

/// The node's own scale, if there is anything worth folding into a size
/// knob — read once, on entering the tree, exactly where the wall reads its
/// own basis. `None` when the node is already at 1, which is every node in
/// the shipped map.
///
/// It is deliberately the LOCAL scale and not the inherited one: a prefab
/// scaled as a whole should carry its contents with it, and only a wall
/// (whose centerline is physics, not decoration) refuses that.
fn scale_to_fold(node: &Gd<StaticBody3D>) -> Option<Vector3> {
    let scale = node.get_scale();
    (!prop_shape::scale_is_neutral(scale)).then_some(scale)
}

/// Put the node back at scale 1 and say what was folded, naming the node.
///
/// The mirror of the wall's "use the length knob, never scale": there, the
/// scale is discarded; here it is ABSORBED, because a prop's knob is a free
/// extent and can hold it. Either way the S key stops being a second size
/// knob that the Inspector cannot see — which is what lets a knob read as
/// prefab documentation.
fn drop_scale(node: &mut Gd<StaticBody3D>, scale: Vector3, lost: &str) {
    let name = node.get_name();
    node.set_scale(Vector3::ONE);
    godot_warn!(
        "'{name}': folded the node scale {scale} into the size knob and reset the \
         scale to 1 — a shape's knob is its one size, so the Inspector reads what is \
         actually drawn.{lost}"
    );
}

/// The basis a shape is actually drawn under: its whole global placement
/// when it is in the tree — the standing law is about the world floor, and
/// a shape hangs under its ancestors' turn as much as its own — and its own
/// local basis when it is not. A knob set during a scene load runs before
/// the node has a parent, and asking for a global transform there is an
/// engine error rather than a transform; `_ready` re-lifts on the real
/// placement a moment later regardless.
fn placement_basis(node: &Gd<StaticBody3D>) -> Basis {
    if node.is_inside_tree() {
        node.get_global_transform().basis
    } else {
        node.get_transform().basis
    }
}

/// Cut a wedge of `size` into the given mesh and hull — the one place the
/// generated geometry crosses into the engine, so `_ready` and a dragged
/// knob cannot disagree about what a wedge is.
///
/// Each vertex takes a CUSTOM0 ordinal from
/// [`prop_shape::WEDGE_TRIANGLE_ORDINALS`], one entry per triangle: that
/// table's own order already matches [`prop_shape::wedge_triangles`]'s —
/// both are built from the same eight triangles in the same order — so
/// `triangle_index / 3` (three vertices per triangle) reads the right
/// ordinal without a second lookup.
fn cut_wedge(mesh: &mut Gd<ArrayMesh>, shape: &mut Gd<ConvexPolygonShape3D>, size: Vector3) {
    let triangles: Vec<(Vector3, Vector3, f32)> = prop_shape::wedge_triangles(size)
        .into_iter()
        .enumerate()
        .map(|(i, (v, n))| (v, n, prop_shape::WEDGE_TRIANGLE_ORDINALS[i / 3]))
        .collect();
    render::paint::resize_triangle_surface(mesh, &triangles);
    shape.set_points(&PackedVector3Array::from(&prop_shape::wedge_hull(size)));
}

/// Ride the two limbs a shape built for itself up onto its lift — and ONLY
/// those two. Walking the node's children instead would teleport whatever a
/// designer nested under it, silently burying a prop grouped inside a
/// barrel; nothing in the engine would notice and no seam would draw.
fn lift_limbs(
    skin: Option<&mut Gd<MeshInstance3D>>,
    collider: Option<&mut Gd<CollisionShape3D>>,
    lift: Vector3,
) {
    if let Some(skin) = skin {
        skin.set_position(lift);
    }
    if let Some(collider) = collider {
        collider.set_position(lift);
    }
}
