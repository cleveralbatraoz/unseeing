//! The companion cat as an engine node — the second creature the world
//! carries, and the first with a mind of its own. A designer drops a
//! WaveCat into a scene, picks a seed and a roam size, and the cat
//! lives: wandering its patch of floor, pausing, sitting, losing
//! interest in blocked paths — all of it deterministic under the seed,
//! replaying bit-for-bit.
//!
//! Like every creature here it is OUTLINE-ONLY, and revealed only while
//! waves sweep it. Its fore paws speak as it walks — soft kind-2 pulses,
//! the least precious slot class — so a walking cat paints its own faint
//! footprints of light and blinks its own body into outline: a little
//! walking lantern the blind hero can hear coming.
//!
//! All laws live in the pure modules — [`cat_brain`] decides,
//! [`cat_gait`] steps, [`cat_body`] poses — cargo-tested without a
//! Godot runtime. This file only carries values across the boundary:
//! physics context for movement and emission, immediate-mesh rebuilds
//! for the silhouette. The clock is handed, never poked: the composition
//! root will advance `tick(now)` like it does the player's.

use godot::classes::{
    ArrayMesh, CapsuleShape3D, CharacterBody3D, CollisionShape3D, Engine, ICharacterBody3D,
    Material, MeshInstance3D,
};
use godot::prelude::*;

use super::solid::clear_limbs;
use crate::cat_body::{self, CatPose, PreparedCatPose, PreparedTail, Tail};
use crate::cat_brain::{CatBrain, PreparedCatBrain, RoamRect};
use crate::cat_gait::{self, CatGait, PreparedCatGait};
use crate::limbs::{LimbBuf, sphere, sphere_lod, tube, tube_res};
use crate::render::{self, Role};
use crate::reproduce::RestoreValueError;
use crate::reproduce::blob::CatCapture;
use crate::sound_source::{Cadence, PreparedCadence};
use crate::support_motion::{
    ActorPosition, ActorTransform, ActorVelocity, ActorYaw, FiniteMeasure, FiniteRotation,
    GodotRotation, MotionState, MotionValueError, PosePoint, StepDuration,
};
use crate::temporal::{PreparedTime, prepare_time};

/// Collider radius — small enough to slip between furniture legs.
const COL_RADIUS: f32 = 0.11;

/// Collider height; its bottom floats a hair above the floor, like the
/// player's capsule — the flat map means nothing ever presses down.
const COL_HEIGHT: f32 = 0.34;

/// The sit blend's ease rate, 1/s — a cat settles, it does not snap.
const SIT_EASE: f64 = 3.0;

/// The two built limbs, named so a rebuilding ready() can free the ghosts a
/// Ctrl+D duplicate carries in (names are the only handle — a duplicate
/// reaches _ready as a fresh Rust object). Both the editor blueprint build
/// and the runtime build use these same two names.
const LIMBS: [&str; 2] = ["CatCollider", "CatSkin"];

/// A complete cat restore after every owner and every engine-width lane has
/// accepted it. The fields are private so the only write door can be the
/// assignment-only [`WaveCat::install_prepared`].
pub(super) struct PreparedCatState {
    position: Vector3,
    rotation: Vector3,
    velocity: Vector3,
    motion: MotionState,
    brain: CatBrain,
    gait: CatGait,
    tail: Tail,
    pose: CatPose,
    presence: Cadence,
    sit: f64,
    sim_t: f64,
    last_pos: Vector3,
    now: PreparedTime,
}

/// The companion cat. Inject `pulses` and `data_mat` before adding to
/// the tree (children run `_ready` first, and the cat refuses to build
/// uninjected); the seed and roam size are designer knobs.
#[derive(GodotClass)]
#[class(tool, init, base=CharacterBody3D)]
pub struct WaveCat {
    /// The wave pool every sound enters — the `WaveCore` itself, upcast to
    /// `RefCounted`. The GDScript `Pulses` shim survives only in
    /// `game/tests/`. The cat only asks it to `emit`, dynamically.
    #[var]
    pulses: Option<Gd<RefCounted>>,
    /// The data-pass material — the world is outline-only, and only
    /// this pass makes anything visible.
    #[var]
    data_mat: Option<Gd<Material>>,
    /// The whimsy seed: same seed, same life. Two cats want two seeds.
    #[export(range = (0.0, 999999.0))]
    #[init(val = 7)]
    seed: i64,
    /// Full extents of the floor rectangle the cat roams, centered on
    /// where it stands when it enters the tree.
    #[export(range = (1.0, 30.0, 0.5, suffix = " m"))]
    #[init(val = Vector2::new(6.0, 6.0))]
    roam_size: Vector2,
    #[init(val = ArrayMesh::new_gd())]
    mesh: Gd<ArrayMesh>,
    /// The frame's raw triangle geometry — cleared and refilled every
    /// rebuild rather than allocated fresh: `Vec::clear` keeps its
    /// capacity, so once this has grown to the cat's steady-state
    /// vertex count it never allocates again for the rest of its life.
    #[init(val = Vec::new())]
    tri_buf: LimbBuf,
    brain: Option<CatBrain>,
    gait: Option<CatGait>,
    tail: Option<Tail>,
    pose: Option<CatPose>,
    /// The idle-presence cadence gate — fires the cat's slow heartbeat
    /// pulse so a standing cat never sinks into full black.
    presence: Cadence,
    sit: f64,
    now: f64,
    sim_t: f64,
    /// The body position at the START of the last physics tick, before
    /// move_and_slide — so this tick's `pos - last_pos` is the planar
    /// distance the body ACTUALLY covered last tick, the brain's honest
    /// progress feed (never zero-across-the-wrong-interval).
    last_pos: Vector3,
    /// The pose changes only on a physics tick (60 Hz); this marks a
    /// fresh pose so `process()` rebuilds the silhouette once per tick,
    /// not once per rendered frame — no wasted rebuilds above 60 Hz.
    mesh_dirty: bool,
    #[init(val = MotionState::initial())]
    motion_state: MotionState,
    support_collider_id: Option<i64>,
    base: Base<CharacterBody3D>,
}

/// The complete runtime facts which must narrow successfully before a cat
/// constructs either a child or a pure owner.
struct PreparedCatReady {
    transform: ActorTransform,
    rotation: FiniteRotation,
    rect: RoamRect,
}

fn prepare_cat_ready(
    transform: Transform3D,
    rotation: Vector3,
    roam_size: Vector2,
) -> Result<PreparedCatReady, MotionValueError> {
    let transform = ActorTransform::try_new(transform)?;
    let rotation = FiniteRotation::try_new(rotation)?;
    let rect = RoamRect::try_around(transform.position(), roam_size)?;
    Ok(PreparedCatReady {
        transform,
        rotation,
        rect,
    })
}

/// The narrow physical capability used by the controlled cat tick. The
/// production adapter and the deterministic fault-injection fake execute the
/// same coordinator; no test-only copy of the boundary transaction exists.
trait CatMotionPort {
    fn read_global_transform(&mut self) -> Transform3D;
    fn read_global_rotation(&mut self) -> Vector3;
    fn read_velocity(&mut self) -> Vector3;
    fn write_global_rotation(&mut self, rotation: Vector3);
    fn write_velocity(&mut self, velocity: Vector3);
    fn move_and_slide_once(&mut self);
    fn write_global_transform(&mut self, transform: Transform3D);
    fn disable_processing(&mut self);
    fn emit_cat_wave(&mut self, at: Vector3, range: f64, gain: f64, now: f64);
}

