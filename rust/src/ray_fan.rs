//! The golden-angle spherical ray fan — the sampling pattern of
//! echo-location. Pure math, mirrored exactly from the GDScript original
//! (pulses.gd): 26 directions spread by the golden angle, culled to the
//! hemisphere in front of the surface the sound was born on.

use godot::builtin::Vector3;

/// Rays per reflection fan. The shipped budget: enough for even coverage,
/// cheap enough to cast on every tap and footstep.
pub const RAYS: usize = 26;

/// The golden angle in radians — successive rays land maximally far apart.
pub const GOLDEN_ANGLE: f32 = 2.399_963;

/// Directions culled when pointing into the surface: dot(normal) below this
/// keeps along-surface rays (the surroundings of a tapped point answer all
/// around it) while dropping true into-the-wall directions.
pub const SHADOW_DOT: f32 = -0.05;

/// The i-th direction of the uniform spherical fan.
/// Identical formula to the GDScript original, so frames don't drift.
#[must_use]
pub fn fan_direction(i: usize) -> Vector3 {
    let y = 1.0 - 2.0 * (i as f32 + 0.5) / RAYS as f32;
    let r = (1.0 - y * y).max(0.0).sqrt();
    let phi = i as f32 * GOLDEN_ANGLE;
    Vector3::new(r * phi.cos(), y, r * phi.sin())
}

/// All fan directions surviving the hemisphere cull against
/// `origin_normal`. A zero normal means an airborne sound: no cull.
pub fn fan_directions(origin_normal: Vector3) -> impl Iterator<Item = Vector3> {
    (0..RAYS)
        .map(fan_direction)
        .filter(move |d| origin_normal == Vector3::ZERO || d.dot(origin_normal) >= SHADOW_DOT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_are_unit_length() {
        for i in 0..RAYS {
            let d = fan_direction(i);
            assert!((d.length() - 1.0).abs() < 1e-5, "ray {i} not unit: {d:?}");
        }
    }

    #[test]
    fn fan_matches_gdscript_formula() {
        // Spot-check ray 0 against hand-computed GDScript values:
        // y = 1 - 2*0.5/26, r = sqrt(1 - y^2), phi = 0.
        let d = fan_direction(0);
        let y = 1.0 - 1.0 / 26.0;
        assert!((d.y - y).abs() < 1e-6);
        assert!((d.x - (1.0f32 - y * y).sqrt()).abs() < 1e-6);
        assert!(d.z.abs() < 1e-6);
    }

    #[test]
    fn zero_normal_keeps_every_ray() {
        assert_eq!(fan_directions(Vector3::ZERO).count(), RAYS);
    }

    #[test]
    fn hemisphere_cull_drops_into_surface_rays_only() {
        let n = Vector3::UP;
        let kept: Vec<_> = fan_directions(n).collect();
        assert!(
            kept.len() < RAYS,
            "an upward normal must cull downward rays"
        );
        for d in &kept {
            assert!(d.dot(n) >= SHADOW_DOT);
        }
        // Every culled ray really points into the surface.
        for i in 0..RAYS {
            let d = fan_direction(i);
            if !kept.iter().any(|k| (*k - d).length() < 1e-6) {
                assert!(d.dot(n) < SHADOW_DOT);
            }
        }
    }

    #[test]
    fn coverage_is_even_no_two_rays_collapse() {
        for i in 0..RAYS {
            for j in (i + 1)..RAYS {
                let angle = fan_direction(i)
                    .dot(fan_direction(j))
                    .clamp(-1.0, 1.0)
                    .acos();
                assert!(angle > 0.15, "rays {i},{j} nearly collapse ({angle} rad)");
            }
        }
    }
}
