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
/// half-thickness on every side, floor to ceiling — flanks AND run ends
/// alike. A junction puts one wall's centerline end ON its partner's
/// centerline (every T and L in the shipped map), so a full [`WALL_T`] of
/// run padding lands the arriving cap exactly in the partner's far flank
/// plane: same-facing, coplanar, overlapping. That used to depth-fight
/// (issue 14) under the flat per-solid id this campaign replaced; under
/// the superface paint pass (`render::superface`) it is instead the
/// intended MERGE — the two faces share one label bit-for-bit, and the
/// only line left is the clean corner crease where the two walls actually
/// bend. (A five-millimetre cap-inset stopgap held the cap short of that
/// coincidence before the paint pass existed to make it safe; it is gone,
/// and `render::superface`'s own junction fixtures are its replacement.)
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

/// One thing a designer placed where the level cannot support it: the
/// node's address, held apart from the sentence a report prints about it.
///
/// `path` is the same handle [`PlacedSolid`] carries and [`SpawnCandidate`]
/// quotes, so a fault points at a node the same way every other complaint
/// does. `text` is the exact line [`unfloored`] or [`sunken`] would have
/// printed before this type existed — kept whole, not reworded, because the
/// boot gate and the shipped map's own suite pin its opening and phrasing.
/// Carrying `path` alongside it, rather than only inside it, lets a
/// consumer that needs to DECIDE something about the node — jump to it,
/// count it against one solid rather than one line — do that without
/// re-parsing a sentence meant for a person.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementFault {
    pub path: String,
    pub text: String,
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
pub fn unfloored(floor: Box3, solids: &[PlacedSolid]) -> Vec<PlacementFault> {
    let (x, z) = (span(floor, 0), span(floor, 2));
    let mut complaints = Vec::new();
    for solid in solids {
        let (their_x, their_z) = (span(solid.area, 0), span(solid.area, 2));
        if span_within(their_x, x) && span_within(their_z, z) {
            continue; // the whole footprint has a slab under it
        }
        let theirs = ground_span(solid.area);
        let ours = ground_span(floor);
        let text = if span_overlaps(their_x, x) && span_overlaps(their_z, z) {
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
        };
        complaints.push(PlacementFault {
            path: solid.path.clone(),
            text,
        });
    }
    complaints
}

/// Every solid that crosses or hides under the floor plane, said out loud
/// — one line per node, in the order the level walk found them. The plane
/// is the floor slab's TOP where it actually stands, never the world's
/// `y = 0`: a level lifted anywhere carries its own floor with it.
///
/// THE ORIGIN LAW IS RIGHT AND IS NOT WHAT IS BROKEN. A box prop is CENTRED
/// on its node because a shelf, a tabletop or a beam floats as often as it
/// stands, while a wall, a column and a wedge STAND on theirs because one
/// that is not resting on something is a mistake. What that costs is the
/// most natural authoring gesture there is: drop a `WaveProp` on the floor
/// plane, and exactly half of it is under the slab — where nothing draws,
/// nothing sounds and nothing can be walked into, and where no system
/// notices. So the law keeps its shapes and the level gains its voice.
///
/// Two degrees again. A box that STRADDLES the plane is half a room and
/// half nothing. A box entirely BELOW it is a node the designer thinks they
/// placed and did not — reported for the same reason, and for one more: the
/// obvious cure for a half-sunk warning is to push the node down until the
/// complaint stops, and a law that went quiet there would reward making the
/// fault total.
#[must_use]
pub fn sunken(floor: Box3, solids: &[PlacedSolid]) -> Vec<PlacementFault> {
    let top = floor.max[1];
    let mut complaints = Vec::new();
    for solid in solids {
        let (lo, hi) = span(solid.area, 1);
        if lo >= top - PLACEMENT_EPS {
            continue; // resting on the floor, or standing clear above it
        }
        let text = if hi > top + PLACEMENT_EPS {
            format!(
                "WaveLevel: '{}' is sunk through the floor — its box spans y {lo:.2}..{hi:.2}, \
                 and the floor's top is at y {top:.2}. What is under the slab never draws, \
                 never sounds and cannot be walked into. A WaveProp is CENTRED on its node, so \
                 dropping one on the floor plane buries exactly half of it, while a wall, a \
                 column and a wedge STAND on theirs. Lift the node until the whole shape clears \
                 y {top:.2}.",
                solid.path,
            )
        } else {
            format!(
                "WaveLevel: '{}' is buried under the floor — its box spans y {lo:.2}..{hi:.2}, \
                 entirely below the floor's top at y {top:.2}. Nothing under the slab draws, \
                 sounds or can be walked into, so the node is in the scene and not in the \
                 world. Lift it until the whole shape clears y {top:.2}.",
                solid.path,
            )
        };
        complaints.push(PlacementFault {
            path: solid.path.clone(),
            text,
        });
    }
    complaints
}

