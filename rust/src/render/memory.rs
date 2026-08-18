//! What the hero still knows about a room they are no longer sounding.
//!
//! # The world neither goes dark nor stays lit
//!
//! Two shipped games took the two pure answers and both landed in the low
//! sixties for it. *Perception* let the reveal linger and reviewers found
//! that "the easiest thing to do is just hammer the button and reveal
//! everything as you wander around" — you quickly forget you are blind.
//! *Blind* darkened the world within seconds and reviewers were "tapping the
//! cane like a madman just to keep a puzzle in sight", one of them reporting
//! nausea from the flash-and-black cycle. *Scanner Sombre* accumulated a
//! permanent point cloud and "turning back to look the way you came lets you
//! see through floors, walls, rocks".
//!
//! The answer is neither, because the question is wrong. A room does not
//! stay lit or go dark — it **loses resolution**. What survives is where the
//! walls were. What does not survive is anything that would let you name a
//! thing, or trust that it is still where you left it.
//!
//! Unseeing can afford the persistence those games could not, for one
//! structural reason: `sight` already answers occlusion analytically, so a
//! remembered surface cannot be seen through a wall. That is the exact point
//! at which both precedents failed.
//!
//! # Why a coarse grid and not a per-surface store
//!
//! Because you remember the shelf, not a patch of the shelf. A per-surface
//! store would also collide head-on with the superface merge law: splitting
//! a coplanar face so its halves could hold different memory would either
//! force differing labels — a seam mid-wall, precisely what
//! [`super::superface`] exists to prevent — or could not vary within one
//! mesh at all.
//!
//! [`CELL`] is chosen coarser than the largest non-pillar solid in the
//! shipped level, so the grid is *structurally incapable* of carrying object
//! detail even if a later maintainer wants it to.
//!
//! # What memory may never do
//!
//! - **It may never draw a crease.** [`CEIL`] is half the detail knee's
//!   floor, so the crease term's own gate can never open on a remembered
//!   pixel. Memory cannot name a thing; it can only say something was
//!   there. This is enforced by construction, not by tuning — see
//!   `memory_can_never_name_a_thing`.
//! - **It may never record anything that moves.** A silent cat is invisible
//!   however recently the room around it was swept. That asymmetry is the
//!   whole value of the mechanic: the map can be trusted about walls and
//!   never about anything alive. It costs no per-mover state — it falls out
//!   of the world-static partition the label allocator already draws.
//! - **It may never relitigate the wall law.** A cell is stamped only where
//!   the wave that swept it actually reached, so remembering cannot see
//!   through a wall the sweep could not pass.

use super::detail::DetailKnee;

/// Cells per side of the floor-plan grid.
///
/// 14 x 14 at [`CELL`] covers 28 m, the shipped map's own diagonal budget
/// (`level_plan::DIST_PACK_RANGE` is 40 m and the map measures under 28 m
/// across). 196 cells, one byte each on the GPU.
pub const GRID: usize = 14;

/// Metres per cell.
///
/// Coarser than the largest non-pillar solid in the shipped level (1.4 m) on
/// purpose: a grid that could resolve a crate would be a grid someone would
/// eventually ask to draw one.
pub const CELL: f64 = 2.0;

/// Seconds for a remembered cell to fade to half strength.
///
/// **NOT physics, and not claimed as any.** Echoic memory is 2–4 seconds —
/// that is the RING, and the shipped fade tails of 6.0/3.5/2.5/2.0 s already
/// sit close to it. Survey-level spatial memory in blind travellers runs to
/// minutes and hours. Anything between the two is craft, and this is craft:
/// chosen finite so that the twentieth minute of a session cannot feel like
/// the first.
pub const HALF_LIFE: f64 = 45.0;

/// Age at which a cell is forgotten outright.
///
/// The exponential never reaches zero, so without an end a room felt once
/// would glow faintly forever and the black the whole aesthetic rests on
/// would silt up. Two half-lives puts the value at a quarter of [`CEIL`]
/// before it is cut, which is already under the film grain.
pub const TAIL: f64 = 2.0 * HALF_LIFE;

/// The brightest a purely remembered surface may ever be drawn.
///
/// Derived, not chosen: half the floor of the detail knee. That single fact
/// is what makes "memory can never name a thing" a theorem rather than a
/// hope — the crease term is multiplied by a smoothstep that does not begin
/// to open until [`DetailKnee::lo`], and memory is pinned strictly below it.
#[must_use]
pub fn ceil() -> f64 {
    DetailKnee::shipped().lo() * 0.5
}

/// A floor plan the hero has walked, as last-swept times per cell.
///
/// Owned by the level, constructed with its origin, reset explicitly. No
/// statics, no interior mutability, no ambient lifetime — the whole state is
/// `GRID * GRID` floats and the corner they are measured from.
#[derive(Clone, Debug, PartialEq)]
pub struct Memory {
    /// When each cell was last swept, or `None` for never felt.
    stamped: Vec<Option<f64>>,
    origin_x: f64,
    origin_z: f64,
}

