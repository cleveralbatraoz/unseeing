//! Why a surface answered, or did not.
//!
//! The golden-angle fan and the clustering behind every echo are computed
//! and thrown away inside one frame, so no snapshot can ever hold them.
//! This re-runs the same pure functions on demand and accounts for every
//! ray — including the ones that struck nothing, because absence of echo
//! is how this world communicates its shape and a report listing only the
//! hits would hide exactly the case worth investigating.
//!
//! Every ray is accounted for exactly once, and the reasons are kept
//! APART. A point dropped as the sound's own birth surface and a point
//! dropped past the echo budget are different answers to "why did that
//! wall stay silent"; collapsing them into one "dropped" number would
//! defeat the whole exercise.
//!
//! Nothing here casts a ray or touches a queue: [`explain_clustering`]
//! takes hits somebody else gathered and reports what the clustering law
//! did with them, so the whole reasoning half is cargo-testable with no
//! engine at all. Casting is the boundary's job, because a physics space
//! is an engine object and only the physics tick may touch it.
//!
//! Precision, per the crate's law (`echo_queue`): distances arrive as f32
//! — a single-precision `Vector3` length, exactly what the engine hands
//! back — and are carried at that width. Only the CLOCK widens: an
//! appointment is f64, computed by the echo book itself rather than
//! restated here.

use godot::builtin::Vector3;

use crate::clustering::{self, RayHit};
use crate::echo_queue::EchoQueue;
use crate::pulse_pool::{CheckedWave, OMNI_COS, WaveOrigin};
use crate::ray_fan;
use crate::temporal::prepare_time;

/// How many collected explanations an observer keeps before the oldest
/// falls off. A debugging loop that requests and never collects must not
/// grow without bound inside the running game; an aged-out id is refused
/// exactly like one that never existed.
pub const EXPLANATION_MEMORY: usize = 32;

/// The primary gain a question is asked with. An explanation is asked
/// ABOUT a sound rather than for one, so it carries no loudness of its
/// own — and the falloff law (`gain * 0.55 / (1 + 0.4 d)`) is linear in
/// gain, so a unit primary reports the FRACTION of any primary's gain
/// that survives to each answering point.
const UNIT_GAIN: f64 = 1.0;

/// A wavefront that does not move cannot keep an appointment: at zero
/// speed every echo fires at infinity, and at a negative one they fire
/// before the sound was made. The engine's own pool refuses non-positive
/// speed and radius (`PulsePool::emit`); a question about a sound it would
/// refuse is refused in the same spirit — and a non-finite number here
/// would reach an agent through `JSON.stringify` as `null`, which reads as
/// a missing field rather than as an error.
pub const REFUSED_SPEED: &str = "reflection request refused: speed must be finite and positive — an appointment needs a moving wavefront";

/// The fan's reach is `min(max_r * 0.8, 6)`, so a non-positive or
/// non-finite range asks for a fan with no length.
pub const REFUSED_MAX_R: &str = "reflection request refused: max_r must be finite and positive — the fan's reach derives from it";

/// Every appointment is measured from `now`, so a non-finite clock makes
/// every one of them non-finite too.
pub const REFUSED_CLOCK: &str =
    "reflection request refused: now must be finite — every appointment is measured from it";

/// A NaN origin or normal casts rays nowhere and reports positions no
/// agent can act on.
pub const REFUSED_GEOMETRY: &str = "reflection request refused: origin, normal, and derived fan geometry must remain finite in f32";

/// The checked live caster supplies representable hits, but the pure
/// explanation remains total over its explicit hit slice. One unschedulable
/// kept point refuses the whole answer instead of disappearing.
pub const REFUSED_APPOINTMENT: &str =
    "reflection explanation refused: echo appointment is not representable";

pub const REFUSED_HIT: &str =
    "reflection fan refused: physics server returned malformed hit geometry";

/// One question: the sound whose reflections are being explained, as it
/// would have been emitted. The kind, the loudness and the space are all
/// absent on purpose — a question must not carry the things that would
/// let it emit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReflectionRequest {
    /// Where the sound would be born.
    pub at: Vector3,
    /// The normal of the surface it would be born on. `ZERO` means an
    /// airborne sound: no hemisphere cull, the whole fan is cast.
    pub normal: Vector3,
    /// The primary wave's range, which sets how far the fan reaches.
    pub max_r: f64,
    /// The primary wave's speed, which sets when each echo would fire.
    pub speed: f64,
    /// The caller's echo budget.
    pub max_echoes: i64,
    /// The clock the appointments are measured from.
    pub now: f64,
}

/// A reflection fan whose complete caller-supplied geometry has been proved
/// representable before any queue, pool, or physics-space mutation. The
/// retained directions and f32 reach are the exact values the engine caster
/// consumes, so no unchecked arithmetic is repeated at the boundary.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CheckedReflectionRequest {
    request: ReflectionRequest,
    ray_origin: Vector3,
    reach_lane: f32,
    directions: Vec<Vector3>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReflectionValueError {
    field: &'static str,
    reason: &'static str,
}

