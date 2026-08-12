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
//!   within [`COPLANAR_EPS`], and — after projecting both polygons to a
//!   valid 2-D representation of the shared plane — their exact convex
//!   intersection's own minimum width, over every direction the
//!   intersection's shape can be measured in (not just the two arbitrary
//!   WORLD axes the projection happened to keep), exceeds [`PATCH_EPS`].
//!   An edge-only touch, at any rotation, is a bend, not a melt.
//! - **The singleton collapse**: a solid alone in its own cluster — no
//!   face of it ever won a cross-solid merge edge, so it stands exactly
//!   as it always has — has ALL of its faces folded into ONE class,
//!   restoring the spec's own law: "singletons keep today's exact look:
//!   one label across the whole solid, one silhouette, no interior
//!   lines." Without this, rule (a) demands a box's six mutually-adjacent
//!   faces take its own three-colour minimum even when nothing ever
//!   merges with it, and two ordinary touching boxes then need rule (c)
//!   to separate six cross-pairs instead of one — measured cost: 93
//!   superface classes starved on the shipped map before this law
//!   landed.
//! - **Separation edge** (two classes must take labels ≥ `MIN_SEP` apart,
//!   `labels::MIN_SEP`, this module only builds the graph): three rules,
//!   checked independently, (a) and (b) scoped to MULTI-MEMBER clusters
//!   only (a singleton's faces are already one class by the time either
//!   rule runs, so neither can act on it) —
//!   (a) two faces of ONE solid sharing a polygon edge — a box's own
//!   corner never disappears, inside any cluster with more than one
//!   member;
//!   (b) faces of two DIFFERENT, TOUCHING solids that ended up in the
//!   SAME multi-member cluster (merged via some other face pair) whose
//!   polygons pass within [`PATCH_EPS`] of each other in 3-D, excluding
//!   pairs already merged and excluding a BURIED ABUTMENT —
//!   opposite-facing coplanar contact, a crate's underside on the floor
//!   it stands on;
//!   (c) every face pair between two touching solids that never merged at
//!   all (different clusters) — the old per-solid law, blanket-applied:
//!   all of one solid's classes separate from all of the other's. Two
//!   touching singletons now see exactly one class each, restoring the
//!   pre-superface two-label law exactly.
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

