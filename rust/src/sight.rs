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
//! world XZ rect ([`wall_rect`]), swept through the world Y span of the box
//! the paint pass draws ([`Occluder`]) — never a global wall height. The rect is
//! SHRUNK by [`RECT_SHRINK`] relative to the wall's real box so a prop
//! standing flush against a wall face is never self-shadowed by contact
//! grazing, and both parametric ends of the sight line are ignored by
//! [`GRAZE_EPS`] so neither the camera nor the shaded fragment counts a
//! surface it merely touches.

use godot::builtin::{Vector2, Vector3, Vector4};

use crate::level_plan;

/// Wall slots the sight shaders allocate (`u_walls[MAXW]`) — a level
/// with more walls than this cannot be occluded honestly and says so.
///
/// Raising it is nearly free and the map has outgrown the old 16: the GLSL
/// loops `break` at `u_wall_count`, so the only cost of an unused slot is
/// its slots in the materials' uniform buffers. A wall now costs TWO array
/// elements — its `vec4` rect and its `vec2` span — and std140 rounds every
/// array element's stride up to a `vec4`, so that is 32 B per wall and 1 KB
/// at 32 slots, not the 512 B this note claimed while the span was still a
/// single global. Against the 3.4 KB the pulse lanes already occupy and a
/// 16 KB floor on the smallest WebGL2 block, still cheap. What is NOT free is a wall a level actually
/// holds: every one of them is another rect in the per-fragment sight loop,
/// which is why [`near`] exists.
pub const MAXW: usize = 32;

/// Meters the occluder rect stops short of the wall's real face, so a
/// prop flush against the wall keeps an unblocked sight line.
///
/// It answers to a second, unrelated caller, and that one sets its floor.
/// `hearing_post` reconstructs a world point from the B channel and asks
/// this table whether a wall stands there; the reconstruction is only as
/// good as the channel's worst quantisation gap, so this tolerance has to
/// exceed half of it or a lit wall reads as an x-rayed source seen through
/// one. Measured across every base of a swept column, that half-gap is
/// 24.4 mm at the shipped packing range — see
/// [`crate::render::channel::WORST_STEP_CODES`] — which is why this is
/// 0.03 and not the 0.02 it sat at while the channel was believed to
/// deliver a clean 10-bit code.
pub const RECT_SHRINK: f64 = 0.03;

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

/// One wall's occluder: the world XZ rect its centerline inflates into
/// ([`wall_rect`]), swept through the world Y span of the SAME box the
/// paint pass draws.
///
/// The span travels INSIDE the value, and that is the whole point. It used
/// to be one global `wall_top` pushed as [`level_plan::WALL_H`], with every
/// occluder swept `[0, WALL_H]` regardless of where its wall actually
/// stood — and nothing constrains a wall's Y.
/// `level_plan::plan_wall_transform` normalises a wall's BASIS and carries
/// `origin.y` through untouched; `level_plan::wall_segment` then writes
/// `(x1, z1, x2, z2)` and discards the height at the one point it could
/// have been noticed. So a wall lifted with the gizmo left a phantom
/// barrier in the open air beneath it and an unoccluded strip across its
/// raised top, and a level root lifted bodily put every occluder below the
/// map — `blocked_from` answering false everywhere, the barrier law failing
/// open across the whole level in silence.
///
/// A table of rects beside a table of tops is two things that can disagree.
/// This is one thing that cannot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Occluder {
    rect: Vector4,
    span: Vector2,
}

impl Occluder {
    /// The occluder a wall stands for: its centerline `segment` inflated by
    /// [`wall_rect`], swept from `bottom` to `top` — the Y lanes of the
    /// same world box the paint pass reads.
    ///
    /// The span is ordered here, so a caller that hands the lanes over
    /// backwards still gets the box it meant. `None` when a lane has no
    /// finite f32 representation: absence is a domain result, never a
    /// silent zero, because a zero-height occluder and an undescribable one
    /// are different facts and only one of them is a bug.
    #[must_use]
    pub fn new(segment: Vector4, bottom: f64, top: f64) -> Option<Self> {
        let (bottom, top) = (bottom as f32, top as f32);
        (bottom.is_finite() && top.is_finite()).then(|| Self {
            rect: wall_rect(segment),
            span: Vector2::new(bottom.min(top), bottom.max(top)),
        })
    }

    /// An occluder built from a solid's own world AABB rather than from a
    /// centerline.
    ///
    /// [`Self::new`] is for WALLS, which are authored as a centerline and
    /// inflate outward by their own thickness. A pillar has no centerline —
    /// it has a footprint — so inflating one would double its width. This
    /// takes the footprint the solid actually occupies and shrinks it by
    /// [`RECT_SHRINK`], the same hair `wall_rect` leaves, so a crate shoved
    /// flush against a pillar is not swallowed by it.
    ///
    /// The shrink is taken off BOTH sides of each axis, so a footprint
    /// thinner than twice it would invert and the solid would stop
    /// occluding — silently, since a refused occluder is simply absent from
    /// the table. The shipped shelf is exactly that thin. So the hair is
    /// capped per axis at a quarter of what the solid has: anything wide
    /// enough pays the full [`RECT_SHRINK`], and anything thinner keeps a
    /// proportionally thinner rect rather than losing its volume.
    ///
    /// Refuses anything that describes no volume: an inverted or degenerate
    /// footprint, or a non-finite corner. A refused solid simply does not
    /// occlude, which is what it did yesterday — the failure direction that
    /// changes nothing rather than the one that blanks a room.
    #[must_use]
    pub fn from_bounds(
        min_x: f64,
        min_z: f64,
        max_x: f64,
        max_z: f64,
        bottom: f64,
        top: f64,
    ) -> Option<Self> {
        let shrink = RECT_SHRINK as f32;
        let (min_x, min_z) = (min_x as f32, min_z as f32);
        let (max_x, max_z) = (max_x as f32, max_z as f32);
        let (bottom, top) = (bottom as f32, top as f32);
        if !(min_x.is_finite()
            && min_z.is_finite()
            && max_x.is_finite()
            && max_z.is_finite()
            && bottom.is_finite()
            && top.is_finite())
        {
            return None;
        }
        // The volume test now has to be its own step. It used to fall out
        // of the shrink — a footprint with nothing in it inverted and
        // `is_empty` caught the inversion — but a shrink that yields to a
        // thin solid cannot invert anything, so a zero-width footprint
        // would sail through as a rect of no width.
        // plain comparisons, not negated ones: every corner is already
        // known finite four lines up, so nothing here is incomparable
        if max_x <= min_x || max_z <= min_z {
            return None;
        }
        // per axis, and never more than a quarter of the extent: a plank
        // 0.04 m thick has no 0.03 m to give away twice over
        let hair = |lo: f32, hi: f32| shrink.min((hi - lo) * 0.25);
        let (hx, hz) = (hair(min_x, max_x), hair(min_z, max_z));
        let rect = Vector4::new(min_x + hx, min_z + hz, max_x - hx, max_z - hz);
        let occluder = Self {
            rect,
            span: Vector2::new(bottom.min(top), bottom.max(top)),
        };
        (!occluder.is_empty()).then_some(occluder)
    }

