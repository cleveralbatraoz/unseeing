//! The debug observability boundary — `WaveObserver`.
//!
//! Adds no law. It holds references to the systems it was injected with,
//! calls the pure functions in [`crate::observe`], and converts the results
//! to `VarDictionary` so GDScript's `JSON.stringify` can encode them. That
//! is the whole job.
//!
//! Every entry point refuses loudly when it cannot reach something it must
//! read: a snapshot of nothing and a snapshot of an empty world must never
//! serialise the same. The refusal carries ONE key — an agent that reads
//! `live_count == 0` from an observer that never had a pool will spend an
//! hour debugging silence that was never there.
//!
//! The camera is not optional equipment for a snapshot, and that is the
//! same rule rather than an exception to it: how many walls stand between
//! the hero and a source, and how muffled that source's standing image is,
//! are measured FROM the eye. Without one the observer would have to
//! invent an eye at the origin, and a plausible wrong number is the most
//! dangerous answer of all. The eye-free explainers keep working.

use godot::classes::{
    Camera3D, INode, Material, MeshInstance3D, PhysicsDirectSpaceState3D, ShaderMaterial,
};
use godot::prelude::*;

use super::level::WaveLevel;
use super::source;
use crate::ffi::{WaveCore, cast_reflection_fan};
use crate::level_plan;
use crate::observe::evict::{EvictionPlan, EvictionRule, explain_eviction};
use crate::observe::oids::{OidExplanation, explain_oids_checked};
use crate::observe::pool::{SlotObservation, SlotState};
use crate::observe::ray::{self, RayExplanation};
use crate::observe::reflect::{
    self, Answer, ClusteredPoint, Collected, ExplanationLedger, ReflectionExplanation,
    ReflectionRequest,
};
use crate::observe::{FrameObservation, SourceObservation, frame};
use crate::ray_fan;

/// No level: the observer was never handed the world to read.
const NO_LEVEL: &str = "observer was never injected a level";

/// No camera: every eye-relative quantity in a snapshot would be a guess.
const NO_CAMERA: &str = "observer was never injected a camera — walls_to_eye and source_floor are measured from the eye";

/// A level whose wave pool never arrived, or arrived as something that is
/// not a pool at all.
const NO_POOL: &str = "the injected level carries no readable wave pool";

/// The level was injected and has since been freed. A scene reload leaves
/// the handle looking perfectly valid, and reading through it would take
/// the game down with the observer — which is the worst thing a debugging
/// tool can do to the run it exists to explain.
const DEAD_LEVEL: &str = "the injected level has been freed";

/// The camera was injected and has since been freed.
const DEAD_CAMERA: &str = "the injected camera has been freed";

/// An explanation id nobody is holding an answer for. Never issued,
/// already collected, or aged out of the book — all three mean the same
/// thing to the asker, and none of them is a fan that struck nothing.
const NO_SUCH_REQUEST: &str =
    "no such explanation request — never issued, already collected, or aged out";

/// The observer stands outside any physics world, so the reflection fan
/// has nothing to cast against. Refused rather than answered with zero
/// hits: a fan that struck nothing is a fact about the ROOM.
const NO_SPACE: &str = "the observer stands in no physics world — reflection rays need one";

/// The agent's window into the running wave engine: it reads every system
/// and drives none.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct WaveObserver {
    level: Option<Gd<WaveLevel>>,
    camera: Option<Gd<Camera3D>>,
    /// Reflection questions waiting on a physics frame, and the answers
    /// that frame produced.
    explanations: ExplanationLedger,
    base: Base<Node>,
}

#[godot_api]
impl INode for WaveObserver {
    /// The one moment a physics space may be touched. Every reflection
    /// question booked since the last tick is cast and answered here, into
    /// a SCRATCH buffer that never reaches the echo book the game drains.
    fn physics_process(&mut self, _dt: f64) {
        let requests = self.explanations.take_requests();
        if requests.is_empty() {
            return; // the overwhelmingly common case: no work, no space lookup
        }
        let space = self.space_state();
        for (id, request) in requests {
            let answer = match space.clone() {
                Some(space) => Answer::Explained(Box::new(cast_and_explain(&request, space))),
                None => Answer::Refused(NO_SPACE),
            };
            self.explanations.answer(id, answer);
        }
    }
}

