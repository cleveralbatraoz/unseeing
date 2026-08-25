//! Pure, atomic preparation of the player's complete visible frame.

use std::fmt;

use godot::builtin::{Transform3D, Vector2, Vector3};

use crate::limbs::{LimbBuf, sphere, tube};
use crate::observe::reflect::{CheckedReflectionRequest, ReflectionRequest, ReflectionValueError};
use crate::pulse_pool::{CheckedWave, OMNI_COS, WaveOrigin, WaveOriginError, WaveValueError};
use crate::render::{self, Role};
use crate::support_motion::{
    ActorPosition, ActorTransform, ActorVelocity, FiniteRotation, FootstepSuppression,
    LandingEvent, MotionValueError, PosePoint, QueuedWaveGate, StepDuration, SupportElevation,
    SupportMotionConfig, landing_voice,
};
use crate::temporal::PreparedTime;
use crate::viewmodel::{self, LegSide, PlanarAxes, Viewmodel};

/// Eye height above the supporting surface.
pub(crate) const EYE: f64 = 1.6;

/// Authored world-root height of a standing hero on flat support.
pub(crate) const PLAYER_STANDING_ROOT_Y: f64 = 0.9;

/// A contact wave is born this far above its accepted support.
pub(crate) const CONTACT_BIRTH_HEIGHT_M: f32 = 0.04;

/// Camera rest height in player-local space.
pub(crate) const CAM_BASE_Y: f64 = EYE - PLAYER_STANDING_ROOT_Y;

/// A tap clock proved to be either the exact initial sentinel or elapsed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PreparedLastTap(f64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreparedLastTapError;

impl PreparedLastTap {
    pub(crate) fn try_new(raw: f64, now: PreparedTime) -> Result<Self, PreparedLastTapError> {
        let sentinel = raw.to_bits() == (-10.0_f64).to_bits();
        let elapsed = raw.is_finite() && raw >= 0.0 && raw <= now.value();
        if sentinel || elapsed {
            Ok(Self(raw))
        } else {
            Err(PreparedLastTapError)
        }
    }

    pub(crate) fn raw(self) -> f64 {
        self.0
    }
}

impl fmt::Display for PreparedLastTapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("must be the exact initial sentinel or an elapsed simulation time")
    }
}

/// Every checked input read once at the engine boundary for one visual frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HeroVisualSample {
    now: PreparedTime,
    dt: StepDuration,
    player_transform: ActorTransform,
    player_rotation: FiniteRotation,
    position: ActorPosition,
    support: SupportElevation,
    velocity: ActorVelocity,
    camera_local_transform: ActorTransform,
    camera_rotation: FiniteRotation,
    axes: PlanarAxes,
    tap_target: PosePoint,
    cane_rest_tip: PosePoint,
    cane_rest_supported: bool,
    last_tap: PreparedLastTap,
    controlled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeroVisualSampleError;

impl fmt::Display for HeroVisualSampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the sampled transform and position must be one engine read")
    }
}

impl HeroVisualSample {
    #[expect(
        clippy::too_many_arguments,
        reason = "the sample deliberately makes every engine dependency explicit"
    )]
    pub(crate) fn try_new(
        now: PreparedTime,
        dt: StepDuration,
        player_transform: ActorTransform,
        player_rotation: FiniteRotation,
        position: ActorPosition,
        support: SupportElevation,
        velocity: ActorVelocity,
        camera_local_transform: ActorTransform,
        camera_rotation: FiniteRotation,
        axes: PlanarAxes,
        tap_target: PosePoint,
        cane_rest_tip: PosePoint,
        cane_rest_supported: bool,
        last_tap: PreparedLastTap,
        controlled: bool,
    ) -> Result<Self, HeroVisualSampleError> {
        let transform_position = player_transform.position().world();
        let sampled_position = position.world();
        if !vector_bits_equal(transform_position, sampled_position) {
            return Err(HeroVisualSampleError);
        }
        Ok(Self {
            now,
            dt,
            player_transform,
            player_rotation,
            position,
            support,
            velocity,
            camera_local_transform,
            camera_rotation,
            axes,
            tap_target,
            cane_rest_tip,
            cane_rest_supported,
            last_tap,
            controlled,
        })
    }
}

#[derive(Debug)]
struct PreparedWaveCommand {
    kind: i64,
    at: Vector3,
    max_r: f64,
    speed: f64,
    gain: f64,
    echoes: i64,
    normal: Vector3,
    gate: QueuedWaveGate,
}

impl PreparedWaveCommand {
    fn into_parts(self) -> (i64, Vector3, f64, f64, f64, i64, Vector3, QueuedWaveGate) {
        (
            self.kind,
            self.at,
            self.max_r,
            self.speed,
            self.gain,
            self.echoes,
            self.normal,
            self.gate,
        )
    }
}

/// A fixed ordinary footstep with both independent admission proofs retained.
#[derive(Debug)]
pub(crate) struct PreparedFootstepRequest {
    command: PreparedWaveCommand,
    now: PreparedTime,
    wave_proof: CheckedWave,
    reflection_proof: CheckedReflectionRequest,
}

impl PreparedFootstepRequest {
    #[expect(
        clippy::type_complexity,
        reason = "one consuming door carries the raw command, its prepared instant, and both proofs"
    )]
    pub(crate) fn into_player_parts(
        self,
    ) -> (
        (i64, Vector3, f64, f64, f64, i64, Vector3, QueuedWaveGate),
        PreparedTime,
        CheckedWave,
        CheckedReflectionRequest,
    ) {
        (
            self.command.into_parts(),
            self.now,
            self.wave_proof,
            self.reflection_proof,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FootstepPreparationError {
    Wave(WaveValueError),
    Reflection(ReflectionValueError),
}

/// Why an audible landing could not be prepared into a complete request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LandingPreparationError {
    Origin(WaveOriginError),
    Wave(WaveValueError),
    Reflection(ReflectionValueError),
}

impl fmt::Display for LandingPreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Origin(error) => {
                write!(
                    formatter,
                    "landing origin {}: {}",
                    error.axis(),
                    error.rule()
                )
            }
            Self::Wave(error) => {
                write!(
                    formatter,
                    "landing wave {}: {}",
                    error.field(),
                    error.rule()
                )
            }
            Self::Reflection(error) => write!(
                formatter,
                "landing reflection {}: {}",
                error.field(),
                error.reason()
            ),
        }
    }
}

/// The lanes of one reflecting wave command, in emit order — shared by
/// the prepared landing and cane accessors.
pub(crate) type ReflectingCommandLanes = (i64, Vector3, f64, f64, f64, i64, Vector3);

/// Every cane vertical law, derived once from one checked support datum.
/// Flat support keeps every legacy bit: each law is the frozen absolute
/// literal lifted by the same single support lane.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CaneVerticals {
    support_y: f32,
}

impl CaneVerticals {
    pub(crate) fn new(support: SupportElevation) -> Self {
        Self {
            support_y: support.y(),
        }
    }

    /// Wall-detection scan height (below tabletops).
    pub(crate) fn wall_scan_y(self) -> f32 {
        self.support_y + 0.85
    }

    /// Top of the settle probe, above any reachable rest surface.
    pub(crate) fn probe_top_y(self) -> f32 {
        self.support_y + 1.05
    }

