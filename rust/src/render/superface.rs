//! The merge law: which faces become ONE superface, and which pairs of
//! superfaces must take separated labels
//! (`docs/superpowers/specs/2026-08-12-superface-outline-rendering-design.md`).
//! This is the module the issue-14 z-fight dies to — two overlapping
//! solids' coplanar same-facing faces are unioned into one class before a
//! label is ever handed out, so the two writers of any pixel their old
//! geometry would have fought over hold **bit-identical G by
//! construction**, not by a shader tie-break.
//!
//! # The law
//!
//! - **Merge edge** (two faces become one class): normals parallel and
//!   SAME direction (dot within [`PARALLEL_EPS`] of `+1.0`), plane offsets
//!   within [`COPLANAR_EPS`], and — after projecting both polygons to the
//!   shared plane's own two tangent axes — their exact convex intersection
//!   exceeds [`PATCH_EPS`] in BOTH tangent extents. An edge-only touch (one
//!   axis of the intersection has zero width) is a bend, not a melt.
//! - **Separation edge** (two classes must take labels ≥ `MIN_SEP` apart,
//!   `labels::MIN_SEP`, this module only builds the graph): three rules,
//!   checked independently —
//!   (a) two faces of ONE solid sharing a polygon edge — a box's own
//!   corner never disappears, merged cluster or not;
//!   (b) faces of two DIFFERENT, TOUCHING solids that ended up in the
//!   SAME cluster (merged via some other face pair) whose polygons pass
//!   within [`PATCH_EPS`] of each other in 3-D, excluding pairs already
//!   merged and excluding a BURIED ABUTMENT — opposite-facing coplanar
//!   contact, a crate's underside on the floor it stands on;
//!   (c) every face pair between two touching solids that never merged at
//!   all (different clusters) — the old per-solid law, blanket-applied:
//!   all of one solid's classes separate from all of the other's.
//!
//! # Determinism
//!
//! Every pass iterates faces and solids in the order the caller handed
//! them; class numbers are assigned by FIRST APPEARANCE in that order
//! (never by the union-find's own internal root numbering, which is an
//! implementation detail of union-by-rank and not part of this module's
//! contract). No hashing, no `HashMap` iteration reaches the output.

use super::faces::Face;

/// Faces whose planes sit within this of each other are coplanar,
/// INCLUSIVE — promoted unchanged from the fight census's own derivation
/// (`observe::oids::COPLANAR_EPS`): one 24-bit depth code spans about
/// 1.191e-6·w² m at eye distance w, and the shipped map's longest
/// sightline (34 m, under the 40 m pack-range ceiling) never needs more
/// than 2 mm of slack to keep every same-facing coincidence inside one
/// depth code.
pub const COPLANAR_EPS: f64 = 2e-3;

/// Two polygons' tangent-plane intersection must exceed this — EXCLUSIVE —
/// along BOTH principal extents to count as an overlapping patch rather
/// than a bare edge. Also the threshold [`polygons_within_patch_eps`] uses
/// for a general 3-D closeness test between non-coplanar faces (rule (b)):
/// promoted unchanged from `observe::oids::PATCH_EPS`.
pub const PATCH_EPS: f64 = 1e-3;

/// Normals within this dot-product distance of exactly parallel (`+1.0`
/// same direction, `-1.0` opposite) are treated as identically facing.
/// Axis-aligned and quadrant-rotated geometry lands on these exactly;
/// arbitrarily rotated props carry the float error a full 3-vector cross
/// product picks up, which never approaches this tolerance in practice.
const PARALLEL_EPS: f64 = 1e-9;

/// Two points closer than this are the SAME construction-time vertex —
/// tight, because a shared box/wedge corner is computed by the identical
/// arithmetic expression twice (same basis, same half-extent, same
/// center), which lands bit-identical on this toolchain; the slack exists
/// only to absorb a differently-ORDERED but algebraically equal
/// evaluation, never a genuinely different point.
const EDGE_POINT_EPS: f64 = 1e-6;

/// The merge law and the separation graph over one set of faces.
pub struct Superfaces {
    /// face index -> class index, normalized to first-appearance order in
    /// the input `faces` slice.
    pub class_of: Vec<usize>,
    /// The number of distinct classes (`class_of`'s values span
    /// `0..classes`).
    pub classes: usize,
    /// Class pairs that must take labels `labels::MIN_SEP` apart, each
    /// stored once as `(min, max)` and deduplicated.
    pub separations: Vec<(usize, usize)>,
    /// Solid index -> cluster index: the connected components of the
    /// MERGE relation lifted to solids (two solids share a cluster iff
    /// some face of one merged with some face of the other, possibly
    /// transitively). Sized to `max(face.solid) + 1`; a solid that
    /// contributed no face to this call has no entry. Used by paint's
    /// wall-merge warning and by this module's own tests.
    pub cluster_of_solid: Vec<usize>,
}

