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
//! # How the envelope is brought to zero
//!
//! The shipped decay `1.3·e^(-t/0.25) + 0.5·e^(-t/3)` is a sum of
//! exponentials, so it is asymptotic: it never reaches zero. A pulse's slot
//! is retired at [`fade_tail`] seconds past the front, and the reveal must
//! be gone by then or the wave's visible end is decided by the slot
//! allocator instead of by the wave — which is exactly what happened: a
//! surface stayed lit until some later sound happened to reuse the slot,
//! and then went dark in one frame.
//!
//! Three ways to end it, and the choice matters more than it looks.
//!
//! *Cutting* the raw decay off at the tail trades an unbounded lifetime for
//! a visible step — 0.068 of peak for a tap, 0.257 for a hum.
//!
//! *Shifting* it down by its own value at the tail reaches zero exactly, but
//! darkens the WHOLE curve by that same kind-dependent amount: 17.3, 39.7,
//! 55.4 and 65.6 codes of 255 for kinds 0..3. A cane tap and its own echo
//! striking one surface would then read 0.3144 and 0.2264 at the same age —
//! a 22-code split with no cause in the world, only in the slot allocator.
//! The fade would become a function of the emitter's slot budget.
//!
//! *A closing window* is what this module does. The decay is the ACOUSTIC
//! law and the tail is the BUDGET, so the budget is allowed to end the wave
//! and nothing more: [`closing`] holds at 1.0 for the first
//! `1 - CLOSE_FRACTION` of every wave's life, leaving the shipped shape
//! untouched and identical for every kind, then smoothsteps to exactly zero
//! at the tail. The whole change is confined to the last quarter of a
//! wave's life, where it is a wave ending rather than a wave dimmed.

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

/// The fraction of a kind's tail spent closing the envelope out. Before it
/// the decay is the shipped shape untouched; across it a smooth window
/// carries the shape to exactly zero at the tail.
pub const CLOSE_FRACTION: f64 = 0.25;

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
    (raw(since_front) * closing(since_front, tail)).clamp(0.0, 1.0)
}

/// The closing window: 1.0 for the whole early life of a wave, then a
/// smoothstep down to exactly 0.0 at `tail`.
///
/// A window rather than a downward shift, and that is the design. The decay
/// is the ACOUSTIC law; the tail is the SLOT BUDGET. Subtracting the shape's
/// own value at the tail — the first repair of the immortal envelope — did
/// reach zero, but it darkened the entire curve by a kind-dependent amount,
/// so a cane tap and its own echo striking one surface read differently at
/// the same age for no reason but how many pool slots each was granted.
/// Multiplying instead confines the whole change to the last
/// [`CLOSE_FRACTION`] of each wave's life, leaves every kind decaying by one
/// law until then, and still lands on exactly zero.
fn closing(since_front: f64, tail: f64) -> f64 {
    let opens = tail * (1.0 - CLOSE_FRACTION);
    if since_front <= opens {
        return 1.0;
    }
    if since_front >= tail {
        return 0.0;
    }
    let across = (since_front - opens) / (tail - opens);
    1.0 - across * across * across.mul_add(-2.0, 3.0)
}

/// A sound source's standing acoustic image, as the two INDEPENDENT
/// numbers the x-ray skin needs rather than the single product it used to
/// be handed.
///
/// They were collapsed into one `volume * muffle` on the CPU and delivered
/// as a floor, which is what made the muffle powerless: a floor only ever
/// competes with the source's own wave reveal, and loses to it. Kept apart,
/// `muffle` can do the one thing a wall must be able to do — take something
/// away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceImage {
    /// How loud this source's silhouette stands on its own, before any wall
    /// is considered (`Volume::image`).
    pub volume: f64,
    /// What survives of it across the walls between the source and the EYE:
    /// `SOURCE_THROUGH^crossings`. A different occluder from the one the
    /// wave law uses — this one counts every wall, including the one the
    /// source stands inside.
    pub muffle: f64,
}

