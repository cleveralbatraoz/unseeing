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

use crate::oid_palette::Box3;

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

/// Whether the level DRAWS one of the two slabs it built. The pair is
/// always BUILT — the level keeps floor and ceiling as one ordered pair,
/// and everything that reads it (the extents knob, the object-id anchors,
/// the seam census) must find the same two slabs at edit time as at run
/// time. Only the drawing bends, and only one way: a lid spanning the
/// whole extents is one opaque quad over the entire map, and the view it
/// covers — straight down — is the one a designer lays a plan out in. So
/// the ceiling is not drawn in the editor. The floor is: it is the ground
/// plane every wall and prop is placed against.
///
/// Nothing about this reaches the running game, where both slabs draw:
/// the hero's world is a closed room, and its lid is where the ceiling
/// reflections and the 0.9 seam come from.
#[must_use]
pub fn slab_drawn(lid: bool, editor_hint: bool) -> bool {
    !(lid && editor_hint)
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

/// The one name a level's spawn marker may carry.
pub const SPAWN_NAME: &str = "SpawnPoint";

/// How a `Marker3D`'s name reads against the spawn contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnName {
    /// Exactly [`SPAWN_NAME`] — the only name that can wake the hero.
    Exact,
    /// [`SPAWN_NAME`] with digits after it: what Ctrl+D leaves behind when
    /// a designer duplicates the marker, since Godot renames a copy whose
    /// name a sibling already holds.
    Numbered,
}

/// One `Marker3D` the level walk recognised as meant for the spawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCandidate {
    /// Where a designer can find the node: its path under the level root.
    /// A NAME cannot do this job — two markers named exactly `SpawnPoint`
    /// under different parents are legal, and are precisely the pair a
    /// report has to tell apart.
    pub path: String,
    /// How that node's own name reads.
    pub kind: SpawnName,
}

/// Which spawn marker the hero wakes at, and everything a designer has to
/// be told about the ones that did not win.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpawnVerdict {
    /// Index into the candidate slice of the marker the hero wakes at, or
    /// `None` when nothing carries the exact name and the caller must fall
    /// back to the level's own origin.
    pub winner: Option<usize>,
    /// One printable line per thing that is wrong, in a fixed order.
    pub complaints: Vec<String>,
}

/// How a node's name reads against the spawn contract — `None` for a name
/// that has nothing to do with the spawn, so an unrelated `Marker3D` never
/// enters a report about one.
#[must_use]
pub fn spawn_name(name: &str) -> Option<SpawnName> {
    let tail = name.strip_prefix(SPAWN_NAME)?;
    if tail.is_empty() {
        return Some(SpawnName::Exact);
    }
    tail.chars()
        .all(|c| c.is_ascii_digit())
        .then_some(SpawnName::Numbered)
}

/// Choose the spawn marker, and say out loud what was ignored.
///
/// THE CHOICE IS DELIBERATELY UNCHANGED: the first candidate named exactly
/// [`SPAWN_NAME`], in the level walk's depth-first scene order, wins —
/// which is what `Option::get_or_insert` did before this function existed,
/// so no scene that is valid today wakes its hero anywhere new. Everything
/// else here is diagnosis:
///
/// - a second marker named exactly `SpawnPoint` — legal in Godot under a
///   different parent — is reported as ignored, never promoted;
/// - `SpawnPoint2`, `SpawnPoint3`: what Ctrl+D leaves behind. Reported as
///   ignored copies and never promoted either, because a marker that won by
///   being the newest would move the hero the moment anyone duplicated one;
/// - no exact marker at all is the loudest case, because the caller's
///   fallback is the level's own ORIGIN, and on a bordered map that is the
///   corner sliver outside the border walls rather than anywhere playable.
#[must_use]
pub fn choose_spawn(candidates: &[SpawnCandidate], fallback: Vector3) -> SpawnVerdict {
    let exact: Vec<&SpawnCandidate> = candidates
        .iter()
        .filter(|c| c.kind == SpawnName::Exact)
        .collect();
    let winner = candidates.iter().position(|c| c.kind == SpawnName::Exact);
    let mut complaints = Vec::new();
    match exact.split_first() {
        None => complaints.push(format!(
            "WaveLevel: no Marker3D named exactly '{SPAWN_NAME}' under the level — the hero has \
             nowhere to wake, so it wakes at the level's own origin, {fallback}. That is the \
             corner of the map, outside the border walls: the hero is very likely sealed into \
             the sliver there and cannot reach the level at all. Add a Marker3D named \
             '{SPAWN_NAME}', standing on the floor, facing where the hero should look."
        )),
        Some((won, ignored)) if !ignored.is_empty() => complaints.push(format!(
            "WaveLevel: {} markers are named exactly '{SPAWN_NAME}' — the hero wakes at the \
             first the level walk reaches, '{}', and ignores {}. Delete or rename every spawn \
             marker but one.",
            exact.len(),
            won.path,
            quoted_paths(ignored.iter().copied()),
        )),
        Some(_) => {}
    }
    let copies: Vec<&SpawnCandidate> = candidates
        .iter()
        .filter(|c| c.kind == SpawnName::Numbered)
        .collect();
    if !copies.is_empty() {
        complaints.push(format!(
            "WaveLevel: auto-numbered spawn copies IGNORED: {}. Only a Marker3D named exactly \
             '{SPAWN_NAME}' wakes the hero, and Ctrl+D renames the copy — so moving the copy \
             moves nothing. Rename the one you want to '{SPAWN_NAME}' and delete the rest.",
            quoted_paths(copies.into_iter()),
        ));
    }
    SpawnVerdict { winner, complaints }
}