#[godot_api]
impl WaveObserver {
    /// Hand the observer the systems to read. Called once by the
    /// composition root; nothing is owned, only borrowed. The wave pool is
    /// not passed separately — the level was already injected with it, and
    /// two references to one pool could disagree.
    #[func]
    fn inject(&mut self, level: Option<Gd<WaveLevel>>, camera: Option<Gd<Camera3D>>) {
        self.level = level;
        self.camera = camera;
    }

    /// The whole state vector as of `now`: the pool slot by slot, the next
    /// eviction, every sound source as an agent reads it, the wall table,
    /// and where the eye stands.
    ///
    /// Anything that could not be observed is named in `unknown` and its
    /// key is ABSENT — never present and zero.
    #[func]
    fn snapshot(&self, now: f64) -> VarDictionary {
        let level = match self.live_level() {
            Ok(level) => level,
            Err(reason) => return unavailable(reason),
        };
        let camera = match self.live_camera() {
            Ok(camera) => camera,
            Err(reason) => return unavailable(reason),
        };
        let level = level.bind();
        let Some(core) = pulse_core(&level) else {
            return unavailable(NO_POOL);
        };
        let eye = camera.get_global_position();
        let rects: Vec<Vector4> = level.wall_rects().as_slice().to_vec();
        let flick = shader_flick(level.data_material());
        let observation = frame(
            core.bind().pool(),
            now,
            // NaN, never zero: it is only read back below when the material
            // actually answered, and a leak would be loud rather than
            // mistaken for a world rendered at flicker zero.
            flick.unwrap_or(f64::NAN),
            sources(&level, eye, &rects),
            rects,
            eye,
            camera.get_global_transform().basis,
        );
        frame_dict(&observation, flick.is_some())
    }

    /// What the walls do to one sight line — the occlusion oracle, keyed to
    /// both occluders (the eye's, which counts every wall, and the source's,
    /// which skips the wall a sound was born inside).
    #[func]
    fn explain_ray(&self, from: Vector3, to: Vector3) -> VarDictionary {
        let level = match self.live_level() {
            Ok(level) => level,
            Err(reason) => return unavailable(reason),
        };
        let level = level.bind();
        let rects: Vec<Vector4> = level.wall_rects().as_slice().to_vec();
        let explanation = ray::explain_ray(from, to, &rects, level_plan::WALL_H as f32);
        ray_dict(&explanation, &level.wall_names())
    }

    /// The touch graph and its colouring, over every painted box in the
    /// level: which seams the flat object ids actually draw.
    #[func]
    fn explain_oids(&self) -> VarDictionary {
        let level = match self.live_level() {
            Ok(level) => level,
            Err(reason) => return unavailable(reason),
        };
        let painted = level.bind().oid_census();
        let boxes: Vec<_> = painted.iter().map(|solid| solid.area).collect();
        let ids: Vec<f64> = painted.iter().map(|solid| solid.oid).collect();
        let names: Vec<&str> = painted.iter().map(|solid| solid.name.as_str()).collect();
        let Some(explanation) = explain_oids_checked(&boxes, &ids) else {
            // unreachable by construction (the census carries one id per
            // box), and still refused rather than reported: a truncated
            // check that found no violations is a vacuous pass
            return unavailable("the level's painted boxes and their ids do not line up");
        };
        oid_dict(&explanation, &names)
    }

    /// Which slot the next sound would claim, and by which rule. Eviction
    /// happens between frames and overwrites its own evidence, so this is
    /// re-derived rather than observed — and never by calling `emit`,
    /// which would answer the question by changing it.
    #[func]
    fn explain_eviction(&self, now: f64) -> VarDictionary {
        let level = match self.live_level() {
            Ok(level) => level,
            Err(reason) => return unavailable(reason),
        };
        let Some(core) = pulse_core(&level.bind()) else {
            return unavailable(NO_POOL);
        };
        let plan = explain_eviction(core.bind().pool(), now);
        eviction_dict(&plan)
    }

