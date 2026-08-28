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
//! hero did not make; its wave stops at a wall like every other kind).
//!
//! Precision law, pinned from the original: clocks and lifetimes live in
//! f64 (`now`, `_t0`, `_end` were GDScript floats / PackedFloat64Array),
//! while everything the shader sees is narrowed to f32 exactly where
//! GDScript builds its Vector4s — no earlier, so eviction and expiry
//! compare full-width times while the packed lanes match the uniforms
//! bit for bit.

use godot::builtin::{Vector3, Vector4};

use crate::reproduce::RestoreValueError;
use crate::support_motion::MAX_POSE_COORD_M;
use crate::temporal::{PreparedTime, RENDERER_VISIBLE_TIME_HORIZON, prepare_time};

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

#[derive(Debug, Clone)]
pub struct PreparedPulsePool(PulsePool);

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

/// Why a raw wave request cannot safely enter every CPU and shader consumer.
/// The field is relative so each boundary can attach its own dotted prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WaveValueError {
    field: &'static str,
    rule: &'static str,
}

impl WaveValueError {
    fn new(field: &'static str, rule: &'static str) -> Self {
        Self { field, rule }
    }

    pub(crate) fn field(self) -> &'static str {
        self.field
    }

    pub(crate) fn rule(self) -> &'static str {
        self.rule
    }
}

impl From<WaveValueError> for EmitRefused {
    fn from(error: WaveValueError) -> Self {
        let _diagnostic = (error.field, error.rule);
        Self
    }
}

/// A world-space pulse origin admitted to the closed numerical envelope shared
/// with [`MAX_POSE_COORD_M`]. The raw f32 lanes are retained verbatim: this is
/// a producer/artifact admission bound, never a clamp or a theorem about an
/// arbitrary actor, camera, matrix, wall, or authored-geometry input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct WaveOrigin(Vector3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WaveOriginError {
    axis: &'static str,
    rule: &'static str,
}

impl WaveOriginError {
    pub(crate) fn axis(self) -> &'static str {
        self.axis
    }

    pub(crate) fn rule(self) -> &'static str {
        self.rule
    }
}

impl WaveOrigin {
    pub(crate) fn try_new(world: Vector3) -> Result<Self, WaveOriginError> {
        for (lane, axis) in [world.x, world.y, world.z].into_iter().zip(["x", "y", "z"]) {
            if !lane.is_finite() {
                return Err(WaveOriginError {
                    axis,
                    rule: "must be finite",
                });
            }
            if lane.abs() > MAX_POSE_COORD_M {
                return Err(WaveOriginError {
                    axis,
                    rule: "must lie inside the renderer coordinate envelope",
                });
            }
        }
        Ok(Self(world))
    }

    pub(crate) fn world(self) -> Vector3 {
        self.0
    }
}

/// A wave request checked once for every CPU and shader consumer.
/// Construction is the one door shared by queue preflight and immediate
/// emission; installation copies its already-packed slot without repacking.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckedWave {
    slot: SlotCapture,
    effective_gain: f64,
    raw_speed: f64,
}