impl ReflectionValueError {
    fn new(field: &'static str, reason: &'static str) -> Self {
        Self { field, reason }
    }

    pub(crate) fn field(self) -> &'static str {
        self.field
    }

    pub(crate) fn reason(self) -> &'static str {
        self.reason
    }
}

impl CheckedReflectionRequest {
    pub(crate) fn prepare(request: ReflectionRequest) -> Result<Self, ReflectionValueError> {
        for (lane, field) in [request.normal.x, request.normal.y, request.normal.z]
            .into_iter()
            .zip(["normal.x", "normal.y", "normal.z"])
        {
            if !lane.is_finite() {
                return Err(ReflectionValueError::new(field, REFUSED_GEOMETRY));
            }
        }
        let now = prepare_time(request.now)
            .map_err(|_| ReflectionValueError::new("now", REFUSED_CLOCK))?;
        CheckedWave::prepare(
            0,
            request.at,
            request.max_r,
            request.speed,
            1.0,
            now,
            Vector3::ZERO,
            OMNI_COS,
        )
        .map_err(|error| {
            let reason = match error.field() {
                "speed" | "end" | "ring_time" => REFUSED_SPEED,
                "max_r" => REFUSED_MAX_R,
                _ => REFUSED_GEOMETRY,
            };
            ReflectionValueError::new(error.field(), reason)
        })?;

        let normal = request.normal;
        if normal != Vector3::ZERO {
            let length_squared = normal.x * normal.x + normal.y * normal.y + normal.z * normal.z;
            if !length_squared.is_finite() || length_squared <= 0.0 {
                return Err(ReflectionValueError::new("normal", REFUSED_GEOMETRY));
            }
        }
        let ray_origin = request.at + normal * clustering::RAY_ORIGIN_LIFT;
        if !ray_origin.is_finite() {
            return Err(ReflectionValueError::new("ray_origin", REFUSED_GEOMETRY));
        }
        let reach = clustering::ray_length(request.max_r);
        let reach_lane = reach as f32;
        if !reach_lane.is_finite() || reach_lane <= 0.0 {
            return Err(ReflectionValueError::new("reach", REFUSED_GEOMETRY));
        }

        let mut directions = Vec::with_capacity(ray_fan::RAYS);
        for index in 0..ray_fan::RAYS {
            let direction = ray_fan::fan_direction(index);
            let dot = direction.dot(normal);
            if !dot.is_finite() {
                return Err(ReflectionValueError::new("fan.dot", REFUSED_GEOMETRY));
            }
            if normal == Vector3::ZERO || dot >= ray_fan::SHADOW_DOT {
                let endpoint = ray_origin + direction * reach_lane;
                if !endpoint.is_finite() {
                    return Err(ReflectionValueError::new("fan.endpoint", REFUSED_GEOMETRY));
                }
                directions.push(direction);
            }
        }

        Ok(Self {
            request,
            ray_origin,
            reach_lane,
            directions,
        })
    }

    pub(crate) fn ray_origin(&self) -> Vector3 {
        self.ray_origin
    }

    pub(crate) fn request(&self) -> ReflectionRequest {
        self.request
    }

    pub(crate) fn reach_lane(&self) -> f32 {
        self.reach_lane
    }

    pub(crate) fn directions(&self) -> impl Iterator<Item = Vector3> + '_ {
        self.directions.iter().copied()
    }
}

/// One physics-server hit proved safe for clustering and echo-origin
/// construction. The engine dictionary is untrusted; this owner validates
/// every lane and the derived f32 distance before a hit enters a cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CheckedRayHit(RayHit);

impl CheckedRayHit {
    pub(crate) fn prepare(
        position: Vector3,
        normal: Vector3,
        request: &CheckedReflectionRequest,
    ) -> Result<Self, ReflectionValueError> {
        for (lane, field) in [position.x, position.y, position.z].into_iter().zip([
            "hit.position.x",
            "hit.position.y",
            "hit.position.z",
        ]) {
            if !lane.is_finite() {
                return Err(ReflectionValueError::new(field, REFUSED_HIT));
            }
        }
        WaveOrigin::try_new(position).map_err(|error| {
            let field = match error.axis() {
                "x" => "hit.position.x",
                "y" => "hit.position.y",
                _ => "hit.position.z",
            };
            ReflectionValueError::new(field, REFUSED_HIT)
        })?;
        for (lane, field) in [normal.x, normal.y, normal.z].into_iter().zip([
            "hit.normal.x",
            "hit.normal.y",
            "hit.normal.z",
        ]) {
            if !lane.is_finite() {
                return Err(ReflectionValueError::new(field, REFUSED_HIT));
            }
        }
        let normal_length_squared = normal.x * normal.x + normal.y * normal.y + normal.z * normal.z;
        if !normal_length_squared.is_finite() || normal_length_squared <= 0.0 {
            return Err(ReflectionValueError::new("hit.normal", REFUSED_HIT));
        }
        let dist = (position - request.ray_origin()).length();
        if !dist.is_finite() || dist < 0.0 || dist > request.reach_lane() {
            return Err(ReflectionValueError::new("hit.distance", REFUSED_HIT));
        }
        let echo_origin = position + normal * clustering::SURFACE_OFFSET;
        WaveOrigin::try_new(echo_origin).map_err(|error| {
            let field = match error.axis() {
                "x" => "hit.echo_origin.x",
                "y" => "hit.echo_origin.y",
                _ => "hit.echo_origin.z",
            };
            ReflectionValueError::new(field, REFUSED_HIT)
        })?;
        Ok(Self(RayHit {
            position,
            normal,
            dist,
        }))
    }

