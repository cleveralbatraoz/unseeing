//! Flat object ids for the data pass, chosen so that every seam draws.
//!
//! The outline pass draws a line two ways (`hearing_post.gdshader`): a
//! Laplacian of the packed distance in B catches SILHOUETTES, where the
//! world steps away from itself; a difference of the flat object id in G
//! catches CREASES, faded over `smoothstep(0.04, 0.08, ..)`.
//!
//! Where two boxes interpenetrate there is no depth step, so the silhouette
//! term has nothing to bite on — the crease is the only thing that can draw
//! their seam. Two touching boxes carrying the SAME id therefore have no
//! line between them at all: they melt into one silhouette. That is a
//! perception bug, not a cosmetic one, because the whole world is contours.
//!
//! Cycling a small palette by scene index cannot honour this. With more
//! boxes than palette entries some neighbouring pair always collides, and
//! which pair depends on the order the designer happened to add nodes in.
//! What matters is not the COUNT of boxes but their ADJACENCY: two boxes at
//! opposite ends of the map may share an id freely, because no pixel ever
//! shows them meeting. So this module builds the touch graph and colours
//! it — ids differ exactly where a seam has to be drawn, and are reused
//! everywhere else.
//!
//! Colouring is greedy in Welsh–Powell order (most-constrained box first,
//! ties by scene index), which is not optimal — optimal graph colouring is
//! NP-hard — but is deterministic and, on the box arrangements a room plan
//! produces, lands on the chromatic number in practice. Determinism is the
//! non-negotiable half: the same scene must colour identically on every
//! platform and every run, or desktop and wasm would draw different worlds.

//!
//! # THE ID BUDGET
//!
//! The G channel is one number in [0, 1] and every object in the world has
//! to fit in it. Most of them do not need their own: the five WORLD ids are
//! handed out by the colouring below and reused wherever no pixel shows two
//! solids meeting. What needs a FIXED id is anything the colouring cannot
//! see — a slab, a creature, the hero's own body, a sound source painting
//! its own limbs. Those are enumerated here, and this list is the one place
//! to look before inventing another.
//!
//! ```text
//!  id    who                                     assigned in
//!  ----  --------------------------------------  ------------------------
//!  0.05  a source's CASE — the part that stands   nodes/radio.rs
//!        on world geometry (the radio's chassis)
//!  0.15  the FLOOR slab                           nodes/level.rs
//!  0.25  }                                        nodes/level.rs, by the
//!  0.34  }  WORLD_OIDS — every wall, box,         colouring below: reused
//!  0.43  }  column and wedge in the level         freely between solids
//!  0.52  }                                        that never touch
//!  0.61  }
//!  0.33  a source's SHELL — the fan's housing,    nodes/fan.rs,
//!        the radio's fascia                       nodes/radio.rs
//!  0.63  a source's MOVING part (fan blades)      nodes/fan.rs
//!  0.70  the companion cat                        nodes/cat.rs
//!  0.82  the hero's legs and torso                nodes/hero.rs
//!  0.90  the CEILING slab                         nodes/level.rs
//!  0.96  the hero's arm and cane                  nodes/hero.rs
//! ```
//!
//! Two entries sit closer than [`MIN_SEP`] on purpose — 0.33 against the
//! world's 0.34, 0.63 against 0.61 — and that is exactly what [`Fixed`]
//! exists for: a wall that touches a source is BANNED from the ids near it
//! and takes another from the palette, while walls elsewhere keep all five.
//! Reserving a clear band for every fixture would spend the channel on
//! adjacencies that never happen.
//!
//! The SOURCE band is reused across sources under the same law as the world
//! palette: the fan's housing and the radio's fascia share 0.33 because the
//! two stand rooms apart and can never touch. Should a level ever place two
//! sources against each other, that pair needs splitting — and the acoustic
//! image is drawn always-on-top, so their silhouette would still be carried
//! by the distance step; only the seam would be missing.
//!
//! ADDING AN OBJECT. If it is a solid a designer places, do nothing: the
//! colouring will paint it. If it paints its own limbs, pick an id at least
//! [`MIN_SEP`] from every id in the table above that it can TOUCH, add it
//! here, and hand it to the level as a [`Fixed`] anchor so the colouring
//! keeps its neighbours clear.

