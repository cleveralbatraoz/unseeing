//! What a sound SOURCE is, as pure math — the world's own sounds, the ones
//! the hero did not make. A blind person FEELS a steady source from another
//! room: its waves stop dead at a wall like any other sound, but its
//! standing acoustic image is still felt through the wall as a dimmed
//! ghost. That standing-image privilege is the pool's kind [`SOURCE_KIND`],
//! and every source in the world speaks through it.
//!
//! Sources differ in FOUR properties, and this module is the whole
//! vocabulary:
//! - [`Volume`] — how loud it is. The one knob a designer thinks in.
//! - [`Spread`] — whether it throws an even sphere or a directed cone.
//! - cadence — how often a wave is born.
//! - speed — how fast the wavefront travels.
//!
//! THE VOLUME LAW. Volume is AMPLITUDE at the hub, normalized to (0, 1].
//! A spherical wave's pressure falls as `p(r) = A / r`; it stays audible
//! while `p(r) > p_min`, so the distance it reaches is `r_max = A / p_min`
//! — LINEAR in amplitude. Hence a source's reach is [`FULL_REACH`] scaled
//! by its volume, and the gain it carries into the pool IS its volume. The
//! same number is its standing image ([`Volume::image`]): how strongly the
//! hero feels the shape of it with no wave nearby. One knob, three honest
//! consequences — a designer cannot author a "loud" source that dies after
//! three meters, because reach is not a separate knob.
//!
//! Everything here is f64 and engine-free (godot's glam-backed math
//! builtins aside): the narrowing to the pool's f32 lanes happens
//! downstream, in the pool itself. The scene work — meshes, colliders,
//! transforms, the emit call — stays in the engine layer
//! ([`crate::nodes`]); this module owns only the math the cargo tests pin.

use godot::builtin::Vector3;

use crate::pulse_pool::{self, OMNI_COS};
use crate::reproduce::RestoreValueError;

/// The pulse kind every world sound source is born as: the one sound the
/// hero did not make. Its waves are cut crisp at a wall exactly like a
/// player-made sound; only its standing acoustic image still passes a
/// wall muffled, dimming toward a ghost rather than vanishing outright.
pub const SOURCE_KIND: i32 = 3;

/// Meters a source at FULL volume reaches. The scale of the whole loudness
/// ladder: every source's range is this times its volume, so the ladder is
/// legible at a glance — 0.75 reaches nine meters, 1.0 reaches twelve.
pub const FULL_REACH: f64 = 12.0;

/// A source's loudness: the wave AMPLITUDE at its hub, 0 = silent,
/// 1 = the loudest thing the world holds. Total at the door — any input,
/// NaN and infinities included, lands inside [0, 1], because a volume that
/// escaped the range would bleed through [`Volume::gain`] into the pool's
/// packed kind digits.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Volume(f64);

impl Volume {
    /// A source that makes no sound at all.
    pub const SILENT: Self = Self(0.0);

    /// The loudest a source may be.
    pub const FULL: Self = Self(1.0);

    /// A volume from a designer's raw number, clamped into (0, 1]. NaN is
    /// [`Self::SILENT`] rather than a poisoned cast downstream.
    #[must_use]
    pub fn new(amplitude: f64) -> Self {
        if amplitude.is_nan() {
            return Self::SILENT;
        }
        Self(amplitude.clamp(0.0, 1.0))
    }

    /// The raw amplitude, in [0, 1].
    #[must_use]
    pub fn amplitude(self) -> f64 {
        self.0
    }

    /// Meters this source's waves travel: [`FULL_REACH`] scaled by
    /// amplitude, from `p(r) = A / r` against a fixed audibility floor.
    #[must_use]
    pub fn reach(self) -> f64 {
        FULL_REACH * self.0
    }

    /// The gain a wave of this source carries into the pool — the
    /// amplitude itself, which is what gain has always meant there.
    #[must_use]
    pub fn gain(self) -> f64 {
        self.0
    }

    /// The source's STANDING acoustic image: how strongly its silhouette
    /// is felt with no wave in flight, before any wall muffles it. Also
    /// the amplitude — the hero feels the shape of a thing exactly as
    /// loudly as it sounds.
    #[must_use]
    pub fn image(self) -> f64 {
        self.0
    }