    pub(crate) fn ray_hit(self) -> RayHit {
        self.0
    }
}

/// One answering point, and when it would answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusteredPoint {
    /// The point, already nudged off the struck surface.
    pub point: Vector3,
    /// Distance from the ray origin. f32, at the width the engine
    /// measured it — this is a geometry length, not a clock.
    pub dist: f32,
    /// When the echo would fire: the appointment the echo book itself
    /// computes, widened to f64 there rather than restated here.
    pub at_t: f64,
    /// The fraction of the primary's gain that reaches this point. A wall
    /// that answers far too faintly to notice is silent for a different
    /// reason than one that was never struck.
    pub gain_fraction: f64,
}

/// The whole fan, accounted for.
///
/// The counts form a total ledger, and the tests pin it:
/// `rays_cast = rays_missed + rays_struck`, and
/// `rays_struck = self_surface_drops + merged_into_cells + cells_found`,
/// and `cells_found = dropped_past_budget + clusters_kept`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionExplanation {
    /// Where the sound would be born.
    pub at: Vector3,
    /// Where the rays actually started — `at` lifted off the surface.
    pub origin: Vector3,
    pub normal: Vector3,
    /// How far each ray reached.
    pub reach: f64,
    /// The nominal fan: every direction before the hemisphere cull.
    pub fan_size: usize,
    /// The directions actually cast, after the cull. Always fewer than
    /// `fan_size` for a sound born on a surface.
    pub rays_cast: usize,
    /// Rays that struck something within `reach`.
    pub rays_struck: usize,
    /// Hits vetoed as the sound's own birth surface.
    pub self_surface_drops: usize,
    /// Hits that lost their 0.9 m cell to a nearer hit — a flat wall
    /// answering as a point rather than as a ray count.
    pub merged_into_cells: usize,
    /// Distinct cells that survived to be ranked, before the budget.
    pub cells_found: usize,
    /// The caller's budget, clamped as the engine clamps it.
    pub budget: usize,
    /// Cells that ranked past the budget and were never scheduled.
    pub dropped_past_budget: usize,
    /// The points that would answer, nearest first.
    pub points: Vec<ClusteredPoint>,
}

impl ReflectionExplanation {
    /// Rays that reached their full length and found nothing. The headline
    /// number: silence with no wall behind it.
    #[must_use]
    pub fn rays_missed(&self) -> usize {
        self.rays_cast.saturating_sub(self.rays_struck)
    }

    /// Points that would actually be scheduled.
    #[must_use]
    pub fn clusters_kept(&self) -> usize {
        self.points.len()
    }
}

/// Explain what the clustering law did with an already-cast fan.
///
/// `rays_cast` is passed in rather than derived: the caster knows how many
/// queries it truly made, and a count re-derived here could disagree with
/// the rays whose results are being explained.
pub(crate) fn explain_clustering(
    request: &CheckedReflectionRequest,
    rays_cast: usize,
    hits: &[RayHit],
) -> Result<ReflectionExplanation, &'static str> {
    let raw = request.request();
    let budget = clustering::echo_budget(raw.max_echoes);
    let self_surface_drops = hits
        .iter()
        .filter(|hit| clustering::is_self_surface(hit.dist))
        .count();
    // The same law, run twice against the same hits: once unbudgeted, to
    // count the cells that existed, and once budgeted, to see which of
    // them survive. Truncating the first list here instead would restate
    // the cap in a second place, and the two would drift.
    let cells_found = clustering::cluster_hits(hits.iter().copied(), usize::MAX).len();
    let kept = clustering::cluster_hits(hits.iter().copied(), budget);
    let points = appointments(request, &kept)?;
    Ok(ReflectionExplanation {
        at: raw.at,
        origin: request.ray_origin(),
        normal: raw.normal,
        reach: f64::from(request.reach_lane()),
        fan_size: ray_fan::RAYS,
        rays_cast,
        rays_struck: hits.len(),
        self_surface_drops,
        // total by construction (every cell holds at least one surviving
        // hit), and saturating anyway: a debug tool must never panic
        merged_into_cells: hits
            .len()
            .saturating_sub(self_surface_drops)
            .saturating_sub(cells_found),
        cells_found,
        budget,
        dropped_past_budget: cells_found.saturating_sub(kept.len()),
        points,
    })
}

