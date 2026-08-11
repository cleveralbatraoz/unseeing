//! The object-id budget, checked and explained.
//!
//! Law #2: where two objects interpenetrate there is no depth step, so a
//! difference in the flat object id is the ONLY thing that can draw their
//! seam. Two touching solids sharing an id melt into one shape. This
//! reports the touch graph, the id handed to each solid, and every pair
//! closer than `oid_palette::MIN_SEP`.

use crate::oid_palette::{Box3, MIN_SEP, separated};

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

/// Faces closer than this are coplanar to the renderer, INCLUSIVE: below
/// a millimetre the compatibility renderer's 24-bit depth buffer cannot
/// tell the two faces apart at the map's far range, so somewhere across
/// the patch they tie and per-pixel interpolation noise picks the winner.
/// The wall fix — [`crate::level_plan::CAP_INSET`], which buries every
/// arriving wall cap 5 mm inside its junction partner — stands five of
/// these clear of the band.
pub const COPLANAR_EPS: f64 = 1e-3;

/// The crease floor: `smoothstep(0.04, 0.08, nrm)` in
/// `game/shaders/hearing_post.gdshader:74`. A floor, and honestly a
/// heuristic one: `nrm` (line 73) is a SUM of two opposite-tap
/// differences, so inside speckle both taps can straddle the noise and a
/// delta in (0.02, 0.04] can cross 0.04 after all. Such a pair is still
/// refused here — safely, because a coplanar overlap implies touching
/// boxes, and a touching pair closer than `MIN_SEP` = 0.08 is already
/// flagged as a violation by the colouring census. Every speckle this
/// floor waves through is a broken seam by definition, and named as one.
pub const CREASE_FLOOR: f64 = 0.04;

/// Two faces' rectangles must overlap by MORE than this (exclusive) in
/// BOTH tangent axes to make a visible patch — less is an edge, not a
/// patch.
const PATCH_EPS: f64 = 1e-3;

/// Two solids whose same-facing faces share a plane and rasterise the same
/// pixels — the z-fight the depth buffer resolves per-pixel, speckling the
/// packed id in G between `oid_a` and `oid_b` wherever a wave reveals it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fight {
    pub a: usize,
    pub b: usize,
    /// The shared plane's axis: 0 = X, 1 = Y, 2 = Z. Vertical planes are
    /// always eligible; a horizontal plane censuses only when the eye is
    /// on the side its shared normal faces — an upward (max-max) pair
    /// below the eye, a downward (min-min) pair above it — so
    /// floor-flush bottoms and wall tops never census.
    pub axis: usize,
    /// The plane coordinate, as the lower-indexed box's face names it.
    pub plane: f64,
    pub oid_a: f64,
    pub oid_b: f64,
    pub delta: f64,
}

/// Census the coplanar fights, or refuse.
///
/// `eye` is the eye's world height, which gates horizontal planes only
/// (see [`Fight::axis`]); the shipped constant is
/// `crate::nodes::player::EYE`, handed in by the boundary so this module
/// stays engine-free.
///
/// Returns `None` when `oids` is shorter than `boxes`: a truncated census
/// that reported no fights would be a vacuous pass, and the caller could
/// not tell it apart from a clean level.
#[must_use]
pub fn coplanar_fights_checked(boxes: &[Box3], oids: &[f64], eye: f64) -> Option<Vec<Fight>> {
    if oids.len() < boxes.len() {
        return None;
    }
    Some(coplanar_fights(boxes, oids, eye))
}

/// Census the coplanar fights of a level whose ids are known complete.
///
/// PRIVATE for the same reason [`explain_oids`] is: this is the half that
/// panics, its only caller has already established the invariant, and the
/// assert survives as the invariant's own statement.
///
/// A pair fights when min-face meets min-face or max-face meets max-face
/// (SAME outward normal — min against max is an abutting interface buried
/// between the solids), the plane is one an eye at height `eye` can see
/// at all ([`eye_sees`]), the plane coordinates agree within
/// [`COPLANAR_EPS`], the rectangles overlap by more than [`PATCH_EPS`] in
/// both tangent axes, and the id step exceeds [`CREASE_FLOOR`] so the
/// crease term actually draws the speckle.
///
/// # Panics
///
/// If `oids` is shorter than `boxes`.
#[must_use]
fn coplanar_fights(boxes: &[Box3], oids: &[f64], eye: f64) -> Vec<Fight> {
    assert!(oids.len() >= boxes.len(), "one oid per box is required");
    let mut fights = Vec::new();
    for a in 0..boxes.len() {
        for b in (a + 1)..boxes.len() {
            let delta = (oids[a] - oids[b]).abs();
            // Positive comparison on purpose: a NaN oid exceeds no floor
            // and censuses no fight.
            if delta > CREASE_FLOOR {
                for axis in [0, 1, 2] {
                    let faces = [
                        (false, boxes[a].min[axis], boxes[b].min[axis]),
                        (true, boxes[a].max[axis], boxes[b].max[axis]),
                    ];
                    for (is_max, plane_a, plane_b) in faces {
                        let coplanar = (plane_a - plane_b).abs() <= COPLANAR_EPS;
                        if coplanar
                            && eye_sees(axis, is_max, plane_a, eye)
                            && rectangles_overlap(&boxes[a], &boxes[b], axis)
                        {
                            fights.push(Fight {
                                a,
                                b,
                                axis,
                                plane: plane_a,
                                oid_a: oids[a],
                                oid_b: oids[b],
                                delta,
                            });
                        }
                    }
                }
            }
        }
    }
    fights
}

