//! The debug observability boundary — `WaveObserver`.
//!
//! Adds no law. It holds references to the systems it was injected with,
//! calls the pure functions in [`crate::observe`], and converts the results
//! to `VarDictionary` so Godot-side transports can encode them with
//! `JSON.stringify`. That is the whole job.
//!
//! Every entry point refuses loudly when it cannot reach something it must
//! read: a snapshot of nothing and a snapshot of an empty world must never
//! serialise the same. The refusal carries ONE key — an agent that reads
//! `live_slots == 0` from an observer that never had a pool will spend an
//! hour debugging silence that was never there.
//!
//! The camera is not optional equipment for a snapshot, and that is the
//! same rule rather than an exception to it: how many walls stand between
//! the hero and a source, and where the eye itself stands and looks, are
//! measured FROM the eye. Without one the observer would have to invent an
//! eye at the origin, and a plausible wrong number is the most dangerous
//! answer of all. The eye-free explainers keep working.
//!
//! The standing image is NOT one of those quantities, and the refusal must
//! not claim it is: `source_volume`/`source_muffle` are read straight back
//! off a source's own limbs by [`standing_image`] and would be reportable
//! with no camera in the scene at all. It is refused along with the rest only because a
//! snapshot is all-or-nothing by design.

use godot::classes::{
    Camera3D, INode, Material, MeshInstance3D, PhysicsDirectSpaceState3D, ShaderMaterial, node,
};
use godot::prelude::*;

use super::hero::HeroBody;
use super::level::{FaceCensusEntry, WaveLevel};
use super::player::UnseeingPlayer;
use super::source;
use crate::cat_body::{CatPose, TAIL_N};
use crate::cat_brain::{BrainCapture, BrainState, RoamRect};
use crate::cat_gait::{GaitCapture, LEGS};
use crate::echo_queue::PendingEcho;
use crate::ffi::{WaveCore, cast_reflection_fan};
use crate::observe::evict::{EvictionPlan, EvictionRule, explain_eviction};
use crate::observe::oids::{
    LabelFault, OidExplanation, coplanar_label_faults, explain_oids_checked,
};
use crate::observe::pool::{SlotObservation, SlotState};
use crate::observe::ray::{self, RayExplanation};
use crate::observe::reflect::{
    self, Answer, CheckedReflectionRequest, ClusteredPoint, Collected, ExplanationLedger,
    ReflectionExplanation, ReflectionRequest,
};
use crate::observe::{
    EchoObservation, EyeObservation, FrameObservation, HeroObservation, QueuedWave,
    SceneObservation, SourceObservation, SpawnObservation, frame,
};
use crate::pulse_pool::{MAXP, SlotCapture};
use crate::ray_fan;
use crate::render::Face;
use crate::reproduce::{
    CaptureState, CatCapture, EnvCapture, FORMAT_VERSION, HeroCapture, SourceCapture,
    first_divergence, state_hash,
};
use crate::support_motion::{
    FiniteVelocity, GodotRotation, LandingEvent, MotionPhase, MotionState, PlanarVelocity,
    QueuedWaveGate, SupportContact,
};
use crate::viewmodel::ViewmodelCapture;

/// No level: the observer was never handed the world to read.
const NO_LEVEL: &str = "observer was never injected a level";

/// No camera: every eye-relative quantity in a snapshot would be a guess.
/// The two that genuinely are eye-relative are named, and nothing else —
/// a refusal that blamed the standing image (read back off the source's own
/// limbs, camera or no camera) would send a reader hunting the wrong system.
const NO_CAMERA: &str = "observer was never injected a camera — walls_to_eye and the camera group are measured \
     from the eye";

const DISABLED_HERO: &str = "the hero physics process is disabled — capture refused";

/// A level whose wave pool never arrived, or arrived as something that is
/// not a pool at all. `pub(super)`: [`super::restorer::WaveRestorer`] reuses
/// this exact string for its own refusal rather than spelling it a second
/// time, so the two absences read as one absence.
pub(super) const NO_POOL: &str = "the injected level carries no readable wave pool";

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

/// No hero at all. A SNAPSHOT names this in `unknown` and reports the
/// world around it; a CAPTURE cannot, because a blob is a world and a
/// world without its hero restores as a different one.
const NO_HERO: &str = "observer was never injected the hero — a blob carries the hero whole";

/// The hero was injected and has since been freed.
const DEAD_HERO: &str = "the injected hero has been freed";

/// The hero exists but its eye has not been built, so there is no pitch to
/// carry. A level gaze invented for an eyeless hero is exactly the
/// plausible wrong number this whole layer refuses to produce.
const NO_EYE: &str = "the hero never built its eye — the game is not running";

/// The hero BODY was never injected. It is a separate handle from the
/// hero because it is a separate node, and it is not optional equipment
/// for a capture: the viewmodel — the footstep clock included — lives
/// there and on no other node.
const NO_BODY: &str = "observer was never injected the hero body — the viewmodel clocks live there";

/// The hero body was injected and has since been freed.
const DEAD_BODY: &str = "the injected hero body has been freed";

/// The body exists but refused to build (uninjected), so there is no
/// viewmodel state to read. A default pose here would restore a walker
/// mid-stride as one standing still.
const NO_VM: &str = "the hero body never built its viewmodel — the game is not running";

/// A cat in the level never built its mind, gait, tail and pose. Refused
/// rather than defaulted: a defaulted cat is a cat with a different life.
const UNBUILT_CAT: &str = "a level cat was never built — capture refuses a defaulted cat";

/// A source is keeping no beat appointment, so there is no date to carry.
/// Restoring it would leave the gate to book a fresh one off the restored
/// clock — the spurious beat the whole appointment capture exists to stop.
const NO_APPOINTMENT: &str = "a source holds no beat appointment — the level has not ticked";

/// The env group is the caller's own dictionary — the one group no type
/// signature guards — so the refusal names the key rather than the group.
const BAD_ENV: &str = "the env group is missing or malformed: ";

/// The agent's window into the running wave engine: it reads every system
/// and drives none.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct WaveObserver {
    level: Option<Gd<WaveLevel>>,
    camera: Option<Gd<Camera3D>>,
    /// The hero to read, injected separately from the world: a suite
    /// building a bare level has no hero, and that absence is REPORTED
    /// (in `unknown`) rather than refusing the whole snapshot.
    player: Option<Gd<UnseeingPlayer>>,
    /// The hero's BODY, injected separately again: the viewmodel's whole
    /// state machine lives there, `snapshot` has never been able to see
    /// it, and `capture` cannot do without it.
    body: Option<Gd<HeroBody>>,
    /// Reflection questions waiting on a physics frame, and the answers
    /// that frame produced.
    explanations: ExplanationLedger,
    base: Base<Node>,
}

#[godot_api]
impl INode for WaveObserver {
    /// The window keeps working while the world is frozen.
    ///
    /// The documented debugging loop is freeze, input, step, snapshot,
    /// explain — so pausing the tree is its FIRST move, and an observer
    /// that stopped ticking under a pause would answer `pending` forever
    /// to every question asked inside the loop it exists to serve.
    /// `SettingsMenu` opts itself out of the pause it causes for the same
    /// reason (it must still hear Escape); this is the same rule, not an
    /// exception to it. Nothing here drives the world, so running while
    /// paused cannot advance the state being observed.
    fn ready(&mut self) {
        self.base_mut().set_process_mode(node::ProcessMode::ALWAYS);
    }

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
                Some(space) => match cast_and_explain(&request, space) {
                    Ok(explanation) => Answer::Explained(Box::new(explanation)),
                    Err(reason) => Answer::Refused(reason),
                },
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
    pub(super) fn inject(&mut self, level: Option<Gd<WaveLevel>>, camera: Option<Gd<Camera3D>>) {
        self.level = level;
        self.camera = camera;
    }

    /// Hand the observer the hero to read, separately from the world: a
    /// suite building a bare level has no hero, and that absence must be
    /// REPORTED (in `unknown`) rather than refusing the world around it.
    #[func]
    pub(super) fn inject_hero(&mut self, player: Option<Gd<UnseeingPlayer>>) {
        self.player = player;
    }

    /// Hand the observer the hero's BODY — a third injection, because the
    /// viewmodel is a third node. Only [`Self::capture`] reads it; a
    /// snapshot never has, so a suite that only snapshots may leave it
    /// unset exactly as before.
    #[func]
    pub(super) fn inject_body(&mut self, body: Option<Gd<HeroBody>>) {
        self.body = body;
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
        let source_observations =
            match sources(&level, eye, level.occluders(), level.prop_occluders()) {
                Ok(sources) => sources,
                Err(reason) => return unavailable(&reason),
            };
        // one bind, so the pool and the echo book are read from the same
        // core at the same instant rather than from two borrows of it
        let core = core.bind();
        let observation = frame(
            core.pool(),
            core.echoes(),
            now,
            // NaN, never zero: it is only read back below when the material
            // actually answered, and a leak would be loud rather than
            // mistaken for a world rendered at flicker zero.
            flick.unwrap_or(f64::NAN),
            SceneObservation {
                sources: source_observations,
                wall_rects: rects,
                eye: EyeObservation {
                    position: eye,
                    basis: camera.get_global_transform().basis,
                    fov: f64::from(camera.get_fov()),
                },
                spawn: SpawnObservation {
                    position: level.spawn_pos(),
                    yaw: level.spawn_yaw(),
                },
                hero: self.hero_observation(),
            },
        );
        frame_dict(&observation, flick.is_some())
    }

    /// The whole world at `now` as one restorable value — the blob.
    ///
    /// Strictly WIDER than [`Self::snapshot`]: the pool's f64 shadow, every
    /// standing echo appointment, each cat's whole private life, the
    /// viewmodel's clocks and the composition root's own `env` all reach a
    /// blob, and not one of them reaches a snapshot.
    ///
    /// Strictly STRICTER too — there is no `unknown` array here. A snapshot
    /// is a REPORT, and a report may admit a gap; a blob is a WORLD, and a
    /// world with a group missing restores into a different world while
    /// hashing like a valid one. So the first subsystem that cannot answer
    /// refuses the whole capture, one key, exactly as an uninjected
    /// snapshot does.
    ///
    /// `now` and `env.now` are the same instant said twice, and they are
    /// checked against each other rather than one of them trusted: every
    /// appointment in the blob is dated against this clock, and a blob
    /// dated at two instants restores into neither.
    ///
    /// Reads only. Nothing here emits a pulse, schedules an echo, advances
    /// a cadence, draws from a stream or moves a node — every handle is
    /// bound immutably, and `&self` is the compiler's half of that promise.
    #[func]
    fn capture(&self, now: f64, env: VarDictionary) -> VarDictionary {
        match self.capture_state(now, &env) {
            Ok(state) => state_dict(&state),
            Err(reason) => unavailable(&reason),
        }
    }

