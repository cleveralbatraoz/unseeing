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

use godot::builtin::{Basis, Transform3D, Vector2, Vector3, Vector4};

use crate::oid_palette::Box3;

/// Wall height in meters — walls run floor to ceiling.
pub const WALL_H: f64 = 3.0;

/// How much a source's own ACOUSTIC IMAGE survives crossing one wall — its
/// wave, by contrast, stops dead at that same wall
/// (`crate::sight::reveal_visibility`), so a source felt through a wall is
/// a shape with nothing behind it: a faint ghost, fainter still through
/// two. A wall dims the shape, never erases it — the source is always
/// felt, just muted.
///
/// It is a MULTIPLIER over the whole image, not a floor under part of it,
/// and the distinction is the difference between a law and a decoration.
/// `crate::render::reveal::source_image` composes it as
/// `muffle * max(wave, volume)`, where `muffle` is this constant raised to
/// the number of walls between the EYE and the source (`WaveLevel`'s
/// `source_muffle`, the camera occluder). Delivered instead as a
/// pre-multiplied standing floor — which is what shipped — it could only
/// compete with the source's own wave reveal through a `max()`, and always
/// lost: a source's hub is unwalled from its own body by construction, so
/// that wave reads near full strength however many walls stand between the
/// source and the player. The 0.30 / 0.09 / 0.027 ladder below held only
/// while the source happened to be silent, which for a running fan or
/// radio is never.
pub const SOURCE_THROUGH: f64 = 0.3;

/// Half-thickness of a wall in meters.
pub const WALL_T: f64 = 0.15;
/// Largest designer wall accepted by the f32 Godot geometry boundary.
pub const MAX_WALL_LENGTH: f64 = 10_000.0;

/// Thickness of the floor and ceiling slabs.
pub const SLAB_T: f64 = 0.1;

/// How far a solid may fall short of floor or ceiling and still count as
/// spanning the corridor.
///
/// Not a tolerance for float noise — 5 cm is far larger than that. It is
/// the gap a designer may leave under a shelf or over a cabinet without the
/// thing ceasing to be, acoustically, a piece of the room's structure. The
/// shipped level separates its own populations by ten times this: the
/// pillars reach 3.00 exactly and the pipes stop at 2.90.
pub const SPAN_EPS: f64 = 0.05;

/// Does this solid actually stand in the way of sound?
///
/// # Why geometry decides, and not the node's class
///
/// `data_core.gdshaderinc` asserted for months that "props are transparent
/// to waves — only walls obstruct", and three separate comments called it
/// deliberate. It was not: the occluder table has only ever been built from
/// the WALL census, so no prop could enter it whatever its shape, and the
/// only argument ever recorded for the transparency was the cost of
/// admitting all 106 of them at once.
///
/// The cost argument is sound and is not overturned here. What is
/// overturned is deciding by class. A pillar from floor to ceiling and half
/// a metre thick stops sound the way a wall does, because it IS a wall that
/// happens to be round; a crate at knee height does not, because sound
/// simply goes over it. Asking the geometry costs nothing, admits the few
/// solids that genuinely block, and refuses the many that do not — where
/// asking the class admits all or none.
///
/// # The two criteria, and why both
///
/// - **It spans the corridor.** From no higher than [`SLAB_T`] + [`SPAN_EPS`]
///   to no lower than [`WALL_H`] − [`SPAN_EPS`]. Sound that can pass over a
///   thing is not stopped by it, and every box prop in the shipped level
///   tops out at 2.00 m against a 3.00 m ceiling and an eye at 1.6 m.
/// - **It is no thinner than a wall.** Minimum horizontal extent at least
///   twice [`WALL_T`]. This is the criterion that refuses standpipes: they
///   run the full height of the room and are 14–20 cm across, and a table
///   of axis-aligned rects would have them casting square metre-wide
///   shadows they have no business casting.
///
/// Deliberately NOT a wavelength test. This engine has no frequency axis
/// anywhere, and its wavefronts travel at 4–5.5 m/s; a Fresnel number
/// computed here would be fiction dressed as physics. The adversarial
/// review that examined exactly that argument refuted it 3/3.
///
/// Total over every f64: any non-finite input answers `false`, which
/// refuses the solid rather than admitting a rect the sight tests would
/// then walk with NaN corners.
#[must_use]
pub fn spans_the_corridor(bottom: f64, top: f64, min_horizontal_extent: f64) -> bool {
    if !bottom.is_finite() || !top.is_finite() || !min_horizontal_extent.is_finite() {
        return false;
    }
    bottom <= SLAB_T + SPAN_EPS && top >= WALL_H - SPAN_EPS && min_horizontal_extent >= 2.0 * WALL_T
}

/// The hero's capsule center over the floor a spawn datum stands on.
pub const SPAWN_LIFT: f64 = 0.9;

/// Height of the dev demo tap on its wall — a natural cane-strike height.
pub const DEMO_TAP_H: f64 = 0.8;

/// The demo tap stays this far from its wall's ends, so the strike lands
/// on the wall's face and never on a corner shared with another wall.
pub const DEMO_TAP_MARGIN: f64 = 0.2;

/// Two segment coordinates within this are the same axis line.
pub const AXIS_EPS: f32 = 0.001;

/// Whether a designer-authored geometry edit may change the live derived
/// object. Before tree entry every setter is scene construction. After ready,
/// the editor remains live while runtime keeps the exact snapshot its owning
/// level derived.
#[must_use]
pub const fn authored_geometry_edit_is_live(inside_tree: bool, editor: bool) -> bool {
    !inside_tree || editor
}

/// One positive wall left after carving openings from an authored run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunSeg {
    pub center: Vector2,
    pub length: f32,
    /// `true` means the segment runs along Z at a fixed X coordinate.
    pub vertical: bool,
}

/// The total result of interpreting a run: every residual segment and the
/// one editor-facing explanation needed when its endpoints were folded or
/// rejected.
#[derive(Debug, Clone, PartialEq)]
pub struct RunPlan {
    pub segments: Vec<RunSeg>,
    pub complaint: Option<String>,
}

/// Saved WaveRun data, expressed without a scene-tree handle. Coordinates
/// are the parent's local X/Z plane and every opening is
/// `(absolute selected-axis coordinate, width)`.
#[derive(Debug, Clone, PartialEq)]
pub struct RunAuthoring {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub openings: Vec<[f64; 2]>,
}

/// A Node3D pose reduced to plain values. `columns` are the basis's X, Y
/// and Z columns in that order; keeping the representation explicit makes
/// pose absorption cargo-testable without a live node or scene tree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunPose {
    pub origin: [f64; 3],
    pub columns: [[f64; 3]; 3],
}

impl RunPose {
    pub const IDENTITY: Self = Self {
        origin: [0.0, 0.0, 0.0],
        columns: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };
}

/// Information a Node3D pose could not carry into the run's planar,
/// f32-backed Inspector data. Rejections return no replacement authoring
/// state, so the boundary can reset the bad pose without poisoning the data
/// the designer had before the gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPoseWarning {
    VerticalLoss,
    NonFiniteTransform,
    NonFiniteAuthoring,
    ProjectionOutOfRange,
}

impl RunPoseWarning {
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Self::VerticalLoss => {
                "WaveRun: Y translation, Y scale, or tilt cannot be represented by planar X/Z \
                 endpoints — the planar projection was kept and vertical components were \
                 discarded."
            }
            Self::NonFiniteTransform => {
                "WaveRun: the node transform contains NaN or infinity and cannot be absorbed \
                 into planar X/Z endpoints — the transform was discarded and the authored \
                 endpoints/openings were left unchanged."
            }
            Self::NonFiniteAuthoring => {
                "WaveRun: from/to or openings contain NaN or infinity, so a node transform \
                 cannot be absorbed safely — the transform was discarded and the authored \
                 endpoints/openings were left unchanged."
            }
            Self::ProjectionOutOfRange => {
                "WaveRun: the node transform projects beyond the finite coordinate range the \
                 Inspector can save — the transform was discarded and the authored \
                 endpoints/openings were left unchanged."
            }
        }
    }
}

/// Total result of absorbing a WaveRun's own pose into its saved endpoints.
#[derive(Debug, Clone, PartialEq)]
pub enum RunPoseResult {
    Applied {
        authored: RunAuthoring,
        warning: Option<RunPoseWarning>,
    },
    Rejected(RunPoseWarning),
}

fn finite_run_authoring(authored: &RunAuthoring) -> bool {
    authored
        .from
        .into_iter()
        .chain(authored.to)
        .chain(authored.openings.iter().flatten().copied())
        .all(f64::is_finite)
}

fn within_f32(value: f64) -> bool {
    value.abs() <= f64::from(f32::MAX)
}

/// Absorb a WaveRun node's local transform into parent-local X/Z endpoint
/// and opening data. This is the whole deterministic mapping law; the Godot
/// node only measures values, applies an `Applied` result, resets its pose,
/// and stores/voices the returned warning.
///
/// Invalid input is non-destructive. A non-finite transform, already-poisoned
/// authoring data, or any projection that cannot round-trip through the
/// Inspector's f32 fields is rejected before replacement data is returned.
#[must_use]
pub fn absorb_run_pose(authored: &RunAuthoring, pose: RunPose) -> RunPoseResult {
    if !pose
        .origin
        .into_iter()
        .chain(pose.columns.into_iter().flatten())
        .all(f64::is_finite)
    {
        return RunPoseResult::Rejected(RunPoseWarning::NonFiniteTransform);
    }
    if !finite_run_authoring(authored) {
        return RunPoseResult::Rejected(RunPoseWarning::NonFiniteAuthoring);
    }
    if !pose
        .origin
        .into_iter()
        .chain(pose.columns.into_iter().flatten())
        .chain(authored.from)
        .chain(authored.to)
        .chain(authored.openings.iter().flatten().copied())
        .all(within_f32)
    {
        return RunPoseResult::Rejected(RunPoseWarning::ProjectionOutOfRange);
    }

    let map = |point: [f64; 2]| -> Option<[f64; 2]> {
        let world = [
            pose.origin[0] + pose.columns[0][0] * point[0] + pose.columns[2][0] * point[1],
            pose.origin[1] + pose.columns[0][1] * point[0] + pose.columns[2][1] * point[1],
            pose.origin[2] + pose.columns[0][2] * point[0] + pose.columns[2][2] * point[1],
        ];
        world
            .into_iter()
            .all(|value| value.is_finite() && within_f32(value))
            .then_some([world[0], world[2]])
    };

    let Some(new_from) = map(authored.from) else {
        return RunPoseResult::Rejected(RunPoseWarning::ProjectionOutOfRange);
    };
    let Some(new_to) = map(authored.to) else {
        return RunPoseResult::Rejected(RunPoseWarning::ProjectionOutOfRange);
    };
    let old_vertical =
        (authored.to[1] - authored.from[1]).abs() > (authored.to[0] - authored.from[0]).abs();
    let new_vertical = (new_to[1] - new_from[1]).abs() > (new_to[0] - new_from[0]).abs();
    let mut mapped_openings = Vec::with_capacity(authored.openings.len());
    for opening in &authored.openings {
        let width = opening[1].abs();
        let start = if old_vertical {
            [authored.from[0], opening[0]]
        } else {
            [opening[0], authored.from[1]]
        };
        let end = if old_vertical {
            [authored.from[0], opening[0] + width]
        } else {
            [opening[0] + width, authored.from[1]]
        };
        let (Some(mapped_start), Some(mapped_end)) = (map(start), map(end)) else {
            return RunPoseResult::Rejected(RunPoseWarning::ProjectionOutOfRange);
        };
        let (a, b) = if new_vertical {
            (mapped_start[1], mapped_end[1])
        } else {
            (mapped_start[0], mapped_end[0])
        };
        let mapped = [a.min(b), (b - a).abs()];
        if !mapped
            .into_iter()
            .all(|value| value.is_finite() && within_f32(value))
        {
            return RunPoseResult::Rejected(RunPoseWarning::ProjectionOutOfRange);
        }
        mapped_openings.push(mapped);
    }

    let up = pose.columns[1];
    let up_len = (up[0] * up[0] + up[1] * up[1] + up[2] * up[2]).sqrt();
    let up_aligned = up_len > 0.0 && up[1] / up_len >= 0.9999;
    let vertical_loss = pose.origin[1].abs() > 1e-4
        || (up_len - 1.0).abs() > 1e-4
        || !up_aligned
        || pose.columns[0][1].abs() > 1e-4
        || pose.columns[2][1].abs() > 1e-4;

    RunPoseResult::Applied {
        authored: RunAuthoring {
            from: new_from,
            to: new_to,
            openings: mapped_openings,
        },
        warning: vertical_loss.then_some(RunPoseWarning::VerticalLoss),
    }
}

