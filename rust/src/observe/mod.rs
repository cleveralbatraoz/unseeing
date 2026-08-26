//! Debug observability — the wave engine described to an agent as data.
//!
//! Four verbs, per `docs/superpowers/specs/2026-08-10-debug-observability-design.md`:
//! SNAPSHOT (state now), DIFF (the caller's job — sample and compare),
//! EXPLAIN (pure re-computations that answer "why"), and DIGEST (the pixel
//! reduction, Plan 2).
//!
//! Everything here is pure and engine-free. The boundary that hands these
//! results to Godot is `crate::nodes::observer`.

pub mod evict;
pub mod oids;
pub mod pool;
pub mod ray;
pub mod reflect;

use std::num::NonZeroU64;

use godot::builtin::{Basis, Vector3, Vector4};

use self::evict::{EvictionPlan, explain_eviction};
use self::pool::{SlotObservation, SlotState, slots};
use crate::echo_queue::EchoQueue;
use crate::pulse_pool::PulsePool;
use crate::sight::MAXW;
use crate::support_motion::{
    ActorPosition, ActorVelocity, LandingEvent, MotionPhase, MotionState, MotionValueError,
    QueuedWaveGate, SupportContact,
};

/// One actor's motion as an agent reads it: the phase/support/landing
/// projected from a single private [`MotionState`], plus the physical
/// velocity the engine actually measured and the transient engine collider
/// identity behind its accepted support — never captured, per the campaign
/// ruling that a restorable blob must never carry a host-instance id.
///
/// `try_new` reads its three inputs exactly once and either builds a
/// wholly valid observation or refuses: a poisoned velocity, a zero
/// collider id, or an identity reported for a phase that carries no
/// accepted support are all impossible states this type does not let a
/// caller construct.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorMotionObservation {
    state: MotionState,
    actual_velocity_mps: ActorVelocity,
    support_collider_id: Option<NonZeroU64>,
}

impl ActorMotionObservation {
    pub fn try_new(
        state: MotionState,
        raw_velocity_mps: Vector3,
        support_collider_id: Option<u64>,
    ) -> Result<Self, MotionValueError> {
        let actual_velocity_mps = ActorVelocity::try_new(raw_velocity_mps)?;
        let support_collider_id = match support_collider_id {
            None => None,
            Some(raw) => Some(
                NonZeroU64::new(raw)
                    .ok_or_else(|| MotionValueError::out_of_range("support_collider_id"))?,
            ),
        };
        if support_collider_id.is_some() && state.support().is_none() {
            return Err(MotionValueError::inconsistent_state("support_collider_id"));
        }
        Ok(Self {
            state,
            actual_velocity_mps,
            support_collider_id,
        })
    }

    pub fn phase(self) -> MotionPhase {
        self.state.phase()
    }

    pub fn actual_velocity(self) -> ActorVelocity {
        self.actual_velocity_mps
    }

    pub fn support(self) -> Option<SupportContact> {
        self.state.support()
    }

    /// `None` means unavailable identity — either no support at all, or a
    /// server-backed support with no Object behind it — and can never be
    /// confused with an invalid zero id, which [`Self::try_new`] refuses
    /// outright rather than storing.
    pub fn support_collider_id(self) -> Option<u64> {
        self.support_collider_id.map(NonZeroU64::get)
    }

    pub fn last_landing(self) -> Option<LandingEvent> {
        self.state.last_landing()
    }
}

/// One cat's motion, named and placed — the census entry
/// [`cat_motion_observations`] builds, and [`FrameObservation::cat_motion`]
/// carries in the exact order the level's recursive census supplies.
#[derive(Debug, Clone, PartialEq)]
pub struct CatMotionObservation {
    pub name: String,
    pub position: ActorPosition,
    pub motion: ActorMotionObservation,
}

/// One cat's raw, not-yet-checked motion facts, in the shape the boundary
/// reads them off a node before [`cat_motion_observations`] checks them:
/// name, world position, private motion state, physical velocity, and
/// transient collider identity.
pub type RawCatMotion = (String, Vector3, MotionState, Vector3, Option<u64>);

