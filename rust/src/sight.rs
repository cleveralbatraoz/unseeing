//! Sight as a pure function — which walls a straight line pierces. The
//! acoustic-image shaders draw sound sources on top of everything (their
//! rasterized depth is faked, so the hardware depth test cannot occlude
//! them); walls must therefore occlude ANALYTICALLY, in the fragment
//! shader, by counting how many wall boxes the sight line from the camera
//! to the shaded point crosses. That counter is defined HERE, cargo-pinned,
//! and the GLSL in `pulse_pool.gdshaderinc` is its literal transliteration
//! — the total-functions doctrine applied to shader math.
//!
//! Geometry: a wall's occluder is its centerline segment inflated into a
//! world XZ rect ([`wall_rect`]), swept floor to ceiling. The rect is
//! SHRUNK by [`RECT_SHRINK`] relative to the wall's real box so a prop
//! standing flush against a wall face is never self-shadowed by contact
//! grazing, and both parametric ends of the sight line are ignored by
//! [`GRAZE_EPS`] so neither the camera nor the shaded fragment counts a
//! surface it merely touches.

use godot::builtin::{Vector3, Vector4};

use crate::level_plan;

/// Wall slots the sight shaders allocate (`u_walls[MAXW]`) — a level
/// with more walls than this cannot be occluded honestly and says so.
pub const MAXW: usize = 16;

/// Meters the occluder rect stops short of the wall's real face, so a
/// prop flush against the wall keeps an unblocked sight line.
pub const RECT_SHRINK: f64 = 0.02;

/// Parametric fraction ignored at each end of the sight line: a crossing
/// counts only with t strictly inside (GRAZE_EPS, 1 - GRAZE_EPS).
pub const GRAZE_EPS: f64 = 0.001;

/// A direction component below this is treated as axis-parallel — the
/// segment can never cross that pair of slab planes.
pub const AXIS_TINY: f32 = 1e-6;

/// A wall centerline segment (x1, z1, x2, z2), inflated into the world XZ
/// occluder rect (min_x, min_z, max_x, max_z) the sight test runs
/// against: a wall half-thickness of padding each way, shrunk by
/// [`RECT_SHRINK`] — the `u_walls` layout.
#[must_use]
pub fn wall_rect(segment: Vector4) -> Vector4 {
    let pad = (level_plan::WALL_T - RECT_SHRINK) as f32;
    Vector4::new(
        segment.x.min(segment.z) - pad,
        segment.y.min(segment.w) - pad,
        segment.x.max(segment.z) + pad,
        segment.y.max(segment.w) + pad,
    )
}

/// Whether the segment `from -> to` crosses the wall box `rect` swept
/// y ∈ [0, `wall_top`] — the classic three-slab test, clamped to the
/// graze-free parametric window. Total on any input: a zero direction
/// component degenerates to a point-in-slab check.
#[must_use]
pub fn crosses(from: Vector3, to: Vector3, rect: Vector4, wall_top: f32) -> bool {
    let a = [from.x, from.y, from.z];
    let d = [to.x - from.x, to.y - from.y, to.z - from.z];
    let lo = [rect.x, 0.0, rect.y];
    let hi = [rect.z, wall_top, rect.w];
    let mut t0 = GRAZE_EPS as f32;
    let mut t1 = (1.0 - GRAZE_EPS) as f32;
    for k in 0..3 {
        if d[k].abs() < AXIS_TINY {
            if a[k] < lo[k] || a[k] > hi[k] {
                return false;
            }
        } else {
            let ta = (lo[k] - a[k]) / d[k];
            let tb = (hi[k] - a[k]) / d[k];
            t0 = t0.max(ta.min(tb));
            t1 = t1.min(ta.max(tb));
            if t0 > t1 {
                return false;
            }
        }
    }
    true
}

/// How many of the wall rects the sight line `from -> to` crosses — the
/// muffle exponent (0.55^n) and the discard predicate (n > 0) of the
/// acoustic-image shaders. This is the CAMERA occluder (eye -> shaded
/// point): every wall the line pierces counts.
#[must_use]
pub fn crossings(from: Vector3, to: Vector3, rects: &[Vector4], wall_top: f32) -> u32 {
    rects
        .iter()
        .map(|r| u32::from(crosses(from, to, *r, wall_top)))
        .sum()
}

/// Whether the point `p` lies inside the wall box `rect` swept
/// y ∈ [0, `wall_top`] — the XZ rect AND the vertical span. Total on any
/// input. The wall a sound is born inside cannot block that sound's own
/// reveal, so [`crossings_from`] skips whatever this reports.
#[must_use]
pub fn contains(rect: Vector4, p: Vector3, wall_top: f32) -> bool {
    p.x >= rect.x
        && p.x <= rect.z
        && p.z >= rect.y
        && p.z <= rect.w
        && p.y >= 0.0
        && p.y <= wall_top
}

