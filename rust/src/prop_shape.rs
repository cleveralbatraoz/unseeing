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

use godot::builtin::{Basis, Vector3};

/// World UP as the shape's own coordinates see it, carrying whatever
/// stretch the basis applies — the second ROW of the basis. Its dot with a
/// local point is that point's world height above the node, which is the
/// only question the standing law asks.
fn world_up_row(basis: Basis) -> Vector3 {
    Vector3::new(basis.col_a().y, basis.col_b().y, basis.col_c().y)
}

/// How far a cylinder of `radius` and `height`, centred on its node and
/// then turned by `basis`, hangs BELOW the node.
///
/// Exact for any basis, scale and shear included: the extreme of a
/// linearly mapped body is the body's own extreme taken along the
/// transposed direction, and a cylinder's is closed form — the half-height
/// along its axis component, the radius across the other two. Upright that
/// is the half-height and nothing else; laid on its side it is the RADIUS,
/// which is exactly the case a "lift by half the height" gets backwards.
#[must_use]
pub fn cylinder_underhang(basis: Basis, radius: f32, height: f32) -> f32 {
    let up = world_up_row(basis);
    height.abs() * 0.5 * up.y.abs() + radius.abs() * up.x.hypot(up.z)
}

/// How far a convex hull, centred on its node and then turned by `basis`,
/// hangs BELOW the node. A convex body's extreme is always at a vertex, so
/// walking the hull points is exact rather than a bound. Total on an empty
/// hull: nothing hangs below nothing.
#[must_use]
pub fn hull_underhang(basis: Basis, points: &[Vector3]) -> f32 {
    let up = world_up_row(basis);
    let low = points
        .iter()
        .map(|p| p.dot(up))
        .fold(f32::INFINITY, f32::min);
    if low.is_finite() { -low } else { 0.0 }
}

/// The LOCAL offset that puts a shape hanging `underhang` below its node
/// back on top of it: `underhang` meters straight up in WORLD space,
/// pulled back through the basis the shape is drawn under. That is what
/// makes "stands on its node" a law about the FLOOR rather than about the
/// node's own +Y — the two only agree while the node is upright.
///
/// Total on a basis that cannot be inverted (a prefab scaled flat, a knob
/// dragged to nothing): there is no world up to travel along, so nothing
/// moves. The determinant is checked BEFORE the inverse is asked for —
/// `Basis::inverse` runs into a `glam_assert` on a singular matrix, which
/// is a panic in a debug build and nonsense in a release one.
#[must_use]
pub fn standing_lift(basis: Basis, underhang: f32) -> Vector3 {
    let det = basis.determinant();
    if !underhang.is_finite() || !det.is_finite() || det == 0.0 {
        return Vector3::ZERO;
    }
    let lift = basis.inverse() * Vector3::new(0.0, underhang, 0.0);
    if lift.is_finite() {
        lift
    } else {
        Vector3::ZERO
    }
}

/// The lift a cylinder needs to stand on its node under `basis` — the one
/// call [`crate::nodes`]' column makes.
#[must_use]
pub fn cylinder_lift(basis: Basis, radius: f32, height: f32) -> Vector3 {
    standing_lift(basis, cylinder_underhang(basis, radius, height))
}

/// The lift a wedge of `size` needs to stand on its node under `basis`.
#[must_use]
pub fn wedge_lift(basis: Basis, size: Vector3) -> Vector3 {
    standing_lift(basis, hull_underhang(basis, &wedge_hull(size)))
}

/// Is a node scale near enough to 1 that folding it would only move things
/// by float dust? Decomposing a rotated basis does not always give back an
/// exact 1, and the shipped map is 129 nodes of scale exactly 1 — a fold
/// that fired on dust would nudge every one of them.
#[must_use]
pub fn scale_is_neutral(scale: Vector3) -> bool {
    (scale - Vector3::ONE).length() <= 1e-6
}

/// A three-component size under a node scale — exact, because a box and a
/// wedge are both built along their own local axes, so scaling the node and
/// scaling the knob are the same operation. Only the MAGNITUDE survives: a
/// mirrored axis is a reflection, not a size.
#[must_use]
pub fn fold_box_scale(size: Vector3, scale: Vector3) -> Vector3 {
    size * scale.abs()
}

/// A cylinder's two knobs under a node scale, and whether the scale was
/// expressible at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnKnobs {
    /// Radius in meters.
    pub radius: f64,
    /// Height in meters.
    pub height: f64,
    /// False when the cross-section was pulled by different amounts across
    /// X and Z. That asks for an ELLIPTIC cylinder — a shape neither
    /// `CylinderMesh` nor `CylinderShape3D` can be, and one this
    /// vocabulary deliberately does not own.
    pub round: bool,
}

