//! Shared temporal domain for renderer-visible simulation.
//!
//! The engine clock, flicker envelope, and demo scheduler must stop at one
//! horizon because they all drive shader state narrowed to `f32`. Keeping the
//! limit here makes those pure transitions share the same complete domain;
//! the Godot composition root only applies their returned values.

use crate::reproduce::RestoreValueError;

/// Largest simulation instant whose 60 Hz successor remains distinguishable
/// after narrowing to the shader's 32-bit time representation.
pub(crate) const RENDERER_VISIBLE_TIME_HORIZON: f64 = 262_144.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PreparedTime(f64);

pub(crate) fn prepare_time(value: f64) -> Result<PreparedTime, RestoreValueError> {
    if value.is_finite() && (0.0..=RENDERER_VISIBLE_TIME_HORIZON).contains(&value) {
        Ok(PreparedTime(value))
    } else {
        Err(RestoreValueError::new(
            "env.now",
            "must be finite and inside the renderer-visible time horizon",
        ))
    }
}

impl PreparedTime {
    pub(crate) fn value(self) -> f64 {
        self.0
    }
}

/// Advance the simulated clock over the complete `f64` input domain.
///
/// Invalid deltas pause one frame; invalid prior state restarts from zero;
/// huge finite deltas saturate at the renderer-visible horizon. The elapsed
/// result is the effective delta every delta-driven child must receive.
pub(crate) fn advance_clock(now: f64, delta: f64) -> (f64, f64, bool) {
    let (now, repaired_now) = valid_time_or_zero(now);
    let repaired_delta =
        !delta.is_finite() || delta < 0.0 || delta > RENDERER_VISIBLE_TIME_HORIZON - now;
    let delta = if delta.is_finite() && delta >= 0.0 {
        delta.min(RENDERER_VISIBLE_TIME_HORIZON)
    } else {
        0.0
    };
    let advanced = (now + delta).min(RENDERER_VISIBLE_TIME_HORIZON);
    (advanced, advanced - now, repaired_now || repaired_delta)
}

/// Accept a renderer-visible simulation instant or repair it to the epoch.
pub(crate) fn valid_time_or_zero(value: f64) -> (f64, bool) {
    if value.is_finite() && (0.0..=RENDERER_VISIBLE_TIME_HORIZON).contains(&value) {
        (value, false)
    } else {
        (0.0, true)
    }
}

#[cfg(test)]
mod tests {
    use super::{advance_clock, prepare_time, valid_time_or_zero};
    use crate::demo_tap::DemoTap;

    #[test]
    fn prepared_restore_rejects_invalid_time_or_demo_appointment() {
        let error = prepare_time(-0.25).expect_err("negative time must be refused");
        assert_eq!(error.path, "env.now");
        let error = DemoTap::prepare_restore(f64::INFINITY)
            .expect_err("infinite appointment must be refused");
        assert_eq!(error.path, "env.demo_next");
    }

    /// Reaching the finite horizon exactly is a valid transition, not a
    /// saturation repair that should warn at the engine boundary.
    #[test]
    fn advancing_exactly_to_the_horizon_is_valid_and_keeps_the_full_delta() {
        assert_eq!(advance_clock(262_140.0, 4.0), (262_144.0, 4.0, false));
    }

    /// Capture can legitimately contain the exact horizon. An exclusive
    /// validation range would rewind that valid final instant to the epoch.
    #[test]
    fn exact_horizon_is_a_valid_restored_time() {
        assert_eq!(valid_time_or_zero(262_144.0), (262_144.0, false));
    }

    #[test]
    fn game_clock_rejects_reversed_and_non_finite_frame_time() {
        for delta in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.01] {
            assert_eq!(
                advance_clock(12.5, delta),
                (12.5, 0.0, true),
                "delta {delta}"
            );
        }
        assert_eq!(advance_clock(f64::NAN, 0.25), (0.25, 0.25, true));
    }

    #[test]
    fn game_clock_saturates_huge_time_without_emitting_infinity() {
        let (now, elapsed, repaired) = advance_clock(12.5, f64::MAX);
        assert!(now.is_finite());
        assert_eq!(now, 262_144.0);
        assert!(elapsed.is_finite());
        assert!(now < f64::MAX);
        assert!(elapsed > 0.0);
        assert!(repaired);
    }

    /// This independently exercises the renderer's actual `f32` arithmetic,
    /// rather than deriving the expected boundary from the production
    /// constant that consumes it.
    #[test]
    fn simulation_horizon_is_last_power_of_two_where_shader_time_advances_at_sixty_hz() {
        let frame = 1.0_f32 / 60.0;
        assert!(262_144.0_f32 + frame > 262_144.0_f32);
        assert_eq!(524_288.0_f32 + frame, 524_288.0_f32);
    }
}
