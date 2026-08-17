//! Colouring the unified face/source-role separation graph against the
//! palette and the role table
//! (`docs/superpowers/specs/2026-08-12-superface-outline-rendering-design.md`).
//! [`superface::superfaces`](super::superface::superfaces) decides which world
//! faces share a class. [`super::paint`] then adds non-geometric source-role
//! classes. This module assigns numeric labels so every edge clears
//! [`MIN_SEP`].
//!
//! # The role table
//!
//! [`role_label`] is the one numeric table for fixed slabs/creatures and for
//! standalone source-blueprint defaults. A WaveLevel does not fix every fan
//! shell or radio case to that global number: it preserves the semantic role
//! grouping while deriving per-instance palette values, so two copied sources
//! can touch and retain a seam. [`Role::Case`] at 0.05 is therefore only the
//! grandfathered standalone radio preview; it is never a newly allocated
//! authored-level label and is not a pattern to copy.
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
//! decided — currently a slab — bans any palette
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

/// A semantic render role. Slabs and creatures use the table value directly;
/// sources use Case/Shell/Moving as preview defaults and role names while a
/// level derives their per-instance numeric labels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A radio case's standalone blueprint default.
    Case,
    /// The floor slab.
    Floor,
    /// A source shell/fascia standalone blueprint default.
    Shell,
    /// A moving-source-part standalone blueprint default.
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

/// The ladder every label in a shipped level stands on: rungs
/// [`LADDER_STEP`] apart from [`LADDER_BASE`], filling the sRGB-safe band
/// [0.15, 0.96] exactly.
///
/// The spacing is FORCED, not chosen. Ten labels have to coexist in one
/// rendered frame — the floor, the five palette entries every wall and prop
/// is coloured from, the cat, the hero's body, the ceiling and the hero's
/// cane — and nine gaps across a band 0.81 wide leaves exactly 0.09 each.
/// Anything wider starves the population; anything narrower than [`MIN_SEP`]
/// draws the seam at reduced strength. There is no slack anywhere in it,
/// which is why the previous hand-picked table could not be repaired
/// locally: pushing the ceiling away from the cane pushed the hero's body
/// into the ceiling, and pushing the cat clear of that pushed it into the
/// palette's top entry.
pub const LADDER_BASE: f64 = 0.15;
pub const LADDER_STEP: f64 = 0.09;
pub const LADDER_RUNGS: usize = 10;

/// The `n`th rung, for tests and diagnostics to derive rather than retype.
/// Rungs beyond [`LADDER_RUNGS`] leave the band and are refused with `None`
/// rather than silently returning a value the shader cannot show.
#[must_use]
pub fn ladder_rung(n: usize) -> Option<f64> {
    (n < LADDER_RUNGS).then_some(LADDER_BASE + LADDER_STEP * n as f64)
}

/// The palette every wall, prop and source instance is coloured from —
/// rungs 1 through 5, leaving rung 0 to the floor below and rungs 6 through
/// 9 to the creatures and viewmodel above.
///
/// Five entries is not a limit on how many solids a level may hold: labels
/// are assigned by colouring the separation graph, so a hundred walls reuse
/// these five freely and differ only where they actually meet.
///
/// It lives HERE, with the role table and [`MIN_SEP`], rather than in the
/// level node that consumes it. The law it belongs to is "no two labels that
/// must draw a seam land within MIN_SEP", and that law is only checkable
/// where the whole label universe is visible at once.
pub const WORLD_PALETTE: [f64; 5] = [0.24, 0.33, 0.42, 0.51, 0.60];

/// The one role/default table, every entry a rung of the ladder above.
///
/// `Case` 0.05 is the exception and stays one: it is the grandfathered
/// standalone radio preview, below the band entirely, and never a label a
/// level allocates.
///
/// `Shell` and `Moving` deliberately REUSE palette rungs. They are
/// standalone blueprint preview defaults — a source dropped in the editor
/// with no level around it — and a level derives per-instance labels for
/// those roles instead, so they never stand beside a palette-coloured wall
/// in a shipped frame. Twelve distinct labels do not fit in a band that
/// holds eleven, and these two are the pair that provably never needs its
/// own rung.
///
/// `const fn` on purpose: creatures/slabs build final labels and sources
/// build standalone preview defaults without repeating numeric literals.
pub const fn role_label(role: Role) -> f64 {
    match role {
        Role::Case => 0.05,
        Role::Floor => 0.15,
        Role::Shell => 0.33,
        Role::Moving => 0.60,
        Role::Cat => 0.69,
        Role::HeroBody => 0.78,
        Role::Ceiling => 0.87,
        Role::HeroCane => 0.96,
    }
}