impl CatMotionPort for WaveCat {
    fn read_global_transform(&mut self) -> Transform3D {
        self.base().get_global_transform()
    }

    fn read_global_rotation(&mut self) -> Vector3 {
        self.base().get_global_rotation()
    }

    fn read_velocity(&mut self) -> Vector3 {
        self.base().get_velocity()
    }

    fn write_global_rotation(&mut self, rotation: Vector3) {
        self.base_mut().set_global_rotation(rotation);
    }

    fn write_velocity(&mut self, velocity: Vector3) {
        self.base_mut().set_velocity(velocity);
    }

    fn move_and_slide_once(&mut self) {
        self.base_mut().move_and_slide();
    }

    fn write_global_transform(&mut self, transform: Transform3D) {
        self.base_mut().set_global_transform(transform);
    }

    fn disable_processing(&mut self) {
        self.base_mut().set_physics_process(false);
        self.base_mut().set_process(false);
    }

    fn emit_cat_wave(&mut self, at: Vector3, range: f64, gain: f64, now: f64) {
        self.emit_wave(at, range, gain, now);
    }
}

#[derive(Clone)]
struct CatControlledState {
    brain: CatBrain,
    gait: CatGait,
    tail: Tail,
    presence: Cadence,
    sit: f64,
    sim_t: f64,
    last_pos: Vector3,
}

struct CatTickSuccess {
    state: CatControlledState,
    pose: CatPose,
    frame: cat_gait::GaitFrame,
}

#[derive(Debug)]
struct CatTickFault {
    phase: &'static str,
    error: MotionValueError,
}

fn refuse_before_move<P: CatMotionPort>(
    port: &mut P,
    phase: &'static str,
    error: MotionValueError,
) -> CatTickFault {
    port.disable_processing();
    CatTickFault { phase, error }
}

fn rollback_after_move<P: CatMotionPort>(
    port: &mut P,
    saved_transform: Transform3D,
    phase: &'static str,
    error: MotionValueError,
) -> CatTickFault {
    // Order is part of the transaction: reinstate the complete body pose,
    // stop every commanded lane, then make the refusal inert.
    port.write_global_transform(saved_transform);
    port.write_velocity(Vector3::ZERO);
    port.disable_processing();
    CatTickFault { phase, error }
}

fn controlled_cat_tick<P: CatMotionPort>(
    port: &mut P,
    prior: &CatControlledState,
    raw_dt: f64,
    now: f64,
) -> Result<CatTickSuccess, CatTickFault> {
    let saved_transform = port.read_global_transform();
    let pre_transform = ActorTransform::try_new(saved_transform)
        .map_err(|error| refuse_before_move(port, "physics transform", error))?;
    let pre_rotation = FiniteRotation::try_new(port.read_global_rotation())
        .map_err(|error| refuse_before_move(port, "physics rotation", error))?;
    ActorVelocity::try_new(port.read_velocity())
        .map_err(|error| refuse_before_move(port, "physics velocity", error))?;
    let last_pos = ActorPosition::try_new(prior.last_pos)
        .map_err(|error| refuse_before_move(port, "physics prior position", error))?;
    if !prior.sit.is_finite() {
        return Err(refuse_before_move(
            port,
            "physics sit",
            MotionValueError::non_finite("cat.sit"),
        ));
    }
    if !(0.0..=1.0).contains(&prior.sit) {
        return Err(refuse_before_move(
            port,
            "physics sit",
            MotionValueError::out_of_range("cat.sit"),
        ));
    }
    FiniteMeasure::try_new(prior.sim_t, "cat.sim_t")
        .map_err(|error| refuse_before_move(port, "physics simulation time", error))?;
    let now = prepare_time(now)
        .map_err(|_| {
            let error = if now.is_finite() {
                MotionValueError::out_of_range("cat.now")
            } else {
                MotionValueError::non_finite("cat.now")
            };
            refuse_before_move(port, "physics clock", error)
        })?
        .value();
    // The raw engine callback is narrowed before any owner advances. Invalid,
    // negative and oversized values become the law's bounded zero/capped step.
    let step = StepDuration::from_raw(raw_dt);
    let pre_position = pre_transform.position();
    let progress = pre_position.planar_distance(last_pos);

    let mut next = prior.clone();
    let drive = next
        .brain
        .advance(step, pre_position, progress)
        .map_err(|error| refuse_before_move(port, "brain advance", error))?;

    // Ordinary motion preserves the live X/Z rotation lanes verbatim. The
    // restore-only GodotRotation canonicalizer must never repair this path.
    let mut commanded_rotation = pre_rotation.world();
    commanded_rotation.y = drive.yaw.godot_lane();
    FiniteRotation::try_new(commanded_rotation)
        .map_err(|error| refuse_before_move(port, "commanded rotation", error))?;
    let commanded_velocity = forward(drive.yaw.radians()) * (drive.speed.value() as f32);
    ActorVelocity::try_new(commanded_velocity)
        .map_err(|error| refuse_before_move(port, "commanded velocity", error))?;
    port.write_global_rotation(commanded_rotation);
    port.write_velocity(commanded_velocity);
    port.move_and_slide_once();

    let post_transform =
        ActorTransform::try_new(port.read_global_transform()).map_err(|error| {
            rollback_after_move(port, saved_transform, "post-move transform", error)
        })?;
    FiniteRotation::try_new(port.read_global_rotation())
        .map_err(|error| rollback_after_move(port, saved_transform, "post-move rotation", error))?;
    ActorVelocity::try_new(port.read_velocity())
        .map_err(|error| rollback_after_move(port, saved_transform, "post-move velocity", error))?;

    let post_position = post_transform.position();
    let moved = post_position.planar_distance(pre_position);
    let actual_speed = if step.seconds() > 0.0 {
        FiniteMeasure::try_new(moved.value() / step.seconds(), "cat.actual_speed")
            .map_err(|error| rollback_after_move(port, saved_transform, "post-move speed", error))?
    } else {
        FiniteMeasure::ZERO
    };
    let frame = next
        .gait
        .advance(step, post_position, drive.yaw, actual_speed)
        .map_err(|error| rollback_after_move(port, saved_transform, "gait advance", error))?;

    let next_sit = prior.sit
        + ((if drive.sitting { 1.0 } else { 0.0 }) - prior.sit)
            * (step.seconds() * SIT_EASE).min(1.0);
    let next_sim_t = prior.sim_t + step.seconds();
    let sway = 0.22 * (frame.phase * std::f64::consts::TAU).sin() * frame.amp
        + 0.10 * (next_sim_t * 0.9).sin() * (1.0 - frame.amp);
    let pose = CatPose::try_from_gait(post_position, drive.yaw, &frame, next_sit)
        .map_err(|error| rollback_after_move(port, saved_transform, "pose", error))?;
    let skeleton = cat_body::skeleton(&pose)
        .map_err(|error| rollback_after_move(port, saved_transform, "skeleton", error))?;
    let tail_root = PosePoint::try_new(skeleton.tail_root)
        .map_err(|error| rollback_after_move(port, saved_transform, "tail root", error))?;
    next.tail
        .transport_y(frame.support_delta_y)
        .map_err(|error| {
            rollback_after_move(port, saved_transform, "tail support transport", error)
        })?;
    next.tail
        .advance(
            step,
            tail_root,
            drive.yaw,
            post_position.elevation(),
            next_sit,
            sway,
        )
        .map_err(|error| rollback_after_move(port, saved_transform, "tail advance", error))?;

    next.sit = next_sit;
    next.sim_t = next_sim_t;
    // The next progress sample spans exactly this move: retain the position
    // sampled before move_and_slide, never its post-move endpoint.
    next.last_pos = pre_position.world();

    // All engine and pure outputs have now been checked. Only this final
    // section is permitted to mutate cadence or cross the wave boundary.
    for contact in frame
        .contacts
        .iter()
        .filter(|contact| cat_gait::paw_sounds(contact.leg))
    {
        port.emit_cat_wave(
            Vector3::new(contact.at.x, contact.at.y + 0.02, contact.at.z),
            cat_gait::PAW_RANGE,
            cat_gait::PAW_GAIN,
            now,
        );
    }
    if next.presence.beat(now).is_some() {
        let raw_post = post_position.world();
        port.emit_cat_wave(
            Vector3::new(
                raw_post.x,
                raw_post.y + cat_gait::PRESENCE_HEIGHT as f32,
                raw_post.z,
            ),
            cat_gait::PRESENCE_RANGE,
            cat_gait::PRESENCE_GAIN,
            now,
        );
    }

    Ok(CatTickSuccess {
        state: next,
        pose,
        frame,
    })
}