impl CheckedWave {
    #[expect(
        clippy::too_many_arguments,
        reason = "the checked door mirrors the existing wave request"
    )]
    pub(crate) fn prepare(
        raw_kind: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: PreparedTime,
        beam_dir: Vector3,
        cos_half: f64,
    ) -> Result<Self, WaveValueError> {
        let kind = i32::try_from(raw_kind)
            .map_err(|_| WaveValueError::new("type", "must fit the pulse kind lane"))?;
        let at = WaveOrigin::try_new(at).map_err(|error| {
            let field = match error.axis() {
                "x" => "at.x",
                "y" => "at.y",
                _ => "at.z",
            };
            WaveValueError::new(field, error.rule())
        })?;
        for (field, value) in [
            ("max_r", max_r),
            ("speed", speed),
            ("gain", gain),
            ("beam_dir.x", f64::from(beam_dir.x)),
            ("beam_dir.y", f64::from(beam_dir.y)),
            ("beam_dir.z", f64::from(beam_dir.z)),
            ("cos_half", cos_half),
        ] {
            if !value.is_finite() {
                return Err(WaveValueError::new(field, "must be finite"));
            }
        }
        if max_r <= 0.0 {
            return Err(WaveValueError::new("max_r", "must be strictly positive"));
        }
        if speed <= 0.0 {
            return Err(WaveValueError::new("speed", "must be strictly positive"));
        }

        let max_r_lane = max_r as f32;
        if !max_r_lane.is_finite() || max_r_lane <= 0.0 {
            return Err(WaveValueError::new(
                "max_r",
                "must narrow to a finite positive shader lane",
            ));
        }
        let speed_lane = speed as f32;
        if !speed_lane.is_finite() || speed_lane <= 0.0 {
            return Err(WaveValueError::new(
                "speed",
                "must narrow to a finite positive shader lane",
            ));
        }

        let clamped_gain = gain.clamp(0.0, 1.0);
        let packed = (f64::from(kind) * 10.0 + clamped_gain * 9.0) as f32;
        let (decoded_kind, decoded_gain) = decode_packed(packed);
        if !packed.is_finite()
            || f64::from(decoded_kind) != f64::from(kind)
            || !decoded_gain.is_finite()
            || !(0.0..=1.0).contains(&decoded_gain)
        {
            return Err(WaveValueError::new(
                "type",
                "kind and gain must round-trip through the GLSL packed lane",
            ));
        }

        let omni = beam_dir == Vector3::ZERO;
        if omni && cos_half.to_bits() != OMNI_COS.to_bits() {
            return Err(WaveValueError::new(
                "cos_half",
                "an omnidirectional request must use the exact shader sentinel",
            ));
        }
        let cone_lane = if omni {
            OMNI_COS as f32
        } else {
            cos_half as f32
        };
        if !cone_lane.is_finite() {
            return Err(WaveValueError::new(
                "cos_half",
                "must narrow to a finite shader lane",
            ));
        }
        let t0 = now.value();
        let end = t0 + max_r / speed + fade_tail(kind);
        if !end.is_finite() {
            return Err(WaveValueError::new(
                "end",
                "must remain finite through the CPU lifetime calculation",
            ));
        }
        validate_shader_arithmetic(t0 as f32, max_r_lane, speed_lane, beam_dir, cone_lane)?;

        Ok(Self {
            slot: SlotCapture {
                pos: at.world(),
                dat: Vector4::new(t0 as f32, max_r_lane, speed_lane, packed),
                dir: Vector4::new(beam_dir.x, beam_dir.y, beam_dir.z, cone_lane),
                t0,
                end,
                kind,
            },
            effective_gain: f64::from(decoded_gain),
            raw_speed: speed,
        })
    }

    pub(crate) fn slot(self) -> SlotCapture {
        self.slot
    }

    pub(crate) fn effective_gain(self) -> f64 {
        self.effective_gain
    }

    pub(crate) fn raw_speed(self) -> f64 {
        self.raw_speed
    }
}

pub(crate) fn decode_packed(packed: f32) -> (f32, f32) {
    let kind = (packed / 10.0).floor();
    let gain = (packed - 10.0 * kind) / 9.0;
    (kind, gain)
}

fn f32_preimage(q: f32) -> Option<(f64, f64)> {
    if !q.is_finite() || q <= 0.0 {
        return None;
    }
    let q64 = f64::from(q);
    let lo = (f64::from(q.next_down()) + q64) / 2.0;
    let next = q.next_up();
    let hi = if next.is_infinite() {
        q64 + (q64 - f64::from(q.next_down())) / 2.0
    } else {
        (q64 + f64::from(next)) / 2.0
    };
    Some((lo, hi))
}

fn end_envelope(slot: &SlotCapture) -> Option<(f64, f64)> {
    let (range_lo, range_hi) = f32_preimage(slot.dat.y)?;
    let (speed_lo, speed_hi) = f32_preimage(slot.dat.z)?;
    let ratio_lo = (range_lo / speed_hi).next_down();
    let ratio_hi = (range_hi / speed_lo).next_up();
    let end_lo = ((slot.t0 + ratio_lo).next_down() + fade_tail(slot.kind)).next_down();
    let end_hi = ((slot.t0 + ratio_hi).next_up() + fade_tail(slot.kind)).next_up();
    (end_lo.is_finite() && end_hi.is_finite()).then_some((end_lo, end_hi))
}

fn validate_shader_arithmetic(
    birth: f32,
    max_r: f32,
    speed: f32,
    beam_dir: Vector3,
    cone: f32,
) -> Result<(), WaveValueError> {
    validate_direction(beam_dir, cone)?;
    let age_cap = RENDERER_VISIBLE_TIME_HORIZON as f32 - birth;
    let radius = speed * age_cap;
    let progress = radius / max_r;
    let ring_time = max_r / speed;
    let capped_radius = radius.min(max_r);
    let radius_squared = capped_radius * capped_radius;
    for (field, value) in [
        ("birth", birth),
        ("age_cap", age_cap),
        ("radius", radius),
        ("progress", progress),
        ("ring_time", ring_time),
        ("capped_radius", capped_radius),
        ("radius_squared", radius_squared),
    ] {
        if !value.is_finite() {
            return Err(WaveValueError::new(
                field,
                "must remain finite in f32 shader arithmetic",
            ));
        }
    }
    Ok(())
}

