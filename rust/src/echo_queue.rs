//! The echo appointment book — reflections waiting for their wavefront.
//! Mirrored exactly from pulses.gd: `emit_reflecting` schedules a
//! [`PendingEcho`] for every answering surface point, and `_drain_echoes`
//! fires the ones whose moment has come. A reflection is an appointment,
//! not an animation: it fires at the exact instant the primary wavefront
//! reaches its surface point (t = distance / speed), never a frame early.
//!
//! Precision law, pinned from the original: `dist` arrives as f32 (a
//! Vector3 length, computed single-precision) and is widened to f64 before
//! any arithmetic — exactly where GDScript widened it into its 64-bit
//! floats. Times and gains then stay f64 end to end.

use godot::builtin::Vector3;

use crate::reproduce::RestoreValueError;

/// The pool kind a fired echo re-enters as: 1, ECHO — a secondary
/// reflection. Echoes never spawn further echoes.
pub const ECHO_KIND: i32 = 1;

/// Max radius of a fired echo's own wave — small on purpose: an echo
/// reveals the surface that answered, not the whole room again.
pub const ECHO_MAX_R: f64 = 2.2;

/// Speed of a fired echo's wave — the cane tap's own 5.5 m/s, so primary
/// and answer read as the same sound family.
pub const ECHO_SPEED: f64 = 5.5;

/// A scheduled reflection: a surface point that answers at the exact
/// moment the primary wavefront reaches it. Field widths mirror the
/// GDScript `Echo` class — `at_t` and `gain` were GDScript floats (f64),
/// `pos` a Vector3 (f32 lanes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingEcho {
    /// The appointment: absolute time the echo fires.
    pub at_t: f64,
    /// The answering point, already nudged off its surface by the caller.
    pub pos: Vector3,
    /// Loudness after the distance falloff, ready to pack as-is.
    pub gain: f64,
}

/// Scheduled reflections, ordered by discovery — the mirror of pulses.gd's
/// `_echoes` array. Bounded in practice by `max_echoes` per primary sound,
/// so the O(n) removals of the drain walk stay trivially cheap.
#[derive(Debug, Clone, Default)]
pub struct EchoQueue {
    pending: Vec<PendingEcho>,
}

#[derive(Debug, Clone)]
pub struct PreparedEchoQueue(EchoQueue);

impl EchoQueue {
    pub fn prepare_restore(
        pending: Vec<PendingEcho>,
    ) -> Result<PreparedEchoQueue, RestoreValueError> {
        for (index, echo) in pending.iter().enumerate() {
            for (field, value) in [
                ("at_t", echo.at_t),
                ("pos.x", f64::from(echo.pos.x)),
                ("pos.y", f64::from(echo.pos.y)),
                ("pos.z", f64::from(echo.pos.z)),
                ("gain", echo.gain),
            ] {
                if !value.is_finite() {
                    return Err(RestoreValueError::new(
                        format!("echoes[{index}].{field}"),
                        "must be finite",
                    ));
                }
            }
            if echo.at_t < 0.0 {
                return Err(RestoreValueError::new(
                    format!("echoes[{index}].at_t"),
                    "must be non-negative",
                ));
            }
            if echo.gain < 0.0 || echo.gain > 1.0 {
                return Err(RestoreValueError::new(
                    format!("echoes[{index}].gain"),
                    "must be in 0..=1",
                ));
            }
        }
        Ok(PreparedEchoQueue(Self::from_pending(pending)))
    }

    #[must_use]
    pub fn from_prepared(value: PreparedEchoQueue) -> Self {
        value.0
    }

    /// An empty book: no appointments.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule the reflection born `dist` meters from the primary sound's
    /// ray origin. The wave equation sets the appointment
    /// (`at_t = now + d / speed`) and the distance law sets the loudness
    /// (`gain * 0.55 / (1 + 0.4 d)`) — both verbatim from
    /// `emit_reflecting`, operation order included.
    pub fn schedule(&mut self, now: f64, dist: f32, pos: Vector3, gain: f64, speed: f64) {
        let d = f64::from(dist);
        self.pending.push(PendingEcho {
            at_t: now + d / speed,
            pos,
            gain: gain * 0.55 / (1.0 + d * 0.4),
        });
    }

