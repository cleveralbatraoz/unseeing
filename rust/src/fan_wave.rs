//! The fan's clockwork — motion curves and whoosh cadence, mirrored
//! exactly from fan.gd. The oscillating pedestal fan is the world's one
//! constant sound source: a blind person FEELS a steady source even from
//! another room, so its hum (pulse kind 3) passes walls muffled, and its
//! directed wash — a cone of waves out of the pivoting head — is born so
//! often it reads as one continuous stream sweeping the room like a
//! lighthouse.
//!
//! Everything here is f64: GDScript floats drove these curves, and the
//! narrowing to the pool's f32 lanes happens downstream, in the pool
//! itself. The scene work (meshes, colliders, transforms) stays in the
//! engine layer; this module owns only the math the headless tests pin.

/// Whoosh cadence in seconds — so frequent the wash reads as continuous.
pub const WHOOSH_EVERY: f64 = 0.4;

/// Meters a hum travels.
pub const HUM_RANGE: f64 = 9.0;

/// Hum wavefront speed, m/s — slower than a cane tap: a big lazy source.
pub const HUM_SPEED: f64 = 4.5;

/// Hum loudness — steady but never as sharp as the hero's own tap.
pub const HUM_GAIN: f64 = 0.75;

/// cos of the wash cone's half-angle (~32°).
pub const BEAM_COS: f64 = 0.85;

/// Radians the head pivots each way from its mounting yaw.
pub const PIVOT_RANGE: f64 = 0.85;

/// Pivot sweep rate — the sin argument's time scale.
pub const PIVOT_SPEED: f64 = 0.55;

/// Blade spin, rad/s — fast enough to read as motion across reveals.
pub const SPIN_SPEED: f64 = 9.0;

/// The head's oscillation at time `t`: a sine sweep, `PIVOT_RANGE` each
/// way. The collider rides the same curve, so this is also a physics
/// bound — verbatim `sin(t * PIVOT_SPEED) * PIVOT_RANGE`.
#[must_use]
pub fn pivot_angle(t: f64) -> f64 {
    (t * PIVOT_SPEED).sin() * PIVOT_RANGE
}

/// The blades' rotation at time `t`, wrapped like GDScript's
/// `fmod(t * SPIN_SPEED, TAU)` — Rust's `%` is the same fmod, sign of
/// the dividend included, so a rewound clock mirrors the original too.
#[must_use]
pub fn spin_angle(t: f64) -> f64 {
    (t * SPIN_SPEED) % std::f64::consts::TAU
}

/// One beat of the wash: the moment a directed hum must be born, aimed
/// wherever the pivoting head points at exactly this instant. The engine
/// layer turns it into `emit(3, hub, HUM_RANGE, HUM_SPEED, HUM_GAIN, at,
/// fwd, BEAM_COS)` — the constants live here so both sides agree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Whoosh {
    /// The beat's time — the hum's birth time.
    pub at: f64,
}

/// The cadence gate, mirroring fan.gd's `_next_whoosh` field: fires when
/// `t` reaches the appointment, then books the next one a cadence after
/// the CURRENT time — so a stalled or jumped clock buys a single beat,
/// never a backfilled burst of them. The cadence is a designer knob on the
/// engine's fan node, so the gate carries its own interval; the default is
/// the shipped [`WHOOSH_EVERY`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WhooshScheduler {
    every: f64,
    next_whoosh: f64,
}

impl Default for WhooshScheduler {
    /// The shipped cadence: first beat due at `WHOOSH_EVERY`, like the
    /// GDScript initializer `_next_whoosh := 0.4` — the fan does not
    /// whoosh at t = 0.
    fn default() -> Self {
        Self::with_cadence(WHOOSH_EVERY)
    }
}

impl WhooshScheduler {
    /// A fresh gate at the shipped cadence, first beat due at
    /// `WHOOSH_EVERY`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A fresh gate at a designer-chosen cadence: the first beat waits one
    /// full interval, and every beat rebooks that same interval ahead —
    /// the `_next_whoosh := 0.4` law, generalized to the knob's value.
    #[must_use]
    pub fn with_cadence(every: f64) -> Self {
        Self {
            every,
            next_whoosh: every,
        }
    }

