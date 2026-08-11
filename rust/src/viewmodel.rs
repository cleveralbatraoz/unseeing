//! The hero's viewmodel mathematics — walk cycle, look-sway, strike
//! envelope, head-bob, footstep cadence and leg placement, mirrored
//! exactly from hero_body.gd. The visible body is the classical
//! first-person viewmodel of the validated design: a figure-eight walk
//! bob, look-sway lag behind the mouse, a strike kick that reaches the
//! cane tip out to the tap target and eases back, and phase-mirrored legs
//! whose footfalls keep honest time.
//!
//! Precision law, pinned from the original: GDScript floats are f64, so
//! every scalar here is f64; positions are Vector3 (f32 lanes), and
//! scalar arithmetic narrows to f32 exactly where GDScript assigned into
//! a Vector3 component — no earlier. The scene work (immediate meshes,
//! camera anchors) stays in the engine layer; this module owns the state
//! machine and curves the headless suites pin.

use godot::builtin::Vector3;

/// Walk-cycle rate, rad/s of leg and swing phase while moving.
pub const WALK_RATE: f64 = 7.4;

/// Head-bob amplitude in meters — the envelope every frame must respect.
pub const BOB_AMP: f64 = 0.028;

/// Look-sway clamp, horizontal (radians of viewmodel offset).
pub const SWAY_X_CLAMP: f64 = 0.07;

/// Look-sway clamp, vertical.
pub const SWAY_Y_CLAMP: f64 = 0.06;

/// Footstep cadence in seconds — one footfall per beat while walking.
pub const STEP_EVERY: f64 = 0.42;

/// The stop grace: standing still re-arms this many seconds, so a resumed
/// walk never double-fires on its first frame.
pub const STOP_GRACE: f64 = 0.1;

/// Seconds the strike takes to reach full extension.
pub const STRIKE_REACH: f64 = 0.07;

/// The ease-back time constant after full extension.
pub const STRIKE_DECAY: f64 = 0.28;

/// The strike envelope at `age` seconds after the tap: a quick visible
/// reach-out to the tap target (linear over [`STRIKE_REACH`]), then an
/// exponential ease back — verbatim from hero_body.gd, `maxf` included,
/// so a tap scheduled in the future reads as zero, not negative.
#[must_use]
pub fn strike_thrust(age: f64) -> f64 {
    if age < STRIKE_REACH {
        age.max(0.0) / STRIKE_REACH
    } else {
        (-(age - STRIKE_REACH) / STRIKE_DECAY).exp()
    }
}

/// The cane hover while sweeping: near the sweep extremes the lift dies
/// and the tip touches down; standing still holds a small constant hover.
/// `moving` is the original's `_walk_amp > 0.5` gate, decided by the
/// caller exactly as hero_body.gd decided it.
#[must_use]
pub fn cane_lift(moving: bool, cane_swing: f64) -> f64 {
    if moving {
        (1.0 - cane_swing.abs() / 0.26).max(0.0)
    } else {
        0.3
    }
}

/// One frame's animation outputs — everything the engine layer needs to
/// pose the arm, the cane and the camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// Whether the walk gate was open this frame (planar speed > 0.1).
    pub moving: bool,
    /// The strike envelope for this frame's cane reach.
    pub thrust: f64,
    /// The walk head-bob: world offset from the base eye height.
    pub bob: f64,
    /// Look-sway, horizontal — always inside [`SWAY_X_CLAMP`].
    pub sway_x: f64,
    /// Look-sway, vertical — always inside [`SWAY_Y_CLAMP`].
    pub sway_y: f64,
    /// The cane's sweep angle, eased toward its figure-eight target.
    pub cane_swing: f64,
    /// The walk cycle's leg phase, shared by legs, wobble and bob.
    pub leg_phase: f64,
    /// The walk amplitude easing between standing (0) and walking (1).
    pub walk_amp: f64,
}

