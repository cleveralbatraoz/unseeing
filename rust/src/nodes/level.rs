//! The level root — the one node a scene of walls, props, a fan, a cat and
//! a spawn marker hangs under, and the engine's single door into it. The
//! composition root injects the two data-writing materials (the world skin
//! and the source image) and the wave pool HERE, once; the level deals
//! them by node class to every child that renders or sounds, and hands the
//! occluding skins the wall table their analytic sight test runs against.
//! When it enters the tree it builds the floor and ceiling slabs from its
//! `extents` knob and DERIVES the technical contracts the systems run on —
//! wall centerlines, the spawn, the dev demo tap — via the pure
//! [`level_plan`] math, so a designer who moves a wall has moved the
//! contracts with it.
//!
//! Occlusion decision: the fan's waves are stopped by the WALLS themselves
//! now — source→surface sight in the data core — not clipped to a derived
//! room rectangle. So a designer may open the fan's room to a corridor
//! without retyping anything or tripping an enclosure law: the hum simply
//! lights what it can reach and stops at what it cannot.
//!
//! Spawn decision: a `Marker3D` child named `SpawnPoint`, standing ON
//! the floor, facing where the hero should look — the designer drags and
//! rotates a gizmo; the engine lifts it to capsule height.

use godot::classes::{
    Engine, INode3D, Marker3D, Material, MeshInstance3D, Node3D, ShaderMaterial, StaticBody3D,
};
use godot::prelude::*;

use super::cat::WaveCat;
use super::fan::SoundFan;
use super::wall::{WaveProp, WaveWall, build_box};
use crate::level_plan;
use crate::oid_palette;
use crate::sight;

/// The floor's flat object id — dedicated, clear of every wall's, because
/// every wall meets the floor and that seam must always draw.
const OID_FLOOR: f64 = 0.15;

/// The ceiling's flat object id — dedicated for the same reason.
const OID_CEIL: f64 = 0.9;

/// The palette every wall and prop is coloured from. Walls and props share
/// ONE palette because a prop leaning on a wall needs the same separation
/// two walls do — the old split palettes left them only 0.06 apart, half
/// strength. Entries are 0.09 apart, above the shader's 0.08 knee rather
/// than exactly on it, and the whole band is clear of the floor below
/// (0.15) and of the creature band above (the cat at 0.7), because every
/// box stands on the floor and anything may walk in front of one.
///
/// Five entries is not a limit on how many boxes a level may hold: ids are
/// assigned by colouring the touch graph, so a hundred walls reuse these
/// five freely and only differ where they actually meet.
const WORLD_OIDS: [f64; 5] = [0.25, 0.34, 0.43, 0.52, 0.61];

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
    cats: Vec<Gd<WaveCat>>,
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
    source_mat: Option<Gd<Material>>,
    pulses: Option<Gd<RefCounted>>,
    slabs: Vec<Slab>,
    segments: Vec<Vector4>,
    occluders: Vec<Vector4>,
    spawn_at: Vector3,
    spawn_heading: f64,
    tap_point: Vector3,
    #[init(val = Vector3::UP)]
    tap_normal: Vector3,
    fan_child: Option<Gd<SoundFan>>,
    cat_children: Vec<Gd<WaveCat>>,
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
        if self.data_mat.is_none() || self.source_mat.is_none() || self.pulses.is_none() {
            godot_error!("WaveLevel: materials/pulses not injected — the level cannot be seen");
        }
        self.derive();
    }
}

