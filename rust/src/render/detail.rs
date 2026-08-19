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
//! full reach and lasts its full tail; a SOURCE's detail resolves only
//! while its image arrives loud. The ordering borrows one finding of
//! non-visual sensing that survived this campaign's adversarial review
//! intact: expert echolocators detect an object's PRESENCE across a room
//! while its SHAPE needs near-contact (Kolarik et al., *Hearing Research*
//! 310 (2014) 60-68; Milne, Goodale & Thaler, *Atten Percept Psychophys*
//! 76(6) (2014) 1828-1837).
//!
//! # The knee's scope: what the eye sees through a wall, nothing else
//!
//! Two playtests narrowed the scope to exactly the theorem's precondition.
//! The first shipped composition gated EVERY crease in the game on this
//! knee, and a playtest rejected it in one sentence per symptom: a
//! footstep's reveal peaks at 0.330 one metre out — under the knee's own
//! 0.30 floor — so a walking hero swept a room whose corners, floor seams
//! and face edges never drew, and a tap's crease field tore off at the
//! 0.30 reveal contour mid-object. The second cut gated everything the
//! depth buffer calls an acoustic image, and a screenshot rejected that
//! too: an UNWALLED source is an image, so a quiet radio in the open drew
//! its chassis and antenna only inside a passing wave's wash and tore
//! everywhere else, the tear's boundary tracking the wave's own rim.
//!
//! The theorem below says a source BEHIND A WALL cannot draw a crease.
//! Behind a wall is the shader's `seen_walled` — the wall-table verdict on
//! the eye's sight line — not "is an image": an unwalled source's muffle
//! is 1 and needs no gating, and the world is never walled at all (a wall
//! would hide it). [`DetailKnee::gate`] is the scoped law and the
//! shader's cargo twin.
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
use super::knee::Knee;
use crate::level_plan::SOURCE_THROUGH;

/// Where a swept surface stops telling you *what* it is, while still
/// telling you *that* it is there — a knee in units of REVEAL.
///
/// A validated type over [`Knee`], which owns the reason: GLSL's
/// `smoothstep` divides by `hi - lo`, so the bad pairs are unrepresentable
/// rather than commented against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DetailKnee(Knee);