    /// Compute the canonical state hash of a blob after syntax parsing only.
    /// This intentionally does not ask the restorer whether the state is
    /// currently admissible: transaction tests and repair tools use it to
    /// label a deliberately future-shaped artifact without touching the
    /// running world.
    #[func]
    fn canonical_hash_of(&self, blob: VarDictionary) -> VarDictionary {
        match parse_blob(&blob) {
            Ok(state) => {
                let mut answer = VarDictionary::new();
                answer.set("hash", hex64(state_hash(&state)).as_str());
                answer
            }
            Err(reason) => unavailable(&reason),
        }
    }

    /// A blob's env group, spelled the way `UnseeingGame::capture_env` spells
    /// it — real floats, and the flicker's stream position as a plain int
    /// — or a one-key refusal naming what was wrong with it.
    ///
    /// The composition root OWNS the env: `now`, the demo tap's schedule,
    /// the flicker envelope and its RNG state. Applying a captured env back
    /// is therefore [`UnseeingGame`](super::game::UnseeingGame)'s boundary
    /// job. Godot's dynamic values cannot safely parse the blob's own
    /// spelling of every float, though. Measured
    /// on this build: `String.to_float` is not correctly rounded (it reads
    /// "0.016666666666666666" back one ULP away from 1/60), it drops the
    /// sign of "-0", and it reads "NaN" as zero — the same three losses
    /// that keep every float in the blob out of JSON's number syntax in
    /// the first place. So the text-to-float step stays here, and the
    /// restore boundary is handed nine already-parsed values it only has to
    /// assign.
    ///
    /// `pub(super)`: `UnseeingGame::restore_blob` calls this directly
    /// through a typed handle — it IS the composition root's own env-owning
    /// half the doc above describes, not a second caller.
    #[func]
    pub(super) fn env_of(&self, blob: VarDictionary) -> VarDictionary {
        let root = Group::new(&blob, Floats::Text, String::new());
        match root.group("env").and_then(|group| parse_env_group(&group)) {
            Ok(env) => env_dict(&env, Floats::Native),
            Err(reason) => unavailable(&reason),
        }
    }

    /// Read a blob and the same blob after a journey, and say whether the
    /// journey changed it: `""` when nothing moved, otherwise the dotted
    /// path of the first field that did — or the parse error, on whichever
    /// side failed to parse.
    ///
    /// A suite surface, and cheap on purpose. The blob's real destination
    /// is a file, and the trip out through `JSON.stringify` and back
    /// through `JSON.parse_string` is LOSSY for types this boundary must
    /// therefore never emit: a Godot vector comes back a pretty-printed
    /// String, every number comes back a float, and NaN has no spelling at
    /// all. This is the one door a test can push both ends through, and it
    /// names the field rather than the symptom — a hash that merely
    /// disagrees would leave the reader to find which of five thousand
    /// bytes moved.
    ///
    /// The blob's own `hash` key is checked here too, against the state it
    /// claims to describe. Nothing else ever verifies it: the restorer
    /// proves itself by re-capturing, not by trusting what a file says
    /// about itself.
    #[func]
    fn blob_round_trip_ok(&self, before: VarDictionary, after: VarDictionary) -> GString {
        let original = match parse_blob(&before) {
            Ok(state) => state,
            Err(reason) => {
                return GString::from(&format!(
                    "the blob before the journey did not parse — {reason}"
                ));
            }
        };
        let returned = match parse_blob(&after) {
            Ok(state) => state,
            Err(reason) => {
                return GString::from(&format!(
                    "the blob after the journey did not parse — {reason}"
                ));
            }
        };
        if let Some(field) = first_divergence(&original, &returned) {
            return GString::from(&format!("the journey changed {field}"));
        }
        let Some(claimed) = before.get("hash").and_then(|v| v.try_to::<GString>().ok()) else {
            return GString::from("the blob before the journey carries no hash key");
        };
        let actual = hex64(state_hash(&original));
        if claimed.to_string() == actual {
            GString::new()
        } else {
            GString::from(&format!(
                "the blob claims hash {claimed}, its own state hashes {actual}"
            ))
        }
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
        // the occluders themselves, spans included — not rebuilt from the
        // rect projection, which no longer carries a wall's height
        let explanation = ray::explain_ray(from, to, level.occluders(), level.prop_occluders());
        ray_dict(&explanation, &level.wall_names())
    }

    /// The touch graph and its colouring, plus the merge law's own
    /// postcondition.
    ///
    /// `pairs`/`violations` stay the SOLID-granularity law over every
    /// painted box (`WaveLevel::oid_census`, still a first-face bridged
    /// read) — legitimate for what it answers, because it only ever
    /// reasons about two SEPARATE (never coplanar-merged) solids, and the
    /// merge law's own singleton collapse guarantees such a solid carries
    /// one uniform label across every face.
    ///
    /// `superfaces` and `faults` are the campaign's own addition, read off
    /// `WaveLevel::face_census` — the rendering subsystem's own per-face
    /// census, not re-derived here. `superfaces` lists every superface
    /// class the last derive coloured, each entry naming the class index
    /// and the DISTINCT solids whose faces belong to it (a merged wall
    /// junction reports both wall names under one class). `faults` is the
    /// postcondition itself: same-facing, coplanar, genuinely overlapping
    /// face pairs whose labels are NOT bit-identical — any plane, no eye
    /// band, no crease threshold. It should always be empty; an entry in
    /// it names a real defect, never a stale read. An empty `faults`
    /// always means "no faults": a census that could not run is refused
    /// with the one-key `unavailable` grammar, never reported empty.
    #[func]
    fn explain_oids(&self) -> VarDictionary {
        let level = match self.live_level() {
            Ok(level) => level,
            Err(reason) => return unavailable(reason),
        };
        let bound = level.bind();
        let painted = bound.oid_census();
        let boxes: Vec<_> = painted.iter().map(|solid| solid.area).collect();
        let ids: Vec<f64> = painted.iter().map(|solid| solid.oid).collect();
        let names: Vec<&str> = painted.iter().map(|solid| solid.name.as_str()).collect();
        let Some(explanation) = explain_oids_checked(&boxes, &ids) else {
            // unreachable by construction (the census carries one id per
            // box), and still refused rather than reported: a truncated
            // check that found no violations is a vacuous pass
            return unavailable("the level's painted boxes and their ids do not line up");
        };
        let census = bound.face_census();
        let faces: Vec<Face> = census.iter().map(|entry| entry.face.clone()).collect();
        let labels: Vec<f64> = census.iter().map(|entry| entry.label).collect();
        let Some(faults) = coplanar_label_faults(&faces, &labels) else {
            // the same impossible misalignment, refused the same way: an
            // empty faults array must always mean "no faults", never
            // "could not check"
            return unavailable("the level's face census and its labels do not line up");
        };
        oid_dict(&explanation, &faults, census, &names, &ids)
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
    /// An id always comes back, and what cannot be answered is answered
    /// AT ONCE with a refusal rather than promised to a frame: a request
    /// whose numbers could only ever produce infinities, and an observer
    /// standing in no physics world, are both known here — and a caller
    /// left polling `pending` forever could not tell either of them from a
    /// frame that simply has not run yet.
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
        let checked = match CheckedReflectionRequest::prepare(request) {
            Ok(checked) => checked,
            Err(error) => return self.explanations.refuse(error.reason()),
        };
        let id = self.explanations.request(checked);
        if self.space_state().is_none() {
            self.explanations.answer(id, Answer::Refused(NO_SPACE));
        }
        id
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
    /// Prove that this observer reads the exact graph the restorer will
    /// write. Capture itself may legitimately omit the camera, but none of
    /// the three state-owning handles may be absent, freed, or point into a
    /// different game root.
    pub(super) fn validate_restore_graph(
        &self,
        expected_level: &Gd<WaveLevel>,
        expected_player: &Gd<UnseeingPlayer>,
        expected_body: &Gd<HeroBody>,
    ) -> Result<(), String> {
        let level = self.live_level().map_err(str::to_string)?;
        if level.instance_id() != expected_level.instance_id() {
            return Err("observer level is not the restorer's exact level".to_string());
        }
        let player = self.player.as_ref().ok_or_else(|| NO_HERO.to_string())?;
        if !player.is_instance_valid() {
            return Err(DEAD_HERO.to_string());
        }
        if player.instance_id() != expected_player.instance_id() {
            return Err("observer hero is not the restorer's exact hero".to_string());
        }
        let body = self
            .live_body()
            .map_err(|reason| format!("observer body: {reason}"))?;
        if body.instance_id() != expected_body.instance_id() {
            return Err("observer body is not the restorer's exact hero body".to_string());
        }
        Ok(())
    }

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

    /// The hero's body, under the same rule again. Only the capture asks:
    /// a snapshot has never carried the viewmodel, so an observer that was
    /// never handed a body still snapshots exactly as it always did.
    fn live_body(&self) -> Result<&Gd<HeroBody>, &'static str> {
        match self.body.as_ref() {
            None => Err(NO_BODY),
            Some(body) if !body.is_instance_valid() => Err(DEAD_BODY),
            Some(body) => Ok(body),
        }
    }

    /// The hero group, if a live, fully-built hero was injected. `None` —
    /// which the snapshot names in `unknown` — covers never-injected,
    /// freed, and a player whose camera has not been built yet: a pitch
    /// invented for an eyeless hero would be a guess, and the group is
    /// all-or-nothing like the capture blob it feeds.
    ///
    /// The reason is DROPPED here and kept by [`Self::read_hero`], because
    /// the two callers need different things from the same fetch: a
    /// snapshot names the group in `unknown` and reports the world around
    /// it, while a capture refuses the whole blob and must say which of
    /// the three absences it hit. One fetch, so they can never drift.
    fn hero_observation(&self) -> Option<HeroObservation> {
        self.read_hero().ok()
    }