/// One leg of the walking body, joint by joint — the exact kinematics of
/// hero_body.gd's `_build_body` loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegPose {
    /// The hip the thigh hangs from.
    pub hip: Vector3,
    /// The knee, thrown forward by the thigh angle.
    pub knee: Vector3,
    /// The ankle, clamped so the foot never digs into the floor.
    pub ankle: Vector3,
    /// The round shoe — where a footstep wave is born.
    pub shoe: Vector3,
}

/// One leg's pose for the current walk phase. `s` is the side (+1 right,
/// -1 left, mirrored by a half-turn phase shift); `p` the player's world
/// position, `fw`/`rv` the flattened forward and right vectors.
#[must_use]
pub fn leg_pose(
    p: Vector3,
    fw: Vector3,
    rv: Vector3,
    leg_phase: f64,
    walk_amp: f64,
    s: i32,
) -> LegPose {
    let ph = leg_phase + if s < 0 { std::f64::consts::PI } else { 0.0 };
    let thigh_a = 0.5 * ph.sin() * walk_amp;
    let knee_a = (0.95 * (ph - 0.9).sin()).max(0.0) * walk_amp;
    let shin_a = thigh_a - knee_a;
    let hip = Vector3::new(p.x, 0.90, p.z) + rv * 0.07 * s as f32 - fw * 0.20;
    // GDScript widened each component into its 64-bit floats and narrowed
    // on assignment; the same story here, operation for operation.
    let mut knee = hip + fw * (thigh_a.sin() as f32) * 0.45;
    knee.y = (f64::from(hip.y) - thigh_a.cos() * 0.45) as f32;
    let mut ankle = knee + fw * (shin_a.sin() as f32) * 0.45;
    ankle.y = (f64::from(knee.y) - shin_a.cos() * 0.45).max(0.07) as f32;
    let mut shoe = ankle + fw * 0.08;
    shoe.y = (f64::from(ankle.y) - 0.02).max(0.065) as f32;
    LegPose {
        hip,
        knee,
        ankle,
        shoe,
    }
}

/// Everything a Viewmodel is, as data — `step_t`/`step_side` included, or
/// a restored walker's next footstep fires on the wrong tick with the
/// wrong shoe. `Viewmodel::new` always starts the clock SPENT
/// (`step_t = 0.0`) and the alternation at right (`step_side = 1`); a
/// restore that reconstructed instead of copying these two would fire a
/// spurious footstep on the very next moving frame and reset which shoe
/// strikes first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewmodelCapture {
    pub walk_amp: f64,
    pub leg_phase: f64,
    pub swing_phase: f64,
    pub cane_swing: f64,
    pub sway_x: f64,
    pub sway_y: f64,
    pub last_yaw: f64,
    pub last_pitch: f64,
    pub step_t: f64,
    pub step_side: i32,
}

/// The viewmodel state machine: walk amplitude, phases, sway lag, and the
/// footstep clock. One instance per hero body, advanced once per rendered
/// frame — never by wall time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewmodel {
    walk_amp: f64,
    leg_phase: f64,
    swing_phase: f64,
    cane_swing: f64,
    sway_x: f64,
    sway_y: f64,
    last_yaw: f64,
    last_pitch: f64,
    step_t: f64,
    step_side: i32,
}

impl Viewmodel {
    /// A fresh viewmodel, caching the current look so the first frame
    /// reads zero look-delta — hero_body.gd's `_ready`. The footstep
    /// clock starts SPENT (`_step_t := 0.0`): a fresh walker steps at
    /// once, the pinned first-frame law.
    #[must_use]
    pub fn new(yaw: f64, pitch: f64) -> Self {
        Self {
            walk_amp: 0.0,
            leg_phase: 0.0,
            swing_phase: 0.0,
            cane_swing: 0.15,
            sway_x: 0.0,
            sway_y: 0.0,
            last_yaw: yaw,
            last_pitch: pitch,
            step_t: 0.0,
            step_side: 1,
        }
    }

