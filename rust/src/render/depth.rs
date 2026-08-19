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

/// Is the fragment at this window `depth` an ACOUSTIC IMAGE rather than a
/// real surface? The Rust twin of `pulse_pool.gdshaderinc`'s
/// `depth_is_acoustic_image`, which had none.
///
/// One comparison, and it is exact rather than inferred: the source skin
/// writes its `DEPTH` inside the band, and
/// [`no_world_fragment_can_reach_the_band`] is the cargo test that no world
/// fragment ever does. Measured on desktop GL by
/// `game/tests/probe/depth_texture_probe.gd` — an always-on-top fragment
/// reads back 1.0000 against a band floor of 0.9990, a wall three metres out
/// reads 0.0158.
///
/// The test is one-sided ABOVE on purpose: a real reading measures 1.0,
/// which is above [`ALWAYS_ON_TOP`] itself, so an upper bound at the band's
/// top would refuse every source in the game. The bound that does apply is
/// that a window depth lives in `[0, 1]` at all.
///
/// Total on any input: anything that is not a window depth answers false —
/// not an image — so a dead or garbage texture degrades every caller to the
/// behaviour it had before the depth read existed, and never past it.
#[must_use]
pub fn is_acoustic_image(depth: f64) -> bool {
    if !depth.is_finite() || !(0.0..=1.0).contains(&depth) {
        return false;
    }
    // Compared in the width the GPU compares in, for the reason
    // [`super::knee::Knee::new`] orders in it: f32 is what the shader holds,
    // and an f64 comparison here is STRICTER than the one that ships. At the
    // band's own floor that difference decides the answer — see
    // `the_bands_floor_survives_the_buffer_that_stores_it`.
    (depth as f32) >= (ALWAYS_ON_TOP - SOURCE_BAND) as f32
}

