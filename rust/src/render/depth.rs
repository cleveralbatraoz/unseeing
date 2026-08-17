//! The acoustic-image depth band — where a sound source's silhouette
//! rasterises so that it rides over the world yet still resolves against
//! itself and against other sources.
//!
//! # The layering trick, and the number it turns on
//!
//! A source is felt through walls, so its skin must defeat the hardware
//! depth test. It does that by writing a fragment `DEPTH` at the very top
//! of the window-depth range, where Godot's reversed-Z puts the near plane
//! — every source fragment then passes `GEQUAL` against every world
//! fragment. A single constant would be enough for one source and wrong for
//! two: two images writing the same value resolve by opaque draw order, and
//! the farther one, passing `GEQUAL` on an equal value, punches a hole
//! through the nearer.
//!
//! So the layer is a BAND: `ALWAYS_ON_TOP - SOURCE_BAND * (dist / range)`.
//! Its width is the whole design, and it is a derived quantity, not a taste.
//! Two conditions bracket it, and this module states both as functions
//! instead of asserting a literal in a comment:
//!
//! - **Wide enough to order.** The band is quantised twice over — the depth
//!   buffer's own [`DEPTH_CODES`] steps, and f32's ULP near 1.0, which is
//!   the same `2^-24` — so it can only carry `band * DEPTH_CODES` distinct
//!   values across the packing range. At `1.0e-5` that was 168 values over
//!   40 m: one code every 24 cm, which is coarser than any source is big.
//!   Every limb of the shipped fan lay inside a single code, so its housing,
//!   its guard and its blades resolved by opaque draw order — and since the
//!   blades' sort key moves as they spin, the crease pattern on the fan head
//!   reshuffled instead of rotating.
//! - **Narrow enough to stay unreachable.** No world fragment may ever
//!   rasterise into the band, or the world would start winning against the
//!   image it is meant to be felt through. Under reversed-Z that is a
//!   statement about distance, and [`deepest_world_fragment_in_band`]
//!   answers it exactly.
//!
//! The gap between those two bounds is enormous — four orders of magnitude —
//! which is why the shipped value could be wrong by a factor of a hundred
//! and still look right in a screenshot of one source.

/// The window depth a source fragment aims at: the near plane's own value
/// under Godot's reversed-Z, minus a hair. Not exactly 1.0, because the band
/// is subtracted from it and the arithmetic must stay inside the range.
pub const ALWAYS_ON_TOP: f64 = 0.999999;

/// How far below [`ALWAYS_ON_TOP`] the acoustic image may reach — the whole
/// layer's thickness in window depth, spread across [`CAM_FAR`]-bounded
/// distance by [`source_depth`].
///
/// `1.0e-3`, which resolves [`SOURCE_BAND`]-many codes to about 2.4 mm over
/// the packing range while still demanding a world fragment stand within
/// 0.05 mm of the near plane to reach it. The old `1.0e-5` satisfied only
/// the second condition.
pub const SOURCE_BAND: f64 = 1.0e-3;

/// Distinguishable values in the depth buffer: `2^24`. Godot's Compatibility
/// renderer uses a 24-bit depth buffer, and f32 — which the shader computes
/// [`source_depth`] in — has exactly the same `2^-24` ULP anywhere in
/// `[0.5, 1.0)`, so the two quantisations coincide and neither can rescue
/// the other.
pub const DEPTH_CODES: f64 = 16_777_216.0;

/// The eye's near and far planes, owned here because the band's whole
/// safety argument is a statement about them. `UnseeingPlayer` builds its
/// camera from these rather than from its own literals, so the derivation
/// below cannot quietly stop describing the shipped camera.
pub const CAM_NEAR: f64 = 0.05;
pub const CAM_FAR: f64 = 60.0;

/// The tightest gap between two surfaces of one sound source that the band
/// must still order correctly, in metres.
///
/// Measured off the shipped fan (`nodes::fan`): its guard torus spans
/// z ∈ [-0.12, -0.08] and its blade paddles z ∈ [-0.108, -0.092], so 0.012 m
/// separates the guard's front from the blade's. They wear different role
/// labels, so whichever wins a pixel decides where a crease is drawn — a gap
/// the band cannot resolve is a crease drawn by draw order.
pub const MIN_SOURCE_LIMB_GAP: f64 = 0.012;

