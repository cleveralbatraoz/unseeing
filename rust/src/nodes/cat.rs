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

use std::fmt;
use std::num::NonZeroU64;

use godot::classes::character_body_3d::{MotionMode, PlatformOnLeave};
use godot::classes::{
    ArrayMesh, CapsuleShape3D, CharacterBody3D, CollisionShape3D, Engine, ICharacterBody3D,
    Material, MeshInstance3D, PhysicsServer3D, PhysicsTestMotionParameters3D,
    PhysicsTestMotionResult3D,
};
use godot::prelude::*;

use super::solid::clear_limbs;
use super::support::{
    FLOOR_MAX_ANGLE_RAD, FLOOR_SNAP_M, MAX_SLIDES, MOTION_RESULT_MAX_CONTACTS, PLATFORM_LAYERS,
    SAFE_MARGIN_M, SNAP_PROBE_MAX_CONTACTS, collision_pair, is_actor_layer,
};
use crate::cat_body::{self, CatPose, PreparedCatPose, PreparedTail, Tail};
use crate::cat_brain::{CatBrain, Mood, PreparedCatBrain, RoamRect};
use crate::cat_gait::{self, CatGait, PreparedCatGait};
use crate::limbs::{LimbBuf, sphere, sphere_lod, tube, tube_res};
use crate::render::{self, Role};
use crate::reproduce::RestoreValueError;
use crate::reproduce::blob::CatCapture;
use crate::sound_source::{Cadence, PreparedCadence};
use crate::support_motion::{
    ActorPosition, ActorTransform, ActorVelocity, ActorYaw, FiniteMeasure, FiniteRotation,
    GodotRotation, LandingEvent, MotionConfigError, MotionConfigField, MotionOutcome, MotionPhase,
    MotionRestoreError, MotionState, MotionValueError, PlanarVelocity, PosePoint, QueuedWaveGate,
    StepDuration, SupportContact, SupportMotionConfig, landing_voice, prepare, reconcile,
    validate_restore,
};
use crate::temporal::PreparedTime;

/// Collider radius — small enough to slip between furniture legs.
const COL_RADIUS: f32 = 0.11;

/// Collider height; its bottom floats a hair above the floor, like the
/// player's capsule — the flat map means nothing ever presses down.
const COL_HEIGHT: f32 = 0.34;

/// Collider centre height above the cat's own support datum — the capsule's
/// bottom meets the floor exactly, with no `+0.02` fudge above it.
const COLLIDER_CENTER_Y: f32 = COL_HEIGHT * 0.5;

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
    /// Downward acceleration in metres per second squared, applied only
    /// while this cat is airborne. Authored per cat so two cats may fall
    /// differently; staged into the active configuration only once the
    /// complete authored set of six motion fields is mutually valid.
    #[export(range = (0.1, 30.0, 0.1, suffix = " m/s²"))]
    #[var(get = get_fall_acceleration, set = set_fall_acceleration)]
    #[init(val = 9.8)]
    fall_acceleration: f64,
    /// Maximum downward speed in metres per second this cat may reach
    /// while falling. Authored per cat; staged into the active
    /// configuration only once the complete authored set is mutually
    /// valid.
    #[export(range = (0.5, 50.0, 0.5, suffix = " m/s"))]
    #[var(get = get_terminal_fall_speed, set = set_terminal_fall_speed)]
    #[init(val = 20.0)]
    terminal_fall_speed: f64,
    /// Landing speed in metres per second at or below which this cat's
    /// landing makes no sound. It must remain below Landing Full Speed;
    /// either threshold may be edited first — an out-of-order pair stages
    /// this scalar, keeps the prior active configuration live, and raises
    /// this cat's editor warning until the complementary edit repairs it.
    #[export(range = (0.0, 10.0, 0.1, suffix = " m/s"))]
    #[var(get = get_landing_silent_speed, set = set_landing_silent_speed)]
    #[init(val = 1.5)]
    landing_silent_speed: f64,
    /// Landing speed in metres per second at which this cat's landing
    /// voice reaches full strength. It must exceed Landing Silent Speed;
    /// either threshold may be edited first — an out-of-order pair
    /// stages this scalar, keeps the prior active configuration live,
    /// and raises this cat's editor warning until the complementary
    /// edit repairs it.
    #[export(range = (0.1, 20.0, 0.1, suffix = " m/s"))]
    #[var(get = get_landing_full_speed, set = set_landing_full_speed)]
    #[init(val = 4.0)]
    landing_full_speed: f64,
    /// Maximum authored landing-wave gain for this cat, unitless. Staged
    /// into the active configuration only once the complete authored set
    /// of six motion fields is mutually valid.
    #[export(range = (0.0, 1.0, 0.01))]
    #[var(get = get_landing_max_gain, set = set_landing_max_gain)]
    #[init(val = 0.60)]
    landing_max_gain: f64,
    /// Maximum authored landing-wave radius in metres for this cat. Staged
    /// into the active configuration only once the complete authored set
    /// of six motion fields is mutually valid.
    #[export(range = (0.0, 10.0, 0.1, suffix = " m"))]
    #[var(get = get_landing_max_range, set = set_landing_max_range)]
    #[init(val = 2.5)]
    landing_max_range: f64,
    /// The always-valid active motion configuration this cat's physics
    /// tick actually uses — distinct from the six authored scalars above,
    /// which may transiently disagree with each other (an out-of-order
    /// threshold pair) without ever installing an invalid active config.
    #[init(val = SupportMotionConfig::CAT_DEFAULT)]
    motion_config: SupportMotionConfig,
    /// The one editor warning this node can raise on itself: an
    /// out-of-order landing threshold pair, naming both. `None` when the
    /// six authored scalars last agreed.
    threshold_warning: Option<String>,
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
    support_collider_id: Option<u64>,
    #[init(val = PhysicsTestMotionParameters3D::new_gd())]
    snap_params: Gd<PhysicsTestMotionParameters3D>,
    #[init(val = PhysicsTestMotionResult3D::new_gd())]
    snap_result: Gd<PhysicsTestMotionResult3D>,
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
/// It carries only the raw physical transaction (transform/rotation/
/// velocity, one move, and the post-move support scan) — brain, gait, tail
/// and wave emission are pure values the tick and its caller carry
/// alongside it, never behind this port.
trait CatMotionPort {
    fn read_global_transform(&mut self) -> Transform3D;
    fn read_global_rotation(&mut self) -> Vector3;
    fn read_velocity(&mut self) -> Vector3;
    fn write_global_rotation(&mut self, rotation: Vector3);
    fn write_velocity(&mut self, velocity: Vector3);
    fn move_and_slide_once(&mut self);
    fn is_on_floor(&mut self) -> bool;
    fn read_slide_collision_count(&mut self) -> i32;
    fn read_slide_contact_count(&mut self, slide: i32) -> Option<i32>;
    fn read_slide_contact_geometry(
        &mut self,
        slide: i32,
        contact: i32,
    ) -> Option<(Vector3, Vector3)>;
    fn read_slide_collider(&mut self, slide: i32, contact: i32) -> Option<(bool, u32, u64)>;
    fn probe_snap(&mut self, post_transform: Transform3D) -> bool;
    fn read_probe_contact_count(&mut self) -> i32;
    fn read_probe_contact_geometry(&mut self, contact: i32) -> (Vector3, Vector3);
    fn read_probe_collider(&mut self, contact: i32) -> (bool, u32, u64);
    fn write_global_transform(&mut self, transform: Transform3D);
    fn disable_processing(&mut self);
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

    fn is_on_floor(&mut self) -> bool {
        self.base().is_on_floor()
    }

    fn read_slide_collision_count(&mut self) -> i32 {
        self.base().get_slide_collision_count()
    }

    fn read_slide_contact_count(&mut self, slide: i32) -> Option<i32> {
        self.base()
            .get_slide_collision(slide)
            .map(|collision| collision.get_collision_count())
    }

    fn read_slide_contact_geometry(
        &mut self,
        slide: i32,
        contact: i32,
    ) -> Option<(Vector3, Vector3)> {
        self.base().get_slide_collision(slide).map(|collision| {
            (
                collision.get_position_ex().collision_index(contact).done(),
                collision.get_normal_ex().collision_index(contact).done(),
            )
        })
    }

