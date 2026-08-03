//! The level's technical plan, derived — how an editor-authored scene of
//! dragged-around wall nodes becomes the exact contracts the engine runs
//! on: box dimensions, axis-snapped orientations, wall centerlines, the
//! hum room around a sound source, and the dev demo tap. A designer only
//! places and rotates nodes; every number the systems need is computed
//! here, so the geometry and the contracts derived from it can never
//! drift apart in two files.
//!
//! Precision law, pinned from the retired GDScript map builder: GDScript
//! floats are f64, so every scalar knob here is f64 and arithmetic
//! narrows to the engine's f32 lanes exactly where the original assigned
//! into a Vector — no earlier. The scene work (meshes, colliders, child
//! walks) stays in the engine layer ([`crate::nodes`]); this module owns
//! only the math the cargo tests pin.
//!
//! Axis law: walls are axis-aligned boxes — the hum-room rect the shader
//! clips by and the centerline table the suites hold invariants against
//! both depend on it. A designer's free-hand rotation is therefore
//! snapped to the nearest quarter turn, and the snapped basis is built
//! from exact unit columns: no trig dust for the rasterizer to chew on.

use std::f64::consts::FRAC_PI_2;

use godot::builtin::{Basis, Vector3, Vector4};

/// Wall height in meters — walls run floor to ceiling.
pub const WALL_H: f64 = 3.0;

/// Half-thickness of a wall in meters.
pub const WALL_T: f64 = 0.15;

/// Thickness of the floor and ceiling slabs.
pub const SLAB_T: f64 = 0.1;

/// The hero's capsule center over the floor a spawn marker stands on.
pub const SPAWN_LIFT: f64 = 0.9;

/// Height of the dev demo tap on its wall — a natural cane-strike height.
pub const DEMO_TAP_H: f64 = 0.8;

/// The demo tap stays this far from its wall's ends, so the strike lands
/// on the wall's face and never on a corner shared with another wall.
pub const DEMO_TAP_MARGIN: f64 = 0.2;

/// Two segment coordinates within this are the same axis line.
pub const AXIS_EPS: f32 = 0.001;

/// The box a wall segment occupies: the centerline padded by a wall
/// half-thickness on every side, floor to ceiling — so two walls whose
/// centerlines meet share a clean corner instead of leaving a gap.
#[must_use]
pub fn wall_box(length: f64) -> Vector3 {
    Vector3::new(
        (length + WALL_T * 2.0) as f32,
        WALL_H as f32,
        (WALL_T * 2.0) as f32,
    )
}

/// The nearest quarter turn to a free-hand yaw: 0 faces +X down the local
/// length axis, 1..3 step counterclockwise by 90°. Total on any input,
/// NaN included (NaN rounds to quadrant 0 rather than poisoning a cast).
#[must_use]
pub fn yaw_quadrant(yaw: f64) -> u8 {
    let steps = yaw / FRAC_PI_2;
    if !steps.is_finite() {
        return 0;
    }
    (steps.round() as i64).rem_euclid(4) as u8
}

/// The exact basis of a quarter turn about Y: unit columns of 0 and ±1,
/// bit-for-bit — the one orientation family under which a rotated box's
/// world vertices are exact coordinate swaps, never trig approximations.
#[must_use]
pub fn quadrant_basis(quadrant: u8) -> Basis {
    let (x, z) = match quadrant % 4 {
        0 => (Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 1.0)),
        1 => (Vector3::new(0.0, 0.0, -1.0), Vector3::new(1.0, 0.0, 0.0)),
        2 => (Vector3::new(-1.0, 0.0, 0.0), Vector3::new(0.0, 0.0, -1.0)),
        _ => (Vector3::new(0.0, 0.0, 1.0), Vector3::new(-1.0, 0.0, 0.0)),
    };
    Basis::from_cols(x, Vector3::new(0.0, 1.0, 0.0), z)
}