    /// Bottom of the settle probe, just under the supporting floor.
    pub(crate) fn probe_bottom_y(self) -> f32 {
        self.support_y - 0.10
    }

    /// Where the unsupported tip hangs over open air.
    pub(crate) fn unsupported_tip_y(self) -> f32 {
        self.support_y + 0.02
    }

    /// A rest tip clearly above the supporting floor — table, chair seat.
    pub(crate) fn is_raised_tip(self, tip_y: f32) -> bool {
        f64::from(tip_y) > f64::from(self.support_y) + 0.15
    }

    /// An aimed strike low enough to read as the supporting floor.
    pub(crate) fn is_floorish_hit(self, hit_y: f32) -> bool {
        f64::from(hit_y) < f64::from(self.support_y) + 0.20
    }

    /// The silent air-swish target height for the given eye pitch.
    pub(crate) fn swish_target_y(self, pitch: f64) -> f64 {
        f64::from(self.support_y) + (EYE + pitch.tan() * 1.5).clamp(0.3, 1.7)
    }
}

/// Arm + white cane: what can truly be touched.
pub(crate) const CANE_REACH: f64 = 1.7;

/// Wall-detection ray length.
pub(crate) const CANE_SCAN_LENGTH: f64 = 3.4;

/// The tip stops this far short of a struck wall face.
pub(crate) const WALL_BACKOFF: f64 = 0.06;

/// The full strike voice — raised surfaces and aimed strikes: (range, gain).
pub(crate) const CANE_FULL_VOICE: (f64, f64) = (6.0, 1.0);

/// The softer floor voice — the supporting floor itself: (range, gain).
pub(crate) const CANE_FLOOR_VOICE: (f64, f64) = (5.0, 0.85);

/// The aim ray from a proven eye, both endpoints proven inside the pose
/// envelope before any physics query may run.
pub(crate) fn cane_aim_ray(
    from: Vector3,
    aim: Vector3,
) -> Result<(Vector3, Vector3), MotionValueError> {
    let to = from + aim * CANE_REACH as f32;
    PosePoint::try_new(from)?;
    PosePoint::try_new(to)?;
    Ok((from, to))
}

/// The cane's wall scan for one validated player sample: endpoints at the
/// support-relative scan height, proven before the port is asked.
pub(crate) fn cane_wall_scan_ray(
    verticals: CaneVerticals,
    gp: Vector3,
    direction: Vector3,
) -> Result<(Vector3, Vector3), MotionValueError> {
    let from = Vector3::new(gp.x, verticals.wall_scan_y(), gp.z);
    let to = from + direction * CANE_SCAN_LENGTH as f32;
    PosePoint::try_new(from)?;
    PosePoint::try_new(to)?;
    Ok((from, to))
}

/// Where the tip comes to rest horizontally: the full cane reach,
/// shortened by a struck wall, along the sweep direction.
pub(crate) fn cane_tip_column(gp: Vector3, direction: Vector3, wall_d: f64) -> (f64, f64) {
    let reach = CANE_REACH.min(wall_d - WALL_BACKOFF);
    (
        f64::from(gp.x) + f64::from(direction.x) * reach,
        f64::from(gp.z) + f64::from(direction.z) * reach,
    )
}

/// The settle probe over the tip column, endpoints proven.
pub(crate) fn cane_settle_ray(
    verticals: CaneVerticals,
    px: f64,
    pz: f64,
) -> Result<(Vector3, Vector3), MotionValueError> {
    let top = Vector3::new(px as f32, verticals.probe_top_y(), pz as f32);
    let bottom = Vector3::new(px as f32, verticals.probe_bottom_y(), pz as f32);
    PosePoint::try_new(top)?;
    PosePoint::try_new(bottom)?;
    Ok((top, bottom))
}

/// One settled rest: on a struck surface the tip hovers the frozen 0.02 m
/// above it; over open air it hangs at the support-relative fallback. The
/// returned tip is proven before anything may publish it.
pub(crate) fn settle_cane_rest(
    verticals: CaneVerticals,
    px: f64,
    pz: f64,
    struck_y: Option<f32>,
) -> Result<(Vector3, bool), MotionValueError> {
    let (tip, supported) = match struck_y {
        Some(surface_y) => (
            Vector3::new(px as f32, (f64::from(surface_y) + 0.02) as f32, pz as f32),
            true,
        ),
        None => (
            Vector3::new(px as f32, verticals.unsupported_tip_y(), pz as f32),
            false,
        ),
    };
    PosePoint::try_new(tip)?;
    Ok((tip, supported))
}

/// The aimed strike's voice: the floor voice when the struck face reads
/// as the supporting floor (an upward-enough normal, low enough relative
/// to the player's own support), the full voice otherwise.
pub(crate) fn aimed_strike_voice(
    verticals: CaneVerticals,
    hit_y: f32,
    normal_y: f32,
) -> (f64, f64) {
    if f64::from(normal_y) > 0.7 && verticals.is_floorish_hit(hit_y) {
        CANE_FLOOR_VOICE
    } else {
        CANE_FULL_VOICE
    }
}

/// The rest tap's three voices, decided from the settle and the eye.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestTapVerdict {
    /// A raised surface — tabletop, chair seat: the full voice.
    Raised,
    /// The supporting floor, tapped deliberately while looking down.
    Floor,
    /// Open air: no wave at all — only the strike animation remembers.
    Swish,
}

pub(crate) fn rest_tap_verdict(
    verticals: CaneVerticals,
    supported: bool,
    tip_y: f32,
    pitch: f64,
) -> RestTapVerdict {
    if supported && verticals.is_raised_tip(tip_y) {
        RestTapVerdict::Raised
    } else if supported && pitch <= -0.12 {
        RestTapVerdict::Floor
    } else {
        RestTapVerdict::Swish
    }
}

/// The silent swish's remembered target: 1.5 m along the flat look, at
/// the support-relative swish height, proven before it may install.
pub(crate) fn swish_target(
    verticals: CaneVerticals,
    from: Vector3,
    flat: Vector3,
    pitch: f64,
) -> Result<PosePoint, MotionValueError> {
    let reach = from + flat * 1.5;
    PosePoint::try_new(Vector3::new(
        reach.x,
        verticals.swish_target_y(pitch) as f32,
        reach.z,
    ))
}

/// The aim's horizontal shadow, normalized — or `None` for an eye looking
/// exactly along the vertical axis, whose swish has no direction at all.
/// Total: the zero vector never reaches a panicking normalization.
pub(crate) fn horizontal_aim(aim: Vector3) -> Option<Vector3> {
    Vector3::new(aim.x, 0.0, aim.z).try_normalized()
}

/// Why a reflecting cane command could not be prepared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanePreparationError {
    Origin(WaveOriginError),
    Wave(WaveValueError),
    Reflection(ReflectionValueError),
}

impl fmt::Display for CanePreparationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Origin(error) => {
                write!(formatter, "cane origin {}: {}", error.axis(), error.rule())
            }
            Self::Wave(error) => {
                write!(formatter, "cane wave {}: {}", error.field(), error.rule())
            }
            Self::Reflection(error) => write!(
                formatter,
                "cane reflection {}: {}",
                error.field(),
                error.reason()
            ),
        }
    }
}

