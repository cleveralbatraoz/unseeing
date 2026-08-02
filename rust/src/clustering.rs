//! Reflection-hit clustering — how a flat wall answers as a few points
//! instead of one point per ray. Mirrored exactly from pulses.gd's
//! `emit_reflecting`: every ray hit lands in a 0.9 m grid cell, the
//! nearest hit per cell wins, and the survivors — nudged just off their
//! surface — are ranked by distance and capped at the caller's echo
//! budget.
//!
//! Determinism is construction, not luck: cells live in a BTreeMap (key
//! order, no hash-seed roulette) and the final ranking is a TOTAL order —
//! distance first, cell key as the tiebreak. The GDScript original sorted
//! by distance alone with Godot's unstable sort, so equal distances could
//! land either way; this module pins the stricter order so the same hits
//! always answer in the same sequence on every platform.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use godot::builtin::Vector3;

/// Reflection hits within the same cell merge, so flat walls answer as a
/// few points instead of one point per ray.
pub const CLUSTER_CELL: f32 = 0.9;

/// Hits closer than this to the ray origin are the surface the sound
/// itself was born on — it must not answer itself.
pub const SELF_SURFACE: f64 = 0.3;

/// Answering points are nudged this far off the struck surface, so the
/// echo they spawn is born in air, not inside a collider.
pub const SURFACE_OFFSET: f32 = 0.02;

/// A reflection ray never reaches past this many meters, whatever the
/// primary wave's range — the fan samples nearby geometry, not the map.
pub const RAY_REACH_CAP: f64 = 6.0;

/// A reflection ray reaches this fraction of the primary wave's range:
/// surfaces near the rim would answer after the wave already died there.
pub const RAY_REACH_FRACTION: f64 = 0.8;

/// One physics ray strike while sampling reflections: where, facing which
/// way, and how far from the sound's ray origin. `dist` is f32 — a
/// single-precision Vector3 length, exactly what the engine hands back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// The struck point on the surface.
    pub position: Vector3,
    /// The struck surface's normal.
    pub normal: Vector3,
    /// Distance from the sound's ray origin to `position`.
    pub dist: f32,
}

/// One clustered answering point: distance from the sound's origin, and
/// the point nudged just off the struck surface — the GDScript
/// `SurfaceHit {d, p}`, field for field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceHit {
    /// Distance from the sound's ray origin (drives echo timing and gain).
    pub dist: f32,
    /// The answering point: `position + normal * SURFACE_OFFSET`.
    pub point: Vector3,
}

/// The 0.9 m grid cell a point falls in — GDScript's
/// `Vector3i((hit_pos / CLUSTER_CELL).floor())`: single-precision divide
/// and floor, then integer cast, so negative coordinates floor toward
/// minus infinity exactly as the original's did.
#[must_use]
pub fn cell_key(position: Vector3) -> (i32, i32, i32) {
    let scaled = position / CLUSTER_CELL;
    (
        scaled.x.floor() as i32,
        scaled.y.floor() as i32,
        scaled.z.floor() as i32,
    )
}

/// True when a hit is the surface the sound itself was born on. The
/// comparison happens in f64 — GDScript widened the f32 length before
/// comparing against its 64-bit 0.3 — so a dist of exactly 0.3f32
/// (which widens ABOVE the f64 threshold) is kept, not rejected.
#[must_use]
pub fn is_self_surface(dist: f32) -> bool {
    f64::from(dist) < SELF_SURFACE
}

/// How far a reflection ray may travel for a primary wave of range
/// `max_r`: min(max_r * 0.8, 6.0), computed in f64 like the GDScript
/// `minf` — the caller narrows to f32 only when scaling the ray vector,
/// as the original did.
#[must_use]
pub fn ray_length(max_r: f64) -> f64 {
    (max_r * RAY_REACH_FRACTION).min(RAY_REACH_CAP)
}

