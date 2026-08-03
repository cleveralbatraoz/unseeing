//! The cat's walk — a four-beat lateral-sequence gait with truly planted
//! paws. Real cats step hind-then-fore down each side (LH, LF, RH, RF),
//! keep two or three paws grounded at a walk, and place each paw where it
//! will stay: a planted paw NEVER slides. The module owns that law as
//! state — per-leg planted world positions, swing arcs re-aimed every
//! frame (so turning mid-swing lands the paw where the body is actually
//! going), and touchdown contact events the engine layer turns into paw
//! waves.
//!
//! The acoustic voice lives here too, like the fan's voice lives in
//! [`crate::fan_wave`]: paw-step waves are pulse kind 2 (footstep — the
//! least precious slot class), small and soft, and only the FORE paws
//! sound. A walking cat direct-registers — each hind paw lands in the
//! ipsilateral fore paw's print — so the fore touch announces a fresh
//! spot and the hind lands on ground already pressed, silently. Half the
//! sounds, and the halving is the realism.
//!
//! Everything is f64 until a Vector3 lane narrows it, per the crate's
//! precision law; phase advance is dt-driven and clamped, so a stalled
//! clock buys a step, never a burst — the [`crate::fan_wave`] doctrine.

use godot::builtin::Vector3;

/// Number of legs; index order is LF, RF, LH, RH.
pub const LEGS: usize = 4;

/// Meters one full stride cycle carries the body at a walk.
pub const STRIDE_LEN: f64 = 0.30;

/// Fraction of the cycle each paw spends grounded (stance). 0.70 keeps
/// two paws down always and three most of the time — the stately walk.
pub const DUTY: f64 = 0.70;

/// The gait's design envelope: the fastest sustained walk the paw-wave
/// budget below is pinned against. The brain's wander speed must stay
/// under this.
pub const TOP_SPEED: f64 = 0.65;

/// Paw arc apex in meters — a cat lifts its paws barely off the ground.
pub const SWING_LIFT: f64 = 0.035;

/// Ride height of the chest/hip line above the floor, meters.
pub const BODY_H: f64 = 0.20;

/// Walk bob amplitude, meters — two gentle rises per stride.
pub const BOB_AMP: f64 = 0.006;

/// Weight-shift roll amplitude, radians — one sway per stride.
pub const ROLL_AMP: f64 = 0.025;

/// Paw wave reach in meters — a whisper next to the hero's 1.6 m steps.
pub const PAW_RANGE: f64 = 0.8;

/// Paw wavefront speed, m/s — same air as the hero's footsteps.
pub const PAW_SPEED: f64 = 4.0;

/// Paw wave loudness — soft pads, not shoes.
pub const PAW_GAIN: f64 = 0.5;

/// Planar speed below which the cat counts as standing.
pub const MOVE_EPS: f64 = 0.05;

/// Stride phase at which each leg touches down — the lateral sequence:
/// LH at 0, LF a quarter later, RH at half, RF at three quarters.
/// Indexed LF, RF, LH, RH.
const OFFSET: [f64; LEGS] = [0.25, 0.75, 0.0, 0.5];

/// Which side each leg hangs on: -1 left, +1 right. Indexed LF, RF, LH, RH.
const SIDE: [f64; LEGS] = [-1.0, 1.0, -1.0, 1.0];

/// Shoulder (fore) / hip (hind) anchor, meters ahead of the body center.
const FORE_AFT: [f64; LEGS] = [0.145, 0.145, -0.145, -0.145];

/// Anchor's lateral distance from the centerline — a narrow track: the
/// silhouette walks a tightrope, the way a cat's prints line up.
const LATERAL: [f64; LEGS] = [0.048, 0.048, 0.055, 0.055];

/// How far ahead of its anchor a paw lands: half the stance sweep, so the
/// plant spends its grounded life passing from ahead of the anchor to
/// behind it while the body walks over it.
const AHEAD: f64 = STRIDE_LEN * DUTY * 0.5;

/// The walk-amplitude ease rate, 1/s — the viewmodel's own constant.
const AMP_RATE: f64 = 6.0;

