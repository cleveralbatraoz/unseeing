//! What one data channel can actually hold, and the geometric tolerance
//! that turns on it.
//!
//! # The two guards that met by accident
//!
//! `hearing_post` reconstructs a world point from the B channel —
//! `cam + rd * unpack_distance(c.b)` — and asks the wall table whether a
//! wall stands there. For a REAL surface that point must land outside the
//! wall it stands against, or the pass decides a lit wall is an x-rayed
//! source seen through one.
//!
//! The tolerance that has to cover that error is [`sight::RECT_SHRINK`]:
//! the occluder rect stops 0.05 m short of the wall's real face. It was
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
//! guard below would be broken outright — a half-code of 78 mm against a
//! 30 mm tolerance.
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
//!
//! # ...and a third number, which is neither
//!
//! Resolution is not accuracy. Everything above is about the smallest
//! DIFFERENCE the channel keeps, and it says nothing about where a single
//! value lands. [`TRANSFER_FLOOR`] is that second measurement, and it is
//! much worse: below about 28 nominal codes the shipped pipeline returns
//! exactly zero, so a wall one metre from the eye — packed as `1/40` —
//! read back as zero metres.
//!
//! [`pack_distance`] is the answer: distance is mapped into `[SAFE_FLOOR, 1]`
//! instead of `[0, 1]`, so nothing is ever written into the part of the
//! channel that dies. The cost is 45% of the codes, which
//! [`unpack_scale`] and the quanta below account for.

use crate::level_plan;
use crate::sight;

/// Distinct values one data channel preserves through the screen texture,
/// measured on desktop GL by `game/tests/probe/channel_probe.gd`.
///
/// 1024 — RGB10_A2. The channel therefore has `CHANNEL_LEVELS - 1` NOMINAL
/// steps between 0.0 and 1.0. That is the divisor every quantum below uses,
/// but it is not the whole of one: [`WORST_STEP_CODES`] says how many of
/// those nominal steps the pipeline actually needs to keep two values
/// apart, and the two are multiplied, never conflated.
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
/// Divided by the [`BAND`] the packing actually uses, not by the whole
/// channel: [`pack_distance`] maps the range onto `[SAFE_FLOOR, 1]`, so the
/// same range is spread over 55.1% of the codes and each one is worth
/// correspondingly more distance. Dividing by 1.0 here would understate the
/// quantum by that factor and hand [`recon_eps`] a tolerance the channel
/// does not deliver.
///
/// Total on any input: a non-finite or non-positive range answers
/// [`f64::INFINITY`] — "nothing is distinguishable" — which fails every
/// guard below rather than passing one.
#[must_use]
pub fn quantum(range: f64) -> f64 {
    if !range.is_finite() || range <= 0.0 {
        return f64::INFINITY;
    }
    range * WORST_STEP_CODES / (f64::from(CHANNEL_LEVELS - 1) * BAND)
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
/// `shrink > range * WORST_STEP_CODES / (2 * (LEVELS - 1) * BAND)`,
/// rearranged. At the shipped `RECT_SHRINK` of 0.05 m, 1024 codes, a worst
/// measured gap of 1.25 of them and a band of 55.1%, that is **45.12 m** —
/// and `DIST_PACK_RANGE` is 40.0, so the shipped build clears it by 5.12 m
/// of range, or 5.7 mm of tolerance. A 12.8% margin on a number
/// [`level_plan::pack_range_budget`] actively invites a designer to raise.
///
/// The ceiling FELL when distance moved into the band — from 49.10 m to
/// 45.12 m — because the same range now has 45% fewer codes to live in.
/// `RECT_SHRINK` rose from 0.03 to 0.05 to pay for it.
///
/// Total on any input: a non-finite or non-positive shrink answers 0.0, so
/// no range is safe rather than every range being safe.
#[must_use]
pub fn max_safe_range(shrink: f64) -> f64 {
    if !shrink.is_finite() || shrink <= 0.0 {
        return 0.0;
    }
    shrink * 2.0 * f64::from(CHANNEL_LEVELS - 1) * BAND / WORST_STEP_CODES
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
             cam + rd * B * range and asks the wall table about it; with {} codes per channel \
             and a widest measured gap of {} of them, that point can be off by {:.4} m, and the \
             only thing keeping it outside the wall it \
             stands against is sight::RECT_SHRINK ({} m). Past {ceiling:.2} m the two cross, a lit \
             wall starts reading as a source seen THROUGH a wall, and every ring is cut and every \
             outline capped at the wrong surface. Shrink the map instead, or stop reconstructing \
             a point from B.",
            CHANNEL_LEVELS,
            WORST_STEP_CODES,
            recon_eps(range),
            sight::RECT_SHRINK,
        ),
    })
}