/// Candidate paths as a report quotes them: `'a', 'b'`, in the order given
/// — which is the level walk's scene order, so the same scene always reads
/// back the same sentence.
fn quoted_paths<'a>(paths: impl Iterator<Item = &'a SpawnCandidate>) -> String {
    paths
        .map(|c| format!("'{}'", c.path))
        .collect::<Vec<_>>()
        .join(", ")
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

/// One sound source as the demo-tap planner sees it — everything the
/// choice needs and nothing the engine layer would have to reach for.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceAim {
    /// The node's name: the tie-break, and what a complaint calls it.
    pub name: String,
    /// Where its waves are born, in world space.
    pub hub: Vector3,
}

/// What the level should do about the dev demo tap: the strike when there
/// is one, and the word a designer needs when there is not.
#[derive(Debug, Clone, PartialEq)]
pub struct TapVerdict {
    /// The planned strike, or `None` when no wall stands in the way.
    pub plan: Option<TapPlan>,
    /// What must be said out loud, or `None` when there is nothing to say
    /// — a level with no source at all is a legal authored state.
    pub complaint: Option<String>,
}

/// Which source the dev demo tap strikes toward: **the nearest hub to the
/// spawn, measured in the XZ plane, ties broken by the node's name in
/// ascending order**. `None` for a silent level.
///
/// Neither half of that rule is arbitrary.
///
/// SCENE ORDER IS NOT A CONTRACT. The target used to be whichever source
/// the level walk reached first, so dragging a row in the Scene dock — an
/// ordinary act in a 129-sibling tree — re-aimed the tap with no message.
/// Distance to the spawn is a property of the authored WORLD, and moving a
/// source is exactly the edit that should move the tap.
///
/// THE PLANE IS THE MEASURE. The tap is a wall crossing read in XZ, and the
/// height a speaker cone hangs at is not distance the hero walks; a 3-D
/// metric would push a radio mounted high on a near wall behind a fan
/// standing further away on the floor.
///
/// THE TIE-BREAK IS THE NAME. A slice index is scene order wearing a
/// different hat, and the determinism law forbids anything hashed; names
/// compare by bytes, identically on x86_64, arm64 and wasm.
#[must_use]
pub fn nearest_source(sources: &[SourceAim], spawn: Vector3) -> Option<usize> {
    sources
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            planar_reach(a.hub, spawn)
                .total_cmp(&planar_reach(b.hub, spawn))
                .then_with(|| a.name.cmp(&b.name))
        })
        .map(|(slot, _)| slot)
}

/// Squared distance between two points with their heights thrown away —
/// squared because only the ORDER is ever asked for, and a square root
/// would only add float dust to a comparison.
#[must_use]
fn planar_reach(a: Vector3, b: Vector3) -> f32 {
    let (dx, dz) = (a.x - b.x, a.z - b.z);
    dx * dx + dz * dz
}

/// The whole demo-tap decision, in one call: choose the source
/// ([`nearest_source`]), plan the strike ([`demo_tap`]), and say what could
/// not be planned.
///
/// The unplannable case is a SILENT WRONG RESULT without this: the caller
/// keeps its zeroed tap and the opening strike of an input-less run fires
/// at the world origin, which is under the floor in the corner of the map.
/// A source standing in the spawn's own room reaches it in one drag.
#[must_use]
pub fn plan_demo_tap(walls: &[Vector4], spawn: Vector3, sources: &[SourceAim]) -> TapVerdict {
    let Some(source) = nearest_source(sources, spawn).and_then(|slot| sources.get(slot)) else {
        // a silent level is legal: nothing to strike toward, nothing to say
        return TapVerdict {
            plan: None,
            complaint: None,
        };
    };
    let plan = demo_tap(walls, spawn, source.hub);
    let complaint = plan.is_none().then(|| {
        format!(
            "WaveLevel: no wall stands between the spawn at {spawn} and '{}', the sound source \
             nearest it, at {} — the dev demo tap cannot be planned and stays at the world \
             origin, where an input-less run (UNSEEING_DEMO=1, or ?demo in the URL) strikes \
             instead of on a wall.",
            source.name, source.hub,
        )
    });
    TapVerdict { plan, complaint }
}