/// Cut openings whose first value is an absolute start coordinate on the
/// selected axis out of an X/Z run. Reversed ends,
/// negative widths, overlaps and out-of-range openings all normalize here so
/// the engine node only has to instantiate the returned positive segments.
#[must_use]
pub fn run_segments(from: Vector2, to: Vector2, openings: &[(f64, f64)]) -> RunPlan {
    let finite = [from.x, from.y, to.x, to.y].into_iter().all(f32::is_finite);
    if !finite {
        return RunPlan {
            segments: vec![],
            complaint: Some(
                "WaveRun: from/to contain a non-finite coordinate — no walls were emitted; replace NaN or infinity with finite X/Z coordinates."
                    .to_string(),
            ),
        };
    }

    // Widen BEFORE arithmetic. Vector2 admits every finite f32, but the
    // difference between -f32::MAX and +f32::MAX is not itself a finite
    // f32. Doing this in the engine lane would manufacture infinities from
    // valid authored inputs before the planner had a chance to refuse them.
    let from_x = f64::from(from.x);
    let from_z = f64::from(from.y);
    let to_x = f64::from(to.x);
    let to_z = f64::from(to.y);
    let dx = (to_x - from_x).abs();
    let dz = (to_z - from_z).abs();
    let vertical = dz > dx; // X wins exact ties.
    let (a, b, fixed) = if vertical {
        (from_z, to_z, from_x)
    } else {
        (from_x, to_x, from_z)
    };
    let lo = a.min(b);
    let hi = a.max(b);
    if hi <= lo {
        return RunPlan {
            segments: vec![],
            complaint: Some(
                "WaveRun: from/to describe a zero-length run — no walls were emitted; move either endpoint along X or Z."
                    .to_string(),
            ),
        };
    }

    let diagonal = dx > f64::from(AXIS_EPS) && dz > f64::from(AXIS_EPS);
    let axis_name = if vertical { "Z" } else { "X" };
    let complaint = diagonal.then(|| {
        format!(
            "WaveRun: diagonal endpoints folded onto the dominant {axis_name} axis — runs emit axis-aligned WaveWalls; move from/to onto one X or Z line to clear this warning."
        )
    });

    let mut cuts: Vec<(f64, f64)> = openings
        .iter()
        .filter_map(|&(coordinate, width)| {
            if !coordinate.is_finite() || !width.is_finite() {
                return None;
            }
            // PackedVector2Array stores both lanes as f32. Quantize each
            // clamped cut boundary to that real authoring lane before the
            // widened residual math, preserving the shipped scene's exact
            // 7.0 m residual instead of mixing an f64 literal 12.4 with an
            // f32 endpoint widened to f64.
            let start = f64::from(coordinate.clamp(lo, hi) as f32);
            let raw_end = coordinate + width.abs();
            // finite + positive magnitude can overflow only toward +inf;
            // semantically that is simply an opening extending past the
            // run, so clamp it to the run's high end.
            let end = if raw_end.is_finite() {
                f64::from(raw_end.clamp(lo, hi) as f32)
            } else {
                hi
            };
            (end > start).then_some((start, end))
        })
        .collect();
    cuts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut merged: Vec<(f64, f64)> = Vec::with_capacity(cuts.len());
    for cut in cuts {
        if let Some(last) = merged.last_mut()
            && cut.0 <= last.1
        {
            last.1 = last.1.max(cut.1);
        } else {
            merged.push(cut);
        }
    }

    let mut segments = Vec::with_capacity(merged.len() + 1);
    let mut cursor = lo;
    for (start, end) in merged.into_iter().chain(std::iter::once((hi, hi))) {
        if start > cursor {
            let along = (cursor + start) * 0.5;
            let length = start - cursor;
            if ![fixed, along, length]
                .into_iter()
                .all(|value| value.is_finite() && within_f32(value))
            {
                return RunPlan {
                    segments: vec![],
                    complaint: Some(
                        "WaveRun: from/to are finite but their derived wall exceeds the finite \
                         coordinate range — no walls were emitted; move the endpoints closer to \
                         the level."
                            .to_string(),
                    ),
                };
            }
            segments.push(RunSeg {
                center: if vertical {
                    Vector2::new(fixed as f32, along as f32)
                } else {
                    Vector2::new(along as f32, fixed as f32)
                },
                length: length as f32,
                vertical,
            });
        }
        cursor = cursor.max(end);
    }
    RunPlan {
        segments,
        complaint,
    }
}

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
    let length = sanitize_wall_length(length, 4.0).value;
    Vector3::new(
        (length.abs() + WALL_T * 2.0) as f32,
        WALL_H as f32,
        (WALL_T * 2.0) as f32,
    )
}

/// A wall length safe for every downstream representation: the f64 authored
/// knob, f32 ArrayMesh/BoxShape size, and f32 centerline endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallLengthPlan {
    pub value: f64,
    pub repaired: bool,
}

/// Fold a finite sign and reject any magnitude that would narrow to NaN/Inf
/// in Godot geometry. Invalid input keeps the last valid value, or the 4 m
/// class default if the supplied fallback is itself malformed.
#[must_use]
pub fn sanitize_wall_length(raw: f64, fallback: f64) -> WallLengthPlan {
    let valid = |value: f64| {
        let magnitude = value.abs();
        value.is_finite()
            && magnitude <= MAX_WALL_LENGTH
            && (magnitude + WALL_T * 2.0).is_finite()
            && ((magnitude + WALL_T * 2.0) as f32).is_finite()
            && ((magnitude * 0.5) as f32).is_finite()
    };
    if valid(raw) {
        WallLengthPlan {
            value: raw.abs(),
            repaired: false,
        }
    } else {
        WallLengthPlan {
            value: if valid(fallback) { fallback.abs() } else { 4.0 },
            repaired: true,
        }
    }
}

/// Whether the level DRAWS one of the two slabs it built. The pair is
/// always BUILT — the level keeps floor and ceiling as one ordered pair,
/// and everything that reads it (the extents knob, the fixed-role label anchors,
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

/// The yaw carried by a global basis, read from its local +X axis. Invalid
/// or degenerate transforms fall back to zero so an editor transform can
/// never put a non-finite heading into the running player.
#[must_use]
pub fn basis_heading(basis: Basis) -> f64 {
    let x = basis.col_a();
    if !x.x.is_finite() || !x.z.is_finite() || (x.x == 0.0 && x.z == 0.0) {
        return 0.0;
    }
    f64::from(-x.z).atan2(f64::from(x.x))
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

/// Collapse any wall basis — including inherited prefab rotation, scale,
/// tilt, degeneracy, or non-finite lanes — onto the nearest exact unit
/// quarter turn. This is the whole deterministic placement law; the Godot
/// node adapter only decides when a changed global transform must apply it.
#[must_use]
pub fn normalized_wall_basis(basis: Basis) -> Basis {
    quadrant_basis(basis_quadrant(basis))
}

/// Whether Godot can map an exact global wall transform back through its
/// current parent. This is a domain result, not an assertion: zero scale and
/// non-finite Inspector input are representable scene states even though no
/// finite affine inverse (and therefore no safe global write) exists for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallParentTransformState {
    /// The parent is absent or has a finite inverse.
    Representable,
    /// The finite parent basis has no inverse, usually because one scale lane
    /// is zero.
    Singular,
    /// The parent or its computed inverse contains NaN or infinity.
    NonFinite,
}

/// The repair state a live wall owns between editor frames. This is domain
/// memory rather than an engine handle: exact local/parent samples identify an
/// unchanged composed placement, `last_finite` repairs poisoned lanes, and the
/// fault names the one designer action still required.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WallTransformMemory {
    pub normalized_local: Option<Transform3D>,
    pub normalized_parent: Option<Transform3D>,
    pub last_finite: Option<Transform3D>,
    /// The last finite local pose Godot actually retained after a successful
    /// write. Recovery must happen in this authored coordinate space before a
    /// parent multiplication can spread one poisoned lane across all three.
    pub last_finite_local: Option<Transform3D>,
    pub fault: Option<WallTransformFault>,
    /// A repaired own-input fault still awaiting a real edit. An ancestor
    /// fault temporarily takes presentation priority without erasing this
    /// acknowledgment debt.
    pub pending_own_acknowledgment: bool,
}

/// A repairable authored-transform fault, deliberately free of presentation
/// text so the Godot adapter can give editor and runtime channels the same
/// words without putting engine logging into this law.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallTransformFault {
    Ancestor,
    Own,
}

/// One complete pure decision for a wall process frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallTransformPlan {
    pub memory: WallTransformMemory,
    pub write_global: Option<Transform3D>,
    pub announce_snap: bool,
}

/// State for the one numeric physics knob WaveWall deliberately exposes.
/// The authored node is a Node3D datum, not a dummy PhysicsBody proxy, so
/// broader inherited body state does not leak into this law.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WallPriorityMemory {
    pub last_valid: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallPriorityPlan {
    pub value: f32,
    pub memory: WallPriorityMemory,
    pub repaired: bool,
    pub warn: bool,
}

/// Collision priority is narrower than an arbitrary scalar: Godot requires a
/// finite f32 greater than zero. Invalid input keeps the last valid value, or
/// the engine default before one exists.
#[must_use]
pub fn sanitize_wall_priority(raw: f32, fallback: Option<f32>) -> (f32, bool) {
    let valid = |value: f32| value.is_finite() && value > 0.0;
    if valid(raw) {
        (raw, false)
    } else {
        (fallback.filter(|value| valid(*value)).unwrap_or(1.0), true)
    }
}

/// Repair an invalid priority, and clear its warning on the next valid setter
/// call. The Node3D boundary stores the repaired backing field directly (it
/// does not re-enter this setter), so even re-entering the displayed value is
/// an unambiguous designer acknowledgment.
#[must_use]
pub fn plan_wall_priority(raw: f32, mut memory: WallPriorityMemory) -> WallPriorityPlan {
    let (value, repaired) = sanitize_wall_priority(raw, memory.last_valid);
    if repaired {
        memory.last_valid = Some(value);
        return WallPriorityPlan {
            value,
            memory,
            repaired: true,
            warn: true,
        };
    }
    memory.last_valid = Some(value);
    WallPriorityPlan {
        value,
        memory,
        repaired: false,
        warn: false,
    }
}

/// Classify the complete parent-global transform before the Godot adapter
/// calls `set_global_transform`. Checking the inverse as well as the input
/// closes the subnormal finite case where division would overflow.
#[must_use]
pub fn wall_parent_transform_state(
    parent: Option<Transform3D>,
    desired_global: Transform3D,
) -> WallParentTransformState {
    if !desired_global.is_finite() {
        return WallParentTransformState::NonFinite;
    }
    let Some(parent) = parent else {
        return WallParentTransformState::Representable;
    };
    if !parent.is_finite() {
        return WallParentTransformState::NonFinite;
    }
    let determinant = parent.basis.determinant();
    if !determinant.is_finite() {
        return WallParentTransformState::NonFinite;
    }
    if determinant == 0.0 {
        return WallParentTransformState::Singular;
    }
    let inverse = parent.affine_inverse();
    if !inverse.is_finite() || !(inverse * desired_global).is_finite() {
        return WallParentTransformState::NonFinite;
    }
    WallParentTransformState::Representable
}

/// Recover a finite global wall placement from poisoned external input. Each
/// finite origin lane is authored data and survives independently; an invalid
/// lane falls back to the last wholly finite placement (or zero before one
/// exists). A finite, non-zero planar X direction still chooses the new yaw
/// even if another basis lane is poisoned; otherwise the last valid yaw wins.
/// The result is always finite and carries an exact unit quadrant basis.
#[must_use]
pub fn recover_wall_transform(current: Transform3D, fallback: Option<Transform3D>) -> Transform3D {
    let fallback = fallback
        .filter(Transform3D::is_finite)
        .unwrap_or(Transform3D::IDENTITY);
    let lane = |current: f32, fallback: f32| {
        if current.is_finite() {
            current
        } else {
            fallback
        }
    };
    let origin = Vector3::new(
        lane(current.origin.x, fallback.origin.x),
        lane(current.origin.y, fallback.origin.y),
        lane(current.origin.z, fallback.origin.z),
    );
    let x = current.basis.col_a();
    let basis = if x.x.is_finite() && x.z.is_finite() && (x.x != 0.0 || x.z != 0.0) {
        normalized_wall_basis(current.basis)
    } else {
        normalized_wall_basis(fallback.basis)
    };
    Transform3D::new(basis, origin)
}

