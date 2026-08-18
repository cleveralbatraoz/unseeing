//! What one data channel can actually hold, and the geometric tolerance
//! that turns on it.
//!
//! # The two guards that met by accident
//!
//! `hearing_post` reconstructs a world point from the B channel —
//! `cam + rd * c.b * DIST_PACK_RANGE` — and asks the wall table whether a
//! wall stands there. For a REAL surface that point must land outside the
//! wall it stands against, or the pass decides a lit wall is an x-rayed
//! source seen through one.
//!
//! The tolerance that has to cover that error is [`sight::RECT_SHRINK`]:
//! the occluder rect stops 0.02 m short of the wall's real face. It was
//! chosen for something else entirely — so a prop standing flush against a
//! wall is not self-shadowed by contact grazing — and it has to exceed the
//! reconstruction error, which is half a B-channel quantum. Neither number
//! knew the other existed.
//!
//! It is not quite the only tolerance in play: [`sight::GRAZE_EPS`] also
//! ignores a thousandth of the sight line at each end, which for an
//! eye-to-surface ray of length L forgives a further `L / 1000` — 4 cm at
//! 40 m, and nothing at all at close range. It is not counted below,
//! deliberately: it shrinks toward zero exactly where the reconstruction
//! error does not, so leaning on it would make the guard weakest where the
//! geometry is nearest and the pixels largest.
//!
//! # The measurement
//!
//! [`CHANNEL_LEVELS`] is not a guess and not a platform assumption. It is
//! measured by `game/tests/probe/channel_probe.gd`, which writes two values
//! one candidate step apart into the data channels, reads both back through
//! `hint_screen_texture`, and amplifies the difference inside the shader —
//! where the precision still exists — so that an 8-bit screenshot can
//! report a deeper buffer.
//!
//! Measured as a WORST CASE across the channel, which matters: a single
//! base sits at one arbitrary place on the quantisation grid and that alone
//! moves the answer by a full bit. `0.5 * 1023 = 511.5` lies exactly
//! between two 10-bit codes, so half a code still crosses a boundary there,
//! while `0.25 * 1023 = 255.75` does not. Measured at fixed bases this
//! channel reported 2^-11 at 0.5 and 2^-10 at 0.25 — one buffer, two
//! answers. Swept across seventeen bases and demanding every one separate,
//! it holds exactly 1024 levels.
//!
//! That settles a question the project had two stories about: the brief
//! said 8-bit LDR and one earlier probe claimed RGB10_A2. It is RGB10_A2.
//! At 8 bits the guard below would already be broken by a factor of four.
//!
//! # What is still unmeasured
//!
//! The WEB target. This is a desktop GL measurement, and WebGL2 may hand
//! Godot a different default format. If the web buffer is 8-bit, half a
//! quantum is 78 mm against a 20 mm tolerance and the reconstruction is
//! wrong there — see [`reconstruction_budget`], which is why the check is a
//! derived predicate rather than a comment saying "fine on my machine".

use crate::level_plan;
use crate::sight;

/// Distinct values one data channel preserves through the screen texture,
/// measured on desktop GL by `game/tests/probe/channel_probe.gd`.
///
/// 1024 — RGB10_A2. The channel therefore has `CHANNEL_LEVELS - 1` steps
/// between 0.0 and 1.0, which is the divisor every quantum below uses.
pub const CHANNEL_LEVELS: u32 = 1024;

/// Metres of camera distance per distinguishable B-channel code, at a given
/// packing range.
///
/// Total on any input: a non-finite or non-positive range answers
/// [`f64::INFINITY`] — "nothing is distinguishable" — which fails every
/// guard below rather than passing one.
#[must_use]
pub fn quantum(range: f64) -> f64 {
    if !range.is_finite() || range <= 0.0 {
        return f64::INFINITY;
    }
    range / f64::from(CHANNEL_LEVELS - 1)
}

/// The worst error in a world point reconstructed from B — half a quantum,
/// since the channel rounds to the nearest code.
#[must_use]
pub fn recon_eps(range: f64) -> f64 {
    quantum(range) * 0.5
}

/// The largest packing range at which a point reconstructed from B is still
/// guaranteed to land outside the wall it stands against, given the
/// occluder's own `shrink`.
///
/// `shrink > range / (2 * (LEVELS - 1))`, rearranged. At the shipped
/// `RECT_SHRINK` of 0.02 m and 1024 levels that is **40.92 m** — and
/// `DIST_PACK_RANGE` is 40.0, so the shipped build clears it by 0.92 m of
/// range, or 0.45 mm of tolerance. A 2.3% margin on a number
/// [`level_plan::pack_range_budget`] actively invites a designer to raise.
///
/// Total on any input: a non-finite or non-positive shrink answers 0.0, so
/// no range is safe rather than every range being safe.
#[must_use]
pub fn max_safe_range(shrink: f64) -> f64 {
    if !shrink.is_finite() || shrink <= 0.0 {
        return 0.0;
    }
    shrink * 2.0 * f64::from(CHANNEL_LEVELS - 1)
}