    /// Advance one frame — hero_body.gd's `update` head, statement for
    /// statement: the walk gate, amplitude easing, phase advance, cane
    /// swing target, strike envelope, look-sway lag and head-bob.
    /// `planar_speed` is the player's horizontal velocity length,
    /// `last_tap` the player's tap clock reading.
    pub fn advance(
        &mut self,
        now: f64,
        dt: f64,
        planar_speed: f64,
        yaw: f64,
        pitch: f64,
        last_tap: f64,
    ) -> Pose {
        let moving = planar_speed > 0.1;
        self.walk_amp += ((if moving { 1.0 } else { 0.0 }) - self.walk_amp) * (dt * 6.0).min(1.0);
        if moving {
            self.swing_phase += dt * WALK_RATE;
            self.leg_phase += dt * WALK_RATE;
        }
        let swing_target = if moving {
            0.26 * self.swing_phase.sin()
        } else {
            0.12
        };
        self.cane_swing += (swing_target - self.cane_swing) * (dt * 10.0).min(1.0);

        let thrust = strike_thrust(now - last_tap);

        // look-sway: the viewmodel lags a touch behind mouse movement
        let inv_dt = 1.0 / dt.max(0.001);
        self.sway_x += (((yaw - self.last_yaw) * inv_dt * 0.02).clamp(-SWAY_X_CLAMP, SWAY_X_CLAMP)
            - self.sway_x)
            * (dt * 9.0).min(1.0);
        self.sway_y += (((pitch - self.last_pitch) * inv_dt * 0.015)
            .clamp(-SWAY_Y_CLAMP, SWAY_Y_CLAMP)
            - self.sway_y)
            * (dt * 9.0).min(1.0);
        self.last_yaw = yaw;
        self.last_pitch = pitch;

        let bob = BOB_AMP * (self.leg_phase * 2.0).sin() * self.walk_amp;

        Pose {
            moving,
            thrust,
            bob,
            sway_x: self.sway_x,
            sway_y: self.sway_y,
            cane_swing: self.cane_swing,
            leg_phase: self.leg_phase,
            walk_amp: self.walk_amp,
        }
    }

    /// The current horizontal look-sway — the bounded-envelope observable
    /// the engine layer republishes for the suites.
    #[must_use]
    pub fn sway_x(&self) -> f64 {
        self.sway_x
    }

    /// The current vertical look-sway — same observable, other axis.
    #[must_use]
    pub fn sway_y(&self) -> f64 {
        self.sway_y
    }

    /// The whole viewmodel as data — every field the walk cycle, the
    /// look-sway and the footstep clock carry between frames, `step_t`/
    /// `step_side` included.
    #[must_use]
    pub fn capture(&self) -> ViewmodelCapture {
        ViewmodelCapture {
            walk_amp: self.walk_amp,
            leg_phase: self.leg_phase,
            swing_phase: self.swing_phase,
            cane_swing: self.cane_swing,
            sway_x: self.sway_x,
            sway_y: self.sway_y,
            last_yaw: self.last_yaw,
            last_pitch: self.last_pitch,
            step_t: self.step_t,
            step_side: self.step_side,
        }
    }

    /// A viewmodel rebuilt mid-stride — the one thing `new` cannot express
    /// (it hard-codes the footstep clock SPENT and the alternation at
    /// right; see [`ViewmodelCapture`]'s doc for why that matters).
    #[must_use]
    pub fn restore(capture: ViewmodelCapture) -> Self {
        Self {
            walk_amp: capture.walk_amp,
            leg_phase: capture.leg_phase,
            swing_phase: capture.swing_phase,
            cane_swing: capture.cane_swing,
            sway_x: capture.sway_x,
            sway_y: capture.sway_y,
            last_yaw: capture.last_yaw,
            last_pitch: capture.last_pitch,
            step_t: capture.step_t,
            step_side: capture.step_side,
        }
    }