/// Repair untrusted authored LOCAL input before it is composed through a
/// parent. Matrix multiplication can spread one NaN/Inf position lane across
/// every world lane (`0 * Inf` is NaN), so recovering a poisoned global pose
/// is too late to preserve the finite values the designer entered.
#[must_use]
pub fn recover_wall_local(current: Transform3D, fallback: Option<Transform3D>) -> Transform3D {
    let fallback = fallback
        .filter(Transform3D::is_finite)
        .unwrap_or(Transform3D::IDENTITY);
    let lane = |current: f32, fallback: f32| {
        if current.is_finite() {
            current
        } else {
            fallback
        }
    };
    let origin = Vector3::new(
        lane(current.origin.x, fallback.origin.x),
        lane(current.origin.y, fallback.origin.y),
        lane(current.origin.z, fallback.origin.z),
    );
    let basis = if current.basis.is_finite() {
        current.basis
    } else {
        let x = current.basis.col_a();
        if x.x.is_finite() && x.z.is_finite() && (x.x != 0.0 || x.z != 0.0) {
            normalized_wall_basis(current.basis)
        } else {
            fallback.basis
        }
    };
    Transform3D::new(basis, origin)
}

/// Decide every live-wall transform transition without touching the scene
/// tree. A stable local/parent pair is a settled generation even when Godot's
/// inverse/recomposition leaves a few low bits of dust in the global basis.
/// Poisoned own input is repaired once and keeps its warning across idle
/// frames; the next finite authored edit clears it. A singular/non-finite
/// ancestor produces no write at all and is retried only as a pure read until
/// it becomes representable.
#[must_use]
pub fn plan_wall_transform(
    current_global: Transform3D,
    current_local: Transform3D,
    parent: Option<Transform3D>,
    mut memory: WallTransformMemory,
) -> WallTransformPlan {
    let own_input_is_finite = current_local.is_finite();
    let authored_wall_edit = own_input_is_finite
        && memory
            .normalized_local
            .is_some_and(|normalized| normalized != current_local);
    if authored_wall_edit {
        memory.pending_own_acknowledgment = false;
    } else if !own_input_is_finite {
        memory.pending_own_acknowledgment = true;
    }

    if memory.normalized_local == Some(current_local) && memory.normalized_parent == parent {
        return WallTransformPlan {
            memory,
            write_global: None,
            announce_snap: false,
        };
    }

    let repaired_local = if own_input_is_finite {
        current_local
    } else {
        recover_wall_local(current_local, memory.last_finite_local)
    };
    let composed = parent.map_or(repaired_local, |parent| parent * repaired_local);
    if !composed.is_finite() {
        if own_input_is_finite {
            memory.normalized_local = Some(current_local);
            memory.normalized_parent = parent;
            memory.last_finite_local = Some(current_local);
        }
        memory.fault = Some(WallTransformFault::Ancestor);
        return WallTransformPlan {
            memory,
            write_global: None,
            announce_snap: false,
        };
    }
    let desired = Transform3D::new(normalized_wall_basis(composed.basis), composed.origin);
    if wall_parent_transform_state(parent, desired) != WallParentTransformState::Representable {
        if own_input_is_finite {
            memory.normalized_local = Some(current_local);
            memory.normalized_parent = parent;
            memory.last_finite_local = Some(current_local);
        }
        memory.fault = Some(WallTransformFault::Ancestor);
        return WallTransformPlan {
            memory,
            write_global: None,
            announce_snap: false,
        };
    }

    if !own_input_is_finite {
        memory.last_finite = Some(desired);
        memory.fault = Some(WallTransformFault::Own);
        return WallTransformPlan {
            memory,
            write_global: Some(desired),
            announce_snap: false,
        };
    }

    memory.fault = memory
        .pending_own_acknowledgment
        .then_some(WallTransformFault::Own);
    if composed.basis == desired.basis {
        memory.last_finite = Some(composed);
        memory.last_finite_local = Some(current_local);
        memory.normalized_local = Some(current_local);
        memory.normalized_parent = parent;
        return WallTransformPlan {
            memory,
            write_global: None,
            announce_snap: false,
        };
    }

    memory.last_finite = Some(desired);
    WallTransformPlan {
        memory,
        write_global: Some(desired),
        announce_snap: current_global.is_finite()
            && !wall_bases_close(current_global.basis, desired.basis),
    }
}

/// Record the actual local transform Godot produced for a successful global
/// write. Keeping that engine round-trip sample makes the next unchanged frame
/// a pure cache hit instead of relying on bit-identical global recomposition.
#[must_use]
pub fn settle_wall_write(
    mut memory: WallTransformMemory,
    actual_local: Transform3D,
    parent: Option<Transform3D>,
) -> WallTransformMemory {
    memory.normalized_local = Some(actual_local);
    memory.normalized_parent = parent;
    if actual_local.is_finite() {
        memory.last_finite_local = Some(actual_local);
    }
    memory
}

fn wall_bases_close(a: Basis, b: Basis) -> bool {
    let eps = 1e-4;
    (a.col_a() - b.col_a()).length() < eps
        && (a.col_b() - b.col_b()).length() < eps
        && (a.col_c() - b.col_c()).length() < eps
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
    let length = sanitize_wall_length(length, 4.0).value;
    let half = length * 0.5;
    let lane = |value: f32| if value.is_finite() { value } else { 0.0 };
    let center = Vector3::new(lane(center.x), lane(center.y), lane(center.z));
    let shifted = |value: f32, delta: f64| {
        let result = (f64::from(value) + delta) as f32;
        if result.is_finite() { result } else { value }
    };
    if quadrant.is_multiple_of(2) {
        Vector4::new(
            shifted(center.x, -half),
            center.z,
            shifted(center.x, half),
            center.z,
        )
    } else {
        Vector4::new(
            center.x,
            shifted(center.z, -half),
            center.x,
            shifted(center.z, half),
        )
    }
}

/// One typed spawn the level walk found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCandidate {
    /// Where a designer can find the node: its path under the level root.
    pub path: String,
}

/// Which spawn datum the hero wakes at, and everything a designer has to
/// be told about the ones that did not win.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpawnVerdict {
    /// Index into the candidate slice of the datum the hero wakes at, or
    /// `None` when the caller must fall back to the level's own origin.
    pub winner: Option<usize>,
    /// Candidate indices ignored because the first typed spawn wins. The
    /// boundary uses these to attach the same diagnosis to every loser.
    pub losers: Vec<usize>,
    /// One printable line per thing that is wrong, in a fixed order.
    pub complaints: Vec<String>,
}