#[godot_api]
impl WaveLevel {
    /// The single injection point: the composition root hands the level
    /// its two data-writing materials and the wave pool ONCE, before
    /// adding it to the tree, and the level deals them by node class — the
    /// WORLD skin (real depth) to walls, props, the cat and the slabs; the
    /// source IMAGE skin to the fan (`source_mat`, through its `data_mat`
    /// property: it IS the fan's data-writing material); the pool to the
    /// fan and the cat. A designer never assigns any of this: dropping a
    /// node under the level is enough.
    #[func]
    fn inject(&mut self, data_mat: Gd<Material>, source_mat: Gd<Material>, pulses: Gd<RefCounted>) {
        self.data_mat = Some(data_mat.clone());
        self.source_mat = Some(source_mat.clone());
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
            fan.set("data_mat", &source_mat.to_variant());
        }
        // creatures render and sound like the world: the wave pool voices
        // their footfalls, the world skin draws their outline at real depth
        for mut cat in census.cats {
            cat.set("pulses", &pulses.to_variant());
            cat.set("data_mat", &data_mat.to_variant());
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

    /// Dev-demo tap: a fixed point on the wall between the spawn and the
    /// fan. ZERO when hero and fan share a room (no wall to strike).
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

    /// The inflated wall OCCLUDER rects (`sight::wall_rect`), truncated to
    /// the shader's slots — the very table the level pushes to the
    /// data-writing skins, exposed so the composition root can hand it to
    /// the hearing pass too, which cuts player-sound shells by these walls.
    #[func]
    fn wall_rects(&self) -> PackedVector4Array {
        PackedVector4Array::from(&self.occluders[..])
    }

    /// The level's sound source, when it has one — the composition root
    /// drives its animation clock.
    #[func]
    fn fan(&self) -> Option<Gd<SoundFan>> {
        self.fan_child.clone()
    }

    /// The level's companion creatures — the composition root drives each
    /// one's clock (`tick`) every frame, exactly as it drives the fan.
    #[func]
    fn cats(&self) -> Array<Gd<WaveCat>> {
        self.cat_children.iter().cloned().collect()
    }

    /// Wall height in meters — a build dimension, served as a static
    /// method: ClassDB constants are integers only.
    #[func]
    fn wall_height() -> f64 {
        level_plan::WALL_H
    }

    /// How muffled a source's SILHOUETTE at `to` reads from the eye at
    /// `from`: `SOURCE_THROUGH` per wall the sight line crosses — a faint
    /// ghost through one wall, fainter through two. The composition root
    /// computes this once per frame per active source and hands it to that
    /// source's skin as one uniform, so a source dims as a COHERENT WHOLE
    /// through a wall instead of splitting bright/dim along the wall's
    /// screen edge — a per-object muffle where a per-fragment sight test
    /// would tear. General to any source, not the fan alone.
    #[func]
    fn source_muffle(&self, from: Vector3, to: Vector3) -> f64 {
        let crossings = sight::crossings(from, to, &self.occluders, level_plan::WALL_H as f32);
        level_plan::SOURCE_THROUGH.powi(crossings as i32)
    }

    /// Derive every technical contract from the children as they stand:
    /// centerlines from the walls, the spawn from the marker, the demo tap
    /// from the wall between the spawn and the fan. Loud about whatever a
    /// designer left unplaceable.
    fn derive(&mut self) {
        let census = self.census();
        self.segments = census.walls.iter().map(|w| w.bind().segment()).collect();
        self.push_wall_table();
        self.assign_oids(&census);
        self.fan_child = census.fan;
        self.cat_children = census.cats;
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
            return; // a fanless level is legal: silence, no source to strike toward
        };
        let fan_at = fan.get_global_position();
        if let Some(plan) = level_plan::demo_tap(&self.segments, self.spawn_at, fan_at) {
            self.tap_point = plan.point;
            self.tap_normal = plan.normal;
        }
    }

    /// Hand every box in the world its flat object id (the data pass's
    /// `u_oid`) so the outline post-pass draws one clean silhouette per box
    /// instead of its interior corners. The floor and ceiling carry
    /// dedicated ids clear of every wall's, because every wall meets them —
    /// that seam must always draw; walls and props cycle small palettes so
    /// neighbours differ and the seam between two of them survives. The
    /// world stays below the creature id band (0.7+), so the cat and the
    /// hero's body always separate from the geometry behind them.
    fn assign_oids(&mut self, census: &Census) {
        // the boxes whose ids are already spoken for: the slabs every wall
        // stands on, and the fan, which paints its own limbs
        let mut anchors: Vec<oid_palette::Fixed> = Vec::new();
        for slab in &mut self.slabs {
            let oid = if slab.lid { OID_CEIL } else { OID_FLOOR };
            slab.skin
                .set_instance_shader_parameter("u_oid", &oid.to_variant());
            if let Some(area) = mesh_world_box(&slab.skin.clone().upcast()) {
                anchors.push(oid_palette::Fixed { area, oid });
            }
        }
        if let Some(fan) = census.fan.as_ref()
            && let Some(area) = mesh_world_box(&fan.clone().upcast())
        {
            // one box for both fan ids: the union over-constrains a
            // neighbour slightly, which is the safe direction to err
            for oid in super::fan::OIDS {
                anchors.push(oid_palette::Fixed { area, oid });
            }
        }

        // walls first, then props, so a box's place in this list is the
        // deterministic scene order every other derivation leans on
        let mut areas: Vec<oid_palette::Box3> = Vec::new();
        let mut walls: Vec<usize> = Vec::new();
        let mut props: Vec<usize> = Vec::new();
        for (i, wall) in census.walls.iter().enumerate() {
            if let Some(area) = mesh_world_box(&wall.clone().upcast()) {
                walls.push(i);
                areas.push(area);
            }
        }
        for (i, prop) in census.props.iter().enumerate() {
            if let Some(area) = mesh_world_box(&prop.clone().upcast()) {
                props.push(i);
                areas.push(area);
            }
        }

        let painted = oid_palette::assign(&areas, &anchors, &WORLD_OIDS);
        if painted.starved > 0 {
            godot_error!(
                "WaveLevel: {} box(es) could not take an id distinct from everything they touch — \
                 those seams will not draw. Spread the geometry or widen WORLD_OIDS.",
                painted.starved
            );
        }
        for (slot, &i) in walls.iter().enumerate() {
            census.walls[i]
                .clone()
                .bind_mut()
                .set_oid(painted.oids[slot]);
        }
        for (offset, &i) in props.iter().enumerate() {
            let slot = walls.len() + offset;
            census.props[i]
                .clone()
                .bind_mut()
                .set_oid(painted.oids[slot]);
        }
    }