/// The largest stride-phase step one tick may take: a stalled or jumped
/// clock advances the cycle a quarter at most, so no leg ever skips a
/// whole swing unseen and no tick fires more than one contact per leg.
const MAX_PHASE_STEP: f64 = 0.25;

/// Only the fore paws sound: the walking cat direct-registers, each hind
/// paw landing in the fore print already pressed — announced once, then
/// silent.
#[must_use]
pub fn paw_sounds(leg: usize) -> bool {
    leg < 2
}

/// One paw touching down: which leg, and the exact floor point (y = 0)
/// the engine layer births a paw wave at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Leg index (LF, RF, LH, RH).
    pub leg: usize,
    /// The touchdown point on the floor.
    pub at: Vector3,
}

/// One tick's gait outputs — everything the body model needs to pose the
/// cat, plus the touchdown events for the paw waves.
#[derive(Debug, Clone, PartialEq)]
pub struct GaitFrame {
    /// Paw world positions, LF RF LH RH; y = 0 exactly while planted.
    pub paws: [Vector3; LEGS],
    /// The paws that touched down THIS tick.
    pub contacts: Vec<Contact>,
    /// Stride phase in [0, 1) — tail sway and bob ride it.
    pub phase: f64,
    /// Walk amplitude easing between standing (0) and walking (1).
    pub amp: f64,
    /// Body lift above [`BODY_H`] this frame (the walk bob).
    pub bob: f64,
    /// Weight-shift roll, radians, +right.
    pub roll: f64,
    /// Whether the walk gate was open this tick.
    pub moving: bool,
}

/// The gait state machine: stride phase, walk amplitude, and the four
/// planted paws. One instance per cat, advanced every physics tick with
/// the body's ACTUAL planar motion — a wall that stops the body stops
/// the stepping too, honestly.
#[derive(Debug, Clone, PartialEq)]
pub struct CatGait {
    phase: f64,
    amp: f64,
    planted: [Vector3; LEGS],
    /// Where each swing is currently aimed; the touchdown point when the
    /// leg comes back to stance.
    aim: [Vector3; LEGS],
    in_swing: [bool; LEGS],
}

impl CatGait {
    /// A fresh gait standing at `pos` facing `yaw`: every paw planted at
    /// its neutral anchor, phase zeroed, standing still.
    #[must_use]
    pub fn new(pos: Vector3, yaw: f64) -> Self {
        let mut planted = [Vector3::ZERO; LEGS];
        for (leg, spot) in planted.iter_mut().enumerate() {
            *spot = anchor(pos, yaw, leg);
        }
        Self {
            phase: 0.0,
            amp: 0.0,
            planted,
            aim: planted,
            in_swing: [false; LEGS],
        }
    }

    /// Advance one tick. `pos` is the body center on the floor (y is
    /// ignored), `yaw` the heading, `speed` the ACTUAL planar speed the
    /// body achieved — feed the measured displacement, not the wish, so
    /// blocked bodies stop stepping.
    pub fn advance(&mut self, dt: f64, pos: Vector3, yaw: f64, speed: f64) -> GaitFrame {
        let moving = speed > MOVE_EPS;
        self.amp += ((if moving { 1.0 } else { 0.0 }) - self.amp) * (dt * AMP_RATE).min(1.0);
        let mut contacts = Vec::new();
        if moving {
            self.phase = (self.phase + (dt * speed / STRIDE_LEN).min(MAX_PHASE_STEP)).fract();
        } else {
            self.settle(&mut contacts);
        }
        let mut paws = [Vector3::ZERO; LEGS];
        for (leg, paw) in paws.iter_mut().enumerate() {
            *paw = if moving {
                self.step_leg(leg, pos, yaw, &mut contacts)
            } else {
                self.planted[leg]
            };
        }
        GaitFrame {
            paws,
            contacts,
            phase: self.phase,
            amp: self.amp,
            bob: BOB_AMP * (self.phase * std::f64::consts::TAU * 2.0).sin() * self.amp,
            roll: ROLL_AMP * (self.phase * std::f64::consts::TAU).sin() * self.amp,
            moving,
        }
    }