/// A fully prepared reflecting cane strike: the raw command plus BOTH
/// admission proofs, constructible only here, consumed whole by the
/// player through one accessor — exactly the shoe/landing shape.
#[derive(Debug)]
pub(crate) struct PreparedCaneRequest {
    kind: i64,
    at: Vector3,
    max_r: f64,
    speed: f64,
    gain: f64,
    echoes: i64,
    normal: Vector3,
    now: PreparedTime,
    wave_proof: CheckedWave,
    reflection_proof: CheckedReflectionRequest,
}

impl PreparedCaneRequest {
    /// The one consuming accessor: command lanes, birth instant, proofs.
    pub(crate) fn into_emit_parts(
        self,
    ) -> (
        ReflectingCommandLanes,
        PreparedTime,
        CheckedWave,
        CheckedReflectionRequest,
    ) {
        (
            (
                self.kind,
                self.at,
                self.max_r,
                self.speed,
                self.gain,
                self.echoes,
                self.normal,
            ),
            self.now,
            self.wave_proof,
            self.reflection_proof,
        )
    }
}

/// Prepare one reflecting cane strike: kind 0 at 5.5 m/s with six echoes
/// — the frozen tap voice — at the given already-validated strike point,
/// with the surface's own normal and the mode's range and gain.
pub(crate) fn prepare_cane_request(
    at: PosePoint,
    max_r: f64,
    gain: f64,
    normal: Vector3,
    now: PreparedTime,
) -> Result<PreparedCaneRequest, CanePreparationError> {
    let origin = WaveOrigin::try_new(at.world()).map_err(CanePreparationError::Origin)?;
    let at = origin.world();
    let wave_proof = CheckedWave::prepare(0, at, max_r, 5.5, gain, now, Vector3::ZERO, OMNI_COS)
        .map_err(CanePreparationError::Wave)?;
    let reflection_proof = CheckedReflectionRequest::prepare(ReflectionRequest {
        at,
        normal,
        max_r,
        speed: 5.5,
        max_echoes: 6,
        now: now.value(),
    })
    .map_err(CanePreparationError::Reflection)?;
    Ok(PreparedCaneRequest {
        kind: 0,
        at,
        max_r,
        speed: 5.5,
        gain,
        echoes: 6,
        normal,
        now,
        wave_proof,
        reflection_proof,
    })
}

/// A fully prepared, already audible landing voice: the raw reflecting
/// command plus BOTH independent admission proofs, constructible only
/// here. The player consumes it whole through one accessor and revalidates
/// nothing.
#[derive(Debug)]
pub(crate) struct PreparedLandingRequest {
    kind: i64,
    at: Vector3,
    max_r: f64,
    speed: f64,
    gain: f64,
    echoes: i64,
    normal: Vector3,
    now: PreparedTime,
    wave_proof: CheckedWave,
    reflection_proof: CheckedReflectionRequest,
}

impl PreparedLandingRequest {
    /// The one consuming accessor: command lanes, the prepared birth
    /// instant, and both retained proofs.
    pub(crate) fn into_emit_parts(
        self,
    ) -> (
        ReflectingCommandLanes,
        PreparedTime,
        CheckedWave,
        CheckedReflectionRequest,
    ) {
        (
            (
                self.kind,
                self.at,
                self.max_r,
                self.speed,
                self.gain,
                self.echoes,
                self.normal,
            ),
            self.now,
            self.wave_proof,
            self.reflection_proof,
        )
    }
}

/// Prepare one landing's complete voice while the pre-move transform is
/// still recoverable. A silent voice (`landing_voice == None`) is a valid
/// prepared result and touches neither emitter nor any physics space; an
/// audible one owns kind 2, the support point lifted by the contact birth
/// height, the voice's own range and gain, the 4.0 m/s wave-law speed,
/// two echoes, the accepted support normal, and both admission proofs.
pub(crate) fn prepare_player_landing(
    event: LandingEvent,
    config: SupportMotionConfig,
    now: PreparedTime,
) -> Result<Option<PreparedLandingRequest>, LandingPreparationError> {
    let Some(voice) = landing_voice(event, config) else {
        return Ok(None);
    };
    let support = event.support();
    let origin =
        WaveOrigin::try_new(support.point() + Vector3::new(0.0, CONTACT_BIRTH_HEIGHT_M, 0.0))
            .map_err(LandingPreparationError::Origin)?;
    let at = origin.world();
    let normal = support.normal();
    let wave_proof = CheckedWave::prepare(
        2,
        at,
        voice.range_m(),
        4.0,
        voice.gain(),
        now,
        Vector3::ZERO,
        OMNI_COS,
    )
    .map_err(LandingPreparationError::Wave)?;
    let reflection_proof = CheckedReflectionRequest::prepare(ReflectionRequest {
        at,
        normal,
        max_r: voice.range_m(),
        speed: 4.0,
        max_echoes: 2,
        now: now.value(),
    })
    .map_err(LandingPreparationError::Reflection)?;
    Ok(Some(PreparedLandingRequest {
        kind: 2,
        at,
        max_r: voice.range_m(),
        speed: 4.0,
        gain: voice.gain(),
        echoes: 2,
        normal,
        now,
        wave_proof,
        reflection_proof,
    }))
}