    fn read_slide_collider(&mut self, slide: i32, contact: i32) -> Option<(bool, u32, u64)> {
        self.base().get_slide_collision(slide).map(|collision| {
            let rid = collision
                .get_collider_rid_ex()
                .collision_index(contact)
                .done();
            let valid = rid.is_valid();
            let layer = if valid {
                PhysicsServer3D::singleton().body_get_collision_layer(rid)
            } else {
                0
            };
            let id = collision
                .get_collider_id_ex()
                .collision_index(contact)
                .done();
            (valid, layer, id)
        })
    }

    fn probe_snap(&mut self, post_transform: Transform3D) -> bool {
        self.snap_params.set_from(post_transform);
        let body = self.base().get_rid();
        PhysicsServer3D::singleton()
            .body_test_motion_ex(body, &self.snap_params)
            .result(&self.snap_result)
            .done()
    }

    fn read_probe_contact_count(&mut self) -> i32 {
        self.snap_result.get_collision_count()
    }

    fn read_probe_contact_geometry(&mut self, contact: i32) -> (Vector3, Vector3) {
        (
            self.snap_result
                .get_collision_point_ex()
                .collision_index(contact)
                .done(),
            self.snap_result
                .get_collision_normal_ex()
                .collision_index(contact)
                .done(),
        )
    }

    fn read_probe_collider(&mut self, contact: i32) -> (bool, u32, u64) {
        let rid = self
            .snap_result
            .get_collider_rid_ex()
            .collision_index(contact)
            .done();
        let valid = rid.is_valid();
        let layer = if valid {
            PhysicsServer3D::singleton().body_get_collision_layer(rid)
        } else {
            0
        };
        let id = self
            .snap_result
            .get_collider_id_ex()
            .collision_index(contact)
            .done();
        (valid, layer, id)
    }

    fn write_global_transform(&mut self, transform: Transform3D) {
        self.base_mut().set_global_transform(transform);
    }

    fn disable_processing(&mut self) {
        self.base_mut().set_physics_process(false);
        self.base_mut().set_process(false);
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
    motion: MotionState,
}

struct CatTickSuccess {
    state: CatControlledState,
    pose: CatPose,
    frame: cat_gait::GaitFrame,
    phase_before: MotionPhase,
    landing: Option<LandingEvent>,
    support_collider_id: Option<u64>,
}

/// Why one cat-owned support scan refused, before any commit. Mirrors the
/// player's own [`super::player`] ledger scan one to one — same bounded
/// ledger, same conditional cached snap fallback, same refusal shapes —
/// but is a cat-owned copy: the cat proves its own port wiring and scratch
/// ownership rather than resting on the player adapter's evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CatSupportReadError {
    InvalidOuterCount(i32),
    MissingSlide(i32),
    InvalidInnerCount { slide: i32, count: i32 },
    InvalidOrdinaryRid { slide: i32, contact: i32 },
    InvalidProbeCount(i32),
    InvalidProbeRid(i32),
    InvalidValue(MotionValueError),
}

impl From<MotionValueError> for CatSupportReadError {
    fn from(error: MotionValueError) -> Self {
        Self::InvalidValue(error)
    }
}

impl fmt::Display for CatSupportReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOuterCount(count) => {
                write!(
                    formatter,
                    "slide collision count {count} is outside 0..={MAX_SLIDES}"
                )
            }
            Self::MissingSlide(slide) => {
                write!(formatter, "slide collision {slide} is missing")
            }
            Self::InvalidInnerCount { slide, count } => write!(
                formatter,
                "slide collision {slide} contact count {count} is outside 1..={MOTION_RESULT_MAX_CONTACTS}"
            ),
            Self::InvalidOrdinaryRid { slide, contact } => write!(
                formatter,
                "slide collision {slide} contact {contact} has an invalid collider RID"
            ),
            Self::InvalidProbeCount(count) => write!(
                formatter,
                "snap probe contact count {count} is outside 1..={SNAP_PROBE_MAX_CONTACTS}"
            ),
            Self::InvalidProbeRid(contact) => {
                write!(
                    formatter,
                    "snap probe contact {contact} has an invalid collider RID"
                )
            }
            Self::InvalidValue(error) => error.fmt(formatter),
        }
    }
}

fn cat_floor_angle_accepts(contact: SupportContact) -> bool {
    let normal = contact.normal();
    let x = f64::from(normal.x);
    let y = f64::from(normal.y);
    let z = f64::from(normal.z);
    let length = (x * x + y * y + z * z).sqrt();
    let cosine = (y / length).clamp(-1.0, 1.0);
    cosine.acos() <= f64::from(FLOOR_MAX_ANGLE_RAD)
}

/// The cat-owned support door: bounded ordinary-ledger scan plus a
/// conditional cached snap-fact probe, run only when `is_on_floor()`
/// reports a floor with no floorish contact in the public ledger. Every
/// indexed point/normal and every floor collider fact is validated before
/// the first retained world candidate is returned; an actor-only ordinary
/// floor yields no support and no fallback probe.
fn read_cat_post_move_support<P: CatMotionPort>(
    port: &mut P,
    post_transform: ActorTransform,
) -> Result<(Option<SupportContact>, Option<u64>), CatSupportReadError> {
    if !port.is_on_floor() {
        return Ok((None, None));
    }

    let slide_count = port.read_slide_collision_count();
    if !(0..=MAX_SLIDES).contains(&slide_count) {
        return Err(CatSupportReadError::InvalidOuterCount(slide_count));
    }
    let mut candidate = None;
    let mut saw_floorish = false;
    for slide in 0..slide_count {
        let contact_count = port
            .read_slide_contact_count(slide)
            .ok_or(CatSupportReadError::MissingSlide(slide))?;
        if !(1..=MOTION_RESULT_MAX_CONTACTS).contains(&contact_count) {
            return Err(CatSupportReadError::InvalidInnerCount {
                slide,
                count: contact_count,
            });
        }
        for contact_index in 0..contact_count {
            let (point, normal) = port
                .read_slide_contact_geometry(slide, contact_index)
                .ok_or(CatSupportReadError::MissingSlide(slide))?;
            let contact = SupportContact::try_new(point, normal)?;
            if !cat_floor_angle_accepts(contact) {
                continue;
            }
            saw_floorish = true;
            let (collider_rid_valid, collider_layer, collider_id) = port
                .read_slide_collider(slide, contact_index)
                .ok_or(CatSupportReadError::MissingSlide(slide))?;
            if !collider_rid_valid {
                return Err(CatSupportReadError::InvalidOrdinaryRid {
                    slide,
                    contact: contact_index,
                });
            }
            if !is_actor_layer(collider_layer) && candidate.is_none() {
                candidate = Some((contact, NonZeroU64::new(collider_id).map(NonZeroU64::get)));
            }
        }
    }
    if let Some((support, collider_id)) = candidate {
        return Ok((Some(support), collider_id));
    }
    if saw_floorish {
        return Ok((None, None));
    }

    if !port.probe_snap(post_transform.world()) {
        return Ok((None, None));
    }
    let contact_count = port.read_probe_contact_count();
    if !(1..=SNAP_PROBE_MAX_CONTACTS).contains(&contact_count) {
        return Err(CatSupportReadError::InvalidProbeCount(contact_count));
    }
    let mut candidate = None;
    for contact_index in 0..contact_count {
        let (point, normal) = port.read_probe_contact_geometry(contact_index);
        let contact = SupportContact::try_new(point, normal)?;
        if !cat_floor_angle_accepts(contact) {
            continue;
        }
        let (collider_rid_valid, collider_layer, collider_id) =
            port.read_probe_collider(contact_index);
        if !collider_rid_valid {
            return Err(CatSupportReadError::InvalidProbeRid(contact_index));
        }
        if !is_actor_layer(collider_layer) && candidate.is_none() {
            candidate = Some((contact, NonZeroU64::new(collider_id).map(NonZeroU64::get)));
        }
    }
    Ok(candidate.map_or((None, None), |(support, collider_id)| {
        (Some(support), collider_id)
    }))
}

/// The one place a cat's brain and yaw may or may not advance. Only
/// [`CatControlPolicy::AdvanceBrain`] may call [`CatBrain::advance`] and
/// produce a fresh yaw command; [`CatControlPolicy::Frozen`] always keeps
/// the cat's planar velocity at zero and its yaw command at `None` — no
/// airborne path may invoke a yaw setter, even to rewrite the same value.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CatControlPolicy {
    AdvanceBrain,
    Frozen { yaw: ActorYaw, sitting: bool },
}