    /// One walking leg: stance holds the plant; swing re-aims every frame
    /// at where the anchor is going and arcs there; the swing-to-stance
    /// edge is the touchdown.
    fn step_leg(
        &mut self,
        leg: usize,
        pos: Vector3,
        yaw: f64,
        contacts: &mut Vec<Contact>,
    ) -> Vector3 {
        let lp = (self.phase - OFFSET[leg]).rem_euclid(1.0);
        if lp < DUTY {
            if self.in_swing[leg] {
                self.in_swing[leg] = false;
                self.planted[leg] = self.aim[leg];
                contacts.push(Contact {
                    leg,
                    at: self.planted[leg],
                });
            }
            return self.planted[leg];
        }
        if !self.in_swing[leg] {
            self.in_swing[leg] = true;
        }
        // Re-aim every swing frame: by touchdown the body will have walked
        // (1 - lp) of a stride, so the anchor's future is one term of
        // STRIDE_LEN — the speed cancels out of its own prediction.
        let fw = forward(yaw);
        self.aim[leg] = anchor(pos, yaw, leg) + fw * ((AHEAD + (1.0 - lp) * STRIDE_LEN) as f32);
        let sw = (lp - DUTY) / (1.0 - DUTY);
        let eased = sw * sw * (3.0 - 2.0 * sw);
        let mut paw = self.planted[leg].lerp(self.aim[leg], eased as f32);
        paw.y = (SWING_LIFT * self.amp * (sw * std::f64::consts::PI).sin()) as f32;
        paw
    }

    /// Standing still: any paw caught mid-swing touches down right where
    /// it hangs — the settle step a halting cat actually takes.
    fn settle(&mut self, contacts: &mut Vec<Contact>) {
        for leg in 0..LEGS {
            if !self.in_swing[leg] {
                continue;
            }
            self.in_swing[leg] = false;
            let spot = Vector3::new(self.aim[leg].x, 0.0, self.aim[leg].z);
            self.planted[leg] = spot;
            contacts.push(Contact { leg, at: spot });
        }
    }
}

/// The heading's forward vector — Godot yaw convention: yaw 0 faces -Z.
fn forward(yaw: f64) -> Vector3 {
    Vector3::new((-yaw.sin()) as f32, 0.0, (-yaw.cos()) as f32)
}

/// The heading's right vector.
fn rightward(yaw: f64) -> Vector3 {
    Vector3::new(yaw.cos() as f32, 0.0, (-yaw.sin()) as f32)
}