/// A wall node's centerline as the classic segment quad (x1, z1, x2, z2):
/// the node's floor position swept half the length each way along its
/// snapped axis. Even quadrants run along world X, odd along world Z.
#[must_use]
pub fn wall_segment(center: Vector3, length: f64, quadrant: u8) -> Vector4 {
    let half = (length * 0.5) as f32;
    if quadrant.is_multiple_of(2) {
        Vector4::new(center.x - half, center.z, center.x + half, center.z)
    } else {
        Vector4::new(center.x, center.z - half, center.x, center.z + half)
    }
}

/// The rectangle of walls immediately around a point — the hum room of a
/// sound source standing at (x, z): the nearest wall centerline in each
/// cardinal direction whose span actually covers the point (give or take
/// a wall half-thickness, so a source flush against a corner still reads
/// the room). Returns (x_min, z_min, x_max, z_max), the `u_hum_room`
/// layout; `None` when any side is open — an unenclosed source has no
/// room for the shader to clip by.
#[must_use]
pub fn room_around(walls: &[Vector4], x: f32, z: f32) -> Option<Vector4> {
    let reach = WALL_T as f32;
    let covers = |a: f32, b: f32, c: f32| a.min(b) - reach <= c && c <= a.max(b) + reach;
    let mut west = f32::NEG_INFINITY;
    let mut east = f32::INFINITY;
    let mut north = f32::NEG_INFINITY;
    let mut south = f32::INFINITY;
    for s in walls {
        if (s.x - s.z).abs() < AXIS_EPS && covers(s.y, s.w, z) {
            if s.x < x {
                west = west.max(s.x);
            } else if s.x > x {
                east = east.min(s.x);
            }
        }
        if (s.y - s.w).abs() < AXIS_EPS && covers(s.x, s.z, x) {
            if s.y < z {
                north = north.max(s.y);
            } else if s.y > z {
                south = south.min(s.y);
            }
        }
    }
    let closed = west.is_finite() && east.is_finite() && north.is_finite() && south.is_finite();
    closed.then(|| Vector4::new(west, north, east, south))
}

/// The dev demo tap, planned: where the input-less demo strikes, and the
/// struck face's outward normal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TapPlan {
    /// The strike point on the wall.
    pub point: Vector3,
    /// The struck face's normal, toward the spawn side.
    pub normal: Vector3,
}