/// Every label that can stand beside another in ONE rendered frame of a
/// shipped level and be asked to draw a seam between them, named so a
/// failure says which pair.
///
/// This is the population the separation law actually governs, and stating
/// it explicitly is the point: the graph colouring enforces `MIN_SEP` only
/// over classes it can see, and creatures, the viewmodel and the palette
/// itself never enter `paint_entries` at all. Nothing checked them against
/// each other before, and the shipped table violated the law twice.
#[must_use]
pub fn coexisting_labels() -> Vec<(&'static str, f64)> {
    let mut all = vec![("Role::Floor", role_label(Role::Floor))];
    for (slot, label) in WORLD_PALETTE.iter().enumerate() {
        all.push((PALETTE_NAMES[slot], *label));
    }
    all.extend([
        ("Role::Cat", role_label(Role::Cat)),
        ("Role::HeroBody", role_label(Role::HeroBody)),
        ("Role::Ceiling", role_label(Role::Ceiling)),
        ("Role::HeroCane", role_label(Role::HeroCane)),
    ]);
    all
}

const PALETTE_NAMES: [&str; 5] = [
    "WORLD_PALETTE[0]",
    "WORLD_PALETTE[1]",
    "WORLD_PALETTE[2]",
    "WORLD_PALETTE[3]",
    "WORLD_PALETTE[4]",
];

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

/// The shader and `ArrayMesh::CUSTOM0` both consume 32-bit lanes. Public
/// planning inputs remain f64 for the engine-independent domain, but this is
/// the canonical renderer representation reported by every assigned label.
/// Invalid values have no renderer representation.
pub fn renderer_label(label: f64) -> Option<f64> {
    let narrowed = label as f32;
    narrowed.is_finite().then(|| f64::from(narrowed))
}

/// Do two labels draw a full-strength seam between them after the exact
/// narrowing and subtraction the shader performs? Comparing the source f64s
/// is insufficient: `0.31` and `0.39` look nominally 0.08 apart there, but
/// their CUSTOM0 lanes subtract to `0.07999998_f32`, below the shader knee.
pub fn separated(a: f64, b: f64) -> bool {
    let (a, b) = (a as f32, b as f32);
    a.is_finite() && b.is_finite() && (a - b).abs() >= MIN_SEP as f32
}