    /// The hero group or the reason there is none.
    ///
    /// Validity is checked on the borrowed reference and the handle is
    /// never cloned at all, the same discipline `live_level`/`live_camera`
    /// use: cloning a `Gd<T>` for a freed instance panics rather than
    /// returning a dead handle, so a freed hero must be caught before any
    /// clone could happen, not after taking ownership of a copy.
    fn read_hero(&self) -> Result<HeroObservation, &'static str> {
        let player = self.player.as_ref().ok_or(NO_HERO)?;
        if !player.is_instance_valid() {
            return Err(DEAD_HERO);
        }
        let position = player.get_global_position();
        let velocity = player.get_velocity();
        let body_rotation = player.get_rotation();
        let yaw = GodotRotation::canonicalize_replacing_yaw(body_rotation, body_rotation.y)
            .map_err(|_| "the hero body does not preserve its configured X/Z rotation")?
            .world()
            .y;
        let bound = player.bind();
        let eye_rotation = bound.eye_rotation().ok_or(NO_EYE)?;
        let pitch = GodotRotation::canonicalize_replacing_pitch(eye_rotation, eye_rotation.x)
            .map_err(|_| "the hero eye does not preserve its configured Y/Z rotation")?
            .world()
            .x;
        Ok(HeroObservation {
            position,
            velocity,
            yaw: f64::from(yaw),
            pitch: f64::from(pitch),
            last_tap: bound.last_tap,
            tap_target: bound.tap_target,
            tap_queued: bound.tap_queued(),
            queued_waves: bound.wave_queue(),
        })
    }

    /// Assemble the blob, refusing at the FIRST subsystem that cannot
    /// answer. The order is cheapest-first and deliberate: the env group
    /// is the caller's own dictionary and needs no node at all, the pool
    /// and the echo book leave one borrow of one core, and the hero — the
    /// group with three separate ways to be absent — is asked before the
    /// cats, which are the only group whose length varies.
    ///
    /// THE CAMERA IS NOT FETCHED, and that is not an oversight. A snapshot
    /// refuses without one because `walls_to_eye` and the camera group are
    /// measured FROM the eye; a blob carries neither, and refusing for a
    /// subsystem the artifact does not contain would be a refusal that
    /// misnames its own limits.
    ///
    /// Visible to the whole `nodes` module because the RESTORER's proof is
    /// a second capture of the world it just wrote — through THIS function,
    /// never a copy of it. A restore that proved itself against its own
    /// idea of what a capture is would prove nothing at all.
    pub(super) fn capture_state(
        &self,
        now: f64,
        env: &VarDictionary,
    ) -> Result<CaptureState, String> {
        let env = parse_env(env)?;
        if env.now.to_bits() != now.to_bits() {
            return Err(format!(
                "{BAD_ENV}now — the capture is dated {now} and the env group says {}",
                env.now
            ));
        }
        let level = self.live_level()?;
        // read through the handle before binding it: `scene_file_path` is
        // a Node property, and the restore refuses a blob from another map
        let level_scene = level.get_scene_file_path().to_string();
        let level = level.bind();
        let Some(core) = pulse_core(&level) else {
            return Err(NO_POOL.to_string());
        };
        // one bind, so the pool and the echo book are read from the same
        // core at the same instant rather than from two borrows of it
        let (slots, echoes) = {
            let core = core.bind();
            (core.capture_pool(), core.capture_echoes())
        };
        let sources = capture_sources(&level)?;
        let hero = self.capture_hero()?;
        let cats = capture_cats(&level)?;
        Ok(CaptureState {
            format_version: FORMAT_VERSION,
            level_scene,
            env,
            slots,
            echoes,
            sources,
            hero,
            cats,
        })
    }

    /// The hero as the blob carries it: the snapshot's own hero fetch,
    /// refusing rather than omitting, plus the viewmodel — which comes off
    /// the BODY, and is the one fact in the whole blob that no snapshot has
    /// ever been able to reach.
    fn capture_hero(&self) -> Result<HeroCapture, &'static str> {
        let player_handle = self.player.as_ref().ok_or(NO_HERO)?;
        if !player_handle.is_instance_valid() {
            return Err(DEAD_HERO);
        }
        if !player_handle.is_physics_processing() {
            return Err(DISABLED_HERO);
        }
        let body = self.live_body()?;
        let viewmodel = body.bind().capture_vm().ok_or(NO_VM)?;
        let hero = self.read_hero()?;
        let player = player_handle.bind();
        Ok(HeroCapture {
            position: hero.position,
            velocity: hero.velocity,
            motion: player.motion_state(),
            yaw: hero.yaw,
            pitch: hero.pitch,
            last_tap: hero.last_tap,
            tap_target: hero.tap_target,
            tap_queued: hero.tap_queued,
            queued_waves: hero.queued_waves,
            footstep_suppression_pending: player.footstep_suppression_pending(),
            viewmodel,
        })
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
    request: &CheckedReflectionRequest,
    space: Gd<PhysicsDirectSpaceState3D>,
) -> Result<ReflectionExplanation, &'static str> {
    let (rays_cast, hits) = cast_reflection_fan(request, space)?;
    reflect::explain_clustering(request, rays_cast, &hits)
}

/// The one refusal shape. A dictionary carrying only this key is how an
/// agent learns it asked a question that could not be answered — as
/// opposed to one whose answer happens to be empty.
///
/// Shared with the restorer: a refused restore and a refused snapshot read
/// the same way to the agent holding them, and two spellings of "no" is
/// one more than a caller can be asked to parse.
pub(super) fn unavailable(reason: &str) -> VarDictionary {
    let mut refusal = VarDictionary::new();
    refusal.set("unavailable", reason);
    refusal
}

/// The Rust wave core behind whatever the level was injected with: the
/// `WaveCore` itself, upcast to `RefCounted`. In shipped code the handle is
/// the core directly; in `game/tests/` the handle is the GDScript `Pulses`
/// shim, and this function reaches through its public `core()` accessor.
///
/// Visible to the whole `nodes` module because the RESTORER writes through
/// the same handle the observer reads through: two ways of finding one
/// pool is exactly how a restore ends up writing into a core nobody is
/// rendering from.
pub(super) fn pulse_core(level: &WaveLevel) -> Option<Gd<WaveCore>> {
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
fn sources(
    level: &WaveLevel,
    eye: Vector3,
    occluders: &[crate::sight::Occluder],
    props: &[crate::sight::Occluder],
) -> Result<Vec<SourceObservation>, String> {
    level
        .source_handles()
        .iter()
        .map(|source| {
            if !source.is_instance_valid() {
                return Err("a level source has been freed — snapshot refused".to_string());
            }
            let node = source.clone().into_gd();
            let name = node.get_name().to_string();
            let bound = source.dyn_bind();
            let voice = bound.voice();
            let position = bound.hub();
            let line = ray::explain_ray(eye, position, occluders, props);
            Ok(SourceObservation {
                name,
                position,
                volume: voice.volume.amplitude(),
                reach: voice.volume.reach(),
                cadence: voice.cadence,
                next_emit: bound.next_emit().unwrap_or(f64::NAN),
                walls_to_eye: line.camera_crossings,
                source_volume: standing_image(&node, source::VOLUME_PARAM).unwrap_or(f64::NAN),
                source_muffle: standing_image(&node, source::MUFFLE_PARAM).unwrap_or(f64::NAN),
                slot_pressure: voice.slot_pressure(),
            })
        })
        .collect()
}

/// Every source's appointment book, in scene order — the only mutable
/// state a source carries. Everything else about it is designer-authored
/// and already in the scene, which is why the blob names sources rather
/// than describing them: the restore finds them BY NAME, never by index.
///
/// A source with no appointment refuses the whole blob. It is not a
/// harmless zero: restoring it would leave the gate to book a fresh date
/// off the restored clock, and the level would sound one wave that the
/// original never made.
fn capture_sources(level: &WaveLevel) -> Result<Vec<SourceCapture>, String> {
    level
        .source_handles()
        .iter()
        .map(|source| {
            if !source.is_instance_valid() {
                return Err("a level source has been freed — capture refused".to_string());
            }
            let name = source.clone().into_gd().get_name().to_string();
            let next_emit = source
                .dyn_bind()
                .next_emit()
                .ok_or_else(|| format!("{NO_APPOINTMENT} ({name})"))?;
            Ok(SourceCapture { name, next_emit })
        })
        .collect()
}

/// Every cat's whole life, in scene order — mind, stride, tail, pose and
/// the two clocks — through the one door [`super::cat::WaveCat`] opens.
/// The order is the blob's precondition, not a convenience: cats are
/// encoded and compared positionally.
fn capture_cats(level: &WaveLevel) -> Result<Vec<CatCapture>, String> {
    level
        .cat_handles()
        .iter()
        .map(|cat| {
            if !cat.is_instance_valid() {
                return Err("a level cat has been freed — capture refused".to_string());
            }
            let capture = cat
                .bind()
                .capture_state()
                .map_err(|reason| format!("{UNBUILT_CAT}: {reason} ({})", cat.get_name()))?;
            if !cat.is_physics_processing() || !cat.is_processing() {
                return Err(format!(
                    "cat {} has disabled processing — capture refused",
                    cat.get_name()
                ));
            }
            Ok(capture)
        })
        .collect()
}

/// One half of the standing acoustic image a source's limbs are actually
/// carrying — whichever instance uniform `param` names, read back off the
/// mesh the level pushed it to. Every limb of one source is pushed the same
/// value, so the first that answers speaks for the source. `None` before
/// any frame has driven it, which is a different fact from a value of zero
/// (a source muffled to silence).
///
/// Parameterised by name rather than split into two walkers because the two
/// halves are pushed together, to the same limbs, by the same call: one
/// missing and the other present would be a bug in `SourceRig::set_image`,
/// and the snapshot reports each independently so that bug is visible
/// rather than averaged away.
fn standing_image(node: &Gd<Node>, param: &str) -> Option<f64> {
    if let Ok(limb) = node.clone().try_cast::<MeshInstance3D>()
        && let Ok(value) = limb.get_instance_shader_parameter(param).try_to::<f64>()
    {
        return Some(value);
    }
    node.get_children()
        .iter_shared()
        .find_map(|child| standing_image(&child, param))
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
    // Two numbers, deliberately named apart: the loop bound the shaders
    // break at spans dead slots under live ones, so a reader that took it
    // for a census would see a saturated pool that is not there.
    state.set("slot_scan_limit", observation.slot_scan_limit as i64);
    state.set("live_slots", observation.live_slots as i64);
    let slots: Array<VarDictionary> = observation.slots.iter().map(slot_dict).collect();
    state.set("slots", &slots);
    state.set("next_eviction", &eviction_dict(&observation.next_eviction));
    let echoes: Array<VarDictionary> = observation.echoes.iter().map(echo_dict).collect();
    state.set("echoes", &echoes);
    let sources: Array<VarDictionary> = observation.sources.iter().map(source_dict).collect();
    for (index, source) in observation.sources.iter().enumerate() {
        if source.source_volume.is_nan() {
            unknown.push(&format!("sources[{index}].source_volume"));
        }
        if source.source_muffle.is_nan() {
            unknown.push(&format!("sources[{index}].source_muffle"));
        }
        if source.next_emit.is_nan() {
            unknown.push(&format!("sources[{index}].next_emit"));
        }
    }
    state.set("sources", &sources);
    state.set(
        "wall_rects",
        &PackedVector4Array::from(&observation.wall_rects[..]),
    );
    state.set("wall_truncated", observation.wall_truncated);
    state.set("camera", &camera_dict(&observation.eye));
    state.set("spawn", &spawn_dict(&observation.spawn));
    match &observation.hero {
        Some(hero) => {
            state.set("hero", &hero_dict(hero));
        }
        None => unknown.push("hero"),
    }
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

/// One scheduled reflection. `fires_in` is the whole point of reporting the
/// book rather than its length: an appointment sitting at a negative wait
/// is an echo the drain owed the world and has not paid.
fn echo_dict(echo: &EchoObservation) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("at_t", echo.at_t);
    entry.set("pos", echo.pos);
    entry.set("gain", echo.gain);
    entry.set("fires_in", echo.fires_in);
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
    entry.set("cadence", source.cadence);
    // the same NaN-means-absent rule as the standing image below: a gate that
    // cannot fire is holding an appointment it will never keep, and a date
    // that never arrives is worse than an admitted absence
    if !source.next_emit.is_nan() {
        entry.set("next_emit", source.next_emit);
    }
    entry.set("walls_to_eye", i64::from(source.walls_to_eye));
    // NaN is the "never pushed" marker set by `standing_image`, and the one
    // value the uniform can never legitimately hold: the key is left out
    // and named in the snapshot's `unknown` instead of reported as a guess.
    if !source.source_volume.is_nan() {
        entry.set("source_volume", source.source_volume);
    }
    if !source.source_muffle.is_nan() {
        entry.set("source_muffle", source.source_muffle);
    }
    entry.set("slot_pressure", source.slot_pressure);
    entry
}

/// Where the eye stands, where it looks, and how wide. A Godot camera
/// looks down its own -Z, so the heading is the negated third basis column;
/// `fov` is the vertical angle in degrees, without which a reader cannot
/// work out what should be on screen at all.
fn camera_dict(eye: &EyeObservation) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("position", eye.position);
    entry.set("forward", -eye.basis.col_c());
    entry.set("fov", eye.fov);
    entry
}

