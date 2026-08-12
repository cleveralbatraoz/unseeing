//! The object-id budget, checked and explained.
//!
//! Law #2: where two objects interpenetrate there is no depth step, so a
//! difference in the flat object id is the ONLY thing that can draw their
//! seam. Two touching solids sharing an id melt into one shape. This
//! reports the touch graph, the id handed to each solid, and every pair
//! closer than `oid_palette::MIN_SEP` — the SOLID-granularity law. It
//! stays true for any two solids that never coplanar-MERGE
//! (`render::superface`): the singleton collapse means a solid alone in
//! its own cluster carries exactly one label across every one of its own
//! faces, so a single bridged read genuinely speaks for the whole solid.
//!
//! [`coplanar_label_faults`] is the superface campaign's own postcondition
//! at FACE granularity, replacing the eye-band-gated, threshold-faded
//! z-fight census this module used to carry before the merge law existed.
//! Two same-facing, coplanar, genuinely overlapping faces rasterise the
//! same pixels — the exact geometry `render::superface::superfaces`
//! merges into one class before a label is ever handed out — so a healthy
//! level can never disagree with itself here: any pair this census flags
//! is a pair the merge law was supposed to have already fused, and the
//! shipped map holds it at zero. It reuses the merge law's own predicate
//! (`render::superface::is_merge_candidate`) rather than a second,
//! hand-rolled copy of "coplanar and overlapping" — this module used to
//! carry its own `COPLANAR_EPS`/`PATCH_EPS` pair, now `render::superface`'s
//! alone — so the two questions, "would this pair merge" and "does this
//! pair actually z-fight", can never drift apart by one file forgetting to
//! update the other.

use crate::oid_palette::{Box3, MIN_SEP, separated};
use crate::render::Face;
use crate::render::superface::is_merge_candidate;

/// Two solids that touch, and whether the seam between them draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchPair {
    pub a: usize,
    pub b: usize,
    pub oid_a: f64,
    pub oid_b: f64,
    pub delta: f64,
    /// True when the ids are at least `MIN_SEP` apart.
    pub draws: bool,
}

/// The touch graph with its colouring checked.
#[derive(Debug, Clone, PartialEq)]
pub struct OidExplanation {
    /// Every touching pair, each reported once, in ascending (a, b) order.
    pub pairs: Vec<TouchPair>,
    /// Indices INTO `pairs` whose seam does not draw.
    pub violations: Vec<usize>,
    pub min_sep: f64,
}

/// Explain the colouring, or refuse.
///
/// Returns `None` when `oids` is shorter than `boxes`: a truncated check
/// that reported no violations would be a vacuous pass, and the caller
/// could not tell it apart from a clean level.
#[must_use]
pub fn explain_oids_checked(boxes: &[Box3], oids: &[f64]) -> Option<OidExplanation> {
    if oids.len() < boxes.len() {
        return None;
    }
    Some(explain_oids(boxes, oids))
}

/// Explain the colouring of a level whose ids are known to be complete.
///
/// PRIVATE, and deliberately so: this is the half of the pair that panics,
/// and the crate's doctrine is small TOTAL functions. Its only caller is
/// [`explain_oids_checked`] two lines above, which has already established
/// the invariant; leaving it `pub` published a panic to every consumer of
/// `crate::observe` and gave the boundary a second door to walk through by
/// mistake. The assert survives as the invariant's own statement — reached
/// only if this file breaks it.
///
/// # Panics
///
/// If `oids` is shorter than `boxes`.
#[must_use]
fn explain_oids(boxes: &[Box3], oids: &[f64]) -> OidExplanation {
    assert!(oids.len() >= boxes.len(), "one oid per box is required");
    let mut pairs = Vec::new();
    for a in 0..boxes.len() {
        for b in (a + 1)..boxes.len() {
            if !boxes[a].touches(&boxes[b]) {
                continue;
            }
            pairs.push(TouchPair {
                a,
                b,
                oid_a: oids[a],
                oid_b: oids[b],
                delta: (oids[a] - oids[b]).abs(),
                draws: separated(oids[a], oids[b]),
            });
        }
    }
    let violations = pairs
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.draws)
        .map(|(i, _)| i)
        .collect();
    OidExplanation {
        pairs,
        violations,
        min_sep: MIN_SEP,
    }
}