/// Build the superface graph: which of `faces` merge into one class, and
/// which resulting classes must separate. `touching` is the solid-level
/// touch relation as the caller already computed it (e.g.
/// `oid_palette::Box3::touches`) — this module never recomputes touch, it
/// only consumes the pairs, in the order given.
///
/// Total for every input: an empty `faces` yields an empty, zero-class
/// [`Superfaces`]; a `touching` pair naming a solid index with no faces in
/// this call, or a self-pair, is silently skipped rather than panicking.
pub fn superfaces(faces: &[Face], touching: &[(usize, usize)]) -> Superfaces {
    let n = faces.len();

    // --- the merge pass: union-find over FACES ---
    let mut face_uf = UnionFind::new(n);
    for i in 0..n {
        for j in (i + 1)..n {
            if is_merge_candidate(&faces[i], &faces[j]) {
                face_uf.union(i, j);
            }
        }
    }
    let (class_of, classes) = normalize(&mut face_uf, n);

    // --- solid-level clusters: the merge relation lifted to solids ---
    let solid_count = faces.iter().map(|f| f.solid).max().map_or(0, |m| m + 1);
    let mut solid_uf = UnionFind::new(solid_count);
    for i in 0..n {
        for j in (i + 1)..n {
            if class_of[i] == class_of[j] && faces[i].solid != faces[j].solid {
                solid_uf.union(faces[i].solid, faces[j].solid);
            }
        }
    }
    let (cluster_of_solid, _clusters) = normalize(&mut solid_uf, solid_count);

    // faces grouped by solid, input order preserved within each group
    let mut faces_of_solid: Vec<Vec<usize>> = vec![Vec::new(); solid_count];
    for (i, f) in faces.iter().enumerate() {
        faces_of_solid[f.solid].push(i);
    }

    let mut separations: Vec<(usize, usize)> = Vec::new();

    // rule (a): two faces of ONE solid sharing a polygon edge
    for i in 0..n {
        for j in (i + 1)..n {
            if faces[i].solid == faces[j].solid
                && class_of[i] != class_of[j]
                && polygons_share_an_edge(&faces[i].poly, &faces[j].poly)
            {
                add_separation(&mut separations, class_of[i], class_of[j]);
            }
        }
    }

    // rules (b)/(c): touching solids
    for &(sa, sb) in touching {
        if sa == sb || sa >= solid_count || sb >= solid_count {
            continue;
        }
        if cluster_of_solid[sa] == cluster_of_solid[sb] {
            // (b): same cluster — fine-grained, per touching face pair
            for &i in &faces_of_solid[sa] {
                for &j in &faces_of_solid[sb] {
                    if class_of[i] == class_of[j] {
                        continue; // merged
                    }
                    if is_opposite_facing_coplanar(&faces[i], &faces[j]) {
                        continue; // buried abutment
                    }
                    if polygons_within_patch_eps(&faces[i].poly, &faces[j].poly) {
                        add_separation(&mut separations, class_of[i], class_of[j]);
                    }
                }
            }
        } else {
            // (c): different clusters — the old law, blanket-applied
            for &i in &faces_of_solid[sa] {
                for &j in &faces_of_solid[sb] {
                    add_separation(&mut separations, class_of[i], class_of[j]);
                }
            }
        }
    }

    Superfaces {
        class_of,
        classes,
        separations,
        cluster_of_solid,
    }
}

/// Record `(a, b)` as a separated class pair, normalized to `(min, max)`
/// and deduplicated. A class never separates from itself.
fn add_separation(seps: &mut Vec<(usize, usize)>, a: usize, b: usize) {
    if a == b {
        return;
    }
    let pair = if a < b { (a, b) } else { (b, a) };
    if !seps.contains(&pair) {
        seps.push(pair);
    }
}

/// Path-compressed, union-by-rank disjoint-set forest over `0..n`. Root
/// numbering is an internal accident of union order and rank ties —
/// never exposed; [`normalize`] is the only sanctioned way this module
/// turns a `UnionFind` into output, and it renumbers by first appearance
/// in the CALLER's own order, not by whatever root happened to win.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u32>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
        } else if self.rank[ra] > self.rank[rb] {
            self.parent[rb] = ra;
        } else {
            self.parent[rb] = ra;
            self.rank[ra] += 1;
        }
    }
}

/// Turn a union-find over `0..n` into a normalized class assignment: the
/// first element (in input order) of each equivalence class gets the next
/// unused class number, starting at 0. This is what makes `class_of`
/// independent of union-by-rank's internal root choice, and what makes
/// shuffled input still comparable after renumbering.
fn normalize(uf: &mut UnionFind, n: usize) -> (Vec<usize>, usize) {
    let mut root_to_class: Vec<Option<usize>> = vec![None; n];
    let mut class_of = vec![0usize; n];
    let mut next = 0usize;
    for (i, slot) in class_of.iter_mut().enumerate() {
        let r = uf.find(i);
        let c = match root_to_class[r] {
            Some(c) => c,
            None => {
                let c = next;
                root_to_class[r] = Some(c);
                next += 1;
                c
            }
        };
        *slot = c;
    }
    (class_of, next)
}

// ---------------------------------------------------------------------
// Pure [f64; 3] vector helpers. Private and duplicated from `faces.rs` on
// purpose — the two-layer pure-module doctrine this crate follows keeps
// each geometry module self-contained rather than reaching across a
// shared internal helper file, matching how `observe/oids.rs` already
// carries its own copy of `rectangles_overlap` beside `oid_palette.rs`'s
// `Box3::touches` rather than importing it.
// ---------------------------------------------------------------------

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale3(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}

fn dist3(a: [f64; 3], b: [f64; 3]) -> f64 {
    dot3(sub3(a, b), sub3(a, b)).sqrt()
}

/// Do `a` and `b` point the same direction, within [`PARALLEL_EPS`]?
fn same_direction(a: [f64; 3], b: [f64; 3]) -> bool {
    (dot3(a, b) - 1.0).abs() < PARALLEL_EPS
}

