//! Pulse-slot observation — the pool as an agent reads it.
//!
//! Every field is derived from the pool's PUBLIC lanes (`pos`, `dat`,
//! `dir`) rather than its private CPU shadow, deliberately: the lanes are
//! what the shaders consume, narrowed to f32, so an observation built from
//! them reports what the renderer actually sees. When the CPU and the GPU
//! disagree, this is the side of the disagreement worth showing.

use godot::builtin::Vector3;

use crate::pulse_pool::{MAXP, PulsePool, fade_tail};

/// Whether a slot ever held a sound, and whether it still does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    /// `dat.x == -1`: no pulse has ever lived here.
    Never,
    /// Held a sound; its ring time plus fade tail has run out.
    Expired,
    /// Still inside its lifetime — the shaders still draw it.
    Live,
}

/// One pool slot, decoded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotObservation {
    pub index: usize,
    pub state: SlotState,
    pub kind: i32,
    pub origin: Vector3,
    pub birth: f64,
    pub max_r: f64,
    pub speed: f64,
    pub gain: f64,
    pub beam: Vector3,
    pub cos_half: f64,
    /// Current wavefront radius, capped at `max_r`. Zero for a dead slot.
    pub ring_radius: f64,
    /// Seconds since birth. Zero for a slot that never lived.
    pub age: f64,
    /// Seconds until the slot expires; zero once it has.
    pub remaining: f64,
    pub end: f64,
}

/// Decode every slot in the pool as of `now`.
///
/// Total on any pool state, including the virgin sentinel: a slot with
/// `dat.x < 0` is reported [`SlotState::Never`] BEFORE any arithmetic, so
/// the `0.0 / 0.0` that reconstructing its end time would compute never
/// happens. A NaN here would report a slot as neither live nor expired.
#[must_use]
pub fn slots(pool: &PulsePool, now: f64) -> Vec<SlotObservation> {
    (0..MAXP).map(|i| slot(pool, i, now)).collect()
}

