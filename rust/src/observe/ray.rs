//! Occlusion, wall by wall — the oracle.
//!
//! `crate::sight` is the cargo-pinned reference that the GLSL in
//! `pulse_pool.gdshaderinc` transliterates. Exposing it as an answerable
//! question turns "the picture looks wrong" into "the Rust says one
//! crossing and the shader drew none", which localises the bug to the
//! shader without a single pixel being inspected.

use godot::builtin::{Vector3, Vector4};

use crate::level_plan::{HUM_THROUGH, SOURCE_THROUGH};
use crate::sight::{contains, crosses, crossings, crossings_from};

/// One wall's answer for one sight line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallVerdict {
    pub index: usize,
    pub rect: Vector4,
    /// Does the segment pierce this wall's box?
    pub crossed: bool,
    /// Does this wall contain the origin? Such a wall is skipped by the
    /// SOURCE occluder — a sound born flush on a wall lights its own face.
    pub contains_origin: bool,
}

/// Everything the occlusion tests say about one sight line.
#[derive(Debug, Clone, PartialEq)]
pub struct RayExplanation {
    pub from: Vector3,
    pub to: Vector3,
    pub wall_top: f32,
    /// Every wall considered, in table order — including the ones that
    /// refused. An empty verdict list and a clear line are different facts.
    pub walls: Vec<WallVerdict>,
    /// Eye to lit point: every wall counts.
    pub camera_crossings: u32,
    /// Source to lit point: the birth wall is skipped.
    pub source_crossings: u32,
    /// `HUM_THROUGH ^ source_crossings` — how much of a source's WAVE
    /// survives (the shader's `source_reveal_vis`, keyed to the SOURCE
    /// occluder so a sound born flush on a wall still lights its own face).
    pub hum_transmission: f64,
    /// `SOURCE_THROUGH ^ camera_crossings` — how much of its SILHOUETTE
    /// survives (the engine's `source_muffle`, keyed to the CAMERA
    /// occluder — every wall between the eye and the source counts).
    pub source_transmission: f64,
}