/// When each surviving point would answer, and how loudly.
///
/// Scheduled into a SCRATCH echo book so the timing and falloff laws are
/// the queue's own rather than a second copy of them — and so that asking
/// the question cannot possibly reach the queue the game drains.
fn appointments(
    request: &CheckedReflectionRequest,
    kept: &[clustering::SurfaceHit],
) -> Result<Vec<ClusteredPoint>, &'static str> {
    let raw = request.request();
    let mut scratch = EchoQueue::new();
    let mut points = Vec::with_capacity(kept.len());
    for hit in kept {
        scratch
            .schedule(raw.now, hit.dist, hit.point, UNIT_GAIN, raw.speed)
            .map_err(|_| REFUSED_APPOINTMENT)?;
        let Some(echo) = scratch.pending().last() else {
            return Err(REFUSED_APPOINTMENT);
        };
        points.push(ClusteredPoint {
            point: hit.point,
            dist: hit.dist,
            at_t: echo.at_t,
            gain_fraction: echo.gain,
        });
    }
    Ok(points)
}

/// What a collected explanation turned out to be.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Explained(Box<ReflectionExplanation>),
    /// The frame ran but could not cast — reported as a refusal, never as
    /// a fan of zero hits, which is a fact about the world rather than
    /// about the observer.
    Refused(&'static str),
}

/// The three states a collected request can be in. An id that was never
/// issued and one already collected are the same answer on purpose: both
/// mean "there is nothing here for you", and neither is a fan of no hits.
#[derive(Debug, Clone, PartialEq)]
pub enum Collected {
    Pending,
    Ready(Answer),
    Unknown,
}

/// The request/collect book.
///
/// A physics space may only be touched inside the physics tick, so a
/// question asked from anywhere else has to wait for one. This holds the
/// waiting questions and the answered ones, and hands each answer over
/// exactly once.
///
/// BOTH halves are bounded by [`EXPLANATION_MEMORY`], and for the same
/// reason. A loop that asks and never collects fills the answered half; an
/// observer whose physics frame never runs — outside a tree, or paused
/// without the process mode that survives a pause — fills the waiting one.
/// Neither may grow without bound inside a running game, and an entry that
/// has aged out of either half reads as an unknown id.
#[derive(Debug, Default)]
pub struct ExplanationLedger {
    next_id: i64,
    waiting: Vec<(i64, CheckedReflectionRequest)>,
    ready: Vec<(i64, Answer)>,
}

impl ExplanationLedger {
    /// Book a question and return its id. Ids start at 1, so 0 is never a
    /// valid one — a caller that dropped the return value cannot collect
    /// somebody else's answer by accident.
    pub(crate) fn request(&mut self, request: CheckedReflectionRequest) -> i64 {
        self.next_id = self.next_id.saturating_add(1);
        self.waiting.push((self.next_id, request));
        if self.waiting.len() > EXPLANATION_MEMORY {
            self.waiting.remove(0);
        }
        self.next_id
    }

    /// Every question waiting on a physics frame, removed from the book.
    /// The caster answers them; anything it fails to answer is simply
    /// forgotten, which reads as an unknown id rather than a lie.
    pub(crate) fn take_requests(&mut self) -> Vec<(i64, CheckedReflectionRequest)> {
        std::mem::take(&mut self.waiting)
    }

    /// Allocate an id for an impossible request and file its answer without
    /// ever placing unchecked geometry in the physics-frame queue.
    pub(crate) fn refuse(&mut self, reason: &'static str) -> i64 {
        self.next_id = self.next_id.saturating_add(1);
        let id = self.next_id;
        self.answer(id, Answer::Refused(reason));
        id
    }

    /// File an answer, ageing out the oldest once the book is full.
    ///
    /// The question leaves the waiting half whether it was taken from
    /// there or not, so a boundary that can answer a request the moment it
    /// is made — an impossible request, or one asked with no world to cast
    /// in — does not leave a phantom waiting behind it.
    pub fn answer(&mut self, id: i64, answer: Answer) {
        self.waiting.retain(|(key, _)| *key != id);
        self.ready.push((id, answer));
        if self.ready.len() > EXPLANATION_MEMORY {
            self.ready.remove(0);
        }
    }