/// A leg's anchor: its shoulder or hip projected onto the floor for the
/// body standing at `pos` facing `yaw`.
fn anchor(pos: Vector3, yaw: f64, leg: usize) -> Vector3 {
    let spot = Vector3::new(pos.x, 0.0, pos.z)
        + forward(yaw) * (FORE_AFT[leg] as f32)
        + rightward(yaw) * ((LATERAL[leg] * SIDE[leg]) as f32);
    Vector3::new(spot.x, 0.0, spot.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pulse_pool::PulsePool;

    const DT: f64 = 1.0 / 60.0;

    /// Walk a straight line at `speed` for `ticks`, collecting frames.
    fn walk_line(speed: f64, ticks: usize) -> Vec<GaitFrame> {
        let mut gait = CatGait::new(Vector3::ZERO, 0.0);
        let mut pos = Vector3::ZERO;
        let mut frames = Vec::new();
        for _ in 0..ticks {
            pos += forward(0.0) * ((speed * DT) as f32);
            frames.push(gait.advance(DT, pos, 0.0, speed));
        }
        frames
    }

    /// A planted paw NEVER slides: during stance its world position is
    /// bit-for-bit identical from frame to frame.
    #[test]
    fn planted_paws_never_slide() {
        let frames = walk_line(0.6, 600);
        for pair in frames.windows(2) {
            for leg in 0..LEGS {
                let (a, b) = (pair[0].paws[leg], pair[1].paws[leg]);
                // grounded both frames and no touchdown between them:
                // the same plant, exactly
                if a.y == 0.0 && b.y == 0.0 && !pair[1].contacts.iter().any(|c| c.leg == leg) {
                    assert_eq!(a, b);
                }
            }
        }
    }

    /// The four beats land in lateral-sequence order — LH, LF, RH, RF —
    /// over and over, the walking-cat signature.
    #[test]
    fn footfalls_keep_lateral_sequence_order() {
        let frames = walk_line(0.6, 1800);
        let order: Vec<usize> = frames
            .iter()
            .flat_map(|f| f.contacts.iter().map(|c| c.leg))
            .collect();
        assert!(order.len() >= 8);
        // the lateral sequence, as leg indices (LF 0, RF 1, LH 2, RH 3):
        // ... LH(2), LF(0), RH(3), RF(1) ...
        const SEQ: [usize; LEGS] = [2, 0, 3, 1];
        let start = SEQ.iter().position(|&s| s == order[0]).unwrap();
        for (i, &leg) in order.iter().enumerate() {
            assert_eq!(leg, SEQ[(start + i) % LEGS]);
        }
    }

    /// Each leg touches down once per stride cycle — no double-taps, no
    /// skipped beats — and every touchdown lies exactly on the floor.
    #[test]
    fn one_contact_per_leg_per_cycle() {
        let speed = 0.6;
        let cycles = 8.0;
        let ticks = (cycles * STRIDE_LEN / speed / DT).round() as usize;
        let frames = walk_line(speed, ticks);
        for leg in 0..LEGS {
            let count = frames
                .iter()
                .flat_map(|f| &f.contacts)
                .filter(|c| c.leg == leg)
                .count() as f64;
            assert!((count - cycles).abs() <= 1.0, "leg {leg}: {count} contacts");
        }
        for c in frames.iter().flat_map(|f| &f.contacts) {
            assert_eq!(c.at.y, 0.0);
        }
    }

    /// Paws stay on or above the floor, and the swing arc never exceeds
    /// its tiny lift — a cat barely clears the ground.
    #[test]
    fn paws_stay_low_and_never_dig() {
        for frame in walk_line(0.6, 1200) {
            for paw in frame.paws {
                assert!(paw.y >= 0.0);
                assert!(f64::from(paw.y) <= SWING_LIFT + 1e-9);
            }
        }
    }

    /// The stately walk: two paws grounded at every instant, three for
    /// most of them.
    #[test]
    fn walk_keeps_two_or_three_paws_down() {
        let frames = walk_line(0.6, 1200);
        let settled = &frames[240..]; // past the walk-amp ramp
        let mut three_or_more = 0;
        for frame in settled {
            let down = frame.paws.iter().filter(|p| p.y == 0.0).count();
            assert!(down >= 2, "only {down} paws down");
            if down >= 3 {
                three_or_more += 1;
            }
        }
        assert!(f64::from(three_or_more) / settled.len() as f64 > 0.6);
    }

    /// A standing cat is a silent statue: no contacts, no paw motion.
    #[test]
    fn standing_cat_is_still_and_silent() {
        let mut gait = CatGait::new(Vector3::new(3.0, 0.0, 2.0), 1.1);
        let first = gait.advance(DT, Vector3::new(3.0, 0.0, 2.0), 1.1, 0.0);
        assert!(first.contacts.is_empty());
        for _ in 0..120 {
            let frame = gait.advance(DT, Vector3::new(3.0, 0.0, 2.0), 1.1, 0.0);
            assert!(frame.contacts.is_empty());
            assert_eq!(frame.paws, first.paws);
            assert!(!frame.moving);
        }
    }

    /// Stopping mid-stride: paws caught in the air settle onto the floor
    /// with a touchdown each — then the cat is fully planted and silent.
    #[test]
    fn stopping_settles_airborne_paws() {
        let mut gait = CatGait::new(Vector3::ZERO, 0.0);
        let mut pos = Vector3::ZERO;
        // walk until at least one paw is mid-swing
        let mut airborne = false;
        for _ in 0..600 {
            pos += forward(0.0) * ((0.6 * DT) as f32);
            let frame = gait.advance(DT, pos, 0.0, 0.6);
            if frame.paws.iter().any(|p| p.y > 0.0) {
                airborne = true;
                break;
            }
        }
        assert!(airborne);
        // halt: the settle tick grounds every paw
        let settle = gait.advance(DT, pos, 0.0, 0.0);
        assert!(!settle.contacts.is_empty());
        assert!(settle.paws.iter().all(|p| p.y == 0.0));
        for _ in 0..60 {
            assert!(gait.advance(DT, pos, 0.0, 0.0).contacts.is_empty());
        }
    }

    /// Same inputs, same walk: two gaits fed the same script produce
    /// bit-identical frames — the determinism law, held per platform.
    #[test]
    fn identical_scripts_replay_identically() {
        let script = |gait: &mut CatGait| {
            let mut pos = Vector3::ZERO;
            let mut yaw = 0.0;
            let mut out = Vec::new();
            for i in 0..900 {
                let speed = if i % 300 < 200 { 0.55 } else { 0.0 };
                yaw += 0.4 * DT;
                pos += forward(yaw) * ((speed * DT) as f32);
                out.push(gait.advance(DT, pos, yaw, speed));
            }
            out
        };
        let mut a = CatGait::new(Vector3::ZERO, 0.0);
        let mut b = CatGait::new(Vector3::ZERO, 0.0);
        assert_eq!(script(&mut a), script(&mut b));
    }

    /// Walking a circle: every paw stays within its leg's honest reach of
    /// its own shoulder/hip anchor — the swing re-aim law. Without the
    /// per-frame re-aim, turning strands paws far behind the anchors.
    /// Measured worst case at this hard sustained turn: 0.197 m to the
    /// anchor, 0.352 m to the body center; pinned just above.
    #[test]
    fn turning_keeps_paws_under_the_body() {
        let mut gait = CatGait::new(Vector3::ZERO, 0.0);
        let mut pos = Vector3::ZERO;
        let mut yaw = 0.0_f64;
        for _ in 0..1800 {
            yaw += 0.9 * DT;
            pos += forward(yaw) * ((0.55 * DT) as f32);
            let frame = gait.advance(DT, pos, yaw, 0.55);
            for (leg, paw) in frame.paws.iter().enumerate() {
                let a = anchor(pos, yaw, leg);
                let da = Vector3::new(paw.x - a.x, 0.0, paw.z - a.z).length();
                assert!(f64::from(da) < 0.22, "leg {leg} strayed {da} m from anchor");
                let dc = Vector3::new(paw.x - pos.x, 0.0, paw.z - pos.z).length();
                assert!(f64::from(dc) < 0.38, "leg {leg} strayed {dc} m from body");
            }
        }
    }

    /// The narrow track: left paws keep left of the heading line, right
    /// paws right — prints that nearly line up, the tightrope walk.
    #[test]
    fn paws_keep_their_side_of_a_narrow_track() {
        let frames = walk_line(0.6, 1200);
        let rv = rightward(0.0);
        for frame in frames {
            for (leg, paw) in frame.paws.iter().enumerate() {
                let lateral = f64::from(paw.x * rv.x + paw.z * rv.z);
                assert!(lateral * SIDE[leg] > 0.0, "leg {leg} crossed the line");
                assert!(lateral.abs() < 0.09);
            }
        }
    }

    /// Only fore paws speak; hind paws direct-register in silence.
    #[test]
    fn fore_paws_sound_hind_paws_do_not() {
        assert!(paw_sounds(0));
        assert!(paw_sounds(1));
        assert!(!paw_sounds(2));
        assert!(!paw_sounds(3));
    }

    /// The paw-wave budget, against the REAL pool: a cat walking flat out
    /// at the design envelope for half a minute keeps its live footstep
    /// slots within the same 12-slot headroom the fan pins — the pool has
    /// 64, and the cat may claim no more of them than the fan does.
    #[test]
    fn paw_waves_stay_within_slot_headroom() {
        let mut gait = CatGait::new(Vector3::ZERO, 0.0);
        let mut pool = PulsePool::new();
        let mut pos = Vector3::ZERO;
        let mut now = 0.0;
        for _ in 0..(30.0 / DT) as usize {
            now += DT;
            pos += forward(0.0) * ((TOP_SPEED * DT) as f32);
            let frame = gait.advance(DT, pos, 0.0, TOP_SPEED);
            for c in frame.contacts.iter().filter(|c| paw_sounds(c.leg)) {
                pool.emit_omni(2, c.at, PAW_RANGE, PAW_SPEED, PAW_GAIN, now)
                    .unwrap();
            }
            assert!(pool.live_count(now) <= 12, "cat flooded the pool");
        }
    }
}
