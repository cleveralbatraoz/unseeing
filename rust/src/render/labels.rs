//! Colouring the superface graph against the palette and the role table
//! (`docs/superpowers/specs/2026-08-12-superface-outline-rendering-design.md`).
//! [`superface::superfaces`](super::superface::superfaces) already decided
//! which faces share ONE class and which resulting classes must take
//! separated labels; this module decides what label each class actually
//! gets — a fixed [`Role`] value for anything the colouring must never
//! touch (a slab, a creature, a source's own limbs), a palette entry for
//! everything else, chosen so that no two classes
//! [`Superfaces::separations`] names ever land within [`MIN_SEP`] of each
//! other.
//!
//! # The role table
//!
//! [`role_label`] IS the one place every fixed label in the game lives.
//! The per-node id constants that used to hold these numbers — `OID_FLOOR`
//! and `OID_CEIL` in `nodes/level.rs`, and the equivalents in
//! `nodes/radio.rs`, `nodes/fan.rs`, `nodes/cat.rs` and `nodes/hero.rs` —
//! are gone; every one of those files builds its own fixed table from this
//! function at compile time (it is a `const fn` for exactly that reason).
//!
//! [`Role::Case`] (0.05) is the one grandfathered exception: it sits BELOW
//! the 0.15 comfort line every other entry respects, carried over
//! unchanged from the radio chassis's pre-existing value. It is safe where
//! it stands and it is not a pattern to copy. Safe, because the only
//! question a label has to answer is whether the seams it must draw clear
//! the hearing pass's crease floor (`smoothstep(0.04, 0.08, nrm)`,
//! `hearing_post.gdshader`), and every label `Case` can meet clears it:
//! `Floor` 0.15 (the radio stands on it) by 0.10, its own `Shell` fascia
//! 0.33 by 0.28, and the whole world palette (`nodes::level::WORLD_OIDS`,
//! lowest entry 0.25) by 0.20 or more. Measured end to end on the web
//! build rather than assumed — the G channel round-trips linearly there,
//! byte = 255 x label within a byte, so the 0.10 gap arrives as ~0.094 and
//! still saturates the crease. Not a pattern to copy, because that margin
//! is the smallest any pair in the table carries, and a SECOND label down
//! here would have nothing to separate from `Case` against.
//!
//! # Colouring
//!
//! Greedy Welsh–Powell over the superface class graph — most-constrained
//! class first, ties by class index — reusing
//! [`oid_palette::welsh_powell`], the exact algorithm this crate's
//! per-solid id crease used before this campaign, through its own
//! `oid_palette::assign`. That per-solid colouring is GONE (Task 10
//! retired it: `assign`/`Fixed` had no caller left once every touch-graph
//! anchor here migrated to this module's own `anchors`), and
//! `render::labels::assign` is `welsh_powell`'s only caller now. It stays
//! borrowed from `oid_palette` — not copied into `render/` — only because
//! `oid_palette` is still the WRITTEN-DOWN home of the algorithm itself;
//! moving it would be a pure relocation with no behaviour riding on it, so
//! it was left alone rather than forced.
//!
//! `anchors` play the role `oid_palette::Fixed` used to play for the
//! retired `oid_palette::assign`: a class whose label is already
//! decided — a slab, a source's swept neighbourhood — bans any palette
//! slot within [`MIN_SEP`] of its own label for every class
//! [`Superfaces::separations`] pairs it with. Unlike that old `Fixed`,
//! though, an anchor's class is itself a member of the SAME class space
//! [`Labelling::label_of_class`] answers for — a superface class can
//! belong to a slab exactly as easily as to a wall, so there is no
//! separate "boxes" array holding only the colourable ones the way the
//! old `assign`'s `boxes` parameter needed. An anchored class is
//! therefore excluded from Welsh–Powell entirely and takes its given
//! label directly rather than being coloured.
//!
//! # Determinism
//!
//! Same law as everywhere else in this vocabulary: Welsh–Powell ties break
//! on class index, never on hash iteration order; the anchor table is a
//! plain slice walked once, in the caller's own order.

use crate::oid_palette;
use crate::render::superface::Superfaces;

