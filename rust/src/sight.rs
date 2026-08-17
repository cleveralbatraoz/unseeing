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
///
/// Raising it is nearly free and the map has outgrown the old 16: the GLSL
/// loops `break` at `u_wall_count`, so the only cost of an unused slot is
/// its 16 bytes in the material's uniform buffer (32 slots = 512 B, against
/// the 3.4 KB the pulse lanes already occupy and a 16 KB floor on the
/// smallest WebGL2 block). What is NOT free is a wall a level actually
/// holds: every one of them is another rect in the per-fragment sight loop,
/// which is why [`near`] exists.
pub const MAXW: usize = 32;

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

/// Could the segment `from -> to` possibly reach `rect` at all? An EXACT
/// pre-rejection, not a heuristic: a segment that crosses the rect has a
/// point inside it, so the segment's own XZ bounding box must overlap the
/// rect. `false` therefore implies [`crosses`] is false, with no false
/// negatives to hunt for later.
///
/// It earns its keep in the fragment shader, where the sight loop runs per
/// pixel per pulse over every wall in the level: four comparisons refuse a
/// wall across the map, where the slab test would first spend three
/// divisions on it. The bigger the map, the more of the table this rejects.
#[must_use]
pub fn near(from: Vector3, to: Vector3, rect: Vector4) -> bool {
    from.x.min(to.x) <= rect.z
        && from.x.max(to.x) >= rect.x
        && from.z.min(to.z) <= rect.w
        && from.z.max(to.z) >= rect.y
}

