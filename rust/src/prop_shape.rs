//! The shapes a prop can be, as pure geometry. The world was boxes only
//! for as long as it had one prop class; a contours-only world reads a
//! curve and a slope completely differently from a box, so the vocabulary
//! is three now — box, column, wedge — and the two that need real geometry
//! are built here where cargo can hold them.
//!
//! Only the WEDGE needs generated triangles: a box and a cylinder are
//! engine primitives with engine colliders, but a triangular prism is
//! neither. Its faces are emitted with explicit normals and separate face
//! vertices so the derive-time paint pass can assign each planar face its
//! own CUSTOM0 label. A wedge with smeared/shared face data would lose the
//! slope boundary the outline must preserve.
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
    /// [`column_triangles`]' circular ring nor `CylinderShape3D` can be,
    /// and one this vocabulary deliberately does not own.
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
/// derivation and per-face paint census need no geometric exception for a
/// wedge beyond selecting this explicit face layout.
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

/// Which of the wedge's five faces each of [`wedge_triangles`]'s eight
/// triangles belongs to — a CUSTOM0 ordinal per triangle, in
/// `render::faces::wedge_faces`'s own emission order: floor (0), tall
/// back wall (1), slope (2), −Z triangular end (3), +Z triangular end
/// (4). The grouping mirrors [`wedge_triangles`]'s own triangle order
/// exactly (compare that function's body against `wedge_faces`'s five
/// polygons), so this table is the one place the correspondence is named
/// rather than re-derived at every call site that paints a wedge.
pub const WEDGE_TRIANGLE_ORDINALS: [f32; 8] = [0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 3.0, 4.0];

/// How many segments a column's rims and flank discretize into — pinned
/// equal to `render::faces::RIM_SEGMENTS` so the mesh a designer sees and
/// the polygon the merge law reasons about describe the same circle.
pub const COLUMN_SEGMENTS: usize = 32;