/// One solid as a PLACEMENT law sees it: where a designer finds the node,
/// and the world box its drawn geometry actually fills.
///
/// The path does the job a name cannot — two crates called `Crate` under
/// different parents are legal, and are exactly the pair a report has to
/// tell apart — and it is the same handle [`SpawnCandidate`] quotes, so
/// every complaint a level prints points at a node the same way.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedSolid {
    /// Where a designer can find the node: its path under the level root.
    pub path: String,
    /// The world box its drawn geometry occupies.
    pub area: Box3,
}

/// Slack on a placement test, in meters. A shape resting exactly ON a
/// boundary — a wall's underside on the floor's top, a prop's face flush
/// with the last centimetre of the extents — has to read as on it and not
/// through it: carrying a local AABB into world space costs a few ULPs, and
/// a law that fired on float dust would be noise a designer learns to
/// ignore. Far below anything a hand places.
pub const PLACEMENT_EPS: f64 = 0.001;

/// One box's span along one axis — the only reading either placement law
/// takes off a [`Box3`], named so the arithmetic below says which axis it
/// is talking about.
#[must_use]
fn span(area: Box3, axis: usize) -> (f64, f64) {
    (area.min[axis % 3], area.max[axis % 3])
}

/// Does the span `a` lie inside the span `b`, float dust allowed?
#[must_use]
fn span_within(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 >= b.0 - PLACEMENT_EPS && a.1 <= b.1 + PLACEMENT_EPS
}

/// Do the spans share more than float dust? Touching end to end is not
/// overlapping: a crate whose face meets the floor's edge stands beside the
/// floor, not on it.
#[must_use]
fn span_overlaps(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.1 - PLACEMENT_EPS && b.0 < a.1 - PLACEMENT_EPS
}

/// A box's ground footprint as a report quotes it.
#[must_use]
fn ground_span(area: Box3) -> String {
    format!(
        "x {:.2}..{:.2}, z {:.2}..{:.2}",
        area.min[0], area.max[0], area.min[2], area.max[2]
    )
}

/// Every solid the level's floor does not reach, said out loud — one line
/// per node, in the order the level walk found them.
///
/// THE FLOOR NEVER MOVES TO MEET THE GEOMETRY. A level's slabs span
/// `0 .. extents` from its own origin and nothing else, so a solid dragged
/// to a negative coordinate — or left behind when the extents shrank — has
/// no slab under it. Not one system notices: the box draws, the waves
/// outline it, the colouring paints it, and the hero who walks there falls
/// with gravity and nothing underfoot. Growing the slabs to fit would be
/// the worse fix, because it silently changes the footprint of an authored
/// map; the fix is to SAY SO.
///
/// Two degrees, because the consequences differ. A footprint that misses
/// the floor entirely has nothing under any of it. One that only hangs over
/// the edge is supported for most of its width and is one drag from
/// correct — telling that designer the hero falls through their crate would
/// be telling them something untrue.
#[must_use]
pub fn unfloored(floor: Box3, solids: &[PlacedSolid]) -> Vec<String> {
    let (x, z) = (span(floor, 0), span(floor, 2));
    let mut complaints = Vec::new();
    for solid in solids {
        let (their_x, their_z) = (span(solid.area, 0), span(solid.area, 2));
        if span_within(their_x, x) && span_within(their_z, z) {
            continue; // the whole footprint has a slab under it
        }
        let theirs = ground_span(solid.area);
        let ours = ground_span(floor);
        complaints.push(if span_overlaps(their_x, x) && span_overlaps(their_z, z) {
            format!(
                "WaveLevel: '{}' hangs over the edge of the floor — its footprint is {theirs}, \
                 and the floor covers {ours}. The part outside has no slab under it. Move it \
                 inside the extents, or grow the level's extents to cover it — the slabs span \
                 0..extents from the level's own origin and never move to meet stray geometry.",
                solid.path,
            )
        } else {
            format!(
                "WaveLevel: '{}' stands off the floor entirely — its footprint is {theirs}, and \
                 the floor covers {ours}. There is no slab under any of it: it draws where \
                 nothing holds it up, and the hero who walks there falls out of the world. Move \
                 it inside the extents, or grow the level's extents to cover it — the slabs \
                 span 0..extents from the level's own origin and never move to meet stray \
                 geometry.",
                solid.path,
            )
        });
    }
    complaints
}

