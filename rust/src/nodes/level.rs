//! The level root — the one node a scene of walls, props, a fan and a
//! spawn marker hangs under, and the engine's single door into it. The
//! composition root injects the data-pass material and the wave pool
//! HERE, once; the level distributes them to every child that renders or
//! sounds. When it enters the tree it builds the floor and ceiling slabs
//! from its `extents` knob and DERIVES the technical contracts the
//! systems run on — wall centerlines, the fan's hum room, the spawn, the
//! dev demo tap — via the pure [`level_plan`] math, so a designer who
//! moves a wall has moved the contracts with it.
//!
//! Hum-room decision: the room is DERIVED from the walls standing around
//! the fan, not hand-authored as a rect knob — a designer who can drag
//! walls should never have to retype their coordinates, and a rect that
//! cannot drift from the walls needs no test to keep it honest. The cost
//! is a law: the fan must be enclosed on all four sides, and the level
//! says so loudly when it is not.
//!
//! Spawn decision: a `Marker3D` child named `SpawnPoint`, standing ON
//! the floor, facing where the hero should look — the designer drags and
//! rotates a gizmo; the engine lifts it to capsule height.

use godot::classes::{Engine, INode3D, Marker3D, Material, MeshInstance3D, Node3D, StaticBody3D};
use godot::prelude::*;

use super::fan::SoundFan;
use super::wall::{WaveProp, WaveWall, build_box};
use crate::level_plan;

/// One floor or ceiling slab, its parts kept for live reshaping when the
/// designer drags the extents knob.
struct Slab {
    body: Gd<StaticBody3D>,
    skin: Gd<MeshInstance3D>,
    mesh: Gd<godot::classes::BoxMesh>,
    shape: Gd<godot::classes::BoxShape3D>,
    /// True for the ceiling, false for the floor.
    lid: bool,
}

/// Everything the level walk can find under the root, collected in one
/// pass and consumed by injection and derivation alike.
#[derive(Default)]
struct Census {
    walls: Vec<Gd<WaveWall>>,
    props: Vec<Gd<WaveProp>>,
    fan: Option<Gd<SoundFan>>,
    spawn: Option<Gd<Marker3D>>,
}

/// The level root node. `inject` BEFORE adding it to the tree — children
/// run `_ready` first, and the fan refuses to build uninjected; then read
/// the derived contracts through the typed getters.
#[derive(GodotClass)]
#[class(tool, init, base=Node3D)]
pub struct WaveLevel {
    /// Floor and ceiling extent in meters, spanning from the level's
    /// origin along +X/+Z — the map's ground plan.
    #[export]
    #[var(get = get_extents, set = set_extents)]
    #[init(val = Vector2::new(20.0, 20.0))]
    extents: Vector2,
    data_mat: Option<Gd<Material>>,
    pulses: Option<Gd<RefCounted>>,
    slabs: Vec<Slab>,
    segments: Vec<Vector4>,
    hum_rect: Vector4,
    spawn_at: Vector3,
    spawn_heading: f64,
    tap_point: Vector3,
    #[init(val = Vector3::UP)]
    tap_normal: Vector3,
    fan_child: Option<Gd<SoundFan>>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WaveLevel {
    fn ready(&mut self) {
        self.build_slabs();
        if Engine::singleton().is_editor_hint() {
            return; // the editor wants shapes to drag, not contracts
        }
        // no silent nulls: an uninjected level would render nothing and
        // sound nothing — say so once, loudly, and still derive honest
        // geometry so the contracts stay readable
        if self.data_mat.is_none() || self.pulses.is_none() {
            godot_error!("WaveLevel: data_mat/pulses not injected — the level cannot be seen");
        }
        self.derive();
    }
}

#[godot_api]
impl WaveLevel {
    /// The single injection point: the composition root hands the level
    /// the data-pass material and the wave pool ONCE, before adding it
    /// to the tree, and the level distributes them — the material to
    /// every wall, prop and slab, both to the fan.
    #[func]
    fn inject(&mut self, data_mat: Gd<Material>, pulses: Gd<RefCounted>) {
        self.data_mat = Some(data_mat.clone());
        self.pulses = Some(pulses.clone());
        let census = self.census();
        for mut wall in census.walls {
            wall.bind_mut().set_material(&data_mat);
        }
        for mut prop in census.props {
            prop.bind_mut().set_material(&data_mat);
        }
        if let Some(mut fan) = census.fan {
            fan.set("pulses", &pulses.to_variant());
            fan.set("data_mat", &data_mat.to_variant());
        }
        for slab in &mut self.slabs {
            slab.skin.set_material_override(&data_mat);
        }
    }

    /// The extents knob reshapes floor and ceiling live, meshes,
    /// colliders and centers together.
    #[func]
    fn set_extents(&mut self, extents: Vector2) {
        self.extents = extents;
        let size = Vector3::new(extents.x, level_plan::SLAB_T as f32, extents.y);
        for slab in &mut self.slabs {
            slab.body.set_position(slab_center(extents, slab.lid));
            slab.mesh.set_size(size);
            slab.shape.set_size(size);
        }
    }

    #[func]
    fn get_extents(&self) -> Vector2 {
        self.extents
    }