/// Where the hero woke — the landmark every other world coordinate in the
/// snapshot is read against.
fn spawn_dict(spawn: &SpawnObservation) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("position", spawn.position);
    entry.set("yaw", spawn.yaw);
    entry
}

fn hero_dict(hero: &HeroObservation) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("position", hero.position);
    entry.set("velocity", hero.velocity);
    entry.set("yaw", hero.yaw);
    entry.set("pitch", hero.pitch);
    entry.set("last_tap", hero.last_tap);
    entry.set("tap_target", hero.tap_target);
    entry.set("tap_queued", hero.tap_queued);
    let queued: Array<VarDictionary> = hero.queued_waves.iter().map(queued_wave_dict).collect();
    entry.set("queued_waves", &queued);
    entry
}

/// Keyed exactly as the player's own `queued_waves` #[func] keys them
/// ("type", not "kind"), so a reader sees one vocabulary for one queue.
fn queued_wave_dict(wave: &QueuedWave) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("type", wave.kind);
    entry.set("at", wave.at);
    entry.set("max_r", wave.max_r);
    entry.set("speed", wave.speed);
    entry.set("gain", wave.gain);
    entry.set("echoes", wave.echoes);
    entry.set("normal", wave.normal);
    entry.set("gate", wave.gate.wire_name());
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
            entry.set("span", wall.span);
            entry.set("crossed", wall.crossed);
            entry.set("contains_origin", wall.contains_origin);
            entry
        })
        .collect();
    let mut entry = VarDictionary::new();
    entry.set("from", explanation.from);
    entry.set("to", explanation.to);
    entry.set("camera_crossings", i64::from(explanation.camera_crossings));
    entry.set("source_crossings", i64::from(explanation.source_crossings));
    entry.set("prop_crossings", i64::from(explanation.prop_crossings));
    entry.set("visible_air", explanation.visible_air);
    entry.set(
        "first_wall",
        explanation
            .first_wall
            .map_or(-1_i64, |i| i64::try_from(i).unwrap_or(-1)),
    );
    entry.set("wave_transmission", explanation.wave_transmission);
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

