//! The level root — the one node a scene of walls, props, sound sources, a
//! cat and a spawn marker hangs under, and the engine's single door into
//! it. The composition root injects the two data-writing materials (the
//! world skin and the source image) and the wave pool HERE, once; the level
//! deals them out and hands the occluding skins the wall table their
//! analytic sight test runs against. When it enters the tree it builds the
//! floor and ceiling slabs from its `extents` knob and DERIVES the technical
//! contracts the systems run on — wall centerlines, the spawn, the dev demo
//! tap — via the pure [`level_plan`] math, so a designer who moves a wall
//! has moved the contracts with it.
//!
//! THE LEVEL NAMES NO SHAPES AND NO SOURCES. It walks its subtree once and
//! sorts every child into two abstractions: [`WaveSolid`] — anything the
//! waves can strike, box or column or wedge or wall — and [`SoundSource`] —
//! anything that makes the world's own sound, fan or radio. Both are Rust
//! traits published to the engine with `#[godot_dyn]`, so
//! [`godot::obj::Gd::try_dynify`] recognises a child by what it CAN DO
//! rather than by what class it is. A new prop shape or a new kind of
//! source is a new file; nothing in this one changes.
//!
//! Occlusion decision: a source's waves are stopped by the WALLS themselves
//! — source→surface sight in the data core — not clipped to a derived room
//! rectangle. So a designer may open a source's room to a corridor without
//! retyping anything or tripping an enclosure law: the waves simply light
//! what they can reach and stop at what they cannot.
//!
//! Spawn decision: a `Marker3D` child named exactly `SpawnPoint`, standing
//! ON the floor, facing where the hero should look — the designer drags and
//! rotates a gizmo; the engine lifts it to capsule height. Every OTHER
//! marker whose name reads like a spawn is collected too — the `SpawnPoint2`
//! Ctrl+D leaves behind, a second exact name under another parent — not to
//! compete with it, but so the level can NAME what it ignored instead of
//! letting a moved copy change nothing in silence.
//!
//! Tap decision: the dev demo strike aims at the sound source whose HUB IS
//! NEAREST THE SPAWN in the XZ plane, ties broken by the node's name in
//! ascending order — never the first source in scene order, because a row
//! dragged in the Scene dock is an authoring convenience and not a contract,
//! and never a slice index or anything hashed, which the determinism law
//! forbids. The plane is the measure because the tap IS a wall crossing read
//! in it. If no wall stands between the two the strike cannot be planned and
//! the level says so, since the alternative is a zeroed tap firing at the
//! world origin; a level with NO source stays silent, which is legal.

use godot::classes::{
    Engine, INode3D, Marker3D, Material, MeshInstance3D, Node3D, ShaderMaterial, StaticBody3D,
};
use godot::obj::DynGd;
use godot::prelude::*;

use super::cat::WaveCat;
use super::solid::{OID_PARAM, WaveSolid, build_box, clear_limbs};
use super::source::SoundSource;
use super::wall::WaveWall;
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
/// Five entries is not a limit on how many solids a level may hold: ids are
/// assigned by colouring the touch graph, so a hundred walls reuse these
/// five freely and only differ where they actually meet.
const WORLD_OIDS: [f64; 5] = [0.25, 0.34, 0.43, 0.52, 0.61];

/// The names the level writes on the two slab bodies it builds for itself —
/// its own limbs, recognised on the way back in exactly as a solid
/// recognises its mesh and collider (see [`clear_limbs`]). Without them a
/// second `_ready` — a scene re-entered after `request_ready()` — would
/// stack a second floor and ceiling inside the first.
const SLAB_NAMES: [&str; 2] = ["WaveFloor", "WaveCeiling"];

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
/// pass and consumed by injection and derivation alike. `solids` holds
/// every strikeable shape in scene order — the walls among them — while
/// `walls` keeps the typed handles the centerline table needs.
#[derive(Default)]
struct Census {
    solids: Vec<DynGd<Node, dyn WaveSolid>>,
    walls: Vec<Gd<WaveWall>>,
    sources: Vec<DynGd<Node, dyn SoundSource>>,
    cats: Vec<Gd<WaveCat>>,
    /// EVERY marker whose name reads as a spawn — the exact one and any
    /// auto-numbered copy Ctrl+D left behind, in walk order. The winner is
    /// still the first exact name, but a copy that was never collected
    /// could never be reported either, which is the whole bug: the level
    /// has to see what it refuses in order to say it refused it.
    spawns: Vec<Gd<Marker3D>>,
}

