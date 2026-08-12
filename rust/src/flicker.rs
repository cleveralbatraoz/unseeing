//! Nervous light: the reveal intensity wavers around 1.0, with rare brief
//! dropouts — part of the mood, not noise. A bit-for-bit transcription of
//! `game/scripts/flicker.gd:33-43`, the validated law it replaces; every
//! constant and the exact `randf()` draw ORDER are carried over verbatim so
//! a seeded stream advances identically whichever side drives it (pinned by
//! `game/tests/flicker_parity_test.gd`, which drives both against the same
//! seed).
//!
//! A pure state machine over an injected [`Randf`] source: no global
//! randomness anywhere, so a seeded stream replays bit-identically (movie-
//! maker runs, frame-comparison CI, the reproduction blob's restore). The
//! Rust boundary is [`Randf`] itself — this module never touches a
//! `RandomNumberGenerator` or any other Godot type; the engine layer
//! (`WaveCore::flicker_probe`) owns the adapter that widens `randf()`'s f32
//! into the f64 this module computes with.

/// A source of randomness the flicker law draws from — the boundary that
/// keeps this module free of Godot types. Godot's own
/// `RandomNumberGenerator::randf()` returns f32; the engine layer adapts it
/// with `rng.randf() as f64` at every draw, widening at exactly the point
/// the GDScript law implicitly did (every GDScript float is f64, so
/// `_rng.randf()` was already widened the instant it entered an
/// expression). Cargo tests implement this trait with a scripted stub.
pub trait Randf {
    /// The next pseudo-random value in [0.0, 1.0].
    fn randf(&mut self) -> f64;
}

/// Floor the eased level may relax to outside a dropout.
pub const LEVEL_MIN: f64 = 0.72;
/// Ceiling the eased level may relax to.
pub const LEVEL_MAX: f64 = 1.2;
/// A dropout dims the clamped level to just over half — `flicker.gd`'s
/// `DROP_DEPTH`.
pub const DROP_DEPTH: f64 = 0.55;
/// How hard the level is pulled back toward 1.0 each frame.
const RELAX: f64 = 0.12;
/// Half-width of the per-frame random jitter added on top of relaxation.
const JITTER: f64 = 0.09;
/// Shortest a dropout can last, in seconds.
const DROP_LEN_MIN: f64 = 0.08;
/// Random extra length a dropout can carry, on top of [`DROP_LEN_MIN`].
const DROP_LEN_JITTER: f64 = 0.1;
/// Shortest gap between the end of one dropout window's scheduling and the
/// next, in seconds.
const DROP_SPACING_MIN: f64 = 8.0;
/// Random extra gap on top of [`DROP_SPACING_MIN`].
const DROP_SPACING_JITTER: f64 = 10.0;

/// A snapshot of [`Flicker`]'s four fields — the shape a reproduction blob
/// carries across a capture/restore boundary, field for field matching
/// `flicker.gd`'s `_t`/`_level`/`_drop_until`/`_next_drop`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlickerState {
    /// Seconds elapsed since this flicker was created.
    pub t: f64,
    /// The current eased intensity, already clamped and (if inside a
    /// dropout) already dimmed — this is the value [`Flicker::next`]
    /// returns, not a pre-dim value.
    pub level: f64,
    /// The instant the current dropout window ends. `-1.0` (the
    /// constructor's value) means "no dropout has ever been scheduled",
    /// which is always before any reachable `t`, so the dimming check
    /// never mistakenly fires before the first one is scheduled.
    pub drop_until: f64,
    /// Seconds remaining until the next dropout is scheduled.
    pub next_drop: f64,
}

/// The mood's envelope: one instance per light, advanced one frame at a
/// time by [`Flicker::next`]. Bounded forever in
/// `[LEVEL_MIN * DROP_DEPTH, LEVEL_MAX]` — the clamp sets the ceiling and
/// ordinary floor, a dropout dimming the clamped floor sets the true floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flicker {
    t: f64,
    level: f64,
    drop_until: f64,
    next_drop: f64,
}

impl Flicker {
    /// A fresh flicker: `flicker.gd`'s field initializers verbatim
    /// (`_t := 0.0`, `_level := 1.0`, `_drop_until := -1.0`,
    /// `_next_drop := 9.0` — the first dropout is scheduled 9 seconds in).
    #[must_use]
    pub fn new() -> Self {
        Self {
            t: 0.0,
            level: 1.0,
            drop_until: -1.0,
            next_drop: 9.0,
        }
    }

