//! The cat's walk — a four-beat lateral-sequence gait with truly planted
//! paws. Real cats step hind-then-fore down each side (LH, LF, RH, RF),
//! keep two or three paws grounded at a walk, and place each paw where it
//! will stay: a planted paw NEVER slides. The module owns that law as
//! state — per-leg planted world positions, swing arcs re-aimed every
//! frame (so turning mid-swing lands the paw where the body is actually
//! going), and touchdown contact events the engine layer turns into paw
//! waves.
//!
//! The acoustic voice lives here too, like the fan's voice lives in
//! [`crate::fan_wave`]: paw-step waves are pulse kind 2 (footstep — the
//! least precious slot class), small and soft, and only the FORE paws
//! sound. A walking cat direct-registers — each hind paw lands in the
//! ipsilateral fore paw's print — so the fore touch announces a fresh
//! spot and the hind lands on ground already pressed, silently. Half the
//! sounds, and the halving is the realism.
//!
//! Everything is f64 until a Vector3 lane narrows it, per the crate's
//! precision law; phase advance is dt-driven and clamped, so a stalled
//! clock buys a step, never a burst — the [`crate::fan_wave`] doctrine.

use godot::builtin::Vector3;

use crate::reproduce::RestoreValueError;
use crate::support_motion::{
    ActorPosition, ActorYaw, FiniteMeasure, MotionValueError, PosePoint, StepDuration,
    SupportElevation,
};

/// Number of legs; index order is LF, RF, LH, RH.
pub const LEGS: usize = 4;

/// Meters one full stride cycle carries the body at a walk.
pub const STRIDE_LEN: f64 = 0.30;

/// Fraction of the cycle each paw spends grounded (stance). 0.70 keeps
/// two paws down always and three most of the time — the stately walk.
pub const DUTY: f64 = 0.70;

/// The gait's design envelope: the fastest sustained walk the paw-wave
/// budget below is pinned against. The brain's wander speed must stay
/// under this.
pub const TOP_SPEED: f64 = 0.65;

/// Paw arc apex in meters — a cat lifts its paws barely off the ground.
pub const SWING_LIFT: f64 = 0.035;

/// Ride height of the chest/hip line above the floor, meters.
pub const BODY_H: f64 = 0.20;

/// Walk bob amplitude, meters — two gentle rises per stride.
pub const BOB_AMP: f64 = 0.006;

/// Paw wave reach in meters — a visible ripple that expands past the cat
/// and washes the floor around it, yet still shorter than the hero's own
/// 1.6 m steps: the cat is quieter than you, never silent.
pub const PAW_RANGE: f64 = 1.3;

/// Paw wavefront speed, m/s — same air as the hero's footsteps.
pub const PAW_SPEED: f64 = 4.0;

/// Paw wave loudness — soft pads, softer than the hero's 0.8 shoes, but
/// bright enough to read as a wave, not a rumour.
pub const PAW_GAIN: f64 = 0.6;

/// Idle "presence" cadence, seconds — even a standing cat breathes a
/// faint wave on this slow beat, so it never sinks into full black. In a
/// world lit only by sound, a companion the hero can lose entirely is a
/// companion the hero cannot keep; the presence pulse is the cat's
/// heartbeat, always findable.
pub const PRESENCE_EVERY: f64 = 1.6;

/// Presence wave reach, meters — a gentle bloom around the whole cat, its
/// tail long enough that the blooms overlap into a continuous soft glow.
pub const PRESENCE_RANGE: f64 = 1.1;

/// Presence loudness — the faintest voice, a heartbeat, not a footfall.
pub const PRESENCE_GAIN: f64 = 0.45;

/// Presence wave birth height — the chest, not the floor.
pub const PRESENCE_HEIGHT: f64 = 0.18;

/// Walk-gate hysteresis: the cat starts stepping only past [`MOVE_HI`]
/// and only stops stepping below [`MOVE_LO`]. The measured speed the node
/// feeds is the real move_and_slide displacement, which can jitter around
/// a single threshold when the body grazes a wall or slides a corner; the
/// band keeps a near-stalled cat from machine-gunning settle contacts.
pub const MOVE_HI: f64 = 0.08;

/// The low edge of the walk-gate hysteresis band — see [`MOVE_HI`].
pub const MOVE_LO: f64 = 0.03;

/// Stride phase at which each leg touches down — the lateral sequence:
/// LH at 0, LF a quarter later, RH at half, RF at three quarters.
/// Indexed LF, RF, LH, RH.
const OFFSET: [f64; LEGS] = [0.25, 0.75, 0.0, 0.5];

/// Which side each leg hangs on: -1 left, +1 right. Indexed LF, RF, LH, RH.
const SIDE: [f64; LEGS] = [-1.0, 1.0, -1.0, 1.0];

/// Shoulder (fore) / hip (hind) anchor, meters ahead of the body center.
const FORE_AFT: [f64; LEGS] = [0.145, 0.145, -0.145, -0.145];

/// Anchor's lateral distance from the centerline — a narrow track: the
/// silhouette walks a tightrope, the way a cat's prints line up.
const LATERAL: [f64; LEGS] = [0.048, 0.048, 0.055, 0.055];

/// How far ahead of its anchor a paw lands: half the stance sweep, so the
/// plant spends its grounded life passing from ahead of the anchor to
/// behind it while the body walks over it.
const AHEAD: f64 = STRIDE_LEN * DUTY * 0.5;

/// The walk-amplitude ease rate, 1/s — the viewmodel's own constant.
const AMP_RATE: f64 = 6.0;

/// The largest stride-phase step one tick may take: a stalled or jumped
/// clock advances the cycle a quarter at most, so no leg ever skips a
/// whole swing unseen and no tick fires more than one contact per leg.
const MAX_PHASE_STEP: f64 = 0.25;