/// One threshold, two directions, by design. The merge test
/// ([`polygon_overlap_exceeds_patch`]) requires a coplanar overlap to
/// EXCEED this — EXCLUSIVE — to count as a real patch rather than a bare
/// edge: at exactly this width, that is still an edge, not a melt. Rule
/// (b)'s closeness test ([`polygons_within_patch_eps`]) asks a different
/// question — "are these two faces close enough that a bend might need a
/// line" — and answers it INCLUSIVE: at exactly this distance, the seam
/// still separates, erring toward drawing the line. Promoted unchanged
/// from `observe::oids::PATCH_EPS`.
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
    /// transitively). Sized to `max(face.solid) + 1` — every index UP TO
    /// that maximum gets an entry, including a solid that contributed no
    /// face to this call (a trivial cluster of one, since it can never
    /// have merged with anything); only a solid index ABOVE the maximum
    /// (never referenced by any face) has no entry at all. Used by
    /// paint's wall-merge warning and by this module's own tests.
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

    // --- solid-level clusters: the merge relation lifted to solids, read
    // directly off the union-find's own roots. `class_of` is not
    // normalized yet at this point — root equality IS "same class" here,
    // and normalizing twice (once now, once again after the singleton
    // collapse below) would be wasted work for the same answer.
    let solid_count = faces.iter().map(|f| f.solid).max().map_or(0, |m| m + 1);
    let mut solid_uf = UnionFind::new(solid_count);
    for i in 0..n {
        for j in (i + 1)..n {
            if faces[i].solid != faces[j].solid && face_uf.find(i) == face_uf.find(j) {
                solid_uf.union(faces[i].solid, faces[j].solid);
            }
        }
    }
    let (cluster_of_solid, clusters) = normalize(&mut solid_uf, solid_count);

    // --- the singleton collapse: a solid alone in its own cluster (no
    // face of it ever won a cross-solid merge edge) folds ALL its own
    // faces into ONE class — see the module doc's own law statement.
    // Determinism: each singleton solid's faces union onto its own FIRST
    // face in input order, never a hash or a hunt for a "canonical" one.
    let mut cluster_size = vec![0usize; clusters];
    for &c in &cluster_of_solid {
        cluster_size[c] += 1;
    }
    let is_singleton_solid: Vec<bool> = (0..solid_count)
        .map(|s| cluster_size[cluster_of_solid[s]] == 1)
        .collect();
    let mut singleton_anchor: Vec<Option<usize>> = vec![None; solid_count];
    for (i, f) in faces.iter().enumerate() {
        if is_singleton_solid[f.solid] {
            match singleton_anchor[f.solid] {
                Some(anchor) => face_uf.union(anchor, i),
                None => singleton_anchor[f.solid] = Some(i),
            }
        }
    }

    let (class_of, classes) = normalize(&mut face_uf, n);

    // faces grouped by solid, input order preserved within each group
    let mut faces_of_solid: Vec<Vec<usize>> = vec![Vec::new(); solid_count];
    for (i, f) in faces.iter().enumerate() {
        faces_of_solid[f.solid].push(i);
    }

    let mut separations: Vec<(usize, usize)> = Vec::new();

    // rule (a): two faces of ONE solid sharing a polygon edge — scoped to
    // MULTI-MEMBER clusters, matching the spec's own text. The explicit
    // `!is_singleton_solid[...]` guard is defense-in-depth rather than the
    // sole mechanism: a singleton's own faces are already one class by
    // the time this runs (the collapse above), so `class_of[i] !=
    // class_of[j]` alone already refuses every same-singleton-solid pair;
    // this guard states the scoping in the code the same way the law
    // states it in words, rather than leaving it as an emergent property
    // of the collapse alone.
    for i in 0..n {
        for j in (i + 1)..n {
            if faces[i].solid == faces[j].solid
                && !is_singleton_solid[faces[i].solid]
                && class_of[i] != class_of[j]
                && polygons_share_an_edge(&faces[i].poly, &faces[j].poly)
            {
                add_separation(&mut separations, class_of[i], class_of[j]);
            }
        }
    }

    // rules (b)/(c): touching solids. Rule (b)'s branch below is likewise
    // scoped to multi-member clusters, but needs no separate guard: a
    // singleton solid's cluster has exactly one member (itself), so
    // `cluster_of_solid[sa] == cluster_of_solid[sb]` for `sa != sb` is
    // already impossible whenever either side is a singleton — every
    // touching pair naming one takes rule (c)'s branch by construction.
    for &(sa, sb) in touching {
        if sa == sb || sa >= solid_count || sb >= solid_count {
            continue;
        }
        if cluster_of_solid[sa] == cluster_of_solid[sb] {
            // (b): same MULTI-MEMBER cluster — fine-grained, per touching
            // face pair
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
            // (c): different clusters — the old law, blanket-applied. A
            // singleton solid contributes exactly ONE class here (the
            // collapse above), so two touching singletons see exactly
            // one cross-pair — the pre-superface two-label law, restored.
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
/// and deduplicated. A class never separates from itself — defensive
/// totality, not a currently load-bearing branch: every call site already
/// gates on `class_of[i] != class_of[j]` (rules (a) and (b) explicitly,
/// rule (c) implicitly, since two faces in the same class imply their
/// solids share a cluster, which rule (c)'s own branch has already
/// excluded) before ever reaching here. Kept anyway, because this
/// function's own contract should hold for ANY caller, not just today's.
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
/// each other anywhere, INCLUSIVE? Computed as the minimum distance over
/// every pair of BOUNDARY edges (segment-to-segment, exact for straight
/// edges). Deliberately inclusive, unlike the merge test's exclusive
/// [`PATCH_EPS`]: this is asking "are these close enough that a seam
/// might need a line", not "is there a real overlap patch" — erring
/// toward drawing the separation at the exact threshold is the safe
/// direction here.
///
/// KNOWN MISS, corrected after review — the precondition is narrower than
/// it first looks: this test finds a touch ONLY when the two polygons'
/// closest approach lands on a boundary EDGE of at least one side. That
/// is true of every T-junction in the shipped map today, but not because
/// "boundary-terminated convex faces always touch at a boundary" — that
/// claim is false in general. It holds here for one concrete reason:
/// `level_plan::wall_box` gives every wall the SAME height (`WALL_H`),
/// so two junctioned walls' flank/cap faces always share the same y=0
/// and y=`WALL_H` bounds, and the crossing line a perpendicular pair
/// traces (see `superface::tests::perpendicular_junction_faces_separate`)
/// always reaches one of those shared bounds. Break that precondition —
/// a SHORTER or vertically offset arriving wall, crossing its taller
/// partner's flank entirely within both polygons' INTERIORS, touching
/// neither one's boundary — and this test would report no touch at all
/// (truth distance 0, edge-only distance whatever the nearest edges
/// actually are) for a seam that needs one. Unreachable today only
/// because uniform wall height is enforced upstream, in the level
/// authoring path, not in this module; a future variable-height wall
/// vocabulary would need this test re-derived, not just re-used.
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
/// axis keeps the PROJECTION well-conditioned for any plane orientation,
/// not just the axis-aligned ones this vocabulary happens to ship today —
/// but the two axes it keeps are still arbitrary WORLD directions, not
/// the face's own. Measuring an overlap's size directly against them
/// (a bounding box in u/v) is NOT itself rotation-invariant, which is
/// exactly the bug a review caught: two coplanar faces rotated 45 degrees
/// about the normal, sharing only a diagonal edge, have a bounding box
/// spanning a full unit on BOTH world axes despite zero true overlap
/// area. [`min_width`] is the fix — it measures against the
/// INTERSECTION polygon's own edges, never the world axes this function
/// picks.
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

/// The minimum width of a convex 2-D polygon over EVERY direction, not
/// just the two world axes the projection happened to keep. For a convex
/// shape the narrowest direction is always perpendicular to one of its
/// own edges (the standard rotating-calipers result), so checking the
/// projected extent along each edge's own outward normal and taking the
/// smallest is exact — and, critically, ROTATION-INVARIANT: it depends
/// only on the polygon's own shape, never on which arbitrary pair of
/// world axes [`project_to_plane`] happened to keep. A polygon with fewer
/// than 3 points (empty or degenerate clip result) has no width at all.
///
/// This replaces an earlier WORLD-AXIS bounding-box measurement that a
/// review caught being wrong: two coplanar squares turned 45 degrees
/// about the shared normal, sharing exactly one full edge (a real overlap
/// AREA of zero), had a shared-edge bounding box spanning a full unit on
/// BOTH world axes — comfortably clearing `PATCH_EPS` on both — and so
/// were reported as an overlapping patch and merged, melting a seam this
/// campaign exists to keep drawn. Unreachable on the shipped map (every
/// wall is axis-aligned or quarter-turned, where world axes and edge
/// normals coincide) but reachable the moment a prop rotates freely,
/// which this vocabulary allows by design.
fn min_width(poly: &[Pt2]) -> f64 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut narrowest = f64::INFINITY;
    for i in 0..n {
        let (ax, ay) = poly[i];
        let (bx, by) = poly[(i + 1) % n];
        let (ex, ey) = (bx - ax, by - ay);
        let len = (ex * ex + ey * ey).sqrt();
        if len < 1e-15 {
            continue; // a degenerate (zero-length) edge names no direction
        }
        // the outward normal of this edge, unit length
        let (nx, ny) = (-ey / len, ex / len);
        let mut min_p = f64::INFINITY;
        let mut max_p = f64::NEG_INFINITY;
        for &(px, py) in poly {
            let projected = px * nx + py * ny;
            min_p = min_p.min(projected);
            max_p = max_p.max(projected);
        }
        let width = max_p - min_p;
        if width < narrowest {
            narrowest = width;
        }
    }
    if narrowest.is_finite() {
        narrowest
    } else {
        0.0
    }
}

/// The merge test's overlap half: project both (near-parallel) polygons
/// to a valid 2-D representation of their shared tangent plane (WORLD
/// axes, chosen only to keep the projection well-conditioned — see
/// [`dominant_axis`]; their orientation carries no other meaning), clip
/// one against the other to the exact convex intersection, and require
/// that intersection's OWN minimum width — over every direction, not just
/// the two world axes the projection happened to keep — to exceed
/// [`PATCH_EPS`]. An edge-only touch, at any rotation, has zero width in
/// the direction perpendicular to the shared edge and is correctly
/// refused; a genuine 2-D patch has a real minimum width in every
/// direction and passes.
fn polygon_overlap_exceeds_patch(a: &Face, b: &Face) -> bool {
    let axis = dominant_axis(a.normal);
    let pa = ensure_ccw(project_to_plane(&a.poly, axis));
    let pb = ensure_ccw(project_to_plane(&b.poly, axis));
    if pa.len() < 3 || pb.len() < 3 {
        return false;
    }
    let inter = clip_convex(&pa, &pb);
    min_width(&inter) > PATCH_EPS
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

    /// Wave S: a standalone unit box, no partner at all, is alone in its
    /// own cluster — the SINGLETON COLLAPSE folds all six of its faces
    /// into ONE class, restoring the spec's own law ("singletons keep
    /// today's exact look: one label across the whole solid, one
    /// silhouette, no interior lines"). Before the collapse landed, this
    /// exact fixture demanded six MUTUALLY SEPARATED classes — rule (a)
    /// applied with no multi-member scoping at all — which is the shape
    /// of the 93-class starvation measured on the shipped map, reproduced
    /// at its simplest: a lone box that touches nothing still cost three
    /// labels for itself.
    #[test]
    fn a_lone_box_yields_one_class() {
        let f = faces(
            0,
            &Shape::Box3d {
                center: [0.0; 3],
                size: [1.0; 3],
                basis: IDENTITY,
            },
        );
        let sf = superfaces(&f, &[]);
        assert_eq!(sf.classes, 1);
        assert_eq!(sf.class_of, vec![0; 6]);
        assert!(sf.separations.is_empty());
    }

    /// Wave S: two boxes standing side by side — touching (they share the
    /// plane x=0.5) but never merging, since a's own +X face and b's own
    /// -X face point OPPOSITE ways (an ordinary abutment, not a coplanar
    /// same-direction overlap) — are each their own singleton cluster.
    /// The pre-superface two-label law, restored exactly: two classes,
    /// one separation between them, not the six-cross-pair demand an
    /// unscoped rule (a) plus rule (c)'s blanket law would make.
    #[test]
    fn two_touching_singleton_boxes_yield_two_classes_and_one_separation() {
        let a = Shape::Box3d {
            center: [0.0, 0.0, 0.0],
            size: [1.0, 1.0, 1.0],
            basis: IDENTITY,
        };
        let b = Shape::Box3d {
            center: [1.0, 0.0, 0.0],
            size: [1.0, 1.0, 1.0],
            basis: IDENTITY,
        };
        let mut all = faces(0, &a);
        all.extend(faces(1, &b));
        assert_eq!(all[1].normal, [1.0, 0.0, 0.0]);
        assert_eq!(all[1].offset, 0.5);
        assert_eq!(all[6].normal, [-1.0, 0.0, 0.0]);
        assert_eq!(all[6].offset, -0.5);

        let sf = superfaces(&all, &[(0, 1)]);
        assert_eq!(sf.classes, 2);
        assert_ne!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);
        assert_ne!(sf.class_of[0], sf.class_of[6]);
        assert_eq!(
            sf.separations,
            vec![ordered(sf.class_of[0], sf.class_of[6])]
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
    /// `touching` at all. Wave S: this lone box is a singleton (nothing
    /// merges with it), so both calls now collapse it to ONE class with
    /// NO separations — the singleton collapse runs regardless of
    /// `touching` (it depends only on cluster membership from the merge
    /// pass), so the bogus pairs' no-op-ness is unaffected by the fix,
    /// only the concrete numbers this test pins are.
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
        assert_eq!(with_bogus_pairs.classes, 1);
        assert!(with_bogus_pairs.separations.is_empty());
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
        // rule (a) cannot have produced a separation for it — its own
        // `faces[i].solid == faces[j].solid` gate rules the pair out
        // before it ever reaches `add_separation`'s own (defensive, and
        // on this call path unreachable) refusal to separate a class
        // from itself.
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

    /// Review finding (IMPORTANT 1): the merge test must be robust to
    /// ROTATION, not just axis-aligned rectangles. Two coplanar diamonds
    /// (unit squares turned 45 degrees about Y) sharing exactly one FULL
    /// EDGE and nothing else — `b` is `a` reflected across the line
    /// containing that shared edge, so their interiors sit on opposite
    /// sides of it and the true overlap AREA is zero. Hand-derived: `a`'s
    /// edge from (x,z) = (1,0) to (0,1) lies on the line x+z=1; reflecting
    /// `a`'s center (0,0) across that line (`P' = P - 2*((P.n-c)/|n|^2)*n`
    /// with n=(1,1), c=1) gives `(1,1)`, which is exactly `b`'s center
    /// below — confirmed independently by checking `b`'s own edge (0,1)-
    /// (1,0) is the identical segment, not by running the code.
    ///
    /// A world-axis bounding box of the shared edge's two points, (1,0)
    /// and (0,1), spans x in [0,1] and z in [0,1] — BOTH exceed
    /// `PATCH_EPS` — so measuring "both extents" against WORLD axes
    /// reports a patch where there is only a bend, and melts a seam this
    /// campaign exists to keep. Verified empirically before the fix: the
    /// pre-fix `polygon_overlap_exceeds_patch` returned `du=1, dv=1`,
    /// `is_merge_candidate` = true.
    #[test]
    fn a_45_degree_rotated_shared_edge_does_not_merge() {
        let a = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 5.0,
            poly: vec![
                [1.0, 5.0, 0.0],
                [0.0, 5.0, 1.0],
                [-1.0, 5.0, 0.0],
                [0.0, 5.0, -1.0],
            ],
            solid: 0,
        };
        let b = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 5.0,
            poly: vec![
                [2.0, 5.0, 1.0],
                [1.0, 5.0, 2.0],
                [0.0, 5.0, 1.0],
                [1.0, 5.0, 0.0],
            ],
            solid: 1,
        };
        assert!(!is_merge_candidate(&a, &b));
    }

    /// The rotated-diamond fixture's positive control: shrink `b` so it
    /// genuinely overlaps `a`'s interior (not just their shared edge) —
    /// `b`'s center moved from (1,1) to (0.7,0.7), keeping the same
    /// diamond shape and orientation, now overlapping `a` in a real 2-D
    /// wedge near (1,0)-(0,1) rather than just touching there. This must
    /// still merge, proving the rotation-invariant width fix does not
    /// simply refuse every rotated pair.
    #[test]
    fn a_45_degree_rotated_genuine_overlap_still_merges() {
        let a = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 5.0,
            poly: vec![
                [1.0, 5.0, 0.0],
                [0.0, 5.0, 1.0],
                [-1.0, 5.0, 0.0],
                [0.0, 5.0, -1.0],
            ],
            solid: 0,
        };
        let b_overlapping = Face {
            normal: [0.0, 1.0, 0.0],
            offset: 5.0,
            poly: vec![
                [1.7, 5.0, 0.7],
                [0.7, 5.0, 1.7],
                [-0.3, 5.0, 0.7],
                [0.7, 5.0, -0.3],
            ],
            solid: 1,
        };
        assert!(is_merge_candidate(&a, &b_overlapping));
    }

    /// Review finding (IMPORTANT 2): mutating rule (b)'s buried-abutment
    /// exclusion to a no-op (`is_opposite_facing_coplanar` never
    /// consulted) left the full suite green, because neither existing
    /// abutment fixture puts its pair in the SAME cluster — both land in
    /// rule (c) instead, which never calls that exclusion at all. This
    /// fixture forces rule (b) itself to make the call.
    ///
    /// Three solids, two genuine (same-direction, overlapping) merges and
    /// one genuine (opposite-direction, overlapping) abutment:
    /// - `slab` (0): x[-2,2], y[-1,1], z[-2,2].
    /// - `post` (1): x[0,0.5], y[-1,2], z[-0.25,0.25] — tall enough to
    ///   reach past `slab`'s own top; merges with `slab` via its OWN -Y
    ///   face (offset 1, matching `slab`'s -Y, footprint [0,0.5]x[-0.25,
    ///   0.25] fully inside `slab`'s [-2,2]x[-2,2]).
    /// - `lid` (2): x[0,2], y[1,2], z[-1,1] — rests flush on `slab`'s top
    ///   (its own -Y at y=1, offset -1, opposite `slab`'s +Y at offset 1,
    ///   sum 0 — a genuine buried abutment, footprint [0,2]x[-1,1] fully
    ///   inside `slab`'s own top); joins the CLUSTER only through `post`,
    ///   via `lid`'s OWN -X face (offset 0, matching `post`'s -X,
    ///   footprint y[1,2]xz[-0.25,0.25] genuinely inside `post`'s y[-1,2]
    ///   xz[-0.25,0.25]) — `lid` never merges with `slab` directly, so
    ///   this specifically exercises the SAME-CLUSTER-via-a-third-solid
    ///   case, not a direct pairwise merge masking the abutment.
    ///
    /// `slab` and `lid` are TOUCHING (share the plane y=1 over a real
    /// footprint) and, once `post` links them, in the SAME cluster — so
    /// rule (b), not rule (c), decides their fate, and the buried
    /// abutment between them (`slab`'s +Y class, `lid`'s -Y class) must
    /// get NO separation entry.
    #[test]
    fn a_same_cluster_abutment_at_nonzero_offset_produces_no_separation() {
        let slab = Shape::Box3d {
            center: [0.0, 0.0, 0.0],
            size: [4.0, 2.0, 4.0],
            basis: IDENTITY,
        };
        let post = Shape::Box3d {
            center: [0.25, 0.5, 0.0],
            size: [0.5, 3.0, 0.5],
            basis: IDENTITY,
        };
        let lid = Shape::Box3d {
            center: [1.0, 1.5, 0.0],
            size: [2.0, 1.0, 2.0],
            basis: IDENTITY,
        };
        let mut all = faces(0, &slab);
        all.extend(faces(1, &post));
        all.extend(faces(2, &lid));

        // slab's +Y (global 3) and lid's -Y (global 14: lid's local index
        // 2 in -X,+X,-Y,+Y,-Z,+Z order, offset by post's 6 faces)
        assert_eq!(all[3].normal, [0.0, 1.0, 0.0]);
        assert_eq!(all[3].offset, 1.0);
        assert_eq!(all[14].normal, [0.0, -1.0, 0.0]);
        assert_eq!(all[14].offset, -1.0);
        assert!(is_opposite_facing_coplanar(&all[3], &all[14]));

        let touching = [(0, 1), (1, 2), (0, 2)];
        let sf = superfaces(&all, &touching);

        // the two merges that build the cluster: post~slab, lid~post
        assert_eq!(sf.class_of[8], sf.class_of[2]); // post's -Y ~ slab's -Y
        assert_eq!(sf.class_of[12], sf.class_of[6]); // lid's -X ~ post's -X
        assert_eq!(sf.cluster_of_solid[0], sf.cluster_of_solid[2]);

        // the abutment itself never merges...
        assert_ne!(sf.class_of[3], sf.class_of[14]);
        // ...and, because it's a buried abutment inside a same-cluster
        // touching pair, rule (b) must not separate it either.
        assert!(
            !sf.separations
                .contains(&ordered(sf.class_of[3], sf.class_of[14]))
        );
    }

    /// Review finding (IMPORTANT 3): mutating the (b)/(c) dispatch so
    /// every touching pair takes rule (c)'s blanket law (ignoring
    /// `cluster_of_solid` entirely) left the full suite green, because
    /// every existing separation assertion is a PRESENCE check — a
    /// dispatch bug that separates MORE than it should never trips one.
    /// This is the absence check: on the shipped map's own shape (the
    /// 17-member wall network, junctioned end to end), rule (b) governs
    /// almost every wall pair, and it must NOT blanket-separate class
    /// combinations that never actually touch just because their SOLIDS
    /// happen to share a cluster.
    ///
    /// The junction fixture's merged cap class (wall A's south flank,
    /// wall B's south cap — global 4 and 10, the SAME class after the
    /// merge test above) is nowhere near wall B's own far north cap
    /// (global 11, z=4.15 — 4.3 m up wall B's own run) or wall A's own
    /// far west end (global 0, x=-2.15). A blanket "every class of A
    /// against every class of B" law would separate both pairs anyway;
    /// the fine-grained rule (b) must not.
    #[test]
    fn same_cluster_pairs_that_never_touch_are_not_blanket_separated() {
        let all = junction_faces();
        let sf = superfaces(&all, &[(0, 1)]);
        assert_eq!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);
        assert_eq!(sf.class_of[4], sf.class_of[10]);

        assert!(
            !sf.separations
                .contains(&ordered(sf.class_of[4], sf.class_of[11]))
        );
        assert!(
            !sf.separations
                .contains(&ordered(sf.class_of[0], sf.class_of[11]))
        );
    }

    /// `polygons_within_patch_eps`'s own boundary, INCLUSIVE — the
    /// mirror image of the merge test's EXCLUSIVE boundary
    /// (`an_overlap_of_exactly_patch_eps_is_an_edge_not_a_patch`), pinned
    /// directly on the private distance helper since the two uses of
    /// `PATCH_EPS` now read oppositely and each needs its own witness.
    /// Two directly-stacked unit squares, one at y=0 and one at
    /// y=0.001 — footprints identical, so the closest boundary EDGES sit
    /// directly above/below each other and the gap is exactly `0.001 -
    /// 0.0`, exact by the same zero-subtraction argument the other
    /// boundary tests use. At that exact gap the pair still counts as
    /// close; one bit further (`0.0011`) it must not.
    #[test]
    fn polygons_within_patch_eps_is_inclusive_at_its_own_boundary() {
        let a: Vec<[f64; 3]> = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];
        let at_the_boundary: Vec<[f64; 3]> = vec![
            [0.0, 0.001, 0.0],
            [1.0, 0.001, 0.0],
            [1.0, 0.001, 1.0],
            [0.0, 0.001, 1.0],
        ];
        assert!(polygons_within_patch_eps(&a, &at_the_boundary));

        let just_past: Vec<[f64; 3]> = vec![
            [0.0, 0.0011, 0.0],
            [1.0, 0.0011, 0.0],
            [1.0, 0.0011, 1.0],
            [0.0, 0.0011, 1.0],
        ];
        assert!(!polygons_within_patch_eps(&a, &just_past));
    }
}