    /// An occluder built from an already-inflated rect and a raw span,
    /// ordering nothing and checking nothing — for tests that need to
    /// reach the degenerate shapes [`Self::new`] refuses to build.
    #[must_use]
    pub const fn from_parts(rect: Vector4, bottom: f32, top: f32) -> Self {
        Self {
            rect,
            span: Vector2::new(bottom, top),
        }
    }

    /// The slot a wall whose world box cannot be described takes: an
    /// occluder no point is inside and no segment crosses.
    ///
    /// A refused wall keeps its SLOT rather than being dropped, because
    /// `WaveLevel::wall_names()[i]` names `occluders[i]` — a hole slides
    /// every later name onto the wrong wall, and a diagnostic that blames
    /// the wrong wall is worse than none.
    pub const NOWHERE: Self = Self::from_parts(Vector4::new(1.0, 1.0, -1.0, -1.0), 1.0, -1.0);

    /// The world XZ rect, `(min_x, min_z, max_x, max_z)` — the `u_walls`
    /// lane.
    #[must_use]
    pub const fn rect(self) -> Vector4 {
        self.rect
    }

    /// The world Y sweep, `(bottom, top)` — the `u_wall_y` lane.
    #[must_use]
    pub const fn span(self) -> Vector2 {
        self.span
    }

    /// Empty on some axis, or non-finite — an occluder that describes no
    /// volume and must therefore stop nothing.
    ///
    /// CHECKED, not left to the slab arithmetic, because that arithmetic
    /// accepts an inverted interval: with `lo = 1`, `hi = -1` a segment
    /// from `x = -5` to `x = +5` gives `ta = 1`, `tb = -1`, so `t0` stays
    /// 0.001 and `t1` stays 0.999 and no axis rejects. `near` does not save
    /// it either — that segment's own bounding box overlaps the inverted
    /// rect.
    ///
    /// Written `!(a <= b)` and not `a > b` on purpose: every comparison
    /// against NaN is false, so the `>` form would read a NaN lane as
    /// ORDERED and hand it to a slab test where `t0 > t1` is also false —
    /// an occluder that swallows the level.
    #[must_use]
    #[expect(
        clippy::neg_cmp_op_on_partial_ord,
        reason = "the negated form is the point: every comparison against \
                  NaN is false, so `!(a <= b)` reads a NaN lane as EMPTY \
                  while the `a > b` clippy suggests reads it as ordered and \
                  hands it to a slab test where `t0 > t1` is also false — an \
                  occluder that swallows the level. The GLSL transliteration \
                  in pulse_pool.gdshaderinc is written the same way and for \
                  the same reason."
    )]
    fn is_empty(self) -> bool {
        !(self.rect.x <= self.rect.z)
            || !(self.rect.y <= self.rect.w)
            || !(self.span.x <= self.span.y)
    }
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

/// WHERE the segment `from -> to` first enters `occ`, as a fraction of the
/// segment, or `None` if it never does — the three-slab test reporting its
/// own `t0` instead of discarding it.
///
/// [`crosses`] is now a reading of this, so the two can never disagree
/// about whether a wall is in the way while disagreeing about where.
///
/// # On NaN, and a claim worth not repeating
///
/// It is tempting to say this answers `Some(NaN)` for a malformed segment,
/// since `lo[k] - a[k]` is NaN and `t0 > t1` is false the way every NaN
/// comparison is false. MEASURED, it does not: Rust's `f32::max` and
/// `f32::min` SUPPRESS NaN — `0.001f32.max(f32::NAN)` is `0.001` — so the
/// window survives intact and the fraction that comes back is finite. A
/// non-finite XZ coordinate is refused earlier still, by [`near`].
///
/// This is a real difference from the GLSL twin, not a detail. GLSL leaves
/// `max`/`min` with a NaN operand implementation-defined, so `wall_entry`
/// CAN hand back a NaN where this cannot, and its caller needs a guard this
/// one does not. [`visible_air`]'s non-finite arm is the matching barrier,
/// kept as defence-in-depth rather than as live arithmetic — see its own
/// note on which direction that guard has to fail in.
#[must_use]
pub fn entry(from: Vector3, to: Vector3, occ: Occluder) -> Option<f32> {
    if occ.is_empty() {
        return None;
    }
    let rect = occ.rect();
    let span = occ.span();
    if !near(from, to, rect) {
        return None;
    }
    let a = [from.x, from.y, from.z];
    let d = [to.x - from.x, to.y - from.y, to.z - from.z];
    let lo = [rect.x, span.x, rect.y];
    let hi = [rect.z, span.y, rect.w];
    let mut t0 = GRAZE_EPS as f32;
    let mut t1 = (1.0 - GRAZE_EPS) as f32;
    for k in 0..3 {
        if d[k].abs() < AXIS_TINY {
            if a[k] < lo[k] || a[k] > hi[k] {
                return None;
            }
        } else {
            let ta = (lo[k] - a[k]) / d[k];
            let tb = (hi[k] - a[k]) / d[k];
            t0 = t0.max(ta.min(tb));
            t1 = t1.min(ta.max(tb));
            if t0 > t1 {
                return None;
            }
        }
    }
    Some(t0)
}

/// Whether the segment `from -> to` crosses `occ` — its XZ rect swept
/// through its OWN y span — by the classic three-slab test, clamped to the
/// graze-free parametric window, behind [`near`]'s exact cheap refusal.
/// Total on any input: a zero direction component degenerates to a
/// point-in-slab check.
#[must_use]
pub fn crosses(from: Vector3, to: Vector3, occ: Occluder) -> bool {
    entry(from, to, occ).is_some()
}