fn cat_control_policy(phase: MotionPhase, current_yaw: ActorYaw, mood: Mood) -> CatControlPolicy {
    match phase {
        MotionPhase::Controlled => CatControlPolicy::AdvanceBrain,
        MotionPhase::Airborne { .. } => CatControlPolicy::Frozen {
            yaw: current_yaw,
            sitting: matches!(mood, Mood::Sit),
        },
    }
}

/// Why one controlled cat tick refused — a physical value error at any
/// stage, or a poisoned/malformed post-move support scan.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CatTickReason {
    Motion(MotionValueError),
    Support(CatSupportReadError),
}

impl fmt::Display for CatTickReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Motion(error) => error.fmt(formatter),
            Self::Support(error) => error.fmt(formatter),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CatTickFault {
    phase: &'static str,
    reason: CatTickReason,
}

fn refuse_cat_motion<P: CatMotionPort>(
    port: &mut P,
    phase: &'static str,
    reason: CatTickReason,
) -> CatTickFault {
    port.disable_processing();
    CatTickFault { phase, reason }
}

fn rollback_cat_motion<P: CatMotionPort>(
    port: &mut P,
    saved_transform: Transform3D,
    phase: &'static str,
    reason: CatTickReason,
) -> CatTickFault {
    // Order is part of the transaction: reinstate the complete body pose,
    // stop every commanded lane, then make the refusal inert.
    port.write_global_transform(saved_transform);
    port.write_velocity(Vector3::ZERO);
    port.disable_processing();
    CatTickFault { phase, reason }
}

/// One controlled cat's physics tick — the same two-phase support law as
/// the player's, adapted to a mind that must freeze whole in the air.
/// `prior` is read by value only; `self`'s own components are never
/// mutated until the caller commits a returned success, so a refusal at
/// any stage leaves brain, gait, tail and motion state exactly as they
/// were before this call.
fn controlled_cat_tick<P: CatMotionPort>(
    port: &mut P,
    prior: &CatControlledState,
    raw_dt: f64,
    config: SupportMotionConfig,
) -> Result<CatTickSuccess, CatTickFault> {
    let duration = StepDuration::from_raw(raw_dt);
    let saved_transform = port.read_global_transform();
    let transform_before = ActorTransform::try_new(saved_transform).map_err(|error| {
        refuse_cat_motion(port, "physics transform", CatTickReason::Motion(error))
    })?;
    let before = transform_before.position();
    let last_pos = ActorPosition::try_new(prior.last_pos).map_err(|error| {
        refuse_cat_motion(port, "physics prior position", CatTickReason::Motion(error))
    })?;
    let rotation_before =
        FiniteRotation::try_new(port.read_global_rotation()).map_err(|error| {
            refuse_cat_motion(port, "physics rotation", CatTickReason::Motion(error))
        })?;
    ActorVelocity::try_new(port.read_velocity()).map_err(|error| {
        refuse_cat_motion(port, "physics velocity", CatTickReason::Motion(error))
    })?;
    if !prior.sit.is_finite() {
        return Err(refuse_cat_motion(
            port,
            "physics sit",
            CatTickReason::Motion(MotionValueError::non_finite("cat.sit")),
        ));
    }
    if !(0.0..=1.0).contains(&prior.sit) {
        return Err(refuse_cat_motion(
            port,
            "physics sit",
            CatTickReason::Motion(MotionValueError::out_of_range("cat.sit")),
        ));
    }
    FiniteMeasure::try_new(prior.sim_t, "cat.sim_t").map_err(|error| {
        refuse_cat_motion(
            port,
            "physics simulation time",
            CatTickReason::Motion(error),
        )
    })?;

    let body_yaw = rotation_before.yaw();
    let mut brain = prior.brain;
    let mut gait = prior.gait.clone();
    let mut tail = prior.tail;
    let phase_before = prior.motion.phase();

    let policy = cat_control_policy(phase_before, body_yaw, brain.mood());
    let (desired, yaw, sitting, yaw_command) = match policy {
        CatControlPolicy::AdvanceBrain => {
            let progress = before.planar_distance(last_pos);
            let drive = brain.advance(duration, before, progress).map_err(|error| {
                refuse_cat_motion(port, "brain advance", CatTickReason::Motion(error))
            })?;
            let desired = match PlanarVelocity::try_new(
                (-drive.yaw.radians().sin() * drive.speed.value()) as f32,
                (-drive.yaw.radians().cos() * drive.speed.value()) as f32,
            ) {
                Ok(value) => value,
                Err(error) => {
                    return Err(refuse_cat_motion(
                        port,
                        "commanded velocity",
                        CatTickReason::Motion(error),
                    ));
                }
            };
            (desired, drive.yaw, drive.sitting, Some(drive.yaw))
        }
        CatControlPolicy::Frozen { yaw, sitting } => (PlanarVelocity::ZERO, yaw, sitting, None),
    };

    // Every fallible pre-move conversion is complete. Only now may yaw
    // mutate — and only when the policy actually produced a command.
    if let Some(command) = yaw_command {
        let mut commanded_rotation = rotation_before.world();
        commanded_rotation.y = command.godot_lane();
        port.write_global_rotation(commanded_rotation);
    }

    let prepared = prepare(prior.motion, desired, duration, config);
    port.write_velocity(prepared.command().world_velocity());
    port.move_and_slide_once();

    let transform_after = match ActorTransform::try_new(port.read_global_transform()) {
        Ok(value) => value,
        Err(error) => {
            return Err(rollback_cat_motion(
                port,
                saved_transform,
                "post-move transform",
                CatTickReason::Motion(error),
            ));
        }
    };
    if let Err(error) = FiniteRotation::try_new(port.read_global_rotation()) {
        return Err(rollback_cat_motion(
            port,
            saved_transform,
            "post-move rotation",
            CatTickReason::Motion(error),
        ));
    }
    let new_position = transform_after.position();
    let (support, collider_id) = match read_cat_post_move_support(port, transform_after) {
        Ok(value) => value,
        Err(error) => {
            return Err(rollback_cat_motion(
                port,
                saved_transform,
                "post-move support",
                CatTickReason::Support(error),
            ));
        }
    };
    let actual_velocity = match ActorVelocity::try_new(port.read_velocity()) {
        Ok(value) => value,
        Err(error) => {
            return Err(rollback_cat_motion(
                port,
                saved_transform,
                "post-move velocity",
                CatTickReason::Motion(error),
            ));
        }
    };
    let outcome = MotionOutcome::new(actual_velocity, support);
    let transition = reconcile(prepared, outcome);

    let actual_speed = if duration.seconds() == 0.0 {
        FiniteMeasure::ZERO
    } else {
        match FiniteMeasure::try_new(
            new_position.planar_distance(before).value() / duration.seconds(),
            "cat.actual_speed",
        ) {
            Ok(value) => value,
            Err(error) => {
                return Err(rollback_cat_motion(
                    port,
                    saved_transform,
                    "post-move speed",
                    CatTickReason::Motion(error),
                ));
            }
        }
    };
    let frame = match gait.advance(duration, new_position, yaw, actual_speed) {
        Ok(frame) => frame,
        Err(error) => {
            return Err(rollback_cat_motion(
                port,
                saved_transform,
                "gait advance",
                CatTickReason::Motion(error),
            ));
        }
    };

    let next_sit = prior.sit
        + ((if sitting { 1.0 } else { 0.0 }) - prior.sit)
            * (duration.seconds() * SIT_EASE).min(1.0);
    let next_sim_t = prior.sim_t + duration.seconds();
    let sway = 0.22 * (frame.phase * std::f64::consts::TAU).sin() * frame.amp
        + 0.10 * (next_sim_t * 0.9).sin() * (1.0 - frame.amp);
    let pose = match CatPose::try_from_gait(new_position, yaw, &frame, next_sit) {
        Ok(pose) => pose,
        Err(error) => {
            return Err(rollback_cat_motion(
                port,
                saved_transform,
                "pose",
                CatTickReason::Motion(error),
            ));
        }
    };
    let skeleton = match cat_body::skeleton(&pose) {
        Ok(skeleton) => skeleton,
        Err(error) => {
            return Err(rollback_cat_motion(
                port,
                saved_transform,
                "skeleton",
                CatTickReason::Motion(error),
            ));
        }
    };
    let tail_root = match PosePoint::try_new(skeleton.tail_root) {
        Ok(root) => root,
        Err(error) => {
            return Err(rollback_cat_motion(
                port,
                saved_transform,
                "tail root",
                CatTickReason::Motion(error),
            ));
        }
    };
    if let Err(error) = tail.transport_y(frame.support_delta_y) {
        return Err(rollback_cat_motion(
            port,
            saved_transform,
            "tail support transport",
            CatTickReason::Motion(error),
        ));
    }
    if let Err(error) = tail.advance(
        duration,
        tail_root,
        yaw,
        new_position.elevation(),
        next_sit,
        sway,
    ) {
        return Err(rollback_cat_motion(
            port,
            saved_transform,
            "tail advance",
            CatTickReason::Motion(error),
        ));
    }

    // While airborne, and on the landing tick itself, keep the brain's next
    // progress sample pinned to the body's actual position: a resumed brain
    // must never read the whole flight as walked distance.
    let airborne_or_landing = matches!(transition.state.phase(), MotionPhase::Airborne { .. })
        || transition.landing.is_some();
    let next_last_pos = if airborne_or_landing {
        new_position.world()
    } else {
        before.world()
    };

    Ok(CatTickSuccess {
        state: CatControlledState {
            brain,
            gait,
            tail,
            presence: prior.presence,
            sit: next_sit,
            sim_t: next_sim_t,
            last_pos: next_last_pos,
            motion: transition.state,
        },
        pose,
        frame,
        phase_before,
        landing: transition.landing,
        support_collider_id: collider_id,
    })
}