/// Pure state-in/state-out preparation boundary used only for real cadence.
pub(crate) trait FootstepPreparer: Sized {
    fn prepare(
        self,
        origin: WaveOrigin,
        now: PreparedTime,
    ) -> Result<(Self, PreparedFootstepRequest), (Self, FootstepPreparationError)>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CheckedFootstepPreparer;

impl FootstepPreparer for CheckedFootstepPreparer {
    fn prepare(
        self,
        origin: WaveOrigin,
        now: PreparedTime,
    ) -> Result<(Self, PreparedFootstepRequest), (Self, FootstepPreparationError)> {
        let command = PreparedWaveCommand {
            kind: 2,
            at: origin.world(),
            max_r: 1.6,
            speed: 4.0,
            gain: 0.8,
            echoes: 2,
            normal: Vector3::UP,
            gate: QueuedWaveGate::ControlledContact,
        };
        let wave_proof = match CheckedWave::prepare(
            command.kind,
            command.at,
            command.max_r,
            command.speed,
            command.gain,
            now,
            Vector3::ZERO,
            OMNI_COS,
        ) {
            Ok(proof) => proof,
            Err(error) => return Err((self, FootstepPreparationError::Wave(error))),
        };
        let reflection_proof = match CheckedReflectionRequest::prepare(ReflectionRequest {
            at: command.at,
            normal: command.normal,
            max_r: command.max_r,
            speed: command.speed,
            max_echoes: command.echoes,
            now: now.value(),
        }) {
            Ok(proof) => proof,
            Err(error) => return Err((self, FootstepPreparationError::Reflection(error))),
        };
        Ok((
            self,
            PreparedFootstepRequest {
                command,
                now,
                wave_proof,
                reflection_proof,
            },
        ))
    }
}

#[derive(Debug)]
struct HeroVisualCandidate {
    vm: Viewmodel,
    suppression: FootstepSuppression,
    bob: f64,
    cane_sweep: f64,
    shoes: [Vector3; 2],
    cane_vertices: LimbBuf,
    body_vertices: LimbBuf,
    footstep: Option<PreparedFootstepRequest>,
}

/// A fully checked visual frame. Its fields remain inaccessible to adapters.
#[derive(Debug)]
pub(crate) struct HeroVisualNext {
    vm: Viewmodel,
    suppression: FootstepSuppression,
    bob: f64,
    cane_sweep: f64,
    shoes: [Vector3; 2],
    cane_vertices: LimbBuf,
    body_vertices: LimbBuf,
    footstep: Option<PreparedFootstepRequest>,
}

pub(crate) type HeroVisualCommitParts = (
    Viewmodel,
    FootstepSuppression,
    f64,
    f64,
    [Vector3; 2],
    LimbBuf,
    LimbBuf,
    Option<PreparedFootstepRequest>,
);

impl HeroVisualNext {
    pub(crate) fn into_commit_parts(self) -> HeroVisualCommitParts {
        (
            self.vm,
            self.suppression,
            self.bob,
            self.cane_sweep,
            self.shoes,
            self.cane_vertices,
            self.body_vertices,
            self.footstep,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeroVisualError {
    Motion,
    Footstep(FootstepPreparationError),
    Viewmodel,
    Bob,
    CaneSweep,
    Shoe,
    CanePosition,
    CaneNormal,
    CaneLabel,
    BodyPosition,
    BodyNormal,
    BodyLabel,
    FootstepRequest,
}

impl fmt::Display for HeroVisualError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "hero visual candidate refused: {self:?}")
    }
}

#[derive(Debug)]
pub(crate) struct HeroVisualRefusal<P> {
    reason: HeroVisualError,
    cane_scratch: LimbBuf,
    body_scratch: LimbBuf,
    preparer: P,
}

impl<P> HeroVisualRefusal<P> {
    pub(crate) fn into_recovery(self) -> (HeroVisualError, LimbBuf, LimbBuf, P) {
        (
            self.reason,
            self.cane_scratch,
            self.body_scratch,
            self.preparer,
        )
    }
}

pub(crate) fn prepare_hero_visual<P: FootstepPreparer>(
    sample: HeroVisualSample,
    prior_vm: Viewmodel,
    prior_suppression: FootstepSuppression,
    cane_scratch: LimbBuf,
    body_scratch: LimbBuf,
    preparer: P,
) -> Result<(HeroVisualNext, P), HeroVisualRefusal<P>> {
    let (candidate, preparer) = build_candidate(
        sample,
        prior_vm,
        prior_suppression,
        cane_scratch,
        body_scratch,
        preparer,
    )?;
    validate_candidate(candidate, preparer)
}

fn build_candidate<P: FootstepPreparer>(
    sample: HeroVisualSample,
    prior_vm: Viewmodel,
    prior_suppression: FootstepSuppression,
    mut cane_scratch: LimbBuf,
    mut body_scratch: LimbBuf,
    preparer: P,
) -> Result<(HeroVisualCandidate, P), HeroVisualRefusal<P>> {
    let planar_speed = if sample.controlled {
        let velocity = sample.velocity.world();
        f64::from(Vector2::new(velocity.x, velocity.z).length())
    } else {
        0.0
    };
    let mut vm = prior_vm;
    let player_rotation = sample.player_rotation.world();
    let camera_rotation = sample.camera_rotation.world();
    let pose = vm.advance(
        sample.now.value(),
        sample.dt.seconds(),
        planar_speed,
        f64::from(player_rotation.y),
        f64::from(camera_rotation.x),
        sample.last_tap.raw(),
    );

    cane_scratch.clear();
    build_cane_vertices(&mut cane_scratch, sample, pose);

    body_scratch.clear();
    let shoes = match build_body_vertices(&mut body_scratch, sample, pose) {
        Ok(shoes) => shoes,
        Err(_error) => {
            return Err(HeroVisualRefusal {
                reason: HeroVisualError::Motion,
                cane_scratch,
                body_scratch,
                preparer,
            });
        }
    };

    let cadence = vm.footstep(sample.dt.seconds(), pose.moving);
    let (suppression, footstep, preparer) = match cadence {
        None => (prior_suppression, None, preparer),
        Some(side) => {
            let (suppression, suppressed) = prior_suppression.acknowledge();
            if suppressed {
                (suppression, None, preparer)
            } else {
                let shoe = shoes[if side < 0 { 0 } else { 1 }];
                let origin =
                    Vector3::new(shoe.x, sample.support.y() + CONTACT_BIRTH_HEIGHT_M, shoe.z);
                let origin = match WaveOrigin::try_new(origin) {
                    Ok(origin) => origin,
                    Err(_error) => {
                        return Err(HeroVisualRefusal {
                            reason: HeroVisualError::Motion,
                            cane_scratch,
                            body_scratch,
                            preparer,
                        });
                    }
                };
                match preparer.prepare(origin, sample.now) {
                    Ok((preparer, request)) => (suppression, Some(request), preparer),
                    Err((preparer, error)) => {
                        return Err(HeroVisualRefusal {
                            reason: HeroVisualError::Footstep(error),
                            cane_scratch,
                            body_scratch,
                            preparer,
                        });
                    }
                }
            }
        }
    };

    Ok((
        HeroVisualCandidate {
            vm,
            suppression,
            bob: pose.bob,
            cane_sweep: pose.cane_swing * (1.0 - pose.thrust),
            shoes,
            cane_vertices: cane_scratch,
            body_vertices: body_scratch,
            footstep,
        },
        preparer,
    ))
}

fn build_cane_vertices(buffer: &mut LimbBuf, sample: HeroVisualSample, pose: viewmodel::Pose) {
    let bx = 0.016 * pose.leg_phase.sin() * pose.walk_amp + pose.sway_x;
    let by = 0.012 * (pose.leg_phase * 2.0).sin() * pose.walk_amp + pose.sway_y;
    let mut camera_local = sample.camera_local_transform.world();
    camera_local.origin.y = (CAM_BASE_Y + pose.bob) as f32;
    let camera_world = sample.player_transform.world() * camera_local;
    let hand = view_to_world(
        camera_world,
        0.30 + bx,
        -0.40 + by - 0.03 * pose.thrust,
        0.55 + 0.16 * pose.thrust,
    );
    let elbow = view_to_world(camera_world, 0.48 + bx * 0.5, -0.64 + by * 0.5, 0.26);

    let mut rest_tip = sample.cane_rest_tip.world();
    // The rest's supported flag rides the sample for parity with the
    // player's published CaneRest; the hover law has never branched on it
    // (an unsupported tip hovers exactly like a resting one).
    let _supported = sample.cane_rest_supported;
    let lift = viewmodel::cane_lift(pose.walk_amp > 0.5, pose.cane_swing);
    rest_tip.y = (f64::from(rest_tip.y) + 0.12 * lift * (1.0 - pose.thrust)) as f32;
    let tip = rest_tip.lerp(
        sample.tap_target.world(),
        pose.thrust.clamp(0.0, 1.0) as f32,
    );

    let label = render::role_label(Role::HeroCane) as f32;
    tube(buffer, elbow, hand, 0.055, 0.045, label);
    sphere(buffer, hand, 0.055, label);
    tube(buffer, hand, tip, 0.013, 0.010, label);
    sphere(buffer, tip, 0.040, label);
}

fn build_body_vertices(
    buffer: &mut LimbBuf,
    sample: HeroVisualSample,
    pose: viewmodel::Pose,
) -> Result<[Vector3; 2], MotionValueError> {
    let position = sample.position.world();
    let forward = sample.axes.forward();
    let label = render::role_label(Role::HeroBody) as f32;
    let torso_center = Vector3::new(position.x, 0.0, position.z) - forward * 0.20;
    tube(
        buffer,
        Vector3::new(torso_center.x, 0.90, torso_center.z),
        Vector3::new(torso_center.x, 1.28, torso_center.z),
        0.11,
        0.10,
        label,
    );
    sphere(
        buffer,
        Vector3::new(torso_center.x, 1.28, torso_center.z),
        0.10,
        label,
    );
    sphere(
        buffer,
        Vector3::new(torso_center.x, 0.90, torso_center.z),
        0.13,
        label,
    );

    let mut shoes = [Vector3::ZERO; 2];
    for (index, side) in [LegSide::Left, LegSide::Right].into_iter().enumerate() {
        let leg = viewmodel::leg_pose(
            sample.position,
            sample.axes,
            pose.leg_phase,
            pose.walk_amp,
            side,
        )?;
        shoes[index] = leg.shoe;
        tube(buffer, leg.hip, leg.knee, 0.06, 0.05, label);
        sphere(buffer, leg.knee, 0.055, label);
        tube(buffer, leg.knee, leg.ankle, 0.05, 0.04, label);
        sphere(buffer, leg.shoe, 0.08, label);
    }

    // The one transport pass — the cat's `translate_skeleton_y` shape:
    // support enters body geometry exactly once, as a single f32 add on
    // every emitted vertex Y and both shoes, so a raised silhouette IS
    // the translated flat silhouette within half an output ULP per lane.
    let support_y = sample.support.y();
    if support_y != 0.0 {
        for (vertex, _normal, _label) in buffer.iter_mut() {
            vertex.y += support_y;
        }
        for shoe in &mut shoes {
            shoe.y += support_y;
        }
    }
    Ok(shoes)
}

fn validate_candidate<P>(
    candidate: HeroVisualCandidate,
    preparer: P,
) -> Result<(HeroVisualNext, P), HeroVisualRefusal<P>> {
    let reason = validate_candidate_value(&candidate).err();
    if let Some(reason) = reason {
        return Err(HeroVisualRefusal {
            reason,
            cane_scratch: candidate.cane_vertices,
            body_scratch: candidate.body_vertices,
            preparer,
        });
    }
    Ok((
        HeroVisualNext {
            vm: candidate.vm,
            suppression: candidate.suppression,
            bob: candidate.bob,
            cane_sweep: candidate.cane_sweep,
            shoes: candidate.shoes,
            cane_vertices: candidate.cane_vertices,
            body_vertices: candidate.body_vertices,
            footstep: candidate.footstep,
        },
        preparer,
    ))
}

fn validate_candidate_value(candidate: &HeroVisualCandidate) -> Result<(), HeroVisualError> {
    Viewmodel::prepare_restore(candidate.vm.capture()).map_err(|_| HeroVisualError::Viewmodel)?;
    if !candidate.bob.is_finite() {
        return Err(HeroVisualError::Bob);
    }
    if !candidate.cane_sweep.is_finite() {
        return Err(HeroVisualError::CaneSweep);
    }
    for shoe in candidate.shoes {
        PosePoint::try_new(shoe).map_err(|_| HeroVisualError::Shoe)?;
    }
    validate_buffer(
        &candidate.cane_vertices,
        render::role_label(Role::HeroCane) as f32,
        HeroVisualError::CanePosition,
        HeroVisualError::CaneNormal,
        HeroVisualError::CaneLabel,
    )?;
    validate_buffer(
        &candidate.body_vertices,
        render::role_label(Role::HeroBody) as f32,
        HeroVisualError::BodyPosition,
        HeroVisualError::BodyNormal,
        HeroVisualError::BodyLabel,
    )?;
    if let Some(request) = candidate.footstep.as_ref() {
        validate_footstep_request(request)?;
    }
    Ok(())
}

fn validate_buffer(
    buffer: &LimbBuf,
    expected_label: f32,
    position_error: HeroVisualError,
    normal_error: HeroVisualError,
    label_error: HeroVisualError,
) -> Result<(), HeroVisualError> {
    for (position, normal, label) in buffer {
        PosePoint::try_new(*position).map_err(|_| position_error)?;
        if !normal.x.is_finite() || !normal.y.is_finite() || !normal.z.is_finite() {
            return Err(normal_error);
        }
        if label.to_bits() != expected_label.to_bits() {
            return Err(label_error);
        }
    }
    Ok(())
}

fn validate_footstep_request(request: &PreparedFootstepRequest) -> Result<(), HeroVisualError> {
    let command = &request.command;
    if command.kind != 2
        || command.max_r.to_bits() != 1.6_f64.to_bits()
        || command.speed.to_bits() != 4.0_f64.to_bits()
        || command.gain.to_bits() != 0.8_f64.to_bits()
        || command.echoes != 2
        || !vector_bits_equal(command.normal, Vector3::UP)
        || command.gate != QueuedWaveGate::ControlledContact
    {
        return Err(HeroVisualError::FootstepRequest);
    }
    let expected_wave = CheckedWave::prepare(
        command.kind,
        command.at,
        command.max_r,
        command.speed,
        command.gain,
        request.now,
        Vector3::ZERO,
        OMNI_COS,
    )
    .map_err(|_| HeroVisualError::FootstepRequest)?;
    if request.wave_proof.slot() != expected_wave.slot()
        || request.wave_proof.effective_gain().to_bits() != expected_wave.effective_gain().to_bits()
        || request.wave_proof.raw_speed().to_bits() != expected_wave.raw_speed().to_bits()
    {
        return Err(HeroVisualError::FootstepRequest);
    }
    let reflection = request.reflection_proof.request();
    if !vector_bits_equal(reflection.at, command.at)
        || !vector_bits_equal(reflection.normal, command.normal)
        || reflection.max_r.to_bits() != command.max_r.to_bits()
        || reflection.speed.to_bits() != command.speed.to_bits()
        || reflection.max_echoes != command.echoes
        || reflection.now.to_bits() != request.now.value().to_bits()
    {
        return Err(HeroVisualError::FootstepRequest);
    }
    Ok(())
}

fn view_to_world(camera: Transform3D, x: f64, y: f64, z: f64) -> Vector3 {
    camera.origin + camera.basis.col_a() * x as f32 + camera.basis.col_b() * y as f32
        - camera.basis.col_c() * z as f32
}

fn vector_bits_equal(left: Vector3, right: Vector3) -> bool {
    left.x.to_bits() == right.x.to_bits()
        && left.y.to_bits() == right.y.to_bits()
        && left.z.to_bits() == right.z.to_bits()
}

#[cfg(test)]
mod tests {
    use super::*;
    use godot::builtin::{Basis, Transform3D, Vector3};

