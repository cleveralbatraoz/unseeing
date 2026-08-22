//! Nervous light: the reveal intensity wavers around 1.0, with rare brief
//! dropouts — part of the mood, not noise. A bit-for-bit transcription of
//! `game/scripts/flicker.gd:33-43`, the validated law it replaces; every
//! constant and the exact `randf()` draw ORDER are carried over verbatim so
//! a seeded stream advances identically whichever side drives it. The
//! 600-frame parity against the original GDScript was proven before the
//! original's retirement (commit 5eea935 → c0ecba9); the law's pins are this
//! file's own cargo tests (`test_flicker_starts_normal`, `test_level_floor_bounce`).
//!
//! A pure state machine over an injected [`Randf`] source: no global
//! randomness anywhere, so a seeded stream replays bit-identically (movie-
//! maker runs, frame-comparison CI, the reproduction blob's restore). The
//! Rust boundary is [`Randf`] itself — this module never touches a
//! `RandomNumberGenerator` or any other Godot type; the engine layer
//! (`crate::ffi`, driven by `UnseeingGame`) owns the adapter that widens `randf()`'s f32
//! into the f64 this module computes with.

use crate::reproduce::RestoreValueError;
use crate::temporal::RENDERER_VISIBLE_TIME_HORIZON;

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

#[derive(Clone, Copy, Debug)]
pub(crate) struct PreparedFlicker(Flicker);

impl Flicker {
    pub(crate) fn prepare_restore(
        state: FlickerState,
    ) -> Result<PreparedFlicker, RestoreValueError> {
        for (field, value, min, max) in [
            ("flicker_t", state.t, 0.0, RENDERER_VISIBLE_TIME_HORIZON),
            (
                "flicker_level",
                state.level,
                LEVEL_MIN * DROP_DEPTH,
                LEVEL_MAX,
            ),
            (
                "flicker_drop_until",
                state.drop_until,
                -1.0,
                RENDERER_VISIBLE_TIME_HORIZON,
            ),
            (
                "flicker_next_drop",
                state.next_drop,
                0.0,
                RENDERER_VISIBLE_TIME_HORIZON,
            ),
        ] {
            if !value.is_finite() {
                return Err(RestoreValueError::new(
                    format!("env.{field}"),
                    "must be finite",
                ));
            }
            if value < min || value > max {
                return Err(RestoreValueError::new(
                    format!("env.{field}"),
                    "is outside its valid range",
                ));
            }
        }
        Ok(PreparedFlicker(Self {
            t: state.t,
            level: state.level,
            drop_until: state.drop_until,
            next_drop: state.next_drop,
        }))
    }

