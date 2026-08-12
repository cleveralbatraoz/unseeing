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
//! Placement decision: the level REPORTS a solid it cannot support and
//! moves nothing. Its slabs span `0 .. extents` from its own origin, so
//! geometry dragged past that has no floor under it and the hero who walks
//! there falls — and until this, in silence. Growing the slabs to cover the
//! stray would be the worse cure, because it changes the footprint of an
//! authored map behind the designer's back. The same holds a metre lower:
//! a box prop is CENTRED on its node, so dropping one on the floor plane
//! buries half of it under the slab where nothing draws or sounds — and
//! centring every shape instead would sink every shelf and beam meant to
//! float. Both faults are reported and neither is repaired. See
//! [`Self::report_placement`].
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
    ArrayMesh, Engine, INode3D, Marker3D, Material, MeshInstance3D, Node3D, ShaderMaterial,
    StaticBody3D,
};
use godot::obj::DynGd;
use godot::prelude::*;

use super::cat::WaveCat;
use super::props::{WaveColumn, WaveProp, WaveWedge};
use super::solid::{
    SKIN_NAME, WaveSolid, basis_columns_f64, build_box, clear_limbs, mesh_first_label, to_f64_3,
};
use super::source::SoundSource;
use super::wall::WaveWall;
use crate::level_plan;
use crate::oid_palette;
use crate::render;
use crate::sight;

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
    mesh: Gd<ArrayMesh>,
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

/// Which concrete thing a [`PaintEntry`] is — the level's own two slabs
/// have no `Skin`-carrying node of their own (no indirection needed to
/// reach their mesh), while every authored solid paints itself back
/// through its own `paint()` method; see [`WaveLevel::paint_entry`].
enum PaintItem {
    Slab { lid: bool },
    Wall(Gd<WaveWall>),
    Prop(Gd<WaveProp>),
    Column(Gd<WaveColumn>),
    Wedge(Gd<WaveWedge>),
}

impl PaintItem {
    /// The `render::paint::ShapeKind` this item's mesh was built with — the
    /// ordinal count [`render::paint::face_count`] answers for.
    fn kind(&self) -> render::paint::ShapeKind {
        match self {
            PaintItem::Slab { .. } => render::paint::ShapeKind::Slab,
            PaintItem::Wall(_) | PaintItem::Prop(_) => render::paint::ShapeKind::Box,
            PaintItem::Column(_) => render::paint::ShapeKind::Column,
            PaintItem::Wedge(_) => render::paint::ShapeKind::Wedge,
        }
    }

    /// The authored scene node this paint entry came from. The two slabs
    /// belong to the level itself, so only authored solids have a node a
    /// configuration warning can be pinned to.
    fn node(&self) -> Option<Gd<Node>> {
        match self {
            PaintItem::Slab { .. } => None,
            PaintItem::Wall(node) => Some(node.clone().upcast()),
            PaintItem::Prop(node) => Some(node.clone().upcast()),
            PaintItem::Column(node) => Some(node.clone().upcast()),
            PaintItem::Wedge(node) => Some(node.clone().upcast()),
        }
    }
}

/// One item [`WaveLevel::paint_labels`] colours: a floor/ceiling slab or an
/// authored solid, carrying its world-space `render::Shape` (what
/// `render::faces` builds faces from) and the world box the touch graph
/// reasons about — the SAME box the retired per-solid `oid_palette::assign`
/// used, measured the identical way.
///
/// A slab's `shape` is a real `Box3d` like any other box (Wave S) — it
/// used to be `None`, back when a slab was fed through `render::faces` not
/// at all; `superface::superfaces`'s own singleton collapse made that
/// workaround unnecessary, so every entry now carries one.
struct PaintEntry {
    name: String,
    area: oid_palette::Box3,
    shape: render::Shape,
    item: PaintItem,
}

/// One face of the real per-face label census [`WaveLevel::paint_labels`]
/// derives and bakes, kept so [`super::observer::WaveObserver::explain_oids`]
/// reads the rendering subsystem's own census rather than re-deriving (and
/// risking a second, possibly-drifting copy of) the merge law. `label` and
/// `class` are read straight off the SAME `out`/`sf` values `paint_labels`
/// bakes into `CUSTOM0` — not recomputed a second time, so the two can
/// never disagree with each other by construction.
pub(super) struct FaceCensusEntry {
    /// The named solid this face belongs to.
    pub(super) name: String,
    /// The face's own world-space geometry.
    pub(super) face: render::Face,
    /// The label actually baked onto every vertex of this face.
    pub(super) label: f64,
    /// The superface class this face was coloured as part of.
    pub(super) class: usize,
}

