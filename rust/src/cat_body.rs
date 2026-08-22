//! The cat's skeleton — joint positions from gait state, nothing more.
//! The perception laws strip a cat of fur, eyes and face; everything the
//! player will ever read is SILHOUETTE: the nose-to-tail line, the
//! narrow-tracked legs with their backward hocks, the ears, the tail
//! carried high on a walk and curled around a sit, whiskers that are
//! literally thin outlines. This module turns a [`crate::cat_gait`]
//! frame plus a sit blend into that silhouette's joints; the engine node
//! merely wraps tubes and spheres around them.
//!
//! Legs are two-bone analytic IK — law of cosines, total at the door:
//! unreachable paws straighten the leg toward them, degenerate targets
//! fall back to pointing straight down, segment lengths are preserved
//! always. The tail is a follow-chain: each joint eases after the one
//! before it with a lag that grows toward the tip, so every turn of the
//! body writes a whip of history into the air — deterministic, dt-driven,
//! no physics.

use godot::builtin::Vector3;

use crate::cat_gait::{self, GaitFrame, LEGS};
use crate::reproduce::RestoreValueError;
use crate::support_motion::{
    ActorPosition, ActorYaw, MAX_POSE_COORD_M, MotionValueError, PosePoint, StepDuration,
    SupportElevation,
};

/// Chest-line height above the current support while standing, meters.
pub const CHEST_H: f64 = cat_gait::BODY_H;

/// Hip-line height while standing — a cat's rump rides a touch higher.
pub const HIP_H: f64 = 0.21;

/// Hip height when fully seated: haunches on the ground.
pub const HIP_SIT_H: f64 = 0.075;

/// Fore upper-leg (shoulder to elbow) length. Leg lengths are sized to
/// the gait's measured steady-state reach demand (0.237 m fore, 0.235 m
/// hind at the stance extremes) plus margin: a standing leg stays
/// pleasantly bent, and only the launch transient — the last leg in the
/// sequence waiting out most of a cycle from a cold standstill — may
/// briefly straighten against the IK clamp.
pub const FORE_UPPER: f64 = 0.13;

/// Fore lower-leg (elbow to paw) length.
pub const FORE_LOWER: f64 = 0.12;

/// Hind upper-leg (hip to hock) length — the long, deep-crouched pair.
pub const HIND_UPPER: f64 = 0.14;

/// Hind lower-leg (hock to paw) length.
pub const HIND_LOWER: f64 = 0.12;

/// Tail joints after the root.
pub const TAIL_N: usize = 5;

/// One tail segment's length, meters.
pub const TAIL_SEG: f64 = 0.062;

/// The most a tail segment may bend from the one before it, radians — a
/// structural ceiling that forbids the follow-chain from coiling into the
/// closed loops that read as glitches. ~55°.
pub const TAIL_MAX_BEND: f64 = 0.96;

/// Whiskers on the muzzle — two per side, four thin outlines. Six turned
/// the distant head into a starburst; four keep the muzzle legible.
pub const WHISKER_N: usize = 4;

/// Whisker length — short thin outlines that don't dominate the head.
pub const WHISKER_LEN: f64 = 0.055;

/// One leg's joints, root to paw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leg {
    /// Shoulder (fore) or hip (hind).
    pub root: Vector3,
    /// Elbow (fore) or hock (hind) — the silhouette's bend.
    pub mid: Vector3,
    /// The paw itself.
    pub paw: Vector3,
}

/// Every joint the mesh needs, one frame's worth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Skeleton {
    /// Front end of the torso line.
    pub chest: Vector3,
    /// Rear end of the torso line.
    pub hip: Vector3,
    /// Where the neck leaves the chest.
    pub neck: Vector3,
    /// The head's center.
    pub head: Vector3,
    /// The muzzle tip the whiskers grow from.
    pub muzzle: Vector3,
    /// Ear base and tip, left then right.
    pub ears: [(Vector3, Vector3); 2],
    /// Whisker root and tip, two per side.
    pub whiskers: [(Vector3, Vector3); WHISKER_N],
    /// LF, RF, LH, RH.
    pub legs: [Leg; LEGS],
    /// Where the tail chain hangs from.
    pub tail_root: Vector3,
    /// The direction away from the body at the tail root.
    pub tail_back: Vector3,
}

/// Everything the skeleton needs for one frame: the gait's outputs plus
/// the eased sit blend the node owns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CatPose {
    /// Body center on the current support.
    pub pos: Vector3,
    /// Heading, radians.
    pub yaw: f64,
    /// Paw positions from the gait, LF RF LH RH.
    pub paws: [Vector3; LEGS],
    /// The walk bob this frame.
    pub bob: f64,
    /// Walk amplitude — the head drops into a prowl as it rises.
    pub amp: f64,
    /// Sit blend, 0 standing to 1 seated.
    pub sit: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct PreparedCatPose(CatPose);

impl CatPose {
    pub fn prepare_restore(capture: CatPose) -> Result<PreparedCatPose, RestoreValueError> {
        validate_restore_point("pose.pos", capture.pos)?;
        for (index, paw) in capture.paws.iter().copied().enumerate() {
            validate_restore_point(&format!("pose.paws[{index}]"), paw)?;
        }
        for (field, value) in [
            ("bob", capture.bob),
            ("amp", capture.amp),
            ("sit", capture.sit),
        ] {
            if !value.is_finite() {
                return Err(RestoreValueError::new(
                    format!("pose.{field}"),
                    "must be finite",
                ));
            }
        }
        ActorYaw::try_new(capture.yaw).map_err(|_| {
            RestoreValueError::new("pose.yaw", "must narrow to a finite Godot rotation lane")
        })?;
        if capture.bob.abs() > cat_gait::BOB_AMP {
            return Err(RestoreValueError::new(
                "pose.bob",
                "is outside the gait bob envelope",
            ));
        }
        for (field, value) in [("amp", capture.amp), ("sit", capture.sit)] {
            if !(0.0..=1.0).contains(&value) {
                return Err(RestoreValueError::new(
                    format!("pose.{field}"),
                    "must be in 0..=1",
                ));
            }
        }
        Ok(PreparedCatPose(capture))
    }

    #[must_use]
    pub fn from_prepared(capture: PreparedCatPose) -> Self {
        capture.0
    }

    /// A pose straight off a fully checked gait frame — the node adds its
    /// eased sit after validating that public frame's complete domain.
    pub fn try_from_gait(
        pos: ActorPosition,
        yaw: ActorYaw,
        frame: &GaitFrame,
        sit: f64,
    ) -> Result<Self, MotionValueError> {
        validate_gait_frame(frame)?;
        validate_unit_interval(sit, "cat_pose.sit")?;
        let pose = Self {
            pos: pos.world(),
            yaw: yaw.radians(),
            paws: frame.paws,
            bob: frame.bob,
            amp: frame.amp,
            sit,
        };
        validate_pose(&pose)?;
        Ok(pose)
    }
}