/// Every cat's motion, in the exact order its caller supplies — the level's
/// own recursive census order, preserved rather than re-sorted. Total over
/// already-decoded raw values: nothing here reaches into a node, and each
/// tuple is read from its actor exactly once by the boundary before this
/// runs.
///
/// The first entry whose position or motion is not a valid physical value
/// refuses the WHOLE list rather than skipping it or returning a partial
/// one — a snapshot that silently dropped one poisoned cat would misreport
/// the census it claims to be exact about. The refusal names the failing
/// entry's index and name, because "the third cat" and "the cat named
/// Fixture" are both facts a reader chasing the fault needs.
pub fn cat_motion_observations(raw: &[RawCatMotion]) -> Result<Vec<CatMotionObservation>, String> {
    raw.iter()
        .enumerate()
        .map(
            |(index, (name, raw_position, state, raw_velocity, support_collider_id))| {
                let position = ActorPosition::try_new(*raw_position)
                    .map_err(|error| format!("cat_motion[{index}] ({name}).position {error}"))?;
                let motion =
                    ActorMotionObservation::try_new(*state, *raw_velocity, *support_collider_id)
                        .map_err(|error| format!("cat_motion[{index}] ({name}).motion {error}"))?;
                Ok(CatMotionObservation {
                    name: name.clone(),
                    position,
                    motion,
                })
            },
        )
        .collect()
}

/// One sound source as an agent reads it. Built at the boundary, where
/// the source nodes live; carried through here unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceObservation {
    pub name: String,
    pub position: Vector3,
    pub volume: f64,
    pub reach: f64,
    /// Seconds between waves — the interval the cadence gate books by, read
    /// live off the designer's knob.
    pub cadence: f64,
    /// When the next wave is due, on the simulated clock. [`f64::NAN`] when
    /// no appointment is being kept — a source that never built, or one
    /// whose cadence cannot fire — so the boundary names it in `unknown`
    /// rather than reporting a date that will never arrive.
    pub next_emit: f64,
    /// Walls between the eye and this source's hub.
    pub walls_to_eye: u32,
    /// This source's standing loudness before any wall — the
    /// `u_source_volume` instance uniform it is pushed. [`f64::NAN`] before
    /// any frame has driven it, which is a different fact from a volume of
    /// zero.
    pub source_volume: f64,
    /// What survives of that image across the walls between it and the eye
    /// — the `u_source_muffle` instance uniform it is pushed.
    ///
    /// Reported apart from [`Self::source_volume`] because the renderer
    /// consumes them apart: their product was once a single pushed number,
    /// and reporting a product the shader no longer forms would be an
    /// observable that agrees with nothing on screen.
    pub source_muffle: f64,
    pub slot_pressure: f64,
}

/// One reflection still waiting on its wavefront.
///
/// An echo is an APPOINTMENT, not an animation: it is scheduled the moment
/// the fan finds a surface and fires when the primary wavefront reaches it.
/// The whole book is reported, because "the echo fired late" and "the echo
/// was never scheduled" are different bugs that look identical from a
/// single frame of pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EchoObservation {
    /// Absolute time the echo fires — the appointment itself.
    pub at_t: f64,
    /// The answering surface point, already nudged off its surface.
    pub pos: Vector3,
    /// Loudness after the distance falloff, as it will be emitted.
    pub gain: f64,
    /// Seconds until it fires. NEGATIVE once the moment has passed while
    /// the drain has not run — a late echo is exactly the fault worth
    /// seeing, and clamping at zero would hide how late it is.
    pub fires_in: f64,
}

/// Where the eye stands, where it looks, and how wide it sees.
///
/// The three travel together because they answer one question together: a
/// reader working out whether a wall should be ON SCREEN needs the field of
/// view as much as the transform, and a snapshot that carried only the
/// transform would leave it guessing the projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EyeObservation {
    pub position: Vector3,
    /// The camera's world basis. A Godot camera looks down its own -Z, so
    /// the heading is the NEGATED third column.
    pub basis: Basis,
    /// Vertical field of view, degrees — `Camera3D::get_fov()` as the
    /// engine holds it.
    pub fov: f64,
}

/// Where the hero woke, as the level derived it from its marker.
///
/// Carried because it is the one landmark that turns the snapshot's world
/// coordinates into a story a reader can follow — "the tap is two metres
/// behind the spawn" — and because nothing else in the snapshot publishes
/// it: the spawn exists only as a derivation from a scene marker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpawnObservation {
    /// The marker's place, already lifted to capsule height.
    pub position: Vector3,
    /// The way the hero faces on waking, in radians.
    pub yaw: f64,
}

