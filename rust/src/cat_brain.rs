//! The cat's mind — a deterministic wanderer. A real cat drifting about a
//! room walks somewhere for its own reasons, pauses, sits a while, moves
//! on; it does not heel, and it does not hurry. This module owns that
//! whimsy as replayable state: every "choice" is a draw from a seeded
//! PCG32 stream, drawn only at discrete events (arrival, a timer
//! expiring, giving up on a blocked path), so the same seed and the same
//! world replay the same life — the crate's determinism law extended to
//! a creature that must look spontaneous.
//!
//! Wander targets snap to a 0.1 m grid, so the point the cat chooses to
//! walk toward never carries float dust. The other decisions — arrival,
//! blocked-give-up, timer expiry — do branch on physics-measured floats
//! (position, displacement), so they are deterministic PER PLATFORM, the
//! v1 contract; a future cross-platform build would quantize those branch
//! inputs too. The continuous outputs (eased speed, rate-limited yaw)
//! stay smooth throughout — snapped choices, fluid motion.
//!
//! The brain knows nothing of scenes or physics. It is handed the body's
//! position and the progress the body ACTUALLY made, and answers with a
//! Drive (speed, yaw, sitting). A cat pressed against a chair leg stops
//! making progress; the brain notices, abandons that ambition, and picks
//! another — no pathfinding in v1, just honest give-up-and-go-elsewhere.

use godot::builtin::{Vector2, Vector3};

use crate::reproduce::RestoreValueError;
use crate::support_motion::{
    ActorPosition, ActorYaw, FiniteMeasure, MAX_ACTOR_COORD_M, MotionValueError, StepDuration,
};

/// The leisurely wander walk, m/s — inside the gait's design envelope.
pub const WANDER_SPEED: f64 = 0.6;

/// Hardest turn, rad/s. Cats swivel comfortably but not instantly.
pub const TURN_RATE: f64 = 2.4;

/// Within this range of the target the cat has arrived.
pub const ARRIVE_R: f64 = 0.22;

/// Approach slowdown radius: speed tapers inside it — no screeching halts.
pub const SLOW_R: f64 = 0.6;

/// Wander targets snap to this grid — decisions never hinge on float dust.
pub const TARGET_GRID: f64 = 0.1;

/// Meters of roam-rect edge the cat leaves untrodden — whiskers' distance
/// from the walls.
pub const WALL_MARGIN: f64 = 0.4;

/// Speed ease rate, 1/s.
pub const SPEED_EASE: f64 = 3.0;

/// Seconds of no progress before the cat abandons a blocked ambition.
pub const BLOCKED_AFTER: f64 = 0.7;

/// A pause between wanders, seconds (min, max).
pub const PAUSE_SECS: (f64, f64) = (1.2, 3.5);

/// A proper sit, seconds (min, max).
pub const SIT_SECS: (f64, f64) = (4.0, 9.0);

/// Melissa O'Neill's PCG32 (XSH-RR) exactly as published — 16 lines of
/// well-studied generator, no dependency. One instance per cat; the
/// stream id keeps two cats with one world seed uncorrelated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    const MUL: u64 = 6_364_136_223_846_793_005;

    /// The reference seeding dance: absorb the stream, step, absorb the
    /// seed, step — pcg_basic.c's `pcg32_srandom_r`, verbatim.
    #[must_use]
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// The raw 32-bit draw — XSH-RR output over the LCG state.
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(Self::MUL).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// A draw in [0, 1).
    pub fn unit(&mut self) -> f64 {
        f64::from(self.next_u32()) / 4_294_967_296.0
    }

    /// A draw in [lo, hi).
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }

    /// The raw stream words, for capture. An advanced PCG32 cannot be
    /// rebuilt from its seed — the draws already taken are gone — so the
    /// capture is the two words themselves.
    #[must_use]
    pub fn capture(&self) -> (u64, u64) {
        (self.state, self.inc)
    }

    /// Rebuild a stream at an exact position, from a capture. Total: any
    /// two words form a valid PCG32 state (inc's low bit being set is a
    /// property `new` guarantees and `capture` preserves).
    #[must_use]
    pub fn restore(state: u64, inc: u64) -> Self {
        Self { state, inc }
    }
}

/// The floor rectangle the cat roams, in world coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoamRect {
    /// West edge.
    min_x: f64,
    /// North edge (smaller z).
    min_z: f64,
    /// East edge.
    max_x: f64,
    /// South edge.
    max_z: f64,
}

/// Raw format-2 image of a roam rectangle. The wire must be able to carry a
/// malformed-but-syntactic artifact through canonical hashing; only the brain
/// owner's prepared restore door narrows these lanes to [`RoamRect`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoamRectCapture {
    pub min_x: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_z: f64,
}