/// Do `a` and `b` point exactly opposite directions, within
/// [`PARALLEL_EPS`]?
fn opposite_direction(a: [f64; 3], b: [f64; 3]) -> bool {
    (dot3(a, b) + 1.0).abs() < PARALLEL_EPS
}

/// The merge predicate: same-direction normals, coplanar offsets, and a
/// genuine tangent-plane overlap beyond [`PATCH_EPS`] in both extents.
fn is_merge_candidate(a: &Face, b: &Face) -> bool {
    same_direction(a.normal, b.normal)
        && (a.offset - b.offset).abs() <= COPLANAR_EPS
        && polygon_overlap_exceeds_patch(a, b)
}

/// A BURIED ABUTMENT: two faces on the exact same plane but facing away
/// from each other — a crate's underside on the floor it stands on. Offset
/// sign flips with the normal (`offset = dot(normal, point_on_plane)`), so
/// the same-plane test for an OPPOSITE pair is `offset_a ≈ -offset_b`, not
/// `offset_a ≈ offset_b` — the two are literally the same plane, described
/// from each side.
fn is_opposite_facing_coplanar(a: &Face, b: &Face) -> bool {
    opposite_direction(a.normal, b.normal) && (a.offset + b.offset).abs() <= COPLANAR_EPS
}

/// A polygon's consecutive-vertex edges, wrapping the last back to the
/// first.
fn polygon_edges(poly: &[[f64; 3]]) -> Vec<([f64; 3], [f64; 3])> {
    let n = poly.len();
    (0..n).map(|i| (poly[i], poly[(i + 1) % n])).collect()
}

/// Do `a` and `b` share a polygon edge — two vertices, in either order,
/// each within [`EDGE_POINT_EPS`] of the other polygon's? This is rule
/// (a)'s exact test: adjacent faces of one convex solid share their edge's
/// two endpoints bit-for-bit (same construction expression evaluated
/// twice), so the tolerance stays tight rather than a general proximity
/// test.
fn polygons_share_an_edge(a: &[[f64; 3]], b: &[[f64; 3]]) -> bool {
    let ea = polygon_edges(a);
    let eb = polygon_edges(b);
    ea.iter()
        .any(|&ea_edge| eb.iter().any(|&eb_edge| edges_match(ea_edge, eb_edge)))
}

fn edges_match(a: ([f64; 3], [f64; 3]), b: ([f64; 3], [f64; 3])) -> bool {
    (points_close(a.0, b.0) && points_close(a.1, b.1))
        || (points_close(a.0, b.1) && points_close(a.1, b.0))
}

fn points_close(a: [f64; 3], b: [f64; 3]) -> bool {
    dist3(a, b) < EDGE_POINT_EPS
}

/// Rule (b)'s closeness test: do `a` and `b`, as general (possibly
/// non-coplanar) bounded polygons in 3-D, come within [`PATCH_EPS`] of
/// each other anywhere? Computed as the minimum distance over every pair
/// of BOUNDARY edges (segment-to-segment, exact for straight edges).
///
/// KNOWN LIMIT, accepted deliberately: this misses a hypothetical pair
/// whose flat interiors cross far from either polygon's own boundary,
/// with no edge coming close. Every real touch this vocabulary's convex,
/// boundary-terminated faces produce — a shared corner, a crossing seam
/// at a T-junction, an edge grazing another face — has its closest
/// approach ON a boundary edge of at least one side, which the module's
/// own tests confirm for the junction fixture rather than assume.
fn polygons_within_patch_eps(a: &[[f64; 3]], b: &[[f64; 3]]) -> bool {
    min_polygon_distance(a, b) <= PATCH_EPS
}

fn min_polygon_distance(a: &[[f64; 3]], b: &[[f64; 3]]) -> f64 {
    let ea = polygon_edges(a);
    let eb = polygon_edges(b);
    let mut best = f64::INFINITY;
    for &(p1, q1) in &ea {
        for &(p2, q2) in &eb {
            let d = segment_distance(p1, q1, p2, q2);
            if d < best {
                best = d;
            }
        }
    }
    best
}