/// How far from the eye the source at this window `depth` stands — the
/// inverse of [`source_depth`] over the band, and the WITNESS the hearing
/// pass corroborates its colour-channel distance with at an x-rayed pixel.
///
/// It exists because the colour channel cannot be trusted alone: the
/// pipeline's transfer destroys the bottom of it
/// ([`super::channel::TRANSFER_FLOOR`]), while the depth buffer is untouched
/// by any colour transform. At an x-rayed pixel the world's own depth is
/// gone — the source overwrote it — so the band inverse is the only reading
/// available there, and this is it.
///
/// `None` unless the depth is one [`is_acoustic_image`] admits, so the two
/// doors cannot disagree about which pixels have a witness at all. That is
/// the whole safety of the design: absence degrades the caller to exactly
/// the colour channel, never below it.
///
/// The band SATURATES past `range`, exactly as [`source_depth`] does, so a
/// source at 50 m and one at 60 m both answer `range`. That is a real limit
/// and it is why the witness must corroborate a reading rather than replace
/// one.
///
/// Total on any input: a reading that is not a window depth, or a range that
/// is not a positive finite length, answers `None`.
#[must_use]
pub fn source_distance(depth: f64, range: f64) -> Option<f64> {
    if !range.is_finite() || range <= 0.0 || !is_acoustic_image(depth) {
        return None;
    }
    Some(((ALWAYS_ON_TOP - depth) / SOURCE_BAND * range).clamp(0.0, range))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE BREAK: the band inverse and the band's own membership test
    /// disagreeing about what an acoustic image IS. They are one law read
    /// two ways — "is this fragment a source?" and "then how far away is
    /// it?" — and a pixel one answers yes to and the other answers None to
    /// is a witness that silently never fires, or one that fires on the
    /// world.
    ///
    /// Swept across the whole window-depth range rather than at the two
    /// endpoints, because the disagreement would live at the boundary.
    #[test]
    fn the_inverse_answers_exactly_where_the_band_test_does() {
        let range = crate::level_plan::DIST_PACK_RANGE;
        for step in 0..=20_000 {
            let depth = f64::from(step) / 20_000.0;
            assert_eq!(
                source_distance(depth, range).is_some(),
                is_acoustic_image(depth),
                "the two doors disagreed at a window depth of {depth}"
            );
        }
        // and the sweep is not vacuous at either verdict
        assert!(is_acoustic_image(1.0));
        assert!(!is_acoustic_image(0.5));
    }

    /// THE BREAK: reading a WORLD fragment's depth through the band inverse.
    /// Hand-derived from the shipped camera: a wall three metres from the
    /// eye writes a window depth of 0.0158, and running that through the
    /// band arithmetic gives (0.999999 - 0.0158) / 1e-3 * 40 = 39,337 m,
    /// which clamps to exactly the packing range. Every world pixel in the
    /// game would report a source sitting at 40 m, and the witness would
    /// then reject the colour channel's correct reading in favour of it.
    ///
    /// Absence is the answer, not a clamped number.
    #[test]
    fn a_world_fragment_is_refused_rather_than_read_as_a_distant_source() {
        let range = crate::level_plan::DIST_PACK_RANGE;
        for &dist in &[0.051, 0.5, 3.0, 12.0, 40.0, 59.9] {
            let depth = window_depth(dist, CAM_NEAR, CAM_FAR);
            assert_eq!(
                source_distance(depth, range),
                None,
                "a world surface {dist} m from the eye read as an acoustic image"
            );
        }
        // the number it would have claimed, so the refusal is not mistaken
        // for a technicality
        let wall = window_depth(3.0, CAM_NEAR, CAM_FAR);
        assert!((wall - 0.015_85).abs() < 1.0e-4, "the wall reads {wall}");
        assert!(((ALWAYS_ON_TOP - wall) / SOURCE_BAND * range).clamp(0.0, range) == range);
    }

    /// THE BREAK: the inverse drifting from `source_depth`, so the witness
    /// corroborates the colour channel with a number that is itself wrong.
    ///
    /// Two halves, and the second is the one that matters. The exact
    /// algebra round-trips to nothing, which only proves the arithmetic was
    /// inverted correctly. What the WITNESS rests on is that the round trip
    /// survives the 24-bit depth buffer the reading actually comes out of —
    /// so the depth is quantised to 2^-24 before it is read back, and the
    /// recovered distance must land inside the band's own resolution,
    /// 2.4 mm at the shipped band and range.
    #[test]
    fn a_sources_distance_survives_the_round_trip_through_a_real_depth_buffer() {
        let range = crate::level_plan::DIST_PACK_RANGE;
        let resolution = band_resolution(SOURCE_BAND, range);
        for step in 0..=400 {
            let dist = f64::from(step) * range / 400.0;
            let exact = source_depth(dist, range);
            let back = source_distance(exact, range).expect("a source depth is in the band");
            assert!(
                (back - dist).abs() < 1.0e-9,
                "{dist} m came back as {back} m exactly"
            );

            // ...and through the buffer the driver actually writes
            let quantised = (exact * DEPTH_CODES).round() / DEPTH_CODES;
            let witnessed =
                source_distance(quantised, range).expect("a quantised source is still in the band");
            assert!(
                (witnessed - dist).abs() <= resolution,
                "{dist} m came back as {witnessed} m through a 24-bit buffer, outside the \
                 {resolution} m the band resolves"
            );
        }
    }

    /// THE BREAK: judging the band in f64 when the shader judges it in f32.
    ///
    /// A source at or beyond the packing range writes exactly the band's
    /// floor. Stored into the 24-bit depth attachment — GL specifies
    /// round-to-nearest for that conversion — and read back, the value comes
    /// out 6e-11 BELOW the floor in exact arithmetic. Compared in f64 that
    /// is a rejection: the pass would call its own farthest acoustic images
    /// walls and fall through to the wall-table inference for exactly the
    /// sources the depth read was added to cover.
    ///
    /// On the GPU it is nothing. f32 near 1.0 has a 2^-24 ULP, five hundred
    /// times that residue, so the read-back narrows to the threshold's own
    /// bit pattern and the comparison passes. The shortfall cannot reach the
    /// comparison that ships.
    ///
    /// Which is why the fix is to compare in f32 and NOT to slacken the
    /// threshold. A slackened floor would widen the band for a shortfall no
    /// shader can see — and half a depth code of slack, the size a rounding
    /// argument asks for, is half an f32 ULP: it lands midway between two
    /// representable values and rounds straight back to the unslackened
    /// threshold, so it would read as a fix and be a no-op.
    #[test]
    fn the_bands_floor_survives_the_buffer_that_stores_it() {
        let range = crate::level_plan::DIST_PACK_RANGE;
        let floor = source_depth(range, range);
        assert_eq!(floor, ALWAYS_ON_TOP - SOURCE_BAND);
        let stored = (floor * (DEPTH_CODES - 1.0)).round() / (DEPTH_CODES - 1.0);

        // in exact arithmetic the reading falls short...
        assert!(stored < floor, "the buffer happened to round up: {stored}");
        // ...by far less than the width the comparison is made in
        let ulp = f64::from((floor as f32).next_up()) - f64::from(floor as f32);
        assert!(
            floor - stored < ulp / 100.0,
            "the shortfall {} is no longer negligible against an f32 ULP of {ulp}",
            floor - stored
        );

        // so every source out there still reads as an acoustic image
        for &dist in &[range, range * 1.5, 1.0e6] {
            let written = source_depth(dist, range);
            let read = (written * (DEPTH_CODES - 1.0)).round() / (DEPTH_CODES - 1.0);
            assert!(
                is_acoustic_image(read),
                "a source {dist} m from the eye read back as a wall"
            );
        }
        // and the f64 comparison, which is the one this test exists to
        // refuse, would have rejected it
        assert!(stored < ALWAYS_ON_TOP - SOURCE_BAND);

        // a reading a whole code below the floor is still not an image: the
        // narrowing is a change of WIDTH, not a slackening
        assert!(!is_acoustic_image(f64::from((floor as f32).next_down())));
    }

    /// The limit `source_depth`'s own clamp creates, stated here too rather
    /// than left for a caller to discover: past the packing range the band
    /// saturates, so every source out there writes the floor and the inverse
    /// can only answer "at least the range". Two sources at 50 and 60 m are
    /// indistinguishable to the witness, exactly as they are to the depth
    /// sort — which is why the witness must be a corroboration and never a
    /// replacement.
    #[test]
    fn the_inverse_saturates_where_the_band_does_instead_of_inventing_a_distance() {
        let range = crate::level_plan::DIST_PACK_RANGE;
        assert_eq!(
            source_distance(source_depth(50.0, range), range),
            Some(range)
        );
        assert_eq!(
            source_distance(source_depth(60.0, range), range),
            Some(range)
        );
        // and nothing ever comes back negative, however far above the band's
        // top the reading sits: a real always-on-top fragment measures 1.0,
        // which is above ALWAYS_ON_TOP itself
        assert_eq!(source_distance(1.0, range), Some(0.0));
    }

    /// Total on the degenerate readings the signature admits. A depth
    /// outside `[0, 1]` is not a window depth at all, and a NaN read out of
    /// a dead texture must be absence rather than a distance — the witness
    /// then degrades to exactly the colour channel, which is the property
    /// the whole design rests on.
    #[test]
    fn a_reading_that_is_not_a_window_depth_is_absence() {
        let range = crate::level_plan::DIST_PACK_RANGE;
        assert_eq!(source_distance(f64::NAN, range), None);
        assert_eq!(source_distance(f64::INFINITY, range), None);
        assert_eq!(source_distance(-0.1, range), None);
        assert_eq!(source_distance(1.1, range), None);
        assert_eq!(source_distance(1.0, 0.0), None);
        assert_eq!(source_distance(1.0, f64::NAN), None);
        assert!(!is_acoustic_image(f64::NAN));
        assert!(!is_acoustic_image(1.1));
        assert!(!is_acoustic_image(-0.1));
    }

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