#[godot_api]
impl ICharacterBody3D for WaveCat {
    fn ready(&mut self) {
        clear_limbs(self, &LIMBS);
        if Engine::singleton().is_editor_hint() {
            // blueprint mode: one standing pose, frozen. The mesh is built
            // in LOCAL space (pose seeded at the origin) so the silhouette
            // rides the node when the designer drags it; the runtime mesh
            // stays world-space + top_level as before. No brain, no clock:
            // an editor-ticking cat would walk the viewport and Ctrl+S
            // would save its drift into the scene.
            self.base_mut().set_physics_process(false);
            self.base_mut().set_process(false);
            self.build_editor_pose();
            return;
        }
        // no silent nulls: without the pool and the data-pass material
        // the cat can neither sound nor be seen — refuse to build
        // instead of crashing later
        if self.pulses.is_none() || self.data_mat.is_none() {
            godot_error!("WaveCat: pulses/data_mat not injected — cat disabled");
            self.base_mut().set_physics_process(false);
            self.base_mut().set_process(false);
            return;
        }

        // Narrow every designer/scene-owned motion fact before constructing a
        // child or a mind. A poisoned transform must leave no half-built cat.
        let prepared = match prepare_cat_ready(
            self.base().get_global_transform(),
            self.base().get_global_rotation(),
            self.roam_size,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                self.disable_after_motion_error("ready inputs", error);
                return;
            }
        };
        let pos = prepared.transform.position();
        let raw_pos = pos.world();
        let yaw = prepared.rotation.yaw();

        let mut col = CollisionShape3D::new_alloc();
        col.set_name("CatCollider");
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(COL_RADIUS);
        capsule.set_height(COL_HEIGHT);
        col.set_shape(&capsule);
        col.set_position(Vector3::new(0.0, COL_HEIGHT * 0.5 + 0.02, 0.0));
        self.base_mut().add_child(&col);

        let mut mi = MeshInstance3D::new_alloc();
        mi.set_name("CatSkin");
        mi.set_mesh(&self.mesh.clone());
        mi.set_material_override(self.data_mat.as_ref());
        // one flat label for the whole cat: the outline post-pass draws it
        // as a single unified silhouette, never a pile of joint circles.
        // CUSTOM0 (baked below, every rebuild) is what the shader reads for
        // G directly — no per-instance bridge to keep in step any more.
        // the mesh mutates every frame in world space; never frustum-cull
        // it by its stale local bounds
        mi.set_extra_cull_margin(16384.0);
        mi.set_as_top_level(true);
        self.base_mut().add_child(&mi);

        // the brain, gait and mesh all work in WORLD space (the roam rect
        // and velocity are world), so the prepared world heading remains
        // correct under a rotated room or grouping folder.
        self.brain = Some(CatBrain::new(self.seed as u64, prepared.rect, yaw));
        let mut gait = match CatGait::new(pos, yaw) {
            Ok(gait) => gait,
            Err(error) => {
                self.brain = None;
                self.disable_after_motion_error("ready gait", error);
                return;
            }
        };
        let frame = match gait.advance(StepDuration::from_raw(0.0), pos, yaw, FiniteMeasure::ZERO) {
            Ok(frame) => frame,
            Err(error) => {
                self.brain = None;
                self.disable_after_motion_error("ready gait frame", error);
                return;
            }
        };
        let pose = match CatPose::try_from_gait(pos, yaw, &frame, 0.0) {
            Ok(pose) => pose,
            Err(error) => {
                self.brain = None;
                self.disable_after_motion_error("ready pose", error);
                return;
            }
        };
        let sk = match cat_body::skeleton(&pose) {
            Ok(skeleton) => skeleton,
            Err(error) => {
                self.brain = None;
                self.disable_after_motion_error("ready skeleton", error);
                return;
            }
        };
        let tail_root = match PosePoint::try_new(sk.tail_root) {
            Ok(root) => root,
            Err(error) => {
                self.brain = None;
                self.disable_after_motion_error("ready tail root", error);
                return;
            }
        };
        let tail = match Tail::new(tail_root, yaw, pos.elevation()) {
            Ok(tail) => tail,
            Err(error) => {
                self.brain = None;
                self.disable_after_motion_error("ready tail", error);
                return;
            }
        };
        self.tail = Some(tail);
        self.gait = Some(gait);
        self.pose = Some(pose);
        self.presence = Cadence::every(cat_gait::PRESENCE_EVERY);
        self.last_pos = raw_pos;
        // built HERE rather than left for the first process() tick: the
        // mesh's CUSTOM0 is the shader's own G-channel source now (no
        // per-instance uniform to carry the label in the meantime), so a
        // census or an observer reading this cat before a frame has ever
        // ticked must already find a real, painted silhouette.
        if let Err(error) = self.build_mesh(&pose, &tail) {
            self.brain = None;
            self.gait = None;
            self.tail = None;
            self.pose = None;
            self.disable_after_motion_error("ready mesh pose", error);
            return;
        }
        self.mesh_dirty = false;
    }

    fn physics_process(&mut self, dt: f64) {
        let (Some(brain), Some(gait), Some(tail)) = (self.brain, self.gait.as_ref(), self.tail)
        else {
            return; // _ready refused: nothing to think with
        };
        let prior = CatControlledState {
            brain,
            gait: gait.clone(),
            tail,
            presence: self.presence,
            sit: self.sit,
            sim_t: self.sim_t,
            last_pos: self.last_pos,
        };
        let now = self.now;
        let success = match controlled_cat_tick(self, &prior, dt, now) {
            Ok(success) => success,
            Err(fault) => {
                godot_error!("WaveCat: {} refused: {}", fault.phase, fault.error);
                return;
            }
        };

        let CatTickSuccess { state, pose, frame } = success;
        // The frame has already driven tail and voice commands inside the
        // transaction; consuming it here makes that one-frame output's
        // lifetime explicit for the future physical adapter.
        let _validated_frame = frame;
        self.pose = Some(pose);
        self.brain = Some(state.brain);
        self.gait = Some(state.gait);
        self.tail = Some(state.tail);
        self.presence = state.presence;
        self.sit = state.sit;
        self.sim_t = state.sim_t;
        self.last_pos = state.last_pos;
        self.mesh_dirty = true;
    }

    fn process(&mut self, _dt: f64) {
        if !self.mesh_dirty {
            return; // pose unchanged since the last rebuild — no wasted work
        }
        let (Some(pose), Some(tail)) = (self.pose, self.tail) else {
            return; // no physics tick yet: nothing to draw
        };
        if let Err(error) = self.build_mesh(&pose, &tail) {
            self.disable_after_motion_error("mesh pose", error);
            return;
        }
        self.mesh_dirty = false;
    }
}

