//! The blind hero as an engine node: first-person movement, mouse look,
//! cane taps — player.gd carried into the Rust layer verbatim.
//!
//! The cane is the ONLY deliberate instrument. A tap picks its mode by
//! what a real ~1.7 m arm-plus-cane could actually touch:
//! - aimed strike — the 3D gaze ray connects within reach: the wave is
//!   born exactly where the player looked (wall, furniture, floor);
//! - rest tap — no aimed hit: the tap lands wherever the cane tip is
//!   physically resting (tabletop, chair seat, or — when the player is
//!   looking down — the floor);
//! - air swish — the cane rests on nothing raised and the player is not
//!   aiming down: NO wave. Air reflects nothing.
//!
//! PHYSICS CONTEXT: every raycast in the game runs inside the physics
//! tick. Input handlers only queue intent; the hero body and the
//! composition root queue wave requests. This keeps all space queries
//! inside Godot's supported physics window.

use godot::classes::character_body_3d::{MotionMode, PlatformOnLeave};
use godot::classes::{
    Camera3D, CapsuleShape3D, CharacterBody3D, CollisionShape3D, ICharacterBody3D, Input,
    InputEvent, InputEventKey, InputEventMouseButton, InputEventMouseMotion, InputMap, Os,
    PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D, PhysicsServer3D,
    PhysicsTestMotionParameters3D, PhysicsTestMotionResult3D, input,
};
use godot::global::{Key, MouseButton};
use godot::prelude::*;
use std::fmt;
use std::num::NonZeroU64;

use super::support::{
    FLOOR_MAX_ANGLE_RAD, FLOOR_SNAP_M, MAX_SLIDES, MOTION_RESULT_MAX_CONTACTS, PLATFORM_LAYERS,
    SAFE_MARGIN_M, SNAP_PROBE_MAX_CONTACTS, collision_pair, is_actor_layer,
};
use crate::hero_visual::{
    CAM_BASE_Y, CANE_FLOOR_VOICE, CANE_FULL_VOICE, CANE_REACH, CANE_SCAN_LENGTH, CaneVerticals,
    LandingPreparationError, PLAYER_STANDING_ROOT_Y, PreparedCaneRequest, PreparedFootstepRequest,
    PreparedLandingRequest, PreparedLastTap, RestTapVerdict, WALL_BACKOFF, aimed_strike_voice,
    cane_aim_ray, cane_settle_ray, cane_tip_column, cane_wall_scan_ray, horizontal_aim,
    prepare_cane_request, rest_tap_verdict, settle_cane_rest, swish_target,
};
use crate::observe::QueuedWave;
use crate::observe::reflect::{CheckedReflectionRequest, ReflectionRequest};
use crate::pulse_pool::{CheckedWave, OMNI_COS};
use crate::render;
use crate::reproduce::{HeroCapture, RestoreValueError};
use crate::support_motion::{
    ActorPosition, ActorTransform, ActorVelocity, FiniteRotation, FootstepSuppression,
    GodotRotation, LandingEvent, MotionOutcome, MotionPhase, MotionRestoreError, MotionState,
    MotionValueError, PlanarVelocity, PosePoint, QueuedWaveGate, StepDuration, SupportContact,
    SupportElevation, SupportMotionConfig, prepare, reconcile, validate_restore,
};
use crate::temporal::{PreparedTime, prepare_time};
use crate::viewmodel::PlanarAxes;

/// Capsule centre relative to the authored standing root.
const PLAYER_CAPSULE_CENTER_Y_M: f32 = -0.05;

/// Walk speed, m/s — a careful walk, not a run.
pub const SPEED: f64 = 2.1;

/// Seconds a too-eager second tap is swallowed for.
pub const TAP_COOLDOWN: f64 = 0.15;

/// Radians per pixel of mouse motion, both axes.
pub const MOUSE_SENS: f64 = 0.0026;

/// Radians the eye may pitch up or down.
pub const PITCH_LIMIT: f64 = 1.35;

/// Move actions bind PHYSICAL keycodes so WASD works on any keyboard
/// layout (ЦФЫВ on Russian, ZQSD keys on AZERTY, etc.).
const MOVE_KEYS: [(&str, Key); 4] = [
    ("move_forward", Key::W),
    ("move_left", Key::A),
    ("move_back", Key::S),
    ("move_right", Key::D),
];

/// Where the cane tip naturally rests, and whether any surface actually
/// holds it up (false over open air at floor level). A registered class:
/// the hero body and the suites read it straight off the player.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct CaneRest {
    /// The resting tip position, settled 0.02 m above its support.
    #[var]
    pub(crate) tip: Vector3,
    /// True when a surface holds the tip up — bare floor included;
    /// "unsupported" is reserved for true open air.
    #[var]
    pub(crate) supported: bool,
    base: Base<RefCounted>,
}

/// What a tap or footstep asks of the wave pool — carried whole from the
/// input/frame context into the physics tick where raycasts may run.
struct WaveRequest {
    kind: i64,
    at: Vector3,
    max_r: f64,
    speed: f64,
    gain: f64,
    echoes: i64,
    normal: Vector3,
    gate: QueuedWaveGate,
    /// The instant the request's admission proofs were made, when one
    /// exists: the emitted wave is born at THAT instant, never a re-read
    /// clock. General and restored requests carry none and use the
    /// draining tick's clock, exactly as before.
    prepared_at: Option<f64>,
}

struct PreparedWaveRequest {
    request: WaveRequest,
    _wave_proof: CheckedWave,
    _reflection_proof: CheckedReflectionRequest,
}

pub(super) struct PreparedPlayerState {
    position: Vector3,
    velocity: Vector3,
    rotation: Vector3,
    eye: Gd<Camera3D>,
    eye_rotation: Vector3,
    motion: MotionState,
    collision_layer: u32,
    collision_mask: u32,
    last_tap: f64,
    tap_target: Vector3,
    tap_queued: bool,
    wave_queue: Vec<PreparedWaveRequest>,
    footstep_suppression: FootstepSuppression,
    now: PreparedTime,
}

/// A cane-rest probe before it is published: the raw physics answer.
struct RestProbe {
    tip: Vector3,
    supported: bool,
}

/// One bounded physics ray's answer through the cane port. The Godot
/// mapping is closed: an empty dictionary is `Miss`, a jointly present
/// correctly typed position/normal pair is `Hit` (other metadata is
/// irrelevant), and a partial or wrongly typed required pair is
/// `Malformed`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CaneRayAnswer {
    Miss,
    Hit { position: Vector3, normal: Vector3 },
    Malformed,
}

/// The cane's narrow world-query contract: raw player/camera samples and
/// bounded rays, nothing else — no emitter and no scene mutation. The
/// production port and the cargo fakes drive the same coordinators.
trait CaneQueryPort {
    fn player_transform(&mut self) -> Transform3D;
    fn camera_transform(&mut self) -> Option<Transform3D>;
    fn camera_rotation(&mut self) -> Option<Vector3>;
    fn cast_ray(&mut self, from: Vector3, to: Vector3) -> CaneRayAnswer;
}

/// Why a cane boundary operation refused before changing any state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaneQueryError {
    Sample,
    Endpoint,
    Hit,
    Request,
}

impl fmt::Display for CaneQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Sample => "poisoned or missing player/camera sample",
            Self::Endpoint => "derived query endpoint left the pose envelope",
            Self::Hit => "malformed or poisoned physics answer",
            Self::Request => "prepared cane command refused",
        })
    }
}

/// Proof that a camera handle is this player's own live eye — produced
/// only by [`UnseeingPlayer::prove_visual_camera`] and consumed exactly
/// once by the visual commit door. Holding the proven handle itself makes
/// the door's camera write unconditional.
pub(super) struct VisualCameraProof(Gd<Camera3D>);

/// A completely decided tap, staged for one adapter install: the spent
/// intent, the advanced tap clock, the strike target, and the optional
/// prepared reflecting command. A swish stages no request.
#[derive(Debug)]
struct PreparedCaneTap {
    last_tap: f64,
    tap_target: Vector3,
    request: Option<PreparedCaneRequest>,
}

/// One validated player sample for cane work — position, the support
/// verticals derived from that same read, and the horizontal facing.
fn validated_cane_player<P: CaneQueryPort>(
    port: &mut P,
) -> Result<(Vector3, CaneVerticals, Vector3), CaneQueryError> {
    let transform = port.player_transform();
    ActorTransform::try_new(transform).map_err(|_| CaneQueryError::Sample)?;
    let position = ActorPosition::try_new(transform.origin).map_err(|_| CaneQueryError::Sample)?;
    let support = support_elevation_at(position.world()).map_err(|_| CaneQueryError::Sample)?;
    let axes = PlanarAxes::try_new(-transform.basis.col_c(), transform.basis.col_a())
        .map_err(|_| CaneQueryError::Sample)?;
    Ok((
        position.world(),
        CaneVerticals::new(support),
        axes.forward(),
    ))
}

/// The rest settle from an already validated player sample: a wall scan
/// shortens the reach, then a settle probe finds the first supporting
/// surface below the tip. The coordinator only sequences the two port
/// queries — every endpoint, answer, and settled tip law lives in the
/// pure owner.
fn cane_rest_probe<P: CaneQueryPort>(
    port: &mut P,
    gp: Vector3,
    verticals: CaneVerticals,
    direction: Vector3,
) -> Result<RestProbe, CaneQueryError> {
    let (from, to) =
        cane_wall_scan_ray(verticals, gp, direction).map_err(|_| CaneQueryError::Endpoint)?;
    let wall_d = match port.cast_ray(from, to) {
        CaneRayAnswer::Miss => CANE_SCAN_LENGTH,
        CaneRayAnswer::Hit { position, .. } => {
            let wall = PosePoint::try_new(position).map_err(|_| CaneQueryError::Hit)?;
            f64::from((wall.world() - from).length())
        }
        CaneRayAnswer::Malformed => return Err(CaneQueryError::Hit),
    };
    let (px, pz) = cane_tip_column(gp, direction, wall_d);
    let (top, bottom) = cane_settle_ray(verticals, px, pz).map_err(|_| CaneQueryError::Endpoint)?;
    let struck_y = match port.cast_ray(top, bottom) {
        CaneRayAnswer::Miss => None,
        CaneRayAnswer::Hit { position, .. } => Some(
            PosePoint::try_new(position)
                .map_err(|_| CaneQueryError::Hit)?
                .world()
                .y,
        ),
        CaneRayAnswer::Malformed => return Err(CaneQueryError::Hit),
    };
    let (tip, supported) =
        settle_cane_rest(verticals, px, pz, struck_y).map_err(|_| CaneQueryError::Endpoint)?;
    Ok(RestProbe { tip, supported })
}

/// The physics-tick cane rest for one sweep offset — published by the
/// adapter only on complete success.
fn prepare_cane_rest<P: CaneQueryPort>(
    port: &mut P,
    yaw_offset: f64,
) -> Result<RestProbe, CaneQueryError> {
    if !yaw_offset.is_finite() {
        return Err(CaneQueryError::Sample);
    }
    let (gp, verticals, forward) = validated_cane_player(port)?;
    let direction = forward.rotated(Vector3::UP, yaw_offset as f32);
    cane_rest_probe(port, gp, verticals, direction)
}

