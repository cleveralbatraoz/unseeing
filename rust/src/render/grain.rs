//! The mood layer's own floor — what `hearing_post` does to a pixel AFTER
//! the perception laws have decided it, and therefore the dimmest a thing
//! can be drawn and still be seen.
//!
//! # Why a perception module owns the film grain
//!
//! Because a law was quietly broken by one. `level_plan::SOURCE_THROUGH`
//! documents a ladder — a source reads 0.30 through one wall, 0.09 through
//! two, 0.027 through three — and settled law 1 says a sound source is
//! ALWAYS VISIBLE, as itself, however many walls stand in the way. But the
//! post pass adds `(shash(…) - 0.5) * u_grain_amp` to every pixel and
//! multiplies by as little as [`VIGNETTE_FLOOR`] before it. At three walls
//! the ladder lands at 0.0122 against a grain half-swing of 0.0170: the
//! source is dimmer than the noise it is drawn in, and the law it is
//! supposed to obey has not held in any shipped frame.
//!
//! Nothing could have caught that, because the two halves lived in
//! different languages and neither knew the other existed. So the grain
//! moves to Rust — not to change it, but so that a quantity derived FROM it
//! ([`super::reveal::PRESENCE`]) can be cargo-tested against it. Same
//! pattern, and the same reason, as [`super::crease`]: a derivation across
//! a language boundary that nothing evaluates is how `MIN_SEP` and the
//! crease knee drifted apart while every test stayed green.

/// Peak-to-peak swing of the film grain, `u_grain_amp` in
/// `hearing_post.gdshader`. Owned here, pushed from the composition root,
/// read back out of the GLSL text by `shader_contract_test.gd`.
pub const GRAIN_AMP: f64 = 0.034;

/// The dimmest the breathing vignette multiplies a pixel by:
/// `mix(0.45, 1.0, vig)` in `hearing_post.gdshader`. At the screen edge,
/// fully breathed in, a pixel keeps less than half of what perception gave
/// it — which is what turns a merely-faint source into an invisible one.
pub const VIGNETTE_FLOOR: f64 = 0.45;

/// How far the grain actually moves a pixel, either way.
///
/// `(shash(…) - 0.5)` lies in `[-0.5, 0.5]`, so the swing about zero is
/// half [`GRAIN_AMP`], not all of it. Anything drawn below this is inside
/// the noise.
#[must_use]
pub fn half_swing() -> f64 {
    GRAIN_AMP * 0.5
}

/// What the mood layer leaves of a perception value at its most hostile —
/// the screen edge, fully breathed in.
///
/// Total over every f64: a non-finite input answers 0.0 rather than
/// carrying NaN into a comparison that would then silently answer `false`.
#[must_use]
pub fn dimmest(col: f64) -> f64 {
    if !col.is_finite() {
        return 0.0;
    }
    col * VIGNETTE_FLOOR
}
