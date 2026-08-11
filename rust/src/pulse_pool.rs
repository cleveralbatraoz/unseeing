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

/// The `dir.w` value marking an omnidirectional pulse — a wave with no
/// direction at all. Public because a source's [`crate::sound_source::Spread`]
/// must speak the same sentinel the shaders decode.
pub const OMNI_COS: f64 = -2.0;

/// Ring time + outline-fade tail; echoes and footsteps expire sooner so the
/// live-slot count (which both shaders loop over per pixel) stays small.
/// Total over every i32: unknown kinds get the tap's long tail, exactly as
/// GDScript's `match` wildcard did. Public because a source's slot budget
/// ([`crate::sound_source::Voice::slot_pressure`]) is this tail plus its
/// ring time — one source of truth for how long a wave occupies its slot.
#[must_use]
pub fn fade_tail(kind: i32) -> f64 {
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

/// One slot, all six lanes — the shader-facing f32 triplet AND the f64
/// shadow eviction runs on. Verbatim copies both ways: decoding and
/// re-encoding the packed lanes would lose gain precision (dat.w packs
/// kind*10 + gain*9 as f32), and re-deriving the shadow from the lanes
/// would narrow the very widths the shadow exists to keep.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotCapture {
    pub pos: Vector3,
    pub dat: Vector4,
    pub dir: Vector4,
    pub t0: f64,
    pub end: f64,
    pub kind: i32,
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

    /// Every slot, verbatim — holes, expired lanes and virgin sentinels
    /// included, because slot_scan_limit and future eviction read them.
    #[must_use]
    pub fn capture_slots(&self) -> Box<[SlotCapture; MAXP]> {
        let mut slots = Box::new(
            [SlotCapture {
                pos: Vector3::ZERO,
                dat: Vector4::ZERO,
                dir: Vector4::ZERO,
                t0: 0.0,
                end: 0.0,
                kind: 0,
            }; MAXP],
        );
        for i in 0..MAXP {
            slots[i] = SlotCapture {
                pos: self.pos[i],
                dat: self.dat[i],
                dir: self.dir[i],
                t0: self.t0[i],
                end: self.end[i],
                kind: self.kind[i],
            };
        }
        slots
    }

    /// A pool rebuilt from a capture, bit-identical. Total: any slot
    /// values are legal — the capture is trusted verbatim.
    ///
    /// Which means a TAMPERED slot cannot show up here, or anywhere
    /// downstream of here: it is copied in exactly, copied back out
    /// exactly, and so the restored world honestly agrees with the file it
    /// came from. [`crate::reproduce::first_divergence`] compares the world
    /// against the blob's fields and is right to see nothing. What knows
    /// the difference is the blob's own stored hash, and the one place that
    /// is compared is `main.gd::restore_blob`, after the transaction
    /// succeeds — see the note there.
    #[must_use]
    pub fn from_slots(slots: &[SlotCapture; MAXP]) -> Self {
        let mut pool = Self::new();
        for (i, slot) in slots.iter().enumerate() {
            pool.pos[i] = slot.pos;
            pool.dat[i] = slot.dat;
            pool.dir[i] = slot.dir;
            pool.t0[i] = slot.t0;
            pool.end[i] = slot.end;
            pool.kind[i] = slot.kind;
        }
        pool
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
            if omni {
                OMNI_COS as f32
            } else {
                cos_half as f32
            },
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
        self.emit(kind, at, max_r, speed, gain, now, Vector3::ZERO, OMNI_COS)
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

    /// Round trip is BIT-identical on every lane — including the expired
    /// slot's stale lanes (they feed slot_scan_limit) and the virgin
    /// asymmetry (dat.x = -1 while t0 = 0.0). Literals from the pool
    /// contract: a kind-2 wave with max_r 1.6, speed 4.0 born at t = 0
    /// dies at 1.6/4.0 + 2.5 = 2.9.
    #[test]
    fn a_captured_pool_restores_bit_identical_holes_and_all() {
        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::new(1.0, 0.0, 2.0), 1.6, 4.0, 0.8, 0.0)
            .unwrap();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        // t = 5.0: slot 0 expired (dead at 2.9), slot 1 live — a hole
        let capture = pool.capture_slots();
        let restored = PulsePool::from_slots(&capture);
        assert_eq!(restored.dat(), pool.dat());
        assert_eq!(restored.pos(), pool.pos());
        assert_eq!(restored.dir(), pool.dir());
        // the shadow survives at full width: the hole still spans
        assert_eq!(restored.live_count(5.0), pool.live_count(5.0));
        assert_eq!(restored.live_count(5.0), 2);
        // virgin slot 2 keeps its asymmetric sentinel
        assert_eq!(capture[2].dat.x, -1.0);
        assert_eq!(capture[2].t0, 0.0);
        assert_eq!(capture[2].end, -1.0);
    }

    /// The restored pool EVICTS like the original: the next emit claims
    /// the same slot for the same reason. This is the f64-shadow property
    /// a lanes-only capture (f32 dat.x) cannot guarantee.
    #[test]
    fn a_restored_pool_evicts_exactly_like_the_original() {
        let mut pool = PulsePool::new();
        // two live recurring waves whose f64 births differ by less than
        // one f32 ULP at this magnitude — indistinguishable in dat.x
        let base = 1000.0;
        let tiny = 1e-5; // < f32 ULP at 1000 (~6.1e-5)
        pool.emit_omni(2, Vector3::ZERO, 60.0, 4.0, 0.8, base + tiny)
            .unwrap();
        pool.emit_omni(2, Vector3::ZERO, 60.0, 4.0, 0.8, base)
            .unwrap();
        assert_eq!(pool.dat()[0].x, pool.dat()[1].x); // f32 cannot tell
        let mut restored = PulsePool::from_slots(&pool.capture_slots());
        // pool full of live waves? No — 62 virgin slots remain; fill
        // rule (1) takes the first expired/virgin slot for both pools.
        // The discriminating emit: claim every remaining slot first...
        for i in 0..62 {
            let t = base + 1.0 + f64::from(i) * 1e-6;
            pool.emit_omni(0, Vector3::ZERO, 600.0, 4.0, 1.0, t)
                .unwrap();
            restored
                .emit_omni(0, Vector3::ZERO, 600.0, 4.0, 1.0, t)
                .unwrap();
        }
        // ...now eviction must choose the OLDER kind-2 wave: slot 1
        // (born at base), not slot 0 (born tiny later). Only the f64
        // shadow can make that call.
        pool.emit_omni(2, Vector3::ZERO, 5.0, 4.0, 0.5, base + 2.0)
            .unwrap();
        restored
            .emit_omni(2, Vector3::ZERO, 5.0, 4.0, 0.5, base + 2.0)
            .unwrap();
        assert_eq!(pool.dat()[1].y, 5.0); // victim was slot 1 in both
        assert_eq!(restored.dat()[1].y, 5.0);
        assert_eq!(restored.dat()[0].y, 60.0);
    }
}
