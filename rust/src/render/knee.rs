//! The one `smoothstep` contract every rendered knee obeys.
//!
//! Three passes in this renderer fade over a knee — the crease knee over a
//! LABEL DIFFERENCE, the detail knee over a REVEAL, the silhouette knee over
//! METRES of distance Laplacian — and each of them needs the same two things
//! to be true of its pair before it reaches the GPU, for reasons that have
//! nothing to do with what the pair measures.
//!
//! GLSL's `smoothstep(lo, hi, x)` is `clamp((x - lo) / (hi - lo), 0, 1)`
//! smoothed. It is undefined when `lo == hi` and inverted when `lo > hi`, so
//! a knee is not any two numbers. And the ordering has to be judged in f32,
//! because f32 is what reaches the hardware: two f64s a billionth apart are
//! strictly ordered right up until they land in the same lane.
//!
//! That argument was written out three times, once per knee, and
//! [`super::detail::DetailKnee`]'s own doc gave up and pointed at
//! [`super::crease::CreaseKnee`] for it. This is the thing it was pointing
//! at. The three keep their own types because they carry incompatible
//! UNITS — a knee in metres pushed into `u_crease_knee` would fade label
//! differences over a distance — and each keeps its own derivation and its
//! own theorem. Only the contract is shared.

/// A `smoothstep` pair GLSL can evaluate: finite as f32 and strictly
/// ordered in the width the GPU will use.
///
/// Carries no units. Wrap it in a type that does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Knee {
    lo: f32,
    hi: f32,
}

impl Knee {
    /// A knee, or `None` if the pair cannot fade.
    ///
    /// Total over every f64 pair. Narrowing happens BEFORE the ordering
    /// test, which is the whole subtlety: an f64 check would pass a pair
    /// that divides by zero on the hardware, and it would also pass
    /// `1.0e300`, which is finite as f64 and infinite as f32.
    #[must_use]
    pub const fn new(lo: f64, hi: f64) -> Option<Self> {
        let (lo, hi) = (lo as f32, hi as f32);
        if lo.is_finite() && hi.is_finite() && lo < hi {
            Some(Self { lo, hi })
        } else {
            None
        }
    }

    /// Where the fade begins, in the width the GPU will use.
    #[must_use]
    pub fn lo(self) -> f64 {
        f64::from(self.lo)
    }

    /// Where the fade reaches full strength, in the width the GPU will use.
    #[must_use]
    pub fn hi(self) -> f64 {
        f64::from(self.hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE BREAK: a pair GLSL cannot evaluate reaching the GPU. `smoothstep`
    /// is `clamp((x - lo) / (hi - lo), 0, 1)` smoothed, so an equal pair is a
    /// division by zero and an inverted pair fades backwards — a seam that
    /// should be bright goes dark, an outline that should appear vanishes.
    /// Refused at construction, so no caller has to guard at every use.
    #[test]
    fn a_pair_glsl_cannot_evaluate_is_not_a_knee() {
        assert_eq!(
            Knee::new(0.08, 0.08),
            None,
            "a zero-width knee divides by zero"
        );
        assert_eq!(
            Knee::new(0.08, 0.04),
            None,
            "an inverted knee fades backwards"
        );
        assert_eq!(Knee::new(f64::NAN, 1.0), None);
        assert_eq!(Knee::new(0.0, f64::NAN), None);
        assert_eq!(Knee::new(0.0, f64::INFINITY), None);
        assert_eq!(Knee::new(f64::NEG_INFINITY, 1.0), None);
    }

    /// THE BREAK: judging the ordering in f64 when f32 is what reaches the
    /// GPU. Two f64s a billionth apart are strictly ordered and land in the
    /// SAME f32 lane, so a knee that passes an f64 check divides by zero on
    /// the hardware. The narrowing must happen before the comparison, not
    /// after it.
    ///
    /// Hand-derived: f32's ULP at 0.08 is about 6e-9, so 1e-9 collapses and
    /// 1e-3 does not.
    #[test]
    fn ordering_is_judged_in_the_width_the_gpu_will_use() {
        assert_eq!(Knee::new(0.08, 0.08 + 1.0e-9), None);
        assert!(Knee::new(0.08, 0.08 + 1.0e-3).is_some());
        // and the value a caller reads back is the narrowed one, because
        // that is the value the shader will actually fade over
        let knee = Knee::new(0.3, 0.6).expect("an ordered pair is a knee");
        assert_eq!(knee.lo(), f64::from(0.3_f32));
        assert_eq!(knee.hi(), f64::from(0.6_f32));
    }

    /// THE BREAK: a finite f64 pair that OVERFLOWS f32 being accepted. f64
    /// 1e300 narrows to f32 infinity, and an infinite knee makes
    /// `(x - lo) / (hi - lo)` a NaN for every x — every fade in the game at
    /// that knee turns into undefined output.
    #[test]
    fn a_pair_that_overflows_f32_is_refused_though_it_is_finite_as_f64() {
        assert_eq!(Knee::new(0.0, 1.0e300), None);
        assert_eq!(Knee::new(-1.0e300, 1.0), None);
    }
}