    /// Advance the envelope by one frame and return this frame's
    /// intensity. Draws from `rng` in the exact order `flicker.gd:33-43`
    /// does: one draw for the level jitter, always; two more — drop length
    /// then drop spacing — only on the frame a new dropout is scheduled.
    /// That order is load-bearing: a seeded stream shared with the
    /// GDScript original (or any other caller of the same stream) desyncs
    /// the instant the draw count or order differs, even if every
    /// individual formula stays correct.
    pub fn next(&mut self, dt: f64, rng: &mut impl Randf) -> f64 {
        self.t += dt;
        self.level += (1.0 - self.level) * RELAX + (rng.randf() - 0.5) * JITTER;
        self.level = self.level.clamp(LEVEL_MIN, LEVEL_MAX);
        self.next_drop -= dt;
        if self.next_drop <= 0.0 {
            self.drop_until = self.t + DROP_LEN_MIN + rng.randf() * DROP_LEN_JITTER;
            self.next_drop = DROP_SPACING_MIN + rng.randf() * DROP_SPACING_JITTER;
        }
        if self.t < self.drop_until {
            // The STORED level is dimmed, not just the returned value —
            // the next frame's relaxation term pulls back from the dimmed
            // number, which is what lets consecutive dropout frames
            // compound down toward the documented floor instead of each
            // one independently dimming a fresh relaxation.
            self.level *= DROP_DEPTH;
        }
        self.level
    }

    /// This flicker's four fields, for a reproduction blob to carry across
    /// a capture boundary.
    #[must_use]
    pub fn state(&self) -> FlickerState {
        FlickerState {
            t: self.t,
            level: self.level,
            drop_until: self.drop_until,
            next_drop: self.next_drop,
        }
    }

    /// Replace this flicker's four fields wholesale — the write side of
    /// [`Self::state`], for a reproduction blob's restore.
    pub fn restore(&mut self, s: FlickerState) {
        self.t = s.t;
        self.level = s.level;
        self.drop_until = s.drop_until;
        self.next_drop = s.next_drop;
    }
}

impl Default for Flicker {
    /// Same as [`Self::new`] — a flicker has no meaningful zero value
    /// besides its own fresh state.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// A fixed sequence of `randf()` results, consumed strictly in call
    /// order. [`Randf::randf`] panics on a call past the end of the
    /// scripted sequence — proof the law drew no MORE draws than the
    /// scenario expects; [`Stub::exhausted`] after the scenario proves it
    /// drew no FEWER, either. Together they pin the exact draw count,
    /// which is what keeps a shared seeded stream in sync with
    /// `flicker.gd`.
    struct Stub {
        draws: VecDeque<f64>,
    }

    impl Stub {
        fn new(draws: impl IntoIterator<Item = f64>) -> Self {
            Self {
                draws: draws.into_iter().collect(),
            }
        }

        fn exhausted(&self) -> bool {
            self.draws.is_empty()
        }
    }

    impl Randf for Stub {
        fn randf(&mut self) -> f64 {
            self.draws
                .pop_front()
                .expect("flicker drew more randf() calls than the scenario scripted")
        }
    }

    /// The constructor, pinned field by field against `flicker.gd:18-21`.
    #[test]
    fn fresh_flicker_starts_at_the_gdscript_defaults() {
        let s = Flicker::new().state();
        assert_eq!(s.t, 0.0);
        assert_eq!(s.level, 1.0);
        assert_eq!(s.drop_until, -1.0);
        assert_eq!(s.next_drop, 9.0);
    }

    /// Nine seconds of a steady 1.0 (relax term is zero at the fixed
    /// point, and every draw of 0.5 zeroes the jitter term too), then the
    /// first scheduled dropout lands exactly on schedule and clamps the
    /// relaxation out of it the following frame — all while drawing
    /// EXACTLY the number of `randf()` calls `flicker.gd` would for this
    /// sequence: one per frame, plus two more on the triggering frame.
    #[test]
    fn lead_in_then_scheduled_dropout_draws_exactly_as_many_randf_calls_as_gdscript() {
        // 8 lead-in frames (1 draw each) + frame 9's trigger (1 + 2 draws)
        // + 2 more ordinary frames (1 draw each) = 13.
        let mut stub = Stub::new([0.5; 13]);
        let mut f = Flicker::new();

        // Frames 1-8: _next_drop counts 9.0 down by 1.0/frame without
        // crossing zero (9.0 - 8*1.0 = 1.0 > 0), and the level sits at its
        // own fixed point: (1.0-1.0)*0.12 + (0.5-0.5)*0.09 = 0.0.
        for frame in 1..=8 {
            assert_eq!(f.next(1.0, &mut stub), 1.0, "frame {frame}");
        }

        // Frame 9: _next_drop = 9.0 - 9*1.0 = 0.0 <= 0.0, so the dropout
        // fires. t = 9.0; drop_until = 9.0 + 0.08 + 0.5*0.1 = 9.13;
        // t(9.0) < drop_until(9.13), so the still-1.0 stored level dims:
        // 1.0 * 0.55.
        assert_eq!(f.next(1.0, &mut stub), 1.0 * 0.55);

        // Frame 10: t = 10.0. Relaxing the DIMMED level back toward 1.0
        // gives 0.55 + (1.0 - 0.55) * 0.12 = 0.604, which undershoots
        // LEVEL_MIN (0.72) and clamps there; t(10.0) is past
        // drop_until(9.13), so no further dimming this frame.
        let relaxed_from_dim = 0.55 + (1.0 - 0.55) * 0.12;
        assert!(relaxed_from_dim < 0.72, "the clamp must actually bite here");
        assert_eq!(f.next(1.0, &mut stub), 0.72);

        // Frame 11: relaxing off the clamped floor, no clamp and no
        // dropout in play.
        assert_eq!(f.next(1.0, &mut stub), 0.72 + (1.0 - 0.72) * 0.12);

        assert!(
            stub.exhausted(),
            "flicker drew fewer randf() calls than the scenario scripted"
        );
    }