/// One wave request still waiting for the physics tick, as an agent reads
/// it — the hero's out-tray, bound into the snapshot at the same instant
/// as the pool it will feed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueuedWave {
    pub kind: i64,
    pub at: Vector3,
    pub max_r: f64,
    pub speed: f64,
    pub gain: f64,
    pub echoes: i64,
    pub normal: Vector3,
    pub gate: QueuedWaveGate,
}

/// The hero as an agent reads them: where the body stands and moves,
/// where the eye points, and the cane's clocks. Before this group existed
/// an agent stitched the same facts from eight separate property reads
/// across frames, so the "one instant" guarantee never covered the hero.
#[derive(Debug, Clone, PartialEq)]
pub struct HeroObservation {
    pub position: ActorPosition,
    /// Phase, physical velocity and collider identity, checked once at the
    /// boundary. The Godot snapshot's `hero.velocity` key and its
    /// `hero.motion.actual_velocity` key are both projected from
    /// [`ActorMotionObservation::actual_velocity`] — one measurement, two
    /// names, never a second raw field that could drift from it.
    pub motion: ActorMotionObservation,
    /// Body yaw, radians — the way the hero faces.
    pub yaw: f64,
    /// Eye pitch, radians, as the look law last clamped it.
    pub pitch: f64,
    /// The tap clock reading of the last ACCEPTED tap (−10.0 when none).
    pub last_tap: f64,
    /// Where that tap landed.
    pub tap_target: Vector3,
    /// A tap accepted this frame that the physics tick has not yet run.
    pub tap_queued: bool,
    /// Every wave request waiting for the next physics tick.
    pub queued_waves: Vec<QueuedWave>,
}

/// The scene as the boundary measured it this frame — everything
/// [`frame`] needs that does not come out of the wave engine itself.
///
/// Grouped rather than passed one by one: these four are obtained the same
/// way (walking Godot nodes in `crate::nodes::observer`) and travel
/// together, where the pool and the clock come from elsewhere entirely.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObservation {
    pub sources: Vec<SourceObservation>,
    pub wall_rects: Vec<Vector4>,
    pub eye: EyeObservation,
    pub spawn: SpawnObservation,
    pub hero: Option<HeroObservation>,
    /// Every cat's motion, in recursive census order.
    pub cat_motion: Vec<CatMotionObservation>,
}

/// The whole state vector for one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameObservation {
    pub now: f64,
    pub flick: f64,
    /// HIGH-WATER MARK, never a census: highest live slot + 1, the bound
    /// the shaders break their per-pixel loop at. Holes are SPANNED — a
    /// dead slot 0 under a live slot 1 scans to 2 — and the shipped pool
    /// wraps continuously, so once slot 63 has been claimed this sits at
    /// [`crate::pulse_pool::MAXP`] for that slot's whole lifetime while far
    /// fewer slots are live. [`Self::live_slots`] is the count.
    pub slot_scan_limit: usize,
    /// How many slots are actually live, counted from [`Self::slots`] — so
    /// it agrees with the per-slot `state` an agent reads beside it, and is
    /// decoded from the same f32 lanes the shaders consume rather than from
    /// the pool's f64 shadow.
    pub live_slots: usize,
    pub slots: Vec<SlotObservation>,
    pub next_eviction: EvictionPlan,
    /// Every reflection scheduled and not yet fired, in discovery order —
    /// the order the drain itself walks.
    pub echoes: Vec<EchoObservation>,
    pub sources: Vec<SourceObservation>,
    pub wall_rects: Vec<Vector4>,
    /// True when the table has reached the shader's ceiling, so walls may
    /// have been dropped. Loud by construction.
    pub wall_truncated: bool,
    pub eye: EyeObservation,
    pub spawn: SpawnObservation,
    pub hero: Option<HeroObservation>,
    /// Every cat's motion, in recursive census order — carried through
    /// from [`SceneObservation::cat_motion`] unchanged.
    pub cat_motion: Vec<CatMotionObservation>,
}

