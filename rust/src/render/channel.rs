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
//! Neither [`CHANNEL_LEVELS`] nor [`WORST_STEP_CODES`] is a guess or a
//! platform assumption. The format is measured by
//! `game/tests/probe/channel_probe.gd`, which writes two values one
//! candidate step apart into the data channels, reads both back through
//! `hint_screen_texture`, and amplifies the difference inside the shader —
//! where the precision still exists — so that an 8-bit screenshot can
//! report a deeper buffer. It is RGB10_A2, which settles a question the
//! project had two stories about: the brief said 8-bit LDR. At 8 bits the
//! guard below would already be broken by a factor of four.
//!
//! # What the format promises and what the pipeline delivers
//!
//! Those are not the same number, and believing they were is what put the
//! guard below in the red for months.
//!
//! `game/tests/probe/platform_probe.tscn` lays a whole ladder out across
//! one frame: each band writes a base on the left and base + step on the
//! right, the base sweeps down the column, and a pixel is white where the
//! two survived as distinct codes. Read across EVERY row rather than a
//! sample of seventeen, and laddered in multiples of a nominal 10-bit step
//! rather than in whole bits, it says:
//!
//! | driver | smallest step that survives at every base |
//! |---|---|
//! | Mesa 25.0 / AMD Radeon, desktop GL | **1.25** nominal codes |
//! | SwiftShader, WebGL2 | 1.02 |
//! | ANGLE / Apple Metal (A18 Pro), WebGL2 | 1.02 |
//!
//! So the channel is a 1024-code buffer whose worst local gap is wider than
//! one code. The rest of the ladder says the probe is reading a real
//! quantiser and not noise: a uniform 1/1023 grid must collapse half its
//! bases at a half step and a tenth at nine tenths, and the same run
//! measured 49.2% and 8.8%.
//!
//! The old seventeen-base sweep missed this because the collapse is rare —
//! two bases in 649 on the AMD part — and a power-of-two ladder could only
//! ever answer 512 or 1024 anyway. It reported 1024, the guard cleared its
//! quantum by 0.45 mm, and the real gap was 25% wider than the one being
//! cleared.
//!
//! 1.25 is the widest gap MEASURED, on three drivers, not a proven ceiling
//! for every driver that will ever run this. That is why the tolerance
//! carries visible margin over it rather than sitting against it, and why
//! [`reconstruction_budget`] stays a derived predicate rather than a
//! hardcoded conclusion.

use crate::level_plan;
use crate::sight;

/// Distinct values one data channel preserves through the screen texture,
/// measured on desktop GL by `game/tests/probe/channel_probe.gd`.
///
/// 1024 — RGB10_A2. The channel therefore has `CHANNEL_LEVELS - 1` steps
/// between 0.0 and 1.0, which is the divisor every quantum below uses.
pub const CHANNEL_LEVELS: u32 = 1024;