/// The touch graph, plus the census it was built over, plus the merge
/// law's own postcondition.
///
/// `names` and `oids` are parallel and complete, because the pairs alone are
/// not a census: a solid standing clear of everything appears in no pair at
/// all, and "which id did this thing actually get?" is the question that
/// follows "which seams are broken" — often about exactly that solid.
///
/// `census` is [`WaveLevel::face_census`]'s own output — `superfaces` and
/// `faults` are both read off it, never off `names`/`oids`, because those
/// two describe the SOLID-granularity census while `census` describes
/// individual FACES.
fn oid_dict(
    explanation: &OidExplanation,
    faults: &[LabelFault],
    census: &[FaceCensusEntry],
    names: &[&str],
    oids: &[f64],
) -> VarDictionary {
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
    let census_names: Array<GString> = names.iter().map(|&name| GString::from(name)).collect();
    let violations: Array<i64> = explanation
        .violations
        .iter()
        .map(|&index| index as i64)
        .collect();
    let mut entry = VarDictionary::new();
    entry.set("names", &census_names);
    entry.set("oids", &PackedFloat64Array::from(oids));
    entry.set("pairs", &pairs);
    entry.set("violations", &violations);
    entry.set("superfaces", &superfaces_array(census));
    entry.set("faults", &faults_array(faults, census));
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

/// Every superface class the last derive coloured, by the DISTINCT names
/// of the solids whose faces belong to it — a merged wall junction reports
/// both wall names under the one class they share. First-appearance
/// order, walked once over `census` (itself in deterministic scene
/// order), never a set's own iteration order.
fn superfaces_array(census: &[FaceCensusEntry]) -> VarArray {
    let mut members: Vec<Vec<&str>> = Vec::new();
    for entry in census {
        if entry.class >= members.len() {
            members.resize(entry.class + 1, Vec::new());
        }
        let bucket = &mut members[entry.class];
        if !bucket.contains(&entry.name.as_str()) {
            bucket.push(entry.name.as_str());
        }
    }
    let mut out = VarArray::new();
    for (class, names) in members.iter().enumerate() {
        let mut dict = VarDictionary::new();
        dict.set("class", class as i64);
        let members_arr: Array<GString> = names.iter().map(|&n| GString::from(n)).collect();
        dict.set("members", &members_arr);
        out.push(&dict.to_variant());
    }
    out
}

/// The postcondition itself, named: every same-facing, coplanar,
/// genuinely overlapping face pair whose labels disagree — always empty
/// on a healthy level.
fn faults_array(faults: &[LabelFault], census: &[FaceCensusEntry]) -> VarArray {
    let mut out = VarArray::new();
    for fault in faults {
        let mut entry = VarDictionary::new();
        entry.set("name_a", &face_name(census, fault.a));
        entry.set("name_b", &face_name(census, fault.b));
        entry.set("label_a", fault.label_a);
        entry.set("label_b", fault.label_b);
        out.push(&entry.to_variant());
    }
    out
}

/// The name of the solid face `index` belongs to. Total, for the same
/// reason as [`box_name`].
fn face_name(census: &[FaceCensusEntry], index: usize) -> GString {
    census.get(index).map_or_else(
        || GString::from(&format!("<unnamed face {index}>")),
        |entry| GString::from(entry.name.as_str()),
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

// ─────────────────────────────────────────────────────────────────────
// THE BLOB, WALKED TWICE
//
// `state_dict` below and `parse_blob` under it are twin walks over the
// SAME keys: one writes the dictionary Godot-side transports can hand to
// `JSON.stringify`, the other reads it back into a `CaptureState`. They
// are kept adjacent, one helper pair per group, because they are one wire
// format wearing two faces — and the RESTORER reads the second half of
// it, so a key written and never read is a field that restores as
// whatever the scene happened to be holding.
//
// The writer's half is COMPILER-ENFORCED, exactly as `reproduce::blob`'s
// encoder is: every helper opens with an exhaustive destructure and there
// is no `..` rest pattern anywhere in it, so a field added to any capture
// struct is an E0027 here before any test runs. The reader's half cannot
// be — it names keys, and a key it forgets to name is a field it silently
// drops — so the net under it is the round-trip test in
// `game/tests/restore_test.gd`: capture, `JSON.stringify`,
// `JSON.parse_string`, parse both ends, compare. A dropped field moves
// the state and nothing else, which is precisely what that test compares.
//
// ── THE WIRE IS JSON, AND JSON CANNOT CARRY WHAT THIS BOUNDARY HOLDS ──
//
// The blob's destination is a FILE, and the hash that validates it
// compares float BIT PATTERNS. Three losses were measured against this
// exact Godot build, and each of them silently breaks that:
//
//   1. `JSON.stringify` renders `Vector3`, `Vector4` and the packed
//      vector arrays through Godot's PRETTY-PRINTER — "(1.5, 2.5, 3.5)" —
//      and they come back Strings.
//   2. `JSON.parse_string` returns a FLOAT for every number, so a 64-bit
//      integer past 2^53 comes back corrupted without a word.
//   3. Godot's own float formatting is NOT round-trip exact even with
//      `full_precision`: 1/60 is written `0.016666666666666666` and read
//      back one ULP away, `-0.0` is written `0.0` and loses a sign the
//      hash counts, and NaN has no JSON spelling at all (`stringify`
//      substitutes null and warns).
//
// So the format's one rule is: **nothing that must survive exactly
// crosses as a JSON number.**
//
//   - EVERY float — the f64 fields and the f32 vector lanes alike —
//     crosses as decimal TEXT that Rust wrote and Rust reads. Rust's
//     `Display` is the shortest decimal that round-trips and its parser
//     is correctly rounded, so the pair is exact for every finite value,
//     for both signed zeros, for the infinities, and for the NaN a cat
//     that never beat carries. It also stays a number a human can read,
//     which a hex bit pattern would not.
//     The ONE deliberate narrowing: a NaN's sign and payload do not
//     survive — every NaN is written "NaN" and read back as the canonical
//     `f64::NAN`. That is chosen, not overlooked. The only NaN this state
//     can hold is the `unwrap_or(f64::NAN)` marker for "no appointment",
//     which is that exact bit pattern; and if some future field ever
//     carried a different one, the loss is LOUD rather than silent — the
//     state hash compares bit patterns, so the round-trip test names the
//     field instead of a restore quietly disagreeing with its blob.
//   - EVERY 64-bit integer crosses as text too: the cat's two PCG words
//     as 16 hex characters, the flicker's stream position as decimal,
//     the state hash as 16 hex characters.
//   - Small integers — a format version, a pulse kind, a footstep side,
//     an echo budget — stay JSON numbers. They are far inside 2^53 and
//     exact there.
//   - No Godot vector type appears anywhere. A vector is an array of
//     float text, and a fixed run of vectors an array of those.
//
// The reader is generous about integer spelling and about nothing else:
// an int field accepts an integral float (a blob may be hand-edited, and
// GDScript will happily write `3.0`), and everything else — a missing
// key, a wrong type, a fractional pulse kind, a short array, an
// unparseable word — is an error naming its dotted path. There is no
// default anywhere in it: a defaulted field is the vacuous pass this
// whole plan exists to prevent.
// ─────────────────────────────────────────────────────────────────────

/// A 64-bit word as the 16 hex characters JSON carries losslessly. Shared
/// with the restorer, whose verdict hands back the hash of the world it
/// restored: how a hash is spelled is part of the wire format, and a
/// second spelling of it would be a second format.
pub(super) fn hex64(word: u64) -> String {
    format!("{word:016x}")
}

/// A vector as bare float text. See the wire note: a `Vector3` left in
/// this dictionary reaches a file as the string "(1.5, 2.5, 3.5)".
fn v3_array(v: Vector3) -> VarArray {
    lane_array(&[v.x, v.y, v.z])
}

fn v4_array(v: Vector4) -> VarArray {
    lane_array(&[v.x, v.y, v.z, v.w])
}

/// Lanes are written and read at their REAL width — f32, the width
/// `Vector3` actually holds and the width the hash actually compares.
/// Widening to f64 on the way out would be a second representation to
/// keep in step, and a needlessly long one: the shortest decimal that
/// round-trips an f32 is much shorter than the one that round-trips the
/// f64 it widened to.
fn lane_array(lanes: &[f32]) -> VarArray {
    let mut out = VarArray::new();
    for &lane in lanes {
        out.push(&format!("{lane:?}").to_variant());
    }
    out
}

/// A run of vectors — a tail, a set of planted paws, a set of aims.
fn v3_list(nodes: &[Vector3]) -> VarArray {
    let mut out = VarArray::new();
    for &node in nodes {
        out.push(&v3_array(node).to_variant());
    }
    out
}

/// A run of dictionaries, as one UNTYPED array. Untyped on purpose: an
/// `Array[Dictionary]` and the plain `Array` that comes back out of
/// `JSON.parse_string` are different Godot types, and the parser has to
/// accept the blob whichever road it arrived by.
fn dict_list<T>(items: impl Iterator<Item = T>, one: impl Fn(T) -> VarDictionary) -> VarArray {
    let mut out = VarArray::new();
    for item in items {
        out.push(&one(item).to_variant());
    }
    out
}

/// The whole blob, hash included. The hash is over the STATE, not over
/// this dictionary: the bytes it is taken from are `reproduce::blob`'s
/// canonical ones, so how a float happens to be spelled on the wire can
/// never change whether two worlds are the same world.
fn state_dict(state: &CaptureState) -> VarDictionary {
    let CaptureState {
        format_version,
        level_scene,
        env,
        slots,
        echoes,
        sources,
        hero,
        cats,
    } = state;
    let mut blob = VarDictionary::new();
    blob.set("format_version", i64::from(*format_version));
    blob.set("level_scene", level_scene.as_str());
    blob.set("hash", hex64(state_hash(state)).as_str());
    blob.set("env", &env_dict(env, Floats::Text));
    blob.set("slots", &dict_list(slots.iter(), slot_capture_dict));
    blob.set("echoes", &dict_list(echoes.iter(), echo_capture_dict));
    blob.set("sources", &dict_list(sources.iter(), source_capture_dict));
    blob.set("hero", &hero_capture_dict(hero));
    blob.set("cats", &dict_list(cats.iter(), cat_capture_dict));
    blob
}

/// The env group has two spellings because capture receives it as live
/// Godot values while the blob carries text like every other group. Handed
/// back to the Rust composition root by [`WaveObserver::env_of`], it is
/// native values that owner can simply assign.
fn env_dict(env: &EnvCapture, floats: Floats) -> VarDictionary {
    let EnvCapture {
        now,
        demo_checked,
        demo_armed,
        demo_next,
        flicker_t,
        flicker_level,
        flicker_drop_until,
        flicker_next_drop,
        flicker_rng_state,
    } = env;
    let mut entry = VarDictionary::new();
    entry.set("now", &float_value(*now, floats));
    entry.set("demo_checked", *demo_checked);
    entry.set("demo_armed", *demo_armed);
    entry.set("demo_next", &float_value(*demo_next, floats));
    entry.set("flicker_t", &float_value(*flicker_t, floats));
    entry.set("flicker_level", &float_value(*flicker_level, floats));
    entry.set(
        "flicker_drop_until",
        &float_value(*flicker_drop_until, floats),
    );
    entry.set(
        "flicker_next_drop",
        &float_value(*flicker_next_drop, floats),
    );
    // a stream POSITION, and all 64 bits of it are the value: as a JSON
    // number it would come back off a file as a float and lose its low
    // bits, which is a flicker that replays a different envelope
    entry.set(
        "flicker_rng_state",
        &match floats {
            Floats::Text => flicker_rng_state.to_string().to_variant(),
            Floats::Native => flicker_rng_state.to_variant(),
        },
    );
    entry
}

/// A float spelled for the road its dictionary is taking.
fn float_value(value: f64, floats: Floats) -> Variant {
    match floats {
        Floats::Native => value.to_variant(),
        Floats::Text => value.to_string().to_variant(),
    }
}

fn slot_capture_dict(slot: &SlotCapture) -> VarDictionary {
    let SlotCapture {
        pos,
        dat,
        dir,
        t0,
        end,
        kind,
    } = slot;
    let mut entry = VarDictionary::new();
    entry.set("pos", &v3_array(*pos));
    entry.set("dat", &v4_array(*dat));
    entry.set("dir", &v4_array(*dir));
    entry.set("t0", t0.to_string().as_str());
    entry.set("end", end.to_string().as_str());
    entry.set("kind", i64::from(*kind));
    entry
}

fn echo_capture_dict(echo: &PendingEcho) -> VarDictionary {
    let PendingEcho { at_t, pos, gain } = echo;
    let mut entry = VarDictionary::new();
    entry.set("at_t", at_t.to_string().as_str());
    entry.set("pos", &v3_array(*pos));
    entry.set("gain", gain.to_string().as_str());
    entry
}

fn source_capture_dict(source: &SourceCapture) -> VarDictionary {
    let SourceCapture { name, next_emit } = source;
    let mut entry = VarDictionary::new();
    entry.set("name", name.as_str());
    entry.set("next_emit", next_emit.to_string().as_str());
    entry
}

fn hero_capture_dict(hero: &HeroCapture) -> VarDictionary {
    let HeroCapture {
        position,
        velocity,
        motion,
        yaw,
        pitch,
        last_tap,
        tap_target,
        tap_queued,
        queued_waves,
        footstep_suppression_pending,
        viewmodel,
    } = hero;
    let mut entry = VarDictionary::new();
    entry.set("position", &v3_array(*position));
    entry.set("velocity", &v3_array(*velocity));
    entry.set("motion", &motion_dict(*motion));
    entry.set("yaw", yaw.to_string().as_str());
    entry.set("pitch", pitch.to_string().as_str());
    entry.set("last_tap", last_tap.to_string().as_str());
    entry.set("tap_target", &v3_array(*tap_target));
    entry.set("tap_queued", *tap_queued);
    entry.set(
        "queued_waves",
        &dict_list(queued_waves.iter(), queued_wave_capture_dict),
    );
    entry.set(
        "footstep_suppression_pending",
        *footstep_suppression_pending,
    );
    entry.set("viewmodel", &viewmodel_dict(viewmodel));
    entry
}

/// Keyed exactly as [`queued_wave_dict`] and the player's own
/// `queued_waves` #[func] key it — "type", not "kind" — so one queue reads
/// with one vocabulary wherever it surfaces. The encoding is the only
/// difference, and it is the whole reason this is a second function.
///
/// This is the wire spelling. `reproduce::blob::diff_wave`'s divergence
/// path names the same field by the `QueuedWave` struct's own field name,
/// "kind" — a reader chasing `hero.queued_waves[i].kind` into a blob file
/// wants this function's "type" key instead. Deliberate, not a typo either
/// side; see the note at `diff_wave`.
fn queued_wave_capture_dict(wave: &QueuedWave) -> VarDictionary {
    let QueuedWave {
        kind,
        at,
        max_r,
        speed,
        gain,
        echoes,
        normal,
        gate,
    } = wave;
    let mut entry = VarDictionary::new();
    entry.set("type", *kind);
    entry.set("at", &v3_array(*at));
    entry.set("max_r", max_r.to_string().as_str());
    entry.set("speed", speed.to_string().as_str());
    entry.set("gain", gain.to_string().as_str());
    entry.set("echoes", *echoes);
    entry.set("normal", &v3_array(*normal));
    entry.set("gate", gate.wire_name());
    entry
}

fn motion_dict(motion: MotionState) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("phase", &phase_dict(motion.phase()));
    entry.set(
        "support",
        &motion
            .support()
            .map_or_else(Variant::nil, |support| support_dict(support).to_variant()),
    );
    entry.set(
        "last_landing",
        &motion
            .last_landing()
            .map_or_else(Variant::nil, |landing| landing_dict(landing).to_variant()),
    );
    entry
}

fn phase_dict(phase: MotionPhase) -> VarDictionary {
    let mut entry = VarDictionary::new();
    match phase {
        MotionPhase::Controlled => entry.set("kind", "controlled"),
        MotionPhase::Airborne {
            planar_velocity_mps,
            vertical_velocity_mps,
        } => {
            entry.set("kind", "airborne");
            entry.set(
                "planar_velocity",
                &lane_array(&[planar_velocity_mps.x_mps(), planar_velocity_mps.z_mps()]),
            );
            entry.set(
                "vertical_velocity",
                format!("{:?}", vertical_velocity_mps.mps()).as_str(),
            );
        }
    }
    entry
}

fn support_dict(support: SupportContact) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set("point", &v3_array(support.point()));
    entry.set("normal", &v3_array(support.normal()));
    entry
}

fn landing_dict(landing: LandingEvent) -> VarDictionary {
    let mut entry = VarDictionary::new();
    entry.set(
        "impact_speed",
        format!("{:?}", landing.impact_speed().mps()).as_str(),
    );
    entry.set("support", &support_dict(landing.support()));
    entry
}

fn viewmodel_dict(viewmodel: &ViewmodelCapture) -> VarDictionary {
    let ViewmodelCapture {
        walk_amp,
        leg_phase,
        swing_phase,
        cane_swing,
        sway_x,
        sway_y,
        last_yaw,
        last_pitch,
        step_t,
        step_side,
    } = viewmodel;
    let mut entry = VarDictionary::new();
    entry.set("walk_amp", walk_amp.to_string().as_str());
    entry.set("leg_phase", leg_phase.to_string().as_str());
    entry.set("swing_phase", swing_phase.to_string().as_str());
    entry.set("cane_swing", cane_swing.to_string().as_str());
    entry.set("sway_x", sway_x.to_string().as_str());
    entry.set("sway_y", sway_y.to_string().as_str());
    entry.set("last_yaw", last_yaw.to_string().as_str());
    entry.set("last_pitch", last_pitch.to_string().as_str());
    entry.set("step_t", step_t.to_string().as_str());
    entry.set("step_side", i64::from(*step_side));
    entry
}

fn cat_capture_dict(cat: &CatCapture) -> VarDictionary {
    let CatCapture {
        position,
        yaw,
        velocity,
        motion,
        brain,
        gait,
        tail,
        pose,
        presence_next,
        sit,
        sim_t,
        last_pos,
    } = cat;
    let mut entry = VarDictionary::new();
    entry.set("position", &v3_array(*position));
    entry.set("yaw", yaw.to_string().as_str());
    entry.set("velocity", &v3_array(*velocity));
    entry.set("motion", &motion_dict(*motion));
    entry.set("brain", &brain_dict(brain));
    entry.set("gait", &gait_dict(gait));
    entry.set("tail", &v3_list(tail));
    entry.set("pose", &pose_dict(pose));
    // the cat that never beat carries NaN, which JSON cannot spell as a
    // number at all — and needs no special case here, because every float
    // in the blob is already text and Rust spells this one "NaN"
    entry.set("presence_next", presence_next.to_string().as_str());
    entry.set("sit", sit.to_string().as_str());
    entry.set("sim_t", sim_t.to_string().as_str());
    entry.set("last_pos", &v3_array(*last_pos));
    entry
}

fn brain_dict(brain: &BrainCapture) -> VarDictionary {
    let BrainCapture {
        rng_state,
        rng_inc,
        rect,
        state,
        yaw,
        speed,
        blocked,
    } = brain;
    let mut entry = VarDictionary::new();
    // the two PCG words: 64 bits each, and a restored cat whose stream is
    // one bit off diverges at its very next whim
    entry.set("rng_state", hex64(*rng_state).as_str());
    entry.set("rng_inc", hex64(*rng_inc).as_str());
    entry.set("rect", &rect_dict(rect));
    entry.set("state", &brain_state_dict(*state));
    entry.set("yaw", yaw.to_string().as_str());
    entry.set("speed", speed.to_string().as_str());
    entry.set("blocked", blocked.to_string().as_str());
    entry
}

fn rect_dict(rect: &RoamRect) -> VarDictionary {
    let RoamRect {
        min_x,
        min_z,
        max_x,
        max_z,
    } = rect;
    let mut entry = VarDictionary::new();
    entry.set("min_x", min_x.to_string().as_str());
    entry.set("min_z", min_z.to_string().as_str());
    entry.set("max_x", max_x.to_string().as_str());
    entry.set("max_z", max_z.to_string().as_str());
    entry
}

/// The mind's state machine on the wire: a spelled-out `kind` and that
/// variant's own payload. Spelled out rather than derived from `Debug`,
/// for the same reason [`state_name`] is — a parser is a contract, and a
/// rename of the enum must not silently break it. The names and the
/// payload keys are part of the format; changing one is a
/// [`crate::reproduce::FORMAT_VERSION`] bump.
fn brain_state_dict(state: BrainState) -> VarDictionary {
    let mut entry = VarDictionary::new();
    match state {
        BrainState::Roam { tx, tz } => {
            entry.set("kind", "Roam");
            entry.set("tx", tx.to_string().as_str());
            entry.set("tz", tz.to_string().as_str());
        }
        BrainState::Pause { left } => {
            entry.set("kind", "Pause");
            entry.set("left", left.to_string().as_str());
        }
        BrainState::Sit { left } => {
            entry.set("kind", "Sit");
            entry.set("left", left.to_string().as_str());
        }
    }
    entry
}

fn gait_dict(gait: &GaitCapture) -> VarDictionary {
    let GaitCapture {
        phase,
        amp,
        support_y,
        planted,
        aim,
        in_swing,
        moving,
    } = gait;
    let mut entry = VarDictionary::new();
    entry.set("phase", phase.to_string().as_str());
    entry.set("amp", amp.to_string().as_str());
    entry.set("support_y", format!("{support_y:?}").as_str());
    entry.set("planted", &v3_list(planted));
    entry.set("aim", &v3_list(aim));
    let mut swinging = VarArray::new();
    for &leg in in_swing {
        swinging.push(&leg.to_variant());
    }
    entry.set("in_swing", &swinging);
    entry.set("moving", *moving);
    entry
}

fn pose_dict(pose: &CatPose) -> VarDictionary {
    let CatPose {
        pos,
        yaw,
        paws,
        bob,
        amp,
        sit,
    } = pose;
    let mut entry = VarDictionary::new();
    entry.set("pos", &v3_array(*pos));
    entry.set("yaw", yaw.to_string().as_str());
    entry.set("paws", &v3_list(paws));
    entry.set("bob", bob.to_string().as_str());
    entry.set("amp", amp.to_string().as_str());
    entry.set("sit", sit.to_string().as_str());
    entry
}

/// How the dictionary being read spells its floats.
///
/// Two dictionaries reach this parser and only one of them has ever been
/// near JSON. The blob has: it is written to a file, so every float in it
/// is text (see the wire note). The env group `UnseeingGame::capture_env`
/// hands to [`WaveObserver::capture`] has NOT: it is passed straight
/// across the boundary in the same process, so its floats are real Godot
/// floats and rendering them as text at the Godot boundary would be the very
/// rounding this format exists to avoid.
#[derive(Clone, Copy)]
enum Floats {
    /// Real Godot floats, exact because they never left the process.
    Native,
    /// Decimal text Rust wrote and Rust reads back.
    Text,
}

/// One refused field, in the codec's one diagnostic grammar: the word
/// `field`, the dotted path, then what went wrong there.
///
/// Every refusal in this codec is built HERE rather than at its own
/// `format!`, and the reason is the boot gate (`test/ci_boot_error_gate.sh`,
/// `ci/boot_error_pattern.sh`). That gate censuses the opening of every
/// diagnostic under `rust/src/` by reading string literals, and bans a
/// message that builds its opening token out of an interpolation — a
/// `format!` whose string opens on the path itself, colon and all,
/// conjures an opening no census can read, so it cannot be checked
/// against `BOOT_ERROR_PATTERN` at all. (That ban is checked by grepping
/// this tree, so the shape it forbids cannot be written out here either,
/// not even inside a comment.) The opening is therefore a literal, and
/// there is exactly one of it, because a convention repeated at seventeen
/// sites is a convention nothing enforces. The gate cannot enforce it: its
/// reader is line-based, so it cannot see a `format!` whose string sits on
/// the next line, and its regex wants the colon to follow the
/// interpolation immediately, so an indexed path — an interpolation, a
/// bracketed element, THEN the colon — walks past it in plain sight.
///
/// `field` is an ordinary word and deliberately NOT a class name. A
/// refused blob field is a value handed back to the caller that asked for
/// it — the same category as `WaveCore`'s refused wave request, which that
/// gate documents as one it has no business failing on — not a node
/// refusing to boot half-wired. `BOOT_ERROR_PATTERN` stays the list of
/// classes that do.
fn fault(path: &str, complaint: &str) -> String {
    format!("field {path}: {complaint}")
}

/// One dictionary being read, how it spells its floats, and the dotted
/// path it sits at.
///
/// Every reader below reports `"field <path>.<key>: <what went wrong>"`
/// (see [`fault`]), because a parser that only says "malformed" hands its
/// reader a five-kilobyte file and a shrug. The dictionary is held by
/// handle (Godot's own refcounted one), so nesting into a group costs
/// nothing.
struct Group {
    dict: VarDictionary,
    floats: Floats,
    path: String,
}

impl Group {
    fn new(dict: &VarDictionary, floats: Floats, path: String) -> Self {
        Self {
            dict: dict.clone(),
            floats,
            path,
        }
    }

    fn path_of(&self, key: &str) -> String {
        if self.path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{key}", self.path)
        }
    }

    fn raw(&self, key: &str) -> Result<Variant, String> {
        self.dict
            .get(key)
            .ok_or_else(|| fault(&self.path_of(key), "missing"))
    }

    fn bool(&self, key: &str) -> Result<bool, String> {
        let value = self.raw(key)?;
        if value.get_type() == VariantType::BOOL {
            Ok(value.to::<bool>())
        } else {
            Err(fault(&self.path_of(key), "expected a bool"))
        }
    }

    fn f64(&self, key: &str) -> Result<f64, String> {
        self.float_of(&self.raw(key)?, &self.path_of(key))
    }

    /// One scalar carried at the engine's real f32 width. Parsing directly
    /// to f32 is load-bearing: widening through f64 can choose a different
    /// rounding boundary, and both zero signs are captured state.
    fn f32(&self, key: &str) -> Result<f32, String> {
        let path = self.path_of(key);
        let value = self.lane_of(&self.raw(key)?, &path)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(fault(&path, "must be finite"))
        }
    }

    /// A float, however this dictionary spells them. NaN, both zeros and
    /// the infinities all survive the text road exactly, which is why
    /// there is no special case for the one field that can hold a NaN.
    fn float_of(&self, value: &Variant, path: &str) -> Result<f64, String> {
        match self.floats {
            Floats::Native => number(value, path),
            Floats::Text => {
                let text = string_of(value, path)?;
                text.parse::<f64>()
                    .map(canonical_nan)
                    .map_err(|_| fault(path, &format!("expected a float as text, found {text:?}")))
            }
        }
    }

    /// One vector lane, at its real f32 width.
    fn lane_of(&self, value: &Variant, path: &str) -> Result<f32, String> {
        match self.floats {
            Floats::Native => Ok(number(value, path)? as f32),
            Floats::Text => {
                let text = string_of(value, path)?;
                text.parse::<f32>()
                    .map(canonical_nan_f32)
                    .map_err(|_| fault(path, &format!("expected a float as text, found {text:?}")))
            }
        }
    }

    /// An integer. A float is accepted iff it is EXACTLY integral — a
    /// hand-edited blob or a GDScript caller may well write `3.0` — and
    /// refused otherwise: a fractional pulse kind is a corrupt blob, not
    /// a roundable one.
    fn i64(&self, key: &str) -> Result<i64, String> {
        let value = self.raw(key)?;
        let path = self.path_of(key);
        match value.get_type() {
            VariantType::INT => Ok(value.to::<i64>()),
            VariantType::FLOAT => {
                let found = value.to::<f64>();
                let whole = found.trunc();
                if found != whole || !(-SAFE_INT..=SAFE_INT).contains(&whole) {
                    return Err(fault(
                        &path,
                        &format!("expected a whole number, found {found}"),
                    ));
                }
                Ok(whole as i64)
            }
            _ => Err(fault(&path, "expected an integer")),
        }
    }

    fn string(&self, key: &str) -> Result<String, String> {
        string_of(&self.raw(key)?, &self.path_of(key))
    }

    /// A 64-bit word as EXACTLY 16 hex characters — see the wire note
    /// above. `u64::from_str_radix` alone would also take 1-15 digits and a
    /// leading `+`; the only producer is `format!("{word:016x}")`
    /// ([`hex64`]), which never emits either, so both are refused rather
    /// than silently zero-extended.
    fn u64_hex(&self, key: &str) -> Result<u64, String> {
        let text = self.string(key)?;
        let malformed = || {
            fault(
                &self.path_of(key),
                &format!("expected 16 hex characters, found {text:?}"),
            )
        };
        if text.len() != 16 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(malformed());
        }
        u64::from_str_radix(&text, 16).map_err(|_| malformed())
    }

    /// A 64-bit signed word as decimal text, for the same reason.
    fn i64_text(&self, key: &str) -> Result<i64, String> {
        let text = self.string(key)?;
        text.parse::<i64>().map_err(|_| {
            fault(
                &self.path_of(key),
                &format!("expected a 64-bit integer as text, found {text:?}"),
            )
        })
    }

    fn group(&self, key: &str) -> Result<Group, String> {
        group_of(&self.raw(key)?, self.floats, self.path_of(key))
    }

    fn optional_group(&self, key: &str) -> Result<Option<Group>, String> {
        let value = self.raw(key)?;
        let path = self.path_of(key);
        match value.get_type() {
            VariantType::NIL => Ok(None),
            VariantType::DICTIONARY => Ok(Some(group_of(&value, self.floats, path)?)),
            _ => Err(fault(&path, "expected null or a dictionary")),
        }
    }

    /// The elements of an array, its length pinned when the format fixes
    /// one. A short run is a truncated blob, never a smaller world.
    fn array(&self, key: &str, expect: Option<usize>) -> Result<Vec<Variant>, String> {
        elements(&self.raw(key)?, expect, &self.path_of(key))
    }

    fn v3(&self, key: &str) -> Result<Vector3, String> {
        self.vector3(&self.raw(key)?, &self.path_of(key))
    }

    fn planar_velocity(&self, key: &str) -> Result<[f32; 2], String> {
        let path = self.path_of(key);
        let values = elements(&self.raw(key)?, Some(2), &path)?;
        let mut lanes = [0.0; 2];
        for (index, value) in values.iter().enumerate() {
            let lane_path = format!("{path}[{index}]");
            let lane = self.lane_of(value, &lane_path)?;
            if !lane.is_finite() {
                return Err(fault(&lane_path, "must be finite"));
            }
            lanes[index] = lane;
        }
        Ok(lanes)
    }

    fn v4(&self, key: &str) -> Result<Vector4, String> {
        let lanes = self.lanes(&self.raw(key)?, 4, &self.path_of(key))?;
        Ok(Vector4::new(lanes[0], lanes[1], lanes[2], lanes[3]))
    }

    fn vector3(&self, value: &Variant, path: &str) -> Result<Vector3, String> {
        let lanes = self.lanes(value, 3, path)?;
        Ok(Vector3::new(lanes[0], lanes[1], lanes[2]))
    }

    fn lanes(&self, value: &Variant, count: usize, path: &str) -> Result<Vec<f32>, String> {
        elements(value, Some(count), path)?
            .iter()
            .enumerate()
            .map(|(index, lane)| self.lane_of(lane, &format!("{path}[{index}]")))
            .collect()
    }

    /// A fixed-arity run of vectors: a tail, a set of paws, a set of aims.
    fn v3_array<const N: usize>(&self, key: &str) -> Result<[Vector3; N], String> {
        let items = self.array(key, Some(N))?;
        let path = self.path_of(key);
        let mut out = [Vector3::ZERO; N];
        for (index, item) in items.iter().enumerate() {
            out[index] = self.vector3(item, &format!("{path}[{index}]"))?;
        }
        Ok(out)
    }

    fn bool_array<const N: usize>(&self, key: &str) -> Result<[bool; N], String> {
        let items = self.array(key, Some(N))?;
        let path = self.path_of(key);
        let mut out = [false; N];
        for (index, item) in items.iter().enumerate() {
            if item.get_type() != VariantType::BOOL {
                return Err(fault(&format!("{path}[{index}]"), "expected a bool"));
            }
            out[index] = item.to::<bool>();
        }
        Ok(out)
    }
}