/// Only the LEAD fore paw (LF) sounds — the cat's one soft pulse per
/// stride. Two reasons braid here: a walking cat direct-registers (each
/// hind paw lands in the ipsilateral fore print already pressed, so the
/// hinds are silent), AND a cat's padded step is so nearly silent that
/// one faint pulse a stride is the honest amount of sound it makes. One
/// emitter also halves the cat's claim on the 64-slot pool it shares
/// with the hero's own footsteps and the fan hum — a lantern that blinks
/// gently, never a chatterbox that evicts the hero's perception.
#[must_use]
pub fn paw_sounds(leg: usize) -> bool {
    leg == 0
}

/// One paw touching down: which leg, and the exact current support point
/// where the engine layer births a paw wave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Leg index (LF, RF, LH, RH).
    pub leg: usize,
    /// The touchdown point on the floor.
    pub at: Vector3,
}

/// One tick's gait outputs — everything the body model needs to pose the
/// cat, plus the touchdown events for the paw waves.
#[derive(Debug, Clone, PartialEq)]
pub struct GaitFrame {
    /// Paw world positions, LF RF LH RH; Y equals the support datum while planted.
    pub paws: [Vector3; LEGS],
    /// The paws that touched down THIS tick.
    pub contacts: Vec<Contact>,
    /// Stride phase in [0, 1) — tail sway and bob ride it.
    pub phase: f64,
    /// Walk amplitude easing between standing (0) and walking (1).
    pub amp: f64,
    /// Body lift above [`BODY_H`] this frame (the walk bob).
    pub bob: f64,
    /// Whether the walk gate was open this tick.
    pub moving: bool,
    /// Exact uniform support translation applied before this tick's gait law.
    pub support_delta_y: f32,
}

/// The gait state machine: stride phase, walk amplitude, and the four
/// planted paws. One instance per cat, advanced every physics tick with
/// the body's ACTUAL planar motion — a wall that stops the body stops
/// the stepping too, honestly.
#[derive(Debug, Clone, PartialEq)]
pub struct CatGait {
    phase: f64,
    amp: f64,
    planted: [Vector3; LEGS],
    /// Where each swing is currently aimed; the touchdown point when the
    /// leg comes back to stance.
    aim: [Vector3; LEGS],
    in_swing: [bool; LEGS],
    /// The hysteretic walk gate's current state — see [`MOVE_HI`].
    moving: bool,
    /// The one support datum shared by every planted paw and swing aim.
    support: SupportElevation,
}

#[derive(Debug, Clone)]
pub struct PreparedCatGait(CatGait);

/// Everything a CatGait is, as data — the planted paws included, or the
/// restored cat's stride starts by sliding into place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaitCapture {
    pub phase: f64,
    pub amp: f64,
    pub support_y: f32,
    pub planted: [Vector3; LEGS],
    pub aim: [Vector3; LEGS],
    pub in_swing: [bool; LEGS],
    pub moving: bool,
}

impl CatGait {
    pub fn prepare_restore(capture: GaitCapture) -> Result<PreparedCatGait, RestoreValueError> {
        if !capture.phase.is_finite() {
            return Err(RestoreValueError::new("gait.phase", "must be finite"));
        }
        if !(0.0..1.0).contains(&capture.phase) {
            return Err(RestoreValueError::new("gait.phase", "must be in 0..1"));
        }
        if !capture.amp.is_finite() {
            return Err(RestoreValueError::new("gait.amp", "must be finite"));
        }
        if !(0.0..=1.0).contains(&capture.amp) {
            return Err(RestoreValueError::new("gait.amp", "must be in 0..=1"));
        }
        let support = SupportElevation::try_new(capture.support_y)
            .map_err(|_| RestoreValueError::new("gait.support_y", "is outside its valid range"))?;
        for (group, points) in [("planted", &capture.planted), ("aim", &capture.aim)] {
            for (index, point) in points.iter().enumerate() {
                PosePoint::try_new(*point).map_err(|error| {
                    let axis = error.field().rsplit('.').next().unwrap_or(error.field());
                    RestoreValueError::new(
                        format!("gait.{group}[{index}].{axis}"),
                        "must be finite and inside the pose envelope",
                    )
                })?;
                if point.y.to_bits() != capture.support_y.to_bits() {
                    return Err(RestoreValueError::new(
                        format!("gait.{group}[{index}].y"),
                        "must match support_y bit-for-bit",
                    ));
                }
            }
        }
        for (leg, swinging) in capture.in_swing.iter().copied().enumerate() {
            let expected = capture.moving && (capture.phase - OFFSET[leg]).rem_euclid(1.0) >= DUTY;
            if swinging != expected {
                return Err(RestoreValueError::new(
                    format!("gait.in_swing[{leg}]"),
                    "must agree with phase and moving",
                ));
            }
        }
        Ok(PreparedCatGait(Self::restore(capture, support)))
    }

    #[must_use]
    pub fn from_prepared(capture: PreparedCatGait) -> Self {
        capture.0
    }

    /// A fresh gait standing at `pos` facing `yaw`: every paw planted at
    /// its neutral anchor, phase zeroed, standing still.
    pub fn new(pos: ActorPosition, yaw: ActorYaw) -> Result<Self, MotionValueError> {
        let world = pos.world();
        let mut planted = [Vector3::ZERO; LEGS];
        for (leg, spot) in planted.iter_mut().enumerate() {
            *spot = PosePoint::try_new(anchor(world, yaw.radians(), leg))?.world();
        }
        Ok(Self {
            phase: 0.0,
            amp: 0.0,
            planted,
            aim: planted,
            in_swing: [false; LEGS],
            moving: false,
            support: pos.elevation(),
        })
    }

