use godot::builtin::{Basis, EulerOrder, Transform3D, Vector3};
use std::fmt;

pub const MAX_ACCEL_DT_S: f64 = 1.0 / 15.0;
pub const MAX_ACTOR_COORD_M: f32 = 1_000_000.0;
pub const MAX_POSE_COORD_M: f32 = 1_000_002.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionConfigField {
    FallAcceleration,
    TerminalFallSpeed,
    LandingSilentSpeed,
    LandingFullSpeed,
    LandingMaxGain,
    LandingMaxRange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionConfigError {
    NonFinite {
        field: MotionConfigField,
        value: f64,
    },
    OutOfRange {
        field: MotionConfigField,
        value: f64,
        min: f64,
        max: f64,
    },
    ThresholdOrder {
        silent_speed_mps: f64,
        full_speed_mps: f64,
    },
}

impl MotionConfigError {
    pub fn field(self) -> Option<MotionConfigField> {
        match self {
            Self::NonFinite { field, .. } | Self::OutOfRange { field, .. } => Some(field),
            Self::ThresholdOrder { .. } => None,
        }
    }
}

impl fmt::Display for MotionConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite { field, value } => {
                write!(formatter, "{field:?} must be finite, got {value}")
            }
            Self::OutOfRange {
                field,
                value,
                min,
                max,
            } => write!(
                formatter,
                "{field:?} must be in the inclusive range {min}..={max}, got {value}"
            ),
            Self::ThresholdOrder {
                silent_speed_mps,
                full_speed_mps,
            } => write!(
                formatter,
                "landing full speed {full_speed_mps} m/s must be greater than silent speed {silent_speed_mps} m/s"
            ),
        }
    }
}

impl std::error::Error for MotionConfigError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionValueProblem {
    NonFinite,
    Negative,
    OutOfRange,
    ZeroVector,
    InconsistentState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionValueError {
    field: &'static str,
    problem: MotionValueProblem,
}

impl MotionValueError {
    pub fn non_finite(field: &'static str) -> Self {
        Self {
            field,
            problem: MotionValueProblem::NonFinite,
        }
    }

    pub fn negative(field: &'static str) -> Self {
        Self {
            field,
            problem: MotionValueProblem::Negative,
        }
    }

    pub fn out_of_range(field: &'static str) -> Self {
        Self {
            field,
            problem: MotionValueProblem::OutOfRange,
        }
    }

    pub fn zero_vector(field: &'static str) -> Self {
        Self {
            field,
            problem: MotionValueProblem::ZeroVector,
        }
    }

    pub fn inconsistent_state(field: &'static str) -> Self {
        Self {
            field,
            problem: MotionValueProblem::InconsistentState,
        }
    }

    pub fn field(self) -> &'static str {
        self.field
    }

    pub fn problem(self) -> MotionValueProblem {
        self.problem
    }
}

impl fmt::Display for MotionValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rule = match self.problem {
            MotionValueProblem::NonFinite => "must be finite",
            MotionValueProblem::Negative => "must be non-negative",
            MotionValueProblem::OutOfRange => "is outside its valid range",
            MotionValueProblem::ZeroVector => "must be a nonzero vector",
            MotionValueProblem::InconsistentState => "is inconsistent with the motion state",
        };
        write!(formatter, "{} {rule}", self.field)
    }
}

impl std::error::Error for MotionValueError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionRestoreError {
    Physical(MotionValueError),
    AirbornePlanarMismatch { axis: &'static str },
    AirborneTerminalExceeded,
}

impl fmt::Display for MotionRestoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Physical(error) => write!(formatter, "physical motion value is invalid: {error}"),
            Self::AirbornePlanarMismatch { axis } => write!(
                formatter,
                "airborne physical and retained planar velocity differ on {axis}"
            ),
            Self::AirborneTerminalExceeded => {
                write!(
                    formatter,
                    "airborne vertical velocity exceeds the terminal bound"
                )
            }
        }
    }
}

impl std::error::Error for MotionRestoreError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepDuration(f64);

impl StepDuration {
    pub fn from_raw(raw_seconds: f64) -> Self {
        Self(if raw_seconds.is_finite() && raw_seconds > 0.0 {
            raw_seconds.min(MAX_ACCEL_DT_S)
        } else {
            0.0
        })
    }

    pub fn seconds(self) -> f64 {
        self.0
    }
}