    /// Ask why a surface answered, or did not, and get an id back.
    ///
    /// This is a REQUEST rather than an answer because the reflection fan
    /// is cast with physics rays, and a space state may only be touched
    /// inside the physics tick — the same reason the player queues its
    /// waves. The next `_physics_process` casts it; [`Self::take_explanation`]
    /// collects it.
    ///
    /// The sound is described, never emitted: no kind, no loudness, no
    /// space is carried in, and nothing the answer touches is the running
    /// game's. `normal` is the birth surface's, or `ZERO` for a sound born
    /// in the air; `now` is the clock the appointments are measured from,
    /// which is why it is asked for rather than invented.
    ///
    #[func]
    fn request_explain_reflection(
        &mut self,
        at: Vector3,
        normal: Vector3,
        max_r: f64,
        speed: f64,
        max_echoes: i64,
        now: f64,
    ) -> i64 {
        let request = ReflectionRequest {
            at,
            normal,
            max_r,
            speed,
            max_echoes,
            now,
        };
        self.explanations.request(request)
    }

    /// Collect a booked explanation: `{"pending": true}` until the physics
    /// frame has run, the explanation exactly once thereafter, and a
    /// refusal for an id nobody holds an answer for.
    #[func]
    fn take_explanation(&mut self, request_id: i64) -> VarDictionary {
        match self.explanations.collect(request_id) {
            Collected::Pending => {
                let mut waiting = VarDictionary::new();
                waiting.set("pending", true);
                waiting
            }
            Collected::Ready(Answer::Explained(explanation)) => {
                reflection_dict(request_id, &explanation)
            }
            Collected::Ready(Answer::Refused(reason)) => unavailable(reason),
            Collected::Unknown => unavailable(NO_SUCH_REQUEST),
        }
    }

    /// The nominal size of the golden-angle reflection fan, before any
    /// hemisphere cull. Served from the same pure core the engine casts
    /// from, so an agent comparing it against an explanation's `rays_cast`
    /// is reading one number, not two that can drift.
    #[func]
    fn ray_fan_size(&self) -> i64 {
        ray_fan::RAYS as i64
    }
}

impl WaveObserver {
    /// The level, if there is one and it still exists. A freed node leaves
    /// its handle looking valid, so every entry point asks first: the
    /// observer must refuse a torn-down scene, not read through it.
    fn live_level(&self) -> Result<&Gd<WaveLevel>, &'static str> {
        match self.level.as_ref() {
            None => Err(NO_LEVEL),
            Some(level) if !level.is_instance_valid() => Err(DEAD_LEVEL),
            Some(level) => Ok(level),
        }
    }

    /// The eye, under the same rule.
    fn live_camera(&self) -> Result<&Gd<Camera3D>, &'static str> {
        match self.camera.as_ref() {
            None => Err(NO_CAMERA),
            Some(camera) if !camera.is_instance_valid() => Err(DEAD_CAMERA),
            Some(camera) => Ok(camera),
        }
    }

    /// The physics space the observer itself stands in. A plain `Node` has
    /// no world of its own — it borrows its viewport's, which is the same
    /// world the level it was injected with is drawn and collided in, as
    /// long as both were placed in the same tree. An observer outside a
    /// tree has none, and says so rather than casting into nothing.
    fn space_state(&self) -> Option<Gd<PhysicsDirectSpaceState3D>> {
        self.base()
            .get_viewport()
            .and_then(|viewport| viewport.get_world_3d())
            .and_then(|world| world.get_direct_space_state())
    }
}

/// Cast one reflection fan and explain it.
///
/// The cast is not mirrored from the engine's — it IS the engine's. Both
/// this and `WaveCore::emit_reflecting` call [`cast_reflection_fan`], so a
/// collision mask, an exclusion, or a changed query added for the game
/// cannot leave the explanation sampling a world the engine stopped
/// sampling. What differs is only the tail: these hits go into a local
/// vector and then into the pure explainer, and no echo is ever scheduled.
fn cast_and_explain(
    request: &ReflectionRequest,
    space: Gd<PhysicsDirectSpaceState3D>,
) -> ReflectionExplanation {
    let (rays_cast, hits) = cast_reflection_fan(request, space);
    reflect::explain_clustering(request, rays_cast, &hits)
}

