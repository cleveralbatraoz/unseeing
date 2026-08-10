//! The level's technical plan, derived — how an editor-authored scene of
//! dragged-around wall nodes becomes the exact contracts the engine runs
//! on: box dimensions, axis-snapped orientations, wall centerlines, and
//! the dev demo tap. A designer only places and rotates nodes; every
//! number the systems need is computed here, so the geometry and the
//! contracts derived from it can never drift apart in two files.
//!
//! Precision law, pinned from the retired GDScript map builder: GDScript
//! floats are f64, so every scalar knob here is f64 and arithmetic
//! narrows to the engine's f32 lanes exactly where the original assigned
//! into a Vector — no earlier. The scene work (meshes, colliders, child
//! walks) stays in the engine layer ([`crate::nodes`]); this module owns
//! only the math the cargo tests pin.
//!
//! Axis law: walls are axis-aligned boxes — the occluder rects the sight
//! shaders count and the centerline table the suites hold invariants
//! against both depend on it. A designer's free-hand rotation is therefore
//! snapped to the nearest quarter turn, and the snapped basis is built
//! from exact unit columns: no trig dust for the rasterizer to chew on.

use std::f64::consts::FRAC_PI_2;

use godot::builtin::{Basis, Vector3, Vector4};

/// Wall height in meters — walls run floor to ceiling.
pub const WALL_H: f64 = 3.0;

/// How much a constant source's hum WAVES survive crossing one wall — the
/// CPU half of the muffle vocabulary the shaders speak as `HUM_THROUGH` in
/// pulse_pool.gdshaderinc (the shells in the air, the surfaces they wash).
/// Kept in step by [`crate::sight`] tests and the shader contract.
pub const HUM_THROUGH: f64 = 0.55;