    /// Fire every reflection whose moment has come: `at_t <= now`, so the
    /// boundary instant itself fires — GDScript's `<=`, pinned. The caller
    /// re-emits each fired echo into the pool as [`ECHO_KIND`] with
    /// [`ECHO_MAX_R`] and [`ECHO_SPEED`], born at drain time.
    ///
    /// Order is the original's, pinned: `_drain_echoes` walks the array by
    /// REVERSE index and emits as it removes, so later-scheduled echoes
    /// fire first and the survivors keep their discovery order. Pool slot
    /// assignment (and so the shader arrays) depends on this order — it
    /// must not drift.
    pub fn drain(&mut self, now: f64) -> Vec<PendingEcho> {
        let mut fired = Vec::new();
        let mut i = self.pending.len();
        while i > 0 {
            i -= 1;
            if self.pending[i].at_t <= now {
                fired.push(self.pending.remove(i));
            }
        }
        fired
    }

    /// The scheduled reflections still waiting, in discovery order —
    /// observable for tests and debug, like `pending_echoes()` was.
    #[must_use]
    pub fn pending(&self) -> &[PendingEcho] {
        &self.pending
    }

    /// Reflections scheduled but not yet fired — `pending_echo_count()`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    /// True when every appointment has been kept.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// The whole book, verbatim, in discovery order — the order the
    /// pinned drain walks. Never sorted: slot assignment depends on it.
    #[must_use]
    pub fn capture(&self) -> Vec<PendingEcho> {
        self.pending.clone()
    }

    /// A book rebuilt from a capture. The Vec is taken as-is — restoring
    /// through schedule() would re-apply the falloff and re-narrow the
    /// distance through f32, neither of which round-trips.
    #[must_use]
    pub fn from_pending(pending: Vec<PendingEcho>) -> Self {
        Self { pending }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_restore_rejects_poisoned_echo_appointment() {
        let pending = vec![PendingEcho {
            at_t: 4.0,
            pos: Vector3::new(1.0, f32::INFINITY, 2.0),
            gain: 0.5,
        }];
        let error = EchoQueue::prepare_restore(pending).expect_err("poison must be refused");
        assert_eq!(error.path, "echoes[0].pos.y");

        let pending = vec![PendingEcho {
            at_t: -0.25,
            pos: Vector3::ZERO,
            gain: 0.5,
        }];
        let error = EchoQueue::prepare_restore(pending)
            .expect_err("an appointment before the simulation epoch must be refused");
        assert_eq!(error.path, "echoes[0].at_t");
    }

    /// An echo is an appointment: drain must not fire it a moment early,
    /// and must fire it the instant at_t arrives (<=, boundary included).
    #[test]
    fn fires_exactly_at_its_appointment() {
        let mut q = EchoQueue::new();
        q.schedule(10.0, 2.2, Vector3::ONE, 1.0, 5.5);
        let at_t = q.pending()[0].at_t;
        // the wave equation, verbatim: now + widened(dist) / speed
        assert_eq!(at_t, 10.0 + f64::from(2.2f32) / 5.5);
        assert!(q.drain(at_t - 0.01).is_empty());
        assert_eq!(q.len(), 1); // too early: still pending
        let fired = q.drain(at_t);
        assert_eq!(fired.len(), 1); // the boundary instant itself fires
        assert!(q.is_empty());
    }

    /// Drain removes what fired and only what fired: the later appointment
    /// survives, untouched and still in order.
    #[test]
    fn drain_removes_fired_only() {
        let mut q = EchoQueue::new();
        q.schedule(0.0, 1.0, Vector3::new(1.0, 0.0, 0.0), 1.0, 5.5); // at_t ~ 0.18
        q.schedule(0.0, 5.0, Vector3::new(2.0, 0.0, 0.0), 1.0, 5.5); // at_t ~ 0.91
        let fired = q.drain(0.5);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].pos, Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(q.len(), 1);
        assert_eq!(q.pending()[0].pos, Vector3::new(2.0, 0.0, 0.0));
    }

    /// The original emits in reverse index order — later-scheduled echoes
    /// fire first. Slot assignment depends on it, so it is pinned, and it
    /// is deterministic: same schedule, same drain, same order, always.
    #[test]
    fn drain_order_is_reverse_and_deterministic() {
        let a = Vector3::new(1.0, 0.0, 0.0);
        let b = Vector3::new(2.0, 0.0, 0.0);
        let c = Vector3::new(3.0, 0.0, 0.0);
        let mut q = EchoQueue::new();
        for p in [a, b, c] {
            q.schedule(0.0, 1.0, p, 1.0, 5.5);
        }
        let fired: Vec<Vector3> = q.drain(1.0).iter().map(|e| e.pos).collect();
        assert_eq!(fired, vec![c, b, a]);
        // and again, identically — no hidden order anywhere
        let mut q2 = EchoQueue::new();
        for p in [a, b, c] {
            q2.schedule(0.0, 1.0, p, 1.0, 5.5);
        }
        let fired2: Vec<Vector3> = q2.drain(1.0).iter().map(|e| e.pos).collect();
        assert_eq!(fired, fired2);
    }

