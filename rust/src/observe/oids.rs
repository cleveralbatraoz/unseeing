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
/// # Panics
///
/// If `oids` is shorter than `boxes`. Callers crossing a boundary should
/// use [`explain_oids_checked`].
#[must_use]
pub fn explain_oids(boxes: &[Box3], oids: &[f64]) -> OidExplanation {
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
}