#[cfg(test)]
mod tests {
    use godot::builtin::EulerOrder;

    use super::*;

    /// One solid at a named path filling one world box — the shape the
    /// level hands a placement law after walking its subtree.
    fn placed(path: &str, min: [f64; 3], max: [f64; 3]) -> PlacedSolid {
        PlacedSolid {
            path: path.to_string(),
            area: Box3 { min, max },
        }
    }

    /// The floor a 20 x 20 level builds for itself: spanning 0..extents
    /// from the level's own origin, its TOP exactly at y = 0.
    fn floor_20x20() -> Box3 {
        Box3 {
            min: [0.0, -0.1, 0.0],
            max: [20.0, 0.0, 20.0],
        }
    }

    /// The ordinary map: every solid inside the extents, and the level says
    /// nothing at all. A law that complained here would train a designer to
    /// ignore it. The second half is the one that catches a footprint
    /// hardcoded to 0..extents: the floor is read WHERE IT STANDS, so a
    /// level dropped at (100, 0, 100) carries its own footprint with it and
    /// the crate beside it is home — while that same crate measured against
    /// a level at the origin is a stray.
    #[test]
    fn a_solid_inside_the_extents_says_nothing() {
        let inside = [
            placed("Crate", [4.0, 0.0, 4.0], [5.0, 1.0, 5.0]),
            placed("Rooms/Barrel", [0.0, 0.0, 0.0], [0.6, 0.9, 0.6]),
            placed("EdgeWall", [19.7, 0.0, 4.0], [20.0, 3.0, 8.0]),
        ];
        let quiet = unfloored(floor_20x20(), &inside);
        assert!(quiet.is_empty(), "{quiet:?}");
        let moved = Box3 {
            min: [100.0, -0.1, 100.0],
            max: [120.0, 0.0, 120.0],
        };
        let there = [placed("Crate", [104.0, 0.0, 104.0], [105.0, 1.0, 105.0])];
        let quiet = unfloored(moved, &there);
        assert!(quiet.is_empty(), "{quiet:?}");
        assert_eq!(unfloored(floor_20x20(), &there).len(), 1);
    }

    /// THE reproduce case: geometry authored at negative coordinates. The
    /// slabs span 0..extents from the level's origin and never grow to meet
    /// it, so there is no floor under any of it and the hero falls — and
    /// before this, not one message was emitted. The complaint names the
    /// node by path and quotes BOTH footprints, because "outside the
    /// extents" without the two spans leaves a designer guessing which way
    /// to drag.
    #[test]
    fn a_solid_at_negative_coordinates_is_named_with_the_floor_it_missed() {
        let complaints = unfloored(
            floor_20x20(),
            &[placed("StrayCrate", [-10.5, 0.0, -10.5], [-9.5, 1.0, -9.5])],
        );
        assert_eq!(
            complaints,
            vec![
                "WaveLevel: 'StrayCrate' stands off the floor entirely — its footprint is x \
                 -10.50..-9.50, z -10.50..-9.50, and the floor covers x 0.00..20.00, z \
                 0.00..20.00. There is no slab under any of it: it draws where nothing holds \
                 it up, and the hero who walks there falls out of the world. Move it inside \
                 the extents, or grow the level's extents to cover it — the slabs span \
                 0..extents from the level's own origin and never move to meet stray geometry."
            ]
        );
    }

    /// The other end of the same fault, and the one a designer reaches by
    /// shrinking the extents rather than by dragging a crate: a solid past
    /// the far edge. It reads as the same degree — nothing under any of it
    /// — so the two ends cannot diverge into two half-written laws.
    #[test]
    fn a_solid_beyond_the_extents_is_off_the_floor_too() {
        let complaints = unfloored(
            floor_20x20(),
            &[placed("FarCrate", [24.0, 0.0, 24.0], [25.0, 1.0, 25.0])],
        );
        assert_eq!(complaints.len(), 1);
        assert!(
            complaints[0].starts_with(
                "WaveLevel: 'FarCrate' stands off the floor entirely — its footprint is x \
                 24.00..25.00, z 24.00..25.00, and the floor covers x 0.00..20.00, z \
                 0.00..20.00."
            ),
            "{}",
            complaints[0]
        );
    }