    /// Survivors of a partial drain keep their discovery order, and a later
    /// drain fires them in reverse of what remains — the exact behavior of
    /// repeated `_drain_echoes` calls.
    #[test]
    fn partial_drain_keeps_survivor_order() {
        let mut q = EchoQueue::new();
        q.schedule(0.0, 1.0, Vector3::new(1.0, 0.0, 0.0), 1.0, 1.0); // at_t ~ 1.0
        q.schedule(0.0, 3.0, Vector3::new(2.0, 0.0, 0.0), 1.0, 1.0); // at_t ~ 3.0
        q.schedule(0.0, 5.0, Vector3::new(3.0, 0.0, 0.0), 1.0, 1.0); // at_t ~ 5.0
        assert_eq!(q.drain(1.5).len(), 1);
        let survivors: Vec<Vector3> = q.pending().iter().map(|e| e.pos).collect();
        assert_eq!(
            survivors,
            vec![Vector3::new(2.0, 0.0, 0.0), Vector3::new(3.0, 0.0, 0.0)]
        );
        let fired: Vec<Vector3> = q.drain(10.0).iter().map(|e| e.pos).collect();
        assert_eq!(
            fired,
            vec![Vector3::new(3.0, 0.0, 0.0), Vector3::new(2.0, 0.0, 0.0)]
        );
    }

    /// The distance law, sampled by hand against the GDScript arithmetic:
    /// gain * 0.55 / (1 + 0.4 d). Exact equality holds because the module
    /// repeats the operation order verbatim in f64.
    #[test]
    fn gain_follows_the_distance_law() {
        let mut q = EchoQueue::new();
        q.schedule(0.0, 0.0, Vector3::ZERO, 1.0, 5.5); // d = 0: no falloff yet
        q.schedule(0.0, 2.5, Vector3::ZERO, 1.0, 5.5); // 0.55 / 2.0
        q.schedule(0.0, 3.0, Vector3::ZERO, 0.8, 5.5); // 0.44 / 2.2
        assert_eq!(q.pending()[0].gain, 0.55);
        assert_eq!(q.pending()[1].gain, 1.0 * 0.55 / (1.0 + 2.5 * 0.4));
        assert!((q.pending()[1].gain - 0.275).abs() < 1e-12);
        assert_eq!(q.pending()[2].gain, 0.8 * 0.55 / (1.0 + 3.0 * 0.4));
        assert!((q.pending()[2].gain - 0.2).abs() < 1e-12);
    }

    /// The appointment follows the wave equation for any speed: the echo
    /// test suite derives d back from at_t as (at_t - now) * speed and it
    /// must round-trip.
    #[test]
    fn appointment_follows_the_wave_equation() {
        let mut q = EchoQueue::new();
        let dist = 3.7f32;
        q.schedule(10.0, dist, Vector3::ZERO, 1.0, 5.5);
        let at_t = q.pending()[0].at_t;
        let d_back = (at_t - 10.0) * 5.5;
        assert!((d_back - f64::from(dist)).abs() < 1e-12);
    }

    /// A restored book drains in the ORIGINAL's order — the pinned
    /// reverse-index walk over discovery order, which pool slot
    /// assignment depends on. Appointments deliberately NOT in at_t
    /// order, so an implementation that sorted would be caught.
    #[test]
    fn a_restored_book_drains_in_the_original_order() {
        let mut book = EchoQueue::new();
        book.schedule(0.0, 5.5, Vector3::new(1.0, 0.0, 0.0), 1.0, 5.5);
        book.schedule(0.0, 2.2, Vector3::new(2.0, 0.0, 0.0), 1.0, 5.5);
        book.schedule(0.0, 4.4, Vector3::new(3.0, 0.0, 0.0), 1.0, 5.5);
        let mut restored = EchoQueue::from_pending(book.capture());
        assert_eq!(restored.pending(), book.pending());
        let fired_original = book.drain(2.0);
        let fired_restored = restored.drain(2.0);
        assert_eq!(fired_restored, fired_original);
        assert_eq!(restored.pending(), book.pending()); // survivors too
    }
}
