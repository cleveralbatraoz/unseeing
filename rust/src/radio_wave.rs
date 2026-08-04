//! The radio's voice — the world's second sound source, and the one that
//! proves the source abstraction is an abstraction. It shares every law
//! with the fan ([`crate::sound_source`]): the same pulse kind, the same
//! privilege of passing walls muffled, the same standing acoustic image
//! felt through them. It differs in exactly the two ways a designer can
//! hear:
//!
//! - **It is LOUDER.** Full volume against the fan's three quarters, which
//!   by the volume law is also further (12 m against 9) and more strongly
//!   felt through a wall. The ladder is the point: a blind hero navigating
//!   by sound must be able to tell two sources apart by loudness alone.
//! - **It does not aim.** A radio fills its room; there is no front to it.
//!   Its waves are an even sphere ([`Spread::Even`]), where the fan's are a
//!   cone swept by a pivoting head — so walking around a radio changes
//!   nothing, while walking around a fan changes everything.
//!
//! It is also LAZIER: one wave every 0.7 s against the fan's 0.4. A cone
//! must be reborn often or the sweep reads as a stutter of separate pings;
//! a sphere already fills every direction, so fewer of them still read as
//! one continuous presence. That is a slot budget too — an even wave that
//! reaches 12 m is the most expensive thing in the pool, and the cadence is
//! where it is paid for.
//!
//! No motion curves live here, because nothing on a radio moves. The module
//! exists so the shipped voice is cargo-pinned beside the fan's, and so the
//! LADDER between the two sources — the ordering the whole feature is about
//! — is a test rather than a hope.

use crate::sound_source::{Spread, Voice, Volume};

/// The radio's loudness: the loudest thing in the world, and the top of
/// the ladder every other source is heard against. By the volume law this
/// is also its 12 m reach and the gain its waves carry.
pub const RADIO_VOLUME: f64 = 1.0;

/// Seconds between waves — lazier than the fan's sweep, because an even
/// sphere needs fewer births to read as one continuous presence.
pub const RADIO_CADENCE: f64 = 0.7;

/// Wavefront speed, m/s — brisker than the fan's lazy hum, still short of
/// the crack of a cane tap.
pub const RADIO_SPEED: f64 = 5.0;

/// The voice the shipped radio speaks with: full volume, an even sphere, a
/// wave every 0.7 s at 5 m/s. Every number is a designer knob on the radio
/// node; this is what a fresh one defaults to.
#[must_use]
pub fn shipped_voice() -> Voice {
    Voice {
        volume: Volume::new(RADIO_VOLUME),
        cadence: RADIO_CADENCE,
        speed: RADIO_SPEED,
        spread: Spread::Even,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fan_wave;
    use godot::builtin::Vector3;

    /// THE ladder, and the reason the feature exists: the fan is quieter
    /// than the radio, and by the volume law that means it is also heard
    /// from less far and felt less strongly through a wall. Every
    /// consequence is checked, not just the knob — a volume that stopped
    /// driving reach would pass a knob-only test and break the game.
    #[test]
    fn the_fan_is_quieter_than_the_radio_in_every_consequence() {
        let fan = fan_wave::shipped_voice();
        let radio = shipped_voice();
        assert!(fan.volume < radio.volume);
        assert!(fan.volume.reach() < radio.volume.reach());
        assert!(fan.volume.gain() < radio.volume.gain());
        assert!(fan.volume.image() < radio.volume.image());
    }

    /// The other difference a designer can hear: the radio does not aim.
    /// Its waves carry the pool's omnidirectional sentinel however the
    /// node happens to be rotated in the scene, where the fan's carry the
    /// direction its head points at the instant of birth.
    #[test]
    fn the_radio_does_not_aim_however_it_is_turned() {
        let radio = shipped_voice();
        assert!(radio.spread.is_even());
        for aim in [
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ] {
            let wave = radio.wave(aim);
            assert_eq!(wave.beam, Vector3::ZERO);
            assert_eq!(wave.cos_half, crate::pulse_pool::OMNI_COS);
        }
        assert!(!fan_wave::shipped_voice().spread.is_even());
    }

    /// The shipped radio's numbers, derived from one volume: 12 m of
    /// reach at gain 1.0.
    #[test]
    fn the_shipped_voice_reaches_twelve_meters_at_full_gain() {
        let wave = shipped_voice().wave(Vector3::ZERO);
        assert_eq!(wave.range, 12.0);
        assert_eq!(wave.gain, 1.0);
        assert_eq!(wave.speed, RADIO_SPEED);
    }

    /// The pool is shared with the hero's own taps, echoes and footsteps,
    /// and a greedy source would evict them. The loudest, longest-reaching
    /// source in the world must still fit the same headroom the fan is
    /// held to — and BOTH sources together must leave the pool most of
    /// itself, which is the budget that actually matters now that the
    /// world has more than one of them.
    #[test]
    fn both_sources_together_leave_the_pool_room_to_breathe() {
        assert!(shipped_voice().slot_pressure() <= 12.0);
        let together = shipped_voice().slot_pressure() + fan_wave::shipped_voice().slot_pressure();
        assert!(
            together <= (crate::pulse_pool::MAXP as f64) * 0.5,
            "the world's sources claim {together} of {} slots",
            crate::pulse_pool::MAXP
        );
    }
}