impl RoamRect {
    /// The rect of the given full extents centered on a point — how the
    /// engine node builds it from its own spawn position and a designer
    /// size knob.
    pub fn try_around(center: ActorPosition, size: Vector2) -> Result<Self, MotionValueError> {
        let size_x = checked_roam_size(f64::from(size.x), "roam_rect.size_x")?;
        let size_z = checked_roam_size(f64::from(size.y), "roam_rect.size_z")?;
        let center = center.world();
        Self::try_restore(
            f64::from(center.x) - size_x * 0.5,
            f64::from(center.z) - size_z * 0.5,
            f64::from(center.x) + size_x * 0.5,
            f64::from(center.z) + size_z * 0.5,
        )
    }

    pub fn try_restore(
        min_x: f64,
        min_z: f64,
        max_x: f64,
        max_z: f64,
    ) -> Result<Self, MotionValueError> {
        for (value, field) in [
            (min_x, "roam_rect.min_x"),
            (min_z, "roam_rect.min_z"),
            (max_x, "roam_rect.max_x"),
            (max_z, "roam_rect.max_z"),
        ] {
            if !value.is_finite() {
                return Err(MotionValueError::non_finite(field));
            }
            if value.abs() > f64::from(MAX_ACTOR_COORD_M) {
                return Err(MotionValueError::out_of_range(field));
            }
        }
        if max_x < min_x {
            return Err(MotionValueError::out_of_range("roam_rect.max_x"));
        }
        if max_z < min_z {
            return Err(MotionValueError::out_of_range("roam_rect.max_z"));
        }
        checked_roam_size(max_x - min_x, "roam_rect.size_x")?;
        checked_roam_size(max_z - min_z, "roam_rect.size_z")?;
        Ok(Self {
            min_x,
            min_z,
            max_x,
            max_z,
        })
    }

    #[must_use]
    pub fn min_x(self) -> f64 {
        self.min_x
    }

    #[must_use]
    pub fn min_z(self) -> f64 {
        self.min_z
    }

    #[must_use]
    pub fn max_x(self) -> f64 {
        self.max_x
    }

    #[must_use]
    pub fn max_z(self) -> f64 {
        self.max_z
    }

    #[must_use]
    pub fn contains_target(self, tx: f64, tz: f64) -> bool {
        tx.is_finite()
            && tz.is_finite()
            && tx.abs() <= f64::from(MAX_ACTOR_COORD_M)
            && tz.abs() <= f64::from(MAX_ACTOR_COORD_M)
            && (self.min_x..=self.max_x).contains(&tx)
            && (self.min_z..=self.max_z).contains(&tz)
    }

    #[must_use]
    fn capture(self) -> RoamRectCapture {
        RoamRectCapture {
            min_x: self.min_x,
            min_z: self.min_z,
            max_x: self.max_x,
            max_z: self.max_z,
        }
    }
}

fn checked_roam_size(value: f64, field: &'static str) -> Result<f64, MotionValueError> {
    if !value.is_finite() {
        return Err(MotionValueError::non_finite(field));
    }
    if !(1.0..=30.0).contains(&value) {
        return Err(MotionValueError::out_of_range(field));
    }
    Ok(value)
}

/// What the cat is at right now — the observable face of the state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    /// Walking somewhere it chose.
    Roam,
    /// Standing, taking the room in.
    Pause,
    /// Sitting properly, staying a while.
    Sit,
}

/// One tick's marching orders for the body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drive {
    /// Commanded planar speed, m/s — already eased.
    pub speed: FiniteMeasure,
    /// Commanded heading, radians — already rate-limited.
    pub yaw: ActorYaw,
    /// Whether the cat means to be sitting.
    pub sitting: bool,
}

/// The internal state, with its data.
#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Roam { tx: f64, tz: f64 },
    Pause { left: f64 },
    Sit { left: f64 },
}

/// The private state machine's public mirror, for capture. One variant
/// per State, payloads included — the Pause/Sit countdown is exactly the
/// state a restored cat resumes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrainState {
    Roam { tx: f64, tz: f64 },
    Pause { left: f64 },
    Sit { left: f64 },
}

/// Everything a CatBrain is, as data. Same-build, same-platform contract
/// as the brain itself (the module's determinism doc).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrainCapture {
    pub rng_state: u64,
    pub rng_inc: u64,
    pub rect: RoamRectCapture,
    pub state: BrainState,
    pub yaw: f64,
    pub speed: f64,
    pub blocked: f64,
}

/// The wanderer. Advanced every physics tick with the body's position
/// and actual progress; all whimsy comes from the seeded stream, drawn
/// only at events.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CatBrain {
    rng: Pcg32,
    rect: RoamRect,
    state: State,
    yaw: f64,
    speed: f64,
    blocked: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct PreparedCatBrain(CatBrain);