    /// Collect an answer, exactly once.
    pub fn collect(&mut self, id: i64) -> Collected {
        if let Some(index) = self.ready.iter().position(|(key, _)| *key == id) {
            return Collected::Ready(self.ready.remove(index).1);
        }
        if self.waiting.iter().any(|(key, _)| *key == id) {
            return Collected::Pending;
        }
        Collected::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cane tap on the floor: 6 m of range, 5.5 m/s, six echoes.
    fn tap() -> ReflectionRequest {
        ReflectionRequest {
            at: Vector3::new(3.0, 0.0, 4.0),
            normal: Vector3::UP,
            max_r: 6.0,
            speed: 5.5,
            max_echoes: 6,
            now: 10.0,
        }
    }

    fn checked(request: ReflectionRequest) -> CheckedReflectionRequest {
        CheckedReflectionRequest::prepare(request).expect("test request must be representable")
    }

    fn checked_tap() -> CheckedReflectionRequest {
        checked(tap())
    }

    fn hit(position: Vector3, dist: f32) -> RayHit {
        RayHit {
            position,
            normal: Vector3::UP,
            dist,
        }
    }

    /// The ledger's total: every ray cast is missed or struck, and every
    /// hit is vetoed, merged into a cell, or a cell of its own — which in
    /// turn is kept or dropped past the budget. Nothing may fall between.
    fn assert_balanced(e: &ReflectionExplanation) {
        assert_eq!(e.rays_cast, e.rays_missed() + e.rays_struck);
        assert_eq!(
            e.rays_struck,
            e.self_surface_drops + e.merged_into_cells + e.cells_found
        );
        assert_eq!(e.cells_found, e.dropped_past_budget + e.clusters_kept());
    }

    /// The headline: rays that struck nothing are REPORTED, not omitted.
    /// A fan of 20 with 3 hits is a room with a lot of open air in it, and
    /// an agent that only saw the 3 would never learn that.
    #[test]
    fn every_ray_is_reported_including_the_ones_that_struck_nothing() {
        let hits = [
            hit(Vector3::new(4.0, 0.0, 4.0), 1.0),
            hit(Vector3::new(0.0, 0.0, 4.0), 3.0),
            hit(Vector3::new(3.0, 0.0, 0.0), 4.0),
        ];
        let e =
            explain_clustering(&checked_tap(), 20, &hits).expect("the tap schedules every point");
        assert_eq!(e.rays_cast, 20);
        assert_eq!(e.rays_struck, 3);
        assert_eq!(e.rays_missed(), 17);
        assert_eq!(e.fan_size, ray_fan::RAYS);
        assert_balanced(&e);
    }

    /// Several rays landing in one 0.9 m cell answer once — and the report
    /// says so by NAME, so "one point from four rays" reads as clustering
    /// rather than as three rays going missing.
    #[test]
    fn hits_sharing_a_cell_collapse_to_one_point_and_the_merge_is_named() {
        let hits = [
            hit(Vector3::new(4.00, 0.0, 4.0), 2.0),
            hit(Vector3::new(4.10, 0.0, 4.1), 1.5),
            hit(Vector3::new(4.20, 0.0, 4.2), 1.8),
            hit(Vector3::new(4.30, 0.0, 4.3), 2.4),
        ];
        let e =
            explain_clustering(&checked_tap(), 26, &hits).expect("the tap schedules every point");
        assert_eq!(e.rays_struck, 4);
        assert_eq!(e.cells_found, 1);
        assert_eq!(e.merged_into_cells, 3);
        assert_eq!(e.clusters_kept(), 1);
        assert_eq!(e.dropped_past_budget, 0);
        // the nearest strike is the one that answers
        assert!((e.points[0].dist - 1.5).abs() < 1e-6);
        assert_balanced(&e);
    }

    /// The budget truncates, and the count reports the SMALLER number —
    /// with the shortfall named, because "the wall was found and then
    /// dropped" is a different answer than "the wall was never struck".
    #[test]
    fn the_budget_truncates_and_the_drop_is_named() {
        let hits: Vec<RayHit> = (1..=5)
            .map(|i| hit(Vector3::new(i as f32 * 2.0, 0.0, 4.0), i as f32))
            .collect();
        let request = ReflectionRequest {
            max_echoes: 2,
            ..tap()
        };
        let request = checked(request);
        let e = explain_clustering(&request, 26, &hits).expect("the tap schedules every point");
        assert_eq!(e.budget, 2);
        assert_eq!(e.cells_found, 5);
        assert_eq!(e.clusters_kept(), 2);
        assert_eq!(e.dropped_past_budget, 3);
        assert_eq!(e.merged_into_cells, 0);
        assert_eq!(e.self_surface_drops, 0);
        assert_balanced(&e);
    }

    /// A hit on the sound's own birth surface is dropped for THAT reason,
    /// counted apart from the budget and apart from cell merging. This is
    /// the distinction the whole explainer exists to preserve.
    #[test]
    fn a_self_surface_hit_is_dropped_for_that_reason_specifically() {
        let hits = [
            hit(Vector3::new(3.1, 0.0, 4.0), 0.1),
            hit(Vector3::new(6.0, 0.0, 4.0), 3.0),
        ];
        let e =
            explain_clustering(&checked_tap(), 26, &hits).expect("the tap schedules every point");
        assert_eq!(e.rays_struck, 2);
        assert_eq!(e.self_surface_drops, 1);
        assert_eq!(e.merged_into_cells, 0);
        assert_eq!(e.dropped_past_budget, 0);
        assert_eq!(e.clusters_kept(), 1);
        assert_balanced(&e);
    }

    /// A fan that struck nothing at all is an EXPLANATION, not an error:
    /// the rays are reported, the reasons are all zero, and the answer
    /// list is empty. Silence with nothing behind it.
    #[test]
    fn a_fan_that_struck_nothing_still_explains_itself() {
        let e = explain_clustering(&checked_tap(), 17, &[]).expect("an empty fan is representable");
        assert_eq!(e.rays_cast, 17);
        assert_eq!(e.rays_missed(), 17);
        assert_eq!(e.cells_found, 0);
        assert!(e.points.is_empty());
        assert_balanced(&e);
    }

    /// The appointment is the echo book's own: t = now + d / speed, and
    /// the surviving gain fraction is the same falloff the queue applies.
    /// Restating either law here is what this test exists to prevent.
    #[test]
    fn appointments_match_the_echo_books_own_law() {
        let e = explain_clustering(
            &checked_tap(),
            26,
            &[hit(Vector3::new(6.0, 0.0, 4.0), 2.75)],
        )
        .expect("the tap schedules every point");
        let point = e.points[0];
        assert!((point.at_t - (10.0 + 2.75 / 5.5)).abs() < 1e-12);
        assert!((point.gain_fraction - 0.55 / (1.0 + 2.75 * 0.4)).abs() < 1e-12);
    }

    /// Where the rays start and how far they go: lifted off the birth
    /// surface, reaching 0.8 of the wave's range and never past 6 m.
    #[test]
    fn the_fan_starts_off_the_surface_and_reaches_the_clamped_length() {
        let request = checked_tap();
        assert_eq!(
            request.ray_origin(),
            Vector3::new(3.0, clustering::RAY_ORIGIN_LIFT, 4.0)
        );
        assert_eq!(request.reach_lane().to_bits(), 4.8_f32.to_bits());
        // the birth surface culls the fan: fewer directions than nominal,
        // and never zero
        let cast = request.directions().count();
        assert!(cast > 0 && cast < ray_fan::RAYS);
    }

    /// An airborne sound culls nothing — the whole fan is cast.
    #[test]
    fn an_airborne_request_casts_the_whole_fan() {
        let request = checked(ReflectionRequest {
            normal: Vector3::ZERO,
            ..tap()
        });
        assert_eq!(request.directions().count(), ray_fan::RAYS);
        assert_eq!(request.ray_origin(), request.request().at);
    }

    /// A negative budget answers nothing, exactly as the engine's own
    /// clamp does — and says the cells were found and then dropped.
    #[test]
    fn a_refused_budget_drops_every_cell() {
        let request = ReflectionRequest {
            max_echoes: -3,
            ..tap()
        };
        let request = checked(request);
        let e = explain_clustering(&request, 26, &[hit(Vector3::new(6.0, 0.0, 4.0), 3.0)])
            .expect("a zero budget schedules no points");
        assert_eq!(e.budget, 0);
        assert_eq!(e.cells_found, 1);
        assert_eq!(e.dropped_past_budget, 1);
        assert!(e.points.is_empty());
        assert_balanced(&e);
    }

    /// A question about a sound that cannot travel is refused, not
    /// answered with infinities. `at_t = now + d / 0` is `+INF`, and a
    /// negative speed schedules echoes before the sound was made — and
    /// `JSON.stringify` renders both as `null`, so an agent would read a
    /// missing field where there was an error.
    #[test]
    fn a_wavefront_that_cannot_travel_is_refused() {
        for speed in [0.0, -5.5, f64::NAN, f64::INFINITY] {
            let request = ReflectionRequest { speed, ..tap() };
            let error = CheckedReflectionRequest::prepare(request)
                .expect_err("an invalid speed must not enter the fan");
            assert_eq!(error.reason(), REFUSED_SPEED, "speed {speed}");
            assert!(matches!(error.field(), "speed" | "end"));
        }
    }

    /// The fan's reach derives from the range, so a range that cannot
    /// reach is refused under its own name.
    #[test]
    fn a_range_that_cannot_reach_is_refused() {
        for max_r in [0.0, -6.0, f64::NAN, f64::INFINITY] {
            let request = ReflectionRequest { max_r, ..tap() };
            let error = CheckedReflectionRequest::prepare(request)
                .expect_err("an invalid range must not enter the fan");
            assert_eq!(error.reason(), REFUSED_MAX_R, "max_r {max_r}");
            assert_eq!(error.field(), "max_r");
        }
    }

    /// A non-finite clock or origin poisons every number downstream.
    #[test]
    fn a_non_finite_clock_or_origin_is_refused() {
        assert_eq!(
            CheckedReflectionRequest::prepare(ReflectionRequest {
                now: f64::NAN,
                ..tap()
            })
            .expect_err("a nonfinite clock must not enter the fan")
            .reason(),
            REFUSED_CLOCK
        );
        assert_eq!(
            CheckedReflectionRequest::prepare(ReflectionRequest {
                at: Vector3::new(f32::NAN, 0.0, 0.0),
                ..tap()
            })
            .expect_err("a nonfinite origin must not enter the fan")
            .reason(),
            REFUSED_GEOMETRY
        );
        assert_eq!(
            CheckedReflectionRequest::prepare(ReflectionRequest {
                normal: Vector3::new(0.0, f32::INFINITY, 0.0),
                ..tap()
            })
            .expect_err("a nonfinite normal must not enter the fan")
            .reason(),
            REFUSED_GEOMETRY
        );
    }

    #[test]
    fn checked_reflection_geometry_refuses_nan_and_overflowing_normals_before_fan_arithmetic() {
        for (normal, field) in [
            (Vector3::new(f32::NAN, 0.0, 0.0), "normal.x"),
            (Vector3::new(f32::MIN_POSITIVE, 0.0, 0.0), "normal"),
            (Vector3::new(f32::MAX, -f32::MAX, f32::MAX), "normal"),
        ] {
            let error = CheckedReflectionRequest::prepare(ReflectionRequest { normal, ..tap() })
                .expect_err("poisoned normal arithmetic must not reach the fan");
            assert_eq!(error.reason(), REFUSED_GEOMETRY);
            assert_eq!(error.field(), field);
        }

        let checked = CheckedReflectionRequest::prepare(tap())
            .expect("the shipped tap geometry must be fully representable");
        assert!(checked.ray_origin().is_finite());
        assert!(checked.reach_lane().is_finite());
        assert!(checked.reach_lane() > 0.0);
        let directions: Vec<Vector3> = checked.directions().collect();
        assert!(!directions.is_empty());
        for direction in directions {
            assert!(direction.dot(tap().normal).is_finite());
            assert!((checked.ray_origin() + direction * checked.reach_lane()).is_finite());
        }
    }

    #[test]
    fn observer_ledger_and_explanation_keep_one_checked_geometry_image() {
        let checked = CheckedReflectionRequest::prepare(tap())
            .expect("the observer may book only a fully checked request");
        let expected = checked.clone();
        let mut ledger = ExplanationLedger::default();

        let id = ledger.request(checked);
        let mut taken = ledger.take_requests();
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].0, id);
        assert_eq!(taken[0].1, expected);
        let explanation = explain_clustering(&taken.remove(0).1, 4, &[])
            .expect("the exact checked image reaches explanation");
        assert_eq!(explanation.rays_cast, 4);
    }

    #[test]
    fn checked_engine_hit_refuses_poisoned_position_normal_and_distance_before_clustering() {
        let request = CheckedReflectionRequest::prepare(tap()).unwrap();
        let origin = request.ray_origin();
        let reach = request.reach_lane();
        let valid = CheckedRayHit::prepare(origin + Vector3::RIGHT, Vector3::LEFT, &request)
            .expect("a finite in-segment surface hit is admissible");
        assert_eq!(valid.ray_hit().dist.to_bits(), 1.0_f32.to_bits());

        for (position, normal, expected_field) in [
            (
                Vector3::new(f32::NAN, 0.0, 0.0),
                Vector3::LEFT,
                "hit.position.x",
            ),
            (
                Vector3::new(1_000_002.0_f32.next_up(), 0.0, 0.0),
                Vector3::LEFT,
                "hit.position.x",
            ),
            (
                origin + Vector3::RIGHT,
                Vector3::new(f32::NAN, 0.0, 0.0),
                "hit.normal.x",
            ),
            (
                origin + Vector3::RIGHT,
                Vector3::new(f32::MAX, -f32::MAX, f32::MAX),
                "hit.normal",
            ),
            (
                origin + Vector3::RIGHT,
                Vector3::new(f32::MIN_POSITIVE, 0.0, 0.0),
                "hit.normal",
            ),
            (
                origin + Vector3::RIGHT * reach.next_up(),
                Vector3::LEFT,
                "hit.distance",
            ),
        ] {
            let error = CheckedRayHit::prepare(position, normal, &request)
                .expect_err("malformed physics-server output must refuse the whole fan");
            assert_eq!(error.field(), expected_field);
        }

        let boundary_request = CheckedReflectionRequest::prepare(ReflectionRequest {
            at: Vector3::new(999_997.25, 0.0, 0.0),
            normal: Vector3::ZERO,
            ..tap()
        })
        .expect("the request itself stays inside the closed origin envelope");
        let error = CheckedRayHit::prepare(
            Vector3::new(1_000_002.0, 0.0, 0.0),
            Vector3::new(2.0, 0.0, 0.0),
            &boundary_request,
        )
        .expect_err("the lifted echo origin must stay inside the same envelope");
        assert_eq!(error.field(), "hit.echo_origin.x");
    }

    /// The shipped cane tap passes, and every number an accepted request
    /// produces is finite — which is what keeps `null` off the wire.
    #[test]
    fn an_accepted_request_produces_only_finite_numbers() {
        let request = checked_tap();
        let e = explain_clustering(&request, 14, &[hit(Vector3::new(6.0, 0.0, 4.0), 3.0)])
            .expect("the accepted request schedules every point");
        assert!(e.reach.is_finite());
        for point in &e.points {
            assert!(point.dist.is_finite());
            assert!(point.at_t.is_finite());
            assert!(point.gain_fraction.is_finite());
        }
    }

    /// The checked engine-hit owner prevents this poisoned point in the live
    /// path, but the explanation remains total over its pure hit slice: an
    /// unrepresentable appointment refuses the whole answer rather than
    /// silently dropping one cluster and presenting an incomplete ledger.
    #[test]
    fn an_unrepresentable_appointment_refuses_the_whole_explanation() {
        let request = checked_tap();
        let refusal = explain_clustering(
            &request,
            1,
            &[hit(Vector3::new(1_000_003.0, 0.0, 0.0), 4.8)],
        )
        .expect_err("a failed appointment must refuse the explanation");
        assert_eq!(
            refusal,
            "reflection explanation refused: echo appointment is not representable"
        );
    }

    fn explanation() -> Answer {
        Answer::Explained(Box::new(
            explain_clustering(&checked_tap(), 4, &[]).expect("an empty fan is representable"),
        ))
    }

    /// The three-state contract: pending until the frame runs, the answer
    /// exactly once, then nothing.
    #[test]
    fn a_request_is_pending_then_answered_once_then_unknown() {
        let mut ledger = ExplanationLedger::default();
        let id = ledger.request(checked_tap());
        assert_eq!(ledger.collect(id), Collected::Pending);
        let taken = ledger.take_requests();
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].0, id);
        // the question has left the book and no answer is filed yet: an
        // id in that gap is unknown, never a pending that never resolves.
        // Nothing outside the physics tick can observe the gap — the
        // caster takes and files inside one call — but the book must not
        // depend on that for its honesty.
        assert_eq!(ledger.collect(id), Collected::Unknown);
        ledger.answer(id, explanation());
        assert!(matches!(ledger.collect(id), Collected::Ready(_)));
        assert_eq!(ledger.collect(id), Collected::Unknown);
    }

    /// Ids are unique and never zero, so a dropped return value cannot
    /// collect somebody else's answer.
    #[test]
    fn ids_are_unique_and_start_at_one() {
        let mut ledger = ExplanationLedger::default();
        let first = ledger.request(checked_tap());
        let second = ledger.request(checked_tap());
        assert_eq!(first, 1);
        assert_eq!(second, 2);
        assert_eq!(ledger.collect(0), Collected::Unknown);
    }

    /// A loop that asks and never collects must not grow inside a running
    /// game: the oldest answer ages out, and reads as unknown rather than
    /// as an empty fan.
    #[test]
    fn uncollected_answers_age_out_rather_than_growing_forever() {
        let mut ledger = ExplanationLedger::default();
        let mut ids = Vec::new();
        for _ in 0..(EXPLANATION_MEMORY + 2) {
            let id = ledger.request(checked_tap());
            ledger.take_requests();
            ledger.answer(id, explanation());
            ids.push(id);
        }
        assert_eq!(ledger.collect(ids[0]), Collected::Unknown);
        assert_eq!(ledger.collect(ids[1]), Collected::Unknown);
        assert!(matches!(ledger.collect(ids[2]), Collected::Ready(_)));
        assert!(matches!(
            ledger.collect(ids[ids.len() - 1]),
            Collected::Ready(_)
        ));
    }

    /// The waiting half is bounded by the same rule as the answered one.
    /// An observer whose physics frame never runs — outside a tree, or
    /// paused — would otherwise accumulate questions forever inside a
    /// running game.
    #[test]
    fn waiting_questions_age_out_rather_than_growing_forever() {
        let mut ledger = ExplanationLedger::default();
        let ids: Vec<i64> = (0..(EXPLANATION_MEMORY + 2))
            .map(|_| ledger.request(checked_tap()))
            .collect();
        assert_eq!(ledger.collect(ids[0]), Collected::Unknown);
        assert_eq!(ledger.collect(ids[1]), Collected::Unknown);
        assert_eq!(ledger.collect(ids[2]), Collected::Pending);
        assert_eq!(ledger.take_requests().len(), EXPLANATION_MEMORY);
    }

    /// A question answered the moment it is asked leaves no phantom
    /// waiting behind it: the boundary refuses an impossible request, or
    /// one with no world to cast in, without a frame ever running.
    #[test]
    fn answering_at_once_withdraws_the_waiting_question() {
        let mut ledger = ExplanationLedger::default();
        let id = ledger.request(checked_tap());
        ledger.answer(id, Answer::Refused("no space"));
        assert!(ledger.take_requests().is_empty());
        assert_eq!(
            ledger.collect(id),
            Collected::Ready(Answer::Refused("no space"))
        );
    }

    /// A refusal survives the book intact: the frame ran, the cast could
    /// not happen, and the collector is told so instead of being handed a
    /// fan of no hits.
    #[test]
    fn a_refusal_is_carried_through_as_a_refusal() {
        let mut ledger = ExplanationLedger::default();
        let id = ledger.request(checked_tap());
        ledger.take_requests();
        ledger.answer(id, Answer::Refused("no space"));
        assert_eq!(
            ledger.collect(id),
            Collected::Ready(Answer::Refused("no space"))
        );
    }
}
