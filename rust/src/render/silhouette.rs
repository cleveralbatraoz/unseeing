//! SHAPE: the outline a break in the world's distance draws, and the one
//! authored number that decides how big a break has to be.
//!
//! # Why this is a module and not two literals in GLSL
//!
//! `hearing_post` drew silhouettes with `smoothstep(0.012, 0.03, lap)`,
//! where `lap` is the five-tap Laplacian of the packed B channel. Both
//! literals are in CHANNEL units, which means both of them silently mean a
//! different depth step whenever the packing range moves — and
//! [`crate::level_plan::pack_range_budget`] exists precisely to tell a
//! designer to move it when the map outgrows it. Following that instruction
//! from 40 m to 60 m would have taken a 0.8 m step from drawing at 42% to
//! drawing at 1.6%, and nothing anywhere would have said so.
//!
//! So the knee is stated in METRES of depth step, and the shader scales its
//! Laplacian into metres before fading over it. [`crate::render::channel`]
//! owns that scale, and it has to: the five-tap coefficients sum to zero, so
//! the packed floor cancels exactly and only the gain survives into the
//! outline.
//!
//! # It is AUTHORED, and says so
//!
//! There is no acoustic law behind 0.48 m. Nothing in this engine derives
//! how large a discontinuity must be before a blind hero's ear resolves it;
//! there is no frequency axis, no energy, and occlusion is a `{0,1}` gate.
//! [`crate::render::detail`] refuses the same temptation in the same words
//! and for the same reason.
//!
//! What the numbers ARE is the shipped look, restated in units that do not
//! move: `0.012 × 40` and `0.03 × 40`, the retired literals evaluated at the
//! range they were tuned against. The migration that gave the channel back
//! its bottom 28 codes must not also retune the outline, or two changes land
//! in one picture and neither can be judged.
//!
//! # What the number MEANS
//!
//! `lap` is `|left + right + up + down - 4 × centre|` of camera distance, so
//! for a straight silhouette edge it is very nearly the depth step across
//! that edge, and for a corner about twice it. A wall seen face-on has a
//! Laplacian of zero; a floor seen at a grazing angle has a small one, which
//! is why the fade starts at half a metre and not at a centimetre.

use super::channel;
use super::knee::Knee;

/// Metres of depth step below which no silhouette draws at all.
///
/// AUTHORED. `0.012 × DIST_PACK_RANGE` at the range the retired GLSL
/// literal was tuned against — the shipped look, in units that survive a
/// change of packing range.
pub const SIL_LO_M: f64 = 0.48;

/// Metres of depth step at which a silhouette draws at full strength.
///
/// AUTHORED, the same way [`SIL_LO_M`] is: `0.03 × DIST_PACK_RANGE`.
pub const SIL_HI_M: f64 = 1.2;

/// The knee the silhouette fades over, in METRES of depth step.
///
/// A distinct type from [`super::crease::CreaseKnee`] and
/// [`super::detail::DetailKnee`] though all three wrap the same
/// [`Knee`]: this one carries metres, and pushing metres where a label
/// difference belongs would fade every seam in the game over a distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SilhouetteKnee(Knee);

impl SilhouetteKnee {
    /// A silhouette knee, if these two depth steps are one.
    ///
    /// Total over every f64 pair, by [`Knee::new`]'s contract.
    #[must_use]
    pub const fn new(lo: f64, hi: f64) -> Option<Self> {
        match Knee::new(lo, hi) {
            Some(knee) => Some(Self(knee)),
            None => None,
        }
    }

    /// The knee the game draws with.
    ///
    /// A `const` item, on the reasoning [`super::crease::CreaseKnee::SHIPPED`]
    /// states: the pair is discharged by the COMPILER, so an authored knee
    /// GLSL cannot fade stops the build naming this line rather than
    /// dividing by zero on a player's hardware, and no unreachable arm is
    /// left holding a second copy of today's answer.
    ///
    /// The `panic!` never runs.
    pub const SHIPPED: Self = match Self::new(SIL_LO_M, SIL_HI_M) {
        Some(knee) => knee,
        None => panic!("the authored silhouette knee is not one GLSL can fade"),
    };

    /// The knee the game draws with — see [`Self::SHIPPED`].
    #[must_use]
    pub const fn shipped() -> Self {
        Self::SHIPPED
    }