/// Wall segments one more room costs a designer: three new sides, plus the
/// doorway, which is the GAP between two segments and so costs a segment of
/// its own. The unit the wall budget speaks in — thirteen free slots is an
/// inventory number, "three more rooms" is a thing a designer can plan
/// around, and the report gives both.
pub const ROOM_SEGMENTS: usize = 4;

/// The range the sight shaders pack camera distance into — the CPU mirror
/// of `DIST_PACK_RANGE` in `game/shaders/pulse_pool.gdshaderinc`, whose
/// copy is the one that renders. Held to it by
/// `game/tests/shader_contract_test.gd`, exactly as [`HUM_THROUGH`] is.
///
/// It is a CEILING ON THE MAP, which is why the level checks itself against
/// it: `data_core.gdshaderinc` writes `clamp(vd / DIST_PACK_RANGE, 0, 1)`
/// into the data pass's B channel, and `hearing_post.gdshader` multiplies
/// it back to recover the scene depth every outline and every wave ring is
/// resolved against.
pub const DIST_PACK_RANGE: f64 = 40.0;

/// How loudly the level says something about itself.
///
/// The split is not decoration. An overflow means the drawn world is
/// ALREADY wrong — walls a designer placed have stopped occluding — while
/// a headroom warning means nothing is broken yet. Shouting both as errors
/// would teach a designer to scroll past the one that matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Nothing is broken; the level is running out of room.
    Warn,
    /// The world the shaders draw no longer matches the authored scene.
    Error,
}

/// One thing the level must say about a shader ceiling it is approaching or
/// has passed, and how loudly to say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    pub severity: Severity,
    pub text: String,
}

/// What the level must say about its wall count against the sight shaders'
/// occluder slots (`sight::MAXW`, mirrored as `MAXW` in
/// `game/shaders/pulse_pool.gdshaderinc`). `None` while the level has a
/// whole room's worth of segments to spare, which is the shipped map's
/// state and must stay silent.
///
/// THE REPORT IS ABOUT HEADROOM, not about today's wall count. A level that
/// outgrows the ceiling used to be discovered by an unrelated red
/// assertion — the frozen census of 19 walls failing first, which reads
/// like a bug in the census rather than a level that outgrew a shader
/// constant. So this names the constant, the slots left, and what a slot is
/// worth in rooms.
///
/// Total on any pair, the degenerate `slots == 0` included: the headroom is
/// a saturating subtraction, never a `usize` that wrapped past zero.
#[must_use]
pub fn wall_budget(walls: usize, slots: usize) -> Option<Budget> {
    if walls > slots {
        return Some(Budget {
            severity: Severity::Error,
            text: format!(
                "WaveLevel: {walls} walls exceed the sight shaders' {slots} slots — the table \
                 keeps the first {slots} and drops {}, which stop occluding entirely: waves pass \
                 straight through them and no sight line counts them. Delete or merge walls, or \
                 raise MAXW (rust/src/sight.rs, mirrored in \
                 game/shaders/pulse_pool.gdshaderinc) — a measured decision and not a free one: \
                 every wall is another rect in the per-fragment sight loop, on every platform.",
                walls - slots,
            ),
        });
    }
    let headroom = slots - walls;
    if headroom >= ROOM_SEGMENTS {
        return None; // room for another room: nothing worth saying
    }
    Some(Budget {
        severity: Severity::Warn,
        text: format!(
            "WaveLevel: {walls} walls against the sight shaders' {slots} slots — {headroom} \
             segments left, short of the {ROOM_SEGMENTS} another room costs (three sides plus the \
             doorway, which is the gap between two segments and so costs a segment of its own). \
             Every wall past the last slot silently stops occluding. Raising MAXW \
             (rust/src/sight.rs, mirrored in game/shaders/pulse_pool.gdshaderinc) is a measured \
             decision and not a free one: every wall is another rect in the per-fragment sight \
             loop, on every platform."
        ),
    })
}