fn validate_gait_frame(frame: &GaitFrame) -> Result<(), MotionValueError> {
    for paw in frame.paws {
        PosePoint::try_new(paw)?;
    }
    if frame.contacts.len() > LEGS {
        return Err(MotionValueError::out_of_range("cat_pose.contacts"));
    }
    for contact in &frame.contacts {
        if contact.leg >= LEGS {
            return Err(MotionValueError::out_of_range("cat_pose.contact.leg"));
        }
        PosePoint::try_new(contact.at)?;
    }
    validate_half_open_phase(frame.phase, "cat_pose.phase")?;
    validate_unit_interval(frame.amp, "cat_pose.amp")?;
    if !frame.bob.is_finite() {
        return Err(MotionValueError::non_finite("cat_pose.bob"));
    }
    if frame.bob.abs() > cat_gait::BOB_AMP {
        return Err(MotionValueError::out_of_range("cat_pose.bob"));
    }
    if !frame.support_delta_y.is_finite() {
        return Err(MotionValueError::non_finite("cat_pose.support_delta_y"));
    }
    Ok(())
}

fn validate_pose(pose: &CatPose) -> Result<(), MotionValueError> {
    PosePoint::try_new(pose.pos)?;
    ActorYaw::try_new(pose.yaw)?;
    for paw in pose.paws {
        PosePoint::try_new(paw)?;
    }
    if !pose.bob.is_finite() {
        return Err(MotionValueError::non_finite("cat_pose.bob"));
    }
    if pose.bob.abs() > cat_gait::BOB_AMP {
        return Err(MotionValueError::out_of_range("cat_pose.bob"));
    }
    validate_unit_interval(pose.amp, "cat_pose.amp")?;
    validate_unit_interval(pose.sit, "cat_pose.sit")?;
    Ok(())
}

fn validate_half_open_phase(value: f64, field: &'static str) -> Result<(), MotionValueError> {
    if !value.is_finite() {
        return Err(MotionValueError::non_finite(field));
    }
    if !(0.0..1.0).contains(&value) {
        return Err(MotionValueError::out_of_range(field));
    }
    Ok(())
}

fn validate_unit_interval(value: f64, field: &'static str) -> Result<(), MotionValueError> {
    if !value.is_finite() {
        return Err(MotionValueError::non_finite(field));
    }
    if !(0.0..=1.0).contains(&value) {
        return Err(MotionValueError::out_of_range(field));
    }
    Ok(())
}

/// The whole skeleton for one frame — total over the still-public wire pose:
/// malformed lanes refuse before arithmetic and every derived joint is
/// checked against the pose envelope before it leaves this pure law.
pub fn skeleton(pose: &CatPose) -> Result<Skeleton, MotionValueError> {
    validate_pose(pose)?;
    let fw = forward(pose.yaw);
    let rv = rightward(pose.yaw);
    let support_y = pose.pos.y;
    let ground = Vector3::new(pose.pos.x, 0.0, pose.pos.z);
    let mut local_paws = pose.paws;
    for paw in &mut local_paws {
        paw.y = (f64::from(paw.y) - f64::from(support_y)) as f32;
    }
    let sit = pose.sit;

    // the torso line: chest and hip ride their standing heights, then the
    // sit folds the haunches to the floor and lifts the chest proud
    let chest_stand = ground + fw * 0.145 + up((CHEST_H + pose.bob) as f32);
    let chest_sit = ground + fw * 0.115 + up(0.235);
    let chest = chest_stand.lerp(chest_sit, sit as f32);
    let hip_stand = ground - fw * 0.145 + up((HIP_H + pose.bob * 0.6) as f32);
    let hip_sit = ground - fw * 0.095 + up(HIP_SIT_H as f32);
    let hip = hip_stand.lerp(hip_sit, sit as f32);

    // the head: alert when standing, carried low into the walk, high and
    // proud in a sit
    let neck = chest + fw * 0.055 + up(0.05);
    let head_stand = neck + fw * 0.07 + up(0.06);
    let head_walk = neck + fw * 0.10 + up(0.025);
    let head_sit = neck + fw * 0.03 + up(0.085);
    let head = head_stand
        .lerp(head_walk, pose.amp as f32)
        .lerp(head_sit, sit as f32);
    let muzzle = head + fw * 0.048 - up(0.006);

    let mut ears = [(Vector3::ZERO, Vector3::ZERO); 2];
    for (e, side) in [-1.0_f32, 1.0].into_iter().enumerate() {
        let base = head + rv * (0.030 * side) + up(0.036) - fw * 0.004;
        let tip = base + up(0.043) + rv * (0.011 * side) - fw * 0.004;
        ears[e] = (base, tip);
    }

    let mut whiskers = [(Vector3::ZERO, Vector3::ZERO); WHISKER_N];
    for (w, spot) in whiskers.iter_mut().enumerate() {
        let side = if w < WHISKER_N / 2 { -1.0_f32 } else { 1.0 };
        let row = (w % (WHISKER_N / 2)) as f64;
        let root = muzzle + rv * (0.011 * side) + up((0.003 - 0.006 * row) as f32);
        // splayed sideways and forward, a gentle downward fan — never the
        // near-vertical spikes that starburst at distance
        let dir = (rv * (side * 1.1) + fw * 0.5 + up((0.04 - 0.09 * row) as f32)).normalized();
        *spot = (root, root + dir * (WHISKER_LEN as f32));
    }

    // legs: roots on the torso line, paws from the gait — folded toward
    // the sit posture as the blend rises (fore planted under the chest,
    // hind tucked beside the dropped haunches)
    let mut legs = [Leg {
        root: Vector3::ZERO,
        mid: Vector3::ZERO,
        paw: Vector3::ZERO,
    }; LEGS];
    for (leg, out) in legs.iter_mut().enumerate() {
        let side = if leg % 2 == 0 { -1.0_f32 } else { 1.0 };
        let fore = leg < 2;
        let (root, upper, lower) = if fore {
            (chest + rv * (0.048 * side), FORE_UPPER, FORE_LOWER)
        } else {
            (hip + rv * (0.055 * side), HIND_UPPER, HIND_LOWER)
        };
        let sit_paw = if fore {
            Vector3::new(chest.x, ground.y, chest.z) + rv * (0.048 * side) + fw * 0.02
        } else {
            Vector3::new(hip.x, ground.y, hip.z) + rv * (0.07 * side) + fw * 0.06
        };
        let paw = local_paws[leg].lerp(sit_paw, sit as f32);
        // the seated hind pair folds its hock UP, not down through the
        // floor: a deep sit drops the hip low, and on the plain backward
        // hint the long hind leg — folded nearly double — bulged its knee
        // under the ground. Lift the bend hint toward vertical as the sit
        // rises. Fore legs straighten under the raised chest and never
        // bulge, so they keep the backward hint.
        let hint = if fore {
            -fw
        } else {
            (-fw).lerp(Vector3::UP, sit as f32)
        };
        let (mid, end) = two_bone(root, paw, upper, lower, hint);
        *out = Leg {
            root,
            mid,
            paw: end,
        };
    }

    let tail_root = hip - fw * 0.065 + up(0.03);

    let mut skeleton = Skeleton {
        chest,
        hip,
        neck,
        head,
        muzzle,
        ears,
        whiskers,
        legs,
        tail_root,
        tail_back: -fw,
    };
    translate_skeleton_y(&mut skeleton, support_y);
    validate_skeleton(&skeleton)?;
    Ok(skeleton)
}

fn translate_skeleton_y(skeleton: &mut Skeleton, delta_y: f32) {
    if delta_y == 0.0 {
        return;
    }
    for point in [
        &mut skeleton.chest,
        &mut skeleton.hip,
        &mut skeleton.neck,
        &mut skeleton.head,
        &mut skeleton.muzzle,
        &mut skeleton.tail_root,
    ] {
        point.y += delta_y;
    }
    for (base, tip) in &mut skeleton.ears {
        base.y += delta_y;
        tip.y += delta_y;
    }
    for (root, tip) in &mut skeleton.whiskers {
        root.y += delta_y;
        tip.y += delta_y;
    }
    for leg in &mut skeleton.legs {
        leg.root.y += delta_y;
        leg.mid.y += delta_y;
        leg.paw.y += delta_y;
    }
}