/// Ids at least this far apart draw a full-strength crease, straight off
/// the shader's `smoothstep(0.04, 0.08, nrm)` upper knee. Below it the seam
/// fades; at zero it is gone.
pub const MIN_SEP: f64 = 0.08;

/// Slack on the separation test. A palette laid out on exact decimal steps
/// misses its own nominal gap by an ULP — `0.31 - 0.23` is
/// 0.0799999999999999, just under [`MIN_SEP`] — and a law that rejected the
/// palette it was written for would be worse than no law.
const SLACK: f64 = 1e-9;

/// Do two ids draw a full-strength seam between them?
pub fn separated(a: f64, b: f64) -> bool {
    (a - b).abs() >= MIN_SEP - SLACK
}

/// Boxes sharing a face touch at exactly zero overlap — a wall's underside
/// sits precisely on the floor's top — so containment tests are grown by
/// this much before asking.
pub const TOUCH_EPS: f64 = 0.01;

/// The data pass's "no id given" sentinel, matching `u_oid`'s default in
/// `data_pass.gdshader`: the shader falls back to a normal-derived id.
pub const NO_OID: f64 = -1.0;

/// An axis-aligned box in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box3 {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Box3 {
    /// A box from its center and its FULL extent along each axis. Negative
    /// or zero sizes are legal and simply give a degenerate box, which
    /// still touches whatever contains it.
    pub fn from_center_size(center: [f64; 3], size: [f64; 3]) -> Self {
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        for axis in 0..3 {
            let half = (size[axis] * 0.5).abs();
            min[axis] = center[axis] - half;
            max[axis] = center[axis] + half;
        }
        Self { min, max }
    }

    /// The smallest box containing both.
    pub fn union(&self, other: &Self) -> Self {
        let mut min = [0.0; 3];
        let mut max = [0.0; 3];
        for axis in 0..3 {
            min[axis] = self.min[axis].min(other.min[axis]);
            max[axis] = self.max[axis].max(other.max[axis]);
        }
        Self { min, max }
    }

    /// This box grown by `margin` on the HORIZONTAL axes only — how a
    /// source whose head sweeps reports the volume it can actually reach,
    /// where the level samples a single pose. A negative margin is ignored:
    /// growing an anchor is always the safe direction, shrinking it is not.
    #[must_use]
    pub fn grown_flat(&self, margin: f64) -> Self {
        if margin.is_nan() || margin <= 0.0 {
            return *self;
        }
        let mut out = *self;
        for axis in [0, 2] {
            out.min[axis] -= margin;
            out.max[axis] += margin;
        }
        out
    }

    /// Do these two meet — overlapping, or sharing a face within
    /// [`TOUCH_EPS`]? Separation on any ONE axis is enough to miss.
    pub fn touches(&self, other: &Self) -> bool {
        (0..3).all(|axis| {
            self.min[axis] - TOUCH_EPS <= other.max[axis]
                && other.min[axis] - TOUCH_EPS <= self.max[axis]
        })
    }
}

/// A box whose id is decided elsewhere and cannot move — the floor and
/// ceiling slabs, the fan's housing. Colouring may not pick an id within
/// [`MIN_SEP`] of one of these for a box that touches it.
#[derive(Clone, Copy, Debug)]
pub struct Fixed {
    pub area: Box3,
    pub oid: f64,
}