/// Where a fragment `dist` metres from the eye lands in window depth under
/// Godot's reversed-Z: 1.0 at the near plane, 0.0 at the far plane.
///
/// `z = (n / (f - n)) * (f / d - 1)`, the standard reversed-Z perspective
/// mapping. Total on any input: a non-positive or non-finite distance, and a
/// degenerate frustum, answer 1.0 — the near plane, the most conservative
/// answer for every caller here, since it is the value the band must prove
/// unreachable.
#[must_use]
pub fn window_depth(dist: f64, near: f64, far: f64) -> f64 {
    if !dist.is_finite() || !near.is_finite() || !far.is_finite() || near <= 0.0 || far <= near {
        return 1.0;
    }
    if dist <= near {
        return 1.0;
    }
    ((near / (far - near)) * (far / dist - 1.0)).clamp(0.0, 1.0)
}

/// The greatest camera distance at which a WORLD fragment's window depth
/// still reaches into a band of width `band` below the near plane — the
/// exact inverse of [`window_depth`] at `1.0 - band`.
///
/// This is the band's safety margin, stated as the thing a reader can
/// picture: how close to the eye a wall would have to be before it started
/// competing with the acoustic image drawn over it. Anything beyond this
/// distance is strictly below the band.
///
/// Total on any input: a band outside `(0, 1)` or a degenerate frustum
/// answers `near`, meaning "nothing beyond the near plane can reach it".
#[must_use]
pub fn deepest_world_fragment_in_band(band: f64, near: f64, far: f64) -> f64 {
    if !band.is_finite() || band <= 0.0 || band >= 1.0 {
        return near;
    }
    if !near.is_finite() || !far.is_finite() || near <= 0.0 || far <= near {
        return near;
    }
    // z >= 1 - band  <=>  d <= f / (1 + (1 - band) * (f - n) / n)
    far / (1.0 + (1.0 - band) * (far - near) / near)
}

/// Where in the acoustic-image band a source fragment `dist` metres from the
/// eye writes its depth — the Rust twin of `data_core.gdshaderinc`'s
/// `source_depth`.
///
/// Total on any input: a non-finite distance or a non-positive range writes
/// the top of the band, which still beats every world fragment.
#[must_use]
pub fn source_depth(dist: f64, range: f64) -> f64 {
    if !dist.is_finite() || !range.is_finite() || range <= 0.0 {
        return ALWAYS_ON_TOP;
    }
    ALWAYS_ON_TOP - SOURCE_BAND * (dist / range).clamp(0.0, 1.0)
}