    /// Does this source sound at all? A silent one would ask the pool for
    /// a zero-radius wave, which the pool rightly refuses.
    #[must_use]
    pub fn audible(self) -> bool {
        self.0 > 0.0
    }
}

/// How a source throws its waves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spread {
    /// Every direction alike: a radio fills its room, and turning the set
    /// around changes nothing. The pool's omnidirectional sentinel.
    Even,
    /// A directed cone of the given half-angle cosine, aimed wherever the
    /// source points at the instant of birth — the fan's sweeping wash.
    Cone {
        /// cos of the cone's half-angle: 1 is a laser, 0 a hemisphere.
        cos_half: f64,
    },
}

impl Spread {
    /// A cone of the given half-angle cosine. Total on any input: a value
    /// outside [-1, 1] is clamped, so a designer's typo narrows or widens
    /// the wash instead of producing a cone that can never be entered.
    #[must_use]
    pub fn cone(cos_half: f64) -> Self {
        if cos_half.is_nan() {
            return Self::Even;
        }
        Self::Cone {
            cos_half: cos_half.clamp(-1.0, 1.0),
        }
    }

    /// Does this spread ignore where the source points?
    #[must_use]
    pub fn is_even(self) -> bool {
        matches!(self, Self::Even)
    }

    /// The pool's two beam lanes for a source aimed along `aim`: an even
    /// spread throws the aim away — the pool reads a ZERO beam vector as
    /// "omnidirectional" however the cosine reads — while a cone carries
    /// the aim with its width.
    #[must_use]
    pub fn beam(self, aim: Vector3) -> (Vector3, f64) {
        match self {
            Self::Even => (Vector3::ZERO, OMNI_COS),
            Self::Cone { cos_half } => (aim, cos_half),
        }
    }
}

/// A source's whole acoustic identity — everything that makes a radio
/// sound unlike a fan. The engine layer holds one of these per source
/// node, built from that node's designer knobs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Voice {
    /// How loud, and so also how far and how strongly felt.
    pub volume: Volume,
    /// Seconds between waves. Frequent enough and the stream reads as one
    /// continuous sound rather than a series of pings.
    pub cadence: f64,
    /// Wavefront speed in m/s. Slower than a cane tap reads as a big lazy
    /// source; faster reads as a sharp one.
    pub speed: f64,
    /// Even sphere or directed cone.
    pub spread: Spread,
}

/// One wave, exactly as the pool wants it — the translation from a voice
/// to [`crate::pulse_pool::PulsePool::emit`]'s arguments, with nothing
/// left for the engine layer to decide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wave {
    /// Always [`SOURCE_KIND`]: this is a world source, not a hero sound.
    pub kind: i32,
    /// Meters the wavefront travels before it dies.
    pub range: f64,
    /// Wavefront speed, m/s.
    pub speed: f64,
    /// Loudness carried into the packed lane.
    pub gain: f64,
    /// The beam vector: ZERO for an even spread (the pool's omni
    /// sentinel), the aim for a cone.
    pub beam: Vector3,
    /// cos of the cone's half-angle, or [`OMNI_COS`] for an even spread.
    pub cos_half: f64,
}

impl Voice {
    /// The wave this voice is about to be born as, aimed along `aim` —
    /// which an even spread ignores.
    #[must_use]
    pub fn wave(&self, aim: Vector3) -> Wave {
        let (beam, cos_half) = self.spread.beam(aim);
        Wave {
            kind: SOURCE_KIND,
            range: self.volume.reach(),
            speed: self.speed,
            gain: self.volume.gain(),
            beam,
            cos_half,
        }
    }

    /// How many of the pool's 64 slots this voice occupies at steady
    /// state: a wave lives its ring time plus its kind's fade tail, and a
    /// new one is born every cadence. The budget every source must be held
    /// to — the pool is shared with the hero's taps, echoes and footsteps,
    /// and a greedy source would evict them.
    ///
    /// Zero for a voice that can never sound (silent, or a non-positive
    /// cadence or speed), because such a voice takes no slot at all.
    #[must_use]
    pub fn slot_pressure(&self) -> f64 {
        if !self.volume.audible() || self.cadence <= 0.0 || self.speed <= 0.0 {
            return 0.0;
        }
        (self.volume.reach() / self.speed + pulse_pool::fade_tail(SOURCE_KIND)) / self.cadence
    }
}