    /// Relaxation plus the maximum jitter draw can overshoot LEVEL_MAX;
    /// the clamp catches it.
    #[test]
    fn relaxation_plus_jitter_clamps_at_the_ceiling() {
        let mut f = Flicker::default();
        f.restore(FlickerState {
            t: 0.0,
            level: 1.19,
            drop_until: -1.0,
            next_drop: 100.0, // far from triggering: isolates the clamp
        });
        let raw = 1.19 + (1.0 - 1.19) * 0.12 + (1.0 - 0.5) * 0.09;
        assert!(raw > 1.2, "the clamp must actually bite here");
        let mut stub = Stub::new([1.0]); // max jitter draw
        assert_eq!(f.next(0.01, &mut stub), 1.2);
        assert!(stub.exhausted());
    }

    /// A triggering frame draws in a fixed order — level jitter, THEN drop
    /// length, THEN drop spacing. Three DISTINCT draws prove it: swap any
    /// two and at least one of the three assertions below reads the wrong
    /// value.
    #[test]
    fn a_trigger_draws_level_jitter_then_drop_length_then_spacing() {
        let mut f = Flicker::default();
        f.restore(FlickerState {
            t: 0.0,
            level: 1.0,
            drop_until: -1.0,
            next_drop: 0.001, // crosses zero on the very next frame
        });
        let mut stub = Stub::new([1.0, 0.0, 1.0]); // level, drop_len, spacing

        let returned = f.next(0.5, &mut stub);

        // Draw #1 (1.0) is the level jitter:
        // 1.0 + (1.0-1.0)*0.12 + (1.0-0.5)*0.09 = 1.045.
        // next_drop = 0.001 - 0.5 <= 0.0: the drop fires.
        // Draw #2 (0.0) is drop length: drop_until = 0.5 + 0.08 + 0.0*0.1.
        // Draw #3 (1.0) is spacing: next_drop = 8.0 + 1.0*10.0.
        // t(0.5) < drop_until(0.58): the stored level dims by 0.55.
        let level_after_jitter = 1.0 + (1.0 - 1.0) * 0.12 + (1.0 - 0.5) * 0.09;
        let expected = level_after_jitter * 0.55;
        assert_eq!(returned, expected);

        let s = f.state();
        assert_eq!(s.level, expected, "the dim is stored, not just returned");
        assert_eq!(s.drop_until, 0.5 + 0.08 + 0.0 * 0.1);
        assert_eq!(s.next_drop, 8.0 + 1.0 * 10.0);
        assert!(stub.exhausted());
    }

    /// Inside a dropout, the stored level compounds down each frame; the
    /// floor it settles at is exactly LEVEL_MIN * DROP_DEPTH — the clamp's
    /// floor, dimmed — which is also what `flicker_test.gd`'s 100k-frame
    /// envelope test asserts as the lower bound.
    #[test]
    fn dropout_compounds_the_stored_level_down_to_the_documented_floor() {
        let mut f = Flicker::default();
        f.restore(FlickerState {
            t: 0.5,
            level: 0.396, // one frame already dimmed: LEVEL_MIN * DROP_DEPTH
            drop_until: 0.6,
            next_drop: 18.0, // far from triggering again
        });
        let relaxed = 0.396 + (1.0 - 0.396) * 0.12;
        assert!(relaxed < 0.72, "the clamp must actually bite here");
        let mut stub = Stub::new([0.5]); // zero jitter: isolates the compounding
        let floor = LEVEL_MIN * DROP_DEPTH;
        assert_eq!(f.next(0.02, &mut stub), floor);
        assert!(stub.exhausted());
    }
}
