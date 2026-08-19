//! The rendered response to a label difference — `hearing_post`'s crease
//! knee, derived from [`labels::MIN_SEP`] rather than retyped in GLSL.
//!
//! # Two numbers that had to agree and never met
//!
//! [`labels::MIN_SEP`] governs ALLOCATION: the colouring keeps any two
//! labels that must draw a seam at least that far apart.
//! `smoothstep(0.04, 0.08, nrm)` in `hearing_post.gdshader` governs the
//! rendered RESPONSE: how bright the seam between two labels actually is.
//! They are the same law seen from two ends, and nothing compared them —
//! `labels.rs`'s own doc comment said as much and shipped anyway, calling
//! the shader literal "the actual authority".
//!
//! The sharp direction is Rust-side. `MIN_SEP` is exactly the knob a
//! maintainer reaches for when the label band starves, and dropping it to
//! 0.05 keeps every `separated()` test in the crate green while rendering
//! those seams at a fraction of full strength — the allocator would hand
//! out separations the renderer no longer draws.
//!
//! By Law 2 the knee is a rendering decision and belongs in Rust; the shader
//! reads it and does not own it. This module is a peer of [`super::depth`],
//! which is the existing precedent for a module owning a shader-facing
//! derived quantity together with its derivation.
//!
//! # Why a validated type rather than a pair of floats
//!
//! GLSL's `smoothstep(lo, hi, x)` divides by `hi - lo`, so a knee is not any
//! two numbers. [`super::knee::Knee`] owns that contract and the reasoning
//! behind it; [`CreaseKnee`] is what gives it UNITS, so a knee in metres
//! cannot be pushed where a knee in label differences belongs.

use super::knee::Knee;
use super::labels;

/// Where the knee opens, as a fraction of where it closes: a seam at half
/// the required separation draws at half strength.
///
/// `0.5` reproduces the shipped `smoothstep(0.04, 0.08, …)` exactly from
/// `MIN_SEP = 0.08`, so moving the knee into Rust changes no pixel — which
/// is the point. A change of behaviour and a change of ownership in one
/// commit is two things to debug at once.
pub const LOW_KNEE_RATIO: f64 = 0.5;

/// The `smoothstep` knee the hearing pass fades a crease over: full
/// strength at `hi`, gone below `lo`, both in units of a LABEL DIFFERENCE.
///
/// Stored narrowed to f32 by the [`Knee`] inside it, because f32 is what
/// reaches the GPU — the same reason [`labels::separated`] compares narrowed
/// lanes rather than the f64 sources it was handed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreaseKnee(Knee);

impl CreaseKnee {
    /// A crease knee, if these two label differences are one.
    ///
    /// Total over every f64 pair, by [`Knee::new`]'s contract.
    #[must_use]
    pub const fn new(lo: f64, hi: f64) -> Option<Self> {
        // `Option::map` is not const, and this must be, so that SHIPPED is
        // discharged by the compiler
        match Knee::new(lo, hi) {
            Some(knee) => Some(Self(knee)),
            None => None,
        }
    }

    /// The shipped derivation: a seam draws at full strength once two
    /// labels are `min_sep` apart, and fades to nothing at
    /// [`LOW_KNEE_RATIO`] of that.
    #[must_use]
    pub const fn from_min_sep(min_sep: f64) -> Option<Self> {
        Self::new(min_sep * LOW_KNEE_RATIO, min_sep)
    }

    /// The knee the game renders with, derived from the one `MIN_SEP` the
    /// colouring allocates against.
    ///
    /// A `const` item rather than a function with a fallback. The derivation
    /// is discharged by the COMPILER, so a `MIN_SEP` that cannot fade is a
    /// build failure naming this line — and there is no unreachable arm left
    /// holding a second, hand-copied definition of today's answer. That arm
    /// used to read `unwrap_or(Self { lo: 0.04, hi: 0.08 })`, which no input
    /// could reach and no mutation could kill: replacing it with nonsense
    /// left all 540 tests green, which is this repository's own definition of
    /// code that should not exist.
    ///
    /// The `panic!` never runs. `const` initialisers are evaluated at compile
    /// time, so an unsatisfiable derivation stops the build instead of
    /// reaching a player.
    pub const SHIPPED: Self = match Self::from_min_sep(labels::MIN_SEP) {
        Some(knee) => knee,
        None => panic!("MIN_SEP does not derive a crease knee GLSL can fade"),
    };

    /// The knee the game renders with — see [`Self::SHIPPED`].
    #[must_use]
    pub const fn shipped() -> Self {
        Self::SHIPPED
    }

    /// Where the fade begins, as a label difference.
    #[must_use]
    pub fn lo(self) -> f64 {
        self.0.lo()
    }