/// One queued tap decided completely — aimed strike, rest tap, or silent
/// swish — with every vertical judged from the player's own support.
/// `Ok(None)` is the cooldown swallow: intent spent, nothing else moves.
fn prepare_cane_tap<P: CaneQueryPort>(
    port: &mut P,
    now: PreparedTime,
    prior_tap: PreparedLastTap,
) -> Result<Option<PreparedCaneTap>, CaneQueryError> {
    if now.value() - prior_tap.raw() < TAP_COOLDOWN {
        return Ok(None);
    }
    let Some(camera_transform) = port.camera_transform() else {
        return Err(CaneQueryError::Sample);
    };
    ActorTransform::try_new(camera_transform).map_err(|_| CaneQueryError::Sample)?;
    let Some(camera_rotation) = port.camera_rotation() else {
        return Err(CaneQueryError::Sample);
    };
    FiniteRotation::try_new(camera_rotation).map_err(|_| CaneQueryError::Sample)?;
    let (gp, verticals, forward) = validated_cane_player(port)?;
    let pitch = f64::from(camera_rotation.x);
    let aim = -camera_transform.basis.col_c();
    let (from, to) =
        cane_aim_ray(camera_transform.origin, aim).map_err(|_| CaneQueryError::Endpoint)?;
    match port.cast_ray(from, to) {
        CaneRayAnswer::Malformed => Err(CaneQueryError::Hit),
        CaneRayAnswer::Hit { position, normal } => {
            // aimed strike: the wave is born exactly where you looked
            let strike = PosePoint::try_new(position).map_err(|_| CaneQueryError::Hit)?;
            for lane in [normal.x, normal.y, normal.z] {
                if !lane.is_finite() {
                    return Err(CaneQueryError::Hit);
                }
            }
            let (max_r, gain) = aimed_strike_voice(verticals, strike.world().y, normal.y);
            let request = prepare_cane_request(strike, max_r, gain, normal, now)
                .map_err(|_| CaneQueryError::Request)?;
            Ok(Some(PreparedCaneTap {
                last_tap: now.value(),
                tap_target: strike.world(),
                request: Some(request),
            }))
        }
        CaneRayAnswer::Miss => {
            let rest = cane_rest_probe(port, gp, verticals, forward)?;
            match rest_tap_verdict(verticals, rest.supported, rest.tip.y, pitch) {
                verdict @ (RestTapVerdict::Raised | RestTapVerdict::Floor) => {
                    // no aim needed: tap whatever the cane is physically
                    // resting on — tabletop, chair seat, or the floor
                    let tip = PosePoint::try_new(rest.tip).map_err(|_| CaneQueryError::Endpoint)?;
                    let (max_r, gain) = if verdict == RestTapVerdict::Raised {
                        CANE_FULL_VOICE
                    } else {
                        CANE_FLOOR_VOICE
                    };
                    let request = prepare_cane_request(tip, max_r, gain, Vector3::UP, now)
                        .map_err(|_| CaneQueryError::Request)?;
                    Ok(Some(PreparedCaneTap {
                        last_tap: now.value(),
                        tap_target: tip.world(),
                        request: Some(request),
                    }))
                }
                RestTapVerdict::Swish => {
                    // air swish: the cane sweeps up through nothing; air
                    // reflects nothing — only the strike animation
                    // remembers. An exactly vertical eye has no swish
                    // direction: refuse the degenerate camera instead of
                    // normalizing zero.
                    let Some(flat) = horizontal_aim(aim) else {
                        return Err(CaneQueryError::Sample);
                    };
                    let target = swish_target(verticals, from, flat, pitch)
                        .map_err(|_| CaneQueryError::Endpoint)?;
                    Ok(Some(PreparedCaneTap {
                        last_tap: now.value(),
                        tap_target: target.world(),
                        request: None,
                    }))
                }
            }
        }
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq)]
struct RawSupportFact {
    point: Vector3,
    normal: Vector3,
    collider_rid_valid: bool,
    collider_layer: u32,
    collider_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SupportReadError {
    InvalidOuterCount(i32),
    MissingSlide(i32),
    InvalidInnerCount { slide: i32, count: i32 },
    InvalidOrdinaryRid { slide: i32, contact: i32 },
    InvalidProbeCount(i32),
    InvalidProbeRid(i32),
    InvalidValue(MotionValueError),
}

impl From<MotionValueError> for SupportReadError {
    fn from(error: MotionValueError) -> Self {
        Self::InvalidValue(error)
    }
}

impl fmt::Display for SupportReadError {
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

trait PlayerMotionPort {
    fn read_global_transform(&mut self) -> Transform3D;
    fn read_global_rotation(&mut self) -> Vector3;
    fn read_velocity(&mut self) -> Vector3;
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
    fn disable_physics(&mut self);
}

/// One completed motion tick, carried whole into the callback commit: the
/// phase the tick left, the state it produced, ONLY the fresh landing
/// event (never a restored memory), the already prepared audible landing
/// voice, the accepted support identity, and the derived — not yet
/// applied — collision pair. It owns a checked reflection fan, so it is
/// deliberately move-only: `Debug` for diagnostics, nothing else.
#[derive(Debug)]
struct PlayerTickSuccess {
    phase_before: MotionPhase,
    state: MotionState,
    landing: Option<LandingEvent>,
    landing_request: Option<PreparedLandingRequest>,
    support_collider_id: Option<u64>,
    collision_layer: u32,
    collision_mask: u32,
}

#[derive(Debug, Clone, Copy)]
struct PreparedPlayerPreMove {
    saved_transform: Transform3D,
    now: PreparedTime,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PlayerTickReason {
    Motion(MotionValueError),
    Support(SupportReadError),
    Clock(&'static str),
    Landing(LandingPreparationError),
}

impl fmt::Display for PlayerTickReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Motion(error) => error.fmt(formatter),
            Self::Support(error) => error.fmt(formatter),
            Self::Clock(rule) => formatter.write_str(rule),
            Self::Landing(error) => error.fmt(formatter),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlayerTickFault {
    phase: &'static str,
    reason: PlayerTickReason,
}

fn floor_angle_accepts(contact: SupportContact) -> bool {
    let normal = contact.normal();
    let x = f64::from(normal.x);
    let y = f64::from(normal.y);
    let z = f64::from(normal.z);
    let length = (x * x + y * y + z * z).sqrt();
    let cosine = (y / length).clamp(-1.0, 1.0);
    cosine.acos() <= f64::from(FLOOR_MAX_ANGLE_RAD)
}

fn read_post_move_support<P: PlayerMotionPort>(
    port: &mut P,
    post_transform: ActorTransform,
) -> Result<(Option<SupportContact>, Option<u64>), SupportReadError> {
    if !port.is_on_floor() {
        return Ok((None, None));
    }

    let slide_count = port.read_slide_collision_count();
    if !(0..=MAX_SLIDES).contains(&slide_count) {
        return Err(SupportReadError::InvalidOuterCount(slide_count));
    }
    let mut candidate = None;
    let mut saw_floorish = false;
    for slide in 0..slide_count {
        let contact_count = port
            .read_slide_contact_count(slide)
            .ok_or(SupportReadError::MissingSlide(slide))?;
        if !(1..=MOTION_RESULT_MAX_CONTACTS).contains(&contact_count) {
            return Err(SupportReadError::InvalidInnerCount {
                slide,
                count: contact_count,
            });
        }
        for contact_index in 0..contact_count {
            let (point, normal) = port
                .read_slide_contact_geometry(slide, contact_index)
                .ok_or(SupportReadError::MissingSlide(slide))?;
            let contact = SupportContact::try_new(point, normal)?;
            if !floor_angle_accepts(contact) {
                continue;
            }
            saw_floorish = true;
            let (collider_rid_valid, collider_layer, collider_id) = port
                .read_slide_collider(slide, contact_index)
                .ok_or(SupportReadError::MissingSlide(slide))?;
            if !collider_rid_valid {
                return Err(SupportReadError::InvalidOrdinaryRid {
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
        return Err(SupportReadError::InvalidProbeCount(contact_count));
    }
    let mut candidate = None;
    for contact_index in 0..contact_count {
        let (point, normal) = port.read_probe_contact_geometry(contact_index);
        let contact = SupportContact::try_new(point, normal)?;
        if !floor_angle_accepts(contact) {
            continue;
        }
        let (collider_rid_valid, collider_layer, collider_id) =
            port.read_probe_collider(contact_index);
        if !collider_rid_valid {
            return Err(SupportReadError::InvalidProbeRid(contact_index));
        }
        if !is_actor_layer(collider_layer) && candidate.is_none() {
            candidate = Some((contact, NonZeroU64::new(collider_id).map(NonZeroU64::get)));
        }
    }
    Ok(candidate.map_or((None, None), |(support, collider_id)| {
        (Some(support), collider_id)
    }))
}

fn refuse_player_before_move<P: PlayerMotionPort>(
    port: &mut P,
    phase: &'static str,
    error: MotionValueError,
) -> PlayerTickFault {
    port.write_velocity(Vector3::ZERO);
    port.disable_physics();
    PlayerTickFault {
        phase,
        reason: PlayerTickReason::Motion(error),
    }
}

fn rollback_player_motion<P: PlayerMotionPort>(
    port: &mut P,
    saved_transform: Transform3D,
    phase: &'static str,
    reason: PlayerTickReason,
) -> PlayerTickFault {
    port.write_global_transform(saved_transform);
    port.write_velocity(Vector3::ZERO);
    port.disable_physics();
    PlayerTickFault { phase, reason }
}

fn prepare_player_pre_move<P: PlayerMotionPort>(
    port: &mut P,
    raw_now: f64,
) -> Result<PreparedPlayerPreMove, PlayerTickFault> {
    let now = match prepare_time(raw_now) {
        Ok(now) => now,
        Err(error) => {
            port.write_velocity(Vector3::ZERO);
            port.disable_physics();
            return Err(PlayerTickFault {
                phase: "current time",
                reason: PlayerTickReason::Clock(error.rule),
            });
        }
    };
    let saved_transform = port.read_global_transform();
    ActorTransform::try_new(saved_transform)
        .map_err(|error| refuse_player_before_move(port, "physics transform", error))?;
    FiniteRotation::try_new(port.read_global_rotation())
        .map_err(|error| refuse_player_before_move(port, "physics rotation", error))?;
    ActorVelocity::try_new(port.read_velocity())
        .map_err(|error| refuse_player_before_move(port, "physics velocity", error))?;
    Ok(PreparedPlayerPreMove {
        saved_transform,
        now,
    })
}

fn controlled_player_tick_from_pre_move<P: PlayerMotionPort>(
    port: &mut P,
    pre_move: PreparedPlayerPreMove,
    prior: MotionState,
    desired_planar: PlanarVelocity,
    raw_dt: f64,
    config: SupportMotionConfig,
) -> Result<PlayerTickSuccess, PlayerTickFault> {
    let saved_transform = pre_move.saved_transform;

    let prepared = prepare(
        prior,
        desired_planar,
        StepDuration::from_raw(raw_dt),
        config,
    );
    port.write_velocity(prepared.command().world_velocity());
    port.move_and_slide_once();

    let post_transform =
        ActorTransform::try_new(port.read_global_transform()).map_err(|error| {
            rollback_player_motion(
                port,
                saved_transform,
                "post-move transform",
                PlayerTickReason::Motion(error),
            )
        })?;
    FiniteRotation::try_new(port.read_global_rotation()).map_err(|error| {
        rollback_player_motion(
            port,
            saved_transform,
            "post-move rotation",
            PlayerTickReason::Motion(error),
        )
    })?;
    let (support, collider_id) = read_post_move_support(port, post_transform).map_err(|error| {
        rollback_player_motion(
            port,
            saved_transform,
            "post-move support",
            PlayerTickReason::Support(error),
        )
    })?;
    let actual_velocity = ActorVelocity::try_new(port.read_velocity()).map_err(|error| {
        rollback_player_motion(
            port,
            saved_transform,
            "post-move velocity",
            PlayerTickReason::Motion(error),
        )
    })?;
    let transition = reconcile(prepared, MotionOutcome::new(actual_velocity, support));
    // the fresh transition event only — never state.last_landing(), which
    // may be a restored memory of a landing another session already voiced
    let landing = transition.landing;
    let landing_request = match landing {
        None => None,
        Some(event) => {
            match crate::hero_visual::prepare_player_landing(event, config, pre_move.now) {
                Ok(prepared_voice) => prepared_voice,
                Err(error) => {
                    return Err(rollback_player_motion(
                        port,
                        saved_transform,
                        "post-move landing",
                        PlayerTickReason::Landing(error),
                    ));
                }
            }
        }
    };
    // derived only: the callback commit applies the pair beside the other
    // installed facts, so a refused later phase never leaves a half-applied
    // layer behind
    let (collision_layer, collision_mask) = collision_pair(transition.state.phase());
    Ok(PlayerTickSuccess {
        phase_before: prior.phase(),
        state: transition.state,
        landing,
        landing_request,
        support_collider_id: transition.state.support().and(collider_id),
        collision_layer,
        collision_mask,
    })
}

#[cfg(test)]
fn controlled_player_tick<P: PlayerMotionPort>(
    port: &mut P,
    raw_now: f64,
    prior: MotionState,
    desired_planar: PlanarVelocity,
    raw_dt: f64,
    config: SupportMotionConfig,
) -> Result<PlayerTickSuccess, PlayerTickFault> {
    let pre_move = prepare_player_pre_move(port, raw_now)?;
    controlled_player_tick_from_pre_move(port, pre_move, prior, desired_planar, raw_dt, config)
}

/// The blind hero. Movement and look happen on the engine's
/// CharacterBody3D; the cane's three voices and every wave request drain
/// through the physics tick. The clock is handed, never poked: the
/// composition root advances the simulated time via `tick`, and the
/// player never reads a wall clock of its own.
#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct UnseeingPlayer {
    /// The wave pool every sound enters — the `WaveCore` itself, upcast to
    /// `RefCounted`. The GDScript `Pulses` shim survives only in
    /// `game/tests/`. The player only asks it to `emit_reflecting`, dynamically.
    #[var]
    pulses: Option<Gd<RefCounted>>,
    /// The eye. Built by `_ready` at the fixed base height; the player
    /// alone moves it (mouse pitch, head-bob).
    #[var]
    camera: Option<Gd<Camera3D>>,
    /// The tap clock reading of the last accepted tap — drives the cane
    /// strike animation.
    #[var]
    #[init(val = -10.0)]
    pub(crate) last_tap: f64,
    /// Where the last tap landed (wall/floor/air) — the strike target
    /// the viewmodel reaches toward.
    #[var]
    pub(crate) tap_target: Vector3,
    /// Cached cane rest, recomputed every physics tick at the sweep
    /// offset the viewmodel requested — the hero body reads this instead
    /// of raycasting itself.
    #[var]
    #[init(val = Some(CaneRest::new_gd()))]
    pub(crate) cane_rest: Option<Gd<CaneRest>>,
    cane_rest_offset: f64,
    now: f64,
    tap_queued: bool,
    wave_queue: Vec<WaveRequest>,
    #[init(val = MotionState::initial())]
    motion_state: MotionState,
    #[init(val = SupportMotionConfig::PLAYER_DEFAULT)]
    motion_config: SupportMotionConfig,
    support_collider_id: Option<u64>,
    #[init(val = PhysicsTestMotionParameters3D::new_gd())]
    snap_params: Gd<PhysicsTestMotionParameters3D>,
    #[init(val = PhysicsTestMotionResult3D::new_gd())]
    snap_result: Gd<PhysicsTestMotionResult3D>,
    #[init(val = FootstepSuppression::CLEAR)]
    footstep_suppression: FootstepSuppression,
    base: Base<CharacterBody3D>,
}

impl PlayerMotionPort for UnseeingPlayer {
    fn read_global_transform(&mut self) -> Transform3D {
        self.base().get_global_transform()
    }

    fn read_global_rotation(&mut self) -> Vector3 {
        self.base().get_global_rotation()
    }

    fn read_velocity(&mut self) -> Vector3 {
        self.base().get_velocity()
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

    fn disable_physics(&mut self) {
        self.base_mut().set_physics_process(false);
    }
}

/// The thin Godot-side cane port: the player body's transform, its own
/// live eye, and physics-space rays under the closed dictionary mapping.
/// It owns handles only — no emitter and no scene mutation runs through
/// it.
struct GodotCaneQueryPort {
    player: Gd<UnseeingPlayer>,
    camera: Option<Gd<Camera3D>>,
    space: Option<Gd<PhysicsDirectSpaceState3D>>,
}

impl CaneQueryPort for GodotCaneQueryPort {
    fn player_transform(&mut self) -> Transform3D {
        self.player.get_global_transform()
    }

    fn camera_transform(&mut self) -> Option<Transform3D> {
        self.camera
            .as_ref()
            .map(|camera| camera.get_global_transform())
    }

    fn camera_rotation(&mut self) -> Option<Vector3> {
        self.camera.as_ref().map(|camera| camera.get_rotation())
    }

    fn cast_ray(&mut self, from: Vector3, to: Vector3) -> CaneRayAnswer {
        let Some(space) = self.space.as_mut() else {
            return CaneRayAnswer::Miss; // outside a world: nothing to strike
        };
        let Some(query) = PhysicsRayQueryParameters3D::create(from, to) else {
            return CaneRayAnswer::Malformed;
        };
        let hit = space.intersect_ray(&query);
        if hit.is_empty() {
            return CaneRayAnswer::Miss;
        }
        match (
            hit.get("position").and_then(|v| v.try_to::<Vector3>().ok()),
            hit.get("normal").and_then(|v| v.try_to::<Vector3>().ok()),
        ) {
            (Some(position), Some(normal)) => CaneRayAnswer::Hit { position, normal },
            _ => CaneRayAnswer::Malformed,
        }
    }
}

#[godot_api]
impl ICharacterBody3D for UnseeingPlayer {
    fn ready(&mut self) {
        // the body and the eye, exactly the script's _init limbs: a
        // capsule collider and a camera at the fixed base height
        let mut col = CollisionShape3D::new_alloc();
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(0.35);
        capsule.set_height(1.7);
        col.set_shape(&capsule);
        col.set_position(Vector3::new(0.0, PLAYER_CAPSULE_CENTER_Y_M, 0.0));
        self.base_mut().add_child(&col);
        let mut camera = Camera3D::new_alloc();
        camera.set_position(Vector3::new(0.0, CAM_BASE_Y as f32, 0.0));
        // Read from render::depth, not retyped here: the acoustic-image
        // band's whole safety argument — that no world fragment can
        // rasterise into it — is a statement about THESE two planes, and a
        // camera built from its own literals could drift out from under it
        // silently.
        camera.set_near(render::depth::CAM_NEAR as f32);
        camera.set_far(render::depth::CAM_FAR as f32);
        camera.set_fov(66.0); // ~1.15 rad vertical, the validated design FOV
        self.base_mut().add_child(&camera);
        self.camera = Some(camera);
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
        self.snap_params.set_motion(Vector3::DOWN * FLOOR_SNAP_M);
        self.snap_params.set_margin(SAFE_MARGIN_M);
        self.snap_params.set_max_collisions(SNAP_PROBE_MAX_CONTACTS);
        self.snap_params.set_recovery_as_collision_enabled(true);
        self.snap_params.set_collide_separation_ray_enabled(true);
        self.apply_collision_pair();
        Self::ensure_actions();
        // no silent nulls: without its pulse pool the player cannot voice a
        // single tap or footstep — refuse to run instead of crashing later
        if self.pulses.is_none() {
            godot_error!("UnseeingPlayer: pulses not injected — physics disabled");
            self.base_mut().set_physics_process(false);
            return;
        }
        // on web the browser only grants capture on a user gesture; the
        // click handler below recaptures, so skip the doomed attempt and
        // console noise
        if !Os::singleton().has_feature("web") {
            Input::singleton().set_mouse_mode(input::MouseMode::CAPTURED);
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            if Input::singleton().get_mouse_mode() == input::MouseMode::CAPTURED {
                self.apply_look(motion.get_relative());
            }
            return;
        }
        // Escape belongs to the settings overlay, which raises itself,
        // frees the mouse and freezes the world — all three at once. The
        // player used to release the mouse here and leave the world
        // running; two owners of the cursor is one too many.
        if let Ok(click) = event.try_cast::<InputEventMouseButton>() {
            if !click.is_pressed() {
                return;
            }
            if Input::singleton().get_mouse_mode() != input::MouseMode::CAPTURED {
                Input::singleton().set_mouse_mode(input::MouseMode::CAPTURED);
            }
            if click.get_button_index() == MouseButton::LEFT {
                self.tap_queued = true; // executed next physics tick, in physics context
            }
        }
    }

    fn physics_process(&mut self, dt: f64) {
        let raw_now = self.now;
        let pre_move = match prepare_player_pre_move(self, raw_now) {
            Ok(prepared) => prepared,
            Err(error) => {
                godot_error!("UnseeingPlayer: {} refused: {}", error.phase, error.reason);
                return;
            }
        };
        let desired = if self.motion_state.accepts_control() {
            match self.desired_planar_velocity() {
                Ok(desired) => desired,
                Err(error) => {
                    let error = refuse_player_before_move(self, "movement input", error);
                    godot_error!("UnseeingPlayer: {} refused: {}", error.phase, error.reason);
                    return;
                }
            }
        } else {
            PlanarVelocity::ZERO
        };
        let prior = self.motion_state;
        let config = self.motion_config();
        let moved = match controlled_player_tick_from_pre_move(
            self, pre_move, prior, desired, dt, config,
        ) {
            Ok(moved) => moved,
            Err(error) => {
                godot_error!("UnseeingPlayer: {} refused: {}", error.phase, error.reason);
                return;
            }
        };
        // the callback commit, in order: state, support identity, the
        // derived collision pair, the latch, the prepared landing voice,
        // then cane intent and the gated queue drain
        let PlayerTickSuccess {
            phase_before,
            state,
            landing,
            landing_request,
            support_collider_id,
            collision_layer,
            collision_mask,
        } = moved;
        self.motion_state = state;
        self.support_collider_id = support_collider_id;
        self.apply_exact_collision_pair(collision_layer, collision_mask);
        // every fresh event arms the latch, audible or not
        self.footstep_suppression = self.footstep_suppression.on_transition(landing);
        let space = self.space_state().to_variant();
        let now = self.now;
        if let Some(request) = landing_request {
            let ((kind, at, max_r, speed, gain, echoes, normal), prepared, _wave, _reflection) =
                request.into_emit_parts();
            self.emit_reflecting(
                kind,
                at,
                max_r,
                speed,
                gain,
                prepared.value(),
                &space,
                echoes,
                normal,
            );
        }

        // cane work goes through the one query port; the rest is
        // published and the tap installed ONLY on complete success —
        // malformed world data retains the queued intent and the prior
        // rest, clock, target, and wave state untouched
        let mut cane_port = GodotCaneQueryPort {
            player: self.to_gd(),
            camera: self
                .camera
                .as_ref()
                .filter(|camera| camera.is_instance_valid())
                .cloned(),
            space: self.space_state(),
        };
        match prepare_cane_rest(&mut cane_port, self.cane_rest_offset) {
            Ok(probe) => self.publish_cane_rest(&probe),
            Err(error) => godot_error!("UnseeingPlayer: cane rest refused: {error}"),
        }
        if self.tap_queued {
            match PreparedLastTap::try_new(self.last_tap, pre_move.now) {
                Err(error) => godot_error!("UnseeingPlayer: cane tap refused: {error}"),
                Ok(prior_tap) => match prepare_cane_tap(&mut cane_port, pre_move.now, prior_tap) {
                    Ok(None) => self.tap_queued = false, // cooldown: swallowed whole
                    Ok(Some(prepared)) => {
                        self.tap_queued = false;
                        self.last_tap = prepared.last_tap;
                        self.tap_target = prepared.tap_target;
                        if let Some(request) = prepared.request {
                            let ((kind, at, max_r, speed, gain, echoes, normal), birth, _w, _r) =
                                request.into_emit_parts();
                            self.emit_reflecting(
                                kind,
                                at,
                                max_r,
                                speed,
                                gain,
                                birth.value(),
                                &space,
                                echoes,
                                normal,
                            );
                        }
                    }
                    Err(error) => {
                        // the queued intent is retained for the next tick
                        godot_error!("UnseeingPlayer: cane tap refused: {error}");
                    }
                },
            }
        }
        // other systems' queued waves: emitted here so reflection raycasts
        // run in physics context. A closed gate consumes its request
        // silently — stale shoe provenance never survives a phase edge.
        for request in std::mem::take(&mut self.wave_queue) {
            if !request.gate.allows(phase_before, state.phase(), landing) {
                continue;
            }
            self.emit_request(request, now, &space);
        }
    }
}

#[godot_api]
impl UnseeingPlayer {
    /// The six active authored motion scalars in constructor order. This is
    /// a read-only observability door; designer configuration belongs to the
    /// composition root and is injected before this node enters the tree.
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

    pub(super) fn prepare_restore(
        &self,
        hero: &HeroCapture,
        now: PreparedTime,
    ) -> Result<PreparedPlayerState, RestoreValueError> {
        if !self.base().is_physics_processing() {
            return Err(RestoreValueError::new(
                "hero",
                "runtime physics is disabled",
            ));
        }
        let position = ActorPosition::try_new(hero.position).map_err(|error| {
            let axis = error.field().rsplit('.').next().unwrap_or("position");
            RestoreValueError::new(
                format!("hero.position.{axis}"),
                "must be finite and inside actor bounds",
            )
        })?;
        let velocity = ActorVelocity::try_new(hero.velocity).map_err(|error| {
            let axis = error.field().rsplit('.').next().unwrap_or("velocity");
            RestoreValueError::new(format!("hero.velocity.{axis}"), "must be finite")
        })?;
        validate_restore(hero.motion, velocity, self.motion_config).map_err(|error| {
            let (path, rule) = match error {
                MotionRestoreError::AirbornePlanarMismatch { axis } => {
                    (
                        format!("hero.motion.phase.planar_velocity.{axis}"),
                        "must match the corresponding physical velocity lane bit-exactly while airborne",
                    )
                }
                MotionRestoreError::AirborneTerminalExceeded => (
                    "hero.motion.phase.vertical_velocity".to_string(),
                    "must remain between zero and the injected terminal fall speed",
                ),
                MotionRestoreError::Physical(_) => {
                    ("hero.motion".to_string(), "contains an invalid physical value")
                }
            };
            RestoreValueError::new(path, rule)
        })?;
        let yaw = exact_f32_lane(hero.yaw, "hero.yaw")?;
        if !hero.pitch.is_finite() || !(-PITCH_LIMIT..=PITCH_LIMIT).contains(&hero.pitch) {
            return Err(RestoreValueError::new(
                "hero.pitch",
                "must be finite and inside the eye-pitch limit",
            ));
        }
        let pitch = exact_f32_lane(hero.pitch, "hero.pitch")?;
        let last_tap = prepare_last_tap(hero.last_tap, now)?;
        PosePoint::try_new(hero.tap_target).map_err(|error| {
            let axis = error.field().rsplit('.').next().unwrap_or("tap_target");
            RestoreValueError::new(
                format!("hero.tap_target.{axis}"),
                "must be finite and inside pose bounds",
            )
        })?;
        let mut wave_queue = Vec::with_capacity(hero.queued_waves.len());
        for (index, wave) in hero.queued_waves.iter().enumerate() {
            let prefix = format!("hero.queued_waves[{index}]");
            let proof = CheckedWave::prepare(
                wave.kind,
                wave.at,
                wave.max_r,
                wave.speed,
                wave.gain,
                now,
                Vector3::ZERO,
                OMNI_COS,
            )
            .map_err(|error| {
                RestoreValueError::new(format!("{prefix}.{}", error.field()), error.rule())
            })?;
            let reflection_proof = CheckedReflectionRequest::prepare(ReflectionRequest {
                at: wave.at,
                normal: wave.normal,
                max_r: wave.max_r,
                speed: wave.speed,
                max_echoes: wave.echoes,
                now: now.value(),
            })
            .map_err(|error| {
                RestoreValueError::new(format!("{prefix}.{}", error.field()), error.reason())
            })?;
            wave_queue.push(PreparedWaveRequest {
                request: WaveRequest {
                    kind: wave.kind,
                    at: wave.at,
                    max_r: wave.max_r,
                    speed: wave.speed,
                    gain: wave.gain,
                    echoes: wave.echoes,
                    normal: wave.normal,
                    gate: wave.gate,
                    prepared_at: None,
                },
                _wave_proof: proof,
                _reflection_proof: reflection_proof,
            });
        }
        let rotation =
            GodotRotation::try_replacing_yaw(self.base().get_rotation(), yaw).map_err(|_| {
                RestoreValueError::new(
                    "hero.yaw",
                    "must form a canonical complete Godot YXZ rotation with the live X/Z lanes",
                )
            })?;
        let eye = self
            .camera
            .as_ref()
            .filter(|camera| camera.is_instance_valid())
            .ok_or_else(|| {
                RestoreValueError::new("hero.pitch", "the runtime eye is missing or freed")
            })?
            .clone();
        let eye_rotation =
            GodotRotation::try_replacing_pitch(eye.get_rotation(), pitch).map_err(|_| {
                RestoreValueError::new(
                    "hero.pitch",
                    "must form a canonical complete Godot YXZ rotation with the live Y/Z lanes",
                )
            })?;
        let (collision_layer, collision_mask) = collision_pair(hero.motion.phase());
        Ok(PreparedPlayerState {
            position: position.world(),
            velocity: velocity.world(),
            rotation: rotation.world(),
            eye,
            eye_rotation: eye_rotation.world(),
            motion: hero.motion,
            collision_layer,
            collision_mask,
            last_tap,
            tap_target: hero.tap_target,
            tap_queued: hero.tap_queued,
            wave_queue,
            footstep_suppression: FootstepSuppression::restore(hero.footstep_suppression_pending),
            now,
        })
    }

    pub(super) fn install_prepared(&mut self, value: PreparedPlayerState) {
        let mut eye = value.eye;
        self.base_mut().set_global_position(value.position);
        self.base_mut().set_velocity(value.velocity);
        self.base_mut().set_rotation(value.rotation);
        eye.set_rotation(value.eye_rotation);
        self.motion_state = value.motion;
        self.apply_exact_collision_pair(value.collision_layer, value.collision_mask);
        self.support_collider_id = None;
        self.footstep_suppression = value.footstep_suppression;
        self.last_tap = value.last_tap;
        self.tap_target = value.tap_target;
        self.tap_queued = value.tap_queued;
        self.wave_queue = value
            .wave_queue
            .into_iter()
            .map(|prepared| prepared.request)
            .collect();
        self.now = value.now.value();
    }

    /// The player registers its own senses: idempotent, so a bare
    /// instance in a test scene polls input without the root's help, and
    /// the boot-time call plus every player `_ready` leave exactly one
    /// key event per action.
    #[func]
    pub(super) fn ensure_actions() {
        let mut map = InputMap::singleton();
        for (action, key) in MOVE_KEYS {
            if map.has_action(action) {
                continue;
            }
            map.add_action(action);
            let mut ev = InputEventKey::new_gd();
            ev.set_physical_keycode(key);
            map.action_add_event(action, &ev);
        }
    }

    /// The registered move actions, in binding order — the observable
    /// face of MOVE_KEYS for the suites (a Dictionary constant cannot
    /// cross the boundary; the physical keycodes stay an engine detail).
    #[func]
    fn move_keys() -> Array<GString> {
        MOVE_KEYS
            .iter()
            .map(|(action, _)| GString::from(*action))
            .collect()
    }

    /// Camera rest height — a float constant served as a static method:
    /// ClassDB registers integer constants only.
    #[func]
    fn cam_base_y() -> f64 {
        CAM_BASE_Y
    }

    /// Walk speed, m/s — static-method constant, same reason.
    #[func]
    fn speed() -> f64 {
        SPEED
    }

    /// Arm + cane reach in meters — static-method constant, same reason.
    #[func]
    fn cane_reach() -> f64 {
        CANE_REACH
    }

    /// The wall backoff in meters — static-method constant, same reason.
    #[func]
    fn wall_backoff() -> f64 {
        WALL_BACKOFF
    }

    /// The pitch clamp in radians — static-method constant, same reason.
    #[func]
    fn pitch_limit() -> f64 {
        PITCH_LIMIT
    }

    /// Mouse sensitivity, radians per pixel — static-method constant,
    /// same reason.
    #[func]
    fn mouse_sens() -> f64 {
        MOUSE_SENS
    }

    /// Relocate the physical hero atomically. Invalid raw positions leave
    /// every body and motion observation untouched.
    #[func]
    fn relocate(&mut self, world_position: Vector3) -> VarDictionary {
        let mut verdict = VarDictionary::new();
        match self.try_relocate(world_position) {
            Ok(()) => verdict.set("relocated", true),
            Err(error) => verdict.set("unavailable", error.to_string().as_str()),
        }
        verdict
    }

    /// Accepted world-support identity, if the engine supplied one. A
    /// server-backed support may lawfully have no Object id.
    #[func]
    fn support_collider_id(&self) -> Variant {
        self.support_collider_id
            .and_then(|id| i64::try_from(id).ok())
            .map_or_else(Variant::nil, |id| id.to_variant())
    }

    /// The clock is handed, never poked: the composition root advances
    /// the simulated time here every frame — and the restorer places it
    /// back on the captured instant before the cane's own clocks land.
    #[func]
    pub(super) fn tick(&mut self, now_t: f64) {
        self.now = now_t;
    }

    /// The cane speaks on command: the scripted twin of the left click,
    /// riding the SAME queued-intent path — executed next physics tick,
    /// in physics context, through the full aimed/rest/swish decision
    /// tree and the [`TAP_COOLDOWN`]. `queue_wave` fakes a wave; this
    /// taps the cane.
    #[func]
    pub fn tap(&mut self) {
        self.tap_queued = true;
    }

    /// One mouse-motion's worth of look, as data: yaw by -x, pitch by -y,
    /// both scaled by [`MOUSE_SENS`], pitch clamped to [`PITCH_LIMIT`] —
    /// the exact law the captured-mouse handler applies, callable without
    /// a mouse so a scripted run turns the hero through the player's real
    /// look path instead of teleporting the rotation around it.
    #[func]
    pub fn look(&mut self, relative: Vector2) {
        self.apply_look(relative);
    }

    /// Other systems (hero footsteps, the demo tap) request waves here;
    /// they are emitted next physics tick so reflection raycasts run
    /// in-context.
    #[func]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the GDScript queue_wave() signature one to one, \
                  so every call site reads like the script it replaces"
    )]
    pub(crate) fn queue_wave(
        &mut self,
        wave_type: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        max_echoes: i64,
        origin_normal: Vector3,
    ) {
        let now = match prepare_time(self.now) {
            Ok(now) => now,
            Err(error) => {
                godot_error!(
                    "UnseeingPlayer.queue_wave: field now: {} — wave refused",
                    error.rule
                );
                return;
            }
        };
        if let Err(error) = CheckedWave::prepare(
            wave_type,
            at,
            max_r,
            speed,
            gain,
            now,
            Vector3::ZERO,
            OMNI_COS,
        ) {
            godot_error!(
                "UnseeingPlayer.queue_wave: field {}: {} — wave refused",
                error.field(),
                error.rule()
            );
            return;
        }
        if let Err(error) = CheckedReflectionRequest::prepare(ReflectionRequest {
            at,
            normal: origin_normal,
            max_r,
            speed,
            max_echoes,
            now: now.value(),
        }) {
            godot_error!(
                "UnseeingPlayer.queue_wave: field {}: {}",
                error.field(),
                error.reason()
            );
            return;
        }
        self.wave_queue.push(WaveRequest {
            kind: wave_type,
            at,
            max_r,
            speed,
            gain,
            echoes: max_echoes,
            normal: origin_normal,
            gate: QueuedWaveGate::Always,
            prepared_at: None,
        });
    }

    /// The waves waiting for the next physics tick, copied out as
    /// dictionaries — the queue's observable face for the suites, which
    /// used to read the script's private array directly.
    #[func]
    fn queued_waves(&self) -> Array<VarDictionary> {
        self.wave_queue
            .iter()
            .map(|w| {
                let mut entry = VarDictionary::new();
                entry.set("type", w.kind);
                entry.set("at", w.at);
                entry.set("max_r", w.max_r);
                entry.set("speed", w.speed);
                entry.set("gain", w.gain);
                entry.set("echoes", w.echoes);
                entry.set("normal", w.normal);
                entry.set("gate", w.gate.wire_name());
                entry
            })
            .collect()
    }

    /// The look law, shared by the captured mouse and the scripted
    /// `look`: the capture GATE stays at the event handler — it is about
    /// who owns the cursor, not about how rotation works.
    fn apply_look(&mut self, relative: Vector2) {
        self.base_mut()
            .rotate_y((f64::from(-relative.x) * MOUSE_SENS) as f32);
        if let Some(camera) = self.camera.as_mut() {
            let mut rot = camera.get_rotation();
            rot.x = (f64::from(rot.x) - f64::from(relative.y) * MOUSE_SENS)
                .clamp(-PITCH_LIMIT, PITCH_LIMIT) as f32;
            camera.set_rotation(rot);
        }
    }

    /// Publish a probe as the frame's `cane_rest` — a fresh CaneRest per
    /// tick, exactly as the script rebuilt its own.
    fn publish_cane_rest(&mut self, probe: &RestProbe) {
        let mut rest = CaneRest::new_gd();
        {
            let mut fields = rest.bind_mut();
            fields.tip = probe.tip;
            fields.supported = probe.supported;
        }
        self.cane_rest = Some(rest);
    }

    /// The one door to the pool: a dynamic `emit_reflecting` on the
    /// injected object, so the shipped `WaveCore` and the test-only
    /// GDScript shim both answer. PHYSICS CONTEXT: callers are all inside
    /// the physics tick, per the module law.
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the pool's emit_reflecting signature one to one, \
                  so the call sites read like the GDScript they replace"
    )]
    fn emit_reflecting(
        &mut self,
        kind: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
        space: &Variant,
        max_echoes: i64,
        origin_normal: Vector3,
    ) {
        let Some(pulses) = self.pulses.as_mut() else {
            return; // unreachable past the _ready guard; total anyway
        };
        pulses.call(
            "emit_reflecting",
            &[
                kind.to_variant(),
                at.to_variant(),
                max_r.to_variant(),
                speed.to_variant(),
                gain.to_variant(),
                now.to_variant(),
                space.clone(),
                max_echoes.to_variant(),
                origin_normal.to_variant(),
            ],
        );
    }

    /// One drained queue entry into the pool — the request's own lanes,
    /// the tick's clock, the tick's space.
    fn emit_request(&mut self, request: WaveRequest, now: f64, space: &Variant) {
        let birth = request.prepared_at.unwrap_or(now);
        self.emit_reflecting(
            request.kind,
            request.at,
            request.max_r,
            request.speed,
            request.gain,
            birth,
            space,
            request.echoes,
            request.normal,
        );
    }

    /// The physics space of the player's world, if it stands in one.
    fn space_state(&self) -> Option<Gd<PhysicsDirectSpaceState3D>> {
        self.base()
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state())
    }
}

fn exact_f32_lane(value: f64, path: &'static str) -> Result<f32, RestoreValueError> {
    if !value.is_finite() {
        return Err(RestoreValueError::new(path, "must be finite"));
    }
    let lane = value as f32;
    if f64::from(lane).to_bits() != value.to_bits() {
        return Err(RestoreValueError::new(
            path,
            "must round-trip through the Godot f32 lane bit-exactly",
        ));
    }
    Ok(lane)
}

/// The one player-elevation law: a standing root's support surface sits
/// exactly the authored standing height below it. The hero's visual
/// boundary derives its support from the same validated position it
/// sampled, through this shared door.
pub(super) fn support_elevation_at(
    world_position: Vector3,
) -> Result<SupportElevation, MotionValueError> {
    let position = ActorPosition::try_new(world_position)?;
    SupportElevation::try_new(position.world().y - PLAYER_STANDING_ROOT_Y as f32)
}

fn prepare_last_tap(raw: f64, now: PreparedTime) -> Result<f64, RestoreValueError> {
    PreparedLastTap::try_new(raw, now)
        .map(PreparedLastTap::raw)
        .map_err(|_| {
            RestoreValueError::new(
                "hero.last_tap",
                "must be the exact initial sentinel or an elapsed simulation time",
            )
        })
}

impl UnseeingPlayer {
    pub(super) fn inject_motion_config(&mut self, config: SupportMotionConfig) {
        self.motion_config = config;
    }