    use crate::support_motion::{
        ActorPosition, ActorTransform, ActorVelocity, FiniteRotation, FootstepSuppression,
        StepDuration, SupportElevation,
    };
    use crate::temporal::prepare_time;
    use crate::viewmodel::{PlanarAxes, Viewmodel, ViewmodelCapture};

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct CountingPreparer {
        calls: u32,
        origin: Option<Vector3>,
        now: Option<f64>,
    }

    impl CountingPreparer {
        const UNCALLED: Self = Self {
            calls: 0,
            origin: None,
            now: None,
        };
    }

    impl FootstepPreparer for CountingPreparer {
        fn prepare(
            mut self,
            origin: crate::pulse_pool::WaveOrigin,
            now: crate::temporal::PreparedTime,
        ) -> Result<(Self, PreparedFootstepRequest), (Self, FootstepPreparationError)> {
            self.calls += 1;
            self.origin = Some(origin.world());
            self.now = Some(now.value());
            match CheckedFootstepPreparer.prepare(origin, now) {
                Ok((_, request)) => Ok((self, request)),
                Err((_, error)) => Err((self, error)),
            }
        }
    }

    fn transform_at(origin: Vector3) -> Transform3D {
        Transform3D::new(Basis::IDENTITY, origin)
    }

    fn sample(
        now: f64,
        support_y: f32,
        velocity: Vector3,
        controlled: bool,
        player_rotation: Vector3,
        camera_rotation: Vector3,
    ) -> HeroVisualSample {
        let player_position = Vector3::new(2.0, PLAYER_STANDING_ROOT_Y as f32 + support_y, -3.0);
        HeroVisualSample::try_new(
            prepare_time(now).unwrap(),
            StepDuration::from_raw(1.0 / 60.0),
            ActorTransform::try_new(transform_at(player_position)).unwrap(),
            FiniteRotation::try_new(player_rotation).unwrap(),
            ActorPosition::try_new(player_position).unwrap(),
            SupportElevation::try_new(support_y).unwrap(),
            ActorVelocity::try_new(velocity).unwrap(),
            ActorTransform::try_new(transform_at(Vector3::new(0.0, CAM_BASE_Y as f32, 0.0)))
                .unwrap(),
            FiniteRotation::try_new(camera_rotation).unwrap(),
            PlanarAxes::try_new(Vector3::FORWARD, Vector3::RIGHT).unwrap(),
            crate::support_motion::PosePoint::try_new(Vector3::new(2.0, support_y + 0.2, -4.0))
                .unwrap(),
            crate::support_motion::PosePoint::try_new(Vector3::new(2.0, support_y + 0.02, -4.5))
                .unwrap(),
            true,
            PreparedLastTap::try_new(-10.0, prepare_time(now).unwrap()).unwrap(),
            controlled,
        )
        .unwrap()
    }