#[godot_api]
impl WaveCat {
    /// The clock is handed, never poked: the composition root advances
    /// the simulated time here every frame, exactly like the player's.
    ///
    /// `pub(super)`: the root's own `process()` drives every cat's clock
    /// through a typed handle, the same precedent
    /// `UnseeingPlayer::tick`/`HeroBody::update` already set.
    #[func]
    pub(super) fn tick(&mut self, now_t: f64) {
        self.now = now_t;
    }

    /// Paw wave reach in meters — the voice constant, served as a static
    /// method: ClassDB registers integer constants only.
    #[func]
    fn paw_range() -> f64 {
        cat_gait::PAW_RANGE
    }

    /// Paw wavefront speed, m/s — static-method constant, same reason.
    #[func]
    fn paw_speed() -> f64 {
        cat_gait::PAW_SPEED
    }

    /// Paw wave loudness — static-method constant, same reason.
    #[func]
    fn paw_gain() -> f64 {
        cat_gait::PAW_GAIN
    }

    /// Idle-presence wave reach in meters — static-method constant.
    #[func]
    fn presence_range() -> f64 {
        cat_gait::PRESENCE_RANGE
    }

    /// Idle-presence loudness — static-method constant.
    #[func]
    fn presence_gain() -> f64 {
        cat_gait::PRESENCE_GAIN
    }

    /// Idle-presence cadence in seconds — static-method constant.
    #[func]
    fn presence_every() -> f64 {
        cat_gait::PRESENCE_EVERY
    }

    /// The four paw world positions, LF RF LH RH — the suites' observable.
    #[func]
    fn paw_positions(&self) -> PackedVector3Array {
        self.pose
            .map(|p| PackedVector3Array::from(&p.paws[..]))
            .unwrap_or_default()
    }

    /// The current mood as an integer: 0 roaming, 1 pausing, 2 sitting.
    #[func]
    fn mood(&self) -> i64 {
        use crate::cat_brain::Mood;
        match self.brain.as_ref().map(CatBrain::mood) {
            Some(Mood::Roam) => 0,
            Some(Mood::Pause) => 1,
            Some(Mood::Sit) => 2,
            None => -1,
        }
    }

    /// The silhouette's baked mesh — observable for mesh-sanity pins.
    #[func]
    fn cat_mesh(&self) -> Gd<ArrayMesh> {
        self.mesh.clone()
    }

