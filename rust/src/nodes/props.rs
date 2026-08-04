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

use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{
    ArrayMesh, BoxMesh, BoxShape3D, ConvexPolygonShape3D, CylinderMesh, CylinderShape3D,
    IStaticBody3D, Material, Mesh, Shape3D, StaticBody3D,
};
use godot::prelude::*;

use super::solid::{Skin, WaveSolid, build_body, build_box};
use crate::prop_shape;

/// A free box obstacle — table top, chair leg, crate, shelf. The node sits
/// at the box's CENTER; `size` is its full extent. Unlike a wall it may
/// rotate freely: props carry no room contract, and waves outline them from
/// any angle.
#[derive(GodotClass)]
#[class(tool, init, base=StaticBody3D)]
pub struct WaveProp {
    /// Full box extent in meters.
    #[export]
    #[var(get = get_size, set = set_size)]
    #[init(val = Vector3::new(0.5, 0.5, 0.5))]
    size: Vector3,
    skin: Skin,
    mesh: Option<Gd<BoxMesh>>,
    shape: Option<Gd<BoxShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveProp {
    fn ready(&mut self) {
        let built = build_box(self.size, Vector3::ZERO, self.skin.material());
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

    /// The id this prop carries — the engine-facing read-back of
    /// [`WaveSolid::oid`].
    #[func]
    fn oid(&self) -> f64 {
        WaveSolid::oid(self)
    }
}

#[godot_dyn]
impl WaveSolid for WaveProp {
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

/// A round obstacle — barrel, pipe, pillar, stove flue. The node stands at
/// the cylinder's BASE, so `y = 0` puts it on the floor. Its silhouette is
/// the one curve in a world of corners, which is the whole reason it
/// exists: with a flat object id across it, the outline pass draws no
/// interior seam and the shape reads purely as its rounded outline.
#[derive(GodotClass)]
#[class(tool, init, base=StaticBody3D)]
pub struct WaveColumn {
    /// Radius in meters.
    #[export]
    #[var(get = get_radius, set = set_radius)]
    #[init(val = 0.28)]
    radius: f64,
    /// Height in meters, rising from the node.
    #[export]
    #[var(get = get_height, set = set_height)]
    #[init(val = 0.9)]
    height: f64,
    skin: Skin,
    mesh: Option<Gd<CylinderMesh>>,
    shape: Option<Gd<CylinderShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveColumn {
    fn ready(&mut self) {
        let mut mesh = CylinderMesh::new_gd();
        // fewer segments than the engine default: a flat object id means the
        // facets never draw as creases, so the count buys nothing but
        // vertices past the point the OUTLINE stops looking polygonal
        mesh.set_radial_segments(24);
        mesh.set_rings(1);
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
        self.mesh = Some(mesh);
        self.shape = Some(shape);
        self.reshape();
    }
}

#[godot_api]
impl WaveColumn {
    #[func]
    fn set_radius(&mut self, radius: f64) {
        self.radius = radius;
        self.reshape();
    }

    #[func]
    fn get_radius(&self) -> f64 {
        self.radius
    }

    #[func]
    fn set_height(&mut self, height: f64) {
        self.height = height;
        self.reshape();
    }

    #[func]
    fn get_height(&self) -> f64 {
        self.height
    }

    /// The id this column carries — the engine-facing read-back of
    /// [`WaveSolid::oid`].
    #[func]
    fn oid(&self) -> f64 {
        WaveSolid::oid(self)
    }

    /// Mesh, collider and lift together, so a knob dragged in the
    /// Inspector moves what the waves strike and not only what is drawn.
    /// The lift is half the height, which is what puts the BASE on the
    /// node — the engine's cylinder primitives are centred.
    fn reshape(&mut self) {
        let radius = self.radius.abs() as f32;
        let height = self.height.abs() as f32;
        let lift = Vector3::new(0.0, height * 0.5, 0.0);
        if let Some(mesh) = self.mesh.as_mut() {
            mesh.set_top_radius(radius);
            mesh.set_bottom_radius(radius);
            mesh.set_height(height);
        }
        if let Some(shape) = self.shape.as_mut() {
            shape.set_radius(radius);
            shape.set_height(height);
        }
        for child in self.base().get_children().iter_shared() {
            if let Ok(mut node) = child.try_cast::<Node3D>() {
                node.set_position(lift);
            }
        }
    }
}

#[godot_dyn]
impl WaveSolid for WaveColumn {
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
    /// Full extent in meters: run along X, rise along Y, width along Z.
    #[export]
    #[var(get = get_size, set = set_size)]
    #[init(val = Vector3::new(1.2, 0.5, 0.8))]
    size: Vector3,
    skin: Skin,
    mesh: Option<Gd<ArrayMesh>>,
    shape: Option<Gd<ConvexPolygonShape3D>>,
    base: Base<StaticBody3D>,
}

#[godot_api]
impl IStaticBody3D for WaveWedge {
    fn ready(&mut self) {
        // the geometry is generated BEFORE the shape is attached: a
        // ConvexPolygonShape3D with no points cannot build a hull, and the
        // engine says so loudly the instant a collider is given one
        let mut mesh = ArrayMesh::new_gd();
        let mut shape = ConvexPolygonShape3D::new_gd();
        cut_wedge(&mut mesh, &mut shape, self.size);
        let built = build_body(
            &mesh.clone().upcast::<Mesh>(),
            &shape.clone().upcast::<Shape3D>(),
            lift_of(self.size),
            self.skin.material(),
        );
        let mut base = self.base_mut();
        base.add_child(&built.skin);
        base.add_child(&built.collider);
        drop(base);
        self.skin.adopt(built.skin);
        self.mesh = Some(mesh);
        self.shape = Some(shape);
    }
}

#[godot_api]
impl WaveWedge {
    #[func]
    fn set_size(&mut self, size: Vector3) {
        self.size = size;
        self.reshape();
    }

    #[func]
    fn get_size(&self) -> Vector3 {
        self.size
    }

    /// The id this wedge carries — the engine-facing read-back of
    /// [`WaveSolid::oid`].
    #[func]
    fn oid(&self) -> f64 {
        WaveSolid::oid(self)
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
        let lift = lift_of(size);
        for child in self.base().get_children().iter_shared() {
            if let Ok(mut node) = child.try_cast::<Node3D>() {
                node.set_position(lift);
            }
        }
    }
}

#[godot_dyn]
impl WaveSolid for WaveWedge {
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

/// Where a wedge's limbs sit above its node: half a height, which is what
/// makes the shape STAND on the node rather than straddle it.
fn lift_of(size: Vector3) -> Vector3 {
    Vector3::new(0.0, size.y.abs() * 0.5, 0.0)
}

/// Cut a wedge of `size` into the given mesh and hull — the one place the
/// generated geometry crosses into the engine, so `_ready` and a dragged
/// knob cannot disagree about what a wedge is.
fn cut_wedge(mesh: &mut Gd<ArrayMesh>, shape: &mut Gd<ConvexPolygonShape3D>, size: Vector3) {
    let mut verts = PackedVector3Array::new();
    let mut normals = PackedVector3Array::new();
    for (v, n) in prop_shape::wedge_triangles(size) {
        verts.push(v);
        normals.push(n);
    }
    let mut arrays = Array::<Variant>::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &verts.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    mesh.clear_surfaces();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    shape.set_points(&PackedVector3Array::from(&prop_shape::wedge_hull(size)));
}
