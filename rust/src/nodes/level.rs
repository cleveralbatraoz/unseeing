//! The level root — the one node a scene of walls, props, sound sources, a
//! cat and a typed spawn datum hangs under, and the engine's single door into
//! it. The composition root injects the two data-writing materials (the
//! world skin and the source image) and the wave pool HERE, once; the level
//! deals them out and hands the occluding skins the wall table their
//! analytic sight test runs against. When it enters the tree it builds the
//! floor and ceiling slabs from its `extents` knob and DERIVES the technical
//! contracts the systems run on — wall centerlines, the spawn, the dev demo
//! tap — via the pure [`level_plan`] math, so a designer who moves a wall
//! has moved the contracts with it.
//!
//! The level's recursive census, material injection and warning-forwarding
//! paths name no concrete solid or source classes. They sort drawn gameplay
//! children into two abstractions: [`WaveSolid`] —
//! anything the waves can strike, box or column or wedge or wall — and
//! [`SoundSource`] — anything that makes the world's own sound, fan or
//! radio. Typed cats, runs and spawn data join that same recursive census
//! through their registered classes. The two heterogeneous families are
//! Rust traits published to the engine with `#[godot_dyn]`, so
//! [`godot::obj::Gd::try_dynify`] recognises a child by what it CAN DO
//! rather than by what class it is. New source kinds therefore compose
//! through their trait alone. A genuinely new solid geometry also declares
//! its exhaustive planar-face/mesh representation in this module's explicit
//! paint boundary; dynamic census removes traversal coupling, not that shape
//! law.
//!
//! Occlusion decision: a source's waves are stopped by the WALLS themselves
//! — source→surface sight in the data core — not clipped to a derived room
//! rectangle. So a designer may open a source's room to a corridor without
//! retyping anything or tripping an enclosure law: the waves simply light
//! what they can reach and stop at what they cannot.
//!
//! Spawn decision: the first `WaveSpawn` in depth-first scene order wins,
//! regardless of its scene name. Every duplicate is named and warned on its
//! own node; missing still falls back to the level origin and complains.
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
    ArrayMesh, Engine, INode3D, Material, MeshInstance3D, Node3D, ShaderMaterial, StaticBody3D,
};
use godot::obj::DynGd;
use godot::prelude::*;

use super::cat::WaveCat;
use super::props::{WaveColumn, WaveProp, WaveWedge};
use super::run::WaveRun;
use super::solid::{
    SKIN_NAME, WaveSolid, basis_columns_f64, build_box, clear_limbs, mesh_first_label, to_f64_3,
};
use super::source::SoundSource;
use super::spawn::WaveSpawn;
use super::wall::WaveWall;
use crate::level_plan;
use crate::oid_palette;
use crate::render;
use crate::sight;

/// The palette every wall and prop is coloured from, read from the one
/// place the whole label universe is visible at once
/// (`render::labels::WORLD_PALETTE`). It used to be a literal here, which
/// is why nothing could check it against the creature and viewmodel labels
/// standing either side of it in the same band.
const WORLD_OIDS: [f64; 5] = render::labels::WORLD_PALETTE;

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
    runs: Vec<Gd<WaveRun>>,
    /// Every typed datum in deterministic depth-first walk order.
    spawns: Vec<Gd<WaveSpawn>>,
}

/// Which concrete thing a [`PaintEntry`] is — the level's own two slabs
/// have no `Skin`-carrying node of their own (no indirection needed to
/// reach their mesh), while every solid node (authored or derived) paints itself back
/// through its own `paint()` method; see [`WaveLevel::paint_entry`].
enum PaintItem {
    Slab { lid: bool },
    Wall(Gd<WaveWall>),
    Prop(Gd<WaveProp>),
    Column(Gd<WaveColumn>),
    Wedge(Gd<WaveWedge>),
}

impl PaintItem {
    /// The solid scene node this paint entry came from. The two slabs belong
    /// to the level itself, so only solid entries have a node address. A
    /// generated RunSeg wall keeps that derived address here; `faults_for`
    /// forwards it to the authored WaveRun that owns the repairable data.
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
/// solid node (authored or derived), carrying its world-space `render::Shape` (what
/// `render::faces` builds faces and conservative touch bounds from).
///
/// A slab's `shape` is a real `Box3d` like any other box (Wave S) — it
/// used to be `None`, back when a slab was fed through `render::faces` not
/// at all; `superface::superfaces`'s own singleton collapse made that
/// workaround unnecessary, so every entry now carries one.
struct PaintEntry {
    name: String,
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
    occluders: Vec<sight::Occluder>,
    /// Skins that occlude by the wall table but are owned by the
    /// composition root rather than by the level — the hearing pass, and
    /// the hero's own cane and body.
    ///
    /// They are REGISTERED rather than pushed to once, because the table is
    /// rebuilt by every `derive()` and a push that happens once, after the
    /// only derive a runtime level performs, is correct solely because of
    /// that ordering. `rederive` is a `#[func]`: anything may call it, and
    /// then two of the five occluding skins would be carrying last
    /// derivation's walls. One owner, every skin, every derive.
    extra_skins: Vec<Gd<Material>>,
    spawn_at: Vector3,
    spawn_heading: f64,
    tap_point: Vector3,
    #[init(val = Vector3::UP)]
    tap_normal: Vector3,
    source_children: Vec<DynGd<Node, dyn SoundSource>>,
    cat_children: Vec<Gd<WaveCat>>,
    /// Every solid geometry REFUSED — the crates, wedges and standpipes
    /// that do not stop waves — kept as world AABBs for the per-source
    /// clarity walk alone.
    ///
    /// CPU-ONLY, and that is the whole reason this is affordable. It is
    /// never pushed to any shader: the muffle is one scalar per source per
    /// frame, so the cost is sources x props (about 212 segment tests on
    /// the shipped level) instead of the 259 M per-fragment near-tests that
    /// killed the idea of putting props in the wall table. The measured
    /// objection to that table was never only cost, either — a column's
    /// bounding square over-approximates by 41% radially and would bite a
    /// visible notch out of every ring. Here the same over-approximation
    /// moves a per-object scalar by a hair, which no player can see.
    prop_occluders: Vec<sight::Occluder>,

