//! A solid's shape becomes its world-space planar faces — the geometric
//! vocabulary the whole superface campaign is built on
//! (`docs/superpowers/specs/2026-08-12-superface-outline-rendering-design.md`).
//! Downstream, the merge law asks only "are two faces coplanar, same
//! facing, and overlapping" — a question this module answers by handing
//! back a flat list of bounded polygons, each with its own outward normal
//! and signed plane offset, tagged with which solid it came from.
//!
//! Pure geometry only: plain `[f64; 3]` triples, no Godot types. The
//! boundary that turns a `Gd<Node3D>` into a [`Shape`] lives elsewhere, so
//! this module stays cargo-testable with no running Godot process.
//!
//! A column's curved flank has no plane at all and so no entry here — only
//! its two flat rims can ever coplanar-merge with anything, which is
//! exactly the property the merge law needs.

/// A solid's bounded planar face, world space.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    /// Unit outward normal, world space.
    pub normal: [f64; 3],
    /// Signed plane offset: dot(normal, p) == offset on the plane.
    pub offset: f64,
    /// The face's bounded polygon, world space, counter-clockwise seen
    /// from outside. Columns' curved flank has no entry here.
    pub poly: Vec<[f64; 3]>,
    /// Which solid this face belongs to (census index).
    pub solid: usize,
}