impl DetailKnee {
    /// A detail knee, or `None` if the pair cannot fade.
    ///
    /// Total over every f64 pair, by [`Knee::new`]'s contract.
    #[must_use]
    pub const fn new(lo: f64, hi: f64) -> Option<Self> {
        match Knee::new(lo, hi) {
            Some(knee) => Some(Self(knee)),
            None => None,
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
        self.0.lo()
    }

    /// Where detail reads at full strength.
    #[must_use]
    pub fn hi(self) -> f64 {
        self.0.hi()
    }

    /// How much detail this fragment keeps — the Rust twin of the shader's
    /// composition, and the SCOPE of this whole module in one branch: the
    /// knee fades the creases of a surface the eye sees THROUGH A WALL —
    /// `walled` is the shader's `seen_walled`, the wall-table verdict on
    /// the eye's own sight line — and everything else passes untouched.
    ///
    /// The scope is the theorem's own precondition and it took two
    /// playtests to land exactly there. Gating every crease in the game
    /// shipped first: a footstep's reveal peaks at 0.330 one metre out —
    /// under this knee's own 0.30 floor — so a walking hero swept a room
    /// whose corners, floor seams and face edges never drew. The second
    /// cut gated everything the depth buffer called an acoustic image, and
    /// a second screenshot rejected that too: an UNWALLED source is an
    /// image, so a quiet radio in the open drew its chassis and antenna
    /// only inside a passing wave's wash and tore at the 0.30 reveal
    /// contour — the tear's boundary tracked the wave's own rim. The
    /// theorem says a source BEHIND A WALL cannot draw a crease; behind a
    /// wall is `walled`, not "is an image", and an unwalled source's
    /// muffle is 1, so nothing about it needs gating.
    ///
    /// Total over every f64 reveal in both arms: the walled arm has
    /// [`Knee::fade`]'s totality (a non-finite reveal draws nothing), and
    /// the clear arm never reads the reveal at all.
    #[must_use]
    pub fn gate(self, walled: bool, reveal: f64) -> f64 {
        if walled { self.0.fade(reveal) } else { 1.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::reveal::{SourceImage, source_image};

    /// THE BREAK: gating the WORLD's creases on the detail knee, which is
    /// what shipped and what a playtest rejected on sight. A footstep's
    /// reveal peaks at 0.35 x atten — 0.330 one metre out — under the
    /// knee's own 0.30 floor almost everywhere, so a walking hero swept a
    /// room whose corners, floor seams and face edges never drew, and a
    /// tap's crease field tore off at the 0.30 reveal contour mid-object.
    /// The world is never walled from the eye — a wall would hide it —
    /// so under the scoped gate its creases pass untouched.
    #[test]
    fn a_footsteps_dim_sweep_keeps_the_worlds_creases() {
        let knee = DetailKnee::shipped();
        // hand-derived: peak 0.35 * atten 1/(1 + 0.06) = 0.3302
        let step_reveal = 0.330;
        assert!(
            (knee.gate(false, step_reveal) - 1.0).abs() < f64::EPSILON,
            "the world's creases no longer pass the knee untouched"
        );
        // ...while the same reveal on a surface seen through a wall stays
        // inside the fade: smoothstep(0.3, 0.6, 0.330) = 0.1^2 * (3 - 0.2)
        let walled_detail = knee.gate(true, step_reveal);
        // 1e-6 and not 1e-12: the smoothstep evaluates through three f64
        // subtractions whose representation error reaches 2.4e-8; any real
        // retune of the knee moves this by whole percents.
        assert!(
            (walled_detail - 0.028).abs() < 1.0e-6,
            "the walled fade moved: {walled_detail}"
        );
    }

    /// THE BREAK: widening the gate's scope from "seen through a wall" to
    /// "is an acoustic image", which shipped for one commit and tore the
    /// second screenshot: an unwalled source is an image too, so a quiet
    /// radio in the open drew its chassis, dial and antenna only inside a
    /// passing wave's wash — bright inside the shell's rim, torn gaps
    /// everywhere else. Its muffle is 1 and the theorem claims nothing
    /// about it; its creases follow its reveal exactly like the world's.
    #[test]
    fn a_quiet_source_in_the_open_keeps_its_creases() {
        let knee = DetailKnee::shipped();
        // the radio's standing image at PRESENCE, far under the knee floor
        let presence_reveal = 0.068;
        assert!(
            (knee.gate(false, presence_reveal) - 1.0).abs() < f64::EPSILON,
            "an unwalled source's creases are gated again — the torn radio"
        );
    }

    /// THE BREAK: the walled arm quietly capped below full strength — a
    /// mutation (`fade(reveal).min(0.5)`) that the pre-merge review ran
    /// against the whole suite and watched survive, because every other
    /// test evaluates the walled arm at 0.330 or below. A loud walled
    /// source washed by a wave in its own doorway must be able to reach
    /// full detail once its reveal clears the knee, and the GLSL twin's
    /// smoothstep does; the cargo twin must match it across the fade.
    #[test]
    fn a_walled_reveal_past_the_knee_keeps_full_detail() {
        let knee = DetailKnee::shipped();
        // at the knee's own hi the plateau is exact; at the nominal 0.6 the
        // f32-narrowed hi sits a hair above, so that read is 1e-8 shy
        assert!(
            (knee.gate(true, knee.hi()) - 1.0).abs() < f64::EPSILON,
            "the fade no longer reaches full strength at the knee's hi"
        );
        assert!((knee.gate(true, 0.6) - 1.0).abs() < 1.0e-6);
        assert!((knee.gate(true, 1.0) - 1.0).abs() < f64::EPSILON);
        // the fade's midpoint, hand-derived: t = 0.5, Hermite 0.25 * 2 = 0.5
        let mid = knee.gate(true, 0.45);
        assert!(
            (mid - 0.5).abs() < 1.0e-6,
            "the fade's midpoint moved: {mid}"
        );
    }

    /// Totality at the boundary the two arms share: a non-finite reveal
    /// fades a walled surface to nothing ([`Knee::fade`]'s contract) and
    /// leaves a clear one alone — the clear arm never reads the reveal.
    #[test]
    fn a_non_finite_reveal_darkens_the_walled_and_spares_the_clear() {
        let knee = DetailKnee::shipped();
        assert_eq!(knee.gate(true, f64::NAN), 0.0);
        assert_eq!(knee.gate(true, f64::INFINITY), 0.0);
        assert!((knee.gate(false, f64::NAN) - 1.0).abs() < f64::EPSILON);
    }

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
                // ...and the composed gate delivers the theorem exactly: a
                // walled source's creases are not merely dim but ZERO
                assert_eq!(
                    knee.gate(true, lit),
                    0.0,
                    "the gate let a walled source draw at reveal {lit}"
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