/// A column's surface as `(position, normal, CUSTOM0 ordinal)` triples,
/// local space, centered on the node with `half_height` above and below
/// it: the bottom rim (ordinal 0, outward normal DOWN), the top rim
/// (ordinal 1, outward normal UP) — matching
/// `render::faces::column_faces`'s own bottom-then-top order — and the
/// curved flank (ordinal 2), which has no plane and so no entry in
/// `faces()` at all. Each rim is a triangle fan from its own center, flat
/// shaded; the flank's radial normal varies smoothly per vertex, the way
/// the `CylinderMesh` primitive this replaces shaded it.
///
/// Every triangle's winding is hand-derived: a fan walking increasing
/// angle points DOWN, the mirrored walk points UP, and a flank quad wound
/// `(bottom[i], top[i], top[i+1])` / `(bottom[i], top[i+1], bottom[i+1])`
/// points radially outward.
///
/// The module's two winding tests hold that derivation to THIS function's
/// own output — every segment, all four triangles of it, cross-producing
/// the emitted vertices rather than a literal transcribed from this
/// comment. That is worth stating because it was not true until a review
/// found `column_caps_wind_outward` cross-producing a hand-typed pair of
/// triangles and never calling `column_triangles` at all: the caps'
/// winding was unprotected, and it is not cosmetic — the world skin
/// renders `cull_disabled`, but `nodes::fan`/`nodes::radio`'s
/// `labelled_cyl` builds every source limb from this geometry and renders
/// it through `data_xray.gdshader`, which is `cull_back`.
#[must_use]
pub fn column_triangles(radius: f32, half_height: f32) -> Vec<(Vector3, Vector3, f32)> {
    let n = COLUMN_SEGMENTS;
    let ring = |y: f32| -> Vec<Vector3> {
        (0..n)
            .map(|i| {
                let theta = i as f32 * std::f32::consts::TAU / n as f32;
                Vector3::new(radius * theta.cos(), y, radius * theta.sin())
            })
            .collect()
    };
    let bottom = ring(-half_height);
    let top = ring(half_height);
    let radial = |i: usize| -> Vector3 {
        let theta = i as f32 * std::f32::consts::TAU / n as f32;
        Vector3::new(theta.cos(), 0.0, theta.sin())
    };

    let mut out = Vec::with_capacity(n * 12);
    let center_bottom = Vector3::new(0.0, -half_height, 0.0);
    let center_top = Vector3::new(0.0, half_height, 0.0);
    for i in 0..n {
        let j = (i + 1) % n;
        // bottom cap: increasing angle winds outward (−Y) — see the
        // module test `column_caps_wind_outward`
        out.push((center_bottom, Vector3::DOWN, 0.0));
        out.push((bottom[i], Vector3::DOWN, 0.0));
        out.push((bottom[j], Vector3::DOWN, 0.0));
        // top cap: the mirrored walk winds outward (+Y)
        out.push((center_top, Vector3::UP, 1.0));
        out.push((top[j], Vector3::UP, 1.0));
        out.push((top[i], Vector3::UP, 1.0));
        // flank: two triangles per segment, radial outward normal
        let (n_i, n_j) = (radial(i), radial(j));
        out.push((bottom[i], n_i, 2.0));
        out.push((top[i], n_i, 2.0));
        out.push((top[j], n_j, 2.0));
        out.push((bottom[i], n_i, 2.0));
        out.push((top[j], n_j, 2.0));
        out.push((bottom[j], n_j, 2.0));
    }
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
    /// it with — which is what lets the level derive its world box for the
    /// touch/separation graph exactly as it does for a plain box prop.
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

    /// The ordinal table has one entry per [`wedge_triangles`] triangle,
    /// grouped 2/2/2/1/1 — floor, tall back wall and slope get two
    /// triangles each (they are quads), the two triangular ends get one —
    /// and every ordinal is inside the wedge's own five-face count
    /// (`render::paint::face_count(ShapeKind::Wedge)`, hand-mirrored here
    /// as 5 rather than imported, so this pure module stays decoupled
    /// from `render`).
    #[test]
    fn wedge_ordinals_group_2_2_2_1_1_and_stay_inside_five_faces() {
        assert_eq!(
            WEDGE_TRIANGLE_ORDINALS.len(),
            wedge_triangles(SIZE).len() / 3
        );
        let mut counts = [0u8; 5];
        for &ord in &WEDGE_TRIANGLE_ORDINALS {
            assert!(
                (0.0..5.0).contains(&ord),
                "ordinal {ord} outside the wedge's 5 faces"
            );
            counts[ord as usize] += 1;
        }
        assert_eq!(counts, [2, 2, 2, 1, 1]);
    }

    /// `column_triangles` at 32 segments: 12 vertices per segment (a
    /// bottom-cap triangle, a top-cap triangle, two flank triangles, three
    /// vertices each), hand-derived from the loop structure rather than
    /// read off the function's own length.
    #[test]
    fn column_triangles_count_matches_the_segment_loop() {
        let tris = column_triangles(RADIUS, HEIGHT * 0.5);
        assert_eq!(tris.len(), COLUMN_SEGMENTS * 12);
    }

    /// Every vertex the column emits is finite and every normal is a unit
    /// vector — a non-unit normal packs a wrong crease id, and a NaN
    /// coordinate poisons everything downstream.
    #[test]
    fn every_column_vertex_is_finite_with_a_unit_normal() {
        for (v, n, _) in column_triangles(RADIUS, HEIGHT * 0.5) {
            assert!(v.is_finite(), "vertex not finite: {v}");
            assert!(n.is_finite(), "normal not finite: {n}");
            assert!((n.length() - 1.0).abs() < 1e-5, "normal not unit: {n}");
        }
    }

    /// The three ordinals land where the doc comment says: bottom rim 0,
    /// top rim 1, flank 2 — and nowhere past 3, the column's own face
    /// count (`render::paint::face_count(ShapeKind::Column)`, hand-mirrored
    /// as 3 for the same decoupling reason as the wedge test above).
    /// Every bottom-rim vertex sits at y = −half_height, every top-rim
    /// vertex at +half_height, and every flank vertex spans both — a
    /// property read from the vertex's OWN y, independent of which
    /// ordinal the triangle carries, so a swapped 0/1 label would fail
    /// this rather than pass by agreeing with itself.
    #[test]
    fn column_ordinals_match_bottom_top_and_flank() {
        let half = HEIGHT * 0.5;
        for (v, _, ord) in column_triangles(RADIUS, half) {
            assert!(
                (0.0..3.0).contains(&ord),
                "ordinal {ord} outside the column's 3 faces"
            );
            if ord == 0.0 {
                assert!(
                    (v.y - -half).abs() < 1e-5,
                    "bottom-rim vertex not at −half: {v}"
                );
            } else if ord == 1.0 {
                assert!(
                    (v.y - half).abs() < 1e-5,
                    "top-rim vertex not at +half: {v}"
                );
            }
        }
    }

    /// Every EMITTED bottom-cap triangle winds toward −Y (outward, seen
    /// from below) and every emitted top-cap triangle toward +Y — the
    /// fan's mirrored walk, recomputed from each triangle's own cross
    /// product rather than trusted from the doc comment that derives it.
    ///
    /// Driven off `column_triangles`, which is the whole point: this test
    /// used to cross-product a hand-typed pair of literal triangles and
    /// never call the function at all, so swapping the emitted caps' push
    /// order passed it and the entire rest of the suite. The stakes are
    /// not only the level's props — `nodes::fan`/`nodes::radio`'s
    /// `labelled_cyl` builds every source limb from this same geometry and
    /// renders it through `data_xray.gdshader`, whose `cull_back` would
    /// make an inverted cap vanish out of the acoustic image entirely.
    ///
    /// Every segment, not just one: the ring walks all four quadrants, and
    /// a sign slip that only bites where a cosine turns negative is
    /// exactly the kind one sample misses.
    #[test]
    fn column_caps_wind_outward() {
        let tris = column_triangles(1.0, 1.0);
        assert_eq!(tris.len(), COLUMN_SEGMENTS * 12);
        for seg in 0..COLUMN_SEGMENTS {
            // each segment emits one 12-vertex block: bottom cap, top cap,
            // then the flank's two triangles
            let base = seg * 12;
            for (first, want) in [(base, Vector3::DOWN), (base + 3, Vector3::UP)] {
                let (v0, v1, v2) = (tris[first].0, tris[first + 1].0, tris[first + 2].0);
                let cross = (v1 - v0).cross(v2 - v0);
                assert!(
                    cross.dot(want) > 0.0,
                    "segment {seg}'s cap triangle {v0:?},{v1:?},{v2:?} does not wind toward \
                     {want:?} (cross {cross:?})"
                );
            }
        }
    }

    /// Both of every segment's flank triangles wind radially outward — the
    /// same independent cross-product check, over the whole ring rather
    /// than the single first-segment sample this used to take.
    ///
    /// The outward direction is read off the triangle's OWN first vertex,
    /// which `column_triangles` always emits as that segment's bottom-rim
    /// point: its horizontal offset from the axis is the outward direction
    /// there, by definition of a ring centred on the axis. Nothing about
    /// it depends on the winding under test, and no angle is recomputed
    /// from the code's own `TAU / n` step.
    #[test]
    fn column_flank_winds_radially_outward() {
        let tris = column_triangles(1.0, 1.0);
        for seg in 0..COLUMN_SEGMENTS {
            let base = seg * 12;
            for first in [base + 6, base + 9] {
                let (v0, v1, v2) = (tris[first].0, tris[first + 1].0, tris[first + 2].0);
                let cross = (v1 - v0).cross(v2 - v0);
                let outward = Vector3::new(v0.x, 0.0, v0.z);
                assert!(
                    cross.dot(outward) > 0.0,
                    "segment {seg}'s flank triangle {v0:?},{v1:?},{v2:?} does not wind outward \
                     (cross {cross:?}, outward {outward:?})"
                );
            }
        }
    }

    /// Total on the shapes a designer can type by accident: zero radius or
    /// height still yields a finite, if degenerate, mesh.
    #[test]
    fn degenerate_columns_stay_finite() {
        for (r, h) in [(0.0, 0.5), (0.3, 0.0), (0.0, 0.0)] {
            for (v, n, ord) in column_triangles(r, h) {
                assert!(v.is_finite(), "vertex not finite for r={r} h={h}: {v}");
                assert!(n.is_finite(), "normal not finite for r={r} h={h}: {n}");
                assert!((0.0..3.0).contains(&ord));
            }
        }
    }
}