/// The nearest entry among `occluders`, as a fraction of the segment.
///
/// `total_cmp` rather than `partial_cmp().unwrap()`: [`entry`] can answer
/// `Some(NaN)` by design, and an ordering that panics on it would be a
/// panic waiting for a future edit rather than a bug caught today.
#[must_use]
pub fn first_entry(from: Vector3, to: Vector3, occluders: &[Occluder]) -> Option<f32> {
    occluders
        .iter()
        .filter_map(|occ| entry(from, to, *occ))
        .min_by(f32::total_cmp)
}

/// How far the eye can see AIR along `from -> to`: the whole segment when
/// nothing is in the way, or the distance to the nearest wall it enters.
///
/// # Why this exists, and what it replaces
///
/// `hearing_post` used to cut the player's expanding rings with a BOOLEAN —
/// "is the surface at this pixel seen through a wall?" — ORed into the
/// depth compare. A boolean is fragment-constant: it kills EVERY ring root
/// at that pixel, including rings that are physically nearer than the wall
/// and nearer than the thing behind it. Because an x-rayed source's skin
/// takes the pixel from the wall behind it, that flag was true across the
/// whole screen-space silhouette of any source seen through a wall, and
/// punched a source-shaped HOLE in rings that had every right to be drawn.
///
/// A cut is a DISTANCE, never a flag. Everything that ends the eye's view
/// of air folds in with a minimum, and a root nearer than all of them
/// survives.
///
/// Total over every input: a non-finite segment answers 0.0 — no air at
/// all — and that single guard is what actually catches every malformed
/// sight line today, because a NaN coordinate makes the segment's own
/// length non-finite before the table is ever consulted.
///
/// The `Some(non-finite)` arm below is DEFENCE-IN-DEPTH and is honestly
/// unreachable through [`entry`]'s current arithmetic (Rust's `max`/`min`
/// suppress NaN). It is kept because the GLSL twin has no such guarantee,
/// and because the direction it fails in is the one that matters: under
/// GLSL every comparison against NaN is false, so a NaN cut distance would
/// make `t >= air_d` false for EVERY ring root, and one bad pixel would
/// draw every ring in the level straight through every wall. Answering
/// "no air" draws nothing instead.
#[must_use]
pub fn visible_air(from: Vector3, to: Vector3, occluders: &[Occluder]) -> f64 {
    let scene_d = f64::from((to - from).length());
    if !scene_d.is_finite() {
        return 0.0;
    }
    match first_entry(from, to, occluders) {
        Some(t) if t.is_finite() => scene_d * f64::from(t),
        Some(_) => 0.0,
        None => scene_d,
    }
}

/// How many of the wall rects the sight line `from -> to` crosses — the
/// `level_plan::SOURCE_THROUGH^n` muffle exponent and the discard
/// predicate (n > 0) of the acoustic-image shaders. This is the CAMERA
/// occluder (eye -> shaded point): every wall the line pierces counts.
#[must_use]
pub fn crossings(from: Vector3, to: Vector3, occluders: &[Occluder]) -> u32 {
    occluders
        .iter()
        .map(|occ| u32::from(crosses(from, to, *occ)))
        .sum()
}