/// The one refusal shape. A dictionary carrying only this key is how an
/// agent learns it asked a question that could not be answered — as
/// opposed to one whose answer happens to be empty.
fn unavailable(reason: &str) -> VarDictionary {
    let mut refusal = VarDictionary::new();
    refusal.set("unavailable", reason);
    refusal
}

/// The Rust wave core behind whatever the level was injected with: the
/// core itself in a suite that hands one over directly, or the GDScript
/// `Pulses` shim's own, reached through its public `core()` accessor.
fn pulse_core(level: &WaveLevel) -> Option<Gd<WaveCore>> {
    let handle = level.pulse_handle()?;
    if !handle.is_instance_valid() {
        return None;
    }
    if let Ok(core) = handle.clone().try_cast::<WaveCore>() {
        return Some(core);
    }
    let mut shim = handle;
    if !shim.has_method("core") {
        return None;
    }
    shim.call("core", &[]).try_to::<Gd<WaveCore>>().ok()
}

/// The flicker the shaders are actually holding, read back off the world
/// skin. `None` when the material cannot answer — an unbound skin, or a
/// level no frame has ever run over — because a flicker of zero is a mood
/// the game can genuinely be in and must not be invented.
fn shader_flick(material: Option<Gd<Material>>) -> Option<f64> {
    material?
        .try_cast::<ShaderMaterial>()
        .ok()?
        .get_shader_parameter("u_flick")
        .try_to::<f64>()
        .ok()
}

/// Every sound source as an agent reads it.
///
/// `walls_to_eye` is a QUESTION — the camera occluder's count, asked here
/// against the same `sight` oracle the shaders transliterate. The standing
/// image is not: it is state the level composed and PUSHED, so it is read
/// straight back off a limb rather than recomposed. Recomputing it would
/// put a rule (`volume x muffle`) in the boundary beside the one in
/// `WaveLevel::tick_sources`, and the two would agree right up until the
/// frame they mattered — a frozen world, a stalled `_process`, a term
/// added to one of them — while the observer went on confidently
/// reporting a number no shader was holding. Unobserved is reported as
/// [`f64::NAN`] and lands in the snapshot's `unknown`, never as a guess.
fn sources(level: &WaveLevel, eye: Vector3, rects: &[Vector4]) -> Vec<SourceObservation> {
    level
        .source_handles()
        .iter()
        .map(|source| {
            let node = source.clone().into_gd();
            let name = node.get_name().to_string();
            let bound = source.dyn_bind();
            let voice = bound.voice();
            let position = bound.hub();
            let line = ray::explain_ray(eye, position, rects, level_plan::WALL_H as f32);
            SourceObservation {
                name,
                position,
                volume: voice.volume.amplitude(),
                reach: voice.volume.reach(),
                walls_to_eye: line.camera_crossings,
                source_floor: standing_image(&node).unwrap_or(f64::NAN),
                slot_pressure: voice.slot_pressure(),
            }
        })
        .collect()
}

/// The standing acoustic image a source's limbs are actually carrying —
/// the `u_source_floor` instance uniform the level pushes each frame. Every
/// limb of one source is pushed the same value, so the first that answers
/// speaks for the source. `None` before any frame has driven it, which is a
/// different fact from an image of zero (a source muffled to silence).
fn standing_image(node: &Gd<Node>) -> Option<f64> {
    if let Ok(limb) = node.clone().try_cast::<MeshInstance3D>()
        && let Ok(image) = limb
            .get_instance_shader_parameter(source::IMAGE_PARAM)
            .try_to::<f64>()
    {
        return Some(image);
    }
    node.get_children()
        .iter_shared()
        .find_map(|child| standing_image(&child))
}