/// Can an eye at height `eye` see a fighting patch on this plane at all?
/// A vertical plane (X or Z) always shows somewhere on screen. A
/// horizontal plane shows only the side its shared outward normal faces:
/// an upward pair (max against max) needs the eye ABOVE the plane, a
/// downward pair (min against min) needs it BELOW. Strict on purpose —
/// at exactly plane height the plane is seen edge-on, zero projected
/// area — and a NaN eye sees no horizontal plane at all, refusing fights
/// rather than inventing them.
fn eye_sees(axis: usize, is_max: bool, plane: f64, eye: f64) -> bool {
    if axis != 1 {
        return true;
    }
    if is_max { plane < eye } else { plane > eye }
}

/// Do two boxes' face rectangles on `axis` share more than a patch's worth
/// of area — over [`PATCH_EPS`] of overlap along BOTH tangent axes?
fn rectangles_overlap(a: &Box3, b: &Box3, axis: usize) -> bool {
    (0..3).filter(|&t| t != axis).all(|t| {
        let overlap = a.max[t].min(b.max[t]) - a.min[t].max(b.min[t]);
        overlap > PATCH_EPS
    })
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

    /// The big box: min [0,0,0], max [2,2,2]. Its partner is embedded in
    /// its +X half with the SAME max-X plane at x = 2: min [1, 0.5, 0.5],
    /// max [2, 1.5, 1.5]. The shared faces both look down +X and overlap
    /// 1 m × 1 m on Y-Z.
    fn flush_capped_pair() -> [Box3; 2] {
        [
            Box3::from_center_size([1.0, 1.0, 1.0], [2.0, 2.0, 2.0]),
            Box3::from_center_size([1.5, 1.0, 1.0], [1.0, 1.0, 1.0]),
        ]
    }

    /// Two same-facing coplanar max-X faces with overlapping area and ids
    /// 0.09 apart speckle: depth-interpolation noise alternates the raster
    /// winner, the crease term draws the per-pixel jag, and the census
    /// must predict it. A detector that misses this pair — or reports it
    /// twice, or on the wrong axis — reports a clean level that flickers.
    #[test]
    fn same_facing_overlapping_coplanar_x_faces_fight() {
        let boxes = flush_capped_pair();
        let fights = coplanar_fights(&boxes, &[0.24, 0.33], 1.6);
        assert_eq!(fights.len(), 1);
        let f = fights[0];
        assert_eq!((f.a, f.b), (0, 1));
        assert_eq!(f.axis, 0);
        assert_eq!(f.plane, 2.0);
        assert_eq!((f.oid_a, f.oid_b), (0.24, 0.33));
        // Hand-derived: 0.33 - 0.24 = 0.09.
        assert!((f.delta - 0.09).abs() < 1e-9);
    }

    /// The same big box with its partner moved OUTSIDE: the partner's
    /// min-X face sits on the big box's max-X plane at x = 2. Opposite
    /// normals make that an abutting interface buried between the two
    /// solids — nothing rasterises there twice. Pairing min against max
    /// would flag every wall standing on the floor as a fight.
    #[test]
    fn a_max_face_meeting_a_min_face_is_abutment_not_a_fight() {
        let boxes = [
            Box3::from_center_size([1.0, 1.0, 1.0], [2.0, 2.0, 2.0]),
            Box3::from_center_size([2.5, 1.0, 1.0], [1.0, 1.0, 1.0]),
        ];
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], 1.6).is_empty());
    }

    /// The exclusions the old blanket Y ban was really protecting, now
    /// earned honestly. Two crates standing on the same floor share
    /// min-Y at 0 — a downward-facing pair only an UNDERGROUND eye could
    /// see — and two interpenetrating walls share max-Y at 3, an upward
    /// pair far overhead. Census either and every room floods with
    /// fights no standing player can witness.
    #[test]
    fn wall_tops_and_floor_flush_bottoms_hide_from_a_standing_eye() {
        let crate_a = Box3 {
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.2, 1.0],
        };
        let crate_b = Box3 {
            min: [0.5, 0.0, 0.5],
            max: [1.5, 1.0, 1.5],
        };
        assert!(coplanar_fights(&[crate_a, crate_b], &[0.24, 0.33], 1.6).is_empty());
        let wall_a = Box3 {
            min: [0.0, 0.0, 0.0],
            max: [2.0, 3.0, 0.4],
        };
        let wall_b = Box3 {
            min: [1.0, 0.0, 0.1],
            max: [3.0, 3.0, 0.3],
        };
        assert!(coplanar_fights(&[wall_a, wall_b], &[0.24, 0.33], 1.6).is_empty());
    }

    /// A table's top with a flush plate: both max-Y faces at y = 1, a
    /// metre under a 1.6 m eye that looks DOWN onto the shared plane.
    /// The old blanket Y exclusion called this invisible and was wrong —
    /// a census that still skips it reports a clean tabletop that
    /// flickers. Lower the eye to 0.5 m and the plane is overhead: an
    /// upward face shows nothing from below. At exactly plane height the
    /// eye sees the plane edge-on — zero projected area, no patch.
    #[test]
    fn an_upward_pair_below_the_eye_shows_its_fight() {
        let table = Box3::from_center_size([1.0, 0.5, 1.0], [2.0, 1.0, 2.0]);
        let plate = Box3 {
            min: [0.5, 0.2, 0.5],
            max: [1.5, 1.0, 1.5],
        };
        let boxes = [table, plate];
        let fights = coplanar_fights(&boxes, &[0.24, 0.33], 1.6);
        assert_eq!(fights.len(), 1);
        assert_eq!(fights[0].axis, 1);
        assert_eq!(fights[0].plane, 1.0);
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], 0.5).is_empty());
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], 1.0).is_empty());
    }

    /// Two flush undersides overhead: both min-Y faces at y = 2.5, above
    /// a 1.6 m eye that looks UP at the shared plane — a ceiling-mounted
    /// pair fights in plain view. Raise the eye past the plane (2.7 m)
    /// and the downward faces turn away; nothing shows.
    #[test]
    fn a_downward_pair_above_the_eye_shows_its_fight() {
        let slab = Box3 {
            min: [0.0, 2.5, 0.0],
            max: [2.0, 3.0, 2.0],
        };
        let lamp = Box3 {
            min: [0.5, 2.5, 0.5],
            max: [1.5, 2.8, 1.5],
        };
        let boxes = [slab, lamp];
        let fights = coplanar_fights(&boxes, &[0.24, 0.33], 1.6);
        assert_eq!(fights.len(), 1);
        assert_eq!(fights[0].axis, 1);
        assert_eq!(fights[0].plane, 2.5);
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], 2.7).is_empty());
    }

    /// Two crates against the same wall line: both max-Z faces at z = 1,
    /// but their X spans are [0,1] and [2,3] — a metre apart. Coplanarity
    /// without shared area rasterises nothing twice; flagging it would
    /// condemn every row of props sharing a wall.
    #[test]
    fn coplanar_planes_with_disjoint_rectangles_do_not_fight() {
        let boxes = [
            Box3::from_center_size([0.5, 0.5, 0.5], [1.0, 1.0, 1.0]),
            Box3::from_center_size([2.5, 0.5, 0.5], [1.0, 1.0, 1.0]),
        ];
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], 1.6).is_empty());
    }

    /// The crease term is `smoothstep(0.04, 0.08, nrm)`: an id step at or
    /// below 0.04 draws NOTHING, so a speckle between such ids never
    /// reaches the screen and must not be reported. 0.27 - 0.24 = 0.03
    /// stays dark; 0.29 - 0.24 = 0.05 draws; 0.04 - 0.0 = 0.04 sits
    /// exactly on the floor, where smoothstep still returns zero.
    #[test]
    fn ids_at_or_below_the_crease_floor_never_reach_the_screen() {
        let boxes = flush_capped_pair();
        assert!(coplanar_fights(&boxes, &[0.24, 0.27], 1.6).is_empty());
        assert_eq!(coplanar_fights(&boxes, &[0.24, 0.29], 1.6).len(), 1);
        assert!(coplanar_fights(&boxes, &[0.0, 0.04], 1.6).is_empty());
    }

    /// The floor's honest caveat: `nrm` is a SUM of two opposite-tap
    /// differences (`hearing_post.gdshader:73`), so inside speckle both
    /// taps can fire and a delta in (0.02, 0.04] can cross the 0.04
    /// smoothstep floor after all. The census still refuses such pairs —
    /// safely, because coplanar overlap implies touching boxes, and a
    /// touching pair under MIN_SEP = 0.08 is already a violation in the
    /// colouring census. The speckle the fight census waves through, the
    /// seam census names; drop either half and a sub-floor speckle
    /// escapes both.
    #[test]
    fn a_sub_floor_speckle_is_already_a_broken_seam() {
        let boxes = flush_capped_pair();
        // 0.27 - 0.24 = 0.03: under the crease floor, no fight...
        assert!(coplanar_fights(&boxes, &[0.24, 0.27], 1.6).is_empty());
        // ...and under min_sep = 0.08, so the colouring already flags it.
        let e = explain_oids_checked(&boxes, &[0.24, 0.27]).expect("complete ids");
        assert_eq!(e.pairs.len(), 1);
        assert_eq!(e.violations, vec![0]);
    }

    /// A cap five millimetres inside its partner is the wall fix working:
    /// 0.005 apart, the 24-bit depth buffer tells the faces apart
    /// everywhere and there is no tie. Half a millimetre — 0.0005 — is
    /// inside the buffer's confusion range and must be flagged. An eps
    /// read the wrong way round would bless the broken gap and condemn
    /// the fixed one.
    #[test]
    fn a_face_five_millimetres_inside_its_partner_is_not_coplanar() {
        let big = Box3::from_center_size([1.0, 1.0, 1.0], [2.0, 2.0, 2.0]);
        let recessed = Box3 {
            min: [1.0, 0.5, 0.5],
            max: [1.995, 1.5, 1.5],
        };
        assert!(coplanar_fights(&[big, recessed], &[0.24, 0.33], 1.6).is_empty());
        let tied = Box3 {
            min: [1.0, 0.5, 0.5],
            max: [1.9995, 1.5, 1.5],
        };
        let fights = coplanar_fights(&[big, tied], &[0.24, 0.33], 1.6);
        assert_eq!(fights.len(), 1);
        // The lower-indexed box names the plane.
        assert_eq!(fights[0].plane, 2.0);
    }

    /// The coincidence band's own edge: a plane gap of exactly 1e-3 —
    /// 0.001 - 0.0, COPLANAR_EPS to the bit — is still coplanar. The
    /// band is INCLUSIVE: "within a millimetre" keeps its boundary, and
    /// an eps quietly turned exclusive would wave through the widest tie
    /// the depth buffer still cannot resolve.
    #[test]
    fn the_millimetre_of_coincidence_is_inclusive() {
        let jamb = Box3 {
            min: [-1.0, 0.0, 0.0],
            max: [0.001, 2.0, 2.0],
        };
        let panel = Box3 {
            min: [-0.5, 0.5, 0.5],
            max: [0.0, 1.5, 1.5],
        };
        let fights = coplanar_fights(&[jamb, panel], &[0.24, 0.33], 1.6);
        assert_eq!(fights.len(), 1);
        assert_eq!(fights[0].axis, 0);
        assert_eq!(fights[0].plane, 0.001);
    }

    /// The patch threshold's own edge: rectangles sharing exactly 1e-3 m
    /// along one tangent axis — 0.001 - 0.0 to the bit — are an edge,
    /// not a patch, and census nothing. The overlap must EXCEED a
    /// millimetre; a threshold quietly turned inclusive would flag every
    /// prop kissing a neighbour along a seam line. Widen the sliver to
    /// 0.1 m and the same pair fights, so it is the boundary doing the
    /// excluding and not the geometry.
    #[test]
    fn a_millimetre_of_overlap_is_an_edge_not_a_patch() {
        let sliver = Box3 {
            min: [-1.0, 0.0, 3.0],
            max: [0.001, 2.0, 4.0],
        };
        let slab = Box3 {
            min: [0.0, 0.5, 3.5],
            max: [2.0, 1.5, 4.0],
        };
        assert!(coplanar_fights(&[sliver, slab], &[0.24, 0.33], 1.6).is_empty());
        let wide = Box3 {
            min: [-1.0, 0.0, 3.0],
            max: [0.1, 2.0, 4.0],
        };
        let fights = coplanar_fights(&[wide, slab], &[0.24, 0.33], 1.6);
        assert_eq!(fights.len(), 1);
        assert_eq!(fights[0].axis, 2);
    }

    /// A short oid list cannot be censused. Reporting zero fights from an
    /// input that was never checked would read as a clean level — the
    /// same vacuous pass `explain_oids_checked` refuses.
    #[test]
    fn a_short_oid_list_refuses_the_fight_census_too() {
        let boxes = flush_capped_pair();
        assert!(coplanar_fights_checked(&boxes, &[0.24], 1.6).is_none());
        assert!(coplanar_fights_checked(&boxes, &[0.24, 0.33], 1.6).is_some());
    }
}