/// A cylinder's knobs under a node scale. The axial component is exact;
/// across the cross-section the LARGER of the two is taken, so the barrel
/// that comes out CONTAINS the one that was drawn. Erring inwards would
/// leave drawn geometry outside the collider — the one failure mode this
/// renderer cannot show and the cane cannot feel.
#[must_use]
pub fn fold_column_scale(radius: f64, height: f64, scale: Vector3) -> ColumnKnobs {
    let across_x = f64::from(scale.x.abs());
    let across_z = f64::from(scale.z.abs());
    let wider = across_x.max(across_z);
    ColumnKnobs {
        radius: radius * wider,
        height: height * f64::from(scale.y.abs()),
        round: (across_x - across_z).abs() <= 1e-6 * wider.max(1.0),
    }
}

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
    const RADIUS: f32 = 0.3;
    const HEIGHT: f32 = 0.9;

    /// A turn of `deg` about `axis`, built from the engine's own primitive
    /// rather than from anything the code under test shares.
    fn turned(axis: Vector3, deg: f32) -> Basis {
        Basis::from_axis_angle(axis.normalized(), deg.to_radians())
    }

    /// The lowest world height any point of a lifted cylinder reaches,
    /// found by walking its two rims — a brute-force answer to hold the
    /// closed form against.
    fn lowest_of_cylinder(basis: Basis, lift: Vector3) -> f32 {
        let mut low = f32::INFINITY;
        for i in 0..720u16 {
            let a = f32::from(i) * std::f32::consts::TAU / 720.0;
            for cap in [-1.0f32, 1.0] {
                let p = Vector3::new(RADIUS * a.cos(), cap * HEIGHT * 0.5, RADIUS * a.sin());
                low = low.min((basis * (p + lift)).y);
            }
        }
        low
    }

    /// The same for a wedge, which needs no sampling: a convex hull's
    /// extreme is always one of its corners.
    fn lowest_of_wedge(basis: Basis, lift: Vector3) -> f32 {
        wedge_hull(SIZE)
            .iter()
            .map(|p| (basis * (*p + lift)).y)
            .fold(f32::INFINITY, f32::min)
    }

    /// The law as every shipped column and wedge meets it: upright, the
    /// lift is exactly half the height and not a float's breadth more.
    /// This is the promise that generalising the law changes no authored
    /// content — the shipped map's 34 columns and wedges are all upright.
    #[test]
    fn an_upright_shape_lifts_exactly_half_its_height() {
        assert_eq!(
            cylinder_lift(Basis::IDENTITY, RADIUS, HEIGHT),
            Vector3::new(0.0, HEIGHT * 0.5, 0.0)
        );
        assert_eq!(
            wedge_lift(Basis::IDENTITY, SIZE),
            Vector3::new(0.0, SIZE.y * 0.5, 0.0)
        );
        // and a yaw — the only turn a room prefab actually carries — is
        // still exactly half a height, because a turn about UP cannot
        // change what is under a shape
        let yawed = turned(Vector3::UP, 90.0);
        assert!(
            (cylinder_lift(yawed, RADIUS, HEIGHT) - Vector3::new(0.0, HEIGHT * 0.5, 0.0)).length()
                < 1e-6
        );
    }

    /// Tipped a quarter turn, a barrel rests on its SIDE: what holds it off
    /// the floor is the radius, and half the height would float it 0.45 m
    /// instead — the mirror image of the sinking a local lift causes.
    #[test]
    fn a_tipped_cylinder_stands_on_its_radius() {
        let tipped = turned(Vector3::BACK, 90.0);
        assert!((cylinder_underhang(tipped, RADIUS, HEIGHT) - RADIUS).abs() < 1e-6);
        // the lift is LOCAL, and it has to come out as world up
        let lift = cylinder_lift(tipped, RADIUS, HEIGHT);
        assert!((tipped * lift - Vector3::new(0.0, RADIUS, 0.0)).length() < 1e-6);
    }

    /// The law itself, on turns no designer would type and the arithmetic
    /// must survive anyway: whatever the placement, the shape's LOWEST
    /// point ends up on the node's own y — never under it (sunk) and never
    /// over it (hovering).
    #[test]
    fn a_shape_turned_any_way_puts_its_lowest_point_on_the_node() {
        for basis in [
            Basis::IDENTITY,
            turned(Vector3::BACK, 90.0),
            turned(Vector3::BACK, 37.0),
            turned(Vector3::RIGHT, 90.0),
            turned(Vector3::RIGHT, 180.0),
            turned(Vector3::new(1.0, 1.0, 1.0), 54.7),
            turned(Vector3::new(-2.0, 0.5, 1.0), 200.0),
        ] {
            let low = lowest_of_cylinder(basis, cylinder_lift(basis, RADIUS, HEIGHT));
            assert!(low.abs() < 1e-4, "cylinder rests at {low}");
            let low = lowest_of_wedge(basis, wedge_lift(basis, SIZE));
            assert!(low.abs() < 1e-5, "wedge rests at {low}");
        }
    }

    /// A shape hangs under its ancestors' transform too, and a prefab may
    /// carry a scale. The support taken through the basis is exact under
    /// one — no linearisation, no bounding box — so a scaled AND turned
    /// barrel still lands on the floor.
    #[test]
    fn a_scaled_placement_still_lands_on_the_floor() {
        for scale in [
            Vector3::new(2.0, 2.0, 2.0),
            Vector3::new(3.0, 0.5, 1.0),
            Vector3::new(0.25, 4.0, 2.0),
        ] {
            for deg in [0.0, 25.0, 90.0] {
                let basis =
                    Basis::from_diagonal(scale.x, scale.y, scale.z) * turned(Vector3::BACK, deg);
                let low = lowest_of_cylinder(basis, cylinder_lift(basis, RADIUS, HEIGHT));
                assert!(
                    low.abs() < 1e-4,
                    "cylinder rests at {low} under {scale}/{deg}"
                );
                let low = lowest_of_wedge(basis, wedge_lift(basis, SIZE));
                assert!(low.abs() < 1e-5, "wedge rests at {low} under {scale}/{deg}");
            }
        }
    }

    /// A box and a wedge take a scale whole: three components into three,
    /// along the same local axes the geometry is built on. Only magnitudes
    /// — a mirrored axis is a reflection, and no size expresses one.
    #[test]
    fn a_box_takes_a_scale_whole() {
        assert_eq!(
            fold_box_scale(Vector3::new(0.5, 0.5, 0.5), Vector3::new(4.0, 1.0, 2.0)),
            Vector3::new(2.0, 0.5, 1.0)
        );
        assert_eq!(
            fold_box_scale(Vector3::new(0.5, 0.5, 0.5), Vector3::new(-2.0, 1.0, 1.0)),
            Vector3::new(1.0, 0.5, 0.5)
        );
    }

    /// A cylinder has ONE radius against three scale components. Uniform,
    /// nothing is lost; pulled unevenly across the cross-section it is not
    /// representable at all, and the fold says so AND grows to contain
    /// what was drawn rather than shrinking inside it.
    #[test]
    fn a_cylinder_takes_the_scale_it_can_and_reports_the_rest() {
        let round = fold_column_scale(0.3, 0.9, Vector3::new(2.0, 2.0, 2.0));
        assert!((round.radius - 0.6).abs() < 1e-9);
        assert!((round.height - 1.8).abs() < 1e-9);
        assert!(round.round);

        let flat = fold_column_scale(0.3, 0.9, Vector3::new(2.0, 3.0, 1.0));
        assert!((flat.radius - 0.6).abs() < 1e-9, "took the narrower axis");
        assert!((flat.height - 2.7).abs() < 1e-9);
        assert!(!flat.round, "an elliptic cross-section reported as round");

        // a mirror is a magnitude here too, and mirroring a cylinder is
        // not a shape change at all
        let mirrored = fold_column_scale(0.3, 0.9, Vector3::new(-2.0, -2.0, -2.0));
        assert_eq!(mirrored, round);
    }

    /// The fold must not fire on the float dust a decomposed rotation
    /// leaves behind — the shipped map is 129 nodes of scale exactly 1.
    #[test]
    fn only_a_real_scale_is_worth_folding() {
        assert!(scale_is_neutral(Vector3::ONE));
        assert!(scale_is_neutral(Vector3::new(1.0, 1.0 - 1e-8, 1.0 + 1e-8)));
        assert!(!scale_is_neutral(Vector3::new(1.0, 1.0, 1.001)));
        assert!(!scale_is_neutral(Vector3::new(-1.0, -1.0, -1.0)));
    }

    /// Total on the placements a designer can reach by accident: a basis
    /// flattened to nothing has no world up to lift along, and the answer
    /// must be a finite offset rather than the infinity a plain inverse
    /// hands back.
    #[test]
    fn a_degenerate_placement_stays_finite() {
        for basis in [
            Basis::from_diagonal(0.0, 0.0, 0.0),
            Basis::from_diagonal(1.0, 0.0, 1.0),
            Basis::from_diagonal(0.0, 1.0, 0.0),
        ] {
            assert!(cylinder_lift(basis, RADIUS, HEIGHT).is_finite());
            assert!(wedge_lift(basis, SIZE).is_finite());
        }
        assert!(standing_lift(Basis::IDENTITY, f32::NAN).is_finite());
        assert_eq!(hull_underhang(Basis::IDENTITY, &[]), 0.0);
    }

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