    /// The whole gait as data — every planted paw and swing aim, or the
    /// restored cat's stride starts by sliding into place.
    #[must_use]
    pub fn capture(&self) -> GaitCapture {
        GaitCapture {
            phase: self.phase,
            amp: self.amp,
            support_y: self.support.y(),
            planted: self.planted,
            aim: self.aim,
            in_swing: self.in_swing,
            moving: self.moving,
        }
    }

    /// A gait rebuilt mid-stride — the one thing `new` cannot express (it
    /// hard-codes every paw planted at its neutral anchor).
    fn restore(capture: GaitCapture, support: SupportElevation) -> Self {
        Self {
            phase: capture.phase,
            amp: capture.amp,
            planted: capture.planted,
            aim: capture.aim,
            in_swing: capture.in_swing,
            moving: capture.moving,
            support,
        }
    }

    /// Advance one tick. `pos` is the body center on its current support;
    /// its exact Y lane transports every grounded gait datum before the
    /// stride law runs. `yaw` is the heading, `speed` the ACTUAL planar speed the
    /// body achieved — feed the measured displacement, not the wish, so
    /// blocked bodies stop stepping. The walk gate is hysteretic
    /// ([`MOVE_HI`]/[`MOVE_LO`]) so a body grazing a wall near the
    /// threshold doesn't flicker between stepping and settling.
    pub fn advance(
        &mut self,
        dt: StepDuration,
        pos: ActorPosition,
        yaw: ActorYaw,
        speed: FiniteMeasure,
    ) -> Result<GaitFrame, MotionValueError> {
        let mut next = self.clone();
        let frame = next.advance_checked(dt.seconds(), pos, yaw.radians(), speed.value())?;
        *self = next;
        Ok(frame)
    }

    fn advance_checked(
        &mut self,
        dt: f64,
        pos: ActorPosition,
        yaw: f64,
        speed: f64,
    ) -> Result<GaitFrame, MotionValueError> {
        let support_delta_y = self.transport_support(pos);
        let pos = pos.world();
        let moving = if self.moving {
            speed > MOVE_LO
        } else {
            speed > MOVE_HI
        };
        self.moving = moving;
        self.amp += ((if moving { 1.0 } else { 0.0 }) - self.amp) * (dt * AMP_RATE).min(1.0);
        let mut contacts = Vec::new();
        if moving {
            self.phase = (self.phase + (dt * speed / STRIDE_LEN).min(MAX_PHASE_STEP)).fract();
        } else {
            self.settle(&mut contacts);
        }
        let mut paws = [Vector3::ZERO; LEGS];
        for (leg, paw) in paws.iter_mut().enumerate() {
            *paw = if moving {
                self.step_leg(leg, pos, yaw, &mut contacts)
            } else {
                self.planted[leg]
            };
        }
        let frame = GaitFrame {
            paws,
            contacts,
            phase: self.phase,
            amp: self.amp,
            bob: BOB_AMP * (self.phase * std::f64::consts::TAU * 2.0).sin() * self.amp,
            moving,
            support_delta_y,
        };
        self.validate_points(&frame)?;
        Ok(frame)
    }

    fn transport_support(&mut self, new_position: ActorPosition) -> f32 {
        let new_support = new_position.elevation();
        let delta_y = new_support.delta_from(self.support);
        let new_support_y = new_support.y();
        for point in &mut self.planted {
            point.y = new_support_y;
        }
        for point in &mut self.aim {
            point.y = new_support_y;
        }
        self.support = new_support;
        delta_y
    }

    fn validate_points(&self, frame: &GaitFrame) -> Result<(), MotionValueError> {
        for point in self
            .planted
            .into_iter()
            .chain(self.aim)
            .chain(frame.paws)
            .chain(frame.contacts.iter().map(|contact| contact.at))
        {
            PosePoint::try_new(point)?;
        }
        Ok(())
    }

    /// One walking leg: stance holds the plant; swing re-aims every frame
    /// at where the anchor is going and arcs there; the swing-to-stance
    /// edge is the touchdown.
    fn step_leg(
        &mut self,
        leg: usize,
        pos: Vector3,
        yaw: f64,
        contacts: &mut Vec<Contact>,
    ) -> Vector3 {
        let lp = (self.phase - OFFSET[leg]).rem_euclid(1.0);
        if lp < DUTY {
            if self.in_swing[leg] {
                self.in_swing[leg] = false;
                self.planted[leg] = self.aim[leg];
                contacts.push(Contact {
                    leg,
                    at: self.planted[leg],
                });
            }
            return self.planted[leg];
        }
        if !self.in_swing[leg] {
            self.in_swing[leg] = true;
        }
        // Re-aim every swing frame: by touchdown the body will have walked
        // (1 - lp) of a stride, so the anchor's future is one term of
        // STRIDE_LEN — the speed cancels out of its own prediction.
        let fw = forward(yaw);
        self.aim[leg] = anchor(pos, yaw, leg) + fw * ((AHEAD + (1.0 - lp) * STRIDE_LEN) as f32);
        let sw = (lp - DUTY) / (1.0 - DUTY);
        let eased = sw * sw * (3.0 - 2.0 * sw);
        let mut paw = self.planted[leg].lerp(self.aim[leg], eased as f32);
        paw.y =
            self.support.y() + (SWING_LIFT * self.amp * (sw * std::f64::consts::PI).sin()) as f32;
        paw
    }