/// Explain what the walls do to the sight line `from -> to`.
///
/// Total on any input, including a degenerate segment (`from == to` still
/// runs every wall test — a point can lie inside a wall's occluder box, so
/// it is not guaranteed to cross nothing). The counts come from `sight`'s
/// own functions rather than from the per-wall verdicts, so a disagreement
/// between the two would surface as a failing test here rather than as a
/// plausible-looking wrong answer in the field.
#[must_use]
pub fn explain_ray(from: Vector3, to: Vector3, rects: &[Vector4], wall_top: f32) -> RayExplanation {
    let walls = rects
        .iter()
        .enumerate()
        .map(|(index, rect)| WallVerdict {
            index,
            rect: *rect,
            crossed: crosses(from, to, *rect, wall_top),
            contains_origin: contains(*rect, from, wall_top),
        })
        .collect();
    let camera_crossings = crossings(from, to, rects, wall_top);
    let source_crossings = crossings_from(from, to, rects, wall_top);
    RayExplanation {
        from,
        to,
        wall_top,
        walls,
        camera_crossings,
        source_crossings,
        // HUM_THROUGH is the source_reveal_vis exponent base
        // (data_core.gdshaderinc), which reads off wall_crossings_from —
        // the SOURCE occluder that skips the wall a source is born inside.
        hum_transmission: HUM_THROUGH.powi(source_crossings as i32),
        // SOURCE_THROUGH is the source_muffle exponent base
        // (nodes/level.rs), which reads off sight::crossings — the CAMERA
        // occluder, every wall counted.
        source_transmission: SOURCE_THROUGH.powi(camera_crossings as i32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_plan;
    use crate::sight::wall_rect;
    use godot::builtin::{Vector3, Vector4};

    const WALL_TOP: f32 = level_plan::WALL_H as f32;

    /// A RETIRED 20×20/10-wall map, not the shipped 28×28/19-wall scene —
    /// see `sight::tests::retired_map_rects` for why it remains a valid
    /// derivation fixture for these particular lines despite that. Kept
    /// deliberately as a duplicate rather than a shared import: a shared
    /// fixture would let one edit move both sides of the oracle at once,
    /// and this contract exists to catch exactly that drift between
    /// `sight.rs` and `observe`.
    fn retired_map_rects() -> Vec<Vector4> {
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

    /// Spawn to fan head crosses exactly one wall — and the explanation
    /// must NAME it, not merely count it. Wall index 4 is DividerNorth.
    /// Transmission is then 0.55^1 for the wave and 0.30^1 for the
    /// silhouette, hand-derived from the constants in level_plan.
    #[test]
    fn one_wall_is_named_and_its_transmission_derived() {
        let e = explain_ray(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 1.15, 4.4),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert_eq!(e.camera_crossings, 1);
        let crossed: Vec<usize> = e
            .walls
            .iter()
            .filter(|w| w.crossed)
            .map(|w| w.index)
            .collect();
        assert_eq!(crossed, vec![4]);
        assert!((e.hum_transmission - 0.55).abs() < 1e-9);
        assert!((e.source_transmission - 0.30).abs() < 1e-9);
    }

    /// Two walls compose as k^2 — the composition law, hand-derived:
    /// 0.55^2 = 0.3025 and 0.30^2 = 0.09.
    #[test]
    fn two_walls_compose_their_transmission() {
        let e = explain_ray(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 10.0),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert_eq!(e.camera_crossings, 2);
        assert!(
            (e.hum_transmission - 0.3025).abs() < 1e-9,
            "got {}",
            e.hum_transmission
        );
        assert!(
            (e.source_transmission - 0.09).abs() < 1e-9,
            "got {}",
            e.source_transmission
        );
    }

    /// A clear line reports full transmission and every wall verdict false
    /// — not an empty list. An agent must be able to see that the walls
    /// were considered and refused.
    #[test]
    fn a_clear_line_still_reports_every_wall() {
        let e = explain_ray(
            Vector3::new(8.0, 1.0, 4.0),
            Vector3::new(12.0, 1.5, 6.0),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert_eq!(e.camera_crossings, 0);
        assert_eq!(e.walls.len(), 10);
        assert!(e.walls.iter().all(|w| !w.crossed));
        assert!((e.hum_transmission - 1.0).abs() < 1e-9);
    }

    /// The birth-wall asymmetry, made visible. A source standing on the
    /// divider centerline lighting an open point: the camera occluder
    /// counts the wall it exits, the source occluder skips the wall it was
    /// born in, and the explanation reports BOTH plus which wall contained
    /// the origin.
    #[test]
    fn the_birth_wall_asymmetry_is_reported_not_hidden() {
        let e = explain_ray(
            Vector3::new(6.4, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 4.0),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert_eq!(e.camera_crossings, 1);
        assert_eq!(e.source_crossings, 0);
        let held: Vec<usize> = e
            .walls
            .iter()
            .filter(|w| w.contains_origin)
            .map(|w| w.index)
            .collect();
        assert_eq!(held, vec![4]);
    }

    /// The two transmissions must be keyed to their own occluder, not both
    /// to the camera's. On the birth-wall geometry the SOURCE occluder
    /// (`source_crossings`) sees zero walls, so `hum_transmission` — the
    /// exponent `source_reveal_vis` in the shader actually applies — is
    /// full at 1.0 (`0.55^0`); the CAMERA occluder still sees the one wall
    /// it exits, so `source_transmission` — the exponent `source_muffle`
    /// applies — is dimmed to 0.30 (`0.30^1`). A version that exponentiates
    /// both by `camera_crossings` would report 0.55 for `hum_transmission`
    /// here instead of 1.0, and no other test in this module can tell the
    /// two exponent bases apart.
    #[test]
    fn the_two_transmissions_use_their_own_occluder() {
        let e = explain_ray(
            Vector3::new(6.4, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 4.0),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert!(
            (e.hum_transmission - 1.0).abs() < 1e-9,
            "got {}",
            e.hum_transmission
        );
        assert!(
            (e.source_transmission - 0.30).abs() < 1e-9,
            "got {}",
            e.source_transmission
        );
    }
}