    /// The cat as data, or the exact missing/noncanonical owner that makes a
    /// complete capture impossible.
    pub(crate) fn capture_state(&self) -> Result<CatCapture, &'static str> {
        let brain = self.brain.as_ref().ok_or("cat brain was never built")?;
        let gait = self.gait.as_ref().ok_or("cat gait was never built")?;
        let tail = self.tail.as_ref().ok_or("cat tail was never built")?;
        let pose = self.pose.as_ref().ok_or("cat pose was never built")?;
        let body_rotation = self.base().get_global_rotation();
        let yaw = GodotRotation::canonicalize_replacing_yaw(body_rotation, body_rotation.y)
            .map_err(|_| "cat body does not preserve its configured X/Z rotation")?
            .world()
            .y;
        Ok(CatCapture {
            position: self.base().get_global_position(),
            yaw: f64::from(yaw),
            velocity: self.base().get_velocity(),
            motion: self.motion_state,
            brain: brain.capture(),
            gait: gait.capture(),
            tail: *tail.nodes(),
            pose: *pose,
            presence_next: self.presence.next_at().unwrap_or(f64::NAN),
            sit: self.sit,
            sim_t: self.sim_t,
            last_pos: self.last_pos,
        })
    }

    /// Validate and narrow a captured cat without changing the node. Pure
    /// owners validate their own private state first; this boundary owner
    /// checks only engine widths, dormant capability and cross-owner
    /// lockstep that no one pure value can know by itself.
    #[expect(
        clippy::too_many_arguments,
        reason = "the adapter composes each independently prepared cat owner plus shared time"
    )]
    pub(super) fn prepare_restore(
        &self,
        capture: &CatCapture,
        brain: PreparedCatBrain,
        gait: PreparedCatGait,
        pose: PreparedCatPose,
        tail: PreparedTail,
        presence: PreparedCadence,
        now: PreparedTime,
    ) -> Result<PreparedCatState, RestoreValueError> {
        if !self.base().is_physics_processing() || !self.base().is_processing() {
            return Err(RestoreValueError::new("", "runtime processing is disabled"));
        }
        let position = ActorPosition::try_new(capture.position).map_err(|error| {
            RestoreValueError::new(
                format!("position.{}", terminal_field(error.field())),
                "must be finite and inside actor bounds",
            )
        })?;
        let velocity = ActorVelocity::try_new(capture.velocity).map_err(|error| {
            RestoreValueError::new(
                format!("velocity.{}", terminal_field(error.field())),
                "must be finite",
            )
        })?;
        if capture.motion != MotionState::initial() {
            return Err(RestoreValueError::new(
                "motion",
                "this runtime admits only initial controlled motion",
            ));
        }
        let rotation = prepare_cat_snapshot_links(
            self.base().get_global_rotation(),
            CatSnapshotLinks {
                body_yaw: capture.yaw,
                brain_yaw: capture.brain.yaw,
                pose_yaw: capture.pose.yaw,
                gait_amp: capture.gait.amp,
                pose_amp: capture.pose.amp,
                cat_sit: capture.sit,
                pose_sit: capture.pose.sit,
            },
        )?;

        for (axis, body, posed) in [
            ("x", capture.position.x, capture.pose.pos.x),
            ("y", capture.position.y, capture.pose.pos.y),
            ("z", capture.position.z, capture.pose.pos.z),
        ] {
            if body.to_bits() != posed.to_bits() {
                return Err(RestoreValueError::new(
                    format!("pose.pos.{axis}"),
                    "must match the captured body position bit-for-bit",
                ));
            }
        }
        if capture.position.y.to_bits() != capture.gait.support_y.to_bits() {
            return Err(RestoreValueError::new(
                "gait.support_y",
                "must match the captured body Y bit-for-bit",
            ));
        }
        if !capture.sit.is_finite() || !(0.0..=1.0).contains(&capture.sit) {
            return Err(RestoreValueError::new("sit", "must be finite and in 0..=1"));
        }
        if !capture.sim_t.is_finite() || capture.sim_t < 0.0 {
            return Err(RestoreValueError::new(
                "sim_t",
                "must be finite and non-negative",
            ));
        }
        let last_pos = ActorPosition::try_new(capture.last_pos).map_err(|error| {
            RestoreValueError::new(
                format!("last_pos.{}", terminal_field(error.field())),
                "must be finite and inside actor bounds",
            )
        })?;

        Ok(PreparedCatState {
            position: position.world(),
            rotation: rotation.world(),
            velocity: velocity.world(),
            motion: capture.motion,
            brain: CatBrain::from_prepared(brain),
            gait: CatGait::from_prepared(gait),
            tail: Tail::from_prepared(tail),
            pose: CatPose::from_prepared(pose),
            presence: Cadence::from_prepared(presence),
            sit: capture.sit,
            sim_t: capture.sim_t,
            last_pos: last_pos.world(),
            now,
        })
    }

    /// Consume a completely checked cat restore. There is deliberately no
    /// repair, narrowing or semantic branch after the transaction starts.
    pub(super) fn install_prepared(&mut self, value: PreparedCatState) {
        self.base_mut().set_global_position(value.position);
        self.base_mut().set_global_rotation(value.rotation);
        self.base_mut().set_velocity(value.velocity);
        self.motion_state = value.motion;
        self.support_collider_id = None;
        self.brain = Some(value.brain);
        self.gait = Some(value.gait);
        self.tail = Some(value.tail);
        self.pose = Some(value.pose);
        self.presence = value.presence;
        self.sit = value.sit;
        self.sim_t = value.sim_t;
        self.last_pos = value.last_pos;
        self.now = value.now.value();
        self.mesh_dirty = true;
    }

    /// One of the cat's own waves into the pool: kind 2 (footstep — the
    /// least precious slot class), omnidirectional, no reflections — a
    /// whisper that reveals the cat and a small circle of floor, not the
    /// room. Both the paw steps and the idle heartbeat speak through here,
    /// differing only in reach and loudness.
    fn emit_wave(&mut self, at: Vector3, range: f64, gain: f64, now: f64) {
        let Some(pulses) = self.pulses.as_mut() else {
            return; // unreachable past the _ready guard; total anyway
        };
        pulses.call(
            "emit",
            &[
                2_i64.to_variant(),
                at.to_variant(),
                range.to_variant(),
                cat_gait::PAW_SPEED.to_variant(),
                gain.to_variant(),
                now.to_variant(),
                Vector3::ZERO.to_variant(),
                (-2.0_f64).to_variant(),
            ],
        );
    }

    /// Blueprint mode: build the same two limbs the runtime path does, but
    /// in LOCAL space around the origin (no material, no top-level, no
    /// cull margin), then write one frozen standing pose into the mesh.
    /// The gait, tail and pose are thrown away the moment the mesh is
    /// written — the persistent `Option` fields stay `None`, exactly as
    /// they are for any node whose `_ready` refused to build, and nothing
    /// reads them because processing is disabled before this runs. No
    /// brain is built here at all: a frozen standing pose needs no roam
    /// decision to render.
    fn build_editor_pose(&mut self) {
        let mut col = CollisionShape3D::new_alloc();
        col.set_name("CatCollider");
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(COL_RADIUS);
        capsule.set_height(COL_HEIGHT);
        col.set_shape(&capsule);
        col.set_position(Vector3::new(0.0, COL_HEIGHT * 0.5 + 0.02, 0.0));
        self.base_mut().add_child(&col);

        let mut mi = MeshInstance3D::new_alloc();
        mi.set_name("CatSkin");
        mi.set_mesh(&self.mesh.clone());
        self.base_mut().add_child(&mi);

        let raw_pos = Vector3::ZERO;
        let raw_yaw = 0.0_f64;
        let Ok(pos) = ActorPosition::try_new(raw_pos) else {
            return;
        };
        let Ok(yaw) = ActorYaw::try_new(raw_yaw) else {
            return;
        };
        let Ok(mut gait) = CatGait::new(pos, yaw) else {
            return;
        };
        let Ok(frame) = gait.advance(StepDuration::from_raw(0.0), pos, yaw, FiniteMeasure::ZERO)
        else {
            return;
        };
        let Ok(pose) = CatPose::try_from_gait(pos, yaw, &frame, 0.0) else {
            return;
        };
        let Ok(sk) = cat_body::skeleton(&pose) else {
            return;
        };
        let Ok(root) = PosePoint::try_new(sk.tail_root) else {
            return;
        };
        let Ok(tail) = Tail::new(root, yaw, pos.elevation()) else {
            return;
        };
        let _ = self.build_mesh(&pose, &tail);
    }

    fn disable_after_motion_error(&mut self, phase: &str, error: MotionValueError) {
        godot_error!("WaveCat: {phase} refused: {error}");
        self.base_mut().set_physics_process(false);
        self.base_mut().set_process(false);
    }

    /// The whole silhouette, rebuilt for this frame's skeleton: torso
    /// line, neck and head, ears, whiskers, four bent legs, the tail
    /// chain — smooth tubes and spheres, one clean outline per shape.
    /// Small joints use the radius-tiered [`sphere_lod`] and whiskers the
    /// low-segment [`tube_res`]: the pea-sized parts read identically at a
    /// fraction of the per-vertex FFI cost the wasm build feels.
    ///
    /// Every vertex carries the SAME [`Role::Cat`] label in `CUSTOM0` — one
    /// silhouette, exactly what the shader's G channel reads for the whole
    /// mesh instance. `tri_buf` is cleared and refilled here rather than
    /// rebuilt fresh, so a cat that has been alive a few frames allocates
    /// nothing more to keep drawing itself.
    fn build_mesh(&mut self, pose: &CatPose, tail: &Tail) -> Result<(), MotionValueError> {
        let sk = cat_body::skeleton(pose)?;
        let label = render::role_label(Role::Cat) as f32;
        self.tri_buf.clear();
        // the torso line, chest proud of hip — the big shapes stay full-res
        tube(&mut self.tri_buf, sk.chest, sk.hip, 0.068, 0.062, label);
        sphere(&mut self.tri_buf, sk.chest, 0.072, label);
        sphere(&mut self.tri_buf, sk.hip, 0.068, label);
        // neck and head
        tube(&mut self.tri_buf, sk.chest, sk.head, 0.045, 0.034, label);
        sphere(&mut self.tri_buf, sk.head, 0.052, label);
        sphere_lod(&mut self.tri_buf, sk.muzzle, 0.028, label);
        for (base, tip) in sk.ears {
            tube(&mut self.tri_buf, base, tip, 0.016, 0.002, label);
        }
        for (root, tip) in sk.whiskers {
            tube_res(&mut self.tri_buf, root, tip, 0.0012, 0.0006, 4, label);
        }
        for leg in sk.legs {
            tube(&mut self.tri_buf, leg.root, leg.mid, 0.030, 0.024, label);
            sphere_lod(&mut self.tri_buf, leg.mid, 0.026, label);
            tube(&mut self.tri_buf, leg.mid, leg.paw, 0.024, 0.020, label);
            // the paw pad, seated ON the shin's end — no lift offset, so an
            // occluded far paw can't survive as a free-floating ball
            sphere_lod(&mut self.tri_buf, leg.paw, 0.021, label);
        }
        let mut prev = sk.tail_root;
        for (i, node) in tail.nodes().iter().enumerate() {
            let r1 = 0.014 - 0.0018 * i as f32;
            let r2 = 0.014 - 0.0018 * (i + 1) as f32;
            tube(&mut self.tri_buf, prev, *node, r1, r2, label);
            sphere_lod(&mut self.tri_buf, *node, r2 * 0.9, label);
            prev = *node;
        }
        render::paint::resize_triangle_surface(&mut self.mesh, &self.tri_buf);
        Ok(())
    }
}