    /// Standing still: any paw caught mid-swing touches straight down
    /// where it hangs — not forward at its far aim. The phase is frozen
    /// (advance did not step it this tick), so the leg's swing progress
    /// is exactly the last moving frame's, and the hang point is that
    /// frame's `planted.lerp(aim, eased)`; the paw drops to that xz, a
    /// small honest step, never a teleport across the stride.
    #[expect(
        clippy::needless_range_loop,
        reason = "the body indexes four parallel per-leg arrays (in_swing, \
                  OFFSET, planted, aim) by the same leg index; a range loop \
                  reads far clearer than zipping four iterators"
    )]
    fn settle(&mut self, contacts: &mut Vec<Contact>) {
        for leg in 0..LEGS {
            if !self.in_swing[leg] {
                continue;
            }
            self.in_swing[leg] = false;
            let lp = (self.phase - OFFSET[leg]).rem_euclid(1.0);
            let sw = ((lp - DUTY) / (1.0 - DUTY)).clamp(0.0, 1.0);
            let eased = sw * sw * (3.0 - 2.0 * sw);
            let hang = self.planted[leg].lerp(self.aim[leg], eased as f32);
            let spot = Vector3::new(hang.x, self.support.y(), hang.z);
            self.planted[leg] = spot;
            contacts.push(Contact { leg, at: spot });
        }
    }
}

/// The heading's forward vector — Godot yaw convention: yaw 0 faces -Z.
fn forward(yaw: f64) -> Vector3 {
    Vector3::new((-yaw.sin()) as f32, 0.0, (-yaw.cos()) as f32)
}

/// The heading's right vector.
fn rightward(yaw: f64) -> Vector3 {
    Vector3::new(yaw.cos() as f32, 0.0, (-yaw.sin()) as f32)
}