fn frame_dict(observation: &FrameObservation, flick_known: bool) -> VarDictionary {
    let mut unknown: Array<GString> = Array::new();
    let mut state = VarDictionary::new();
    state.set("now", observation.now);
    if flick_known {
        state.set("flick", observation.flick);
    } else {
        unknown.push("flick");
    }
    state.set("live_count", observation.live_count as i64);
    let slots: Array<VarDictionary> = observation.slots.iter().map(slot_dict).collect();
    state.set("slots", &slots);
    state.set("next_eviction", &eviction_dict(&observation.next_eviction));
    let sources: Array<VarDictionary> = observation.sources.iter().map(source_dict).collect();
    for (index, source) in observation.sources.iter().enumerate() {
        if source.source_floor.is_nan() {
            unknown.push(&format!("sources[{index}].source_floor"));
        }
    }
    state.set("sources", &sources);
    state.set(
        "wall_rects",
        &PackedVector4Array::from(&observation.wall_rects[..]),
    );
    state.set("wall_truncated", observation.wall_truncated);
    state.set("camera", &camera_dict(observation));
    state.set("unknown", &unknown);
    state
}

fn slot_dict(slot: &SlotObservation) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("index", slot.index as i64);
    entry.set("state", state_name(slot.state));
    entry.set("kind", i64::from(slot.kind));
    entry.set("origin", slot.origin);
    entry.set("birth", slot.birth);
    entry.set("max_r", slot.max_r);
    entry.set("speed", slot.speed);
    entry.set("gain", slot.gain);
    entry.set("beam", slot.beam);
    entry.set("cos_half", slot.cos_half);
    entry.set("ring_radius", slot.ring_radius);
    entry.set("age", slot.age);
    entry.set("remaining", slot.remaining);
    entry.set("end", slot.end);
    entry
}

fn eviction_dict(plan: &EvictionPlan) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("slot", plan.slot as i64);
    entry.set("rule", rule_name(plan.rule));
    entry.set("victim_kind", i64::from(plan.victim_kind));
    entry
}

fn source_dict(source: &SourceObservation) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("name", source.name.as_str());
    entry.set("position", source.position);
    entry.set("volume", source.volume);
    entry.set("reach", source.reach);
    entry.set("walls_to_eye", i64::from(source.walls_to_eye));
    // NaN is the "never pushed" marker set by `standing_image`, and the one
    // value the uniform can never legitimately hold: the key is left out
    // and named in the snapshot's `unknown` instead of reported as a guess.
    if !source.source_floor.is_nan() {
        entry.set("source_floor", source.source_floor);
    }
    entry.set("slot_pressure", source.slot_pressure);
    entry
}

/// Where the eye stands and where it looks. A Godot camera looks down its
/// own -Z, so the heading is the negated third basis column.
fn camera_dict(observation: &FrameObservation) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("position", observation.camera);
    entry.set("forward", -observation.camera_basis.col_c());
    entry
}

fn ray_dict(explanation: &RayExplanation, names: &[String]) -> VarDictionary {
    let walls: Array<VarDictionary> = explanation
        .walls
        .iter()
        .map(|wall| {
            let mut entry = VarDictionary::new();
            entry.set("index", wall.index as i64);
            entry.set("name", wall_name(names, wall.index).as_str());
            entry.set("rect", wall.rect);
            entry.set("crossed", wall.crossed);
            entry.set("contains_origin", wall.contains_origin);
            entry
        })
        .collect();
    let mut entry = VarDictionary::new();
    entry.set("from", explanation.from);
    entry.set("to", explanation.to);
    entry.set("wall_top", f64::from(explanation.wall_top));
    entry.set("camera_crossings", i64::from(explanation.camera_crossings));
    entry.set("source_crossings", i64::from(explanation.source_crossings));
    entry.set("hum_transmission", explanation.hum_transmission);
    entry.set("source_transmission", explanation.source_transmission);
    entry.set("walls", &walls);
    entry
}