/// How brightly a sound source's own body reads: its standing image and any
/// wave currently washing it, whichever is stronger, and then everything
/// dimmed together by the walls between it and the eye.
///
/// The order is the law. `muffle * max(wave, volume)` lets a wall dim the
/// whole acoustic image; `max(wave, volume * muffle)` — which is what
/// shipped — lets the source's own wave step straight over the muffle,
/// because a source's hub is by construction unwalled from its own body, so
/// `wave` there is near 1.0 whatever stands between the source and the
/// player. Two walls, three walls, a whole map of walls: the silhouette
/// read the same. The documented `0.30 / 0.09 / 0.027` ladder existed only
/// while the source happened to be silent.
///
/// Total over every input: non-finite or negative arguments answer 0.0
/// rather than propagating into the G-buffer, and the result is clamped to
/// the channel's own `[0, 1]`.
#[must_use]
pub fn source_image(wave: f64, image: SourceImage) -> f64 {
    if !wave.is_finite() || !image.volume.is_finite() || !image.muffle.is_finite() {
        return 0.0;
    }
    let wave = wave.max(0.0);
    let volume = image.volume.max(0.0);
    let muffle = image.muffle.clamp(0.0, 1.0);
    (muffle * wave.max(volume)).clamp(0.0, 1.0)
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

    /// THE break this catches: the fade must be the SAME law for every kind
    /// until that kind's own life is nearly over. It is an acoustic decay,
    /// not a budget: how brightly a surface still rings a second after the
    /// front passed cannot depend on how many pool slots the emitter was
    /// granted.
    ///
    /// The first repair of the immortal envelope subtracted the shape's own
    /// value at the tail, which reached zero correctly but darkened the
    /// WHOLE curve by a kind-dependent amount — 17.3, 39.7, 55.4 and 65.6
    /// codes of 255 for kinds 0..3. A cane tap and its own echo striking one
    /// surface then read 0.3144 and 0.2264 at the same age: a 22-code split
    /// with no cause in the world, only in the slot allocator. Hand-derived
    /// from the two time constants: every kind must read
    /// 1.3·e^-4 + 0.5·e^-(1/3) = 0.3820760 one second after the front.
    #[test]
    fn the_fade_is_one_law_for_every_kind_until_its_own_close() {
        let expected = 1.3 * (-1.0_f64 / 0.25).exp() + 0.5 * (-1.0_f64 / 3.0).exp();
        assert!(
            (expected - 0.382_076).abs() < 1.0e-6,
            "derivation drifted: {expected}"
        );
        for kind in [0, 1, 2, 3] {
            let tail = reveal_tail(kind);
            let got = flare(1.0, tail);
            assert!(
                (got - expected).abs() < 1.0e-12,
                "kind {kind} (tail {tail} s) reads {got} one second after the front, \
                 not the {expected} every other kind reads — its slot budget is \
                 leaking into its acoustics"
            );
        }
    }

    /// The same law, stated as the boundary it actually holds to: until a
    /// wave enters its own closing window it decays EXACTLY as the shipped
    /// pre-fix shape did, clamped. That is what makes the death gate a
    /// bounded change rather than a re-tuning of the whole envelope, and it
    /// is checked across the full span rather than at one convenient point.
    #[test]
    fn before_its_close_the_envelope_is_the_shipped_shape_exactly() {
        for kind in [0, 1, 2, 3] {
            let tail = reveal_tail(kind);
            let close_starts = tail * (1.0 - CLOSE_FRACTION);
            for step in 0..=200 {
                let since = close_starts * f64::from(step) / 200.0;
                let shipped =
                    (1.3 * (-since / 0.25).exp() + 0.5 * (-since / 3.0).exp()).clamp(0.0, 1.0);
                let got = flare(since, tail);
                assert!(
                    (got - shipped).abs() < 1.0e-12,
                    "kind {kind} at {since} s reads {got}, not the shipped {shipped}"
                );
            }
        }
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

    /// THE break this catches: a wall must be able to take something away
    /// from a source's silhouette, and under the shipped
    /// `max(wave, volume * muffle)` it could not. A source's own hub is
    /// unwalled from its own body by construction, so the wave washing it
    /// is near full strength whatever stands between that source and the
    /// player — and the max then hands back that full strength, discarding
    /// the muffle entirely.
    ///
    /// Hand-derived from `SOURCE_THROUGH = 0.3`: a source at volume 1.0
    /// behind one wall must read 0.3 and behind two 0.09, whether or not
    /// its own wave is washing it at the time.
    #[test]
    fn a_wall_dims_a_source_even_while_its_own_wave_washes_it() {
        let one_wall = SourceImage {
            volume: 1.0,
            muffle: 0.3,
        };
        let two_walls = SourceImage {
            volume: 1.0,
            muffle: 0.09,
        };
        // silent: the ladder both the old and the new law agree on
        assert!((source_image(0.0, one_wall) - 0.3).abs() < 1e-12);
        assert!((source_image(0.0, two_walls) - 0.09).abs() < 1e-12);
        // and sounding, at the full strength its own wave reaches its own
        // body with — where the old law returned 1.0 for both
        assert!(
            (source_image(1.0, one_wall) - 0.3).abs() < 1e-12,
            "one wall bought the source nothing: {}",
            source_image(1.0, one_wall)
        );
        assert!(
            (source_image(1.0, two_walls) - 0.09).abs() < 1e-12,
            "two walls bought the source nothing: {}",
            source_image(1.0, two_walls)
        );
    }

    /// The ladder must keep DESCENDING as walls accumulate. Under the old
    /// law it flattened from the first wall on, so a source three rooms
    /// away was exactly as present as one next door — the perception
    /// fiction's whole point, inverted.
    #[test]
    fn the_muffle_ladder_still_descends_wall_after_wall() {
        let mut previous = f64::INFINITY;
        let mut muffle = 1.0;
        for wall in 0..4 {
            let now = source_image(
                1.0,
                SourceImage {
                    volume: 0.75,
                    muffle,
                },
            );
            assert!(
                now < previous,
                "wall {wall} changed nothing: {previous} -> {now}"
            );
            previous = now;
            muffle *= crate::level_plan::SOURCE_THROUGH;
        }
    }

    /// An unwalled source is untouched — the fix must not dim what the eye
    /// can see plainly. With `muffle` at 1.0 the law is exactly the old
    /// `max`, which is what makes this change invisible in the room the
    /// player is standing in and decisive everywhere else.
    #[test]
    fn an_unwalled_source_reads_exactly_as_before() {
        for wave in [0.0, 0.2, 0.5, 0.9, 1.0] {
            for volume in [0.0, 0.4, 0.75, 1.0] {
                let image = SourceImage {
                    volume,
                    muffle: 1.0,
                };
                assert!((source_image(wave, image) - wave.max(volume)).abs() < 1e-12);
            }
        }
    }

    /// Total over inputs the type admits and no shipped caller produces.
    /// A NaN here reaches G unclamped and poisons every neighbouring tap
    /// the hearing pass reads, so absence must answer darkness.
    #[test]
    fn a_degenerate_source_image_answers_darkness() {
        let sane = SourceImage {
            volume: 0.5,
            muffle: 0.5,
        };
        assert_eq!(source_image(f64::NAN, sane), 0.0);
        assert_eq!(
            source_image(
                1.0,
                SourceImage {
                    volume: f64::NAN,
                    muffle: 0.5
                }
            ),
            0.0
        );
        assert_eq!(
            source_image(
                1.0,
                SourceImage {
                    volume: 0.5,
                    muffle: f64::NAN
                }
            ),
            0.0
        );
        // negatives cannot darken below black, and an over-unity muffle
        // cannot brighten a source past the channel
        assert_eq!(source_image(-5.0, sane), 0.25);
        assert_eq!(
            source_image(
                1.0,
                SourceImage {
                    volume: 1.0,
                    muffle: 9.0
                }
            ),
            1.0
        );
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
