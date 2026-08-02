//! The pulse pool — the wave system's beating heart, mirrored exactly from
//! the GDScript original (pulses.gd). A fixed pool of 64 pulse slots shared
//! with BOTH shaders as uniform arrays: a pulse is an expanding spherical
//! wavefront, and the shaders derive everything (ring position, outline
//! reveal, fade) from its birth time — the CPU never animates waves, it
//! only records that a sound happened.
//!
//! Slot data layout (the shader-side contract lives in
//! pulse_pool.gdshaderinc):
//! - `pos[i]` — world origin of the sound
//! - `dat[i]` — (birth time, max radius, speed m/s, kind*10 + gain*9)
//! - `dir[i]` — beam direction xyz + cos(half-angle); w = -2 means
//!   omnidirectional (cane taps, footsteps)
//!
//! Kinds: 0 = cane tap, 1 = ECHO (secondary reflection), 2 = footstep,
//! 3 = source hum (constant world sources like the fan — the one sound the
//! hero did not make; drawn through walls, muffled).
//!
//! Precision law, pinned from the original: clocks and lifetimes live in
//! f64 (`now`, `_t0`, `_end` were GDScript floats / PackedFloat64Array),
//! while everything the shader sees is narrowed to f32 exactly where
//! GDScript builds its Vector4s — no earlier, so eviction and expiry
//! compare full-width times while the packed lanes match the uniforms
//! bit for bit.

use godot::builtin::{Vector3, Vector4};

/// Pool capacity — the size of the uniform arrays both shaders loop over
/// per pixel. Fixed forever at the shader contract's 64.
pub const MAXP: usize = 64;

/// The exact message pulses.gd pushes when a wave is refused. The FFI shim
/// keeps pushing this very string on [`EmitRefused`], so the observable
/// GDScript behavior (loud refusal, no slot taken) survives the migration.
pub const REFUSAL_MESSAGE: &str = "Pulses.emit: speed and max_r must be positive — wave refused";

/// A wave that must not exist: non-positive speed or radius. A zero-speed
/// wave would divide by zero into an immortal slot (end = now + max_r / 0);
/// a zero-radius wave is no sound at all. The pure core refuses with a
/// value instead of GDScript's `push_error` — the FFI shim maps this to
/// `push_error(REFUSAL_MESSAGE)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmitRefused;

impl std::fmt::Display for EmitRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REFUSAL_MESSAGE)
    }
}

impl std::error::Error for EmitRefused {}

/// Ring time + outline-fade tail; echoes and footsteps expire sooner so the
/// live-slot count (which both shaders loop over per pixel) stays small.
/// Total over every i32: unknown kinds get the tap's long tail, exactly as
/// GDScript's `match` wildcard did.
fn fade_tail(kind: i32) -> f64 {
    match kind {
        1 => 3.5,
        2 => 2.5,
        3 => 2.0,
        _ => 6.0,
    }
}

/// The fixed pool of 64 pulse slots. `pos`/`dat`/`dir` are the shader-bound
/// lanes; `t0`/`end`/`kind` are the CPU-side shadow (full-width times) that
/// drives eviction and the live count.
#[derive(Debug, Clone)]
pub struct PulsePool {
    pos: [Vector3; MAXP],
    dat: [Vector4; MAXP],
    dir: [Vector4; MAXP],
    t0: [f64; MAXP],
    end: [f64; MAXP],
    kind: [i32; MAXP],
}

impl Default for PulsePool {
    fn default() -> Self {
        Self::new()
    }
}

impl PulsePool {
    /// A dark pool: every slot born dead. The `-1` birth-time sentinel in
    /// `dat` and the `-1` end time mirror pulses.gd's `_init` exactly —
    /// the shaders read dat.x = -1 as "no pulse ever lived here".
    #[must_use]
    pub fn new() -> Self {
        Self {
            pos: [Vector3::ZERO; MAXP],
            dat: [Vector4::new(-1.0, 0.0, 0.0, 0.0); MAXP],
            dir: [Vector4::ZERO; MAXP],
            t0: [0.0; MAXP],
            end: [-1.0; MAXP],
            kind: [0; MAXP],
        }
    }

    /// World origins of the pool's sounds — the shader's `u_ppos` lane.
    #[must_use]
    pub fn pos(&self) -> &[Vector3; MAXP] {
        &self.pos
    }

    /// Packed pulse data — the shader's `u_pdat` lane:
    /// (birth time, max radius, speed, kind*10 + gain*9).
    #[must_use]
    pub fn dat(&self) -> &[Vector4; MAXP] {
        &self.dat
    }