/// The level root node. `inject` BEFORE adding it to the tree — children
/// run `_ready` first, and a source refuses to build uninjected; then read
/// the derived contracts through the typed getters.
#[derive(GodotClass)]
#[class(tool, init, base=Node3D)]
pub struct WaveLevel {
    /// Floor and ceiling extent in meters, spanning from the level's
    /// origin along +X/+Z — the map's ground plan.
    #[export(range = (4.0, 60.0, 1.0, or_greater, suffix = " m"))]
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
    /// How many solids the last derivation found standing where the floor
    /// does not reach — one per line the level printed. Zero on a healthy
    /// level; see [`Self::report_placement`].
    unfloored: i64,
    /// How many solids the last derivation found crossing or hiding under
    /// the floor plane. Zero on a healthy level, same as above.
    sunken: i64,
    /// The real per-face label census the last `paint_labels` baked — see
    /// [`FaceCensusEntry`] and [`Self::face_census`]. Empty before the
    /// first successful `derive()` (the editor, or a level never added to
    /// the tree), exactly as `segments`/`occluders` start empty.
    face_census: Vec<FaceCensusEntry>,
    /// Every level-wide complaint the last derivation produced — spawn
    /// faults, the demo-tap fault, the wall-budget and pack-range budgets,
    /// the label-starvation count — in the order `derive` produced them.
    /// Rewritten from scratch on EVERY derivation, editor or run, so a
    /// fault a designer already fixed does not linger; this is exactly
    /// what [`Self::get_configuration_warnings`] reports.
    level_faults: Vec<String>,
    /// Every fault the last derivation pinned to a SPECIFIC node — an
    /// unfloored or sunken solid, one entry per authored owner of a
    /// starved face class — so a
    /// consumer can ask "what is wrong with THIS node" rather than read a
    /// level-wide list and guess. Rewritten alongside `level_faults`, same
    /// rule, same reason. See [`Self::faults_for`].
    node_faults: Vec<level_plan::PlacementFault>,
    /// The scene signature ([`Self::scene_signature`]) as of the last
    /// derivation — the condition [`INode3D::process`]'s editor-only poll
    /// watches. Seeded from the FIRST derive in [`INode3D::ready`], so a
    /// scene that has not changed since the level entered the tree does
    /// not re-derive on its very first editor frame.
    last_signature: u64,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WaveLevel {
    fn ready(&mut self) {
        self.build_slabs();
        let editor = Engine::singleton().is_editor_hint();
        // no silent nulls: an uninjected level would render nothing and
        // sound nothing — say so once, loudly, and still derive honest
        // geometry so the contracts stay readable. Never checked in the
        // editor: a scene open there is legitimately uninjected (injection
        // is the composition root's job, at run time), and printing this
        // on every scene open would be noise a designer learns to skip.
        if !editor
            && (self.data_mat.is_none() || self.source_mat.is_none() || self.pulses.is_none())
        {
            godot_error!("WaveLevel: materials/pulses not injected — the level cannot be seen");
        }
        self.derive();
        if editor {
            // seed the condition-watch with what was JUST derived, so the
            // first editor frame after a scene opens sees no change and
            // does not re-derive a second time for nothing
            self.last_signature = self.scene_signature();
            self.base_mut().set_process(true);
        } else {
            // defining `process` below would otherwise enable per-frame
            // processing here too (gdext auto-enables it once the
            // INode3D::process override exists) and charge a running level
            // an O(scene) census walk every frame for a poll only the
            // editor needs — so the runtime branch turns it back off
            // explicitly.
            self.base_mut().set_process(false);
        }
    }

    /// Editor-only: watch the scene for the condition [`Self::derive`]
    /// actually depends on, and re-derive the moment it changes — a
    /// designer drags a wall or a knob and sees the wall table, the
    /// warnings and the object-id colouring update without ever pressing
    /// play or calling [`Self::rederive`] by hand.
    ///
    /// DESIGN: condition-watching, not dirty-flag plumbing. Six classes'
    /// setters and transform notifications would each have to remember to
    /// mark the level dirty, and any one that forgot would be a silent
    /// stale-until-play bug with no test able to see it was missing. A
    /// signature folded fresh every frame cannot forget: it is the same
    /// census `derive` itself would walk (~130 nodes, microseconds), so
    /// whatever `derive` reads, this watches, automatically, forever.
    /// Nothing here runs at run time — see the branch in [`INode3D::ready`]
    /// that turns processing back off outside the editor.
    fn process(&mut self, _delta: f64) {
        if !Engine::singleton().is_editor_hint() {
            return;
        }
        let sig = self.scene_signature();
        if sig != self.last_signature {
            self.last_signature = sig;
            self.derive();
        }
    }

    /// The Scene dock's warning icon, editor-only — the same faults a
    /// running level shouts through `godot_error!`/`godot_warn!`, read back
    /// off `level_faults` instead of the log. [`Self::derive`] calls
    /// [`Node::update_configuration_warnings`] itself after every pass, so
    /// this is asked for exactly when the engine already knows it might
    /// have changed.
    fn get_configuration_warnings(&self) -> PackedStringArray {
        self.level_faults.iter().map(GString::from).collect()
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
    pub(super) fn inject(
        &mut self,
        data_mat: Gd<Material>,
        source_mat: Gd<Material>,
        pulses: Gd<RefCounted>,
    ) {
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
            render::paint::resize_box_surface(&mut slab.mesh, size, super::solid::BOX_ORDINALS);
            slab.shape.set_size(size);
        }
    }

    #[func]
    fn get_extents(&self) -> Vector2 {
        self.extents
    }

    /// Manual refresh: re-run the whole derivation against the scene as it
    /// stands right now. `ready` already calls [`Self::derive`] once, so
    /// this exists for whoever changes the scene AFTER that — the editor,
    /// moving a wall or adding a marker, and this probe. In-tree only, the
    /// same contract `derive` itself carries: `WaveWall::segment` and every
    /// `mesh_world_box` read global transforms, which only exist once the
    /// level has entered the tree.
    ///
    /// Reseeds `last_signature` afterward, exactly as [`INode3D::ready`]
    /// does after its own first derive: without this, [`INode3D::process`]'s
    /// condition-watch would find the scene still differs from whatever
    /// signature it last saw and re-derive a second time on the very next
    /// editor frame, for nothing.
    #[func]
    fn rederive(&mut self) {
        self.derive();
        self.last_signature = self.scene_signature();
    }

    /// The engine-facing read-back of [`INode3D::get_configuration_warnings`]
    /// — needed because that override is a pure GDVIRTUAL: Godot's editor
    /// calls it directly through the C++ virtual table and never binds it
    /// to `ClassDB`, so no script can reach it (measured: neither
    /// `has_method` nor `.call()` finds it, on this class or on a bare
    /// `Node3D`). Exactly the same shape as [`WaveWall::oid`] disambiguating
    /// from [`WaveSolid::oid`] — an inherent `#[func]` of the same name,
    /// so a suite can see what the override would have told the Scene
    /// dock.
    #[func]
    fn get_configuration_warnings(&self) -> PackedStringArray {
        INode3D::get_configuration_warnings(self)
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
    pub(super) fn demo_tap(&self) -> Vector3 {
        self.tap_point
    }

    /// The demo-tapped wall's outward normal, toward the spawn side.
    #[func]
    pub(super) fn demo_tap_normal(&self) -> Vector3 {
        self.tap_normal
    }

    /// How many solids stand where the level's floor does not reach — zero
    /// on a healthy level, and one per complaint the level printed while
    /// deriving. Exposed as a number, not as the sentences, because the
    /// sentences are for a person and this is for whatever has to DECIDE
    /// something: a suite holding the shipped map silent today, and the
    /// editor-side warning that has yet to be built.
    #[func]
    fn unfloored_solids(&self) -> i64 {
        self.unfloored
    }

    /// How many solids cross the floor plane or hide under it — zero on a
    /// healthy level, and read for the same reasons as
    /// [`Self::unfloored_solids`].
    #[func]
    fn sunken_solids(&self) -> i64 {
        self.sunken
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

    /// Every fault the last derivation pinned to `node` specifically — an
    /// unfloored/sunken placement, or a starved face-class seam — matched by the
    /// same `root.get_path_to` address every entry in `node_faults`
    /// carries. Not `#[func]`: this is [`super::solid::warnings_from_level`]'s
    /// door into the level, called from every solid's own
    /// `get_configuration_warnings`, not a designer-facing knob.
    pub(super) fn faults_for(&self, node: &Gd<Node>) -> PackedStringArray {
        let root = self.base().clone().upcast::<Node>();
        let path = root.get_path_to(node).to_string();
        self.node_faults
            .iter()
            .filter(|fault| fault.path == path)
            .map(|fault| GString::from(&fault.text))
            .collect()
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
    pub(super) fn cats(&self) -> Array<Gd<WaveCat>> {
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

    /// The range the sight shaders pack camera distance into
    /// ([`level_plan::DIST_PACK_RANGE`]) — the ceiling on how big a map may
    /// be, served so `game/tests/shader_contract_test.gd` can hold this
    /// mirror against the include's own `DIST_PACK_RANGE`, which is the
    /// copy that actually renders. Nothing else could catch that drift: a
    /// level measuring itself against 40 while the shader packs against 30
    /// would call a broken map fine.
    #[func]
    fn pack_range() -> f64 {
        level_plan::DIST_PACK_RANGE
    }

    /// Debug-facing shim, served the same way [`Self::wall_height`] is: not
    /// a designer knob, only a door for `game/tests/mesh_label_test.gd` to
    /// reach [`render::paint::labelled_box`] — the spike proving `CUSTOM0`
    /// rides a gdext `ArrayMesh` at all. `face_labels` must carry exactly
    /// six entries, read −X,+X,−Y,+Y,−Z,+Z; a wrong count is reported
    /// rather than guessed at, and an empty box drawn instead of a wrong
    /// one — there is no "closest" reading of a five- or seven-entry array
    /// that would not be a silent lie about which face got which label.
    #[func]
    fn debug_labelled_box(
        size: Vector3,
        lift: Vector3,
        face_labels: PackedFloat32Array,
    ) -> Gd<ArrayMesh> {
        let Ok(labels): Result<[f32; 6], _> = face_labels.to_vec().try_into() else {
            godot_error!(
                "WaveLevel.debug_labelled_box: face_labels had {} entries, not the 6 a box's \
                 faces need (−X,+X,−Y,+Y,−Z,+Z) — returning an empty mesh rather than guessing \
                 which face a wrong-length array meant.",
                face_labels.len()
            );
            return ArrayMesh::new_gd();
        };
        render::paint::labelled_box(size, lift, labels)
    }

    /// Debug-facing shim, the triangle path's sibling of
    /// [`Self::debug_labelled_box`] and served for the same reason: no
    /// SHIPPED caller of [`render::paint::resize_triangle_surface`] ever
    /// varies the label it hands the same mesh — a column and a wedge
    /// write placeholder ordinals, and every creature, viewmodel and
    /// source limb bakes one constant role label — so the one behaviour
    /// that separates that function from
    /// [`render::paint::resize_triangle_surface_preserving_labels`] is
    /// unreachable from any node, and would go untested without a door.
    ///
    /// Rebuilds `mesh` in place as ONE triangle whose three vertices all
    /// carry `label`, exactly the way a per-frame builder rebuilds its own
    /// limb buffer, and hands the same resource back.
    #[func]
    fn debug_triangle_surface(mesh: Gd<ArrayMesh>, label: f32) -> Gd<ArrayMesh> {
        let mut mesh = mesh;
        let triangles = [
            (Vector3::ZERO, Vector3::UP, label),
            (Vector3::RIGHT, Vector3::UP, label),
            (Vector3::FORWARD, Vector3::UP, label),
        ];
        render::paint::resize_triangle_surface(&mut mesh, &triangles);
        mesh
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
    pub(super) fn tick_sources(&mut self, t: f64, eye: Vector3) {
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
    ///
    /// `editor` is [`Self::derive`]'s one read of `Engine::is_editor_hint`:
    /// every complaint is ALWAYS filed into `level_faults` (the totals
    /// rewrite an editor's warning icon reads), and printed through
    /// `godot_error!` only at run time, byte-identical to before — the
    /// boot gate reads exactly these prints and must keep seeing them.
    fn derive_spawn(&mut self, markers: &[Gd<Marker3D>], editor: bool) {
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
        for complaint in verdict.complaints {
            if !editor {
                godot_error!("{}", complaint);
            }
            self.level_faults.push(complaint);
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
    /// hub and name off the live nodes. `editor` behaves exactly as it does
    /// in [`Self::derive_spawn`].
    fn derive_tap(&mut self, editor: bool) {
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
            if !editor {
                godot_error!("{}", complaint);
            }
            self.level_faults.push(complaint);
        }
    }

    /// Derive every technical contract from the children as they stand:
    /// centerlines from the walls, the spawn from its marker, the demo tap
    /// from the wall between the spawn and the nearest source. Loud about
    /// whatever a designer left unplaceable — at run time through
    /// `godot_error!`/`godot_warn!`, and in EITHER mode into `level_faults`
    /// / `node_faults`, which are cleared and rewritten here on every call
    /// rather than accumulated, so a fault a designer already fixed cannot
    /// linger in the next warning read. Runs in the editor now as well as
    /// at run time — `ready` no longer returns before it — so a designer
    /// sees the level's complaints while dragging, not only after pressing
    /// play; [`Self::rederive`] is the manual replay of this same pass.
    ///
    /// The level's own icon is not the only one that can go stale: a fault
    /// this pass pinned to one solid's path a moment ago may no longer
    /// apply to it, so every censused solid is told to refresh its icon
    /// too, right after the level tells the engine about its own.
    ///
    /// Both refreshes are DEFERRED (`call_deferred`), never called
    /// straight — and now that [`INode3D::process`] can reach `derive`
    /// every editor frame, on top of [`Self::rederive`] and
    /// [`INode3D::ready`], that is load-bearing rather than tidy. `derive`
    /// always runs with `self` exclusively bound (every caller holds
    /// `&mut self`), and `update_configuration_warnings` can make the
    /// Scene dock read a warning back SYNCHRONOUSLY through
    /// `node_configuration_warning_changed`: a solid's override walks up
    /// to THIS level and binds it again (`warnings_from_level` →
    /// `faults_for`), and the level's own override would re-enter itself
    /// the same way. Either is a bind attempted while this call is still
    /// holding the exclusive one — a re-entrancy panic in the designer's
    /// face, not a hypothetical. Deferring moves both reads to idle time,
    /// after `derive` has returned and the bind is released. A probe never
    /// sees the difference: every check below reads the fault store
    /// through the `#[func]` forwarders (`get_configuration_warnings`,
    /// `faults_for`), synchronously, never through the dock's cached copy.
    fn derive(&mut self) {
        let editor = Engine::singleton().is_editor_hint();
        self.level_faults.clear();
        self.node_faults.clear();
        let census = self.census();
        self.segments = census.walls.iter().map(|w| w.bind().segment()).collect();
        self.push_wall_table(editor);
        self.report_pack_range(editor);
        self.paint_labels(&census, editor);
        self.report_placement(&census, editor);
        self.source_children = census.sources;
        self.cat_children = census.cats;
        self.wall_children = census.walls;
        self.derive_spawn(&census.spawns, editor);
        self.derive_tap(editor);
        // the Scene dock's warning icon only repaints when told to; every
        // fault site above just rewrote level_faults, so this is the one
        // place that needs to say so — deferred, see the doc comment above
        self.base_mut()
            .call_deferred("update_configuration_warnings", &[]);
        // node_faults was rewritten too, and a solid's own icon reads it
        // through solid::warnings_from_level — refresh every censused
        // solid so a cleared fault stops showing and a new one starts,
        // deferred for the same reentrancy reason
        for solid in &census.solids {
            solid
                .clone()
                .into_gd()
                .call_deferred("update_configuration_warnings", &[]);
        }
    }

    /// Say out loud what a designer has placed where the level cannot hold
    /// it — off the floor's footprint ([`level_plan::unfloored`]) or
    /// through its top ([`level_plan::sunken`]). Both DECISIONS are pure;
    /// this end only measures — the floor slab where it actually stands,
    /// and each painted solid's world box — and both read the same one
    /// walk of the subtree, since the two faults are two questions about
    /// one set of boxes.
    ///
    /// Nothing MOVES here, deliberately, and neither origin law bends.
    /// Growing the slabs to cover stray geometry would silently change the
    /// footprint of an authored map; centring every shape on its node
    /// would sink every shelf and beam that is meant to float. Both cures
    /// are worse than the faults, so the level reports and leaves it.
    ///
    /// A level with no floor to measure against — nothing built to stand
    /// on — has no verdict rather than an early return, so the counts are
    /// rewritten on EVERY derivation. An early return would leave the last
    /// build's numbers standing as this build's, which is the quietest kind
    /// of wrong a report can be.
    ///
    /// `editor` reaches a designer now too: every fault is always filed
    /// into `node_faults` (Task 6 reads it per-node), and `godot_error!` —
    /// unchanged text — fires only at run time, exactly as it always has.
    fn report_placement(&mut self, census: &Census, editor: bool) {
        let (strays, sunk) = match self.floor_box() {
            Some(floor) => {
                let placed = self.placed_solids(census);
                (
                    level_plan::unfloored(floor, &placed),
                    level_plan::sunken(floor, &placed),
                )
            }
            None => (Vec::new(), Vec::new()),
        };
        self.unfloored = strays.len() as i64;
        self.sunken = sunk.len() as i64;
        for fault in strays.into_iter().chain(sunk) {
            if !editor {
                godot_error!("{}", fault.text);
            }
            self.node_faults.push(fault);
        }
    }

    /// The world box of the floor slab — what "the floor" MEANS to every
    /// placement law: the footprint that has a slab under it, and the
    /// plane its top stands at. Read where the slab actually is rather
    /// than from the extents knob, so a level dropped anywhere in the
    /// world carries its own footprint with it. `None` before the slabs
    /// are built, or for a floor that draws nothing.
    fn floor_box(&self) -> Option<oid_palette::Box3> {
        let floor = self.slabs.iter().find(|slab| !slab.lid)?;
        mesh_world_box(&floor.skin.clone().upcast())
    }

    /// The world box of the ceiling slab — symmetric with [`Self::floor_box`],
    /// and the other half of the pair [`Self::report_pack_range`] measures
    /// the map's own diagonal against. `None` before the slabs are built.
    fn ceiling_box(&self) -> Option<oid_palette::Box3> {
        let ceiling = self.slabs.iter().find(|slab| slab.lid)?;
        mesh_world_box(&ceiling.skin.clone().upcast())
    }

    /// Every painted solid with the world box it fills and the path a
    /// designer finds it at — the shape the placement laws read.
    ///
    /// A solid that draws nothing is left out: it occupies no space, so
    /// there is nowhere for it to be misplaced. The box is
    /// [`mesh_world_box`]'s, the same measure the object-id colouring and
    /// the seam census take, so a complaint and a seam always describe the
    /// same shape — including that measure's stop at a nested censused
    /// child: a prop grouped under a crate keeps its OWN box here, so a
    /// placement fault blames whichever of the two actually sits wrong,
    /// never the parent for where the child sits.
    fn placed_solids(&self, census: &Census) -> Vec<level_plan::PlacedSolid> {
        let root = self.base().clone().upcast::<Node>();
        census
            .solids
            .iter()
            .filter_map(|solid| {
                let node = solid.clone().into_gd();
                mesh_world_box(&node).map(|area| level_plan::PlacedSolid {
                    path: root.get_path_to(&node).to_string(),
                    area,
                })
            })
            .collect()
    }

    /// Bake every solid in the world its real per-face label — the
    /// derive-time paint pass: solids become `render::Shape`s, shapes
    /// become world-space faces, coplanar overlapping faces MERGE into one
    /// superface class ([`render::superfaces`]), and the resulting graph is
    /// coloured ([`render::assign`]) so no two classes a seam must draw
    /// between ever land within [`render::MIN_SEP`] of each other. The
    /// floor and ceiling carry dedicated role labels clear of every wall's,
    /// because every wall meets them — that seam must always draw; the
    /// rest are coloured by the SAME touch graph the old per-solid
    /// colouring used ([`oid_palette::Box3::touches`], see [`Self::census`]
    /// callers), so neighbours differ and the seam between two of them
    /// survives, UNLESS their faces genuinely coplanar-overlap, in which
    /// case they now MELT together on purpose — the whole point of this
    /// campaign.
    ///
    /// The shader reads `CUSTOM0` straight through for G now, so baking it
    /// onto each solid's mesh is the whole of painting — there is no
    /// per-instance uniform left to keep in step with it.
    ///
    /// Starvation is both a level-wide warning and one node warning for
    /// every authored solid that owns a starved superface class. The latter
    /// is deliberately mapped from `classes_of_entry`, not from a retired
    /// per-solid colour slot: one solid can now own several face classes,
    /// and one merged class can belong to several solids.
    fn paint_labels(&mut self, census: &Census, editor: bool) {
        let entries = self.paint_entries(census);

        // the touch graph — the SAME law the retired per-solid
        // `oid_palette::assign` used, over the SAME world boxes:
        // `superfaces`'s own merge pass needs no touch information at all
        // (it tests every face pair directly), but the separation rules
        // (b)/(c) do, exactly as the old colouring needed the touch graph
        // to know which solids must differ.
        let areas: Vec<oid_palette::Box3> = entries.iter().map(|e| e.area).collect();
        let mut touching: Vec<(usize, usize)> = Vec::new();
        for i in 0..areas.len() {
            for j in (i + 1)..areas.len() {
                if areas[i].touches(&areas[j]) {
                    touching.push((i, j));
                }
            }
        }

        // every entry's world-space faces, concatenated in entry order,
        // alongside each face's own ORDINAL within its entry — for a
        // well-formed (non-degenerate) shape this is simply its position
        // in `render::faces`' own output, which is Task 5's ordinal
        // contract: every builder emits its faces/triangles in the
        // identical order `render::faces` re-derives them in.
        //
        // SLABS CONTRIBUTE REAL FACES HERE, same as any other box — the
        // phantom-class workaround this comment used to describe is gone.
        // `superface::superfaces`'s own SINGLETON COLLAPSE (Wave S) makes
        // it safe: a floor or ceiling never genuinely MERGES with
        // anything (anything resting on it presents an OPPOSITE-facing
        // surface — a buried abutment, never a same-direction coplanar
        // overlap), so it is alone in its own cluster and its six faces
        // fold into ONE class before rule (a) ever runs, exactly as the
        // spec's own law promises ("singletons keep today's exact look:
        // one label across the whole solid"). That one real class is
        // anchored to its role label through the NORMAL anchor path below
        // — the same `(class, label)` mechanism any other fixed class
        // takes — rather than a slab-specific phantom one.
        let mut faces: Vec<render::Face> = Vec::new();
        let mut ordinal_of_face: Vec<usize> = Vec::new();
        let mut refused: Vec<bool> = vec![false; entries.len()];
        for (i, entry) in entries.iter().enumerate() {
            let entry_faces = render::faces(i, &entry.shape);
            // THE ORDINAL CONTRACT'S OWN GUARD: `ordinal_of_face` below
            // trusts that `render::faces`' i-th entry for this solid IS
            // face ordinal i — true whenever every one of the shape's
            // PLANAR faces survives (Task 5's contract), false the moment
            // a degenerate size folds one away (`face_from_poly` refuses
            // a collapsed polygon). Silently accepting a SHORT list here
            // would mislabel every ordinal past the gap — not a face
            // missing a colour, a face wearing another face's colour.
            // Total instead: refuse this ONE solid outright — its faces
            // never enter the census AND `refused` keeps the bake loop
            // below from calling `paint_entry` for it at all, so its mesh
            // keeps the placeholder ordinals its builder wrote rather than
            // risking a silently wrong label on the ones that did survive.
            // (Skipping the bake is the load-bearing half: `relabel` maps
            // EVERY in-range placeholder ordinal, so a bake with the
            // all-zero `labels_by_ordinal` this entry would get flattens
            // the whole mesh to 0.0 — out of band, and 0.05 from
            // `Role::Case`.)
            //
            // A column's own expectation is ONE LESS than
            // `render::paint::face_count`'s own ordinal count, on
            // purpose: that count spans every CUSTOM0 ordinal the MESH
            // carries, flank included, but the flank has no plane at all
            // and `render::faces` never emits an entry for it — a
            // healthy column always reports exactly 2 (its two rims),
            // never 3, and treating 3 as the target would refuse every
            // column in the level.
            let kind = entry.item.kind();
            let expected = match kind {
                render::paint::ShapeKind::Column => render::paint::face_count(kind) - 1,
                _ => render::paint::face_count(kind),
            };
            if entry_faces.len() != expected {
                godot_error!(
                    "WaveLevel: '{}' built {} planar face(s) from its shape, not the {} it \
                     should — a degenerate size folded one or more away. Its own seams cannot be \
                     painted correctly this derive; skipping it rather than mislabeling by \
                     position. Give every extent a real size.",
                    entry.name,
                    entry_faces.len(),
                    expected
                );
                refused[i] = true;
                continue;
            }
            ordinal_of_face.extend(0..entry_faces.len());
            faces.extend(entry_faces);
        }

        let sf = render::superfaces(&faces, &touching);

        // a column's curved flank has no plane at all — give it its own
        // permanently-singleton class (see `render::paint::add_flank_classes`
        // for the full justification)
        let flank_solids: Vec<usize> = entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.item, PaintItem::Column(_)))
            .map(|(i, _)| i)
            .collect();
        let (flank_class, classes1, seps1) =
            render::paint::add_flank_classes(&sf, &faces, &touching, &flank_solids);
        let mut flank_class_of: Vec<Option<usize>> = vec![None; entries.len()];
        for (slot, &i) in flank_solids.iter().enumerate() {
            flank_class_of[i] = Some(flank_class[slot]);
        }

        // every class (real or flank) each entry owns — the floor/ceiling
        // anchors below fix ALL of a slab's own classes to its role label,
        // and the source bans below need the full set of classes whatever
        // touches a source actually owns
        let mut classes_of_entry: Vec<Vec<usize>> = vec![Vec::new(); entries.len()];
        for (fi, face) in faces.iter().enumerate() {
            classes_of_entry[face.solid].push(sf.class_of[fi]);
        }
        for (i, flank) in flank_class_of.iter().enumerate() {
            if let Some(flank) = *flank {
                classes_of_entry[i].push(flank);
            }
        }

        // the floor and ceiling anchor their own REAL classes to their
        // fixed role label directly — the NORMAL `(class, label)` anchor
        // path any other fixed class takes, now that a slab is a boxed
        // singleton like any other and contributes real classes of its
        // own (Wave S) rather than none at all. A slab's own six faces
        // all collapsed to the SAME one class (the singleton law), so
        // this anchors that one class, once, per slab — not a phantom
        // class banning neighbours the way a source's swept envelope
        // still needs below, since a source contributes no real face to
        // the census at all.
        let mut direct_anchors: Vec<(usize, f64)> = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            let PaintItem::Slab { lid } = entry.item else {
                continue;
            };
            let role = if lid {
                render::Role::Ceiling
            } else {
                render::Role::Floor
            };
            let label = render::role_label(role);
            let mut seen: Vec<usize> = Vec::new();
            for &c in &classes_of_entry[i] {
                if !seen.contains(&c) {
                    seen.push(c);
                    direct_anchors.push((c, label));
                }
            }
        }

        // sound sources stay OUT of the face census entirely (their limbs
        // bake their own role labels directly into CUSTOM0), but their
        // FIXED ids still have to ban the world palette entries near them
        // for whatever touches their swept envelope —
        // `render::labels::role_label(Shell)` (0.33) sits a centimetre from
        // the world palette's own 0.34, and without a ban a wall or a crate
        // touching a source would be free to land there.
        let mut extra_anchors: Vec<(f64, Vec<usize>)> = Vec::new();
        for source in &census.sources {
            let Some(source_area) = mesh_world_box(&source.clone().into_gd()) else {
                continue; // a source that draws nothing can show no seam
            };
            let bound = source.dyn_bind();
            let source_area = source_area.grown_flat(bound.sweep_margin());
            let mut touching_classes: Vec<usize> = Vec::new();
            for (i, entry) in entries.iter().enumerate() {
                if entry.area.touches(&source_area) {
                    for &c in &classes_of_entry[i] {
                        if !touching_classes.contains(&c) {
                            touching_classes.push(c);
                        }
                    }
                }
            }
            if touching_classes.is_empty() {
                continue; // nothing touches this source: no ban needed
            }
            for &oid in bound.oids() {
                extra_anchors.push((oid, touching_classes.clone()));
            }
        }
        let (classes, separations, mut anchors) =
            render::paint::add_anchor_classes(classes1, &seps1, &extra_anchors);
        anchors.extend(direct_anchors);
        let augmented = render::Superfaces {
            class_of: sf.class_of.clone(),
            classes,
            separations,
            cluster_of_solid: sf.cluster_of_solid.clone(),
        };
        let out = render::assign(&augmented, &anchors, &WORLD_OIDS);
        // Wave S: the singleton collapse (`render::superface::superfaces`)
        // closed the gap that used to starve the palette here — a lone
        // box now costs the world palette exactly what it always did
        // before this campaign, and two touching, un-merged solids need
        // ONE separation, not rule (a)'s old unscoped six. Loud again,
        // matching the pre-superface `assign_oids` voice: a starved class
        // is a seam that will not draw, and the shipped-map pin
        // (`game/tests/map_test.gd::test_shipped_level_derives_with_no_starved_classes`)
        // holds the count at zero. In the editor, file the same words for
        // the warning triangle instead of flooding the output panel.
        if out.starved > 0 {
            let message = format!(
                "WaveLevel: {} superface class(es) could not take a label distinct from \
                 everything they touch — those seams will not draw. Spread the geometry or \
                 widen WORLD_OIDS.",
                out.starved
            );
            if !editor {
                godot_error!("{}", message);
            }
            self.level_faults.push(message);
        }

        // A class, unlike the retired object-id slot, may belong to more
        // than one solid after coplanar faces merge. Walk every authored
        // entry once and pin one warning to it if ANY class it owns starved;
        // slabs have no separate scene node and remain represented by the
        // level-wide warning above.
        let root = self.base().clone().upcast::<Node>();
        for entry_index in
            render::paint::starved_entry_indices(&out.starved_classes, &classes_of_entry)
        {
            let Some(entry) = entries.get(entry_index) else {
                continue; // helper is total; retain that law at this boundary
            };
            let Some(node) = entry.item.node() else {
                continue;
            };
            self.node_faults.push(level_plan::PlacementFault {
                path: root.get_path_to(&node).to_string(),
                text: "one or more face classes cannot take a label distinct from everything they \
                       touch — those seams will not draw."
                    .to_string(),
            });
        }
        // bake: gather each entry's own labels by ordinal and rewrite its
        // mesh's CUSTOM0 — the shader's own G-channel source now.
        //
        // A REFUSED entry is skipped entirely: it contributed no face to
        // the census, so every one of its ordinals would bake the `0.0`
        // fill below, and `relabel` would write that over all of its
        // vertices. Leaving it alone is what actually keeps its
        // placeholder ordinals on the mesh — pinned by
        // `level_test.gd::test_a_degenerate_solid_is_refused_not_mislabelled`.
        for (i, entry) in entries.iter().enumerate() {
            if refused[i] {
                continue;
            }
            let n = render::paint::face_count(entry.item.kind());
            let mut labels_by_ordinal: Vec<f32> = vec![0.0; n];
            for (fi, face) in faces.iter().enumerate() {
                if face.solid == i {
                    let ord = ordinal_of_face[fi];
                    if let Some(slot) = labels_by_ordinal.get_mut(ord) {
                        *slot = out.label_of_class[sf.class_of[fi]] as f32;
                    }
                }
            }
            if let Some(flank) = flank_class_of[i]
                && let Some(slot) = labels_by_ordinal.get_mut(2)
            {
                *slot = out.label_of_class[flank] as f32;
            }
            self.paint_entry(&entry.item, &labels_by_ordinal);
        }

        // record what actually got baked, by FACE — not the ordinal-baked
        // mesh bytes, but the exact `(class, label)` pair every face above
        // was baked from, so the debug census reads the rendering
        // subsystem's own numbers rather than a second derivation of them.
        // Column flanks are deliberately absent: they have no polygon at
        // all (`render::faces::column_faces` never emits one for the
        // curved flank), so they can never enter a coplanar-overlap
        // predicate in the first place.
        self.face_census = faces
            .iter()
            .enumerate()
            .map(|(fi, f)| FaceCensusEntry {
                name: entries[f.solid].name.clone(),
                face: f.clone(),
                label: out.label_of_class[sf.class_of[fi]],
                class: sf.class_of[fi],
            })
            .collect();

        // the wall-merge voice: any non-wall solid sharing a MERGE cluster
        // with a wall is drawn as part of the wall structure now — say so.
        let is_wall: Vec<bool> = entries
            .iter()
            .map(|e| matches!(e.item, PaintItem::Wall(_)))
            .collect();
        let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
        for message in render::paint::wall_merge_warnings(&sf.cluster_of_solid, &is_wall, &names) {
            godot_warn!("{}", message);
        }
    }

    /// One item per floor/ceiling slab and per authored solid — everything
    /// [`Self::paint_labels`] colours — with the `render::Shape` each
    /// builds its faces from and the world box the touch graph reasons
    /// about, both measured the identical way the old colouring measured
    /// them.
    fn paint_entries(&self, census: &Census) -> Vec<PaintEntry> {
        let mut entries = Vec::new();
        for slab in &self.slabs {
            let Some(area) = mesh_world_box(&slab.skin.clone().upcast()) else {
                continue; // a slab with no mesh draws no seam
            };
            // the same world-space Box3d shape a wall or a prop reads off
            // its own node (`WaveWall::world_shape` et al.) — a slab's
            // BODY carries its world position (built at lift ZERO, never
            // rotated), and its own `BoxShape3D` carries the full extent.
            let transform = slab.body.get_global_transform();
            let shape = render::Shape::Box3d {
                center: to_f64_3(transform.origin),
                size: to_f64_3(slab.shape.get_size()),
                basis: basis_columns_f64(transform.basis),
            };
            entries.push(PaintEntry {
                name: if slab.lid { "Ceiling" } else { "Floor" }.to_string(),
                area,
                shape,
                item: PaintItem::Slab { lid: slab.lid },
            });
        }
        for solid in &census.solids {
            let node = solid.clone().into_gd();
            let Some(area) = mesh_world_box(&node) else {
                continue; // draws nothing, so it can show no seam
            };
            let name = node.get_name().to_string();
            if let Ok(wall) = node.clone().try_cast::<WaveWall>() {
                let shape = wall.bind().world_shape();
                entries.push(PaintEntry {
                    name,
                    area,
                    shape,
                    item: PaintItem::Wall(wall),
                });
            } else if let Ok(prop) = node.clone().try_cast::<WaveProp>() {
                let shape = prop.bind().world_shape();
                entries.push(PaintEntry {
                    name,
                    area,
                    shape,
                    item: PaintItem::Prop(prop),
                });
            } else if let Ok(column) = node.clone().try_cast::<WaveColumn>() {
                let shape = column.bind().world_shape();
                entries.push(PaintEntry {
                    name,
                    area,
                    shape,
                    item: PaintItem::Column(column),
                });
            } else if let Ok(wedge) = node.clone().try_cast::<WaveWedge>() {
                let shape = wedge.bind().world_shape();
                entries.push(PaintEntry {
                    name,
                    area,
                    shape,
                    item: PaintItem::Wedge(wedge),
                });
            }
            // else: unreachable today — every `WaveSolid` impl the census
            // can collect is one of the four arms above; skipped rather
            // than guessed at if that ever stops being true.
        }
        entries
    }

    /// Hand one entry its chosen labels — the concrete-type dispatch
    /// [`PaintEntry`]'s own `item` exists for, since a floor/ceiling slab
    /// is owned directly by the level (no `Skin` indirection) while every
    /// authored solid paints itself through its own `paint()` method.
    fn paint_entry(&mut self, item: &PaintItem, labels_by_ordinal: &[f32]) {
        match item {
            PaintItem::Slab { lid } => {
                let Some(slab) = self.slabs.iter_mut().find(|s| s.lid == *lid) else {
                    return;
                };
                render::paint::relabel(
                    &mut slab.mesh,
                    render::paint::ShapeKind::Slab,
                    labels_by_ordinal,
                );
            }
            PaintItem::Wall(w) => w.clone().bind_mut().paint(labels_by_ordinal),
            PaintItem::Prop(p) => p.clone().bind_mut().paint(labels_by_ordinal),
            PaintItem::Column(c) => c.clone().bind_mut().paint(labels_by_ordinal),
            PaintItem::Wedge(w) => w.clone().bind_mut().paint(labels_by_ordinal),
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
    fn push_wall_table(&mut self, editor: bool) {
        let mut rects: Vec<Vector4> = self.segments.iter().map(|s| sight::wall_rect(*s)).collect();
        let budget = level_plan::wall_budget(rects.len(), sight::MAXW);
        self.say(editor, budget);
        rects.truncate(sight::MAXW); // a no-op below the ceiling
        // kept for the per-object source muffle: the walls a camera→source
        // sight line is counted against, once per frame on the CPU
        self.occluders = rects.clone();
        let table = PackedVector4Array::from(&rects[..]);
        let count = rects.len() as i64;
        self.push_table_to(self.data_mat.clone(), &table, count);
        self.push_table_to(self.source_mat.clone(), &table, count);
    }

    /// Loud when the authored map has outgrown the range the sight shaders
    /// pack camera distance into ([`level_plan::pack_range_budget`]) — a
    /// ceiling on the map's SIZE rather than on its wall count, and one a
    /// designer meets by widening a room rather than by adding geometry.
    ///
    /// Measured off the floor and ceiling slab boxes — read where they
    /// actually stand in world space, never the raw `extents` knob a level
    /// dropped off-origin would desync from — with the wall table unioned
    /// in belt-and-braces ([`level_plan::slab_diagonal`]). The slabs are
    /// what moves this number on every real map: they span the whole
    /// footprint whether or not a wall stands on any of it, which is
    /// exactly the courtyard a sparse room's short wall centerlines used to
    /// hide (issue #45). The words and the verdict are pure, and cargo
    /// holds them; this end only measures.
    fn report_pack_range(&mut self, editor: bool) {
        let diagonal = match (self.floor_box(), self.ceiling_box()) {
            (Some(floor), Some(ceiling)) => {
                level_plan::slab_diagonal(floor, ceiling, &self.segments)
            }
            // slabs not yet built: nothing drawn to measure, nothing to say
            _ => 0.0,
        };
        let budget = level_plan::pack_range_budget(diagonal, level_plan::DIST_PACK_RANGE);
        self.say(editor, budget);
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

    /// The condition [`INode3D::process`]'s editor-only poll watches: the
    /// level's own `extents` knob plus a fresh census, folded by
    /// [`level_plan::scene_signature`] into a single `u64` — every solid,
    /// source, cat and spawn marker's path and global pose, plus a
    /// solid's skin AABB, so the fold changes the moment ANYTHING
    /// [`Self::derive`] reads would read differently. `extents` has to be
    /// folded here explicitly rather than discovered through the census:
    /// it is not a censused node's property, it is read straight off
    /// `self` — `report_placement` measures the floor slab's world box,
    /// which `set_extents` resizes the instant the knob is dragged, and
    /// `assign_oids` anchors the slab ids against that same box — so a
    /// resize with every node held still is a real change `derive` would
    /// answer differently, and the fold has to see it as one.
    ///
    /// TWO CENSUS WALKS, deliberately, not one shared with `derive`. This
    /// runs every editor frame; `derive` runs only when the signature just
    /// computed differs from the last one — the ordinary case is a still
    /// scene, one walk, no second one to share. Threading a `&Census`
    /// through `derive` (which also mutates `self.segments`,
    /// `self.source_children` and the rest from it) would couple two
    /// pieces of code that change for different reasons — "what does the
    /// level derive" and "what does the level watch" — to save a walk that
    /// is already microseconds at ~130 nodes. If this ever profiles hot,
    /// that coupling is the next place to look, not before.
    fn scene_signature(&self) -> u64 {
        let census = self.census();
        let root = self.base().clone().upcast::<Node>();
        let mut nodes: Vec<level_plan::SignatureNode> = Vec::new();
        for solid in &census.solids {
            let node = solid.clone().into_gd();
            nodes.push(level_plan::SignatureNode {
                path: root.get_path_to(&node).to_string(),
                transform: transform_floats(&node),
                aabb: skin_local_aabb(&node),
            });
        }
        for source in &census.sources {
            let node = source.clone().into_gd();
            nodes.push(level_plan::SignatureNode {
                path: root.get_path_to(&node).to_string(),
                transform: transform_floats(&node),
                aabb: None,
            });
        }
        for cat in &census.cats {
            let node = cat.clone().upcast::<Node>();
            nodes.push(level_plan::SignatureNode {
                path: root.get_path_to(&node).to_string(),
                transform: transform_floats(&node),
                aabb: None,
            });
        }
        for spawn in &census.spawns {
            let node = spawn.clone().upcast::<Node>();
            nodes.push(level_plan::SignatureNode {
                path: root.get_path_to(&node).to_string(),
                transform: transform_floats(&node),
                aabb: None,
            });
        }
        level_plan::scene_signature([self.extents.x, self.extents.y], &nodes)
    }

    /// Say one shader-budget verdict out loud at the volume it asked for,
    /// and nothing at all when there is nothing to say — and, in EITHER
    /// mode, file its text into `level_faults`, which is what makes it
    /// readable from the editor's warning icon at all.
    ///
    /// The two volumes are not interchangeable. An overflow is an ERROR
    /// because the world the shaders draw no longer matches the scene a
    /// designer authored — walls that were placed have stopped occluding.
    /// Running out of headroom is a WARNING because nothing is broken yet,
    /// and a level that shouted about a ceiling it had not hit would teach
    /// the reader to scroll past the one that matters. That mapping is
    /// unchanged and still fires only at run time — an editor's scene-open
    /// is not the moment for either volume.
    fn say(&mut self, editor: bool, budget: Option<level_plan::Budget>) {
        let Some(budget) = budget else {
            return; // comfortably inside every ceiling: silence is the report
        };
        if !editor {
            match budget.severity {
                level_plan::Severity::Error => godot_error!("{}", budget.text),
                level_plan::Severity::Warn => godot_warn!("{}", budget.text),
            }
        }
        self.level_faults.push(budget.text);
    }
}

/// One painted box as the object-id colouring sees it: what it is called,
/// the world box it fills, and the flat id it actually carries.
///
/// Used to also carry a `swept` flag marking a sound source's box as a
/// SWEPT ENVELOPE (limbs' union grown by `sweep_margin`) rather than drawn
/// faces, so the OLD fight census could skip it — an envelope's planes
/// rasterise nothing, and every per-id copy of one union box was
/// coplanar-same-facing with its siblings on all six faces, so a census
/// fed the envelope reported each source z-fighting itself. The new
/// per-face postcondition ([`FaceCensusEntry`], `observe::oids::
/// coplanar_label_faults`) never sees a source at all — a source's limbs
/// bake their role labels directly and contribute no `render::Face` to
/// the census in the first place — so the skip flag had nothing left to
/// guard and is gone with it.
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
    /// handed over — the `WaveCore` itself, upcast to `RefCounted`. The
    /// GDScript `Pulses` shim survives only in `game/tests/`. Resolving it
    /// to a `WaveCore` is the observer's job.
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

    /// Every companion creature in scene order, still TYPED — [`Self::cats`]
    /// hands the same handles to GDScript, which drives their clocks; this
    /// is the in-crate door the capture reads each cat's whole private life
    /// through (brain, gait, tail, pose), none of which is `#[func]`.
    ///
    /// Scene order is not a convenience here, it is the blob's own
    /// precondition: the capture encodes and compares cats POSITIONALLY,
    /// so two captures of one unchanged world that walked the tree in
    /// different orders would diverge at `cats[0]` and blame a bug that is
    /// not there.
    pub(super) fn cat_handles(&self) -> &[Gd<WaveCat>] {
        &self.cat_children
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
    /// The set and the SHAPES are the ones [`Self::paint_labels`] reasons
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
    ///
    /// EVERY `oid` HERE IS STILL A SOLID-GRANULARITY READ: it reads a
    /// mesh's FIRST `CUSTOM0` vertex ([`mesh_first_label`]), one number
    /// standing in for a whole mesh — the same trade `Skin::oid()`
    /// (`super::solid`) makes, for the same reason: EXACT for a solid that
    /// never coplanar-MERGED with anything
    /// (`render::superface::superfaces`'s own SINGLETON COLLAPSE folds
    /// every one of its faces to one class, so the first genuinely speaks
    /// for the whole solid), APPROXIMATE for one that did — a merged
    /// solid's bridged value names only its OWN first face's class, never
    /// the partner's it fused with.
    ///
    /// This walk is NOT scoped to solids that stayed singletons, and
    /// neither is `WaveObserver::explain_oids`'s `pairs`/`violations`,
    /// which reads every touching pair this census reports: a genuinely
    /// merged pair (BorderNorth and DividerNorth, on the shipped map) is
    /// checked here exactly like any other touching pair, and whether it
    /// reads clean or reports a false violation depends on which of its
    /// six faces happens to be "first" for each wall — not on anything
    /// this function knows about the merge (`map_test.gd`'s
    /// `test_shipped_touching_boxes_draw_their_seam` currently passes for
    /// this pair by that same accident, an open fragility, not a proven
    /// law). The real per-face truth — every face, its own label, no
    /// bridging, no ordinal luck — lives in [`Self::face_census`] instead,
    /// and `explain_oids`'s `faults` census reads THAT.
    pub(super) fn oid_census(&self) -> Vec<PaintedSolid> {
        let census = self.census();
        let mut painted = Vec::new();
        for slab in &self.slabs {
            let Some(area) = mesh_world_box(&slab.skin.clone().upcast()) else {
                // a slab with NO MESH — not a hidden one. The census
                // measures through the AABB and the transform, neither of
                // which visibility touches, so the ceiling still reports
                // itself in the editor, where it is built but not drawn.
                continue;
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
                oid: painted_oid(&node).unwrap_or(oid_palette::NO_OID),
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
            // colouring anchored on — see `paint_labels`
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

    /// The real per-face label census the last `derive()` baked — every
    /// face this level's meshes actually carry, with the SOLID it belongs
    /// to, the label baked onto every one of its vertices, and the
    /// superface CLASS that label came from.
    ///
    /// This is what [`Self::paint_labels`] computed and wrote, read back
    /// rather than re-derived a second time: the debug layer's own
    /// postcondition (`WaveObserver::explain_oids`'s `superfaces`/
    /// `faults`) reads the rendering subsystem's own census instead of
    /// risking a second, possibly-drifting copy of the merge law.
    ///
    /// Empty before the first successful `derive()` — the editor, or a
    /// level never added to the tree — exactly as every other derived
    /// table on this struct (`segments`, `occluders`) starts empty.
    pub(super) fn face_census(&self) -> &[FaceCensusEntry] {
        &self.face_census
    }
}

/// The flat object id a creature is painted with, read off the first limb
/// whose mesh carries a `CUSTOM0` channel. A creature paints every limb
/// with one id (the whole animal is one silhouette), so the first limb
/// speaks for it; `None` for a node with no `MeshInstance3D` descendant
/// carrying one at all, which the census then leaves out rather than
/// reporting under [`oid_palette::NO_OID`] as though the shader had been
/// handed a real value.
fn painted_oid(node: &Gd<Node>) -> Option<f64> {
    if let Ok(skin) = node.clone().try_cast::<MeshInstance3D>()
        && let Some(oid) = mesh_first_label(&skin)
    {
        return Some(oid);
    }
    node.get_children()
        .iter_shared()
        .find_map(|child| painted_oid(&child))
}

/// The flat object id a mesh instance carries right now — the one source
/// of truth, read straight back off the skin's own `CUSTOM0`, exactly what
/// the shader itself reads for G. [`oid_palette::NO_OID`] when nothing has
/// painted it.
fn read_oid(skin: &Gd<MeshInstance3D>) -> f64 {
    mesh_first_label(skin).unwrap_or(oid_palette::NO_OID)
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

/// The 12 floats of a node's global transform — basis columns (X, Y, Z)
/// then origin — the pose half of [`WaveLevel::scene_signature`]. Every
/// censused node is Node3D-derived (a solid stands on `StaticBody3D`, a
/// source and the cat on their own `Node3D`-family bases, a spawn on
/// `Marker3D`), so the cast is total in the scenes this ever runs
/// against; a node that somehow were not simply contributes the identity
/// transform, which still moves the signature the instant any sibling's
/// path or AABB does.
fn transform_floats(node: &Gd<Node>) -> [f32; 12] {
    let xf = node
        .clone()
        .try_cast::<Node3D>()
        .map(|n3| n3.get_global_transform())
        .unwrap_or(Transform3D::IDENTITY);
    let (bx, by, bz) = (xf.basis.col_a(), xf.basis.col_b(), xf.basis.col_c());
    [
        bx.x,
        bx.y,
        bx.z,
        by.x,
        by.y,
        by.z,
        bz.x,
        bz.y,
        bz.z,
        xf.origin.x,
        xf.origin.y,
        xf.origin.z,
    ]
}

/// A solid's skin-mesh LOCAL AABB — position then size, six floats — the
/// shape half of [`WaveLevel::scene_signature`]. Read straight off the
/// child every solid class names [`SKIN_NAME`] ([`build_body`]/
/// [`build_box`] in [`super::solid`]), so a designer dragging a `radius`
/// or `size` knob moves the signature even though it moves the node's own
/// transform not at all. `None` before a skin exists — the instant a
/// fresh node enters the tree, ahead of its own `_ready` — which is itself
/// a real difference the signature is right to notice.
fn skin_local_aabb(node: &Gd<Node>) -> Option<[f32; 6]> {
    let skin = node
        .get_children()
        .iter_shared()
        .find(|child| child.get_name() == SKIN_NAME)
        .and_then(|child| child.try_cast::<MeshInstance3D>().ok())?;
    let aabb = skin.get_aabb();
    Some([
        aabb.position.x,
        aabb.position.y,
        aabb.position.z,
        aabb.size.x,
        aabb.size.y,
        aabb.size.z,
    ])
}

/// Whether a CHILD is its own censused entity — a solid, a sound source, or
/// the cat — the same vocabulary [`collect`] recognises, its two
/// `try_dynify` arms and its `WaveCat` arm mirrored exactly (a `Marker3D`
/// is deliberately not on this list: [`collect`] treats it as a spawn
/// point, never as something with its own drawn box).
///
/// This is the STOP the recursion below reads: a node found here has its
/// own [`mesh_world_box`] elsewhere in the same walk, so folding its mesh
/// into the node asking the question would count it twice — once under its
/// own name, once again inflating whatever it happens to be grouped under
/// in the scene tree.
fn is_censused_child(node: &Gd<Node>) -> bool {
    node.clone().try_dynify::<dyn WaveSolid>().is_ok()
        || node.clone().try_dynify::<dyn SoundSource>().is_ok()
        || node.clone().try_cast::<WaveCat>().is_ok()
}

/// The world box a node's drawn geometry occupies — the union over every
/// `MeshInstance3D` beneath it, the node itself included, EXCEPT through a
/// CHILD [`is_censused_child`] recognises: a prop nested under a crate, a
/// source's own limbs rig, a cat sitting on a shelf — each is its own
/// censused entity with its own box elsewhere in the level's walk, so
/// recursion stops there instead of folding its geometry into the parent's
/// (issue #35 — that folding used to inflate a solid's colouring box and
/// its placement footprint past anything it actually draws, purely because
/// a designer grouped a second prop under it in the Scene dock).
///
/// The check is never applied to `node` itself, only to its children:
/// [`WaveLevel::paint_labels`] and [`WaveLevel::oid_census`] call this
/// function DIRECTLY on a source's or a cat's own node, and a root that
/// refused itself would report no box at all, dropping it from the
/// labelling and the census entirely rather than measuring it. A plain
/// grouping `Node3D`, a limb mesh, or a `Marker3D` is not on that list and
/// still recurses — a solid's box is still its own skin child's box, and a
/// source's box is still the union of its own limbs.
///
/// `None` for a node that draws nothing, which can never show a seam with
/// anything.
fn mesh_world_box(node: &Gd<Node>) -> Option<oid_palette::Box3> {
    let mut found: Option<oid_palette::Box3> = None;
    if let Ok(mesh) = node.clone().try_cast::<MeshInstance3D>() {
        found = Some(world_box(mesh.get_aabb(), mesh.get_global_transform()));
    }
    for child in node.get_children().iter_shared() {
        if is_censused_child(&child) {
            continue;
        }
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