/// Metres of camera distance per distinguishable depth code inside the band
/// — how far two source surfaces must be apart before the hardware can tell
/// which is nearer. Two limbs closer than this resolve by opaque draw order.
///
/// Total on any input: a degenerate band or range answers [`f64::INFINITY`],
/// "nothing is resolvable", which fails every gate rather than passing one.
#[must_use]
pub fn band_resolution(band: f64, range: f64) -> f64 {
    if !band.is_finite() || !range.is_finite() || band <= 0.0 || range <= 0.0 {
        return f64::INFINITY;
    }
    range / (band * DEPTH_CODES)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE break this catches: a band too narrow to order a source against
    /// ITSELF, which is what the fourteen-line comment above the constant
    /// promises it does. At `1.0e-5` the band carried 1e-5 * 2^24 = 168
    /// codes across 40 m — one every 0.238 m — so the shipped fan's guard,
    /// blades and housing, all within 0.09 m of one another, wrote the same
    /// depth and resolved by opaque draw order.
    ///
    /// Hand-derived: the shipped band must resolve `MIN_SOURCE_LIMB_GAP`
    /// (0.012 m, the fan's guard-to-blade separation) with margin.
    #[test]
    fn the_band_resolves_a_sources_own_overlapping_limbs() {
        let resolution = band_resolution(SOURCE_BAND, crate::level_plan::DIST_PACK_RANGE);
        assert!(
            resolution < MIN_SOURCE_LIMB_GAP,
            "one depth code spans {resolution} m, coarser than the {MIN_SOURCE_LIMB_GAP} m gap \
             between a fan's guard and its blades — those surfaces resolve by draw order"
        );
        // and the old value is the counter-example that proves this
        // assertion is not vacuous
        assert!(band_resolution(1.0e-5, crate::level_plan::DIST_PACK_RANGE) > MIN_SOURCE_LIMB_GAP);
    }

    /// The other bound, and the one the band must never cross: no world
    /// fragment may rasterise into it. Hand-derived from the shipped camera
    /// — near 0.05, far 60 — a surface must stand within 0.05005 m of the
    /// eye to reach a band of 1e-3, which is 0.05 mm past a near plane that
    /// already clips everything closer.
    #[test]
    fn no_world_fragment_can_reach_the_band() {
        let deepest = deepest_world_fragment_in_band(SOURCE_BAND, CAM_NEAR, CAM_FAR);
        assert!(
            deepest < CAM_NEAR + 0.001,
            "a world surface {deepest} m from the eye reaches the acoustic-image band; \
             the near plane is at {CAM_NEAR}"
        );
        // stated the other way round, over the whole playable range: every
        // world fragment a millimetre past the near plane is strictly below
        // the band's floor
        let floor = ALWAYS_ON_TOP - SOURCE_BAND;
        for &dist in &[0.051, 0.1, 1.0, 5.0, 20.0, 40.0, 59.9] {
            let depth = window_depth(dist, CAM_NEAR, CAM_FAR);
            assert!(
                depth < floor,
                "a world fragment at {dist} m writes depth {depth}, inside the band below {floor}"
            );
        }
    }

    /// The band orders sources by TRUE distance, nearer over farther, which
    /// is the whole reason it is a band and not a point. Strictly monotone
    /// at the resolution the previous test establishes.
    #[test]
    fn a_nearer_source_always_wins_the_band() {
        let range = crate::level_plan::DIST_PACK_RANGE;
        let step = MIN_SOURCE_LIMB_GAP;
        let mut previous = f64::INFINITY;
        let mut dist = 0.0;
        while dist <= range {
            let depth = source_depth(dist, range);
            assert!(
                depth < previous,
                "a source at {dist} m did not sort under the one {step} m nearer"
            );
            assert!(depth <= ALWAYS_ON_TOP);
            assert!(depth >= ALWAYS_ON_TOP - SOURCE_BAND);
            previous = depth;
            dist += step;
        }
    }

    /// Past the packing range the band saturates rather than wrapping: two
    /// sources 50 and 60 m away write the same depth and resolve by draw
    /// order again. That is a real limit and it is stated here rather than
    /// discovered — the clamp is deliberate, because wrapping would put a
    /// distant source ABOVE a near one, which is far worse.
    #[test]
    fn the_band_saturates_past_the_packing_range_instead_of_wrapping() {
        let range = crate::level_plan::DIST_PACK_RANGE;
        let at_range = source_depth(range, range);
        assert_eq!(source_depth(range * 1.5, range), at_range);
        assert_eq!(at_range, ALWAYS_ON_TOP - SOURCE_BAND);
        // and nothing ever escapes the band upward, however close
        assert_eq!(source_depth(-5.0, range), ALWAYS_ON_TOP);
    }

    /// Total on the degenerate frusta and ranges the types admit. A NaN
    /// written to DEPTH is undefined behaviour on the GPU, so absence must
    /// answer the conservative end of the band rather than propagate.
    #[test]
    fn degenerate_frusta_answer_conservatively_rather_than_crashing() {
        assert_eq!(window_depth(f64::NAN, CAM_NEAR, CAM_FAR), 1.0);
        assert_eq!(window_depth(1.0, 0.0, CAM_FAR), 1.0);
        assert_eq!(window_depth(1.0, 10.0, 10.0), 1.0);
        assert_eq!(window_depth(0.0, CAM_NEAR, CAM_FAR), 1.0);
        assert_eq!(
            deepest_world_fragment_in_band(0.0, CAM_NEAR, CAM_FAR),
            CAM_NEAR
        );
        assert_eq!(
            deepest_world_fragment_in_band(2.0, CAM_NEAR, CAM_FAR),
            CAM_NEAR
        );
        assert_eq!(
            deepest_world_fragment_in_band(f64::NAN, CAM_NEAR, CAM_FAR),
            CAM_NEAR
        );
        assert_eq!(source_depth(f64::NAN, 40.0), ALWAYS_ON_TOP);
        assert_eq!(source_depth(1.0, 0.0), ALWAYS_ON_TOP);
        assert_eq!(band_resolution(0.0, 40.0), f64::INFINITY);
        assert_eq!(band_resolution(SOURCE_BAND, 0.0), f64::INFINITY);
    }
}