    pub(crate) fn motion_config(&self) -> SupportMotionConfig {
        self.motion_config
    }

    /// Prove the given handle is this player's own live eye — the narrow
    /// adapter identity check the hero performs before sampling. `None`
    /// for a missing camera and for either freed handle. The returned
    /// token is the ONLY way through the visual commit door, so a frame
    /// can never be committed against an unproven or absent eye.
    pub(super) fn prove_visual_camera(&self, camera: &Gd<Camera3D>) -> Option<VisualCameraProof> {
        let own = self.camera.as_ref()?;
        if !own.is_instance_valid() || !camera.is_instance_valid() {
            return None;
        }
        (own.instance_id() == camera.instance_id()).then(|| VisualCameraProof(camera.clone()))
    }

    /// The installed footstep latch, copied out for the hero's pure
    /// visual preparation — acknowledged only through the commit door.
    pub(super) fn footstep_suppression(&self) -> FootstepSuppression {
        self.footstep_suppression
    }

    /// The one Rust-only visual commit door: an already-prepared hero
    /// frame lands here whole — camera-local `CAM_BASE_Y + bob`, the cane
    /// sweep offset, the acknowledged latch, and the optional checked
    /// footstep request appended without revalidation. No raw bob or
    /// sweep door exists beside it, and the consumed eye proof makes the
    /// camera write unconditional: the door cannot half-apply a frame.
    pub(super) fn commit_hero_frame(
        &mut self,
        eye: VisualCameraProof,
        bob: f64,
        cane_sweep: f64,
        suppression: FootstepSuppression,
        footstep: Option<PreparedFootstepRequest>,
    ) {
        let VisualCameraProof(mut camera) = eye;
        {
            let mut position = camera.get_position();
            position.y = (CAM_BASE_Y + bob) as f32;
            camera.set_position(position);
        }
        self.cane_rest_offset = cane_sweep;
        self.footstep_suppression = suppression;
        if let Some(request) = footstep {
            let (
                (kind, at, max_r, speed, gain, echoes, normal, gate),
                prepared,
                _wave,
                _reflection,
            ) = request.into_player_parts();
            self.wave_queue.push(WaveRequest {
                kind,
                at,
                max_r,
                speed,
                gain,
                echoes,
                normal,
                gate,
                prepared_at: Some(prepared.value()),
            });
        }
    }