impl Memory {
    /// An unfelt floor plan whose cell `(0, 0)` starts at `(origin_x,
    /// origin_z)` and which therefore covers `GRID * CELL` metres each way.
    ///
    /// A non-finite origin is replaced by zero rather than refused: a level
    /// with a broken transform should forget everything, not fail to build.
    #[must_use]
    pub fn new(origin_x: f64, origin_z: f64) -> Self {
        Self {
            stamped: vec![None; GRID * GRID],
            origin_x: if origin_x.is_finite() { origin_x } else { 0.0 },
            origin_z: if origin_z.is_finite() { origin_z } else { 0.0 },
        }
    }

    /// Forget everything. Explicit rather than implicit, because a level
    /// reload that kept the previous level's floor plan would draw a map of
    /// a room that is not there.
    pub fn reset(&mut self) {
        self.stamped.iter_mut().for_each(|cell| *cell = None);
    }

    /// The cell a world XZ position falls in, or `None` if it is off the
    /// plan or not a finite place.
    #[must_use]
    pub fn cell_of(&self, x: f64, z: f64) -> Option<usize> {
        if !x.is_finite() || !z.is_finite() {
            return None;
        }
        let col = ((x - self.origin_x) / CELL).floor();
        let row = ((z - self.origin_z) / CELL).floor();
        let limit = GRID as f64;
        if col < 0.0 || row < 0.0 || col >= limit || row >= limit {
            return None;
        }
        // Both are in [0, GRID) and finite, so the casts are exact.
        Some(row as usize * GRID + col as usize)
    }

    /// Record that a wave reached this place at `now`.
    ///
    /// Off-plan positions and non-finite times are dropped rather than
    /// clamped: a stamp is a claim about somewhere, and clamping one into
    /// the nearest cell would remember a room the hero never entered.
    pub fn stamp(&mut self, x: f64, z: f64, now: f64) {
        if !now.is_finite() {
            return;
        }
        if let Some(cell) = self.cell_of(x, z) {
            self.stamped[cell] = Some(now);
        }
    }

    /// How brightly this place is still remembered, in `[0, ceil()]`.
    ///
    /// Total over every input: never felt, felt in the future (a clock that
    /// went backwards), non-finite, or off the plan all answer `0.0` rather
    /// than an exponential of a negative age — which would return a value
    /// ABOVE the ceiling and light the room.
    #[must_use]
    pub fn trace_at(&self, x: f64, z: f64, now: f64) -> f64 {
        if !now.is_finite() {
            return 0.0;
        }
        let Some(cell) = self.cell_of(x, z) else {
            return 0.0;
        };
        let Some(stamped) = self.stamped[cell] else {
            return 0.0;
        };
        let age = now - stamped;
        if !age.is_finite() || !(0.0..TAIL).contains(&age) {
            return 0.0;
        }
        (ceil() * (-age / HALF_LIFE).exp2()).clamp(0.0, ceil())
    }

    /// Every cell's current strength, row-major, for the GPU upload and for
    /// the observer.
    #[must_use]
    pub fn field(&self, now: f64) -> Vec<f64> {
        (0..GRID * GRID)
            .map(|cell| {
                let x = self.origin_x + (cell % GRID) as f64 * CELL;
                let z = self.origin_z + (cell / GRID) as f64 * CELL;
                self.trace_at(x, z, now)
            })
            .collect()
    }

