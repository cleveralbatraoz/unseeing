//! How long a surface keeps hearing a wave that already swept it — the
//! reveal envelope, as a pure function of one number.
//!
//! `sight` owns WHERE a wave reaches; this owns WHEN it stops. Both are
//! cargo-pinned references that `data_core.gdshaderinc` transliterates,
//! because the alternative is what shipped before this module existed: the
//! decay lived only in GLSL, nothing could evaluate it, and it had no end.
//!
//! # The one time coordinate
//!
//! Everything here is a function of `since_front` — seconds since the
//! wavefront passed the lit point, the shader's `ga = age - dist / speed`.
//! It is the right coordinate because it is the only one under which the
//! law is the same for every point: a surface near the source and a surface
//! at the wave's full reach both flare and fade identically, offset in
//! wall-clock time by their own travel delay. Writing the law against raw
//! `age` instead would make the fade depend on distance twice.
//!
//! # Why the envelope is shifted rather than merely cut
//!
//! The shipped decay `1.3·e^(-t/0.25) + 0.5·e^(-t/3)` is a sum of
//! exponentials, so it is asymptotic: it never reaches zero. A pulse's slot
//! is retired at [`pulse_pool::fade_tail`] seconds past the front, and the
//! reveal must be gone by then or the wave's visible end is decided by the
//! slot allocator instead of by the wave — which is exactly what happened:
//! a surface stayed lit until some later sound happened to reuse the slot,
//! and then went dark in one frame.
//!
//! Cutting the raw decay off at the tail would trade an unbounded lifetime
//! for a visible step (0.068 of peak for a tap, 0.257 for a hum). Shifting
//! it down by its own value at the tail instead reaches zero exactly, with
//! a continuous derivative-free landing, and leaves the perceptually
//! important part — the strike flash, which saturates the [0, 1] clamp
//! anyway — bit-for-bit where it was.

use crate::pulse_pool::fade_tail;

/// Weight and time constant of the strike flash: the bright, fast half of
/// the decay, gone within a few tenths of a second.
const FAST_WEIGHT: f64 = 1.3;
const FAST_TIME: f64 = 0.25;

/// Weight and time constant of the lingering half — the part that keeps a
/// swept surface faintly present after the flash, and the one that made the
/// un-shifted envelope immortal.
const SLOW_WEIGHT: f64 = 0.5;
const SLOW_TIME: f64 = 3.0;

/// The raw, un-shifted two-term decay. Asymptotic on purpose: it is the
/// shape, not the law. [`flare`] is the law.
fn raw(since_front: f64) -> f64 {
    FAST_WEIGHT * (-since_front / FAST_TIME).exp() + SLOW_WEIGHT * (-since_front / SLOW_TIME).exp()
}

/// How strongly a wave still reveals a point `since_front` seconds after
/// its front passed, given the `tail` that point's kind of sound is allowed
/// (see [`fade_tail`]). In `[0, 1]`, exactly `0.0` at and after `tail`.
///
/// Total over every f64 pair, including the ones no shipped caller can
/// produce: a negative `since_front` (the front has not arrived) answers
/// 0.0 rather than an exponential blow-up, a non-positive `tail` answers
/// 0.0 rather than dividing the shape by nothing, and any non-finite input
/// answers 0.0 rather than propagating NaN into the G-buffer. The clamp to
/// 1.0 is the same one the shader's `min(flare, 1.0)` applied before this
/// module existed, moved into the law so the pinned reference and the
/// rendered value are the same number.
#[must_use]
pub fn flare(since_front: f64, tail: f64) -> f64 {
    if !since_front.is_finite() || !tail.is_finite() || tail <= 0.0 {
        return 0.0;
    }
    if since_front < 0.0 || since_front >= tail {
        return 0.0;
    }
    (raw(since_front) - raw(tail)).clamp(0.0, 1.0)
}