/// Cluster ray hits into at most `max_echoes` answering points.
///
/// The laws, verbatim from `emit_reflecting`:
/// - self-surface hits (dist < 0.3, compared in f64) never answer;
/// - hits sharing a cell merge, the strictly nearer one winning — on an
///   exact distance tie the FIRST hit keeps the cell (GDScript's
///   `cells[key].d > dist`);
/// - each answering point is the hit nudged off its surface by
///   `normal * SURFACE_OFFSET`;
/// - survivors rank by distance, then cell key — a total order, so any
///   permutation of the same hits yields the identical answer list
///   (stricter than the original's unstable distance-only sort);
/// - the nearest `max_echoes` survive the cap.
#[must_use]
pub fn cluster_hits(hits: impl IntoIterator<Item = RayHit>, max_echoes: usize) -> Vec<SurfaceHit> {
    let mut cells: BTreeMap<(i32, i32, i32), SurfaceHit> = BTreeMap::new();
    for hit in hits {
        if is_self_surface(hit.dist) {
            continue;
        }
        let answer = SurfaceHit {
            dist: hit.dist,
            point: hit.position + hit.normal * SURFACE_OFFSET,
        };
        match cells.entry(cell_key(hit.position)) {
            Entry::Vacant(v) => {
                v.insert(answer);
            }
            Entry::Occupied(mut o) => {
                if o.get().dist > hit.dist {
                    o.insert(answer);
                }
            }
        }
    }
    let mut found: Vec<((i32, i32, i32), SurfaceHit)> = cells.into_iter().collect();
    // total_cmp keeps the order total even for pathological dist values;
    // finite distances rank exactly as the original's `<`.
    found.sort_by(|a, b| a.1.dist.total_cmp(&b.1.dist).then_with(|| a.0.cmp(&b.0)));
    found.truncate(max_echoes);
    found.into_iter().map(|(_, h)| h).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(position: Vector3, normal: Vector3, dist: f32) -> RayHit {
        RayHit {
            position,
            normal,
            dist,
        }
    }

    /// Two rays striking the same 0.9 m cell answer once, from the nearer
    /// strike — a flat wall answers as a point, not a ray count.
    #[test]
    fn same_cell_merge_keeps_nearest() {
        let n = Vector3::new(1.0, 0.0, 0.0);
        let far = hit(Vector3::new(0.1, 0.0, 0.1), n, 2.0);
        let near = hit(Vector3::new(0.2, 0.0, 0.2), n, 1.5);
        let out = cluster_hits([far, near], 6);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].dist, 1.5);
        assert_eq!(out[0].point, near.position + n * SURFACE_OFFSET);
    }

    /// An exact distance tie keeps the FIRST hit — GDScript's strict
    /// `cells[key].d > dist` never replaces on equality. Pinned, because
    /// slot order downstream depends on which point answers.
    #[test]
    fn same_cell_tie_keeps_first() {
        let n = Vector3::new(0.0, 1.0, 0.0);
        let first = hit(Vector3::new(0.1, 0.0, 0.1), n, 1.5);
        let second = hit(Vector3::new(0.3, 0.0, 0.3), n, 1.5);
        let out = cluster_hits([first, second], 6);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].point, first.position + n * SURFACE_OFFSET);
    }

    /// The echo budget holds: only the nearest `max_echoes` cells answer.
    #[test]
    fn cap_keeps_the_nearest() {
        let n = Vector3::new(0.0, 1.0, 0.0);
        let hits = [
            hit(Vector3::new(4.0, 0.0, 0.0), n, 4.0),
            hit(Vector3::new(1.0, 0.0, 0.0), n, 1.0),
            hit(Vector3::new(2.0, 0.0, 0.0), n, 2.0),
        ];
        let out = cluster_hits(hits, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].dist, 1.0);
        assert_eq!(out[1].dist, 2.0);
    }

    /// Determinism under permutation: fixed reorderings of the same hits
    /// (no RNG anywhere) produce the byte-identical answer list. Distances
    /// are distinct where cells compete, so nearest-wins decides — and the
    /// equal-distance pair across DIFFERENT cells is decided by the cell
    /// key, the total order the module pins.
    #[test]
    fn permuted_input_answers_identically() {
        let n = Vector3::new(0.0, 1.0, 0.0);
        let hits = [
            hit(Vector3::new(0.1, 0.0, 0.1), n, 2.0), // cell (0,0,0), loses to the 1.4
            hit(Vector3::new(0.2, 0.0, 0.2), n, 1.4), // cell (0,0,0), wins it
            hit(Vector3::new(1.0, 0.0, 0.0), n, 1.0), // cell (1,0,0)
            hit(Vector3::new(-1.0, 0.0, 0.0), n, 1.0), // cell (-2,0,0): dist tie vs (1,0,0)
            hit(Vector3::new(2.0, 0.0, 2.0), n, 3.0), // cell (2,0,2)
            hit(Vector3::new(0.0, 2.0, 0.0), n, 2.5), // cell (0,2,0)
        ];
        let canonical = cluster_hits(hits, 6);
        assert_eq!(canonical.len(), 5);
        // the tied pair: the smaller cell key (-2,0,0) answers first
        assert_eq!(canonical[0].dist, 1.0);
        assert_eq!(canonical[0].point.x, -1.0);
        assert_eq!(canonical[1].dist, 1.0);
        assert_eq!(canonical[1].point.x, 1.0);
        for perm in [[3, 0, 5, 1, 4, 2], [5, 4, 3, 2, 1, 0], [2, 4, 0, 3, 1, 5]] {
            let shuffled: Vec<RayHit> = perm.into_iter().map(|i| hits[i]).collect();
            // ties within a cell are input-order-dependent (as in GDScript);
            // these hits have none, so every permutation must agree
            assert_eq!(cluster_hits(shuffled, 6), canonical);
        }
    }

    /// Every answering point sits exactly `SURFACE_OFFSET` off its surface
    /// along the normal — born in air, not inside the collider.
    #[test]
    fn answering_points_are_nudged_off_surface() {
        let n = Vector3::new(0.0, 0.0, -1.0);
        let p = Vector3::new(1.3, 0.7, 2.0);
        let out = cluster_hits([hit(p, n, 2.4)], 6);
        assert_eq!(out[0].point, p + n * SURFACE_OFFSET);
    }

    /// Cell keys floor toward minus infinity, exactly like
    /// `Vector3i(v.floor())`: -0.1 / 0.9 lands in cell -1, not 0.
    #[test]
    fn cell_key_floors_negative_coordinates() {
        assert_eq!(cell_key(Vector3::new(-0.1, 0.0, 0.0)), (-1, 0, 0));
        assert_eq!(cell_key(Vector3::new(0.89, 0.0, 0.0)), (0, 0, 0));
        assert_eq!(cell_key(Vector3::new(0.91, -0.91, 1.81)), (1, -2, 2));
    }

    /// The self-surface veto compares in f64, as the original did: 0.29 is
    /// the birth surface, 0.31 is not — and exactly 0.3f32 is KEPT,
    /// because it widens to 0.30000001..., above the 64-bit threshold.
    #[test]
    fn self_surface_veto_is_a_f64_comparison() {
        assert!(is_self_surface(0.29));
        assert!(!is_self_surface(0.31));
        assert!(!is_self_surface(0.3f32));
        let n = Vector3::new(0.0, 1.0, 0.0);
        let out = cluster_hits([hit(Vector3::new(0.1, 0.0, 0.0), n, 0.29)], 6);
        assert!(out.is_empty());
    }

    /// The ray reach: 0.8 of the wave's range, never past 6 m — the exact
    /// minf(max_r * 0.8, 6.0) of the original (the cane tap's 6.0 m wave
    /// samples 4.8 m, the echo test suite's pinned RAY_LEN).
    #[test]
    fn ray_length_clamps_the_reach() {
        assert_eq!(ray_length(6.0), 6.0 * 0.8);
        assert!((ray_length(6.0) - 4.8).abs() < 1e-9);
        assert_eq!(ray_length(10.0), 6.0);
        assert_eq!(ray_length(0.5), 0.4);
    }

    /// Equal distances across different cells rank by cell key — the total
    /// order that replaces the original's unstable ties.
    #[test]
    fn equal_distances_rank_by_cell_key() {
        let n = Vector3::new(0.0, 1.0, 0.0);
        let hi = hit(Vector3::new(0.0, 2.0, 0.0), n, 2.0); // cell (0,2,0)
        let lo = hit(Vector3::new(0.0, 1.0, 0.0), n, 2.0); // cell (0,1,0)
        let out = cluster_hits([hi, lo], 6);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].point, lo.position + n * SURFACE_OFFSET);
        assert_eq!(out[1].point, hi.position + n * SURFACE_OFFSET);
    }
}