fn slot(pool: &PulsePool, index: usize, now: f64) -> SlotObservation {
    let dat = pool.dat()[index];
    let dir = pool.dir()[index];
    let origin = pool.pos()[index];
    let birth = f64::from(dat.x);
    let beam = Vector3::new(dir.x, dir.y, dir.z);
    let cos_half = f64::from(dir.w);
    // The sentinel FIRST: max_r and speed are both zero here, and the
    // pool's own `new()` leaves end at -1.0 with t0 at 0.0.
    if dat.x < 0.0 {
        return SlotObservation {
            index,
            state: SlotState::Never,
            kind: 0,
            origin,
            birth,
            max_r: 0.0,
            speed: 0.0,
            gain: 0.0,
            beam,
            cos_half,
            ring_radius: 0.0,
            age: 0.0,
            remaining: 0.0,
            end: -1.0,
        };
    }
    let max_r = f64::from(dat.y);
    let speed = f64::from(dat.z);
    // The shader's own decode of the packed lane.
    let kind = (f64::from(dat.w) / 10.0).floor() as i32;
    let gain = (f64::from(dat.w) % 10.0) / 9.0;
    let end = birth + max_r / speed + fade_tail(kind);
    let age = now - birth;
    let state = if end >= now {
        SlotState::Live
    } else {
        SlotState::Expired
    };
    SlotObservation {
        index,
        state,
        kind,
        origin,
        birth,
        max_r,
        speed,
        gain,
        beam,
        cos_half,
        ring_radius: (age * speed).clamp(0.0, max_r),
        age: age.max(0.0),
        remaining: (end - now).max(0.0),
        end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pulse_pool::PulsePool;
    use godot::builtin::Vector3;

    /// A virgin pool holds dat = (-1, 0, 0, 0). Reconstructing `end` from
    /// those lanes computes 0.0/0.0 — NaN — which would make every
    /// comparison against `now` false and report the slot as neither live
    /// nor expired. The sentinel is checked FIRST, and the slot reports
    /// Never with the pool's own -1.0 end time.
    #[test]
    fn virgin_slots_report_never_not_nan() {
        let pool = PulsePool::new();
        let obs = slots(&pool, 0.0);
        assert_eq!(obs.len(), 64);
        for s in &obs {
            assert_eq!(s.state, SlotState::Never);
            assert_eq!(s.end, -1.0);
            assert!(!s.remaining.is_nan(), "slot {} remaining is NaN", s.index);
            assert!(!s.ring_radius.is_nan(), "slot {} radius is NaN", s.index);
        }
    }

    /// Hand-derived from the wave contract, NOT from the code under test.
    /// A cane tap (kind 0) with max_r 6.0 and speed 5.5 born at t = 0:
    ///   ring radius at t = 0.5  =  0.5 * 5.5            =  2.75
    ///   end                     =  0 + 6.0/5.5 + 6.0    =  7.0909090909…
    ///   remaining at t = 0.5    =  7.0909090909… - 0.5  =  6.5909090909…
    #[test]
    fn a_live_tap_reports_hand_derived_geometry() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let s = &slots(&pool, 0.5)[0];
        assert_eq!(s.state, SlotState::Live);
        assert_eq!(s.kind, 0);
        assert!((s.ring_radius - 2.75).abs() < 1e-5, "got {}", s.ring_radius);
        assert!(
            (s.remaining - 6.590_909_090_9).abs() < 1e-5,
            "got {}",
            s.remaining
        );
        assert!((s.age - 0.5).abs() < 1e-5);
    }

    /// The ring stops growing at max_r; it does not run away with the clock.
    /// max_r 6.0 at speed 5.5 reaches full radius at t = 1.0909…, so at
    /// t = 3.0 the radius is still exactly 6.0 while the slot is alive on
    /// its 6-second fade tail.
    #[test]
    fn the_ring_is_capped_at_max_radius() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let s = &slots(&pool, 3.0)[0];
        assert_eq!(s.state, SlotState::Live);
        assert!((s.ring_radius - 6.0).abs() < 1e-5, "got {}", s.ring_radius);
    }

    /// Kind and gain come back through the shader's own decode, so an
    /// observation of a footstep at gain 0.8 reports 2 and 0.8 — not the
    /// packed 20.8 that lives in the lane.
    #[test]
    fn kind_and_gain_are_decoded_not_raw() {
        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::ONE, 1.6, 4.0, 0.8, 0.0).unwrap();
        let s = &slots(&pool, 0.1)[0];
        assert_eq!(s.kind, 2);
        assert!((s.gain - 0.8).abs() < 0.001, "got {}", s.gain);
        assert_eq!(s.origin, Vector3::ONE);
    }

    /// A slot past its end is Expired, distinct from Never — the pool
    /// reuses it, and an agent must be able to tell "died" from "never
    /// lived". A footstep (2.5 s tail) with ring time 1.6/4.0 = 0.4 s ends
    /// at t = 2.9.
    #[test]
    fn a_dead_slot_is_expired_not_never() {
        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::ONE, 1.6, 4.0, 0.8, 0.0).unwrap();
        assert_eq!(slots(&pool, 2.8)[0].state, SlotState::Live);
        assert_eq!(slots(&pool, 3.0)[0].state, SlotState::Expired);
    }

    /// A beamed source pulse keeps its cone; an omni pulse reports the
    /// -2 sentinel so an agent can see at a glance which gate applies.
    #[test]
    fn beam_and_omni_are_distinguishable() {
        let mut pool = PulsePool::new();
        let beam = Vector3::new(0.0, 0.0, -1.0);
        pool.emit(3, Vector3::ZERO, 9.0, 4.5, 0.75, 0.0, beam, 0.85)
            .unwrap();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let obs = slots(&pool, 0.1);
        assert_eq!(obs[0].beam, beam);
        assert!((obs[0].cos_half - 0.85).abs() < 1e-5);
        assert_eq!(obs[1].cos_half, -2.0);
    }
}