/// The tail a pulse of `kind` grants the points it sweeps — the same
/// number [`PulsePool::emit`](crate::pulse_pool::PulsePool::emit) budgets
/// the slot's lifetime with, so a wave's reveal and its slot die together
/// rather than one outliving the other.
#[must_use]
pub fn reveal_tail(kind: i32) -> f64 {
    fade_tail(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE break this catches, and the reason the module exists: a wave
    /// whose front passed a surface a full tail ago must reveal NOTHING.
    /// Before this law the shader evaluated the raw decay with no end at
    /// all, so a tap's 6-second tail left 0.068 of peak on the surface and
    /// kept it there — the reveal outlived the sound and went dark only
    /// when a later sound reused its pool slot.
    ///
    /// Hand-derived, per kind, from the two constants: a tap's tail is 6.0
    /// s, where the raw shape still stands at 1.3·e^-24 + 0.5·e^-2 =
    /// 0.0677; a hum's is 2.0 s, where it stands at 0.5·e^-0.667 = 0.2568.
    /// Both must read exactly zero here.
    #[test]
    fn a_wave_reveals_nothing_once_its_tail_has_run_out() {
        for kind in [0, 1, 2, 3] {
            let tail = reveal_tail(kind);
            assert_eq!(
                flare(tail, tail),
                0.0,
                "kind {kind} still reveals at its own tail ({tail} s)"
            );
            assert_eq!(
                flare(tail + 1.0, tail),
                0.0,
                "kind {kind} still reveals a second past its tail"
            );
            assert_eq!(
                flare(1.0e6, tail),
                0.0,
                "kind {kind} reveals forever, not merely too long"
            );
        }
    }

    /// The landing is continuous, not a cliff: approaching the tail the
    /// envelope must already be vanishing, so the wave ends by fading
    /// rather than by being switched off. A cut-off raw decay would read
    /// 0.0677 one millisecond before a tap's tail; the shifted one reads
    /// under a thousandth.
    #[test]
    fn the_envelope_lands_on_zero_instead_of_stepping_to_it() {
        let tail = reveal_tail(0);
        let last = flare(tail - 0.001, tail);
        assert!(last > 0.0, "the envelope died early: {last}");
        assert!(
            last < 0.001,
            "the envelope steps off a cliff at the tail: {last}"
        );
    }

    /// The strike flash is untouched by the shift. Hand-derived: the raw
    /// shape at the front is 1.3 + 0.5 = 1.8, and every kind's shift is at
    /// most 0.2568, so every kind still saturates the clamp for the whole
    /// early part of its life. This is what makes the fix invisible where
    /// the player is actually looking and decisive where the bug was.
    #[test]
    fn the_strike_flash_still_saturates_for_every_kind() {
        for kind in [0, 1, 2, 3] {
            let tail = reveal_tail(kind);
            assert_eq!(flare(0.0, tail), 1.0, "kind {kind} lost its strike flash");
            // 0.25 s in, the raw shape is 1.3·e^-1 + 0.5·e^-0.0833 =
            // 0.9384; minus a hum's worst-case 0.2568 shift it is 0.6816,
            // so this is the region where the shift is genuinely visible
            // and must still be a strong reveal, not a dim one.
            let quarter = flare(0.25, tail);
            assert!(
                quarter > 0.6,
                "kind {kind} faded too fast right after the strike: {quarter}"
            );
        }
    }

    /// Monotone all the way down. A wave that brightens as it ages is not a
    /// wave, and the shift must not introduce a bump anywhere — the break
    /// this catches is a sign error in the subtraction.
    #[test]
    fn the_envelope_never_brightens_with_age() {
        let tail = reveal_tail(1);
        let mut previous = f64::INFINITY;
        for step in 0..=700 {
            let since = f64::from(step) * tail / 700.0;
            let now = flare(since, tail);
            assert!(
                now <= previous,
                "the envelope brightened at {since} s: {previous} -> {now}"
            );
            previous = now;
        }
        assert_eq!(previous, 0.0);
    }

    /// Total over inputs no shipped caller produces but the type admits.
    /// The shader guards the same cases with the same answers; without
    /// them a NaN reaches the G-buffer, where it is neither bright nor
    /// dark but poisons every neighbouring tap the hearing pass reads.
    #[test]
    fn degenerate_inputs_answer_darkness_rather_than_nonsense() {
        assert_eq!(flare(-1.0, 6.0), 0.0, "a front that has not arrived");
        assert_eq!(flare(1.0, 0.0), 0.0, "a kind granted no tail at all");
        assert_eq!(flare(1.0, -6.0), 0.0, "a negative tail");
        assert_eq!(flare(f64::NAN, 6.0), 0.0);
        assert_eq!(flare(1.0, f64::NAN), 0.0);
        assert_eq!(flare(f64::INFINITY, 6.0), 0.0);
        assert_eq!(flare(1.0, f64::INFINITY), 0.0);
        assert_eq!(flare(f64::NEG_INFINITY, 6.0), 0.0);
    }

    /// The tail this module hands the renderer is the SAME number the pool
    /// retires the slot with. Were they to drift, one of the two failure
    /// modes returns: a longer reveal tail outlives its own slot data, and
    /// a shorter one darkens a wave the ring in the air is still drawing.
    #[test]
    fn the_reveal_tail_is_the_pools_own_slot_lifetime() {
        for kind in [-1, 0, 1, 2, 3, 4, 99] {
            assert_eq!(reveal_tail(kind), fade_tail(kind), "kind {kind}");
        }
    }
}