/// How many wall rects the sight line `from -> to` crosses, IGNORING any
/// rect that already [`contains`] `from`: the wall a sound is born inside
/// — a cane tap struck flush on it, a source standing within a
/// half-thickness — never occludes that sound's own reveal. This is the
/// SOURCE occluder (source -> lit point); [`crossings`] is the CAMERA
/// occluder (eye -> lit point). The two differ only on the birth wall:
/// a source reaches its OWN wall's near face, but the eye behind that
/// wall still cannot.
#[must_use]
pub fn crossings_from(from: Vector3, to: Vector3, rects: &[Vector4], wall_top: f32) -> u32 {
    rects
        .iter()
        .filter(|r| !contains(**r, from, wall_top))
        .map(|r| u32::from(crosses(from, to, *r, wall_top)))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WALL_TOP: f32 = level_plan::WALL_H as f32;

    /// The shipped map's wall centerlines — the same fixture level_plan's
    /// suites derive from, inflated here into sight occluders.
    fn shipped_rects() -> Vec<Vector4> {
        [
            Vector4::new(0.6, 0.6, 19.4, 0.6),
            Vector4::new(19.4, 0.6, 19.4, 19.4),
            Vector4::new(19.4, 19.4, 0.6, 19.4),
            Vector4::new(0.6, 19.4, 0.6, 0.6),
            Vector4::new(6.4, 0.6, 6.4, 8.0),
            Vector4::new(6.4, 12.4, 6.4, 19.4),
            Vector4::new(6.4, 8.0, 14.0, 8.0),
            Vector4::new(14.0, 8.0, 14.0, 15.6),
            Vector4::new(9.0, 15.6, 14.0, 15.6),
            Vector4::new(0.6, 13.0, 4.0, 13.0),
        ]
        .iter()
        .map(|s| wall_rect(*s))
        .collect()
    }

    /// The inflation is a half-thickness pad shrunk by the contact
    /// epsilon, and reversed segments normalize into min/max order.
    #[test]
    fn wall_rect_inflates_and_normalizes() {
        let divider = wall_rect(Vector4::new(6.4, 0.6, 6.4, 8.0));
        assert!((divider.x - 6.27).abs() < 1e-4);
        assert!((divider.y - 0.47).abs() < 1e-4);
        assert!((divider.z - 6.53).abs() < 1e-4);
        assert!((divider.w - 8.13).abs() < 1e-4);
        let reversed = wall_rect(Vector4::new(19.4, 19.4, 0.6, 19.4));
        assert!((reversed.x - 0.47).abs() < 1e-4);
        assert!((reversed.z - 19.53).abs() < 1e-4);
    }

    /// Spawn to fan head: exactly one wall (DividerNorth) stands between
    /// the hero's waking pose and the hum — the muffle exponent the fan's
    /// through-wall outline rides.
    #[test]
    fn spawn_to_fan_crosses_exactly_one_wall() {
        let n = crossings(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 1.15, 4.4),
            &shipped_rects(),
            WALL_TOP,
        );
        assert_eq!(n, 1);
    }

    /// A sight line within one room crosses nothing: same-room props keep
    /// their full reveal.
    #[test]
    fn same_room_sight_line_crosses_nothing() {
        let n = crossings(
            Vector3::new(8.0, 1.0, 4.0),
            Vector3::new(12.0, 1.5, 6.0),
            &shipped_rects(),
            WALL_TOP,
        );
        assert_eq!(n, 0);
    }

    /// A diagonal from the spawn room into the far corridor pierces the
    /// divider and the fan room's south wall: two crossings, muffled
    /// twice.
    #[test]
    fn two_wall_diagonal_counts_two() {
        let n = crossings(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 10.0),
            &shipped_rects(),
            WALL_TOP,
        );
        assert_eq!(n, 2);
    }

    /// A prop flush against a wall face is NOT self-blocked: the shrunk
    /// rect leaves its surface a clear 0.02 m outside — while the same
    /// point against the UNSHRUNK wall box would already count as inside.
    #[test]
    fn flush_prop_is_not_self_blocked_by_the_shrink() {
        let eye = Vector3::new(8.0, 1.2, 4.0);
        // 5 mm inside the wall's REAL east face at x = 6.55 — where world
        // reconstruction dust can land a fragment of a flush prop
        let grazed = Vector3::new(6.545, 1.2, 4.0);
        assert_eq!(crossings(eye, grazed, &shipped_rects(), WALL_TOP), 0);
        let unshrunk = Vector4::new(6.25, 0.45, 6.55, 8.15);
        assert!(crosses(eye, grazed, unshrunk, WALL_TOP));
    }

    /// A wall BEHIND the shaded point never blocks it: the divider lies
    /// past the target, outside the parametric window.
    #[test]
    fn wall_behind_the_target_does_not_count() {
        let n = crossings(
            Vector3::new(8.0, 1.0, 4.0),
            Vector3::new(7.0, 1.0, 4.4),
            &shipped_rects(),
            WALL_TOP,
        );
        assert_eq!(n, 0);
    }

    /// Grazing an occluder face exactly at either endpoint is not a
    /// crossing: a fragment ON the rect, or a camera flush against it,
    /// stays unblocked — the GRAZE_EPS window at work.
    #[test]
    fn endpoint_grazes_are_not_crossings() {
        let divider = wall_rect(Vector4::new(6.4, 0.6, 6.4, 8.0));
        let on_face = Vector3::new(6.27, 0.9, 4.0);
        assert!(!crosses(
            Vector3::new(3.0, 0.9, 4.0),
            on_face,
            divider,
            WALL_TOP
        ));
        assert!(!crosses(
            on_face,
            Vector3::new(3.0, 0.9, 4.0),
            divider,
            WALL_TOP
        ));
    }

    /// A sight line OVER the walls is clear: the slab test is 3D, and a
    /// wall stops at the ceiling.
    #[test]
    fn sight_over_the_wall_top_is_clear() {
        let divider = wall_rect(Vector4::new(6.4, 0.6, 6.4, 8.0));
        assert!(!crosses(
            Vector3::new(3.0, 3.2, 4.0),
            Vector3::new(8.6, 3.4, 4.4),
            divider,
            WALL_TOP,
        ));
    }

    /// `contains` reads the padded rect as a solid box floor to ceiling:
    /// a point on the divider centerline is inside; the same point at the
    /// spawn is outside; a point above the wall top is outside.
    #[test]
    fn contains_reads_the_padded_box() {
        let divider = wall_rect(Vector4::new(6.4, 0.6, 6.4, 8.0));
        assert!(contains(divider, Vector3::new(6.4, 0.9, 4.0), WALL_TOP));
        assert!(!contains(divider, Vector3::new(3.0, 0.9, 4.0), WALL_TOP));
        assert!(!contains(divider, Vector3::new(6.4, 3.2, 4.0), WALL_TOP));
    }

    /// The birth-wall skip: a source standing ON the divider centerline
    /// lighting an open fan-room point is blocked by NO wall through
    /// `crossings_from` (its own wall skipped) — while the camera-side
    /// `crossings` still counts that wall it exits. The two occluders
    /// diverge exactly on the birth wall.
    #[test]
    fn source_is_not_blocked_by_the_wall_it_is_born_in() {
        let born_in_divider = Vector3::new(6.4, 0.9, 4.0);
        let open_fan_room = Vector3::new(10.0, 0.9, 4.0);
        assert_eq!(
            crossings_from(born_in_divider, open_fan_room, &shipped_rects(), WALL_TOP),
            0,
        );
        assert_eq!(
            crossings(born_in_divider, open_fan_room, &shipped_rects(), WALL_TOP),
            1,
        );
    }

    /// The skip is surgical: a source born inside the divider still has
    /// every OTHER wall block it — the diagonal into the far corridor
    /// crosses FanRoomSouth, counted once (the divider it is born in is
    /// not).
    #[test]
    fn birth_wall_skip_still_counts_every_other_wall() {
        let born_in_divider = Vector3::new(6.4, 0.9, 4.0);
        let far_corridor = Vector3::new(10.0, 0.9, 10.0);
        assert_eq!(
            crossings_from(born_in_divider, far_corridor, &shipped_rects(), WALL_TOP),
            1,
        );
    }

    /// A source standing clear of every wall occludes identically either
    /// way: with nothing to skip, `crossings_from` equals `crossings` —
    /// the spawn-to-fan line still counts its one divider.
    #[test]
    fn source_clear_of_walls_matches_the_camera_occluder() {
        let spawn = Vector3::new(3.0, 0.9, 4.0);
        let fan = Vector3::new(8.6, 1.15, 4.4);
        assert_eq!(
            crossings_from(spawn, fan, &shipped_rects(), WALL_TOP),
            crossings(spawn, fan, &shipped_rects(), WALL_TOP),
        );
        assert_eq!(crossings_from(spawn, fan, &shipped_rects(), WALL_TOP), 1);
    }
}