/// The largest magnitude an f64 can hold every integer up to — the point
/// past which "this float is a whole number" stops meaning "this float is
/// that integer".
const SAFE_INT: f64 = 9_007_199_254_740_992.0;

/// Every NaN spells "NaN" and reads back as THE NaN, so the bit pattern
/// the state hash compares is the one the capture carried rather than
/// whichever quiet NaN the parser happened to build.
fn canonical_nan(value: f64) -> f64 {
    if value.is_nan() { f64::NAN } else { value }
}

/// [`canonical_nan`]'s f32 sibling, for [`Group::lane_of`]. No vector lane
/// can hold a NaN today — the round-trip test would name any field that
/// ever did, loudly — but the scalar road canonicalizes on principle, and
/// the lane road should not be the one place that silently didn't.
fn canonical_nan_f32(value: f32) -> f32 {
    if value.is_nan() { f32::NAN } else { value }
}

fn string_of(value: &Variant, path: &str) -> Result<String, String> {
    if value.get_type() == VariantType::STRING {
        Ok(value.to::<GString>().to_string())
    } else {
        Err(fault(path, "expected a string"))
    }
}

fn group_of(value: &Variant, floats: Floats, path: String) -> Result<Group, String> {
    if value.get_type() == VariantType::DICTIONARY {
        Ok(Group {
            dict: value.to::<VarDictionary>(),
            floats,
            path,
        })
    } else {
        Err(fault(&path, "expected a dictionary"))
    }
}