/// Plan the demo tap on the room's west wall — the wall between the fan's
/// room and the validated spawn, so movie-maker runs and the deployed
/// `?demo` build always catch a wave from the hero's side of it. The tap
/// lands at the spawn's z clamped into the wall's span (its middle when
/// the wall is shorter than two margins), [`DEMO_TAP_H`] up, facing
/// whichever side the spawn stands on. `None` when no wall centerline
/// lies on the room's west edge — no room, no demo wall.
#[must_use]
pub fn demo_tap(walls: &[Vector4], room: Vector4, spawn: Vector3) -> Option<TapPlan> {
    // the west wall nearest the spawn's z — collinear walls beyond the
    // room's own edge (a corridor divider further down the same axis
    // line) are not this room's wall and never take the tap; slice order
    // breaks exact ties, so the same scene always plans the same tap
    let mut best: Option<(f32, f32, f32)> = None;
    for s in walls {
        if (s.x - s.z).abs() >= AXIS_EPS || (s.x - room.x).abs() >= AXIS_EPS {
            continue;
        }
        let (lo, hi) = (s.y.min(s.w), s.y.max(s.w));
        if lo > room.w + AXIS_EPS || hi < room.y - AXIS_EPS {
            continue; // no overlap with the room's west edge
        }
        let miss = (lo - spawn.z).max(spawn.z - hi).max(0.0);
        if best.is_none_or(|(d, _, _)| miss < d) {
            best = Some((miss, lo, hi));
        }
    }
    let (_, lo, hi) = best?;
    let margin = DEMO_TAP_MARGIN as f32;
    let z = if lo + margin > hi - margin {
        (lo + hi) * 0.5
    } else {
        spawn.z.clamp(lo + margin, hi - margin)
    };
    let normal = if spawn.x < room.x {
        Vector3::new(-1.0, 0.0, 0.0)
    } else {
        Vector3::new(1.0, 0.0, 0.0)
    };
    Some(TapPlan {
        point: Vector3::new(room.x, DEMO_TAP_H as f32, z),
        normal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shipped map's wall centerlines — the validated design the
    /// level scene mirrors, held here as the derivation fixtures.
    fn shipped_walls() -> Vec<Vector4> {
        vec![
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
    }

    /// The box pads the centerline by a half-thickness each way — the
    /// retired map builder's numbers, bit for bit.
    #[test]
    fn wall_box_pads_the_centerline() {
        assert_eq!(wall_box(7.4), Vector3::new(7.7, 3.0, 0.3));
        assert_eq!(wall_box(18.8), Vector3::new(19.1, 3.0, 0.3));
    }

    /// Free-hand yaws round to the nearest quarter turn, whole windings
    /// and negative angles folded into 0..4.
    #[test]
    fn yaw_rounds_to_the_nearest_quarter_turn() {
        assert_eq!(yaw_quadrant(0.0), 0);
        assert_eq!(yaw_quadrant(0.2), 0);
        assert_eq!(yaw_quadrant(1.2), 1);
        assert_eq!(yaw_quadrant(3.0), 2);
        assert_eq!(yaw_quadrant(-1.2), 3);
        assert_eq!(yaw_quadrant(-3.3), 2);
        assert_eq!(yaw_quadrant(7.0), 0);
        assert_eq!(yaw_quadrant(f64::NAN), 0);
    }

    /// Every quadrant basis is exact: unit columns of 0 and ±1, so a
    /// snapped wall's world vertices are coordinate swaps, not trig.
    #[test]
    fn quadrant_bases_are_exact() {
        let up = Vector3::new(0.0, 1.0, 0.0);
        for (k, x, z) in [
            (0, Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, 0.0, 1.0)),
            (1, Vector3::new(0.0, 0.0, -1.0), Vector3::new(1.0, 0.0, 0.0)),
            (
                2,
                Vector3::new(-1.0, 0.0, 0.0),
                Vector3::new(0.0, 0.0, -1.0),
            ),
            (3, Vector3::new(0.0, 0.0, 1.0), Vector3::new(-1.0, 0.0, 0.0)),
        ] {
            let basis = quadrant_basis(k);
            assert_eq!(basis.col_a(), x, "quadrant {k} x column");
            assert_eq!(basis.col_b(), up, "quadrant {k} y column");
            assert_eq!(basis.col_c(), z, "quadrant {k} z column");
        }
    }

    /// A wall's centerline runs along its snapped axis: even quadrants
    /// along world X, odd along world Z, half the length each way.
    #[test]
    fn wall_segment_runs_along_the_snapped_axis() {
        let along_x = wall_segment(Vector3::new(10.0, 0.0, 0.6), 18.8, 0);
        assert!((along_x.x - 0.6).abs() < 1e-4);
        assert!((along_x.y - 0.6).abs() < 1e-6);
        assert!((along_x.z - 19.4).abs() < 1e-4);
        assert!((along_x.w - 0.6).abs() < 1e-6);
        let along_z = wall_segment(Vector3::new(6.4, 0.0, 4.3), 7.4, 3);
        assert!((along_z.x - 6.4).abs() < 1e-6);
        assert!((along_z.y - 0.6).abs() < 1e-4);
        assert!((along_z.z - 6.4).abs() < 1e-6);
        assert!((along_z.w - 8.0).abs() < 1e-4);
    }

    /// The shipped fan's room derives to the validated hum-room rect —
    /// the exact numbers the retired LevelData carried by hand.
    #[test]
    fn shipped_fan_room_derives_to_the_validated_rect() {
        let room = room_around(&shipped_walls(), 8.6, 4.4);
        assert_eq!(room, Some(Vector4::new(6.4, 0.6, 19.4, 8.0)));
    }

    /// An unenclosed source has no room: with the east border gone the
    /// fan's east side is open air and derivation refuses.
    #[test]
    fn open_side_yields_no_room() {
        let mut walls = shipped_walls();
        walls.remove(1);
        assert_eq!(room_around(&walls, 8.6, 4.4), None);
    }

    /// A source flush against a wall's end is still enclosed: spans reach
    /// a wall half-thickness past their centerline ends.
    #[test]
    fn span_ends_reach_a_half_thickness() {
        let walls = vec![
            Vector4::new(0.0, 0.0, 4.0, 0.0),
            Vector4::new(4.0, 0.0, 4.0, 4.0),
            Vector4::new(4.0, 4.0, 0.0, 4.0),
            Vector4::new(0.0, 4.0, 0.0, 0.0),
        ];
        // 0.1 past the south wall's x-span end, within the 0.15 reach
        assert!(room_around(&walls, 4.1, 2.0).is_none()); // outside the room
        assert_eq!(
            room_around(&walls, 3.9, 3.9),
            Some(Vector4::new(0.0, 0.0, 4.0, 4.0))
        );
    }

    /// The shipped tap plan: the west hum wall at the spawn's z, 0.8 up,
    /// striking toward the spawn side — the validated numbers, bit for
    /// bit.
    #[test]
    fn shipped_demo_tap_derives_to_the_validated_point() {
        let walls = shipped_walls();
        let room = room_around(&walls, 8.6, 4.4).expect("shipped room derives");
        let plan = demo_tap(&walls, room, Vector3::new(3.0, 0.9, 4.0)).expect("tap derives");
        assert_eq!(plan.point, Vector3::new(6.4, 0.8, 4.0));
        assert_eq!(plan.normal, Vector3::new(-1.0, 0.0, 0.0));
    }

    /// A spawn beyond the wall's end clamps onto the wall, one margin
    /// short of the corner.
    #[test]
    fn demo_tap_clamps_into_the_wall_span() {
        let walls = shipped_walls();
        let room = room_around(&walls, 8.6, 4.4).expect("shipped room derives");
        let plan = demo_tap(&walls, room, Vector3::new(3.0, 0.9, 20.0)).expect("tap derives");
        assert_eq!(plan.point, Vector3::new(6.4, 0.8, 7.8));
    }

    /// A spawn inside the room taps the same wall from its own side: the
    /// normal flips to face it.
    #[test]
    fn demo_tap_faces_a_spawn_inside_the_room() {
        let walls = shipped_walls();
        let room = room_around(&walls, 8.6, 4.4).expect("shipped room derives");
        let plan = demo_tap(&walls, room, Vector3::new(8.0, 0.9, 4.0)).expect("tap derives");
        assert_eq!(plan.normal, Vector3::new(1.0, 0.0, 0.0));
    }

    /// A wall shorter than two margins takes the tap in its middle
    /// instead of refusing or panicking on a crossed clamp.
    #[test]
    fn a_stub_wall_takes_the_tap_in_its_middle() {
        let walls = vec![
            Vector4::new(2.0, 1.0, 2.0, 1.3),
            Vector4::new(2.0, 1.0, 4.0, 1.0),
            Vector4::new(4.0, 1.0, 4.0, 1.3),
            Vector4::new(2.0, 1.3, 4.0, 1.3),
        ];
        let room = Vector4::new(2.0, 1.0, 4.0, 1.3);
        let plan = demo_tap(&walls, room, Vector3::new(0.0, 0.9, 9.0)).expect("tap derives");
        assert!((plan.point.z - 1.15).abs() < 1e-6);
    }

    /// No wall on the room's west edge: no tap plan at all.
    #[test]
    fn no_west_wall_no_tap() {
        let walls = vec![Vector4::new(0.0, 0.0, 4.0, 0.0)];
        let room = Vector4::new(1.0, 0.0, 3.0, 4.0);
        assert_eq!(demo_tap(&walls, room, Vector3::ZERO), None);
    }
}