fn validate_direction(beam_dir: Vector3, cone: f32) -> Result<(), WaveValueError> {
    if beam_dir == Vector3::ZERO {
        if cone.to_bits() != (OMNI_COS as f32).to_bits() {
            return Err(WaveValueError::new(
                "beam_dir",
                "an omnidirectional wave must use the exact shader sentinel",
            ));
        }
        return Ok(());
    }

    let length_squared =
        beam_dir.x * beam_dir.x + beam_dir.y * beam_dir.y + beam_dir.z * beam_dir.z;
    if !length_squared.is_finite() || length_squared <= 0.0 {
        return Err(WaveValueError::new(
            "beam_dir",
            "must have a finite positive f32 length-squared",
        ));
    }
    let cone_lo = cone - 0.15;
    let cone_hi = cone + 0.05;
    if !cone_lo.is_finite() || !cone_hi.is_finite() || cone_lo >= cone_hi {
        return Err(WaveValueError::new(
            "cos_half",
            "must produce finite strictly ordered cone edges",
        ));
    }
    Ok(())
}

impl Default for PulsePool {
    fn default() -> Self {
        Self::new()
    }
}

impl PulsePool {
    pub(crate) fn prepare_restore(
        slots: &[SlotCapture; MAXP],
        now: PreparedTime,
    ) -> Result<PreparedPulsePool, RestoreValueError> {
        for (index, slot) in slots.iter().enumerate() {
            for (field, value) in [
                ("pos.x", f64::from(slot.pos.x)),
                ("pos.y", f64::from(slot.pos.y)),
                ("pos.z", f64::from(slot.pos.z)),
                ("dat.x", f64::from(slot.dat.x)),
                ("dat.y", f64::from(slot.dat.y)),
                ("dat.z", f64::from(slot.dat.z)),
                ("dat.w", f64::from(slot.dat.w)),
                ("dir.x", f64::from(slot.dir.x)),
                ("dir.y", f64::from(slot.dir.y)),
                ("dir.z", f64::from(slot.dir.z)),
                ("dir.w", f64::from(slot.dir.w)),
                ("t0", slot.t0),
                ("end", slot.end),
            ] {
                if !value.is_finite() {
                    return Err(RestoreValueError::new(
                        format!("slots[{index}].{field}"),
                        "must be finite",
                    ));
                }
            }
        }
        let now = now.value();
        let scan_high_water = scan_high_water(slots, now);
        for (index, slot) in slots.iter().enumerate().skip(scan_high_water) {
            if slot.end >= now {
                return Err(RestoreValueError::new(
                    format!("slots[{index}].end"),
                    "must be below restore time above the scan high-water mark",
                ));
            }
        }
        for (index, slot) in slots.iter().enumerate() {
            if is_virgin_slot(slot) {
                continue;
            }
            WaveOrigin::try_new(slot.pos).map_err(|error| {
                RestoreValueError::new(format!("slots[{index}].pos.{}", error.axis()), error.rule())
            })?;
            if slot.t0 < 0.0 || slot.t0 > now {
                return Err(RestoreValueError::new(
                    format!("slots[{index}].t0"),
                    "must lie between the simulation epoch and restore time",
                ));
            }
            if slot.dat.x.to_bits() != (slot.t0 as f32).to_bits() {
                return Err(RestoreValueError::new(
                    format!("slots[{index}].dat.x"),
                    "must match the f32 image of the CPU birth time bit-for-bit",
                ));
            }
            if slot.dat.y <= 0.0 {
                return Err(RestoreValueError::new(
                    format!("slots[{index}].dat.y"),
                    "must be strictly positive for shader arithmetic",
                ));
            }
            if slot.dat.z <= 0.0 {
                return Err(RestoreValueError::new(
                    format!("slots[{index}].dat.z"),
                    "must be strictly positive for shader arithmetic",
                ));
            }
            let (gpu_kind, gpu_gain) = decode_packed(slot.dat.w);
            if f64::from(gpu_kind) != f64::from(slot.kind) {
                return Err(RestoreValueError::new(
                    format!("slots[{index}].kind"),
                    "must agree with the GLSL-decoded packed kind",
                ));
            }
            if !gpu_gain.is_finite() || !(0.0..=1.0).contains(&gpu_gain) {
                return Err(RestoreValueError::new(
                    format!("slots[{index}].dat.w"),
                    "must decode to a finite gain in 0..=1",
                ));
            }
            validate_shader_arithmetic(
                slot.dat.x,
                slot.dat.y,
                slot.dat.z,
                Vector3::new(slot.dir.x, slot.dir.y, slot.dir.z),
                slot.dir.w,
            )
            .map_err(|error| {
                RestoreValueError::new(format!("slots[{index}].{}", error.field), error.rule)
            })?;
            let (end_lo, end_hi) = end_envelope(slot).ok_or_else(|| {
                RestoreValueError::new(
                    format!("slots[{index}].end"),
                    "must have a finite lifetime envelope",
                )
            })?;
            if !(end_lo..=end_hi).contains(&slot.end) {
                return Err(RestoreValueError::new(
                    format!("slots[{index}].end"),
                    "must lie inside the f32 range/speed preimage lifetime envelope",
                ));
            }
        }
        Ok(PreparedPulsePool(Self::from_slots(slots)))
    }

    #[must_use]
    pub fn from_prepared(value: PreparedPulsePool) -> Self {
        value.0
    }

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