    #[must_use]
    pub(crate) fn from_prepared(value: PreparedFlicker) -> Self {
        value.0
    }

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
        let dt = valid_delta(dt);
        self.t = (self.t + dt).min(RENDERER_VISIBLE_TIME_HORIZON);
        self.level += (1.0 - self.level) * RELAX + (valid_draw(rng.randf()) - 0.5) * JITTER;
        self.level = self.level.clamp(LEVEL_MIN, LEVEL_MAX);
        self.next_drop -= dt;
        if self.next_drop <= 0.0 {
            self.drop_until = (self.t + DROP_LEN_MIN + valid_draw(rng.randf()) * DROP_LEN_JITTER)
                .min(RENDERER_VISIBLE_TIME_HORIZON);
            self.next_drop = DROP_SPACING_MIN + valid_draw(rng.randf()) * DROP_SPACING_JITTER;
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
    pub fn restore(&mut self, s: FlickerState) -> bool {
        self.t = bounded_or(s.t, 0.0, 0.0, RENDERER_VISIBLE_TIME_HORIZON);
        self.level = bounded_or(s.level, 1.0, LEVEL_MIN * DROP_DEPTH, LEVEL_MAX);
        self.drop_until = bounded_or(s.drop_until, -1.0, -1.0, RENDERER_VISIBLE_TIME_HORIZON);
        self.next_drop = bounded_or(s.next_drop, 9.0, 0.0, RENDERER_VISIBLE_TIME_HORIZON);
        self.state() != s
    }
}

fn valid_delta(dt: f64) -> f64 {
    if dt.is_finite() && dt >= 0.0 {
        dt.min(RENDERER_VISIBLE_TIME_HORIZON)
    } else {
        0.0
    }
}

fn valid_draw(draw: f64) -> f64 {
    if draw.is_finite() {
        draw.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

fn bounded_or(value: f64, fallback: f64, min: f64, max: f64) -> f64 {
    if value.is_finite() && value >= min && value <= max {
        value
    } else {
        fallback
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

    #[test]
    fn prepared_restore_rejects_invalid_flicker_state() {
        let mut state = Flicker::new().state();
        state.level = LEVEL_MAX + 0.01;
        let error = Flicker::prepare_restore(state).expect_err("level poison must be refused");
        assert_eq!(error.path, "env.flicker_level");
    }

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

    /// Frame deltas are an engine boundary, so every f64 is admissible.
    /// Reversed and non-finite time must act like a paused frame while still
    /// consuming the ordinary jitter draw, preserving the seeded stream's
    /// one-draw-per-process contract without poisoning stored state.
    #[test]
    fn invalid_frame_deltas_cannot_poison_the_envelope_or_rng_cadence() {
        for delta in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            let mut flicker = Flicker::new();
            let mut stub = Stub::new([0.5]);

            assert_eq!(flicker.next(delta, &mut stub), 1.0, "delta {delta}");
            assert_eq!(flicker.state(), Flicker::new().state(), "delta {delta}");
            assert!(stub.exhausted(), "delta {delta}");
        }
    }

    /// A finite delta can still overflow addition. The state must remain
    /// finite and bounded, and a scheduling frame must never manufacture an
    /// infinite dropout deadline from it.
    #[test]
    fn a_huge_delta_saturates_at_a_representable_simulation_horizon() {
        let mut flicker = Flicker::new();
        let mut stub = Stub::new([0.5, 0.5, 0.5]);

        let level = flicker.next(f64::MAX, &mut stub);
        let state = flicker.state();

        assert!(level.is_finite());
        assert!(state.t.is_finite());
        assert!(state.drop_until.is_finite());
        assert!(state.next_drop.is_finite());
        assert_eq!(state.t, 262_144.0);
        assert_eq!(state.drop_until, 262_144.0);
        assert_eq!(state.next_drop, 13.0);
        assert_eq!(state.level, 1.0);
        assert!(stub.exhausted());
    }

    /// Restore is fed by a Variant dictionary and therefore cannot assume a
    /// well-formed capture. Every malformed float is repaired to a valid
    /// deterministic state before it can reach the next transition.
    #[test]
    fn malformed_restored_state_is_repaired_before_it_can_emit_poison() {
        let mut flicker = Flicker::new();
        assert!(flicker.restore(FlickerState {
            t: f64::NAN,
            level: f64::INFINITY,
            drop_until: f64::NEG_INFINITY,
            next_drop: -f64::MAX,
        }));

        assert_eq!(flicker.state(), Flicker::new().state());
        let mut stub = Stub::new([0.5]);
        assert_eq!(flicker.next(1.0 / 60.0, &mut stub), 1.0);
        assert!(flicker.state().t.is_finite());
        assert!(stub.exhausted());
    }

    /// The injected trait declares f64, not a refined random-sample type.
    /// Bad adapter output therefore has a defined neutral-jitter meaning.
    #[test]
    fn malformed_random_draw_is_neutral_and_never_poisonous() {
        for (draw, expected) in [
            (f64::NAN, 1.0),
            (f64::INFINITY, 1.0),
            (f64::NEG_INFINITY, 1.0),
            (-2.0, 0.955),
            (3.0, 1.045),
        ] {
            let mut flicker = Flicker::new();
            let mut stub = Stub::new([draw]);

            assert_eq!(flicker.next(0.25, &mut stub), expected, "draw {draw}");
            assert!(flicker.state().level.is_finite(), "draw {draw}");
            assert!(stub.exhausted(), "draw {draw}");
        }
    }

    /// Drop length and spacing consume the same untrusted trait values as
    /// jitter. Non-finite values are neutral; finite values clamp at the
    /// declared [0, 1] random-sample domain, with exact appointments proving
    /// neither branch can leak poison or silently skip a draw.
    #[test]
    fn malformed_trigger_draws_have_exact_neutral_and_clamped_appointments() {
        for (length, spacing, expected_until, expected_next) in [
            (f64::NAN, f64::INFINITY, 0.13, 13.0),
            (f64::NEG_INFINITY, f64::NAN, 0.13, 13.0),
            (-2.0, 3.0, 0.08, 18.0),
            (3.0, -2.0, 0.18, 8.0),
        ] {
            let mut flicker = Flicker::new();
            assert!(!flicker.restore(FlickerState {
                t: 0.0,
                level: 1.0,
                drop_until: -1.0,
                next_drop: 0.0,
            }));
            let mut stub = Stub::new([0.5, length, spacing]);

            assert_eq!(flicker.next(0.0, &mut stub), 0.55);
            let state = flicker.state();
            assert_eq!(state.drop_until, expected_until, "length {length}");
            assert_eq!(state.next_drop, expected_next, "spacing {spacing}");
            assert!(stub.exhausted());
        }
    }
}