/// The level root node. `inject` BEFORE adding it to the tree — children
/// run `_ready` first, and a source refuses to build uninjected; then read
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
    source_children: Vec<DynGd<Node, dyn SoundSource>>,
    cat_children: Vec<Gd<WaveCat>>,
    /// The walls the occluder table was built FROM, in table order, kept so
    /// a crossing can be named without re-walking the scene — and, more
    /// importantly, so the names cannot drift out of step with the table.
    /// See [`Self::wall_names`].
    wall_children: Vec<Gd<WaveWall>>,
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
    /// adding it to the tree, and the level deals them out — the WORLD skin
    /// (real depth) to every solid, the cat and the slabs; the source IMAGE
    /// skin plus the pool to every sound source, through the one
    /// [`SoundSource::inject`] door. A designer never assigns any of this:
    /// dropping a node under the level is enough.
    #[func]
    fn inject(&mut self, data_mat: Gd<Material>, source_mat: Gd<Material>, pulses: Gd<RefCounted>) {
        // Order is not a preference here, it is the whole contract. By the
        // time the level is in the tree, `derive` has already run: it pushed
        // an EMPTY wall table to skins that did not exist, and it coloured
        // every wall and prop without the sources' ids as anchors — so a
        // source injected now would render with seams that silently do not
        // draw, in a world whose walls no longer occlude. Nothing later can
        // repair either. Say so rather than limp.
        if self.base().is_inside_tree() {
            godot_error!(
                "WaveLevel: inject() after the level entered the tree — the wall table and the \
                 object-id colouring were already derived without it. Inject BEFORE add_child()."
            );
        }
        self.data_mat = Some(data_mat.clone());
        self.source_mat = Some(source_mat.clone());
        self.pulses = Some(pulses.clone());
        let census = self.census();
        for mut solid in census.solids {
            solid.dyn_bind_mut().set_material(&data_mat);
        }
        for mut source in census.sources {
            source
                .dyn_bind_mut()
                .inject(pulses.clone(), source_mat.clone());
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
    pub(super) fn spawn_pos(&self) -> Vector3 {
        self.spawn_at
    }

    /// The way the hero faces on waking (yaw, radians) — the marker's.
    #[func]
    pub(super) fn spawn_yaw(&self) -> f64 {
        self.spawn_heading
    }

    /// Dev-demo tap: a fixed point on the wall between the spawn and the
    /// sound source NEAREST it ([`level_plan::nearest_source`], not the
    /// first in scene order). ZERO when they share a room — which the level
    /// says out loud, because the strike then fires at the world origin —
    /// or when the level is silent, which is legal and says nothing.
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
    pub(super) fn wall_rects(&self) -> PackedVector4Array {
        PackedVector4Array::from(&self.occluders[..])
    }

    /// The level's sound sources, in scene order — exposed as plain nodes
    /// so a suite can read their knobs, while the level itself drives them
    /// through the trait.
    #[func]
    fn sources(&self) -> Array<Gd<Node3D>> {
        self.source_children
            .iter()
            .filter_map(|s| s.clone().into_gd().try_cast::<Node3D>().ok())
            .collect()
    }

    /// The level's companion creatures — the composition root drives each
    /// one's clock (`tick`) every frame.
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

    /// Occluder slots the sight shaders allocate ([`sight::MAXW`]) — the
    /// ceiling [`level_plan::wall_budget`] measures a level's headroom
    /// against, served the same way [`Self::wall_height`] is.
    ///
    /// It exists so the number can be READ BACK from the engine layer. The
    /// constant lives in two languages — here, and as `MAXW` in
    /// `game/shaders/pulse_pool.gdshaderinc` — and the level now quotes it
    /// to a designer as a count of free slots. If the two copies drifted,
    /// that count would be a lie in the most expensive direction: a level
    /// told it has room while the shaders have already stopped occluding.
    /// `game/tests/shader_contract_test.gd` holds them together.
    #[func]
    fn wall_slots() -> i64 {
        sight::MAXW as i64
    }

    /// Wall segments one more room costs a designer
    /// ([`level_plan::ROOM_SEGMENTS`]) — the unit the wall budget's
    /// headroom is measured in, exposed so the suite that pins the shipped
    /// map's headroom and the warning a designer actually sees are
    /// threshold-for-threshold the same number.
    #[func]
    fn room_segments() -> i64 {
        level_plan::ROOM_SEGMENTS as i64
    }

    /// Drive every sound source for one frame: advance its clockwork with
    /// the SIMULATED clock (so movie-maker runs and time scaling stay
    /// correct), then tell it how strongly its standing acoustic image is
    /// felt from `eye` — its own volume, dimmed once per wall between the
    /// eye and its hub.
    ///
    /// The muffle is computed HERE, per source, per frame, on the CPU, and
    /// pushed to that source's limbs as a per-INSTANCE uniform. Two
    /// reasons, and both are load-bearing: a per-fragment sight test would
    /// tear a source's silhouette bright/dim along a wall's screen edge
    /// instead of dimming it as a coherent whole, and a per-MATERIAL
    /// uniform would make the quiet fan and the loud radio — which share
    /// one acoustic-image skin — dim and brighten together.
    #[func]
    fn tick_sources(&mut self, t: f64, eye: Vector3) {
        // cloned handles: the muffle reads the level's wall table while the
        // sources are being driven, and the two must not borrow it at once
        for mut source in self.source_children.clone() {
            let hub;
            let volume;
            {
                let mut voice = source.dyn_bind_mut();
                voice.advance(t);
                hub = voice.hub();
                volume = voice.voice().volume.image();
            }
            let image = volume * self.source_muffle(eye, hub);
            source.dyn_bind_mut().set_image(image);
        }
    }

    /// How muffled a source's SILHOUETTE at `to` reads from the eye at
    /// `from`: `SOURCE_THROUGH` per wall the sight line crosses — a faint
    /// ghost through one wall, fainter through two. General to any source,
    /// not the fan alone; exposed so the suites can hold the law directly.
    #[func]
    fn source_muffle(&self, from: Vector3, to: Vector3) -> f64 {
        let crossings = sight::crossings(from, to, &self.occluders, level_plan::WALL_H as f32);
        level_plan::SOURCE_THROUGH.powi(crossings as i32)
    }

    /// Where the hero wakes, and every word a designer needs about the
    /// markers that did not win. The DECISION is pure and lives in
    /// [`level_plan::choose_spawn`]; this end only measures — the winner's
    /// world position lifted to capsule height, the level's own origin as
    /// the fallback, and each candidate's path under the level root, which
    /// is the only thing that tells two markers named `SpawnPoint` apart.
    fn derive_spawn(&mut self, markers: &[Gd<Marker3D>]) {
        let lift = Vector3::new(0.0, level_plan::SPAWN_LIFT as f32, 0.0);
        let fallback = self.base().get_global_position() + lift;
        let root = self.base().clone().upcast::<Node>();
        // the two lists are grown in ONE pass, so a verdict's index cannot
        // slide off the marker it names — the walk already applied the same
        // predicate, but a filter that ever disagreed with it would silently
        // wake the hero at the wrong node
        let mut kept: Vec<&Gd<Marker3D>> = Vec::new();
        let mut candidates: Vec<level_plan::SpawnCandidate> = Vec::new();
        for marker in markers {
            let Some(kind) = level_plan::spawn_name(&marker.get_name().to_string()) else {
                continue; // renamed since the walk: no longer a spawn marker
            };
            kept.push(marker);
            candidates.push(level_plan::SpawnCandidate {
                path: root.get_path_to(marker).to_string(),
                kind,
            });
        }
        let verdict = level_plan::choose_spawn(&candidates, fallback);
        for complaint in &verdict.complaints {
            godot_error!("{}", complaint);
        }
        match verdict.winner.and_then(|slot| kept.get(slot)) {
            Some(marker) => {
                self.spawn_at = marker.get_global_position() + lift;
                self.spawn_heading = f64::from(marker.get_rotation().y);
            }
            None => {
                self.spawn_at = fallback;
                self.spawn_heading = 0.0;
            }
        }
    }

    /// Where the input-less demo strikes. The DECISION is pure and lives in
    /// [`level_plan::plan_demo_tap`] — which source, which wall, and what to
    /// say when there is no wall at all; this end only reads each source's
    /// hub and name off the live nodes.
    fn derive_tap(&mut self) {
        let aims: Vec<level_plan::SourceAim> = self
            .source_children
            .iter()
            .map(|source| level_plan::SourceAim {
                name: source.clone().into_gd().get_name().to_string(),
                hub: source.dyn_bind().hub(),
            })
            .collect();
        let verdict = level_plan::plan_demo_tap(&self.segments, self.spawn_at, &aims);
        if let Some(plan) = verdict.plan {
            self.tap_point = plan.point;
            self.tap_normal = plan.normal;
        }
        if let Some(complaint) = verdict.complaint {
            godot_error!("{}", complaint);
        }
    }

    /// Derive every technical contract from the children as they stand:
    /// centerlines from the walls, the spawn from its marker, the demo tap
    /// from the wall between the spawn and the nearest source. Loud about
    /// whatever a designer left unplaceable.
    fn derive(&mut self) {
        let census = self.census();
        self.segments = census.walls.iter().map(|w| w.bind().segment()).collect();
        self.push_wall_table();
        self.assign_oids(&census);
        self.source_children = census.sources;
        self.cat_children = census.cats;
        self.wall_children = census.walls;
        self.derive_spawn(&census.spawns);
        self.derive_tap();
    }

    /// Hand every solid in the world its flat object id (the data pass's
    /// `u_oid`) so the outline post-pass draws one clean silhouette per
    /// object instead of its interior corners. The floor and ceiling carry
    /// dedicated ids clear of every wall's, because every wall meets them —
    /// that seam must always draw; the rest are coloured by the touch graph
    /// so neighbours differ and the seam between two of them survives. The
    /// world stays below the creature id band (0.7+), so the cat and the
    /// hero's body always separate from the geometry behind them.
    fn assign_oids(&mut self, census: &Census) {
        // the solids whose ids are already spoken for: the slabs everything
        // stands on, and the sound sources, which paint their own limbs
        let mut anchors: Vec<oid_palette::Fixed> = Vec::new();
        for slab in &mut self.slabs {
            let oid = if slab.lid { OID_CEIL } else { OID_FLOOR };
            slab.skin
                .set_instance_shader_parameter("u_oid", &oid.to_variant());
            if let Some(area) = mesh_world_box(&slab.skin.clone().upcast()) {
                anchors.push(oid_palette::Fixed { area, oid });
            }
        }
        for source in &census.sources {
            let Some(area) = mesh_world_box(&source.clone().into_gd()) else {
                continue; // a source that draws nothing can show no seam
            };
            // grown by whatever the source's moving parts sweep beyond this
            // one pose, so a prop the fan's head only reaches on half its
            // cycle is still banned from the ids it could melt into
            let bound = source.dyn_bind();
            let area = area.grown_flat(bound.sweep_margin());
            // one box for all of a source's ids: the union over-constrains a
            // neighbour slightly, which is the safe direction to err
            for &oid in bound.oids() {
                anchors.push(oid_palette::Fixed { area, oid });
            }
        }

        // scene order is the deterministic order every other derivation
        // leans on, and the tiebreak the colouring itself is stable under
        let mut areas: Vec<oid_palette::Box3> = Vec::new();
        let mut painted: Vec<usize> = Vec::new();
        for (i, solid) in census.solids.iter().enumerate() {
            if let Some(area) = mesh_world_box(&solid.clone().into_gd()) {
                painted.push(i);
                areas.push(area);
            }
        }

        let chosen = oid_palette::assign(&areas, &anchors, &WORLD_OIDS);
        if chosen.starved > 0 {
            godot_error!(
                "WaveLevel: {} solid(s) could not take an id distinct from everything they touch \
                 — those seams will not draw. Spread the geometry or widen WORLD_OIDS.",
                chosen.starved
            );
        }
        for (slot, &i) in painted.iter().enumerate() {
            census.solids[i]
                .clone()
                .dyn_bind_mut()
                .set_oid(chosen.oids[slot]);
        }
    }

    /// Tell the occluding skins where the walls stand: the derived
    /// centerlines inflated into shrunk occluder rects ([`sight::wall_rect`]),
    /// pushed as `u_walls`/`u_wall_count`/`u_wall_top` onto the world and
    /// source skins — the wall table their analytic sight test runs
    /// against.
    ///
    /// Loud about the shaders' slot ceiling BEFORE it is hit as well as
    /// after ([`level_plan::wall_budget`]): a level past it has walls that
    /// silently stopped occluding, and a level one room short of it is
    /// about to. Only the truncation stays here, because it is the act the
    /// message describes — the words themselves are a decision over two
    /// numbers, and live in the pure plan where cargo can hold them.
    fn push_wall_table(&mut self) {
        let mut rects: Vec<Vector4> = self.segments.iter().map(|s| sight::wall_rect(*s)).collect();
        say(level_plan::wall_budget(rects.len(), sight::MAXW));
        rects.truncate(sight::MAXW); // a no-op below the ceiling
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
    ///
    /// Idempotent, because `_ready` is not once-only: a scene re-entering
    /// the tree after `request_ready()` runs it again, and a duplicated
    /// level arrives carrying copies of the slabs the original built. This
    /// build owns the pair — whatever it finds under those names goes.
    ///
    /// BOTH slabs are always built, in this order, in the editor too — the
    /// pair is what `set_extents`, the object-id anchors and the seam
    /// census all read, and a level that carried one slab at edit time and
    /// two at run time would describe two different worlds through the
    /// same accessors. What bends is the DRAWING, per
    /// [`level_plan::slab_drawn`]: the lid is hidden in the editor, where
    /// it would otherwise cover the top-down view the map is laid out in.
    fn build_slabs(&mut self) {
        self.slabs.clear();
        clear_limbs(self, &SLAB_NAMES);
        let editor_hint = Engine::singleton().is_editor_hint();
        for lid in [false, true] {
            let built = build_box(
                Vector3::new(self.extents.x, level_plan::SLAB_T as f32, self.extents.y),
                Vector3::ZERO,
                self.data_mat.as_ref(),
            );
            let mut body = StaticBody3D::new_alloc();
            body.set_name(SLAB_NAMES[usize::from(lid)]);
            body.set_position(slab_center(self.extents, lid));
            // the whole body, so the collision debug draw of a 28 × 28 lid
            // goes with the mesh it belongs to
            body.set_visible(level_plan::slab_drawn(lid, editor_hint));
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

/// Say one shader-budget verdict out loud at the volume it asked for, and
/// nothing at all when there is nothing to say.
///
/// The two volumes are not interchangeable. An overflow is an ERROR because
/// the world the shaders draw no longer matches the scene a designer
/// authored — walls that were placed have stopped occluding. Running out of
/// headroom is a WARNING because nothing is broken yet, and a level that
/// shouted about a ceiling it had not hit would teach the reader to scroll
/// past the one that matters.
fn say(budget: Option<level_plan::Budget>) {
    let Some(budget) = budget else {
        return; // comfortably inside every ceiling: silence is the report
    };
    match budget.severity {
        level_plan::Severity::Error => godot_error!("{}", budget.text),
        level_plan::Severity::Warn => godot_warn!("{}", budget.text),
    }
}

/// One painted box as the object-id colouring sees it: what it is called,
/// the world box it fills, and the flat id it actually carries.
pub(super) struct PaintedSolid {
    pub(super) name: String,
    pub(super) area: oid_palette::Box3,
    pub(super) oid: f64,
}

/// What the debug observer ([`super::observer`]) reads back off a level.
///
/// None of it is `#[func]`: the designer-facing API does not grow for a
/// debugging tool. Nothing here is stored either — every accessor
/// re-derives from the scene as it stands, so the observer reports the
/// world the renderer will actually draw rather than a mirrored copy that
/// can drift away from it.
impl WaveLevel {
    /// The wave pool the composition root injected, exactly as it was
    /// handed over — the GDScript `Pulses` shim in the game, possibly a
    /// bare core in a suite. Resolving it is the observer's job.
    pub(super) fn pulse_handle(&self) -> Option<Gd<RefCounted>> {
        self.pulses.clone()
    }

    /// The world skin, whose shader parameters carry the per-frame globals
    /// (`u_time`, `u_flick`) the composition root pushes every frame.
    pub(super) fn data_material(&self) -> Option<Gd<Material>> {
        self.data_mat.clone()
    }

    /// Every sound source in scene order, still TYPED — [`Self::sources`]
    /// hands out plain nodes, which can no longer answer `hub()`.
    pub(super) fn source_handles(&self) -> &[DynGd<Node, dyn SoundSource>] {
        &self.source_children
    }

    /// The name of every wall in the occluder table, in table order, so a
    /// crossing can be NAMED and not merely counted. Truncated exactly as
    /// the table is: past the shader's last slot a wall does not occlude,
    /// and naming it would describe an occlusion that never happens.
    ///
    /// Read off the handles [`Self::derive`] built the table from, NOT off a
    /// fresh walk of the tree. Two reasons, and the first is correctness:
    /// the table is derived once and `names[i]` claims to name
    /// `occluders[i]`, so a walk that found the scene rearranged — a wall
    /// added, moved to the front, or freed — would slide every name one slot
    /// off its rect and blame an innocent wall for an occlusion. The second
    /// is cost: the walk is O(scene nodes) with a dynamic-cast probe per
    /// node, and a fan of sight lines paid it per ray.
    ///
    /// This is not a mirrored copy of the names. The handles are live and
    /// the NAME is read through them on every call, so a renamed wall
    /// reports its new name; only the identity and the order are pinned,
    /// exactly as `source_children` and `cat_children` already are. A wall
    /// freed since derivation keeps its SLOT, under a placeholder — dropping
    /// it would shift every name after it, which is the bug this exists to
    /// prevent.
    pub(super) fn wall_names(&self) -> Vec<String> {
        self.wall_children
            .iter()
            .take(self.occluders.len())
            .enumerate()
            .map(|(index, wall)| {
                if wall.is_instance_valid() {
                    wall.get_name().to_string()
                } else {
                    format!("<freed wall {index}>")
                }
            })
            .collect()
    }

    /// Every painted box in the level with the id it ACTUALLY carries,
    /// read back off the mesh instances rather than mirrored from the
    /// colouring's own choice.
    ///
    /// The set and the SHAPES are the ones [`Self::assign_oids`] reasons
    /// about, deliberately measured the same way: the solids it paints, the
    /// two slabs everything stands on, and each source under its SWEPT box —
    /// grown by `sweep_margin` exactly as the colouring grows it, because a
    /// check that measured the fan by the single pose it happens to hold
    /// would be weaker than the law it explains, and would clear a crate the
    /// guard ring reaches on half of every cycle.
    ///
    /// The level's creatures are here too, though the colouring does not
    /// paint them: [`WaveCat`] carries a hardcoded id in the 0.7+ band, and a
    /// census that stopped at the static world would give a seam bug
    /// involving the one moving thing in the room a clean bill of health.
    ///
    /// THE HERO'S BODY IS NOT HERE, and cannot be: `HeroBody` is a child of
    /// the composition root, not of the level, so this walk never sees it.
    /// The other occupant of the creature band is therefore outside every
    /// report this function feeds.
    pub(super) fn oid_census(&self) -> Vec<PaintedSolid> {
        let census = self.census();
        let mut painted = Vec::new();
        for slab in &self.slabs {
            let Some(area) = mesh_world_box(&slab.skin.clone().upcast()) else {
                continue; // a slab that draws nothing can show no seam
            };
            painted.push(PaintedSolid {
                name: if slab.lid { "Ceiling" } else { "Floor" }.to_string(),
                area,
                oid: read_oid(&slab.skin),
            });
        }
        for solid in &census.solids {
            let node = solid.clone().into_gd();
            let Some(area) = mesh_world_box(&node) else {
                continue;
            };
            painted.push(PaintedSolid {
                name: node.get_name().to_string(),
                area,
                oid: solid.dyn_bind().oid(),
            });
        }
        for source in &census.sources {
            let node = source.clone().into_gd();
            let Some(area) = mesh_world_box(&node) else {
                continue;
            };
            let name = node.get_name().to_string();
            let bound = source.dyn_bind();
            let area = area.grown_flat(bound.sweep_margin());
            // one box for all of a source's ids, and the same union the
            // colouring anchored on — see `assign_oids`
            for &oid in bound.oids() {
                painted.push(PaintedSolid {
                    name: format!("{name} @{oid}"),
                    area,
                    oid,
                });
            }
        }
        for cat in &census.cats {
            let node = cat.clone().upcast::<Node>();
            let (Some(area), Some(oid)) = (mesh_world_box(&node), painted_oid(&node)) else {
                continue; // a creature that draws nothing can show no seam
            };
            painted.push(PaintedSolid {
                name: node.get_name().to_string(),
                area,
                oid,
            });
        }
        painted
    }
}

/// The flat object id a creature is painted with, read off the first limb
/// that carries one. A creature paints every limb with one id (the whole
/// animal is one silhouette), so the first limb speaks for it; `None` for a
/// node whose limbs were never painted, which the census then leaves out
/// rather than reporting under [`oid_palette::NO_OID`] as though the shader
/// had been handed a real value.
fn painted_oid(node: &Gd<Node>) -> Option<f64> {
    if let Ok(skin) = node.clone().try_cast::<MeshInstance3D>()
        && let Ok(oid) = skin
            .get_instance_shader_parameter(OID_PARAM)
            .try_to::<f64>()
    {
        return Some(oid);
    }
    node.get_children()
        .iter_shared()
        .find_map(|child| painted_oid(&child))
}

/// The flat object id a mesh instance carries right now — the one source
/// of truth, read straight back off the skin the data pass will write
/// from. [`oid_palette::NO_OID`] when nothing has painted it.
fn read_oid(skin: &Gd<MeshInstance3D>) -> f64 {
    skin.get_instance_shader_parameter(OID_PARAM)
        .try_to::<f64>()
        .unwrap_or(oid_palette::NO_OID)
}

/// The recursive half of [`WaveLevel::census`]: depth-first, scene
/// order — the deterministic order every derivation tiebreak leans on.
///
/// A child is recognised by what it CAN DO, not by what it is: `try_dynify`
/// asks the `#[godot_dyn]` registry whether this node's dynamic class
/// implements the trait. The two typed arms that remain are the two that
/// need more than the trait offers — a wall's centerline, a cat's clock.
fn collect(node: &Gd<Node>, census: &mut Census) {
    for child in node.get_children().iter_shared() {
        if let Ok(solid) = child.clone().try_dynify::<dyn WaveSolid>() {
            census.solids.push(solid);
            if let Ok(wall) = child.clone().try_cast::<WaveWall>() {
                census.walls.push(wall);
            }
        } else if let Ok(source) = child.clone().try_dynify::<dyn SoundSource>() {
            census.sources.push(source);
        } else if let Ok(cat) = child.clone().try_cast::<WaveCat>() {
            census.cats.push(cat);
        } else if let Ok(marker) = child.clone().try_cast::<Marker3D>()
            && level_plan::spawn_name(&marker.get_name().to_string()).is_some()
        {
            census.spawns.push(marker);
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