fn validate_skeleton(skeleton: &Skeleton) -> Result<(), MotionValueError> {
    for point in [
        skeleton.chest,
        skeleton.hip,
        skeleton.neck,
        skeleton.head,
        skeleton.muzzle,
        skeleton.tail_root,
        skeleton.tail_back,
    ] {
        PosePoint::try_new(point)?;
    }
    for (base, tip) in skeleton.ears {
        PosePoint::try_new(base)?;
        PosePoint::try_new(tip)?;
    }
    for (root, tip) in skeleton.whiskers {
        PosePoint::try_new(root)?;
        PosePoint::try_new(tip)?;
    }
    for leg in skeleton.legs {
        for point in [leg.root, leg.mid, leg.paw] {
            PosePoint::try_new(point)?;
        }
    }
    Ok(())
}

/// Two-bone analytic IK, total over any target: the reach is clamped
/// into the triangle inequality (an unreachable paw straightens the leg
/// toward it, a target at the root points it straight down), and the
/// bend leaves toward `hint` — behind the cat, where elbows and hocks
/// live.
#[must_use]
fn two_bone(
    root: Vector3,
    target: Vector3,
    upper: f64,
    lower: f64,
    hint: Vector3,
) -> (Vector3, Vector3) {
    let to = target - root;
    let d_raw = f64::from(to.length());
    let dir = if d_raw > 1e-6 {
        to / (d_raw as f32)
    } else {
        Vector3::new(0.0, -1.0, 0.0)
    };
    let d = d_raw.clamp((upper - lower).abs() + 1e-4, upper + lower - 1e-4);
    let end = root + dir * (d as f32);
    let along = (upper * upper + d * d - lower * lower) / (2.0 * d);
    let h = (upper * upper - along * along).max(0.0).sqrt();
    let lean = hint - dir * hint.dot(dir);
    let perp = if f64::from(lean.length()) > 1e-6 {
        lean.normalized()
    } else {
        // the hint lies along the leg: bend anywhere consistent — pick
        // the world axis least aligned with the leg
        let fallback = if f64::from(dir.x.abs()) < 0.9 {
            Vector3::new(1.0, 0.0, 0.0)
        } else {
            Vector3::new(0.0, 0.0, 1.0)
        };
        (fallback - dir * fallback.dot(dir)).normalized()
    };
    let mid = root + dir * (along as f32) + perp * (h as f32);
    (mid, end)
}

/// The tail: a follow-chain of [`TAIL_N`] joints hanging off the hip.
/// Each joint eases toward its rest place behind the previous one, the
/// tip lagging most — the body's history, written in the air.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tail {
    nodes: [Vector3; TAIL_N],
}

#[derive(Debug, Clone, Copy)]
pub struct PreparedTail(Tail);

impl Tail {
    pub fn prepare_restore(capture: [Vector3; TAIL_N]) -> Result<PreparedTail, RestoreValueError> {
        for (index, point) in capture.iter().copied().enumerate() {
            PosePoint::try_new(point).map_err(|error| {
                let axis = error.field().rsplit('.').next().unwrap_or(error.field());
                RestoreValueError::new(
                    format!("tail[{index}].{axis}"),
                    "must be finite and inside the pose envelope",
                )
            })?;
        }
        Ok(PreparedTail(Self::restore(capture)))
    }

    #[must_use]
    pub fn from_prepared(capture: PreparedTail) -> Self {
        capture.0
    }

    /// A fresh tail already in its standing rest curve.
    pub fn new(
        root: PosePoint,
        yaw: ActorYaw,
        support: SupportElevation,
    ) -> Result<Self, MotionValueError> {
        let mut tail = Self {
            nodes: [root.world(); TAIL_N],
        };
        // settle instantly into the rest pose: a long advance from rest
        for _ in 0..120 {
            tail.advance(StepDuration::from_raw(0.1), root, yaw, support, 0.0, 0.0)?;
        }
        Ok(tail)
    }

    /// The joints, root-side first.
    #[must_use]
    pub fn nodes(&self) -> &[Vector3; TAIL_N] {
        &self.nodes
    }

    /// A chain rebuilt at an exact curve. `new` settles toward rest by
    /// iterating — correct for a spawn, wrong for a restore.
    fn restore(nodes: [Vector3; TAIL_N]) -> Self {
        Self { nodes }
    }

    /// Translate the whole remembered curve by one exact support delta.
    /// Validation happens on a copy so an invalid delta or boundary overflow
    /// preserves every prior node bit.
    pub fn transport_y(&mut self, delta_y: f32) -> Result<(), MotionValueError> {
        if !delta_y.is_finite() {
            return Err(MotionValueError::non_finite("tail.support_delta_y"));
        }
        if delta_y == 0.0 {
            return Ok(());
        }
        let lift = Vector3::new(0.0, delta_y, 0.0);
        let mut next = self.nodes;
        for node in &mut next {
            *node = PosePoint::try_new(*node + lift)?.world();
        }
        self.nodes = next;
        Ok(())
    }

    /// One tick: yaw supplies the direction away from the body and the cat's
    /// right; `sit` curls the tail around the haunches and `sway` is the
    /// signed lateral weight driven by stride and idle breath.
    pub fn advance(
        &mut self,
        dt: StepDuration,
        root: PosePoint,
        yaw: ActorYaw,
        support: SupportElevation,
        sit: f64,
        sway: f64,
    ) -> Result<(), MotionValueError> {
        if !sit.is_finite() {
            return Err(MotionValueError::non_finite("tail.sit"));
        }
        if !(0.0..=1.0).contains(&sit) {
            return Err(MotionValueError::out_of_range("tail.sit"));
        }
        if !sway.is_finite() {
            return Err(MotionValueError::non_finite("tail.sway"));
        }
        let mut next = self.nodes;
        let support_y = support.y();
        let world_root = root.world();
        let mut local_root = world_root;
        // The shipped flat curve is an exact capture/replay contract. Keep its
        // world-space f32 path untouched at +0 support; once elevated, follow
        // in the root frame so a shared world translation cannot change a bend.
        if support_y != 0.0 {
            local_root = Vector3::ZERO;
            for node in &mut next {
                *node -= world_root;
            }
        }
        Self::advance_nodes(
            &mut next,
            dt.seconds(),
            local_root,
            -forward(yaw.radians()),
            rightward(yaw.radians()),
            sit,
            sway,
        );
        if support_y != 0.0 {
            for point in &mut next {
                *point += world_root;
            }
        }
        for point in next {
            PosePoint::try_new(point)?;
        }
        self.nodes = next;
        Ok(())
    }