    /// Beam directions + cos(half-angle) — the shader's `u_pdir` lane;
    /// w = -2 marks an omnidirectional pulse.
    #[must_use]
    pub fn dir(&self) -> &[Vector4; MAXP] {
        &self.dir
    }

    /// Record a sound. Total at the door: gain is clamped to [0, 1] (a raw
    /// value would bleed into the packed kind digits) and non-positive speed
    /// or radius is refused — a zero-speed wave would occupy its slot
    /// forever. Slot eviction prefers expired slots, then the oldest
    /// footstep or hum (least precious — both recur), then the oldest of
    /// anything.
    ///
    /// `beam_dir == Vector3::ZERO` means omnidirectional: the packed
    /// `dir.w` becomes the -2 sentinel regardless of `cos_half`, exactly
    /// as the GDScript default-argument path behaved.
    ///
    /// # Errors
    ///
    /// [`EmitRefused`] when `speed <= 0` or `max_r <= 0`; the pool is left
    /// untouched — no slot is taken, nothing immortal is created.
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the GDScript emit() signature one to one; the FFI \
                  shim forwards it verbatim, so grouping would only add a \
                  translation layer to drift in"
    )]
    pub fn emit(
        &mut self,
        kind: i32,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
        beam_dir: Vector3,
        cos_half: f64,
    ) -> Result<(), EmitRefused> {
        if speed <= 0.0 || max_r <= 0.0 {
            return Err(EmitRefused);
        }
        let gain = gain.clamp(0.0, 1.0);
        let mut slot: Option<usize> = None;
        let mut old_step: Option<usize> = None;
        let mut oldest: Option<usize> = None;
        let mut t_old_step = f64::INFINITY;
        let mut t_old = f64::INFINITY;
        for i in 0..MAXP {
            if self.end[i] < now {
                slot = Some(i);
                break;
            }
            if self.kind[i] >= 2 && self.t0[i] < t_old_step {
                t_old_step = self.t0[i];
                old_step = Some(i);
            }
            if self.t0[i] < t_old {
                t_old = self.t0[i];
                oldest = Some(i);
            }
        }
        // Unreachable unless every t0 is non-finite; GDScript's -1 index
        // would land on the last slot — keep the same landing spot.
        let slot = slot.or(old_step).or(oldest).unwrap_or(MAXP - 1);
        self.pos[slot] = at;
        // The one narrowing point: GDScript computed these in f64 and let
        // the Vector4 constructor narrow each lane to f32.
        self.dat[slot] = Vector4::new(
            now as f32,
            max_r as f32,
            speed as f32,
            (f64::from(kind) * 10.0 + gain * 9.0) as f32,
        );
        let omni = beam_dir == Vector3::ZERO;
        self.dir[slot] = Vector4::new(
            beam_dir.x,
            beam_dir.y,
            beam_dir.z,
            if omni { -2.0 } else { cos_half as f32 },
        );
        self.t0[slot] = now;
        self.end[slot] = now + max_r / speed + fade_tail(kind);
        self.kind[slot] = kind;
        Ok(())
    }

    /// [`Self::emit`] without a beam — the mirror of GDScript's default
    /// arguments (`beam_dir := Vector3.ZERO, cos_half := -2.0`), which is
    /// how every cane tap, footstep and echo emits.
    ///
    /// # Errors
    ///
    /// [`EmitRefused`] when `speed <= 0` or `max_r <= 0`.
    pub fn emit_omni(
        &mut self,
        kind: i32,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
    ) -> Result<(), EmitRefused> {
        self.emit(kind, at, max_r, speed, gain, now, Vector3::ZERO, -2.0)
    }

    /// Highest live slot + 1 — lets the shaders break out of dead loop
    /// iterations. A loop bound, not a census: a dead low slot under a live
    /// high slot still counts — holes are spanned, never skipped.
    #[must_use]
    pub fn live_count(&self, now: f64) -> usize {
        let mut n = 0;
        for (i, end) in self.end.iter().enumerate() {
            if *end >= now {
                n = i + 1;
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shader's decode of the packed kind digit: floor(w / 10).
    fn kind_of(w: f32) -> i32 {
        (f64::from(w) / 10.0).floor() as i32
    }

    /// The shader's decode of the packed gain: mod(w, 10) / 9.
    fn gain_of(w: f32) -> f64 {
        (f64::from(w) % 10.0) / 9.0
    }

    #[test]
    fn virgin_pool_is_dark() {
        let p = PulsePool::new();
        for i in 0..MAXP {
            assert_eq!(p.dat()[i], Vector4::new(-1.0, 0.0, 0.0, 0.0));
        }
        assert_eq!(p.live_count(0.0), 0);
    }

    /// The shader decodes kind/gain from dat.w as floor(w/10) and
    /// mod(w,10)/9 — verify emit() packs exactly what that decode expects.
    #[test]
    fn packing_roundtrip() {
        let mut p = PulsePool::new();
        p.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 10.0).unwrap();
        p.emit_omni(2, Vector3::ONE, 1.6, 4.0, 0.8, 10.0).unwrap();
        let w0 = p.dat()[0].w;
        let w1 = p.dat()[1].w;
        assert_eq!(kind_of(w0), 0);
        assert!((gain_of(w0) - 1.0).abs() < 0.001);
        assert_eq!(kind_of(w1), 2);
        assert!((gain_of(w1) - 0.8).abs() < 0.001);
        assert_eq!(p.dat()[0].x, 10.0);
        assert_eq!(p.dat()[0].y, 6.0);
        assert_eq!(p.dat()[0].z, 5.5);
    }

    /// Echoes and footsteps must expire sooner than cane taps: the
    /// live-slot count drives per-pixel shader cost.
    #[test]
    fn per_type_lifetimes() {
        let mut p = PulsePool::new();
        p.emit_omni(0, Vector3::ZERO, 5.5, 5.5, 1.0, 0.0).unwrap(); // tap: ring 1s + 6s tail
        p.emit_omni(1, Vector3::ZERO, 5.5, 5.5, 1.0, 0.0).unwrap(); // echo: ring 1s + 3.5s tail
        p.emit_omni(2, Vector3::ZERO, 5.5, 5.5, 1.0, 0.0).unwrap(); // step: ring 1s + 2.5s tail
        assert_eq!(p.live_count(3.0), 3);
        assert_eq!(p.live_count(4.0), 2);
        assert_eq!(p.live_count(5.0), 1);
        assert_eq!(p.live_count(8.0), 0);
    }

    /// When the pool is full, the oldest footstep is evicted before
    /// anything precious (taps) is touched.
    #[test]
    fn eviction_prefers_footsteps() {
        let mut p = PulsePool::new();
        for i in 0..MAXP {
            let kind = if i == 10 { 2 } else { 0 };
            let at = Vector3::new(i as f32, 0.0, 0.0);
            p.emit_omni(kind, at, 6.0, 5.5, 1.0, 100.0 + i as f64 * 0.001)
                .unwrap();
        }
        p.emit_omni(0, Vector3::new(999.0, 0.0, 0.0), 6.0, 5.5, 1.0, 101.0)
            .unwrap();
        assert_eq!(p.pos()[10], Vector3::new(999.0, 0.0, 0.0));
        assert_eq!(p.pos()[0], Vector3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn live_count_is_highest_slot() {
        let mut p = PulsePool::new();
        assert_eq!(p.live_count(0.0), 0);
        p.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0).unwrap();
        p.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0).unwrap();
        assert_eq!(p.live_count(0.5), 2);
    }

    /// Total at the door: gain outside [0, 1] is clamped before packing. A
    /// raw gain of -1 would bleed into the kind digits (floor(w/10) reads
    /// one kind lower); the clamp keeps the kind field undamaged in both
    /// directions.
    #[test]
    fn gain_clamped_into_pack() {
        let mut p = PulsePool::new();
        p.emit_omni(2, Vector3::ZERO, 6.0, 5.5, 1.5, 0.0).unwrap();
        p.emit_omni(2, Vector3::ZERO, 6.0, 5.5, -1.0, 0.0).unwrap();
        assert_eq!(kind_of(p.dat()[0].w), 2);
        assert!((gain_of(p.dat()[0].w) - 1.0).abs() < 0.001);
        assert_eq!(kind_of(p.dat()[1].w), 2);
        assert!(gain_of(p.dat()[1].w).abs() < 0.001);
    }

    /// A zero-speed wave would divide by zero into an immortal slot (end =
    /// now + max_r / 0); a zero-radius wave is no sound at all. emit
    /// refuses both and takes no slot.
    #[test]
    fn non_positive_speed_or_radius_refused() {
        let mut p = PulsePool::new();
        assert_eq!(
            p.emit_omni(0, Vector3::ZERO, 6.0, 0.0, 1.0, 0.0),
            Err(EmitRefused)
        );
        assert_eq!(
            p.emit_omni(0, Vector3::ZERO, 0.0, 5.5, 1.0, 0.0),
            Err(EmitRefused)
        );
        assert_eq!(p.live_count(0.1), 0);
        assert_eq!(p.live_count(1.0e9), 0); // nothing immortal left behind
    }

    /// Slot reuse prefers the dead: with an expired footstep in slot 0 and
    /// a still-live tap in slot 1, a new emit lands in slot 0.
    #[test]
    fn expired_slot_reused_first() {
        let mut p = PulsePool::new();
        p.emit_omni(2, Vector3::new(1.0, 0.0, 0.0), 1.6, 4.0, 0.8, 0.0)
            .unwrap(); // dead by t = 2.9
        p.emit_omni(0, Vector3::new(2.0, 0.0, 0.0), 6.0, 5.5, 1.0, 0.0)
            .unwrap(); // lives past t = 7
        p.emit_omni(0, Vector3::new(3.0, 0.0, 0.0), 6.0, 5.5, 1.0, 5.0)
            .unwrap();
        assert_eq!(p.pos()[0], Vector3::new(3.0, 0.0, 0.0));
        assert_eq!(p.pos()[1], Vector3::new(2.0, 0.0, 0.0));
    }

    /// All 64 slots hold live taps — nothing cheap to sacrifice: the
    /// oldest tap goes.
    #[test]
    fn full_tap_pool_evicts_oldest_tap() {
        let mut p = PulsePool::new();
        for i in 0..MAXP {
            let at = Vector3::new(i as f32, 0.0, 0.0);
            p.emit_omni(0, at, 6.0, 5.5, 1.0, 100.0 + i as f64 * 0.001)
                .unwrap();
        }
        p.emit_omni(0, Vector3::new(999.0, 0.0, 0.0), 6.0, 5.5, 1.0, 100.1)
            .unwrap();
        assert_eq!(p.pos()[0], Vector3::new(999.0, 0.0, 0.0));
        assert_eq!(p.pos()[1], Vector3::new(1.0, 0.0, 0.0));
    }

    /// A hum (kind 3) recurs every second, so it is less precious than any
    /// tap: with the pool full it is sacrificed even when it is not the
    /// oldest slot.
    #[test]
    fn old_hum_sacrificed_before_taps() {
        let mut p = PulsePool::new();
        for i in 0..MAXP {
            let kind = if i == 7 { 3 } else { 0 };
            let at = Vector3::new(i as f32, 0.0, 0.0);
            p.emit_omni(kind, at, 6.0, 5.5, 1.0, 100.0 + i as f64 * 0.001)
                .unwrap();
        }
        p.emit_omni(0, Vector3::new(999.0, 0.0, 0.0), 6.0, 5.5, 1.0, 100.1)
            .unwrap();
        assert_eq!(p.pos()[7], Vector3::new(999.0, 0.0, 0.0));
        assert_eq!(p.pos()[0], Vector3::new(0.0, 0.0, 0.0)); // oldest tap untouched
    }

    /// live_count is the shader's loop bound, not a census: a dead low slot
    /// under a live high slot still yields high + 1 — holes are spanned,
    /// never skipped.
    #[test]
    fn live_count_spans_holes() {
        let mut p = PulsePool::new();
        p.emit_omni(2, Vector3::ZERO, 1.6, 4.0, 0.8, 0.0).unwrap(); // slot 0: dead by t = 2.9
        p.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0).unwrap(); // slot 1: lives past t = 7
        assert_eq!(p.live_count(5.0), 2);
    }

    /// The fan's hum is kind 3: omnidirectional, short-tailed (it recurs
    /// every second, so slots must free fast), and packed like every other
    /// pulse.
    #[test]
    fn hum_pulses() {
        let mut p = PulsePool::new();
        p.emit_omni(3, Vector3::new(8.6, 1.15, 4.4), 9.0, 4.5, 0.75, 0.0)
            .unwrap();
        assert_eq!(kind_of(p.dat()[0].w), 3);
        assert!(p.dir()[0].w < -1.5);
        // ring time 9/4.5 = 2s, tail 2s -> gone just after 4s
        assert_eq!(p.live_count(3.9), 1);
        assert_eq!(p.live_count(4.1), 0);
    }

    /// A beamed pulse keeps its direction and cone width; a zero beam_dir
    /// collapses to the -2 omni sentinel no matter what cos_half says —
    /// the exact GDScript `beam_dir == Vector3.ZERO` law.
    #[test]
    fn beam_packs_direction_and_omni_sentinel() {
        let mut p = PulsePool::new();
        let beam = Vector3::new(0.0, 0.0, -1.0);
        p.emit(3, Vector3::ZERO, 9.0, 4.5, 0.75, 0.0, beam, 0.85)
            .unwrap();
        p.emit(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0, Vector3::ZERO, 0.85)
            .unwrap();
        let d0 = p.dir()[0];
        assert_eq!(Vector3::new(d0.x, d0.y, d0.z), beam);
        assert_eq!(d0.w, 0.85f32);
        assert_eq!(p.dir()[1].w, -2.0);
    }
}