/// The longest sight line the authored map allows: the diagonal of the
/// wall-centerline footprint, floor to ceiling.
///
/// The WALL CENTERLINES are the measure, and deliberately so — it is the
/// one `game/tests/shader_contract_test.gd` already holds DIST_PACK_RANGE
/// against, so the level's own report and that suite's assertion can never
/// describe the same map two different ways. It is also a slight
/// UNDERSTATEMENT: the floor and ceiling slabs span the whole `extents`
/// knob, which on the shipped map reaches about a metre past the border
/// walls' centerlines. That is why [`pack_range_budget`] refuses equality
/// rather than only excess.
///
/// Total on any table, the empty one included: a level with no walls has no
/// footprint, and answering with the difference of two infinities would
/// poison every comparison downstream.
#[must_use]
pub fn map_diagonal(segments: &[Vector4]) -> f64 {
    let Some(first) = segments.first() else {
        return 0.0; // no walls, no footprint, nothing to outgrow
    };
    let (mut lo_x, mut hi_x) = (first.x.min(first.z), first.x.max(first.z));
    let (mut lo_z, mut hi_z) = (first.y.min(first.w), first.y.max(first.w));
    for s in segments {
        lo_x = lo_x.min(s.x).min(s.z);
        hi_x = hi_x.max(s.x).max(s.z);
        lo_z = lo_z.min(s.y).min(s.w);
        hi_z = hi_z.max(s.y).max(s.w);
    }
    let (across, along) = (f64::from(hi_x - lo_x), f64::from(hi_z - lo_z));
    (across * across + WALL_H * WALL_H + along * along).sqrt()
}