/// The cadence gate every source's clock runs through: fires when `t`
/// reaches the appointment, then books the next one a cadence after the
/// CURRENT time — so a stalled or jumped clock buys a single wave, never a
/// backfilled burst of them. The first wave waits one full interval, so a
/// source does not sound at t = 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cadence {
    every: f64,
    next: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PreparedCadence {
    interval_s: f64,
    next_s: Option<f64>,
}

impl Cadence {
    pub(crate) fn prepare_restore(
        interval: f64,
        next: f64,
        allow_absent_nan: bool,
    ) -> Result<PreparedCadence, RestoreValueError> {
        if !interval.is_finite() {
            return Err(RestoreValueError::new(
                "cadence.interval_s",
                "must be finite",
            ));
        }
        if interval <= 0.0 {
            return Err(RestoreValueError::new(
                "cadence.interval_s",
                "must be strictly positive",
            ));
        }
        let next_s = if allow_absent_nan && next.is_nan() {
            None
        } else {
            if !next.is_finite() {
                return Err(RestoreValueError::new("cadence.next_s", "must be finite"));
            }
            if next < 0.0 {
                return Err(RestoreValueError::new(
                    "cadence.next_s",
                    "must be non-negative",
                ));
            }
            Some(next)
        };
        Ok(PreparedCadence {
            interval_s: interval,
            next_s,
        })
    }

    #[must_use]
    pub(crate) fn from_prepared(value: PreparedCadence) -> Self {
        Self {
            every: value.interval_s,
            next: value.next_s,
        }
    }

    /// A fresh gate at the given interval, first wave due one interval in.
    /// A non-positive interval never fires — a source with no cadence is
    /// silent, not a per-frame flood.
    #[must_use]
    pub fn every(interval: f64) -> Self {
        Self {
            every: interval,
            next: interval.is_finite().then_some(interval),
        }
    }

    /// A gate rebuilt mid-flight: the interval AND the standing
    /// appointment, exactly as captured. `every` cannot express this (it
    /// books one interval out) and `retune` deliberately keeps the old
    /// date — this is the one door for a restored clock, and re-pinning
    /// through it AFTER the clock lands is what keeps a jumped clock
    /// from buying its one spurious beat per source.
    #[must_use]
    pub fn restore(interval: f64, next: f64) -> Self {
        Self {
            every: interval,
            next: next.is_finite().then_some(next),
        }
    }

    /// The interval this gate books by.
    #[must_use]
    pub fn interval(self) -> f64 {
        self.every
    }

    /// When the next wave is due, on the same clock [`Self::beat`] is
    /// driven with — the appointment itself, so "the fan has gone quiet"
    /// can be diagnosed without waiting to see whether it stays quiet.
    ///
    /// `None` when no appointment is being kept: a gate whose interval is
    /// non-positive or non-finite never fires whatever the booked time
    /// says, and a gate built from a poisoned interval carries that poison
    /// in the appointment until a beat repairs it. Both would otherwise
    /// hand back a stale or non-finite number that reads as a real date —
    /// and a plausible wrong date is worse than an admitted absence.
    #[must_use]
    pub fn next_at(self) -> Option<f64> {
        if self.every <= 0.0 || !self.every.is_finite() {
            return None;
        }
        self.next
    }

    /// Adopt a new interval mid-flight, so a cadence knob is as live as
    /// every other knob on a source — `volume`, `speed` and the cone width
    /// are all re-read on each beat, and a cadence frozen at build time
    /// would be the one piece of hidden state the abstraction exists to
    /// remove (and would make `slot_pressure` describe a source that is not
    /// the one running).
    ///
    /// The appointment already booked STANDS; the new interval governs every
    /// appointment after it. So a knob moved mid-flight never makes a source
    /// jump, double-fire, or fall silent for longer than one old interval —
    /// it simply keeps its next date and then settles into the new rhythm.
    pub fn retune(&mut self, every: f64) {
        self.every = every;
    }