/// A fixed role a class colours to directly — the palette colouring never
/// touches these; see [`role_label`] for the table and the module doc for
/// why `Case` is the one exception to the sRGB comfort line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A source's own case — the part that stands on world geometry (the
    /// radio's chassis). Grandfathered below the sRGB comfort line.
    Case,
    /// The floor slab.
    Floor,
    /// A source's shell — the fan's housing, the radio's fascia.
    Shell,
    /// A source's moving part (fan blades).
    Moving,
    /// The companion cat.
    Cat,
    /// The hero's legs and torso.
    HeroBody,
    /// The ceiling slab.
    Ceiling,
    /// The hero's arm and cane.
    HeroCane,
}

/// The one label table. `Case` 0.05 stays only for the radio chassis
/// (pre-existing, grandfathered below the 0.15 comfort line — the module
/// doc derives why it is safe there and why it is the last of its kind),
/// `Floor` 0.15, `Shell` 0.33, `Moving` 0.63, `Cat` 0.70, `HeroBody` 0.82,
/// `Ceiling` 0.90, `HeroCane` 0.96.
///
/// `const fn` on purpose: every creature and source builds its own fixed
/// `oids()`/`OIDS` table from this function at compile time now that the
/// id constants that used to hold these numbers locally (`CAT_OID`,
/// `FAN_OID`, `RADIO_CASE_OID`, ...) are gone — this is the one place any
/// of them may be spelled again.
pub const fn role_label(role: Role) -> f64 {
    match role {
        Role::Case => 0.05,
        Role::Floor => 0.15,
        Role::Shell => 0.33,
        Role::Moving => 0.63,
        Role::Cat => 0.70,
        Role::HeroBody => 0.82,
        Role::Ceiling => 0.90,
        Role::HeroCane => 0.96,
    }
}

/// Labels at least this far apart draw a full-strength crease off the
/// shader's own `smoothstep(0.04, 0.08, nrm)` upper knee. Below it the
/// seam fades; at zero it is gone.
///
/// THE one definition in the crate. `oid_palette` carried a second,
/// textually independent copy of the same number for the per-solid id path
/// this campaign replaced, with nothing asserting the two agreed — so
/// tuning the shader knee and updating one copy would have left the
/// colouring and the seam census judging by different thresholds. That
/// copy is gone and `observe::oids` reads this one. What is still NOT
/// single-sourced, and cannot be from Rust, is the shader literal itself:
/// `hearing_post.gdshader`'s `smoothstep(0.04, 0.08, nrm)` is the actual
/// authority, and no gate compares this constant against it.
pub const MIN_SEP: f64 = 0.08;

/// Slack on the separation test: a palette laid out on exact decimal steps
/// misses its own nominal gap by an ULP — `0.31 - 0.23` is
/// `0.0799999999999999`, just under [`MIN_SEP`] — and a law that rejected
/// the palette it was written for would be worse than no law.
const SLACK: f64 = 1e-9;

/// Do two labels draw a full-strength seam between them?
pub fn separated(a: f64, b: f64) -> bool {
    (a - b).abs() >= MIN_SEP - SLACK
}

/// The outcome: one label per superface class, in class-index order
/// (`label_of_class.len() == sf.classes`), plus how many colourable
/// classes — those with no anchor — the palette could not satisfy.
/// `starved` never counts an anchored class: it never competes for a
/// palette slot in the first place.
#[derive(Clone, Debug, PartialEq)]
pub struct Labelling {
    pub label_of_class: Vec<f64>,
    pub starved: usize,
}

