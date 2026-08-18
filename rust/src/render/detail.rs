//! The second knee: how much a swept surface tells you, as distinct from
//! whether it is there at all.
//!
//! # One max() threw away the whole model
//!
//! `hearing_post` has always drawn with two independent mechanisms:
//!
//! - `lap`, a Laplacian of the packed camera distance — **SHAPE**. Where a
//!   surface stops. It needs no labels and survives any amount of dimming
//!   that leaves the distance readable.
//! - `nrm`, a difference of per-vertex labels — **DETAIL**. The fan's
//!   blades against its guard, the radio's dial against its case, a crate's
//!   pierce line. It is identity, not presence.
//!
//! and then fused them, `edge = max(sil, crease)`, and multiplied by one
//! shared reveal. Two physically distinct facts, one law, and the
//! distinction gone.
//!
//! Recovering it is the whole perception model. SHAPE carries to a wave's
//! full reach and lasts its full tail; DETAIL resolves near the hand and
//! dies in about a second and a half. That is the one thing about
//! non-visual sensing that survived this campaign's adversarial review
//! intact: expert echolocators detect an object's PRESENCE across a room
//! while its SHAPE needs near-contact (Kolarik et al., *Hearing Research*
//! 310 (2014) 60-68; Milne, Goodale & Thaler, *Atten Percept Psychophys*
//! 76(6) (2014) 1828-1837).
//!
//! That is where the borrowing stops. An earlier draft of this paragraph
//! also quoted the aperture limit `λ/D` — "about 0.7 m of resolvable feature
//! at 3 m with a human head" — and it had to go. `λ` is a wavelength, and
//! there is no frequency axis anywhere in this codebase; the figure is in
//! metres, while the knee below is a threshold on `reveal` in `[0, 1]` and
//! has no length in it, so the number could not enter the derivation in any
//! direction even if it were sound. A cited finding may ORDER two effects.
//! It may not supply a number. [`crate::level_plan::prop_through`] refuses
//! the same temptation in the same words, and two modules in one crate
//! cannot answer it differently.
//!
//! # The knee is derived, and it buys a theorem
//!
//! [`DetailKnee::shipped`] is `(SOURCE_THROUGH, SOURCE_THROUGH /
//! LOW_KNEE_RATIO)` = `(0.30, 0.60)`. Not chosen — taken from the wall
//! factor that already ships, and reusing [`crease::LOW_KNEE_RATIO`]
//! rather than declaring a second ratio nobody would keep in step.
//!
//! [`reveal::source_image`] is `muffle · max(wave, volume)` with both
//! factors at most 1. So through one wall a source's reveal cannot exceed
//! `SOURCE_THROUGH` — which is exactly where this knee's fade *begins*:
//!
//! > A source behind a wall can never draw a crease. For any wave, for any
//! > volume, by construction rather than by tuning.
//!
//! You always know a source is sounding in there. Past the first wall you
//! stop knowing it is a radio. `the_first_wall_takes_a_sources_identity`
//! is that theorem, and it breaks the moment someone raises
//! `SOURCE_THROUGH` or lowers this knee without moving the other.
//!
//! # What this deliberately is NOT
//!
//! It is **not** the refuted claim that "occlusion removes detail rather
//! than brightness". Fifteen independent reviewers refuted that 3/3: a
//! partition applies a large broadband cut *first* (42–52 dB through this
//! game's own 0.30 m walls) and the ~6 dB/octave spectral tilt rides on top
//! of it, an order of magnitude smaller in JND terms. The ordering here is
//! the corrected one — brightness falls first, via the muffle that already
//! ships, and detail falls as a **consequence**, because this knee is
//! gated on reveal. Detail loss is never bought by keeping brightness up.
//!
//! Folding this gate into [`crease::CreaseKnee`] is the obvious wrong
//! shortcut: that knee responds to a label DIFFERENCE, so making occlusion
//! a function of it would make *which* creases die depend on the graph
//! colouring's incidental rung assignment — the sideways coupling AGENTS.md
//! forbids. Two knees, two inputs, one composition in the shader.

use super::crease::LOW_KNEE_RATIO;
use crate::level_plan::SOURCE_THROUGH;

/// Where a swept surface stops telling you *what* it is, while still
/// telling you *that* it is there.
///
/// A validated type for the same reason [`crease::CreaseKnee`] is one:
/// GLSL's `smoothstep(lo, hi, x)` divides by `hi - lo`, so an equal pair is
/// a division by zero and an inverted pair fades the wrong way. The bad
/// ones are unrepresentable rather than commented against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetailKnee {
    lo: f32,
    hi: f32,
}

impl DetailKnee {
    /// A knee, or `None` if the pair cannot fade.
    ///
    /// Narrowing to f32 happens BEFORE the ordering test, exactly as in
    /// [`crease::CreaseKnee::new`]: two f64s a nanometre apart are strictly
    /// ordered right up until they land in the same f32 lane, and the GPU
    /// only ever sees the f32.
    #[must_use]
    pub const fn new(lo: f64, hi: f64) -> Option<Self> {
        let (lo, hi) = (lo as f32, hi as f32);
        if lo.is_finite() && hi.is_finite() && lo < hi {
            Some(Self { lo, hi })
        } else {
            None
        }
    }

    /// The derivation: detail begins to fade exactly at the reveal a source
    /// one wall away is capped to, and reaches full strength at
    /// [`LOW_KNEE_RATIO`] above it.
    #[must_use]
    pub const fn from_muffle(muffle: f64) -> Option<Self> {
        Self::new(muffle, muffle / LOW_KNEE_RATIO)
    }