    fn advance_nodes(
        nodes: &mut [Vector3; TAIL_N],
        dt: f64,
        root: Vector3,
        back: Vector3,
        rv: Vector3,
        sit: f64,
        sway: f64,
    ) {
        let mut prev = root;
        let mut prev_dir = back.normalized();
        for (i, node) in nodes.iter_mut().enumerate() {
            let u = i as f64 / (TAIL_N - 1) as f64;
            // standing: a graceful trailing arc — mostly back at the base,
            // rising to ~45° up-and-back at the tip (never hooked straight
            // over the spine). sitting: laid gently to one side beside the
            // haunches, not wrapped into a loop.
            let stand = back * ((1.0 - u * 0.35) as f32)
                + Vector3::UP * ((0.12 + u * 0.55) as f32)
                + rv * ((sway * (0.3 + u * 0.7)) as f32);
            let curl = back * ((0.6 - u * 0.25) as f32) + rv * ((u * 0.85) as f32)
                - Vector3::UP * ((0.05 + u * 0.05) as f32);
            let rest = stand.lerp(curl, sit as f32);
            let rest = if f64::from(rest.length()) > 1e-6 {
                rest.normalized()
            } else {
                prev_dir
            };
            let target = prev + rest * (TAIL_SEG as f32);
            let rate = 14.0 - 2.2 * i as f64;
            *node += (target - *node) * ((dt * rate).min(1.0) as f32);
            // the chain never stretches, and never coils: re-seat on the
            // segment length, then clamp the bend against the previous
            // segment so the follow-chain can't fold into a closed loop
            let span = *node - prev;
            let dir = if f64::from(span.length()) > 1e-6 {
                span.normalized()
            } else {
                prev_dir
            };
            let dir = bend_clamp(dir, prev_dir);
            *node = prev + dir * (TAIL_SEG as f32);
            prev = *node;
            prev_dir = dir;
        }
    }
}

fn validate_restore_point(path: &str, point: Vector3) -> Result<(), RestoreValueError> {
    for (axis, lane) in [("x", point.x), ("y", point.y), ("z", point.z)] {
        if !lane.is_finite() {
            return Err(RestoreValueError::new(
                format!("{path}.{axis}"),
                "must be finite",
            ));
        }
        if lane.abs() > MAX_POSE_COORD_M {
            return Err(RestoreValueError::new(
                format!("{path}.{axis}"),
                "is outside its valid range",
            ));
        }
    }
    Ok(())
}

/// Clamp `dir` so it bends at most [`TAIL_MAX_BEND`] radians from `prev`.
/// Beyond the cone, `dir` is pulled back to the cone's edge in the plane
/// the two share — the structural guard against tail loops.
fn bend_clamp(dir: Vector3, prev: Vector3) -> Vector3 {
    let max_cos = TAIL_MAX_BEND.cos() as f32;
    let max_sin = TAIL_MAX_BEND.sin() as f32;
    let cos = dir.dot(prev).clamp(-1.0, 1.0);
    if cos >= max_cos {
        return dir;
    }
    let perp = dir - prev * cos;
    if f64::from(perp.length()) <= 1e-6 {
        return dir; // antiparallel: no defined plane, leave it
    }
    (prev * max_cos + perp.normalized() * max_sin).normalized()
}

/// The heading's forward vector — Godot yaw convention: yaw 0 faces -Z.
fn forward(yaw: f64) -> Vector3 {
    Vector3::new((-yaw.sin()) as f32, 0.0, (-yaw.cos()) as f32)
}

/// The heading's right vector.
fn rightward(yaw: f64) -> Vector3 {
    Vector3::new(yaw.cos() as f32, 0.0, (-yaw.sin()) as f32)
}