/// The three solid shapes the level vocabulary is built from. Each is
/// already described in world space — any node transform is folded in by
/// the caller before this point — so [`faces`] itself never fights a
/// basis stack or a scale.
pub enum Shape {
    /// center, size, basis columns (unit, possibly rotated)
    Box3d {
        center: [f64; 3],
        size: [f64; 3],
        basis: [[f64; 3]; 3],
    },
    /// wedge per prop_shape::wedge_hull's 6 points, world space
    Wedge { hull: [[f64; 3]; 6] },
    /// upright circle faces only: center, radius, half_height
    Column {
        center: [f64; 3],
        radius: f64,
        half_height: f64,
    },
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The unit direction of `v`, or nothing when `v` is too short to carry
/// one. A degenerate polygon — a collapsed box, a wedge whose hull has
/// folded to a point — must yield no face rather than a NaN normal out of
/// dividing by a near-zero length.
///
/// The guard checks `!len.is_finite()` FIRST, before the threshold — a
/// plain `len < 1e-12` alone is not NaN-total, because every ordered
/// comparison with NaN is false under IEEE 754. A NaN `len` (from a
/// NaN-poisoned input coordinate, most directly) would make `len < 1e-12`
/// false too, so that spelling alone falls through to the "valid" branch
/// and divides by NaN. `is_finite()` catches both NaN and an overflowing
/// `+Inf` length and routes them to `None` like every other length this
/// vocabulary can't carry a direction for.
fn unit(v: [f64; 3]) -> Option<[f64; 3]> {
    let len = dot(v, v).sqrt();
    if !len.is_finite() || len < 1e-12 {
        None
    } else {
        Some(scale(v, 1.0 / len))
    }
}

/// Build a face from a bounded world-space polygon whose winding already
/// carries its own outward direction: the normal is the first two edges'
/// cross product, normalized, and the offset is that normal dotted with a
/// point already known to lie on the plane (`poly[0]`, exact by
/// construction — no separate formula to drift out of sync with it). Total:
/// fewer than three points, or a polygon so collapsed its first two edges
/// are parallel or vanish, yields no face at all.
fn face_from_poly(solid: usize, poly: Vec<[f64; 3]>) -> Option<Face> {
    if poly.len() < 3 {
        return None;
    }
    let normal = unit(cross(sub(poly[1], poly[0]), sub(poly[2], poly[0])))?;
    let offset = dot(normal, poly[0]);
    Some(Face {
        normal,
        offset,
        poly,
        solid,
    })
}

/// Each face's four corners as ±1 multiples along the box's own LOCAL
/// axes, order −X,+X,−Y,+Y,−Z,+Z, wound so consecutive corners' cross
/// product already points along that face's own local outward direction —
/// hand-derived the same way `render::paint::FACE_CORNERS` derives its
/// own, independently, because the two tables solve different problems:
/// this one is later carried into an arbitrary basis, paint bakes a fixed
/// axis-aligned mesh.
const BOX_FACE_CORNERS: [[[f64; 3]; 4]; 6] = [
    // -X
    [
        [-1.0, -1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [-1.0, 1.0, 1.0],
        [-1.0, 1.0, -1.0],
    ],
    // +X
    [
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [1.0, 1.0, 1.0],
        [1.0, -1.0, 1.0],
    ],
    // -Y
    [
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, -1.0, 1.0],
        [-1.0, -1.0, 1.0],
    ],
    // +Y
    [
        [-1.0, 1.0, -1.0],
        [-1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, -1.0],
    ],
    // -Z
    [
        [-1.0, -1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [1.0, 1.0, -1.0],
        [1.0, -1.0, -1.0],
    ],
    // +Z
    [
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ],
];

/// A box's six faces: each is the local axis-aligned quad above, mapped
/// into world space by the basis columns and half-extent, then handed to
/// [`face_from_poly`] to recover its own normal and offset independently
/// rather than trusting the local table's comment. A rotation of
/// determinant +1 — every basis this vocabulary is fed — preserves the
/// cross product's orientation exactly, so the recovered normal lands on
/// the same basis column the local table names; the quarter-turn test
/// below proves that rather than assuming it.
fn box_faces(solid: usize, center: [f64; 3], size: [f64; 3], basis: [[f64; 3]; 3]) -> Vec<Face> {
    let half = [
        size[0].abs() * 0.5,
        size[1].abs() * 0.5,
        size[2].abs() * 0.5,
    ];
    BOX_FACE_CORNERS
        .iter()
        .filter_map(|corners| {
            let poly = corners
                .iter()
                .map(|c| {
                    let local = [c[0] * half[0], c[1] * half[1], c[2] * half[2]];
                    add(
                        center,
                        add(
                            add(scale(basis[0], local[0]), scale(basis[1], local[1])),
                            scale(basis[2], local[2]),
                        ),
                    )
                })
                .collect();
            face_from_poly(solid, poly)
        })
        .collect()
}

/// A wedge's five faces, indexed straight off `prop_shape::wedge_hull`'s
/// own point order — the four floor corners first, then the two tall-edge
/// corners — as the floor, the tall back wall, the sloped face, and the
/// two triangular ends. Each polygon's winding is hand-derived from that
/// same order (see the module tests), so no separate basis argument is
/// needed here: the hull already lives in world space, and a proper
/// rotation cannot flip a winding that was correct before it was applied.
fn wedge_faces(solid: usize, hull: [[f64; 3]; 6]) -> Vec<Face> {
    let [a, b, d, e, c, f] = hull;
    [
        vec![a, b, e, d], // floor
        vec![b, c, f, e], // tall back wall
        vec![a, d, f, c], // slope
        vec![a, c, b],    // -Z triangular end
        vec![d, e, f],    // +Z triangular end
    ]
    .into_iter()
    .filter_map(|poly| face_from_poly(solid, poly))
    .collect()
}

/// How many points a circle rim discretizes to. The merge law only ever
/// needs polygon overlap tests, never a curvature — rims merge with slab
/// tops when a column stands flush, which is a legal melt, not a hazard
/// this count needs to guard against.
const RIM_SEGMENTS: usize = 32;

/// A column's two flat rims — the only planar faces it has, since its
/// curved flank carries no plane and so can never coplanar-merge with
/// anything. The bottom rim (outward normal −Y) walks increasing angle;
/// the top rim (outward normal +Y) walks decreasing angle. A circle's own
/// curvature turns the same way relative to increasing angle no matter
/// which end caps it, so the two rims need OPPOSITE walking directions to
/// both wind outward — proved by the module's winding test, not assumed.
fn column_faces(solid: usize, center: [f64; 3], radius: f64, half_height: f64) -> Vec<Face> {
    let r = radius.abs();
    let hh = half_height.abs();
    let ring = |y: f64, reverse: bool| -> Vec<[f64; 3]> {
        (0..RIM_SEGMENTS)
            .map(|i| {
                let step = if reverse { RIM_SEGMENTS - 1 - i } else { i };
                let theta = step as f64 * std::f64::consts::TAU / RIM_SEGMENTS as f64;
                [center[0] + r * theta.cos(), y, center[2] + r * theta.sin()]
            })
            .collect()
    };
    let bottom = ring(center[1] - hh, false);
    let top = ring(center[1] + hh, true);
    let bottom_normal = [0.0, -1.0, 0.0];
    let top_normal = [0.0, 1.0, 0.0];
    vec![
        Face {
            normal: bottom_normal,
            offset: dot(bottom_normal, bottom[0]),
            poly: bottom,
            solid,
        },
        Face {
            normal: top_normal,
            offset: dot(top_normal, top[0]),
            poly: top,
            solid,
        },
    ]
}

/// A solid's shape decomposed into its world-space planar faces — the one
/// entry point every downstream stage (the merge law, the label
/// colouring) consumes.
pub fn faces(solid: usize, shape: &Shape) -> Vec<Face> {
    match shape {
        Shape::Box3d {
            center,
            size,
            basis,
        } => box_faces(solid, *center, *size, *basis),
        Shape::Wedge { hull } => wedge_faces(solid, *hull),
        Shape::Column {
            center,
            radius,
            half_height,
        } => column_faces(solid, *center, *radius, *half_height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit box at the origin yields six faces whose normals are the six
    /// axis directions and whose offsets are ±0.5 — the break this catches
    /// is a face built from the wrong basis column or a sign slip.
    #[test]
    fn a_box_yields_six_outward_faces() {
        let f = faces(
            0,
            &Shape::Box3d {
                center: [0.0; 3],
                size: [1.0; 3],
                basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            },
        );
        assert_eq!(f.len(), 6);
        let px = f.iter().find(|f| f.normal == [1.0, 0.0, 0.0]).unwrap();
        assert!((px.offset - 0.5).abs() < 1e-12);
        assert_eq!(px.poly.len(), 4);
    }

    /// A column contributes only its two rims: the curved flank has no
    /// plane and can never coplanar-merge with anything.
    #[test]
    fn a_column_contributes_only_its_rims() {
        let f = faces(
            3,
            &Shape::Column {
                center: [1.0, 0.5, 2.0],
                radius: 0.3,
                half_height: 0.5,
            },
        );
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].normal, [0.0, -1.0, 0.0]);
        assert!((f[0].offset - 0.0).abs() < 1e-12); // dot((0,-1,0),(x,0,z))
        assert_eq!(f[1].normal, [0.0, 1.0, 0.0]);
    }

    /// A wedge yields five faces (two triangles, three quads) built from
    /// its hull points; the diagonal face's normal is not axis-aligned.
    /// The hull literals are hand-derived from `prop_shape::wedge_hull`'s
    /// own formula (`h = size.abs() * 0.5`) at that file's own test
    /// fixture size (1.2, 0.6, 0.8) — never by calling `faces` or any of
    /// its neighbours to produce the expectation.
    #[test]
    fn a_wedge_yields_five_faces_including_the_diagonal() {
        // wedge_hull((1.2, 0.6, 0.8)): h = (0.6, 0.3, 0.4)
        let hull = [
            [-0.6, -0.3, -0.4], // A: low edge, -Z
            [0.6, -0.3, -0.4],  // B: tall edge foot, -Z
            [-0.6, -0.3, 0.4],  // D: low edge, +Z
            [0.6, -0.3, 0.4],   // E: tall edge foot, +Z
            [0.6, 0.3, -0.4],   // C: tall edge top, -Z
            [0.6, 0.3, 0.4],    // F: tall edge top, +Z
        ];
        let f = faces(1, &Shape::Wedge { hull });
        assert_eq!(f.len(), 5);

        let triangles: Vec<_> = f.iter().filter(|f| f.poly.len() == 3).collect();
        let quads: Vec<_> = f.iter().filter(|f| f.poly.len() == 4).collect();
        assert_eq!(triangles.len(), 2);
        assert_eq!(quads.len(), 3);

        // the two triangular ends: ∓Z, axis-aligned. Approximate, not
        // exact: unlike the box tests' power-of-two extents, 0.6 and 1.2
        // are not binary-exact, so sqrt(x*x) lands a float's breadth off
        // |x| rather than bit-for-bit on it.
        let mut tri_normals: Vec<[f64; 3]> = triangles.iter().map(|f| f.normal).collect();
        tri_normals.sort_by(|a, b| a[2].partial_cmp(&b[2]).unwrap());
        for (got, want) in tri_normals.iter().zip([[0.0, 0.0, -1.0], [0.0, 0.0, 1.0]]) {
            for i in 0..3 {
                assert!((got[i] - want[i]).abs() < 1e-9, "{got:?} != {want:?}");
            }
        }

        // the floor and the tall back wall are axis-aligned quads
        assert!(quads.iter().any(|f| f.normal == [0.0, -1.0, 0.0]));
        assert!(quads.iter().any(|f| f.normal == [1.0, 0.0, 0.0]));

        // the diagonal — the slope — is the one quad with neither the x
        // nor the y component zero: hand-derived from
        // prop_shape::wedge_slope_normal's own formula,
        // normalize((-size.y.abs(), size.x.abs(), 0)) = normalize((-0.6, 1.2, 0))
        let slope = quads
            .iter()
            .find(|f| f.normal[0].abs() > 1e-9 && f.normal[1].abs() > 1e-9)
            .expect("no diagonal face found");
        let len = (0.6_f64 * 0.6 + 1.2 * 1.2).sqrt();
        assert!((slope.normal[0] - (-0.6 / len)).abs() < 1e-9);
        assert!((slope.normal[1] - (1.2 / len)).abs() < 1e-9);
        assert!((slope.normal[2] - 0.0).abs() < 1e-12);
    }

    /// A box under a quarter-turn basis yields the same six planes with
    /// swapped axes — exact, no trig dust: `quadrant_basis(1)`'s columns
    /// (`rust/src/level_plan.rs`) are unit 0/±1 by construction, hand-
    /// derived here rather than called, since this module never depends on
    /// Godot's `Basis` type. Local +X (half-extent 1.0) becomes world −Z;
    /// local +Z (half-extent 2.0) becomes world +X; local Y, untouched by
    /// a yaw, is exactly where it always was.
    #[test]
    fn a_quarter_turned_box_swaps_axes_exactly() {
        // quadrant_basis(1): X-col (0,0,-1), Y-col (0,1,0), Z-col (1,0,0)
        let basis = [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let f = faces(
            2,
            &Shape::Box3d {
                center: [0.0; 3],
                size: [2.0, 1.0, 4.0],
                basis,
            },
        );
        assert_eq!(f.len(), 6);

        let world_neg_z = f.iter().find(|f| f.normal == [0.0, 0.0, -1.0]).unwrap();
        assert!((world_neg_z.offset - 1.0).abs() < 1e-12);
        assert_eq!(world_neg_z.solid, 2);

        let world_pos_x = f.iter().find(|f| f.normal == [1.0, 0.0, 0.0]).unwrap();
        assert!((world_pos_x.offset - 2.0).abs() < 1e-12);

        let world_pos_y = f.iter().find(|f| f.normal == [0.0, 1.0, 0.0]).unwrap();
        assert!((world_pos_y.offset - 0.5).abs() < 1e-12);
    }

    /// Every emitted polygon winds counter-clockwise as seen from OUTSIDE
    /// its own normal — the contract `Face::poly`'s doc comment states.
    ///
    /// This is a GENUINE independent check only for the column: its rims'
    /// normal is a fixed constant (`column_faces` hardcodes `[0,-1,0]`/
    /// `[0,1,0]`), entirely independent of the ring's own point order, so
    /// a swapped walking direction on either rim is a real bug this test
    /// can catch. Confirmed by mutation, not assumed: forcing the top rim
    /// to walk the same direction as the bottom fails ONLY here, not in
    /// `a_column_contributes_only_its_rims` (recorded in the task report).
    ///
    /// For Box3d and Wedge it is NOT such a check, and must not be read as
    /// one: `face_from_poly` derives `normal` FROM this exact `poly` (the
    /// first two edges' own cross product), so `dot(cross(e1, e2), normal)
    /// > 0` holds BY CONSTRUCTION for any point order `face_from_poly` was
    /// given — a transcribed-backwards row in `BOX_FACE_CORNERS`, or a
    /// scrambled wedge polygon, still passes this test, because the
    /// normal simply flips to agree with whatever order it was handed.
    /// Also confirmed by mutation: a bowtie-scrambled wedge floor row
    /// passes this test unchanged. That class of bug is instead caught by
    /// tests that check a face's normal against an independently derived
    /// expectation rather than trusting the face's own construction:
    /// `a_wedge_yields_five_faces_including_the_diagonal` for the wedge,
    /// and `every_box_face_normal_matches_its_own_position` for the box.
    #[test]
    fn every_emitted_face_winds_outward() {
        let mut all = faces(
            0,
            &Shape::Box3d {
                center: [0.3, -0.2, 1.1],
                size: [2.0, 1.0, 0.5],
                basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            },
        );
        all.extend(faces(
            1,
            &Shape::Column {
                center: [0.4, 0.1, -0.6],
                radius: 0.4,
                half_height: 0.6,
            },
        ));
        all.extend(faces(
            2,
            &Shape::Wedge {
                hull: [
                    [-0.6, -0.3, -0.4],
                    [0.6, -0.3, -0.4],
                    [-0.6, -0.3, 0.4],
                    [0.6, -0.3, 0.4],
                    [0.6, 0.3, -0.4],
                    [0.6, 0.3, 0.4],
                ],
            },
        ));
        assert!(!all.is_empty());
        for face in all {
            let e1 = sub(face.poly[1], face.poly[0]);
            let e2 = sub(face.poly[2], face.poly[0]);
            let c = cross(e1, e2);
            assert!(
                dot(c, face.normal) > 0.0,
                "face normal {:?} not outward for poly {:?}",
                face.normal,
                face.poly
            );
        }
    }

    /// Every one of a box's six faces has the normal its own GEOMETRIC
    /// POSITION implies — derived from where the face's polygon actually
    /// sits (its centroid's offset along each basis axis), never from
    /// `face.normal` itself. A centroid is invariant under any reordering
    /// of the same four corners, so unlike `every_emitted_face_winds_outward`
    /// (which recomputes the very cross product `face_from_poly` used to
    /// build the normal, and so cannot tell a correct row from a
    /// transcribed-backwards one — see that test's doc comment) this one
    /// can: a scrambled `BOX_FACE_CORNERS` row still puts its four points
    /// on the right plane, so the centroid still names the right face,
    /// even though the wrong-order cross product flips which way
    /// `face.normal` points. Run at two bases — identity and a quarter
    /// turn — so all six local rows, negative ones included, get checked:
    /// the two brief-given box tests together only ever inspect four of
    /// the six (+X plain, and −Z/+X/+Y turned).
    #[test]
    fn every_box_face_normal_matches_its_own_position() {
        let center = [0.3, -0.1, 0.4];
        let size = [2.0, 1.0, 4.0];
        let half = [1.0, 0.5, 2.0];
        for basis in [
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            // quadrant_basis(1): X-col (0,0,-1), Y-col (0,1,0), Z-col (1,0,0)
            [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
        ] {
            let f = faces(
                0,
                &Shape::Box3d {
                    center,
                    size,
                    basis,
                },
            );
            assert_eq!(f.len(), 6);
            for face in &f {
                let n = face.poly.len() as f64;
                let sum = face.poly.iter().fold([0.0; 3], |acc, p| add(acc, *p));
                let centroid = scale(sum, 1.0 / n);
                let rel = sub(centroid, center);
                let (axis, sign) = (0..3)
                    .map(|a| (a, dot(rel, basis[a]) / half[a]))
                    .find(|&(_, t)| (t.abs() - 1.0).abs() < 1e-9)
                    .map(|(a, t)| (a, t.signum()))
                    .unwrap_or_else(|| panic!("centroid {centroid:?} not on any face plane"));
                let expected = scale(basis[axis], sign);
                assert_eq!(
                    face.normal, expected,
                    "face at centroid {centroid:?} (basis {basis:?})"
                );
            }
        }
    }

    /// A box collapsed to zero size has no faces to draw — total rather
    /// than six zero-area quads carrying a NaN normal out of a zero-length
    /// cross product.
    #[test]
    fn a_collapsed_box_yields_no_faces() {
        let f = faces(
            0,
            &Shape::Box3d {
                center: [0.0; 3],
                size: [0.0; 3],
                basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            },
        );
        assert!(f.is_empty());
    }

    /// A wedge whose hull has folded to a single point yields no faces
    /// either, for the same reason: never a NaN normal.
    #[test]
    fn a_collapsed_wedge_yields_no_faces() {
        let p = [1.0, 2.0, 3.0];
        let f = faces(0, &Shape::Wedge { hull: [p; 6] });
        assert!(f.is_empty());
    }

    /// A NaN-poisoned center still yields no faces. The break this catches
    /// is a `unit()` guard written as a plain `len < eps`: under IEEE 754
    /// `NaN < eps` is FALSE (every ordered comparison with NaN is false),
    /// so a naive guard falls through to the "valid" branch and hands back
    /// six faces carrying a `[NaN, NaN, NaN]` normal instead of an empty
    /// list — the exact failure mode `unit()`'s own doc comment promises
    /// can't happen.
    #[test]
    fn a_nan_centered_box_yields_no_faces() {
        let f = faces(
            0,
            &Shape::Box3d {
                center: [f64::NAN, 0.0, 0.0],
                size: [1.0; 3],
                basis: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            },
        );
        assert!(f.is_empty());
    }

    /// `face_from_poly` itself refuses fewer than three points — the
    /// shared guard every shape funnels through. Tested directly, since
    /// none of the three `Shape` variants can hand it a 1- or 2-point
    /// polygon to exercise the guard from the public entry point.
    #[test]
    fn face_from_poly_refuses_fewer_than_three_points() {
        assert!(face_from_poly(0, vec![]).is_none());
        assert!(face_from_poly(0, vec![[0.0, 0.0, 0.0]]).is_none());
        assert!(face_from_poly(0, vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]]).is_none());
    }

    /// A negative size behaves exactly like its magnitude — a designer's
    /// minus sign flips nothing, the same law `prop_shape::wedge_hull`
    /// states for the same case.
    #[test]
    fn a_box_with_negative_size_is_the_same_box() {
        let basis = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let a = faces(
            0,
            &Shape::Box3d {
                center: [0.0; 3],
                size: [1.0, -2.0, 3.0],
                basis,
            },
        );
        let b = faces(
            0,
            &Shape::Box3d {
                center: [0.0; 3],
                size: [1.0, 2.0, 3.0],
                basis,
            },
        );
        assert_eq!(a, b);
    }

    /// Circle rims discretize to 32 points, named here so a change to
    /// `RIM_SEGMENTS` is a deliberate, reviewed edit rather than a silent
    /// drift the merge law downstream would never notice.
    #[test]
    fn a_column_rim_discretizes_to_32_points() {
        let f = faces(
            0,
            &Shape::Column {
                center: [0.0; 3],
                radius: 0.5,
                half_height: 1.0,
            },
        );
        assert_eq!(f[0].poly.len(), 32);
        assert_eq!(f[1].poly.len(), 32);
    }
}