/// Colour `sf`'s superface classes. Every class named in `anchors` takes
/// its given label directly; every other class takes a `palette` entry
/// chosen so that no two classes [`Superfaces::separations`] pairs land
/// within [`MIN_SEP`] of each other, nor within [`MIN_SEP`] of an anchor
/// they separate from.
///
/// Total for every input: an empty palette answers every non-anchored
/// class with [`oid_palette::NO_OID`] and counts every one of them as
/// starved; an anchor naming a class outside `0..sf.classes` is ignored
/// rather than panicking; a class the palette cannot satisfy takes the
/// least-contended slot it can, counted in `starved`, rather than failing
/// the caller.
pub fn assign(sf: &Superfaces, anchors: &[(usize, f64)], palette: &[f64]) -> Labelling {
    let n = sf.classes;

    let mut anchor_label: Vec<Option<f64>> = vec![None; n];
    for &(class, label) in anchors {
        if class < n {
            anchor_label[class] = Some(label);
        }
    }

    if palette.is_empty() {
        let label_of_class: Vec<f64> = (0..n)
            .map(|c| anchor_label[c].unwrap_or(oid_palette::NO_OID))
            .collect();
        let starved = anchor_label.iter().filter(|a| a.is_none()).count();
        return Labelling {
            label_of_class,
            starved,
        };
    }

    // the full class-level adjacency the separation graph names
    let mut full_adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in &sf.separations {
        if a < n && b < n {
            full_adjacency[a].push(b);
            full_adjacency[b].push(a);
        }
    }

    // classes the palette must colour: everything no anchor already fixed
    let colourable: Vec<usize> = (0..n).filter(|&c| anchor_label[c].is_none()).collect();
    let mut local_of: Vec<Option<usize>> = vec![None; n];
    for (local, &global) in colourable.iter().enumerate() {
        local_of[global] = Some(local);
    }

    let m = colourable.len();
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); m];
    let mut banned: Vec<Vec<bool>> = vec![vec![false; palette.len()]; m];
    for (local, &global) in colourable.iter().enumerate() {
        for &neighbour in &full_adjacency[global] {
            if let Some(other_local) = local_of[neighbour] {
                adjacency[local].push(other_local);
            } else if let Some(fixed_label) = anchor_label[neighbour] {
                for (slot, &id) in palette.iter().enumerate() {
                    if !separated(id, fixed_label) {
                        banned[local][slot] = true;
                    }
                }
            }
        }
    }

    let (chosen, starved) = oid_palette::welsh_powell(&adjacency, &banned, palette.len());

    let mut label_of_class = vec![oid_palette::NO_OID; n];
    for c in 0..n {
        label_of_class[c] = if let Some(label) = anchor_label[c] {
            label
        } else if let Some(local) = local_of[c] {
            palette[chosen[local]]
        } else {
            // Unreachable by construction: every class is either anchored
            // or in `colourable`. Kept as a safe fallback, never a panic,
            // matching this crate's total-function doctrine everywhere
            // else.
            oid_palette::NO_OID
        };
    }

    Labelling {
        label_of_class,
        starved,
    }
}

#[cfg(test)]
mod tests {
    use super::super::faces::{Shape, faces};
    use super::super::superface::{Superfaces, superfaces};
    use super::*;

    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    const PALETTE: [f64; 5] = [0.25, 0.34, 0.43, 0.52, 0.61];

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
    fn junction() -> Superfaces {
        let mut all = faces(0, &wall_a());
        all.extend(faces(1, &wall_b()));
        superfaces(&all, &[(0, 1)])
    }

    /// The role table, spot-checked against the brief's exact numbers —
    /// the break this catches is a transposed row or a copy from the
    /// wrong id in `oid_palette`'s own budget table.
    #[test]
    fn role_table_matches_the_brief() {
        assert_eq!(role_label(Role::Case), 0.05);
        assert_eq!(role_label(Role::Floor), 0.15);
        assert_eq!(role_label(Role::Shell), 0.33);
        assert_eq!(role_label(Role::Moving), 0.63);
        assert_eq!(role_label(Role::Cat), 0.70);
        assert_eq!(role_label(Role::HeroBody), 0.82);
        assert_eq!(role_label(Role::Ceiling), 0.90);
        assert_eq!(role_label(Role::HeroCane), 0.96);
    }

    /// End-to-end: faces -> superfaces -> assign. The junction's merged
    /// cap class (global faces 4 and 10, proven one class by
    /// `superface::tests::a_junction_cap_merges_into_the_partners_flank`)
    /// must read back as ONE label through the whole chain — the wiring
    /// under test is that `assign`'s output stays addressable by
    /// `class_of` without losing or duplicating an index anywhere along
    /// the way.
    #[test]
    fn the_junction_caps_merged_class_gets_one_label() {
        let sf = junction();
        let out = assign(&sf, &[], &PALETTE);
        assert_eq!(out.starved, 0);
        assert_eq!(out.label_of_class.len(), sf.classes);
        assert!(out.label_of_class.iter().all(|l| PALETTE.contains(l)));
        assert_eq!(
            out.label_of_class[sf.class_of[4]],
            out.label_of_class[sf.class_of[10]]
        );
    }

