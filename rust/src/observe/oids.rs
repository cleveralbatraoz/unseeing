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

/// Faces whose planes sit within this of each other are coplanar to the
/// renderer, INCLUSIVE. The derivation: one 24-bit depth code spans
/// about 1.191e-6·w² metres at eye distance w (near 0.05, far 60 — the
/// player's camera), so a same-facing pair can TIE somewhere only while
/// its gap is under one code at a reachable distance. The shipped map's
/// longest sightline is 34.0 m — a 1.38 mm tie band — and the
/// [`crate::level_plan::DIST_PACK_RANGE`] ceiling (40 m) admits maps
/// whose band reaches 1.9 mm, so the census draws its line at 2 mm:
/// nothing above this gap can tie on any map the pack range admits. This
/// module's own census still treats an exact wall-junction coincidence as
/// a FIGHT (a same-facing coplanar pair with unequal ids); the
/// `render::superface` merge law, promoted from this exact predicate,
/// reads the identical coincidence as the intended MELT instead — the two
/// laws describe the same geometry from either side of the migration.
pub const COPLANAR_EPS: f64 = 2e-3;

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
    /// always eligible; a horizontal plane censuses only when the
    /// walking eye's band reaches the side its shared normal faces — an
    /// upward (max-max) pair below the bob crest, a downward (min-min)
    /// pair above the bob trough — so floor-flush bottoms and wall tops
    /// never census.
    pub axis: usize,
    /// The plane coordinate, as the lower-indexed box's face names it.
    pub plane: f64,
    pub oid_a: f64,
    pub oid_b: f64,
    pub delta: f64,
}

/// The vertical band the walking eye sweeps: the standing height plus
/// and minus the head-bob amplitude. The boundary hands in
/// `player::EYE ± viewmodel::BOB_AMP`, so this pure module never
/// imports the engine layer. The two edges gate independently and both
/// are EXCLUSIVE — at exactly an edge the bob's extreme meets the plane
/// edge-on, zero projected area — and a NaN edge gates its own side
/// shut, refusing fights rather than inventing them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EyeBand {
    /// The bob trough — the lowest height the walking eye reaches.
    pub low: f64,
    /// The bob crest — the highest.
    pub high: f64,
}

/// Census the coplanar fights, or refuse.
///
/// `eye` is the band the walking eye sweeps, which gates horizontal
/// planes only (see [`Fight::axis`]). `skip` marks entries that census
/// no fight at all — the boundary marks swept source envelopes, whose
/// planes rasterise nothing — and a returned fight's indices ALWAYS
/// index the full input lists, skipped entries included, so the
/// caller's parallel name list lines up with no re-keying.
///
/// Returns `None` when `oids` OR `skip` is shorter than `boxes`: a
/// truncated census that reported no fights would be a vacuous pass —
/// and a skip tail padded with "not skipped" would quietly census
/// envelope boxes the moment the boundary miscounts — so both
/// shortfalls refuse, exactly as [`explain_oids_checked`] does.
///
/// KNOWN MISS, accepted and named: the census compares axis-aligned
/// world-box faces, so a freely-rotated flush assembly (props rotate
/// freely by design) fights on an OBLIQUE shared plane this law cannot
/// represent. Outside the census — like the sources' real limbs and the
/// hero's body — not silently covered.
#[must_use]
pub fn coplanar_fights_checked(
    boxes: &[Box3],
    oids: &[f64],
    eye: EyeBand,
    skip: &[bool],
) -> Option<Vec<Fight>> {
    if oids.len() < boxes.len() || skip.len() < boxes.len() {
        return None;
    }
    Some(coplanar_fights(boxes, oids, eye, skip))
}