/// The highest written value at which the shipped pipeline still moved a
/// reading by more than [`WORST_STEP_CODES`], MEASURED by
/// `game/tests/probe/tap_error_probe.gd` on Mesa 25.0.7 / AMD Radeon,
/// desktop GL, through the LDR path the game actually renders.
///
/// # The defect this number is the size of
///
/// Godot 4.7.1's `gl_compatibility` pass puts every value a spatial shader
/// writes to `ALBEDO` through an sRGB pair whose halves are not inverses:
/// `linear_to_srgb_exact(srgb_to_linear_cubic(v))`, the cubic polynomial in
/// one direction against the exact power law in the other. It is not a
/// driver bug — AMD radeonsi and llvmpipe software agree to the code — and
/// it is not storage quantisation: a `use_hdr_2d` `SubViewport`, a
/// half-float target, shows the transfer PURE. There is no render-target
/// escape hatch in this renderer.
///
/// Below about 28 nominal codes it does not merely bend, it ANNIHILATES:
/// everything at or under `v = 0.0274` comes back as exactly zero. The
/// shipped packing wrote `vd / 40`, so a wall one metre from the eye wrote
/// 0.025 and read back as zero metres — a 1.02 m error against a
/// [`sight::RECT_SHRINK`] of 0.03, and a systematic UNDERSHOOT, never an
/// overshoot.
///
/// # Why this is measured and not modelled
///
/// A transfer model fitted to the HDR path put this floor at 0.2384. The
/// shipped path measures 0.4484 — nearly twice as high, in the dangerous
/// direction. Something beyond the fitted transfer crushes the bottom of the
/// LDR channel and is not yet identified. So the constant is a READING, and
/// the probe that takes it runs before anything derived from it may be
/// typed. `docs/superpowers/specs/2026-08-19-distance-leaves-the-damaged-band-design.md`
/// carries the full tolerance ladder.
///
/// STILL TO MEASURE: SwiftShader and ANGLE/Apple Metal, through
/// `tools/measure_web_platform.sh`. This ships as the worst driver measured,
/// never the best, so a web reading above 0.4484 raises it.
pub const TRANSFER_FLOOR: f64 = 0.4484;

/// The bottom of the band distance is packed into: [`TRANSFER_FLOOR`]
/// rounded UP to the next whole channel code, never to a prettier decimal.
///
/// `ceil(0.4484 * 1023) / 1023 = 459 / 1023`. Rounding it down, or to 0.45,
/// puts the bottom of the packed band back inside the region the probe
/// measured as unreliable while every derivation below still computes.
pub const SAFE_FLOOR: f64 = 459.0 / 1023.0;

/// The part of the channel that survives the transfer — 55.1% of it.
///
/// Every derivation over the packed channel divides by this rather than by
/// 1.0, because that is how much room distance actually has.
pub const BAND: f64 = 1.0 - SAFE_FLOOR;

/// THE BREAK, discharged by the COMPILER rather than by a test, on the same
/// reasoning as [`super::crease::CreaseKnee::SHIPPED`]: rounding the
/// measured floor to a prettier decimal, or rounding it DOWN, puts the
/// bottom of the packed band back inside the region the probe measured as
/// unreliable — and every derivation in this module still computes, so
/// nothing else would notice.
///
/// The law is a BRACKET between two independently-sourced constants, not a
/// restatement of either: [`SAFE_FLOOR`] is at or above the reading, and
/// within one whole code of it, which is exactly "rounded up to the next
/// representable code". 0.4484 × 1023 = 458.71, so the floor is code 459
/// and not code 458 — and not 0.45, which would throw away 1.5 codes of
/// band for a rounder number.
const _: () = {
    assert!(
        SAFE_FLOOR >= TRANSFER_FLOOR,
        "the packed band starts below the measured transfer floor"
    );
    assert!(
        SAFE_FLOOR - TRANSFER_FLOOR < 1.0 / (CHANNEL_LEVELS - 1) as f64,
        "the floor was rounded up by more than one code, spending band for nothing"
    );
    // the band has to exist at all, and be most of the channel: it is the
    // divisor under every quantum and every gain here
    assert!(
        SAFE_FLOOR > 0.0 && BAND > 0.5,
        "the packed band is degenerate or smaller than half the channel"
    );
};