    /// Tell the occluding skins where the walls stand: the derived
    /// centerlines inflated into shrunk occluder rects ([`sight::wall_rect`]),
    /// pushed as `u_walls`/`u_wall_count`/`u_wall_top` onto the world and
    /// source skins — the wall table their analytic sight test runs
    /// against (the world occludes a source's reveal by them, a wall
    /// behind a wall not lit through; the source silhouette is muffled by
    /// them per-object on the CPU). Loud when a level outgrows the
    /// shader's slots (truncated: the overflow walls stop occluding).
    fn push_wall_table(&mut self) {
        let mut rects: Vec<Vector4> = self.segments.iter().map(|s| sight::wall_rect(*s)).collect();
        if rects.len() > sight::MAXW {
            godot_error!(
                "WaveLevel: {} walls exceed the sight shaders' {} slots — the rest stop occluding",
                rects.len(),
                sight::MAXW
            );
            rects.truncate(sight::MAXW);
        }
        // kept for the per-object source muffle: the walls a camera→source
        // sight line is counted against, once per frame on the CPU
        self.occluders = rects.clone();
        let table = PackedVector4Array::from(&rects[..]);
        let count = rects.len() as i64;
        self.push_table_to(self.data_mat.clone(), &table, count);
        self.push_table_to(self.source_mat.clone(), &table, count);
    }

    /// Push the wall table onto one data-writing material — loud when it is
    /// no ShaderMaterial (legal in tests, blind in the game).
    fn push_table_to(&self, mat: Option<Gd<Material>>, table: &PackedVector4Array, count: i64) {
        let Some(mat) = mat else {
            return; // uninjected: ready() already said so loudly
        };
        match mat.try_cast::<ShaderMaterial>() {
            Ok(mut shader_mat) => {
                shader_mat.set_shader_parameter("u_walls", &table.to_variant());
                shader_mat.set_shader_parameter("u_wall_count", &count.to_variant());
                shader_mat.set_shader_parameter("u_wall_top", &level_plan::WALL_H.to_variant());
            }
            Err(other) => {
                godot_warn!(
                    "WaveLevel: '{}' is not a ShaderMaterial — no wall table, no occlusion",
                    other.get_class(),
                );
            }
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
        } else if let Ok(cat) = child.clone().try_cast::<WaveCat>() {
            census.cats.push(cat);
        } else if let Ok(marker) = child.clone().try_cast::<Marker3D>()
            && marker.get_name() == "SpawnPoint"
        {
            census.spawn.get_or_insert(marker);
        }
        collect(&child, census);
    }
}

/// The world box a node's drawn geometry occupies — the union over every
/// `MeshInstance3D` beneath it, the node itself included. `None` for a node
/// that draws nothing, which can never show a seam with anything.
fn mesh_world_box(node: &Gd<Node>) -> Option<oid_palette::Box3> {
    let mut found: Option<oid_palette::Box3> = None;
    if let Ok(mesh) = node.clone().try_cast::<MeshInstance3D>() {
        found = Some(world_box(mesh.get_aabb(), mesh.get_global_transform()));
    }
    for child in node.get_children().iter_shared() {
        if let Some(area) = mesh_world_box(&child) {
            found = Some(match found {
                Some(acc) => acc.union(&area),
                None => area,
            });
        }
    }
    found
}

/// One local AABB carried into world space and re-squared to the axes. The
/// half-extent is summed through the ABSOLUTE of the basis — the standard
/// trick that bounds a freely rotated prop without trigonometry.
fn world_box(local: Aabb, xf: Transform3D) -> oid_palette::Box3 {
    let center = xf * (local.position + local.size * 0.5);
    let half = local.size * 0.5;
    let (bx, by, bz) = (xf.basis.col_a(), xf.basis.col_b(), xf.basis.col_c());
    let reach = Vector3::new(
        bx.x.abs() * half.x + by.x.abs() * half.y + bz.x.abs() * half.z,
        bx.y.abs() * half.x + by.y.abs() * half.y + bz.y.abs() * half.z,
        bx.z.abs() * half.x + by.z.abs() * half.y + bz.z.abs() * half.z,
    );
    oid_palette::Box3::from_center_size(
        [center.x as f64, center.y as f64, center.z as f64],
        [
            (reach.x * 2.0) as f64,
            (reach.y * 2.0) as f64,
            (reach.z * 2.0) as f64,
        ],
    )
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