/// The outcome: one id per input box, in input order, plus how many boxes
/// the palette could not satisfy. `starved` is zero on a healthy level and
/// is the caller's cue to say so loudly — never a panic, because a level
/// that colours badly must still be playable and inspectable.
#[derive(Clone, Debug, PartialEq)]
pub struct Assignment {
    pub oids: Vec<f64>,
    pub starved: usize,
}

/// Colour `boxes` from `palette` so that no two touching boxes land within
/// [`MIN_SEP`] of each other, nor within [`MIN_SEP`] of a `fixed` box they
/// touch.
///
/// Total for every input: an empty palette yields all [`NO_OID`] and counts
/// every box as starved; a box that cannot be satisfied takes the least
/// contended slot rather than failing the level.
///
/// Cost is O(n²) in the box count for the touch graph, which is the honest
/// price of an exact adjacency and is paid once, when the level is built.
pub fn assign(boxes: &[Box3], fixed: &[Fixed], palette: &[f64]) -> Assignment {
    let n = boxes.len();
    if palette.is_empty() {
        return Assignment {
            oids: vec![NO_OID; n],
            starved: n,
        };
    }

    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for j in (i + 1)..n {
            if boxes[i].touches(&boxes[j]) {
                adjacency[i].push(j);
                adjacency[j].push(i);
            }
        }
    }

    // slots a touching fixed-id neighbour rules out, box by box
    let mut banned: Vec<Vec<bool>> = vec![vec![false; palette.len()]; n];
    for (i, area) in boxes.iter().enumerate() {
        for anchor in fixed {
            if !area.touches(&anchor.area) {
                continue;
            }
            for (slot, id) in palette.iter().enumerate() {
                if !separated(*id, anchor.oid) {
                    banned[i][slot] = true;
                }
            }
        }
    }

    let (chosen, starved) = welsh_powell(&adjacency, &banned, palette.len());

    Assignment {
        oids: chosen.iter().map(|&slot| palette[slot]).collect(),
        starved,
    }
}