/// Choose by typed candidate count. The first depth-first walk candidate wins;
/// every later candidate is a loser regardless of its arbitrary scene name.
#[must_use]
pub fn choose_spawn(candidates: &[SpawnCandidate], fallback: Vector3) -> SpawnVerdict {
    let winner = (!candidates.is_empty()).then_some(0);
    let losers: Vec<usize> = (1..candidates.len()).collect();
    let mut complaints = Vec::new();
    match candidates.split_first() {
        None => complaints.push(format!(
            "WaveLevel: no WaveSpawn stands under the level — the hero has nowhere to wake, so \
             it wakes at the level's own origin, {fallback}. Add one WaveSpawn on the floor, \
             facing where the hero should look."
        )),
        Some((won, ignored)) if !ignored.is_empty() => complaints.push(format!(
            "WaveLevel: {} WaveSpawn nodes stand under the level — the hero wakes at the first \
             the level walk reaches, '{}', and ignores {}. Delete every extra WaveSpawn.",
            candidates.len(),
            won.path,
            quoted_paths(ignored.iter()),
        )),
        Some(_) => {}
    }
    SpawnVerdict {
        winner,
        losers,
        complaints,
    }
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

/// The complaint a wall earns when its own world box cannot be turned into
/// an occluder — a non-finite transform, a size that is not a number.
///
/// It is a NAMED sentence rather than a silent fallback because the wall
/// still occupies its slot in the table (`sight::Occluder::NOWHERE`):
/// it draws, and it stops nothing. Without a word, a level would look whole
/// while sound walked through one of its walls.
#[must_use]
pub fn unoccludable_wall(path: &str) -> PlacementFault {
    PlacementFault {
        path: path.to_string(),
        text: format!(
            "WaveLevel: '{path}' has no describable world box, so it occludes nothing — it will \
             draw, and sound will pass straight through it. Give it a finite transform and size."
        ),
    }
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
/// `game/tests/shader_contract_test.gd`.
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
pub fn occluder_budget(walls: usize, spanning: usize, slots: usize) -> Option<Budget> {
    let used = walls.saturating_add(spanning);
    // Only worth naming the second population when there IS one — a level
    // with no pillars should read exactly as it always did.
    let breakdown = if spanning == 0 {
        format!("{walls} walls")
    } else {
        format!(
            "{used} occluders ({walls} authored walls + {spanning} solids admitted by geometry: \
             floor-to-ceiling and at least {:.2} m thick, see level_plan::spans_the_corridor)",
            2.0 * WALL_T,
        )
    };
    if used > slots {
        return Some(Budget {
            severity: Severity::Error,
            text: format!(
                "WaveLevel: {breakdown} exceed the sight shaders' {slots} slots — the table keeps \
                 the first {slots} and drops {}, which stop occluding entirely: waves pass \
                 straight through them and no sight line counts them. Note that a pillar can cost \
                 a wall its slot: solids are appended after the walls, so the drops come off the \
                 end of whichever population runs over. Delete or merge walls, lower or thin a \
                 spanning solid so it stops qualifying, or raise MAXW (rust/src/sight.rs, \
                 mirrored in game/shaders/pulse_pool.gdshaderinc) — a measured decision and not a \
                 free one: every occluder is another rect in the per-fragment sight loop, on \
                 every platform.",
                used - slots,
            ),
        });
    }
    let headroom = slots - used;
    if headroom >= ROOM_SEGMENTS {
        return None; // room for another room: nothing worth saying
    }
    Some(Budget {
        severity: Severity::Warn,
        text: format!(
            "WaveLevel: {breakdown} against the sight shaders' {slots} slots — {headroom} \
             segments left, short of the {ROOM_SEGMENTS} another room costs (three sides plus the \
             doorway, which is the gap between two segments and so costs a segment of its own). \
             Every occluder past the last slot silently stops occluding. Raising MAXW \
             (rust/src/sight.rs, mirrored in game/shaders/pulse_pool.gdshaderinc) is a measured \
             decision and not a free one: every occluder is another rect in the per-fragment \
             sight loop, on every platform."
        ),
    })
}

/// The XZ bounding box of every wall centerline, standing floor to ceiling
/// (`y` 0..[`WALL_H`]) — belt-and-braces geometry for [`slab_diagonal`],
/// never the measure on its own. `None` for an empty table: a level with no
/// walls contributes no wall footprint, and a caller must not read that as
/// a real box at the origin.
///
/// A centerline is a QUAD, not an ordered pair, and BOTH ends of both axes
/// have to be read — [`wall_segment`] happens to sweep its ends in
/// ascending order today, which is exactly what makes the other half easy
/// to lose: a walk that read only `x` and `y` would still measure a
/// same-handed table correctly and would silently shrink any table whose
/// quads arrived the other way round.
fn wall_footprint(segments: &[Vector4], sweep: (f64, f64)) -> Option<Box3> {
    let first = segments.first()?;
    let (mut lo_x, mut hi_x) = (first.x.min(first.z), first.x.max(first.z));
    let (mut lo_z, mut hi_z) = (first.y.min(first.w), first.y.max(first.w));
    for s in segments {
        lo_x = lo_x.min(s.x).min(s.z);
        hi_x = hi_x.max(s.x).max(s.z);
        lo_z = lo_z.min(s.y).min(s.w);
        hi_z = hi_z.max(s.y).max(s.w);
    }
    Some(Box3 {
        min: [f64::from(lo_x), sweep.0, f64::from(lo_z)],
        max: [f64::from(hi_x), sweep.1, f64::from(hi_z)],
    })
}

/// The vertical extent the walls actually occupy, unioned over the whole
/// table — `(0.0, WALL_H)` on a level whose walls all stand on its floor,
/// and something else the moment one is lifted or the level root is.
///
/// Taken off the occluders because they are the only place a wall's own
/// height survives: [`wall_segment`] writes `(x1, z1, x2, z2)` and discards
/// it. An empty table answers `(0.0, 0.0)`, which contributes nothing to a
/// union rather than inventing a storey.
#[must_use]
pub fn wall_sweep(occluders: &[crate::sight::Occluder]) -> (f64, f64) {
    let mut span: Option<(f64, f64)> = None;
    for occ in occluders {
        let (lo, hi) = (f64::from(occ.span().x), f64::from(occ.span().y));
        span = Some(match span {
            Some((was_lo, was_hi)) => (was_lo.min(lo), was_hi.max(hi)),
            None => (lo, hi),
        });
    }
    span.unwrap_or((0.0, 0.0))
}

/// The longest sight line the authored map allows: the full 3D diagonal of
/// the drawn world's own outer shell.
///
/// THE HONEST MEASURE IS THE SLAB PAIR, not the wall centerlines
/// [`wall_footprint`] alone used to stand for the whole map. The floor and
/// ceiling slabs span the whole `extents` knob whether or not a single wall
/// stands on them — issue #45's courtyard blind spot was exactly this: a
/// large, sparsely walled room whose few short wall centerlines measured a
/// tiny footprint while the slab underfoot, which is what every silhouette
/// and every footstep actually draws against, reached far past shader
/// range in silence. So `floor` and `ceiling` — read off where the slabs
/// actually stand, in world space, never the raw `extents` knob a level
/// dropped off-origin would desync from — are unioned first and are never
/// optional: a level always has both slabs once built, so there is no
/// empty case to special-case away.
///
/// The wall footprint is unioned in on top, belt-and-braces: a wall is
/// authored to stand on its level's slab, but nothing stops one from
/// reaching past its edge, and drawn geometry outside the slab is still
/// geometry a sight line can reach. On every map that behaves — walls
/// resting within their own slab — this union changes nothing, since the
/// slab pair already contains it.
///
/// The walls contribute their centerlines in XZ — this measures DRAWN
/// geometry, so the occluder's shrunk sight rect would be the wrong
/// vocabulary here — but their VERTICAL extent has to be measured rather
/// than assumed. It used to be stamped as a global `[0, WALL_H]`, the last
/// global wall-height read in the crate, and it made this measure lie about
/// exactly the case the occluder rework exists for: lift a level ROOT by
/// 2.557 m and the slabs rise with it, but the injected `[0, 3]` stretched
/// the union from 3.2 m tall to 5.66 m and pushed the diagonal from 39.73
/// to 40.00002 — a `Severity::Error` telling a designer to shrink a map
/// whose true diagonal had not moved at all.
#[must_use]
pub fn slab_diagonal(floor: Box3, ceiling: Box3, walls: &[Vector4], sweep: (f64, f64)) -> f64 {
    let mut extent = floor.union(&ceiling);
    if let Some(wall_box) = wall_footprint(walls, sweep) {
        extent = extent.union(&wall_box);
    }
    extent.diagonal()
}

/// What the level must say about its own size against the range the sight
/// shaders pack camera distance into ([`DIST_PACK_RANGE`]). `None` while
/// the range strictly exceeds the diagonal, which is the shipped map's
/// state — 39.73 m against 40, 0.27 m of headroom — and must stay silent.
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
///    they are diffed out of the per-face label channel instead.
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
/// past it — a map that JUST reaches the range is already indistinguishable
/// from one that has outgrown it. The existing shader contract demands
/// `range > diagonal` for the same reason, and the two must agree.
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

/// One censused node's contribution to [`scene_signature`]: where a
/// designer finds it, which live Godot object currently occupies that address,
/// its global pose, and — for a solid — its skin mesh's LOCAL AABB (position
/// then size, six floats). Identity catches generation changes whose authored
/// geometry stays byte-identical: a WaveRun setter frees and recreates its
/// ownerless RunSeg walls at the same path, pose and AABB, and the fresh meshes
/// still need derivation. The AABB catches a `radius` or `size` knob reshaping
/// one existing object without moving it.
#[derive(Debug, Clone)]
pub struct SignatureNode {
    /// The node's address under the level root — the same handle every
    /// other derived report (`PlacedSolid`, `SpawnCandidate`) quotes.
    pub path: String,
    /// Opaque live-object generation supplied by the engine boundary. It is
    /// compared only by folding its bits; the pure planner never interprets
    /// or resolves an engine handle.
    pub instance_identity: i64,
    /// The node's global transform: basis columns (X, Y, Z, three floats
    /// each) then origin — twelve floats, the same twelve a `Transform3D`
    /// is built from.
    pub transform: [f32; 12],
    /// The skin mesh's local AABB (position then size), for a solid only.
    /// `None` for a wall's typed twin (never folded twice — see
    /// [`scene_signature`]), a source, a cat or a spawn datum, all of
    /// which carry no shape a knob can reshape independent of their pose.
    pub aabb: Option<[f32; 6]>,
}

/// FNV-1a's 64-bit offset basis and prime — the classic non-cryptographic
/// fold. Nothing here needs to resist an adversary: the signature exists
/// so the level can tell "changed" from "unchanged", never to be
/// unguessable, so the textbook constants are exactly enough machinery.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Fold one byte into a running FNV-1a hash.
#[must_use]
fn fnv_byte(hash: u64, byte: u8) -> u64 {
    (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
}

/// Fold a byte slice into a running hash, one byte at a time.
#[must_use]
fn fnv_bytes(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |h, &b| fnv_byte(h, b))
}

/// Fold one f32's raw bit pattern into a running hash — the BITS, not a
/// float comparison, so the fold is answering one question only ("did
/// ANYTHING change"), never "is this float equal to that one", which is
/// the comparison this whole mechanism exists to avoid needing.
#[must_use]
fn fnv_f32(hash: u64, value: f32) -> u64 {
    fnv_bytes(hash, &value.to_bits().to_le_bytes())
}

/// Fold one opaque engine identity without assigning it any domain meaning.
#[must_use]
fn fnv_i64(hash: u64, value: i64) -> u64 {
    fnv_bytes(hash, &value.to_le_bytes())
}

/// The level's condition-watch signature: one `u64` FNV-1a fold over the
/// level's own `extents` knob, then every censused node's path, live-object
/// identity, global transform and (for a solid) skin AABB, in SCENE order —
/// the same deterministic walk order every other derivation leans on. The
/// same unchanged live scene generation therefore always folds to the same
/// number, while reordering the Scene dock is a real edit and a newly loaded
/// copy deliberately carries new engine identities.
///
/// `extents` folds FIRST and outside the per-node loop, not as a synthetic
/// node, because it is not a censused node's property at all: it is read
/// straight off `WaveLevel` itself — `derive`'s `report_placement` reads
/// it through the floor slab's own world box, and the per-face paint pass
/// anchors the slabs' fixed role labels against that same box — so the fold
/// has to name it as
/// what it is, the level's own top-level condition, rather than dress it
/// up as a node with an invented path. A dedicated boundary byte (`2`,
/// distinct from the `0`/`1` presence bytes every node folds around its
/// AABB) closes the extents section before the first node's path bytes
/// begin, so nothing after it can be misread as more extents floats.
///
/// Every node folds a boundary byte after its path, its fixed-width opaque
/// identity, then a presence byte before its AABB (`1` then six floats, or a
/// bare `0`). The fold therefore distinguishes a replacement generation even
/// when every authored geometric byte agrees, and still notices a solid whose
/// AABB disappears while its mesh is being rebuilt.
#[must_use]
pub fn scene_signature(extents: [f32; 2], nodes: &[SignatureNode]) -> u64 {
    let mut hash = FNV_OFFSET;
    hash = fnv_f32(hash, extents[0]);
    hash = fnv_f32(hash, extents[1]);
    hash = fnv_byte(hash, 2); // extents/nodes boundary
    for node in nodes {
        hash = fnv_bytes(hash, node.path.as_bytes());
        hash = fnv_byte(hash, 0); // path/identity boundary
        hash = fnv_i64(hash, node.instance_identity);
        for &f in &node.transform {
            hash = fnv_f32(hash, f);
        }
        match node.aabb {
            Some(aabb) => {
                hash = fnv_byte(hash, 1); // "an aabb follows"
                for &f in &aabb {
                    hash = fnv_f32(hash, f);
                }
            }
            None => hash = fnv_byte(hash, 0), // "no aabb here"
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use godot::builtin::EulerOrder;

    use super::*;

    /// Removing either live door, or opening the runtime door, breaks this
    /// lifecycle table. Scene construction happens before tree entry; only
    /// the editor may keep changing authored geometry after ready.
    #[test]
    fn runtime_freezes_authored_geometry_but_construction_and_editor_stay_live() {
        assert!(authored_geometry_edit_is_live(false, false));
        assert!(authored_geometry_edit_is_live(false, true));
        assert!(authored_geometry_edit_is_live(true, true));
        assert!(!authored_geometry_edit_is_live(true, false));
    }

    #[test]
    fn divider_run_reproduces_both_shipped_segments() {
        let plan = run_segments(
            Vector2::new(6.4, 0.6),
            Vector2::new(6.4, 19.4),
            &[(8.0, 4.4)],
        );
        assert_eq!(plan.complaint, None);
        assert_eq!(plan.segments.len(), 2);
        assert_eq!(
            plan.segments[0],
            RunSeg {
                center: Vector2::new(6.4, 4.3),
                length: 7.4,
                vertical: true
            }
        );
        assert_eq!(
            plan.segments[1],
            RunSeg {
                center: Vector2::new(6.4, 15.9),
                length: 7.0,
                vertical: true
            }
        );
    }

    #[test]
    fn openings_are_absolute_axis_coordinates_and_negative_widths_are_magnitudes() {
        let plan = run_segments(
            Vector2::new(0.0, 2.0),
            Vector2::new(10.0, 2.0),
            &[(3.0, -2.0)],
        );
        assert_eq!(
            plan.segments,
            vec![
                RunSeg {
                    center: Vector2::new(1.5, 2.0),
                    length: 3.0,
                    vertical: false
                },
                RunSeg {
                    center: Vector2::new(7.5, 2.0),
                    length: 5.0,
                    vertical: false
                },
            ]
        );
    }

    #[test]
    fn reversed_endpoints_and_unsorted_overlapping_openings_normalize() {
        let plan = run_segments(
            Vector2::new(10.0, 1.0),
            Vector2::new(0.0, 1.0),
            &[(7.0, 2.0), (4.0, 4.0)],
        );
        assert_eq!(
            plan.segments,
            vec![
                RunSeg {
                    center: Vector2::new(2.0, 1.0),
                    length: 4.0,
                    vertical: false
                },
                RunSeg {
                    center: Vector2::new(9.5, 1.0),
                    length: 1.0,
                    vertical: false
                },
            ]
        );
    }

    #[test]
    fn openings_clamp_to_the_run_and_nonpositive_residue_disappears() {
        let plan = run_segments(
            Vector2::new(0.0, 0.0),
            Vector2::new(5.0, 0.0),
            &[(0.0, 4.0), (4.0, 6.0)],
        );
        assert!(plan.segments.is_empty());
    }

    #[test]
    fn a_run_without_openings_is_one_positive_segment() {
        let plan = run_segments(Vector2::new(2.0, 3.0), Vector2::new(2.0, 9.0), &[]);
        assert_eq!(
            plan.segments,
            vec![RunSeg {
                center: Vector2::new(2.0, 6.0),
                length: 6.0,
                vertical: true
            }]
        );
    }

    #[test]
    fn a_diagonal_folds_to_the_dominant_axis_with_x_winning_ties() {
        let plan = run_segments(Vector2::new(1.0, 2.0), Vector2::new(5.0, 6.0), &[]);
        assert_eq!(
            plan.segments[0],
            RunSeg {
                center: Vector2::new(3.0, 2.0),
                length: 4.0,
                vertical: false
            }
        );
        assert_eq!(
            plan.complaint.as_deref(),
            Some(
                "WaveRun: diagonal endpoints folded onto the dominant X axis — runs emit axis-aligned WaveWalls; move from/to onto one X or Z line to clear this warning."
            )
        );
    }

    #[test]
    fn zero_and_nonfinite_runs_are_total_and_emit_nothing() {
        assert!(
            run_segments(Vector2::ZERO, Vector2::ZERO, &[])
                .segments
                .is_empty()
        );
        assert!(
            run_segments(Vector2::new(f32::NAN, 0.0), Vector2::ONE, &[])
                .segments
                .is_empty()
        );
        assert!(
            run_segments(Vector2::ZERO, Vector2::new(f32::INFINITY, 0.0), &[])
                .segments
                .is_empty()
        );
    }

    /// Finite endpoints can still overflow f32 subtraction/midpoint math.
    /// They are admitted by Vector2 and the Inspector boundary, so the
    /// planner must refuse them rather than emit an infinite center/length.
    #[test]
    fn extreme_finite_runs_are_refused_without_infinite_segments() {
        let across_domain = run_segments(
            Vector2::new(-f32::MAX, 0.0),
            Vector2::new(f32::MAX, 0.0),
            &[],
        );
        assert!(across_domain.segments.is_empty());
        assert_eq!(
            across_domain.complaint.as_deref(),
            Some(
                "WaveRun: from/to are finite but their derived wall exceeds the finite \
                 coordinate range — no walls were emitted; move the endpoints closer to the \
                 level."
            )
        );

        let same_side = run_segments(
            Vector2::new(f32::MAX * 0.75, 0.0),
            Vector2::new(f32::MAX, 0.0),
            &[],
        );
        assert_eq!(same_side.segments.len(), 1);
        for segment in same_side.segments {
            assert!(segment.center.x.is_finite());
            assert!(segment.center.y.is_finite());
            assert!(segment.length.is_finite());
        }
    }

    /// Positive is the authored lower bound, not AXIS_EPS: a short run may
    /// be impractical, but silently deleting it would make the scene and
    /// derived wall table disagree. The literal is half the axis-alignment
    /// tolerance to catch using that tolerance as a length cutoff.
    #[test]
    fn a_sub_millimetre_positive_run_is_still_emitted() {
        let plan = run_segments(Vector2::ZERO, Vector2::new(0.0005, 0.0), &[]);
        assert_eq!(plan.complaint, None);
        assert_eq!(plan.segments.len(), 1);
        assert_eq!(plan.segments[0].length, 0.0005);
    }

    /// Two doorway cuts separated by a real 0.5 mm wall remnant must not be
    /// merged merely because they are within the diagonal/alignment epsilon.
    #[test]
    fn every_positive_residual_between_openings_is_emitted() {
        let plan = run_segments(
            Vector2::ZERO,
            Vector2::new(2.0, 0.0),
            &[(0.0, 0.5), (0.5005, 0.5)],
        );
        assert_eq!(plan.segments.len(), 2);
        assert!((plan.segments[0].length - 0.0005).abs() < 1e-7);
        assert!((plan.segments[1].length - 0.9995).abs() < 1e-7);
    }

    /// The pure transform law carries endpoints and absolute-axis opening
    /// coordinates together. Translation followed by a quarter turn is
    /// hand-derived from Godot's column basis: the X run becomes a reversed
    /// Z run, and the old [1,3] doorway becomes the new [1,3] interval.
    #[test]
    fn run_pose_absorption_maps_endpoints_and_openings_together() {
        let authored = RunAuthoring {
            from: [0.0, 0.0],
            to: [4.0, 0.0],
            openings: vec![[1.0, 2.0]],
        };
        let pose = RunPose {
            origin: [3.0, 0.0, 4.0],
            columns: [[0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
        };
        assert_eq!(
            absorb_run_pose(&authored, pose),
            RunPoseResult::Applied {
                authored: RunAuthoring {
                    from: [3.0, 4.0],
                    to: [3.0, 0.0],
                    openings: vec![[1.0, 2.0]],
                },
                warning: None,
            }
        );
    }

    /// Y scale cannot be represented by planar authoring data, but its X/Z
    /// projection is still exact and finite. The result explicitly carries
    /// that information loss so the boundary can store and voice it.
    #[test]
    fn y_scale_projects_planarly_and_reports_the_loss() {
        let authored = RunAuthoring {
            from: [0.0, 0.0],
            to: [4.0, 0.0],
            openings: vec![],
        };
        let pose = RunPose {
            origin: [1.0, 0.0, 2.0],
            columns: [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]],
        };
        assert_eq!(
            absorb_run_pose(&authored, pose),
            RunPoseResult::Applied {
                authored: RunAuthoring {
                    from: [1.0, 2.0],
                    to: [5.0, 2.0],
                    openings: vec![],
                },
                warning: Some(RunPoseWarning::VerticalLoss),
            }
        );

        let inverted_y = RunPose {
            origin: [0.0, 0.0, 0.0],
            columns: [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        assert_eq!(
            absorb_run_pose(&authored, inverted_y),
            RunPoseResult::Applied {
                authored,
                warning: Some(RunPoseWarning::VerticalLoss),
            }
        );
    }

    /// Untrusted Inspector/scene data must never poison the saved endpoint
    /// state. Non-finite transform lanes and finite arithmetic that exceeds
    /// f32's exported domain are rejected without returning replacement data.
    #[test]
    fn invalid_or_overflowing_poses_are_rejected_before_mapping() {
        let authored = RunAuthoring {
            from: [1.0, 2.0],
            to: [3.0, 4.0],
            openings: vec![[2.0, 0.5]],
        };
        let mut nonfinite = RunPose::IDENTITY;
        nonfinite.origin[0] = f64::NAN;
        assert_eq!(
            absorb_run_pose(&authored, nonfinite),
            RunPoseResult::Rejected(RunPoseWarning::NonFiniteTransform)
        );

        let mut huge = RunPose::IDENTITY;
        huge.columns[0][0] = f64::from(f32::MAX);
        let large_authored = RunAuthoring {
            from: [f64::from(f32::MAX), 0.0],
            to: [f64::from(f32::MAX), 1.0],
            openings: vec![],
        };
        assert_eq!(
            absorb_run_pose(&large_authored, huge),
            RunPoseResult::Rejected(RunPoseWarning::ProjectionOutOfRange)
        );
    }

    /// A pose cannot repair already-poisoned endpoint/opening data. Refuse
    /// the absorption and leave the existing run warning to name that data.
    #[test]
    fn nonfinite_authored_run_data_is_not_transformed() {
        let authored = RunAuthoring {
            from: [f64::NAN, 0.0],
            to: [4.0, 0.0],
            openings: vec![],
        };
        assert_eq!(
            absorb_run_pose(&authored, RunPose::IDENTITY),
            RunPoseResult::Rejected(RunPoseWarning::NonFiniteAuthoring)
        );
    }

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

    /// One typed spawn found by the level walk.
    fn spawn(path: &str) -> SpawnCandidate {
        SpawnCandidate { path: path.into() }
    }

    /// The level's own origin lifted to capsule height on the shipped map —
    /// the corner the hero is dumped in when no marker names itself.
    const CORNER: Vector3 = Vector3::new(0.0, 0.9, 0.0);

    /// Removing the only candidate must change this from a silent winner to
    /// the fallback branch.
    #[test]
    fn one_typed_spawn_wins_in_silence() {
        let verdict = choose_spawn(&[spawn("Start")], CORNER);
        assert_eq!(verdict.winner, Some(0));
        assert!(verdict.losers.is_empty());
        assert!(verdict.complaints.is_empty(), "{:?}", verdict.complaints);
    }

    /// Deleting the duplicate complaint or choosing the last node must make
    /// this fail: walk order is the deterministic tiebreak and the loser is
    /// named independently of either node's arbitrary scene name.
    #[test]
    fn a_second_typed_spawn_loses_and_is_named_by_path() {
        let verdict = choose_spawn(&[spawn("Start"), spawn("Rooms/Other")], CORNER);
        assert_eq!(verdict.winner, Some(0));
        assert_eq!(verdict.losers, vec![1]);
        assert_eq!(
            verdict.complaints,
            vec![
                "WaveLevel: 2 WaveSpawn nodes stand under the level — the hero wakes at the \
                 first the level walk reaches, 'Start', and ignores 'Rooms/Other'. Delete \
                 every extra WaveSpawn."
            ]
        );
    }

    /// Reporting only the first duplicate must fail: every losing typed node
    /// gets its own warning triangle, in walk order.
    #[test]
    fn three_typed_spawns_keep_the_first_and_name_every_loser() {
        let verdict = choose_spawn(
            &[spawn("Start"), spawn("East/Arrival"), spawn("West/Arrival")],
            CORNER,
        );
        assert_eq!(verdict.winner, Some(0));
        assert_eq!(verdict.losers, vec![1, 2]);
        assert_eq!(
            verdict.complaints,
            vec![
                "WaveLevel: 3 WaveSpawn nodes stand under the level — the hero wakes at the \
                 first the level walk reaches, 'Start', and ignores 'East/Arrival', \
                 'West/Arrival'. Delete every extra WaveSpawn."
            ]
        );
    }

    /// Returning origin silently would strand the hero without telling the
    /// designer which typed datum is missing.
    #[test]
    fn no_typed_spawn_names_the_fallback() {
        let verdict = choose_spawn(&[], Vector3::new(14.0, 0.9, 2.5));
        assert_eq!(verdict.winner, None);
        assert!(verdict.losers.is_empty());
        assert_eq!(
            verdict.complaints,
            vec![
                "WaveLevel: no WaveSpawn stands under the level — the hero has nowhere to \
                 wake, so it wakes at the level's own origin, (14, 0.9, 2.5). Add one \
                 WaveSpawn on the floor, facing where the hero should look."
            ]
        );
    }

    /// Reading local yaw would answer zero for the nested quarter-turn case;
    /// poisoned or degenerate bases must stay finite rather than leaking NaN.
    #[test]
    fn global_basis_heading_is_total_and_reads_the_forward_axis() {
        assert_eq!(basis_heading(quadrant_basis(1)), FRAC_PI_2);
        assert_eq!(basis_heading(Basis::default()), 0.0);
        let nan = Vector3::new(f32::NAN, f32::NAN, f32::NAN);
        let poisoned = Basis::from_cols(nan, Vector3::UP, nan);
        assert_eq!(basis_heading(poisoned), 0.0);
        let infinity = Vector3::new(f32::INFINITY, 0.0, 1.0);
        let poisoned = Basis::from_cols(infinity, Vector3::UP, Vector3::FORWARD);
        assert_eq!(basis_heading(poisoned), 0.0);
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

    /// A finite f64 can still overflow the f32 mesh/collider boundary. The
    /// knob keeps its last valid geometry for every non-finite or narrowing-
    /// overflow input, while ordinary signs remain magnitudes.
    #[test]
    fn wall_length_refuses_every_value_that_would_poison_godot_geometry() {
        assert_eq!(
            sanitize_wall_length(-7.4, 4.0),
            WallLengthPlan {
                value: 7.4,
                repaired: false
            }
        );
        for bad in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            1.0e300,
            f64::MAX,
        ] {
            assert_eq!(
                sanitize_wall_length(bad, 9.0),
                WallLengthPlan {
                    value: 9.0,
                    repaired: true
                }
            );
        }
        assert_eq!(sanitize_wall_length(f64::NAN, f64::NAN).value, 4.0);
        assert!(wall_box(sanitize_wall_length(f64::MAX, 9.0).value).is_finite());
        for malformed in [
            Vector3::new(f32::MAX, 0.0, f32::MAX),
            Vector3::new(f32::NAN, 0.0, f32::INFINITY),
        ] {
            for quadrant in 0..4 {
                assert!(wall_segment(malformed, f64::MAX, quadrant).is_finite());
            }
        }
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

    /// Live editor transforms carry scale and free-hand rotation back onto a
    /// wall after `_ready`. Normalizing the inherited WORLD basis must return
    /// the same exact unit quadrant family as initial placement, including a
    /// deterministic identity fallback for poisoned input.
    #[test]
    fn wall_basis_normalization_discards_inherited_scale_exactly() {
        let inherited = Basis::from_cols(
            Vector3::new(0.0, 0.0, -0.5),
            Vector3::new(0.0, 3.0, 0.0),
            Vector3::new(2.0, 0.0, 0.0),
        );
        let snapped = normalized_wall_basis(inherited);
        assert_eq!(snapped.col_a(), Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(snapped.col_b(), Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(snapped.col_c(), Vector3::new(1.0, 0.0, 0.0));

        let poisoned = Basis::from_cols(
            Vector3::new(f32::NAN, 0.0, f32::INFINITY),
            Vector3::ZERO,
            Vector3::ZERO,
        );
        assert_eq!(normalized_wall_basis(poisoned), Basis::IDENTITY);
    }

    /// A global basis can only be written through an ancestor whose inverse
    /// exists and remains finite. Zero scale and poisoned transforms are
    /// ordinary editor inputs, not permission for the adapter to call into
    /// Godot's singular affine inverse or retry that failed write forever.
    #[test]
    fn wall_parent_transform_classifies_every_representability_failure() {
        assert_eq!(
            wall_parent_transform_state(None, Transform3D::IDENTITY),
            WallParentTransformState::Representable
        );
        assert_eq!(
            wall_parent_transform_state(Some(Transform3D::IDENTITY), Transform3D::IDENTITY),
            WallParentTransformState::Representable
        );
        assert_eq!(
            wall_parent_transform_state(
                Some(Transform3D::new(
                    Basis::from_scale(Vector3::new(-2.0, 3.0, 0.5)),
                    Vector3::new(7.0, 0.0, 11.0),
                )),
                Transform3D::IDENTITY,
            ),
            WallParentTransformState::Representable
        );
        assert_eq!(
            wall_parent_transform_state(
                Some(Transform3D::new(
                    Basis::from_scale(Vector3::new(1.0, 0.0, 1.0)),
                    Vector3::ZERO,
                )),
                Transform3D::IDENTITY,
            ),
            WallParentTransformState::Singular
        );
        assert_eq!(
            wall_parent_transform_state(
                Some(Transform3D::new(
                    Basis::IDENTITY,
                    Vector3::new(f32::NAN, 0.0, 0.0),
                )),
                Transform3D::IDENTITY,
            ),
            WallParentTransformState::NonFinite
        );
        assert_eq!(
            wall_parent_transform_state(
                Some(Transform3D::new(
                    Basis::from_scale(Vector3::new(f32::MAX, f32::MAX, f32::MAX)),
                    Vector3::ZERO,
                )),
                Transform3D::IDENTITY,
            ),
            WallParentTransformState::NonFinite,
            "a finite basis whose determinant overflows must be rejected before inverse()"
        );
        assert_eq!(
            wall_parent_transform_state(
                Some(Transform3D::new(
                    Basis::from_cols(
                        Vector3::new(f32::INFINITY, 0.0, 0.0),
                        Vector3::UP,
                        Vector3::BACK,
                    ),
                    Vector3::ZERO,
                )),
                Transform3D::IDENTITY,
            ),
            WallParentTransformState::NonFinite
        );
        let subnormal = f32::from_bits(1);
        assert_eq!(
            wall_parent_transform_state(
                Some(Transform3D::new(
                    Basis::from_scale(Vector3::new(subnormal, 1.0, 1.0)),
                    Vector3::ZERO,
                )),
                Transform3D::IDENTITY,
            ),
            WallParentTransformState::NonFinite
        );
        assert_eq!(
            wall_parent_transform_state(
                Some(Transform3D::new(
                    Basis::from_scale(Vector3::new(0.01, 1.0, 1.0)),
                    Vector3::new(f32::MAX, 0.0, 0.0),
                )),
                Transform3D::IDENTITY,
            ),
            WallParentTransformState::NonFinite
        );
        assert_eq!(
            wall_parent_transform_state(
                Some(Transform3D::IDENTITY),
                Transform3D::new(Basis::IDENTITY, Vector3::new(f32::INFINITY, 0.0, 0.0)),
            ),
            WallParentTransformState::NonFinite
        );
    }

    /// A poisoned wall transform is repaired lane-by-lane from its last valid
    /// placement: finite position input survives, the invalid lanes fall back,
    /// and a valid planar X direction still chooses the authored quadrant even
    /// if an irrelevant basis lane was NaN.
    #[test]
    fn wall_transform_recovery_preserves_every_finite_authored_lane() {
        let fallback = Transform3D::new(quadrant_basis(1), Vector3::new(7.0, 2.0, 11.0));
        let poisoned = Transform3D::new(
            Basis::from_cols(
                Vector3::new(-1.0, 0.0, 0.0),
                Vector3::new(0.0, f32::NAN, 0.0),
                Vector3::new(0.0, 0.0, f32::INFINITY),
            ),
            Vector3::new(f32::NAN, 5.0, f32::INFINITY),
        );
        let recovered = recover_wall_transform(poisoned, Some(fallback));
        assert_eq!(recovered.origin, Vector3::new(7.0, 5.0, 11.0));
        assert_eq!(recovered.basis, quadrant_basis(2));
        assert!(recovered.is_finite());

        let no_history = recover_wall_transform(
            Transform3D::new(
                Basis::from_cols(
                    Vector3::new(f32::NAN, 0.0, f32::INFINITY),
                    Vector3::ZERO,
                    Vector3::ZERO,
                ),
                Vector3::new(f32::NAN, f32::NEG_INFINITY, 3.0),
            ),
            None,
        );
        assert_eq!(no_history.origin, Vector3::new(0.0, 0.0, 3.0));
        assert_eq!(no_history.basis, Basis::IDENTITY);
        assert!(no_history.is_finite());
    }

    /// Recovery happens before parent composition. With IEEE arithmetic one
    /// infinite local lane contaminates every world lane through zero-times-
    /// infinity terms, but the finite authored Y value must still survive.
    #[test]
    fn parented_wall_recovery_preserves_finite_local_lanes_before_composition() {
        let parent = Transform3D::new(Basis::IDENTITY, Vector3::new(8.0, 0.0, 12.0));
        let initial = plan_wall_transform(
            parent,
            Transform3D::IDENTITY,
            Some(parent),
            WallTransformMemory::default(),
        );
        let memory = initial.memory;
        let poisoned_local =
            Transform3D::new(Basis::IDENTITY, Vector3::new(f32::NAN, 5.0, f32::INFINITY));
        let contaminated_global = parent * poisoned_local;
        assert!(!contaminated_global.is_finite());

        let repaired =
            plan_wall_transform(contaminated_global, poisoned_local, Some(parent), memory);
        assert_eq!(repaired.memory.fault, Some(WallTransformFault::Own));
        assert_eq!(
            repaired.write_global.expect("poison must be repaired"),
            Transform3D::new(Basis::IDENTITY, Vector3::new(8.0, 5.0, 12.0))
        );
    }

    /// Collision priority accepts only positive finite f32 values. Every
    /// malformed reading retains the last valid value (or Godot's default),
    /// including a malformed fallback supplied by an untrusted caller.
    #[test]
    fn wall_priority_repairs_every_value_godot_refuses() {
        assert_eq!(sanitize_wall_priority(2.5, None), (2.5, false));
        for invalid_priority in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                sanitize_wall_priority(invalid_priority, Some(2.5)),
                (2.5, true)
            );
        }
        assert_eq!(sanitize_wall_priority(0.0, Some(-4.0)), (1.0, true));
        assert_eq!(sanitize_wall_priority(0.0, Some(f32::NAN)), (1.0, true));
    }

    /// The backing field is repaired without re-entering the setter. Any later
    /// valid setter call is therefore a genuine acknowledgment, including a
    /// designer re-entering the displayed repaired value.
    #[test]
    fn wall_priority_repair_warning_clears_on_the_next_valid_setter_call() {
        let first = plan_wall_priority(f32::NAN, WallPriorityMemory::default());
        assert!(first.repaired);
        assert!(first.warn);
        assert_eq!(first.value, 1.0);

        let echo = plan_wall_priority(first.value, first.memory);
        assert!(!echo.repaired);
        assert!(!echo.warn);

        let edited = plan_wall_priority(2.5, first.memory);
        assert!(!edited.repaired);
        assert!(!edited.warn);
    }

    /// A finite wall never owns its ancestor's poison. Once the ancestor is
    /// repaired the warning clears without demanding an unrelated wall move;
    /// finite composition overflow is classified the same way.
    #[test]
    fn wall_transform_attributes_parent_poison_and_overflow_to_the_parent() {
        let local = Transform3D::new(Basis::IDENTITY, Vector3::new(1.0, 2.0, 3.0));
        let poisoned_parent = Transform3D::new(Basis::IDENTITY, Vector3::new(f32::NAN, 0.0, 0.0));
        let blocked = plan_wall_transform(
            poisoned_parent * local,
            local,
            Some(poisoned_parent),
            WallTransformMemory::default(),
        );
        assert_eq!(blocked.memory.fault, Some(WallTransformFault::Ancestor));
        assert!(!blocked.memory.pending_own_acknowledgment);

        let repaired =
            plan_wall_transform(local, local, Some(Transform3D::IDENTITY), blocked.memory);
        assert_eq!(repaired.memory.fault, None);
        assert!(!repaired.memory.pending_own_acknowledgment);

        let overflowing_parent = Transform3D::new(
            Basis::from_scale(Vector3::new(f32::MAX, 1.0, 1.0)),
            Vector3::ZERO,
        );
        let overflow_local = Transform3D::new(Basis::IDENTITY, Vector3::new(2.0, 2.0, 3.0));
        let overflow = plan_wall_transform(
            overflowing_parent * overflow_local,
            overflow_local,
            Some(overflowing_parent),
            WallTransformMemory::default(),
        );
        assert_eq!(overflow.memory.fault, Some(WallTransformFault::Ancestor));
        assert!(!overflow.memory.pending_own_acknowledgment);
        assert!(overflow.write_global.is_none());
    }

    /// The live-wall transition is stable across engine float round-trips,
    /// holds a poisoned-input warning across idle frames, and clears it only
    /// after a genuinely new finite authored placement. This pins the state
    /// machine independently of Godot process ordering.
    #[test]
    fn wall_transform_plan_settles_and_keeps_recovery_faults_until_an_edit() {
        let parent = Transform3D::new(
            Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, 0.73, 0.0))
                .scaled(Vector3::new(2.0, 3.0, 0.5)),
            Vector3::new(7.0, 0.0, 11.0),
        );
        let current = parent;
        let first = plan_wall_transform(
            current,
            Transform3D::IDENTITY,
            Some(parent),
            WallTransformMemory::default(),
        );
        assert!(first.write_global.is_some());
        let memory = settle_wall_write(first.memory, Transform3D::IDENTITY, Some(parent));
        let settled = plan_wall_transform(
            Transform3D::new(
                Basis::from_cols(
                    Vector3::new(1.0, 0.0, 7.0e-9),
                    Vector3::UP,
                    Vector3::new(-7.0e-9, 0.0, 1.0),
                ),
                Vector3::new(7.0, 0.0, 11.0),
            ),
            Transform3D::IDENTITY,
            Some(parent),
            memory,
        );
        assert!(settled.write_global.is_none());
        assert_eq!(settled.memory, memory);

        let poisoned =
            Transform3D::new(Basis::IDENTITY, Vector3::new(f32::NAN, 5.0, f32::INFINITY));
        let repaired =
            plan_wall_transform(poisoned, poisoned, None, WallTransformMemory::default());
        assert_eq!(repaired.memory.fault, Some(WallTransformFault::Own));
        let repaired_global = repaired.write_global.expect("poison must be repaired");
        let repaired_memory = settle_wall_write(repaired.memory, repaired_global, None);
        for _ in 0..3 {
            let idle = plan_wall_transform(repaired_global, repaired_global, None, repaired_memory);
            assert_eq!(idle.memory.fault, Some(WallTransformFault::Own));
            assert!(idle.write_global.is_none());
        }
        let parent_only = Transform3D::new(Basis::IDENTITY, Vector3::new(9.0, 0.0, 12.0));
        let inherited_move = plan_wall_transform(
            parent_only * repaired_global,
            repaired_global,
            Some(parent_only),
            repaired_memory,
        );
        assert_eq!(
            inherited_move.memory.fault,
            Some(WallTransformFault::Own),
            "moving only an ancestor must not acknowledge the wall's own repaired input"
        );
        let singular_parent = Transform3D::new(
            Basis::from_scale(Vector3::new(0.0, 1.0, 1.0)),
            Vector3::ZERO,
        );
        let blocked = plan_wall_transform(
            repaired_global,
            repaired_global,
            Some(singular_parent),
            repaired_memory,
        );
        assert_eq!(blocked.memory.fault, Some(WallTransformFault::Ancestor));
        assert!(blocked.memory.pending_own_acknowledgment);
        let unblocked = plan_wall_transform(
            repaired_global,
            repaired_global,
            Some(Transform3D::IDENTITY),
            blocked.memory,
        );
        assert_eq!(
            unblocked.memory.fault,
            Some(WallTransformFault::Own),
            "repairing an ancestor must restore the still-unacknowledged own fault"
        );
        let changed_while_blocked = Transform3D::new(Basis::IDENTITY, Vector3::new(3.0, 5.0, 0.0));
        let acknowledged_while_blocked = plan_wall_transform(
            singular_parent * changed_while_blocked,
            changed_while_blocked,
            Some(singular_parent),
            blocked.memory,
        );
        assert_eq!(
            acknowledged_while_blocked.memory.fault,
            Some(WallTransformFault::Ancestor)
        );
        assert!(!acknowledged_while_blocked.memory.pending_own_acknowledgment);
        let repaired_after_acknowledgment = plan_wall_transform(
            changed_while_blocked,
            changed_while_blocked,
            Some(Transform3D::IDENTITY),
            acknowledged_while_blocked.memory,
        );
        assert_eq!(repaired_after_acknowledgment.memory.fault, None);
        let moved = Transform3D::new(Basis::IDENTITY, Vector3::new(4.0, 5.0, 6.0));
        let acknowledged = plan_wall_transform(moved, moved, None, unblocked.memory);
        assert_eq!(acknowledged.memory.fault, None);
        assert!(!acknowledged.memory.pending_own_acknowledgment);
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
        let occluders: Vec<crate::sight::Occluder> = retired_map_walls()
            .iter()
            .map(|s| crate::sight::Occluder::new(*s, 0.0, WALL_H).expect("a floor-standing wall"))
            .collect();
        assert_eq!(
            crate::sight::crossings_from(plan.point, Vector3::new(4.0, 0.8, 4.0), &occluders),
            0,
        );
        assert_eq!(
            crate::sight::crossings_from(plan.point, Vector3::new(8.0, 0.8, 4.0), &occluders),
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
        assert_eq!(occluder_budget(19, 0, 32), None);
        assert_eq!(occluder_budget(0, 0, 32), None);
        assert_eq!(occluder_budget(28, 0, 32), None);
    }

    /// The heads-up fires one room short of the ceiling and quotes THE
    /// HEADROOM, because how much room is left is the number a designer
    /// can act on — "you have 29 walls" is not. 32 − 29 = 3 segments,
    /// one short of the four another room costs.
    #[test]
    fn a_level_one_room_short_of_the_slots_reports_the_headroom_left() {
        let budget = occluder_budget(29, 0, 32).expect("a heads-up");
        assert_eq!(budget.severity, Severity::Warn);
        assert_eq!(
            budget.text,
            "WaveLevel: 29 walls against the sight shaders' 32 slots — 3 segments left, short of \
             the 4 another room costs (three sides plus the doorway, which is the gap between two \
             segments and so costs a segment of its own). Every occluder past the last slot \
             silently stops occluding. Raising MAXW (rust/src/sight.rs, mirrored in \
             game/shaders/pulse_pool.gdshaderinc) is a measured decision and not a free one: \
             every occluder is another rect in the per-fragment sight loop, on every platform."
        );
    }

    /// A level standing exactly ON the ceiling is not broken YET — 32 walls
    /// fit 32 slots and nothing is truncated — so it warns rather than
    /// errors, with zero headroom. Erroring here would cry wolf on a legal
    /// level; staying silent would hide that the very next wall is dropped.
    #[test]
    fn a_level_that_fills_every_slot_warns_with_no_headroom_left() {
        let budget = occluder_budget(32, 0, 32).expect("a heads-up");
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
        let budget = occluder_budget(33, 0, 32).expect("an error");
        assert_eq!(budget.severity, Severity::Error);
        // a level with no spanning solids must still read exactly as it
        // always did — the second population is named only when it exists
        assert!(
            budget.text.starts_with(
                "WaveLevel: 33 walls exceed the sight shaders' 32 slots — the table keeps the \
                 first 32 and drops 1, which stop occluding entirely"
            ),
            "{}",
            budget.text
        );
        assert!(
            !budget.text.contains("admitted by geometry"),
            "{}",
            budget.text
        );
        let far_past = occluder_budget(40, 0, 32).expect("an error");
        assert!(far_past.text.contains("and drops 8,"), "{}", far_past.text);
    }

    /// THE BREAK: a designer being told to delete walls when what actually
    /// consumed the slots was a pillar they dragged in — the message that
    /// counts only walls cannot say so, and they would go looking for a
    /// wall that does not exist.
    ///
    /// Both counts must appear, and the total must be their sum: solids are
    /// APPENDED after the walls, so the population that overflows is
    /// whichever one runs off the end.
    #[test]
    fn a_pillar_that_costs_a_wall_its_slot_says_so() {
        let budget = occluder_budget(30, 4, 32).expect("an error");
        assert_eq!(budget.severity, Severity::Error);
        assert!(budget.text.contains("34 occluders"), "{}", budget.text);
        assert!(
            budget
                .text
                .contains("30 authored walls + 4 solids admitted by geometry"),
            "{}",
            budget.text
        );
        assert!(budget.text.contains("and drops 2,"), "{}", budget.text);
        assert!(
            budget.text.contains("a pillar can cost a wall its slot"),
            "{}",
            budget.text
        );
        // and the way out that does not involve deleting a wall
        assert!(
            budget.text.contains("lower or thin a spanning solid"),
            "{}",
            budget.text
        );
    }

    /// The shipped level, measured off `game/scenes/level_01.tscn`: 19
    /// authored wall segments and 7 floor-to-ceiling pillars. It must fit
    /// with room to spare and say nothing at all — if this ever starts
    /// warning, the admission rule has widened past the pillars.
    #[test]
    fn the_shipped_level_fits_its_pillars_without_a_word() {
        assert_eq!(occluder_budget(19, 7, 32), None);
        // What the pillars actually cost, stated rather than glossed. 26 of
        // 32 slots leaves 6, which clears the 4 a room costs — so the
        // shipped level is silent. But the NEXT room lands at 30 and warns.
        // Before the pillars were admitted the same level sat at 19 with
        // room for three more. Seven slots is close to two rooms of
        // headroom, spent on occlusion a player can actually hear, and the
        // warning is where the designer finds out.
        assert!(
            occluder_budget(19 + ROOM_SEGMENTS, 7, 32).is_some(),
            "the very next room must warn — 26 of 32 leaves 6 slots and a \
             room costs 4, so adding one drops the headroom under the \
             threshold. If this goes quiet the budget has stopped counting \
             the pillars."
        );
    }

    /// Total on the degenerate budget a caller could hand it: a shader with
    /// no slots at all overflows rather than subtracting past zero, which
    /// on a `usize` is not a small number but a colossal one.
    #[test]
    fn a_slotless_shader_is_reported_not_subtracted_past_zero() {
        assert_eq!(
            occluder_budget(0, 0, 0).map(|b| b.severity),
            Some(Severity::Warn)
        );
        assert_eq!(
            occluder_budget(1, 0, 0).map(|b| b.severity),
            Some(Severity::Error)
        );
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

    /// The floor and ceiling slab boxes a level of `x` by `z` extents
    /// builds (`rust/src/nodes/level.rs`'s `slab_center`/`build_slabs`,
    /// hand-transcribed rather than called into): both slabs span `0..x` by
    /// `0..z` in the horizontal plane, the floor's top sits at `y = 0` and
    /// the ceiling's underside at `y = WALL_H`, each [`SLAB_T`] thick on
    /// the far side of that face from the room.
    fn slab_boxes(x: f64, z: f64) -> (Box3, Box3) {
        let floor = Box3 {
            min: [0.0, -SLAB_T, 0.0],
            max: [x, 0.0, z],
        };
        let ceiling = Box3 {
            min: [0.0, WALL_H, 0.0],
            max: [x, WALL_H + SLAB_T, z],
        };
        (floor, ceiling)
    }

    /// `None` for an empty table: a level with no walls contributes no wall
    /// footprint, and a caller must not read that as a real box at the
    /// origin.
    /// The sentence a wall earns when its own world box cannot be turned
    /// into an occluder — pinned, because its direct sibling
    /// `source_paint_fault_text` is pinned byte-for-byte and this one was
    /// not: deleting the word "occludes" from it failed nothing.
    ///
    /// It has to say three things, because a designer reading it in the
    /// Scene dock has to act: WHICH node, that the wall still DRAWS, and
    /// that sound passes THROUGH it. A wall that silently stopped occluding
    /// looks exactly like a wall that works.
    #[test]
    fn an_unoccludable_wall_says_which_node_and_what_it_costs() {
        let fault = unoccludable_wall("Rooms/North/CrookedWall");
        assert_eq!(fault.path, "Rooms/North/CrookedWall");
        assert!(fault.text.contains("Rooms/North/CrookedWall"));
        assert!(fault.text.contains("occludes nothing"));
        assert!(fault.text.contains("draw"));
        assert!(fault.text.contains("pass straight through"));
    }

    #[test]
    fn wall_footprint_is_none_for_an_empty_table() {
        assert_eq!(wall_footprint(&[], (0.0, WALL_H)), None);
    }

    /// THE break this catches: a footprint that stamps a global height on
    /// every wall instead of reading each wall's own sweep.
    ///
    /// It did, and it was the last global wall-height read in the crate.
    /// The cost lands on `slab_diagonal`, and through it on the budget that
    /// tells a designer their map has outgrown DIST_PACK_RANGE. Hand-derived
    /// on the shipped 28 x 28 map: lift the level ROOT by 2.557 m and the
    /// slabs rise to y in [2.457, 5.657], a union still only 3.2 m tall and
    /// a diagonal still 39.727 — but a stamped [0, 3] stretches that union
    /// to 5.657 m and the diagonal to 40.00002, one hundredth of a
    /// millimetre past the range, raising a Severity::Error against a map
    /// that never moved.
    #[test]
    fn a_footprint_reads_each_walls_own_sweep_not_a_global_height() {
        // the sweep the level derives for a wall lifted with the gizmo
        let lifted =
            [
                crate::sight::Occluder::new(
                    Vector4::new(1.0, 2.0, 9.0, 2.0),
                    2.557,
                    2.557 + WALL_H,
                )
                .expect("finite"),
            ];
        let sweep = wall_sweep(&lifted);
        assert!((sweep.0 - 2.557).abs() < 1e-6, "lo {}", sweep.0);
        assert!((sweep.1 - 5.557).abs() < 1e-6, "hi {}", sweep.1);
        assert_eq!(wall_sweep(&[]), (0.0, 0.0), "no walls invent no storey");

        // and the consequence, end to end: a lifted level's diagonal must
        // not move, because nothing about the map got bigger
        let (floor, ceiling) = slab_boxes(28.0, 28.0);
        let lift = |b: Box3| Box3 {
            min: [b.min[0], b.min[1] + 2.557, b.min[2]],
            max: [b.max[0], b.max[1] + 2.557, b.max[2]],
        };
        let walls = border(0.6, 0.6, 27.4, 27.4);
        let diagonal = slab_diagonal(lift(floor), lift(ceiling), &walls, sweep);
        assert!(
            (diagonal - 39.727_068_857_392_44).abs() < 1e-6,
            "a lifted level measured {diagonal} against an unmoved 39.727"
        );
        assert!(
            pack_range_budget(diagonal, DIST_PACK_RANGE).is_none(),
            "a map that did not grow was told to shrink"
        );

        // the counter-example that makes this non-vacuous: the global
        // [0, WALL_H] the code used to stamp
        let stamped = slab_diagonal(lift(floor), lift(ceiling), &walls, (0.0, WALL_H));
        assert!(
            stamped > DIST_PACK_RANGE,
            "the old stamp no longer misfires, so this test proves nothing: {stamped}"
        );
    }

    /// A centerline is a QUAD, not an ordered pair, and BOTH ends of both
    /// axes have to be read. [`wall_segment`] happens to sweep its ends in
    /// ascending order today, which is exactly what makes the other half
    /// easy to lose: a walk that read only `x` and `y` would still measure
    /// a same-handed table correctly and would silently shrink any table
    /// whose quads arrived the other way round — the failure mode
    /// `sight::wall_rect` normalises against and the `wall_segment` doc
    /// warns about.
    ///
    /// So the same footprint is measured twice, once with every quad
    /// reversed. The stub first is not decoration: the walk seeds itself
    /// from the first quad, so an extreme that lives there would be found
    /// by the seeding whatever the loop dropped.
    #[test]
    fn wall_footprint_reads_both_ends_of_a_centerline_whichever_way_round_it_arrives() {
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
        let want = Box3 {
            min: [1.0, 0.0, 2.0],
            max: [9.0, WALL_H, 7.0],
        };
        assert_eq!(wall_footprint(&forward, (0.0, WALL_H)), Some(want));
        assert_eq!(wall_footprint(&flipped, (0.0, WALL_H)), Some(want));
    }

    /// The shipped 28 × 28 map: its slab pair alone spans
    /// sqrt(28² + 3.2² + 28²) = sqrt(1578.24) = 39.727 m — the 3.2 m height
    /// is WALL_H's 3.0 plus a SLAB_T of 0.1 past the floor's top and
    /// another past the ceiling's underside. The walls border at 0.6 and
    /// 27.4, fully inside the slab on every side, so unioning them in
    /// changes nothing. 0.27 m of headroom under the shaders' 40, and
    /// silence: the shipped scene must keep emitting zero WaveLevel
    /// messages.
    #[test]
    fn the_slab_diagonal_spans_the_shipped_maps_floor_and_ceiling() {
        let (floor, ceiling) = slab_boxes(28.0, 28.0);
        let walls = border(0.6, 0.6, 27.4, 27.4);
        let diagonal = slab_diagonal(floor, ceiling, &walls, (0.0, WALL_H));
        assert!(
            (diagonal - 39.727_068_857_392_44).abs() < 1e-9,
            "{diagonal}"
        );
        assert_eq!(pack_range_budget(diagonal, DIST_PACK_RANGE), None);
    }

    /// ISSUE #45's own reproduction, and the RED this fix exists to turn
    /// green: an 80 × 80 courtyard with one small 6 × 6 walled room. The
    /// OLD wall-centerline measure saw only that room's short walls and
    /// reported a tiny, harmless footprint — silence, while the slab
    /// underfoot, which is what every silhouette and every footstep
    /// actually draws against, spans sqrt(80² + 3.2² + 80²) =
    /// sqrt(12810.24) = 113.182 m, nearly three times the shaders' 40 m
    /// range. The room's walls sit fully inside the slab, so they change
    /// nothing; the slab alone was always the whole story, and the bug was
    /// never asking it.
    #[test]
    fn a_courtyard_with_one_small_room_still_saturates_the_pack_range() {
        let (floor, ceiling) = slab_boxes(80.0, 80.0);
        let walls = border(10.0, 10.0, 16.0, 16.0); // the one 6 × 6 room
        let diagonal = slab_diagonal(floor, ceiling, &walls, (0.0, WALL_H));
        assert!(
            (diagonal - 113.182_330_776_495_32).abs() < 1e-9,
            "{diagonal}"
        );
        assert_eq!(
            pack_range_budget(diagonal, DIST_PACK_RANGE).map(|b| b.severity),
            Some(Severity::Error)
        );
    }

    /// The short-circuit this fix removes: the OLD `map_diagonal` returned
    /// 0.0 for an empty wall table, so a bare courtyard with not one wall
    /// in it reported NOTHING — silence on a level whose floor alone was
    /// already far past shader range. The slab pair is never optional now,
    /// so a wall-less level still measures: sqrt(50² + 3.2² + 50²) =
    /// 70.783 m.
    #[test]
    fn a_wall_less_courtyard_still_measures_off_its_slabs() {
        let (floor, ceiling) = slab_boxes(50.0, 50.0);
        let diagonal = slab_diagonal(floor, ceiling, &[], (0.0, WALL_H));
        assert!(
            (diagonal - 70.783_048_818_202_23).abs() < 1e-9,
            "{diagonal}"
        );
        assert_eq!(
            pack_range_budget(diagonal, DIST_PACK_RANGE).map(|b| b.severity),
            Some(Severity::Error)
        );
    }

    /// Belt-and-braces: a wall is authored to stand on its level's slab,
    /// but nothing enforces it, and drawn geometry outside the slab is
    /// still geometry a sight line can reach. A 10 × 10 slab with one wall
    /// poking clean through both its x-edges (from x = −2 to x = 12) grows
    /// the union from a 10 m span to a 14 m one: sqrt(14² + 3.2² + 10²) =
    /// sqrt(306.24) = 17.500 m, wider than the slab alone would ever
    /// report.
    #[test]
    fn a_wall_reaching_past_the_slab_edge_still_widens_the_diagonal() {
        let (floor, ceiling) = slab_boxes(10.0, 10.0);
        let walls = [Vector4::new(-2.0, 5.0, 12.0, 5.0)];
        let diagonal = slab_diagonal(floor, ceiling, &walls, (0.0, WALL_H));
        assert!(
            (diagonal - 17.499_714_283_381_888).abs() < 1e-9,
            "{diagonal}"
        );
    }

    /// The shipped map's 39.73 m against the shaders' 40 m range: 0.27 m
    /// of headroom, and silence. The shipped scene emits zero WaveLevel
    /// messages today and must keep emitting zero.
    #[test]
    fn a_map_inside_the_packing_range_says_nothing() {
        assert_eq!(pack_range_budget(39.73, 40.0), None);
        assert_eq!(pack_range_budget(39.99, 40.0), None);
        assert_eq!(pack_range_budget(0.0, 40.0), None);
    }

    /// EQUALITY IS ALREADY TOO FAR, and this is what keeps the level's own
    /// report in step with `game/tests/shader_contract_test.gd`, which
    /// demands the range be strictly GREATER than the diagonal: at
    /// vd == range the packed value is already 1.0, the top of the band
    /// and the first value that cannot be told from anything beyond it.
    #[test]
    fn a_diagonal_that_exactly_reaches_the_range_is_already_reported() {
        assert_eq!(
            pack_range_budget(40.0, 40.0).map(|b| b.severity),
            Some(Severity::Error)
        );
    }

    /// Issue #45's own reproduction, and the message it must produce: the
    /// 80 × 80 courtyard's slab pair pushes the diagonal to 113.18 m.
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
        let budget = pack_range_budget(113.182_330_776_495_32, 40.0).expect("a report");
        assert_eq!(budget.severity, Severity::Error);
        assert_eq!(
            budget.text,
            "WaveLevel: the map's 113.18 m diagonal reaches the sight shaders' DIST_PACK_RANGE \
             of 40 m. Packed camera distance SATURATES there, it does not wrap: the data core \
             packs clamp(vd / DIST_PACK_RANGE, 0, 1) into B, so everything past 40 m reads a \
             flat 1.0 — its silhouette Laplacian is zero and it draws no outline at all, and the \
             hearing pass cuts player-sound rings against a world it believes is exactly 40 m \
             away. Shrink the map, or raise DIST_PACK_RANGE in \
             game/shaders/pulse_pool.gdshaderinc — a measured decision and not a free one: it \
             rescales every packed distance, and the outline thresholds in hearing_post are tuned \
             against this range."
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

    /// The extents every scene-signature test below folds against unless
    /// it is the one thing under test — a 20x20 level, same as the shipped
    /// map's own fixtures elsewhere in this file.
    const STILL_EXTENTS: [f32; 2] = [20.0, 20.0];

    /// One node standing still: identity transform, a 0.5 m cube skin. The
    /// scene-signature tests below all start from this and change exactly
    /// one thing at a time.
    fn still_node(path: &str) -> SignatureNode {
        SignatureNode {
            path: path.to_string(),
            instance_identity: 7,
            transform: [
                1.0, 0.0, 0.0, // basis X
                0.0, 1.0, 0.0, // basis Y
                0.0, 0.0, 1.0, // basis Z
                0.0, 0.0, 0.0, // origin
            ],
            aabb: Some([0.0, 0.0, 0.0, 0.5, 0.5, 0.5]),
        }
    }

    /// The whole point of a condition-watch: an unchanged scene folds to
    /// the SAME number twice, so the level's per-frame poll can compare
    /// against last frame's answer at all.
    #[test]
    fn the_same_scene_folds_to_the_same_signature_twice() {
        let scene = vec![still_node("Walls/North"), still_node("Rooms/Crate")];
        assert_eq!(
            scene_signature(STILL_EXTENTS, &scene),
            scene_signature(STILL_EXTENTS, &scene.clone())
        );
    }

    /// Rebuilding a WaveRun replaces its ownerless RunSeg wall with a new
    /// Godot object at the same path, pose and AABB. Identity is the only
    /// condition that changes, and therefore the only signal that can make
    /// the editor level repaint the fresh mesh and replace its freed handle.
    #[test]
    fn replacing_a_node_generation_alone_moves_the_signature() {
        let original = still_node("Doorway/RunSeg1");
        let mut replacement = original.clone();
        replacement.instance_identity = 8;
        assert_ne!(
            scene_signature(STILL_EXTENTS, &[original]),
            scene_signature(STILL_EXTENTS, &[replacement])
        );
    }

    /// A designer nudging a node one float's smallest step still moves the
    /// signature — the fold has to be sensitive to the whole bit pattern,
    /// not just to a value a human would call "different".
    #[test]
    fn one_bit_of_transform_change_moves_the_signature() {
        let mut nudged = still_node("Rooms/Crate");
        nudged.transform[9] = f32::from_bits(nudged.transform[9].to_bits() ^ 1);
        assert_ne!(
            scene_signature(STILL_EXTENTS, &[still_node("Rooms/Crate")]),
            scene_signature(STILL_EXTENTS, &[nudged])
        );
    }

    /// The fold reads the census in SCENE order, and order is part of the
    /// condition: two nodes folded in one order and the same two folded in
    /// the other must not collide, or a designer dragging a row in the
    /// Scene dock — which reorders the walk without moving anything —
    /// would silently escape the watch on one specific reorder.
    #[test]
    fn swapping_two_nodes_moves_the_signature() {
        let a = still_node("Walls/North");
        let b = still_node("Rooms/Crate");
        assert_ne!(
            scene_signature(STILL_EXTENTS, &[a.clone(), b.clone()]),
            scene_signature(STILL_EXTENTS, &[b, a])
        );
    }

    /// THE mutation this test exists to catch: a solid's AABB changing —
    /// the `radius`/`size` knob a designer drags in the Inspector — with
    /// its path and its transform both held fixed. Nothing but the AABB
    /// fold in `scene_signature` can possibly notice this; drop that fold
    /// and this is the one test in the file that goes green for the wrong
    /// reason and then quietly stops meaning anything.
    #[test]
    fn a_solids_aabb_change_alone_moves_the_signature_a_transform_did_not() {
        let mut resized = still_node("Rooms/Crate");
        resized.aabb = Some([0.0, 0.0, 0.0, 0.8, 0.5, 0.5]);
        assert_ne!(
            scene_signature(STILL_EXTENTS, &[still_node("Rooms/Crate")]),
            scene_signature(STILL_EXTENTS, &[resized])
        );
    }

    /// A wall's skin never disappears once built, but the fold must still
    /// tell a node WITH a shape apart from an otherwise-identical node
    /// with none — the boundary byte earns its keep here.
    #[test]
    fn losing_the_aabb_entirely_moves_the_signature() {
        let mut bare = still_node("Rooms/Crate");
        bare.aabb = None;
        assert_ne!(
            scene_signature(STILL_EXTENTS, &[still_node("Rooms/Crate")]),
            scene_signature(STILL_EXTENTS, &[bare])
        );
    }

    /// THE mutation this test exists to catch, this time: the level's
    /// `extents` knob changing with the censused scene held perfectly
    /// still. This is the designer-facing gap the fold used to have —
    /// `derive` genuinely reads `extents` (through the floor slab's own
    /// world box, in `report_placement` and the per-face paint pass), so a
    /// resize that is not in the fold is a resize the condition-watch cannot see.
    /// Drop the extents fold and this is the test that goes quiet.
    #[test]
    fn a_different_extents_alone_moves_the_signature() {
        let scene = [still_node("Walls/North"), still_node("Rooms/Crate")];
        assert_ne!(
            scene_signature(STILL_EXTENTS, &scene),
            scene_signature([2.0, 20.0], &scene)
        );
    }

    /// THE BREAK: the admission rule drifting until it swallows the whole
    /// prop census (blowing the wall table and the shader's hottest loop)
    /// or refuses the pillars it exists to admit.
    ///
    /// The numbers are the shipped level's own, measured off
    /// `game/scenes/level_01.tscn` and written down here so the rule is
    /// pinned against real content and not against itself: seven pillars
    /// span [0.00, 3.00] and are 0.44–0.50 m across; seven standpipes span
    /// [0.00, 2.90] and are 0.14–0.20 m; sixty-two boxes and wedges reach
    /// at most 2.00 m.
    #[test]
    fn only_the_solids_that_really_stand_in_the_way_occlude() {
        // pillars: floor to ceiling, thicker than a wall — admitted
        assert!(spans_the_corridor(0.0, 3.0, 0.44));
        assert!(spans_the_corridor(0.0, 3.0, 0.50));

        // standpipes: full height, but far thinner than a wall. A rect
        // table would give them metre-wide square shadows.
        assert!(!spans_the_corridor(0.0, 2.9, 0.14));
        assert!(!spans_the_corridor(0.0, 2.9, 0.20));
        // even if one DID reach the ceiling, the thickness still refuses it
        assert!(!spans_the_corridor(0.0, 3.0, 0.20));

        // boxes and wedges: sound goes over them
        assert!(!spans_the_corridor(0.0, 2.00, 1.4));
        assert!(!spans_the_corridor(0.0, 0.70, 1.4));

        // a shelf hung clear of the floor is not structure either
        assert!(!spans_the_corridor(0.9, 3.0, 0.6));
    }

    /// THE BREAK: SPAN_EPS quietly widening into a tolerance that admits
    /// the pipes, which sit exactly 0.10 m under the ceiling — twice the
    /// slack the rule allows, and the margin the whole separation rests on.
    #[test]
    fn the_span_slack_stays_narrower_than_the_gap_the_level_authored() {
        let pipe_top = 2.90;
        let pillar_top = 3.00;
        assert!(!spans_the_corridor(0.0, pipe_top, 0.5), "the pipes got in");
        assert!(spans_the_corridor(0.0, pillar_top, 0.5));
        // the authored gap is 0.10; the rule may not spend all of it
        assert!(
            SPAN_EPS < pillar_top - pipe_top,
            "SPAN_EPS {SPAN_EPS} has grown to reach the pipes"
        );
    }

    /// THE BREAK: a malformed or degenerate solid producing a NaN-cornered
    /// rect that the sight tests then walk, where every comparison answers
    /// false and the wall silently stops occluding.
    #[test]
    fn a_solid_that_cannot_be_measured_is_refused_rather_than_admitted() {
        assert!(!spans_the_corridor(f64::NAN, 3.0, 0.5));
        assert!(!spans_the_corridor(0.0, f64::NAN, 0.5));
        assert!(!spans_the_corridor(0.0, 3.0, f64::NAN));
        assert!(!spans_the_corridor(f64::NEG_INFINITY, f64::INFINITY, 0.5));
        // a degenerate zero-extent solid occupies no space and blocks nothing
        assert!(!spans_the_corridor(0.0, 3.0, 0.0));
    }
}