/// The outcome: one label per superface class, in class-index order
/// (`label_of_class.len() == sf.classes`), each already narrowed to the exact
/// CUSTOM0 f32 value and widened back to f64 for the pure contract, plus how
/// many colourable classes — those with no anchor — the palette could not
/// satisfy. The class indices are retained so editor diagnostics can point at
/// the nodes whose faces could not be separated. Anchored classes never
/// starve: they do not compete for a palette slot in the first place.
#[derive(Clone, Debug, PartialEq)]
pub struct Labelling {
    pub label_of_class: Vec<f64>,
    pub starved: usize,
    pub starved_classes: Vec<usize>,
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
/// the caller. Finite assigned inputs are returned through
/// [`renderer_label`], so diagnostics and the later mesh boundary share one
/// numeric representation.
pub(crate) fn assign(sf: &Superfaces, anchors: &[(usize, f64)], palette: &[f64]) -> Labelling {
    let n = sf.classes;

    let mut anchor_label: Vec<Option<f64>> = vec![None; n];
    for &(class, label) in anchors {
        if class < n
            && let Some(label) = renderer_label(label)
        {
            anchor_label[class] = Some(label);
        }
    }

    let no_label = renderer_label(oid_palette::NO_OID).unwrap_or(0.0);

    if palette.is_empty() {
        let label_of_class: Vec<f64> = (0..n)
            .map(|c| anchor_label[c].unwrap_or(no_label))
            .collect();
        let starved_classes: Vec<usize> = (0..n).filter(|&c| anchor_label[c].is_none()).collect();
        let starved = starved_classes.len();
        return Labelling {
            label_of_class,
            starved,
            starved_classes,
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

    let (chosen, starved_local) = oid_palette::welsh_powell(&adjacency, &banned, palette.len());
    let starved_classes: Vec<usize> = starved_local
        .iter()
        .filter_map(|&local| colourable.get(local).copied())
        .collect();
    let starved = starved_classes.len();

    let mut label_of_class = vec![no_label; n];
    for c in 0..n {
        label_of_class[c] = if let Some(label) = anchor_label[c] {
            label
        } else if let Some(local) = local_of[c] {
            renderer_label(palette[chosen[local]]).unwrap_or(no_label)
        } else {
            // Unreachable by construction: every class is either anchored
            // or in `colourable`. Kept as a safe fallback, never a panic,
            // matching this crate's total-function doctrine everywhere
            // else.
            no_label
        };
    }

    Labelling {
        label_of_class,
        starved,
        starved_classes,
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

    fn palette_contains_renderer_label(label: f64) -> bool {
        PALETTE
            .iter()
            .any(|&candidate| candidate as f32 == label as f32)
    }

    /// THE break this catches, and the law nothing enforced before: EVERY
    /// pair of labels that can stand together in one rendered frame must be
    /// able to draw a seam between them.
    ///
    /// The graph colouring enforces `MIN_SEP` only over classes it can see,
    /// and creatures, the viewmodel and the palette itself never enter
    /// `paint_entries` — so the one mechanism that could have caught this
    /// was structurally blind to it, and the only test over the table was a
    /// per-row mirror assertion that would have agreed with any numbers at
    /// all. The shipped table failed twice: `HeroBody` 0.82 against
    /// `Ceiling` 0.90 subtracted to 0.079999983 in the f32 the shader
    /// actually compares, a hair under the knee; `Ceiling` 0.90 against
    /// `HeroCane` 0.96 was 0.06, and the cane CAN touch the ceiling (eye at
    /// 1.6, pitch limit 1.35 rad, reach 1.7 — 3.26 m against a 3.0 m
    /// ceiling), where the distance Laplacian is dead and `nrm` draws the
    /// seam alone, at half strength.
    #[test]
    fn every_label_that_can_share_a_frame_can_draw_a_seam() {
        let all = coexisting_labels();
        assert_eq!(all.len(), LADDER_RUNGS, "the ladder is exactly full");
        for (i, &(first_name, first)) in all.iter().enumerate() {
            for &(second_name, second) in all.iter().skip(i + 1) {
                assert!(
                    separated(first, second),
                    "{first_name} ({first}) and {second_name} ({second}) land \
                     {} apart, under MIN_SEP {MIN_SEP} — the seam between them \
                     draws at reduced strength or not at all",
                    ((first as f32) - (second as f32)).abs()
                );
            }
        }
    }

    /// A standalone source blueprint previewed in the editor with no level
    /// around it shows its own default labels, and those must separate from
    /// each other too — the same law, over the only other population that
    /// can share a frame. `Case` sits below the band by grandfathering, so
    /// this is the one place its clearance is checked at all.
    #[test]
    fn a_standalone_source_previews_with_separable_defaults() {
        let preview = [
            ("Role::Case", role_label(Role::Case)),
            ("Role::Shell", role_label(Role::Shell)),
            ("Role::Moving", role_label(Role::Moving)),
        ];
        for (i, &(first_name, first)) in preview.iter().enumerate() {
            for &(second_name, second) in preview.iter().skip(i + 1) {
                assert!(
                    separated(first, second),
                    "{first_name} ({first}) and {second_name} ({second}) cannot \
                     draw a seam in a standalone preview"
                );
            }
        }
    }

    /// Every label in the band is a rung of the one ladder, derived rather
    /// than retyped — the break this catches is a value nudged by hand to
    /// fix one pair, which is exactly how the shipped table drifted out of
    /// the law in the first place. `Case` is the single documented
    /// exception and is asserted to be exactly that, so a second exception
    /// cannot be added quietly.
    #[test]
    fn every_shipped_label_stands_on_a_rung() {
        let rungs: Vec<f64> = (0..LADDER_RUNGS)
            .map(|n| ladder_rung(n).expect("rung in range"))
            .collect();
        let on_a_rung = |label: f64| rungs.iter().any(|rung| (rung - label).abs() < 1.0e-9);
        for (name, label) in coexisting_labels() {
            assert!(on_a_rung(label), "{name} ({label}) is not on the ladder");
        }
        for role in [Role::Shell, Role::Moving] {
            assert!(
                on_a_rung(role_label(role)),
                "{role:?} ({}) is not on the ladder",
                role_label(role)
            );
        }
        assert_eq!(role_label(Role::Case), 0.05);
        assert!(!on_a_rung(role_label(Role::Case)));
        assert_eq!(ladder_rung(LADDER_RUNGS), None);
    }

    /// The ladder fills the sRGB-safe band exactly: its first rung IS the
    /// band's floor and its last IS the ceiling, so no rung is wasted and
    /// none escapes. Hand-derived: 0.15 + 9 x 0.09 = 0.96.
    #[test]
    fn the_ladder_fills_the_band_end_to_end() {
        assert_eq!(ladder_rung(0), Some(0.15));
        let top = ladder_rung(LADDER_RUNGS - 1).expect("top rung");
        assert!((top - 0.96).abs() < 1.0e-9, "top rung is {top}");
    }

    /// The role table, spot-checked against the brief's exact numbers —
    /// the break this catches is a transposed row. Kept alongside the
    /// all-pairs law above rather than instead of it: this one would agree
    /// with any self-consistent set of numbers, which is precisely how the
    /// two violations above survived.
    #[test]
    fn role_table_matches_the_brief() {
        assert_eq!(role_label(Role::Case), 0.05);
        assert_eq!(role_label(Role::Floor), 0.15);
        assert_eq!(role_label(Role::Shell), 0.33);
        assert_eq!(role_label(Role::Moving), 0.60);
        assert_eq!(role_label(Role::Cat), 0.69);
        assert_eq!(role_label(Role::HeroBody), 0.78);
        assert_eq!(role_label(Role::Ceiling), 0.87);
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
        assert!(
            out.label_of_class
                .iter()
                .all(|&label| palette_contains_renderer_label(label))
        );
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
            cluster_of_solid: [(0, 0), (1, 1)].into_iter().collect(),
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
            cluster_of_solid: (0..n).map(|solid| (solid, solid)).collect(),
        };
        let out = assign(&sf, &[], &PALETTE);
        assert_eq!(out.label_of_class.len(), n);
        assert_eq!(out.starved, 2);
        assert_eq!(out.starved_classes, vec![5, 6]);
        assert!(
            out.label_of_class
                .iter()
                .all(|&label| palette_contains_renderer_label(label))
        );
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
            cluster_of_solid: [(0, 0), (1, 1), (2, 2)].into_iter().collect(),
        };
        let out = assign(&sf, &[(0, 0.15)], &PALETTE);
        assert_eq!(out.starved, 0);
        assert_eq!(out.label_of_class[0], f64::from(0.15_f32));
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
            cluster_of_solid: [(0, 0), (1, 1)].into_iter().collect(),
        };
        // 0.20 sits within MIN_SEP (0.08) of PALETTE[0] (0.25, gap 0.05)
        // but clear of every other entry (next-closest is 0.34, gap 0.14).
        let out = assign(&sf, &[(0, 0.20)], &PALETTE);
        assert_eq!(out.starved, 0);
        assert_eq!(out.label_of_class[0], f64::from(0.20_f32));
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
            cluster_of_solid: [(0, 0), (1, 1)].into_iter().collect(),
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
            cluster_of_solid: [(0, 0), (1, 1)].into_iter().collect(),
        };
        let clean = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![(0, 1)],
            cluster_of_solid: [(0, 0), (1, 1)].into_iter().collect(),
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
            cluster_of_solid: std::collections::BTreeMap::new(),
        };
        assert_eq!(
            assign(&empty_sf, &[], &PALETTE),
            Labelling {
                label_of_class: vec![],
                starved: 0,
                starved_classes: vec![],
            }
        );
        let sf = Superfaces {
            class_of: vec![0, 1],
            classes: 2,
            separations: vec![],
            cluster_of_solid: [(0, 0), (1, 1)].into_iter().collect(),
        };
        let out = assign(&sf, &[], &[]);
        assert_eq!(
            out.label_of_class,
            vec![oid_palette::NO_OID, oid_palette::NO_OID]
        );
        assert_eq!(out.starved, 2);
        assert_eq!(out.starved_classes, vec![0, 1]);
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