/// A vertical lift — sugar for the many `+ up * h` spots.
fn up(h: f32) -> Vector3 {
    Vector3::new(0.0, h, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    use godot::builtin::Vector2;

    use crate::cat_brain::{CatBrain, RoamRect};
    use crate::support_motion::{
        ActorPosition, ActorYaw, FiniteMeasure, MotionValueProblem, PosePoint, StepDuration,
    };

    fn actor_position(world: Vector3) -> ActorPosition {
        ActorPosition::try_new(world).expect("test position must be in the actor domain")
    }

    fn actor_yaw(radians: f64) -> ActorYaw {
        ActorYaw::try_new(radians).expect("test yaw must be in the actor domain")
    }

    fn pose_point(world: Vector3) -> PosePoint {
        PosePoint::try_new(world).expect("test point must be in the pose domain")
    }

    fn support(y: f32) -> SupportElevation {
        SupportElevation::try_new(y).expect("test support must be in the elevation domain")
    }

    fn duration(seconds: f64) -> StepDuration {
        StepDuration::from_raw(seconds)
    }

    fn speed(meters_per_second: f64) -> FiniteMeasure {
        FiniteMeasure::try_new(meters_per_second, "test.actual_speed")
            .expect("test speed must be finite and non-negative")
    }

    fn pose_and_frame_at(y: f32, sit: f64) -> (CatPose, GaitFrame) {
        let position = actor_position(Vector3::new(0.25, y, -0.5));
        let yaw = actor_yaw(0.35);
        let mut gait = CatGait::new(position, yaw).unwrap();
        let frame = gait
            .advance(duration(0.0), position, yaw, speed(0.0))
            .unwrap();
        let pose = CatPose::try_from_gait(position, yaw, &frame, sit).unwrap();
        (pose, frame)
    }

    fn skeleton_points(skeleton: &Skeleton) -> Vec<Vector3> {
        let mut points = vec![
            skeleton.chest,
            skeleton.hip,
            skeleton.neck,
            skeleton.head,
            skeleton.muzzle,
            skeleton.tail_root,
        ];
        for (base, tip) in skeleton.ears {
            points.extend([base, tip]);
        }
        for (root, tip) in skeleton.whiskers {
            points.extend([root, tip]);
        }
        for leg in skeleton.legs {
            points.extend([leg.root, leg.mid, leg.paw]);
        }
        points
    }

    fn assert_uniform_y_translation(flat: Vector3, raised: Vector3, delta_y: f32) {
        assert_eq!(flat.x.to_bits(), raised.x.to_bits());
        assert_eq!(flat.z.to_bits(), raised.z.to_bits());
        let observed_delta = raised.y - flat.y;
        assert!(
            ordered_f32_bits(observed_delta).abs_diff(ordered_f32_bits(delta_y)) <= 1,
            "expected {delta_y} m Y translation, got {} -> {}",
            flat.y,
            raised.y
        );
    }

    fn ordered_f32_bits(value: f32) -> u32 {
        let bits = value.to_bits();
        if bits & 0x8000_0000 == 0 {
            bits | 0x8000_0000
        } else {
            !bits
        }
    }

    fn assert_translation_with_one_ulp_xz(flat: Vector3, raised: Vector3, delta_y: f32) {
        assert!(ordered_f32_bits(flat.x).abs_diff(ordered_f32_bits(raised.x)) <= 1);
        assert!(ordered_f32_bits(flat.z).abs_diff(ordered_f32_bits(raised.z)) <= 1);
        let observed_delta = raised.y - flat.y;
        assert!(
            ordered_f32_bits(observed_delta).abs_diff(ordered_f32_bits(delta_y)) <= 1,
            "expected {delta_y} m Y translation, got {} -> {}",
            flat.y,
            raised.y
        );
    }

    #[test]
    fn elevated_skeleton_is_the_flat_skeleton_translated_once() {
        let (flat_pose, _) = pose_and_frame_at(0.0, 0.0);
        let (raised_pose, _) = pose_and_frame_at(0.75, 0.0);
        let flat = skeleton(&flat_pose).unwrap();
        let raised = skeleton(&raised_pose).unwrap();
        let flat_points = skeleton_points(&flat);
        let raised_points = skeleton_points(&raised);
        assert_eq!(flat_points.len(), raised_points.len());
        for (flat, raised) in flat_points.into_iter().zip(raised_points) {
            assert_uniform_y_translation(flat, raised, 0.75);
        }
    }

    #[test]
    fn elevated_sit_keeps_every_joint_above_its_support() {
        for sit in [0.5, 0.75, 1.0] {
            let (pose, _) = pose_and_frame_at(0.75, sit);
            let derived = skeleton(&pose).unwrap();
            for point in skeleton_points(&derived) {
                assert!(
                    point.y >= 0.749,
                    "sit {sit} derived a joint below its support at {}",
                    point.y
                );
            }
        }
    }

    #[test]
    fn extreme_actor_roots_keep_every_skeleton_joint_inside_pose_envelope() {
        for (position, yaw) in [
            (Vector3::new(1_000_000.0, 1_000_000.0, 0.0), 0.0),
            (Vector3::new(-1_000_000.0, -1_000_000.0, 0.0), 0.0),
            (
                Vector3::new(1_000_000.0, 1_000_000.0, -1_000_000.0),
                std::f64::consts::FRAC_PI_4,
            ),
            (
                Vector3::new(-1_000_000.0, -1_000_000.0, 1_000_000.0),
                -std::f64::consts::FRAC_PI_4,
            ),
        ] {
            let position = actor_position(position);
            let yaw = actor_yaw(yaw);
            let mut gait = CatGait::new(position, yaw).unwrap();
            let frame = gait
                .advance(duration(0.0), position, yaw, speed(0.0))
                .unwrap();
            let pose = CatPose::try_from_gait(position, yaw, &frame, 1.0).unwrap();
            let derived = skeleton(&pose).unwrap();
            for point in skeleton_points(&derived) {
                for lane in [point.x, point.y, point.z] {
                    assert!(lane.is_finite());
                    assert!(lane.abs() <= 1_000_002.0);
                }
            }
        }
    }

    #[test]
    fn tail_transport_preserves_the_curve_before_following() {
        let root = pose_point(Vector3::new(0.0, 0.24, 0.0));
        let mut tail = Tail::new(root, actor_yaw(0.0), support(0.0)).unwrap();
        let before = *tail.nodes();
        tail.transport_y(0.75).unwrap();
        for (flat, raised) in before.into_iter().zip(*tail.nodes()) {
            assert_uniform_y_translation(flat, raised, 0.75);
        }
    }

    #[test]
    fn tail_follow_is_translation_equivariant() {
        let yaw = actor_yaw(0.45);
        let flat_root = pose_point(Vector3::new(0.25, 0.24, -0.5));
        let raised_root = pose_point(Vector3::new(0.25, 0.99, -0.5));
        let mut flat = Tail::new(flat_root, yaw, support(0.0)).unwrap();
        let mut raised = flat;
        raised.transport_y(0.75).unwrap();

        flat.advance(duration(DT), flat_root, yaw, support(0.0), 0.35, -0.18)
            .unwrap();
        raised
            .advance(duration(DT), raised_root, yaw, support(0.75), 0.35, -0.18)
            .unwrap();
        for (flat, raised) in (*flat.nodes()).into_iter().zip(*raised.nodes()) {
            assert_translation_with_one_ulp_xz(flat, raised, 0.75);
        }
    }

    #[test]
    fn zero_support_tail_new_and_follow_preserve_legacy_lane_bits() {
        let root = pose_point(Vector3::new(0.25, 0.24, -0.5));
        let yaw = actor_yaw(0.45);
        let mut tail = Tail::new(root, yaw, support(0.0)).unwrap();
        assert_eq!(
            tail.nodes()
                .map(|node| [node.x.to_bits(), node.y.to_bits(), node.z.to_bits()]),
            [
                [1_049_474_446, 1_048_400_644, 3_202_588_332],
                [1_050_345_326, 1_049_053_321, 3_200_785_472],
                [1_051_161_492, 1_049_951_715, 3_199_095_880],
                [1_051_895_133, 1_051_169_547, 3_197_577_126],
                [1_052_525_220, 1_052_662_712, 3_196_272_746],
            ]
        );

        tail.advance(duration(DT), root, yaw, support(0.0), 0.35, -0.18)
            .unwrap();
        assert_eq!(
            tail.nodes()
                .map(|node| [node.x.to_bits(), node.y.to_bits(), node.z.to_bits()]),
            [
                [1_049_457_795, 1_048_353_209, 3_202_577_386],
                [1_050_353_713, 1_049_012_011, 3_200_781_380],
                [1_051_196_353, 1_049_902_385, 3_199_100_561],
                [1_051_944_385, 1_051_121_136, 3_197_589_584],
                [1_052_572_744, 1_052_620_197, 3_196_291_147],
            ]
        );
    }

    #[test]
    fn tail_transport_rejects_nonfinite_delta_without_poisoning_nodes() {
        let root = pose_point(Vector3::new(0.0, 0.24, 0.0));
        let tail = Tail::new(root, actor_yaw(0.0), support(0.0)).unwrap();
        for delta_y in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut attempt = tail;
            let before = *attempt.nodes();
            let error = attempt.transport_y(delta_y).unwrap_err();
            assert_eq!(error.field(), "tail.support_delta_y");
            assert_eq!(error.problem(), MotionValueProblem::NonFinite);
            assert_eq!(*attempt.nodes(), before);
        }
    }

    #[test]
    fn tail_transport_rejects_finite_boundary_overflow_without_mutation() {
        for (edge, delta_y) in [
            (1_000_002.0_f32, 0.0625_f32),
            (-1_000_002.0_f32, -0.0625_f32),
        ] {
            let mut capture = [Vector3::ZERO; TAIL_N];
            capture[2].y = edge;
            let mut tail = Tail::from_prepared(
                Tail::prepare_restore(capture).expect("the exact pose edge must restore"),
            );
            let before = *tail.nodes();
            let error = tail.transport_y(delta_y).unwrap_err();
            assert_eq!(error.field(), "pose_point.y");
            assert_eq!(error.problem(), MotionValueProblem::OutOfRange);
            assert_eq!(*tail.nodes(), before);
        }
    }

    #[test]
    fn cat_pose_rejects_poisoned_gait_frame_before_deriving_joints() {
        let position = actor_position(Vector3::ZERO);
        let yaw = actor_yaw(0.0);
        let mut gait = CatGait::new(position, yaw).unwrap();
        let frame = gait
            .advance(duration(0.0), position, yaw, speed(0.0))
            .unwrap();

        let mut bad_paw = frame.clone();
        bad_paw.paws[1].x = f32::INFINITY;
        let error = CatPose::try_from_gait(position, yaw, &bad_paw, 0.0).unwrap_err();
        assert_eq!(error.field(), "pose_point.x");
        assert_eq!(error.problem(), MotionValueProblem::NonFinite);

        let contact = cat_gait::Contact {
            leg: 0,
            at: Vector3::ZERO,
        };
        let mut exact_contact_budget = frame.clone();
        exact_contact_budget.contacts = vec![contact; 4];
        CatPose::try_from_gait(position, yaw, &exact_contact_budget, 0.0)
            .expect("one possible contact per leg is the bounded public frame domain");

        let mut unbounded_contacts = frame.clone();
        unbounded_contacts.contacts = vec![
            cat_gait::Contact {
                leg: 0,
                at: Vector3::ZERO,
            };
            5
        ];
        let error = CatPose::try_from_gait(position, yaw, &unbounded_contacts, 0.0).unwrap_err();
        assert_eq!(error.field(), "cat_pose.contacts");
        assert_eq!(error.problem(), MotionValueProblem::OutOfRange);

        let mut bad_contact_leg = frame.clone();
        bad_contact_leg.contacts.push(cat_gait::Contact {
            leg: LEGS,
            at: Vector3::ZERO,
        });
        let error = CatPose::try_from_gait(position, yaw, &bad_contact_leg, 0.0).unwrap_err();
        assert_eq!(error.field(), "cat_pose.contact.leg");
        assert_eq!(error.problem(), MotionValueProblem::OutOfRange);

        let mut bad_contact_point = frame.clone();
        bad_contact_point.contacts.push(cat_gait::Contact {
            leg: 0,
            at: Vector3::new(0.0, f32::NAN, 0.0),
        });
        let error = CatPose::try_from_gait(position, yaw, &bad_contact_point, 0.0).unwrap_err();
        assert_eq!(error.field(), "pose_point.y");
        assert_eq!(error.problem(), MotionValueProblem::NonFinite);

        for (phase, amp, bob, support_delta_y, sit, field, problem) in [
            (
                f64::NAN,
                frame.amp,
                frame.bob,
                0.0,
                0.0,
                "cat_pose.phase",
                MotionValueProblem::NonFinite,
            ),
            (
                1.0,
                frame.amp,
                frame.bob,
                0.0,
                0.0,
                "cat_pose.phase",
                MotionValueProblem::OutOfRange,
            ),
            (
                frame.phase,
                f64::NAN,
                frame.bob,
                0.0,
                0.0,
                "cat_pose.amp",
                MotionValueProblem::NonFinite,
            ),
            (
                frame.phase,
                1.01,
                frame.bob,
                0.0,
                0.0,
                "cat_pose.amp",
                MotionValueProblem::OutOfRange,
            ),
            (
                frame.phase,
                frame.amp,
                f64::NAN,
                0.0,
                0.0,
                "cat_pose.bob",
                MotionValueProblem::NonFinite,
            ),
            (
                frame.phase,
                frame.amp,
                0.006_001,
                0.0,
                0.0,
                "cat_pose.bob",
                MotionValueProblem::OutOfRange,
            ),
            (
                frame.phase,
                frame.amp,
                frame.bob,
                f32::NAN,
                0.0,
                "cat_pose.support_delta_y",
                MotionValueProblem::NonFinite,
            ),
            (
                frame.phase,
                frame.amp,
                frame.bob,
                0.0,
                f64::NAN,
                "cat_pose.sit",
                MotionValueProblem::NonFinite,
            ),
            (
                frame.phase,
                frame.amp,
                frame.bob,
                0.0,
                -0.01,
                "cat_pose.sit",
                MotionValueProblem::OutOfRange,
            ),
        ] {
            let mut poisoned = frame.clone();
            poisoned.phase = phase;
            poisoned.amp = amp;
            poisoned.bob = bob;
            poisoned.support_delta_y = support_delta_y;
            let error = CatPose::try_from_gait(position, yaw, &poisoned, sit).unwrap_err();
            assert_eq!(error.field(), field);
            assert_eq!(error.problem(), problem);
        }
    }

    #[test]
    fn skeleton_rejects_public_pose_poison_and_unrepresentable_yaw() {
        let (base, _) = pose_and_frame_at(0.0, 0.0);
        for poisoned in [
            CatPose {
                pos: Vector3::new(f32::NAN, 0.0, 0.0),
                ..base
            },
            CatPose {
                yaw: f64::MAX,
                ..base
            },
            CatPose { amp: 1.01, ..base },
            CatPose { sit: -0.01, ..base },
            CatPose {
                bob: 0.006_001,
                ..base
            },
        ] {
            assert!(skeleton(&poisoned).is_err());
        }

        let edge = Vector3::new(1_000_002.0, 0.0, 0.0);
        let derived_overflow = CatPose {
            pos: edge,
            yaw: -std::f64::consts::FRAC_PI_2,
            paws: [edge; LEGS],
            bob: 0.0,
            amp: 0.0,
            sit: 0.0,
        };
        let error = skeleton(&derived_overflow)
            .expect_err("a valid raw pose whose derived chest crosses the pose edge must refuse");
        assert_eq!(error.field(), "pose_point.x");
        assert_eq!(error.problem(), MotionValueProblem::OutOfRange);
    }

    #[test]
    fn poisoned_tail_capture_refuses_checked_restore() {
        let mut nodes = [Vector3::ZERO; TAIL_N];
        nodes[3].x = f32::INFINITY;
        let error = Tail::prepare_restore(nodes).expect_err("tail poison must be refused");
        assert_eq!(error.path, "tail[3].x");

        let mut nodes = [Vector3::ZERO; TAIL_N];
        nodes[1].z = crate::support_motion::MAX_POSE_COORD_M + 1.0;
        let error = Tail::prepare_restore(nodes).expect_err("tail overflow must be refused");
        assert_eq!(error.path, "tail[1].z");
    }

    #[test]
    fn brain_and_tail_typed_steps_reject_invalid_inputs_without_mutating_prior_state() {
        let position = actor_position(Vector3::ZERO);
        let yaw = actor_yaw(0.0);
        let rect = RoamRect::try_around(position, Vector2::new(6.0, 6.0)).unwrap();
        let brain = CatBrain::new(7, rect, yaw);
        let brain_before = brain.capture();
        assert!(ActorPosition::try_new(Vector3::new(f32::NAN, 0.0, 0.0)).is_err());
        assert!(ActorYaw::try_new(f64::INFINITY).is_err());
        assert!(FiniteMeasure::try_new(f64::NAN, "cat.progress").is_err());
        assert_eq!(brain.capture(), brain_before);

        let root = PosePoint::try_new(Vector3::new(0.0, 0.24, 0.0)).unwrap();
        let mut tail = Tail::new(root, yaw, support(0.0)).unwrap();
        let tail_before = tail;
        for (sit, sway, field, problem) in [
            (f64::NAN, 0.0, "tail.sit", MotionValueProblem::NonFinite),
            (-0.01, 0.0, "tail.sit", MotionValueProblem::OutOfRange),
            (1.01, 0.0, "tail.sit", MotionValueProblem::OutOfRange),
            (
                0.5,
                f64::INFINITY,
                "tail.sway",
                MotionValueProblem::NonFinite,
            ),
        ] {
            let error = tail
                .advance(
                    StepDuration::from_raw(DT),
                    root,
                    yaw,
                    support(0.0),
                    sit,
                    sway,
                )
                .expect_err("invalid tail controls must be refused");
            assert_eq!(error.field(), field);
            assert_eq!(error.problem(), problem);
            assert_eq!(tail, tail_before);
        }

        let edge = Vector3::new(1_000_002.0, 0.0, 0.0);
        let mut edge_tail = Tail::from_prepared(
            Tail::prepare_restore([edge; TAIL_N]).expect("the exact pose edge must restore"),
        );
        let edge_before = edge_tail;
        let error = edge_tail
            .advance(
                StepDuration::from_raw(DT),
                pose_point(edge),
                actor_yaw(std::f64::consts::FRAC_PI_2),
                support(0.0),
                0.0,
                0.0,
            )
            .expect_err("the follow step beyond the pose edge must refuse");
        assert_eq!(error.field(), "pose_point.x");
        assert_eq!(error.problem(), MotionValueProblem::OutOfRange);
        assert_eq!(edge_tail, edge_before);
    }

    #[test]
    fn prepared_restore_rejects_invalid_pose_or_tail_point() {
        let pose = CatPose {
            pos: Vector3::ZERO,
            yaw: 0.0,
            paws: [Vector3::ZERO; LEGS],
            bob: 0.0,
            amp: 0.0,
            sit: f64::NAN,
        };
        let error = CatPose::prepare_restore(pose).expect_err("pose poison must be refused");
        assert_eq!(error.path, "pose.sit");

        let pose = CatPose {
            pos: Vector3::ZERO,
            yaw: f64::MAX,
            paws: [Vector3::ZERO; LEGS],
            bob: 0.0,
            amp: 0.0,
            sit: 0.0,
        };
        let error = CatPose::prepare_restore(pose)
            .expect_err("a yaw that cannot narrow to Godot must be refused");
        assert_eq!(error.path, "pose.yaw");

        let mut nodes = [Vector3::ZERO; TAIL_N];
        nodes[3].x = f32::INFINITY;
        let error = Tail::prepare_restore(nodes).expect_err("tail poison must be refused");
        assert_eq!(error.path, "tail[3].x");
    }
    use crate::cat_gait::CatGait;

    const DT: f64 = 1.0 / 60.0;

    /// A settled walking pose sweep straight off the real gait.
    fn walk_poses(sit: f64) -> Vec<CatPose> {
        let yaw = actor_yaw(0.0);
        let mut gait = CatGait::new(actor_position(Vector3::ZERO), yaw).unwrap();
        let mut pos = Vector3::ZERO;
        let mut out = Vec::new();
        for _ in 0..600 {
            pos += Vector3::new(0.0, 0.0, -(0.6 * DT) as f32);
            let position = actor_position(pos);
            let frame = gait
                .advance(duration(DT), position, yaw, speed(0.6))
                .unwrap();
            out.push(CatPose::try_from_gait(position, yaw, &frame, sit).unwrap());
        }
        out
    }

    /// Leg segments keep their lengths through a full walking sweep, at
    /// every sit blend — the law-of-cosines never stretches a bone.
    #[test]
    fn legs_preserve_segment_lengths() {
        for sit in [0.0, 0.35, 1.0] {
            for pose in walk_poses(sit) {
                let sk = skeleton(&pose).unwrap();
                for (leg, l) in sk.legs.iter().enumerate() {
                    let (upper, lower) = if leg < 2 {
                        (FORE_UPPER, FORE_LOWER)
                    } else {
                        (HIND_UPPER, HIND_LOWER)
                    };
                    let a = f64::from((l.mid - l.root).length());
                    let b = f64::from((l.paw - l.mid).length());
                    assert!((a - upper).abs() < 3e-4, "upper {a} vs {upper}");
                    assert!((b - lower).abs() < 3e-4, "lower {b} vs {lower}");
                }
            }
        }
    }

    /// The IK is total: an unreachable paw straightens the leg toward it,
    /// a paw at the root points it down, and nothing is ever NaN.
    #[test]
    fn two_bone_is_total() {
        let root = Vector3::new(0.0, 0.2, 0.0);
        for target in [
            Vector3::new(5.0, -3.0, 2.0),        // far out of reach
            root,                                // degenerate: at the root
            Vector3::new(0.0, 0.2 - 0.001, 0.0), // just under the root
            Vector3::new(0.0, -5.0, 0.0),        // straight down, far
            Vector3::new(0.001, 0.199, -0.001),  // a hair away
        ] {
            let (mid, end) = two_bone(root, target, 0.115, 0.105, Vector3::new(0.0, 0.0, 1.0));
            for v in [mid, end] {
                assert!(v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
            }
            let a = f64::from((mid - root).length());
            let b = f64::from((end - mid).length());
            assert!((a - 0.115).abs() < 3e-4);
            assert!((b - 0.105).abs() < 3e-4);
        }
        // the hint lying along the leg still bends somewhere definite
        let (mid, _) = two_bone(
            root,
            Vector3::new(0.0, 0.0, 0.0),
            0.115,
            0.105,
            Vector3::new(0.0, -1.0, 0.0),
        );
        assert!(mid.x.is_finite() && mid.y.is_finite() && mid.z.is_finite());
    }

    /// Elbows and hocks bend BEHIND the cat — the feline silhouette.
    #[test]
    fn legs_bend_backward() {
        let poses = walk_poses(0.0);
        let pose = &poses[300];
        let sk = skeleton(pose).unwrap();
        let fw = Vector3::new(0.0, 0.0, -1.0); // yaw 0 faces -Z
        for l in sk.legs {
            let straight = (l.paw - l.root) * 0.5 + l.root;
            let bend = l.mid - straight;
            assert!(f64::from(bend.dot(fw)) <= 1e-6, "a knee points forward");
        }
    }

    /// Standing tall vs fully seated: the haunches drop to the floor, the
    /// chest and head rise proud, and the hind paws tuck in beside them.
    #[test]
    fn sitting_drops_the_haunches_and_lifts_the_head() {
        let position = actor_position(Vector3::ZERO);
        let yaw = actor_yaw(0.0);
        let mut gait = CatGait::new(position, yaw).unwrap();
        let frame = gait
            .advance(duration(DT), position, yaw, speed(0.0))
            .unwrap();
        let stand_pose = CatPose::try_from_gait(position, yaw, &frame, 0.0).unwrap();
        let seated_pose = CatPose::try_from_gait(position, yaw, &frame, 1.0).unwrap();
        let stand = skeleton(&stand_pose).unwrap();
        let seated = skeleton(&seated_pose).unwrap();
        assert!(f64::from(seated.hip.y) < 0.09);
        assert!(f64::from(stand.hip.y) > 0.19);
        assert!(seated.head.y > stand.head.y);
        assert!(seated.head.y > seated.chest.y);
        for leg in 2..LEGS {
            let paw = seated.legs[leg].paw;
            let hip_dist = Vector3::new(paw.x - seated.hip.x, 0.0, paw.z - seated.hip.z).length();
            assert!(f64::from(hip_dist) < 0.12, "hind paw not tucked");
        }
    }

    /// A seated cat never drives a leg through the floor. The walking
    /// `paws_ride_the_floor` law checks only PAWS mid-stride; a deep sit
    /// drops the hip low and the long hind pair, folded double, bulged its
    /// HOCK below y = 0 on the old backward bend hint — visible as the hind
    /// legs sinking under the floor. Every joint of every leg must stay on
    /// or above the ground through the whole sit.
    #[test]
    fn seated_legs_never_pierce_the_floor() {
        let position = actor_position(Vector3::ZERO);
        let yaw = actor_yaw(0.0);
        let mut gait = CatGait::new(position, yaw).unwrap();
        let frame = gait
            .advance(duration(DT), position, yaw, speed(0.0))
            .unwrap();
        for sit in [0.5, 0.75, 1.0] {
            let pose = CatPose::try_from_gait(position, yaw, &frame, sit).unwrap();
            let sk = skeleton(&pose).unwrap();
            for (leg, l) in sk.legs.iter().enumerate() {
                for (joint, p) in [("root", l.root), ("mid", l.mid), ("paw", l.paw)] {
                    assert!(
                        f64::from(p.y) >= -0.001,
                        "sit {sit}: leg {leg} {joint} pierces the floor at y = {}",
                        p.y
                    );
                }
            }
        }
    }

    /// The face stays assembled: ears on the head, muzzle ahead of it,
    /// four whiskers of exactly whisker length fanning from the muzzle.
    #[test]
    fn ears_muzzle_whiskers_stay_on_the_face() {
        for pose in walk_poses(0.0).iter().step_by(60) {
            let sk = skeleton(pose).unwrap();
            for (base, tip) in sk.ears {
                assert!(f64::from((base - sk.head).length()) < 0.06);
                assert!(f64::from((tip - base).length()) < 0.06);
                assert!(tip.y > base.y, "ears point up");
            }
            let fw = Vector3::new(0.0, 0.0, -1.0);
            assert!(f64::from((sk.muzzle - sk.head).dot(fw)) > 0.0);
            for (root, tip) in sk.whiskers {
                assert!(f64::from((root - sk.muzzle).length()) < 0.04);
                let len = f64::from((tip - root).length());
                assert!((len - WHISKER_LEN).abs() < 1e-4);
            }
        }
    }

    /// Standing skeleton paws are the gait's paws, untouched — the body
    /// never repaints where the gait planted. Exact in the settled walk;
    /// the cold-standstill launch transient (one leg waiting out most of
    /// a cycle) may briefly straighten against the IK clamp, bounded.
    #[test]
    fn standing_paws_pass_through() {
        let poses = walk_poses(0.0);
        for (i, pose) in poses.iter().enumerate() {
            let sk = skeleton(pose).unwrap();
            for (leg, l) in sk.legs.iter().enumerate() {
                let d = f64::from((l.paw - pose.paws[leg]).length());
                if i >= 300 {
                    assert!(d < 0.005, "settled leg {leg} missed its paw by {d}");
                } else {
                    assert!(d < 0.05, "launch leg {leg} missed its paw by {d}");
                }
            }
        }
    }

    /// The tail chain: segment lengths exact, no NaN, the tip lagging the
    /// base after a sharp turn, and a sit curling the tip sideways.
    #[test]
    fn tail_follows_lags_and_curls() {
        let root = Vector3::new(0.0, 0.24, 0.0);
        let root_point = pose_point(root);
        let yaw = actor_yaw(0.0);
        let rv = Vector3::new(1.0, 0.0, 0.0);
        let mut tail = Tail::new(root_point, yaw, support(0.0)).unwrap();
        for node in tail.nodes() {
            assert!(node.x.is_finite() && node.y.is_finite() && node.z.is_finite());
        }
        let mut prev = root;
        for node in tail.nodes() {
            let seg = f64::from((*node - prev).length());
            assert!((seg - TAIL_SEG).abs() < 1e-5);
            prev = *node;
        }
        // swing the body 90 degrees in one tick: the base joint obeys
        // quickly, the tip barely moves yet — lag written in the air
        let before = *tail.nodes();
        tail.advance(
            duration(1.0 / 60.0),
            root_point,
            actor_yaw(std::f64::consts::FRAC_PI_2),
            support(0.0),
            0.0,
            0.0,
        )
        .unwrap();
        let after = *tail.nodes();
        let base_move = f64::from((after[0] - before[0]).length());
        let tip_move = f64::from((after[TAIL_N - 1] - before[TAIL_N - 1]).length());
        assert!(base_move > tip_move, "the tip failed to lag the base");
        // settle into a sit: the tip ends up swung to the side
        let mut sitting = Tail::new(root_point, yaw, support(0.0)).unwrap();
        for _ in 0..600 {
            sitting
                .advance(
                    duration(1.0 / 60.0),
                    root_point,
                    yaw,
                    support(0.0),
                    1.0,
                    0.0,
                )
                .unwrap();
        }
        let tip = sitting.nodes()[TAIL_N - 1];
        assert!(
            f64::from((tip - root).dot(rv)).abs() > 0.1,
            "a seated tail must curl sideways"
        );
    }

    /// A mid-sway tail restores verbatim — Tail::new SETTLES (120
    /// iterations toward rest), so a restore door must bypass it.
    #[test]
    fn a_restored_tail_holds_its_exact_curve() {
        let yaw = actor_yaw(0.0);
        let mut tail = Tail::new(pose_point(Vector3::ZERO), yaw, support(0.0)).unwrap();
        for i in 0..30 {
            let root = Vector3::new(f32::from(i as u8) * 0.01, 0.0, 0.0);
            tail.advance(
                duration(0.05),
                pose_point(root),
                yaw,
                support(0.0),
                0.2,
                0.1,
            )
            .unwrap();
        }
        let restored = Tail::from_prepared(
            Tail::prepare_restore(*tail.nodes()).expect("self-capture must restore"),
        );
        assert_eq!(restored.nodes(), tail.nodes());
    }

    /// The follow-chain never coils into a loop: at every joint, in a
    /// standing rest, a hard walk, and a full sit, the segment bends no
    /// more than TAIL_MAX_BEND from the one before it — the structural
    /// guard that killed the teacup-handle and squirrel-arc glitches.
    #[test]
    fn tail_never_coils_into_a_loop() {
        let root = Vector3::new(0.0, 0.24, 0.0);
        let root_point = pose_point(root);
        let yaw = actor_yaw(0.0);
        let back = Vector3::new(0.0, 0.0, 1.0);
        let max_cos = TAIL_MAX_BEND.cos();
        for sit in [0.0, 0.5, 1.0] {
            for sway in [-0.25, 0.0, 0.25] {
                let mut tail = Tail::new(root_point, yaw, support(0.0)).unwrap();
                for _ in 0..600 {
                    tail.advance(duration(DT), root_point, yaw, support(0.0), sit, sway)
                        .unwrap();
                }
                let mut prev_dir = back;
                let mut prev = root;
                for node in tail.nodes() {
                    let dir = (*node - prev).normalized();
                    let cos = f64::from(dir.dot(prev_dir));
                    assert!(
                        cos >= max_cos - 1e-4,
                        "tail coiled: bend cos {cos} at sit {sit} sway {sway}"
                    );
                    prev_dir = dir;
                    prev = *node;
                }
            }
        }
    }

    /// The whole skeleton is finite over a hostile sweep — sit blends,
    /// walk frames, spun yaws, a bobbing body: total, always.
    #[test]
    fn skeleton_is_total() {
        let mut pos = Vector3::new(2.0, 0.0, -1.0);
        let mut yaw = 0.3;
        let mut gait = CatGait::new(actor_position(pos), actor_yaw(yaw)).unwrap();
        for i in 0..900 {
            yaw += 1.4 * DT;
            let speed = if i % 200 < 130 { 0.6 } else { 0.0 };
            pos +=
                Vector3::new((-yaw.sin()) as f32, 0.0, (-yaw.cos()) as f32) * ((speed * DT) as f32);
            let frame = gait
                .advance(
                    duration(DT),
                    actor_position(pos),
                    actor_yaw(yaw),
                    self::speed(speed),
                )
                .unwrap();
            let sit = (f64::from(i % 300) / 300.0).min(1.0);
            let position = actor_position(pos);
            let yaw = actor_yaw(yaw);
            let pose = CatPose::try_from_gait(position, yaw, &frame, sit).unwrap();
            let sk = skeleton(&pose).unwrap();
            for v in [sk.chest, sk.hip, sk.neck, sk.head, sk.muzzle, sk.tail_root] {
                assert!(v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
            }
            for l in sk.legs {
                for v in [l.root, l.mid, l.paw] {
                    assert!(v.x.is_finite() && v.y.is_finite() && v.z.is_finite());
                }
            }
        }
    }
}