/// The minimum distance between two 3-D line segments — the standard
/// closest-points-between-segments construction (clamped parametric
/// projection, degenerate-segment guards on both sides).
fn segment_distance(p1: [f64; 3], q1: [f64; 3], p2: [f64; 3], q2: [f64; 3]) -> f64 {
    const EPS: f64 = 1e-15;
    let d1 = sub3(q1, p1);
    let d2 = sub3(q2, p2);
    let r = sub3(p1, p2);
    let a = dot3(d1, d1);
    let e = dot3(d2, d2);
    let f = dot3(d2, r);

    let (s, t) = if a <= EPS && e <= EPS {
        (0.0, 0.0)
    } else if a <= EPS {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot3(d1, r);
        if e <= EPS {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot3(d1, d2);
            let denom = a * e - b * b;
            let mut s = if denom.abs() > EPS {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };

    let c1 = add3(p1, scale3(d1, s));
    let c2 = add3(p2, scale3(d2, t));
    dist3(c1, c2)
}

/// A 2-D point in a face's own tangent plane, after dropping the axis its
/// normal points closest to.
type Pt2 = (f64, f64);

/// Which world axis to drop when projecting a face's polygon to 2-D: the
/// one its normal has the LARGEST absolute component on. Dropping that
/// axis keeps the projection well-conditioned for any plane orientation,
/// not just the axis-aligned ones this vocabulary happens to ship today.
fn dominant_axis(normal: [f64; 3]) -> usize {
    let (ax, ay, az) = (normal[0].abs(), normal[1].abs(), normal[2].abs());
    if ax >= ay && ax >= az {
        0
    } else if ay >= az {
        1
    } else {
        2
    }
}

fn project_to_plane(poly: &[[f64; 3]], drop_axis: usize) -> Vec<Pt2> {
    let (u, v) = match drop_axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    poly.iter().map(|p| (p[u], p[v])).collect()
}

fn signed_area2(poly: &[Pt2]) -> f64 {
    let n = poly.len();
    let mut sum = 0.0;
    for i in 0..n {
        let (x1, y1) = poly[i];
        let (x2, y2) = poly[(i + 1) % n];
        sum += x1 * y2 - x2 * y1;
    }
    sum * 0.5
}

/// Sutherland–Hodgman clipping needs its CLIP polygon wound
/// counter-clockwise; a projected face's winding depends on which two
/// world axes survived the drop, so this reorders rather than assumes.
fn ensure_ccw(poly: Vec<Pt2>) -> Vec<Pt2> {
    if signed_area2(&poly) < 0.0 {
        poly.into_iter().rev().collect()
    } else {
        poly
    }
}

/// Is `p` on the left of, or on, the directed edge `a -> b`? Inclusive
/// with a small numeric slack, so a subject vertex sitting exactly on a
/// clip edge is kept rather than dropped by rounding.
fn is_inside_or_on(a: Pt2, b: Pt2, p: Pt2) -> bool {
    let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    cross >= -1e-12
}

/// The intersection point of infinite lines `p1-p2` and `a-b`, or `None`
/// for (near-)parallel lines.
fn segment_intersection(p1: Pt2, p2: Pt2, a: Pt2, b: Pt2) -> Option<Pt2> {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    let (x3, y3) = a;
    let (x4, y4) = b;
    let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if denom.abs() < 1e-15 {
        return None;
    }
    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
    Some((x1 + t * (x2 - x1), y1 + t * (y2 - y1)))
}

/// Sutherland–Hodgman: clip `subject` (any simple polygon) against the
/// convex, CCW-wound `clip` polygon, returning the exact intersection
/// polygon (possibly empty).
fn clip_convex(subject: &[Pt2], clip: &[Pt2]) -> Vec<Pt2> {
    let mut output = subject.to_vec();
    for i in 0..clip.len() {
        if output.is_empty() {
            break;
        }
        let a = clip[i];
        let b = clip[(i + 1) % clip.len()];
        let input = std::mem::take(&mut output);
        let n = input.len();
        for k in 0..n {
            let cur = input[k];
            let prev = input[(k + n - 1) % n];
            let cur_in = is_inside_or_on(a, b, cur);
            let prev_in = is_inside_or_on(a, b, prev);
            if cur_in {
                if !prev_in && let Some(ip) = segment_intersection(prev, cur, a, b) {
                    output.push(ip);
                }
                output.push(cur);
            } else if prev_in && let Some(ip) = segment_intersection(prev, cur, a, b) {
                output.push(ip);
            }
        }
    }
    output
}

fn bbox_extents(poly: &[Pt2]) -> (f64, f64) {
    if poly.is_empty() {
        return (0.0, 0.0);
    }
    let mut min_u = f64::INFINITY;
    let mut max_u = f64::NEG_INFINITY;
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for &(u, v) in poly {
        min_u = min_u.min(u);
        max_u = max_u.max(u);
        min_v = min_v.min(v);
        max_v = max_v.max(v);
    }
    (max_u - min_u, max_v - min_v)
}

/// The merge test's overlap half: project both (near-parallel) polygons
/// to their shared tangent plane, clip one against the other to the exact
/// convex intersection, and require that intersection to exceed
/// [`PATCH_EPS`] along BOTH tangent axes — an edge-only touch fails one
/// axis and is correctly refused.
fn polygon_overlap_exceeds_patch(a: &Face, b: &Face) -> bool {
    let axis = dominant_axis(a.normal);
    let pa = ensure_ccw(project_to_plane(&a.poly, axis));
    let pb = ensure_ccw(project_to_plane(&b.poly, axis));
    if pa.len() < 3 || pb.len() < 3 {
        return false;
    }
    let inter = clip_convex(&pa, &pb);
    let (du, dv) = bbox_extents(&inter);
    du > PATCH_EPS && dv > PATCH_EPS
}

#[cfg(test)]
mod tests {
    use super::super::faces::{Shape, faces};
    use super::*;

    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    fn ordered(a: usize, b: usize) -> (usize, usize) {
        if a < b { (a, b) } else { (b, a) }
    }

    /// The issue-14 junction, built by hand at full `WALL_T` (0.15)
    /// padding — the PRE-cap-inset geometry this campaign replaces the
    /// inset with. Wall A runs along X, centerline (-2,0)..(2,0): full
    /// length 4 + 2*0.15 = 4.3, center [0,1.5,0], size [4.3,3,0.3]. Wall B
    /// runs along Z, centerline (0,0)..(0,4): full length 4 + 2*0.15 =
    /// 4.3, center [0,1.5,2], size [0.3,3,4.3]. Wall B's south end lands
    /// exactly WALL_T past wall A's own centerline, at z=-0.15 — the same
    /// plane as wall A's own south (-Z) flank.
    fn wall_a() -> Shape {
        Shape::Box3d {
            center: [0.0, 1.5, 0.0],
            size: [4.3, 3.0, 0.3],
            basis: IDENTITY,
        }
    }
    fn wall_b() -> Shape {
        Shape::Box3d {
            center: [0.0, 1.5, 2.0],
            size: [0.3, 3.0, 4.3],
            basis: IDENTITY,
        }
    }

    /// The junction fixture's 12 faces: wall A's six (global 0..6, order
    /// -X,+X,-Y,+Y,-Z,+Z) then wall B's six (global 6..12, same order).
    fn junction_faces() -> Vec<Face> {
        let mut all = faces(0, &wall_a());
        all.extend(faces(1, &wall_b()));
        all
    }

    /// THE issue-14 case: wall B's south end cap (global index 10, its own
    /// -Z face) lands exactly in wall A's south flank plane (global index
    /// 4, wall A's own -Z face) — both z=-0.15, both normal [0,0,-1], and
    /// their footprints overlap over a 0.3 x 3 m patch (wall B's own
    /// width by the shared wall height). Offset is `dot(normal, point)`,
    /// so a -Z face's offset is the NEGATED z coordinate: (-1)*(-0.15) =
    /// 0.15, not -0.15 — both faces share that same positive offset,
    /// which is what makes them coplanar. The two faces MERGE: one class,
    /// one label, bit-identical G, nothing left to fight.
    #[test]
    fn a_junction_cap_merges_into_the_partners_flank() {
        let all = junction_faces();
        assert_eq!(all[4].normal, [0.0, 0.0, -1.0]);
        assert!((all[4].offset - 0.15).abs() < 1e-9);
        assert_eq!(all[10].normal, [0.0, 0.0, -1.0]);
        assert!((all[10].offset - 0.15).abs() < 1e-9);

        let sf = superfaces(&all, &[(0, 1)]);
        assert_eq!(sf.class_of[4], sf.class_of[10]);

        // Wall A's own OTHER flank (global 5, +Z, opposite-facing and 0.3
        // m away) must stay a different class — wall A is symmetric about
        // its own centerline, so both its flanks carry the SAME offset
        // magnitude (0.15), and a merge law that quietly required
        // opposite-facing normals instead of same-facing would find a
        // same-offset "match" here too (a coincidence particular to a
        // wall centered on its own thickness) and wrongly fuse the whole
        // slab into one class. This is what actually catches that flip —
        // verified: the (4,10) assertion above alone stays green under
        // that exact mutation, because 4 and 10 both end up transitively
        // unioned through 5 anyway; only this assertion tells the two
        // apart.
        assert_ne!(sf.class_of[4], sf.class_of[5]);
    }

    /// The same junction's PERPENDICULAR pairs separate, checked two ways:
    /// wall B's own west flank (global 6, -X) against wall B's own south
    /// cap (global 10, now merged into wall A's flank class) — the box's
    /// own silhouette law must survive even though index 10 just merged
    /// into a bigger cluster in the test above; and wall A's untouched
    /// north flank (global 5, +Z) against wall B's east flank (global 7,
    /// +X), perpendicular planes meeting exactly at the T's outside
    /// corner — never merge candidates (dot of their normals is 0,
    /// nowhere near +-1), but their polygons touch (the line
    /// x=0.15,z=0.15 lies on both, and reaches each polygon's own y=0
    /// boundary edge), so this bend needs separated labels too.
    ///
    /// NOT a clean per-rule split, verified rather than assumed: the
    /// (6,10) pair is reachable through EITHER rule (a) (6 and 10 are
    /// wall B's own adjacent faces, sharing a corner edge) OR rule (b)
    /// (wall A's -Z, index 4, sits in the same class as 10 and directly
    /// touches wall B's -X at their shared corner too) — disabling rule
    /// (a) alone leaves this assertion green, because rule (b) re-derives
    /// the identical class pair through a different face pair. Rule (a)
    /// in true isolation (no second solid to fall back on) is what
    /// `edge_sharing_faces_of_one_solid_separate` below pins.
    #[test]
    fn perpendicular_junction_faces_separate() {
        let all = junction_faces();
        let sf = superfaces(&all, &[(0, 1)]);

        assert_eq!(all[6].normal, [-1.0, 0.0, 0.0]);
        assert_ne!(sf.class_of[6], sf.class_of[10]);
        assert!(
            sf.separations
                .contains(&ordered(sf.class_of[6], sf.class_of[10]))
        );

        assert_eq!(all[5].normal, [0.0, 0.0, 1.0]);
        assert_eq!(all[7].normal, [1.0, 0.0, 0.0]);
        assert_ne!(sf.class_of[5], sf.class_of[7]);
        assert!(
            sf.separations
                .contains(&ordered(sf.class_of[5], sf.class_of[7]))
        );
    }

    /// A floor and a crate resting on it, lifted to y=5 so the abutment's
    /// plane offset is genuinely nonzero on both sides (floor top
    /// offset=+5, crate bottom offset=-5 — the opposite-normal "same
    /// plane" test is offset_a ~= -offset_b, not offset_a ~= offset_b, and
    /// a fixture straddling y=0 could pass a sign-blind implementation by
    /// accident). Abutment is not a merge (opposite normals) and not a
    /// separation-by-contact (buried, excluded from rule (b)) — but the
    /// crate and floor never merge on any face pair, so they stay
    /// different clusters, and the OLD blanket law (rule (c)) still
    /// separates them at solid level, exactly as today.
    #[test]
    fn opposite_facing_abutment_neither_merges_nor_fights() {
        let floor = Shape::Box3d {
            center: [0.0, 4.95, 0.0],
            size: [10.0, 0.1, 10.0],
            basis: IDENTITY,
        };
        let crate_box = Shape::Box3d {
            center: [0.0, 5.5, 0.0],
            size: [1.0, 1.0, 1.0],
            basis: IDENTITY,
        };
        let mut all = faces(0, &floor);
        all.extend(faces(1, &crate_box));

        // floor's own +Y (top, global 3) and the crate's own -Y (bottom,
        // global 8: crate's local index 2 in -X,+X,-Y,+Y,-Z,+Z order)
        assert_eq!(all[3].normal, [0.0, 1.0, 0.0]);
        assert!((all[3].offset - 5.0).abs() < 1e-9);
        assert_eq!(all[8].normal, [0.0, -1.0, 0.0]);
        assert!((all[8].offset - (-5.0)).abs() < 1e-9);

        let sf = superfaces(&all, &[(0, 1)]);
        assert_ne!(sf.class_of[3], sf.class_of[8]);
        assert_ne!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);
        assert!(
            sf.separations
                .contains(&ordered(sf.class_of[3], sf.class_of[8]))
        );
    }

    /// The origin-plane twin of the abutment test above: a floor and a
    /// crate meeting EXACTLY at y=0, so both offsets are literally 0 —
    /// the one place a same-direction check broadened to ACCEPT opposite
    /// direction too (e.g. comparing `|dot|` instead of `dot`) would slip
    /// past undetected by the y=5 fixture (whose offsets differ by 10, so
    /// a plain equal-offset test rejects it regardless of the direction
    /// bug). At y=0 that same broadened check would ALSO pass the offset
    /// gate (0 ~= 0), so only a genuinely direction-aware merge law keeps
    /// the crate's silhouette against the floor it stands on.
    #[test]
    fn an_abutment_through_the_coordinate_origin_still_does_not_merge() {
        let floor = Shape::Box3d {
            center: [0.0, -0.05, 0.0],
            size: [10.0, 0.1, 10.0],
            basis: IDENTITY,
        };
        let crate_box = Shape::Box3d {
            center: [0.0, 0.5, 0.0],
            size: [1.0, 1.0, 1.0],
            basis: IDENTITY,
        };
        let mut all = faces(0, &floor);
        all.extend(faces(1, &crate_box));
        assert_eq!(all[3].normal, [0.0, 1.0, 0.0]);
        assert_eq!(all[3].offset, 0.0);
        assert_eq!(all[8].normal, [0.0, -1.0, 0.0]);
        assert_eq!(all[8].offset, 0.0);

        let sf = superfaces(&all, &[(0, 1)]);
        assert_ne!(sf.class_of[3], sf.class_of[8]);
    }

    /// `is_opposite_facing_coplanar` directly: no fixture through the
    /// public `superfaces` entry point currently puts two solids in the
    /// SAME cluster (the only place this private helper's exclusion is
    /// even consulted, inside rule (b)) while also abutting at a nonzero,
    /// sign-sensitive offset — both `opposite_facing_abutment_*` tests
    /// above land their pair in rule (c) instead (different clusters),
    /// which never calls this function at all. Verified: a plain
    /// `(a.offset - b.offset)` formula here (rather than the correct
    /// `+`) passes the full suite unnoticed. Two faces truly on the same
    /// y=5 plane, opposite normals — offset +5 one way, -5 the other —
    /// must read as coplanar; the same normal pairing at a genuinely
    /// different plane (y=6) must not.
    #[test]
    fn opposite_facing_coplanar_uses_the_sign_aware_same_plane_test() {
        let top = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 5.0,
            poly: vec![
                [-1.0, 5.0, -1.0],
                [1.0, 5.0, -1.0],
                [1.0, 5.0, 1.0],
                [-1.0, 5.0, 1.0],
            ],
            solid: 0,
        };
        let bottom_same_plane = Face {
            normal: [0.0, -1.0, 0.0],
            offset: -5.0,
            poly: vec![
                [-0.5, 5.0, -0.5],
                [0.5, 5.0, -0.5],
                [0.5, 5.0, 0.5],
                [-0.5, 5.0, 0.5],
            ],
            solid: 1,
        };
        assert!(is_opposite_facing_coplanar(&top, &bottom_same_plane));

        let bottom_different_plane = Face {
            offset: -6.0,
            ..bottom_same_plane
        };
        assert!(!is_opposite_facing_coplanar(&top, &bottom_different_plane));
    }

    /// A standalone unit box, no partner at all: none of its six faces
    /// can ever be a merge candidate with another (a convex box's six
    /// planes are all distinct), so every face keeps its own class — and
    /// two adjacent faces (-X and -Y share a corner edge) must still
    /// separate. A box's own silhouette law survives inside multi-member
    /// clusters is the point of the junction test above; this is the
    /// same law's simplest possible witness, with no cluster at all.
    #[test]
    fn edge_sharing_faces_of_one_solid_separate() {
        let f = faces(
            0,
            &Shape::Box3d {
                center: [0.0; 3],
                size: [1.0; 3],
                basis: IDENTITY,
            },
        );
        let sf = superfaces(&f, &[]);
        assert_eq!(sf.classes, 6);
        assert_ne!(sf.class_of[0], sf.class_of[2]);
        assert!(
            sf.separations
                .contains(&ordered(sf.class_of[0], sf.class_of[2]))
        );
    }

    /// Two collinear wall segments across a 4 m doorway gap: both south
    /// flanks (-Z) sit on the exact same plane, same outward direction —
    /// coplanar by every measure the merge law checks EXCEPT overlap. The
    /// gap between the walls (x in (-2,2)) is four orders of magnitude
    /// past `PATCH_EPS`, so the tangent rectangles never intersect and the
    /// two faces must not merge.
    #[test]
    fn coplanar_but_disjoint_faces_do_not_merge() {
        let wall_c = Shape::Box3d {
            center: [-3.0, 1.5, 0.0],
            size: [2.0, 3.0, 0.3],
            basis: IDENTITY,
        };
        let wall_d = Shape::Box3d {
            center: [3.0, 1.5, 0.0],
            size: [2.0, 3.0, 0.3],
            basis: IDENTITY,
        };
        let mut all = faces(0, &wall_c);
        all.extend(faces(1, &wall_d));
        assert_eq!(all[4].normal, [0.0, 0.0, -1.0]);
        assert_eq!(all[10].normal, [0.0, 0.0, -1.0]);
        assert!((all[4].offset - all[10].offset).abs() < 1e-9);

        let sf = superfaces(&all, &[]);
        assert_ne!(sf.class_of[4], sf.class_of[10]);
    }

    /// Determinism: the junction fixture's faces, fed in REVERSED order,
    /// must produce the same GROUPING (which faces share a class) and the
    /// same SEPARATION graph, once each run's own class numbers —
    /// necessarily different, since both are numbered by first appearance
    /// in their own input order — are translated through the known
    /// reversal. `touching` is unaffected: it names SOLID indices, which
    /// don't move when face order shuffles.
    #[test]
    fn class_assignment_is_input_order_stable() {
        let natural = junction_faces();
        let touching = [(0, 1)];
        let forward = superfaces(&natural, &touching);

        let reversed: Vec<Face> = natural.iter().rev().cloned().collect();
        let backward = superfaces(&reversed, &touching);

        let n = natural.len();
        let pos = |i: usize| n - 1 - i;

        assert_eq!(forward.classes, backward.classes);
        for i in 0..n {
            for j in 0..n {
                assert_eq!(
                    forward.class_of[i] == forward.class_of[j],
                    backward.class_of[pos(i)] == backward.class_of[pos(j)],
                    "faces {i},{j} grouped differently after reordering"
                );
            }
        }

        let mut nat_to_rev = vec![0usize; forward.classes];
        for i in 0..n {
            nat_to_rev[forward.class_of[i]] = backward.class_of[pos(i)];
        }
        let mut mapped: Vec<(usize, usize)> = forward
            .separations
            .iter()
            .map(|&(a, b)| ordered(nat_to_rev[a], nat_to_rev[b]))
            .collect();
        let mut expected = backward.separations.clone();
        mapped.sort();
        expected.sort();
        assert_eq!(mapped, expected);
    }

    /// Totality on the empty input: no faces, no classes, nothing to
    /// separate — never a panic, matching every other module in this
    /// vocabulary.
    #[test]
    fn empty_faces_yield_an_empty_graph() {
        let sf = superfaces(&[], &[]);
        assert_eq!(sf.classes, 0);
        assert!(sf.class_of.is_empty());
        assert!(sf.separations.is_empty());
        assert!(sf.cluster_of_solid.is_empty());
    }

    /// A `touching` pair naming a solid this call never received a face
    /// for (out of range of `cluster_of_solid`) is skipped rather than
    /// panicking on an out-of-bounds index — the boundary can pass a
    /// touch list built against the FULL census even when only some
    /// solids' faces are in scope for a given call. The out-of-range
    /// pairs must be pure no-ops: the result is identical to passing no
    /// `touching` at all (rule (a)'s same-solid edge separations still
    /// fire regardless — they never consult `touching` — so the box's
    /// own six faces are not expected to come back with an EMPTY
    /// separations list, only the SAME one an empty touch list gives).
    #[test]
    fn a_touching_pair_beyond_the_known_solids_does_not_panic() {
        let f = faces(
            0,
            &Shape::Box3d {
                center: [0.0; 3],
                size: [1.0; 3],
                basis: IDENTITY,
            },
        );
        let with_bogus_pairs = superfaces(&f, &[(0, 7), (7, 0), (3, 9)]);
        let with_no_pairs = superfaces(&f, &[]);
        assert_eq!(with_bogus_pairs.classes, 6);
        assert_eq!(with_bogus_pairs.class_of, with_no_pairs.class_of);
        assert_eq!(with_bogus_pairs.separations, with_no_pairs.separations);
    }

    /// Two DIFFERENT solids' faces can never accidentally satisfy the
    /// edge-sharing rule (a) by construction alone — that rule is scoped
    /// to `faces[i].solid == faces[j].solid`; this pins the scope itself,
    /// since two touching walls' faces literally do share coordinates
    /// (the merged pair) without being edge-neighbours of the same solid.
    #[test]
    fn rule_a_never_fires_across_two_different_solids() {
        let all = junction_faces();
        let sf = superfaces(&all, &[(0, 1)]);
        // global 4 (wall A's -Z) and 10 (wall B's -Z) are a DIFFERENT
        // solid pair that MERGES (same class) rather than an edge-share:
        // rule (a) cannot have produced a separation for it, because a
        // class never separates from itself.
        assert!(
            !sf.separations
                .contains(&ordered(sf.class_of[4], sf.class_of[10]))
        );
    }

    /// Two faces on the exact same plane, same direction, but positioned
    /// so their tangent rectangles touch along only an edge (zero-width
    /// overlap on one axis) do not merge — the merge test's "beyond
    /// PATCH_EPS in BOTH extents" requirement, isolated from the doorway
    /// fixture's much larger gap. Two unit-square top faces, one at
    /// x in [0,1], the other at x in [1,2] (sharing the line x=1 exactly,
    /// zero overlap along x, full overlap along z).
    #[test]
    fn an_edge_only_touch_does_not_merge() {
        let a = Shape::Box3d {
            center: [0.5, 0.0, 0.0],
            size: [1.0, 1.0, 1.0],
            basis: IDENTITY,
        };
        let b = Shape::Box3d {
            center: [1.5, 0.0, 0.0],
            size: [1.0, 1.0, 1.0],
            basis: IDENTITY,
        };
        let mut all = faces(0, &a);
        all.extend(faces(1, &b));
        // +Y tops: global 3 (a) and global 9 (b), both normal [0,1,0],
        // both offset 0.5, touching only along the line x=1
        assert_eq!(all[3].normal, [0.0, 1.0, 0.0]);
        assert_eq!(all[9].normal, [0.0, 1.0, 0.0]);
        assert_eq!(all[3].offset, all[9].offset);
        let sf = superfaces(&all, &[]);
        assert_ne!(sf.class_of[3], sf.class_of[9]);
    }

    /// `PATCH_EPS`'s own boundary, isolated to one axis and built from
    /// direct polygon literals (not `Shape::Box3d`'s center+half-extent
    /// arithmetic, which does not reliably land bit-exact on a decimal
    /// threshold like `0.001` — verified empirically: a box centered at
    /// `0.999` misses this same boundary by more than a ULP). Two
    /// rectangles on the y=0.5 plane, one spanning x in [-1, 0.001], the
    /// other x in [0, 1] — the overlap is `0.001 - 0.0`, a subtraction
    /// against zero that IEEE 754 guarantees exact, so it lands
    /// bit-identical to `PATCH_EPS`'s own parse of the same literal. That
    /// exact tie must NOT merge — the boundary is EXCLUSIVE, the same
    /// convention `observe::oids::rectangles_overlap` draws ("a
    /// millimetre of overlap is an edge, not a patch"). Nudge the small
    /// rectangle's near edge 0.002 past zero and the same pair merges, so
    /// the boundary itself is what excludes the first case, not the
    /// geometry.
    #[test]
    fn an_overlap_of_exactly_patch_eps_is_an_edge_not_a_patch() {
        let big = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 0.5,
            poly: vec![
                [-1.0, 0.5, -1.0],
                [0.001, 0.5, -1.0],
                [0.001, 0.5, 1.0],
                [-1.0, 0.5, 1.0],
            ],
            solid: 0,
        };
        let flush = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 0.5,
            poly: vec![
                [0.0, 0.5, -1.0],
                [1.0, 0.5, -1.0],
                [1.0, 0.5, 1.0],
                [0.0, 0.5, 1.0],
            ],
            solid: 1,
        };
        assert!(!is_merge_candidate(&big, &flush));

        let overlapping = Face {
            poly: vec![
                [-0.002, 0.5, -1.0],
                [1.0, 0.5, -1.0],
                [1.0, 0.5, 1.0],
                [-0.002, 0.5, 1.0],
            ],
            ..flush
        };
        assert!(is_merge_candidate(&big, &overlapping));
    }

    /// `COPLANAR_EPS`'s own boundary, INCLUSIVE — the same convention
    /// `observe::oids::two_millimetres_of_coincidence_is_inclusive` pins
    /// for the identical constant this module promotes it from. Offsets
    /// `0.0` and `0.002` differ by `0.002 - 0.0`, exact by the same
    /// zero-subtraction argument the `PATCH_EPS` test above uses, landing
    /// bit-identical to `COPLANAR_EPS`'s own parse. At that exact gap the
    /// two overlapping faces still merge; one bit further apart
    /// (`0.0021`) they must not.
    #[test]
    fn coplanar_eps_is_inclusive_at_its_own_boundary() {
        let a = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 0.0,
            poly: vec![
                [-1.0, 0.0, -1.0],
                [1.0, 0.0, -1.0],
                [1.0, 0.0, 1.0],
                [-1.0, 0.0, 1.0],
            ],
            solid: 0,
        };
        let at_the_boundary = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 0.002,
            poly: vec![
                [-1.0, 0.002, -1.0],
                [1.0, 0.002, -1.0],
                [1.0, 0.002, 1.0],
                [-1.0, 0.002, 1.0],
            ],
            solid: 1,
        };
        assert!(is_merge_candidate(&a, &at_the_boundary));

        let just_past = Face {
            offset: 0.0021,
            poly: vec![
                [-1.0, 0.0021, -1.0],
                [1.0, 0.0021, -1.0],
                [1.0, 0.0021, 1.0],
                [-1.0, 0.0021, 1.0],
            ],
            ..at_the_boundary
        };
        assert!(!is_merge_candidate(&a, &just_past));
    }
}
