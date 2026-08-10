//! Which slot the next sound would claim, and by which rule.
//!
//! Eviction happens between frames and overwrites its own evidence, so no
//! snapshot can show it. This re-derives `PulsePool::emit`'s selection
//! independently — it must never CALL emit, or asking the question would
//! answer it by changing it.

use crate::observe::pool::{SlotState, slots};
use crate::pulse_pool::{MAXP, PulsePool};

/// Why a slot was chosen. Mirrors the three-branch preference in
/// `PulsePool::emit`, plus the unreachable landing spot it falls back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionRule {
    /// A slot past its end time (or one that never lived).
    Expired,
    /// The oldest slot of kind >= 2 — footsteps and source hums, which
    /// recur and are therefore the cheapest live thing to lose.
    OldestRecurring,
    /// The oldest slot of any kind: nothing cheap was available.
    OldestOverall,
    /// Unreachable unless every birth time is non-finite; `emit` lands on
    /// the last slot, so this reports the same.
    Fallback,
}

/// The slot the next `emit` would take, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvictionPlan {
    pub slot: usize,
    pub rule: EvictionRule,
    /// The kind currently occupying that slot — what would be lost.
    pub victim_kind: i32,
}

/// Predict the next eviction as of `now`.
///
/// Total on any pool state. Scans in slot order and stops at the first
/// expired slot, exactly as `emit` does — the ORDER matters, because an
/// expired slot later in the pool must not win over an earlier one.
#[must_use]
pub fn explain_eviction(pool: &PulsePool, now: f64) -> EvictionPlan {
    let obs = slots(pool, now);
    let mut oldest_recurring: Option<usize> = None;
    let mut oldest_overall: Option<usize> = None;
    let mut t_recurring = f64::INFINITY;
    let mut t_overall = f64::INFINITY;
    for s in &obs {
        if s.state != SlotState::Live {
            return plan(&obs, s.index, EvictionRule::Expired);
        }
        if s.kind >= 2 && s.birth < t_recurring {
            t_recurring = s.birth;
            oldest_recurring = Some(s.index);
        }
        if s.birth < t_overall {
            t_overall = s.birth;
            oldest_overall = Some(s.index);
        }
    }
    if let Some(i) = oldest_recurring {
        return plan(&obs, i, EvictionRule::OldestRecurring);
    }
    if let Some(i) = oldest_overall {
        return plan(&obs, i, EvictionRule::OldestOverall);
    }
    plan(&obs, MAXP - 1, EvictionRule::Fallback)
}

fn plan(obs: &[super::pool::SlotObservation], slot: usize, rule: EvictionRule) -> EvictionPlan {
    EvictionPlan {
        slot,
        rule,
        victim_kind: obs[slot].kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pulse_pool::{MAXP, PulsePool};
    use godot::builtin::Vector3;

    /// The prediction is worthless unless it matches what emit() actually
    /// does. Fill the pool so the interesting rule fires, predict, then
    /// emit for real and check the slot that changed is the predicted one.
    /// This compares two independent implementations, so it is not a
    /// mirror assertion — the prediction never calls emit().
    fn assert_prediction_matches_reality(mut pool: PulsePool, now: f64) {
        let plan = explain_eviction(&pool, now);
        let before = *pool.pos();
        let marker = Vector3::new(999.0, 999.0, 999.0);
        pool.emit_omni(0, marker, 6.0, 5.5, 1.0, now).unwrap();
        let changed: Vec<usize> = (0..MAXP).filter(|i| pool.pos()[*i] != before[*i]).collect();
        assert_eq!(changed, vec![plan.slot], "predicted {:?}", plan);
    }

    /// An expired slot is claimed before anything living is touched.
    #[test]
    fn expired_slots_are_claimed_first() {
        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::new(1.0, 0.0, 0.0), 1.6, 4.0, 0.8, 0.0)
            .unwrap();
        pool.emit_omni(0, Vector3::new(2.0, 0.0, 0.0), 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let plan = explain_eviction(&pool, 5.0);
        assert_eq!(plan.slot, 0);
        assert_eq!(plan.rule, EvictionRule::Expired);
        assert_prediction_matches_reality(pool, 5.0);
    }

    /// A full pool of live taps with one hum: the hum goes, because it
    /// recurs and a cane tap does not. Slot 7 holds the hum and is NOT the
    /// oldest slot — so this distinguishes the recurring rule from a plain
    /// oldest-wins rule.
    #[test]
    fn a_recurring_hum_is_sacrificed_before_any_tap() {
        let mut pool = PulsePool::new();
        for i in 0..MAXP {
            let kind = if i == 7 { 3 } else { 0 };
            let at = Vector3::new(i as f32, 0.0, 0.0);
            pool.emit_omni(kind, at, 6.0, 5.5, 1.0, 100.0 + i as f64 * 0.001)
                .unwrap();
        }
        let plan = explain_eviction(&pool, 100.1);
        assert_eq!(plan.slot, 7);
        assert_eq!(plan.rule, EvictionRule::OldestRecurring);
        assert_eq!(plan.victim_kind, 3);
        assert_prediction_matches_reality(pool, 100.1);
    }

    /// Nothing cheap to sacrifice: 64 live taps, so the oldest tap goes.
    #[test]
    fn a_full_tap_pool_gives_up_its_oldest() {
        let mut pool = PulsePool::new();
        for i in 0..MAXP {
            let at = Vector3::new(i as f32, 0.0, 0.0);
            pool.emit_omni(0, at, 6.0, 5.5, 1.0, 100.0 + i as f64 * 0.001)
                .unwrap();
        }
        let plan = explain_eviction(&pool, 100.1);
        assert_eq!(plan.slot, 0);
        assert_eq!(plan.rule, EvictionRule::OldestOverall);
        assert_eq!(plan.victim_kind, 0);
        assert_prediction_matches_reality(pool, 100.1);
    }

    /// A virgin pool: slot 0 has never lived, so it is Expired-by-sentinel
    /// and claimed first.
    #[test]
    fn a_virgin_pool_claims_slot_zero() {
        let pool = PulsePool::new();
        let plan = explain_eviction(&pool, 0.0);
        assert_eq!(plan.slot, 0);
        assert_eq!(plan.rule, EvictionRule::Expired);
        assert_prediction_matches_reality(pool, 0.0);
    }
}