/// The greedy Welsh–Powell colouring core, shared by [`assign`] (over the
/// box-touch graph above) and `render::labels::assign` (over the superface
/// separation graph) so the one greedy/ban algorithm has one
/// implementation instead of two that could quietly drift apart.
/// `adjacency[i]` lists node `i`'s neighbours; `banned[i][slot]` marks a
/// palette slot node `i` may never take (some touching fixed anchor sits
/// within `MIN_SEP` of it). Node order is most-constrained-first (highest
/// degree), ties broken by index — the same stable, platform-independent
/// order [`assign`] always used, now named once rather than inlined twice.
///
/// It stays here, in the module Task 10 eventually retires, rather than
/// moving to `render/` now: this is still the WRITTEN-DOWN home of the
/// algorithm today, and `render::labels` borrowing it is exactly how a
/// migration is supposed to work — the new consumer reaches into the old
/// implementation until the old path dies, rather than forking a second
/// copy that has to be kept in sync by hand in the meantime.
///
/// Total for every input but one precondition the caller must uphold:
/// `palette_len` > 0 — both [`assign`] and `render::labels::assign` refuse
/// an empty palette before ever reaching here, because `palette_len == 0`
/// would make the starved fallback's `i % palette_len` divide by zero.
/// Never panics otherwise: a node the palette cannot satisfy takes the
/// least-contended slot it can get, counted in the returned `starved`
/// total, rather than failing its caller.
pub(crate) fn welsh_powell(
    adjacency: &[Vec<usize>],
    banned: &[Vec<bool>],
    palette_len: usize,
) -> (Vec<usize>, usize) {
    debug_assert!(palette_len > 0, "welsh_powell needs a non-empty palette");
    let n = adjacency.len();

    // most-constrained first, ties by index: a stable, platform-
    // independent order, which is what keeps two machines drawing one world
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| adjacency[b].len().cmp(&adjacency[a].len()).then(a.cmp(&b)));

    let mut chosen: Vec<usize> = vec![0; n];
    let mut decided: Vec<bool> = vec![false; n];
    let mut starved = 0;
    for &i in &order {
        let mut taken = vec![false; palette_len];
        for &j in &adjacency[i] {
            if decided[j] {
                taken[chosen[j]] = true;
            }
        }
        let free = (0..palette_len).find(|&slot| !taken[slot] && !banned[i][slot]);
        chosen[i] = match free {
            Some(slot) => slot,
            None => {
                starved += 1;
                // honour the neighbours we still can, then the banned slots
                (0..palette_len)
                    .find(|&slot| !taken[slot])
                    .or_else(|| (0..palette_len).find(|&slot| !banned[i][slot]))
                    .unwrap_or(i % palette_len)
            }
        };
        decided[i] = true;
    }

    (chosen, starved)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PALETTE: [f64; 5] = [0.25, 0.34, 0.43, 0.52, 0.61];

    fn unit_at(x: f64) -> Box3 {
        Box3::from_center_size([x, 0.0, 0.0], [1.0, 1.0, 1.0])
    }

    /// The whole point: two boxes that meet must not share an id, or their
    /// seam has nothing to draw it.
    #[test]
    fn touching_boxes_get_separable_ids() {
        let boxes = [unit_at(0.0), unit_at(0.9)];
        let out = assign(&boxes, &[], &PALETTE);
        assert_eq!(out.starved, 0);
        assert!(separated(out.oids[0], out.oids[1]));
    }

    /// ...and the converse, which is what makes a small palette enough:
    /// boxes that never meet reuse ids freely.
    #[test]
    fn distant_boxes_may_share_an_id() {
        let boxes = [unit_at(0.0), unit_at(50.0)];
        let out = assign(&boxes, &[], &PALETTE);
        assert_eq!(out.oids[0], out.oids[1]);
        assert_eq!(out.starved, 0);
    }

    /// A corridor of overlapping boxes, far more of them than the palette
    /// holds: index cycling would collide, colouring alternates instead.
    #[test]
    fn a_long_chain_alternates_without_starving() {
        let boxes: Vec<Box3> = (0..40).map(|i| unit_at(i as f64 * 0.9)).collect();
        let out = assign(&boxes, &[], &PALETTE);
        assert_eq!(out.starved, 0);
        for i in 0..boxes.len() {
            for j in (i + 1)..boxes.len() {
                if boxes[i].touches(&boxes[j]) {
                    assert!(
                        separated(out.oids[i], out.oids[j]),
                        "chain links {i} and {j} melted together"
                    );
                }
            }
        }
    }

    /// A swept anchor covers ground the sampled pose does not: a box just
    /// outside a fan's still silhouette is inside the volume its head
    /// reaches, and must be banned from the ids it could melt into.
    #[test]
    fn a_grown_anchor_reaches_what_the_sampled_pose_misses() {
        let still = Box3::from_center_size([0.0, 1.0, 0.0], [0.5, 2.0, 0.5]);
        let crate_beside = Box3::from_center_size([0.6, 0.3, 0.0], [0.5, 0.6, 0.5]);
        assert!(!still.touches(&crate_beside));
        assert!(still.grown_flat(0.35).touches(&crate_beside));
        // ...and only sideways: the sweep is a yaw, so the ceiling above is
        // no closer than it was
        let swept = still.grown_flat(0.35);
        assert_eq!(swept.min[1], still.min[1]);
        assert_eq!(swept.max[1], still.max[1]);
        // a still source asks for nothing and is unchanged
        assert_eq!(still.grown_flat(0.0), still);
        assert_eq!(still.grown_flat(-1.0), still);
        assert_eq!(still.grown_flat(f64::NAN), still);
    }

    /// Boxes sharing a face exactly — a wall standing on a floor — count as
    /// touching. Without the epsilon this is the pair that silently melts.
    #[test]
    fn boxes_sharing_a_face_exactly_are_touching() {
        let floor = Box3::from_center_size([0.0, -0.05, 0.0], [10.0, 0.1, 10.0]);
        let wall = Box3::from_center_size([0.0, 1.5, 0.0], [4.0, 3.0, 0.3]);
        assert!(floor.touches(&wall));
        assert!(wall.touches(&floor));
    }

    /// A fixed-id neighbour pushes a box off the ids near it, but only a box
    /// that actually touches it.
    #[test]
    fn a_fixed_neighbour_bans_only_its_own_touchers() {
        let fan = Fixed {
            area: unit_at(0.0),
            oid: 0.33,
        };
        let boxes = [unit_at(0.9), unit_at(50.0)];
        let out = assign(&boxes, &[fan], &PALETTE);
        assert_eq!(out.starved, 0);
        // 0.31 and 0.39 sit within 0.08 of the fan and are refused
        assert!(separated(out.oids[0], fan.oid));
        // the far box is unconstrained and takes the first slot
        assert_eq!(out.oids[1], PALETTE[0]);
    }

    /// The floor touches every wall in the world, so the palette exists to
    /// be clear of it: its presence must cost the colouring nothing.
    #[test]
    fn a_floor_under_everything_constrains_nothing() {
        let floor = Fixed {
            area: Box3::from_center_size([20.0, -0.05, 0.0], [80.0, 0.1, 80.0]),
            oid: 0.15,
        };
        let boxes: Vec<Box3> = (0..12)
            .map(|i| Box3::from_center_size([i as f64 * 0.9, 1.5, 0.0], [1.0, 3.0, 1.0]))
            .collect();
        let bare = assign(&boxes, &[], &PALETTE);
        let floored = assign(&boxes, &[floor], &PALETTE);
        assert_eq!(bare, floored);
        assert_eq!(floored.starved, 0);
    }

    /// More mutually touching boxes than the palette can separate: the
    /// level still gets ids and still runs, and the caller is told.
    #[test]
    fn an_impossible_clique_is_reported_not_panicked() {
        let boxes: Vec<Box3> = (0..7).map(|_| unit_at(0.0)).collect();
        let out = assign(&boxes, &[], &PALETTE);
        assert_eq!(out.oids.len(), 7);
        assert_eq!(out.starved, 2);
        assert!(out.oids.iter().all(|id| PALETTE.contains(id)));
    }

    /// Two machines must colour one scene identically — the wasm build and
    /// the desktop build draw the same world or neither is trustworthy.
    #[test]
    fn colouring_is_deterministic() {
        let boxes: Vec<Box3> = (0..30).map(|i| unit_at(i as f64 * 0.6)).collect();
        let fixed = [Fixed {
            area: unit_at(3.0),
            oid: 0.33,
        }];
        let first = assign(&boxes, &fixed, &PALETTE);
        for _ in 0..8 {
            assert_eq!(assign(&boxes, &fixed, &PALETTE), first);
        }
    }

    /// Total on the degenerate inputs: no boxes, and no palette to draw on.
    #[test]
    fn empty_inputs_are_answered_not_crashed() {
        assert_eq!(
            assign(&[], &[], &PALETTE),
            Assignment {
                oids: vec![],
                starved: 0
            }
        );
        let out = assign(&[unit_at(0.0), unit_at(9.0)], &[], &[]);
        assert_eq!(out.oids, vec![NO_OID, NO_OID]);
        assert_eq!(out.starved, 2);
    }

    /// Every palette entry the level ships must be far enough from its
    /// neighbours to draw at full strength — a palette that violates its own
    /// law would make the colouring pointless.
    #[test]
    fn the_shipped_palette_separates_its_own_entries() {
        for (i, near) in PALETTE.iter().enumerate() {
            for (j, far) in PALETTE.iter().enumerate().skip(i + 1) {
                assert!(
                    separated(*near, *far),
                    "palette entries {i} and {j} are too close"
                );
            }
        }
    }
}