fn elements(value: &Variant, expect: Option<usize>, path: &str) -> Result<Vec<Variant>, String> {
    if value.get_type() != VariantType::ARRAY {
        return Err(fault(path, "expected an array"));
    }
    let items: Vec<Variant> = value.to::<VarArray>().iter_shared().collect();
    if let Some(count) = expect
        && items.len() != count
    {
        return Err(fault(
            path,
            &format!("expected {count} entries, found {}", items.len()),
        ));
    }
    Ok(items)
}

/// A number as a Godot Variant holds it. Only the env group the caller
/// passes in reaches this: everything in the blob itself is text.
fn number(value: &Variant, path: &str) -> Result<f64, String> {
    match value.get_type() {
        VariantType::FLOAT => Ok(value.to::<f64>()),
        VariantType::INT => Ok(value.to::<i64>() as f64),
        _ => Err(fault(path, "expected a number")),
    }
}

/// A blob back into a state, or the dotted path of the first thing wrong
/// with it. Total, and defaulting NOTHING: the restorer writes every field
/// this returns straight into the running world, so a field quietly
/// defaulted here is a subsystem restored to whatever the scene happened
/// to be holding — which would then pass the hash gate, because the
/// re-capture would read back the same default.
///
/// The blob's own `hash` key is deliberately NOT checked here. It is
/// metadata about the artifact, not a field of the state, and a parser
/// that rejected a hash mismatch would turn a TAMPERED blob into "parse
/// error" — where the restore proof turns it into the name of the exact
/// field that disagreed, which is the difference between a shrug and a
/// bug report.
pub(super) fn parse_blob(dict: &VarDictionary) -> Result<CaptureState, String> {
    let blob = Group::new(dict, Floats::Text, String::new());
    let version = blob.i64("format_version")?;
    let format_version = u32::try_from(version).map_err(|_| {
        fault(
            "format_version",
            &format!("expected a small positive integer, found {version}"),
        )
    })?;
    let level_scene = blob.string("level_scene")?;
    let env = parse_env_group(&blob.group("env")?)?;
    let slots = parse_slots(&blob)?;
    let echoes = parse_run(&blob, "echoes", parse_echo)?;
    let sources = parse_run(&blob, "sources", parse_source)?;
    let hero = parse_hero(&blob.group("hero")?)?;
    let cats = parse_run(&blob, "cats", parse_cat)?;
    Ok(CaptureState {
        format_version,
        level_scene,
        env,
        slots,
        echoes,
        sources,
        hero,
        cats,
    })
}

/// A variable-length run of groups, each parsed under its own indexed
/// path — `"cats[1].brain.rng_state: missing"`.
fn parse_run<T>(
    blob: &Group,
    key: &str,
    one: impl Fn(&Group) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    let items = blob.array(key, None)?;
    let path = blob.path_of(key);
    items
        .iter()
        .enumerate()
        .map(|(index, item)| one(&group_of(item, blob.floats, format!("{path}[{index}]"))?))
        .collect()
}

/// The env group as the BLOB carries it — the stream position as text.
fn parse_env_group(group: &Group) -> Result<EnvCapture, String> {
    let flicker_rng_state = group.i64_text("flicker_rng_state")?;
    parse_env_fields(group, flicker_rng_state)
}

/// The env group as the CALLER hands it to [`WaveObserver::capture`]: the
/// same nine fields, but composed as live Godot values and passed straight across
/// the boundary rather than through a file — so the floats are real
/// floats, the stream position is still a plain int, and the refusal
/// wears the [`BAD_ENV`] grammar rather than a dotted path, because the
/// reader who has to fix it is looking at `UnseeingGame::capture_env`.
fn parse_env(env: &VarDictionary) -> Result<EnvCapture, String> {
    let group = Group::new(env, Floats::Native, String::new());
    let flicker_rng_state = group.i64("flicker_rng_state").map_err(bad_env)?;
    parse_env_fields(&group, flicker_rng_state).map_err(bad_env)
}