/// Compose one frame's observation from parts the boundary supplies.
///
/// Pure: every argument is plain data. The boundary
/// (`crate::nodes::observer`) is what knows how to obtain them.
#[must_use]
pub fn frame(
    pool: &PulsePool,
    book: &EchoQueue,
    now: f64,
    flick: f64,
    scene: SceneObservation,
) -> FrameObservation {
    let slots = slots(pool, now);
    let live_slots = slots
        .iter()
        .filter(|slot| slot.state == SlotState::Live)
        .count();
    FrameObservation {
        now,
        flick,
        slot_scan_limit: pool.live_count(now),
        live_slots,
        slots,
        next_eviction: explain_eviction(pool, now),
        echoes: echoes(book, now),
        sources: scene.sources,
        wall_truncated: scene.wall_rects.len() >= MAXW,
        wall_rects: scene.wall_rects,
        eye: scene.eye,
        spawn: scene.spawn,
        hero: scene.hero,
        cat_motion: scene.cat_motion,
    }
}

/// Decode the echo book as of `now`.
///
/// Pure and total: an empty book is an empty list, and an appointment
/// already past its moment reports a negative wait rather than being
/// dropped — the drain has simply not run yet, and that gap is the fault
/// worth seeing.
#[must_use]
fn echoes(book: &EchoQueue, now: f64) -> Vec<EchoObservation> {
    book.pending()
        .iter()
        .map(|echo| EchoObservation {
            at_t: echo.at_t,
            pos: echo.pos,
            gain: echo.gain,
            fires_in: echo.at_t - now,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::evict::EvictionRule;
    use super::pool::SlotState;
    use super::*;
    use crate::echo_queue::EchoQueue;
    use crate::pulse_pool::PulsePool;
    use crate::support_motion::{FiniteVelocity, MotionValueProblem, PlanarVelocity};
    use godot::builtin::{Basis, Vector3, Vector4};

    /// An eye with a field of view nothing else in these tests produces,
    /// so a snapshot that invented one instead of carrying this through
    /// would be obvious.
    const TEST_FOV: f64 = 61.0;

    fn test_eye() -> EyeObservation {
        EyeObservation {
            position: Vector3::ZERO,
            basis: Basis::IDENTITY,
            fov: TEST_FOV,
        }
    }

    /// A spawn nothing else in these tests produces, for the same reason
    /// [`TEST_FOV`] exists.
    fn test_spawn() -> SpawnObservation {
        SpawnObservation {
            position: Vector3::new(3.0, 0.9, 4.0),
            yaw: -1.9,
        }
    }

    /// A world with no walls and no sources, seen from the test eye.
    fn test_scene(wall_rects: Vec<Vector4>) -> SceneObservation {
        SceneObservation {
            sources: Vec::new(),
            wall_rects,
            eye: test_eye(),
            spawn: test_spawn(),
            hero: None,
            cat_motion: Vec::new(),
        }
    }

    fn empty_frame(pool: &PulsePool, now: f64) -> FrameObservation {
        frame(pool, &EchoQueue::new(), now, 1.0, test_scene(Vec::new()))
    }

    /// The composer carries the pieces through without recomputing them:
    /// both pool numbers agree with the pool, and the eviction plan is
    /// present. One emit is the one case where a bound and a census are
    /// numerically identical — the next test is what tells them apart.
    #[test]
    fn a_frame_carries_pool_state_and_the_next_eviction() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let f = empty_frame(&pool, 0.5);
        assert_eq!(f.now, 0.5);
        assert_eq!(f.slot_scan_limit, 1);
        assert_eq!(f.live_slots, 1);
        assert_eq!(f.slots.len(), 64);
        assert_eq!(f.next_eviction.rule, EvictionRule::Expired);
        assert_eq!(f.next_eviction.slot, 1);
    }

    /// The two pool numbers are different questions, and a hole is where
    /// they part company: a dead slot 0 under a live slot 1 scans to 2
    /// while exactly ONE slot is live. The shipped pool wraps continuously,
    /// so once slot 63 has been claimed the scan limit sits at 64 for that
    /// slot's whole lifetime — a reader that took it for a census would
    /// diagnose a saturated pool and chase eviction pressure that is not
    /// there.
    #[test]
    fn the_scan_limit_spans_holes_that_the_live_census_does_not() {
        let mut pool = PulsePool::new();
        // slot 0: kind 2, ring 1.6/4.0 = 0.4 s + a 2.5 s tail — dead by 2.9
        pool.emit_omni(2, Vector3::ZERO, 1.6, 4.0, 0.8, 0.0)
            .unwrap();
        // slot 1: kind 0, ring 6/5.5 s + a 6 s tail — alive well past 5
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let f = empty_frame(&pool, 5.0);
        assert_eq!(f.slot_scan_limit, 2);
        assert_eq!(f.live_slots, 1);
        assert_eq!(
            f.slots
                .iter()
                .filter(|s| s.state == SlotState::Live)
                .count(),
            1
        );
    }

    /// A wall table at the shader's ceiling is flagged. The level
    /// truncates at MAXW and must say so — a silently clipped table
    /// occludes with walls the level does not have.
    #[test]
    fn a_full_wall_table_is_flagged_as_truncated() {
        let pool = PulsePool::new();
        let rect = Vector4::new(0.0, 0.0, 1.0, 1.0);
        let book = EchoQueue::new();
        let short = frame(&pool, &book, 0.0, 1.0, test_scene(vec![rect; 31]));
        let full = frame(&pool, &book, 0.0, 1.0, test_scene(vec![rect; 32]));
        assert!(!short.wall_truncated);
        assert!(full.wall_truncated);
    }

    /// A level with no sources is legal and reports an empty list — not
    /// an error, and not an absence of the field.
    #[test]
    fn a_silent_level_is_legal() {
        let pool = PulsePool::new();
        assert!(empty_frame(&pool, 0.0).sources.is_empty());
    }

    /// The echo book, as an agent reads it: every appointment still
    /// waiting, with the seconds left before it fires. Hand-derived from
    /// the reflection contract (`rust/src/echo_queue.rs`): a hit 5.5 m
    /// down a 5.5 m/s ray scheduled at t = 0 fires at t = 1, so at t =
    /// 0.25 it is 0.75 s away, and its loudness is 1 x 0.55 / (1 + 0.4 x
    /// 5.5) = 0.171875.
    ///
    /// "The echo fired a frame late" is a whole question class, and it
    /// cannot be asked of a snapshot that does not carry the book.
    #[test]
    fn the_echo_book_reports_the_wait_on_every_appointment() {
        let pool = PulsePool::new();
        let mut book = EchoQueue::new();
        book.schedule(0.0, 5.5, Vector3::new(1.0, 2.0, 3.0), 1.0, 5.5)
            .unwrap();
        let f = frame(&pool, &book, 0.25, 1.0, test_scene(Vec::new()));
        assert_eq!(f.echoes.len(), 1);
        assert!((f.echoes[0].at_t - 1.0).abs() < 1e-9);
        assert!((f.echoes[0].fires_in - 0.75).abs() < 1e-9);
        assert!((f.echoes[0].gain - 0.171_875).abs() < 1e-9);
        assert_eq!(f.echoes[0].pos, Vector3::new(1.0, 2.0, 3.0));
    }

    /// An appointment whose moment has passed while the drain has not run
    /// reports a NEGATIVE wait rather than a clamped zero. A late echo is
    /// precisely the bug this group exists to make visible, and a floor at
    /// zero would hide how late it is.
    #[test]
    fn an_overdue_appointment_reports_how_late_it_is() {
        let pool = PulsePool::new();
        let mut book = EchoQueue::new();
        book.schedule(0.0, 5.5, Vector3::ZERO, 1.0, 5.5).unwrap();
        let f = frame(&pool, &book, 1.5, 1.0, test_scene(Vec::new()));
        assert!((f.echoes[0].fires_in + 0.5).abs() < 1e-9);
    }

    /// An empty book is an empty list, never a missing key: a level with
    /// nothing scheduled and a level whose book could not be read must not
    /// serialise the same.
    #[test]
    fn an_empty_echo_book_is_an_empty_list() {
        let pool = PulsePool::new();
        assert!(empty_frame(&pool, 0.0).echoes.is_empty());
    }

    /// The eye is one thing — where it stands, where it looks, and how
    /// wide it sees. The field of view is what turns a world position into
    /// a screen position, so a reader reasoning about what should be ON
    /// SCREEN cannot do it from the transform alone.
    #[test]
    fn the_eye_carries_its_field_of_view() {
        let pool = PulsePool::new();
        assert_eq!(empty_frame(&pool, 0.0).eye.fov, TEST_FOV);
    }

    /// The composer carries the hero through untouched — and an absent
    /// hero stays absent rather than becoming a hero at the origin, which
    /// would be the vacuous pass this layer exists to prevent.
    #[test]
    fn a_frame_carries_the_hero_when_the_scene_has_one() {
        let pool = PulsePool::new();
        let motion = ActorMotionObservation::try_new(
            MotionState::initial(),
            Vector3::new(0.0, 0.0, -2.1),
            None,
        )
        .unwrap();
        let hero = HeroObservation {
            position: ActorPosition::try_new(Vector3::new(1.0, 0.9, -2.0)).unwrap(),
            motion,
            yaw: 0.7,
            pitch: -0.3,
            last_tap: 4.5,
            tap_target: Vector3::new(1.0, 0.0, -3.5),
            tap_queued: true,
            queued_waves: vec![QueuedWave {
                kind: 2,
                at: Vector3::ZERO,
                max_r: 4.0,
                speed: 4.0,
                gain: 0.5,
                echoes: 0,
                normal: Vector3::UP,
                gate: QueuedWaveGate::Always,
            }],
        };
        let mut scene = test_scene(Vec::new());
        scene.hero = Some(hero.clone());
        let f = frame(&pool, &EchoQueue::new(), 0.0, 1.0, scene);
        assert_eq!(f.hero, Some(hero));
        assert_eq!(empty_frame(&pool, 0.0).hero, None);
    }

    /// A poisoned physical velocity is refused on every lane and every
    /// non-finite shape — the same `ActorVelocity` contract, read through
    /// the checked observation rather than duplicated here.
    #[test]
    fn actor_motion_observation_rejects_poisoned_velocity() {
        for lane in 0..3 {
            for poison in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                let mut lanes = [0.0_f32, 0.0, 0.0];
                lanes[lane] = poison;
                let velocity = Vector3::new(lanes[0], lanes[1], lanes[2]);
                let error = ActorMotionObservation::try_new(MotionState::initial(), velocity, None)
                    .unwrap_err();
                assert_eq!(error.problem(), MotionValueProblem::NonFinite);
            }
        }
    }

    /// A zero collider id is refused as out of range regardless of
    /// support, and a nonzero id reported for a phase carrying no accepted
    /// support is refused as inconsistent — while a genuinely supported
    /// state accepts both a real id and the legitimate absence of one a
    /// server-backed body may report.
    #[test]
    fn actor_motion_observation_rejects_zero_or_unsupported_identity() {
        let unsupported = MotionState::initial();
        let zero_id =
            ActorMotionObservation::try_new(unsupported, Vector3::ZERO, Some(0)).unwrap_err();
        assert_eq!(zero_id.problem(), MotionValueProblem::OutOfRange);
        assert_eq!(zero_id.field(), "support_collider_id");

        let unsupported_id =
            ActorMotionObservation::try_new(unsupported, Vector3::ZERO, Some(7)).unwrap_err();
        assert_eq!(
            unsupported_id.problem(),
            MotionValueProblem::InconsistentState
        );
        assert_eq!(unsupported_id.field(), "support_collider_id");

        let support = SupportContact::try_new(Vector3::ZERO, Vector3::UP).unwrap();
        let supported = MotionState::restore(MotionPhase::Controlled, Some(support), None).unwrap();
        let with_id = ActorMotionObservation::try_new(supported, Vector3::ZERO, Some(7)).unwrap();
        assert_eq!(with_id.support_collider_id(), Some(7));
        let server_backed =
            ActorMotionObservation::try_new(supported, Vector3::ZERO, None).unwrap();
        assert_eq!(server_backed.support_collider_id(), None);
    }

    /// Every phase/support/landing getter projects the one private
    /// `MotionState`, and the physical velocity travels through untouched
    /// — a controlled, never-landed actor; an airborne actor whose held
    /// planar and vertical velocities differ from its just-measured
    /// physical one; and a controlled, landed actor carrying its last
    /// landing event and a real collider identity.
    #[test]
    fn actor_motion_observation_projects_controlled_airborne_never_landed_and_landed_cases() {
        let never_landed = MotionState::initial();
        let idle = ActorMotionObservation::try_new(never_landed, Vector3::new(1.0, 0.0, 2.0), None)
            .unwrap();
        assert_eq!(idle.phase(), MotionPhase::Controlled);
        assert_eq!(idle.support(), None);
        assert_eq!(idle.last_landing(), None);
        assert_eq!(idle.actual_velocity().world(), Vector3::new(1.0, 0.0, 2.0));

        let planar = PlanarVelocity::try_new(3.0, -1.0).unwrap();
        let vertical = FiniteVelocity::try_new(-4.0).unwrap();
        let airborne_phase = MotionPhase::Airborne {
            planar_velocity_mps: planar,
            vertical_velocity_mps: vertical,
        };
        let airborne_state = MotionState::restore(airborne_phase, None, None).unwrap();
        let airborne =
            ActorMotionObservation::try_new(airborne_state, Vector3::new(3.0, -4.0, -1.0), None)
                .unwrap();
        assert_eq!(airborne.phase(), airborne_phase);
        assert_eq!(
            airborne.actual_velocity().world(),
            Vector3::new(3.0, -4.0, -1.0)
        );

        let support = SupportContact::try_new(Vector3::new(1.0, 2.0, 3.0), Vector3::UP).unwrap();
        let landing = LandingEvent::try_new(6.0, support).unwrap();
        let landed_state =
            MotionState::restore(MotionPhase::Controlled, Some(support), Some(landing)).unwrap();
        let landed = ActorMotionObservation::try_new(landed_state, Vector3::ZERO, Some(9)).unwrap();
        assert_eq!(landed.support(), Some(support));
        assert_eq!(landed.last_landing(), Some(landing));
        assert_eq!(landed.support_collider_id(), Some(9));
    }

    /// Every cat's motion comes back in the exact order it was supplied,
    /// and a poisoned position anywhere in the list refuses the WHOLE
    /// list — naming the failing entry's census index and name — rather
    /// than silently dropping it or returning a partial census.
    #[test]
    fn cat_motion_observations_preserve_census_order() {
        let alpha = (
            "Alpha".to_string(),
            Vector3::new(1.0, 0.0, 0.0),
            MotionState::initial(),
            Vector3::ZERO,
            None,
        );
        let beta = (
            "Beta".to_string(),
            Vector3::new(2.0, 0.0, 0.0),
            MotionState::initial(),
            Vector3::ZERO,
            None,
        );
        let ordered = cat_motion_observations(&[alpha.clone(), beta.clone()]).unwrap();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].name, "Alpha");
        assert_eq!(ordered[1].name, "Beta");
        assert_eq!(ordered[0].position.world(), Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(ordered[1].position.world(), Vector3::new(2.0, 0.0, 0.0));

        let poisoned = (
            "Ghost".to_string(),
            Vector3::new(f32::NAN, 0.0, 0.0),
            MotionState::initial(),
            Vector3::ZERO,
            None,
        );
        let error = cat_motion_observations(&[alpha, poisoned]).unwrap_err();
        assert!(error.contains("cat_motion[1]"));
        assert!(error.contains("Ghost"));
    }

    /// The composer carries every cat's motion through in census order,
    /// same as it does the hero — and no cats is an empty list, never a
    /// missing key.
    #[test]
    fn a_frame_carries_cat_motion_in_census_order() {
        let pool = PulsePool::new();
        let raw = [
            (
                "First".to_string(),
                Vector3::new(1.0, 0.0, 0.0),
                MotionState::initial(),
                Vector3::ZERO,
                None,
            ),
            (
                "Second".to_string(),
                Vector3::new(2.0, 0.0, 0.0),
                MotionState::initial(),
                Vector3::ZERO,
                None,
            ),
        ];
        let cats = cat_motion_observations(&raw).unwrap();
        let mut scene = test_scene(Vec::new());
        scene.cat_motion = cats.clone();
        let f = frame(&pool, &EchoQueue::new(), 0.0, 1.0, scene);
        assert_eq!(f.cat_motion, cats);
        assert!(empty_frame(&pool, 0.0).cat_motion.is_empty());
    }
}