/// Whether the point `p` lies inside `occ` — its XZ rect AND its own
/// vertical span. Total on any input. The wall a sound is born inside cannot block that sound's own
/// reveal, so [`crossings_from`] skips whatever this reports.
#[must_use]
pub fn contains(occ: Occluder, p: Vector3) -> bool {
    if occ.is_empty() {
        return false;
    }
    let rect = occ.rect();
    let span = occ.span();
    p.x >= rect.x
        && p.x <= rect.z
        && p.z >= rect.y
        && p.z <= rect.w
        && p.y >= span.x
        && p.y <= span.y
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
pub fn crossings_from(from: Vector3, to: Vector3, occluders: &[Occluder]) -> u32 {
    occluders
        .iter()
        .filter(|occ| !contains(**occ, from))
        .map(|occ| u32::from(crosses(from, to, *occ)))
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
pub fn blocked_from(from: Vector3, to: Vector3, occluders: &[Occluder]) -> bool {
    occluders
        .iter()
        .any(|occ| !contains(*occ, from) && crosses(from, to, *occ))
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

    const WALL_TOP: f64 = level_plan::WALL_H;

    /// The occluder a floor-standing wall stands for — the shipped shape,
    /// swept 0 to WALL_H. Every count pinned in this module predates
    /// per-wall spans and must survive them unchanged, so the fixtures say
    /// so explicitly rather than by omission.
    fn standing(segment: Vector4) -> Occluder {
        Occluder::new(segment, 0.0, WALL_TOP).expect("a floor-standing wall is describable")
    }

    /// The three-slab test with NO [`near`] gate in front of it — the
    /// reference the fast path is held against, kept deliberately as a
    /// duplicate so a change to one is caught by the other.
    fn crosses_slabs_only(from: Vector3, to: Vector3, occ: Occluder) -> bool {
        let (rect, span) = (occ.rect(), occ.span());
        let a = [from.x, from.y, from.z];
        let d = [to.x - from.x, to.y - from.y, to.z - from.z];
        let lo = [rect.x, span.x, rect.y];
        let hi = [rect.z, span.y, rect.w];
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
        // A vertically NON-UNIFORM table, not the shipped flat one: `near`
        // is an XZ-only refusal and its exactness argument has to survive
        // walls that sweep different heights, which is the whole point of
        // the occluder carrying its own span. Every third wall is lifted
        // clear of the floor and every fifth is a low kerb.
        let occluders: Vec<Occluder> = retired_map_occluders()
            .iter()
            .enumerate()
            .map(|(i, occ)| match i % 5 {
                0 => Occluder::from_parts(occ.rect(), 1.0, 4.0),
                3 => Occluder::from_parts(occ.rect(), 0.0, 0.6),
                _ => *occ,
            })
            .collect();
        let mut agreed = 0_u64;
        let mut ever_crossed = false;
        let mut ever_refused = false;
        let step = 1.7_f32;
        for i in 0..12 {
            for j in 0..12 {
                let from = Vector3::new(i as f32 * step, 0.9, j as f32 * step);
                for k in 0..12 {
                    for l in 0..12 {
                        let to = Vector3::new(k as f32 * step, 2.4, l as f32 * step);
                        for occ in &occluders {
                            let fast = crosses(from, to, *occ);
                            let slow = crosses_slabs_only(from, to, *occ);
                            assert_eq!(fast, slow, "{from} -> {to} against {occ:?}");
                            ever_crossed |= slow;
                            ever_refused |= !near(from, to, occ.rect());
                            agreed += 1;
                        }
                    }
                }
            }
        }
        // a sweep that never crossed anything, or never refused anything,
        // would agree vacuously
        assert!(ever_crossed);
        assert!(ever_refused);
        assert!(agreed > 100_000);
    }

    /// `near` is the necessary condition it claims to be: a wall whose rect
    /// the segment's own bounding box misses can never be crossed, and this
    /// fixture has plenty of such pairs — that is where the saving is.
    #[test]
    fn a_far_wall_is_refused_without_the_slab_test() {
        let rects = retired_map_occluders();
        let from = Vector3::new(1.0, 0.9, 1.0);
        let to = Vector3::new(2.0, 0.9, 2.0);
        let refused = rects.iter().filter(|r| !near(from, to, r.rect())).count();
        assert!(refused > 0, "no wall was cheaply refused");
        for occ in rects.iter().filter(|r| !near(from, to, r.rect())) {
            assert!(!crosses(from, to, *occ));
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
    fn retired_map_occluders() -> Vec<Occluder> {
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
        .map(|s| standing(*s))
        .collect()
    }

    /// The inflation is a half-thickness pad shrunk by the contact
    /// epsilon, and reversed segments normalize into min/max order.
    /// THE break this catches: an occluder swept `[0, WALL_H]` no matter
    /// where its wall actually stands. Nothing constrains a wall's Y —
    /// `level_plan::plan_wall_transform` normalises the BASIS and carries
    /// `origin.y` through untouched, and `wall_segment` then discards it —
    /// so a wall lifted with the gizmo leaves a phantom barrier in the open
    /// air beneath it and an unoccluded strip across its raised top.
    ///
    /// Hand-derived from the retired DividerNorth centerline
    /// `(6.4, 0.6) -> (6.4, 8.0)` lifted one metre: pad is
    /// `WALL_T - RECT_SHRINK = 0.15 - 0.03 = 0.12`, so the rect is
    /// `(6.28, 0.48, 6.52, 8.12)` swept `y in [1, 4]`. Three lines from
    /// x = 3 to x = 10 at z = 4 — under it, through its raised top half,
    /// and a control through the middle that must not move.
    #[test]
    fn a_lifted_wall_occludes_where_it_stands_and_nowhere_else() {
        let lifted = Occluder::new(Vector4::new(6.4, 0.6, 6.4, 8.0), 1.0, 4.0)
            .expect("a finite span is describable");
        let rect = lifted.rect();
        // written as centre minus pad, not as 6.28: the literal is a
        // hair under TAU and clippy reads it as a mistyped constant
        let pad = 0.12_f32;
        assert!((rect.x - (6.4 - pad)).abs() < 1e-4 && (rect.y - (0.6 - pad)).abs() < 1e-4);
        assert!((rect.z - (6.4 + pad)).abs() < 1e-4 && (rect.w - (8.0 + pad)).abs() < 1e-4);
        assert_eq!(lifted.span(), Vector2::new(1.0, 4.0));
        let table = [lifted];
        let across = |y: f32| {
            crossings(
                Vector3::new(3.0, y, 4.0),
                Vector3::new(10.0, y, 4.0),
                &table,
            )
        };
        assert_eq!(
            across(0.5),
            0,
            "a phantom barrier in the air under the wall"
        );
        assert_eq!(across(3.5), 1, "an unoccluded strip across the raised top");
        assert_eq!(
            across(2.0),
            1,
            "the control moved: this is an inversion, not a fix"
        );

        // ...and the birth-wall skip, which fails in the DANGEROUS
        // direction. A source on the floor at (6.4, 0.5, 4.0) stands in
        // open air UNDER the lifted wall, so it is not born inside it and
        // the wall must still occlude that source in every direction.
        // Judged "inside" — as a `[0, WALL_H]` sweep judges it — the skip
        // silently disables this wall for this source entirely.
        let under = Vector3::new(6.4, 0.5, 4.0);
        assert!(
            !contains(lifted, under),
            "open air under a wall is not inside it"
        );
        assert_eq!(
            crossings_from(under, Vector3::new(7.0, 3.5, 4.0), &table),
            1
        );
    }

    /// A floor-standing wall sweeps EXACTLY zero to the wall height — no
    /// epsilon, no dust. This is the pixel-identity criterion for the whole
    /// per-wall-span change stated as a test: every count pinned in this
    /// module, and every reading the rendered probe takes, predates it and
    /// must survive it unchanged.
    ///
    /// Built the way `WaveWall::world_shape` builds it — the wall's origin
    /// lifted by `(0, WALL_H/2, 0)`, sized by `level_plan::wall_box` — and
    /// taken through `render::faces::bounds`, which is the same path the
    /// level uses. It fails if the span is ever derived from a mesh AABB,
    /// or shrunk by `RECT_SHRINK` along with the rect.
    #[test]
    fn a_floor_standing_wall_sweeps_exactly_zero_to_the_wall_height() {
        assert_eq!(level_plan::WALL_H, 3.0, "the derivation below assumes it");
        let size = level_plan::wall_box(4.0);
        for basis in [
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
        ] {
            let shape = crate::render::Shape::Box3d {
                center: [5.0, level_plan::WALL_H / 2.0, 5.0],
                size: [size.x as f64, size.y as f64, size.z as f64],
                basis,
            };
            let box3 = crate::render::faces::bounds(&shape).expect("a finite wall box");
            let occ = Occluder::new(Vector4::new(3.0, 5.0, 7.0, 5.0), box3.min[1], box3.max[1])
                .expect("describable");
            assert_eq!(
                occ.span(),
                Vector2::new(0.0, 3.0),
                "a floor-standing wall must sweep exactly 0..WALL_H"
            );
        }
    }

    /// An occluder that describes nothing must be crossed by nothing — and
    /// the slab arithmetic alone does NOT deliver that, which is why
    /// `is_empty` is a checked guard rather than an emergent property.
    ///
    /// The counterexample: an inverted interval still yields a valid
    /// parametric window. With `lo = 1`, `hi = -1`, a segment from x = -5
    /// to x = +5 gives `ta = 1`, `tb = -1`, so `t0` stays 0.001 and `t1`
    /// stays 0.999 and no axis rejects — the function answers CROSSED.
    /// `near` does not save it either: that segment's own bounding box
    /// overlaps the inverted rect. A NaN lane is worse, since every
    /// comparison against it is false.
    #[test]
    fn an_empty_occluder_is_crossed_by_nothing_the_slab_test_would_accept() {
        let from = Vector3::new(-5.0, 0.0, 0.0);
        let to = Vector3::new(5.0, 0.0, 0.0);
        assert!(!crosses(from, to, Occluder::NOWHERE));
        assert!(!contains(Occluder::NOWHERE, Vector3::ZERO));
        // an inverted span specifically, built past the ordering `new` does
        let inverted = Occluder::from_parts(Vector4::new(-1.0, -1.0, 1.0, 1.0), 1.0, -1.0);
        assert!(!crosses(from, to, inverted));
        assert!(!contains(inverted, Vector3::ZERO));
        // and a non-finite one: `!(a <= b)` reads NaN as empty, where
        // `a > b` would read it as ordered and let it swallow the level
        let nan = Occluder::from_parts(Vector4::new(-1.0, -1.0, 1.0, 1.0), f32::NAN, 1.0);
        assert!(!crosses(from, to, nan));
        assert!(!contains(nan, Vector3::ZERO));
        assert_eq!(
            Occluder::new(Vector4::new(0.0, 0.0, 1.0, 1.0), f64::NAN, 3.0),
            None
        );
        assert_eq!(
            Occluder::new(Vector4::new(0.0, 0.0, 1.0, 1.0), 0.0, f64::INFINITY),
            None
        );
    }

    /// Two walls at different heights occlude independently — the property
    /// a single global `wall_top` cannot express at all, and the reason the
    /// span travels inside the occluder rather than beside it.
    #[test]
    fn two_walls_at_different_heights_occlude_independently() {
        let standing = Occluder::new(Vector4::new(6.4, 0.6, 6.4, 8.0), 0.0, 3.0).expect("finite");
        let raised = Occluder::new(Vector4::new(12.0, 0.6, 12.0, 8.0), 1.0, 4.0).expect("finite");
        let table = [standing, raised];
        let low = (Vector3::new(3.0, 0.5, 4.0), Vector3::new(16.0, 0.5, 4.0));
        let high = (Vector3::new(3.0, 3.5, 4.0), Vector3::new(16.0, 3.5, 4.0));
        assert_eq!(
            crossings(low.0, low.1, &table),
            1,
            "only the floor-standing wall"
        );
        assert_eq!(crossings(high.0, high.1, &table), 1, "only the raised wall");
        // the predicate agrees with the count on both
        assert!(blocked_from(low.0, low.1, &table));
        assert!(blocked_from(high.0, high.1, &table));
        // and a line that clears both: above the standing wall, below the raised
        let clear = crossings(
            Vector3::new(3.0, 0.5, 20.0),
            Vector3::new(16.0, 0.5, 20.0),
            &table,
        );
        assert_eq!(clear, 0, "a line past both walls in z crosses neither");
    }

    #[test]
    fn wall_rect_inflates_and_normalizes() {
        let divider = wall_rect(Vector4::new(6.4, 0.6, 6.4, 8.0));
        let pad = 0.12_f32;
        assert!((divider.x - (6.4 - pad)).abs() < 1e-4);
        assert!((divider.y - (0.6 - pad)).abs() < 1e-4);
        assert!((divider.z - (6.4 + pad)).abs() < 1e-4);
        assert!((divider.w - (8.0 + pad)).abs() < 1e-4);
        let reversed = wall_rect(Vector4::new(19.4, 19.4, 0.6, 19.4));
        assert!((reversed.x - 0.48).abs() < 1e-4);
        assert!((reversed.z - 19.52).abs() < 1e-4);
        // ...and an Occluder inflates through the very same function, so
        // the rect law has one home whatever sweep is wrapped around it
        assert_eq!(standing(Vector4::new(6.4, 0.6, 6.4, 8.0)).rect(), divider);
    }

    /// Spawn to fan head: exactly one wall (DividerNorth) stands between
    /// the hero's waking pose and the hum — the muffle exponent the fan's
    /// through-wall outline rides.
    #[test]
    fn spawn_to_fan_crosses_exactly_one_wall() {
        let n = crossings(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 1.15, 4.4),
            &retired_map_occluders(),
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
            &retired_map_occluders(),
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
            &retired_map_occluders(),
        );
        assert_eq!(n, 2);
    }

    /// A prop flush against a wall face is NOT self-blocked: the shrunk
    /// rect leaves its surface a clear 0.03 m outside — while the same
    /// point against the UNSHRUNK wall box would already count as inside.
    #[test]
    fn flush_prop_is_not_self_blocked_by_the_shrink() {
        let eye = Vector3::new(8.0, 1.2, 4.0);
        // 5 mm inside the wall's REAL east face at x = 6.55 — where world
        // reconstruction dust can land a fragment of a flush prop
        let grazed = Vector3::new(6.545, 1.2, 4.0);
        assert_eq!(crossings(eye, grazed, &retired_map_occluders()), 0);
        let unshrunk = Occluder::from_parts(Vector4::new(6.25, 0.45, 6.55, 8.15), 0.0, 3.0);
        assert!(crosses(eye, grazed, unshrunk));
    }

    /// A wall BEHIND the shaded point never blocks it: the divider lies
    /// past the target, outside the parametric window.
    #[test]
    fn wall_behind_the_target_does_not_count() {
        let n = crossings(
            Vector3::new(8.0, 1.0, 4.0),
            Vector3::new(7.0, 1.0, 4.4),
            &retired_map_occluders(),
        );
        assert_eq!(n, 0);
    }

    /// Grazing an occluder face exactly at either endpoint is not a
    /// crossing: a fragment ON the rect, or a camera flush against it,
    /// stays unblocked — the GRAZE_EPS window at work.
    ///
    /// The endpoint must sit ON the shrunk face, `6.4 - (WALL_T -
    /// RECT_SHRINK) = 6.28`, and re-deriving it is not cosmetic: at the old
    /// 6.27 the point is a centimetre CLEAR of the rect, the slab walk
    /// misses it outright, and this passes with GRAZE_EPS set to zero. A
    /// test that no longer needs the window it is named for.
    #[test]
    fn endpoint_grazes_are_not_crossings() {
        let divider = standing(Vector4::new(6.4, 0.6, 6.4, 8.0));
        // 6.4 - 0.12, written as the subtraction because the literal is a
        // hair under TAU and clippy reads it as a mistyped constant
        let on_face = Vector3::new(6.4 - 0.12, 0.9, 4.0);
        assert!(!crosses(Vector3::new(3.0, 0.9, 4.0), on_face, divider));
        assert!(!crosses(on_face, Vector3::new(3.0, 0.9, 4.0), divider));
    }

    /// A sight line OVER the walls is clear: the slab test is 3D, and a
    /// wall stops at the ceiling.
    #[test]
    fn sight_over_the_wall_top_is_clear() {
        let divider = standing(Vector4::new(6.4, 0.6, 6.4, 8.0));
        assert!(!crosses(
            Vector3::new(3.0, 3.2, 4.0),
            Vector3::new(8.6, 3.4, 4.4),
            divider
        ));
    }

    /// `contains` reads the padded rect as a solid box floor to ceiling:
    /// a point on the divider centerline is inside; the same point at the
    /// spawn is outside; a point above the wall top is outside.
    #[test]
    fn contains_reads_the_padded_box() {
        let divider = standing(Vector4::new(6.4, 0.6, 6.4, 8.0));
        assert!(contains(divider, Vector3::new(6.4, 0.9, 4.0)));
        assert!(!contains(divider, Vector3::new(3.0, 0.9, 4.0)));
        assert!(!contains(divider, Vector3::new(6.4, 3.2, 4.0)));
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
            crossings_from(born_in_divider, open_fan_room, &retired_map_occluders()),
            0,
        );
        assert_eq!(
            crossings(born_in_divider, open_fan_room, &retired_map_occluders()),
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
            crossings_from(born_in_divider, far_corridor, &retired_map_occluders()),
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
            crossings_from(spawn, fan, &retired_map_occluders()),
            crossings(spawn, fan, &retired_map_occluders()),
        );
        assert_eq!(crossings_from(spawn, fan, &retired_map_occluders()), 1);
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
        let rects = retired_map_occluders();
        let through = reveal_visibility(blocked_from(
            Vector3::new(8.6, 0.9, 10.2),
            Vector3::new(3.0, 0.9, 10.2),
            &rects,
        ));
        let beside = reveal_visibility(blocked_from(
            Vector3::new(8.6, 0.9, 4.0),
            Vector3::new(3.0, 0.9, 4.0),
            &rects,
        ));
        assert!((through - 1.0).abs() < 1e-12);
        assert!(beside.abs() < 1e-12);
        // and the birth wall stays skipped through the composition: a
        // source standing on the divider's own centerline still lights east
        assert!(
            (reveal_visibility(blocked_from(
                Vector3::new(6.4, 0.9, 4.0),
                Vector3::new(10.0, 0.9, 4.0),
                &rects
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
        let rects = retired_map_occluders();
        assert!(!blocked_from(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(5.0, 0.9, 6.0),
            &rects
        ));
        assert!(blocked_from(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 1.15, 4.4),
            &rects
        ));
        assert!(!blocked_from(
            Vector3::new(6.4, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 4.0),
            &rects
        ));
    }

    /// THE POSITIVE HALF OF THE BARRIER LAW: a wave reaches the next room
    /// through a DOORWAY, and the doorway is not a special case in the code
    /// — it is the absence of a rect. `retired_map_rects` runs the divider
    /// as two segments, z ∈ [0.48, 8.12] and z ∈ [12.28, 19.52], leaving
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
        let rects = retired_map_occluders();
        let through = Vector3::new(3.0, 0.9, 10.2);
        let beside = Vector3::new(3.0, 0.9, 4.0);
        assert!(
            !blocked_from(Vector3::new(8.6, 0.9, 10.2), through, &rects),
            "the divider's opening spans z 8.12..12.28; a line at z = 10.2 crosses no rect"
        );
        assert!(blocked_from(Vector3::new(8.6, 0.9, 4.0), beside, &rects));
        // and the counter agrees with the predicate on both, so the two
        // Rust forms of the source occluder cannot drift apart on the very
        // geometry the law is stated over
        assert_eq!(
            crossings_from(Vector3::new(8.6, 0.9, 10.2), through, &rects),
            0
        );
        assert_eq!(
            crossings_from(Vector3::new(8.6, 0.9, 4.0), beside, &rects),
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
        let rects = retired_map_occluders();
        let mut blocked = 0;
        let mut clear = 0;
        let mut born_in_wall = 0;
        for i in 0..24_u8 {
            for j in 0..24_u8 {
                let from = Vector3::new(0.5 + f32::from(i) * 0.85, 0.9, 0.5 + f32::from(j) * 0.85);
                for k in 0..8_u8 {
                    let a = f32::from(k) * std::f32::consts::FRAC_PI_4;
                    let to = from + Vector3::new(a.cos() * 7.0, 0.0, a.sin() * 7.0);
                    let counted = crossings_from(from, to, &rects) > 0;
                    assert_eq!(
                        blocked_from(from, to, &rects),
                        counted,
                        "{from:?} -> {to:?}"
                    );
                    if counted {
                        blocked += 1;
                    } else {
                        clear += 1;
                    }
                    if rects.iter().any(|r| contains(*r, from)) {
                        born_in_wall += 1;
                    }
                }
            }
        }
        assert!(blocked > 100, "grid never crossed a wall: {blocked}");
        assert!(clear > 100, "grid never had a clear line: {clear}");
        assert!(born_in_wall > 0, "grid never started inside a wall");
    }

    /// THE BREAK: raising [`RECT_SHRINK`] quietly deleting the thin props
    /// already standing in the shipped map.
    ///
    /// `from_bounds` takes the shrink off BOTH sides, so a footprint
    /// thinner than twice it inverts and the solid stops occluding —
    /// silently, because a refused occluder simply is not in the table. The
    /// shipped shelf is exactly there: `ShelfBack` is 0.04 m thick in x and
    /// `ShelfSideA/B` are 0.06 m in z (game/scenes/level_01.tscn). At a
    /// 0.02 shrink the back survived as a zero-width rect and the sides sat
    /// one ULP from the same fate; at 0.03 the back inverts outright and a
    /// source seen through it stops paying `prop_through()`.
    ///
    /// So the shrink is per-axis and capped at a quarter of what the solid
    /// has. A thin solid keeps a proportionally thinner rect instead of
    /// losing its volume, and nothing wide enough to afford the full hair
    /// is touched.
    #[test]
    fn a_prop_thinner_than_twice_the_shrink_still_occludes() {
        // ShelfBack: 0.04 m thick, centred at x = 0.92
        let back = Occluder::from_bounds(0.90, 2.0, 0.94, 3.4, 0.0, 1.8)
            .expect("a 4 cm shelf back is still a volume");
        let r = back.rect();
        assert!(r.z > r.x, "the back inverted: {} .. {}", r.x, r.z);
        // a quarter off each side leaves half the plank
        assert!(
            (f64::from(r.z - r.x) - 0.02).abs() < 1.0e-6,
            "width {}",
            r.z - r.x
        );

        // ShelfSideA: 0.06 m deep, which used to survive as exactly zero
        let side = Occluder::from_bounds(0.90, 1.97, 1.40, 2.03, 0.0, 1.8)
            .expect("a 6 cm shelf side is still a volume");
        let s = side.rect();
        assert!(s.w > s.y, "the side collapsed: {} .. {}", s.y, s.w);

        // ...and a solid wide enough to afford the full hair still pays it:
        // a 0.5 m pillar keeps 0.5 - 2 * 0.03 = 0.44
        let pillar = Occluder::from_bounds(1.75, 2.75, 2.25, 3.25, 0.0, 3.0)
            .expect("a half-metre pillar describes a volume");
        let p = pillar.rect();
        assert!(
            (f64::from(p.z - p.x) - 0.44).abs() < 1.0e-6,
            "width {}",
            p.z - p.x
        );
    }

    /// THE BREAK: a solid's footprint being inflated like a wall's
    /// centerline, doubling every pillar's shadow — or shrunk the wrong
    /// way, so a pillar stops occluding the moment anything leans on it.
    ///
    /// Hand-derived: a 0.5 m pillar centred at (2, 3) occupies
    /// [1.75, 2.25] x [2.75, 3.25]; RECT_SHRINK takes 0.03 off each side.
    #[test]
    fn a_solids_footprint_is_taken_as_given_and_not_inflated() {
        let occ = Occluder::from_bounds(1.75, 2.75, 2.25, 3.25, 0.0, 3.0)
            .expect("a half-metre pillar describes a volume");
        let r = occ.rect();
        assert!((f64::from(r.x) - 1.78).abs() < 1e-6, "min_x was {}", r.x);
        assert!((f64::from(r.y) - 2.78).abs() < 1e-6, "min_z was {}", r.y);
        assert!((f64::from(r.z) - 2.22).abs() < 1e-6, "max_x was {}", r.z);
        assert!((f64::from(r.w) - 3.22).abs() < 1e-6, "max_z was {}", r.w);
        assert!((f64::from(occ.span().x) - 0.0).abs() < 1e-6);
        assert!((f64::from(occ.span().y) - 3.0).abs() < 1e-6);
        // The property that matters, stated directly: a FOOTPRINT shrinks
        // inward, strictly inside what the solid occupies. A CENTERLINE
        // does the opposite — the same middle, fed to the wall
        // constructor, straddles it — so feeding a footprint to `new`
        // would hand the sight tests a shadow wider than the pillar.
        assert!(f64::from(r.x) > 1.75 && f64::from(r.z) < 2.25);
        assert!(f64::from(r.y) > 2.75 && f64::from(r.w) < 3.25);
        let centreline = Occluder::new(Vector4::new(2.0, 3.0, 2.0, 3.0), 0.0, 3.0).unwrap();
        let c = centreline.rect();
        assert!(
            f64::from(c.x) < 2.0 && f64::from(c.z) > 2.0,
            "a centerline must straddle its own line, got {c:?}"
        );
    }

    /// THE BREAK: a degenerate or malformed solid entering the table as a
    /// NaN-cornered rect, which every comparison in the slab walk answers
    /// `false` for — a wall that silently stops occluding.
    #[test]
    fn a_footprint_describing_no_volume_is_refused() {
        // NO VOLUME is the criterion, not "thin": a footprint with zero
        // width describes nothing to occlude...
        assert_eq!(Occluder::from_bounds(0.0, 0.0, 0.0, 1.0, 0.0, 3.0), None);
        // ...and one that is merely thinner than twice the shrink is a real
        // volume, kept, because the shrink yields to it
        assert!(Occluder::from_bounds(0.0, 0.0, 0.03, 1.0, 0.0, 3.0).is_some());
        assert_eq!(Occluder::from_bounds(1.0, 0.0, 0.0, 1.0, 0.0, 3.0), None);
        assert_eq!(
            Occluder::from_bounds(f64::NAN, 0.0, 1.0, 1.0, 0.0, 3.0),
            None
        );
        assert_eq!(
            Occluder::from_bounds(0.0, 0.0, 1.0, 1.0, f64::INFINITY, 3.0),
            None
        );
    }

    /// THE BREAK, and it shipped: the ring cut being a fragment-constant
    /// BOOLEAN rather than a distance. A source seen through a wall takes
    /// its own pixels from the wall behind it, so "this pixel is walled"
    /// was true across that source's whole screen-space silhouette — and
    /// killed every ring root there, including rings physically nearer than
    /// the wall. A source-shaped hole, punched in air the eye can see.
    ///
    /// Hand-derived: `Occluder::new` inflates a centerline by
    /// `WALL_T - RECT_SHRINK` = 0.15 - 0.03 = 0.12, so a wall centred at
    /// x = 3 spans x in [2.88, 3.12]. From the eye at the origin to a
    /// source 6 m along +x, the entry fraction is 2.88/6 = 0.48, and the
    /// air the eye can see is exactly 2.88 m.
    #[test]
    fn a_ring_nearer_than_the_wall_survives_a_source_seen_through_it() {
        let wall = Occluder::new(Vector4::new(3.0, -5.0, 3.0, 5.0), 0.0, 3.0)
            .expect("a 6 m wall across the path");
        let eye = Vector3::new(0.0, 1.5, 0.0);
        let src = Vector3::new(6.0, 1.5, 0.0);

        assert!(
            crosses(eye, src, wall),
            "the fixture must actually be walled"
        );
        let air = visible_air(eye, src, &[wall]);
        assert!(
            (air - 2.88).abs() < 1e-4,
            "the eye can see {air} m of air, not the 2.88 m up to the wall"
        );
        // the whole point: a ring at 1.5 m is NEARER than the wall and must
        // still be drawn, where the old boolean discarded it
        assert!(1.5 < air, "a ring 1.5 m out was cut by a wall at 2.88 m");
        // ...and one past the wall is still cut
        assert!(4.0 > air);
    }

    /// THE BREAK: folding the entries with max, or taking the last one — a
    /// table whose walls happen to be ordered far-to-near would then let the
    /// eye see straight through the nearer one.
    #[test]
    fn the_nearest_wall_ends_the_air_whatever_order_the_table_holds() {
        let near_wall = Occluder::new(Vector4::new(3.0, -5.0, 3.0, 5.0), 0.0, 3.0).unwrap();
        let far_wall = Occluder::new(Vector4::new(5.0, -5.0, 5.0, 5.0), 0.0, 3.0).unwrap();
        let eye = Vector3::new(0.0, 1.5, 0.0);
        let src = Vector3::new(6.0, 1.5, 0.0);
        for table in [[near_wall, far_wall], [far_wall, near_wall]] {
            let air = visible_air(eye, src, &table);
            assert!(
                (air - 2.88).abs() < 1e-4,
                "table order changed the answer: {air}"
            );
        }
    }

    /// THE BREAK: [`entry`]'s deliberate NaN reaching a shader. Under GLSL
    /// every comparison against NaN is false, so `t >= air_d` would be
    /// false for EVERY root and one bad pixel would draw every ring in the
    /// level through every wall. The barrier must fail toward drawing
    /// NOTHING.
    #[test]
    fn a_malformed_sight_line_sees_no_air_rather_than_infinite_air() {
        let wall = Occluder::new(Vector4::new(3.0, -5.0, 3.0, 5.0), 0.0, 3.0).unwrap();
        let src = Vector3::new(6.0, 1.5, 0.0);
        for bad in [
            Vector3::new(f32::NAN, 1.5, 0.0),
            Vector3::new(0.0, f32::NAN, 0.0),
            Vector3::new(0.0, 1.5, f32::NAN),
            Vector3::new(f32::INFINITY, 1.5, 0.0),
            Vector3::new(f32::NEG_INFINITY, 1.5, 0.0),
        ] {
            assert_eq!(
                visible_air(bad, src, &[wall]),
                0.0,
                "an undescribable eye at {bad:?} was told it can see air"
            );
            assert_eq!(visible_air(bad, src, &[]), 0.0);
        }
        // and the arithmetic that makes the `Some(non-finite)` arm
        // defence-in-depth rather than live: Rust's max/min suppress NaN,
        // so entry itself never hands one back. GLSL gives no such promise,
        // which is why the shader twin carries its own guard.
        assert_eq!(0.001_f32.max(f32::NAN), 0.001);
        assert_eq!(0.999_f32.min(f32::NAN), 0.999);
        let nan_y = Vector3::new(0.0, f32::NAN, 0.0);
        assert!(entry(nan_y, src, wall).is_none_or(|t| t.is_finite()));
    }

    /// THE BREAK: [`entry`] reporting a fraction that is not where the
    /// segment meets the box — a scaled, offset or stale `t0`, a `min`/`max`
    /// swapped in the slab fold, or the axis loop narrowed so one slab stops
    /// clipping. [`crosses`] cannot catch any of them, because `crosses` IS
    /// `entry(..).is_some()`: the predicate and the value are one function,
    /// and only the VALUE carries the distance [`visible_air`] cuts every
    /// ring against.
    ///
    /// The property is geometric, not a restatement of the arithmetic: for a
    /// segment that begins outside the box, the point at `t` lies ON the
    /// box's boundary, and the point a hair before `t` is outside it. That
    /// is what "entry" means, and nothing weaker distinguishes `t` from
    /// `2 * t`.
    #[test]
    fn the_reported_entry_is_where_the_segment_meets_the_box() {
        // built the way the shipped table builds walls: from a centerline,
        // inflated by WALL_T and shrunk by RECT_SHRINK. The box is read back
        // off the occluder rather than restated here — the property under
        // test is where `entry` lands, not what the constructor built.
        let wall = Occluder::new(Vector4::new(3.0, -5.0, 3.0, 5.0), 0.0, 3.0).unwrap();
        let (rect, span) = (wall.rect(), wall.span());
        let lo = [rect.x, span.x, rect.y];
        let hi = [rect.z, span.y, rect.w];
        // world-unit slack. f32 over coordinates of order 10 carries about
        // 1e-6, and the step back below moves 0.02 m along x, so 1e-3
        // separates "on the face" from "through it" by two orders of
        // magnitude at both ends.
        let skin = 1.0e-3_f32;
        let on_boundary = |p: [f32; 3]| {
            let within = (0..3).all(|k| p[k] >= lo[k] - skin && p[k] <= hi[k] + skin);
            let touching =
                (0..3).any(|k| (p[k] - lo[k]).abs() < skin || (p[k] - hi[k]).abs() < skin);
            within && touching
        };

        let mut entered = 0;
        for i in 0..40 {
            let z_from = f32::from(i as i16) * 0.3 - 6.0;
            for j in 0..40 {
                // y and z both sweep, so lines leave through the vertical
                // slab as well as the horizontal ones and every axis of the
                // fold is exercised
                let z_to = f32::from(j as i16) * 0.3 - 6.0;
                let y_to = f32::from(j as i16) * 0.2 - 1.0;
                let from = Vector3::new(-2.0, 1.5, z_from);
                let to = Vector3::new(8.0, y_to, z_to);
                // a segment born inside reports the GRAZE_EPS clamp rather
                // than a boundary — a different law, owned by the birth-wall
                // cases
                if contains(wall, from) {
                    continue;
                }
                let Some(t) = entry(from, to, wall) else {
                    continue;
                };
                assert!(
                    t.is_finite() && (0.0..=1.0).contains(&t),
                    "t left the segment: {t}"
                );
                let at = |s: f32| {
                    [
                        from.x + s * (to.x - from.x),
                        from.y + s * (to.y - from.y),
                        from.z + s * (to.z - from.z),
                    ]
                };
                assert!(
                    on_boundary(at(t)),
                    "t = {t} is not on the wall's surface: {:?} for {from:?} -> {to:?}",
                    at(t)
                );
                let back = at(t - 2.0e-3);
                assert!(
                    !contains(wall, Vector3::new(back[0], back[1], back[2])),
                    "the segment was already inside before t = {t}: {back:?}"
                );
                entered += 1;
            }
        }
        // a sweep that entered nothing would pass vacuously. This is a floor
        // on coverage, not a count of the loop: it does not move when the
        // loop bounds do.
        assert!(
            entered > 200,
            "only {entered} of the sampled lines entered the wall"
        );
    }

    /// THE BREAK: an unwalled sight line reporting anything other than its
    /// own whole length, which would cut every ring short in open air.
    #[test]
    fn open_air_is_the_whole_segment() {
        let eye = Vector3::new(0.0, 1.5, 0.0);
        let src = Vector3::new(6.0, 1.5, 0.0);
        assert!((visible_air(eye, src, &[]) - 6.0).abs() < 1e-6);
        let elsewhere = Occluder::new(Vector4::new(3.0, 20.0, 3.0, 25.0), 0.0, 3.0).unwrap();
        assert!((visible_air(eye, src, &[elsewhere]) - 6.0).abs() < 1e-6);
    }
}
