//! The shapes a prop can be, as pure geometry. The world was boxes only
//! for as long as it had one prop class; a contours-only world reads a
//! curve and a slope completely differently from a box, so the vocabulary
//! is three now — box, column, wedge — and the two that need real geometry
//! are built here where cargo can hold them.
//!
//! Only the WEDGE needs generated triangles: a box and a cylinder are
//! engine primitives with engine colliders, but a triangular prism is
//! neither. Its faces are emitted with EXPLICIT normals, because normals
//! are not decoration in this renderer — the data pass packs them into the
//! G channel as the crease id for any surface the level did not paint with
//! a flat object id, and the outline pass draws a line wherever they step.
//! A wedge with smeared normals would draw a smeared edge.
//!
//! Winding is not load-bearing: the world skin renders `cull_disabled`
//! (`data_pass.gdshader`), so a face is never dropped for facing away. The
//! normals are what must be right.

use godot::builtin::Vector3;

/// The six corners of a wedge that fills `size`: a box whose top has been
/// sloped away, rising from the bottom of the −X end to the full height at
/// the +X end. The hull is centred on the origin; the wedge NODE lifts it
/// half a height, so the shape stands on the node a designer placed.
///
/// Order is the deterministic one the triangles below index into: the four
/// floor corners first, then the two at the tall edge.
#[must_use]
pub fn wedge_hull(size: Vector3) -> [Vector3; 6] {
    let h = size.abs() * 0.5;
    [
        Vector3::new(-h.x, -h.y, -h.z), // A: low edge, −Z
        Vector3::new(h.x, -h.y, -h.z),  // B: tall edge foot, −Z
        Vector3::new(-h.x, -h.y, h.z),  // D: low edge, +Z
        Vector3::new(h.x, -h.y, h.z),   // E: tall edge foot, +Z
        Vector3::new(h.x, h.y, -h.z),   // C: tall edge top, −Z
        Vector3::new(h.x, h.y, h.z),    // F: tall edge top, +Z
    ]
}

/// The outward normal of the sloped face — up and back over the low end.
/// Total on degenerate sizes: a wedge with no length and no height has no
/// slope to speak of, and answers UP rather than a zero vector that would
/// poison the packed normal id.
#[must_use]
pub fn wedge_slope_normal(size: Vector3) -> Vector3 {
    let n = Vector3::new(-size.y.abs(), size.x.abs(), 0.0);
    if n.length() < 1e-9 {
        return Vector3::UP;
    }
    n.normalized()
}

/// The wedge's surface as triangles: `(position, normal)` per vertex, three
/// vertices per triangle, eight triangles — two for the floor, two for the
/// tall back face, two for the slope, and one for each triangular side.
///
/// Every vertex lies on the bounding box `size`, so the level's world-box
/// derivation (and therefore the object-id colouring) needs no special case
/// for a wedge.
#[must_use]
pub fn wedge_triangles(size: Vector3) -> Vec<(Vector3, Vector3)> {
    let [a, b, d, e, c, f] = wedge_hull(size);
    let slope = wedge_slope_normal(size);
    let mut out = Vec::with_capacity(24);
    let mut face = |tri: [Vector3; 3], normal: Vector3| {
        for v in tri {
            out.push((v, normal));
        }
    };
    face([a, b, e], Vector3::DOWN);
    face([a, e, d], Vector3::DOWN);
    face([b, c, f], Vector3::RIGHT);
    face([b, f, e], Vector3::RIGHT);
    face([a, d, f], slope);
    face([a, f, c], slope);
    face([a, c, b], Vector3::FORWARD);
    face([d, e, f], Vector3::BACK);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Vector3 = Vector3::new(1.2, 0.6, 0.8);

    /// Every vertex the wedge emits lies inside the box a designer sized
    /// it with — which is what lets the level derive its world box, and so
    /// its object id, exactly as it does for a plain box prop.
    #[test]
    fn the_wedge_stays_inside_its_own_box() {
        let h = SIZE * 0.5;
        for (v, _) in wedge_triangles(SIZE) {
            assert!(v.x >= -h.x - 1e-6 && v.x <= h.x + 1e-6, "x out of box: {v}");
            assert!(v.y >= -h.y - 1e-6 && v.y <= h.y + 1e-6, "y out of box: {v}");
            assert!(v.z >= -h.z - 1e-6 && v.z <= h.z + 1e-6, "z out of box: {v}");
        }
    }

    /// A wedge is a triangular prism: six corners, all distinct, four on
    /// the floor and two on the tall edge.
    #[test]
    fn the_hull_is_six_distinct_corners() {
        let hull = wedge_hull(SIZE);
        for i in 0..hull.len() {
            for j in (i + 1)..hull.len() {
                assert!(
                    (hull[i] - hull[j]).length() > 1e-6,
                    "corners {i}/{j} coincide"
                );
            }
        }
        assert_eq!(hull.iter().filter(|v| v.y < 0.0).count(), 4);
        assert_eq!(hull.iter().filter(|v| v.y > 0.0).count(), 2);
    }

    /// Eight triangles, twenty-four vertices, every normal a unit vector —
    /// a non-unit normal would pack a wrong crease id into G and draw an
    /// outline where there is no edge.
    #[test]
    fn every_normal_is_a_unit_vector() {
        let tris = wedge_triangles(SIZE);
        assert_eq!(tris.len(), 24);
        for (_, n) in &tris {
            assert!((n.length() - 1.0).abs() < 1e-5, "normal not unit: {n}");
        }
    }

    /// The slope's normal is perpendicular to the slope itself — the test
    /// that would catch a sign slip that a "looks fine" screenshot would
    /// not: it must be orthogonal to both the rising edge and the run.
    #[test]
    fn the_slope_normal_is_perpendicular_to_the_slope() {
        let [a, _b, d, _e, c, _f] = wedge_hull(SIZE);
        let n = wedge_slope_normal(SIZE);
        let rise = c - a; // low edge to tall edge, up the ramp
        let run = d - a; // across the ramp
        assert!(n.dot(rise).abs() < 1e-5, "normal not perpendicular to rise");
        assert!(n.dot(run).abs() < 1e-5, "normal not perpendicular to run");
        assert!(n.y > 0.0, "the slope faces upward");
        assert!(n.x < 0.0, "the slope faces back over its low end");
    }

    /// Total on the shapes a designer can type by accident: a zero or
    /// negative extent yields a degenerate but finite wedge, never a NaN
    /// normal that would poison the packed data channels.
    #[test]
    fn degenerate_sizes_stay_finite() {
        for size in [
            Vector3::ZERO,
            Vector3::new(0.0, 1.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(-1.0, -1.0, -1.0),
        ] {
            let n = wedge_slope_normal(size);
            assert!(n.is_finite(), "normal not finite for {size}");
            assert!(
                (n.length() - 1.0).abs() < 1e-5,
                "normal not unit for {size}"
            );
            for (v, n) in wedge_triangles(size) {
                assert!(v.is_finite() && n.is_finite());
            }
        }
    }

    /// A negative extent is the same wedge as its absolute — a designer's
    /// minus sign flips nothing, exactly as a box mesh ignores it.
    #[test]
    fn a_negative_extent_is_the_same_wedge() {
        assert_eq!(
            wedge_hull(Vector3::new(-1.2, 0.6, -0.8)),
            wedge_hull(Vector3::new(1.2, 0.6, 0.8))
        );
    }
}