/// Two same-facing, coplanar, genuinely overlapping faces whose labels are
/// NOT bit-identical — the two writers of a shared pixel disagreeing on
/// what colour it is. `a`/`b` index into whichever `faces` slice
/// [`coplanar_label_faults`] was called with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelFault {
    pub a: usize,
    pub b: usize,
    pub label_a: f64,
    pub label_b: f64,
}

/// Census every same-facing, coplanar, genuinely overlapping pair of
/// `faces` whose `labels` disagree, or refuse.
///
/// The predicate is [`is_merge_candidate`] — the IDENTICAL test
/// `render::superface::superfaces` uses to decide which faces share a
/// class — so a reported fault names a genuine defect: somewhere between
/// the class graph and the label actually carried for each face, the
/// merge law's own promise (bit-identical labels for anything it merged)
/// was broken. No eye band, no crease threshold, any plane: this census
/// answers a simpler question than the old fight census did, because the
/// merge law already resolved the geometry question for it.
///
/// Returns `None` when `labels` is shorter than `faces`: a truncated
/// check that reported no faults would be a vacuous pass indistinguishable
/// from a clean level.
#[must_use]
pub fn coplanar_label_faults(faces: &[Face], labels: &[f64]) -> Option<Vec<LabelFault>> {
    if labels.len() < faces.len() {
        return None;
    }
    let mut faults = Vec::new();
    for i in 0..faces.len() {
        for j in (i + 1)..faces.len() {
            if is_merge_candidate(&faces[i], &faces[j])
                && labels[i].to_bits() != labels[j].to_bits()
            {
                faults.push(LabelFault {
                    a: i,
                    b: j,
                    label_a: labels[i],
                    label_b: labels[j],
                });
            }
        }
    }
    Some(faults)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid_palette::Box3;

    fn unit_at(x: f64) -> Box3 {
        Box3::from_center_size([x, 0.5, 0.0], [1.0, 1.0, 1.0])
    }

    /// Two touching boxes with identical ids melt into one silhouette:
    /// the crease is the ONLY line between interpenetrating solids, and it
    /// comes from a difference in the flat object id. Delta 0 means no
    /// line, and the explanation must say so as a violation.
    #[test]
    fn touching_boxes_with_equal_ids_are_a_violation() {
        let boxes = [unit_at(0.0), unit_at(0.9)];
        let e = explain_oids(&boxes, &[0.24, 0.24]);
        assert_eq!(e.pairs.len(), 1);
        assert_eq!(e.pairs[0].delta, 0.0);
        assert!(!e.pairs[0].draws);
        assert_eq!(e.violations, vec![0]);
    }

    /// Exactly at the minimum separation the seam draws — the law is
    /// "at least 0.08", not "more than". Hand-derived: 0.32 - 0.24 = 0.08.
    #[test]
    fn the_minimum_separation_itself_draws() {
        let boxes = [unit_at(0.0), unit_at(0.9)];
        let e = explain_oids(&boxes, &[0.24, 0.32]);
        assert!((e.pairs[0].delta - 0.08).abs() < 1e-9);
        assert!(e.pairs[0].draws);
        assert!(e.violations.is_empty());
    }

    /// Boxes that do not touch are not pairs at all. Two solids across the
    /// room share an id harmlessly — the budget would be unusable
    /// otherwise, and reporting them would bury the real violations.
    #[test]
    fn distant_boxes_with_equal_ids_are_not_reported() {
        let boxes = [unit_at(0.0), unit_at(50.0)];
        let e = explain_oids(&boxes, &[0.24, 0.24]);
        assert!(e.pairs.is_empty());
        assert!(e.violations.is_empty());
    }

    /// Every touching pair is reported once, not twice — a-b and b-a are
    /// the same seam. Three mutually touching boxes give three pairs.
    #[test]
    fn each_seam_is_reported_once() {
        // Spacing 0.4 keeps every pair — including the two-hop 0-2 pair —
        // inside the unit boxes' 1.0-wide touch range with room to spare,
        // so all three are genuinely mutually touching (unlike a 0.9
        // spacing, which only makes ADJACENT boxes touch: see
        // oid_palette::tests::a_long_chain_alternates_without_starving).
        let boxes = [unit_at(0.0), unit_at(0.4), unit_at(0.8)];
        let e = explain_oids(&boxes, &[0.0, 0.16, 0.32]);
        let ids: Vec<(usize, usize)> = e.pairs.iter().map(|p| (p.a, p.b)).collect();
        assert_eq!(ids, vec![(0, 1), (0, 2), (1, 2)]);
    }

    /// A short oid list cannot be explained. Reporting zero pairs here
    /// would be a vacuous pass — the caller would read "no violations"
    /// from an input that was never checked.
    #[test]
    fn a_short_oid_list_is_refused_not_silently_truncated() {
        let boxes = [unit_at(0.0), unit_at(0.9)];
        assert!(explain_oids_checked(&boxes, &[0.24]).is_none());
        assert!(explain_oids_checked(&boxes, &[0.24, 0.32]).is_some());
    }

    // -----------------------------------------------------------------
    // coplanar_label_faults — the new, face-granularity postcondition
    // replacing the eye-band/threshold fight census above.
    // -----------------------------------------------------------------

    use crate::render::Face;

    /// Two axis-aligned unit squares on the world plane `z = offset`, both
    /// facing +Z, centered on X at `a_x`/`b_x` and spanning y in
    /// [-0.5, 0.5] — hand-built `Face` literals, never routed through
    /// `render::faces::faces`, so the fixture cannot inherit a bug from the
    /// code that predicate is meant to police.
    fn coplanar_z_faces(offset: f64, a_x: f64, b_x: f64) -> [Face; 2] {
        let square = |cx: f64| {
            vec![
                [cx - 0.5, -0.5, offset],
                [cx + 0.5, -0.5, offset],
                [cx + 0.5, 0.5, offset],
                [cx - 0.5, 0.5, offset],
            ]
        };
        [
            Face {
                normal: [0.0, 0.0, 1.0],
                offset,
                poly: square(a_x),
                solid: 0,
            },
            Face {
                normal: [0.0, 0.0, 1.0],
                offset,
                poly: square(b_x),
                solid: 1,
            },
        ]
    }

    /// THE law: two faces the merge test would fuse into one class, caught
    /// instead with disagreeing labels — the exact defect this census
    /// exists to name.
    #[test]
    fn a_same_facing_coplanar_overlap_with_unequal_labels_is_a_fault() {
        let [a, b] = coplanar_z_faces(0.5, 0.0, 0.0);
        let faults = coplanar_label_faults(&[a, b], &[0.24, 0.33]).unwrap();
        assert_eq!(faults.len(), 1);
        assert_eq!((faults[0].a, faults[0].b), (0, 1));
        assert_eq!((faults[0].label_a, faults[0].label_b), (0.24, 0.33));
    }

    /// The healthy twin of the fixture above: the SAME geometry, but the
    /// labels agree — exactly what a genuine merge produces, and exactly
    /// what must NOT be reported.
    #[test]
    fn the_identical_pair_with_equal_labels_is_not_a_fault() {
        let [a, b] = coplanar_z_faces(0.5, 0.0, 0.0);
        assert!(
            coplanar_label_faults(&[a, b], &[0.24, 0.24])
                .unwrap()
                .is_empty()
        );
    }

    /// "Bit-equal" is the literal contract, not `==`: -0.0 and 0.0 compare
    /// equal under IEEE 754 but are two different bit patterns, and a
    /// shader reading G off two vertices with those two patterns is NOT
    /// guaranteed to draw them identically. A predicate written as a plain
    /// `!=` would silently wave this pair through.
    #[test]
    fn negative_and_positive_zero_are_not_bit_equal() {
        let [a, b] = coplanar_z_faces(0.5, 0.0, 0.0);
        let faults = coplanar_label_faults(&[a, b], &[0.0, -0.0]).unwrap();
        assert_eq!(faults.len(), 1);
    }

    /// Perpendicular faces are never merge candidates, however far apart
    /// their labels sit — the predicate must gate on geometry FIRST, never
    /// report a fault just because two labels differ.
    #[test]
    fn perpendicular_faces_never_fault_however_their_labels_differ() {
        let [a, _] = coplanar_z_faces(0.5, 0.0, 0.0);
        let b = Face {
            normal: [1.0, 0.0, 0.0],
            offset: 0.5,
            poly: vec![
                [0.5, -0.5, 0.0],
                [0.5, -0.5, 1.0],
                [0.5, 0.5, 1.0],
                [0.5, 0.5, 0.0],
            ],
            solid: 1,
        };
        assert!(
            coplanar_label_faults(&[a, b], &[0.24, 0.99])
                .unwrap()
                .is_empty()
        );
    }

    /// A buried abutment — same plane, OPPOSITE normals — is not a merge
    /// candidate either: `is_merge_candidate` requires SAME direction, so
    /// this must never fault no matter how the labels differ.
    #[test]
    fn opposite_facing_abutment_never_faults() {
        let [a, mut b] = coplanar_z_faces(0.5, 0.0, 0.0);
        b.normal = [0.0, 0.0, -1.0];
        b.offset = -0.5; // the same plane, described from the other side
        assert!(
            coplanar_label_faults(&[a, b], &[0.24, 0.99])
                .unwrap()
                .is_empty()
        );
    }

    /// Coplanar, same-facing, but DISJOINT — a doorway gap between two
    /// collinear wall segments — never faults: no rasterised area is ever
    /// shared, so disagreeing labels there are harmless.
    #[test]
    fn coplanar_same_facing_but_disjoint_faces_never_fault() {
        let [a, b] = coplanar_z_faces(0.5, 0.0, 5.0);
        assert!(
            coplanar_label_faults(&[a, b], &[0.24, 0.99])
                .unwrap()
                .is_empty()
        );
    }

    /// A short label list cannot be censused. Reporting zero faults from
    /// an input that was never checked would read as a clean level — the
    /// same vacuous-pass doctrine every other census in this module holds
    /// to. The boundary is EXACT arity, not merely "long enough": pinned
    /// below by the labels-len-equals-faces-len case reporting `Some`.
    #[test]
    fn a_short_label_list_is_refused_not_silently_truncated() {
        let [a, b] = coplanar_z_faces(0.5, 0.0, 0.0);
        assert!(coplanar_label_faults(&[a.clone(), b.clone()], &[0.24]).is_none());
        assert!(coplanar_label_faults(&[a, b], &[0.24, 0.33]).is_some());
    }

    /// Three faces, one genuine coplanar pair and one far-flung stranger:
    /// every pair is checked once, and only the genuine one is reported —
    /// the same "each seam once, no false positives" discipline
    /// `explain_oids`'s own tests hold `TouchPair` to.
    #[test]
    fn each_pair_is_checked_once_and_only_genuine_pairs_fault() {
        let [a, b] = coplanar_z_faces(0.5, 0.0, 0.0);
        let mut c = a.clone();
        c.solid = 2;
        c.poly = c.poly.iter().map(|p| [p[0] + 20.0, p[1], p[2]]).collect();
        let faults = coplanar_label_faults(&[a, b, c], &[0.24, 0.33, 0.99]).unwrap();
        assert_eq!(faults.len(), 1);
        assert_eq!((faults[0].a, faults[0].b), (0, 1));
    }

    /// End to end, through the REAL pipeline: the issue-14 wall junction
    /// (`render::superface::tests::a_junction_cap_merges_into_the_partners_flank`'s
    /// own fixture), carried through `render::superfaces` and
    /// `render::labels::assign` exactly as `WaveLevel::paint_labels` does,
    /// must report NO faults — the postcondition holding on a realistic
    /// map, not only on the hand-built adversarial fixtures above. This is
    /// expected to pass by construction (the merge law and this census
    /// share one predicate), and is kept anyway as the regression pin a
    /// future change to either side could quietly break.
    #[test]
    fn a_genuine_superface_merge_reports_no_fault_end_to_end() {
        use crate::render::faces::{Shape, faces};
        use crate::render::{labels, superface};

        const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        const PALETTE: [f64; 5] = [0.25, 0.34, 0.43, 0.52, 0.61];

        let wall_a = Shape::Box3d {
            center: [0.0, 1.5, 0.0],
            size: [4.3, 3.0, 0.3],
            basis: IDENTITY,
        };
        let wall_b = Shape::Box3d {
            center: [0.0, 1.5, 2.0],
            size: [0.3, 3.0, 4.3],
            basis: IDENTITY,
        };
        let mut all = faces(0, &wall_a);
        all.extend(faces(1, &wall_b));

        let sf = superface::superfaces(&all, &[(0, 1)]);
        let out = labels::assign(&sf, &[], &PALETTE);
        let per_face_labels: Vec<f64> = (0..all.len())
            .map(|fi| out.label_of_class[sf.class_of[fi]])
            .collect();

        assert!(
            coplanar_label_faults(&all, &per_face_labels)
                .unwrap()
                .is_empty()
        );
    }
}
