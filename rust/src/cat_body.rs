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
use crate::support_motion::MAX_POSE_COORD_M;

/// Chest-line height above the floor while standing, meters.
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
    /// Body center on the floor.
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
            ("yaw", capture.yaw),
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

    /// A pose straight off a gait frame — the node adds its eased sit.
    #[must_use]
    pub fn from_gait(pos: Vector3, yaw: f64, frame: &GaitFrame, sit: f64) -> Self {
        Self {
            pos,
            yaw,
            paws: frame.paws,
            bob: frame.bob,
            amp: frame.amp,
            sit,
        }
    }
}

/// The whole skeleton for one frame — total over any pose: every joint
/// finite, every segment length preserved.
#[must_use]
pub fn skeleton(pose: &CatPose) -> Skeleton {
    let fw = forward(pose.yaw);
    let rv = rightward(pose.yaw);
    let ground = Vector3::new(pose.pos.x, 0.0, pose.pos.z);
    let sit = pose.sit.clamp(0.0, 1.0);

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
            Vector3::new(chest.x, 0.0, chest.z) + rv * (0.048 * side) + fw * 0.02
        } else {
            Vector3::new(hip.x, 0.0, hip.z) + rv * (0.07 * side) + fw * 0.06
        };
        let paw = pose.paws[leg].lerp(sit_paw, sit as f32);
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

    Skeleton {
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
    }
}