    /// Advance the clock. Returns the beat's time when its moment has come
    /// — `t >= next`, the boundary instant firing — and rebooks from `t`,
    /// not from the missed appointment: no backfill after a time jump.
    pub fn beat(&mut self, t: f64) -> Option<f64> {
        if self.every <= 0.0 || !self.every.is_finite() || !t.is_finite() {
            return None;
        }
        // A gate built from a non-finite interval carries that value in its
        // APPOINTMENT too, and no later retune touches it — so a source
        // whose knob was once `inf` would stay silent for the rest of the
        // session however the knob moved afterwards, and one built from
        // `nan` would fire on the first tick regardless of its interval.
        // Repair it here, where `t` is known: book one full interval out,
        // exactly as a fresh gate does. The repair lives in `beat` rather
        // than in `retune` because the cat's presence gate shares this
        // clock and never retunes.
        let Some(next) = self.next else {
            self.next = finite_sum(t, self.every);
            return None;
        };
        if t < next {
            return None;
        }
        self.next = finite_sum(t, self.every);
        Some(t)
    }
}

fn finite_sum(a: f64, b: f64) -> Option<f64> {
    let sum = a + b;
    sum.is_finite().then_some(sum)
}

impl Default for Cadence {
    /// A gate that never fires: an uninitialised source is silent rather
    /// than emitting every frame.
    fn default() -> Self {
        Self::every(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_restore_rejects_invalid_cadence_interval_or_appointment() {
        let error =
            Cadence::prepare_restore(0.0, 1.0, false).expect_err("zero interval must be refused");
        assert_eq!(error.path, "cadence.interval_s");
        let error = Cadence::prepare_restore(1.0, f64::NAN, false)
            .expect_err("source NaN appointment must be refused");
        assert_eq!(error.path, "cadence.next_s");
        assert!(Cadence::prepare_restore(1.0, f64::NAN, true).is_ok());
    }

    /// THE volume law, in one test: amplitude is gain, reach is linear in
    /// it, and the standing image rides along. Half as loud is half as
    /// far, half as bright, and half as strongly felt — one knob.
    #[test]
    fn volume_is_amplitude_and_reach_is_linear_in_it() {
        let half = Volume::new(0.5);
        assert_eq!(half.gain(), 0.5);
        assert_eq!(half.image(), 0.5);
        assert_eq!(half.reach(), FULL_REACH * 0.5);
        assert_eq!(Volume::FULL.reach(), FULL_REACH);
        assert_eq!(Volume::SILENT.reach(), 0.0);
    }

    /// The shipped fan's numbers, derived: the validated hum (range 9 m,
    /// gain 0.75) falls straight out of a single volume of 0.75, which is
    /// what makes the volume ladder a REPLACEMENT for the old pair of
    /// knobs rather than a third one beside them.
    #[test]
    fn the_shipped_fan_numbers_fall_out_of_one_volume() {
        let fan = Volume::new(0.75);
        assert_eq!(fan.reach(), 9.0);
        assert_eq!(fan.gain(), 0.75);
    }

    /// Total at the door: a designer's raw number cannot escape [0, 1],
    /// because a gain outside it bleeds into the pool's packed kind digits
    /// and a negative reach would be refused as a wave.
    #[test]
    fn volume_clamps_every_input() {
        assert_eq!(Volume::new(1.5), Volume::FULL);
        assert_eq!(Volume::new(-1.0), Volume::SILENT);
        assert_eq!(Volume::new(f64::NAN), Volume::SILENT);
        assert_eq!(Volume::new(f64::INFINITY), Volume::FULL);
        assert_eq!(Volume::new(f64::NEG_INFINITY), Volume::SILENT);
    }

    /// A silent source must not ask the pool for a wave: a zero radius is
    /// refused there, and asking every frame would be a per-frame refusal.
    #[test]
    fn silence_is_not_audible() {
        assert!(!Volume::SILENT.audible());
        assert!(Volume::new(0.01).audible());
    }

    /// An even spread throws the aim away — the pool reads a ZERO beam as
    /// omnidirectional however the cosine reads — while a cone carries it.
    #[test]
    fn spread_maps_onto_the_pools_beam_lanes() {
        let aim = Vector3::new(0.0, 0.0, -1.0);
        assert_eq!(Spread::Even.beam(aim), (Vector3::ZERO, OMNI_COS));
        assert_eq!(Spread::cone(0.85).beam(aim), (aim, 0.85));
    }

    /// Total on any cone width: a typo narrows or widens the wash, never
    /// producing a cone no fragment can ever be inside.
    #[test]
    fn cone_width_is_clamped_and_nan_falls_back_to_even() {
        assert_eq!(Spread::cone(4.0), Spread::Cone { cos_half: 1.0 });
        assert_eq!(Spread::cone(-9.0), Spread::Cone { cos_half: -1.0 });
        assert_eq!(Spread::cone(f64::NAN), Spread::Even);
        assert!(Spread::Even.is_even());
        assert!(!Spread::cone(0.85).is_even());
    }

    /// A voice becomes exactly the pool's emit arguments — kind, range,
    /// speed, gain and the two beam lanes — with nothing left to decide.
    #[test]
    fn a_voice_becomes_the_pools_emit_arguments() {
        let aim = Vector3::new(1.0, 0.0, 0.0);
        let directed = Voice {
            volume: Volume::new(0.75),
            cadence: 0.4,
            speed: 4.5,
            spread: Spread::cone(0.85),
        };
        assert_eq!(
            directed.wave(aim),
            Wave {
                kind: SOURCE_KIND,
                range: 9.0,
                speed: 4.5,
                gain: 0.75,
                beam: aim,
                cos_half: 0.85,
            }
        );
        let even = Voice {
            spread: Spread::Even,
            volume: Volume::FULL,
            ..directed
        };
        let wave = even.wave(aim);
        assert_eq!(wave.beam, Vector3::ZERO);
        assert_eq!(wave.cos_half, OMNI_COS);
        assert_eq!(wave.range, FULL_REACH);
    }

    /// Slot pressure is the budget every source is held to: ring time plus
    /// the kind's fade tail, divided by the cadence.
    #[test]
    fn slot_pressure_counts_ring_time_plus_tail() {
        let voice = Voice {
            volume: Volume::new(0.75),
            cadence: 0.4,
            speed: 4.5,
            spread: Spread::cone(0.85),
        };
        // 9 / 4.5 = 2 s of ring, + 2 s of tail, a wave every 0.4 s
        assert!((voice.slot_pressure() - 10.0).abs() < 1e-9);
    }

    /// A voice that can never sound occupies no slot — and the test says
    /// so for every way of being silent, so a division by zero can never
    /// hide behind an infinite budget.
    #[test]
    fn a_voice_that_cannot_sound_costs_nothing() {
        let base = Voice {
            volume: Volume::FULL,
            cadence: 0.5,
            speed: 4.0,
            spread: Spread::Even,
        };
        for dead in [
            Voice {
                volume: Volume::SILENT,
                ..base
            },
            Voice {
                cadence: 0.0,
                ..base
            },
            Voice {
                speed: -1.0,
                ..base
            },
        ] {
            assert_eq!(dead.slot_pressure(), 0.0);
        }
    }

    /// The cadence gate: the first beat lands exactly on the interval (the
    /// boundary fires), a call inside the interval stays silent, and each
    /// beat books the next one an interval later.
    #[test]
    fn one_beat_per_cadence() {
        let mut gate = Cadence::every(0.4);
        assert_eq!(gate.interval(), 0.4);
        assert_eq!(gate.beat(0.2), None);
        assert_eq!(gate.beat(0.4), Some(0.4));
        assert_eq!(gate.beat(0.41), None);
        assert_eq!(gate.beat(0.79), None);
        assert_eq!(gate.beat(0.8), Some(0.8));
    }

    /// A stalled clock buys a single beat, never a backfilled burst: after
    /// jumping to t = 5.0 the gate fires once and rebooks from 5.0.
    #[test]
    fn a_time_jump_buys_a_single_beat() {
        let mut gate = Cadence::every(0.4);
        assert!(gate.beat(0.4).is_some());
        assert_eq!(gate.beat(5.0), Some(5.0));
        assert_eq!(gate.beat(5.39), None);
        assert_eq!(gate.beat(5.4), Some(5.4));
    }

    /// A cadence knob is live: retuning changes what the gate books next,
    /// without disturbing the appointment it has already made.
    #[test]
    fn retuning_takes_effect_from_the_next_beat() {
        let mut gate = Cadence::every(0.4);
        assert_eq!(gate.beat(0.4), Some(0.4)); // booked: 0.8
        gate.retune(1.5);
        assert_eq!(gate.interval(), 1.5);
        assert_eq!(gate.beat(0.79), None);
        assert_eq!(gate.beat(0.8), Some(0.8)); // the booked appointment stands
        assert_eq!(gate.beat(1.9), None); // ...and rebooks by the NEW interval
        assert_eq!(gate.beat(2.3), Some(2.3));
    }

    /// Retuning to silence stops a source dead rather than leaving it on its
    /// old gate — a designer turning the cadence to zero means "stop".
    #[test]
    fn retuning_to_no_cadence_silences_the_gate() {
        let mut gate = Cadence::every(0.4);
        assert!(gate.beat(0.4).is_some());
        gate.retune(0.0);
        assert_eq!(gate.beat(9.0), None);
        assert_eq!(gate.beat(1e6), None);
    }

    /// A knob can always bring a gate back. One built from a non-finite
    /// interval carries that value in its appointment as well; retuning to a
    /// usable interval must make the source sound again rather than leave it
    /// silent — or, for NaN, firing on every tick — for the rest of the run.
    #[test]
    fn a_poisoned_appointment_is_repaired_by_a_usable_interval() {
        for poison in [f64::INFINITY, f64::NAN, f64::NEG_INFINITY] {
            let mut gate = Cadence::every(poison);
            assert_eq!(gate.beat(1.0), None, "{poison} gate must start silent");
            gate.retune(0.7);
            assert_eq!(gate.beat(1.0), None); // repaired: books one interval out
            assert_eq!(gate.beat(1.69), None);
            assert_eq!(gate.beat(1.7), Some(1.7));
            assert_eq!(gate.beat(2.4), Some(2.4)); // ...and keeps the rhythm
        }
    }

    /// The appointment is readable, so "why has the fan gone quiet?" can be
    /// answered without waiting to see whether it does. A gate that CANNOT
    /// fire reports `None` rather than the stale number it is holding: a
    /// silenced source sits on an appointment it will never keep, and
    /// reporting 0.4 for a gate retuned to zero would be a plausible wrong
    /// answer — the worst kind.
    #[test]
    fn the_next_appointment_is_readable_and_absent_when_none_is_kept() {
        let mut gate = Cadence::every(0.4);
        assert_eq!(gate.next_at(), Some(0.4));
        assert_eq!(gate.beat(0.4), Some(0.4));
        assert_eq!(gate.next_at(), Some(0.8));
        gate.retune(0.0);
        assert_eq!(gate.next_at(), None);
        for silent in [Cadence::default(), Cadence::every(-1.0)] {
            assert_eq!(silent.next_at(), None);
        }
    }

    /// A gate built from a poisoned interval holds a poisoned appointment
    /// until its next beat repairs it. Reporting `at_t = NaN` or `inf` would
    /// serialise as JSON `null` and read as a missing field; the whole
    /// non-finite case is one answer — no appointment is being kept.
    #[test]
    fn a_poisoned_appointment_reads_as_no_appointment() {
        for poison in [f64::INFINITY, f64::NAN, f64::NEG_INFINITY] {
            assert_eq!(Cadence::every(poison).next_at(), None, "{poison}");
        }
    }

    /// A gate with no interval never fires — an uninitialised or
    /// misconfigured source is silent, not a wave every single frame.
    #[test]
    fn a_cadenceless_gate_is_silent_forever() {
        for mut gate in [
            Cadence::default(),
            Cadence::every(0.0),
            Cadence::every(-1.0),
        ] {
            assert_eq!(gate.beat(0.0), None);
            assert_eq!(gate.beat(1e6), None);
        }
        let mut broken = Cadence::every(f64::NAN);
        assert_eq!(broken.beat(1e6), None);
    }

    /// A restored gate holds EXACTLY the captured appointment: nothing
    /// fires before it, the appointment fires on time, and the next
    /// rebooking runs on the restored interval. Literals hand-picked:
    /// interval 4.0, appointment at 10.0, clock restored to 9.0.
    #[test]
    fn a_restored_appointment_stands_and_nothing_fires_early() {
        let mut gate = Cadence::restore(4.0, 10.0);
        assert_eq!(gate.next_at(), Some(10.0));
        assert_eq!(gate.beat(9.0), None); // the restore instant: silence
        assert_eq!(gate.beat(10.0), Some(10.0)); // fires on the dot
        assert_eq!(gate.next_at(), Some(14.0)); // rebooks from t
    }

    /// An OVERDUE captured appointment stays overdue and fires on the
    /// very next beat — exactly as it would have in the original run.
    #[test]
    fn an_overdue_restored_appointment_fires_at_once() {
        let mut gate = Cadence::restore(4.0, 8.0);
        assert_eq!(gate.beat(9.0), Some(9.0));
        assert_eq!(gate.next_at(), Some(13.0));
    }
}