    fn walking_vm(step_t: f64) -> Viewmodel {
        let mut capture = Viewmodel::new(0.0, 0.0).capture();
        capture.walk_amp = 1.0;
        capture.leg_phase = 0.31;
        capture.swing_phase = 0.57;
        capture.step_t = step_t;
        Viewmodel::restore(capture)
    }

    fn lane_tolerance(expected: f32) -> f64 {
        let next = if expected.is_sign_negative() {
            expected.next_down()
        } else {
            expected.next_up()
        };
        (f64::from(next) - f64::from(expected)).abs()
    }

    fn assert_vector_within_one_output_ulp(actual: Vector3, expected: Vector3) {
        for (actual, expected) in [
            (actual.x, expected.x),
            (actual.y, expected.y),
            (actual.z, expected.z),
        ] {
            assert!(
                (f64::from(actual) - f64::from(expected)).abs() <= lane_tolerance(expected),
                "{actual:?} is farther than one output ULP from {expected:?}"
            );
        }
    }

    /// The reflecting cane command mirrors the landing's shape: one door,
    /// the raw lanes, the prepared instant, and BOTH retained admission
    /// proofs — so no lane and neither proof can drift unpinned.
    #[test]
    fn prepared_cane_request_owns_wave_and_reflection_proofs_for_the_strike() {
        let now = prepare_time(3.25).unwrap();
        let at = crate::support_motion::PosePoint::try_new(Vector3::new(2.0, 0.5, -4.0)).unwrap();
        let normal = Vector3::new(0.0, 0.6, 0.8);
        let request = prepare_cane_request(at, 6.0, 1.0, normal, now).unwrap();
        let (command, prepared_now, wave_proof, reflection_proof) = request.into_emit_parts();
        let (kind, world, max_r, speed, gain, echoes, out_normal) = command;
        assert_eq!(kind, 0);
        assert_eq!(world, Vector3::new(2.0, 0.5, -4.0));
        assert_eq!(max_r.to_bits(), 6.0_f64.to_bits());
        assert_eq!(speed.to_bits(), 5.5_f64.to_bits());
        assert_eq!(gain.to_bits(), 1.0_f64.to_bits());
        assert_eq!(echoes, 6);
        assert_eq!(out_normal, normal);
        assert_eq!(prepared_now.value().to_bits(), 3.25_f64.to_bits());
        // the WAVE proof pins the tap's own kind and gain
        assert_eq!(wave_proof.slot().kind, 0);
        assert_eq!(wave_proof.slot().pos, world);
        let expected_packed = (0.0 * 10.0 + 1.0 * 9.0) as f32;
        assert_eq!(wave_proof.slot().dat.w.to_bits(), expected_packed.to_bits());
        assert_eq!(wave_proof.raw_speed().to_bits(), 5.5_f64.to_bits());
        // the REFLECTION proof pins origin, normal, reach, budget, time
        let reflected = reflection_proof.request();
        assert_eq!(reflected.at, world);
        assert_eq!(reflected.normal, normal);
        assert_eq!(reflected.max_r.to_bits(), 6.0_f64.to_bits());
        assert_eq!(reflected.speed.to_bits(), 5.5_f64.to_bits());
        assert_eq!(reflected.max_echoes, 6);
        assert_eq!(reflected.now.to_bits(), 3.25_f64.to_bits());
        // and the floor voice's lanes survive the same door
        let floor = prepare_cane_request(at, 5.0, 0.85, Vector3::UP, now).unwrap();
        let (floor_command, _, floor_wave, _) = floor.into_emit_parts();
        assert_eq!(floor_command.2.to_bits(), 5.0_f64.to_bits());
        assert_eq!(floor_command.4.to_bits(), 0.85_f64.to_bits());
        let floor_packed = (0.85 * 9.0) as f32;
        assert_eq!(floor_wave.slot().dat.w.to_bits(), floor_packed.to_bits());
    }

    #[test]
    fn prepared_last_tap_accepts_only_the_exact_sentinel_or_elapsed_sample_time() {
        let now = prepare_time(12.5).unwrap();
        for accepted in [-10.0, 0.0, 3.25, 12.5] {
            let prepared = PreparedLastTap::try_new(accepted, now).unwrap();
            assert_eq!(prepared.raw().to_bits(), accepted.to_bits());
        }
        for refused in [
            -10.0_f64.next_down(),
            -10.0_f64.next_up(),
            -0.25,
            12.5_f64.next_up(),
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            assert!(PreparedLastTap::try_new(refused, now).is_err());
        }
    }

