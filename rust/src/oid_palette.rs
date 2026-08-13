//! Geometry and colouring PRIMITIVES the object-id budget is built from —
//! not the budget itself anymore.
//!
//! Before the superface campaign, this module ran the whole per-SOLID
//! colouring: build the touch graph over every box in the level, colour it
//! Welsh–Powell, done. That is gone — `render::superface::superfaces` and
//! `render::labels::assign` run the identical idea one level down, over the
//! FACE graph, so that two overlapping solids' coplanar faces can MERGE
//! into one class before a label is ever handed out rather than merely
//! avoiding each other's colour. `Fixed`, `Assignment` and `assign` — the
//! per-solid machinery — were retired with that migration; every remaining
//! caller of a fixed id now goes through `render::labels::role_label`'s one
//! table, and every remaining caller of a colouring goes through
//! `render::labels::assign`.
//!
//! What SURVIVES here is what still has a live consumer outside `render/`:
//! - [`Box3`] — the axis-aligned world box every static solid measures
//!   itself by (`WaveLevel::mesh_world_box` and its callers), with the
//!   touch/union/grow primitives `observe::oids`'s SOLID-granularity law
//!   still reasons over.
//! - [`TOUCH_EPS`] — the epsilon [`Box3::touches`] grows a box by before
//!   asking, since boxes sharing a face overlap by exactly zero.
//! - [`NO_OID`] — the "nothing painted this yet" sentinel, read by
//!   `nodes::solid`/`nodes::level` wherever a mesh's `CUSTOM0` cannot be
//!   read at all.
//! - [`welsh_powell`] — the shared greedy colouring core `render::labels::assign`
//!   borrows rather than forking; this stays its written-down home only
//!   because moving it is not itself a load-bearing change.
//!
//! The outline pass still draws a line two ways (`hearing_post.gdshader`):
//! a Laplacian of the packed distance in B catches SILHOUETTES, where the
//! world steps away from itself; a difference of the flat object id in G
//! catches CREASES, faded over `smoothstep(0.04, 0.08, ..)`. Where two
//! boxes interpenetrate there is no depth step, so the silhouette term has
//! nothing to bite on — the crease is the only thing that can draw their
//! seam. The threshold that law is stated in, `MIN_SEP`, lives in
//! `render::labels` and nowhere else: this module used to carry a second,
//! textually independent copy of it with nothing asserting the two agreed,
//! and `observe::oids` — its one consumer here — reads the render
//! subsystem's own constant now.

/// Boxes sharing a face touch at exactly zero overlap — a wall's underside
/// sits precisely on the floor's top — so containment tests are grown by
/// this much before asking.
pub const TOUCH_EPS: f64 = 0.01;

/// The engine-side "no id given" sentinel: what a census reports for a
/// solid whose mesh carries no `CUSTOM0` channel at all, never a value the
/// shader itself sees — the data pass reads `CUSTOM0` straight through for
/// every drawn vertex and has no separate "unset" case of its own.
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

/// The greedy Welsh–Powell colouring core `render::labels::assign` borrows
/// (over the superface separation graph) rather than forking a second
/// implementation that could quietly drift apart. Once shared with this
/// module's own retired `assign` (the per-solid touch-graph colouring);
/// `render::labels::assign` is its only caller now.
/// `adjacency[i]` lists node `i`'s neighbours; `banned[i][slot]` marks a
/// palette slot node `i` may never take (some touching fixed anchor sits
/// within `MIN_SEP` of it). Node order is most-constrained-first (highest
/// degree), ties broken by index — the same stable, platform-independent
/// order this crate's colouring has always used.
///
/// It stays here rather than moving to `render/`: this is still the
/// WRITTEN-DOWN home of the algorithm, and `render::labels` borrowing it
/// is exactly how the migration away from per-solid colouring worked — the
/// new consumer reached into the old implementation rather than forking a
/// second copy to keep in sync by hand. Moving it now would be a pure
/// relocation with no behaviour riding on it, so it is left where it is.
///
/// Total for every input but one precondition the caller must uphold:
/// `palette_len` > 0 — `render::labels::assign` refuses an empty palette
/// before ever reaching here, because `palette_len == 0` would make the
/// starved fallback's `i % palette_len` divide by zero. Never panics
/// otherwise: a node the palette cannot satisfy takes the least-contended
/// slot it can get, counted in the returned `starved` total, rather than
/// failing its caller.
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
}