fn bad_env(reason: String) -> String {
    format!("{BAD_ENV}{reason}")
}

/// The eight fields both roads spell the same way. The ninth is passed
/// in, because it is the one they do not.
fn parse_env_fields(group: &Group, flicker_rng_state: i64) -> Result<EnvCapture, String> {
    Ok(EnvCapture {
        now: group.f64("now")?,
        demo_checked: group.bool("demo_checked")?,
        demo_armed: group.bool("demo_armed")?,
        demo_next: group.f64("demo_next")?,
        flicker_t: group.f64("flicker_t")?,
        flicker_level: group.f64("flicker_level")?,
        flicker_drop_until: group.f64("flicker_drop_until")?,
        flicker_next_drop: group.f64("flicker_next_drop")?,
        flicker_rng_state,
    })
}

/// All 64 slots, the arity pinned by the pool's own contract. A blob with
/// 63 is not a smaller pool, it is a truncated file.
fn parse_slots(blob: &Group) -> Result<Box<[SlotCapture; MAXP]>, String> {
    let items = blob.array("slots", Some(MAXP))?;
    let mut slots = Box::new(
        [SlotCapture {
            pos: Vector3::ZERO,
            dat: Vector4::ZERO,
            dir: Vector4::ZERO,
            t0: 0.0,
            end: 0.0,
            kind: 0,
        }; MAXP],
    );
    for (index, item) in items.iter().enumerate() {
        let group = group_of(item, blob.floats, format!("slots[{index}]"))?;
        slots[index] = parse_slot(&group)?;
    }
    Ok(slots)
}

fn parse_slot(group: &Group) -> Result<SlotCapture, String> {
    Ok(SlotCapture {
        pos: group.v3("pos")?,
        dat: group.v4("dat")?,
        dir: group.v4("dir")?,
        t0: group.f64("t0")?,
        end: group.f64("end")?,
        kind: i32::try_from(group.i64("kind")?)
            .map_err(|_| fault(&group.path_of("kind"), "out of range for a pulse kind"))?,
    })
}

fn parse_echo(group: &Group) -> Result<PendingEcho, String> {
    Ok(PendingEcho {
        at_t: group.f64("at_t")?,
        pos: group.v3("pos")?,
        gain: group.f64("gain")?,
    })
}

fn parse_source(group: &Group) -> Result<SourceCapture, String> {
    Ok(SourceCapture {
        name: group.string("name")?,
        next_emit: group.f64("next_emit")?,
    })
}

fn parse_hero(group: &Group) -> Result<HeroCapture, String> {
    Ok(HeroCapture {
        position: group.v3("position")?,
        velocity: group.v3("velocity")?,
        motion: parse_motion(&group.group("motion")?)?,
        yaw: group.f64("yaw")?,
        pitch: group.f64("pitch")?,
        last_tap: group.f64("last_tap")?,
        tap_target: group.v3("tap_target")?,
        tap_queued: group.bool("tap_queued")?,
        queued_waves: parse_run(group, "queued_waves", parse_wave)?,
        footstep_suppression_pending: group.bool("footstep_suppression_pending")?,
        viewmodel: parse_viewmodel(&group.group("viewmodel")?)?,
    })
}

fn parse_wave(group: &Group) -> Result<QueuedWave, String> {
    Ok(QueuedWave {
        kind: group.i64("type")?,
        at: group.v3("at")?,
        max_r: group.f64("max_r")?,
        speed: group.f64("speed")?,
        gain: group.f64("gain")?,
        echoes: group.i64("echoes")?,
        normal: group.v3("normal")?,
        gate: parse_queued_gate(group)?,
    })
}

fn parse_queued_gate(group: &Group) -> Result<QueuedWaveGate, String> {
    let gate = group.string("gate")?;
    match gate.as_str() {
        "always" => Ok(QueuedWaveGate::Always),
        "controlled_contact" => Ok(QueuedWaveGate::ControlledContact),
        other => Err(fault(
            &group.path_of("gate"),
            &format!("unknown queued-wave gate {other:?}"),
        )),
    }
}

fn parse_motion(group: &Group) -> Result<MotionState, String> {
    let phase = parse_phase(&group.group("phase")?)?;
    let support = group
        .optional_group("support")?
        .map(|support| parse_support(&support))
        .transpose()?;
    let last_landing = group
        .optional_group("last_landing")?
        .map(|landing| parse_landing(&landing))
        .transpose()?;
    MotionState::restore(phase, support, last_landing).map_err(|error| {
        let path = match error.field() {
            "motion_state.support" => group.path_of("support"),
            "motion_phase.vertical_velocity_mps" => group.path_of("phase.vertical_velocity"),
            _ => group.path.clone(),
        };
        fault(&path, "is inconsistent with the motion state")
    })
}

fn parse_phase(group: &Group) -> Result<MotionPhase, String> {
    let kind = group.string("kind")?;
    match kind.as_str() {
        "controlled" => Ok(MotionPhase::Controlled),
        "airborne" => {
            let planar = group.planar_velocity("planar_velocity")?;
            let planar_velocity_mps = PlanarVelocity::try_new(planar[0], planar[1])
                .map_err(|error| fault(&group.path, &error.to_string()))?;
            let vertical = group.f32("vertical_velocity")?;
            let vertical_velocity_mps = FiniteVelocity::try_new(vertical)
                .map_err(|error| fault(&group.path_of("vertical_velocity"), &error.to_string()))?;
            Ok(MotionPhase::Airborne {
                planar_velocity_mps,
                vertical_velocity_mps,
            })
        }
        other => Err(fault(
            &group.path_of("kind"),
            &format!("unknown motion phase {other:?}"),
        )),
    }
}

fn parse_support(group: &Group) -> Result<SupportContact, String> {
    let point = group.v3("point")?;
    let normal = group.v3("normal")?;
    for (key, value) in [("point", point), ("normal", normal)] {
        for (index, lane) in [value.x, value.y, value.z].into_iter().enumerate() {
            if !lane.is_finite() {
                return Err(fault(
                    &format!("{}[{index}]", group.path_of(key)),
                    "must be finite",
                ));
            }
        }
    }
    SupportContact::try_new(point, normal).map_err(|error| {
        let path = if error.field() == "support.normal" {
            group.path_of("normal")
        } else {
            group.path.clone()
        };
        fault(&path, "must be a nonzero vector")
    })
}

fn parse_landing(group: &Group) -> Result<LandingEvent, String> {
    let impact = group.f32("impact_speed")?;
    if impact < 0.0 {
        return Err(fault(
            &group.path_of("impact_speed"),
            "must be non-negative",
        ));
    }
    let support = parse_support(&group.group("support")?)?;
    LandingEvent::try_new(impact, support)
        .map_err(|error| fault(&group.path_of("impact_speed"), &error.to_string()))
}

fn parse_viewmodel(group: &Group) -> Result<ViewmodelCapture, String> {
    Ok(ViewmodelCapture {
        walk_amp: group.f64("walk_amp")?,
        leg_phase: group.f64("leg_phase")?,
        swing_phase: group.f64("swing_phase")?,
        cane_swing: group.f64("cane_swing")?,
        sway_x: group.f64("sway_x")?,
        sway_y: group.f64("sway_y")?,
        last_yaw: group.f64("last_yaw")?,
        last_pitch: group.f64("last_pitch")?,
        step_t: group.f64("step_t")?,
        step_side: i32::try_from(group.i64("step_side")?).map_err(|_| {
            fault(
                &group.path_of("step_side"),
                "out of range for a footstep side",
            )
        })?,
    })
}

fn parse_cat(group: &Group) -> Result<CatCapture, String> {
    Ok(CatCapture {
        position: group.v3("position")?,
        yaw: group.f64("yaw")?,
        velocity: group.v3("velocity")?,
        motion: parse_motion(&group.group("motion")?)?,
        brain: parse_brain(&group.group("brain")?)?,
        gait: parse_gait(&group.group("gait")?)?,
        tail: group.v3_array::<TAIL_N>("tail")?,
        pose: parse_pose(&group.group("pose")?)?,
        presence_next: group.f64("presence_next")?,
        sit: group.f64("sit")?,
        sim_t: group.f64("sim_t")?,
        last_pos: group.v3("last_pos")?,
    })
}

fn parse_brain(group: &Group) -> Result<BrainCapture, String> {
    Ok(BrainCapture {
        rng_state: group.u64_hex("rng_state")?,
        rng_inc: group.u64_hex("rng_inc")?,
        rect: parse_rect(&group.group("rect")?)?,
        state: parse_brain_state(&group.group("state")?)?,
        yaw: group.f64("yaw")?,
        speed: group.f64("speed")?,
        blocked: group.f64("blocked")?,
    })
}

fn parse_rect(group: &Group) -> Result<RoamRect, String> {
    Ok(RoamRect {
        min_x: group.f64("min_x")?,
        min_z: group.f64("min_z")?,
        max_x: group.f64("max_x")?,
        max_z: group.f64("max_z")?,
    })
}

fn parse_brain_state(group: &Group) -> Result<BrainState, String> {
    let kind = group.string("kind")?;
    match kind.as_str() {
        "Roam" => Ok(BrainState::Roam {
            tx: group.f64("tx")?,
            tz: group.f64("tz")?,
        }),
        "Pause" => Ok(BrainState::Pause {
            left: group.f64("left")?,
        }),
        "Sit" => Ok(BrainState::Sit {
            left: group.f64("left")?,
        }),
        other => Err(fault(
            &group.path_of("kind"),
            &format!("unknown brain state {other:?}"),
        )),
    }
}

fn parse_gait(group: &Group) -> Result<GaitCapture, String> {
    let planted = group.v3_array::<LEGS>("planted")?;
    let aim = group.v3_array::<LEGS>("aim")?;
    let support_y = group.f32("support_y")?;
    Ok(GaitCapture {
        phase: group.f64("phase")?,
        amp: group.f64("amp")?,
        support_y,
        planted,
        aim,
        in_swing: group.bool_array::<LEGS>("in_swing")?,
        moving: group.bool("moving")?,
    })
}

fn parse_pose(group: &Group) -> Result<CatPose, String> {
    Ok(CatPose {
        pos: group.v3("pos")?,
        yaw: group.f64("yaw")?,
        paws: group.v3_array::<LEGS>("paws")?,
        bob: group.f64("bob")?,
        amp: group.f64("amp")?,
        sit: group.f64("sit")?,
    })
}