    /// Where the seam reaches full strength — the separation
    /// [`labels::MIN_SEP`] demands.
    #[must_use]
    pub fn hi(self) -> f64 {
        self.0.hi()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE break this catches: the allocator and the renderer disagreeing
    /// about what "far enough apart to draw a seam" means.
    ///
    /// `MIN_SEP` is the knob a maintainer reaches for when the label band
    /// starves. Lower it and every `separated()` test in the crate stays
    /// green — the colouring simply packs labels tighter — while the shader
    /// goes on fading over a knee it no longer matches, drawing the seams
    /// the allocator just approved at reduced strength or not at all.
    /// Hand-derived: at `MIN_SEP = 0.05` the knee must follow to
    /// (0.025, 0.05), not stay at (0.04, 0.08) where a 0.05 difference
    /// lands at smoothstep's 25%.
    #[test]
    fn the_rendered_knee_follows_the_separation_it_is_the_response_to() {
        // compared in the width the GPU will hold, which is the whole
        // reason the knee is stored narrowed
        let shipped = CreaseKnee::shipped();
        assert_eq!(shipped.hi(), f64::from(labels::MIN_SEP as f32));
        assert_eq!(
            shipped.lo(),
            f64::from((labels::MIN_SEP * LOW_KNEE_RATIO) as f32)
        );

        // The shipped knee IS the derivation applied to the constant.
        //
        // Stated plainly, because no test can do better and pretending
        // otherwise would be the more dangerous error: `from_min_sep(0.08)`
        // returns exactly `(0.04, 0.08)`, so a `shipped()` that hardcoded
        // that literal is INDISTINGUISHABLE from the derivation today. That
        // is not a hole in the test, it is the "no pixel moves" property
        // this commit was built on. The guard fires the moment MIN_SEP
        // moves — at which point a hardcoded knee fails the first
        // assertion above — and not one commit before.
        assert_eq!(
            shipped,
            CreaseKnee::from_min_sep(labels::MIN_SEP).expect("MIN_SEP is a positive separation")
        );

        // the derivation tracks its input across the range, at separations
        // the shipped constant does not sit on
        for (sep, lo, hi) in [(0.05, 0.025, 0.05), (0.12, 0.06, 0.12), (0.2, 0.1, 0.2)] {
            let knee = CreaseKnee::from_min_sep(sep).expect("a positive separation is a knee");
            assert!(
                (knee.hi() - hi).abs() < 1.0e-7,
                "hi at {sep}: {}",
                knee.hi()
            );
            assert!(
                (knee.lo() - lo).abs() < 1.0e-7,
                "lo at {sep}: {}",
                knee.lo()
            );
            assert_ne!(
                knee, shipped,
                "the knee did not follow its separation at {sep}"
            );
        }
    }

    /// Moving the knee into Rust must change NO pixel: the shipped
    /// derivation reproduces the literal pair the GLSL carried, exactly.
    /// A change of behaviour and a change of ownership in one commit is two
    /// things to debug at once.
    #[test]
    fn the_shipped_knee_is_the_literal_the_shader_used_to_carry() {
        let knee = CreaseKnee::shipped();
        assert_eq!(knee.lo(), f64::from(0.04_f32));
        assert_eq!(knee.hi(), f64::from(0.08_f32));
    }

    /// A knee GLSL cannot evaluate is not a knee. `smoothstep(lo, hi, x)`
    /// divides by `hi - lo`, so an equal pair is a division by zero and an
    /// inverted pair fades the wrong way — a seam that should be bright
    /// going dark. Refused at construction rather than guarded at every
    /// use.
    #[test]
    fn a_knee_glsl_cannot_evaluate_is_refused() {
        assert_eq!(
            CreaseKnee::new(0.08, 0.08),
            None,
            "a zero-width knee divides by zero"
        );
        assert_eq!(
            CreaseKnee::new(0.08, 0.04),
            None,
            "an inverted knee fades backwards"
        );
        assert_eq!(CreaseKnee::new(f64::NAN, 0.08), None);
        assert_eq!(CreaseKnee::new(0.04, f64::INFINITY), None);
        assert_eq!(CreaseKnee::new(0.04, f64::NAN), None);
        assert_eq!(
            CreaseKnee::from_min_sep(0.0),
            None,
            "no separation, no seam"
        );
        assert_eq!(CreaseKnee::from_min_sep(-0.08), None);
        assert_eq!(CreaseKnee::from_min_sep(f64::NAN), None);
    }

    /// The ordering is checked AFTER narrowing, because f32 is what reaches
    /// the GPU. Two f64s a billionth apart are strictly ordered in f64 and
    /// land in the same f32 lane — a knee that passes an f64 check and then
    /// divides by zero on the hardware.
    #[test]
    fn ordering_is_judged_in_the_width_the_gpu_will_use() {
        assert_eq!(CreaseKnee::new(0.08, 0.08 + 1.0e-9), None);
        // ...while a difference f32 can hold is still a knee
        assert!(CreaseKnee::new(0.08, 0.08 + 1.0e-3).is_some());
    }
}