/// Whether the segment `from -> to` crosses the wall box `rect` swept
/// y ∈ [0, `wall_top`] — the classic three-slab test, clamped to the
/// graze-free parametric window, behind [`near`]'s exact cheap refusal.
/// Total on any input: a zero direction component degenerates to a
/// point-in-slab check.
#[must_use]
pub fn crosses(from: Vector3, to: Vector3, rect: Vector4, wall_top: f32) -> bool {
    if !near(from, to, rect) {
        return false;
    }
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
/// `level_plan::SOURCE_THROUGH^n` muffle exponent and the discard
/// predicate (n > 0) of the acoustic-image shaders. This is the CAMERA
/// occluder (eye -> shaded point): every wall the line pierces counts.
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

/// Does ANY wall stand between a sound's source and `to`? The SOURCE
/// occluder as a predicate — [`crossings_from`]'s question without its
/// arithmetic.
///
/// A wall is a barrier and not a fade, so no reader of the source occluder
/// needs the count any more: one wall extinguishes a wave exactly as ten
/// do. This returns on the FIRST wall it finds instead of testing all
/// [`MAXW`] of them, which is what makes the law affordable in the hearing
/// pass, where it is now paid per fragment per live pulse per sphere root
/// rather than once per fragment.
///
/// The birth wall is skipped exactly as [`crossings_from`] skips it, so a
/// tap struck flush on a wall still reaches that wall's own near face.
///
/// The GLSL `wall_blocked_from` in `game/shaders/pulse_pool.gdshaderinc`
/// transliterates this function, and both of its readers —
/// `source_reveal_vis` in `data_core.gdshaderinc` and the shell loop in
/// `hearing_post.gdshader` — ask it rather than a count.
#[must_use]
pub fn blocked_from(from: Vector3, to: Vector3, rects: &[Vector4], wall_top: f32) -> bool {
    rects
        .iter()
        .any(|r| !contains(*r, from, wall_top) && crosses(from, to, *r, wall_top))
}

/// How much of a wave's REVEAL survives the walls between its source and
/// the lit point.
///
/// A wall is a barrier no sound crosses, so this is a gate and not an
/// attenuation: full reveal with a clear line, nothing at all once any
/// wall stands in the way. Pulse kind is deliberately absent — a cane
/// tap, its echoes, a footstep and a world source's wave all stop at a
/// wall alike, and a parameter that cannot change the answer would be a
/// lie about the domain.
///
/// `blocked` comes from [`blocked_from`], which skips the wall a source is
/// born inside, so a sound struck flush on a wall still lights that wall's
/// own near face. It takes the PREDICATE and not a crossing count on
/// purpose: a count would imply the answer could depend on how many walls
/// stood there, and it cannot — one is a barrier exactly as ten are, and a
/// parameter that cannot change the answer is a lie about the domain.
///
/// The GLSL `source_reveal_vis` in `game/shaders/data_core.gdshaderinc`
/// transliterates this composition — `wall_blocked_from(src, world) ? 0.0 :
/// 1.0` — and the two are held in step by
/// `game/tests/shader_contract_test.gd`.
#[must_use]
pub const fn reveal_visibility(blocked: bool) -> f64 {
    if blocked { 0.0 } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WALL_TOP: f32 = level_plan::WALL_H as f32;

    /// The three-slab test with NO [`near`] gate in front of it — the
    /// reference the fast path is held against, kept deliberately as a
    /// duplicate so a change to one is caught by the other.
    fn crosses_slabs_only(from: Vector3, to: Vector3, rect: Vector4, wall_top: f32) -> bool {
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

    /// The cheap refusal must be EXACT, not a heuristic. Over a dense sweep
    /// of sight lines against this fixture's walls — including vertical
    /// ones, degenerate points, and lines that graze a corner — gating on
    /// [`near`] answers identically to the bare slab test. A false negative
    /// here would silently un-occlude a wall inside a fragment shader,
    /// where nothing can be stepped through with a debugger.
    #[test]
    fn the_cheap_refusal_never_changes_an_answer() {
        let rects = retired_map_rects();
        let mut agreed = 0_u64;
        let mut ever_crossed = false;
        let step = 1.7_f32;
        for i in 0..12 {
            for j in 0..12 {
                let from = Vector3::new(i as f32 * step, 0.9, j as f32 * step);
                for k in 0..12 {
                    for l in 0..12 {
                        let to = Vector3::new(k as f32 * step, 2.4, l as f32 * step);
                        for rect in &rects {
                            let fast = crosses(from, to, *rect, WALL_TOP);
                            let slow = crosses_slabs_only(from, to, *rect, WALL_TOP);
                            assert_eq!(fast, slow, "{from} -> {to} against {rect}");
                            ever_crossed |= slow;
                            agreed += 1;
                        }
                    }
                }
            }
        }
        // a sweep that never crossed anything would agree vacuously
        assert!(ever_crossed);
        assert!(agreed > 100_000);
    }

    /// `near` is the necessary condition it claims to be: a wall whose rect
    /// the segment's own bounding box misses can never be crossed, and this
    /// fixture has plenty of such pairs — that is where the saving is.
    #[test]
    fn a_far_wall_is_refused_without_the_slab_test() {
        let rects = retired_map_rects();
        let from = Vector3::new(1.0, 0.9, 1.0);
        let to = Vector3::new(2.0, 0.9, 2.0);
        let refused = rects.iter().filter(|r| !near(from, to, **r)).count();
        assert!(refused > 0, "no wall was cheaply refused");
        for rect in rects.iter().filter(|r| !near(from, to, **r)) {
            assert!(!crosses(from, to, *rect, WALL_TOP));
        }
    }

    /// A RETIRED 20×20/10-wall map — NOT the shipped 28×28/19-wall scene in
    /// `game/scenes/level_01.tscn`. Kept as the derivation fixture because
    /// DividerNorth and FanRoomSouth are byte-identical between the two
    /// maps and every sight line this module tests stays inside their
    /// shared bounding boxes; the other nine walls this fixture carries do
    /// not exist in the shipped scene, and it is not extended when the
    /// scene grows. A passing test here is not a claim about the shipped
    /// map — `WaveLevel::wall_rects()` derives the real, current table, and
    /// `game/tests/data_skins_test.gd`'s
    /// `test_explain_ray_matches_the_pinned_crossing_counts` runs these same
    /// lines against the REAL scene, through `WaveObserver`.
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
            &retired_map_rects(),
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
            &retired_map_rects(),
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
            &retired_map_rects(),
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
        assert_eq!(crossings(eye, grazed, &retired_map_rects(), WALL_TOP), 0);
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
            &retired_map_rects(),
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
            crossings_from(
                born_in_divider,
                open_fan_room,
                &retired_map_rects(),
                WALL_TOP
            ),
            0,
        );
        assert_eq!(
            crossings(
                born_in_divider,
                open_fan_room,
                &retired_map_rects(),
                WALL_TOP
            ),
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
            crossings_from(
                born_in_divider,
                far_corridor,
                &retired_map_rects(),
                WALL_TOP
            ),
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
            crossings_from(spawn, fan, &retired_map_rects(), WALL_TOP),
            crossings(spawn, fan, &retired_map_rects(), WALL_TOP),
        );
        assert_eq!(
            crossings_from(spawn, fan, &retired_map_rects(), WALL_TOP),
            1
        );
    }

    /// The reveal law is TOTAL and kind-free: a wave reveals fully on a
    /// clear line and NOTHING once a wall stands in the way. Catches the
    /// break this branch exists to fix — any per-kind transmission
    /// privilege reintroduced here (a hum surviving at 0.55, say) makes one
    /// of these fail, because there is no third answer to return.
    #[test]
    fn a_wall_extinguishes_a_wave_whatever_made_it() {
        assert!((reveal_visibility(false) - 1.0).abs() < 1e-12);
        assert!(reveal_visibility(true).abs() < 1e-12);
    }

    /// ...and the composition the GLSL actually performs, over the real
    /// geometry rather than over a bare bool: the same westward line is
    /// full reveal through the divider's opening and none at all through
    /// the wall beside it. This is the assertion that fails if
    /// `reveal_visibility` is ever composed with the wrong predicate — with
    /// the CAMERA occluder, say, which on the birth-wall geometry disagrees.
    #[test]
    fn the_reveal_law_composes_with_the_source_occluder() {
        let rects = retired_map_rects();
        let top = WALL_TOP;
        let through = reveal_visibility(blocked_from(
            Vector3::new(8.6, 0.9, 10.2),
            Vector3::new(3.0, 0.9, 10.2),
            &rects,
            top,
        ));
        let beside = reveal_visibility(blocked_from(
            Vector3::new(8.6, 0.9, 4.0),
            Vector3::new(3.0, 0.9, 4.0),
            &rects,
            top,
        ));
        assert!((through - 1.0).abs() < 1e-12);
        assert!(beside.abs() < 1e-12);
        // and the birth wall stays skipped through the composition: a
        // source standing on the divider's own centerline still lights east
        assert!(
            (reveal_visibility(blocked_from(
                Vector3::new(6.4, 0.9, 4.0),
                Vector3::new(10.0, 0.9, 4.0),
                &rects,
                top
            )) - 1.0)
                .abs()
                < 1e-12
        );
    }

    /// The three lines whose ANSWER the whole barrier law turns on, hand
    /// derived against `retired_map_rects` rather than against the counter:
    /// a line inside one room meets no wall, spawn-to-fan meets
    /// DividerNorth, and a source standing on that same divider's
    /// centerline reaches east past it because the birth wall is skipped.
    /// A `blocked_from` that dropped the `contains` skip would report the
    /// third as blocked; one that answered the complement would fail all
    /// three.
    #[test]
    fn blocked_from_reads_the_source_occluder() {
        let rects = retired_map_rects();
        assert!(!blocked_from(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(5.0, 0.9, 6.0),
            &rects,
            WALL_TOP
        ));
        assert!(blocked_from(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 1.15, 4.4),
            &rects,
            WALL_TOP
        ));
        assert!(!blocked_from(
            Vector3::new(6.4, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 4.0),
            &rects,
            WALL_TOP
        ));
    }

    /// THE POSITIVE HALF OF THE BARRIER LAW: a wave reaches the next room
    /// through a DOORWAY, and the doorway is not a special case in the code
    /// — it is the absence of a rect. `retired_map_rects` runs the divider
    /// as two segments, z ∈ [0.47, 8.13] and z ∈ [12.27, 19.53], leaving
    /// the opening between them, so the SAME westward line answers both
    /// ways depending only on the z it is drawn at.
    ///
    /// Without this pair every wave test in the repository asserts that
    /// something goes DARK, which an occluder that swallowed the whole
    /// level would satisfy perfectly. This is the case that fails if a
    /// doorway ever seals — if the shrink grew, if run segments overlapped
    /// their residues, or if `crosses` stopped honouring the gap.
    #[test]
    fn a_doorway_admits_the_wave_the_wall_beside_it_stops() {
        let rects = retired_map_rects();
        let through = Vector3::new(3.0, 0.9, 10.2);
        let beside = Vector3::new(3.0, 0.9, 4.0);
        assert!(
            !blocked_from(Vector3::new(8.6, 0.9, 10.2), through, &rects, WALL_TOP),
            "the divider's opening spans z 8.13..12.27; a line at z = 10.2 crosses no rect"
        );
        assert!(blocked_from(
            Vector3::new(8.6, 0.9, 4.0),
            beside,
            &rects,
            WALL_TOP
        ));
        // and the counter agrees with the predicate on both, so the two
        // Rust forms of the source occluder cannot drift apart on the very
        // geometry the law is stated over
        assert_eq!(
            crossings_from(Vector3::new(8.6, 0.9, 10.2), through, &rects, WALL_TOP),
            0
        );
        assert_eq!(
            crossings_from(Vector3::new(8.6, 0.9, 4.0), beside, &rects, WALL_TOP),
            1
        );
    }

    /// `blocked_from` exists only to stop walking walls once the answer can
    /// no longer change — the hearing pass now pays this walk per fragment
    /// per pulse — so it must agree with `crossings_from(..) > 0` on EVERY
    /// line, not merely on the three hand-derived above. Swept over a grid
    /// that variously misses every wall, clips one, crosses two, grazes an
    /// endpoint and starts inside a wall: an early return placed on the
    /// wrong branch, a lost birth-wall skip, or a loop that stops before
    /// the last rect all disagree somewhere in the sweep. The grid is
    /// asserted to contain all three verdicts, so a fixture that drifted
    /// into testing only one of them fails instead of passing vacuously.
    #[test]
    fn blocked_from_agrees_with_counting_on_every_line() {
        let rects = retired_map_rects();
        let mut blocked = 0;
        let mut clear = 0;
        let mut born_in_wall = 0;
        for i in 0..24_u8 {
            for j in 0..24_u8 {
                let from = Vector3::new(0.5 + f32::from(i) * 0.85, 0.9, 0.5 + f32::from(j) * 0.85);
                for k in 0..8_u8 {
                    let a = f32::from(k) * std::f32::consts::FRAC_PI_4;
                    let to = from + Vector3::new(a.cos() * 7.0, 0.0, a.sin() * 7.0);
                    let counted = crossings_from(from, to, &rects, WALL_TOP) > 0;
                    assert_eq!(
                        blocked_from(from, to, &rects, WALL_TOP),
                        counted,
                        "{from:?} -> {to:?}"
                    );
                    if counted {
                        blocked += 1;
                    } else {
                        clear += 1;
                    }
                    if rects.iter().any(|r| contains(*r, from, WALL_TOP)) {
                        born_in_wall += 1;
                    }
                }
            }
        }
        assert!(blocked > 100, "grid never crossed a wall: {blocked}");
        assert!(clear > 100, "grid never had a clear line: {clear}");
        assert!(born_in_wall > 0, "grid never started inside a wall");
    }
}