    /// A solid HANGING over the edge is a milder fault than one standing
    /// off the floor entirely — most of it is supported and one drag fixes
    /// it — so it gets its own sentence. Collapsing the two would tell a
    /// designer the hero falls through a crate that is 90% on the floor.
    #[test]
    fn a_solid_straddling_the_edge_reads_as_an_overhang() {
        let complaints = unfloored(
            floor_20x20(),
            &[placed("LedgeCrate", [19.5, 0.0, 4.0], [20.5, 1.0, 5.0])],
        );
        assert_eq!(
            complaints,
            vec![
                "WaveLevel: 'LedgeCrate' hangs over the edge of the floor — its footprint is x \
                 19.50..20.50, z 4.00..5.00, and the floor covers x 0.00..20.00, z \
                 0.00..20.00. The part outside has no slab under it. Move it inside the \
                 extents, or grow the level's extents to cover it — the slabs span 0..extents \
                 from the level's own origin and never move to meet stray geometry."
            ]
        );
    }

    /// Flush with the edge is ON the floor, not over it. Carrying a local
    /// AABB into world space costs a few ULPs, and a law that fired on
    /// float dust would be noise a designer learns to ignore — so a solid
    /// standing a hair past the edge is silent, and one standing a
    /// centimetre past it is not.
    #[test]
    fn float_dust_at_the_edge_is_not_an_overhang() {
        let dust = [placed("Flush", [-0.0004, 0.0, 4.0], [20.0004, 3.0, 8.0])];
        let quiet = unfloored(floor_20x20(), &dust);
        assert!(quiet.is_empty(), "{quiet:?}");
        let real = [placed("Proud", [-0.01, 0.0, 4.0], [20.0, 3.0, 8.0])];
        assert_eq!(unfloored(floor_20x20(), &real).len(), 1);
    }

    /// Every stray is reported, in the order the level walk found them —
    /// scene order, the deterministic order every other derivation leans
    /// on. A report that stopped at the first would send a designer around
    /// the loop once per misplaced crate.
    #[test]
    fn every_stray_is_reported_in_walk_order() {
        let complaints = unfloored(
            floor_20x20(),
            &[
                placed("Home", [4.0, 0.0, 4.0], [5.0, 1.0, 5.0]),
                placed("Second", [-3.0, 0.0, 4.0], [-2.0, 1.0, 5.0]),
                placed("Third", [19.5, 0.0, 4.0], [20.5, 1.0, 5.0]),
                placed("Fourth", [4.0, 0.0, 30.0], [5.0, 1.0, 31.0]),
            ],
        );
        assert_eq!(complaints.len(), 3);
        assert!(complaints[0].contains("'Second'"), "{}", complaints[0]);
        assert!(complaints[1].contains("'Third'"), "{}", complaints[1]);
        assert!(complaints[2].contains("'Fourth'"), "{}", complaints[2]);
    }

    /// A marker a designer named by hand, on purpose.
    fn exact(path: &str) -> SpawnCandidate {
        SpawnCandidate {
            path: path.to_string(),
            kind: SpawnName::Exact,
        }
    }

    /// A marker Ctrl+D named, by leaving a number on the end.
    fn numbered(path: &str) -> SpawnCandidate {
        SpawnCandidate {
            path: path.to_string(),
            kind: SpawnName::Numbered,
        }
    }

    /// The level's own origin lifted to capsule height on the shipped map —
    /// the corner the hero is dumped in when no marker names itself.
    const CORNER: Vector3 = Vector3::new(0.0, 0.9, 0.0);

    /// The name is the only trace a duplicated marker leaves: exactly
    /// `SpawnPoint` is the spawn, `SpawnPoint` plus digits is what Ctrl+D
    /// wrote, and anything else is a `Marker3D` that means something else
    /// entirely and must not be dragged into the report.
    #[test]
    fn a_spawn_name_reads_as_exact_or_as_an_auto_numbered_copy() {
        assert_eq!(spawn_name("SpawnPoint"), Some(SpawnName::Exact));
        assert_eq!(spawn_name("SpawnPoint2"), Some(SpawnName::Numbered));
        assert_eq!(spawn_name("SpawnPoint17"), Some(SpawnName::Numbered));
        assert_eq!(spawn_name("SpawnPointB"), None);
        assert_eq!(spawn_name("SpawnPoint 2"), None);
        assert_eq!(spawn_name("spawnpoint"), None);
        assert_eq!(spawn_name("Spawn"), None);
        assert_eq!(spawn_name("CameraTarget"), None);
        assert_eq!(spawn_name(""), None);
    }