/// Two-bone analytic IK, total over any target: the reach is clamped
/// into the triangle inequality (an unreachable paw straightens the leg
/// toward it, a target at the root points it straight down), and the
/// bend leaves toward `hint` — behind the cat, where elbows and hocks
/// live.
#[must_use]
pub fn two_bone(
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
            validate_restore_point(&format!("tail[{index}]"), point)?;
        }
        Ok(PreparedTail(Self::restore(capture)))
    }

    #[must_use]
    pub fn from_prepared(capture: PreparedTail) -> Self {
        capture.0
    }

    /// A fresh tail already in its standing rest curve.
    #[must_use]
    pub fn new(root: Vector3, back: Vector3, rv: Vector3) -> Self {
        let mut tail = Self {
            nodes: [root; TAIL_N],
        };
        // settle instantly into the rest pose: a long advance from rest
        for _ in 0..120 {
            tail.advance(0.1, root, back, rv, 0.0, 0.0);
        }
        tail
    }

    /// The joints, root-side first.
    #[must_use]
    pub fn nodes(&self) -> &[Vector3; TAIL_N] {
        &self.nodes
    }

    /// A chain rebuilt at an exact curve. `new` settles toward rest by
    /// iterating — correct for a spawn, wrong for a restore.
    #[must_use]
    pub fn restore(nodes: [Vector3; TAIL_N]) -> Self {
        Self { nodes }
    }

    /// One tick: `back` is the direction away from the body, `rv` the
    /// cat's right, `sit` the sit blend (the tail curls around the
    /// haunches), `sway` a signed lateral weight the node drives with
    /// the stride and an idle breath.
    pub fn advance(
        &mut self,
        dt: f64,
        root: Vector3,
        back: Vector3,
        rv: Vector3,
        sit: f64,
        sway: f64,
    ) {
        let sit = sit.clamp(0.0, 1.0);
        let mut prev = root;
        let mut prev_dir = back.normalized();
        for (i, node) in self.nodes.iter_mut().enumerate() {
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

        let mut nodes = [Vector3::ZERO; TAIL_N];
        nodes[3].x = f32::INFINITY;
        let error = Tail::prepare_restore(nodes).expect_err("tail poison must be refused");
        assert_eq!(error.path, "tail[3].x");
    }
    use crate::cat_gait::CatGait;

    const DT: f64 = 1.0 / 60.0;

    /// A settled walking pose sweep straight off the real gait.
    fn walk_poses(sit: f64) -> Vec<CatPose> {
        let mut gait = CatGait::new(Vector3::ZERO, 0.0);
        let mut pos = Vector3::ZERO;
        let mut out = Vec::new();
        for _ in 0..600 {
            pos += Vector3::new(0.0, 0.0, -(0.6 * DT) as f32);
            let frame = gait.advance(DT, pos, 0.0, 0.6);
            out.push(CatPose::from_gait(pos, 0.0, &frame, sit));
        }
        out
    }

    /// Leg segments keep their lengths through a full walking sweep, at
    /// every sit blend — the law-of-cosines never stretches a bone.
    #[test]
    fn legs_preserve_segment_lengths() {
        for sit in [0.0, 0.35, 1.0] {
            for pose in walk_poses(sit) {
                let sk = skeleton(&pose);
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
        let sk = skeleton(pose);
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
        let mut gait = CatGait::new(Vector3::ZERO, 0.0);
        let frame = gait.advance(DT, Vector3::ZERO, 0.0, 0.0);
        let stand = skeleton(&CatPose::from_gait(Vector3::ZERO, 0.0, &frame, 0.0));
        let seated = skeleton(&CatPose::from_gait(Vector3::ZERO, 0.0, &frame, 1.0));
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
        let mut gait = CatGait::new(Vector3::ZERO, 0.0);
        let frame = gait.advance(DT, Vector3::ZERO, 0.0, 0.0);
        for sit in [0.5, 0.75, 1.0] {
            let sk = skeleton(&CatPose::from_gait(Vector3::ZERO, 0.0, &frame, sit));
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
            let sk = skeleton(pose);
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
            let sk = skeleton(pose);
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
        let back = Vector3::new(0.0, 0.0, 1.0);
        let rv = Vector3::new(1.0, 0.0, 0.0);
        let mut tail = Tail::new(root, back, rv);
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
        let new_back = Vector3::new(1.0, 0.0, 0.0);
        let new_rv = Vector3::new(0.0, 0.0, -1.0);
        let before = *tail.nodes();
        tail.advance(1.0 / 60.0, root, new_back, new_rv, 0.0, 0.0);
        let after = *tail.nodes();
        let base_move = f64::from((after[0] - before[0]).length());
        let tip_move = f64::from((after[TAIL_N - 1] - before[TAIL_N - 1]).length());
        assert!(base_move > tip_move, "the tip failed to lag the base");
        // settle into a sit: the tip ends up swung to the side
        let mut sitting = Tail::new(root, back, rv);
        for _ in 0..600 {
            sitting.advance(1.0 / 60.0, root, back, rv, 1.0, 0.0);
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
        let rv = Vector3::new(1.0, 0.0, 0.0);
        let mut tail = Tail::new(Vector3::ZERO, Vector3::new(0.0, 0.1, -0.2), rv);
        for i in 0..30 {
            let root = Vector3::new(f32::from(i as u8) * 0.01, 0.0, 0.0);
            tail.advance(
                0.05,
                root,
                root + Vector3::new(0.0, 0.1, -0.2),
                rv,
                0.2,
                0.1,
            );
        }
        let restored = Tail::restore(*tail.nodes());
        assert_eq!(restored.nodes(), tail.nodes());
    }

    /// The follow-chain never coils into a loop: at every joint, in a
    /// standing rest, a hard walk, and a full sit, the segment bends no
    /// more than TAIL_MAX_BEND from the one before it — the structural
    /// guard that killed the teacup-handle and squirrel-arc glitches.
    #[test]
    fn tail_never_coils_into_a_loop() {
        let root = Vector3::new(0.0, 0.24, 0.0);
        let back = Vector3::new(0.0, 0.0, 1.0);
        let rv = Vector3::new(1.0, 0.0, 0.0);
        let max_cos = TAIL_MAX_BEND.cos();
        for sit in [0.0, 0.5, 1.0] {
            for sway in [-0.25, 0.0, 0.25] {
                let mut tail = Tail::new(root, back, rv);
                for _ in 0..600 {
                    tail.advance(DT, root, back, rv, sit, sway);
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
        let mut gait = CatGait::new(Vector3::ZERO, 0.3);
        let mut pos = Vector3::new(2.0, 0.0, -1.0);
        let mut yaw = 0.3;
        for i in 0..900 {
            yaw += 1.4 * DT;
            let speed = if i % 200 < 130 { 0.6 } else { 0.0 };
            pos +=
                Vector3::new((-yaw.sin()) as f32, 0.0, (-yaw.cos()) as f32) * ((speed * DT) as f32);
            let frame = gait.advance(DT, pos, yaw, speed);
            let sit = (f64::from(i % 300) / 300.0).min(1.0);
            let sk = skeleton(&CatPose::from_gait(pos, yaw, &frame, sit));
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