/// How much a source's own SILHOUETTE survives crossing one wall — dimmer
/// than its waves, so a source felt through a wall is a faint ghost of
/// itself, fainter still through two. This attenuates the source skin's
/// standing floor per wall between the eye and the source (see
/// [`crate::nodes`]' `source_muffle`); a wall dims the shape, never erases
/// it — the source is always felt, just muted.
pub const SOURCE_THROUGH: f64 = 0.3;

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
///
/// A length is a MAGNITUDE: a minus sign is folded away here rather than
/// carried into the engine, where a negative extent means two different
/// things to the two halves of one wall — a mesh draws it, a collider
/// refuses it and keeps whatever size it had.
#[must_use]
pub fn wall_box(length: f64) -> Vector3 {
    Vector3::new(
        (length.abs() + WALL_T * 2.0) as f32,
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

/// The nearest quarter turn to whatever yaw a BASIS carries, read off its
/// X column — the axis a wall's length runs down, and the only column that
/// decides the answer. A scale stretches that column without turning an
/// axis-aligned one, so a wall inheriting a scaled room still reads the
/// axis it actually draws along. Total on any basis, a degenerate (zero)
/// column included: that reads as quadrant 0 rather than poisoning the
/// arithmetic.
#[must_use]
pub fn basis_quadrant(basis: Basis) -> u8 {
    // a yaw of θ puts the X column at (cos θ, 0, −sin θ)
    let x = basis.col_a();
    yaw_quadrant(f64::from(-x.z).atan2(f64::from(x.x)))
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
///
/// The sweep takes the length's MAGNITUDE, so the ends always come back in
/// order. A negative half-sweep would hand the level a centerline running
/// backwards, and the systems that read it disagree about that: the
/// occluder rect normalises the ends, the tap planner normalises them, and
/// anything new that trusts the quad's declared order would not.
#[must_use]
pub fn wall_segment(center: Vector3, length: f64, quadrant: u8) -> Vector4 {
    let half = (length.abs() * 0.5) as f32;
    if quadrant.is_multiple_of(2) {
        Vector4::new(center.x - half, center.z, center.x + half, center.z)
    } else {
        Vector4::new(center.x, center.z - half, center.x, center.z + half)
    }
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

/// Parameter `t` in [0, 1] at which the segment `a -> b` (read in the XZ
/// plane) crosses the axis-aligned wall centerline `wall`, or `None` when
/// it does not cross within both spans. Total on axis-parallel inputs: a
/// segment running parallel to the wall never crosses it.
#[must_use]
fn crossing_param(a: Vector3, b: Vector3, wall: Vector4) -> Option<f32> {
    if (wall.y - wall.w).abs() < AXIS_EPS {
        // wall runs along X at z = wall.y
        let dz = b.z - a.z;
        if dz.abs() < AXIS_EPS {
            return None;
        }
        let t = (wall.y - a.z) / dz;
        if !(0.0..=1.0).contains(&t) {
            return None;
        }
        let x = a.x + t * (b.x - a.x);
        (wall.x.min(wall.z)..=wall.x.max(wall.z))
            .contains(&x)
            .then_some(t)
    } else {
        // wall runs along Z at x = wall.x
        let dx = b.x - a.x;
        if dx.abs() < AXIS_EPS {
            return None;
        }
        let t = (wall.x - a.x) / dx;
        if !(0.0..=1.0).contains(&t) {
            return None;
        }
        let z = a.z + t * (b.z - a.z);
        (wall.y.min(wall.w)..=wall.y.max(wall.w))
            .contains(&z)
            .then_some(t)
    }
}

/// Clamp `v` into the span `[lo, hi]` shrunk by `margin` at each end, or
/// the span's midpoint when it is shorter than two margins — a strike
/// near a corner slides one margin clear of it.
#[must_use]
fn clamp_span(v: f32, lo: f32, hi: f32, margin: f32) -> f32 {
    if lo + margin > hi - margin {
        (lo + hi) * 0.5
    } else {
        v.clamp(lo + margin, hi - margin)
    }
}

/// Plan the demo tap from the walls alone — no room rectangle: the wall
/// between the hero and the fan is the nearest wall centerline the
/// spawn→fan line crosses, and the tap lands on that wall's spawn-facing
/// FACE, a half-thickness off the centerline. Born just outside the wall,
/// the strike lights the near side while the wall does not occlude its own
/// reveal — the source stands on the face, not inside the box. The point
/// is [`DEMO_TAP_H`] up, clamped into the wall's span one
/// [`DEMO_TAP_MARGIN`] short of each end (its midpoint on a stub wall).
/// `None` when the spawn→fan line crosses no wall — hero and fan share a
/// room, and there is nothing between them to demo-strike.
#[must_use]
pub fn demo_tap(walls: &[Vector4], spawn: Vector3, fan: Vector3) -> Option<TapPlan> {
    // slice order breaks exact ties, so the same scene always plans the
    // same tap
    let mut best: Option<(f32, Vector4)> = None;
    for &wall in walls {
        if let Some(t) = crossing_param(spawn, fan, wall)
            && best.is_none_or(|(bt, _)| t < bt)
        {
            best = Some((t, wall));
        }
    }
    let (_, wall) = best?;
    let margin = DEMO_TAP_MARGIN as f32;
    let half = WALL_T as f32;
    let h = DEMO_TAP_H as f32;
    if (wall.y - wall.w).abs() < AXIS_EPS {
        // X-run wall at z = wall.y: the tap slides along X and faces ±Z
        let x = clamp_span(spawn.x, wall.x.min(wall.z), wall.x.max(wall.z), margin);
        let toward = if spawn.z < wall.y { -1.0 } else { 1.0 };
        Some(TapPlan {
            point: Vector3::new(x, h, wall.y + toward * half),
            normal: Vector3::new(0.0, 0.0, toward),
        })
    } else {
        // Z-run wall at x = wall.x: the tap slides along Z and faces ±X
        let z = clamp_span(spawn.z, wall.y.min(wall.w), wall.y.max(wall.w), margin);
        let toward = if spawn.x < wall.x { -1.0 } else { 1.0 };
        Some(TapPlan {
            point: Vector3::new(wall.x + toward * half, h, z),
            normal: Vector3::new(toward, 0.0, 0.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use godot::builtin::EulerOrder;

    use super::*;

    /// A RETIRED 20×20/10-wall map — not the shipped 28×28/19-wall scene in
    /// `game/scenes/level_01.tscn`. Kept as the derivation fixture for the
    /// tap-plan tests below, which only ever touch DividerNorth and the
    /// spawn corridor, byte-identical between the two maps.
    fn retired_map_walls() -> Vec<Vector4> {
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

    /// A designer's minus sign is a typo, not a wall pointing backwards.
    /// Left raw it is three different bugs at once: a padded extent that
    /// `BoxShape3D` refuses (leaving the collider at its default cube while
    /// the mesh draws the wall), a shorter box than the number asked for
    /// (`-4 + 0.3`), and a centerline swept backwards. All three are
    /// answered here, where the arithmetic lives, so no caller can reach
    /// the engine with one of them.
    #[test]
    fn a_negative_length_is_the_same_wall_as_its_magnitude() {
        assert_eq!(wall_box(-7.4), wall_box(7.4));
        assert_eq!(wall_box(-0.1), wall_box(0.1));
        let center = Vector3::new(4.0, 0.0, 9.0);
        for quadrant in 0..4 {
            let back = wall_segment(center, -4.0, quadrant);
            assert_eq!(back, wall_segment(center, 4.0, quadrant));
            assert!(back.x <= back.z && back.y <= back.w, "ends out of order");
        }
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

    /// A basis reports the quarter turn it is nearest to, whatever it
    /// carries besides the turn: every quadrant basis reads back as
    /// itself, a free-hand yaw rounds, a TILT does not disturb the reading
    /// (only the X column's ground shadow decides), and — the case a room
    /// prefab makes routine — an inherited SCALE stretches the columns
    /// without turning them, so the answer is unchanged.
    #[test]
    fn a_basis_reports_the_quarter_turn_it_is_nearest_to() {
        for k in 0..4u8 {
            assert_eq!(
                basis_quadrant(quadrant_basis(k)),
                k,
                "quadrant {k} round trip"
            );
        }
        let yawed =
            |yaw: f64| Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, yaw as f32, 0.0));
        assert_eq!(basis_quadrant(yawed(0.2)), 0);
        assert_eq!(basis_quadrant(yawed(1.2)), 1);
        assert_eq!(basis_quadrant(yawed(-1.2)), 3);
        // tilted and rolled off every axis, the way a dragged gizmo leaves it
        let tilted = Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.2, 1.2, -0.1));
        assert_eq!(basis_quadrant(tilted), 1);
        // a 2x room: same turn, twice the columns
        assert_eq!(
            basis_quadrant(quadrant_basis(1).scaled(Vector3::splat(2.0))),
            1
        );
        assert_eq!(
            basis_quadrant(quadrant_basis(3).scaled(Vector3::new(0.5, 3.0, 2.0))),
            3,
        );
        // and a collapsed basis answers rather than diverging
        assert_eq!(basis_quadrant(quadrant_basis(2).scaled(Vector3::ZERO)), 0);
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

    /// The shipped tap plan, roomless: the spawn→fan line crosses
    /// DividerNorth, so the tap lands on that wall's west FACE (a
    /// half-thickness off the x = 6.4 centerline) at the spawn's z, 0.8
    /// up, striking toward the spawn.
    #[test]
    fn shipped_demo_tap_derives_to_the_validated_point() {
        let plan = demo_tap(
            &retired_map_walls(),
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 0.0, 4.4),
        )
        .expect("tap derives");
        assert_eq!(plan.point, Vector3::new(6.25, 0.8, 4.0));
        assert_eq!(plan.normal, Vector3::new(-1.0, 0.0, 0.0));
    }

    /// The face nudge earns its keep: the tap stands OUTSIDE the divider's
    /// occluder box, so the wall it strikes does not shadow its own
    /// strike — sight crosses zero walls to a point on the tap's own
    /// (west) side — while that same wall still blocks the far, fan-room
    /// side. This is exactly what keeps a wall-struck tap lighting its
    /// near face without leaking through.
    #[test]
    fn demo_tap_face_is_not_self_occluded() {
        let plan = demo_tap(
            &retired_map_walls(),
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 0.0, 4.4),
        )
        .expect("tap derives");
        let rects: Vec<Vector4> = retired_map_walls()
            .iter()
            .map(|s| crate::sight::wall_rect(*s))
            .collect();
        let top = WALL_H as f32;
        assert_eq!(
            crate::sight::crossings_from(plan.point, Vector3::new(4.0, 0.8, 4.0), &rects, top),
            0,
        );
        assert_eq!(
            crate::sight::crossings_from(plan.point, Vector3::new(8.0, 0.8, 4.0), &rects, top),
            1,
        );
    }

    /// A spawn whose z runs past the wall span clamps onto the wall, one
    /// margin short of the corner — the line still crossing it.
    #[test]
    fn demo_tap_clamps_into_the_wall_span() {
        let plan = demo_tap(
            &retired_map_walls(),
            Vector3::new(3.0, 0.9, 9.0),
            Vector3::new(8.6, 0.0, 4.4),
        )
        .expect("tap derives");
        assert_eq!(plan.point, Vector3::new(6.25, 0.8, 7.8));
    }

    /// Hero and fan in the same room: the line between them crosses no
    /// wall, so there is nothing between them to demo-strike.
    #[test]
    fn same_room_spawn_and_fan_have_no_tap() {
        assert_eq!(
            demo_tap(
                &retired_map_walls(),
                Vector3::new(10.0, 0.9, 4.0),
                Vector3::new(8.6, 0.0, 4.4),
            ),
            None,
        );
    }

    /// A wall shorter than two margins takes the tap at its midpoint
    /// instead of panicking on a crossed clamp, still on its face.
    #[test]
    fn a_stub_wall_takes_the_tap_at_its_midpoint() {
        let walls = vec![Vector4::new(2.0, 1.0, 2.0, 1.3)]; // z-run stub, span 0.3
        let plan = demo_tap(
            &walls,
            Vector3::new(0.0, 0.9, 1.15),
            Vector3::new(4.0, 0.0, 1.15),
        )
        .expect("tap derives");
        assert!((plan.point.z - 1.15).abs() < 1e-6);
        assert_eq!(plan.point.x, 1.85); // 2.0 centerline − 0.15 to the west face
    }

    /// A line parallel to the only wall crosses nothing: no tap.
    #[test]
    fn no_crossing_no_tap() {
        let walls = vec![Vector4::new(0.0, 0.0, 4.0, 0.0)]; // x-run at z = 0
        assert_eq!(
            demo_tap(
                &walls,
                Vector3::new(1.0, 0.9, 3.0),
                Vector3::new(3.0, 0.0, 3.0),
            ),
            None,
        );
    }
}