#[godot_api]
impl ICharacterBody3D for WaveCat {
    fn ready(&mut self) {
        clear_limbs(self, &LIMBS);
        // All eleven solver values from the shared table, applied before the
        // editor/runtime split: an editor-frozen cat never moves, so these
        // are harmless there, and the runtime cat needs every one of them
        // configured before its first physics tick.
        self.base_mut().set_motion_mode(MotionMode::GROUNDED);
        self.base_mut().set_up_direction(Vector3::UP);
        self.base_mut().set_floor_snap_length(FLOOR_SNAP_M);
        self.base_mut().set_floor_max_angle(FLOOR_MAX_ANGLE_RAD);
        self.base_mut().set_safe_margin(SAFE_MARGIN_M);
        self.base_mut().set_max_slides(MAX_SLIDES);
        self.base_mut().set_floor_stop_on_slope_enabled(true);
        self.base_mut().set_floor_constant_speed_enabled(false);
        self.base_mut().set_platform_floor_layers(PLATFORM_LAYERS);
        self.base_mut().set_platform_wall_layers(PLATFORM_LAYERS);
        self.base_mut()
            .set_platform_on_leave(PlatformOnLeave::DO_NOTHING);
        self.apply_collision_pair();
        self.snap_params.set_motion(Vector3::DOWN * FLOOR_SNAP_M);
        self.snap_params.set_margin(SAFE_MARGIN_M);
        self.snap_params.set_max_collisions(SNAP_PROBE_MAX_CONTACTS);
        self.snap_params.set_recovery_as_collision_enabled(true);
        self.snap_params.set_collide_separation_ray_enabled(true);
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

        // The final authored six, validated fresh — never the possibly
        // stale active config an out-of-order edit sequence could leave
        // behind. An invalid final pair disables motion outright rather
        // than silently keeping whatever the active config last was.
        match SupportMotionConfig::try_new(
            self.fall_acceleration,
            self.terminal_fall_speed,
            self.landing_silent_speed,
            self.landing_full_speed,
            self.landing_max_gain,
            self.landing_max_range,
        ) {
            Ok(config) => self.motion_config = config,
            Err(error) => {
                godot_error!("WaveCat: invalid motion configuration — {error}");
                self.base_mut().set_physics_process(false);
                self.base_mut().set_process(false);
                return;
            }
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
        col.set_position(Vector3::new(0.0, COLLIDER_CENTER_Y, 0.0));
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
            motion: self.motion_state,
        };
        let config = self.motion_config;
        let success = match controlled_cat_tick(self, &prior, dt, config) {
            Ok(success) => success,
            Err(fault) => {
                godot_error!("WaveCat: {} refused: {}", fault.phase, fault.reason);
                return;
            }
        };

        let CatTickSuccess {
            state,
            pose,
            frame,
            phase_before,
            landing,
            support_collider_id,
        } = success;
        self.pose = Some(pose);
        self.brain = Some(state.brain);
        self.gait = Some(state.gait);
        self.tail = Some(state.tail);
        self.sit = state.sit;
        self.sim_t = state.sim_t;
        self.last_pos = state.last_pos;
        self.motion_state = state.motion;
        self.support_collider_id = support_collider_id;
        self.apply_collision_pair();
        self.mesh_dirty = true;

        // Voices last: every value/component/layer install above has
        // already committed, so a wave never fires against half-applied
        // state. A paw contact fires only across an unbroken controlled
        // stretch with no landing on it — the same law the player's queued
        // waves obey, never a second copy of the phase boolean.
        let now = self.now;
        for contact in frame
            .contacts
            .iter()
            .filter(|contact| cat_gait::paw_sounds(contact.leg))
        {
            if !QueuedWaveGate::ControlledContact.allows(
                phase_before,
                state.motion.phase(),
                landing,
            ) {
                continue;
            }
            self.emit_wave(
                Vector3::new(contact.at.x, contact.at.y + 0.02, contact.at.z),
                cat_gait::PAW_RANGE,
                cat_gait::PAW_GAIN,
                now,
            );
        }
        if self.presence.beat(now).is_some() {
            let raw_post = pose.pos;
            self.emit_wave(
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
        if let Some(event) = landing
            && let Some(voice) = landing_voice(event, config)
        {
            let point = event.support().point();
            self.emit_wave(
                Vector3::new(point.x, point.y + 0.02, point.z),
                voice.range_m(),
                voice.gain(),
                now,
            );
        }
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

    /// The Scene-dock warning triangle: an out-of-order landing threshold
    /// pair, naming both — `None` staged means no warning at all.
    fn get_configuration_warnings(&self) -> PackedStringArray {
        let mut warnings = PackedStringArray::new();
        if let Some(message) = self.threshold_warning.as_ref() {
            warnings.push(message.as_str());
        }
        warnings
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

    /// The six active authored motion scalars in constructor order — the
    /// same read-only observability door Task 2 defined for the player.
    /// Exported authored getters alone are not injection evidence: this is
    /// the ACTIVE config a valid stage installed, which may transiently
    /// differ from the raw authored scalars during an out-of-order edit.
    #[func]
    fn motion_config_snapshot(&self) -> PackedFloat64Array {
        let config = self.motion_config;
        PackedFloat64Array::from(
            &[
                config.fall_acceleration_mps2(),
                config.terminal_fall_speed_mps(),
                config.landing_silent_speed_mps(),
                config.landing_full_speed_mps(),
                config.landing_max_gain(),
                config.landing_max_range_m(),
            ][..],
        )
    }

    /// The registered callable twin of the virtual warning read — the
    /// dual-channel contract every warning-bearing node keeps.
    #[func]
    fn get_configuration_warnings(&self) -> PackedStringArray {
        ICharacterBody3D::get_configuration_warnings(self)
    }

    #[func]
    fn get_fall_acceleration(&self) -> f64 {
        self.fall_acceleration
    }

    #[func]
    fn set_fall_acceleration(&mut self, value: f64) {
        self.try_stage_motion_field(MotionConfigField::FallAcceleration, value);
    }

    #[func]
    fn get_terminal_fall_speed(&self) -> f64 {
        self.terminal_fall_speed
    }

    #[func]
    fn set_terminal_fall_speed(&mut self, value: f64) {
        self.try_stage_motion_field(MotionConfigField::TerminalFallSpeed, value);
    }

    #[func]
    fn get_landing_silent_speed(&self) -> f64 {
        self.landing_silent_speed
    }

    #[func]
    fn set_landing_silent_speed(&mut self, value: f64) {
        self.try_stage_motion_field(MotionConfigField::LandingSilentSpeed, value);
    }

    #[func]
    fn get_landing_full_speed(&self) -> f64 {
        self.landing_full_speed
    }

    #[func]
    fn set_landing_full_speed(&mut self, value: f64) {
        self.try_stage_motion_field(MotionConfigField::LandingFullSpeed, value);
    }

    #[func]
    fn get_landing_max_gain(&self) -> f64 {
        self.landing_max_gain
    }

    #[func]
    fn set_landing_max_gain(&mut self, value: f64) {
        self.try_stage_motion_field(MotionConfigField::LandingMaxGain, value);
    }

    #[func]
    fn get_landing_max_range(&self) -> f64 {
        self.landing_max_range
    }

    #[func]
    fn set_landing_max_range(&mut self, value: f64) {
        self.try_stage_motion_field(MotionConfigField::LandingMaxRange, value);
    }

    /// Accepted world-support identity, if the engine supplied one. A
    /// server-backed support may lawfully have no Object id.
    #[func]
    fn support_collider_id(&self) -> Variant {
        self.support_collider_id
            .and_then(|id| i64::try_from(id).ok())
            .map_or_else(Variant::nil, |id| id.to_variant())
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
        validate_restore(capture.motion, velocity, self.motion_config).map_err(|error| {
            let (path, rule) = match error {
                MotionRestoreError::AirbornePlanarMismatch { axis } => (
                    format!("motion.phase.planar_velocity.{axis}"),
                    "must match the corresponding physical velocity lane bit-exactly while airborne",
                ),
                MotionRestoreError::AirborneTerminalExceeded => (
                    "motion.phase.vertical_velocity".to_string(),
                    "must remain between zero and the injected terminal fall speed",
                ),
                MotionRestoreError::Physical(_) => (
                    "motion".to_string(),
                    "contains an invalid physical value",
                ),
            };
            RestoreValueError::new(path, rule)
        })?;
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
        col.set_position(Vector3::new(0.0, COLLIDER_CENTER_Y, 0.0));
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

    fn apply_collision_pair(&mut self) {
        let (layer, mask) = collision_pair(self.motion_state.phase());
        self.apply_exact_collision_pair(layer, mask);
    }

    fn apply_exact_collision_pair(&mut self, layer: u32, mask: u32) {
        if self.base().get_collision_layer() != layer {
            self.base_mut().set_collision_layer(layer);
        }
        if self.base().get_collision_mask() != mask {
            self.base_mut().set_collision_mask(mask);
        }
    }

    /// The six authored motion scalars, in constructor order — never the
    /// active config, which may transiently lag an out-of-order edit.
    fn authored_motion_scalars(&self) -> [f64; 6] {
        [
            self.fall_acceleration,
            self.terminal_fall_speed,
            self.landing_silent_speed,
            self.landing_full_speed,
            self.landing_max_gain,
            self.landing_max_range,
        ]
    }

    fn assign_motion_field(&mut self, field: MotionConfigField, value: f64) {
        match field {
            MotionConfigField::FallAcceleration => self.fall_acceleration = value,
            MotionConfigField::TerminalFallSpeed => self.terminal_fall_speed = value,
            MotionConfigField::LandingSilentSpeed => self.landing_silent_speed = value,
            MotionConfigField::LandingFullSpeed => self.landing_full_speed = value,
            MotionConfigField::LandingMaxGain => self.landing_max_gain = value,
            MotionConfigField::LandingMaxRange => self.landing_max_range = value,
        }
    }

    fn set_threshold_warning(&mut self, message: String) {
        self.threshold_warning = Some(message);
        self.base_mut()
            .call_deferred("update_configuration_warnings", &[]);
    }

    fn clear_threshold_warning(&mut self) {
        if self.threshold_warning.take().is_some() {
            self.base_mut()
                .call_deferred("update_configuration_warnings", &[]);
        }
    }

    /// One exported scalar's edit, staged against the complete candidate
    /// six-tuple: a `NonFinite`/`OutOfRange` verdict on the field just
    /// edited rejects that scalar outright (the active config and every
    /// other authored scalar are untouched); a `ThresholdOrder` verdict
    /// stages the individually valid scalar, keeps the prior active config
    /// live, and raises this cat's own editor warning; a fully valid
    /// candidate stages the scalar, installs the new active config, and
    /// clears that warning.
    fn try_stage_motion_field(&mut self, field: MotionConfigField, value: f64) {
        let mut candidate = self.authored_motion_scalars();
        candidate[motion_field_index(field)] = value;
        match SupportMotionConfig::try_new(
            candidate[0],
            candidate[1],
            candidate[2],
            candidate[3],
            candidate[4],
            candidate[5],
        ) {
            Ok(config) => {
                self.assign_motion_field(field, value);
                self.motion_config = config;
                self.clear_threshold_warning();
            }
            Err(error @ MotionConfigError::ThresholdOrder { .. }) => {
                self.assign_motion_field(field, value);
                self.set_threshold_warning(error.to_string());
            }
            Err(_) => {}
        }
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

/// The constructor-order index of one motion field within the six-tuple
/// [`WaveCat::authored_motion_scalars`] carries.
fn motion_field_index(field: MotionConfigField) -> usize {
    match field {
        MotionConfigField::FallAcceleration => 0,
        MotionConfigField::TerminalFallSpeed => 1,
        MotionConfigField::LandingSilentSpeed => 2,
        MotionConfigField::LandingFullSpeed => 3,
        MotionConfigField::LandingMaxGain => 4,
        MotionConfigField::LandingMaxRange => 5,
    }
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
        Probe(Transform3D),
        ReadProbeCount,
        ReadProbeContact(i32),
        SetTransform(Transform3D),
        Disable,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct CatRawSupportFact {
        point: Vector3,
        normal: Vector3,
        collider_rid_valid: bool,
        collider_layer: u32,
        collider_id: u64,
    }

    fn cat_world_floor() -> CatRawSupportFact {
        CatRawSupportFact {
            point: Vector3::new(2.0, 0.0, -3.0),
            normal: Vector3::UP,
            collider_rid_valid: true,
            collider_layer: 1,
            collider_id: 41,
        }
    }

    fn cat_actor_floor() -> CatRawSupportFact {
        CatRawSupportFact {
            collider_layer: 2,
            collider_id: 17,
            ..cat_world_floor()
        }
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
        on_floor: bool,
        slides: Vec<Option<Vec<CatRawSupportFact>>>,
        outer_count_override: Option<i32>,
        inner_count_override: Option<(i32, i32)>,
        probe_hit: bool,
        probe_contacts: Vec<CatRawSupportFact>,
        probe_count_override: Option<i32>,
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
                on_floor: false,
                slides: Vec::new(),
                outer_count_override: None,
                inner_count_override: None,
                probe_hit: false,
                probe_contacts: Vec::new(),
                probe_count_override: None,
                trace: Vec::new(),
            }
        }

        fn effect_trace(&self) -> Vec<MotionTrace> {
            self.trace
                .iter()
                .copied()
                .filter(|entry| {
                    matches!(
                        entry,
                        MotionTrace::SetRotation(_)
                            | MotionTrace::SetVelocity(_)
                            | MotionTrace::MoveAndSlide
                            | MotionTrace::SetTransform(_)
                            | MotionTrace::Disable
                    )
                })
                .collect()
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

        fn is_on_floor(&mut self) -> bool {
            self.on_floor
        }

        fn read_slide_collision_count(&mut self) -> i32 {
            self.outer_count_override
                .unwrap_or_else(|| i32::try_from(self.slides.len()).unwrap_or(i32::MAX))
        }

        fn read_slide_contact_count(&mut self, slide: i32) -> Option<i32> {
            if let Some((target, count)) = self.inner_count_override
                && slide == target
            {
                return Some(count);
            }
            self.slides
                .get(usize::try_from(slide).ok()?)?
                .as_ref()
                .map(|contacts| i32::try_from(contacts.len()).unwrap_or(i32::MAX))
        }

        fn read_slide_contact_geometry(
            &mut self,
            slide: i32,
            contact: i32,
        ) -> Option<(Vector3, Vector3)> {
            self.slides
                .get(usize::try_from(slide).ok()?)?
                .as_ref()?
                .get(usize::try_from(contact).ok()?)
                .copied()
                .map(|fact| (fact.point, fact.normal))
        }

        fn read_slide_collider(&mut self, slide: i32, contact: i32) -> Option<(bool, u32, u64)> {
            self.slides
                .get(usize::try_from(slide).ok()?)?
                .as_ref()?
                .get(usize::try_from(contact).ok()?)
                .copied()
                .map(|fact| {
                    (
                        fact.collider_rid_valid,
                        fact.collider_layer,
                        fact.collider_id,
                    )
                })
        }

        fn probe_snap(&mut self, post_transform: Transform3D) -> bool {
            self.trace.push(MotionTrace::Probe(post_transform));
            self.probe_hit
        }

        fn read_probe_contact_count(&mut self) -> i32 {
            self.trace.push(MotionTrace::ReadProbeCount);
            self.probe_count_override
                .unwrap_or_else(|| i32::try_from(self.probe_contacts.len()).unwrap_or(i32::MAX))
        }

        fn read_probe_contact_geometry(&mut self, contact: i32) -> (Vector3, Vector3) {
            self.trace.push(MotionTrace::ReadProbeContact(contact));
            let fact = self
                .probe_contacts
                .get(usize::try_from(contact).unwrap_or(usize::MAX))
                .copied()
                .unwrap_or_else(cat_world_floor);
            (fact.point, fact.normal)
        }

        fn read_probe_collider(&mut self, contact: i32) -> (bool, u32, u64) {
            let fact = self
                .probe_contacts
                .get(usize::try_from(contact).unwrap_or(usize::MAX))
                .copied()
                .unwrap_or_else(cat_world_floor);
            (
                fact.collider_rid_valid,
                fact.collider_layer,
                fact.collider_id,
            )
        }

        fn write_global_transform(&mut self, transform: Transform3D) {
            self.trace.push(MotionTrace::SetTransform(transform));
            self.post_transform = transform;
        }

        fn disable_processing(&mut self) {
            self.trace.push(MotionTrace::Disable);
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
            motion: MotionState::initial(),
        }
    }

    fn assert_state_bits_eq(actual: &CatControlledState, expected: &CatControlledState) {
        assert_eq!(actual.brain.capture(), expected.brain.capture());
        assert_eq!(actual.gait.capture(), expected.gait.capture());
        assert_eq!(actual.tail.nodes(), expected.tail.nodes());
        assert_eq!(actual.presence.next_at(), expected.presence.next_at());
        assert_eq!(actual.sit.to_bits(), expected.sit.to_bits());
        assert_eq!(actual.sim_t.to_bits(), expected.sim_t.to_bits());
        assert_eq!(actual.motion, expected.motion);
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
    fn cat_valid_tick_calls_move_and_slide_once() {
        let seed = FakeCatMotionPort::valid();
        let prior = controlled_state(&seed);
        let mut port = seed.clone();
        let success = controlled_cat_tick(
            &mut port,
            &prior,
            1.0 / 60.0,
            SupportMotionConfig::CAT_DEFAULT,
        )
        .expect("a completely finite tick must commit");
        assert_eq!(
            port.trace
                .iter()
                .filter(|entry| matches!(entry, MotionTrace::MoveAndSlide))
                .count(),
            1
        );
        assert!(
            matches!(success.state.motion.phase(), MotionPhase::Airborne { .. }),
            "an unsupported controlled tick must depart to airborne, not stay planted"
        );
    }

    /// A grounded tick whose achieved Y differs from the prior tick's — a
    /// real step up or down, not a hover at constant elevation — must carry
    /// the tail's `transport_y(support_delta_y)` exactly once. A doubled
    /// call at the site in `controlled_cat_tick` survives all fixtures that
    /// hold elevation constant (delta 0 makes `transport_y`'s early return
    /// a no-op whichever number of times it runs), so this fixture forces a
    /// genuine nonzero delta through a full tick.
    ///
    /// The zero step duration is deliberate, not incidental: it makes every
    /// per-node ease step in `Tail::advance_nodes` a hard no-op (its rate
    /// factor is `(dt * rate).min(1.0)`), so the expected tail below can be
    /// predicted by calling the *same* already-tested `transport_y`/
    /// `advance` once each — without needing this test to also reproduce
    /// the internal yaw the brain would have driven, since yaw only reaches
    /// the eased (here suppressed) target.
    #[test]
    fn cat_controlled_tick_with_a_real_support_change_transports_the_tail_exactly_once() {
        let mut seed = FakeCatMotionPort::valid();
        seed.on_floor = true;
        seed.slides = vec![Some(vec![cat_world_floor()])];
        let step_up: f32 = 0.5;
        seed.post_transform.origin.y = seed.pre_transform.origin.y + step_up;

        let prior = controlled_state(&seed);
        let mut port = seed.clone();
        let success = controlled_cat_tick(&mut port, &prior, 0.0, SupportMotionConfig::CAT_DEFAULT)
            .expect("a grounded tick across a real, floor-supported step must commit");

        // confirm this fixture actually exercises a nonzero delta and stays
        // grounded — otherwise the test would prove nothing about
        // transport_y's call count
        assert_eq!(success.frame.support_delta_y.to_bits(), step_up.to_bits());
        assert!(
            matches!(success.state.motion.phase(), MotionPhase::Controlled),
            "a floor-supported step must stay grounded, not depart to airborne"
        );

        // independently predict the single-application tail by calling the
        // production Tail::transport_y/advance once each on a fresh copy of
        // the prior tail. `success.pose` is unaffected by how many times
        // the tick's own transport_y ran (it is built from position, yaw,
        // gait frame and sit — never from the tail), so re-deriving the
        // root through it, rather than through the tick's tail, keeps this
        // prediction independent of the very call this test is checking.
        let root =
            PosePoint::try_new(cat_body::skeleton(&success.pose).unwrap().tail_root).unwrap();
        let yaw = ActorYaw::try_new(f64::from(seed.pre_rotation.y)).unwrap();
        let support =
            crate::support_motion::SupportElevation::try_new(seed.post_transform.origin.y).unwrap();
        let mut expected_tail = prior.tail;
        expected_tail.transport_y(step_up).unwrap();
        expected_tail
            .advance(StepDuration::from_raw(0.0), root, yaw, support, 0.0, 0.0)
            .unwrap();

        assert_eq!(success.state.tail.nodes(), expected_tail.nodes());
    }

    /// A departing (Controlled-to-Airborne) tick must carry the brain's
    /// next progress sample forward to the ACHIEVED post-move position, not
    /// leave it pinned at the pre-move sample: a resumed brain's `progress`
    /// is `before.planar_distance(last_pos)`, so a stale `last_pos` left one
    /// tick behind would silently misreport how far the body has actually
    /// travelled once support is regained. `cat_valid_tick_calls_move_and_
    /// slide_once` already proves this exact fixture departs to Airborne;
    /// this test reuses it to pin the carried value bit-exact against
    /// `FakeCatMotionPort::valid()`'s deliberately distinct post transform.
    #[test]
    fn cat_departing_tick_carries_last_pos_to_the_achieved_position_not_the_stale_one() {
        let seed = FakeCatMotionPort::valid();
        let prior = controlled_state(&seed);
        let mut port = seed.clone();
        let success = controlled_cat_tick(
            &mut port,
            &prior,
            1.0 / 60.0,
            SupportMotionConfig::CAT_DEFAULT,
        )
        .expect("a completely finite tick must commit");
        assert!(
            matches!(success.state.motion.phase(), MotionPhase::Airborne { .. }),
            "this fixture is proven elsewhere to depart to airborne; a stale-last_pos test needs \
             that exact branch exercised"
        );
        let achieved = seed.post_transform.origin;
        assert_eq!(success.state.last_pos.x.to_bits(), achieved.x.to_bits());
        assert_eq!(success.state.last_pos.z.to_bits(), achieved.z.to_bits());
    }

    /// The gait's own leg-aim math (`cat_gait::step_leg`/`anchor`) anchors a
    /// swinging paw to whatever position it is fed THIS tick, so wiring in
    /// the pre-move `before` instead of the achieved `new_position` would
    /// aim every swinging paw at where the body USED to be. Leg 0 (LF) is
    /// guaranteed mid-swing the instant this fresh gait's phase leaves
    /// 0.0 (`OFFSET[0] = 0.25` puts `lp` at `0.75`, already past
    /// `DUTY = 0.70`), and `FakeCatMotionPort::valid()`'s pre/post
    /// displacement is fast enough to clear the walk-gate on this very
    /// first tick. Replay the same tick's gait state against the stale
    /// position on a clone: if production ever goes back to feeding
    /// `advance` a position that discards the achieved post-slide
    /// displacement, the two paw targets collapse to the same value.
    #[test]
    fn cat_gait_paw_targets_reflect_the_achieved_post_move_position_not_the_stale_one() {
        let seed = FakeCatMotionPort::valid();
        let prior = controlled_state(&seed);
        let stale_pos = ActorPosition::try_new(seed.pre_transform.origin).unwrap();
        let yaw = ActorYaw::try_new(f64::from(seed.pre_rotation.y)).unwrap();
        let duration = StepDuration::from_raw(1.0 / 60.0);
        let actual_speed = FiniteMeasure::try_new(
            ActorPosition::try_new(seed.post_transform.origin)
                .unwrap()
                .planar_distance(stale_pos)
                .value()
                / duration.seconds(),
            "test.speed",
        )
        .unwrap();
        let mut reference_gait = prior.gait.clone();
        let stale_frame = reference_gait
            .advance(duration, stale_pos, yaw, actual_speed)
            .expect("a completely finite reference tick must commit");

        let mut port = seed.clone();
        let success = controlled_cat_tick(
            &mut port,
            &prior,
            1.0 / 60.0,
            SupportMotionConfig::CAT_DEFAULT,
        )
        .expect("a completely finite tick must commit");

        assert_ne!(
            success.frame.paws[0].x.to_bits(),
            stale_frame.paws[0].x.to_bits(),
            "leg 0's swing aim must move with the achieved post-slide position, not stay pinned \
             to the stale pre-move sample: {:?} vs {:?}",
            success.frame.paws[0],
            stale_frame.paws[0]
        );
    }

    /// The frozen airborne policy must never invoke a yaw setter, not even
    /// to write back the exact value already there: comparing rotation
    /// VALUES before and after (as the GDScript suite does) cannot catch a
    /// policy that recommits the same yaw every tick, so this asserts the
    /// port's own effect trace carries no `SetRotation` at all while
    /// airborne.
    #[test]
    fn cat_airborne_policy_never_calls_a_yaw_setter_even_to_write_the_same_value() {
        let seed = FakeCatMotionPort::valid();
        let mut prior = controlled_state(&seed);
        prior.motion = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::ZERO,
                vertical_velocity_mps: crate::support_motion::FiniteVelocity::try_new(-1.0)
                    .unwrap(),
            },
            None,
            None,
        )
        .unwrap();
        let mut port = seed.clone();
        controlled_cat_tick(
            &mut port,
            &prior,
            1.0 / 60.0,
            SupportMotionConfig::CAT_DEFAULT,
        )
        .expect("a completely finite airborne tick must commit");
        assert!(
            !port
                .trace
                .iter()
                .any(|event| matches!(event, MotionTrace::SetRotation(_))),
            "an airborne tick must never invoke a yaw setter, even to write back the same \
             value: {:?}",
            port.trace
        );
    }

    #[test]
    fn cat_pre_move_poison_disables_without_move_or_support_scan() {
        let seed = FakeCatMotionPort::valid();
        let base_prior = controlled_state(&seed);
        let mut cases: Vec<(FakeCatMotionPort, CatControlledState)> = Vec::new();
        for lane in 0..12 {
            let mut port = seed.clone();
            port.pre_transform = poison_transform_lane(port.pre_transform, lane);
            cases.push((port, base_prior.clone()));
        }
        for lane in 0..3 {
            let mut port = seed.clone();
            port.pre_rotation = poison_vector_lane(port.pre_rotation, lane);
            cases.push((port, base_prior.clone()));
        }
        for lane in 0..3 {
            let mut port = seed.clone();
            port.pre_velocity = poison_vector_lane(port.pre_velocity, lane);
            cases.push((port, base_prior.clone()));
        }
        for lane in 0..3 {
            let mut poisoned_prior = base_prior.clone();
            poisoned_prior.last_pos = poison_vector_lane(poisoned_prior.last_pos, lane);
            cases.push((seed.clone(), poisoned_prior));
        }

        for (mut port, prior) in cases {
            let before = prior.clone();
            let result = controlled_cat_tick(
                &mut port,
                &prior,
                1.0 / 60.0,
                SupportMotionConfig::CAT_DEFAULT,
            );
            assert!(result.is_err(), "a poisoned pre-move sample was accepted");
            assert_state_bits_eq(&prior, &before);
            assert!(
                !port
                    .trace
                    .iter()
                    .any(|event| matches!(event, MotionTrace::MoveAndSlide)),
                "a pre-move refusal must never call move_and_slide: {:?}",
                port.trace
            );
            assert_eq!(
                port.effect_trace(),
                [MotionTrace::Disable],
                "a pre-move refusal must disable and touch no other effect lane"
            );
        }
    }

    #[test]
    fn cat_post_move_poison_writes_exact_saved_transform_then_zero_velocity_then_disables() {
        let seed = FakeCatMotionPort::valid();
        let prior = controlled_state(&seed);
        let mut cases = Vec::new();
        for lane in 0..12 {
            let mut port = seed.clone();
            port.post_transform = poison_transform_lane(port.post_transform, lane);
            cases.push(port);
        }
        for lane in 0..3 {
            let mut port = seed.clone();
            port.post_rotation = poison_vector_lane(port.post_rotation, lane);
            cases.push(port);
        }
        for lane in 0..3 {
            let mut port = seed.clone();
            port.post_velocity = poison_vector_lane(port.post_velocity, lane);
            cases.push(port);
        }
        for lane in 0..6 {
            let mut port = seed.clone();
            port.on_floor = true;
            let mut fact = cat_world_floor();
            if lane < 3 {
                fact.point = poison_vector_lane(fact.point, lane);
            } else {
                fact.normal = poison_vector_lane(fact.normal, lane - 3);
            }
            port.slides = vec![Some(vec![fact])];
            cases.push(port);
        }

        for mut port in cases {
            let saved = port.pre_transform;
            let result = controlled_cat_tick(
                &mut port,
                &prior,
                1.0 / 60.0,
                SupportMotionConfig::CAT_DEFAULT,
            );
            assert!(result.is_err(), "a poisoned post-move sample was accepted");
            assert_transform_bits_eq(port.post_transform, saved);
            assert_eq!(port.post_velocity, Vector3::ZERO);
            assert_eq!(
                port.effect_trace(),
                [
                    MotionTrace::SetRotation(seed.pre_rotation),
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::MoveAndSlide,
                    MotionTrace::SetTransform(saved),
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::Disable,
                ],
                "a post-move refusal must restore the saved transform, zero velocity, then disable"
            );
        }
    }

    #[test]
    fn cat_support_reader_scans_nested_contacts_actor_then_world_and_preserves_zero_id() {
        let mut port = FakeCatMotionPort::valid();
        port.on_floor = true;
        port.slides = vec![
            Some(vec![cat_actor_floor()]),
            Some(vec![
                CatRawSupportFact {
                    collider_id: 0,
                    ..cat_world_floor()
                },
                cat_world_floor(),
            ]),
        ];
        let transform = ActorTransform::try_new(port.post_transform).unwrap();
        let (support, collider_id) = read_cat_post_move_support(&mut port, transform).unwrap();
        assert!(support.is_some(), "the first non-actor floor must be kept");
        assert_eq!(
            collider_id, None,
            "a zero collider id must surface as no id, never a fabricated one"
        );

        port.slides = vec![Some(vec![cat_actor_floor(), cat_world_floor()])];
        let (support, collider_id) = read_cat_post_move_support(&mut port, transform).unwrap();
        assert!(support.is_some());
        assert_eq!(collider_id, Some(cat_world_floor().collider_id));

        port.on_floor = false;
        port.slides = vec![Some(vec![cat_world_floor()])];
        let (support, collider_id) = read_cat_post_move_support(&mut port, transform).unwrap();
        assert_eq!(support, None, "a hidden floor must never be read at all");
        assert_eq!(collider_id, None);
    }

    /// An ordinary ledger that saw a floorish contact on *every* slide, but
    /// only ever an actor's own collider, must read as no support and must
    /// never fall back to the snap probe — the probe exists to find a
    /// *hidden* world floor, not to look past an actor floor the ordinary
    /// ledger already saw and rejected. This is the named case behind
    /// `read_cat_post_move_support`'s `if saw_floorish { return Ok((None,
    /// None)); }` early return: every other existing test either supplies a
    /// genuine world floor somewhere in the ledger (so `candidate` is
    /// already `Some` before that line is reached) or a non-floorish wall
    /// contact (so `saw_floorish` never becomes true), so neither one can
    /// observe this line at all.
    #[test]
    fn cat_support_reader_never_probes_when_every_slide_saw_only_an_actor_floor() {
        let mut port = FakeCatMotionPort::valid();
        port.on_floor = true;
        port.slides = vec![Some(vec![cat_actor_floor()]), Some(vec![cat_actor_floor()])];
        port.probe_hit = true;
        port.probe_contacts = vec![cat_world_floor()];
        let transform = ActorTransform::try_new(port.post_transform).unwrap();

        let (support, collider_id) = read_cat_post_move_support(&mut port, transform).unwrap();

        assert_eq!(
            support, None,
            "an actor-only floorish ledger must never be read as support"
        );
        assert_eq!(collider_id, None);
        assert!(
            !port
                .trace
                .iter()
                .any(|event| matches!(event, MotionTrace::Probe(_))),
            "an actor-only floorish ledger must never fall back to the snap probe"
        );
    }

    #[test]
    fn cat_support_reader_rejects_every_poisoned_lane_and_bad_count() {
        let mut port = FakeCatMotionPort::valid();
        port.on_floor = true;
        let transform = ActorTransform::try_new(port.post_transform).unwrap();

        port.outer_count_override = Some(MAX_SLIDES + 1);
        assert_eq!(
            read_cat_post_move_support(&mut port, transform),
            Err(CatSupportReadError::InvalidOuterCount(MAX_SLIDES + 1))
        );
        port.outer_count_override = None;

        port.slides = vec![None];
        assert_eq!(
            read_cat_post_move_support(&mut port, transform),
            Err(CatSupportReadError::MissingSlide(0))
        );

        port.slides = vec![Some(vec![cat_world_floor()])];
        port.inner_count_override = Some((0, MOTION_RESULT_MAX_CONTACTS + 1));
        assert_eq!(
            read_cat_post_move_support(&mut port, transform),
            Err(CatSupportReadError::InvalidInnerCount {
                slide: 0,
                count: MOTION_RESULT_MAX_CONTACTS + 1
            })
        );
        port.inner_count_override = None;

        let mut invalid_rid = cat_world_floor();
        invalid_rid.collider_rid_valid = false;
        port.slides = vec![Some(vec![invalid_rid])];
        assert_eq!(
            read_cat_post_move_support(&mut port, transform),
            Err(CatSupportReadError::InvalidOrdinaryRid {
                slide: 0,
                contact: 0
            })
        );

        let mut poisoned = cat_world_floor();
        poisoned.point.x = f32::NAN;
        port.slides = vec![Some(vec![poisoned])];
        assert!(matches!(
            read_cat_post_move_support(&mut port, transform),
            Err(CatSupportReadError::InvalidValue(_))
        ));

        // An empty contact list is a distinct, explicit refusal from a
        // missing slide entirely: zero contacts fails the inner count
        // range, it is never silently read as "no slide here".
        port.slides = vec![Some(vec![])];
        assert_eq!(
            read_cat_post_move_support(&mut port, transform),
            Err(CatSupportReadError::InvalidInnerCount { slide: 0, count: 0 })
        );
    }

    #[test]
    fn cat_snap_probe_runs_only_for_a_hidden_floor_and_rejects_its_own_poisoned_lanes() {
        let mut port = FakeCatMotionPort::valid();
        let transform = ActorTransform::try_new(port.post_transform).unwrap();

        // No ordinary ledger contact at all, and the physics floor flag is
        // false: the probe must never fire.
        port.on_floor = false;
        port.probe_hit = true;
        port.probe_contacts = vec![cat_world_floor()];
        let (support, _) = read_cat_post_move_support(&mut port, transform).unwrap();
        assert_eq!(support, None);
        assert!(
            !port
                .trace
                .iter()
                .any(|event| matches!(event, MotionTrace::Probe(_))),
            "an unset floor flag must never run the cached snap probe"
        );

        // A floor flag with a wall-only ordinary ledger (no floorish contact)
        // must fall back to the probe.
        let mut wall = cat_world_floor();
        wall.normal = Vector3::new(1.0, 0.0, 0.0);
        port.on_floor = true;
        port.slides = vec![Some(vec![wall])];
        port.probe_hit = true;
        port.probe_contacts = vec![cat_world_floor()];
        let (support, collider_id) = read_cat_post_move_support(&mut port, transform).unwrap();
        assert!(support.is_some(), "the probe fallback must be read");
        assert_eq!(collider_id, Some(cat_world_floor().collider_id));

        // A floor flag whose probe misses yields no support and no error.
        port.probe_hit = false;
        let (support, collider_id) = read_cat_post_move_support(&mut port, transform).unwrap();
        assert_eq!(support, None);
        assert_eq!(collider_id, None);

        // The probe's own count and RID lanes are validated exactly like the
        // ordinary ledger's.
        port.probe_hit = true;
        port.probe_count_override = Some(SNAP_PROBE_MAX_CONTACTS + 1);
        assert_eq!(
            read_cat_post_move_support(&mut port, transform),
            Err(CatSupportReadError::InvalidProbeCount(
                SNAP_PROBE_MAX_CONTACTS + 1
            ))
        );
        port.probe_count_override = None;

        let mut invalid_probe_rid = cat_world_floor();
        invalid_probe_rid.collider_rid_valid = false;
        port.probe_contacts = vec![invalid_probe_rid];
        assert_eq!(
            read_cat_post_move_support(&mut port, transform),
            Err(CatSupportReadError::InvalidProbeRid(0))
        );
    }

    /// The snap-probe fallback must skip an actor-layer contact exactly
    /// like the ordinary ledger scan does — a cat probing under itself and
    /// finding another actor's collider before the world floor must not
    /// accept that actor as support (`actor_support_test.gd`'s "walking off
    /// world onto cat" law depends on this holding at both call sites, not
    /// only the first one the ordinary ledger scan reaches).
    #[test]
    fn cat_snap_probe_skips_an_actor_layer_contact_and_keeps_the_world_floor() {
        let mut port = FakeCatMotionPort::valid();
        let transform = ActorTransform::try_new(port.post_transform).unwrap();
        port.on_floor = true;
        port.probe_hit = true;
        port.probe_contacts = vec![cat_actor_floor(), cat_world_floor()];
        let (support, collider_id) = read_cat_post_move_support(&mut port, transform).unwrap();
        assert!(support.is_some(), "the first non-actor floor must be kept");
        assert_eq!(collider_id, Some(cat_world_floor().collider_id));
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

        // A body_yaw that already IS the canonical f32 engine image of some
        // yaw (so the trailing `try_replacing_yaw` guard would accept it on
        // its own terms) but of a DIFFERENT yaw than brain_yaw (so it is not
        // the canonical image the cross-owner equality guard demands). Only
        // the first guard can reject this value; if that guard were deleted,
        // the second would wave it through. Derived through
        // `canonicalize_replacing_yaw` at test runtime, never a hard-coded
        // bit pattern, per the cross-platform-libm caution on this file.
        let other_yaw = 0.9_f32;
        let owned_by_other_yaw = f64::from(
            GodotRotation::canonicalize_replacing_yaw(current_full, other_yaw)
                .unwrap()
                .world()
                .y,
        );
        assert_ne!(
            owned_by_other_yaw.to_bits(),
            canonical_body_yaw.to_bits(),
            "the isolating case must not silently collapse onto brain_yaw's own canonical image"
        );

        let cases = [
            (
                "pose.yaw",
                "must match brain.yaw bit-for-bit",
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
                "must match gait.amp bit-for-bit",
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
                "must match the cat sit blend bit-for-bit",
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
                "must match the canonical f32 engine image of brain.yaw",
                [0.2, brain_yaw, brain_yaw, 0.375, 0.375, 0.625, 0.625],
            ),
            (
                "yaw",
                "must match the canonical f32 engine image of brain.yaw",
                [
                    owned_by_other_yaw,
                    brain_yaw,
                    brain_yaw,
                    0.375,
                    0.375,
                    0.625,
                    0.625,
                ],
            ),
        ];
        for (path, rule, values) in cases {
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
            assert_eq!(error.rule, rule);
        }
    }
}

#[cfg(all(test, feature = "editor-docs"))]
mod editor_docs_tests {
    #[test]
    fn cat_motion_property_purpose_units_and_threshold_rule_reach_editor_docs() {
        let xml = godot::docs::gather_xml_docs()
            .find(|xml| xml.contains("<class name=\"WaveCat\""))
            .expect("WaveCat must register an editor-docs XML class");
        for phrase in [
            "Downward acceleration in metres per second squared",
            "Maximum downward speed in metres per second this cat may reach",
            "must remain below Landing Full Speed",
            "must exceed Landing Silent Speed",
            "Maximum authored landing-wave gain for this cat, unitless",
            "Maximum authored landing-wave radius in metres for this cat",
            "staged into the active configuration only once the",
        ] {
            assert!(
                xml.contains(phrase),
                "WaveCat editor XML omitted `{phrase}`: {xml}"
            );
        }
    }
}