    #[test]
    fn prepared_visual_uses_the_next_bob_camera_transform() {
        let sample = sample(
            7.0,
            0.0,
            Vector3::new(1.0, 0.0, 0.0),
            true,
            Vector3::ZERO,
            Vector3::ZERO,
        );
        let prior = walking_vm(0.3);
        let mut expected_vm = prior;
        let pose = expected_vm.advance(7.0, 1.0 / 60.0, 1.0, 0.0, 0.0, -10.0);
        assert_ne!(pose.bob.to_bits(), 0.0_f64.to_bits());

        let ((next, _preparer), expected_hand) = {
            let result = prepare_hero_visual(
                sample,
                prior,
                FootstepSuppression::CLEAR,
                Vec::new(),
                Vec::new(),
                CountingPreparer::UNCALLED,
            )
            .unwrap();
            let camera_origin = Vector3::new(
                2.0,
                (PLAYER_STANDING_ROOT_Y + CAM_BASE_Y + pose.bob) as f32,
                -3.0,
            );
            let bx = 0.016 * pose.leg_phase.sin() * pose.walk_amp + pose.sway_x;
            let by = 0.012 * (pose.leg_phase * 2.0).sin() * pose.walk_amp + pose.sway_y;
            let hand = camera_origin
                + Vector3::RIGHT * (0.30 + bx) as f32
                + Vector3::UP * (-0.40 + by - 0.03 * pose.thrust) as f32
                + Vector3::FORWARD * (0.55 + 0.16 * pose.thrust) as f32;
            (result, hand)
        };

        // The first cane tube occupies 60 vertices; its following sphere's
        // north pole is exactly hand + UP * radius.
        let encoded_hand = next.cane_vertices[60].0 - Vector3::UP * 0.055;
        assert_vector_within_one_output_ulp(encoded_hand, expected_hand);

        // The ten first-ring points of that tube average back to the elbow.
        let mut elbow_sum = Vector3::ZERO;
        for segment in 0..10 {
            elbow_sum += next.cane_vertices[segment * 6].0;
        }
        let encoded_elbow = elbow_sum / 10.0;
        let bx = 0.016 * pose.leg_phase.sin() * pose.walk_amp + pose.sway_x;
        let by = 0.012 * (pose.leg_phase * 2.0).sin() * pose.walk_amp + pose.sway_y;
        let expected_elbow = Vector3::new(
            (2.48 + bx * 0.5) as f32,
            (PLAYER_STANDING_ROOT_Y + CAM_BASE_Y + pose.bob - 0.64 + by * 0.5) as f32,
            -3.26,
        );
        for (actual, expected) in [
            (encoded_elbow.x, expected_elbow.x),
            (encoded_elbow.y, expected_elbow.y),
            (encoded_elbow.z, expected_elbow.z),
        ] {
            assert!(
                (f64::from(actual) - f64::from(expected)).abs() <= 8.0 * lane_tolerance(expected)
            );
        }
    }

    #[test]
    fn prepared_visual_adds_support_once_to_every_body_vertex() {
        let prior = walking_vm(0.3);
        let (flat, _) = prepare_hero_visual(
            sample(
                3.0,
                0.0,
                Vector3::new(1.0, 0.0, 0.0),
                true,
                Vector3::ZERO,
                Vector3::ZERO,
            ),
            prior,
            FootstepSuppression::CLEAR,
            Vec::new(),
            Vec::new(),
            CountingPreparer::UNCALLED,
        )
        .unwrap();
        let (raised, _) = prepare_hero_visual(
            sample(
                3.0,
                0.45,
                Vector3::new(1.0, 0.0, 0.0),
                true,
                Vector3::ZERO,
                Vector3::ZERO,
            ),
            prior,
            FootstepSuppression::CLEAR,
            Vec::new(),
            Vec::new(),
            CountingPreparer::UNCALLED,
        )
        .unwrap();

        assert_eq!(flat.body_vertices.len(), raised.body_vertices.len());
        for (index, (flat, raised)) in flat
            .body_vertices
            .iter()
            .zip(&raised.body_vertices)
            .enumerate()
        {
            assert_eq!(flat.0.x.to_bits(), raised.0.x.to_bits());
            assert_eq!(flat.0.z.to_bits(), raised.0.z.to_bits());
            assert_eq!(flat.1, raised.1);
            assert_eq!(flat.2.to_bits(), raised.2.to_bits());
            let expected = f64::from(flat.0.y) + f64::from(0.45_f32);
            assert!(
                (f64::from(raised.0.y) - expected).abs()
                    <= lane_tolerance(flat.0.y) + lane_tolerance(raised.0.y),
                "vertex {index}: flat={}, raised={}, expected={expected}, tolerance={}",
                flat.0.y,
                raised.0.y,
                lane_tolerance(flat.0.y) + lane_tolerance(raised.0.y),
            );
        }
        // Both shoes ride the same single transport as the vertices.
        for (flat_shoe, raised_shoe) in flat.shoes.into_iter().zip(raised.shoes) {
            assert_eq!(flat_shoe.x.to_bits(), raised_shoe.x.to_bits());
            assert_eq!(flat_shoe.z.to_bits(), raised_shoe.z.to_bits());
            let expected = f64::from(flat_shoe.y) + f64::from(0.45_f32);
            assert!(
                (f64::from(raised_shoe.y) - expected).abs()
                    <= lane_tolerance(flat_shoe.y) + lane_tolerance(raised_shoe.y)
            );
        }
    }

    #[test]
    fn airborne_visual_advances_look_and_cane_but_not_walk_or_cadence() {
        let mut capture = walking_vm(crate::viewmodel::STOP_GRACE).capture();
        capture.last_yaw = 0.0;
        capture.last_pitch = 0.0;
        let prior = Viewmodel::restore(capture);
        let (next, preparer) = prepare_hero_visual(
            sample(
                8.0,
                0.6,
                Vector3::new(20.0, -3.0, -12.0),
                false,
                Vector3::new(0.0, 0.35, 0.0),
                Vector3::new(-0.2, 0.0, 0.0),
            ),
            prior,
            FootstepSuppression::CLEAR,
            Vec::new(),
            Vec::new(),
            CountingPreparer::UNCALLED,
        )
        .unwrap();
        let before = prior.capture();
        let after = next.vm.capture();
        assert_eq!(after.leg_phase.to_bits(), before.leg_phase.to_bits());
        assert_eq!(after.swing_phase.to_bits(), before.swing_phase.to_bits());
        assert_eq!(after.step_t.to_bits(), before.step_t.to_bits());
        assert_ne!(after.cane_swing.to_bits(), before.cane_swing.to_bits());
        assert_ne!(after.sway_x.to_bits(), before.sway_x.to_bits());
        assert_ne!(after.sway_y.to_bits(), before.sway_y.to_bits());
        assert!(next.footstep.is_none());
        assert_eq!(preparer.calls, 0);
    }

    #[test]
    fn prepared_footstep_uses_the_sample_time_and_has_fixed_voice_and_controlled_contact_provenance()
     {
        let support_y = 0.45;
        let (next, preparer) = prepare_hero_visual(
            sample(
                9.25,
                support_y,
                Vector3::new(1.0, 0.0, 0.0),
                true,
                Vector3::ZERO,
                Vector3::ZERO,
            ),
            walking_vm(0.0),
            FootstepSuppression::CLEAR,
            Vec::new(),
            Vec::new(),
            CountingPreparer::UNCALLED,
        )
        .unwrap();
        assert_eq!(preparer.calls, 1);
        assert_eq!(preparer.now.unwrap().to_bits(), 9.25_f64.to_bits());
        let request = next.footstep.as_ref().unwrap();
        let origin = preparer.origin.unwrap();
        assert_eq!(origin.x.to_bits(), next.shoes[1].x.to_bits());
        assert_eq!(
            origin.y.to_bits(),
            (support_y + CONTACT_BIRTH_HEIGHT_M).to_bits()
        );
        assert_eq!(origin.z.to_bits(), next.shoes[1].z.to_bits());
        assert_eq!(request.command.kind, 2);
        assert_eq!(request.command.at, origin);
        assert_eq!(request.command.max_r.to_bits(), 1.6_f64.to_bits());
        assert_eq!(request.command.speed.to_bits(), 4.0_f64.to_bits());
        assert_eq!(request.command.gain.to_bits(), 0.8_f64.to_bits());
        assert_eq!(request.command.echoes, 2);
        assert_eq!(request.command.normal, Vector3::UP);
        assert_eq!(
            request.command.gate,
            crate::support_motion::QueuedWaveGate::ControlledContact
        );
        assert_eq!(request.now.value().to_bits(), 9.25_f64.to_bits());
        assert_eq!(request.wave_proof.slot().pos, origin);
        assert_eq!(request.reflection_proof.request().at, origin);
    }