/// Pack a camera distance into the part of the channel the pipeline gives
/// back: `0..range` mapped affinely onto `[SAFE_FLOOR, 1]`.
///
/// The Rust twin of `data_core.gdshaderinc`'s `pack_data`. One fused
/// multiply-add where the shipped path had one clamp, and it closes the
/// sub-1.1 m hole at its root rather than moving it: the silhouette
/// Laplacian's five coefficients sum to zero, so the floor cancels EXACTLY
/// and only [`unpack_scale`]'s gain survives into the outline.
///
/// Total on any input: a non-finite distance, or a range that is not a
/// positive finite length, answers [`SAFE_FLOOR`] — the packing of zero
/// distance. That is the refusing end: the hearing pass reads it as a
/// surface at the eye, so `air_d` is zero, no ring root survives, and the
/// Laplacian of a plateau is zero, so no outline draws either. Absence
/// draws nothing rather than drawing something wrong.
#[must_use]
pub fn pack_distance(dist: f64, range: f64) -> f64 {
    if !dist.is_finite() || !range.is_finite() || range <= 0.0 {
        return SAFE_FLOOR;
    }
    SAFE_FLOOR + BAND * (dist / range).clamp(0.0, 1.0)
}

/// Recover a camera distance from a packed channel reading — the exact
/// inverse of [`pack_distance`], and the Rust twin of the
/// `unpack_distance` every B read in `hearing_post` now goes through.
///
/// Total on any input: a non-finite reading, or a range that is not a
/// positive finite length, answers 0.0. A reading BELOW the floor — what a
/// dead pass, or a material still writing the old packing, produces —
/// clamps to zero rather than answering a negative metre, and one above the
/// channel saturates at the range rather than running past the map.
#[must_use]
pub fn unpack_distance(packed: f64, range: f64) -> f64 {
    if !packed.is_finite() || !range.is_finite() || range <= 0.0 {
        return 0.0;
    }
    ((packed - SAFE_FLOOR) * unpack_scale(range)).clamp(0.0, range)
}