    /// Metres of depth step where the outline begins to appear.
    #[must_use]
    pub fn lo(self) -> f64 {
        self.0.lo()
    }

    /// Metres of depth step where the outline reads at full strength.
    #[must_use]
    pub fn hi(self) -> f64 {
        self.0.hi()
    }
}

/// How brightly a break in the world draws, given the five-tap Laplacian of
/// the PACKED channel and the range that channel was packed against.
///
/// The cargo-pinned reference for `hearing_post`'s `sil`. The knee arrives
/// as an argument rather than being read from [`SilhouetteKnee::SHIPPED`]
/// inside, so the law can be exercised at knees the game does not ship
/// without constructing the game.
///
/// `lap_packed` is deliberately the RAW, SIGNED channel Laplacian and not a
/// metric one: that is the value the shader holds, and converting it here is
/// the one place the packing's gain enters the outline. Passing an already
/// metric Laplacian would apply the gain twice. The sign is discarded here
/// rather than by the caller, because discarding it IS part of the law — a
/// surface stepping toward the eye and one stepping away are the same break
/// in the world.
///
/// Total on any input: a non-finite Laplacian, or a range that is not a
/// positive finite length, draws nothing — [`channel::unpack_scale`] answers
/// 0.0 for the bad ranges and [`Knee::fade`] answers 0.0 for the bad
/// readings. The outline is white on black, so absence must draw nothing
/// rather than stamp a white pixel over a blind hero's world.
#[must_use]
pub fn outline(knee: SilhouetteKnee, lap_packed: f64, range: f64) -> f64 {
    knee.0.fade(lap_packed.abs() * channel::unpack_scale(range))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_plan::DIST_PACK_RANGE;
    use crate::render::channel;

    /// THE BREAK: a channel repair that quietly retunes the look as well,
    /// so that when the picture changes nobody can tell which of the two
    /// changes did it.
    ///
    /// Hand-derived ONCE, here in prose, and then written as the metres it
    /// came out as: `hearing_post` carried `smoothstep(0.012, 0.03, lap)`
    /// over a channel packed as `vd / 40`, so the knee it drew was 0.012 x
    /// 40 = 0.48 m and 0.03 x 40 = 1.2 m of depth step.
    ///
    /// The assertion compares against 0.48 and 1.2 and NOT against
    /// `0.012 * DIST_PACK_RANGE`, which is the whole point of the module
    /// and was got wrong here first: computing the expectation from the
    /// packing range restores the exact coupling this type exists to
    /// remove, so raising the range to 60 would fail this test and demand
    /// the authored knee follow it to 0.72 m. It must not. These metres are
    /// a look, and a look does not move because a map grew.
    #[test]
    fn the_channel_repair_does_not_retune_the_outline() {
        let knee = SilhouetteKnee::shipped();
        assert!((knee.lo() - 0.48).abs() < 1.0e-6, "lo is {}", knee.lo());
        assert!((knee.hi() - 1.2).abs() < 1.0e-6, "hi is {}", knee.hi());
    }

    /// THE BREAK, and it is a live one rather than a hypothetical:
    /// `level_plan::pack_range_budget` actively tells a designer to raise
    /// DIST_PACK_RANGE when the map outgrows it, and while the knee lived in
    /// CHANNEL units that instruction silently raised the depth step an
    /// outline needs before it draws at all.
    ///
    /// Hand-derived counter-example. A 0.8 m step draws at 41.7% under the
    /// metric law at every range. Under the retired channel-unit knee it
    /// drew at 41.7% at a range of 40 and at 1.6% at a range of 60 — a 27×
    /// change in the look, from a constant a designer was told to move for
    /// an unrelated reason.
    #[test]
    fn the_outline_stops_moving_when_the_packing_range_does() {
        let knee = SilhouetteKnee::shipped();
        let step = 0.8;
        for &range in &[DIST_PACK_RANGE, 60.0, 25.0] {
            let lap = step / channel::unpack_scale(range);
            let drawn = outline(knee, lap, range);
            assert!(
                (drawn - 0.417_012).abs() < 1.0e-5,
                "a {step} m step drew at {drawn} with a packing range of {range} m"
            );
        }
        // the retired law, evaluated as it stood, so the counter-example is
        // arithmetic rather than assertion
        let retired = Knee::new(0.012, 0.03).expect("the literals the shader carried");
        assert!((retired.fade(step / 40.0) - 0.417_012).abs() < 1.0e-5);
        assert!((retired.fade(step / 60.0) - 0.015_647).abs() < 1.0e-5);
    }

    /// THE BREAK: the knee collapsed or inverted, so every surface in the
    /// game outlines or none does. The authored perception, stated at both
    /// ends and monotone between them: a flat wall draws nothing, a room
    /// corner draws fully.
    #[test]
    fn a_flat_surface_draws_nothing_and_a_room_corner_draws_fully() {
        let knee = SilhouetteKnee::shipped();
        let range = DIST_PACK_RANGE;
        let at = |step: f64| outline(knee, step / channel::unpack_scale(range), range);
        assert_eq!(at(0.0), 0.0, "a flat surface drew an outline");
        assert_eq!(at(0.4), 0.0, "a step below the knee drew an outline");
        // at the authored hi itself, within a rounding of full: the knee is
        // NARROWED to f32 on the way in, so 1.2 m lands a hair under a hi of
        // f32(1.2) = 1.20000005, and the round trip through the packing gain
        // costs another few ULP. That is the type's promise showing, not a
        // fade that stops short.
        assert!(
            at(1.2) > 0.999_999,
            "at the knee's hi the outline read {}",
            at(1.2)
        );
        assert_eq!(at(3.0), 1.0, "a room corner did not draw fully");
        let mut previous = -1.0;
        for step in 0..=120 {
            let drawn = at(f64::from(step) * 0.025);
            assert!(drawn >= previous, "the outline dimmed as the step grew");
            assert!((0.0..=1.0).contains(&drawn));
            previous = drawn;
        }
    }

    /// Total on the degenerate inputs the signature admits. A NaN out of the
    /// screen texture, or a range a level never pushed, must draw NOTHING —
    /// the outline is white on black, so failing bright would stamp a solid
    /// white pixel over a blind hero's world.
    #[test]
    fn a_degenerate_reading_draws_nothing_rather_than_a_white_pixel() {
        let knee = SilhouetteKnee::shipped();
        assert_eq!(outline(knee, f64::NAN, DIST_PACK_RANGE), 0.0);
        assert_eq!(outline(knee, f64::INFINITY, DIST_PACK_RANGE), 0.0);
        assert_eq!(outline(knee, 0.5, 0.0), 0.0);
        assert_eq!(outline(knee, 0.5, f64::NAN), 0.0);
    }

    /// THE BREAK: the sign of the Laplacian reaching the fade. The shader
    /// computes `abs(...)` before its smoothstep, and it must — a surface
    /// that steps TOWARD the eye and one that steps away are the same break
    /// in the world and draw the same outline. Drop the abs and every
    /// silhouette in the game is drawn on one side of its edge only.
    #[test]
    fn a_break_draws_the_same_whichever_way_it_steps() {
        let knee = SilhouetteKnee::shipped();
        for &lap in &[0.004, 0.009, 0.011, 0.02, 0.5] {
            assert_eq!(
                outline(knee, lap, DIST_PACK_RANGE),
                outline(knee, -lap, DIST_PACK_RANGE),
                "a Laplacian of {lap} drew differently from its negative"
            );
        }
        // and it is not vacuous: the middle of that list is inside the fade
        let inside = outline(knee, 0.009, DIST_PACK_RANGE);
        assert!(
            inside > 0.0 && inside < 1.0,
            "nothing in the sweep was mid-fade"
        );
    }

    /// THE BREAK: a knee GLSL cannot fade reaching the GPU. Refused at
    /// construction by the shared contract, and the shipped one is
    /// discharged by the compiler, so an unfadeable authored pair stops the
    /// build rather than dividing by zero on a player's hardware.
    #[test]
    fn an_unfadeable_authored_pair_is_unrepresentable() {
        assert_eq!(SilhouetteKnee::new(0.48, 0.48), None);
        assert_eq!(SilhouetteKnee::new(1.2, 0.48), None);
        assert_eq!(SilhouetteKnee::new(f64::NAN, 1.2), None);
    }
}