    #[test]
    fn no_footfall_returns_the_same_uncalled_preparer_and_reuses_both_buffers() {
        let mut cane = Vec::with_capacity(4_096);
        let mut body = Vec::with_capacity(4_096);
        cane.push((Vector3::ZERO, Vector3::UP, 0.96));
        body.push((Vector3::ZERO, Vector3::UP, 0.78));
        let cane_pointer = cane.as_ptr();
        let body_pointer = body.as_ptr();
        let cane_capacity = cane.capacity();
        let body_capacity = body.capacity();
        let (next, preparer) = prepare_hero_visual(
            sample(
                2.0,
                0.0,
                Vector3::new(1.0, 0.0, 0.0),
                true,
                Vector3::ZERO,
                Vector3::ZERO,
            ),
            walking_vm(crate::viewmodel::STEP_EVERY),
            FootstepSuppression::CLEAR,
            cane,
            body,
            CountingPreparer::UNCALLED,
        )
        .unwrap();
        assert!(next.footstep.is_none());
        assert_eq!(preparer, CountingPreparer::UNCALLED);
        assert_eq!(next.cane_vertices.as_ptr(), cane_pointer);
        assert_eq!(next.body_vertices.as_ptr(), body_pointer);
        assert_eq!(next.cane_vertices.capacity(), cane_capacity);
        assert_eq!(next.body_vertices.capacity(), body_capacity);
    }

    fn poison_vector_lane(value: Vector3, lane: usize) -> Vector3 {
        match lane {
            0 => Vector3::new(f32::NAN, value.y, value.z),
            1 => Vector3::new(value.x, f32::NAN, value.z),
            _ => Vector3::new(value.x, value.y, f32::NAN),
        }
    }

    fn assert_late_refusal(mut poison: impl FnMut(&mut HeroVisualCandidate)) {
        let (mut candidate, preparer) = build_candidate(
            sample(
                4.0,
                0.0,
                Vector3::new(1.0, 0.0, 0.0),
                true,
                Vector3::ZERO,
                Vector3::ZERO,
            ),
            walking_vm(0.0),
            FootstepSuppression::CLEAR,
            Vec::with_capacity(2_048),
            Vec::with_capacity(2_048),
            CountingPreparer::UNCALLED,
        )
        .unwrap();
        poison(&mut candidate);
        let cane_pointer = candidate.cane_vertices.as_ptr();
        let body_pointer = candidate.body_vertices.as_ptr();
        let refusal = validate_candidate(candidate, preparer)
            .expect_err("poisoned complete candidate must not be installable");
        let (_reason, cane, body, returned) = refusal.into_recovery();
        assert_eq!(cane.as_ptr(), cane_pointer);
        assert_eq!(body.as_ptr(), body_pointer);
        assert_eq!(returned.calls, 1);
    }

    #[test]
    fn late_candidate_validation_returns_buffers_and_preparer_without_an_installable_value() {
        for field in 0..9 {
            assert_late_refusal(|candidate| {
                let mut capture = candidate.vm.capture();
                match field {
                    0 => capture.walk_amp = f64::NAN,
                    1 => capture.leg_phase = f64::NAN,
                    2 => capture.swing_phase = f64::NAN,
                    3 => capture.cane_swing = f64::NAN,
                    4 => capture.sway_x = f64::NAN,
                    5 => capture.sway_y = f64::NAN,
                    6 => capture.last_yaw = f64::NAN,
                    7 => capture.last_pitch = f64::NAN,
                    _ => capture.step_t = f64::NAN,
                }
                candidate.vm = Viewmodel::restore(capture);
            });
        }
        assert_late_refusal(|candidate| {
            let mut capture = candidate.vm.capture();
            capture.step_side = 0;
            candidate.vm = Viewmodel::restore(capture);
        });
        assert_late_refusal(|candidate| candidate.bob = f64::NAN);
        assert_late_refusal(|candidate| candidate.cane_sweep = f64::NAN);
        for shoe in 0..2 {
            for lane in 0..3 {
                assert_late_refusal(|candidate| {
                    candidate.shoes[shoe] = poison_vector_lane(candidate.shoes[shoe], lane);
                });
            }
        }
        for cane in [true, false] {
            for lane in 0..3 {
                assert_late_refusal(|candidate| {
                    let buffer = if cane {
                        &mut candidate.cane_vertices
                    } else {
                        &mut candidate.body_vertices
                    };
                    buffer[0].0 = poison_vector_lane(buffer[0].0, lane);
                });
                assert_late_refusal(|candidate| {
                    let buffer = if cane {
                        &mut candidate.cane_vertices
                    } else {
                        &mut candidate.body_vertices
                    };
                    buffer[0].1 = poison_vector_lane(buffer[0].1, lane);
                });
            }
            assert_late_refusal(|candidate| {
                let buffer = if cane {
                    &mut candidate.cane_vertices
                } else {
                    &mut candidate.body_vertices
                };
                buffer[0].2 = buffer[0].2.next_down();
            });
        }
        for lane in 0..3 {
            assert_late_refusal(|candidate| {
                let request = candidate.footstep.as_mut().unwrap();
                request.command.at = poison_vector_lane(request.command.at, lane);
            });
            assert_late_refusal(|candidate| {
                let request = candidate.footstep.as_mut().unwrap();
                request.command.normal = poison_vector_lane(request.command.normal, lane);
            });
        }
        assert_late_refusal(|candidate| {
            candidate.footstep.as_mut().unwrap().command.kind = 3;
        });
        assert_late_refusal(|candidate| {
            candidate.footstep.as_mut().unwrap().command.max_r = f64::NAN;
        });
        assert_late_refusal(|candidate| {
            candidate.footstep.as_mut().unwrap().command.speed = f64::NAN;
        });
        assert_late_refusal(|candidate| {
            candidate.footstep.as_mut().unwrap().command.gain = f64::NAN;
        });
        assert_late_refusal(|candidate| {
            candidate.footstep.as_mut().unwrap().command.echoes = 3;
        });
        assert_late_refusal(|candidate| {
            candidate.footstep.as_mut().unwrap().command.gate =
                crate::support_motion::QueuedWaveGate::Always;
        });
        assert_late_refusal(|candidate| {
            candidate.footstep.as_mut().unwrap().now = prepare_time(4.25).unwrap();
        });
    }

    #[allow(dead_code)]
    fn _capture_type_is_complete(_: ViewmodelCapture) {}
}