/// A leg's anchor: its shoulder or hip projected onto the floor for the
/// body standing at `pos` facing `yaw`.
fn anchor(pos: Vector3, yaw: f64, leg: usize) -> Vector3 {
    let spot = pos
        + forward(yaw) * (FORE_AFT[leg] as f32)
        + rightward(yaw) * ((LATERAL[leg] * SIDE[leg]) as f32);
    Vector3::new(spot.x, pos.y, spot.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::support_motion::{ActorPosition, ActorYaw, FiniteMeasure, PosePoint, StepDuration};

    fn actor_position(world: Vector3) -> ActorPosition {
        ActorPosition::try_new(world).expect("test position must be in the actor domain")
    }

    fn actor_yaw(radians: f64) -> ActorYaw {
        ActorYaw::try_new(radians).expect("test yaw must be in the actor domain")
    }

    fn duration(seconds: f64) -> StepDuration {
        StepDuration::from_raw(seconds)
    }

    fn speed(meters_per_second: f64) -> FiniteMeasure {
        FiniteMeasure::try_new(meters_per_second, "test.actual_speed")
            .expect("test speed must be finite and non-negative")
    }

    #[test]
    fn height_change_transports_every_planted_paw_and_swing_aim() {
        let yaw = actor_yaw(0.0);
        let origin = actor_position(Vector3::ZERO);
        let mut gait = CatGait::new(origin, yaw).unwrap();
        for tick in 1..=24 {
            let position = actor_position(Vector3::new(0.0, 0.0, -0.01 * tick as f32));
            let _ = gait
                .advance(duration(DT), position, yaw, speed(0.6))
                .unwrap();
        }
        let before = gait.capture();
        assert!(before.in_swing.iter().any(|swinging| *swinging));

        let lifted_position = actor_position(Vector3::new(0.0, 0.75, -0.24));
        let frame = gait
            .advance(duration(0.0), lifted_position, yaw, speed(0.6))
            .unwrap();
        let after = gait.capture();

        assert_eq!(frame.support_delta_y.to_bits(), 0.75_f32.to_bits());
        assert_eq!(after.support_y.to_bits(), 0.75_f32.to_bits());
        for index in 0..LEGS {
            assert_eq!(after.planted[index].y.to_bits(), 0.75_f32.to_bits());
            assert_eq!(after.aim[index].y.to_bits(), 0.75_f32.to_bits());
            assert_eq!(
                after.planted[index].x.to_bits(),
                before.planted[index].x.to_bits()
            );
            assert_eq!(
                after.planted[index].z.to_bits(),
                before.planted[index].z.to_bits()
            );
            assert_eq!(after.aim[index].x.to_bits(), before.aim[index].x.to_bits());
            assert_eq!(after.aim[index].z.to_bits(), before.aim[index].z.to_bits());
        }
    }

    #[test]
    fn elevated_swing_and_settle_never_return_to_world_zero() {
        let yaw = actor_yaw(0.0);
        let mut position = Vector3::new(0.0, 0.75, 0.0);
        let mut gait = CatGait::new(actor_position(position), yaw).unwrap();
        let airborne = (0..240)
            .find_map(|_| {
                position.z -= (0.6 * DT) as f32;
                let frame = gait
                    .advance(duration(DT), actor_position(position), yaw, speed(0.6))
                    .unwrap();
                if frame.paws.iter().any(|paw| paw.y > 0.75) {
                    Some(frame)
                } else {
                    None
                }
            })
            .expect("a walking gait must enter swing within four seconds");
        assert!(airborne.paws.iter().all(|paw| paw.y >= 0.75));

        let settled = gait
            .advance(duration(DT), actor_position(position), yaw, speed(0.0))
            .unwrap();
        for paw in settled.paws {
            assert_eq!(paw.y.to_bits(), 0.75_f32.to_bits());
        }
        for contact in settled.contacts {
            assert_eq!(contact.at.y.to_bits(), 0.75_f32.to_bits());
        }
    }

    #[test]
    fn captured_gait_restores_elevated_state_in_lockstep() {
        let yaw = actor_yaw(0.3);
        let position = actor_position(Vector3::new(1.25, 0.75, -2.5));
        let mut original = CatGait::new(position, yaw).unwrap();
        for _ in 0..17 {
            let _ = original
                .advance(duration(DT), position, yaw, speed(0.55))
                .unwrap();
        }
        let capture = original.capture();
        let mut restored = CatGait::from_prepared(
            CatGait::prepare_restore(capture).expect("self-capture must restore"),
        );
        assert_eq!(restored.capture(), capture);

        let expected = original
            .advance(duration(DT), position, yaw, speed(0.55))
            .unwrap();
        let actual = restored
            .advance(duration(DT), position, yaw, speed(0.55))
            .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn adversarial_fractional_height_round_trip_has_no_future_ulp_shift() {
        let yaw = actor_yaw(-0.45);
        let old_y = 0.146_820_16_f32;
        let new_y = -0.440_136_85_f32;
        let old_position = actor_position(Vector3::new(0.5, old_y, -0.75));
        let new_position = actor_position(Vector3::new(0.5, new_y, -0.75));
        let mut gait = CatGait::new(old_position, yaw).unwrap();
        for _ in 0..23 {
            let _ = gait
                .advance(duration(DT), old_position, yaw, speed(0.6))
                .unwrap();
        }
        let _ = gait
            .advance(duration(0.0), new_position, yaw, speed(0.6))
            .unwrap();
        let capture = gait.capture();
        let mut restored = CatGait::from_prepared(
            CatGait::prepare_restore(capture).expect("self-capture must restore"),
        );

        let _ = restored
            .advance(duration(0.0), new_position, yaw, speed(0.6))
            .unwrap();
        let after = restored.capture();
        assert_eq!(after.support_y.to_bits(), new_y.to_bits());
        assert_eq!(after.phase.to_bits(), capture.phase.to_bits());
        assert_eq!(after.amp.to_bits(), capture.amp.to_bits());
        assert_eq!(after.in_swing, capture.in_swing);
        assert_eq!(after.moving, capture.moving);
        for index in 0..LEGS {
            for (before, future) in [
                (capture.planted[index], after.planted[index]),
                (capture.aim[index], after.aim[index]),
            ] {
                assert_eq!(future.x.to_bits(), before.x.to_bits());
                assert_eq!(future.y.to_bits(), new_y.to_bits());
                assert_eq!(future.y.to_bits(), before.y.to_bits());
                assert_eq!(future.z.to_bits(), before.z.to_bits());
            }
        }
    }

    #[test]
    fn extreme_valid_root_produces_pose_points_inside_the_derived_envelope() {
        for (root, yaw) in [
            (Vector3::new(1_000_000.0, 1_000_000.0, 0.0), 0.0),
            (Vector3::new(-1_000_000.0, -1_000_000.0, 0.0), 0.0),
            (
                Vector3::new(1_000_000.0, 1_000_000.0, -1_000_000.0),
                std::f64::consts::FRAC_PI_4,
            ),
            (
                Vector3::new(-1_000_000.0, -1_000_000.0, 1_000_000.0),
                -std::f64::consts::FRAC_PI_4,
            ),
        ] {
            let root = actor_position(root);
            let yaw = actor_yaw(yaw);
            let mut gait = CatGait::new(root, yaw).unwrap();
            let frame = gait
                .advance(duration(DT), root, yaw, speed(TOP_SPEED))
                .unwrap();
            let capture = gait.capture();
            for point in frame
                .paws
                .into_iter()
                .chain(capture.planted)
                .chain(capture.aim)
            {
                PosePoint::try_new(point)
                    .expect("derived gait point must stay in the pose envelope");
                for lane in [point.x, point.y, point.z] {
                    assert!(lane.abs() <= 1_000_002.0);
                }
            }
        }
    }

    #[test]
    fn malformed_captured_point_phase_or_amplitude_refuses_gait_restore() {
        let position = actor_position(Vector3::ZERO);
        let yaw = actor_yaw(0.0);
        let base = CatGait::new(position, yaw).unwrap().capture();

        let mut bad_phase = base;
        bad_phase.phase = 1.0;
        assert_eq!(
            CatGait::prepare_restore(bad_phase).unwrap_err().path,
            "gait.phase"
        );

        let mut bad_amp = base;
        bad_amp.amp = 1.25;
        assert_eq!(
            CatGait::prepare_restore(bad_amp).unwrap_err().path,
            "gait.amp"
        );

        let mut bad_point = base;
        bad_point.aim[2].x = f32::INFINITY;
        assert_eq!(
            CatGait::prepare_restore(bad_point).unwrap_err().path,
            "gait.aim[2].x"
        );

        let mut bad_support = base;
        bad_support.support_y = f32::NAN;
        assert_eq!(
            CatGait::prepare_restore(bad_support).unwrap_err().path,
            "gait.support_y"
        );

        let mut bad_planted_height = base;
        bad_planted_height.planted[1].y = 0.5;
        assert_eq!(
            CatGait::prepare_restore(bad_planted_height)
                .unwrap_err()
                .path,
            "gait.planted[1].y"
        );

        let mut bad_aim_height_bits = base;
        bad_aim_height_bits.aim[3].y = -0.0;
        assert_eq!(
            CatGait::prepare_restore(bad_aim_height_bits)
                .unwrap_err()
                .path,
            "gait.aim[3].y"
        );

        let mut bad_swing = base;
        bad_swing.in_swing[0] = true;
        assert_eq!(
            CatGait::prepare_restore(bad_swing).unwrap_err().path,
            "gait.in_swing[0]"
        );
    }

    #[test]
    fn zero_support_delta_preserves_flat_lane_bits() {
        let position = actor_position(Vector3::new(0.25, 0.0, -0.5));
        let yaw = actor_yaw(0.25);
        let mut gait = CatGait::new(position, yaw).unwrap();
        let before = gait.capture();
        let frame = gait
            .advance(duration(DT), position, yaw, speed(0.0))
            .unwrap();
        let after = gait.capture();
        assert_eq!(frame.support_delta_y.to_bits(), 0.0_f32.to_bits());
        assert_eq!(before.support_y.to_bits(), after.support_y.to_bits());
        for index in 0..LEGS {
            for (old, new) in [
                (before.planted[index], after.planted[index]),
                (before.aim[index], after.aim[index]),
            ] {
                assert_eq!(old.x.to_bits(), new.x.to_bits());
                assert_eq!(old.y.to_bits(), new.y.to_bits());
                assert_eq!(old.z.to_bits(), new.z.to_bits());
            }
        }
    }

    #[test]
    fn prepared_restore_rejects_invalid_gait_phase_point_or_support() {
        let mut capture = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0))
            .unwrap()
            .capture();
        capture.phase = 1.0;
        let error = CatGait::prepare_restore(capture).expect_err("phase must be refused");
        assert_eq!(error.path, "gait.phase");
    }
    use crate::pulse_pool::PulsePool;

    const DT: f64 = 1.0 / 60.0;

    /// Walk a straight line at `speed` for `ticks`, collecting frames.
    fn walk_line(speed: f64, ticks: usize) -> Vec<GaitFrame> {
        let mut gait = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0)).unwrap();
        let mut pos = Vector3::ZERO;
        let mut frames = Vec::new();
        for _ in 0..ticks {
            pos += forward(0.0) * ((speed * DT) as f32);
            frames.push(
                gait.advance(
                    duration(DT),
                    actor_position(pos),
                    actor_yaw(0.0),
                    self::speed(speed),
                )
                .unwrap(),
            );
        }
        frames
    }

    /// A planted paw NEVER slides: during stance its world position is
    /// bit-for-bit identical from frame to frame.
    #[test]
    fn planted_paws_never_slide() {
        let frames = walk_line(0.6, 600);
        for pair in frames.windows(2) {
            for leg in 0..LEGS {
                let (a, b) = (pair[0].paws[leg], pair[1].paws[leg]);
                // grounded both frames and no touchdown between them:
                // the same plant, exactly
                if a.y == 0.0 && b.y == 0.0 && !pair[1].contacts.iter().any(|c| c.leg == leg) {
                    assert_eq!(a, b);
                }
            }
        }
    }

    /// The four beats land in lateral-sequence order — LH, LF, RH, RF —
    /// over and over, the walking-cat signature.
    #[test]
    fn footfalls_keep_lateral_sequence_order() {
        let frames = walk_line(0.6, 1800);
        let order: Vec<usize> = frames
            .iter()
            .flat_map(|f| f.contacts.iter().map(|c| c.leg))
            .collect();
        assert!(order.len() >= 8);
        // the lateral sequence, as leg indices (LF 0, RF 1, LH 2, RH 3):
        // ... LH(2), LF(0), RH(3), RF(1) ...
        const SEQ: [usize; LEGS] = [2, 0, 3, 1];
        let start = SEQ.iter().position(|&s| s == order[0]).unwrap();
        for (i, &leg) in order.iter().enumerate() {
            assert_eq!(leg, SEQ[(start + i) % LEGS]);
        }
    }

    /// Each leg touches down once per stride cycle — no double-taps, no
    /// skipped beats — and every touchdown lies exactly on the floor.
    #[test]
    fn one_contact_per_leg_per_cycle() {
        let speed = 0.6;
        let cycles = 8.0;
        let ticks = (cycles * STRIDE_LEN / speed / DT).round() as usize;
        let frames = walk_line(speed, ticks);
        for leg in 0..LEGS {
            let count = frames
                .iter()
                .flat_map(|f| &f.contacts)
                .filter(|c| c.leg == leg)
                .count() as f64;
            assert!((count - cycles).abs() <= 1.0, "leg {leg}: {count} contacts");
        }
        for c in frames.iter().flat_map(|f| &f.contacts) {
            assert_eq!(c.at.y, 0.0);
        }
    }

    /// Paws stay on or above the floor, and the swing arc never exceeds
    /// its tiny lift — a cat barely clears the ground.
    #[test]
    fn paws_stay_low_and_never_dig() {
        for frame in walk_line(0.6, 1200) {
            for paw in frame.paws {
                assert!(paw.y >= 0.0);
                assert!(f64::from(paw.y) <= SWING_LIFT + 1e-9);
            }
        }
    }

    /// The stately walk: two paws grounded at every instant, three for
    /// most of them.
    #[test]
    fn walk_keeps_two_or_three_paws_down() {
        let frames = walk_line(0.6, 1200);
        let settled = &frames[240..]; // past the walk-amp ramp
        let mut three_or_more = 0;
        for frame in settled {
            let down = frame.paws.iter().filter(|p| p.y == 0.0).count();
            assert!(down >= 2, "only {down} paws down");
            if down >= 3 {
                three_or_more += 1;
            }
        }
        assert!(f64::from(three_or_more) / settled.len() as f64 > 0.6);
    }

    /// A standing cat is a silent statue: no contacts, no paw motion.
    #[test]
    fn standing_cat_is_still_and_silent() {
        let position = actor_position(Vector3::new(3.0, 0.0, 2.0));
        let yaw = actor_yaw(1.1);
        let mut gait = CatGait::new(position, yaw).unwrap();
        let first = gait
            .advance(duration(DT), position, yaw, speed(0.0))
            .unwrap();
        assert!(first.contacts.is_empty());
        for _ in 0..120 {
            let frame = gait
                .advance(duration(DT), position, yaw, speed(0.0))
                .unwrap();
            assert!(frame.contacts.is_empty());
            assert_eq!(frame.paws, first.paws);
            assert!(!frame.moving);
        }
    }

    /// Stopping mid-stride: paws caught in the air drop STRAIGHT DOWN to
    /// where they hang, never teleporting forward to their far aim. The
    /// last airborne xz and the settled xz must coincide — the halt is a
    /// small step down, not a lurch across the stride.
    #[test]
    fn stopping_settles_paws_straight_down() {
        let mut gait = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0)).unwrap();
        let mut pos = Vector3::ZERO;
        // walk until at least one paw is mid-swing, remembering the last
        // airborne paw positions
        let mut airborne = None;
        for _ in 0..600 {
            pos += forward(0.0) * ((0.6 * DT) as f32);
            let frame = gait
                .advance(
                    duration(DT),
                    actor_position(pos),
                    actor_yaw(0.0),
                    speed(0.6),
                )
                .unwrap();
            if frame.paws.iter().any(|p| p.y > 0.0) {
                airborne = Some(frame.paws);
                break;
            }
        }
        let last_air = airborne.expect("never lifted a paw");
        // halt: the settle tick grounds every airborne paw at its own xz
        let settle = gait
            .advance(
                duration(DT),
                actor_position(pos),
                actor_yaw(0.0),
                speed(0.0),
            )
            .unwrap();
        assert!(!settle.contacts.is_empty());
        assert!(settle.paws.iter().all(|p| p.y == 0.0));
        for (leg, (before, after)) in last_air.iter().zip(settle.paws.iter()).enumerate() {
            if before.y > 0.0 {
                let jump =
                    f64::from(Vector3::new(after.x - before.x, 0.0, after.z - before.z).length());
                assert!(jump < 0.02, "leg {leg} teleported {jump} m on settle");
            }
        }
        for _ in 0..60 {
            assert!(
                gait.advance(
                    duration(DT),
                    actor_position(pos),
                    actor_yaw(0.0),
                    speed(0.0),
                )
                .unwrap()
                .contacts
                .is_empty()
            );
        }
    }

    /// The walk gate is hysteretic: a body whose measured speed jitters
    /// across the old single threshold — grazing a wall, sliding a
    /// corner — must NOT machine-gun settle contacts. Alternating 0.06 and
    /// 0.04 m/s (astride the retired MOVE_EPS) for two seconds yields a
    /// small handful of contacts, not dozens.
    #[test]
    fn hysteresis_silences_near_threshold_jitter() {
        let mut gait = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0)).unwrap();
        let mut pos = Vector3::ZERO;
        let mut contacts = 0;
        for i in 0..120 {
            let speed = if i % 2 == 0 { 0.06 } else { 0.04 };
            pos += forward(0.0) * ((speed * DT) as f32);
            contacts += gait
                .advance(
                    duration(DT),
                    actor_position(pos),
                    actor_yaw(0.0),
                    self::speed(speed),
                )
                .unwrap()
                .contacts
                .len();
        }
        // both jitter speeds sit inside [MOVE_LO, MOVE_HI], so once the
        // cat is standing it never re-enters the walk — near-total silence
        assert!(contacts <= 4, "jitter machine-gunned {contacts} contacts");
    }

    /// Same inputs, same walk: two gaits fed the same script produce
    /// bit-identical frames — the determinism law, held per platform.
    #[test]
    fn identical_scripts_replay_identically() {
        let script = |gait: &mut CatGait| {
            let mut pos = Vector3::ZERO;
            let mut yaw = 0.0;
            let mut out = Vec::new();
            for i in 0..900 {
                let speed = if i % 300 < 200 { 0.55 } else { 0.0 };
                yaw += 0.4 * DT;
                pos += forward(yaw) * ((speed * DT) as f32);
                out.push(
                    gait.advance(
                        duration(DT),
                        actor_position(pos),
                        actor_yaw(yaw),
                        self::speed(speed),
                    )
                    .unwrap(),
                );
            }
            out
        };
        let mut a = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0)).unwrap();
        let mut b = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0)).unwrap();
        assert_eq!(script(&mut a), script(&mut b));
    }

    /// Walking a circle: every paw stays within its leg's honest reach of
    /// its own shoulder/hip anchor — the swing re-aim law. Without the
    /// per-frame re-aim, turning strands paws far behind the anchors.
    /// Measured worst case at this hard sustained turn: 0.197 m to the
    /// anchor, 0.352 m to the body center; pinned just above.
    #[test]
    fn turning_keeps_paws_under_the_body() {
        let mut gait = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0)).unwrap();
        let mut pos = Vector3::ZERO;
        let mut yaw = 0.0_f64;
        for _ in 0..1800 {
            yaw += 0.9 * DT;
            pos += forward(yaw) * ((0.55 * DT) as f32);
            let frame = gait
                .advance(
                    duration(DT),
                    actor_position(pos),
                    actor_yaw(yaw),
                    speed(0.55),
                )
                .unwrap();
            for (leg, paw) in frame.paws.iter().enumerate() {
                let a = anchor(pos, yaw, leg);
                let da = Vector3::new(paw.x - a.x, 0.0, paw.z - a.z).length();
                assert!(f64::from(da) < 0.22, "leg {leg} strayed {da} m from anchor");
                let dc = Vector3::new(paw.x - pos.x, 0.0, paw.z - pos.z).length();
                assert!(f64::from(dc) < 0.38, "leg {leg} strayed {dc} m from body");
            }
        }
    }

    /// The narrow track: left paws keep left of the heading line, right
    /// paws right — prints that nearly line up, the tightrope walk.
    #[test]
    fn paws_keep_their_side_of_a_narrow_track() {
        let frames = walk_line(0.6, 1200);
        let rv = rightward(0.0);
        for frame in frames {
            for (leg, paw) in frame.paws.iter().enumerate() {
                let lateral = f64::from(paw.x * rv.x + paw.z * rv.z);
                assert!(lateral * SIDE[leg] > 0.0, "leg {leg} crossed the line");
                assert!(lateral.abs() < 0.09);
            }
        }
    }

    /// Only the lead fore paw (LF) speaks; the other three are silent —
    /// the cat's single soft pulse per stride.
    #[test]
    fn only_the_lead_fore_paw_sounds() {
        assert!(paw_sounds(0));
        assert!(!paw_sounds(1));
        assert!(!paw_sounds(2));
        assert!(!paw_sounds(3));
    }

    /// The paw-wave budget, against the REAL pool: a cat walking flat out
    /// at the design envelope for half a minute, then halting (the settle
    /// tick can add the fore contact), keeps its live footstep slots to a
    /// gentle handful — roughly half the fan's 12-slot pin, real headroom
    /// on the 64-slot pool it shares with the hero's footsteps and the
    /// fan hum.
    ///
    /// One fore paw emits per 0.3 m stride and a kind-2 slot lives
    /// 1.3/4.0 + 2.5 = 2.825 s, so the steady-state peak is ~7 (6.1
    /// rounded up). The ceiling is pinned at 8 to leave slack for the
    /// settle transient; a retune that sounds more paws, lengthens the
    /// tail, or quickens the cadence trips it by design — the cat's claim
    /// on the shared pool is a contract, not a coincidence.
    #[test]
    fn paw_waves_stay_within_slot_ceiling() {
        let mut gait = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0)).unwrap();
        let mut pool = PulsePool::new();
        let mut pos = Vector3::ZERO;
        let mut now = 0.0;
        let mut peak = 0;
        for _ in 0..(31.0 / DT) as usize {
            now += DT;
            // walk 30 s at the envelope, then stop dead for the last second
            // so a mid-swing fore paw settles into the busiest window
            let speed = if now < 30.0 { TOP_SPEED } else { 0.0 };
            pos += forward(0.0) * ((speed * DT) as f32);
            let frame = gait
                .advance(
                    duration(DT),
                    actor_position(pos),
                    actor_yaw(0.0),
                    self::speed(speed),
                )
                .unwrap();
            for c in frame.contacts.iter().filter(|c| paw_sounds(c.leg)) {
                pool.emit_omni(2, c.at, PAW_RANGE, PAW_SPEED, PAW_GAIN, now)
                    .unwrap();
            }
            peak = peak.max(pool.live_count(now));
        }
        assert!(peak <= 8, "cat flooded the pool: peak {peak}");
        assert!(peak >= 4, "budget probe too weak: peak only {peak}");
    }

    /// The restored gait walks the SAME stride: drive to mid-stride,
    /// capture, then advance both original and restored with identical
    /// inputs — every GaitFrame must match, planted paws included.
    #[test]
    fn a_restored_gait_walks_the_same_stride() {
        let mut pos = Vector3::ZERO;
        let mut original = CatGait::new(actor_position(pos), actor_yaw(0.0)).unwrap();
        for _ in 0..40 {
            pos += Vector3::new(0.02, 0.0, 0.0);
            let _ = original
                .advance(
                    duration(0.05),
                    actor_position(pos),
                    actor_yaw(0.0),
                    speed(0.4),
                )
                .unwrap();
        }
        let mut restored = CatGait::from_prepared(
            CatGait::prepare_restore(original.capture()).expect("self-capture must restore"),
        );
        assert_eq!(restored, original);
        for _ in 0..100 {
            pos += Vector3::new(0.02, 0.0, 0.0);
            let a = original
                .advance(
                    duration(0.05),
                    actor_position(pos),
                    actor_yaw(0.0),
                    speed(0.4),
                )
                .unwrap();
            let b = restored
                .advance(
                    duration(0.05),
                    actor_position(pos),
                    actor_yaw(0.0),
                    speed(0.4),
                )
                .unwrap();
            assert_eq!(a, b);
        }
    }

    /// The FULL voice against the real pool: a cat walking flat out AND
    /// breathing its idle presence pulse the whole time. Both are kind-2;
    /// together they must still leave the shared 64-slot pool room to
    /// breathe — pinned at 10, comfortably under the pool and under the
    /// pressure a chatty companion would put on the hero's own footsteps.
    #[test]
    fn paw_and_presence_together_stay_within_budget() {
        let mut gait = CatGait::new(actor_position(Vector3::ZERO), actor_yaw(0.0)).unwrap();
        let mut pool = PulsePool::new();
        let mut pos = Vector3::ZERO;
        let mut now = 0.0;
        let mut next_presence = PRESENCE_EVERY;
        let mut peak = 0;
        for _ in 0..(30.0 / DT) as usize {
            now += DT;
            pos += forward(0.0) * ((TOP_SPEED * DT) as f32);
            let frame = gait
                .advance(
                    duration(DT),
                    actor_position(pos),
                    actor_yaw(0.0),
                    speed(TOP_SPEED),
                )
                .unwrap();
            for c in frame.contacts.iter().filter(|c| paw_sounds(c.leg)) {
                pool.emit_omni(2, c.at, PAW_RANGE, PAW_SPEED, PAW_GAIN, now)
                    .unwrap();
            }
            if now >= next_presence {
                next_presence += PRESENCE_EVERY;
                let chest = Vector3::new(pos.x, PRESENCE_HEIGHT as f32, pos.z);
                pool.emit_omni(2, chest, PRESENCE_RANGE, PAW_SPEED, PRESENCE_GAIN, now)
                    .unwrap();
            }
            peak = peak.max(pool.live_count(now));
        }
        assert!(peak <= 10, "cat voice flooded the pool: peak {peak}");
        assert!(peak >= 6, "combined probe too weak: peak only {peak}");
    }
}
