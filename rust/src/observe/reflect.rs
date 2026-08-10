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
use crate::ray_fan;

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

impl ReflectionRequest {
    /// Where the rays actually start: lifted off the birth surface, or
    /// they would begin inside the struck collider and answer from places
    /// the wave never reached.
    #[must_use]
    pub fn ray_origin(&self) -> Vector3 {
        self.at + self.normal * clustering::RAY_ORIGIN_LIFT
    }

    /// How far a ray of this fan may travel. The single commonest answer
    /// to "why did that wall stay silent" is that it stands past this.
    #[must_use]
    pub fn reach(&self) -> f64 {
        clustering::ray_length(self.max_r)
    }

    /// The directions this request would cast — the nominal fan culled to
    /// the hemisphere in front of the birth surface.
    pub fn directions(&self) -> impl Iterator<Item = Vector3> {
        ray_fan::fan_directions(self.normal)
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
#[must_use]
pub fn explain_clustering(
    request: &ReflectionRequest,
    rays_cast: usize,
    hits: &[RayHit],
) -> ReflectionExplanation {
    let budget = clustering::echo_budget(request.max_echoes);
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
    ReflectionExplanation {
        at: request.at,
        origin: request.ray_origin(),
        normal: request.normal,
        reach: request.reach(),
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
        points: appointments(request, &kept),
    }
}

/// When each surviving point would answer, and how loudly.
///
/// Scheduled into a SCRATCH echo book so the timing and falloff laws are
/// the queue's own rather than a second copy of them — and so that asking
/// the question cannot possibly reach the queue the game drains.
fn appointments(
    request: &ReflectionRequest,
    kept: &[clustering::SurfaceHit],
) -> Vec<ClusteredPoint> {
    let mut scratch = EchoQueue::new();
    for hit in kept {
        scratch.schedule(request.now, hit.dist, hit.point, UNIT_GAIN, request.speed);
    }
    kept.iter()
        .zip(scratch.pending())
        .map(|(hit, echo)| ClusteredPoint {
            point: hit.point,
            dist: hit.dist,
            at_t: echo.at_t,
            gain_fraction: echo.gain,
        })
        .collect()
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
#[derive(Debug, Default)]
pub struct ExplanationLedger {
    next_id: i64,
    waiting: Vec<(i64, ReflectionRequest)>,
    ready: Vec<(i64, Answer)>,
}

impl ExplanationLedger {
    /// Book a question and return its id. Ids start at 1, so 0 is never a
    /// valid one — a caller that dropped the return value cannot collect
    /// somebody else's answer by accident.
    pub fn request(&mut self, request: ReflectionRequest) -> i64 {
        self.next_id = self.next_id.saturating_add(1);
        self.waiting.push((self.next_id, request));
        self.next_id
    }

    /// Every question waiting on a physics frame, removed from the book.
    /// The caster answers them; anything it fails to answer is simply
    /// forgotten, which reads as an unknown id rather than a lie.
    pub fn take_requests(&mut self) -> Vec<(i64, ReflectionRequest)> {
        std::mem::take(&mut self.waiting)
    }

    /// File an answer, ageing out the oldest once the book is full.
    pub fn answer(&mut self, id: i64, answer: Answer) {
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
        let e = explain_clustering(&tap(), 20, &hits);
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
        let e = explain_clustering(&tap(), 26, &hits);
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
        let e = explain_clustering(&request, 26, &hits);
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
        let e = explain_clustering(&tap(), 26, &hits);
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
        let e = explain_clustering(&tap(), 17, &[]);
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
        let e = explain_clustering(&tap(), 26, &[hit(Vector3::new(6.0, 0.0, 4.0), 2.75)]);
        let point = e.points[0];
        assert!((point.at_t - (10.0 + 2.75 / 5.5)).abs() < 1e-12);
        assert!((point.gain_fraction - 0.55 / (1.0 + 2.75 * 0.4)).abs() < 1e-12);
    }

    /// Where the rays start and how far they go: lifted off the birth
    /// surface, reaching 0.8 of the wave's range and never past 6 m.
    #[test]
    fn the_fan_starts_off_the_surface_and_reaches_the_clamped_length() {
        let request = tap();
        assert_eq!(
            request.ray_origin(),
            Vector3::new(3.0, clustering::RAY_ORIGIN_LIFT, 4.0)
        );
        assert!((request.reach() - 4.8).abs() < 1e-9);
        // the birth surface culls the fan: fewer directions than nominal,
        // and never zero
        let cast = request.directions().count();
        assert!(cast > 0 && cast < ray_fan::RAYS);
    }

    /// An airborne sound culls nothing — the whole fan is cast.
    #[test]
    fn an_airborne_request_casts_the_whole_fan() {
        let request = ReflectionRequest {
            normal: Vector3::ZERO,
            ..tap()
        };
        assert_eq!(request.directions().count(), ray_fan::RAYS);
        assert_eq!(request.ray_origin(), request.at);
    }

    /// A negative budget answers nothing, exactly as the engine's own
    /// clamp does — and says the cells were found and then dropped.
    #[test]
    fn a_refused_budget_drops_every_cell() {
        let request = ReflectionRequest {
            max_echoes: -3,
            ..tap()
        };
        let e = explain_clustering(&request, 26, &[hit(Vector3::new(6.0, 0.0, 4.0), 3.0)]);
        assert_eq!(e.budget, 0);
        assert_eq!(e.cells_found, 1);
        assert_eq!(e.dropped_past_budget, 1);
        assert!(e.points.is_empty());
        assert_balanced(&e);
    }

    fn explanation() -> Answer {
        Answer::Explained(Box::new(explain_clustering(&tap(), 4, &[])))
    }

    /// The three-state contract: pending until the frame runs, the answer
    /// exactly once, then nothing.
    #[test]
    fn a_request_is_pending_then_answered_once_then_unknown() {
        let mut ledger = ExplanationLedger::default();
        let id = ledger.request(tap());
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
        let first = ledger.request(tap());
        let second = ledger.request(tap());
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
            let id = ledger.request(tap());
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

    /// A refusal survives the book intact: the frame ran, the cast could
    /// not happen, and the collector is told so instead of being handed a
    /// fan of no hits.
    #[test]
    fn a_refusal_is_carried_through_as_a_refusal() {
        let mut ledger = ExplanationLedger::default();
        let id = ledger.request(tap());
        ledger.take_requests();
        ledger.answer(id, Answer::Refused("no space"));
        assert_eq!(
            ledger.collect(id),
            Collected::Ready(Answer::Refused("no space"))
        );
    }
}
