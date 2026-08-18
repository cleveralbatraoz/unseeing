//! Occlusion, wall by wall — the oracle.
//!
//! `crate::sight` is the cargo-pinned reference that the GLSL in
//! `pulse_pool.gdshaderinc` transliterates. Exposing it as an answerable
//! question turns "the picture looks wrong" into "the Rust says one
//! crossing and the shader drew none", which localises the bug to the
//! shader without a single pixel being inspected.

use godot::builtin::{Vector2, Vector3, Vector4};

use crate::sight::{
    Occluder, blocked_from, contains, crosses, crossings, crossings_from, reveal_visibility,
};

/// One wall's answer for one sight line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallVerdict {
    pub index: usize,
    pub rect: Vector4,
    /// This wall's own world Y sweep, `(bottom, top)`. Reported per wall
    /// because there is no longer any single answer: an occluder carries
    /// the span of the box it stands for, so a lifted wall and a
    /// floor-standing one differ here and a reader needs to see which.
    pub span: Vector2,
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
    /// Every wall considered, in table order — including the ones that
    /// refused. An empty verdict list and a clear line are different facts.
    pub walls: Vec<WallVerdict>,
    /// Eye to lit point: every wall counts.
    pub camera_crossings: u32,
    /// Source to lit point: the birth wall is skipped.
    pub source_crossings: u32,
    /// Eye to lit point, PROPS only — the solids `spans_the_corridor`
    /// refused, which stop no wave but each take [`level_plan::prop_through`]
    /// from a source's standing image. A source can read muffled with zero
    /// walls crossed, and this is the only thing that explains it.
    pub prop_crossings: u32,
    /// How much of the source's WAVE survives — the shader's
    /// `source_reveal_vis`, keyed to the SOURCE occluder so a sound born
    /// flush on a wall still lights its own face. A gate, not a fade: a
    /// wall stops a wave whatever kind made it.
    pub wave_transmission: f64,
    /// How much of a source's SILHOUETTE survives everything between it and
    /// the eye — composed by [`level_plan::source_muffle`] itself, not by a
    /// restatement of it, so the oracle cannot drift from the engine again.
    /// `SOURCE_THROUGH` per wall and [`level_plan::prop_through`] per prop,
    /// both keyed to the CAMERA occluder.
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
pub fn explain_ray(
    from: Vector3,
    to: Vector3,
    occluders: &[Occluder],
    props: &[Occluder],
) -> RayExplanation {
    let walls = occluders
        .iter()
        .enumerate()
        .map(|(index, occ)| WallVerdict {
            index,
            rect: occ.rect(),
            span: occ.span(),
            crossed: crosses(from, to, *occ),
            contains_origin: contains(*occ, from),
        })
        .collect();
    let camera_crossings = crossings(from, to, occluders);
    let prop_crossings = crossings(from, to, props);
    let source_crossings = crossings_from(from, to, occluders);
    RayExplanation {
        from,
        to,
        walls,
        camera_crossings,
        source_crossings,
        prop_crossings,
        // The oracle asks the SAME predicate the shipped shader asks, not
        // an equivalent restatement of it: `source_reveal_vis` in
        // data_core.gdshaderinc is `wall_blocked_from(src, world) ? 0.0 :
        // 1.0`, and this is its Rust twin composed the same way round.
        // `source_crossings` above still reports the count, because a
        // reader debugging a sight line wants to know how many walls stand
        // there even though the law stops caring after the first.
        wave_transmission: reveal_visibility(blocked_from(from, to, occluders)),
        // The engine's own function, called — not `SOURCE_THROUGH` raised
        // by hand to the wall count, which is what this was and which made
        // the oracle wrong on every sight line through a crate. Props stop
        // no wave, so they are absent from `wave_transmission` above; they
        // dim a standing image, so they belong here.
        source_transmission: crate::level_plan::source_muffle(camera_crossings, prop_crossings),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_plan;
    use godot::builtin::{Vector3, Vector4};

    const WALL_TOP: f64 = level_plan::WALL_H;

    /// THE BREAK: the oracle and the engine composing a source's muffle by
    /// different laws. This module's whole purpose is that "a disagreement
    /// between the two would surface as a failing test here rather than as a
    /// plausible-looking wrong answer in the field", and
    /// `source_transmission` is documented as "the engine's `source_muffle`".
    /// The engine's is `level_plan::source_muffle(walls, props)`; the oracle
    /// raised SOURCE_THROUGH to the wall count alone, so on any sight line
    /// through a crate it reported a number no shader was holding — and
    /// `WaveObserver.snapshot()` showed both, side by side, disagreeing.
    ///
    /// Hand-derived: one wall and two props is `0.30 * sqrt(0.30)^2` = 0.09,
    /// exactly two walls' worth, which is `prop_through`'s "two props cost
    /// one wall" stated as a number.
    #[test]
    fn the_oracle_muffles_by_the_law_the_engine_applies() {
        let wall = Occluder::new(Vector4::new(3.0, -5.0, 3.0, 5.0), 0.0, WALL_TOP).unwrap();
        let crate_a = Occluder::from_bounds(4.9, -0.5, 5.1, 0.5, 0.0, 0.8).unwrap();
        let crate_b = Occluder::from_bounds(5.9, -0.5, 6.1, 0.5, 0.0, 0.8).unwrap();
        let eye = Vector3::new(0.0, 0.4, 0.0);
        let src = Vector3::new(9.0, 0.4, 0.0);

        let e = explain_ray(eye, src, &[wall], &[crate_a, crate_b]);
        assert_eq!(e.camera_crossings, 1, "the wall");
        assert_eq!(e.prop_crossings, 2, "both crates");
        assert!(
            (e.source_transmission - 0.09).abs() < 1.0e-9,
            "one wall and two crates should leave 0.09, not {}",
            e.source_transmission
        );

        // and the law itself, so the oracle cannot be "repaired" by copying
        // the arithmetic instead of calling the function the engine calls
        assert_eq!(
            e.source_transmission,
            level_plan::source_muffle(e.camera_crossings, e.prop_crossings)
        );

        // the defect, stated: walls alone would have answered 0.30
        assert!(
            (e.source_transmission - level_plan::SOURCE_THROUGH).abs() > 1.0e-3,
            "the props were ignored again"
        );
    }

    /// A RETIRED 20×20/10-wall map, not the shipped 28×28/19-wall scene —
    /// see `sight::tests::retired_map_rects` for why it remains a valid
    /// derivation fixture for these particular lines despite that. Kept
    /// deliberately as a duplicate rather than a shared import: a shared
    /// fixture would let one edit move both sides of the oracle at once,
    /// and this contract exists to catch exactly that drift between
    /// `sight.rs` and `observe`.
    fn retired_map_rects() -> Vec<Occluder> {
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
        .map(|s| Occluder::new(*s, 0.0, WALL_TOP).expect("a floor-standing wall is describable"))
        .collect()
    }

    /// Spawn to fan head crosses exactly one wall — and the explanation
    /// must NAME it, not merely count it. Wall index 4 is DividerNorth.
    /// Transmission is then 0.0 for the wave (a wall is a gate, not an
    /// attenuator) and 0.30^1 for the silhouette, hand-derived from the
    /// constant in level_plan.
    #[test]
    fn one_wall_is_named_and_its_transmission_derived() {
        let e = explain_ray(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 1.15, 4.4),
            &retired_map_rects(),
            &[],
        );
        assert_eq!(e.camera_crossings, 1);
        let crossed: Vec<usize> = e
            .walls
            .iter()
            .filter(|w| w.crossed)
            .map(|w| w.index)
            .collect();
        assert_eq!(crossed, vec![4]);
        // One wall stands between source and lit point, so the wave is
        // extinguished — 0.0, not a surviving fraction. The silhouette is
        // a DIFFERENT law and still dims to 0.30; asserting both here is
        // what keeps a future edit from collapsing the two.
        assert!((e.wave_transmission - 0.0).abs() < 1e-9);
        assert!((e.source_transmission - 0.30).abs() < 1e-9);
    }

    /// Composition is now the SILHOUETTE's law alone: 0.30^2 = 0.09,
    /// hand-derived. The wave's answer is a gate, not an exponent — two
    /// walls extinguish exactly as one does, so it stays 0.0.
    #[test]
    fn two_walls_compose_their_transmission() {
        let e = explain_ray(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 10.0),
            &retired_map_rects(),
            &[],
        );
        assert_eq!(e.camera_crossings, 2);
        assert!(
            (e.wave_transmission - 0.0).abs() < 1e-9,
            "got {}",
            e.wave_transmission
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
            &[],
        );
        assert_eq!(e.camera_crossings, 0);
        assert_eq!(e.walls.len(), 10);
        assert!(e.walls.iter().all(|w| !w.crossed));
        assert!((e.wave_transmission - 1.0).abs() < 1e-9);
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
            &[],
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
    /// (`source_crossings`) sees zero walls, so `wave_transmission` — the
    /// gate `source_reveal_vis` in the shader actually applies — is full
    /// at 1.0; the CAMERA occluder still sees the one wall it exits, so
    /// `source_transmission` — the exponent `source_muffle` applies — is
    /// dimmed to 0.30 (`0.30^1`). A version that exponentiates both by
    /// `camera_crossings` would report 0.0 for `wave_transmission` here
    /// instead of 1.0, and no other test in this module can tell the two
    /// occluders apart.
    #[test]
    fn the_two_transmissions_use_their_own_occluder() {
        let e = explain_ray(
            Vector3::new(6.4, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 4.0),
            &retired_map_rects(),
            &[],
        );
        assert!(
            (e.wave_transmission - 1.0).abs() < 1e-9,
            "got {}",
            e.wave_transmission
        );
        assert!(
            (e.source_transmission - 0.30).abs() < 1e-9,
            "got {}",
            e.source_transmission
        );
    }
}