    /// The footstep clock — hero_body.gd's `_footsteps`, verbatim: idling
    /// re-arms the stop grace every frame; while walking the cadence
    /// counts down, and each expiry answers with the striking side (+1
    /// right, -1 left, starting right) and re-books a full beat.
    pub fn footstep(&mut self, dt: f64, moving: bool) -> Option<i32> {
        if !moving {
            self.step_t = STOP_GRACE;
            return None;
        }
        self.step_t -= dt;
        if self.step_t > 0.0 {
            return None;
        }
        let side = self.step_side;
        self.step_side = -self.step_side;
        self.step_t = STEP_EVERY;
        Some(side)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f64 = 1.0 / 60.0;

    fn walker() -> Viewmodel {
        Viewmodel::new(0.0, 0.0)
    }

    /// One scripted walking frame at the fixed step; `last_tap` far in
    /// the past unless the scenario taps.
    fn step(vm: &mut Viewmodel, now: &mut f64, speed: f64, last_tap: f64) -> (Pose, Option<i32>) {
        *now += DT;
        let pose = vm.advance(*now, DT, speed, 0.0, 0.0, last_tap);
        let fired = vm.footstep(DT, pose.moving);
        (pose, fired)
    }

    /// 1.5 s of walking: the first-ever step falls on the first moving
    /// frame (the clock starts spent), then every following step lands
    /// exactly when the 0.42 s cadence has fully elapsed — 26 fixed
    /// frames, never drifting. The gdUnit walk-cadence pin, in cargo.
    #[test]
    fn walk_cadence_never_drifts() {
        let mut vm = walker();
        let mut now = 0.0;
        let mut fires = Vec::new();
        for frame in 0..90 {
            if step(&mut vm, &mut now, 2.1, -10.0).1.is_some() {
                fires.push(frame);
            }
        }
        assert_eq!(fires, vec![0, 26, 52, 78]);
    }

    /// The shoes take turns, starting with the right one (+1) — the
    /// alternation half of the footstep-voice pin.
    #[test]
    fn steps_alternate_starting_right() {
        let mut vm = walker();
        let mut now = 0.0;
        let mut sides = Vec::new();
        for _ in 0..90 {
            if let Some(side) = step(&mut vm, &mut now, 2.1, -10.0).1 {
                sides.push(side);
            }
        }
        assert_eq!(sides, vec![1, -1, 1, -1]);
    }

    /// Standing still re-arms the 0.1 s stop grace every idle frame: the
    /// pause itself is silent, and the resumed walk fires exactly ONE
    /// step only after the grace has elapsed — never an instant
    /// double-fire on the first frame.
    #[test]
    fn stop_grace_prevents_a_double_fire_on_resume() {
        let mut vm = walker();
        let mut now = 0.0;
        let mut walked = 0;
        for _ in 0..30 {
            if step(&mut vm, &mut now, 2.1, -10.0).1.is_some() {
                walked += 1;
            }
        }
        assert_eq!(walked, 2); // frames 0 and 26
        for _ in 0..30 {
            assert_eq!(step(&mut vm, &mut now, 0.0, -10.0).1, None); // silence while still
        }
        let mut resume_fires = Vec::new();
        for frame in 0..12 {
            if step(&mut vm, &mut now, 2.1, -10.0).1.is_some() {
                resume_fires.push(frame);
            }
        }
        assert_eq!(resume_fires.len(), 1);
        assert!((5..=7).contains(&resume_fires[0])); // ~0.1 s of walking first
    }

    /// The walk gate is a strict threshold on planar speed: a crawl at
    /// 0.1 m/s is standing, anything past it walks.
    #[test]
    fn walk_gate_threshold() {
        let mut vm = walker();
        assert!(!vm.advance(DT, DT, 0.1, 0.0, 0.0, -10.0).moving);
        assert!(vm.advance(DT * 2.0, DT, 0.100_001, 0.0, 0.0, -10.0).moving);
    }

    /// The whole scripted life of the gdUnit envelope suite — a nervous
    /// walk under an aggressive look wander, a tap, a stop — with the
    /// envelope held on EVERY frame: head-bob inside its amplitude,
    /// look-sway inside its clamps, thrust inside [0, 1].
    #[test]
    fn walk_tap_stop_stays_bounded() {
        let mut vm = walker();
        let mut now = 0.0;
        let mut yaw = 0.0;
        let mut last_tap = -10.0;
        let check = |pose: Pose| {
            assert!(pose.bob.abs() <= BOB_AMP);
            assert!(pose.sway_x.abs() <= SWAY_X_CLAMP);
            assert!(pose.sway_y.abs() <= SWAY_Y_CLAMP);
            assert!((0.0..=1.0).contains(&pose.thrust));
        };
        for frame in 0..120 {
            yaw += 0.2 * (f64::from(frame) * 0.3).sin();
            let pitch = 0.9 * (f64::from(frame) * 0.17).sin();
            now += DT;
            let pose = vm.advance(now, DT, 2.1, yaw, pitch, last_tap);
            vm.footstep(DT, pose.moving);
            check(pose);
        }
        last_tap = now;
        for _ in 0..30 {
            now += DT;
            let pose = vm.advance(now, DT, 2.1, yaw, 0.0, last_tap);
            vm.footstep(DT, pose.moving);
            check(pose);
        }
        for _ in 0..60 {
            now += DT;
            let pose = vm.advance(now, DT, 0.0, yaw, 0.0, last_tap);
            vm.footstep(DT, pose.moving);
            check(pose);
        }
        // eased home: the stop drains the walk amplitude
        assert!(vm.advance(now + DT, DT, 0.0, yaw, 0.0, last_tap).walk_amp < 0.05);
    }

    /// The strike envelope: zero before the tap (maxf law), a linear ramp
    /// to full extension at exactly 0.07 s, then a monotonic ease back.
    #[test]
    fn strike_thrust_reaches_then_eases_back() {
        assert_eq!(strike_thrust(-1.0), 0.0);
        assert_eq!(strike_thrust(0.0), 0.0);
        assert!((strike_thrust(0.035) - 0.5).abs() < 1e-12);
        assert_eq!(strike_thrust(STRIKE_REACH), 1.0);
        let mut prev = 1.0;
        for i in 1..40 {
            let v = strike_thrust(STRIKE_REACH + f64::from(i) * 0.05);
            assert!(v < prev);
            assert!(v > 0.0);
            prev = v;
        }
    }

    /// Both shoes stay at or above their floor through a full walk cycle,
    /// at every amplitude — the gdUnit shoe-floor pin (>= 0.0649), plus
    /// the ankle's own 0.07 clamp.
    #[test]
    fn shoes_stay_on_or_above_the_floor() {
        let p = Vector3::new(0.0, 0.9, 0.0);
        let fw = Vector3::new(0.0, 0.0, -1.0);
        let rv = Vector3::new(1.0, 0.0, 0.0);
        for amp10 in 0..=10 {
            let amp = f64::from(amp10) / 10.0;
            for i in 0..200 {
                let phase = f64::from(i) * 0.05;
                for s in [-1, 1] {
                    let leg = leg_pose(p, fw, rv, phase, amp, s);
                    assert!(leg.shoe.y >= 0.0649);
                    assert!(leg.ankle.y >= 0.07 - 1e-6);
                }
            }
        }
    }

    /// The mirrored walk: at any phase the two legs are half a cycle
    /// apart — the left leg at phase t strikes the pose the right leg
    /// strikes at t + PI, shoes included.
    #[test]
    fn legs_mirror_half_a_cycle_apart() {
        let p = Vector3::new(0.0, 0.9, 0.0);
        let fw = Vector3::new(0.0, 0.0, -1.0);
        let rv = Vector3::new(1.0, 0.0, 0.0);
        let left = leg_pose(p, fw, rv, 1.3, 1.0, -1);
        let right = leg_pose(p, fw, rv, 1.3 + std::f64::consts::PI, 1.0, 1);
        // same gait, opposite hips: the z/y trajectories coincide
        assert!((f64::from(left.shoe.z) - f64::from(right.shoe.z)).abs() < 1e-5);
        assert!((f64::from(left.shoe.y) - f64::from(right.shoe.y)).abs() < 1e-5);
        assert!((f64::from(left.hip.x) + f64::from(right.hip.x)).abs() < 1e-5);
    }

    /// The shoes sit a hip's width apart: facing -Z the right shoe rides
    /// world x = +0.07 and the left x = -0.07 — the footstep-wave
    /// birthplace the gdUnit suite pins to ±0.07.
    #[test]
    fn shoes_ride_the_hip_offsets() {
        let p = Vector3::new(0.0, 0.9, 0.0);
        let fw = Vector3::new(0.0, 0.0, -1.0);
        let rv = Vector3::new(1.0, 0.0, 0.0);
        for s in [-1, 1] {
            let leg = leg_pose(p, fw, rv, 2.2, 1.0, s);
            assert!((f64::from(leg.shoe.x) - 0.07 * f64::from(s)).abs() < 1e-6);
        }
    }

    /// The cane hover: full lift at the sweep's center, touchdown at the
    /// extremes, a constant 0.3 hover while standing.
    #[test]
    fn cane_lift_touches_down_at_the_extremes() {
        assert_eq!(cane_lift(true, 0.0), 1.0);
        assert_eq!(cane_lift(true, 0.26), 0.0);
        assert_eq!(cane_lift(true, -0.3), 0.0);
        assert!((cane_lift(true, 0.13) - 0.5).abs() < 1e-12);
        assert_eq!(cane_lift(false, 0.26), 0.3);
    }

    /// The restored walker's NEXT footstep lands exactly when the
    /// original's would — timing and which shoe. A reconstructed
    /// viewmodel (new()) cannot do this: it starts with the step clock
    /// SPENT and the alternation reset to right.
    #[test]
    fn a_restored_walker_keeps_its_step_clock_and_its_next_shoe() {
        let mut original = Viewmodel::new(0.0, 0.0);
        // walk under a turning, nodding look until the third footstep
        // fires: advance() runs every frame so every eased field —
        // walk_amp, leg_phase, swing_phase, sway_x, sway_y, last_yaw,
        // last_pitch — settles at a distinct, nonzero value before
        // capture. A restore that transposed any pair of captured fields
        // (e.g. swapped sway_x and sway_y) must be observable here, not
        // masked by two matching zeros.
        let mut now = 0.0;
        let mut frame = 0.0;
        let mut fired = None;
        let mut steps = 0;
        while fired.is_none() || steps < 3 {
            now += 0.05;
            frame += 1.0;
            let pose = original.advance(now, 0.05, 2.1, 0.11 * frame, 0.04 * frame, -10.0);
            fired = original.footstep(0.05, pose.moving);
            if fired.is_some() {
                steps += 1;
            }
        }
        // the loop exits right after the 3rd firing: step_t is freshly
        // re-booked to the full 0.42 s interval (not mid-count), and
        // step_side is whichever shoe the alternation has reached — both
        // still exactly the state a restore must reproduce.
        let mut restored = Viewmodel::restore(original.capture());
        assert_eq!(restored, original);
        // lockstep to the next firing: same tick, same side
        loop {
            let a = original.footstep(0.05, true);
            let b = restored.footstep(0.05, true);
            assert_eq!(a, b);
            if a.is_some() {
                break;
            }
        }
        // and the spurious-first-step failure a fresh walker would show:
        let mut fresh = Viewmodel::new(0.0, 0.0);
        assert!(fresh.footstep(0.05, true).is_some()); // fires at once
    }
}