fn terminal_field(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

#[derive(Clone, Copy)]
struct CatSnapshotLinks {
    body_yaw: f64,
    brain_yaw: f64,
    pose_yaw: f64,
    gait_amp: f64,
    pose_amp: f64,
    cat_sit: f64,
    pose_sit: f64,
}

fn prepare_cat_snapshot_links(
    current_full: Vector3,
    links: CatSnapshotLinks,
) -> Result<GodotRotation, RestoreValueError> {
    if links.pose_yaw.to_bits() != links.brain_yaw.to_bits() {
        return Err(RestoreValueError::new(
            "pose.yaw",
            "must match brain.yaw bit-for-bit",
        ));
    }
    if links.pose_amp.to_bits() != links.gait_amp.to_bits() {
        return Err(RestoreValueError::new(
            "pose.amp",
            "must match gait.amp bit-for-bit",
        ));
    }
    if links.pose_sit.to_bits() != links.cat_sit.to_bits() {
        return Err(RestoreValueError::new(
            "pose.sit",
            "must match the cat sit blend bit-for-bit",
        ));
    }
    let brain_lane = links.brain_yaw as f32;
    if !brain_lane.is_finite() {
        return Err(RestoreValueError::new(
            "brain.yaw",
            "must narrow to a finite Godot rotation lane",
        ));
    }
    let canonical =
        GodotRotation::canonicalize_replacing_yaw(current_full, brain_lane).map_err(|_| {
            RestoreValueError::new(
                "yaw",
                "brain yaw must preserve the live complete Godot YXZ X/Z configuration",
            )
        })?;
    if links.body_yaw.to_bits() != f64::from(canonical.world().y).to_bits() {
        return Err(RestoreValueError::new(
            "yaw",
            "must match the canonical f32 engine image of brain.yaw",
        ));
    }
    GodotRotation::try_replacing_yaw(current_full, links.body_yaw as f32).map_err(|_| {
        RestoreValueError::new(
            "yaw",
            "artifact yaw must already be canonical in the live complete rotation",
        )
    })
}

/// The heading's forward vector — Godot yaw convention: yaw 0 faces -Z.
fn forward(yaw: f64) -> Vector3 {
    Vector3::new((-yaw.sin()) as f32, 0.0, (-yaw.cos()) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum MotionTrace {
        ReadTransform,
        ReadRotation,
        ReadVelocity,
        SetRotation(Vector3),
        SetVelocity(Vector3),
        MoveAndSlide,
        SetTransform(Transform3D),
        Disable,
        EmitWave {
            at: Vector3,
            range: f64,
            gain: f64,
            now: f64,
        },
    }

    #[derive(Clone)]
    struct FakeCatMotionPort {
        pre_transform: Transform3D,
        pre_rotation: Vector3,
        pre_velocity: Vector3,
        post_transform: Transform3D,
        post_rotation: Vector3,
        post_velocity: Vector3,
        moved: bool,
        trace: Vec<MotionTrace>,
    }

    impl FakeCatMotionPort {
        fn valid() -> Self {
            let rotation = Vector3::new(0.125, -0.25, 0.0625);
            let transform = Transform3D::new(
                Basis::from_euler(EulerOrder::YXZ, rotation),
                Vector3::new(1.25, 0.75, -2.5),
            );
            Self {
                pre_transform: transform,
                pre_rotation: rotation,
                pre_velocity: Vector3::new(0.125, 0.0, -0.25),
                post_transform: Transform3D::new(
                    transform.basis,
                    Vector3::new(1.265_625, 0.75, -2.531_25),
                ),
                post_rotation: rotation,
                post_velocity: Vector3::new(0.125, 0.0, -0.25),
                moved: false,
                trace: Vec::new(),
            }
        }
    }

    impl CatMotionPort for FakeCatMotionPort {
        fn read_global_transform(&mut self) -> Transform3D {
            self.trace.push(MotionTrace::ReadTransform);
            if self.moved {
                self.post_transform
            } else {
                self.pre_transform
            }
        }

        fn read_global_rotation(&mut self) -> Vector3 {
            self.trace.push(MotionTrace::ReadRotation);
            if self.moved {
                self.post_rotation
            } else {
                self.pre_rotation
            }
        }

        fn read_velocity(&mut self) -> Vector3 {
            self.trace.push(MotionTrace::ReadVelocity);
            if self.moved {
                self.post_velocity
            } else {
                self.pre_velocity
            }
        }

        fn write_global_rotation(&mut self, rotation: Vector3) {
            self.trace.push(MotionTrace::SetRotation(rotation));
            self.pre_rotation = rotation;
        }

        fn write_velocity(&mut self, velocity: Vector3) {
            self.trace.push(MotionTrace::SetVelocity(velocity));
            if self.moved {
                self.post_velocity = velocity;
            } else {
                self.pre_velocity = velocity;
            }
        }

        fn move_and_slide_once(&mut self) {
            self.trace.push(MotionTrace::MoveAndSlide);
            self.moved = true;
        }

        fn write_global_transform(&mut self, transform: Transform3D) {
            self.trace.push(MotionTrace::SetTransform(transform));
            self.post_transform = transform;
        }

        fn disable_processing(&mut self) {
            self.trace.push(MotionTrace::Disable);
        }

        fn emit_cat_wave(&mut self, at: Vector3, range: f64, gain: f64, now: f64) {
            self.trace.push(MotionTrace::EmitWave {
                at,
                range,
                gain,
                now,
            });
        }
    }

    fn poison_transform_lane(value: Transform3D, lane: usize) -> Transform3D {
        let mut columns = [
            value.basis.col_a(),
            value.basis.col_b(),
            value.basis.col_c(),
        ];
        let mut origin = value.origin;
        if lane < 9 {
            let column = lane / 3;
            let row = lane % 3;
            match row {
                0 => columns[column].x = f32::NAN,
                1 => columns[column].y = f32::NAN,
                _ => columns[column].z = f32::NAN,
            }
        } else {
            match lane - 9 {
                0 => origin.x = f32::NAN,
                1 => origin.y = f32::NAN,
                _ => origin.z = f32::NAN,
            }
        }
        Transform3D::new(Basis::from_cols(columns[0], columns[1], columns[2]), origin)
    }

    fn poison_vector_lane(mut value: Vector3, lane: usize) -> Vector3 {
        match lane {
            0 => value.x = f32::NAN,
            1 => value.y = f32::NAN,
            _ => value.z = f32::NAN,
        }
        value
    }

    fn assert_transform_bits_eq(actual: Transform3D, expected: Transform3D) {
        for (actual_lane, expected_lane) in [
            (actual.basis.col_a().x, expected.basis.col_a().x),
            (actual.basis.col_a().y, expected.basis.col_a().y),
            (actual.basis.col_a().z, expected.basis.col_a().z),
            (actual.basis.col_b().x, expected.basis.col_b().x),
            (actual.basis.col_b().y, expected.basis.col_b().y),
            (actual.basis.col_b().z, expected.basis.col_b().z),
            (actual.basis.col_c().x, expected.basis.col_c().x),
            (actual.basis.col_c().y, expected.basis.col_c().y),
            (actual.basis.col_c().z, expected.basis.col_c().z),
            (actual.origin.x, expected.origin.x),
            (actual.origin.y, expected.origin.y),
            (actual.origin.z, expected.origin.z),
        ] {
            assert_eq!(actual_lane.to_bits(), expected_lane.to_bits());
        }
    }

    fn ordered_f32_bits(value: f32) -> u32 {
        let bits = value.to_bits();
        if bits & 0x8000_0000 == 0 {
            bits | 0x8000_0000
        } else {
            !bits
        }
    }

    fn controlled_state(port: &FakeCatMotionPort) -> CatControlledState {
        let pos = ActorPosition::try_new(port.pre_transform.origin).unwrap();
        let yaw = ActorYaw::try_new(f64::from(port.pre_rotation.y)).unwrap();
        let rect = RoamRect::try_around(pos, Vector2::new(6.0, 6.0)).unwrap();
        let brain = CatBrain::new(7, rect, yaw);
        let mut gait = CatGait::new(pos, yaw).unwrap();
        let frame = gait
            .advance(StepDuration::from_raw(0.0), pos, yaw, FiniteMeasure::ZERO)
            .unwrap();
        let pose = CatPose::try_from_gait(pos, yaw, &frame, 0.0).unwrap();
        let skeleton = cat_body::skeleton(&pose).unwrap();
        let tail = Tail::new(
            PosePoint::try_new(skeleton.tail_root).unwrap(),
            yaw,
            pos.elevation(),
        )
        .unwrap();
        CatControlledState {
            brain,
            gait,
            tail,
            presence: Cadence::every(cat_gait::PRESENCE_EVERY),
            sit: 0.0,
            sim_t: 0.0,
            last_pos: port.pre_transform.origin,
        }
    }

    fn assert_state_bits_eq(actual: &CatControlledState, expected: &CatControlledState) {
        assert_eq!(actual.brain.capture(), expected.brain.capture());
        assert_eq!(actual.gait.capture(), expected.gait.capture());
        assert_eq!(actual.tail.nodes(), expected.tail.nodes());
        assert_eq!(actual.presence.next_at(), expected.presence.next_at());
        assert_eq!(actual.sit.to_bits(), expected.sit.to_bits());
        assert_eq!(actual.sim_t.to_bits(), expected.sim_t.to_bits());
        for (actual_lane, expected_lane) in [
            (actual.last_pos.x, expected.last_pos.x),
            (actual.last_pos.y, expected.last_pos.y),
            (actual.last_pos.z, expected.last_pos.z),
        ] {
            assert_eq!(actual_lane.to_bits(), expected_lane.to_bits());
        }
    }

    #[test]
    fn cat_ready_rejects_poisoned_position_yaw_and_roam_size_before_brain_construction() {
        let port = FakeCatMotionPort::valid();
        let mut rejected = Vec::new();
        for lane in 0..12 {
            rejected.push(prepare_cat_ready(
                poison_transform_lane(port.pre_transform, lane),
                port.pre_rotation,
                Vector2::new(6.0, 6.0),
            ));
        }
        for lane in 0..3 {
            rejected.push(prepare_cat_ready(
                port.pre_transform,
                poison_vector_lane(port.pre_rotation, lane),
                Vector2::new(6.0, 6.0),
            ));
        }
        for roam_size in [
            Vector2::new(f32::NAN, 6.0),
            Vector2::new(6.0, f32::NAN),
            Vector2::new(0.5, 6.0),
            Vector2::new(6.0, 30.5),
        ] {
            rejected.push(prepare_cat_ready(
                port.pre_transform,
                port.pre_rotation,
                roam_size,
            ));
        }

        let mut brain_constructions = 0;
        for prepared in rejected {
            if prepared.is_ok() {
                brain_constructions += 1;
            }
        }
        assert_eq!(brain_constructions, 0);
    }

    #[test]
    fn cat_physics_refuses_clock_outside_renderer_horizon_before_owner_or_engine_advance() {
        let valid = FakeCatMotionPort::valid();
        let prior = controlled_state(&valid);

        for (now, problem) in [
            (-0.25, crate::support_motion::MotionValueProblem::OutOfRange),
            (
                262_144.000_000_000_06,
                crate::support_motion::MotionValueProblem::OutOfRange,
            ),
            (
                f64::INFINITY,
                crate::support_motion::MotionValueProblem::NonFinite,
            ),
            (
                f64::NAN,
                crate::support_motion::MotionValueProblem::NonFinite,
            ),
        ] {
            let mut port = valid.clone();
            let before = prior.clone();
            let Err(error) = controlled_cat_tick(&mut port, &prior, 1.0 / 60.0, now) else {
                panic!("an invalid renderer-visible clock must be refused");
            };

            assert_eq!(error.phase, "physics clock");
            assert_eq!(error.error.field(), "cat.now");
            assert_eq!(error.error.problem(), problem);
            assert_state_bits_eq(&prior, &before);
            assert_eq!(
                port.trace,
                [
                    MotionTrace::ReadTransform,
                    MotionTrace::ReadRotation,
                    MotionTrace::ReadVelocity,
                    MotionTrace::Disable,
                ],
                "clock {now:?} crossed the movement boundary"
            );
        }
    }

    #[test]
    fn cat_physics_rejects_poisoned_pre_or_post_move_sample_without_advancing_brain_gait_tail_or_waves()
     {
        let valid = FakeCatMotionPort::valid();
        let prior = controlled_state(&valid);

        for lane in 0..12 {
            let mut port = valid.clone();
            port.pre_transform = poison_transform_lane(port.pre_transform, lane);
            let before = prior.clone();
            let result = controlled_cat_tick(&mut port, &prior, 1.0 / 60.0, 2.0);
            assert!(result.is_err(), "pre transform lane {lane} was accepted");
            assert_state_bits_eq(&prior, &before);
            assert_eq!(
                port.trace,
                [MotionTrace::ReadTransform, MotionTrace::Disable]
            );
        }
        for lane in 0..3 {
            let mut port = valid.clone();
            port.pre_rotation = poison_vector_lane(port.pre_rotation, lane);
            let before = prior.clone();
            let result = controlled_cat_tick(&mut port, &prior, 1.0 / 60.0, 2.0);
            assert!(result.is_err(), "pre rotation lane {lane} was accepted");
            assert_state_bits_eq(&prior, &before);
            assert_eq!(
                port.trace,
                [
                    MotionTrace::ReadTransform,
                    MotionTrace::ReadRotation,
                    MotionTrace::Disable,
                ]
            );
        }
        for lane in 0..3 {
            let mut port = valid.clone();
            port.pre_velocity = poison_vector_lane(port.pre_velocity, lane);
            let before = prior.clone();
            let result = controlled_cat_tick(&mut port, &prior, 1.0 / 60.0, 2.0);
            assert!(result.is_err(), "pre velocity lane {lane} was accepted");
            assert_state_bits_eq(&prior, &before);
            assert_eq!(
                port.trace,
                [
                    MotionTrace::ReadTransform,
                    MotionTrace::ReadRotation,
                    MotionTrace::ReadVelocity,
                    MotionTrace::Disable,
                ]
            );
        }
        for lane in 0..3 {
            let mut poisoned_state = prior.clone();
            poisoned_state.last_pos = poison_vector_lane(poisoned_state.last_pos, lane);
            let before = poisoned_state.clone();
            let mut port = valid.clone();
            let result = controlled_cat_tick(&mut port, &poisoned_state, 1.0 / 60.0, 2.0);
            assert!(result.is_err(), "prior position lane {lane} was accepted");
            assert_state_bits_eq(&poisoned_state, &before);
            assert_eq!(
                port.trace,
                [
                    MotionTrace::ReadTransform,
                    MotionTrace::ReadRotation,
                    MotionTrace::ReadVelocity,
                    MotionTrace::Disable,
                ]
            );
        }

        for kind in 0..3 {
            let lanes = if kind == 0 { 12 } else { 3 };
            for lane in 0..lanes {
                let mut port = valid.clone();
                match kind {
                    0 => port.post_transform = poison_transform_lane(port.post_transform, lane),
                    1 => port.post_rotation = poison_vector_lane(port.post_rotation, lane),
                    _ => port.post_velocity = poison_vector_lane(port.post_velocity, lane),
                }
                let saved = port.pre_transform;
                let before = prior.clone();
                let result = controlled_cat_tick(&mut port, &prior, 1.0 / 60.0, 2.0);
                assert!(
                    result.is_err(),
                    "post sample kind {kind} lane {lane} was accepted"
                );
                assert_state_bits_eq(&prior, &before);
                let mut expected = vec![
                    MotionTrace::ReadTransform,
                    MotionTrace::ReadRotation,
                    MotionTrace::ReadVelocity,
                    MotionTrace::SetRotation(valid.pre_rotation),
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::MoveAndSlide,
                    MotionTrace::ReadTransform,
                ];
                if kind >= 1 {
                    expected.push(MotionTrace::ReadRotation);
                }
                if kind >= 2 {
                    expected.push(MotionTrace::ReadVelocity);
                }
                expected.extend([
                    MotionTrace::SetTransform(saved),
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::Disable,
                ]);
                assert_eq!(port.trace, expected);
                let rollback = &port.trace[port.trace.len() - 3..];
                let MotionTrace::SetTransform(restored) = rollback[0] else {
                    panic!("rollback must restore the saved full transform first");
                };
                assert_transform_bits_eq(restored, saved);
                assert_eq!(rollback[1], MotionTrace::SetVelocity(Vector3::ZERO));
                assert_eq!(rollback[2], MotionTrace::Disable);
            }
        }

        let mut successful_port = valid.clone();
        let success = controlled_cat_tick(&mut successful_port, &prior, 1.0 / 60.0, 2.0)
            .expect("a completely finite tick must commit");
        assert_eq!(
            successful_port
                .trace
                .iter()
                .filter(|event| **event == MotionTrace::MoveAndSlide)
                .count(),
            1
        );
        let emitted: Vec<Vector3> = successful_port
            .trace
            .iter()
            .filter_map(|event| match event {
                MotionTrace::EmitWave { at, .. } => Some(*at),
                _ => None,
            })
            .collect();
        assert!(
            emitted.iter().any(|at| {
                at.x.to_bits() == 1.265_625_f32.to_bits()
                    && at.y.to_bits() == 0.93_f32.to_bits()
                    && at.z.to_bits() == (-2.531_25_f32).to_bits()
            }),
            "the elevated presence voice must follow the post-move root"
        );
        for (actual, expected) in [
            (success.state.last_pos.x, valid.pre_transform.origin.x),
            (success.state.last_pos.y, valid.pre_transform.origin.y),
            (success.state.last_pos.z, valid.pre_transform.origin.z),
        ] {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
        let commanded_rotation = successful_port
            .trace
            .iter()
            .find_map(|event| match event {
                MotionTrace::SetRotation(rotation) => Some(*rotation),
                _ => None,
            })
            .expect("a successful tick must set its ordinary world yaw");
        assert_eq!(
            commanded_rotation.x.to_bits(),
            valid.pre_rotation.x.to_bits()
        );
        assert_eq!(
            commanded_rotation.z.to_bits(),
            valid.pre_rotation.z.to_bits()
        );

        let mut raised_port = valid.clone();
        raised_port.post_transform.origin.y = 1.5;
        let raised = controlled_cat_tick(&mut raised_port, &prior, 1.0 / 60.0, 2.0)
            .expect("a uniform elevated post-move sample must commit");
        assert_eq!(raised.frame.support_delta_y.to_bits(), 0.75_f32.to_bits());
        assert_eq!(raised.pose.pos.y.to_bits(), 1.5_f32.to_bits());
        for (flat, elevated) in success
            .state
            .tail
            .nodes()
            .iter()
            .zip(raised.state.tail.nodes())
        {
            assert!(
                ordered_f32_bits(flat.x).abs_diff(ordered_f32_bits(elevated.x)) <= 1,
                "one support transport must preserve tail X"
            );
            assert!(
                ordered_f32_bits(flat.z).abs_diff(ordered_f32_bits(elevated.z)) <= 1,
                "one support transport must preserve tail Z"
            );
            assert!(
                ((elevated.y - flat.y) - 0.75).abs() <= f32::EPSILON,
                "one support transport must lift every tail node exactly once"
            );
        }

        let mut contact_prior = prior.clone();
        let position = ActorPosition::try_new(valid.pre_transform.origin).unwrap();
        let yaw = ActorYaw::try_new(f64::from(valid.pre_rotation.y)).unwrap();
        for _ in 0..7 {
            contact_prior
                .gait
                .advance(
                    StepDuration::from_raw(1.0 / 60.0),
                    position,
                    yaw,
                    FiniteMeasure::try_new(0.6, "test.speed").unwrap(),
                )
                .unwrap();
        }
        assert!(contact_prior.gait.capture().in_swing[0]);
        let mut contact_port = valid.clone();
        controlled_cat_tick(&mut contact_port, &contact_prior, 1.0 / 60.0, 2.0)
            .expect("the elevated touchdown tick must commit");
        assert!(
            contact_port.trace.iter().any(|event| matches!(
                event,
                MotionTrace::EmitWave { at, .. } if at.y.to_bits() == 0.77_f32.to_bits()
            )),
            "the elevated paw voice must follow its contact support: {:?}",
            contact_port.trace
        );
    }

    #[test]
    fn copied_cat_state_requires_the_exact_producer_relationships() {
        let current_full = Vector3::new(0.25, 0.0, 0.125);
        let brain_yaw = -0.5_f64;
        let canonical_body_yaw = f64::from(
            GodotRotation::canonicalize_replacing_yaw(current_full, brain_yaw as f32)
                .unwrap()
                .world()
                .y,
        );
        assert_ne!(canonical_body_yaw.to_bits(), brain_yaw.to_bits());
        prepare_cat_snapshot_links(
            current_full,
            CatSnapshotLinks {
                body_yaw: canonical_body_yaw,
                brain_yaw,
                pose_yaw: brain_yaw,
                gait_amp: 0.375,
                pose_amp: 0.375,
                cat_sit: 0.625,
                pose_sit: 0.625,
            },
        )
        .unwrap();

        let cases = [
            (
                "pose.yaw",
                [
                    canonical_body_yaw,
                    brain_yaw,
                    0.2,
                    0.375,
                    0.375,
                    0.625,
                    0.625,
                ],
            ),
            (
                "pose.amp",
                [
                    canonical_body_yaw,
                    brain_yaw,
                    brain_yaw,
                    0.375,
                    0.5,
                    0.625,
                    0.625,
                ],
            ),
            (
                "pose.sit",
                [
                    canonical_body_yaw,
                    brain_yaw,
                    brain_yaw,
                    0.375,
                    0.375,
                    0.625,
                    0.5,
                ],
            ),
            (
                "yaw",
                [0.2, brain_yaw, brain_yaw, 0.375, 0.375, 0.625, 0.625],
            ),
        ];
        for (path, values) in cases {
            let error = prepare_cat_snapshot_links(
                current_full,
                CatSnapshotLinks {
                    body_yaw: values[0],
                    brain_yaw: values[1],
                    pose_yaw: values[2],
                    gait_amp: values[3],
                    pose_amp: values[4],
                    cat_sit: values[5],
                    pose_sit: values[6],
                },
            )
            .expect_err("contradictory copied state must be refused");
            assert_eq!(error.path, path);
        }
    }
}