fn validate_vector_lanes(
    value: Vector3,
    fields: [&'static str; 3],
) -> Result<(), MotionValueError> {
    for (lane, field) in [value.x, value.y, value.z].into_iter().zip(fields) {
        if !lane.is_finite() {
            return Err(MotionValueError::non_finite(field));
        }
    }
    Ok(())
}

fn validate_bounded_vector_lanes(
    value: Vector3,
    fields: [&'static str; 3],
    maximum: f32,
) -> Result<(), MotionValueError> {
    validate_vector_lanes(value, fields)?;
    for (lane, field) in [value.x, value.y, value.z].into_iter().zip(fields) {
        if lane.abs() > maximum {
            return Err(MotionValueError::out_of_range(field));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorPosition(Vector3);

impl ActorPosition {
    pub fn try_new(world: Vector3) -> Result<Self, MotionValueError> {
        validate_bounded_vector_lanes(
            world,
            ["actor_position.x", "actor_position.y", "actor_position.z"],
            MAX_ACTOR_COORD_M,
        )?;
        Ok(Self(world))
    }

    pub fn world(self) -> Vector3 {
        self.0
    }

    pub fn planar_distance(self, prior: Self) -> FiniteMeasure {
        let dx = f64::from(self.0.x) - f64::from(prior.0.x);
        let dz = f64::from(self.0.z) - f64::from(prior.0.z);
        FiniteMeasure(dx.hypot(dz))
    }

    pub fn elevation(self) -> SupportElevation {
        SupportElevation(self.0.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PosePoint(Vector3);

impl PosePoint {
    pub fn try_new(world: Vector3) -> Result<Self, MotionValueError> {
        validate_bounded_vector_lanes(
            world,
            ["pose_point.x", "pose_point.y", "pose_point.z"],
            MAX_POSE_COORD_M,
        )?;
        Ok(Self(world))
    }

    pub fn world(self) -> Vector3 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorTransform(Transform3D);

impl ActorTransform {
    pub fn try_new(world: Transform3D) -> Result<Self, MotionValueError> {
        ActorPosition::try_new(world.origin)?;
        let a = world.basis.col_a();
        let b = world.basis.col_b();
        let c = world.basis.col_c();
        for (lane, field) in [
            (a.x, "actor_transform.basis.x.x"),
            (a.y, "actor_transform.basis.x.y"),
            (a.z, "actor_transform.basis.x.z"),
            (b.x, "actor_transform.basis.y.x"),
            (b.y, "actor_transform.basis.y.y"),
            (b.z, "actor_transform.basis.y.z"),
            (c.x, "actor_transform.basis.z.x"),
            (c.y, "actor_transform.basis.z.y"),
            (c.z, "actor_transform.basis.z.z"),
        ] {
            if !lane.is_finite() {
                return Err(MotionValueError::non_finite(field));
            }
        }
        Ok(Self(world))
    }

    pub fn world(self) -> Transform3D {
        self.0
    }

    pub fn position(self) -> ActorPosition {
        ActorPosition(self.0.origin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteRotation(Vector3);

impl FiniteRotation {
    pub fn try_new(euler_radians: Vector3) -> Result<Self, MotionValueError> {
        validate_vector_lanes(euler_radians, ["rotation.x", "rotation.y", "rotation.z"])?;
        Ok(Self(euler_radians))
    }

    pub fn world(self) -> Vector3 {
        self.0
    }

    pub fn yaw(self) -> ActorYaw {
        ActorYaw(f64::from(self.0.y))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GodotRotation(Vector3);

impl GodotRotation {
    pub fn canonicalize(euler_radians: Vector3) -> Result<Self, MotionValueError> {
        let mut current = FiniteRotation::try_new(euler_radians)?.world();
        for _ in 0..16 {
            let mapped =
                Basis::from_euler(EulerOrder::YXZ, current).get_euler_with(EulerOrder::YXZ);
            FiniteRotation::try_new(mapped)?;
            if rotation_lanes_equivalent(current, mapped) {
                return Ok(Self(canonical_wire_rotation(mapped)));
            }
            current = mapped;
        }
        Err(MotionValueError::inconsistent_state("rotation"))
    }

    pub fn try_canonical(euler_radians: Vector3) -> Result<Self, MotionValueError> {
        let canonical = Self::canonicalize(euler_radians)?;
        if rotation_lanes_equal_bits(euler_radians, canonical.world()) {
            Ok(Self(euler_radians))
        } else {
            Err(MotionValueError::inconsistent_state("rotation"))
        }
    }

    pub fn try_replacing_yaw(
        current_full: Vector3,
        captured_yaw: f32,
    ) -> Result<Self, MotionValueError> {
        FiniteRotation::try_new(current_full)?;
        let target = Vector3::new(current_full.x, captured_yaw, current_full.z);
        let canonical = Self::canonicalize(target)?;
        if target.y.to_bits() == canonical.0.y.to_bits()
            && rotation_lane_equivalent(target.x, canonical.0.x)
            && rotation_lane_equivalent(target.z, canonical.0.z)
        {
            Ok(Self(target))
        } else {
            Err(MotionValueError::inconsistent_state("rotation"))
        }
    }

    pub fn canonicalize_replacing_yaw(
        current_full: Vector3,
        captured_yaw: f32,
    ) -> Result<Self, MotionValueError> {
        FiniteRotation::try_new(current_full)?;
        let target = Vector3::new(current_full.x, captured_yaw, current_full.z);
        let canonical = Self::canonicalize(target)?;
        if rotation_lane_equivalent(target.x, canonical.0.x)
            && rotation_lane_equivalent(target.z, canonical.0.z)
        {
            Ok(Self(Vector3::new(
                current_full.x,
                canonical.0.y,
                current_full.z,
            )))
        } else {
            Err(MotionValueError::inconsistent_state("rotation"))
        }
    }

    pub fn try_replacing_pitch(
        current_full: Vector3,
        captured_pitch: f32,
    ) -> Result<Self, MotionValueError> {
        FiniteRotation::try_new(current_full)?;
        let target = Vector3::new(captured_pitch, current_full.y, current_full.z);
        let canonical = Self::canonicalize(target)?;
        if target.x.to_bits() == canonical.0.x.to_bits()
            && rotation_lane_equivalent(target.y, canonical.0.y)
            && rotation_lane_equivalent(target.z, canonical.0.z)
        {
            Ok(Self(target))
        } else {
            Err(MotionValueError::inconsistent_state("rotation"))
        }
    }

    pub fn canonicalize_replacing_pitch(
        current_full: Vector3,
        captured_pitch: f32,
    ) -> Result<Self, MotionValueError> {
        FiniteRotation::try_new(current_full)?;
        let target = Vector3::new(captured_pitch, current_full.y, current_full.z);
        let canonical = Self::canonicalize(target)?;
        if rotation_lane_equivalent(target.y, canonical.0.y)
            && rotation_lane_equivalent(target.z, canonical.0.z)
        {
            Ok(Self(Vector3::new(
                canonical.0.x,
                current_full.y,
                current_full.z,
            )))
        } else {
            Err(MotionValueError::inconsistent_state("rotation"))
        }
    }

    pub fn world(self) -> Vector3 {
        self.0
    }
}

fn canonical_wire_rotation(rotation: Vector3) -> Vector3 {
    Vector3::new(
        if rotation.x == 0.0 { 0.0 } else { rotation.x },
        if rotation.y == 0.0 { 0.0 } else { rotation.y },
        if rotation.z == 0.0 { 0.0 } else { rotation.z },
    )
}

fn rotation_lanes_equal_bits(a: Vector3, b: Vector3) -> bool {
    [a.x, a.y, a.z]
        .into_iter()
        .zip([b.x, b.y, b.z])
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn rotation_lanes_equivalent(a: Vector3, b: Vector3) -> bool {
    [a.x, a.y, a.z]
        .into_iter()
        .zip([b.x, b.y, b.z])
        .all(|(left, right)| left.to_bits() == right.to_bits() || (left == 0.0 && right == 0.0))
}

fn rotation_lane_equivalent(left: f32, right: f32) -> bool {
    left.to_bits() == right.to_bits() || (left == 0.0 && right == 0.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportElevation(f32);

impl SupportElevation {
    pub fn try_new(y: f32) -> Result<Self, MotionValueError> {
        if !y.is_finite() {
            return Err(MotionValueError::non_finite("support_elevation"));
        }
        if y.abs() > MAX_POSE_COORD_M {
            return Err(MotionValueError::out_of_range("support_elevation"));
        }
        Ok(Self(y))
    }

    pub fn y(self) -> f32 {
        self.0
    }

    pub fn delta_from(self, prior: Self) -> f32 {
        (f64::from(self.0) - f64::from(prior.0)) as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorYaw(f64);

impl ActorYaw {
    pub fn try_new(radians: f64) -> Result<Self, MotionValueError> {
        if !radians.is_finite() {
            return Err(MotionValueError::non_finite("actor_yaw"));
        }
        if radians.abs() > f32::MAX as f64 {
            return Err(MotionValueError::out_of_range("actor_yaw"));
        }
        Ok(Self(radians))
    }

    pub fn radians(self) -> f64 {
        self.0
    }

    pub fn godot_lane(self) -> f32 {
        self.0 as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteMeasure(f64);

impl FiniteMeasure {
    pub const ZERO: Self = Self(0.0);

    pub fn try_new(value: f64, field: &'static str) -> Result<Self, MotionValueError> {
        if !value.is_finite() {
            return Err(MotionValueError::non_finite(field));
        }
        if value < 0.0 {
            return Err(MotionValueError::negative(field));
        }
        Ok(Self(value))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorVelocity(Vector3);

impl ActorVelocity {
    pub fn try_new(world_mps: Vector3) -> Result<Self, MotionValueError> {
        validate_vector_lanes(
            world_mps,
            ["actor_velocity.x", "actor_velocity.y", "actor_velocity.z"],
        )?;
        Ok(Self(world_mps))
    }

    pub fn world(self) -> Vector3 {
        self.0
    }

    pub fn planar(self) -> PlanarVelocity {
        PlanarVelocity {
            x_mps: self.0.x,
            z_mps: self.0.z,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanarVelocity {
    x_mps: f32,
    z_mps: f32,
}

impl PlanarVelocity {
    pub const ZERO: Self = Self {
        x_mps: 0.0,
        z_mps: 0.0,
    };

    pub fn try_new(x_mps: f32, z_mps: f32) -> Result<Self, MotionValueError> {
        if !x_mps.is_finite() {
            return Err(MotionValueError::non_finite("planar_velocity.x"));
        }
        if !z_mps.is_finite() {
            return Err(MotionValueError::non_finite("planar_velocity.z"));
        }
        Ok(Self { x_mps, z_mps })
    }

    pub fn try_from_world(raw: Vector3) -> Result<Self, MotionValueError> {
        ActorVelocity::try_new(raw).map(ActorVelocity::planar)
    }

    pub fn x_mps(self) -> f32 {
        self.x_mps
    }

    pub fn z_mps(self) -> f32 {
        self.z_mps
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteVelocity(f32);

impl FiniteVelocity {
    const ZERO: Self = Self(0.0);

    pub fn try_new(mps: f32) -> Result<Self, MotionValueError> {
        if !mps.is_finite() {
            return Err(MotionValueError::non_finite("vertical_velocity_mps"));
        }
        Ok(Self(mps))
    }

    // Callers prove finiteness through bounded duration, configuration, and
    // previously validated finite operands before reaching this constructor.
    fn from_finite(mps: f32) -> Self {
        Self(mps)
    }

    pub fn mps(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteSpeed(f32);

impl FiniteSpeed {
    pub fn try_new(mps: f32) -> Result<Self, MotionValueError> {
        if !mps.is_finite() {
            return Err(MotionValueError::non_finite("impact_speed_mps"));
        }
        if mps < 0.0 {
            return Err(MotionValueError::negative("impact_speed_mps"));
        }
        Ok(Self(mps))
    }

    // Reconciliation supplies the absolute value of its validated finite
    // downward command, so this retained speed is finite and non-negative.
    fn from_finite_nonnegative(mps: f32) -> Self {
        Self(mps)
    }

    pub fn mps(self) -> f32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportContact {
    point: Vector3,
    normal: Vector3,
}

impl SupportContact {
    pub fn try_new(point: Vector3, normal: Vector3) -> Result<Self, MotionValueError> {
        validate_vector_lanes(
            point,
            ["support.point.x", "support.point.y", "support.point.z"],
        )?;
        validate_vector_lanes(
            normal,
            ["support.normal.x", "support.normal.y", "support.normal.z"],
        )?;
        if normal.x == 0.0 && normal.y == 0.0 && normal.z == 0.0 {
            return Err(MotionValueError::zero_vector("support.normal"));
        }
        Ok(Self { point, normal })
    }

    pub fn point(self) -> Vector3 {
        self.point
    }

    pub fn normal(self) -> Vector3 {
        self.normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LandingEvent {
    impact_speed: FiniteSpeed,
    support: SupportContact,
}

impl LandingEvent {
    pub fn try_new(
        impact_speed_mps: f32,
        support: SupportContact,
    ) -> Result<Self, MotionValueError> {
        Ok(Self {
            impact_speed: FiniteSpeed::try_new(impact_speed_mps)?,
            support,
        })
    }

    pub fn impact_speed(self) -> FiniteSpeed {
        self.impact_speed
    }

    pub fn support(self) -> SupportContact {
        self.support
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionPhase {
    Controlled,
    Airborne {
        planar_velocity_mps: PlanarVelocity,
        vertical_velocity_mps: FiniteVelocity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionState {
    phase: MotionPhase,
    support: Option<SupportContact>,
    last_landing: Option<LandingEvent>,
}

impl MotionState {
    pub fn initial() -> Self {
        Self {
            phase: MotionPhase::Controlled,
            support: None,
            last_landing: None,
        }
    }

    pub fn restore(
        phase: MotionPhase,
        support: Option<SupportContact>,
        last_landing: Option<LandingEvent>,
    ) -> Result<Self, MotionValueError> {
        if let MotionPhase::Airborne {
            vertical_velocity_mps,
            ..
        } = phase
        {
            if vertical_velocity_mps.mps() > 0.0 {
                return Err(MotionValueError::inconsistent_state(
                    "motion_phase.vertical_velocity_mps",
                ));
            }
            if support.is_some() {
                return Err(MotionValueError::inconsistent_state("motion_state.support"));
            }
        }
        Ok(Self {
            phase,
            support,
            last_landing,
        })
    }

    pub fn relocated(self) -> Self {
        Self {
            phase: MotionPhase::Controlled,
            support: None,
            last_landing: self.last_landing,
        }
    }

    pub fn phase(self) -> MotionPhase {
        self.phase
    }

    pub fn support(self) -> Option<SupportContact> {
        self.support
    }

    pub fn last_landing(self) -> Option<LandingEvent> {
        self.last_landing
    }

    pub fn accepts_control(self) -> bool {
        matches!(self.phase, MotionPhase::Controlled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportMotionConfig {
    fall_acceleration_mps2: f64,
    terminal_fall_speed_mps: f64,
    terminal_fall_speed_lane_mps: f32,
    landing_silent_speed_mps: f64,
    landing_full_speed_mps: f64,
    landing_max_gain: f64,
    landing_max_range_m: f64,
}

fn validate_config_field(
    field: MotionConfigField,
    value: f64,
    min: f64,
    max: f64,
) -> Result<(), MotionConfigError> {
    if !value.is_finite() {
        return Err(MotionConfigError::NonFinite { field, value });
    }
    if !(min..=max).contains(&value) {
        return Err(MotionConfigError::OutOfRange {
            field,
            value,
            min,
            max,
        });
    }
    Ok(())
}

impl SupportMotionConfig {
    pub const PLAYER_DEFAULT: Self = Self::from_validated_constants(9.8, 20.0, 1.5, 4.0, 0.85, 5.0);
    pub const CAT_DEFAULT: Self = Self::from_validated_constants(9.8, 20.0, 1.5, 4.0, 0.60, 2.5);

    const fn from_validated_constants(
        fall_acceleration_mps2: f64,
        terminal_fall_speed_mps: f64,
        landing_silent_speed_mps: f64,
        landing_full_speed_mps: f64,
        landing_max_gain: f64,
        landing_max_range_m: f64,
    ) -> Self {
        Self {
            fall_acceleration_mps2,
            terminal_fall_speed_mps,
            terminal_fall_speed_lane_mps: terminal_fall_speed_mps as f32,
            landing_silent_speed_mps,
            landing_full_speed_mps,
            landing_max_gain,
            landing_max_range_m,
        }
    }

    pub fn try_new(
        fall_acceleration_mps2: f64,
        terminal_fall_speed_mps: f64,
        landing_silent_speed_mps: f64,
        landing_full_speed_mps: f64,
        landing_max_gain: f64,
        landing_max_range_m: f64,
    ) -> Result<Self, MotionConfigError> {
        validate_config_field(
            MotionConfigField::FallAcceleration,
            fall_acceleration_mps2,
            0.1,
            30.0,
        )?;
        validate_config_field(
            MotionConfigField::TerminalFallSpeed,
            terminal_fall_speed_mps,
            0.5,
            50.0,
        )?;
        validate_config_field(
            MotionConfigField::LandingSilentSpeed,
            landing_silent_speed_mps,
            0.0,
            10.0,
        )?;
        validate_config_field(
            MotionConfigField::LandingFullSpeed,
            landing_full_speed_mps,
            0.1,
            20.0,
        )?;
        validate_config_field(
            MotionConfigField::LandingMaxGain,
            landing_max_gain,
            0.0,
            1.0,
        )?;
        validate_config_field(
            MotionConfigField::LandingMaxRange,
            landing_max_range_m,
            0.0,
            10.0,
        )?;
        if landing_full_speed_mps <= landing_silent_speed_mps {
            return Err(MotionConfigError::ThresholdOrder {
                silent_speed_mps: landing_silent_speed_mps,
                full_speed_mps: landing_full_speed_mps,
            });
        }
        Ok(Self {
            fall_acceleration_mps2,
            terminal_fall_speed_mps,
            terminal_fall_speed_lane_mps: terminal_fall_speed_mps as f32,
            landing_silent_speed_mps,
            landing_full_speed_mps,
            landing_max_gain,
            landing_max_range_m,
        })
    }

    pub fn fall_acceleration_mps2(self) -> f64 {
        self.fall_acceleration_mps2
    }

    pub fn terminal_fall_speed_mps(self) -> f64 {
        self.terminal_fall_speed_mps
    }

    pub fn terminal_fall_speed_lane_mps(self) -> f32 {
        self.terminal_fall_speed_lane_mps
    }

    pub fn landing_silent_speed_mps(self) -> f64 {
        self.landing_silent_speed_mps
    }

    pub fn landing_full_speed_mps(self) -> f64 {
        self.landing_full_speed_mps
    }

    pub fn landing_max_gain(self) -> f64 {
        self.landing_max_gain
    }

    pub fn landing_max_range_m(self) -> f64 {
        self.landing_max_range_m
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionCommand {
    world_velocity_mps: Vector3,
}

impl MotionCommand {
    fn from_finite_parts(planar: PlanarVelocity, vertical: FiniteVelocity) -> Self {
        Self {
            world_velocity_mps: Vector3::new(planar.x_mps(), vertical.mps(), planar.z_mps()),
        }
    }

    pub fn world_velocity(self) -> Vector3 {
        self.world_velocity_mps
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedMotion {
    prior: MotionState,
    command: MotionCommand,
}

impl PreparedMotion {
    pub fn command(self) -> MotionCommand {
        self.command
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionOutcome {
    actual_planar_velocity_mps: PlanarVelocity,
    accepted_support: Option<SupportContact>,
}

impl MotionOutcome {
    pub fn new(
        actual_velocity_mps: ActorVelocity,
        accepted_support: Option<SupportContact>,
    ) -> Self {
        Self {
            actual_planar_velocity_mps: actual_velocity_mps.planar(),
            accepted_support,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionTransition {
    pub state: MotionState,
    pub landing: Option<LandingEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LandingVoice {
    gain: f64,
    range_m: f64,
}

impl LandingVoice {
    pub fn gain(self) -> f64 {
        self.gain
    }

    pub fn range_m(self) -> f64 {
        self.range_m
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuedWaveGate {
    Always,
    ControlledContact,
}

impl QueuedWaveGate {
    pub fn wire_name(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::ControlledContact => "controlled_contact",
        }
    }

    pub fn allows(
        self,
        before: MotionPhase,
        after: MotionPhase,
        landing: Option<LandingEvent>,
    ) -> bool {
        match self {
            Self::Always => true,
            Self::ControlledContact => {
                matches!(before, MotionPhase::Controlled)
                    && matches!(after, MotionPhase::Controlled)
                    && landing.is_none()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FootstepSuppression {
    pending: bool,
}

impl FootstepSuppression {
    pub const CLEAR: Self = Self { pending: false };

    pub fn restore(pending: bool) -> Self {
        Self { pending }
    }

    pub fn pending(self) -> bool {
        self.pending
    }

    pub fn on_transition(self, landing: Option<LandingEvent>) -> Self {
        Self {
            pending: self.pending || landing.is_some(),
        }
    }

    pub fn acknowledge(self) -> (Self, bool) {
        (Self::CLEAR, self.pending)
    }
}

pub fn prepare(
    state: MotionState,
    desired_planar: PlanarVelocity,
    duration: StepDuration,
    config: SupportMotionConfig,
) -> PreparedMotion {
    let command = match state.phase() {
        MotionPhase::Controlled => {
            MotionCommand::from_finite_parts(desired_planar, FiniteVelocity::ZERO)
        }
        MotionPhase::Airborne {
            planar_velocity_mps,
            vertical_velocity_mps,
        } => {
            let next_y = (f64::from(vertical_velocity_mps.mps())
                - config.fall_acceleration_mps2() * duration.seconds())
            .max(-f64::from(config.terminal_fall_speed_lane_mps())) as f32;
            MotionCommand::from_finite_parts(
                planar_velocity_mps,
                FiniteVelocity::from_finite(next_y),
            )
        }
    };
    PreparedMotion {
        prior: state,
        command,
    }
}

pub fn reconcile(prepared: PreparedMotion, outcome: MotionOutcome) -> MotionTransition {
    let prior_landing = prepared.prior.last_landing;
    match (prepared.prior.phase, outcome.accepted_support) {
        (MotionPhase::Controlled, Some(support)) => MotionTransition {
            state: MotionState {
                phase: MotionPhase::Controlled,
                support: Some(support),
                last_landing: prior_landing,
            },
            landing: None,
        },
        (MotionPhase::Controlled, None) => MotionTransition {
            state: MotionState {
                phase: MotionPhase::Airborne {
                    planar_velocity_mps: outcome.actual_planar_velocity_mps,
                    vertical_velocity_mps: FiniteVelocity::from_finite(-0.0),
                },
                support: None,
                last_landing: prior_landing,
            },
            landing: None,
        },
        (MotionPhase::Airborne { .. }, None) => MotionTransition {
            state: MotionState {
                phase: MotionPhase::Airborne {
                    planar_velocity_mps: outcome.actual_planar_velocity_mps,
                    vertical_velocity_mps: FiniteVelocity::from_finite(
                        prepared.command.world_velocity_mps.y,
                    ),
                },
                support: None,
                last_landing: prior_landing,
            },
            landing: None,
        },
        (MotionPhase::Airborne { .. }, Some(support)) => {
            let event = LandingEvent {
                impact_speed: FiniteSpeed::from_finite_nonnegative(
                    prepared.command.world_velocity_mps.y.abs(),
                ),
                support,
            };
            MotionTransition {
                state: MotionState {
                    phase: MotionPhase::Controlled,
                    support: Some(support),
                    last_landing: Some(event),
                },
                landing: Some(event),
            }
        }
    }
}

pub fn landing_voice(event: LandingEvent, config: SupportMotionConfig) -> Option<LandingVoice> {
    let speed = f64::from(event.impact_speed.mps());
    if speed <= config.landing_silent_speed_mps() {
        return None;
    }
    let severity = ((speed - config.landing_silent_speed_mps())
        / (config.landing_full_speed_mps() - config.landing_silent_speed_mps()))
    .min(1.0);
    let gain = severity * config.landing_max_gain();
    let range_m = severity * config.landing_max_range_m();
    if gain == 0.0 || range_m == 0.0 {
        None
    } else {
        Some(LandingVoice { gain, range_m })
    }
}

pub fn validate_restore(
    state: MotionState,
    physical_velocity_mps: ActorVelocity,
    config: SupportMotionConfig,
) -> Result<(), MotionRestoreError> {
    let MotionPhase::Airborne {
        planar_velocity_mps,
        vertical_velocity_mps,
    } = state.phase
    else {
        return Ok(());
    };
    let physical = physical_velocity_mps.planar();
    if planar_velocity_mps.x_mps().to_bits() != physical.x_mps().to_bits() {
        return Err(MotionRestoreError::AirbornePlanarMismatch { axis: "x" });
    }
    if planar_velocity_mps.z_mps().to_bits() != physical.z_mps().to_bits() {
        return Err(MotionRestoreError::AirbornePlanarMismatch { axis: "z" });
    }
    if vertical_velocity_mps.mps() < -config.terminal_fall_speed_lane_mps()
        || vertical_velocity_mps.mps() > 0.0
    {
        return Err(MotionRestoreError::AirborneTerminalExceeded);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use godot::builtin::{Basis, Transform3D, Vector3};

    fn support() -> SupportContact {
        SupportContact::try_new(Vector3::new(4.0, 1.0, -2.0), Vector3::UP).unwrap()
    }

    fn config_with(values: [f64; 6]) -> Result<SupportMotionConfig, MotionConfigError> {
        SupportMotionConfig::try_new(
            values[0], values[1], values[2], values[3], values[4], values[5],
        )
    }

    fn transform_lanes(transform: Transform3D) -> [u32; 12] {
        let a = transform.basis.col_a();
        let b = transform.basis.col_b();
        let c = transform.basis.col_c();
        [
            a.x.to_bits(),
            a.y.to_bits(),
            a.z.to_bits(),
            b.x.to_bits(),
            b.y.to_bits(),
            b.z.to_bits(),
            c.x.to_bits(),
            c.y.to_bits(),
            c.z.to_bits(),
            transform.origin.x.to_bits(),
            transform.origin.y.to_bits(),
            transform.origin.z.to_bits(),
        ]
    }

    fn transform_with_lane(mut lanes: [f32; 12], index: usize, value: f32) -> Transform3D {
        lanes[index] = value;
        Transform3D::new(
            Basis::from_cols(
                Vector3::new(lanes[0], lanes[1], lanes[2]),
                Vector3::new(lanes[3], lanes[4], lanes[5]),
                Vector3::new(lanes[6], lanes[7], lanes[8]),
            ),
            Vector3::new(lanes[9], lanes[10], lanes[11]),
        )
    }

    #[test]
    fn malformed_durations_are_zero_and_large_steps_are_capped() {
        for raw in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                StepDuration::from_raw(raw).seconds().to_bits(),
                0.0_f64.to_bits()
            );
        }
        assert_eq!(StepDuration::from_raw(0.5).seconds(), 1.0 / 15.0);
    }

    #[test]
    fn airborne_acceleration_is_bounded_by_dt_and_terminal_speed() {
        let config = SupportMotionConfig::PLAYER_DEFAULT;
        let state = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::ZERO,
                vertical_velocity_mps: FiniteVelocity::try_new(0.0).unwrap(),
            },
            None,
            None,
        )
        .unwrap();
        let first = prepare(
            state,
            PlanarVelocity::ZERO,
            StepDuration::from_raw(1.0 / 60.0),
            config,
        );
        assert_eq!(first.command().world_velocity().y, -0.163_333_33_f32);
        let near_terminal = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::ZERO,
                vertical_velocity_mps: FiniteVelocity::try_new(-19.9).unwrap(),
            },
            None,
            None,
        )
        .unwrap();
        let capped = prepare(
            near_terminal,
            PlanarVelocity::ZERO,
            StepDuration::from_raw(0.5),
            config,
        );
        assert_eq!(capped.command().world_velocity().y, -20.0);
    }

    #[test]
    fn landing_voice_is_silent_linear_and_capped() {
        let support = SupportContact::try_new(Vector3::ZERO, Vector3::UP).unwrap();
        assert!(
            landing_voice(
                LandingEvent::try_new(1.5, support).unwrap(),
                SupportMotionConfig::PLAYER_DEFAULT
            )
            .is_none()
        );
        let half = landing_voice(
            LandingEvent::try_new(2.75, support).unwrap(),
            SupportMotionConfig::PLAYER_DEFAULT,
        )
        .unwrap();
        assert_eq!(half.gain(), 0.425);
        assert_eq!(half.range_m(), 2.5);
        let full = landing_voice(
            LandingEvent::try_new(9.0, support).unwrap(),
            SupportMotionConfig::PLAYER_DEFAULT,
        )
        .unwrap();
        assert_eq!((full.gain(), full.range_m()), (0.85, 5.0));
    }

    #[test]
    fn defaults_are_valid_and_raw_config_obeys_the_authored_ranges() {
        let player = SupportMotionConfig::PLAYER_DEFAULT;
        assert_eq!(
            (
                player.fall_acceleration_mps2(),
                player.terminal_fall_speed_mps(),
                player.landing_silent_speed_mps(),
                player.landing_full_speed_mps(),
                player.landing_max_gain(),
                player.landing_max_range_m(),
            ),
            (9.8, 20.0, 1.5, 4.0, 0.85, 5.0)
        );
        let cat = SupportMotionConfig::CAT_DEFAULT;
        assert_eq!(
            (
                cat.fall_acceleration_mps2(),
                cat.terminal_fall_speed_mps(),
                cat.landing_silent_speed_mps(),
                cat.landing_full_speed_mps(),
                cat.landing_max_gain(),
                cat.landing_max_range_m(),
            ),
            (9.8, 20.0, 1.5, 4.0, 0.60, 2.5)
        );

        for values in [
            [0.1, 20.0, 1.5, 4.0, 0.85, 5.0],
            [30.0, 20.0, 1.5, 4.0, 0.85, 5.0],
            [9.8, 0.5, 0.0, 4.0, 0.85, 5.0],
            [9.8, 50.0, 1.5, 4.0, 0.85, 5.0],
            [9.8, 20.0, 0.0, 4.0, 0.85, 5.0],
            [9.8, 20.0, 10.0, 20.0, 0.85, 5.0],
            [9.8, 20.0, 0.0, 0.1, 0.85, 5.0],
            [9.8, 20.0, 1.5, 20.0, 0.85, 5.0],
            [9.8, 20.0, 1.5, 4.0, 0.0, 5.0],
            [9.8, 20.0, 1.5, 4.0, 1.0, 5.0],
            [9.8, 20.0, 1.5, 4.0, 0.85, 0.0],
            [9.8, 20.0, 1.5, 4.0, 0.85, 10.0],
        ] {
            assert!(config_with(values).is_ok(), "endpoint {values:?}");
        }

        for (values, field) in [
            (
                [0.09, 20.0, 1.5, 4.0, 0.85, 5.0],
                MotionConfigField::FallAcceleration,
            ),
            (
                [30.01, 20.0, 1.5, 4.0, 0.85, 5.0],
                MotionConfigField::FallAcceleration,
            ),
            (
                [9.8, 0.49, 0.0, 4.0, 0.85, 5.0],
                MotionConfigField::TerminalFallSpeed,
            ),
            (
                [9.8, 50.01, 1.5, 4.0, 0.85, 5.0],
                MotionConfigField::TerminalFallSpeed,
            ),
            (
                [9.8, 20.0, -0.01, 4.0, 0.85, 5.0],
                MotionConfigField::LandingSilentSpeed,
            ),
            (
                [9.8, 20.0, 10.01, 20.0, 0.85, 5.0],
                MotionConfigField::LandingSilentSpeed,
            ),
            (
                [9.8, 20.0, 0.0, 0.09, 0.85, 5.0],
                MotionConfigField::LandingFullSpeed,
            ),
            (
                [9.8, 20.0, 1.5, 20.01, 0.85, 5.0],
                MotionConfigField::LandingFullSpeed,
            ),
            (
                [9.8, 20.0, 1.5, 4.0, -0.01, 5.0],
                MotionConfigField::LandingMaxGain,
            ),
            (
                [9.8, 20.0, 1.5, 4.0, 1.01, 5.0],
                MotionConfigField::LandingMaxGain,
            ),
            (
                [9.8, 20.0, 1.5, 4.0, 0.85, -0.01],
                MotionConfigField::LandingMaxRange,
            ),
            (
                [9.8, 20.0, 1.5, 4.0, 0.85, 10.01],
                MotionConfigField::LandingMaxRange,
            ),
        ] {
            let error = config_with(values).unwrap_err();
            assert_eq!(error.field(), Some(field));
            assert!(
                matches!(error, MotionConfigError::OutOfRange { field: got, .. } if got == field)
            );
        }
    }

    #[test]
    fn every_config_field_reports_its_exact_error_variant_and_display() {
        for (index, field) in [
            MotionConfigField::FallAcceleration,
            MotionConfigField::TerminalFallSpeed,
            MotionConfigField::LandingSilentSpeed,
            MotionConfigField::LandingFullSpeed,
            MotionConfigField::LandingMaxGain,
            MotionConfigField::LandingMaxRange,
        ]
        .into_iter()
        .enumerate()
        {
            let mut values = [9.8, 20.0, 1.5, 4.0, 0.85, 5.0];
            values[index] = f64::NAN;
            let error = config_with(values).unwrap_err();
            assert!(
                matches!(error, MotionConfigError::NonFinite { field: got, value } if got == field && value.is_nan())
            );
            assert_eq!(error.field(), Some(field));
            let display = error.to_string();
            assert!(display.contains(&format!("{field:?}")));
            assert!(display.contains("finite"));
            let source: &dyn std::error::Error = &error;
            assert!(source.source().is_none());
        }

        let out = config_with([0.09, 20.0, 1.5, 4.0, 0.85, 5.0]).unwrap_err();
        assert_eq!(
            out,
            MotionConfigError::OutOfRange {
                field: MotionConfigField::FallAcceleration,
                value: 0.09,
                min: 0.1,
                max: 30.0,
            }
        );
        assert!(out.to_string().contains("range"));
    }

    #[test]
    fn threshold_order_reports_both_supplied_values() {
        let error = config_with([9.8, 20.0, 4.0, 3.0, 0.85, 5.0]).unwrap_err();
        assert_eq!(
            error,
            MotionConfigError::ThresholdOrder {
                silent_speed_mps: 4.0,
                full_speed_mps: 3.0,
            }
        );
        assert_eq!(error.field(), None);
        assert_eq!(
            error.to_string(),
            "landing full speed 3 m/s must be greater than silent speed 4 m/s"
        );
    }

    #[test]
    fn actor_position_rejects_each_poisoned_or_out_of_envelope_lane() {
        let outside = f32::from_bits(1_000_000.0_f32.to_bits() + 1);
        for lane in 0..3 {
            for (value, problem) in [
                (f32::NAN, MotionValueProblem::NonFinite),
                (f32::INFINITY, MotionValueProblem::NonFinite),
                (f32::NEG_INFINITY, MotionValueProblem::NonFinite),
                (outside, MotionValueProblem::OutOfRange),
            ] {
                let mut values = [0.0, 0.0, 0.0];
                values[lane] = value;
                let error = ActorPosition::try_new(Vector3::new(values[0], values[1], values[2]))
                    .unwrap_err();
                assert_eq!(error.problem(), problem);
                assert_eq!(
                    error.field(),
                    ["actor_position.x", "actor_position.y", "actor_position.z"][lane]
                );
                assert!(error.to_string().contains(error.field()));
            }
        }
        for value in [-1_000_000.0_f32, 1_000_000.0_f32] {
            let world = Vector3::new(value, -value, value);
            let accepted = ActorPosition::try_new(world).unwrap().world();
            assert_eq!(accepted.x.to_bits(), world.x.to_bits());
            assert_eq!(accepted.y.to_bits(), world.y.to_bits());
            assert_eq!(accepted.z.to_bits(), world.z.to_bits());
        }
    }

    #[test]
    fn actor_transform_rejects_each_poisoned_origin_or_basis_lane_and_preserves_valid_bits() {
        let lanes = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, -10.0, 11.0, -12.0,
        ];
        for lane in 0..12 {
            for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let error =
                    ActorTransform::try_new(transform_with_lane(lanes, lane, poison)).unwrap_err();
                assert_eq!(error.problem(), MotionValueProblem::NonFinite);
            }
        }
        let valid = transform_with_lane(lanes, 0, lanes[0]);
        assert_eq!(
            transform_lanes(ActorTransform::try_new(valid).unwrap().world()),
            transform_lanes(valid)
        );
        assert_eq!(
            ActorTransform::try_new(valid).unwrap().position().world(),
            valid.origin
        );
    }

    #[test]
    fn finite_rotation_rejects_each_poisoned_lane() {
        for lane in 0..3 {
            for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut values = [0.0, 0.0, 0.0];
                values[lane] = poison;
                let error = FiniteRotation::try_new(Vector3::new(values[0], values[1], values[2]))
                    .unwrap_err();
                assert_eq!(error.problem(), MotionValueProblem::NonFinite);
            }
        }
        let valid = Vector3::new(-f32::MAX, -0.0, f32::MAX);
        let rotation = FiniteRotation::try_new(valid).unwrap();
        assert_eq!(rotation.world().x.to_bits(), valid.x.to_bits());
        assert_eq!(rotation.world().y.to_bits(), valid.y.to_bits());
        assert_eq!(rotation.world().z.to_bits(), valid.z.to_bits());
        assert_eq!(rotation.yaw().radians().to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn godot_rotation_refuses_a_noncanonical_exact_f32_yaw() {
        let requested = Vector3::new(0.0, 4.0, 0.0);
        let error = GodotRotation::try_canonical(requested)
            .expect_err("Godot rewrites this exact-f32 Euler angle");
        assert_eq!(error.field(), "rotation");

        let basis_image =
            godot::builtin::Basis::from_euler(godot::builtin::EulerOrder::YXZ, requested)
                .get_euler_with(godot::builtin::EulerOrder::YXZ);
        let canonical = GodotRotation::canonicalize(basis_image)
            .expect("the Basis/Euler image must have a stable wire spelling")
            .world();
        let checked = GodotRotation::try_canonical(canonical)
            .expect("the stable Basis/Euler serialization must be canonical");
        for (actual, expected) in [checked.world().x, checked.world().y, checked.world().z]
            .into_iter()
            .zip([canonical.x, canonical.y, canonical.z])
        {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
    }

    #[test]
    fn godot_rotation_accepts_the_zero_rotation_a_new_node_produces() {
        let canonical =
            godot::builtin::Basis::from_euler(godot::builtin::EulerOrder::YXZ, Vector3::ZERO)
                .get_euler_with(godot::builtin::EulerOrder::YXZ);
        assert_eq!(canonical.x.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(Vector3::ZERO.x.to_bits(), 0.0_f32.to_bits());
        for spelling in [Vector3::ZERO, Vector3::new(-0.0, -0.0, -0.0)] {
            let serialized = GodotRotation::canonicalize(spelling)
                .expect("both engine zero spellings must have one stable serialization");
            for lane in [
                serialized.world().x,
                serialized.world().y,
                serialized.world().z,
            ] {
                assert_eq!(lane.to_bits(), 0.0_f32.to_bits());
            }
        }
        GodotRotation::try_canonical(Vector3::ZERO)
            .expect("the stable positive-zero artifact must be restorable");
        GodotRotation::try_canonical(Vector3::new(-0.0, 0.0, 0.0))
            .expect_err("a hand-edited negative-zero artifact is not canonical wire state");
    }

    #[test]
    fn godot_rotation_canonicalization_is_idempotent_for_observed_pitch_and_wrapped_yaw() {
        for (requested, changed_axis, changes) in [
            (Vector3::new(-0.2, 0.0, 0.0), 0, false),
            (Vector3::new(0.0, 4.0, 0.0), 1, true),
        ] {
            let canonical = GodotRotation::canonicalize(requested)
                .expect("finite observed rotations must reach a fixed point");
            let canonical_world = canonical.world();
            let requested_lane = [requested.x, requested.y, requested.z][changed_axis];
            let canonical_lane =
                [canonical_world.x, canonical_world.y, canonical_world.z][changed_axis];
            if changes {
                assert_ne!(canonical_lane.to_bits(), requested_lane.to_bits());
                assert!(GodotRotation::try_canonical(requested).is_err());
            } else {
                assert_eq!(canonical_lane.to_bits(), requested_lane.to_bits());
                GodotRotation::try_canonical(requested)
                    .expect("the observed pitch is already a fixed point");
            }

            let admitted = GodotRotation::try_canonical(canonical_world)
                .expect("the fixed point must be accepted bit-exactly");
            let repeated = GodotRotation::canonicalize(admitted.world())
                .expect("canonicalization must remain total at its own output");
            for (once, twice) in [canonical_world.x, canonical_world.y, canonical_world.z]
                .into_iter()
                .zip([repeated.world().x, repeated.world().y, repeated.world().z])
            {
                assert_eq!(once.to_bits(), twice.to_bits());
            }
        }
    }

    #[test]
    fn godot_rotation_lane_replacement_preserves_uncaptured_bits_and_checks_the_complete_yxz_target()
     {
        fn assert_bits(actual: Vector3, expected: Vector3) {
            for (actual, expected) in [actual.x, actual.y, actual.z]
                .into_iter()
                .zip([expected.x, expected.y, expected.z])
            {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }

        let complete = Vector3::new(
            f32::from_bits(0x3e80_0000),
            f32::from_bits(0xbf00_0001),
            f32::from_bits(0x3e00_0000),
        );
        assert_bits(
            GodotRotation::canonicalize(Vector3::new(0.25, -0.5, 0.125))
                .expect("the known YXZ image must converge")
                .world(),
            complete,
        );

        let yaw_current = Vector3::new(complete.x, 0.0, complete.z);
        let yaw = GodotRotation::try_replacing_yaw(yaw_current, complete.y)
            .expect("yaw replacement must retain X/Z bits");
        assert_bits(yaw.world(), complete);

        let pitch_current = Vector3::new(0.0, complete.y, complete.z);
        let pitch = GodotRotation::try_replacing_pitch(pitch_current, complete.x)
            .expect("pitch replacement must retain Y/Z bits");
        assert_bits(pitch.world(), complete);
    }

    #[test]
    fn godot_rotation_lane_replacement_refuses_an_untouched_noncanonical_lane() {
        let complete = Vector3::new(
            f32::from_bits(0x3e80_0000),
            f32::from_bits(0xbf00_0001),
            f32::from_bits(0x3e00_0000),
        );
        GodotRotation::try_replacing_yaw(Vector3::new(4.0, 0.0, complete.z), complete.y)
            .expect_err("an untouched noncanonical X lane must be refused");
        GodotRotation::try_replacing_pitch(Vector3::new(0.0, 4.0, complete.z), complete.x)
            .expect_err("an untouched noncanonical Y lane must be refused");
        GodotRotation::try_replacing_yaw(Vector3::new(0.0, 0.0, 4.0), 0.0)
            .expect_err("an isolated noncanonical roll must not hide behind a canonical yaw");
        GodotRotation::try_replacing_pitch(Vector3::new(0.0, 4.0, 0.0), 0.0)
            .expect_err("an isolated noncanonical yaw must not hide behind a canonical pitch");
    }

    #[test]
    fn godot_rotation_canonicalizing_lane_replacement_allows_owned_ulp_only() {
        let expected = Vector3::new(
            f32::from_bits(0x3e80_0000),
            f32::from_bits(0xbf00_0001),
            f32::from_bits(0x3e00_0000),
        );
        let yaw_current = Vector3::new(expected.x, 0.0, expected.z);
        let yaw = GodotRotation::canonicalize_replacing_yaw(yaw_current, -0.5)
            .expect("the owned yaw lane may move to its canonical f32 image")
            .world();
        assert_eq!(yaw.x.to_bits(), yaw_current.x.to_bits());
        assert_eq!(yaw.z.to_bits(), yaw_current.z.to_bits());
        assert_eq!(yaw.y.to_bits(), expected.y.to_bits());
        assert_ne!(yaw.y.to_bits(), (-0.5_f32).to_bits());

        let pitch_current = Vector3::new(0.0, expected.y, expected.z);
        let pitch = GodotRotation::canonicalize_replacing_pitch(pitch_current, expected.x)
            .expect("canonical pitch replacement must preserve Y/Z")
            .world();
        for (actual, expected) in [pitch.x, pitch.y, pitch.z]
            .into_iter()
            .zip([expected.x, expected.y, expected.z])
        {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }

        let decoy_yaw = GodotRotation::canonicalize_replacing_yaw(expected, 0.5)
            .expect("the external restore decoy must preserve X/Z")
            .world();
        assert_ne!(decoy_yaw.y.to_bits(), expected.y.to_bits());
        let decoy_pitch = GodotRotation::canonicalize_replacing_pitch(expected, -0.25)
            .expect("the external restore decoy must preserve Y/Z")
            .world();
        assert_ne!(decoy_pitch.x.to_bits(), expected.x.to_bits());
    }

    #[test]
    fn godot_rotation_lane_replacement_preserves_omitted_signed_zero_bits() {
        let current = Vector3::new(-0.0, 0.25, -0.0);
        let canonicalized = GodotRotation::canonicalize_replacing_yaw(current, -0.0)
            .expect("omitted zero signs are live configuration")
            .world();
        assert_eq!(canonicalized.x.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(canonicalized.y.to_bits(), 0.0_f32.to_bits());
        assert_eq!(canonicalized.z.to_bits(), (-0.0_f32).to_bits());

        let installed = GodotRotation::try_replacing_yaw(current, 0.0)
            .expect("a canonical owned +0 must retain omitted zero signs")
            .world();
        assert_eq!(installed.x.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(installed.y.to_bits(), 0.0_f32.to_bits());
        assert_eq!(installed.z.to_bits(), (-0.0_f32).to_bits());
        GodotRotation::try_replacing_yaw(current, -0.0)
            .expect_err("the owned artifact lane still requires canonical +0");
    }

    #[test]
    fn actor_yaw_and_measure_reject_poison_and_unrepresentable_or_negative_values() {
        for poison in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                ActorYaw::try_new(poison).unwrap_err().problem(),
                MotionValueProblem::NonFinite
            );
            assert_eq!(
                FiniteMeasure::try_new(poison, "distance")
                    .unwrap_err()
                    .problem(),
                MotionValueProblem::NonFinite
            );
        }
        let next = f64::from_bits((f32::MAX as f64).to_bits() + 1);
        assert_eq!(
            ActorYaw::try_new(next).unwrap_err().problem(),
            MotionValueProblem::OutOfRange
        );
        assert_eq!(
            FiniteMeasure::try_new(-0.01, "distance")
                .unwrap_err()
                .problem(),
            MotionValueProblem::Negative
        );
        for accepted in [-0.0, 0.0, 2_000_000.0] {
            assert_eq!(
                FiniteMeasure::try_new(accepted, "distance")
                    .unwrap()
                    .value()
                    .to_bits(),
                accepted.to_bits()
            );
        }
        let yaw = ActorYaw::try_new(-(f32::MAX as f64)).unwrap();
        assert_eq!(yaw.godot_lane().to_bits(), (-f32::MAX).to_bits());
    }

    #[test]
    fn derived_pose_and_player_support_envelopes_admit_both_extreme_roots() {
        for y in [-1_000_002.0_f32, 1_000_002.0_f32] {
            assert_eq!(
                PosePoint::try_new(Vector3::new(0.0, y, 0.0))
                    .unwrap()
                    .world()
                    .y
                    .to_bits(),
                y.to_bits()
            );
            assert_eq!(
                SupportElevation::try_new(y).unwrap().y().to_bits(),
                y.to_bits()
            );
        }
        let high = SupportElevation::try_new(1_000_002.0).unwrap();
        let low = SupportElevation::try_new(-1_000_002.0).unwrap();
        assert_eq!(high.delta_from(low), 2_000_004.0);
        assert_eq!(low.delta_from(high), -2_000_004.0);
        let at_subnormal =
            SupportContact::try_new(Vector3::ZERO, Vector3::new(f32::from_bits(1), -0.0, 0.0))
                .unwrap();
        assert_eq!(at_subnormal.normal().x.to_bits(), 1);
        let zero =
            SupportContact::try_new(Vector3::ZERO, Vector3::new(-0.0, 0.0, -0.0)).unwrap_err();
        assert_eq!(
            (zero.field(), zero.problem()),
            ("support.normal", MotionValueProblem::ZeroVector)
        );
        assert_eq!(
            ActorPosition::try_new(Vector3::new(1_000_000.0, 0.0, 0.0))
                .unwrap()
                .planar_distance(
                    ActorPosition::try_new(Vector3::new(-1_000_000.0, 0.0, 0.0)).unwrap()
                )
                .value(),
            2_000_000.0
        );
    }

    #[test]
    fn pose_point_and_support_elevation_reject_exact_malformed_and_outside_lanes() {
        let outside = f32::from_bits(1_000_002.0_f32.to_bits() + 1);
        for lane in 0..3 {
            for (value, problem) in [
                (f32::NAN, MotionValueProblem::NonFinite),
                (f32::INFINITY, MotionValueProblem::NonFinite),
                (f32::NEG_INFINITY, MotionValueProblem::NonFinite),
                (outside, MotionValueProblem::OutOfRange),
                (-outside, MotionValueProblem::OutOfRange),
            ] {
                let mut lanes = [0.0, 0.0, 0.0];
                lanes[lane] = value;
                let error =
                    PosePoint::try_new(Vector3::new(lanes[0], lanes[1], lanes[2])).unwrap_err();
                assert_eq!(
                    (error.field(), error.problem()),
                    (
                        ["pose_point.x", "pose_point.y", "pose_point.z"][lane],
                        problem
                    )
                );
            }
        }
        let boundary = Vector3::new(-1_000_002.0, 1_000_002.0, -0.0);
        let accepted = PosePoint::try_new(boundary).unwrap().world();
        assert_eq!(
            [
                accepted.x.to_bits(),
                accepted.y.to_bits(),
                accepted.z.to_bits()
            ],
            [
                boundary.x.to_bits(),
                boundary.y.to_bits(),
                boundary.z.to_bits()
            ]
        );

        for (value, problem) in [
            (f32::NAN, MotionValueProblem::NonFinite),
            (f32::INFINITY, MotionValueProblem::NonFinite),
            (f32::NEG_INFINITY, MotionValueProblem::NonFinite),
            (outside, MotionValueProblem::OutOfRange),
            (-outside, MotionValueProblem::OutOfRange),
        ] {
            let error = SupportElevation::try_new(value).unwrap_err();
            assert_eq!(
                (error.field(), error.problem()),
                ("support_elevation", problem)
            );
        }
        for boundary in [-1_000_002.0_f32, 1_000_002.0_f32] {
            assert_eq!(
                SupportElevation::try_new(boundary).unwrap().y().to_bits(),
                boundary.to_bits()
            );
        }
    }

    #[test]
    fn support_contact_rejects_each_poisoned_point_or_normal_lane_and_preserves_bits() {
        for lane in 0..3 {
            for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut point = [1.0, 2.0, 3.0];
                point[lane] = poison;
                let error = SupportContact::try_new(
                    Vector3::new(point[0], point[1], point[2]),
                    Vector3::UP,
                )
                .unwrap_err();
                assert_eq!(
                    (error.field(), error.problem()),
                    (
                        ["support.point.x", "support.point.y", "support.point.z"][lane],
                        MotionValueProblem::NonFinite
                    )
                );

                let mut normal = [1.0, 2.0, 3.0];
                normal[lane] = poison;
                let error = SupportContact::try_new(
                    Vector3::ZERO,
                    Vector3::new(normal[0], normal[1], normal[2]),
                )
                .unwrap_err();
                assert_eq!(
                    (error.field(), error.problem()),
                    (
                        ["support.normal.x", "support.normal.y", "support.normal.z"][lane],
                        MotionValueProblem::NonFinite
                    )
                );
            }
        }
        let point = Vector3::new(-f32::MAX, f32::MAX, -0.0);
        let normal = Vector3::new(f32::from_bits(1), -f32::MAX, -0.0);
        let accepted = SupportContact::try_new(point, normal).unwrap();
        assert_eq!(
            [
                accepted.point().x.to_bits(),
                accepted.point().y.to_bits(),
                accepted.point().z.to_bits(),
                accepted.normal().x.to_bits(),
                accepted.normal().y.to_bits(),
                accepted.normal().z.to_bits(),
            ],
            [
                point.x.to_bits(),
                point.y.to_bits(),
                point.z.to_bits(),
                normal.x.to_bits(),
                normal.y.to_bits(),
                normal.z.to_bits(),
            ]
        );
    }

    #[test]
    fn velocity_value_doors_reject_exact_poison_and_preserve_boundary_bits() {
        for (x, z, field) in [
            (f32::NAN, 0.0, "planar_velocity.x"),
            (f32::INFINITY, 0.0, "planar_velocity.x"),
            (f32::NEG_INFINITY, 0.0, "planar_velocity.x"),
            (0.0, f32::NAN, "planar_velocity.z"),
            (0.0, f32::INFINITY, "planar_velocity.z"),
            (0.0, f32::NEG_INFINITY, "planar_velocity.z"),
        ] {
            let error = PlanarVelocity::try_new(x, z).unwrap_err();
            assert_eq!(
                (error.field(), error.problem()),
                (field, MotionValueProblem::NonFinite)
            );
        }
        let planar = PlanarVelocity::try_new(-0.0, f32::MAX).unwrap();
        assert_eq!(planar.x_mps().to_bits(), (-0.0_f32).to_bits());
        assert_eq!(planar.z_mps().to_bits(), f32::MAX.to_bits());

        for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let error = FiniteVelocity::try_new(poison).unwrap_err();
            assert_eq!(
                (error.field(), error.problem()),
                ("vertical_velocity_mps", MotionValueProblem::NonFinite)
            );
            let error = FiniteSpeed::try_new(poison).unwrap_err();
            assert_eq!(
                (error.field(), error.problem()),
                ("impact_speed_mps", MotionValueProblem::NonFinite)
            );
            let error = LandingEvent::try_new(poison, support()).unwrap_err();
            assert_eq!(
                (error.field(), error.problem()),
                ("impact_speed_mps", MotionValueProblem::NonFinite)
            );
        }
        assert_eq!(
            FiniteVelocity::try_new(-0.0).unwrap().mps().to_bits(),
            (-0.0_f32).to_bits()
        );
        assert_eq!(
            FiniteSpeed::try_new(-0.0).unwrap().mps().to_bits(),
            (-0.0_f32).to_bits()
        );
        for negative in [-f32::from_bits(1), -1.0] {
            let speed_error = FiniteSpeed::try_new(negative).unwrap_err();
            assert_eq!(
                (speed_error.field(), speed_error.problem()),
                ("impact_speed_mps", MotionValueProblem::Negative)
            );
            let event_error = LandingEvent::try_new(negative, support()).unwrap_err();
            assert_eq!(
                (event_error.field(), event_error.problem()),
                ("impact_speed_mps", MotionValueProblem::Negative)
            );
        }
        let event = LandingEvent::try_new(-0.0, support()).unwrap();
        assert_eq!(event.impact_speed().mps().to_bits(), (-0.0_f32).to_bits());
        assert_eq!(event.support(), support());
    }

    #[test]
    fn actor_velocity_rejects_each_poisoned_lane() {
        for lane in 0..3 {
            for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut values = [1.0, -2.0, 3.0];
                values[lane] = poison;
                let error = ActorVelocity::try_new(Vector3::new(values[0], values[1], values[2]))
                    .unwrap_err();
                assert_eq!(error.problem(), MotionValueProblem::NonFinite);
                assert_eq!(
                    error.field(),
                    ["actor_velocity.x", "actor_velocity.y", "actor_velocity.z"][lane]
                );
                assert!(
                    PlanarVelocity::try_from_world(Vector3::new(values[0], values[1], values[2]))
                        .is_err()
                );
            }
        }
        let velocity = ActorVelocity::try_new(Vector3::new(-0.0, -2.0, 3.0)).unwrap();
        assert_eq!(velocity.world().x.to_bits(), (-0.0_f32).to_bits());
        assert_eq!(velocity.planar().x_mps().to_bits(), (-0.0_f32).to_bits());
        assert_eq!(
            FiniteVelocity::try_new(f32::NAN).unwrap_err().field(),
            "vertical_velocity_mps"
        );
        assert_eq!(
            FiniteSpeed::try_new(-0.01).unwrap_err().problem(),
            MotionValueProblem::Negative
        );
    }

    #[test]
    fn controlled_support_never_creates_a_landing() {
        let prior_support =
            SupportContact::try_new(Vector3::new(-5.0, -4.0, -3.0), Vector3::UP).unwrap();
        let prior_landing = LandingEvent::try_new(0.5, prior_support).unwrap();
        let prior = MotionState::restore(
            MotionPhase::Controlled,
            Some(prior_support),
            Some(prior_landing),
        )
        .unwrap();
        let desired = PlanarVelocity::try_new(2.0, -3.0).unwrap();
        let prepared = prepare(
            prior,
            desired,
            StepDuration::from_raw(1.0 / 60.0),
            SupportMotionConfig::PLAYER_DEFAULT,
        );
        assert_eq!(
            prepared.command().world_velocity(),
            Vector3::new(2.0, 0.0, -3.0)
        );
        let transition = reconcile(
            prepared,
            MotionOutcome::new(
                ActorVelocity::try_new(Vector3::new(1.5, 0.25, -2.5)).unwrap(),
                Some(support()),
            ),
        );
        assert_eq!(transition.landing, None);
        assert_eq!(transition.state.phase(), MotionPhase::Controlled);
        assert_eq!(transition.state.support(), Some(support()));
        assert_eq!(transition.state.last_landing(), Some(prior_landing));
    }

    #[test]
    fn an_edge_captures_actual_trajectory_and_air_ignores_new_intent() {
        let prior_support =
            SupportContact::try_new(Vector3::new(-2.0, -1.0, 0.0), Vector3::UP).unwrap();
        let prior_landing = LandingEvent::try_new(0.75, prior_support).unwrap();
        let prior = MotionState::restore(
            MotionPhase::Controlled,
            Some(prior_support),
            Some(prior_landing),
        )
        .unwrap();
        let prepared = prepare(
            prior,
            PlanarVelocity::try_new(3.0, -4.0).unwrap(),
            StepDuration::from_raw(0.01),
            SupportMotionConfig::PLAYER_DEFAULT,
        );
        let edge = reconcile(
            prepared,
            MotionOutcome::new(
                ActorVelocity::try_new(Vector3::new(2.0, 0.0, -3.0)).unwrap(),
                None,
            ),
        );
        assert_eq!(edge.landing, None);
        assert_eq!(
            edge.state.phase(),
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::try_new(2.0, -3.0).unwrap(),
                vertical_velocity_mps: FiniteVelocity::try_new(-0.0).unwrap(),
            }
        );
        assert_eq!(edge.state.support(), None);
        assert_eq!(edge.state.last_landing(), Some(prior_landing));
        assert!(!edge.state.accepts_control());
        let air = prepare(
            edge.state,
            PlanarVelocity::try_new(99.0, 88.0).unwrap(),
            StepDuration::from_raw(0.0),
            SupportMotionConfig::PLAYER_DEFAULT,
        );
        let command = air.command().world_velocity();
        assert_eq!((command.x, command.z), (2.0, -3.0));
        assert_eq!(command.y.to_bits(), (-0.0_f32).to_bits());
    }

    #[test]
    fn a_wall_keeps_the_collision_adjusted_planar_trajectory() {
        let history_support =
            SupportContact::try_new(Vector3::new(8.0, 7.0, 6.0), Vector3::UP).unwrap();
        let prior_landing = LandingEvent::try_new(1.25, history_support).unwrap();
        let state = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::try_new(2.0, -3.0).unwrap(),
                vertical_velocity_mps: FiniteVelocity::try_new(-1.0).unwrap(),
            },
            None,
            Some(prior_landing),
        )
        .unwrap();
        let prepared = prepare(
            state,
            PlanarVelocity::ZERO,
            StepDuration::from_raw(0.0),
            SupportMotionConfig::PLAYER_DEFAULT,
        );
        let transition = reconcile(
            prepared,
            MotionOutcome::new(
                ActorVelocity::try_new(Vector3::new(0.0, 0.0, -2.5)).unwrap(),
                None,
            ),
        );
        assert_eq!(
            transition.state.phase(),
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::try_new(0.0, -2.5).unwrap(),
                vertical_velocity_mps: FiniteVelocity::try_new(-1.0).unwrap(),
            }
        );
        assert_eq!(transition.landing, None);
        assert_eq!(transition.state.support(), None);
        assert_eq!(transition.state.last_landing(), Some(prior_landing));
    }

    #[test]
    fn landing_changes_phase_once_and_keeps_the_event_as_observation() {
        let old_support =
            SupportContact::try_new(Vector3::new(9.0, 8.0, 7.0), Vector3::UP).unwrap();
        let old_landing = LandingEvent::try_new(0.25, old_support).unwrap();
        let airborne = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::try_new(1.0, 2.0).unwrap(),
                vertical_velocity_mps: FiniteVelocity::try_new(-3.0).unwrap(),
            },
            None,
            Some(old_landing),
        )
        .unwrap();
        let prepared = prepare(
            airborne,
            PlanarVelocity::ZERO,
            StepDuration::from_raw(0.0),
            SupportMotionConfig::PLAYER_DEFAULT,
        );
        let landed = reconcile(
            prepared,
            MotionOutcome::new(
                ActorVelocity::try_new(Vector3::new(0.5, 0.0, 1.0)).unwrap(),
                Some(support()),
            ),
        );
        let expected = LandingEvent::try_new(3.0, support()).unwrap();
        assert_eq!(landed.landing, Some(expected));
        assert_eq!(landed.state.phase(), MotionPhase::Controlled);
        assert_eq!(landed.state.support(), Some(support()));
        assert_eq!(landed.state.last_landing(), Some(expected));

        let controlled = prepare(
            landed.state,
            PlanarVelocity::ZERO,
            StepDuration::from_raw(0.1),
            SupportMotionConfig::PLAYER_DEFAULT,
        );
        let still_supported = reconcile(
            controlled,
            MotionOutcome::new(
                ActorVelocity::try_new(Vector3::ZERO).unwrap(),
                Some(support()),
            ),
        );
        assert_eq!(still_supported.landing, None);
        assert_eq!(still_supported.state.phase(), MotionPhase::Controlled);
        assert_eq!(still_supported.state.support(), Some(support()));
        assert_eq!(still_supported.state.last_landing(), Some(expected));
    }

    #[test]
    fn relocation_retains_only_inert_landing_history() {
        let event = LandingEvent::try_new(3.0, support()).unwrap();
        let state =
            MotionState::restore(MotionPhase::Controlled, Some(support()), Some(event)).unwrap();
        let relocated = state.relocated();
        assert_eq!(relocated.phase(), MotionPhase::Controlled);
        assert_eq!(relocated.support(), None);
        assert_eq!(relocated.last_landing(), Some(event));
        assert!(relocated.accepts_control());
    }

    #[test]
    fn restore_validation_rejects_poison_mismatch_and_terminal_violations() {
        let positive = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::ZERO,
                vertical_velocity_mps: FiniteVelocity::try_new(0.01).unwrap(),
            },
            None,
            None,
        )
        .unwrap_err();
        assert_eq!(
            (positive.field(), positive.problem()),
            (
                "motion_phase.vertical_velocity_mps",
                MotionValueProblem::InconsistentState
            )
        );
        let with_support = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::ZERO,
                vertical_velocity_mps: FiniteVelocity::try_new(-1.0).unwrap(),
            },
            Some(support()),
            None,
        )
        .unwrap_err();
        assert_eq!(
            (with_support.field(), with_support.problem()),
            (
                "motion_state.support",
                MotionValueProblem::InconsistentState
            )
        );

        let airborne = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::try_new(1.0, -2.0).unwrap(),
                vertical_velocity_mps: FiniteVelocity::try_new(-5.0).unwrap(),
            },
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            validate_restore(
                airborne,
                ActorVelocity::try_new(Vector3::new(1.25, 30.0, -2.0)).unwrap(),
                SupportMotionConfig::PLAYER_DEFAULT,
            ),
            Err(MotionRestoreError::AirbornePlanarMismatch { axis: "x" })
        );
        assert!(
            validate_restore(
                airborne,
                ActorVelocity::try_new(Vector3::new(1.0, 30.0, -2.0)).unwrap(),
                SupportMotionConfig::PLAYER_DEFAULT,
            )
            .is_ok()
        );
        let too_fast = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::ZERO,
                vertical_velocity_mps: FiniteVelocity::try_new(-20.000_002).unwrap(),
            },
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            validate_restore(
                too_fast,
                ActorVelocity::try_new(Vector3::ZERO).unwrap(),
                SupportMotionConfig::PLAYER_DEFAULT,
            ),
            Err(MotionRestoreError::AirborneTerminalExceeded)
        );
        assert!(
            MotionRestoreError::AirborneTerminalExceeded
                .to_string()
                .contains("terminal")
        );
    }

    #[test]
    fn signed_zero_planar_restore_is_bit_exact() {
        let state = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::try_new(-0.0, 0.0).unwrap(),
                vertical_velocity_mps: FiniteVelocity::try_new(-0.0).unwrap(),
            },
            None,
            None,
        )
        .unwrap();
        assert!(
            validate_restore(
                state,
                ActorVelocity::try_new(Vector3::new(-0.0, 91.0, 0.0)).unwrap(),
                SupportMotionConfig::PLAYER_DEFAULT,
            )
            .is_ok()
        );
        assert_eq!(
            validate_restore(
                state,
                ActorVelocity::try_new(Vector3::new(0.0, 91.0, 0.0)).unwrap(),
                SupportMotionConfig::PLAYER_DEFAULT,
            ),
            Err(MotionRestoreError::AirbornePlanarMismatch { axis: "x" })
        );
        assert_eq!(
            validate_restore(
                state,
                ActorVelocity::try_new(Vector3::new(-0.0, 91.0, -0.0)).unwrap(),
                SupportMotionConfig::PLAYER_DEFAULT,
            ),
            Err(MotionRestoreError::AirbornePlanarMismatch { axis: "z" })
        );
    }

    #[test]
    fn prepared_terminal_state_validates_for_nonrepresentable_decimal_config() {
        let config = SupportMotionConfig::try_new(9.8, 0.6, 0.0, 4.0, 0.85, 5.0).unwrap();
        let prior = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::try_new(1.0, -2.0).unwrap(),
                vertical_velocity_mps: FiniteVelocity::try_new(-0.59).unwrap(),
            },
            None,
            None,
        )
        .unwrap();
        let prepared = prepare(
            prior,
            PlanarVelocity::ZERO,
            StepDuration::from_raw(0.5),
            config,
        );
        assert_eq!(
            prepared.command().world_velocity().y.to_bits(),
            (-0.6_f32).to_bits()
        );
        let transition = reconcile(
            prepared,
            MotionOutcome::new(
                ActorVelocity::try_new(Vector3::new(1.0, 0.0, -2.0)).unwrap(),
                None,
            ),
        );
        assert!(
            validate_restore(
                transition.state,
                ActorVelocity::try_new(Vector3::new(1.0, 19.0, -2.0)).unwrap(),
                config,
            )
            .is_ok()
        );
        let too_negative = f32::from_bits((-0.6_f32).to_bits() + 1);
        let invalid = MotionState::restore(
            MotionPhase::Airborne {
                planar_velocity_mps: PlanarVelocity::try_new(1.0, -2.0).unwrap(),
                vertical_velocity_mps: FiniteVelocity::try_new(too_negative).unwrap(),
            },
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            validate_restore(
                invalid,
                ActorVelocity::try_new(Vector3::new(1.0, 0.0, -2.0)).unwrap(),
                config,
            ),
            Err(MotionRestoreError::AirborneTerminalExceeded)
        );
    }

    #[test]
    fn controlled_contact_gate_requires_two_controlled_phases_without_landing() {
        let controlled = MotionPhase::Controlled;
        let air = MotionPhase::Airborne {
            planar_velocity_mps: PlanarVelocity::ZERO,
            vertical_velocity_mps: FiniteVelocity::try_new(-1.0).unwrap(),
        };
        let landing = Some(LandingEvent::try_new(2.0, support()).unwrap());
        for (before, after, event, always, controlled_contact) in [
            (controlled, controlled, None, true, true),
            (controlled, controlled, landing, true, false),
            (controlled, air, None, true, false),
            (controlled, air, landing, true, false),
            (air, controlled, None, true, false),
            (air, controlled, landing, true, false),
            (air, air, None, true, false),
            (air, air, landing, true, false),
        ] {
            assert_eq!(QueuedWaveGate::Always.allows(before, after, event), always);
            assert_eq!(
                QueuedWaveGate::ControlledContact.allows(before, after, event),
                controlled_contact
            );
        }
    }

    #[test]
    fn queued_wave_gate_wire_names_are_stable() {
        assert_eq!(QueuedWaveGate::Always.wire_name(), "always");
        assert_eq!(
            QueuedWaveGate::ControlledContact.wire_name(),
            "controlled_contact"
        );
    }

    #[test]
    fn footstep_suppression_persists_until_acknowledged() {
        let landing = Some(LandingEvent::try_new(2.0, support()).unwrap());
        assert_eq!(
            FootstepSuppression::CLEAR.on_transition(None),
            FootstepSuppression::CLEAR
        );
        let pending = FootstepSuppression::CLEAR.on_transition(landing);
        assert!(pending.pending());
        assert!(pending.on_transition(None).pending());
        let (clear, acknowledged) = pending.acknowledge();
        assert_eq!((clear, acknowledged), (FootstepSuppression::CLEAR, true));
        assert_eq!(clear.acknowledge(), (FootstepSuppression::CLEAR, false));
        assert!(FootstepSuppression::restore(true).pending());
    }

    #[test]
    fn zero_gain_or_range_produces_no_voice() {
        let event = LandingEvent::try_new(4.0, support()).unwrap();
        let zero_gain = SupportMotionConfig::try_new(9.8, 20.0, 1.5, 4.0, 0.0, 5.0).unwrap();
        let zero_range = SupportMotionConfig::try_new(9.8, 20.0, 1.5, 4.0, 0.85, 0.0).unwrap();
        assert_eq!(landing_voice(event, zero_gain), None);
        assert_eq!(landing_voice(event, zero_range), None);
        assert_eq!(
            LandingEvent::try_new(-0.1, support())
                .unwrap_err()
                .problem(),
            MotionValueProblem::Negative
        );
    }

    #[test]
    fn a_landing_strictly_below_the_silent_threshold_stays_silent() {
        // Below `landing_silent_speed_mps`, severity is negative, so the
        // trailing `gain == 0.0 || range_m == 0.0` fallback never fires (it
        // would need to compare against a negative, not zero). Only the
        // early return can silence this speed; without it the function
        // would hand back `Some(LandingVoice { gain: -0.17, range_m: -1.0 })`
        // for the player and a matching negative pair for the cat.
        let event = LandingEvent::try_new(1.0, support()).unwrap();
        assert_eq!(
            landing_voice(event, SupportMotionConfig::PLAYER_DEFAULT),
            None
        );
        assert_eq!(landing_voice(event, SupportMotionConfig::CAT_DEFAULT), None);
    }
}