impl CatBrain {
    pub fn prepare_restore(capture: BrainCapture) -> Result<PreparedCatBrain, RestoreValueError> {
        let raw_rect = capture.rect;
        let rect = RoamRect::try_restore(
            raw_rect.min_x,
            raw_rect.min_z,
            raw_rect.max_x,
            raw_rect.max_z,
        )
        .map_err(|error| {
            let field = error
                .field()
                .strip_prefix("roam_rect.")
                .unwrap_or(error.field());
            RestoreValueError::new(
                format!("brain.rect.{field}"),
                "must be a finite ordered 1..=30 m rectangle inside actor bounds",
            )
        })?;
        for (field, value) in [
            ("yaw", capture.yaw),
            ("speed", capture.speed),
            ("blocked", capture.blocked),
        ] {
            if !value.is_finite() {
                return Err(RestoreValueError::new(
                    format!("brain.{field}"),
                    "must be finite",
                ));
            }
        }
        ActorYaw::try_new(capture.yaw).map_err(|_| {
            RestoreValueError::new("brain.yaw", "must narrow to a finite Godot rotation lane")
        })?;
        if capture.rng_inc & 1 == 0 {
            return Err(RestoreValueError::new(
                "brain.rng_inc",
                "must be an odd PCG stream word",
            ));
        }
        match capture.state {
            BrainState::Roam { tx, tz } => {
                for (field, target) in [("state.tx", tx), ("state.tz", tz)] {
                    if !target.is_finite() {
                        return Err(RestoreValueError::new(
                            format!("brain.{field}"),
                            "must be finite",
                        ));
                    }
                }
                if !rect.contains_target(tx, tz) {
                    let field = if tx.abs() > f64::from(MAX_ACTOR_COORD_M)
                        || !(rect.min_x..=rect.max_x).contains(&tx)
                    {
                        "brain.state.tx"
                    } else {
                        "brain.state.tz"
                    };
                    return Err(RestoreValueError::new(
                        field,
                        "must lie inside the raw roam rectangle and actor bounds",
                    ));
                }
            }
            BrainState::Pause { left } => {
                if !left.is_finite() {
                    return Err(RestoreValueError::new("brain.state.left", "must be finite"));
                }
                if left <= 0.0 || left > PAUSE_SECS.1 {
                    return Err(RestoreValueError::new(
                        "brain.state.left",
                        "is outside its valid countdown range",
                    ));
                }
            }
            BrainState::Sit { left } => {
                if !left.is_finite() {
                    return Err(RestoreValueError::new("brain.state.left", "must be finite"));
                }
                if left <= 0.0 || left > SIT_SECS.1 {
                    return Err(RestoreValueError::new(
                        "brain.state.left",
                        "is outside its valid countdown range",
                    ));
                }
            }
        }
        if capture.speed < 0.0 || capture.speed > WANDER_SPEED {
            return Err(RestoreValueError::new(
                "brain.speed",
                "must be in 0..=WANDER_SPEED",
            ));
        }
        if capture.blocked < 0.0 || capture.blocked > BLOCKED_AFTER {
            return Err(RestoreValueError::new(
                "brain.blocked",
                "must be in 0..=BLOCKED_AFTER",
            ));
        }
        Ok(PreparedCatBrain(Self::restore(capture, rect)))
    }

    #[must_use]
    pub fn from_prepared(capture: PreparedCatBrain) -> Self {
        capture.0
    }

    /// A fresh mind: the cat wakes facing `yaw`, takes a short breath
    /// (a fixed first pause — no draw, so seed streams start aligned at
    /// the first real choice), then begins to wander `rect`.
    #[must_use]
    pub fn new(seed: u64, rect: RoamRect, yaw: ActorYaw) -> Self {
        Self {
            rng: Pcg32::new(seed, 0xCA7),
            rect,
            state: State::Pause { left: 0.8 },
            yaw: yaw.radians(),
            speed: 0.0,
            blocked: 0.0,
        }
    }

    /// The current mood — observable for suites and future ambient logic.
    #[must_use]
    pub fn mood(&self) -> Mood {
        match self.state {
            State::Roam { .. } => Mood::Roam,
            State::Pause { .. } => Mood::Pause,
            State::Sit { .. } => Mood::Sit,
        }
    }

    /// The current wander target, when there is one — observable.
    #[must_use]
    pub fn target(&self) -> Option<(f64, f64)> {
        match self.state {
            State::Roam { tx, tz } => Some((tx, tz)),
            _ => None,
        }
    }

    /// One tick of mind. `pos` is the body's position, `progress` the
    /// planar meters it ACTUALLY moved since the last tick — feed the
    /// measurement, not the wish, so blocked bodies read as blocked.
    pub fn advance(
        &mut self,
        dt: StepDuration,
        pos: ActorPosition,
        progress: FiniteMeasure,
    ) -> Result<Drive, MotionValueError> {
        let mut next = *self;
        let drive = next.advance_checked(dt.seconds(), pos.world(), progress.value())?;
        *self = next;
        Ok(drive)
    }