/// The widest gap the channel actually showed, in units of one nominal
/// code, measured by `game/tests/probe/platform_probe.tscn` across every
/// base in a swept column.
///
/// 1.25 on Mesa/AMD desktop GL; 1.02 on SwiftShader and on ANGLE/Apple
/// Metal in a browser. The largest is the one every derivation below uses,
/// because a guard that holds on the friendliest driver holds nowhere.
///
/// This is separate from [`CHANNEL_LEVELS`] on purpose. That one is the
/// format — RGB10_A2, a fact about the buffer. This one is what the whole
/// path from `ALBEDO` through the back-buffer copy and back out of
/// `hint_screen_texture` delivers, which is the thing the hearing pass
/// actually suffers. Folding them into one "effective levels" number would
/// bury a measurement inside a format.
pub const WORST_STEP_CODES: f64 = 1.25;

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
    range * WORST_STEP_CODES / f64::from(CHANNEL_LEVELS - 1)
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
    shrink * 2.0 * f64::from(CHANNEL_LEVELS - 1) / WORST_STEP_CODES
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
    /// Hand-derived at the shipped settings. 1024 codes give 1023 nominal
    /// steps across the range, so one nominal B code is 40 / 1023 =
    /// 0.03910 m; the widest gap a driver actually showed is 1.25 of those,
    /// 0.04888 m, and the worst reconstruction error is half of that,
    /// 0.02444 m. RECT_SHRINK is 0.03 m, so the guard holds by 5.56 mm.
    #[test]
    fn a_reconstructed_point_lands_outside_the_wall_it_stands_against() {
        let eps = recon_eps(level_plan::DIST_PACK_RANGE);
        assert!(
            (eps - 0.024_438).abs() < 1.0e-5,
            "the derivation moved: {eps}"
        );
        assert!(
            sight::RECT_SHRINK > eps,
            "RECT_SHRINK ({}) no longer clears the B channel's half-gap ({eps}): a lit wall \
             will start reading as a source seen through one",
            sight::RECT_SHRINK
        );
        // and with margin this time, rather than the 0.45 mm the nominal
        // derivation left
        assert!(sight::RECT_SHRINK - eps > 0.005);
    }

    /// THE BREAK: reading the format's promise as the pipeline's delivery.
    ///
    /// A nominal 10-bit code is 1/1023 of the range. What the ladder in
    /// `game/tests/probe/platform_probe.tscn` measures, at every base of a
    /// swept column rather than seventeen of them, is that a step that size
    /// collapses to ONE code at a few bases on Mesa/AMD — the smallest step
    /// that always survived there was 1.25 nominal codes.
    ///
    /// So the quantum must be wider than the nominal one, and by exactly
    /// that factor. Setting [`WORST_STEP_CODES`] back to 1.0 restores the
    /// old fiction: the guard above would then clear its quantum by 0.45 mm
    /// while the gap it is really covering is 25% larger than the one it
    /// checked, which is a guard that passes its own test and fails on a
    /// screen.
    #[test]
    fn the_quantum_is_the_gap_measured_not_the_code_promised() {
        let range = level_plan::DIST_PACK_RANGE;
        let nominal = range / f64::from(CHANNEL_LEVELS - 1);
        assert!(
            (nominal - 0.039_100).abs() < 1.0e-5,
            "nominal moved: {nominal}"
        );
        assert!(
            quantum(range) > nominal,
            "the quantum stopped accounting for the measured gap"
        );
        assert!((quantum(range) / nominal - 1.25).abs() < 1.0e-9);
        // the fiction, stated so it cannot come back unnoticed: at a
        // nominal code the half-quantum is 19.55 mm, which the RETIRED
        // 0.02 m tolerance cleared by 0.45 mm and the real gap does not
        assert!(nominal * 0.5 < 0.02);
        assert!(quantum(range) * 0.5 > 0.02);
    }

    /// The range that breaks it, stated rather than discovered. 49.10 m —
    /// `0.03 * 2 * 1023 / 1.25` — against a shipped 40.0 and a map diagonal
    /// of 39.73. The wider tolerance bought headroom on a number the
    /// pack-range budget tells designers to raise when the map grows: it
    /// was under one metre at the old shrink and nominal code.
    #[test]
    fn the_budget_refuses_a_range_the_channel_cannot_reconstruct() {
        let ceiling = max_safe_range(sight::RECT_SHRINK);
        assert!(
            (ceiling - 49.104).abs() < 1.0e-9,
            "ceiling moved: {ceiling}"
        );
        assert!(reconstruction_budget(level_plan::DIST_PACK_RANGE).is_none());
        assert!(reconstruction_budget(49.0).is_none());
        assert!(reconstruction_budget(49.2).is_some());
        assert!(reconstruction_budget(55.0).is_some());
        let complaint = reconstruction_budget(55.0).expect("55 m is past the ceiling");
        assert_eq!(complaint.severity, level_plan::Severity::Error);
        assert!(complaint.text.contains("49.10"));
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
        // still a counter-example at the wider tolerance, and by 2.6x
        assert!(
            sight::RECT_SHRINK < eight_bit_eps,
            "the counter-example stopped being one"
        );
    }
}