    fn desired_planar_velocity(&self) -> Result<PlanarVelocity, MotionValueError> {
        let input =
            Input::singleton().get_vector("move_left", "move_right", "move_forward", "move_back");
        let local = Vector3::new(input.x, 0.0, input.y);
        let transformed = self.base().get_transform().basis * local * SPEED as f32;
        PlanarVelocity::try_from_world(transformed)
    }

    pub(super) fn try_relocate(&mut self, world_position: Vector3) -> Result<(), MotionValueError> {
        let position = ActorPosition::try_new(world_position)?;
        self.base_mut().set_global_position(position.world());
        self.motion_state = self.motion_state.relocated();
        self.support_collider_id = None;
        self.base_mut().set_velocity(Vector3::ZERO);
        self.apply_collision_pair();
        Ok(())
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

    /// The cane's queued-intent flag, for the observer: a tap accepted
    /// this frame that the physics tick has not yet executed.
    pub(crate) fn tap_queued(&self) -> bool {
        self.tap_queued
    }

    pub(crate) fn motion_state(&self) -> MotionState {
        self.motion_state
    }

    pub(crate) fn footstep_suppression_pending(&self) -> bool {
        self.footstep_suppression.pending()
    }

    /// The eye's complete local rotation — `None` before `_ready` has built
    /// the camera or after it was freed. Capture owns only pitch, but must
    /// prove the uncaptured Y/Z configuration before serialising that lane.
    pub(crate) fn eye_rotation(&self) -> Option<Vector3> {
        self.camera
            .as_ref()
            .and_then(|camera| camera.is_instance_valid().then(|| camera.get_rotation()))
    }

    /// The wave queue as pure observations — the same content the
    /// `queued_waves` #[func] serialises for the suites.
    pub(crate) fn wave_queue(&self) -> Vec<QueuedWave> {
        self.wave_queue
            .iter()
            .map(|w| QueuedWave {
                kind: w.kind,
                at: w.at,
                max_r: w.max_r,
                speed: w.speed,
                gain: w.gain,
                echoes: w.echoes,
                normal: w.normal,
                gate: w.gate,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support_motion::MotionPhase;
    use crate::temporal::prepare_time;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum MotionTrace {
        ReadTransform,
        ReadRotation,
        ReadVelocity,
        SetVelocity(Vector3),
        MoveAndSlide,
        Probe(Transform3D),
        ReadProbeCount,
        ReadProbeContact(i32),
        SetTransform(Transform3D),
        DisablePhysics,
    }

    struct FakePlayerMotionPort {
        pre_transform: Transform3D,
        pre_rotation: Vector3,
        pre_velocity: Vector3,
        post_transform: Transform3D,
        post_rotation: Vector3,
        post_velocity: Vector3,
        moved: bool,
        on_floor: bool,
        slides: Vec<Option<Vec<RawSupportFact>>>,
        outer_count_override: Option<i32>,
        inner_count_override: Option<(i32, i32)>,
        probe_hit: bool,
        probe_contacts: Vec<RawSupportFact>,
        probe_count_override: Option<i32>,
        trace: Vec<MotionTrace>,
    }

    impl FakePlayerMotionPort {
        fn valid() -> Self {
            let rotation = Vector3::new(0.125, -0.25, 0.0625);
            let transform = Transform3D::new(
                Basis::from_euler(EulerOrder::YXZ, rotation),
                Vector3::new(1.25, 0.9, -2.5),
            );
            Self {
                pre_transform: transform,
                pre_rotation: rotation,
                pre_velocity: Vector3::ZERO,
                post_transform: transform,
                post_rotation: rotation,
                post_velocity: Vector3::new(0.75, 0.0, -1.25),
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

        fn probe_calls(&self) -> usize {
            self.trace
                .iter()
                .filter(|entry| matches!(entry, MotionTrace::Probe(_)))
                .count()
        }

        fn effect_trace(&self) -> Vec<MotionTrace> {
            self.trace
                .iter()
                .copied()
                .filter(|entry| {
                    matches!(
                        entry,
                        MotionTrace::SetVelocity(_)
                            | MotionTrace::MoveAndSlide
                            | MotionTrace::SetTransform(_)
                            | MotionTrace::DisablePhysics
                    )
                })
                .collect()
        }
    }

    impl PlayerMotionPort for FakePlayerMotionPort {
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
                .unwrap_or_else(world_floor);
            (fact.point, fact.normal)
        }

        fn read_probe_collider(&mut self, contact: i32) -> (bool, u32, u64) {
            let fact = self
                .probe_contacts
                .get(usize::try_from(contact).unwrap_or(usize::MAX))
                .copied()
                .unwrap_or_else(world_floor);
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

        fn disable_physics(&mut self) {
            self.trace.push(MotionTrace::DisablePhysics);
        }
    }

    fn world_floor() -> RawSupportFact {
        RawSupportFact {
            point: Vector3::new(2.0, 0.0, -3.0),
            normal: Vector3::UP,
            collider_rid_valid: true,
            collider_layer: 1,
            collider_id: 41,
        }
    }

    fn actor_floor() -> RawSupportFact {
        RawSupportFact {
            collider_layer: 2,
            collider_id: 17,
            ..world_floor()
        }
    }

    fn wall_contact() -> RawSupportFact {
        RawSupportFact {
            normal: Vector3::RIGHT,
            collider_id: 29,
            ..world_floor()
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
            match lane % 3 {
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

    fn post_transform(port: &FakePlayerMotionPort) -> ActorTransform {
        ActorTransform::try_new(port.post_transform).unwrap()
    }

    #[test]
    fn valid_tick_calls_move_and_slide_once() {
        let mut port = FakePlayerMotionPort::valid();
        let result = controlled_player_tick(
            &mut port,
            2.0,
            MotionState::initial(),
            PlanarVelocity::try_new(1.0, -2.0).unwrap(),
            1.0 / 60.0,
            SupportMotionConfig::PLAYER_DEFAULT,
        )
        .unwrap();
        assert_eq!(
            port.trace
                .iter()
                .filter(|entry| matches!(entry, MotionTrace::MoveAndSlide))
                .count(),
            1
        );
        let MotionPhase::Airborne {
            planar_velocity_mps,
            vertical_velocity_mps,
        } = result.state.phase()
        else {
            panic!("unsupported controlled motion must become airborne")
        };
        assert_eq!(planar_velocity_mps.x_mps().to_bits(), 0.75_f32.to_bits());
        assert_eq!(planar_velocity_mps.z_mps().to_bits(), (-1.25_f32).to_bits());
        assert_eq!(vertical_velocity_mps.mps().to_bits(), (-0.0_f32).to_bits());
        // the coordinator only DERIVES the airborne pair — applying it is
        // the callback commit's job, beside every other installed fact
        assert_eq!(
            (result.collision_layer, result.collision_mask),
            (4, 4_294_967_289)
        );
    }

    #[test]
    fn poisoned_player_pre_move_transform_or_rotation_refuses_without_move_or_wave() {
        let mut cases = Vec::new();
        for lane in 0..12 {
            let mut port = FakePlayerMotionPort::valid();
            port.pre_transform = poison_transform_lane(port.pre_transform, lane);
            cases.push(port);
        }
        for lane in 0..3 {
            let mut port = FakePlayerMotionPort::valid();
            port.pre_rotation = poison_vector_lane(port.pre_rotation, lane);
            cases.push(port);
        }
        for lane in 0..3 {
            let mut port = FakePlayerMotionPort::valid();
            port.pre_velocity = poison_vector_lane(port.pre_velocity, lane);
            cases.push(port);
        }

        for mut port in cases {
            let post_before = port.post_transform;
            assert!(
                controlled_player_tick(
                    &mut port,
                    2.0,
                    MotionState::initial(),
                    PlanarVelocity::try_new(1.0, -2.0).unwrap(),
                    1.0 / 60.0,
                    SupportMotionConfig::PLAYER_DEFAULT,
                )
                .is_err()
            );
            assert!(!port.moved);
            assert_transform_bits_eq(port.post_transform, post_before);
            assert_eq!(port.pre_velocity, Vector3::ZERO);
            assert_eq!(
                port.effect_trace(),
                [
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::DisablePhysics,
                ]
            );
        }
    }

    #[test]
    fn post_move_poison_writes_exact_saved_transform_then_zero_velocity_then_disables() {
        let mut cases = Vec::new();
        for lane in 0..12 {
            let mut port = FakePlayerMotionPort::valid();
            port.post_transform = poison_transform_lane(port.post_transform, lane);
            cases.push(port);
        }
        for lane in 0..3 {
            let mut port = FakePlayerMotionPort::valid();
            port.post_rotation = poison_vector_lane(port.post_rotation, lane);
            cases.push(port);
        }
        for lane in 0..3 {
            let mut port = FakePlayerMotionPort::valid();
            port.post_velocity = poison_vector_lane(port.post_velocity, lane);
            cases.push(port);
        }
        for lane in 0..6 {
            let mut port = FakePlayerMotionPort::valid();
            port.on_floor = true;
            let mut fact = world_floor();
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
            let prior = MotionState::initial();
            assert!(
                controlled_player_tick(
                    &mut port,
                    2.0,
                    prior,
                    PlanarVelocity::ZERO,
                    1.0 / 60.0,
                    SupportMotionConfig::PLAYER_DEFAULT,
                )
                .is_err()
            );
            assert_transform_bits_eq(port.post_transform, saved);
            assert_eq!(port.post_velocity, Vector3::ZERO);
            assert_eq!(
                port.effect_trace(),
                [
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::MoveAndSlide,
                    MotionTrace::SetTransform(saved),
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::DisablePhysics,
                ]
            );
            assert_eq!(prior, MotionState::initial());
        }
    }

    #[test]
    fn support_reader_scans_nested_contacts_actor_then_world_and_preserves_zero_id() {
        let mut nested = FakePlayerMotionPort::valid();
        nested.on_floor = true;
        nested.slides = vec![Some(vec![wall_contact(), world_floor()])];
        let transform = post_transform(&nested);
        let (support, id) = read_post_move_support(&mut nested, transform).unwrap();
        assert_eq!(
            support,
            Some(SupportContact::try_new(world_floor().point, Vector3::UP).unwrap())
        );
        assert_eq!(id, Some(41));
        assert_eq!(nested.probe_calls(), 0);

        let mut actor_then_world = FakePlayerMotionPort::valid();
        actor_then_world.on_floor = true;
        let mut zero_id = world_floor();
        zero_id.collider_id = 0;
        actor_then_world.slides = vec![Some(vec![actor_floor()]), Some(vec![zero_id])];
        let transform = post_transform(&actor_then_world);
        let (support, id) = read_post_move_support(&mut actor_then_world, transform).unwrap();
        assert_eq!(support.map(SupportContact::point), Some(zero_id.point));
        assert_eq!(id, None);
        assert_eq!(actor_then_world.probe_calls(), 0);
    }

    #[test]
    fn support_reader_rejects_counts_missing_entries_rids_and_every_poisoned_lane() {
        for count in [-1, 7] {
            let mut port = FakePlayerMotionPort::valid();
            port.on_floor = true;
            port.outer_count_override = Some(count);
            let transform = post_transform(&port);
            assert_eq!(
                read_post_move_support(&mut port, transform),
                Err(SupportReadError::InvalidOuterCount(count))
            );
        }
        let mut missing = FakePlayerMotionPort::valid();
        missing.on_floor = true;
        missing.outer_count_override = Some(1);
        let transform = post_transform(&missing);
        assert_eq!(
            read_post_move_support(&mut missing, transform),
            Err(SupportReadError::MissingSlide(0))
        );
        for count in [0, 7] {
            let mut port = FakePlayerMotionPort::valid();
            port.on_floor = true;
            port.slides = vec![Some(vec![world_floor()])];
            port.inner_count_override = Some((0, count));
            let transform = post_transform(&port);
            assert_eq!(
                read_post_move_support(&mut port, transform),
                Err(SupportReadError::InvalidInnerCount { slide: 0, count })
            );
        }
        let mut invalid_rid = FakePlayerMotionPort::valid();
        invalid_rid.on_floor = true;
        let mut fact = world_floor();
        fact.collider_rid_valid = false;
        invalid_rid.slides = vec![Some(vec![fact])];
        let transform = post_transform(&invalid_rid);
        assert_eq!(
            read_post_move_support(&mut invalid_rid, transform),
            Err(SupportReadError::InvalidOrdinaryRid {
                slide: 0,
                contact: 0,
            })
        );
        for lane in 0..6 {
            let mut port = FakePlayerMotionPort::valid();
            port.on_floor = true;
            let mut fact = world_floor();
            if lane < 3 {
                fact.point = poison_vector_lane(fact.point, lane);
            } else {
                fact.normal = poison_vector_lane(fact.normal, lane - 3);
            }
            port.slides = vec![Some(vec![fact])];
            let transform = post_transform(&port);
            assert!(matches!(
                read_post_move_support(&mut port, transform),
                Err(SupportReadError::InvalidValue(_))
            ));
        }
    }

    #[test]
    fn support_reader_accepts_exact_bounds_and_preserves_first_world_in_ledger_order() {
        let mut later_world = world_floor();
        later_world.point = Vector3::new(-7.0, 0.25, 9.0);
        later_world.collider_id = 83;
        let mut bounded_ledger = FakePlayerMotionPort::valid();
        bounded_ledger.on_floor = true;
        bounded_ledger.slides = vec![
            Some(vec![
                world_floor(),
                wall_contact(),
                wall_contact(),
                wall_contact(),
                wall_contact(),
                wall_contact(),
            ]),
            Some(vec![wall_contact(); 6]),
            Some(vec![wall_contact(); 6]),
            Some(vec![wall_contact(); 6]),
            Some(vec![wall_contact(); 6]),
            Some(vec![
                wall_contact(),
                wall_contact(),
                wall_contact(),
                wall_contact(),
                wall_contact(),
                later_world,
            ]),
        ];
        let transform = post_transform(&bounded_ledger);
        let (support, id) = read_post_move_support(&mut bounded_ledger, transform).unwrap();
        assert_eq!(
            support.map(SupportContact::point),
            Some(world_floor().point)
        );
        assert_eq!(id, Some(41));
        assert_eq!(bounded_ledger.probe_calls(), 0);

        let mut bounded_probe = FakePlayerMotionPort::valid();
        bounded_probe.on_floor = true;
        bounded_probe.probe_hit = true;
        bounded_probe.probe_contacts =
            vec![world_floor(), later_world, actor_floor(), wall_contact()];
        let transform = post_transform(&bounded_probe);
        let (support, id) = read_post_move_support(&mut bounded_probe, transform).unwrap();
        assert_eq!(
            support.map(SupportContact::point),
            Some(world_floor().point)
        );
        assert_eq!(id, Some(41));
        assert_eq!(bounded_probe.probe_calls(), 1);
    }

    #[test]
    fn support_reader_validates_later_facts_before_returning_first_world_floor() {
        let mut port = FakePlayerMotionPort::valid();
        port.on_floor = true;
        let mut poison = wall_contact();
        poison.point.z = f32::NAN;
        port.slides = vec![Some(vec![world_floor(), poison])];
        let transform = post_transform(&port);
        assert!(matches!(
            read_post_move_support(&mut port, transform),
            Err(SupportReadError::InvalidValue(_))
        ));
    }

    #[test]
    fn snap_probe_runs_only_for_a_hidden_floor_and_never_reads_stale_false_results() {
        let mut off_floor = FakePlayerMotionPort::valid();
        let transform = post_transform(&off_floor);
        assert_eq!(
            read_post_move_support(&mut off_floor, transform).unwrap(),
            (None, None)
        );
        assert_eq!(off_floor.probe_calls(), 0);

        let mut actor_only = FakePlayerMotionPort::valid();
        actor_only.on_floor = true;
        actor_only.slides = vec![Some(vec![actor_floor()])];
        actor_only.probe_hit = true;
        actor_only.probe_contacts = vec![world_floor()];
        let transform = post_transform(&actor_only);
        assert_eq!(
            read_post_move_support(&mut actor_only, transform).unwrap(),
            (None, None)
        );
        assert_eq!(actor_only.probe_calls(), 0);

        let mut false_probe = FakePlayerMotionPort::valid();
        false_probe.on_floor = true;
        false_probe.probe_hit = false;
        false_probe.probe_count_override = Some(5);
        false_probe.probe_contacts = vec![world_floor()];
        let transform = post_transform(&false_probe);
        assert_eq!(
            read_post_move_support(&mut false_probe, transform).unwrap(),
            (None, None)
        );
        assert_eq!(false_probe.probe_calls(), 1);
        assert!(!false_probe.trace.contains(&MotionTrace::ReadProbeCount));

        let mut hidden = FakePlayerMotionPort::valid();
        hidden.on_floor = true;
        hidden.probe_hit = true;
        hidden.probe_contacts = vec![actor_floor(), world_floor()];
        hidden.post_transform = Transform3D::new(
            Basis::from_euler(EulerOrder::XYZ, Vector3::new(-0.125, 0.375, -0.0625)),
            Vector3::new(5.5, 1.25, -8.0),
        );
        let transform = post_transform(&hidden);
        let (support, id) = read_post_move_support(&mut hidden, transform).unwrap();
        assert_eq!(
            support.map(SupportContact::point),
            Some(world_floor().point)
        );
        assert_eq!(id, Some(41));
        assert_eq!(hidden.probe_calls(), 1);
        assert!(
            hidden
                .trace
                .contains(&MotionTrace::Probe(transform.world()))
        );

        let mut actor_probe = FakePlayerMotionPort::valid();
        actor_probe.on_floor = true;
        actor_probe.probe_hit = true;
        actor_probe.probe_contacts = vec![actor_floor()];
        let transform = post_transform(&actor_probe);
        assert_eq!(
            read_post_move_support(&mut actor_probe, transform).unwrap(),
            (None, None)
        );
        assert_eq!(actor_probe.probe_calls(), 1);
        assert!(
            actor_probe
                .trace
                .contains(&MotionTrace::ReadProbeContact(0))
        );
    }

    #[test]
    fn snap_probe_validates_later_facts_before_returning_first_world_floor() {
        let mut poisoned_point = wall_contact();
        poisoned_point.point.z = f32::NAN;
        let mut poisoned_normal = wall_contact();
        poisoned_normal.normal.x = f32::NAN;
        let mut invalid_floor_rid = world_floor();
        invalid_floor_rid.collider_rid_valid = false;

        for (later, expected_invalid_rid) in [
            (poisoned_point, false),
            (poisoned_normal, false),
            (invalid_floor_rid, true),
        ] {
            let mut port = FakePlayerMotionPort::valid();
            port.on_floor = true;
            port.probe_hit = true;
            port.probe_contacts = vec![world_floor(), later];
            let transform = post_transform(&port);
            let result = read_post_move_support(&mut port, transform);
            if expected_invalid_rid {
                assert_eq!(result, Err(SupportReadError::InvalidProbeRid(1)));
            } else {
                assert!(matches!(result, Err(SupportReadError::InvalidValue(_))));
            }
            assert!(port.trace.contains(&MotionTrace::ReadProbeContact(1)));
        }
    }

    #[test]
    fn snap_probe_rejects_counts_rids_and_every_poisoned_lane() {
        for count in [0, 5] {
            let mut port = FakePlayerMotionPort::valid();
            port.on_floor = true;
            port.probe_hit = true;
            port.probe_count_override = Some(count);
            let transform = post_transform(&port);
            assert_eq!(
                read_post_move_support(&mut port, transform),
                Err(SupportReadError::InvalidProbeCount(count))
            );
        }
        let mut invalid_rid = FakePlayerMotionPort::valid();
        invalid_rid.on_floor = true;
        invalid_rid.probe_hit = true;
        let mut fact = world_floor();
        fact.collider_rid_valid = false;
        invalid_rid.probe_contacts = vec![fact];
        let transform = post_transform(&invalid_rid);
        assert_eq!(
            read_post_move_support(&mut invalid_rid, transform),
            Err(SupportReadError::InvalidProbeRid(0))
        );
        for lane in 0..6 {
            let mut port = FakePlayerMotionPort::valid();
            port.on_floor = true;
            port.probe_hit = true;
            let mut fact = world_floor();
            if lane < 3 {
                fact.point = poison_vector_lane(fact.point, lane);
            } else {
                fact.normal = poison_vector_lane(fact.normal, lane - 3);
            }
            port.probe_contacts = vec![fact];
            let transform = post_transform(&port);
            assert!(matches!(
                read_post_move_support(&mut port, transform),
                Err(SupportReadError::InvalidValue(_))
            ));
        }
    }

    #[test]
    fn support_elevation_keeps_flat_positive_zero_and_checked_extreme_lane_results() {
        let flat = support_elevation_at(Vector3::new(3.0, 0.9, -4.0)).unwrap();
        assert_eq!(flat.y().to_bits(), 0.0_f32.to_bits());

        let high = support_elevation_at(Vector3::new(0.0, 1_000_000.0, 0.0)).unwrap();
        assert_eq!(high.y().to_bits(), 0x4974_23f2);
        let low = support_elevation_at(Vector3::new(0.0, -1_000_000.0, 0.0)).unwrap();
        assert_eq!(low.y().to_bits(), 0xc974_240e);

        for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(support_elevation_at(Vector3::new(0.0, poison, 0.0)).is_err());
        }
    }

    /// An airborne prior about to land on the world floor — the fresh
    /// landing fixture every event test starts from.
    fn landing_port(prior_vertical_mps: f32) -> (FakePlayerMotionPort, MotionState) {
        let mut port = FakePlayerMotionPort::valid();
        port.on_floor = true;
        port.slides = vec![Some(vec![world_floor()])];
        let phase = MotionPhase::Airborne {
            planar_velocity_mps: PlanarVelocity::try_new(0.75, -1.25).unwrap(),
            vertical_velocity_mps: crate::support_motion::FiniteVelocity::try_new(
                prior_vertical_mps,
            )
            .unwrap(),
        };
        (port, MotionState::restore(phase, None, None).unwrap())
    }

    #[test]
    fn invalid_current_time_refuses_before_body_move() {
        for poisoned in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.25] {
            let mut port = FakePlayerMotionPort::valid();
            assert!(
                controlled_player_tick(
                    &mut port,
                    poisoned,
                    MotionState::initial(),
                    PlanarVelocity::try_new(1.0, -2.0).unwrap(),
                    1.0 / 60.0,
                    SupportMotionConfig::PLAYER_DEFAULT,
                )
                .is_err()
            );
            assert!(!port.moved);
            assert_eq!(
                port.effect_trace(),
                [
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::DisablePhysics,
                ]
            );
        }
    }

    #[test]
    fn tick_success_carries_only_the_fresh_landing_event() {
        let (mut port, prior) = landing_port(-3.0);
        let success = controlled_player_tick(
            &mut port,
            12.5,
            prior,
            PlanarVelocity::ZERO,
            1.0 / 60.0,
            SupportMotionConfig::PLAYER_DEFAULT,
        )
        .unwrap();
        assert!(matches!(success.phase_before, MotionPhase::Airborne { .. }));
        let landing = success
            .landing
            .expect("an airborne tick onto a world floor must carry its fresh event");
        assert_eq!(landing.support().point(), world_floor().point);
        assert_eq!(landing.support().normal(), Vector3::UP);
        // the commanded fall this tick: -3.0 accelerated one 1/60 step
        let expected_impact = (f64::from(-3.0_f32) - 9.8 / 60.0).abs() as f32;
        assert_eq!(
            landing.impact_speed().mps().to_bits(),
            expected_impact.to_bits()
        );
        assert!(matches!(success.state.phase(), MotionPhase::Controlled));
        assert!(success.landing_request.is_some());
    }

    #[test]
    fn restored_last_landing_never_becomes_a_fresh_event() {
        let support = SupportContact::try_new(world_floor().point, Vector3::UP).unwrap();
        let old = crate::support_motion::LandingEvent::try_new(3.25, support).unwrap();
        let prior =
            MotionState::restore(MotionPhase::Controlled, Some(support), Some(old)).unwrap();
        let mut port = FakePlayerMotionPort::valid();
        port.on_floor = true;
        port.slides = vec![Some(vec![world_floor()])];
        let success = controlled_player_tick(
            &mut port,
            3.0,
            prior,
            PlanarVelocity::ZERO,
            1.0 / 60.0,
            SupportMotionConfig::PLAYER_DEFAULT,
        )
        .unwrap();
        assert!(success.landing.is_none());
        assert!(success.landing_request.is_none());
        assert_eq!(success.state.last_landing(), Some(old));
    }

    #[test]
    fn tick_success_defers_collision_pair_until_callback_commit() {
        let (mut port, prior) = landing_port(-3.0);
        let success = controlled_player_tick(
            &mut port,
            4.0,
            prior,
            PlanarVelocity::ZERO,
            1.0 / 60.0,
            SupportMotionConfig::PLAYER_DEFAULT,
        )
        .unwrap();
        // a landing changes the pair airborne → controlled, yet the
        // coordinator only derives it; the callback commit applies it
        assert_eq!(
            (success.collision_layer, success.collision_mask),
            (2, 4_294_967_291)
        );
        assert_eq!(
            port.effect_trace(),
            [
                MotionTrace::SetVelocity(Vector3::new(
                    0.75,
                    (f64::from(-3.0_f32) - 9.8 / 60.0) as f32,
                    -1.25
                )),
                MotionTrace::MoveAndSlide,
            ]
        );
    }

    #[test]
    fn audible_landing_request_owns_wave_and_reflection_proofs_for_current_time() {
        let (mut port, prior) = landing_port(-3.0);
        let now = 12.5_f64;
        let config = SupportMotionConfig::PLAYER_DEFAULT;
        let success = controlled_player_tick(
            &mut port,
            now,
            prior,
            PlanarVelocity::ZERO,
            1.0 / 60.0,
            config,
        )
        .unwrap();
        let landing = success.landing.expect("the fixture lands");
        let voice = crate::support_motion::landing_voice(landing, config)
            .expect("a 3.16 m/s impact is audible");
        let request = success
            .landing_request
            .expect("an audible landing must arrive fully prepared");
        let (command, prepared_now, wave_proof, reflection_proof) = request.into_emit_parts();
        let (kind, at, max_r, speed, gain, echoes, normal) = command;
        assert_eq!(kind, 2);
        // the support point lifted by the contact birth height, exactly
        assert_eq!(at, Vector3::new(2.0, 0.04, -3.0));
        assert_eq!(max_r.to_bits(), voice.range_m().to_bits());
        assert_eq!(speed.to_bits(), 4.0_f64.to_bits());
        assert_eq!(gain.to_bits(), voice.gain().to_bits());
        assert_eq!(echoes, 2);
        assert_eq!(normal, Vector3::UP);
        assert_eq!(prepared_now.value().to_bits(), now.to_bits());
        // the WAVE proof pins the landing's own kind and gain — the
        // reflection's internal synthetic wave is kind 0 / gain 1.0 and
        // cannot substitute for it
        assert_eq!(wave_proof.slot().kind, 2);
        assert_eq!(wave_proof.slot().pos, at);
        let expected_packed = (2.0 * 10.0 + voice.gain() * 9.0) as f32;
        assert_eq!(wave_proof.slot().dat.w.to_bits(), expected_packed.to_bits());
        assert_eq!(wave_proof.raw_speed().to_bits(), 4.0_f64.to_bits());
        // the REFLECTION proof pins origin, normal, reach, budget, time
        let reflected = reflection_proof.request();
        assert_eq!(reflected.at, at);
        assert_eq!(reflected.normal, Vector3::UP);
        assert_eq!(reflected.max_r.to_bits(), voice.range_m().to_bits());
        assert_eq!(reflected.speed.to_bits(), 4.0_f64.to_bits());
        assert_eq!(reflected.max_echoes, 2);
        assert_eq!(reflected.now.to_bits(), now.to_bits());
    }

    #[test]
    fn invalid_landing_request_uses_the_exact_post_move_refusal_trace() {
        let mut out_of_envelope = world_floor();
        out_of_envelope.point = Vector3::new(0.0, 2_000_000.0, 0.0);
        let mut overflowing_normal = world_floor();
        overflowing_normal.normal = Vector3::new(0.0, 3.0e38, 0.0);
        for fact in [out_of_envelope, overflowing_normal] {
            let (mut port, prior) = landing_port(-3.0);
            port.slides = vec![Some(vec![fact])];
            let saved = port.pre_transform;
            assert!(
                controlled_player_tick(
                    &mut port,
                    5.0,
                    prior,
                    PlanarVelocity::ZERO,
                    1.0 / 60.0,
                    SupportMotionConfig::PLAYER_DEFAULT,
                )
                .is_err()
            );
            assert_transform_bits_eq(port.post_transform, saved);
            assert_eq!(port.post_velocity, Vector3::ZERO);
            assert_eq!(
                port.effect_trace()[1..],
                [
                    MotionTrace::MoveAndSlide,
                    MotionTrace::SetTransform(saved),
                    MotionTrace::SetVelocity(Vector3::ZERO),
                    MotionTrace::DisablePhysics,
                ]
            );
        }
    }

    #[derive(Debug)]
    struct FakeCaneQueryPort {
        player_transform: Transform3D,
        camera: Option<(Transform3D, Vector3)>,
        answers: Vec<CaneRayAnswer>,
        rays: Vec<(Vector3, Vector3)>,
    }

    impl FakeCaneQueryPort {
        fn standing_at(origin: Vector3) -> Self {
            Self {
                player_transform: Transform3D::new(Basis::IDENTITY, origin),
                camera: Some((
                    Transform3D::new(Basis::IDENTITY, origin + Vector3::new(0.0, 0.7, 0.0)),
                    Vector3::ZERO,
                )),
                answers: Vec::new(),
                rays: Vec::new(),
            }
        }
    }

    impl CaneQueryPort for FakeCaneQueryPort {
        fn player_transform(&mut self) -> Transform3D {
            self.player_transform
        }

        fn camera_transform(&mut self) -> Option<Transform3D> {
            self.camera.map(|(transform, _)| transform)
        }

        fn camera_rotation(&mut self) -> Option<Vector3> {
            self.camera.map(|(_, rotation)| rotation)
        }

        fn cast_ray(&mut self, from: Vector3, to: Vector3) -> CaneRayAnswer {
            self.rays.push((from, to));
            if self.answers.is_empty() {
                CaneRayAnswer::Miss
            } else {
                self.answers.remove(0)
            }
        }
    }

    #[test]
    fn cane_endpoints_translate_once_from_one_checked_support_datum() {
        let support = 1.35_f32 - 0.9_f32;
        let mut port = FakeCaneQueryPort::standing_at(Vector3::new(2.0, 1.35, -3.0));
        port.answers = vec![
            CaneRayAnswer::Miss,
            CaneRayAnswer::Hit {
                position: Vector3::new(2.0, 1.9, -4.7),
                normal: Vector3::UP,
            },
        ];
        let probe = prepare_cane_rest(&mut port, 0.0).unwrap();
        assert_eq!(port.rays.len(), 2);
        let (wall_from, wall_to) = port.rays[0];
        assert_eq!(wall_from, Vector3::new(2.0, support + 0.85, -3.0));
        assert_eq!(wall_to.x.to_bits(), 2.0_f32.to_bits());
        assert_eq!(wall_to.y.to_bits(), (support + 0.85).to_bits());
        // the scan endpoint: from.z minus the full scan length, exactly
        assert_eq!(wall_to.z.to_bits(), (-3.0_f32 - 3.4_f32).to_bits());
        let (down_from, down_to) = port.rays[1];
        assert_eq!(down_from, Vector3::new(2.0, support + 1.05, -4.7));
        assert_eq!(down_to, Vector3::new(2.0, support - 0.10, -4.7));
        assert!(probe.supported);
        assert_eq!(probe.tip.x.to_bits(), 2.0_f32.to_bits());
        // legacy settle law, bit for bit: the struck surface plus 0.02
        assert_eq!(
            probe.tip.y.to_bits(),
            ((f64::from(1.9_f32) + 0.02) as f32).to_bits()
        );
        assert_eq!(probe.tip.z.to_bits(), (-4.7_f32).to_bits());

        // no strike below: the tip hangs at the raised fallback height
        let mut open = FakeCaneQueryPort::standing_at(Vector3::new(2.0, 1.35, -3.0));
        let probe = prepare_cane_rest(&mut open, 0.0).unwrap();
        assert_eq!(open.rays.len(), 2);
        assert!(!probe.supported);
        assert_eq!(probe.tip, Vector3::new(2.0, support + 0.02, -4.7));

        // a cooled-down tap is swallowed with ZERO queries
        let now = prepare_time(2.0).unwrap();
        let recent = PreparedLastTap::try_new(1.9, now).unwrap();
        let mut cooled = FakeCaneQueryPort::standing_at(Vector3::new(2.0, 1.35, -3.0));
        assert!(
            prepare_cane_tap(&mut cooled, now, recent)
                .unwrap()
                .is_none()
        );
        assert!(cooled.rays.is_empty());
    }

    /// A camera pitched exactly vertical is finite and admitted by every
    /// sample validator, yet its aim has no horizontal shadow: the swish
    /// must refuse the degenerate eye explicitly — never panic across the
    /// FFI on a zero-vector normalization.
    #[test]
    fn a_vertical_aim_swish_refuses_the_degenerate_camera_without_panic() {
        let now = prepare_time(2.0).unwrap();
        let tap_clock = PreparedLastTap::try_new(-10.0, now).unwrap();
        let pitch = -std::f64::consts::FRAC_PI_2;
        let mut port = FakeCaneQueryPort::standing_at(Vector3::new(2.0, 0.9, -3.0));
        port.camera = Some((
            Transform3D::new(
                // exact columns: forward (-col_c) points straight down
                Basis::from_cols(
                    Vector3::new(1.0, 0.0, 0.0),
                    Vector3::new(0.0, 0.0, -1.0),
                    Vector3::new(0.0, 1.0, 0.0),
                ),
                Vector3::new(2.0, 1.6, -3.0),
            ),
            Vector3::new(pitch as f32, 0.0, 0.0),
        ));
        // straight-down aim misses, the rest finds nothing raised, the
        // pitch qualifies for nothing supported: the swish is the branch
        let verdict = prepare_cane_tap(&mut port, now, tap_clock);
        assert!(verdict.is_err());
        assert_eq!(port.rays.len(), 3); // aim, wall scan, settle probe
    }

    #[test]
    fn poisoned_cane_player_or_camera_sample_queries_nothing_and_changes_no_cane_state() {
        let now = prepare_time(2.0).unwrap();
        let tap_clock = PreparedLastTap::try_new(-10.0, now).unwrap();
        let standing = Vector3::new(2.0, 0.9, -3.0);
        for lane in 0..12 {
            let mut rest_port = FakeCaneQueryPort::standing_at(standing);
            rest_port.player_transform = poison_transform_lane(rest_port.player_transform, lane);
            assert!(prepare_cane_rest(&mut rest_port, 0.0).is_err());
            assert!(rest_port.rays.is_empty());

            let mut tap_port = FakeCaneQueryPort::standing_at(standing);
            tap_port.player_transform = poison_transform_lane(tap_port.player_transform, lane);
            assert!(prepare_cane_tap(&mut tap_port, now, tap_clock).is_err());
            assert!(tap_port.rays.is_empty());

            let mut camera_port = FakeCaneQueryPort::standing_at(standing);
            let (camera_transform, camera_rotation) = camera_port.camera.unwrap();
            camera_port.camera = Some((
                poison_transform_lane(camera_transform, lane),
                camera_rotation,
            ));
            assert!(prepare_cane_tap(&mut camera_port, now, tap_clock).is_err());
            assert!(camera_port.rays.is_empty());
        }
        for lane in 0..3 {
            let mut port = FakeCaneQueryPort::standing_at(standing);
            let (camera_transform, camera_rotation) = port.camera.unwrap();
            port.camera = Some((camera_transform, poison_vector_lane(camera_rotation, lane)));
            assert!(prepare_cane_tap(&mut port, now, tap_clock).is_err());
            assert!(port.rays.is_empty());
        }
        let mut missing = FakeCaneQueryPort::standing_at(standing);
        missing.camera = None;
        assert!(prepare_cane_tap(&mut missing, now, tap_clock).is_err());
        assert!(missing.rays.is_empty());
    }

    #[test]
    fn poisoned_cane_query_endpoint_queries_nothing_and_changes_no_cane_state() {
        // a legal actor position whose derived wall-scan endpoint leaves
        // the pose envelope: refused before the port is asked anything
        let mut port = FakeCaneQueryPort::standing_at(Vector3::new(0.0, 0.9, -1_000_000.0));
        assert!(prepare_cane_rest(&mut port, 0.0).is_err());
        assert!(port.rays.is_empty());
    }

    #[test]
    fn poisoned_or_malformed_cane_hit_changes_no_tap_state_or_wave() {
        let now = prepare_time(2.0).unwrap();
        let tap_clock = PreparedLastTap::try_new(-10.0, now).unwrap();
        let standing = Vector3::new(2.0, 0.9, -3.0);

        // malformed aim answer: refused after exactly the one aim query
        let mut port = FakeCaneQueryPort::standing_at(standing);
        port.answers = vec![CaneRayAnswer::Malformed];
        assert!(prepare_cane_tap(&mut port, now, tap_clock).is_err());
        assert_eq!(port.rays.len(), 1);

        for lane in 0..3 {
            let mut position_port = FakeCaneQueryPort::standing_at(standing);
            position_port.answers = vec![CaneRayAnswer::Hit {
                position: poison_vector_lane(Vector3::new(2.0, 1.0, -4.0), lane),
                normal: Vector3::UP,
            }];
            assert!(prepare_cane_tap(&mut position_port, now, tap_clock).is_err());
            assert_eq!(position_port.rays.len(), 1);

            let mut normal_port = FakeCaneQueryPort::standing_at(standing);
            normal_port.answers = vec![CaneRayAnswer::Hit {
                position: Vector3::new(2.0, 1.0, -4.0),
                normal: poison_vector_lane(Vector3::UP, lane),
            }];
            assert!(prepare_cane_tap(&mut normal_port, now, tap_clock).is_err());
            assert_eq!(normal_port.rays.len(), 1);
        }

        // malformed wall answer in the rest path: refused after [wall]
        let mut wall = FakeCaneQueryPort::standing_at(standing);
        wall.answers = vec![CaneRayAnswer::Malformed];
        assert!(prepare_cane_rest(&mut wall, 0.0).is_err());
        assert_eq!(wall.rays.len(), 1);

        // poisoned down hit: refused after [wall, down]
        for lane in 0..3 {
            let mut down = FakeCaneQueryPort::standing_at(standing);
            down.answers = vec![
                CaneRayAnswer::Miss,
                CaneRayAnswer::Hit {
                    position: poison_vector_lane(Vector3::new(2.0, 0.0, -4.7), lane),
                    normal: Vector3::UP,
                },
            ];
            assert!(prepare_cane_rest(&mut down, 0.0).is_err());
            assert_eq!(down.rays.len(), 2);
        }

        // a malformed rest inside the TAP path: the exact aim/wall/down
        // trace ends at the malformed answer, and nothing is returned
        let mut tap_rest = FakeCaneQueryPort::standing_at(standing);
        tap_rest.answers = vec![
            CaneRayAnswer::Miss,
            CaneRayAnswer::Miss,
            CaneRayAnswer::Malformed,
        ];
        assert!(prepare_cane_tap(&mut tap_rest, now, tap_clock).is_err());
        assert_eq!(tap_rest.rays.len(), 3);
    }

    #[test]
    fn prepared_last_tap_accepts_only_the_exact_sentinel_or_elapsed_time() {
        let now = prepare_time(12.5).unwrap();
        for accepted in [-10.0_f64, 0.0, 12.5] {
            assert_eq!(
                prepare_last_tap(accepted, now).unwrap().to_bits(),
                accepted.to_bits()
            );
        }
        for refused in [
            -9.0,
            12.5_f64.next_up(),
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            let error = prepare_last_tap(refused, now)
                .expect_err("only the sentinel or an elapsed timestamp may pass");
            assert_eq!(error.path, "hero.last_tap");
        }
    }
}