    /// How many cells are currently remembered at all — the observable that
    /// makes a faint persistent wireframe distinguishable from a stuck
    /// framebuffer, which this repository has shipped before.
    #[must_use]
    pub fn felt_cells(&self, now: f64) -> usize {
        self.field(now).iter().filter(|v| **v > 0.0).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE BREAK: memory growing bright enough to open the crease gate, so
    /// that a room you merely remember starts naming the things in it — the
    /// fan's blades legible from a floor plan.
    ///
    /// This is the theorem the ceiling exists for, and it is stated against
    /// the knee rather than against a literal so that moving either one
    /// without the other fails here.
    #[test]
    fn memory_can_never_name_a_thing() {
        let knee = DetailKnee::shipped();
        assert!(
            ceil() < knee.lo(),
            "memory ceils at {} against a detail knee opening at {} — at or \
             above that floor a remembered pixel draws creases",
            ceil(),
            knee.lo()
        );
        // and the strongest a cell can ever be is the ceiling itself
        let mut m = Memory::new(0.0, 0.0);
        m.stamp(1.0, 1.0, 100.0);
        assert!((m.trace_at(1.0, 1.0, 100.0) - ceil()).abs() < 1e-12);
        for age in [0.0, 1.0, 10.0, 44.9, 45.0, 89.9] {
            assert!(m.trace_at(1.0, 1.0, 100.0 + age) <= ceil());
        }
    }

    /// THE BREAK: the decay ceasing to decay, or never ending — a room felt
    /// once glowing faintly forever, silting up the black the whole
    /// aesthetic rests on.
    ///
    /// Hand-derived: a half-life of 45 s means exactly half strength at 45 s
    /// and a quarter at 90 s, and TAIL cuts it at 90 s so it lands on zero
    /// rather than approaching it.
    #[test]
    fn a_room_is_forgotten_rather_than_faded_forever() {
        let mut m = Memory::new(0.0, 0.0);
        m.stamp(3.0, 3.0, 0.0);
        assert!((m.trace_at(3.0, 3.0, 0.0) - ceil()).abs() < 1e-12);
        assert!((m.trace_at(3.0, 3.0, 45.0) - ceil() * 0.5).abs() < 1e-12);
        // strictly monotone on the way down
        let mut last = f64::INFINITY;
        for t in 0..90 {
            let v = m.trace_at(3.0, 3.0, f64::from(t));
            assert!(v <= last, "trace rose at t={t}: {v} after {last}");
            last = v;
        }
        // and gone, exactly, at the tail
        assert_eq!(m.trace_at(3.0, 3.0, 90.0), 0.0);
        assert_eq!(m.trace_at(3.0, 3.0, 1000.0), 0.0);
    }

    /// THE BREAK: the grid resolving fine enough to carry an object, after
    /// which someone will ask it to draw one — and a floor plan that can
    /// draw a crate is a floor plan that has stopped being a floor plan.
    ///
    /// Hand-derived: the largest non-pillar solid in the shipped level is
    /// 1.4 m across, and CELL must be strictly coarser.
    #[test]
    fn one_cell_is_coarser_than_anything_it_could_describe() {
        // the largest non-pillar solid in the shipped level, measured off
        // game/scenes/level_01.tscn
        let widest_prop = 1.4_f64;
        assert!(
            CELL > widest_prop,
            "CELL {CELL} can resolve a {widest_prop} m prop"
        );
        let m = Memory::new(0.0, 0.0);
        // two points a metre apart inside one cell are the same memory
        assert_eq!(m.cell_of(0.2, 0.2), m.cell_of(1.2, 1.2));
        // and the next cell over is a different one
        assert_ne!(m.cell_of(1.2, 1.2), m.cell_of(2.2, 1.2));
    }

    /// THE BREAK: an off-plan or malformed position being clamped into the
    /// nearest cell, which remembers a room the hero was never in — or a
    /// clock that steps backwards producing an exponential of a negative
    /// age, which is a value ABOVE the ceiling that lights the level.
    #[test]
    fn a_place_that_is_not_on_the_plan_is_forgotten_not_clamped() {
        let mut m = Memory::new(0.0, 0.0);
        assert_eq!(m.cell_of(-0.1, 1.0), None);
        assert_eq!(m.cell_of(1.0, -0.1), None);
        assert_eq!(m.cell_of(GRID as f64 * CELL, 1.0), None);
        assert_eq!(m.cell_of(f64::NAN, 1.0), None);
        assert_eq!(m.cell_of(1.0, f64::INFINITY), None);

        m.stamp(-5.0, -5.0, 10.0);
        m.stamp(1.0, 1.0, f64::NAN);
        assert_eq!(m.felt_cells(10.0), 0, "a dropped stamp still landed");

        m.stamp(1.0, 1.0, 100.0);
        // the clock went backwards: an age of -50 must not exceed the ceiling
        assert_eq!(m.trace_at(1.0, 1.0, 50.0), 0.0);
        assert_eq!(m.trace_at(1.0, 1.0, f64::NAN), 0.0);
        assert_eq!(m.trace_at(f64::NAN, 1.0, 100.0), 0.0);
    }

    /// THE BREAK: a level reload keeping the previous level's floor plan, so
    /// the hero remembers a map of a room that is not there.
    #[test]
    fn a_reset_plan_remembers_nothing() {
        let mut m = Memory::new(0.0, 0.0);
        m.stamp(1.0, 1.0, 0.0);
        m.stamp(5.0, 5.0, 0.0);
        assert_eq!(m.felt_cells(0.0), 2);
        m.reset();
        assert_eq!(m.felt_cells(0.0), 0);
        assert_eq!(m, Memory::new(0.0, 0.0));
    }

    /// THE BREAK: the grid's origin being ignored, so a level built away
    /// from the world origin remembers the wrong squares entirely.
    #[test]
    fn the_plan_is_measured_from_its_own_corner() {
        let mut m = Memory::new(-10.0, -10.0);
        assert_eq!(m.cell_of(0.0, 0.0), Some(5 * GRID + 5));
        assert_eq!(m.cell_of(-10.0, -10.0), Some(0));
        assert_eq!(m.cell_of(-11.0, 0.0), None);
        m.stamp(0.0, 0.0, 0.0);
        assert!(m.trace_at(0.5, 0.5, 0.0) > 0.0);
        assert_eq!(m.trace_at(-5.0, -5.0, 0.0), 0.0);
    }
}
