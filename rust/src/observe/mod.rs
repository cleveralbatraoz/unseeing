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

use godot::builtin::{Basis, Vector3, Vector4};

use self::evict::{EvictionPlan, explain_eviction};
use self::pool::{SlotObservation, SlotState, slots};
use crate::pulse_pool::PulsePool;
use crate::sight::MAXW;

/// One sound source as an agent reads it. Built at the boundary, where
/// the source nodes live; carried through here unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceObservation {
    pub name: String,
    pub position: Vector3,
    pub volume: f64,
    pub reach: f64,
    /// Walls between the eye and this source's hub.
    pub walls_to_eye: u32,
    /// The standing image floor after muffling — the `u_source_floor`
    /// instance uniform this source is pushed.
    pub source_floor: f64,
    pub slot_pressure: f64,
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
    pub sources: Vec<SourceObservation>,
    pub wall_rects: Vec<Vector4>,
    /// True when the table has reached the shader's ceiling, so walls may
    /// have been dropped. Loud by construction.
    pub wall_truncated: bool,
    pub camera: Vector3,
    pub camera_basis: Basis,
}

/// Compose one frame's observation from parts the boundary supplies.
///
/// Pure: every argument is plain data. The boundary
/// (`crate::nodes::observer`) is what knows how to obtain them.
#[must_use]
pub fn frame(
    pool: &PulsePool,
    now: f64,
    flick: f64,
    sources: Vec<SourceObservation>,
    wall_rects: Vec<Vector4>,
    camera: Vector3,
    camera_basis: Basis,
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
        sources,
        wall_truncated: wall_rects.len() >= MAXW,
        wall_rects,
        camera,
        camera_basis,
    }
}

#[cfg(test)]
mod tests {
    use super::evict::EvictionRule;
    use super::pool::SlotState;
    use super::*;
    use crate::pulse_pool::PulsePool;
    use godot::builtin::{Basis, Vector3, Vector4};

    fn empty_frame(pool: &PulsePool, now: f64) -> FrameObservation {
        frame(
            pool,
            now,
            1.0,
            Vec::new(),
            Vec::new(),
            Vector3::ZERO,
            Basis::IDENTITY,
        )
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
        let short = frame(
            &pool,
            0.0,
            1.0,
            Vec::new(),
            vec![rect; 31],
            Vector3::ZERO,
            Basis::IDENTITY,
        );
        let full = frame(
            &pool,
            0.0,
            1.0,
            Vec::new(),
            vec![rect; 32],
            Vector3::ZERO,
            Basis::IDENTITY,
        );
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
}