/// The complaint a packing range earns when the hearing pass could no
/// longer trust the world point it reconstructs from B.
///
/// `None` while the range is safe. This is a SEPARATE law from
/// [`level_plan::pack_range_budget`], which is about the map outgrowing the
/// range; this one is about the range outgrowing the channel, and the two
/// pull in opposite directions — a designer told to raise the range because
/// the map is too big is being walked straight into this one.
#[must_use]
pub fn reconstruction_budget(range: f64) -> Option<level_plan::Budget> {
    let ceiling = max_safe_range(sight::RECT_SHRINK);
    if range.is_finite() && range > 0.0 && range < ceiling {
        return None;
    }
    Some(level_plan::Budget {
        severity: level_plan::Severity::Error,
        text: format!(
            "WaveLevel: a DIST_PACK_RANGE of {range} m is too coarse for the B channel to \
             reconstruct a world point safely. hearing_post rebuilds the visible surface as \
             cam + rd * B * range and asks the wall table about it; with {} levels per channel \
             that point can be off by {:.4} m, and the only thing keeping it outside the wall it \
             stands against is sight::RECT_SHRINK ({} m). Past {ceiling:.2} m the two cross, a lit \
             wall starts reading as a source seen THROUGH a wall, and every ring is cut and every \
             outline capped at the wrong surface. Shrink the map instead, or stop reconstructing \
             a point from B.",
            CHANNEL_LEVELS,
            recon_eps(range),
            sight::RECT_SHRINK,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE guard, and the two numbers that had never been compared: the
    /// occluder's geometric tolerance and the channel's own quantum.
    ///
    /// Hand-derived at the shipped settings. 1024 levels give 1023 steps
    /// across the range, so one B code is 40 / 1023 = 0.03910 m and the
    /// worst reconstruction error is half of that, 0.01955 m. RECT_SHRINK
    /// is 0.02 m. The guard holds by 0.45 mm — a margin of 2.3%, on a
    /// tolerance chosen for an unrelated reason.
    #[test]
    fn a_reconstructed_point_lands_outside_the_wall_it_stands_against() {
        let eps = recon_eps(level_plan::DIST_PACK_RANGE);
        assert!(
            (eps - 0.019_550).abs() < 1.0e-5,
            "the derivation moved: {eps}"
        );
        assert!(
            sight::RECT_SHRINK > eps,
            "RECT_SHRINK ({}) no longer clears the B channel's half-quantum ({eps}): a lit wall \
             will start reading as a source seen through one",
            sight::RECT_SHRINK
        );
        // ...and the margin is thin enough to be worth saying out loud
        assert!(sight::RECT_SHRINK - eps < 0.001);
    }

    /// The range that breaks it, stated rather than discovered. 40.92 m,
    /// against a shipped 40.0 and a map diagonal of 39.73 — under one metre
    /// of headroom on a number the pack-range budget tells designers to
    /// raise when the map grows.
    #[test]
    fn the_budget_refuses_a_range_the_channel_cannot_reconstruct() {
        let ceiling = max_safe_range(sight::RECT_SHRINK);
        assert!((ceiling - 40.92).abs() < 1.0e-9, "ceiling moved: {ceiling}");
        assert!(reconstruction_budget(level_plan::DIST_PACK_RANGE).is_none());
        assert!(reconstruction_budget(40.9).is_none());
        assert!(reconstruction_budget(41.0).is_some());
        assert!(reconstruction_budget(45.0).is_some());
        let complaint = reconstruction_budget(45.0).expect("45 m is past the ceiling");
        assert_eq!(complaint.severity, level_plan::Severity::Error);
        assert!(complaint.text.contains("40.92"));
    }

    /// Total on the degenerate ranges the type admits: absence answers "no
    /// range is safe", never "every range is".
    #[test]
    fn a_degenerate_range_is_refused_rather_than_waved_through() {
        assert_eq!(quantum(0.0), f64::INFINITY);
        assert_eq!(quantum(-40.0), f64::INFINITY);
        assert_eq!(quantum(f64::NAN), f64::INFINITY);
        assert!(reconstruction_budget(0.0).is_some());
        assert!(reconstruction_budget(-40.0).is_some());
        assert!(reconstruction_budget(f64::NAN).is_some());
        assert_eq!(max_safe_range(0.0), 0.0);
        assert_eq!(max_safe_range(f64::NAN), 0.0);
    }

    /// At 8 bits — which is what the project's own brief claimed the
    /// channel was — the guard fails by a factor of four. Kept as the
    /// counter-example that makes the measurement load-bearing rather than
    /// decorative: if `CHANNEL_LEVELS` is ever set from a story instead of
    /// a probe, this is the size of the mistake.
    #[test]
    fn at_eight_bits_the_shipped_tolerance_would_not_clear_the_quantum() {
        let eight_bit_eps = level_plan::DIST_PACK_RANGE / f64::from(255_u32) * 0.5;
        assert!((eight_bit_eps - 0.078_431).abs() < 1.0e-5);
        assert!(
            sight::RECT_SHRINK < eight_bit_eps,
            "the counter-example stopped being one"
        );
    }
}