    /// Every class pair the superface graph separates must take labels at
    /// least `MIN_SEP` apart — checked over the junction's OWN separation
    /// list (24 pairs, not a hand-picked one or two), so this catches any
    /// colouring bug a narrower fixture might miss.
    #[test]
    fn separated_classes_take_labels_min_sep_apart() {
        let sf = junction();
        let out = assign(&sf, &[], &PALETTE);
        assert!(!sf.separations.is_empty());
        for &(a, b) in &sf.separations {
            assert!(
                separated(out.label_of_class[a], out.label_of_class[b]),
                "classes {a} and {b} share a seam but landed within MIN_SEP: {} vs {}",
                out.label_of_class[a],
                out.label_of_class[b]
            );
        }
    }

    /// Classes with no separation between them may share a label — what
    /// makes a small palette enough for hundreds of classes, mirroring
    /// `oid_palette::tests::distant_boxes_may_share_an_id`.
    #[test]
    fn non_adjacent_classes_may_share_a_label() {
        let sf = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![],
            cluster_of_solid: vec![0, 1],
        };
        let out = assign(&sf, &[], &PALETTE);
        assert_eq!(out.starved, 0);
        assert_eq!(out.label_of_class[0], out.label_of_class[1]);
        assert_eq!(out.label_of_class[0], PALETTE[0]);
    }

    /// A clique bigger than the palette: colouring still labels every
    /// class and honestly reports how many it could not satisfy, mirroring
    /// `oid_palette::tests::an_impossible_clique_is_reported_not_panicked`
    /// (same 7-into-5 shape, same expected `starved == 2`).
    #[test]
    fn an_impossible_clique_is_reported_not_panicked() {
        let n = 7;
        let mut separations = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                separations.push((i, j));
            }
        }
        let sf = Superfaces {
            class_of: (0..n).collect(),
            classes: n,
            separations,
            cluster_of_solid: (0..n).collect(),
        };
        let out = assign(&sf, &[], &PALETTE);
        assert_eq!(out.label_of_class.len(), n);
        assert_eq!(out.starved, 2);
        assert!(out.label_of_class.iter().all(|l| PALETTE.contains(l)));
    }

    /// An anchor's own class takes its given label directly, and a class
    /// that never separates from it is completely unconstrained —
    /// mirroring `oid_palette::tests::a_fixed_neighbour_bans_only_its_own_touchers`.
    #[test]
    fn an_anchor_labels_itself_and_leaves_non_neighbours_free() {
        let sf = Superfaces {
            class_of: vec![0, 1, 2],
            classes: 3,
            separations: vec![(0, 1)],
            cluster_of_solid: vec![0, 1, 2],
        };
        let out = assign(&sf, &[(0, 0.15)], &PALETTE);
        assert_eq!(out.starved, 0);
        assert_eq!(out.label_of_class[0], 0.15);
        // class 2 never separates from the anchor and is unconstrained
        assert_eq!(out.label_of_class[2], PALETTE[0]);
    }

    /// The ban itself, isolated: an anchor whose label sits within
    /// `MIN_SEP` of the palette's own first entry must push its one
    /// separating neighbour off that entry.
    #[test]
    fn an_anchor_close_to_the_palette_pushes_its_neighbour_off_it() {
        let sf = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![(0, 1)],
            cluster_of_solid: vec![0, 1],
        };
        // 0.20 sits within MIN_SEP (0.08) of PALETTE[0] (0.25, gap 0.05)
        // but clear of every other entry (next-closest is 0.34, gap 0.14).
        let out = assign(&sf, &[(0, 0.20)], &PALETTE);
        assert_eq!(out.starved, 0);
        assert_eq!(out.label_of_class[0], 0.20);
        assert_ne!(out.label_of_class[1], PALETTE[0]);
        assert!(separated(out.label_of_class[1], 0.20));
    }

    /// An anchor naming a class outside `0..sf.classes` is a no-op, not a
    /// panic — the boundary can pass an anchor list built against a wider
    /// scope than the classes actually in play for this call.
    #[test]
    fn an_anchor_beyond_the_known_classes_does_not_panic() {
        let sf = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![(0, 1)],
            cluster_of_solid: vec![0, 1],
        };
        let with_bogus = assign(&sf, &[(9, 0.5)], &PALETTE);
        let without = assign(&sf, &[], &PALETTE);
        assert_eq!(with_bogus, without);
    }

    /// A separation pair naming a class outside `0..sf.classes` is a
    /// no-op too — `Superfaces` itself never emits one, but this module
    /// takes `sf.separations` on faith rather than re-validating it, so
    /// its own bound check earns a direct witness rather than resting on
    /// an upstream invariant it cannot see.
    #[test]
    fn a_separation_beyond_the_known_classes_does_not_panic() {
        let with_bogus = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![(0, 1), (0, 5), (5, 1)],
            cluster_of_solid: vec![0, 1],
        };
        let clean = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![(0, 1)],
            cluster_of_solid: vec![0, 1],
        };
        assert_eq!(
            assign(&with_bogus, &[], &PALETTE),
            assign(&clean, &[], &PALETTE)
        );
    }

    /// Total on the degenerate inputs: no classes, and no palette.
    #[test]
    fn empty_inputs_are_answered_not_crashed() {
        let empty_sf = Superfaces {
            class_of: vec![],
            classes: 0,
            separations: vec![],
            cluster_of_solid: vec![],
        };
        assert_eq!(
            assign(&empty_sf, &[], &PALETTE),
            Labelling {
                label_of_class: vec![],
                starved: 0
            }
        );
        let sf = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![],
            cluster_of_solid: vec![0, 1],
        };
        let out = assign(&sf, &[], &[]);
        assert_eq!(
            out.label_of_class,
            vec![oid_palette::NO_OID, oid_palette::NO_OID]
        );
        assert_eq!(out.starved, 2);
    }

    /// Two machines must colour one graph identically — the wasm build and
    /// the desktop build draw the same world or neither is trustworthy.
    #[test]
    fn colouring_is_deterministic() {
        let sf = junction();
        let first = assign(&sf, &[], &PALETTE);
        for _ in 0..8 {
            assert_eq!(assign(&sf, &[], &PALETTE), first);
        }
    }

    /// Scale reality: a graph shaped like the shipped map's own (per Task
    /// 3's review measurement over the real level — 539 classes, 5660
    /// separations, the whole wall network ONE cluster needing only ~3
    /// labels) must still colour clean with the five-entry world palette.
    /// Built from REAL `faces`/`superfaces` output over a synthetic
    /// "comb" of touching wall segments — one long spine with many
    /// perpendicular teeth, each forming its own T-junction with the
    /// spine exactly like the `junction()` fixture above, repeated along
    /// the spine's length — not the shipped scene, and not a hand-typed
    /// separations list: the actual shape a level's own wall network
    /// produces. 200 teeth land at 606 classes / 2412 separations
    /// (measured, not assumed) — same order of magnitude as the shipped
    /// map on both counts, and every tooth transitively shares the
    /// spine's one cluster exactly as the real 17-member wall network
    /// does, so rule (b)'s fine-grained path governs here too, not rule
    /// (c)'s blanket one.
    #[test]
    fn a_graph_at_shipped_map_scale_colours_without_starving() {
        const TEETH: usize = 200;
        const STEP: f64 = 3.0;

        let spine = Shape::Box3d {
            center: [(TEETH as f64 - 1.0) * STEP / 2.0, 1.5, 0.0],
            size: [(TEETH as f64) * STEP + 4.0, 3.0, 0.3],
            basis: IDENTITY,
        };
        let mut all = faces(0, &spine);
        let mut touching = Vec::new();
        for i in 0..TEETH {
            let tooth = Shape::Box3d {
                center: [i as f64 * STEP, 1.5, 2.0],
                size: [0.3, 3.0, 4.3],
                basis: IDENTITY,
            };
            all.extend(faces(i + 1, &tooth));
            touching.push((0, i + 1));
        }

        let sf = superfaces(&all, &touching);
        // sanity: this fixture really is in the shipped map's ballpark,
        // not an accidentally trivial graph
        assert!(
            sf.classes > 400,
            "fixture too small to be shape-comparable: {} classes",
            sf.classes
        );
        assert!(
            sf.separations.len() > 2000,
            "fixture too sparse to be shape-comparable: {} separations",
            sf.separations.len()
        );

        let out = assign(&sf, &[], &PALETTE);
        assert_eq!(out.label_of_class.len(), sf.classes);
        assert_eq!(
            out.starved, 0,
            "the world palette starved on a graph this shape must not starve on"
        );
        for &(a, b) in &sf.separations {
            assert!(
                separated(out.label_of_class[a], out.label_of_class[b]),
                "classes {a} and {b} share a seam but landed within MIN_SEP"
            );
        }
    }
}