/// What the level must say about its own size against the range the sight
/// shaders pack camera distance into ([`DIST_PACK_RANGE`]). `None` while
/// the range strictly exceeds the diagonal, which is the shipped map's
/// state — 38.02 m against 40 — and must stay silent.
///
/// WHAT ACTUALLY BREAKS, since the packed value does NOT alias: the data
/// core writes `clamp(vd / DIST_PACK_RANGE, 0, 1)` into B, so a point
/// beyond the range saturates rather than wrapping, and everything out
/// there reads the same flat 1.0. Three things follow from that flatness,
/// in the order a growing map meets them:
///
/// 1. The silhouette outline is a LAPLACIAN of B, and the Laplacian of a
///    plateau is zero. Far geometry simply stops drawing its outline — the
///    perception law's one line per object, gone. Creases survive, because
///    they are diffed out of the object-id channel instead.
/// 2. The hearing pass recovers scene depth as `c_c.b * DIST_PACK_RANGE`,
///    which pins at the range. A player-made ring is cut where it meets the
///    world, so past the range it is cut against a world that is not there
///    — the sound dies on an invisible sphere around the eye — and the
///    x-ray test that decides whether a surface is a source seen through a
///    wall probes the wrong point entirely.
/// 3. A source's acoustic-image depth is the always-on-top value minus a
///    hair proportional to `clamp(dist / DIST_PACK_RANGE, 0, 1)`, so two
///    sources past the range write the identical depth and resolve by
///    opaque draw order again — the exact collision that band exists to
///    prevent, where a far dim ghost punches through a near loud one.
///
/// EQUALITY ALREADY COUNTS. At `vd == range` the packed value is 1.0, the
/// top of the band and the first value indistinguishable from everything
/// past it; and [`map_diagonal`] understates the map, since the slabs reach
/// past the wall centerlines it measures. The existing shader contract
/// demands `range > diagonal` for the same reason, and the two must agree.
#[must_use]
pub fn pack_range_budget(diagonal: f64, range: f64) -> Option<Budget> {
    if diagonal < range {
        return None; // the whole map packs below 1.0: nothing to say
    }
    Some(Budget {
        severity: Severity::Error,
        text: format!(
            "WaveLevel: the map's {diagonal:.2} m diagonal reaches the sight shaders' \
             DIST_PACK_RANGE of {range} m. Packed camera distance SATURATES there, it does not \
             wrap: the data core packs clamp(vd / DIST_PACK_RANGE, 0, 1) into B, so everything \
             past {range} m reads a flat 1.0 — its silhouette Laplacian is zero and it draws no \
             outline at all, and the hearing pass cuts player-sound rings against a world it \
             believes is exactly {range} m away. Shrink the map, or raise DIST_PACK_RANGE in \
             game/shaders/pulse_pool.gdshaderinc — a measured decision and not a free one: it \
             rescales every packed distance, and the outline thresholds in hearing_post are tuned \
             against this range."
        ),
    })
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
        assert_eq!(complaints.len(), 1);
        assert_eq!(complaints[0].path, "StrayCrate");
        assert_eq!(
            complaints[0].text,
            "WaveLevel: 'StrayCrate' stands off the floor entirely — its footprint is x \
             -10.50..-9.50, z -10.50..-9.50, and the floor covers x 0.00..20.00, z \
             0.00..20.00. There is no slab under any of it: it draws where nothing holds \
             it up, and the hero who walks there falls out of the world. Move it inside \
             the extents, or grow the level's extents to cover it — the slabs span \
             0..extents from the level's own origin and never move to meet stray geometry."
        );
    }

    /// `path` is the raw address [`PlacedSolid`] carried in, including any
    /// `/` a nested node contributes — not something re-derived from the
    /// sentence, which only QUOTES it. A consumer that jumps to the node by
    /// path (Task 5's editor warning) has to get exactly what the census
    /// walk gave, slashes and all, not a rewritten or truncated form of it.
    #[test]
    fn a_faults_path_is_the_solids_own_nested_address() {
        let complaints = unfloored(
            floor_20x20(),
            &[placed(
                "Rooms/StrayCrate",
                [-10.5, 0.0, -10.5],
                [-9.5, 1.0, -9.5],
            )],
        );
        assert_eq!(complaints.len(), 1);
        assert_eq!(complaints[0].path, "Rooms/StrayCrate");
        assert!(
            complaints[0].text.contains("'Rooms/StrayCrate'"),
            "{}",
            complaints[0].text
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
        assert_eq!(complaints[0].path, "FarCrate");
        assert!(
            complaints[0].text.starts_with(
                "WaveLevel: 'FarCrate' stands off the floor entirely — its footprint is x \
                 24.00..25.00, z 24.00..25.00, and the floor covers x 0.00..20.00, z \
                 0.00..20.00."
            ),
            "{}",
            complaints[0].text
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
        assert_eq!(complaints.len(), 1);
        assert_eq!(complaints[0].path, "LedgeCrate");
        assert_eq!(
            complaints[0].text,
            "WaveLevel: 'LedgeCrate' hangs over the edge of the floor — its footprint is x \
             19.50..20.50, z 4.00..5.00, and the floor covers x 0.00..20.00, z \
             0.00..20.00. The part outside has no slab under it. Move it inside the \
             extents, or grow the level's extents to cover it — the slabs span 0..extents \
             from the level's own origin and never move to meet stray geometry."
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
        assert_eq!(complaints[0].path, "Second");
        assert_eq!(complaints[1].path, "Third");
        assert_eq!(complaints[2].path, "Fourth");
    }

    /// A room's worth of well-placed shapes: a wall standing exactly on
    /// the floor's top, a crate lifted onto it, a shelf floating clear.
    /// None of these is sunk, and a law that said so would be noise.
    #[test]
    fn a_solid_resting_on_or_above_the_floor_says_nothing() {
        let clear = [
            placed("BorderWall", [4.0, 0.0, 0.5], [8.0, 3.0, 0.8]),
            placed("Crate", [4.0, 0.0, 4.0], [4.5, 0.5, 4.5]),
            placed("Rooms/Shelf", [1.0, 1.2, 2.0], [1.4, 1.6, 3.4]),
        ];
        let quiet = sunken(floor_20x20(), &clear);
        assert!(quiet.is_empty(), "{quiet:?}");
    }

    /// THE authoring gesture the issue is named for: drag a `WaveProp` onto
    /// the floor plane. A box prop is CENTRED on its node, so exactly half
    /// of it ends up under the slab — where it never draws, never sounds
    /// and cannot be walked into — and the only thing that ever noticed was
    /// a CI assertion in the shipped map's own suite. The message carries
    /// the origin law, because a designer who does not know it will keep
    /// reaching the same state.
    #[test]
    fn a_prop_dropped_on_the_floor_plane_is_reported_as_half_sunk() {
        let complaints = sunken(
            floor_20x20(),
            &[placed(
                "DesignerCrate",
                [3.75, -0.25, 3.75],
                [4.25, 0.25, 4.25],
            )],
        );
        assert_eq!(complaints.len(), 1);
        assert_eq!(complaints[0].path, "DesignerCrate");
        assert_eq!(
            complaints[0].text,
            "WaveLevel: 'DesignerCrate' is sunk through the floor — its box spans y \
             -0.25..0.25, and the floor's top is at y 0.00. What is under the slab never \
             draws, never sounds and cannot be walked into. A WaveProp is CENTRED on its \
             node, so dropping one on the floor plane buries exactly half of it, while a \
             wall, a column and a wedge STAND on theirs. Lift the node until the whole \
             shape clears y 0.00."
        );
    }

    /// A prop entirely BELOW the floor is reported too, and differently. It
    /// is a node the designer thinks they placed and did not — nothing
    /// under the slab is ever lit or struck — and it is one nudge past the
    /// straddle: the obvious cure for a half-sunk warning is to push the
    /// node down until the complaint stops, so a law that went quiet here
    /// would reward making the fault total.
    #[test]
    fn a_solid_entirely_below_the_floor_is_reported_as_buried() {
        let complaints = sunken(
            floor_20x20(),
            &[placed("LostCrate", [3.75, -1.5, 3.75], [4.25, -0.5, 4.25])],
        );
        assert_eq!(complaints.len(), 1);
        assert_eq!(complaints[0].path, "LostCrate");
        assert_eq!(
            complaints[0].text,
            "WaveLevel: 'LostCrate' is buried under the floor — its box spans y \
             -1.50..-0.50, entirely below the floor's top at y 0.00. Nothing under the slab \
             draws, sounds or can be walked into, so the node is in the scene and not in \
             the world. Lift it until the whole shape clears y 0.00."
        );
    }

    /// Same guarantee as the unfloored case above, on the other law:
    /// `sunken`'s fault carries the solid's raw nested path too, not a
    /// copy trimmed to the leaf name.
    #[test]
    fn a_sunken_faults_path_is_the_solids_own_nested_address() {
        let complaints = sunken(
            floor_20x20(),
            &[placed(
                "Rooms/LostCrate",
                [3.75, -1.5, 3.75],
                [4.25, -0.5, 4.25],
            )],
        );
        assert_eq!(complaints.len(), 1);
        assert_eq!(complaints[0].path, "Rooms/LostCrate");
        assert!(
            complaints[0].text.contains("'Rooms/LostCrate'"),
            "{}",
            complaints[0].text
        );
    }

    /// The predicate is "crosses the plane", not "crosses it by enough to
    /// see": a centimetre under the slab is still a shape cut in two. Float
    /// dust is not — the arithmetic that carries a local AABB into world
    /// space costs ULPs, and a wall's underside sits on the floor's top by
    /// construction.
    #[test]
    fn a_hair_of_straddle_is_reported_and_float_dust_is_not() {
        let hair = [placed("Barely", [4.0, -0.01, 4.0], [4.5, 0.49, 4.5])];
        assert_eq!(sunken(floor_20x20(), &hair).len(), 1);
        let dust = [placed("Flush", [4.0, -0.0004, 4.0], [4.5, 0.5, 4.5])];
        let quiet = sunken(floor_20x20(), &dust);
        assert!(quiet.is_empty(), "{quiet:?}");
    }

    /// The plane is the floor's TOP where the slab actually stands, never
    /// the world's y = 0. A level lifted to y = 5 carries its floor up with
    /// it, so a crate at y 4.75..5.25 is half-sunk there and a crate at
    /// y 0.0..0.5 is not merely sunk but five metres under the map.
    #[test]
    fn the_floor_plane_is_the_slab_top_not_the_world_origin() {
        let raised = Box3 {
            min: [0.0, 4.9, 0.0],
            max: [20.0, 5.0, 20.0],
        };
        let complaints = sunken(
            raised,
            &[
                placed("Half", [3.75, 4.75, 3.75], [4.25, 5.25, 4.25]),
                placed("Under", [3.75, 0.0, 3.75], [4.25, 0.5, 4.25]),
            ],
        );
        assert_eq!(complaints.len(), 2);
        assert_eq!(complaints[0].path, "Half");
        assert!(
            complaints[0]
                .text
                .contains("spans y 4.75..5.25, and the floor's top is at y 5.00"),
            "{}",
            complaints[0].text
        );
        assert_eq!(complaints[1].path, "Under");
        assert!(
            complaints[1]
                .text
                .contains("spans y 0.00..0.50, entirely below the floor's top at y 5.00"),
            "{}",
            complaints[1].text
        );
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

    /// The box pads the centerline by a full half-thickness on every
    /// side — flanks AND run ends alike: 7.4 + 2 × 0.15 = 7.7 and
    /// 18.8 + 0.3 = 19.1 along the run, flank 0.3 and height 3.0. The
    /// literals are hand-derived and the f32 narrowing agrees with them
    /// exactly. A flush run axis (7.7, not 7.69) is exactly the face that
    /// ties with a junction partner's flank plane — the superface merge
    /// law's own MERGE candidate now, not a z-fight to avoid.
    #[test]
    fn wall_box_pads_the_flanks_and_run_flush() {
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

    /// The shipped map spends 19 of the sight shaders' 32 slots, and a
    /// level with rooms to spare must say NOTHING — a budget that spoke on
    /// every level would be noise a designer learns to scroll past, and
    /// the shipped scene emits zero WaveLevel messages today. 32 − 19 = 13
    /// segments left, about three more rooms at four apiece; 28 walls is
    /// the last count with a whole room still in hand.
    #[test]
    fn a_map_with_rooms_to_spare_says_nothing_about_its_wall_budget() {
        assert_eq!(wall_budget(19, 32), None);
        assert_eq!(wall_budget(0, 32), None);
        assert_eq!(wall_budget(28, 32), None);
    }

    /// The heads-up fires one room short of the ceiling and quotes THE
    /// HEADROOM, because how much room is left is the number a designer
    /// can act on — "you have 29 walls" is not. 32 − 29 = 3 segments,
    /// one short of the four another room costs.
    #[test]
    fn a_level_one_room_short_of_the_slots_reports_the_headroom_left() {
        let budget = wall_budget(29, 32).expect("a heads-up");
        assert_eq!(budget.severity, Severity::Warn);
        assert_eq!(
            budget.text,
            "WaveLevel: 29 walls against the sight shaders' 32 slots — 3 segments left, short of \
             the 4 another room costs (three sides plus the doorway, which is the gap between two \
             segments and so costs a segment of its own). Every wall past the last slot silently \
             stops occluding. Raising MAXW (rust/src/sight.rs, mirrored in \
             game/shaders/pulse_pool.gdshaderinc) is a measured decision and not a free one: every \
             wall is another rect in the per-fragment sight loop, on every platform."
        );
    }

    /// A level standing exactly ON the ceiling is not broken YET — 32 walls
    /// fit 32 slots and nothing is truncated — so it warns rather than
    /// errors, with zero headroom. Erroring here would cry wolf on a legal
    /// level; staying silent would hide that the very next wall is dropped.
    #[test]
    fn a_level_that_fills_every_slot_warns_with_no_headroom_left() {
        let budget = wall_budget(32, 32).expect("a heads-up");
        assert_eq!(budget.severity, Severity::Warn);
        assert!(
            budget
                .text
                .contains("32 walls against the sight shaders' 32 slots — 0 segments left"),
            "{}",
            budget.text
        );
    }

    /// Past the ceiling the world is already wrong, so this is an ERROR
    /// rather than a heads-up: the table keeps the first MAXW rects and
    /// every wall after them stops occluding entirely. It COUNTS the
    /// dropped walls instead of saying "the rest" — a designer deleting
    /// walls to get back under the ceiling needs to know how many.
    #[test]
    fn a_level_past_the_slots_errors_and_counts_what_stopped_occluding() {
        let budget = wall_budget(33, 32).expect("an error");
        assert_eq!(budget.severity, Severity::Error);
        assert_eq!(
            budget.text,
            "WaveLevel: 33 walls exceed the sight shaders' 32 slots — the table keeps the first 32 \
             and drops 1, which stop occluding entirely: waves pass straight through them and no \
             sight line counts them. Delete or merge walls, or raise MAXW (rust/src/sight.rs, \
             mirrored in game/shaders/pulse_pool.gdshaderinc) — a measured decision and not a free \
             one: every wall is another rect in the per-fragment sight loop, on every platform."
        );
        let far_past = wall_budget(40, 32).expect("an error");
        assert!(far_past.text.contains("and drops 8,"), "{}", far_past.text);
    }

    /// Total on the degenerate budget a caller could hand it: a shader with
    /// no slots at all overflows rather than subtracting past zero, which
    /// on a `usize` is not a small number but a colossal one.
    #[test]
    fn a_slotless_shader_is_reported_not_subtracted_past_zero() {
        assert_eq!(wall_budget(0, 0).map(|b| b.severity), Some(Severity::Warn));
        assert_eq!(wall_budget(1, 0).map(|b| b.severity), Some(Severity::Error));
    }

    /// The four border walls of a rectangular map, as centerlines.
    fn border(x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<Vector4> {
        vec![
            Vector4::new(x0, z0, x1, z0),
            Vector4::new(x1, z0, x1, z1),
            Vector4::new(x1, z1, x0, z1),
            Vector4::new(x0, z1, x0, z0),
        ]
    }

    /// The map's diagonal is the longest sight line the level allows,
    /// measured across the WALL CENTERLINES and floor to ceiling — the same
    /// measure `game/tests/shader_contract_test.gd` already holds
    /// DIST_PACK_RANGE against, so the level's own report and that suite's
    /// assertion can never disagree about the same map.
    ///
    /// The shipped 28 × 28 map borders its walls at 0.6 and 27.4, a 26.8 m
    /// span each way: sqrt(26.8² + 3² + 26.8²) = sqrt(1445.48) = 38.019 m.
    /// Widen it to 32 × 28, the reproduction the issue names, and the walls
    /// border at 0.6/31.4 and 0.6/27.4: sqrt(30.8² + 3² + 26.8²) =
    /// sqrt(1675.88) = 40.938 m, past the shaders' 40.
    #[test]
    fn the_map_diagonal_spans_the_wall_centerlines_floor_to_ceiling() {
        let shipped = map_diagonal(&border(0.6, 0.6, 27.4, 27.4));
        assert!((shipped - 38.0195).abs() < 1e-3, "{shipped}");
        let widened = map_diagonal(&border(0.6, 0.6, 31.4, 27.4));
        assert!((widened - 40.9375).abs() < 1e-3, "{widened}");
    }

    /// A level with no walls has no footprint to measure, and answering
    /// with the difference of two infinities would poison the very
    /// comparison that decides whether to shout.
    #[test]
    fn a_level_with_no_walls_has_no_diagonal() {
        assert_eq!(map_diagonal(&[]), 0.0);
    }

    /// A centerline is a QUAD, not an ordered pair, and BOTH ends of both
    /// axes have to be read. [`wall_segment`] happens to sweep its ends in
    /// ascending order today, which is exactly what makes the other half
    /// easy to lose: a walk that read only `x` and `y` would still measure
    /// the shipped map correctly and would silently shrink any table whose
    /// quads arrived the other way round — the failure mode `sight::wall_rect`
    /// normalises against and the `wall_segment` doc warns about.
    ///
    /// So the same footprint is measured twice, once with every quad
    /// reversed. The stub first is not decoration: the walk seeds itself
    /// from the first quad, so an extreme that lives there would be found
    /// by the seeding whatever the loop dropped. sqrt(8² + 3² + 5²) =
    /// sqrt(98) = 9.8994949366.
    #[test]
    fn both_ends_of_a_centerline_are_read_whichever_way_round_it_arrives() {
        let forward = [
            Vector4::new(4.0, 4.0, 4.0, 5.0), // a stub in the middle, seeding the walk
            Vector4::new(1.0, 2.0, 9.0, 7.0), // the whole footprint, ends ascending
        ];
        let flipped: Vec<Vector4> = forward
            .iter()
            .map(|s| Vector4::new(s.z, s.w, s.x, s.y))
            .collect();
        // the literal twice over, never one measure against the other: an
        // expectation computed by the function under test would hold on any
        // footprint it happened to return, including a wrong one
        assert!(
            (map_diagonal(&forward) - 9.899_494_936_611_665).abs() < 1e-12,
            "{}",
            map_diagonal(&forward)
        );
        assert!(
            (map_diagonal(&flipped) - 9.899_494_936_611_665).abs() < 1e-12,
            "{}",
            map_diagonal(&flipped)
        );
    }

    /// One wall is a footprint too, and the height is never dropped: a 4 m
    /// segment with no depth still spans floor to ceiling, so the diagonal
    /// is the 3-4-5 triangle's 5 m exactly.
    #[test]
    fn a_single_wall_still_measures_floor_to_ceiling() {
        assert_eq!(map_diagonal(&[Vector4::new(1.0, 2.0, 5.0, 2.0)]), 5.0);
    }

    /// The shipped map's 38.02 m against the shaders' 40 m range: nearly
    /// two metres of headroom, and silence. The shipped scene emits zero
    /// WaveLevel messages today and must keep emitting zero.
    #[test]
    fn a_map_inside_the_packing_range_says_nothing() {
        assert_eq!(pack_range_budget(38.02, 40.0), None);
        assert_eq!(pack_range_budget(39.99, 40.0), None);
        assert_eq!(pack_range_budget(0.0, 40.0), None);
    }

    /// EQUALITY IS ALREADY TOO FAR, and this is what keeps the level's own
    /// report in step with `game/tests/shader_contract_test.gd`, which
    /// demands the range be strictly GREATER than the diagonal. Two reasons
    /// the law is strict: at vd == range the packed value is already 1.0,
    /// the top of the band and the first value that cannot be told from
    /// anything beyond it; and the diagonal is measured across the wall
    /// centerlines while the floor and ceiling slabs span the whole extents
    /// knob, so real drawn geometry reaches further than the number checked.
    #[test]
    fn a_diagonal_that_exactly_reaches_the_range_is_already_reported() {
        assert_eq!(
            pack_range_budget(40.0, 40.0).map(|b| b.severity),
            Some(Severity::Error)
        );
    }

    /// The issue's reproduction, and the message it must produce: widening
    /// the shipped map to 32 × 28 pushes the diagonal to 40.94 m.
    ///
    /// The message says SATURATES, not aliases, because that is what the
    /// GLSL does: data_core.gdshaderinc:149 packs
    /// `clamp(vd / DIST_PACK_RANGE, 0.0, 1.0)` into B, so nothing wraps and
    /// nothing folds — everything past the range reads a flat 1.0. The
    /// consequences follow from the flatness rather than from a wrap:
    /// hearing_post's silhouette Laplacian (line 72) over a plateau is
    /// zero, so far geometry draws no outline at all, and its
    /// `scene_d = c_c.b * DIST_PACK_RANGE` (line 57) pins at the range, so
    /// the ring cut at line 123 kills a player's sound against a world that
    /// is not where it says it is.
    #[test]
    fn a_map_past_the_packing_range_names_the_diagonal_and_what_saturates() {
        let budget = pack_range_budget(40.9375, 40.0).expect("a report");
        assert_eq!(budget.severity, Severity::Error);
        assert_eq!(
            budget.text,
            "WaveLevel: the map's 40.94 m diagonal reaches the sight shaders' DIST_PACK_RANGE of \
             40 m. Packed camera distance SATURATES there, it does not wrap: the data core packs \
             clamp(vd / DIST_PACK_RANGE, 0, 1) into B, so everything past 40 m reads a flat 1.0 — \
             its silhouette Laplacian is zero and it draws no outline at all, and the hearing pass \
             cuts player-sound rings against a world it believes is exactly 40 m away. Shrink the \
             map, or raise DIST_PACK_RANGE in game/shaders/pulse_pool.gdshaderinc — a measured \
             decision and not a free one: it rescales every packed distance, and the outline \
             thresholds in hearing_post are tuned against this range."
        );
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