/// Metres of camera distance per unit of packed channel — the GAIN the
/// silhouette Laplacian is scaled by, and the one number the outline knee
/// turns on.
///
/// `range / BAND`: 72.55 m per unit at the shipped 40 m range, against the
/// 40 the unpacked channel would give. It is a separate door from
/// [`unpack_distance`] because the Laplacian never unpacks anything — the
/// floor cancels in the five-tap sum — so it needs the slope alone, and a
/// caller that reached for `unpack_distance` to get it would subtract a
/// floor that was never there.
///
/// Total on any input: a range that is not a positive finite length answers
/// 0.0, which flattens the Laplacian and draws no outline rather than an
/// infinite one — and so does a range whose GAIN overflows, which is the
/// case no guard on the range alone catches. Dividing by [`BAND`] scales a
/// range up by 1.81, so a finite range near [`f64::MAX`] produces an
/// infinite gain, and [`unpack_distance`] then computes `0.0 * inf` for a
/// reading exactly at the floor. That is NaN, and a NaN distance makes
/// `t >= air_d` false for every ring root at that pixel — one fragment
/// drawing every ring in the level through every wall.
#[must_use]
pub fn unpack_scale(range: f64) -> f64 {
    if !range.is_finite() || range <= 0.0 {
        return 0.0;
    }
    let scale = range / BAND;
    if scale.is_finite() { scale } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE BREAK: packing distance into `[0, 1]` again, which is what the
    /// shipped path did and what put a wall one metre from the eye at zero
    /// metres on screen.
    ///
    /// Hand-derived from the defect rather than from the packing: the
    /// pipeline's non-inverse sRGB pair returns everything at or below about
    /// 28 nominal codes — `v = 0.0274` — as exactly zero, and the old
    /// packing wrote `1 / 40 = 0.025` for a wall at one metre. It landed
    /// INSIDE the dead zone. Every packed value must now clear the measured
    /// floor, at every distance the range admits, including zero.
    #[test]
    fn nothing_is_ever_written_into_the_part_of_the_channel_that_dies() {
        const DEAD_ZONE: f64 = 0.0274;
        let range = level_plan::DIST_PACK_RANGE;
        // the old packing's own counter-example, stated so the fix cannot be
        // mistaken for a no-op
        assert!(1.0 / range < DEAD_ZONE, "1 m used to pack above the floor");
        for step in 0..=400 {
            let dist = f64::from(step) * range / 400.0;
            let packed = pack_distance(dist, range);
            assert!(
                packed >= SAFE_FLOOR,
                "a surface {dist} m away packed to {packed}, below the safe floor {SAFE_FLOOR}"
            );
            assert!(
                packed <= 1.0,
                "{dist} m packed to {packed}, past the channel"
            );
        }
        // and past the range it saturates rather than escaping the channel
        assert_eq!(pack_distance(range * 2.0, range), 1.0);
        assert_eq!(pack_distance(0.0, range), SAFE_FLOOR);
    }

    /// THE BREAK: the two ends of the packing drifting apart — a write that
    /// maps into the band and a read that still divides by the whole
    /// channel, or either one losing the floor. The hearing pass would then
    /// place every visible surface at the wrong distance, and it would look
    /// plausible, because the error is affine and smooth.
    ///
    /// Hand-derived at one point so the pair cannot BOTH be wrong in the
    /// same direction and still agree: a wall one metre from the eye packs
    /// to `459/1023 + (1 - 459/1023) / 40 = 0.462463`.
    #[test]
    fn a_distance_survives_the_round_trip_through_the_band() {
        let range = level_plan::DIST_PACK_RANGE;
        assert!(
            (pack_distance(1.0, range) - 0.462_463).abs() < 1.0e-6,
            "one metre packs to {}",
            pack_distance(1.0, range)
        );
        for step in 0..=400 {
            let dist = f64::from(step) * range / 400.0;
            let back = unpack_distance(pack_distance(dist, range), range);
            assert!(
                (back - dist).abs() < 1.0e-9,
                "{dist} m came back as {back} m"
            );
        }
    }

    /// THE BREAK: the silhouette Laplacian's gain drifting from the packing
    /// it is a Laplacian OF. The five-tap Laplacian's coefficients sum to
    /// zero, so the floor cancels exactly and only this gain survives — get
    /// it wrong and every silhouette knee in the game is off by the band
    /// width, which is a 45% error that looks like a taste decision.
    ///
    /// Hand-derived: 40 m spread over `1 - 459/1023` of the channel is
    /// 72.5532 m per unit of channel.
    #[test]
    fn the_unpack_gain_is_the_slope_the_packing_actually_has() {
        let range = level_plan::DIST_PACK_RANGE;
        assert!(
            (unpack_scale(range) - 72.553_191).abs() < 1.0e-5,
            "the gain moved: {}",
            unpack_scale(range)
        );
        // it IS the derivative, measured as one: a metre of separation
        // between two surfaces must survive the packing and the gain
        for &(near, far) in &[(0.0, 1.0), (3.0, 4.0), (17.5, 18.5), (38.0, 39.0)] {
            let packed_gap = pack_distance(far, range) - pack_distance(near, range);
            assert!(
                (packed_gap * unpack_scale(range) - (far - near)).abs() < 1.0e-9,
                "a {} m gap at {near} m came back as {} m",
                far - near,
                packed_gap * unpack_scale(range)
            );
        }
    }

    /// Total on the degenerate inputs the signatures admit. A NaN written to
    /// ALBEDO is undefined on the GPU, and a NaN read back out of B would
    /// make `t >= air_d` false for every ring root at that pixel — one bad
    /// pixel drawing every ring in the level through every wall.
    ///
    /// Absence answers the REFUSING end at each door: a packed floor (zero
    /// distance, so `air_d` is zero and no ring survives) and an unpacked
    /// zero (the same), never a distance that lets something through.
    #[test]
    fn a_degenerate_distance_or_range_answers_the_refusing_end() {
        assert_eq!(pack_distance(f64::NAN, 40.0), SAFE_FLOOR);
        assert_eq!(pack_distance(f64::INFINITY, 40.0), SAFE_FLOOR);
        assert_eq!(pack_distance(-1.0, 40.0), SAFE_FLOOR);
        assert_eq!(pack_distance(1.0, 0.0), SAFE_FLOOR);
        assert_eq!(pack_distance(1.0, f64::NAN), SAFE_FLOOR);

        assert_eq!(unpack_distance(f64::NAN, 40.0), 0.0);
        assert_eq!(unpack_distance(0.5, 0.0), 0.0);
        assert_eq!(unpack_distance(0.5, f64::NAN), 0.0);
        // a reading BELOW the floor is what a dead or pre-migration pass
        // writes; it must clamp to zero rather than answer a negative metre
        assert_eq!(unpack_distance(0.0, 40.0), 0.0);
        assert_eq!(unpack_distance(SAFE_FLOOR * 0.5, 40.0), 0.0);
        // ...and one above the channel saturates at the range
        assert_eq!(unpack_distance(2.0, 40.0), 40.0);

        assert_eq!(unpack_scale(0.0), 0.0);
        assert_eq!(unpack_scale(f64::NAN), 0.0);
        assert_eq!(unpack_scale(-40.0), 0.0);
        // A range that is finite and positive but so large that the gain
        // OVERFLOWS is still in the declared domain, and it is the one input
        // that reaches NaN by a route no guard above covers: the gain goes
        // infinite, and a reading exactly AT the floor then computes
        // `0.0 * inf`, which is NaN in every IEEE implementation. A NaN
        // distance makes `t >= air_d` false for every ring root at the
        // pixel, so one bad fragment draws every ring in the level through
        // every wall.
        assert_eq!(unpack_scale(f64::MAX), 0.0);
        assert_eq!(unpack_distance(SAFE_FLOOR, f64::MAX), 0.0);
        assert_eq!(unpack_distance(1.0, f64::MAX), 0.0);
        assert!(unpack_distance(SAFE_FLOOR, f64::MAX).is_finite());
    }

    /// THE guard, and the two numbers that had never been compared: the
    /// occluder's geometric tolerance and the channel's own quantum.
    ///
    /// Hand-derived at the shipped settings. 1024 codes give 1023 nominal
    /// steps, but distance lives in only 55.1% of them — [`BAND`] — so one
    /// nominal B code is worth 40 / (1023 x 0.55132) = 0.07092 m; the widest
    /// gap a driver actually showed is 1.25 of those, 0.08865 m, and the
    /// worst reconstruction error is half of that, 0.04433 m. RECT_SHRINK is
    /// 0.05 m, so the guard holds by 5.7 mm.
    ///
    /// It was 0.02444 m against a 0.03 m shrink while distance was packed
    /// into the whole channel — a channel whose bottom the pipeline destroys,
    /// so the number was tighter and describing a reconstruction that was
    /// already out by 1.02 m for a different reason entirely.
    #[test]
    fn a_reconstructed_point_lands_outside_the_wall_it_stands_against() {
        let eps = recon_eps(level_plan::DIST_PACK_RANGE);
        assert!(
            (eps - 0.044_326).abs() < 1.0e-5,
            "the derivation moved: {eps}"
        );
        assert!(
            sight::RECT_SHRINK > eps,
            "RECT_SHRINK ({}) no longer clears the B channel's half-gap ({eps}): a lit wall \
             will start reading as a source seen through one",
            sight::RECT_SHRINK
        );
        // and with margin, rather than sitting against it
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
        // TWO factors above the nominal code and they multiply: the measured
        // 1.25-code gap, and the 55.1% of the channel distance is packed
        // into. 1.25 / 0.55132 = 2.2673.
        assert!((quantum(range) / nominal - 1.25 / BAND).abs() < 1.0e-9);
        assert!((quantum(range) / nominal - 2.267_3).abs() < 1.0e-4);
        // the fiction, stated so it cannot come back unnoticed: at a
        // nominal code the half-quantum is 19.55 mm, which the RETIRED
        // 0.02 m tolerance cleared by 0.45 mm and the real gap does not
        assert!(nominal * 0.5 < 0.02);
        assert!(quantum(range) * 0.5 > 0.02);
    }

    /// The range that breaks it, stated rather than discovered. 45.12 m —
    /// `0.05 * 2 * 1023 * 0.55132 / 1.25` — against a shipped 40.0 and a map
    /// diagonal of 39.73.
    ///
    /// The ceiling FELL when distance moved into the band, from 49.10 m, and
    /// that is the honest cost of the repair: the same range now has 45%
    /// fewer codes to live in. Raising RECT_SHRINK from 0.03 to 0.05 bought
    /// most of it back. The margin a designer has on a number
    /// `pack_range_budget` invites them to raise is 5.12 m, down from 9.10.
    #[test]
    fn the_budget_refuses_a_range_the_channel_cannot_reconstruct() {
        let ceiling = max_safe_range(sight::RECT_SHRINK);
        assert!((ceiling - 45.12).abs() < 1.0e-9, "ceiling moved: {ceiling}");
        assert!(reconstruction_budget(level_plan::DIST_PACK_RANGE).is_none());
        assert!(reconstruction_budget(45.0).is_none());
        assert!(reconstruction_budget(45.2).is_some());
        assert!(reconstruction_budget(55.0).is_some());
        let complaint = reconstruction_budget(55.0).expect("55 m is past the ceiling");
        assert_eq!(complaint.severity, level_plan::Severity::Error);
        assert!(complaint.text.contains("45.12"));
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
    /// channel was — the guard fails by 2.6x even at the wider tolerance,
    /// and that is before the measured gap is applied to it. Kept as the
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