    /// Copy already-proven slots bit-identically. Private so production can
    /// only reach it through [`Self::prepare_restore`]; tests use it to pin
    /// the historical bit-preserving copy and eviction behavior.
    #[must_use]
    fn from_slots(slots: &[SlotCapture; MAXP]) -> Self {
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
    /// Omnidirectional requests use the exact shader tuple
    /// `beam_dir == Vector3::ZERO`, `cos_half == -2`; mismatched tuples are
    /// refused rather than rewritten.
    ///
    /// # Errors
    ///
    /// [`EmitRefused`] when `speed` or `max_r` is non-finite or non-positive;
    /// the pool is left
    /// untouched — no slot is taken, nothing immortal is created.
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the GDScript emit() signature one to one; the FFI \
                  shim forwards it verbatim, so grouping would only add a \
                  translation layer to drift in"
    )]
    pub fn emit(
        &mut self,
        kind: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
        beam_dir: Vector3,
        cos_half: f64,
    ) -> Result<(), EmitRefused> {
        let now = prepare_time(now).map_err(|_| EmitRefused)?;
        let checked = CheckedWave::prepare(kind, at, max_r, speed, gain, now, beam_dir, cos_half)
            .map_err(EmitRefused::from)?;
        self.install_checked(checked);
        Ok(())
    }

    pub(crate) fn install_checked(&mut self, checked: CheckedWave) {
        let prepared = checked.slot();
        let mut slot: Option<usize> = None;
        let mut old_step: Option<usize> = None;
        let mut oldest: Option<usize> = None;
        let mut t_old_step = f64::INFINITY;
        let mut t_old = f64::INFINITY;
        for i in 0..MAXP {
            if self.end[i] < prepared.t0 {
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
        self.pos[slot] = prepared.pos;
        self.dat[slot] = prepared.dat;
        self.dir[slot] = prepared.dir;
        self.t0[slot] = prepared.t0;
        self.end[slot] = prepared.end;
        self.kind[slot] = prepared.kind;
    }

    /// [`Self::emit`] without a beam — the mirror of GDScript's default
    /// arguments (`beam_dir := Vector3.ZERO, cos_half := -2.0`), which is
    /// how every cane tap, footstep and echo emits.
    ///
    /// # Errors
    ///
    /// [`EmitRefused`] when `speed` or `max_r` is non-finite or non-positive.
    pub fn emit_omni(
        &mut self,
        kind: i64,
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

fn is_virgin_slot(slot: &SlotCapture) -> bool {
    [slot.pos.x, slot.pos.y, slot.pos.z]
        .into_iter()
        .all(|lane| lane.to_bits() == 0.0_f32.to_bits())
        && slot.dat.x.to_bits() == (-1.0_f32).to_bits()
        && [slot.dat.y, slot.dat.z, slot.dat.w]
            .into_iter()
            .all(|lane| lane.to_bits() == 0.0_f32.to_bits())
        && [slot.dir.x, slot.dir.y, slot.dir.z, slot.dir.w]
            .into_iter()
            .all(|lane| lane.to_bits() == 0.0_f32.to_bits())
        && slot.t0.to_bits() == 0.0_f64.to_bits()
        && slot.end.to_bits() == (-1.0_f64).to_bits()
        && slot.kind == 0
}

fn scan_high_water(slots: &[SlotCapture; MAXP], now: f64) -> usize {
    slots
        .iter()
        .rposition(|slot| slot.end >= now)
        .map_or(0, |index| index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::temporal::prepare_time;

    /// The shader's decode of the packed kind digit: floor(w / 10).
    fn kind_of(w: f32) -> i32 {
        (f64::from(w) / 10.0).floor() as i32
    }

    /// The shader's decode of the packed gain: mod(w, 10) / 9.
    fn gain_of(w: f32) -> f64 {
        (f64::from(w) % 10.0) / 9.0
    }

    #[test]
    fn wave_origin_accepts_each_closed_boundary_and_refuses_each_adjacent_outer_lane() {
        fn lane(axis: usize, value: f32) -> Vector3 {
            match axis {
                0 => Vector3::new(value, 0.0, 0.0),
                1 => Vector3::new(0.0, value, 0.0),
                _ => Vector3::new(0.0, 0.0, value),
            }
        }

        let maximum = 1_000_002.0_f32;
        for (axis, name) in ["x", "y", "z"].into_iter().enumerate() {
            for boundary in [-maximum, maximum] {
                let accepted = WaveOrigin::try_new(lane(axis, boundary))
                    .expect("the numerical coordinate envelope is closed");
                let actual = [accepted.world().x, accepted.world().y, accepted.world().z][axis];
                assert_eq!(actual.to_bits(), boundary.to_bits(), "{name} boundary");
            }
            for outside in [(-maximum).next_down(), maximum.next_up()] {
                let error = WaveOrigin::try_new(lane(axis, outside))
                    .expect_err("the adjacent outer f32 lane must be refused");
                assert_eq!(error.axis(), name);
            }
        }
    }

    #[test]
    fn checked_wave_refuses_an_origin_that_makes_the_hearing_discriminant_nan() {
        let oc = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let rd = Vector3::new(1.0, 0.0, 0.0);
        let b = rd.dot(oc);
        let oc_squared = oc.dot(oc);
        let discriminant = b * b - (oc_squared - 36.0_f32);
        assert_eq!(b * b, f32::INFINITY);
        assert_eq!(oc_squared, f32::INFINITY);
        assert!(discriminant.is_nan());

        let mut pool = PulsePool::new();
        let before = pool.capture_slots();
        assert_eq!(
            pool.emit_omni(0, Vector3::new(f32::MAX, 0.0, 0.0), 6.0, 5.5, 1.0, 0.0,),
            Err(EmitRefused)
        );
        assert_eq!(pool.capture_slots(), before);
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

    /// THE BREAK: a NaN reaching the pool, and through it the G-buffer.
    /// `speed <= 0.0` is FALSE for NaN, so the plain comparison waves it
    /// through — and `speed` arrives from a designer `#[export]` on a
    /// `WaveFan` or `WaveRadio`, which is exactly the untrusted-Godot-value
    /// boundary AGENTS.md says to validate. Downstream, `since_front =
    /// age - dist / speed` is then NaN for every fragment the pulse
    /// reaches, and GLSL leaves `exp(NaN)` and `clamp(NaN, 0, 1)`
    /// undefined, so the whole silhouette it touches is undefined output.
    /// `max_r` has the same hole for the same reason.
    #[test]
    fn a_non_finite_speed_or_radius_is_refused_as_a_non_positive_one_is() {
        let mut p = PulsePool::new();
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                p.emit_omni(0, Vector3::ZERO, 6.0, bad, 1.0, 0.0),
                Err(EmitRefused),
                "speed {bad} took a slot"
            );
            assert_eq!(
                p.emit_omni(0, Vector3::ZERO, bad, 5.5, 1.0, 0.0),
                Err(EmitRefused),
                "radius {bad} took a slot"
            );
            // and the ORIGIN, which is the other road into the same NaN:
            // the fragment law is `age - dist / speed`, and `dist` is
            // measured from this point
            let bad_at = Vector3::new(bad as f32, 0.0, 0.0);
            assert_eq!(
                p.emit_omni(0, bad_at, 6.0, 5.5, 1.0, 0.0),
                Err(EmitRefused),
                "origin {bad} took a slot"
            );
        }
        assert_eq!(p.live_count(0.1), 0);
        assert_eq!(p.live_count(1.0e9), 0);
    }

    /// A positive f64 is not enough: the shader receives f32 lanes. A
    /// radius that narrows to infinity and a speed that narrows to zero
    /// would make the next shader frame non-finite even though both raw
    /// inputs passed the legacy sign check.
    #[test]
    fn checked_wave_rejects_positive_f64_that_narrows_to_zero_or_infinity() {
        let mut pool = PulsePool::new();
        assert_eq!(
            pool.emit_omni(0, Vector3::ZERO, f64::MAX, 5.5, 1.0, 0.0),
            Err(EmitRefused)
        );
        assert_eq!(
            pool.emit_omni(0, Vector3::ZERO, 6.0, f64::MIN_POSITIVE, 1.0, 0.0,),
            Err(EmitRefused)
        );
        assert_eq!(pool.capture_slots(), PulsePool::new().capture_slots());

        for (max_r, speed, field) in [(f64::MAX, 5.5, "max_r"), (6.0, f64::MIN_POSITIVE, "speed")] {
            let error = CheckedWave::prepare(
                0,
                Vector3::ZERO,
                max_r,
                speed,
                1.0,
                prepare_time(0.0).unwrap(),
                Vector3::ZERO,
                OMNI_COS,
            )
            .expect_err("narrowed shader lanes must be checked at their own width");
            assert_eq!(error.field(), field);
            assert!(error.rule().contains("narrow"));
        }
    }

    #[test]
    fn checked_wave_refuses_a_directed_cone_whose_f32_edges_collapse() {
        let mut pool = PulsePool::new();
        let before = pool.capture_slots();

        for cone in [f64::from(16_777_216.0_f32), f64::from(f32::MAX)] {
            let result = pool.emit(
                3,
                Vector3::ZERO,
                9.0,
                4.5,
                0.75,
                0.0,
                Vector3::new(0.0, 0.0, -1.0),
                cone,
            );
            assert_eq!(result, Err(EmitRefused));
        }
        assert_eq!(pool.capture_slots(), before);
    }

    #[test]
    fn checked_wave_refuses_a_nonexact_omni_tuple() {
        let error = CheckedWave::prepare(
            0,
            Vector3::ZERO,
            6.0,
            5.5,
            1.0,
            prepare_time(0.0).unwrap(),
            Vector3::ZERO,
            -1.0,
        )
        .expect_err("zero direction must arrive with the exact omni cone sentinel");
        assert_eq!(error.field(), "cos_half");
    }

    #[test]
    fn checked_wave_carries_effective_gain_and_raw_speed_for_echo_scheduling() {
        let raw_speed = f64::from_bits(5.5_f64.to_bits() + 1);
        let checked = CheckedWave::prepare(
            0,
            Vector3::ZERO,
            6.0,
            raw_speed,
            2.0,
            prepare_time(0.0).unwrap(),
            Vector3::ZERO,
            OMNI_COS,
        )
        .unwrap();

        assert_eq!(checked.effective_gain().to_bits(), 1.0_f64.to_bits());
        assert_eq!(checked.raw_speed().to_bits(), raw_speed.to_bits());
    }

    #[test]
    fn checked_wave_carries_the_exact_glsl_gain_image_for_echoes() {
        for (kind, gain) in [(0, 0.0), (0, 1.0), (1, 0.75), (2, 0.8), (3, 0.85)] {
            CheckedWave::prepare(
                kind,
                Vector3::ZERO,
                6.0,
                5.5,
                gain,
                prepare_time(0.0).unwrap(),
                Vector3::ZERO,
                OMNI_COS,
            )
            .expect("every shipped kind/gain image must remain admissible");
        }

        let checked = CheckedWave::prepare(
            1_000_000,
            Vector3::ZERO,
            6.0,
            5.5,
            0.5,
            prepare_time(0.0).unwrap(),
            Vector3::ZERO,
            OMNI_COS,
        )
        .expect("the adversarial kind still has an exact packed image");

        assert_eq!(checked.slot().dat.w.to_bits(), 0x4b18_9684);
        assert_eq!(checked.effective_gain().to_bits(), 0x3fdc_71c7_2000_0000);
    }

    #[test]
    fn checked_wave_refuses_wrapping_or_lossy_kind_pack() {
        for kind in [i64::MAX, i64::from(i32::MAX)] {
            let error = CheckedWave::prepare(
                kind,
                Vector3::ZERO,
                6.0,
                5.5,
                0.5,
                prepare_time(0.0).unwrap(),
                Vector3::ZERO,
                OMNI_COS,
            )
            .expect_err("kind must survive both integer and packed shader lanes");
            assert_eq!(error.field(), "type");
        }

        let error = CheckedWave::prepare(
            16_777_220,
            Vector3::ZERO,
            6.0,
            5.5,
            0.1,
            prepare_time(0.0).unwrap(),
            Vector3::ZERO,
            OMNI_COS,
        )
        .expect_err("packed gain must also survive the exact f32 GLSL decoder");
        assert_eq!(error.field(), "type");
    }

    /// Queue preflight and immediate emission must not maintain parallel
    /// packers. The checked request's exact slot is the slot direct emit
    /// installs, including the f64 shadow and every narrowed shader lane.
    #[test]
    fn checked_queue_and_direct_emit_produce_identical_slot_bits() {
        let now = prepare_time(12.25).unwrap();
        let checked = CheckedWave::prepare(
            2,
            Vector3::new(-0.0, 1.25, -3.5),
            1.6,
            4.0,
            0.8,
            now,
            Vector3::ZERO,
            OMNI_COS,
        )
        .unwrap();

        let expected = SlotCapture {
            pos: Vector3::new(-0.0, 1.25, -3.5),
            dat: Vector4::new(12.25, 1.6, 4.0, 27.2),
            dir: Vector4::new(0.0, 0.0, 0.0, -2.0),
            t0: 12.25,
            end: 15.15,
            kind: 2,
        };
        assert_eq!(checked.slot(), expected);

        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::new(-0.0, 1.25, -3.5), 1.6, 4.0, 0.8, 12.25)
            .unwrap();

        assert_eq!(pool.capture_slots()[0], expected);
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

    /// A beamed pulse keeps its direction and cone width; an omni request
    /// carries the exact zero-direction/-2 tuple through the checked door.
    #[test]
    fn beam_packs_direction_and_omni_sentinel() {
        let mut p = PulsePool::new();
        let beam = Vector3::new(0.0, 0.0, -1.0);
        p.emit(3, Vector3::ZERO, 9.0, 4.5, 0.75, 0.0, beam, 0.85)
            .unwrap();
        p.emit(
            0,
            Vector3::ZERO,
            6.0,
            5.5,
            1.0,
            0.0,
            Vector3::ZERO,
            OMNI_COS,
        )
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

    #[test]
    fn prepared_restore_rejects_nonfinite_pool_slot() {
        let mut slots = [SlotCapture {
            pos: Vector3::ZERO,
            dat: Vector4::new(-1.0, 0.0, 0.0, 0.0),
            dir: Vector4::ZERO,
            t0: 0.0,
            end: -1.0,
            kind: 0,
        }; MAXP];
        slots[17].dir.z = f32::NAN;
        let error = PulsePool::prepare_restore(&slots, prepare_time(0.0).unwrap())
            .expect_err("poison must be refused");
        assert_eq!(error.path, "slots[17].dir.z");
    }

    #[test]
    fn prepared_restore_accepts_self_produced_virgin_live_and_expired_hole_slots_bit_exact() {
        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::new(1.0, 0.0, 2.0), 1.6, 4.0, 0.8, 0.0)
            .unwrap();
        pool.emit_omni(0, Vector3::new(-0.0, 0.5, 0.0), 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let slots = pool.capture_slots();

        let prepared = PulsePool::prepare_restore(&slots, prepare_time(5.0).unwrap()).unwrap();
        let restored = PulsePool::from_prepared(prepared);

        assert_eq!(restored.capture_slots(), slots);
        assert_eq!(restored.live_count(5.0), 2);
        assert_eq!(slots[2].dat.x.to_bits(), (-1.0_f32).to_bits());

        let mut poisoned_hole = slots;
        poisoned_hole[0].dat.z = 0.0;
        let error = PulsePool::prepare_restore(&poisoned_hole, prepare_time(5.0).unwrap())
            .expect_err("an expired hole below a later live slot remains shader-reachable");
        assert_eq!(error.path, "slots[0].dat.z");
    }

    #[test]
    fn prepared_restore_accepts_f64_shadows_that_share_f32_birth_range_and_speed_lanes() {
        let range_a = f64::from(1.6_f32);
        let range_b = f64::from_bits(range_a.to_bits() - 1);
        let speed_a = f64::from(4.0_f32);
        let speed_b = f64::from_bits(speed_a.to_bits() + 1);
        let birth_a = f64::from(10.0_f32);
        let birth_b = f64::from_bits(birth_a.to_bits() + 1);

        let mut first = PulsePool::new();
        first
            .emit_omni(2, Vector3::ZERO, range_a, speed_a, 0.8, birth_a)
            .unwrap();
        let mut second = PulsePool::new();
        second
            .emit_omni(2, Vector3::ZERO, range_b, speed_b, 0.8, birth_b)
            .unwrap();
        let first_slots = first.capture_slots();
        let second_slots = second.capture_slots();

        assert_eq!(
            first_slots[0].dat.x.to_bits(),
            second_slots[0].dat.x.to_bits()
        );
        assert_eq!(
            first_slots[0].dat.y.to_bits(),
            second_slots[0].dat.y.to_bits()
        );
        assert_eq!(
            first_slots[0].dat.z.to_bits(),
            second_slots[0].dat.z.to_bits()
        );
        assert_ne!(first_slots[0].t0.to_bits(), second_slots[0].t0.to_bits());
        assert_ne!(first_slots[0].end.to_bits(), second_slots[0].end.to_bits());

        for slots in [&first_slots, &second_slots] {
            let prepared = PulsePool::prepare_restore(slots, prepare_time(10.5).unwrap()).unwrap();
            assert_eq!(*PulsePool::from_prepared(prepared).capture_slots(), **slots);
        }
    }

    #[test]
    fn reachable_slot_refuses_zero_or_negative_range_and_speed() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let baseline = pool.capture_slots();

        for (field, poison) in [
            ("dat.y", 0.0_f32),
            ("dat.y", -1.0),
            ("dat.z", 0.0),
            ("dat.z", -1.0),
        ] {
            let mut slots = baseline.clone();
            if field == "dat.y" {
                slots[0].dat.y = poison;
            } else {
                slots[0].dat.z = poison;
            }
            let error = PulsePool::prepare_restore(&slots, prepare_time(0.5).unwrap())
                .expect_err("a shader-reachable non-positive lane must be refused");
            assert_eq!(error.path, format!("slots[0].{field}"));
        }
    }

    #[test]
    fn reachable_slot_refuses_gpu_kind_different_from_cpu_kind() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let mut slots = pool.capture_slots();
        slots[0].kind = 2;

        let error = PulsePool::prepare_restore(&slots, prepare_time(0.5).unwrap())
            .expect_err("CPU and GLSL kinds must agree");
        assert_eq!(error.path, "slots[0].kind");

        let mut invalid_gain = pool.capture_slots();
        invalid_gain[0].dat.w = 9.5;
        let error = PulsePool::prepare_restore(&invalid_gain, prepare_time(0.5).unwrap())
            .expect_err("the GLSL remainder must decode to a normalized gain");
        assert_eq!(error.path, "slots[0].dat.w");
    }

    #[test]
    fn reachable_slot_refuses_future_t0_or_birth_lane_mismatch() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let baseline = pool.capture_slots();
        let now = prepare_time(0.5).unwrap();

        let mut future = baseline.clone();
        future[0].t0 = 0.75;
        future[0].dat.x = 0.75;
        let error = PulsePool::prepare_restore(&future, now)
            .expect_err("a future CPU birth must be refused");
        assert_eq!(error.path, "slots[0].t0");

        let mut before_epoch = baseline.clone();
        before_epoch[0].t0 = -0.25;
        before_epoch[0].dat.x = -0.25;
        let error = PulsePool::prepare_restore(&before_epoch, now)
            .expect_err("a pre-epoch CPU birth must be refused");
        assert_eq!(error.path, "slots[0].t0");

        let mut mismatched = baseline;
        mismatched[0].dat.x = 0.25;
        let error = PulsePool::prepare_restore(&mismatched, now)
            .expect_err("CPU and shader births must agree at f32 width");
        assert_eq!(error.path, "slots[0].dat.x");
    }

    #[test]
    fn reachable_slot_accepts_end_envelope_endpoints_and_refuses_each_adjacent_outer_f64() {
        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::ZERO, 1.6, 4.0, 0.8, 10.0)
            .unwrap();
        let baseline = pool.capture_slots();
        // Hand-derived once from the specified adjacent-f32 midpoint law for
        // range 1.6f32, speed 4.0f32, t0 10 and kind-2 tail 2.5.
        let end_lo = f64::from_bits(0x4029_cccc_cbb3_3332);
        let end_hi = f64::from_bits(0x4029_cccc_cde6_6669);
        let now = prepare_time(10.5).unwrap();

        for endpoint in [end_lo, end_hi] {
            let mut slots = baseline.clone();
            slots[0].end = endpoint;
            let prepared = PulsePool::prepare_restore(&slots, now)
                .expect("closed f64 envelope endpoints must be accepted");
            assert_eq!(
                PulsePool::from_prepared(prepared).capture_slots()[0]
                    .end
                    .to_bits(),
                endpoint.to_bits()
            );
        }

        for outside in [end_lo.next_down(), end_hi.next_up()] {
            let mut slots = baseline.clone();
            slots[0].end = outside;
            let error = PulsePool::prepare_restore(&slots, now)
                .expect_err("the adjacent outer f64 must be refused");
            assert_eq!(error.path, "slots[0].end");
        }
    }

    #[test]
    fn end_equal_to_now_is_reachable_and_validated() {
        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::ZERO, 1.6, 4.0, 0.8, 0.0)
            .unwrap();
        let baseline = pool.capture_slots();
        let now = baseline[0].end;

        assert_eq!(scan_high_water(&baseline, now), 1);
        PulsePool::prepare_restore(&baseline, prepare_time(now).unwrap())
            .expect("end equal to now is still shader-reachable");

        let mut poisoned = baseline;
        poisoned[0].dat.z = 0.0;
        let error = PulsePool::prepare_restore(&poisoned, prepare_time(now).unwrap())
            .expect_err("the equality-live slot must still be validated");
        assert_eq!(error.path, "slots[0].dat.z");
    }

    #[test]
    fn reachable_slot_refuses_each_nonfinite_f32_shader_intermediate() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let baseline = pool.capture_slots();
        let cases = [
            ("radius", 1.0_f32, f32::MAX),
            ("progress", f32::from_bits(1), 1.0),
            ("ring_time", f32::MAX, f32::from_bits(1)),
            ("radius_squared", f32::MAX, 1.0e14_f32),
        ];
        let now = prepare_time(RENDERER_VISIBLE_TIME_HORIZON).unwrap();

        for (intermediate, range, speed) in cases {
            let mut slots = baseline.clone();
            slots[0].dat.y = range;
            slots[0].dat.z = speed;
            slots[0].end = f64::from(range) / f64::from(speed) + fade_tail(slots[0].kind);

            let error = PulsePool::prepare_restore(&slots, now)
                .expect_err("a nonfinite shader intermediate must be refused");
            assert_eq!(
                error.path,
                format!("slots[0].{intermediate}"),
                "wrong diagnostic for {intermediate}"
            );
        }
    }

    #[test]
    fn restored_slot_refuses_nonexact_omni_and_invalid_directed_cone_arithmetic() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0)
            .unwrap();
        let baseline = pool.capture_slots();
        let now = prepare_time(0.5).unwrap();

        let mut nonexact_omni = baseline.clone();
        nonexact_omni[0].dir.w = (OMNI_COS as f32).next_up();
        let error = PulsePool::prepare_restore(&nonexact_omni, now)
            .expect_err("zero direction must carry the exact omni sentinel");
        assert_eq!(error.path, "slots[0].beam_dir");

        let mut infinite_length = baseline.clone();
        infinite_length[0].dir = Vector4::new(f32::MAX, 0.0, 0.0, 0.5);
        let error = PulsePool::prepare_restore(&infinite_length, now)
            .expect_err("directed f32 length-squared must remain finite");
        assert_eq!(error.path, "slots[0].beam_dir");

        let mut collapsed_edges = baseline;
        collapsed_edges[0].dir = Vector4::new(1.0, 0.0, 0.0, f32::MAX);
        let error = PulsePool::prepare_restore(&collapsed_edges, now)
            .expect_err("directed cone edges must remain finite and strictly ordered");
        assert_eq!(error.path, "slots[0].cos_half");
    }
}