    /// The knee the game renders with.
    ///
    /// A `const` item, on the same reasoning as [`crease::CreaseKnee::SHIPPED`]:
    /// the derivation from `SOURCE_THROUGH` is discharged at compile time, so
    /// a muffle that cannot derive a knee stops the build rather than falling
    /// back to a hand-copy of today's `(0.30, 0.60)` that no input could
    /// reach. The theorem this module rests on — a walled source cannot draw
    /// a crease — is a statement about THIS pair, and a fallback holding a
    /// different pair would have quietly falsified it.
    ///
    /// The `panic!` never runs: an unsatisfiable derivation is a build error.
    pub const SHIPPED: Self = match Self::from_muffle(SOURCE_THROUGH) {
        Some(knee) => knee,
        None => panic!("SOURCE_THROUGH does not derive a detail knee GLSL can fade"),
    };

    /// The knee the game renders with — see [`Self::SHIPPED`].
    #[must_use]
    pub const fn shipped() -> Self {
        Self::SHIPPED
    }

    /// Where detail begins to fade — and, by the theorem above, the ceiling
    /// on a once-walled source's reveal.
    #[must_use]
    pub fn lo(self) -> f64 {
        f64::from(self.lo)
    }

    /// Where detail reads at full strength.
    #[must_use]
    pub fn hi(self) -> f64 {
        f64::from(self.hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::reveal::{SourceImage, source_image};

    /// THE BREAK: the detail knee and the wall factor drifting apart, so a
    /// source behind a wall starts drawing its own internal creases again —
    /// the fan's blades legible through stone — and the perception model
    /// silently stops being the one that was designed.
    ///
    /// This is the theorem, not a mirror assertion: it sweeps the whole
    /// input domain of `source_image` rather than restating one constant in
    /// terms of another. Raise SOURCE_THROUGH, lower the knee, or change
    /// how `source_image` composes its two factors, and it fails.
    #[test]
    fn the_first_wall_takes_a_sources_identity() {
        let knee = DetailKnee::shipped();
        for w in 0..=40 {
            for v in 0..=40 {
                let wave = f64::from(w) / 40.0;
                let volume = f64::from(v) / 40.0;
                let image = SourceImage {
                    volume,
                    muffle: SOURCE_THROUGH,
                };
                let lit = source_image(wave, image);
                assert!(
                    lit <= knee.lo(),
                    "a source one wall away read {lit} against a detail \
                     knee opening at {} — at or above that floor it draws \
                     creases, so a walled radio becomes identifiable as a \
                     radio (wave {wave}, volume {volume})",
                    knee.lo()
                );
            }
        }
    }

    /// THE BREAK: the knee ceasing to track the wall factor, so that
    /// raising `SOURCE_THROUGH` (a thing a designer may reasonably do — it
    /// is how findable a source is through stone) leaves the detail knee
    /// where it was, and the theorem above starts holding only by luck.
    ///
    /// Both halves are needed and neither is redundant. The SYMBOLIC pair
    /// catches the derivation being severed; the HAND-DERIVED pair catches
    /// `SOURCE_THROUGH` moving without anyone meaning to move what the
    /// renderer fades over, which the symbolic pair would follow in
    /// silence.
    ///
    /// HONEST LIMIT, stated rather than papered over: retyping
    /// [`DetailKnee::shipped`] as the literal `{ lo: 0.3, hi: 0.6 }` passes
    /// every test in this module *today*, because the literal is what the
    /// derivation currently yields. Nothing here can distinguish them at
    /// one point. What the symbolic assertion buys is that the severed
    /// version fails the instant `SOURCE_THROUGH` changes — which is the
    /// only moment at which the difference has ever mattered.
    #[test]
    fn the_knee_tracks_the_wall_factor_and_the_creases_own_ratio() {
        let knee = DetailKnee::shipped();
        // narrowed the same way the knee itself narrows: the GPU only ever
        // sees f32, so f64::from(0.3_f32) is 0.30000001192092896 and an
        // exact f64 comparison would be testing the rounding, not the law
        assert_eq!(
            knee.lo(),
            f64::from(SOURCE_THROUGH as f32),
            "the knee stopped opening at the wall factor"
        );
        assert_eq!(
            knee.hi(),
            f64::from((SOURCE_THROUGH / LOW_KNEE_RATIO) as f32),
            "the knee stopped closing at the creases' own ratio above it"
        );
        // what those resolve to in the shipped build, hand-derived, so the
        // rendered value cannot move unremarked
        assert!((knee.lo() - 0.3).abs() < 1e-6, "lo was {}", knee.lo());
        assert!((knee.hi() - 0.6).abs() < 1e-6, "hi was {}", knee.hi());
    }

    /// THE BREAK: an unfadeable pair reaching the GPU, where it is a
    /// division by zero or an inverted fade rather than an error.
    #[test]
    fn a_knee_that_cannot_fade_is_unrepresentable() {
        assert_eq!(DetailKnee::new(0.5, 0.5), None);
        assert_eq!(DetailKnee::new(0.6, 0.3), None);
        assert_eq!(DetailKnee::new(f64::NAN, 1.0), None);
        assert_eq!(DetailKnee::new(0.0, f64::INFINITY), None);
        // strictly ordered as f64, the same lane as f32 — the narrowing
        // must happen first or this pair slips through
        assert_eq!(DetailKnee::new(0.3, 0.3 + 1e-12), None);
        // a muffle of zero has no knee to derive: nothing gets through, so
        // there is no fade to describe
        assert_eq!(DetailKnee::from_muffle(0.0), None);
        assert_eq!(DetailKnee::from_muffle(f64::NAN), None);
    }
}