    fn advance_checked(
        &mut self,
        dt: f64,
        pos: Vector3,
        progress: f64,
    ) -> Result<Drive, MotionValueError> {
        match self.state {
            State::Pause { left } | State::Sit { left } => {
                self.speed += (0.0 - self.speed) * (dt * SPEED_EASE).min(1.0);
                let left = left - dt;
                if left <= 0.0 {
                    let (tx, tz) = self.pick_target();
                    self.state = State::Roam { tx, tz };
                } else {
                    self.state = match self.state {
                        State::Sit { .. } => State::Sit { left },
                        _ => State::Pause { left },
                    };
                }
            }
            State::Roam { tx, tz } => self.roam(dt, pos, progress, tx, tz),
        }
        Ok(Drive {
            speed: FiniteMeasure::try_new(self.speed, "cat_brain.speed")?,
            yaw: ActorYaw::try_new(self.yaw)?,
            sitting: matches!(self.state, State::Sit { .. }),
        })
    }

    /// The walking mind: steer toward the target, slow into arrivals and
    /// out of sharp turns, notice being stuck, and choose what comes next
    /// when the spot is reached.
    fn roam(&mut self, dt: f64, pos: Vector3, progress: f64, tx: f64, tz: f64) {
        let dx = tx - f64::from(pos.x);
        let dz = tz - f64::from(pos.z);
        let dist = (dx * dx + dz * dz).sqrt();
        if dist < ARRIVE_R {
            self.blocked = 0.0;
            self.state = self.next_whim();
            return;
        }
        let desired = (-dx).atan2(-dz);
        let err = wrap_pi(desired - self.yaw);
        self.yaw += err.clamp(-TURN_RATE * dt, TURN_RATE * dt);
        // slow into the spot, and slow through a sharp turn — a cat
        // wheels around, it does not drift like a cart
        let arrive = (dist / SLOW_R).min(1.0);
        let facing = (1.25 - err.abs()).clamp(0.25, 1.0);
        let want = WANDER_SPEED * arrive * facing;
        self.speed += (want - self.speed) * (dt * SPEED_EASE).min(1.0);
        // no progress while honestly trying: something is in the way;
        // cats do not push — they lose interest
        if self.speed > 0.15 && progress < 0.35 * self.speed * dt {
            self.blocked += dt;
        } else {
            self.blocked = 0.0;
        }
        if self.blocked >= BLOCKED_AFTER {
            self.blocked = 0.0;
            let (tx, tz) = self.pick_target();
            self.state = State::Roam { tx, tz };
        }
    }

    /// Arrived: what now? Mostly onward, often a pause, sometimes a
    /// proper sit — the draw order is fixed, so replays agree.
    fn next_whim(&mut self) -> State {
        let whim = self.rng.unit();
        if whim < 0.45 {
            let (tx, tz) = self.pick_target();
            State::Roam { tx, tz }
        } else if whim < 0.80 {
            State::Pause {
                left: self.rng.range(PAUSE_SECS.0, PAUSE_SECS.1),
            }
        } else {
            State::Sit {
                left: self.rng.range(SIT_SECS.0, SIT_SECS.1),
            }
        }
    }

    /// The whole brain as data — the RNG words included, or the restored
    /// cat diverges at its first whim.
    #[must_use]
    pub fn capture(&self) -> BrainCapture {
        let (rng_state, rng_inc) = self.rng.capture();
        BrainCapture {
            rng_state,
            rng_inc,
            rect: self.rect.capture(),
            state: match self.state {
                State::Roam { tx, tz } => BrainState::Roam { tx, tz },
                State::Pause { left } => BrainState::Pause { left },
                State::Sit { left } => BrainState::Sit { left },
            },
            yaw: self.yaw,
            speed: self.speed,
            blocked: self.blocked,
        }
    }

    /// A brain rebuilt mid-life — the one thing `new` cannot express (it
    /// hard-codes the first pause and a fresh stream).
    fn restore(capture: BrainCapture, rect: RoamRect) -> Self {
        Self {
            rng: Pcg32::restore(capture.rng_state, capture.rng_inc),
            rect,
            state: match capture.state {
                BrainState::Roam { tx, tz } => State::Roam { tx, tz },
                BrainState::Pause { left } => State::Pause { left },
                BrainState::Sit { left } => State::Sit { left },
            },
            yaw: capture.yaw,
            speed: capture.speed,
            blocked: capture.blocked,
        }
    }

    /// A fresh ambition: a grid-snapped point inside the rect, margin
    /// respected. A rect too small for the margin collapses to its
    /// center — total, never panicking.
    fn pick_target(&mut self) -> (f64, f64) {
        let lo_x = self.rect.min_x + WALL_MARGIN;
        let hi_x = self.rect.max_x - WALL_MARGIN;
        let lo_z = self.rect.min_z + WALL_MARGIN;
        let hi_z = self.rect.max_z - WALL_MARGIN;
        if lo_x >= hi_x || lo_z >= hi_z {
            return (
                quantize((self.rect.min_x + self.rect.max_x) * 0.5),
                quantize((self.rect.min_z + self.rect.max_z) * 0.5),
            );
        }
        (
            quantize(self.rng.range(lo_x, hi_x)),
            quantize(self.rng.range(lo_z, hi_z)),
        )
    }
}