    /// The ordinary map: one marker wins and the level says nothing at all.
    /// A rule that complained here would train a designer to ignore it.
    #[test]
    fn one_spawn_marker_wins_in_silence() {
        let verdict = choose_spawn(&[exact("SpawnPoint")], CORNER);
        assert_eq!(verdict.winner, Some(0));
        assert!(verdict.complaints.is_empty(), "{:?}", verdict.complaints);
    }

    /// Two markers named EXACTLY `SpawnPoint` is legal in Godot under two
    /// different parents, and used to be settled in silence by whichever
    /// the walk reached first. The first still wins — no scene that is
    /// valid today moves its hero — but the loser is NAMED by path, which
    /// is the only thing that tells two identically named nodes apart.
    #[test]
    fn a_second_exact_marker_loses_and_is_named_by_path() {
        let verdict = choose_spawn(&[exact("SpawnPoint"), exact("Rooms/SpawnPoint")], CORNER);
        assert_eq!(verdict.winner, Some(0));
        assert_eq!(
            verdict.complaints,
            vec![
                "WaveLevel: 2 markers are named exactly 'SpawnPoint' — the hero wakes at the \
                 first the level walk reaches, 'SpawnPoint', and ignores 'Rooms/SpawnPoint'. \
                 Delete or rename every spawn marker but one."
            ]
        );
    }

    /// THE Ctrl+D case: duplicate the marker, drag the copy, press play,
    /// and the hero wakes at the original. The copy is reported as ignored
    /// and never promoted — a marker that won by being the newest would
    /// move the hero the moment anyone copied one to measure a distance.
    #[test]
    fn an_auto_numbered_copy_is_reported_and_never_promoted() {
        let verdict = choose_spawn(&[exact("SpawnPoint"), numbered("SpawnPoint2")], CORNER);
        assert_eq!(verdict.winner, Some(0));
        assert_eq!(
            verdict.complaints,
            vec![
                "WaveLevel: auto-numbered spawn copies IGNORED: 'SpawnPoint2'. Only a Marker3D \
                 named exactly 'SpawnPoint' wakes the hero, and Ctrl+D renames the copy — so \
                 moving the copy moves nothing. Rename the one you want to 'SpawnPoint' and \
                 delete the rest."
            ]
        );
        // and the copy does not win by standing earlier in the walk either:
        // a level whose only exact marker sits under a folder still wakes
        // its hero there, not at whatever Ctrl+D dropped at the top
        let reordered = choose_spawn(
            &[numbered("SpawnPoint2"), exact("Rooms/SpawnPoint")],
            CORNER,
        );
        assert_eq!(reordered.winner, Some(1));
    }

    /// Copies are listed in walk order, all of them, so a designer who
    /// duplicated twice is not told about one and left to find the other.
    #[test]
    fn every_auto_numbered_copy_is_listed() {
        let verdict = choose_spawn(
            &[
                exact("SpawnPoint"),
                numbered("SpawnPoint2"),
                numbered("Rooms/SpawnPoint3"),
            ],
            CORNER,
        );
        assert_eq!(verdict.winner, Some(0));
        assert_eq!(verdict.complaints.len(), 1);
        assert!(
            verdict.complaints[0].contains("'SpawnPoint2', 'Rooms/SpawnPoint3'"),
            "{}",
            verdict.complaints[0]
        );
    }

    /// The worst case a designer can reach by accident: duplicate the
    /// marker, delete the original, and nothing carries the exact name any
    /// more. Both facts are said — the copy that could have been the spawn,
    /// and where the hero was actually put — because the fallback is the
    /// level's own origin, which on any bordered map is the sliver outside
    /// the border walls.
    #[test]
    fn no_exact_marker_names_the_fallback_and_the_copy_that_was_not_promoted() {
        let verdict = choose_spawn(&[numbered("SpawnPoint2")], CORNER);
        assert_eq!(verdict.winner, None);
        assert_eq!(
            verdict.complaints,
            vec![
                "WaveLevel: no Marker3D named exactly 'SpawnPoint' under the level — the hero \
                 has nowhere to wake, so it wakes at the level's own origin, (0, 0.9, 0). That \
                 is the corner of the map, outside the border walls: the hero is very likely \
                 sealed into the sliver there and cannot reach the level at all. Add a Marker3D \
                 named 'SpawnPoint', standing on the floor, facing where the hero should look."
                    .to_string(),
                "WaveLevel: auto-numbered spawn copies IGNORED: 'SpawnPoint2'. Only a Marker3D \
                 named exactly 'SpawnPoint' wakes the hero, and Ctrl+D renames the copy — so \
                 moving the copy moves nothing. Rename the one you want to 'SpawnPoint' and \
                 delete the rest."
                    .to_string(),
            ]
        );
    }

