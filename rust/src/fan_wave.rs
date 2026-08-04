//! The fan's clockwork and its shipped voice. The oscillating pedestal fan
//! is a world sound source ([`crate::sound_source`]) with one distinguishing
//! property: it does not sound in every direction. It blows a DIRECTED wash
//! — a cone of waves out of the pivoting head, born so often it reads as one
//! continuous stream sweeping the room like a lighthouse — and because the
//! head oscillates, that cone is aimed somewhere new at every beat.
//!
//! This module owns two things and no more: the motion curves the head and
//! blades ride (which are also physics bounds, since the head's collider
//! rides the same curve), and the [`Voice`] the shipped fan speaks with. The
//! general laws of being a source — the volume ladder, the cadence gate,
//! the translation into the pool's lanes — live in [`crate::sound_source`],
//! shared with every other source in the world.
//!
//! Everything here is f64: GDScript floats drove these curves, and the
//! narrowing to the pool's f32 lanes happens downstream, in the pool
//! itself. The scene work (meshes, colliders, transforms) stays in the
//! engine layer; this module owns only the math the headless tests pin.

use crate::sound_source::{Spread, Voice, Volume};

/// The fan's loudness: quieter than the radio, which is the whole point of
/// the ladder being visible. By the volume law this is also its 9 m reach
/// and the gain its waves carry — the exact numbers the fan shipped with
/// before one volume replaced that pair of knobs.
pub const FAN_VOLUME: f64 = 0.75;

/// Seconds between whooshes — so frequent the wash reads as continuous.
pub const FAN_CADENCE: f64 = 0.4;

/// Wavefront speed, m/s — slower than a cane tap: a big lazy source.
pub const FAN_SPEED: f64 = 4.5;

/// cos of the wash cone's half-angle (~32°).
pub const FAN_BEAM_COS: f64 = 0.85;

/// Radians the head pivots each way from its mounting yaw.
pub const PIVOT_RANGE: f64 = 0.85;

/// Pivot sweep rate — the sin argument's time scale.
pub const PIVOT_SPEED: f64 = 0.55;

/// Blade spin, rad/s — fast enough to read as motion across reveals.
pub const SPIN_SPEED: f64 = 9.0;

/// The voice the shipped fan speaks with: three quarters loud, a directed
/// cone, a wave every 0.4 s at 4.5 m/s. Every number is a designer knob on
/// the fan node; this is what a fresh one defaults to.
#[must_use]
pub fn shipped_voice() -> Voice {
    Voice {
        volume: Volume::new(FAN_VOLUME),
        cadence: FAN_CADENCE,
        speed: FAN_SPEED,
        spread: Spread::cone(FAN_BEAM_COS),
    }
}

/// The head's oscillation at time `t`: a sine sweep, [`PIVOT_RANGE`] each
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

    /// The validated hum the fan has always emitted, now DERIVED from one
    /// volume: 9 m of reach at gain 0.75. A regression here would mean the
    /// volume ladder silently retuned the shipped world.
    #[test]
    fn the_shipped_voice_reproduces_the_validated_hum() {
        let wave = shipped_voice().wave(godot::builtin::Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(wave.range, 9.0);
        assert_eq!(wave.gain, 0.75);
        assert_eq!(wave.speed, 4.5);
        assert_eq!(wave.cos_half, FAN_BEAM_COS);
    }

    /// A hum slot lives ring + tail; the constant wash must not flood the
    /// 64-slot pool. Headroom pinned at 12 concurrent hums, worst case.
    #[test]
    fn wash_stays_within_slot_headroom() {
        assert!(shipped_voice().slot_pressure() <= 12.0);
    }

    /// The wash is a directed cone, neither a laser nor a floodlight —
    /// and it is DIRECTED at all, which is what distinguishes the fan from
    /// every even-spread source in the world.
    #[test]
    fn wash_is_a_directed_cone() {
        match shipped_voice().spread {
            Spread::Cone { cos_half } => {
                assert!(cos_half > 0.7);
                assert!(cos_half < 0.95);
            }
            Spread::Even => panic!("the fan's wash must be directed"),
        }
    }
}