    /// Non-wall solids admitted to the occluder table by geometry, in the
    /// order they were appended after the walls.
    ///
    /// Kept and reported because a geometric admission rule without a
    /// diagnostic is how a designer loses a wall in silence: a pillar now
    /// consumes a `MAXW` slot, and the budget message must be able to say
    /// so rather than telling them to delete walls they can already count.
    spanning_solids: Vec<String>,

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
    /// starved face class, or an authored-node paint fault — so a
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
    /// Read-only observability for the editor watch. A pass may stage wall
    /// geometry, but it must reseed the resulting signature rather than run a
    /// redundant second derive on the next idle frame.
    derive_count: u64,
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
            // one O(scene) census walk every idle frame for a poll only the
            // editor needs — so the runtime branch turns it back off
            // explicitly.
            self.base_mut().set_process(false);
        }
    }

    /// Editor-only: watch the scene for the condition [`Self::derive`]
    /// actually depends on, and re-derive the moment it changes — a
    /// designer drags a wall or a knob and sees the wall table, the
    /// warnings and the per-face superface labels update without ever pressing
    /// play or calling [`Self::rederive`] by hand.
    ///
    /// DESIGN: condition-watching, not dirty-flag plumbing. Six classes'
    /// setters and transform notifications would each have to remember to
    /// mark the level dirty, and any one that forgot would be a silent
    /// stale-until-play bug with no test able to see it was missing. A
    /// signature folded fresh every frame cannot forget: it is the same
    /// census `derive` itself walks, so
    /// whatever `derive` reads, this watches, automatically, forever.
    /// Nothing here runs at run time — see the branch in [`INode3D::ready`]
    /// that turns processing back off outside the editor.
    fn process(&mut self, _delta: f64) {
        if !Engine::singleton().is_editor_hint() {
            return;
        }
        let sig = self.scene_signature();
        if sig != self.last_signature {
            self.derive();
            self.last_signature = self.scene_signature();
        }
    }

    /// The level root's Scene-dock warning icon, editor-only — the
    /// level-wide faults a running level shouts through
    /// `godot_error!`/`godot_warn!`, read back off `level_faults` instead
    /// of the log. Node-specific placement, starvation and paint faults
    /// deliberately live only on the authored node that caused them.
    /// [`Self::derive`] calls
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
        // an EMPTY wall table to skins that did not exist, and it labelled
        // every wall, prop and source role without injected source geometry
        // to measure — so a
        // source injected now would render with seams that silently do not
        // draw, in a world whose walls no longer occlude. Nothing later can
        // repair either. Say so rather than limp.
        if self.base().is_inside_tree() {
            godot_error!(
                "WaveLevel: inject() after the level entered the tree — the wall table and the \
                 per-face labels were already derived without it. Inject BEFORE add_child()."
            );
        }
        self.data_mat = Some(data_mat.clone());
        self.source_mat = Some(source_mat.clone());
        self.pulses = Some(pulses.clone());
        let census = self.census();
        for mut solid in census.solids {
            solid.dyn_bind_mut().set_material(&data_mat);
        }
        for mut run in census.runs {
            run.bind_mut().set_material(&data_mat);
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

    /// Where the hero wakes: the selected WaveSpawn lifted to capsule
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
    /// something: a suite holding the shipped map silent and the live
    /// editor-warning boundary.
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

    /// Read-only observability for the names parallel to
    /// [`Self::wall_segments`]. The editor probe reads this exact retained
    /// table after a WaveRun rebuild; deriving names from the current tree
    /// instead would conceal freed handles in the table the renderer and
    /// [`super::observer::WaveObserver`] actually use.
    #[func(rename = wall_names)]
    fn observed_wall_names(&self) -> PackedStringArray {
        self.wall_names().iter().map(GString::from).collect()
    }

    /// Number of complete derive passes this level has run. This is a narrow
    /// editor-performance witness, never authored or captured state.
    #[func]
    fn derive_count(&self) -> i64 {
        i64::try_from(self.derive_count).unwrap_or(i64::MAX)
    }

    /// The inflated wall OCCLUDER rects (`sight::wall_rect`), truncated to
    /// the shader's slots — the very table the level pushes to the
    /// data-writing skins, exposed so the composition root can hand it to
    /// the hearing pass too, which cuts player-sound shells by these walls.
    #[func]
    pub(super) fn wall_rects(&self) -> PackedVector4Array {
        self.occluders.iter().map(|occ| occ.rect()).collect()
    }

    /// Each occluder's world Y sweep, `(bottom, top)` — the `u_wall_y`
    /// lane, in the same slot order as [`Self::wall_rects`].
    ///
    /// Two arrays only because the uniform layout forces two; they are
    /// projections of ONE `Vec<Occluder>`, so they cannot desync in length,
    /// in order, or under truncation. A wall's height used to be a single
    /// global `u_wall_top`, which is why a wall lifted with the gizmo
    /// occluded a strip of empty air beneath itself and nothing at all
    /// across its raised top.
    #[func]
    pub(super) fn wall_spans(&self) -> PackedVector2Array {
        self.occluders.iter().map(|occ| occ.span()).collect()
    }

    /// The occluder table itself, for the debug observer — which must
    /// describe the walls the shaders were actually handed, spans included,
    /// rather than rebuild them from a projection.
    pub(super) fn occluders(&self) -> &[sight::Occluder] {
        &self.occluders
    }

    /// Every fault the last derivation pinned to `node` specifically — an
    /// unfloored/sunken placement, a starved face-class seam, or a paint
    /// fault — matched by the same `root.get_path_to` address every entry
    /// in `node_faults` carries. An authored WaveRun also surfaces faults
    /// addressed to its generated RunSeg children, because endpoints and
    /// openings are the only saved data a designer can repair; the
    /// ephemeral child itself stays silent so one fault does not create two
    /// competing triangles. Not `#[func]`: this is
    /// [`super::solid::warnings_from_level`]'s door into the level, called
    /// from every warning-bearing node's `get_configuration_warnings`, not
    /// a designer-facing knob.
    pub(super) fn faults_for(&self, node: &Gd<Node>) -> PackedStringArray {
        if super::run::is_generated_segment(node) {
            return PackedStringArray::new();
        }
        let root = self.base().clone().upcast::<Node>();
        let mut paths = vec![root.get_path_to(node).to_string()];
        for segment in super::run::generated_segments(node) {
            paths.push(root.get_path_to(&segment).to_string());
        }
        self.node_faults
            .iter()
            .filter(|fault| paths.contains(&fault.path))
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
    /// ceiling [`level_plan::occluder_budget`] measures a level's headroom
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

    /// The film grain's peak-to-peak swing ([`render::grain::GRAIN_AMP`]),
    /// served so `game/tests/shader_contract_test.gd` can hold it against
    /// `hearing_post.gdshader`'s own `u_grain_amp` default.
    ///
    /// The grain is a mood knob and would normally have no business in
    /// Rust — except that [`render::reveal::PRESENCE`] is DERIVED from it,
    /// and settled law 1 rests on that derivation. If the two drifted, a
    /// source could sink back under the noise with every cargo test still
    /// green, which is precisely the failure this pair exists to make
    /// impossible.
    #[func]
    fn grain_amp() -> f64 {
        render::grain::GRAIN_AMP
    }

    /// The floor under a sound source's packed reveal
    /// ([`render::reveal::PRESENCE`]), served so the contract suite can
    /// check that what the composition root pushes into `u_presence` is
    /// what Rust derived.
    #[func]
    fn source_presence() -> f64 {
        render::reveal::PRESENCE
    }

    /// The detail knee ([`render::detail::DetailKnee::shipped`]) as
    /// `(lo, hi)`, served so `game/tests/shader_contract_test.gd` can hold
    /// what the composition root pushes against what Rust derived — the
    /// same drift the crease knee's own mirror exists to catch.
    /// What one wall leaves of a source's silhouette
    /// ([`level_plan::SOURCE_THROUGH`]), served so the contract suite can
    /// check the detail knee opens at exactly that ceiling — the
    /// precondition the "a walled source cannot name itself" theorem rests
    /// on, and the one number that would silently break it.
    #[func]
    fn source_through() -> f64 {
        level_plan::SOURCE_THROUGH
    }

    #[func]
    fn detail_knee() -> Vector2 {
        let knee = render::detail::DetailKnee::shipped();
        Vector2::new(knee.lo() as f32, knee.hi() as f32)
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
    /// [`Self::debug_labelled_box`]. No shipped direct-door caller rebuilds
    /// the same mesh with a different constant role label, so the
    /// write-through behaviour separating [`render::paint::resize_triangle_surface`]
    /// from the carry door would otherwise be unreachable from a test.
    ///
    /// Rebuilds `mesh` in place as ONE triangle whose three vertices all
    /// carry `label`, exactly the way a per-frame builder rebuilds its own
    /// limb buffer, and hands the same resource back.
    #[func]
    fn debug_triangle_surface(mesh: Gd<ArrayMesh>, label: f32) -> Gd<ArrayMesh> {
        let mut mesh = mesh;
        let triangles = [
            (Vector3::ZERO, Vector3::UP, label),
            (Vector3::FORWARD, Vector3::UP, label),
            (Vector3::RIGHT, Vector3::UP, label),
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
            // Delivered as two numbers, never as their product: see
            // render::reveal::source_image for why the muffle must
            // multiply the whole acoustic image rather than floor half of
            // it.
            let image = render::reveal::SourceImage {
                volume,
                muffle: self.source_muffle(eye, hub),
            };
            source.dyn_bind_mut().set_image(image);
        }
    }

    /// How muffled a source's SILHOUETTE at `to` reads from the eye at
    /// `from`: [`level_plan::SOURCE_THROUGH`] per wall the sight line
    /// crosses and [`level_plan::prop_through`] per prop, so two props cost
    /// exactly one wall. General to any source, not the fan alone; exposed
    /// so the suites can hold the law directly.
    ///
    /// The two walks read two different tables and that is deliberate. The
    /// wall table is the one the SHADERS also read, so a wave and a
    /// silhouette agree about what a barrier is. The prop table exists only
    /// here: those solids do not stop waves at all, and never enter a
    /// uniform.
    #[func]
    fn source_muffle(&self, from: Vector3, to: Vector3) -> f64 {
        let walls = sight::crossings(from, to, &self.occluders);
        let props = sight::crossings(from, to, &self.prop_occluders);
        level_plan::source_muffle(walls, props)
    }

    /// Where the hero wakes, and every word a designer needs about the
    /// typed data that did not win. The DECISION is pure and lives in
    /// [`level_plan::choose_spawn`]; this end only measures — the winner's
    /// world position lifted to capsule height, the level's own origin as
    /// the fallback, and each candidate's path under the level root.
    ///
    /// `editor` is [`Self::derive`]'s one read of `Engine::is_editor_hint`:
    /// every complaint is ALWAYS filed into `level_faults` (the totals
    /// rewrite an editor's warning icon reads), and printed through
    /// `godot_error!` only at run time, where the boot gate pins the text.
    fn derive_spawn(&mut self, spawns: &[Gd<WaveSpawn>], editor: bool) {
        let lift = Vector3::new(0.0, level_plan::SPAWN_LIFT as f32, 0.0);
        let fallback = self.base().get_global_position() + lift;
        let root = self.base().clone().upcast::<Node>();
        let candidates: Vec<level_plan::SpawnCandidate> = spawns
            .iter()
            .map(|marker| level_plan::SpawnCandidate {
                path: root.get_path_to(marker).to_string(),
            })
            .collect();
        let verdict = level_plan::choose_spawn(&candidates, fallback);
        for complaint in &verdict.complaints {
            if !editor {
                godot_error!("{}", complaint);
            }
            self.level_faults.push(complaint.clone());
        }
        if let Some(complaint) = verdict.complaints.first() {
            for &slot in &verdict.losers {
                if let Some(candidate) = candidates.get(slot) {
                    self.node_faults.push(level_plan::PlacementFault {
                        path: candidate.path.clone(),
                        text: complaint.clone(),
                    });
                }
            }
        }
        match verdict.winner.and_then(|slot| spawns.get(slot)) {
            Some(marker) => {
                self.spawn_at = marker.get_global_position() + lift;
                self.spawn_heading = level_plan::basis_heading(marker.get_global_basis());
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

    /// Store a paint-time diagnostic where a designer can act on it. Every
    /// authored paint entry owns a Scene-dock icon; the level's generated
    /// floor/ceiling slabs do not, so their only total destination is the
    /// level root. The text is already complete and is shared byte-for-byte
    /// with the runtime log at the call site.
    fn file_paint_fault(&mut self, entry: &PaintEntry, text: String) {
        if let Some(node) = entry.item.node() {
            let root = self.base().clone().upcast::<Node>();
            self.node_faults.push(level_plan::PlacementFault {
                path: root.get_path_to(&node).to_string(),
                text,
            });
        } else {
            self.level_faults.push(text);
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
    /// The level's own icon is not the only one that can go stale: this pass
    /// may add or clear a fault on a solid, source, spawn or run, so every
    /// warning-bearing census family is refreshed after the level. Source
    /// roles consume graph classes without becoming world superfaces, and
    /// may own starvation like a solid does. Omit one family and
    /// the synchronous fault store can be right while the Scene dock keeps
    /// a stale triangle until an unrelated repaint jars it loose.
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
        self.derive_count = self.derive_count.saturating_add(1);
        let editor = Engine::singleton().is_editor_hint();
        self.level_faults.clear();
        self.node_faults.clear();
        let mut census = self.census();
        // Canonicalize walls explicitly before reading any wall-owned
        // geometry. A rederive may be requested in the same callback that
        // changed a nested prefab transform, before WaveWall's editor process
        // has run; staging here removes that ambient process-order dependency.
        for wall in &mut census.walls {
            wall.bind_mut().prepare_for_derive();
        }
        // One loop, one bind per wall, both tables: a wall's centerline and
        // the world Y span of the very box the paint pass draws. Built
        // together on purpose — a table of rects beside a table of heights
        // is two things that can disagree, and the disagreement is exactly
        // the bug this replaced.
        self.segments.clear();
        let mut occluders: Vec<sight::Occluder> = Vec::with_capacity(census.walls.len());
        let root = self.base().clone().upcast::<Node>();
        let mut unoccludable: Vec<String> = Vec::new();
        for wall in &census.walls {
            let (segment, shape) = {
                let bound = wall.bind();
                (bound.segment(), bound.world_shape())
            };
            let occluder = render::faces::bounds(&shape)
                .and_then(|box3| sight::Occluder::new(segment, box3.min[1], box3.max[1]));
            if occluder.is_none() {
                unoccludable.push(root.get_path_to(wall).to_string());
            }
            self.segments.push(segment);
            // A refused wall keeps its SLOT rather than vanishing:
            // `wall_names()[i]` names `occluders[i]`, and a hole slides
            // every later name onto the wrong wall.
            occluders.push(occluder.unwrap_or(sight::Occluder::NOWHERE));
        }
        for path in unoccludable {
            let fault = level_plan::unoccludable_wall(&path);
            if !editor {
                godot_error!("{}", fault.text);
            }
            self.node_faults.push(fault);
        }
        // Solids that ACTUALLY STAND IN THE WAY join the wall table.
        //
        // Occlusion is decided by geometry, never by node class. A pillar
        // running floor to ceiling and half a metre thick stops sound the
        // way a wall does, because it is a wall that happens to be round;
        // a crate at knee height does not, because sound goes over it.
        // `data_core.gdshaderinc` asserted for months that props are
        // transparent "deliberately"; the occluder table had simply only
        // ever been built from the wall census, and the sole recorded
        // argument was the cost of admitting all 106 props at once. That
        // cost argument stands and is why the rule is narrow.
        //
        // AFTER the walls, never interleaved: `wall_names()[i]` names
        // `occluders[i]`, and every authored wall must keep the slot index
        // it has always had so a designer's fault message still points at
        // the right node.
        let mut admitted: Vec<String> = Vec::new();
        let mut refused: Vec<sight::Occluder> = Vec::new();
        for solid in &census.solids {
            let node = solid.clone().into_gd();
            let Some(shape) = Self::unwalled_world_shape(&node) else {
                continue;
            };
            let Some(box3) = render::faces::bounds(&shape) else {
                continue; // unmeasurable: refused, which is what it did before
            };
            let width = box3.max[0] - box3.min[0];
            let depth = box3.max[2] - box3.min[2];
            if !level_plan::spans_the_corridor(box3.min[1], box3.max[1], width.min(depth)) {
                // Refused as a WAVE occluder — sound goes over a crate — but
                // it still stands between an eye and a sounding thing, and
                // takes something from how clearly that thing reads.
                if let Some(prop) = sight::Occluder::from_bounds(
                    box3.min[0],
                    box3.min[2],
                    box3.max[0],
                    box3.max[2],
                    box3.min[1],
                    box3.max[1],
                ) {
                    refused.push(prop);
                }
                continue;
            }
            let Some(occluder) = sight::Occluder::from_bounds(
                box3.min[0],
                box3.min[2],
                box3.max[0],
                box3.max[2],
                box3.min[1],
                box3.max[1],
            ) else {
                continue;
            };
            admitted.push(root.get_path_to(&node).to_string());
            occluders.push(occluder);
        }
        self.spanning_solids = admitted;
        self.prop_occluders = refused;
        self.push_wall_table(occluders, editor);
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
        // Refresh warning-bearing sources alongside the solids. Their role
        // classes can gain or clear starvation while their geometry remains
        // outside the world-superface census; `self.source_children` already
        // owns them after the move above.
        for source in &self.source_children {
            source
                .clone()
                .into_gd()
                .call_deferred("update_configuration_warnings", &[]);
        }
        // Typed spawn data can gain or lose a duplicate warning when the
        // scene walk changes. Refresh every candidate, including the winner,
        // so a loser that becomes unique clears its cached triangle.
        for spawn in &census.spawns {
            spawn
                .clone()
                .upcast::<Node>()
                .call_deferred("update_configuration_warnings", &[]);
        }
        // A run surfaces its own endpoint/transform complaints plus any
        // level fault addressed to an ephemeral RunSeg child. Refresh the
        // authored node too, because the generated child intentionally
        // keeps its own icon silent and cannot be the designer's repair
        // target.
        for run in &census.runs {
            run.clone()
                .upcast::<Node>()
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
    /// [`mesh_world_box`]'s, the same measure the superface paint pass and
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
    /// coloured ([`render::paint_plan::plan`]) so no two classes a seam must draw
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
    /// Starvation is both a level-wide warning and one node warning for every
    /// authored solid or source that owns a starved class. Solid ownership is
    /// deliberately mapped through `classes_of_entry`, not a retired
    /// per-solid slot; source role classes map through `classes_of_source`.
    fn paint_labels(&mut self, census: &Census, editor: bool) {
        let entries = self.paint_entries(census);
        let request = render::paint_plan::PaintRequest {
            entries: entries
                .iter()
                .map(|entry| render::paint_plan::PaintEntryInput {
                    shape: entry.shape.clone(),
                    // The anchor names ONE face: the side of the slab the
                    // room actually meets — the floor's top, the ceiling's
                    // underside. Never the whole slab. A slab owns one
                    // class only while nothing merges with it, and the
                    // instant a prop set flush into the floor splits its
                    // faces, an entry-wide anchor put the same label on
                    // classes the merge law had just separated and the
                    // whole level went unpainted.
                    anchor: match entry.item {
                        PaintItem::Slab { lid } => Some(render::paint_plan::FaceAnchor {
                            label: render::role_label(if lid {
                                render::Role::Ceiling
                            } else {
                                render::Role::Floor
                            }),
                            // off the slab's OWN basis, not a world-up
                            // literal: a level tipped by its node transform
                            // would otherwise anchor the role onto whichever
                            // flank happened to face world up
                            facing: {
                                let up = render::paint_plan::shape_up(&entry.shape);
                                if lid { [-up[0], -up[1], -up[2]] } else { up }
                            },
                        }),
                        _ => None,
                    },
                    is_wall: matches!(entry.item, PaintItem::Wall(_)),
                })
                .collect(),
            sources: census
                .sources
                .iter()
                .map(|source| {
                    let area = mesh_world_box(&source.clone().into_gd());
                    let bound = source.dyn_bind();
                    render::paint_plan::PaintSourceInput {
                        area,
                        sweep_margin: bound.sweep_margin(),
                        roles: bound.role_count(),
                    }
                })
                .collect(),
            palette: WORLD_OIDS.to_vec(),
        };
        let plan = match render::paint_plan::plan(request) {
            Ok(plan) => plan,
            Err(error) => {
                let message = format!(
                    "WaveLevel: paint planning rejected invalid input ({error:?}); keeping every existing label."
                );
                if !editor {
                    godot_error!("{}", message);
                }
                self.level_faults.push(message);
                return;
            }
        };

        for fault in &plan.entry_faults {
            let Some(entry) = entries.get(fault.entry) else {
                continue;
            };
            let message = match fault.fault {
                render::paint_plan::EntryFault::WrongFaceCount { actual, expected } => format!(
                    "WaveLevel: '{}' built {} planar face(s) from its shape, not the {} it should — a degenerate size folded one or more away. Its own seams cannot be painted correctly this derive; skipping it rather than mislabeling by position. Give every extent a real size.",
                    entry.name, actual, expected
                ),
                render::paint_plan::EntryFault::InvalidArea => format!(
                    "WaveLevel: '{}' has a non-finite or reversed paint bound; keeping its existing labels.",
                    entry.name
                ),
            };
            if !editor {
                godot_error!("{}", message);
            }
            self.file_paint_fault(entry, message);
        }
        let root = self.base().clone().upcast::<Node>();
        for fault in &plan.source_faults {
            let Some(source) = census.sources.get(fault.source) else {
                continue;
            };
            let node = source.clone().into_gd();
            let path = root.get_path_to(&node).to_string();
            let message = source_paint_fault_text(&path, fault.fault);
            if !editor {
                godot_error!("{}", message);
            }
            self.node_faults.push(level_plan::PlacementFault {
                path,
                text: message,
            });
        }
        if !plan.starved_classes.is_empty() {
            let message = format!(
                "WaveLevel: {} face/source-role class(es) could not take a label distinct from everything they touch — those seams will not draw. Spread the geometry or widen WORLD_OIDS.",
                plan.starved_classes.len()
            );
            if !editor {
                godot_error!("{}", message);
            }
            self.level_faults.push(message);
        }
        for &entry_index in &plan.starved_entries {
            let Some(entry) = entries.get(entry_index) else {
                continue;
            };
            let Some(node) = entry.item.node() else {
                continue;
            };
            self.node_faults.push(level_plan::PlacementFault {
                path: root.get_path_to(&node).to_string(),
                text: "one or more face classes cannot take a label distinct from everything they touch — those seams will not draw.".to_string(),
            });
        }
        for &source_index in &plan.starved_sources {
            let Some(source) = census.sources.get(source_index) else {
                continue;
            };
            let node = source.clone().into_gd();
            self.node_faults.push(level_plan::PlacementFault {
                path: root.get_path_to(&node).to_string(),
                text: "one or more source roles cannot take a label distinct from everything they touch — those seams will not draw.".to_string(),
            });
        }

        for (entry, command) in entries.iter().zip(&plan.entry_commands) {
            if let render::paint_plan::PaintCommand::Relabel(labels) = command {
                let labels: Vec<f32> = labels
                    .iter()
                    .map(|&label| {
                        let narrowed = label as f32;
                        debug_assert_eq!(label, f64::from(narrowed));
                        narrowed
                    })
                    .collect();
                self.paint_entry(&entry.item, &labels);
            }
        }
        for (source, command) in census.sources.iter().zip(&plan.source_commands) {
            if let render::paint_plan::PaintCommand::Relabel(labels) = command {
                source.clone().dyn_bind_mut().set_role_labels(labels);
            }
        }
        self.face_census = plan
            .faces
            .into_iter()
            .filter_map(|painted| {
                entries.get(painted.entry).map(|entry| FaceCensusEntry {
                    name: entry.name.clone(),
                    face: painted.face,
                    label: painted.label,
                    class: painted.class,
                })
            })
            .collect();
        for &entry_index in &plan.wall_merge_entries {
            let Some(entry) = entries.get(entry_index) else {
                continue;
            };
            let message = format!(
                "WaveLevel: '{}' overlaps the wall structure and is drawn as part of it — its faces take the walls' labels and its pierce lines draw. Pull it clear of the wall if that was a nudge, or leave it if the bump is authored.",
                entry.name
            );
            if !editor {
                godot_warn!("{}", message);
            }
            self.file_paint_fault(entry, message);
        }
    }

    /// One item per floor/ceiling slab and per authored solid — everything
    /// [`Self::paint_labels`] colours — with the `render::Shape` each
    /// builds its faces and derives its touch bound from.
    fn paint_entries(&self, census: &Census) -> Vec<PaintEntry> {
        let mut entries = Vec::new();
        let root = self.base().clone().upcast::<Node>();
        for slab in &self.slabs {
            if mesh_world_box(&slab.skin.clone().upcast()).is_none() {
                continue; // a slab with no mesh draws no seam
            }
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
                shape,
                item: PaintItem::Slab { lid: slab.lid },
            });
        }
        for solid in &census.solids {
            let node = solid.clone().into_gd();
            if mesh_world_box(&node).is_none() {
                continue; // draws nothing, so it can show no seam
            }
            let name = root.get_path_to(&node).to_string();
            if let Ok(wall) = node.clone().try_cast::<WaveWall>() {
                let shape = wall.bind().world_shape();
                entries.push(PaintEntry {
                    name,
                    shape,
                    item: PaintItem::Wall(wall),
                });
            } else if let Ok(prop) = node.clone().try_cast::<WaveProp>() {
                let shape = prop.bind().world_shape();
                entries.push(PaintEntry {
                    name,
                    shape,
                    item: PaintItem::Prop(prop),
                });
            } else if let Ok(column) = node.clone().try_cast::<WaveColumn>() {
                let shape = column.bind().world_shape();
                entries.push(PaintEntry {
                    name,
                    shape,
                    item: PaintItem::Column(column),
                });
            } else if let Ok(wedge) = node.clone().try_cast::<WaveWedge>() {
                let shape = wedge.bind().world_shape();
                entries.push(PaintEntry {
                    name,
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

    /// The world shape of a solid that is NOT a wall, for the occlusion
    /// admission walk.
    ///
    /// The same concrete dispatch [`Self::paint_entries`] performs, and for
    /// the same reason: `WaveSolid` carries only `set_material`, so the
    /// geometry lives on the concrete classes. Walls are excluded here
    /// because they are admitted unconditionally, by class contract, in the
    /// loop above — asking geometry about a wall could only ever refuse one.
    fn unwalled_world_shape(node: &Gd<Node>) -> Option<render::Shape> {
        if node.clone().try_cast::<WaveWall>().is_ok() {
            return None;
        }
        if let Ok(prop) = node.clone().try_cast::<WaveProp>() {
            return Some(prop.bind().world_shape());
        }
        if let Ok(column) = node.clone().try_cast::<WaveColumn>() {
            return Some(column.bind().world_shape());
        }
        if let Ok(wedge) = node.clone().try_cast::<WaveWedge>() {
            return Some(wedge.bind().world_shape());
        }
        None
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
    /// pushed as `u_walls`/`u_wall_y`/`u_wall_count` onto the world and
    /// source skins — the wall table their analytic sight test runs
    /// against.
    ///
    /// Loud about the shaders' slot ceiling BEFORE it is hit as well as
    /// after ([`level_plan::occluder_budget`]): a level past it has walls that
    /// silently stopped occluding, and a level one room short of it is
    /// about to. Only the truncation stays here, because it is the act the
    /// message describes — the words themselves are a decision over two
    /// numbers, and live in the pure plan where cargo can hold them.
    fn push_wall_table(&mut self, mut occluders: Vec<sight::Occluder>, editor: bool) {
        let budget = level_plan::occluder_budget(
            occluders.len().saturating_sub(self.spanning_solids.len()),
            self.spanning_solids.len(),
            sight::MAXW,
        );
        self.say(editor, budget);
        occluders.truncate(sight::MAXW); // a no-op below the ceiling
        // kept for the per-object source muffle: the walls a camera→source
        // sight line is counted against, once per frame on the CPU
        self.occluders = occluders;
        let rects = self.wall_rects();
        let spans = self.wall_spans();
        let count = self.occluders.len() as i64;
        self.push_table_to(self.data_mat.clone(), &rects, &spans, count);
        self.push_table_to(self.source_mat.clone(), &rects, &spans, count);
        for skin in self.extra_skins.clone() {
            self.push_table_to(Some(skin), &rects, &spans, count);
        }
    }

    /// Register a skin the composition root owns that occludes by this
    /// level's wall table, and hand it the table as it stands.
    ///
    /// Every later `derive()` refreshes it along with the level's own two.
    /// Registering is the point: pushing once, from outside, is correct
    /// only while nothing re-derives, and `rederive` is callable by anyone.
    pub(super) fn add_occluding_skin(&mut self, skin: Gd<Material>) {
        self.extra_skins.push(skin.clone());
        let rects = self.wall_rects();
        let spans = self.wall_spans();
        let count = self.occluders.len() as i64;
        self.push_table_to(Some(skin), &rects, &spans, count);
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
            (Some(floor), Some(ceiling)) => level_plan::slab_diagonal(
                floor,
                ceiling,
                &self.segments,
                level_plan::wall_sweep(&self.occluders),
            ),
            // slabs not yet built: nothing drawn to measure, nothing to say
            _ => 0.0,
        };
        let budget = level_plan::pack_range_budget(diagonal, level_plan::DIST_PACK_RANGE);
        self.say(editor, budget);
        // ...and the OTHER direction, which pulls against it. The budget
        // above tells a designer to raise DIST_PACK_RANGE when the map
        // outgrows it; this one refuses a range the B channel can no longer
        // reconstruct a world point from, which is what the hearing pass
        // does with it. They cross at 40.92 m and the shipped range is 40.0.
        let recon = render::channel::reconstruction_budget(level_plan::DIST_PACK_RANGE);
        self.say(editor, recon);
    }

    /// Push the wall table onto one data-writing material — loud when it is
    /// no ShaderMaterial (legal in tests, blind in the game).
    fn push_table_to(
        &self,
        mat: Option<Gd<Material>>,
        rects: &PackedVector4Array,
        spans: &PackedVector2Array,
        count: i64,
    ) {
        let Some(mat) = mat else {
            return; // uninjected: ready() already said so loudly
        };
        match mat.try_cast::<ShaderMaterial>() {
            Ok(mut shader_mat) => {
                shader_mat.set_shader_parameter("u_walls", &rects.to_variant());
                shader_mat.set_shader_parameter("u_wall_y", &spans.to_variant());
                shader_mat.set_shader_parameter("u_wall_count", &count.to_variant());
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
    /// pair is what `set_extents`, the fixed slab-label anchors and the seam
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
    /// source, cat and spawn datum's path and global pose, plus a
    /// solid's skin AABB, so the fold changes the moment ANYTHING
    /// [`Self::derive`] reads would read differently. `extents` has to be
    /// folded here explicitly rather than discovered through the census:
    /// it is not a censused node's property, it is read straight off
    /// `self` — `report_placement` measures the floor slab's world box,
    /// which `set_extents` resizes the instant the knob is dragged, and
    /// the face labeller anchors the slab roles against that same box — so a
    /// resize with every node held still is a real change `derive` would
    /// answer differently, and the fold has to see it as one.
    ///
    /// THREE CENSUS WALKS on a changed frame, deliberately: the signature
    /// measurement, then `derive`'s own census, then its post-derive signature
    /// refresh. An unchanged frame performs only the first. This
    /// runs every editor frame; `derive` runs only when the signature just
    /// computed differs from the last one — the ordinary case is a still
    /// scene, one walk, no second one to share. Threading a `&Census`
    /// through `derive` (which also mutates `self.segments`,
    /// `self.source_children` and the rest from it) would couple two
    /// pieces of code that change for different reasons — "what does the
    /// level derive" and "what does the level watch" — to save a walk that
    /// is bounded by the authored scene size. If this ever profiles hot,
    /// that coupling is the next place to look, not before.
    fn scene_signature(&self) -> u64 {
        let census = self.census();
        let root = self.base().clone().upcast::<Node>();
        let mut nodes: Vec<level_plan::SignatureNode> = Vec::new();
        for solid in &census.solids {
            let node = solid.clone().into_gd();
            let aabb = node
                .clone()
                .try_cast::<WaveWall>()
                .ok()
                .and_then(|wall| wall.bind().signature_aabb())
                .or_else(|| skin_local_aabb(&node));
            nodes.push(level_plan::SignatureNode {
                path: root.get_path_to(&node).to_string(),
                instance_identity: node.instance_id().to_i64(),
                transform: transform_floats(&node),
                aabb,
            });
        }
        for source in &census.sources {
            let node = source.clone().into_gd();
            nodes.push(level_plan::SignatureNode {
                path: root.get_path_to(&node).to_string(),
                instance_identity: node.instance_id().to_i64(),
                transform: transform_floats(&node),
                aabb: None,
            });
        }
        for cat in &census.cats {
            let node = cat.clone().upcast::<Node>();
            nodes.push(level_plan::SignatureNode {
                path: root.get_path_to(&node).to_string(),
                instance_identity: node.instance_id().to_i64(),
                transform: transform_floats(&node),
                aabb: None,
            });
        }
        for spawn in &census.spawns {
            let node = spawn.clone().upcast::<Node>();
            nodes.push(level_plan::SignatureNode {
                path: root.get_path_to(&node).to_string(),
                instance_identity: node.instance_id().to_i64(),
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

/// One painted solid as observer compatibility sees it: what it is called,
/// the world box it fills, and the first real face label exposed by the
/// legacy solid-granularity `oid()` bridge.
///
/// Used to also carry a `swept` flag marking a sound source's box as a
/// SWEPT ENVELOPE (limbs' union grown by `sweep_margin`) rather than drawn
/// faces, so the OLD fight census could skip it — an envelope's planes
/// rasterise nothing, and every per-id copy of one union box was
/// coplanar-same-facing with its siblings on all six faces, so a census
/// fed the envelope reported each source z-fighting itself. The new
/// per-face postcondition ([`FaceCensusEntry`], `observe::oids::
/// coplanar_label_faults`) never treats a source as a world face. Source
/// limbs bake their graph-coloured semantic roles directly, so the skip flag
/// had nothing left to guard and is gone with it.
pub(super) struct PaintedSolid {
    pub(super) name: String,
    pub(super) area: oid_palette::Box3,
    pub(super) oid: f64,
}

/// What the debug observer ([`super::observer`]) reads back off a level.
///
/// Most of it is not `#[func]`: the designer-facing API does not grow for a
/// debugging tool. The sole read-only forwarding surface is `wall_names`,
/// paired with the already-public wall segment table so the editor probe can
/// catch a stale retained generation. Nothing here is stored independently —
/// every accessor reads the level state or scene as it stands, so the observer
/// reports the world the renderer will actually draw rather than a mirrored
/// copy that can drift away from it.
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
    /// exposes only the test-facing node list, while Rust drives their clocks.
    /// This is the in-crate door capture uses to read each cat's whole private
    /// life (brain, gait, tail, pose), none of which is `#[func]`.
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
        let root = self.base().clone().upcast::<Node>();
        // The authored walls first, in the slot order they were built in.
        let mut names: Vec<String> = self
            .wall_children
            .iter()
            .enumerate()
            .map(|(index, wall)| {
                if wall.is_instance_valid() {
                    root.get_path_to(&wall.clone().upcast::<Node>()).to_string()
                } else {
                    format!("<freed wall {index}>")
                }
            })
            .collect();
        // ...then the solids geometry admitted, in the order `derive`
        // appended them. Without this the table would carry occluders no
        // name could reach, and `explain_ray` — whose whole job is to say
        // WHICH wall stopped a ray — would either run off the end of this
        // list or blame the last authored wall for a pillar's work. That is
        // the confident-wrong answer the observability layer exists to
        // prevent, so the two lists grow together or the invariant is a lie.
        names.extend(self.spanning_solids.iter().cloned());
        names.truncate(self.occluders.len());
        names
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
            // One swept box for every semantic source role, carrying the
            // actual graph-coloured label baked by `paint_labels`.
            for role in 0..bound.role_count() {
                let Some(oid) = bound.role_label(usize::from(role)) else {
                    continue;
                };
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

/// The fixed role label a creature is painted with, read off the first limb
/// whose mesh carries a `CUSTOM0` channel. A creature paints every limb
/// with one label (the whole animal is one silhouette), so the first limb
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

/// The first per-vertex label a mesh instance carries right now — the
/// backward-compatible solid-granularity observer bridge, read straight
/// back off the skin's own `CUSTOM0`, exactly what the shader reads for G.
/// [`oid_palette::NO_OID`] when nothing has painted it.
fn read_oid(skin: &Gd<MeshInstance3D>) -> f64 {
    mesh_first_label(skin).unwrap_or(oid_palette::NO_OID)
}

/// The recursive half of [`WaveLevel::census`]: depth-first, scene
/// order — the deterministic order every derivation tiebreak leans on.
///
/// A child is recognised by what it CAN DO, not by what it is: `try_dynify`
/// asks the `#[godot_dyn]` registry whether this node's dynamic class
/// implements the trait. Typed arms retain the extra contracts the two
/// traits intentionally do not offer: wall centerlines, cat clocks,
/// drawless spawn data, and run generation/material injection.
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
        } else if let Ok(spawn) = child.clone().try_cast::<WaveSpawn>() {
            census.spawns.push(spawn);
        } else if let Ok(run) = child.clone().try_cast::<WaveRun>() {
            census.runs.push(run);
        }
        collect(&child, census);
    }
}

/// The 12 floats of a node's global transform — basis columns (X, Y, Z)
/// then origin — the pose half of [`WaveLevel::scene_signature`]. Every
/// censused node is Node3D-derived (walls and runs are authored Node3D
/// data, other solids stand on their own Node3D-family bases, and a spawn
/// stands on `Marker3D`), so the cast is total in the scenes this ever runs
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

fn source_paint_fault_text(path: &str, fault: render::paint_plan::SourceFault) -> String {
    match fault {
        render::paint_plan::SourceFault::InvalidArea => format!(
            "WaveLevel: source '{path}' has non-finite or reversed paint bounds — keeping its existing role labels."
        ),
        render::paint_plan::SourceFault::InvalidSweepMargin => format!(
            "WaveLevel: source '{path}' has a non-finite sweep margin — keeping its existing role labels."
        ),
    }
}

#[cfg(test)]
mod paint_fault_tests {
    use super::*;

    /// Current source implementations expose finite engine constants for
    /// sweep margins, and Godot does not provide a reliable fixture API for a
    /// poisoned mesh AABB. Pin the boundary's complete shared diagnostic here:
    /// it includes the actionable level-relative owner and is used byte-for-
    /// byte for both the runtime log and stored node fault.
    #[test]
    fn malformed_source_paint_faults_name_their_distinct_owners() {
        assert_eq!(
            source_paint_fault_text("Sources/FanA", render::paint_plan::SourceFault::InvalidArea),
            "WaveLevel: source 'Sources/FanA' has non-finite or reversed paint bounds — keeping its existing role labels."
        );
        assert_eq!(
            source_paint_fault_text(
                "Sources/FanB",
                render::paint_plan::SourceFault::InvalidSweepMargin
            ),
            "WaveLevel: source 'Sources/FanB' has a non-finite sweep margin — keeping its existing role labels."
        );
    }
}