    /// Advance the clock. Returns the beat when its time has come —
    /// `t >= next`, the boundary instant firing, exactly the original's
    /// early-return `if t < _next_whoosh` — and rebooks from `t`, not
    /// from the missed appointment: no backfill after time jumps.
    pub fn update(&mut self, t: f64) -> Option<Whoosh> {
        if t < self.next_whoosh {
            return None;
        }
        self.next_whoosh = t + self.every;
        Some(Whoosh { at: t })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The head's oscillation must actually sweep, and never exceed its
    /// range — the collider rides the same curve, so this is also a
    /// physics bound.
    #[test]
    fn pivot_sweeps_full_range_both_ways() {
        let mut lo: f64 = 0.0;
        let mut hi: f64 = 0.0;
        for i in 0..200 {
            let a = pivot_angle(f64::from(i) * 0.1);
            lo = lo.min(a);
            hi = hi.max(a);
        }
        assert!(hi <= PIVOT_RANGE + 0.001);
        assert!(lo >= -PIVOT_RANGE - 0.001);
        assert!(hi > PIVOT_RANGE * 0.9);
        assert!(lo < -PIVOT_RANGE * 0.9);
    }

    #[test]
    fn blades_spin() {
        assert!(spin_angle(1.0) != spin_angle(1.1));
    }

    /// The spin wraps exactly like fmod: within a turn, and matching the
    /// unwrapped angle minus whole turns.
    #[test]
    fn spin_wraps_like_fmod() {
        let t = 3.7;
        let raw = t * SPIN_SPEED;
        let wrapped = spin_angle(t);
        assert!((0.0..std::f64::consts::TAU).contains(&wrapped));
        assert_eq!(wrapped, raw % std::f64::consts::TAU);
    }

    /// A hum slot lives ring + 2s; the constant wash must not flood the
    /// 64-slot pool. Headroom pinned at 12 concurrent hums, worst case.
    #[test]
    fn wash_stays_within_slot_headroom() {
        let concurrent = (HUM_RANGE / HUM_SPEED + 2.0) / WHOOSH_EVERY;
        assert!(concurrent <= 12.0);
    }

    /// The wash is a directed cone, neither a laser nor a floodlight.
    #[test]
    #[expect(
        clippy::assertions_on_constants,
        reason = "the point IS the constant: a design-envelope pin ported \
                  from the GDScript suite, guarding future retuning"
    )]
    fn wash_is_a_directed_cone() {
        assert!(BEAM_COS > 0.7);
        assert!(BEAM_COS < 0.95);
    }

    /// The cadence gate: the first beat lands exactly at 0.4 (the boundary
    /// fires), the next call inside the cadence stays silent, and each
    /// beat books the next one WHOOSH_EVERY later.
    #[test]
    fn one_beat_per_cadence() {
        let mut gate = WhooshScheduler::new();
        assert_eq!(gate.update(0.2), None); // before the first appointment
        assert_eq!(gate.update(0.4), Some(Whoosh { at: 0.4 })); // the boundary fires
        assert_eq!(gate.update(0.41), None); // inside the cadence: not a sound
        assert_eq!(gate.update(0.79), None);
        assert_eq!(gate.update(0.8), Some(Whoosh { at: 0.8 }));
    }

    /// The cadence is a designer knob: a gate built at 0.6 s waits 0.6 for
    /// its first beat and rebooks 0.6 ahead — the same first-beat and
    /// no-backfill laws, at the knob's interval.
    #[test]
    fn designer_cadence_books_by_its_own_interval() {
        let mut gate = WhooshScheduler::with_cadence(0.6);
        assert_eq!(gate.update(0.4), None); // the shipped cadence is not this gate's
        assert_eq!(gate.update(0.6), Some(Whoosh { at: 0.6 }));
        assert_eq!(gate.update(1.19), None);
        assert_eq!(gate.update(1.2), Some(Whoosh { at: 1.2 }));
    }

    /// A stalled clock buys a single beat, never a backfilled burst: after
    /// jumping to t = 5.0 the gate fires once and rebooks from 5.0.
    #[test]
    fn time_jump_buys_a_single_beat() {
        let mut gate = WhooshScheduler::new();
        assert!(gate.update(0.4).is_some());
        assert_eq!(gate.update(5.0), Some(Whoosh { at: 5.0 })); // one beat, not eleven
        assert_eq!(gate.update(5.39), None); // rebooked from 5.0, not from 0.8
        assert_eq!(gate.update(5.4), Some(Whoosh { at: 5.4 }));
    }
}