    /// An empty level reports the fallback it was actually given, not a
    /// hardcoded origin: a level dropped at (14, 0, 14) puts its hero
    /// there, and a message quoting (0, 0.9, 0) would send the designer
    /// looking in the wrong corner.
    #[test]
    fn the_missing_spawn_message_quotes_the_fallback_it_was_given() {
        let verdict = choose_spawn(&[], Vector3::new(14.0, 0.9, 2.5));
        assert_eq!(verdict.winner, None);
        assert_eq!(verdict.complaints.len(), 1);
        assert!(
            verdict.complaints[0].contains("(14, 0.9, 2.5)"),
            "{}",
            verdict.complaints[0]
        );
    }

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

    /// The map is laid out from above, and a lid spanning the whole
    /// extents is one opaque quad across that entire view — no walls, no
    /// props, nothing to place against. So the ceiling is not drawn at
    /// edit time. The floor is: it is the ground plane every wall and prop
    /// is dragged onto, and dropping it would trade one blind view for
    /// another.
    #[test]
    fn the_editor_draws_the_floor_but_not_the_lid() {
        assert!(slab_drawn(false, true), "the editor lost its ground plane");
        assert!(!slab_drawn(true, true), "the editor drew the lid");
    }

    /// The other half, and the one that keeps the fix honest: the hero's
    /// world is a closed room, so at run time BOTH slabs draw. Deleting
    /// the ceiling outright — or carrying the editor's rule into the game
    /// — satisfies the test above and opens the level to the sky, where
    /// the ceiling reflections and the 0.9 lid seam come from.
    #[test]
    fn the_running_game_draws_both_slabs() {
        assert!(slab_drawn(false, false), "the game lost its floor");
        assert!(slab_drawn(true, false), "the game lost its ceiling");
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

    /// A source at a named place, the way the tap planner is handed one.
    fn aim(name: &str, x: f32, y: f32, z: f32) -> SourceAim {
        SourceAim {
            name: name.to_string(),
            hub: Vector3::new(x, y, z),
        }
    }

    /// THE order-independence law: reordering the Scene dock used to
    /// re-aim the demo tap in silence, because the target was whichever
    /// source the walk reached first. The nearest hub wins instead, so the
    /// same scene aims the same way however its 129 siblings are listed.
    #[test]
    fn the_nearest_source_wins_wherever_it_sits_in_the_dock() {
        let spawn = Vector3::new(0.0, 0.9, 0.0);
        let far = aim("Radio", 20.0, 1.0, 0.0);
        let near = aim("Fan", 4.0, 1.0, 0.0);
        assert_eq!(nearest_source(&[far.clone(), near.clone()], spawn), Some(1));
        assert_eq!(nearest_source(&[near, far], spawn), Some(0));
    }

    /// Distance is measured in the XZ PLANE, and that is a decision rather
    /// than a shortcut: the tap is a wall CROSSING read in that plane, and
    /// the height a speaker cone hangs at is not distance the hero walks.
    /// A radio mounted high three metres away is nearer than a fan on the
    /// floor five metres away — a 3-D metric would pick the fan.
    #[test]
    fn height_does_not_decide_which_source_the_tap_aims_at() {
        let spawn = Vector3::new(0.0, 0.9, 0.0);
        let low_and_far = aim("Fan", 5.0, 0.1, 0.0);
        let high_and_near = aim("Radio", 3.0, 9.0, 0.0);
        assert_eq!(
            nearest_source(&[low_and_far, high_and_near], spawn),
            Some(1)
        );
    }

    /// An exact tie is broken by the NAME, ascending — never by position
    /// in the slice, which is the scene order this whole rule exists to
    /// stop depending on, and never by anything hashed, which the
    /// determinism law forbids outright. Two sources equidistant from the
    /// spawn therefore aim the tap the same way in either dock order.
    #[test]
    fn an_exact_tie_is_broken_by_name_not_by_scene_order() {
        let spawn = Vector3::new(0.0, 0.9, 0.0);
        let alpha = aim("Alpha", 3.0, 1.0, 4.0); // 5 m out
        let zulu = aim("Zulu", -3.0, 1.0, -4.0); // 5 m out the other way
        assert_eq!(
            nearest_source(&[zulu.clone(), alpha.clone()], spawn),
            Some(1)
        );
        assert_eq!(nearest_source(&[alpha, zulu], spawn), Some(0));
    }

    /// A silent level has nothing to aim at, and that is legal.
    #[test]
    fn a_silent_level_aims_at_nothing() {
        assert_eq!(nearest_source(&[], Vector3::ZERO), None);
    }

    /// The shipped map, measured off the literals below: the fan's hub is
    /// 5.7 m from the spawn in the XZ plane (dx 5.7, dz 0.4) and the
    /// radio's is 18.5 (dx 18.5, dz −0.71), so the nearest-hub rule aims at
    /// the same fan the old first-in-scene-order rule did. That is the
    /// whole reason this fix is invisible to `level_01.tscn` — and it stays
    /// invisible with the dock listed the other way round.
    ///
    /// The hubs are not the node positions: the fan's is its node at
    /// (8.6, 0, 4.4) plus the spinner's local (0, HEAD_H, −0.10) carried
    /// through the node's quarter turn, and the radio's is its node at
    /// (21.4, 0.78, 3.4) times the same basis applied to HUB.
    #[test]
    fn the_shipped_map_still_aims_the_tap_at_the_fan() {
        let spawn = Vector3::new(3.0, 0.9, 4.0);
        let fan = aim("Fan", 8.7, 1.15, 4.4);
        let radio = aim("Radio", 21.5, 0.92, 3.29);
        assert_eq!(
            nearest_source(&[fan.clone(), radio.clone()], spawn),
            Some(0)
        );
        assert_eq!(nearest_source(&[radio, fan], spawn), Some(1));
    }

    /// The whole tap decision in one call: pick the source, plan the
    /// strike, say nothing when there is nothing to say.
    #[test]
    fn a_plannable_layout_yields_the_plan_and_no_complaint() {
        let verdict = plan_demo_tap(
            &retired_map_walls(),
            Vector3::new(3.0, 0.9, 4.0),
            &[aim("Fan", 8.6, 1.15, 4.4)],
        );
        assert_eq!(
            verdict.plan.expect("tap plans").point,
            Vector3::new(6.25, 0.8, 4.0)
        );
        assert_eq!(verdict.complaint, None);
    }

    /// The order-independence law all the way through the planner, not
    /// only through the choice: the fan is listed FIRST and is the one the
    /// retired rule struck the divider toward, but a radio three metres
    /// west of the spawn is nearer — so the tap lands on the west border's
    /// inward face instead, and the dock's order decided nothing.
    #[test]
    fn the_plan_follows_the_nearest_source_not_the_first_listed() {
        let walls = retired_map_walls();
        let spawn = Vector3::new(3.0, 0.9, 4.0);
        let fan = aim("Fan", 8.6, 1.15, 4.4); // 5.6 east, 5.61 out, past the divider
        let radio = aim("WestRadio", 0.0, 1.0, 4.0); // 3.0 m west, behind the border
        let plan = plan_demo_tap(&walls, spawn, &[fan.clone(), radio.clone()])
            .plan
            .expect("tap plans");
        assert_eq!(plan.point, Vector3::new(0.75, 0.8, 4.0));
        assert_eq!(plan.normal, Vector3::new(1.0, 0.0, 0.0));
        // and the retired rule's answer, for contrast: the fan alone puts
        // the strike on the divider's west face, five metres the other way
        let fan_only = plan_demo_tap(&walls, spawn, &[fan])
            .plan
            .expect("tap plans");
        assert_eq!(fan_only.point, Vector3::new(6.25, 0.8, 4.0));
    }

    /// A silent level plans nothing AND complains about nothing: a level
    /// with no source is a legal authored state, not a mistake.
    #[test]
    fn a_silent_level_plans_no_tap_and_says_nothing() {
        let verdict = plan_demo_tap(&retired_map_walls(), Vector3::new(3.0, 0.9, 4.0), &[]);
        assert_eq!(verdict.plan, None);
        assert_eq!(verdict.complaint, None);
    }

    /// The silent wrong result that issue #20 is really about: a source in
    /// the spawn's own room leaves nothing to strike, the caller keeps its
    /// zeroed tap, and the opening demo strike fires at the world origin.
    /// The plan is still `None` — there IS no wall — but the level now says
    /// which source it could not reach past and what happens instead.
    #[test]
    fn an_unplannable_layout_names_the_source_and_the_consequence() {
        let verdict = plan_demo_tap(
            &retired_map_walls(),
            Vector3::new(10.0, 0.9, 4.0),
            &[aim("Fan", 8.6, 1.15, 4.4)],
        );
        assert_eq!(verdict.plan, None);
        assert_eq!(
            verdict.complaint.as_deref(),
            Some(
                "WaveLevel: no wall stands between the spawn at (10, 0.9, 4) and 'Fan', the \
                 sound source nearest it, at (8.6, 1.15, 4.4) — the dev demo tap cannot be \
                 planned and stays at the world origin, where an input-less run \
                 (UNSEEING_DEMO=1, or ?demo in the URL) strikes instead of on a wall."
            )
        );
    }
}
