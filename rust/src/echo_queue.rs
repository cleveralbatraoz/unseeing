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

use crate::pulse_pool::WaveOrigin;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EchoAppointment {
    pub(crate) now: f64,
    pub(crate) dist: f32,
    pub(crate) pos: Vector3,
    pub(crate) gain: f64,
    pub(crate) speed: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EchoScheduleError {
    field: &'static str,
    rule: &'static str,
}

impl EchoScheduleError {
    fn new(field: &'static str, rule: &'static str) -> Self {
        Self { field, rule }
    }

    pub fn field(self) -> &'static str {
        self.field
    }

    pub fn rule(self) -> &'static str {
        self.rule
    }
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
            WaveOrigin::try_new(echo.pos).map_err(|error| {
                RestoreValueError::new(
                    format!("echoes[{index}].pos.{}", error.axis()),
                    error.rule(),
                )
            })?;
            for (field, value) in [("at_t", echo.at_t), ("gain", echo.gain)] {
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
    pub fn schedule(
        &mut self,
        now: f64,
        dist: f32,
        pos: Vector3,
        gain: f64,
        speed: f64,
    ) -> Result<(), EchoScheduleError> {
        let pos = WaveOrigin::try_new(pos).map_err(|error| {
            let field = match error.axis() {
                "x" => "pos.x",
                "y" => "pos.y",
                _ => "pos.z",
            };
            EchoScheduleError::new(field, error.rule())
        })?;
        for (field, value) in [
            ("now", now),
            ("dist", f64::from(dist)),
            ("gain", gain),
            ("speed", speed),
        ] {
            if !value.is_finite() {
                return Err(EchoScheduleError::new(field, "must be finite"));
            }
        }
        if dist < 0.0 {
            return Err(EchoScheduleError::new("dist", "must be non-negative"));
        }
        if speed <= 0.0 {
            return Err(EchoScheduleError::new("speed", "must be strictly positive"));
        }
        if !(0.0..=1.0).contains(&gain) {
            return Err(EchoScheduleError::new("gain", "must be clamped to 0..=1"));
        }
        let d = f64::from(dist);
        let travel = d / speed;
        if !travel.is_finite() {
            return Err(EchoScheduleError::new("at_t", "travel must be finite"));
        }
        let at_t = now + travel;
        if !at_t.is_finite() {
            return Err(EchoScheduleError::new("at_t", "appointment must be finite"));
        }
        if at_t < 0.0 {
            return Err(EchoScheduleError::new(
                "at_t",
                "appointment must be non-negative",
            ));
        }
        let denominator = 1.0 + d * 0.4;
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(EchoScheduleError::new(
                "gain",
                "distance denominator must be finite and positive",
            ));
        }
        let echo_gain = gain * 0.55 / denominator;
        if !echo_gain.is_finite() || !(0.0..=1.0).contains(&echo_gain) {
            return Err(EchoScheduleError::new(
                "gain",
                "scheduled gain must be finite and in 0..=1",
            ));
        }
        self.pending.push(PendingEcho {
            at_t,
            pos: pos.world(),
            gain: echo_gain,
        });
        Ok(())
    }

    /// Validate a complete reflection fan against a scratch copy. The live
    /// book remains untouched until the caller installs the prepared value,
    /// so one bad appointment cannot leave an earlier echo behind.
    pub(crate) fn prepare_schedule_batch(
        &self,
        appointments: &[EchoAppointment],
    ) -> Result<PreparedEchoQueue, EchoScheduleError> {
        let mut prepared = self.clone();
        for appointment in appointments {
            prepared.schedule(
                appointment.now,
                appointment.dist,
                appointment.pos,
                appointment.gain,
                appointment.speed,
            )?;
        }
        Ok(PreparedEchoQueue(prepared))
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
    fn from_pending(pending: Vec<PendingEcho>) -> Self {
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

    #[test]
    fn prepared_restore_refuses_an_out_of_domain_echo_origin_before_install() {
        let mut live = EchoQueue::new();
        live.schedule(0.0, 1.0, Vector3::ONE, 0.5, 5.5).unwrap();
        let before = live.capture();
        let pending = vec![PendingEcho {
            at_t: 4.0,
            pos: Vector3::new(0.0, 0.0, (-1_000_002.0_f32).next_down()),
            gain: 0.5,
        }];

        let error = EchoQueue::prepare_restore(pending)
            .expect_err("an echo outside the renderer envelope must not prepare");
        assert_eq!(error.path, "echoes[0].pos.z");
        assert_eq!(live.capture(), before);
    }

    #[test]
    fn echo_schedule_refuses_a_nonfinite_appointment_before_queue_mutation() {
        let mut queue = EchoQueue::new();
        let before = queue.capture();
        let error = queue
            .schedule(
                262_144.0,
                f32::MAX,
                Vector3::new(1.0, 2.0, 3.0),
                1.0,
                f64::MIN_POSITIVE,
            )
            .expect_err("a nonfinite appointment must be refused");
        assert_eq!(error.field(), "at_t");
        assert_eq!(error.rule(), "travel must be finite");
        assert_eq!(queue.capture(), before);
    }

    #[test]
    fn echo_schedule_refuses_a_pre_epoch_appointment_before_queue_mutation() {
        let mut queue = EchoQueue::new();
        let before = queue.capture();

        let error = queue
            .schedule(-1.0, 0.0, Vector3::ZERO, 1.0, 5.5)
            .expect_err("an appointment before the simulation epoch must be refused");

        assert_eq!(error.field(), "at_t");
        assert_eq!(error.rule(), "appointment must be non-negative");
        assert_eq!(queue.capture(), before);
    }

    #[test]
    fn echo_schedule_refuses_an_out_of_domain_origin_before_queue_mutation() {
        let mut queue = EchoQueue::new();
        queue
            .schedule(0.0, 1.0, Vector3::new(2.0, 3.0, 4.0), 0.5, 5.5)
            .unwrap();
        let before = queue.capture();

        let error = queue
            .schedule(
                0.0,
                1.0,
                Vector3::new(0.0, 1_000_002.0_f32.next_up(), 0.0),
                0.5,
                5.5,
            )
            .expect_err("an echo outside the renderer envelope must be refused");
        assert_eq!(error.field(), "pos.y");
        assert_eq!(queue.capture(), before);
    }

    #[test]
    fn a_refused_echo_batch_never_partially_mutates_the_live_book() {
        let mut queue = EchoQueue::new();
        queue
            .schedule(0.0, 1.0, Vector3::new(1.0, 0.0, 0.0), 0.5, 5.5)
            .unwrap();
        let before = queue.capture();
        let appointments = [
            EchoAppointment {
                now: 1.0,
                dist: 2.0,
                pos: Vector3::new(2.0, 0.0, 0.0),
                gain: 0.5,
                speed: 5.5,
            },
            EchoAppointment {
                now: 1.0,
                dist: 3.0,
                pos: Vector3::new(1_000_002.0_f32.next_up(), 0.0, 0.0),
                gain: 0.5,
                speed: 5.5,
            },
        ];

        queue
            .prepare_schedule_batch(&appointments)
            .expect_err("one invalid appointment must refuse the complete batch");
        assert_eq!(queue.capture(), before);
    }

    /// An echo is an appointment: drain must not fire it a moment early,
    /// and must fire it the instant at_t arrives (<=, boundary included).
    #[test]
    fn fires_exactly_at_its_appointment() {
        let mut q = EchoQueue::new();
        q.schedule(10.0, 2.2, Vector3::ONE, 1.0, 5.5).unwrap();
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
        q.schedule(0.0, 1.0, Vector3::new(1.0, 0.0, 0.0), 1.0, 5.5)
            .unwrap(); // at_t ~ 0.18
        q.schedule(0.0, 5.0, Vector3::new(2.0, 0.0, 0.0), 1.0, 5.5)
            .unwrap(); // at_t ~ 0.91
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
            q.schedule(0.0, 1.0, p, 1.0, 5.5).unwrap();
        }
        let fired: Vec<Vector3> = q.drain(1.0).iter().map(|e| e.pos).collect();
        assert_eq!(fired, vec![c, b, a]);
        // and again, identically — no hidden order anywhere
        let mut q2 = EchoQueue::new();
        for p in [a, b, c] {
            q2.schedule(0.0, 1.0, p, 1.0, 5.5).unwrap();
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
        q.schedule(0.0, 1.0, Vector3::new(1.0, 0.0, 0.0), 1.0, 1.0)
            .unwrap(); // at_t ~ 1.0
        q.schedule(0.0, 3.0, Vector3::new(2.0, 0.0, 0.0), 1.0, 1.0)
            .unwrap(); // at_t ~ 3.0
        q.schedule(0.0, 5.0, Vector3::new(3.0, 0.0, 0.0), 1.0, 1.0)
            .unwrap(); // at_t ~ 5.0
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
        q.schedule(0.0, 0.0, Vector3::ZERO, 1.0, 5.5).unwrap(); // d = 0: no falloff yet
        q.schedule(0.0, 2.5, Vector3::ZERO, 1.0, 5.5).unwrap(); // 0.55 / 2.0
        q.schedule(0.0, 3.0, Vector3::ZERO, 0.8, 5.5).unwrap(); // 0.44 / 2.2
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
        q.schedule(10.0, dist, Vector3::ZERO, 1.0, 5.5).unwrap();
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
        book.schedule(0.0, 5.5, Vector3::new(1.0, 0.0, 0.0), 1.0, 5.5)
            .unwrap();
        book.schedule(0.0, 2.2, Vector3::new(2.0, 0.0, 0.0), 1.0, 5.5)
            .unwrap();
        book.schedule(0.0, 4.4, Vector3::new(3.0, 0.0, 0.0), 1.0, 5.5)
            .unwrap();
        let mut restored = EchoQueue::from_pending(book.capture());
        assert_eq!(restored.pending(), book.pending());
        let fired_original = book.drain(2.0);
        let fired_restored = restored.drain(2.0);
        assert_eq!(fired_restored, fired_original);
        assert_eq!(restored.pending(), book.pending()); // survivors too
    }
}