/// One reflection fan, as an agent reads it.
///
/// Every ray is present in the counts, and every reason a hit failed to
/// answer is a SEPARATE key. An agent asking "why did that wall stay
/// silent" gets to distinguish a ray that reached its full length and
/// found nothing, a hit vetoed as the sound's own birth surface, a hit
/// that merged into a nearer strike's cell, and a cell that ranked past
/// the echo budget — four different bugs, four different fixes.
fn reflection_dict(request_id: i64, explanation: &ReflectionExplanation) -> VarDictionary {
    let points: Array<VarDictionary> = explanation.points.iter().map(point_dict).collect();
    let mut entry = VarDictionary::new();
    entry.set("request_id", request_id);
    entry.set("at", explanation.at);
    entry.set("origin", explanation.origin);
    entry.set("normal", explanation.normal);
    entry.set("reach", explanation.reach);
    entry.set("fan_size", explanation.fan_size as i64);
    entry.set("rays_cast", explanation.rays_cast as i64);
    entry.set("rays_struck", explanation.rays_struck as i64);
    entry.set("rays_missed", explanation.rays_missed() as i64);
    entry.set("self_surface_drops", explanation.self_surface_drops as i64);
    entry.set("merged_into_cells", explanation.merged_into_cells as i64);
    entry.set("cells_found", explanation.cells_found as i64);
    entry.set("budget", explanation.budget as i64);
    entry.set(
        "dropped_past_budget",
        explanation.dropped_past_budget as i64,
    );
    entry.set("clusters_kept", explanation.clusters_kept() as i64);
    entry.set("points", &points);
    entry
}

/// One answering point. `dist` widens to f64 here and nowhere earlier —
/// it is a single-precision geometry length everywhere inside the engine,
/// and Godot has no narrower float on the wire.
fn point_dict(point: &ClusteredPoint) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("point", point.point);
    entry.set("dist", f64::from(point.dist));
    entry.set("at_t", point.at_t);
    entry.set("gain_fraction", point.gain_fraction);
    entry
}

/// The name of the wall in table slot `index`. Total: a table longer than
/// the census that produced the names still answers, and says so.
fn wall_name(names: &[String], index: usize) -> String {
    names
        .get(index)
        .cloned()
        .unwrap_or_else(|| format!("<unnamed wall {index}>"))
}

fn oid_dict(explanation: &OidExplanation, names: &[&str]) -> VarDictionary {
    let pairs: Array<VarDictionary> = explanation
        .pairs
        .iter()
        .map(|pair| {
            let mut entry = VarDictionary::new();
            entry.set("a", pair.a as i64);
            entry.set("b", pair.b as i64);
            entry.set("name_a", &box_name(names, pair.a));
            entry.set("name_b", &box_name(names, pair.b));
            entry.set("oid_a", pair.oid_a);
            entry.set("oid_b", pair.oid_b);
            entry.set("delta", pair.delta);
            entry.set("draws", pair.draws);
            entry
        })
        .collect();
    let census: Array<GString> = names.iter().map(|&name| GString::from(name)).collect();
    let violations: Array<i64> = explanation
        .violations
        .iter()
        .map(|&index| index as i64)
        .collect();
    let mut entry = VarDictionary::new();
    entry.set("names", &census);
    entry.set("pairs", &pairs);
    entry.set("violations", &violations);
    entry.set("min_sep", explanation.min_sep);
    entry
}

/// The name of painted box `index`. Total, for the same reason as
/// [`wall_name`].
fn box_name(names: &[&str], index: usize) -> GString {
    names.get(index).map_or_else(
        || GString::from(&format!("<unnamed box {index}>")),
        |&name| GString::from(name),
    )
}

/// The wire name of a slot's state. Spelled out rather than derived from
/// `Debug`, because an agent's parser is a contract and a rename of the
/// enum must not silently break it.
fn state_name(state: SlotState) -> &'static str {
    match state {
        SlotState::Never => "Never",
        SlotState::Expired => "Expired",
        SlotState::Live => "Live",
    }
}

/// The wire name of an eviction rule, spelled out for the same reason.
fn rule_name(rule: EvictionRule) -> &'static str {
    match rule {
        EvictionRule::Expired => "Expired",
        EvictionRule::OldestRecurring => "OldestRecurring",
        EvictionRule::OldestOverall => "OldestOverall",
        EvictionRule::Fallback => "Fallback",
    }
}