    /// The fan's room as wall-centerline bounds (x_min, z_min, x_max,
    /// z_max): hum waves reveal nothing beyond it — walls stop air.
    /// ZERO when the level has no fan or the fan stands unenclosed.
    #[func]
    fn hum_room(&self) -> Vector4 {
        self.hum_rect
    }

    /// Where the hero wakes: the SpawnPoint marker lifted to capsule
    /// height.
    #[func]
    fn spawn_pos(&self) -> Vector3 {
        self.spawn_at
    }

    /// The way the hero faces on waking (yaw, radians) — the marker's.
    #[func]
    fn spawn_yaw(&self) -> f64 {
        self.spawn_heading
    }

    /// Dev-demo tap: a fixed point on the hum room's west wall. ZERO
    /// when no hum room derived.
    #[func]
    fn demo_tap(&self) -> Vector3 {
        self.tap_point
    }

    /// The demo-tapped wall's outward normal, toward the spawn side.
    #[func]
    fn demo_tap_normal(&self) -> Vector3 {
        self.tap_normal
    }

    /// Every wall's centerline as (x1, z1, x2, z2) — the derived table
    /// the suites hold their invariants against.
    #[func]
    fn wall_segments(&self) -> PackedVector4Array {
        PackedVector4Array::from(&self.segments[..])
    }

    /// The level's sound source, when it has one — the composition root
    /// drives its animation clock.
    #[func]
    fn fan(&self) -> Option<Gd<SoundFan>> {
        self.fan_child.clone()
    }

    /// Wall height in meters — a build dimension, served as a static
    /// method: ClassDB constants are integers only.
    #[func]
    fn wall_height() -> f64 {
        level_plan::WALL_H
    }

    /// Derive every technical contract from the children as they stand:
    /// centerlines from the walls, the hum room from the walls around
    /// the fan, the spawn from the marker, the demo tap from the room's
    /// west wall. Loud about whatever a designer left unplaceable.
    fn derive(&mut self) {
        let census = self.census();
        self.segments = census.walls.iter().map(|w| w.bind().segment()).collect();
        self.fan_child = census.fan;
        let lift = Vector3::new(0.0, level_plan::SPAWN_LIFT as f32, 0.0);
        if let Some(marker) = census.spawn {
            self.spawn_at = marker.get_global_position() + lift;
            self.spawn_heading = f64::from(marker.get_rotation().y);
        } else {
            godot_error!("WaveLevel: no SpawnPoint marker — the hero has nowhere to wake");
            self.spawn_at = self.base().get_global_position() + lift;
            self.spawn_heading = 0.0;
        }
        let Some(fan) = self.fan_child.as_ref() else {
            return; // a fanless level is legal: silence, no room to clip
        };
        let at = fan.get_global_position();
        let Some(room) = level_plan::room_around(&self.segments, at.x, at.z) else {
            godot_error!(
                "WaveLevel: the fan is not enclosed by walls — its hum will reveal everywhere"
            );
            return;
        };
        self.hum_rect = room;
        if let Some(plan) = level_plan::demo_tap(&self.segments, room, self.spawn_at) {
            self.tap_point = plan.point;
            self.tap_normal = plan.normal;
        }
    }

    /// Floor and ceiling: thin slabs spanning the extents; only their
    /// inward faces are ever seen.
    fn build_slabs(&mut self) {
        for lid in [false, true] {
            let built = build_box(
                Vector3::new(self.extents.x, level_plan::SLAB_T as f32, self.extents.y),
                Vector3::ZERO,
                self.data_mat.as_ref(),
            );
            let mut body = StaticBody3D::new_alloc();
            body.set_position(slab_center(self.extents, lid));
            body.add_child(&built.skin);
            body.add_child(&built.collider);
            self.base_mut().add_child(&body);
            self.slabs.push(Slab {
                body,
                skin: built.skin,
                mesh: built.mesh,
                shape: built.shape,
                lid,
            });
        }
    }

    /// One walk over the whole subtree, every engine child collected —
    /// grouping folders a designer may have added included.
    fn census(&self) -> Census {
        let mut census = Census::default();
        collect(&self.base().clone().upcast::<Node>(), &mut census);
        census
    }
}

/// The recursive half of [`WaveLevel::census`]: depth-first, scene
/// order — the deterministic order every derivation tiebreak leans on.
fn collect(node: &Gd<Node>, census: &mut Census) {
    for child in node.get_children().iter_shared() {
        if let Ok(wall) = child.clone().try_cast::<WaveWall>() {
            census.walls.push(wall);
        } else if let Ok(prop) = child.clone().try_cast::<WaveProp>() {
            census.props.push(prop);
        } else if let Ok(fan) = child.clone().try_cast::<SoundFan>() {
            census.fan.get_or_insert(fan);
        } else if let Ok(marker) = child.clone().try_cast::<Marker3D>()
            && marker.get_name() == "SpawnPoint"
        {
            census.spawn.get_or_insert(marker);
        }
        collect(&child, census);
    }
}

/// Where a slab's body stands: centered on the extents, the floor's top
/// exactly at y = 0, the ceiling's underside exactly at wall height.
fn slab_center(extents: Vector2, lid: bool) -> Vector3 {
    let y = if lid {
        level_plan::WALL_H + level_plan::SLAB_T * 0.5
    } else {
        -level_plan::SLAB_T * 0.5
    };
    Vector3::new(extents.x * 0.5, y as f32, extents.y * 0.5)
}