/// Snap to the target grid: choices sit on 0.1 m marks, never on float
/// dust.
fn quantize(v: f64) -> f64 {
    (v / TARGET_GRID).round() * TARGET_GRID
}

/// Wrap an angle into (-PI, PI].
fn wrap_pi(a: f64) -> f64 {
    let tau = std::f64::consts::TAU;
    let wrapped = (a + std::f64::consts::PI).rem_euclid(tau);
    wrapped - std::f64::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    use godot::builtin::Vector2;

    use crate::support_motion::{ActorPosition, ActorYaw, FiniteMeasure, StepDuration};

    fn actor_position(world: Vector3) -> ActorPosition {
        ActorPosition::try_new(world).expect("test position must be in the actor domain")
    }

    fn actor_yaw(radians: f64) -> ActorYaw {
        ActorYaw::try_new(radians).expect("test yaw must be in the actor domain")
    }

    fn progress(meters: f64) -> FiniteMeasure {
        FiniteMeasure::try_new(meters, "test.progress")
            .expect("test progress must be finite and non-negative")
    }

    fn duration(seconds: f64) -> StepDuration {
        StepDuration::from_raw(seconds)
    }

    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "999_985.0625 is the hand-derived adjacent f32 centre, exactly one ULP above the safe centre"
    )]
    fn roam_rect_accepts_last_exact_safe_edge_and_rejects_adjacent_excess() {
        for (center, expected_edge, expected_field) in [
            (
                Vector3::new(999_985.0, 0.0, 0.0),
                1_000_000.0_f64,
                "roam_rect.max_x",
            ),
            (
                Vector3::new(-999_985.0, 0.0, 0.0),
                -1_000_000.0_f64,
                "roam_rect.min_x",
            ),
            (
                Vector3::new(0.0, 0.0, 999_985.0),
                1_000_000.0_f64,
                "roam_rect.max_z",
            ),
            (
                Vector3::new(0.0, 0.0, -999_985.0),
                -1_000_000.0_f64,
                "roam_rect.min_z",
            ),
        ] {
            let rect = RoamRect::try_around(actor_position(center), Vector2::new(30.0, 30.0))
                .expect("the last centre whose rect meets the actor bound must pass");
            let edge = match expected_field {
                "roam_rect.min_x" => rect.min_x(),
                "roam_rect.min_z" => rect.min_z(),
                "roam_rect.max_z" => rect.max_z(),
                _ => rect.max_x(),
            };
            assert_eq!(edge.to_bits(), expected_edge.to_bits());
        }

        for (center, expected_field) in [
            (Vector3::new(999_985.062_5, 0.0, 0.0), "roam_rect.max_x"),
            (Vector3::new(-999_985.062_5, 0.0, 0.0), "roam_rect.min_x"),
            (Vector3::new(0.0, 0.0, 999_985.062_5), "roam_rect.max_z"),
            (Vector3::new(0.0, 0.0, -999_985.062_5), "roam_rect.min_z"),
        ] {
            let error = RoamRect::try_around(actor_position(center), Vector2::new(30.0, 30.0))
                .expect_err("the adjacent centre puts one rect edge outside actor space");
            assert_eq!(error.field(), expected_field);
        }
    }

    #[test]
    fn rounded_brain_target_round_trips_after_margin_sampling() {
        let center = actor_position(Vector3::new(0.08, 0.0, 0.0));
        let rect = RoamRect::try_around(center, Vector2::new(1.0, 1.0)).unwrap();
        let yaw = actor_yaw(0.0);
        let mut brain = CatBrain::new(5, rect, yaw);
        for _ in 0..13 {
            brain
                .advance(duration(1.0 / 15.0), center, progress(0.0))
                .expect("valid brain inputs must produce a drive");
        }
        let capture = brain.capture();
        let BrainState::Roam { tx, tz } = capture.state else {
            panic!("seed 5 must have left the initial pause")
        };
        assert_eq!(tx.to_bits(), 0.2_f64.to_bits());
        assert!(rect.contains_target(tx, tz));
        let restored = CatBrain::from_prepared(
            CatBrain::prepare_restore(capture).expect("self-produced rounded target must restore"),
        );
        assert_eq!(restored.capture(), capture);
    }

    #[test]
    fn brain_restore_rejects_out_of_envelope_rect_or_target() {
        let outside = 1_000_000.25_f64;
        assert_eq!(
            RoamRect::try_restore(-1.0, -1.0, outside, 1.0)
                .unwrap_err()
                .field(),
            "roam_rect.max_x"
        );
        assert_eq!(
            RoamRect::try_restore(0.041, -0.5, 0.049, 0.5)
                .unwrap_err()
                .field(),
            "roam_rect.size_x"
        );
        assert_eq!(
            RoamRect::try_restore(-15.000_001, -0.5, 15.000_001, 0.5)
                .unwrap_err()
                .field(),
            "roam_rect.size_x"
        );
        for (min_x, min_z, max_x, max_z, expected_field) in [
            (-0.5, 0.041, 0.5, 0.049, "roam_rect.size_z"),
            (-0.5, -15.000_001, 0.5, 15.000_001, "roam_rect.size_z"),
            (0.5, -0.5, -0.5, 0.5, "roam_rect.max_x"),
            (-0.5, 0.5, 0.5, -0.5, "roam_rect.max_z"),
        ] {
            assert_eq!(
                RoamRect::try_restore(min_x, min_z, max_x, max_z)
                    .unwrap_err()
                    .field(),
                expected_field
            );
        }

        let rect = RoamRect::try_restore(-1.0, -1.0, 1.0, 1.0).unwrap();
        for (tx, tz) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            let mut edge = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
            edge.state = BrainState::Roam { tx, tz };
            CatBrain::prepare_restore(edge).expect("either raw rectangle edge must restore");
        }
        let mut capture = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
        capture.state = BrainState::Roam {
            tx: 1.000_000_000_000_000_2,
            tz: 0.0,
        };
        let error = CatBrain::prepare_restore(capture)
            .expect_err("a target outside the raw rect must be refused");
        assert_eq!(error.path, "brain.state.tx");
    }

    #[test]
    fn poisoned_brain_capture_refuses_checked_restore() {
        let rect = RoamRect::try_restore(-2.0, -3.0, 2.0, 3.0).unwrap();
        let mut capture = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
        capture.speed = f64::NAN;
        assert_eq!(
            CatBrain::prepare_restore(capture).unwrap_err().path,
            "brain.speed"
        );

        let mut capture = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
        capture.blocked = -0.25;
        assert_eq!(
            CatBrain::prepare_restore(capture).unwrap_err().path,
            "brain.blocked"
        );
    }

    const DT: f64 = 1.0 / 60.0;

    #[test]
    fn prepared_restore_rejects_invalid_brain_rect_target_or_rng() {
        let rect =
            RoamRect::try_restore(-2.0, -3.0, 2.0, 3.0).expect("test rectangle must be valid");
        let mut capture = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
        capture.rng_inc &= !1;
        let error = CatBrain::prepare_restore(capture).expect_err("even stream must be refused");
        assert_eq!(error.path, "brain.rng_inc");

        let mut capture = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
        capture.state = BrainState::Pause { left: 8.0 };
        let error = CatBrain::prepare_restore(capture)
            .expect_err("a pause longer than the pause author's ceiling must be refused");
        assert_eq!(error.path, "brain.state.left");

        let mut capture = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
        capture.rect.max_x = f64::MAX;
        let error = CatBrain::prepare_restore(capture)
            .expect_err("a roam edge outside actor space must be refused");
        assert_eq!(error.path, "brain.rect.max_x");
    }

    #[test]
    fn prepared_restore_rejects_yaw_that_cannot_reach_a_finite_godot_lane() {
        let rect = RoamRect::try_around(actor_position(Vector3::ZERO), Vector2::new(6.0, 6.0))
            .expect("test rectangle must be valid");
        let mut poisoned = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
        poisoned.yaw = f64::MAX;
        let error = CatBrain::prepare_restore(poisoned)
            .expect_err("a yaw that narrows to infinity must be refused");
        assert_eq!(error.path, "brain.yaw");

        for boundary in [f64::from(f32::MAX), -f64::from(f32::MAX)] {
            let mut accepted = CatBrain::new(7, rect, actor_yaw(0.0)).capture();
            accepted.yaw = boundary;
            let restored = CatBrain::from_prepared(
                CatBrain::prepare_restore(accepted).expect("finite f32 boundary must pass"),
            );
            assert_eq!(restored.capture().yaw.to_bits(), boundary.to_bits());
            assert!((restored.capture().yaw as f32).is_finite());
        }
    }

    /// The published pcg_basic.c reference stream: srandom(42, 54) must
    /// yield exactly these first six draws — the implementation is the
    /// paper's, bit for bit.
    #[test]
    fn pcg32_matches_the_reference_stream() {
        let mut rng = Pcg32::new(42, 54);
        let got: Vec<u32> = (0..6).map(|_| rng.next_u32()).collect();
        assert_eq!(
            got,
            vec![
                0xa15c_02b7,
                0x7b47_f409,
                0xba1d_3330,
                0x83d2_f293,
                0xbfa4_784b,
                0xcbed_606e
            ]
        );
    }

    /// unit() stays in [0, 1); range() stays in its bounds.
    #[test]
    fn draws_stay_in_bounds() {
        let mut rng = Pcg32::new(7, 0xCA7);
        for _ in 0..10_000 {
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u));
            let r = rng.range(-3.0, 5.0);
            assert!((-3.0..5.0).contains(&r));
        }
    }

    /// A captured stream, restored, continues EXACTLY where the original
    /// would have — the property every cat restore rests on. Literals are
    /// the module's own pinned reference stream (srandom(42, 54)), not
    /// values read back from the code under test.
    #[test]
    fn a_restored_stream_continues_where_the_original_left_off() {
        let mut original = Pcg32::new(42, 54);
        let _ = original.next_u32(); // 0xa15c02b7, per the pinned stream
        let (state, inc) = original.capture();
        let mut restored = Pcg32::restore(state, inc);
        // both must produce the identical next five draws
        for _ in 0..5 {
            assert_eq!(restored.next_u32(), original.clone().next_u32());
            let _ = original.next_u32();
        }
    }

    /// A pure integrator: the brain drives a point body with no walls.
    struct Sim {
        brain: CatBrain,
        pos: Vector3,
        last: Vector3,
    }

    impl Sim {
        fn new(seed: u64) -> Self {
            let rect = RoamRect::try_around(
                actor_position(Vector3::new(3.0, 0.0, 3.0)),
                Vector2::new(6.0, 6.0),
            )
            .expect("test rectangle must be valid");
            Self {
                brain: CatBrain::new(seed, rect, actor_yaw(0.0)),
                pos: Vector3::new(3.0, 0.0, 3.0),
                last: Vector3::new(3.0, 0.0, 3.0),
            }
        }

        fn step(&mut self) -> Drive {
            let measured_progress = f64::from((self.pos - self.last).length());
            self.last = self.pos;
            let drive = self
                .brain
                .advance(
                    duration(DT),
                    actor_position(self.pos),
                    progress(measured_progress),
                )
                .expect("valid simulation inputs must produce a drive");
            let yaw = drive.yaw.radians();
            let fw = Vector3::new((-yaw.sin()) as f32, 0.0, (-yaw.cos()) as f32);
            self.pos += fw * ((drive.speed.value() * DT) as f32);
            drive
        }
    }

    /// Over five simulated minutes the cat visits several distinct
    /// targets, and every one is grid-snapped inside the margined rect.
    #[test]
    fn targets_are_snapped_and_stay_inside_the_rect() {
        let mut sim = Sim::new(7);
        let mut targets: Vec<(f64, f64)> = Vec::new();
        for _ in 0..(300.0 / DT) as usize {
            sim.step();
            if let Some(t) = sim.brain.target()
                && targets.last() != Some(&t)
            {
                targets.push(t);
            }
        }
        assert!(targets.len() >= 5, "only {} targets", targets.len());
        for (tx, tz) in targets {
            // rect spans [0, 6] on both axes; the margin trims it to
            // [0.4, 5.6]. Grid-snapping may overshoot the bound by float
            // dust (56 * 0.1 lands one ULP past the literal 5.6), never
            // by a grid cell — the draw range is half-open.
            assert!((0.4 - 1e-9..=5.6 + 1e-9).contains(&tx));
            assert!((0.4 - 1e-9..=5.6 + 1e-9).contains(&tz));
            assert!((tx / TARGET_GRID - (tx / TARGET_GRID).round()).abs() < 1e-9);
            assert!((tz / TARGET_GRID - (tz / TARGET_GRID).round()).abs() < 1e-9);
        }
    }

    /// The whole scripted life replays bit-identically from one seed —
    /// and a different seed lives a different life.
    #[test]
    fn same_seed_same_life_different_seed_different_life() {
        let run = |seed: u64| -> Vec<Drive> {
            let mut sim = Sim::new(seed);
            (0..(120.0 / DT) as usize).map(|_| sim.step()).collect()
        };
        assert_eq!(run(7), run(7));
        assert_ne!(run(7), run(8));
    }

    /// The cat truly wanders: real distance covered, and both a pause and
    /// a proper sit happen within five minutes of seed 7's life.
    #[test]
    fn the_cat_wanders_pauses_and_sits() {
        let mut sim = Sim::new(7);
        let mut walked = 0.0;
        let mut sat = false;
        let mut paused_after_start = false;
        let mut last = sim.pos;
        for i in 0..(300.0 / DT) as usize {
            sim.step();
            walked += f64::from((sim.pos - last).length());
            last = sim.pos;
            match sim.brain.mood() {
                Mood::Sit => sat = true,
                Mood::Pause if i > (5.0 / DT) as usize => paused_after_start = true,
                _ => {}
            }
        }
        assert!(walked > 10.0, "walked only {walked} m");
        assert!(sat, "never sat");
        assert!(paused_after_start, "never paused");
    }

    /// The yaw never jumps: consecutive commands differ by at most the
    /// turn rate — the wheel-around is smooth, not teleported.
    #[test]
    fn turning_is_rate_limited() {
        let mut sim = Sim::new(11);
        let mut prev = sim.step().yaw.radians();
        for _ in 0..(120.0 / DT) as usize {
            let yaw = sim.step().yaw.radians();
            assert!((yaw - prev).abs() <= TURN_RATE * DT + 1e-12);
            prev = yaw;
        }
    }

    /// Commanded speed honors the gait's design envelope at every tick.
    #[test]
    #[expect(
        clippy::assertions_on_constants,
        reason = "the point IS the constant: the brain's wander speed must \
                  stay inside the gait's paw-wave budget envelope, and this \
                  pin guards future retuning of either knob"
    )]
    fn speed_stays_inside_the_gait_envelope() {
        assert!(WANDER_SPEED < crate::cat_gait::TOP_SPEED);
        let mut sim = Sim::new(5);
        for _ in 0..(120.0 / DT) as usize {
            let drive = sim.step();
            assert!(drive.speed.value() <= WANDER_SPEED + 1e-9);
            assert!(drive.speed.value() >= 0.0);
        }
    }

    /// A cat pressed against furniture makes no progress and, within a
    /// couple of seconds, abandons that ambition for a fresh target.
    #[test]
    fn a_blocked_cat_loses_interest() {
        let position = actor_position(Vector3::ZERO);
        let rect = RoamRect::try_around(position, Vector2::new(8.0, 8.0))
            .expect("test rectangle must be valid");
        let mut brain = CatBrain::new(3, rect, actor_yaw(0.0));
        // wake through the initial pause into a first target
        let mut first = None;
        for _ in 0..(2.0 / DT) as usize {
            brain
                .advance(duration(DT), position, progress(0.0))
                .expect("valid brain inputs must produce a drive");
            if let Some(t) = brain.target() {
                first = Some(t);
                break;
            }
        }
        let first = first.expect("never started roaming");
        // the body never moves (progress 0): interest must move instead
        let mut repicked = false;
        for _ in 0..(3.0 / DT) as usize {
            brain
                .advance(duration(DT), position, progress(0.0))
                .expect("valid brain inputs must produce a drive");
            if brain.target().is_some_and(|t| t != first) {
                repicked = true;
                break;
            }
        }
        assert!(repicked, "still pushing at the same blocked target");
    }

    /// A subminimum rectangle is refused at construction, while the smallest
    /// valid rectangle still produces a target inside its raw extent.
    #[test]
    fn subminimum_rect_is_rejected_and_smallest_valid_rect_stays_inside() {
        let position = actor_position(Vector3::new(1.0, 0.0, 2.0));
        let error = RoamRect::try_around(position, Vector2::new(0.5, 0.5))
            .expect_err("a roam rectangle smaller than one meter must be refused");
        assert_eq!(error.field(), "roam_rect.size_x");

        let rect = RoamRect::try_around(position, Vector2::new(1.0, 1.0))
            .expect("the one-meter lower boundary must be accepted");
        let mut brain = CatBrain::new(1, rect, actor_yaw(0.0));
        for _ in 0..(5.0 / DT) as usize {
            brain
                .advance(duration(DT), position, progress(0.0))
                .expect("valid brain inputs must produce a drive");
        }
        if let Some((tx, tz)) = brain.target() {
            assert!(rect.contains_target(tx, tz));
        }
    }

    /// The restored brain lives the SAME future: drive a brain to
    /// mid-life, capture, then advance both original and restored with
    /// identical inputs — every Drive must match. This is the
    /// same-seed-same-life law, lifted to same-capture-same-future.
    #[test]
    fn a_restored_brain_lives_the_same_future() {
        let rect = RoamRect::try_around(actor_position(Vector3::ZERO), Vector2::new(6.0, 6.0))
            .expect("test rectangle must be valid");
        let mut original = CatBrain::new(7, rect, actor_yaw(0.3));
        let mut pos = Vector3::ZERO;
        for _ in 0..120 {
            let drive = original
                .advance(duration(0.1), actor_position(pos), progress(0.05))
                .expect("valid brain inputs must produce a drive");
            pos += Vector3::new(0.05, 0.0, 0.02) * (drive.speed.value() as f32);
        }
        let capture = original.capture();
        let mut restored = CatBrain::from_prepared(
            CatBrain::prepare_restore(capture).expect("self-produced capture must restore"),
        );
        assert_eq!(restored, original); // PartialEq on the whole brain
        for _ in 0..200 {
            let a = original
                .advance(duration(0.1), actor_position(pos), progress(0.04))
                .expect("valid brain inputs must produce a drive");
            let b = restored
                .advance(duration(0.1), actor_position(pos), progress(0.04))
                .expect("valid brain inputs must produce a drive");
            assert_eq!(a.speed, b.speed);
            assert_eq!(a.yaw, b.yaw);
            assert_eq!(a.sitting, b.sitting);
            pos += Vector3::new(0.03, 0.0, 0.01);
        }
    }
}