/// Census the coplanar fights of a level whose ids are known complete.
///
/// PRIVATE for the same reason [`explain_oids`] is: this is the half that
/// panics, its only caller has already established the invariant, and the
/// assert survives as the invariant's own statement.
///
/// A pair fights when neither endpoint is skip-marked, min-face meets
/// min-face or max-face meets max-face (SAME outward normal — min
/// against max is an abutting interface buried between the solids), the
/// plane is one the walking eye's band can face at all ([`eye_sees`]),
/// the plane coordinates agree within [`COPLANAR_EPS`], the rectangles
/// overlap by more than [`PATCH_EPS`] in both tangent axes, and the id
/// step exceeds [`CREASE_FLOOR`] so the crease term actually draws the
/// speckle.
///
/// # Panics
///
/// If `oids` or `skip` is shorter than `boxes`.
#[must_use]
fn coplanar_fights(boxes: &[Box3], oids: &[f64], eye: EyeBand, skip: &[bool]) -> Vec<Fight> {
    assert!(oids.len() >= boxes.len(), "one oid per box is required");
    assert!(
        skip.len() >= boxes.len(),
        "one skip flag per box is required"
    );
    let mut fights = Vec::new();
    for a in 0..boxes.len() {
        for b in (a + 1)..boxes.len() {
            if skip[a] || skip[b] {
                continue;
            }
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

/// Can the walking eye see a fighting patch on this plane at all? A
/// vertical plane (X or Z) always shows somewhere on screen. A
/// horizontal plane shows only the side its shared outward normal
/// faces: an upward pair (max against max) needs some swept eye height
/// ABOVE the plane — the bob crest — and a downward pair (min against
/// min) needs one BELOW — the trough. Strict on purpose: at exactly a
/// band edge the bob's extreme meets the plane edge-on, zero projected
/// area, and a NaN edge sees no horizontal plane at all, refusing
/// fights rather than inventing them.
fn eye_sees(axis: usize, is_max: bool, plane: f64, eye: EyeBand) -> bool {
    if axis != 1 {
        return true;
    }
    if is_max {
        plane < eye.high
    } else {
        plane > eye.low
    }
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

    /// The band the walking eye sweeps, hand-derived from the shipped
    /// constants: 1.6 ± 0.028 (player `EYE`, viewmodel `BOB_AMP`).
    /// Spelled as literals so the law is held to numbers a reviewer can
    /// re-derive, never to whatever the constants happen to be today.
    fn standing() -> EyeBand {
        EyeBand {
            low: 1.572,
            high: 1.628,
        }
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
        let fights = coplanar_fights(&boxes, &[0.24, 0.33], standing(), &[false, false]);
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
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], standing(), &[false, false]).is_empty());
    }

    /// The exclusions the old blanket Y ban was really protecting, now
    /// earned honestly. Two crates standing on the same floor share
    /// min-Y at 0 — a downward-facing pair even the bob trough (1.572)
    /// stands far above — and two interpenetrating walls share max-Y at
    /// 3, an upward pair the bob crest (1.628) never reaches. Census
    /// either and every room floods with fights no walking player can
    /// witness.
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
        assert!(
            coplanar_fights(
                &[crate_a, crate_b],
                &[0.24, 0.33],
                standing(),
                &[false, false]
            )
            .is_empty()
        );
        let wall_a = Box3 {
            min: [0.0, 0.0, 0.0],
            max: [2.0, 3.0, 0.4],
        };
        let wall_b = Box3 {
            min: [1.0, 0.0, 0.1],
            max: [3.0, 3.0, 0.3],
        };
        assert!(
            coplanar_fights(
                &[wall_a, wall_b],
                &[0.24, 0.33],
                standing(),
                &[false, false]
            )
            .is_empty()
        );
    }

    /// A table's top with a flush plate: both max-Y faces at y = 1, well
    /// under the standing band — the eye looks DOWN onto the shared
    /// plane and sees the speckle. The old blanket Y exclusion called
    /// this invisible and was wrong. Crouch the band to [0.472, 0.528]
    /// and the plane is overhead: an upward face shows nothing from
    /// below. With the bob CREST exactly at plane height — band
    /// [0.944, 1.0] — the crest meets the plane edge-on: zero projected
    /// area, no patch, the gate is strict.
    #[test]
    fn an_upward_pair_below_the_eye_shows_its_fight() {
        let table = Box3::from_center_size([1.0, 0.5, 1.0], [2.0, 1.0, 2.0]);
        let plate = Box3 {
            min: [0.5, 0.2, 0.5],
            max: [1.5, 1.0, 1.5],
        };
        let boxes = [table, plate];
        let fights = coplanar_fights(&boxes, &[0.24, 0.33], standing(), &[false, false]);
        assert_eq!(fights.len(), 1);
        assert_eq!(fights[0].axis, 1);
        assert_eq!(fights[0].plane, 1.0);
        let crouched = EyeBand {
            low: 0.472,
            high: 0.528,
        };
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], crouched, &[false, false]).is_empty());
        let crest_on_plane = EyeBand {
            low: 0.944,
            high: 1.0,
        };
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], crest_on_plane, &[false, false]).is_empty());
    }

    /// Two flush undersides overhead: both min-Y faces at y = 2.5, above
    /// the whole standing band — the eye looks UP at the shared plane and
    /// a ceiling-mounted pair fights in plain view. Raise the band past
    /// the plane ([2.672, 2.728]) and the downward faces turn away;
    /// nothing shows.
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
        let fights = coplanar_fights(&boxes, &[0.24, 0.33], standing(), &[false, false]);
        assert_eq!(fights.len(), 1);
        assert_eq!(fights[0].axis, 1);
        assert_eq!(fights[0].plane, 2.5);
        let raised = EyeBand {
            low: 2.672,
            high: 2.728,
        };
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], raised, &[false, false]).is_empty());
    }

    /// The walk bob sweeps the eye through EYE ± BOB_AMP = [1.572,
    /// 1.628] (player.rs, viewmodel.rs), so a gate reading the static
    /// 1.6 leaves a 56 mm blind band. An upward pair at 1.61 — over the
    /// static eye, under the bob crest — IS seen, from the crest of
    /// every step. At exactly the band's top, 1.628, the crest meets the
    /// plane edge-on and nothing shows: the edge is EXCLUSIVE.
    #[test]
    fn the_bob_crest_widens_the_upward_gaze() {
        let counter = Box3 {
            min: [0.0, 0.0, 0.0],
            max: [2.0, 1.61, 2.0],
        };
        let board = Box3 {
            min: [0.5, 1.0, 0.5],
            max: [1.5, 1.61, 1.5],
        };
        let fights = coplanar_fights(
            &[counter, board],
            &[0.24, 0.33],
            standing(),
            &[false, false],
        );
        assert_eq!(fights.len(), 1);
        assert_eq!((fights[0].axis, fights[0].plane), (1, 1.61));
        let tall = Box3 {
            min: [0.0, 0.0, 0.0],
            max: [2.0, 1.628, 2.0],
        };
        let shelf = Box3 {
            min: [0.5, 1.0, 0.5],
            max: [1.5, 1.628, 1.5],
        };
        assert!(
            coplanar_fights(&[tall, shelf], &[0.24, 0.33], standing(), &[false, false]).is_empty()
        );
    }

    /// The bob trough: a downward pair at 1.59 sits under the static
    /// 1.6 m eye but ABOVE the trough of the walk bob (1.572), so the
    /// dipping eye passes below the plane and looks up at it — a
    /// static-eye gate calls it invisible and is wrong. At exactly the
    /// band's bottom the trough meets the plane edge-on: strict again,
    /// so `>` quietly mutated to `>=` dies here.
    #[test]
    fn the_bob_trough_lowers_the_downward_gaze() {
        let soffit = Box3 {
            min: [0.0, 1.59, 0.0],
            max: [2.0, 2.0, 2.0],
        };
        let vent = Box3 {
            min: [0.5, 1.59, 0.5],
            max: [1.5, 1.8, 1.5],
        };
        let fights = coplanar_fights(&[soffit, vent], &[0.24, 0.33], standing(), &[false, false]);
        assert_eq!(fights.len(), 1);
        assert_eq!((fights[0].axis, fights[0].plane), (1, 1.59));
        let low = Box3 {
            min: [0.0, 1.572, 0.0],
            max: [2.0, 2.0, 2.0],
        };
        let cap = Box3 {
            min: [0.5, 1.572, 0.5],
            max: [1.5, 1.8, 1.5],
        };
        assert!(
            coplanar_fights(&[low, cap], &[0.24, 0.33], standing(), &[false, false]).is_empty()
        );
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
        assert!(coplanar_fights(&boxes, &[0.24, 0.33], standing(), &[false, false]).is_empty());
    }

    /// The crease term is `smoothstep(0.04, 0.08, nrm)`: an id step at or
    /// below 0.04 draws NOTHING, so a speckle between such ids never
    /// reaches the screen and must not be reported. 0.27 - 0.24 = 0.03
    /// stays dark; 0.29 - 0.24 = 0.05 draws; 0.04 - 0.0 = 0.04 sits
    /// exactly on the floor, where smoothstep still returns zero.
    #[test]
    fn ids_at_or_below_the_crease_floor_never_reach_the_screen() {
        let boxes = flush_capped_pair();
        assert!(coplanar_fights(&boxes, &[0.24, 0.27], standing(), &[false, false]).is_empty());
        assert_eq!(
            coplanar_fights(&boxes, &[0.24, 0.29], standing(), &[false, false]).len(),
            1
        );
        assert!(coplanar_fights(&boxes, &[0.0, 0.04], standing(), &[false, false]).is_empty());
    }

    /// Totality is a contract, not a hope: a NaN oid exceeds no crease
    /// floor and censuses NOTHING. Rewriting `delta > floor` as its
    /// negation (`!(delta <= floor)`) silently inverts exactly this —
    /// NaN would start fighting — so the contract is pinned, not
    /// implied.
    #[test]
    fn a_nan_oid_censuses_nothing() {
        let boxes = flush_capped_pair();
        assert!(coplanar_fights(&boxes, &[0.24, f64::NAN], standing(), &[false, false]).is_empty());
    }

    /// A NaN eye band sees no horizontal plane — refusing fights rather
    /// than inventing them — while vertical planes, which no eye height
    /// gates, keep censusing. Negating either band comparison would flip
    /// the NaN half of this contract silently, so both halves are pinned
    /// together.
    #[test]
    fn a_nan_eye_censuses_no_horizontal_fight_but_keeps_vertical_ones() {
        let nan_band = EyeBand {
            low: f64::NAN,
            high: f64::NAN,
        };
        let table = Box3::from_center_size([1.0, 0.5, 1.0], [2.0, 1.0, 2.0]);
        let plate = Box3 {
            min: [0.5, 0.2, 0.5],
            max: [1.5, 1.0, 1.5],
        };
        assert!(
            coplanar_fights(&[table, plate], &[0.24, 0.33], nan_band, &[false, false]).is_empty()
        );
        let vertical = coplanar_fights(
            &flush_capped_pair(),
            &[0.24, 0.33],
            nan_band,
            &[false, false],
        );
        assert_eq!(vertical.len(), 1);
        assert_eq!(vertical[0].axis, 0);
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
        assert!(coplanar_fights(&boxes, &[0.24, 0.27], standing(), &[false, false]).is_empty());
        // ...and under min_sep = 0.08, so the colouring already flags it.
        let e = explain_oids_checked(&boxes, &[0.24, 0.27]).expect("complete ids");
        assert_eq!(e.pairs.len(), 1);
        assert_eq!(e.violations, vec![0]);
    }

    /// A cap five millimetres inside its partner is the wall fix
    /// working: a 0.005 gap is two and a half coincidence bands wide —
    /// more than one depth code apart at any distance the pack range
    /// admits — so the buffer resolves it everywhere and nothing ties.
    /// Half a millimetre — 0.0005 — ties beyond a ~20 m sightline (well
    /// inside the shipped 34 m) and must be flagged. An eps read the
    /// wrong way round would bless the broken gap and condemn the fixed
    /// one.
    #[test]
    fn a_face_five_millimetres_inside_its_partner_is_not_coplanar() {
        let big = Box3::from_center_size([1.0, 1.0, 1.0], [2.0, 2.0, 2.0]);
        let recessed = Box3 {
            min: [1.0, 0.5, 0.5],
            max: [1.995, 1.5, 1.5],
        };
        assert!(
            coplanar_fights(&[big, recessed], &[0.24, 0.33], standing(), &[false, false])
                .is_empty()
        );
        let tied = Box3 {
            min: [1.0, 0.5, 0.5],
            max: [1.9995, 1.5, 1.5],
        };
        let fights = coplanar_fights(&[big, tied], &[0.24, 0.33], standing(), &[false, false]);
        assert_eq!(fights.len(), 1);
        // The lower-indexed box names the plane.
        assert_eq!(fights[0].plane, 2.0);
    }

    /// The coincidence band's own edge: a plane gap of exactly 2e-3 —
    /// 0.002 - 0.0, COPLANAR_EPS to the bit — is still coplanar,
    /// INCLUSIVE. The band is sized to the 40 m pack-range ceiling (a
    /// 1.9 mm tie band at one depth code — see the constant), so an eps
    /// quietly turned exclusive, or quietly shrunk back to the old
    /// millimetre, would wave through the widest tie a legal map can
    /// still produce.
    #[test]
    fn two_millimetres_of_coincidence_is_inclusive() {
        let jamb = Box3 {
            min: [-1.0, 0.0, 0.0],
            max: [0.002, 2.0, 2.0],
        };
        let panel = Box3 {
            min: [-0.5, 0.5, 0.5],
            max: [0.0, 1.5, 1.5],
        };
        let fights = coplanar_fights(&[jamb, panel], &[0.24, 0.33], standing(), &[false, false]);
        assert_eq!(fights.len(), 1);
        assert_eq!((fights[0].axis, fights[0].plane), (0, 0.002));
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
        assert!(
            coplanar_fights(&[sliver, slab], &[0.24, 0.33], standing(), &[false, false]).is_empty()
        );
        let wide = Box3 {
            min: [-1.0, 0.0, 3.0],
            max: [0.1, 2.0, 4.0],
        };
        let fights = coplanar_fights(&[wide, slab], &[0.24, 0.33], standing(), &[false, false]);
        assert_eq!(fights.len(), 1);
        assert_eq!(fights[0].axis, 2);
    }

    /// A skip-marked solid censuses no fight at all. The boundary marks
    /// swept source envelopes — boxes whose planes rasterise nothing —
    /// and a mask the law ignored would go back to reporting every
    /// source z-fighting itself. Two IDENTICAL boxes are the loudest
    /// possible fight (four vertical coincidences); either endpoint
    /// being masked must silence all of them.
    #[test]
    fn a_skip_marked_solid_censuses_no_fight() {
        let twin = Box3::from_center_size([1.0, 1.0, 1.0], [2.0, 2.0, 2.0]);
        let boxes = [twin, twin];
        let oids = [0.24, 0.33];
        // Unmasked: X min+max and Z min+max coincide; the Y pair is eye-
        // gated (0 below the trough's view, 2 above the crest's).
        assert_eq!(
            coplanar_fights(&boxes, &oids, standing(), &[false, false]).len(),
            4
        );
        assert!(coplanar_fights(&boxes, &oids, standing(), &[true, false]).is_empty());
        assert!(coplanar_fights(&boxes, &oids, standing(), &[false, true]).is_empty());
    }

    /// Skipping an entry must not shift anyone else's indices: a fight's
    /// `a` and `b` always index the FULL input lists, so the boundary
    /// can name solids with no re-keying. A census that compacted its
    /// inputs would report this fight as (0, 1) and hang it on the
    /// skipped envelope's name.
    #[test]
    fn skipped_entries_do_not_shift_the_survivors_indices() {
        let [big, cap] = flush_capped_pair();
        // The envelope at index 0 is the big box's twin: unmasked it
        // would fight box 1 four ways and box 2 once.
        let boxes = [big, big, cap];
        let fights = coplanar_fights(
            &boxes,
            &[0.5, 0.24, 0.33],
            standing(),
            &[true, false, false],
        );
        assert_eq!(fights.len(), 1);
        assert_eq!((fights[0].a, fights[0].b), (1, 2));
        assert_eq!((fights[0].oid_a, fights[0].oid_b), (0.24, 0.33));
    }

    /// A short skip list refuses the census outright — the same
    /// vacuous-pass doctrine as a short oid list. Padding the tail with
    /// "not skipped" instead would quietly census envelope boxes the
    /// moment the boundary miscounts, and a clean answer from an
    /// unchecked mask is indistinguishable from a clean level.
    #[test]
    fn a_short_skip_list_refuses_the_census() {
        let boxes = flush_capped_pair();
        assert!(coplanar_fights_checked(&boxes, &[0.24, 0.33], standing(), &[false]).is_none());
        assert!(
            coplanar_fights_checked(&boxes, &[0.24, 0.33], standing(), &[false, false]).is_some()
        );
    }

    /// A short oid list cannot be censused. Reporting zero fights from an
    /// input that was never checked would read as a clean level — the
    /// same vacuous pass `explain_oids_checked` refuses.
    #[test]
    fn a_short_oid_list_refuses_the_fight_census_too() {
        let boxes = flush_capped_pair();
        assert!(coplanar_fights_checked(&boxes, &[0.24], standing(), &[false, false]).is_none());
        assert!(
            coplanar_fights_checked(&boxes, &[0.24, 0.33], standing(), &[false, false]).is_some()
        );
    }
}
