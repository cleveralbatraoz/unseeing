//! The engine boundary — the ONE module where Godot engine classes may
//! appear. [`WaveCore`] is the wave system's engine-facing organ: it owns
//! the pure core's pulse pool and echo book and translates between the
//! GDScript surface and the pure modules, adding no law of its own. Every
//! rule it enforces — packing, eviction, clustering, appointments — lives
//! below in the pure crate; this file only carries values across.

use godot::classes::{
    PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D, RandomNumberGenerator,
};
use godot::prelude::*;

use crate::clustering::{self, RayHit};
use crate::echo_queue::{ECHO_KIND, ECHO_MAX_R, ECHO_SPEED, EchoQueue, PendingEcho};
use crate::flicker::{Flicker, Randf};
use crate::observe::reflect::ReflectionRequest;
use crate::pulse_pool::{MAXP, PulsePool, REFUSAL_MESSAGE, SlotCapture};
use crate::ray_fan;

/// The flicker law's randomness adapter: Godot's `randf()` returns f32,
/// widened to f64 at the exact point the GDScript law implicitly did (every
/// GDScript float is already f64, so `_rng.randf()` widens the instant it
/// enters an expression). This impl is the ONLY place `Gd<RandomNumberGenerator>`
/// meets the pure `Randf` trait — [`crate::flicker`] itself stays free of
/// Godot types.
impl Randf for Gd<RandomNumberGenerator> {
    fn randf(&mut self) -> f64 {
        // Fully-qualified on purpose: `self.randf()` would resolve to this
        // very trait method (our `Randf` is in scope on exactly this
        // receiver type) before Rust ever tries the deref-to-engine-method
        // step, recursing forever instead of calling Godot's f32 randf().
        RandomNumberGenerator::randf(self) as f64
    }
}

// gdext's entry-point API requires the `unsafe` keyword on this impl; the
// module scope carries the one permitted exception to deny(unsafe_code).
#[allow(unsafe_code)]
mod entry {
    use godot::prelude::*;

    struct UnseeingCore;

    #[gdextension]
    unsafe impl ExtensionLibrary for UnseeingCore {}
}

/// The wave core behind the GDScript `Pulses` shim: the 64 pulse slots and
/// the echo appointment book, one instance per game world. Method for
/// method it mirrors the pulses.gd surface it replaces — same semantics,
/// same refusal message, same slot and drain order — so every suite that
/// pinned the GDScript pool holds against this class unchanged.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct WaveCore {
    pool: PulsePool,
    echoes: EchoQueue,
    base: Base<RefCounted>,
}

#[godot_api]
impl WaveCore {
    /// Record a sound — `Pulses.emit`, verbatim: gain clamped into the
    /// pack, eviction preferring expired slots then old footsteps/hums,
    /// and a non-positive speed or radius refused loudly with no slot
    /// taken. `beam_dir = ZERO` means omnidirectional however `cos_half`
    /// reads — the GDScript default-argument path.
    #[func]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the GDScript emit() signature one to one; the \
                  shim forwards it verbatim, so grouping would only add a \
                  translation layer to drift in"
    )]
    fn emit(
        &mut self,
        kind: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
        beam_dir: Vector3,
        cos_half: f64,
    ) {
        self.emit_or_refuse(kind, at, max_r, speed, gain, now, beam_dir, cos_half);
    }

    /// Emit a primary sound AND schedule its reflections — the heart of
    /// echo-location, `Pulses.emit_reflecting` verbatim: the golden-angle
    /// fan samples the hemisphere in front of `origin_normal` from just
    /// off the birth surface, struck points cluster per 0.9 m cell, and
    /// each answering point books an echo for the instant the wavefront
    /// reaches it. Without a `space` only the primary emits — the web/CI
    /// degradation path. PHYSICS CONTEXT: with a space this must run
    /// inside the physics tick; the GDScript call sites guarantee it.
    #[func]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the GDScript emit_reflecting() signature one to \
                  one, for the same no-drift reason as emit()"
    )]
    fn emit_reflecting(
        &mut self,
        kind: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
        space: Option<Gd<PhysicsDirectSpaceState3D>>,
        max_echoes: i64,
        origin_normal: Vector3,
    ) {
        // The original emitted the primary first and sampled reflections
        // regardless of its outcome; the order is kept, refusal included.
        self.emit_or_refuse(kind, at, max_r, speed, gain, now, Vector3::ZERO, -2.0);
        let Some(space) = space else {
            return;
        };
        // The same struct the debug explainer describes a fan with, and the
        // same cast: one implementation, so the two can never sample the
        // world differently. The ray count it also returns is the
        // explainer's business, not the emitter's.
        let request = ReflectionRequest {
            at,
            normal: origin_normal,
            max_r,
            speed,
            max_echoes,
            now,
        };
        let (_, hits) = cast_reflection_fan(&request, space);
        for hit in clustering::cluster_hits(hits, clustering::echo_budget(max_echoes)) {
            self.echoes.schedule(now, hit.dist, hit.point, gain, speed);
        }
    }

    /// Fire every echo whose appointment has come — the drain half of
    /// `Pulses.apply`: each fired reflection re-enters the pool as an
    /// ECHO pulse born at drain time, in the pinned reverse-index order.
    #[func]
    fn tick(&mut self, now: f64) {
        for echo in self.echoes.drain(now) {
            // ECHO_MAX_R and ECHO_SPEED are positive constants: refusal
            // is impossible, so the Result carries nothing here.
            let _ = self
                .pool
                .emit_omni(ECHO_KIND, echo.pos, ECHO_MAX_R, ECHO_SPEED, echo.gain, now);
        }
    }

    /// Highest live slot + 1 — the shaders' loop bound, holes spanned.
    #[func]
    fn live_count(&self, now: f64) -> i64 {
        self.pool.live_count(now) as i64
    }

    /// World origins of the pool's sounds — the `u_ppos` uniform lane.
    #[func]
    fn positions(&self) -> PackedVector3Array {
        PackedVector3Array::from(&self.pool.pos()[..])
    }

    /// Packed pulse data (birth, max radius, speed, kind*10 + gain*9) —
    /// the `u_pdat` uniform lane.
    #[func]
    fn pulse_data(&self) -> PackedVector4Array {
        PackedVector4Array::from(&self.pool.dat()[..])
    }

    /// Beam directions + cos(half-angle), w = -2 for omni — the `u_pdir`
    /// uniform lane.
    #[func]
    fn pulse_dirs(&self) -> PackedVector4Array {
        PackedVector4Array::from(&self.pool.dir()[..])
    }

    /// Reflections scheduled but not yet fired — `pending_echo_count()`.
    #[func]
    fn pending_echo_count(&self) -> i64 {
        self.echoes.len() as i64
    }

    /// The scheduled reflections themselves, in discovery order, copied
    /// out as `{at_t, pos, gain}` dictionaries — observable for tests and
    /// debug, like `pending_echoes()` was.
    #[func]
    fn pending_echoes(&self) -> Array<VarDictionary> {
        self.echoes
            .pending()
            .iter()
            .map(|echo| {
                let mut entry = VarDictionary::new();
                entry.set("at_t", echo.at_t);
                entry.set("pos", echo.pos);
                entry.set("gain", echo.gain);
                entry
            })
            .collect()
    }

    /// Proof-of-life for the extension boundary: the number of rays in
    /// the golden-angle reflection fan, served from the pure core.
    #[func]
    fn ray_fan_size(&self) -> i64 {
        ray_fan::RAYS as i64
    }

    /// The pulse pool's capacity, exposed for Godot code to query the
    /// maximum number of concurrent sounds without keeping a duplicate
    /// constant. Mirrors `pulse_pool::MAXP` from the Rust core.
    #[func]
    fn max_pulses(&self) -> i64 {
        MAXP as i64
    }

    /// TEST-ONLY SURFACE: seeds a fresh `RandomNumberGenerator` and runs
    /// the Rust flicker law across `dts`, one [`Flicker::next`] call per
    /// entry, returning one output level per input dt. Exists solely so
    /// `flicker_parity_test.gd` can drive the SAME seeded stream through
    /// both the GDScript `Flicker` and this Rust one and assert the two
    /// arrays match exactly — the bit-exactness proof for the migration.
    /// Cheap (one flicker, one RNG, no allocation beyond the output array)
    /// but never called from the game's own boot path.
    #[func]
    fn flicker_probe(&self, seed: i64, dts: PackedFloat64Array) -> PackedFloat64Array {
        let mut rng = RandomNumberGenerator::new_gd();
        rng.set_seed(seed as u64);
        let mut flicker = Flicker::new();
        let out: Vec<f64> = dts
            .as_slice()
            .iter()
            .map(|&dt| flicker.next(dt, &mut rng))
            .collect();
        PackedFloat64Array::from(&out[..])
    }
}

impl WaveCore {
    /// The pool itself, for READING only — the debug observer decodes it
    /// into an agent-facing snapshot. Deliberately not a `#[func]`: the
    /// GDScript surface stays the shim's mirrored methods, and nothing
    /// outside this crate can reach the slots at all.
    pub(crate) fn pool(&self) -> &PulsePool {
        &self.pool
    }

    /// The echo appointment book, for READING only, under the same rule as
    /// [`Self::pool`]. `pending_echoes()` already copies it out one
    /// dictionary at a time for the suites; the observer decodes the whole
    /// book with the wait on each appointment, from the same source of
    /// truth rather than a second one.
    pub(crate) fn echoes(&self) -> &EchoQueue {
        &self.echoes
    }

    /// All 64 slots as data, for the capture blob — the f32 shader lanes
    /// AND the f64 shadow eviction runs on, verbatim. Dead slots and
    /// virgin sentinels included: a restore that left a stale pulse behind
    /// must not hash the same as one that did not.
    pub(crate) fn capture_pool(&self) -> Box<[SlotCapture; MAXP]> {
        self.pool.capture_slots()
    }

    /// Every reflection still waiting for its appointment, for the capture
    /// blob. Read through the same borrow as [`Self::capture_pool`] by the
    /// observer, so the pool and the book leave one core at one instant.
    pub(crate) fn capture_echoes(&self) -> Vec<PendingEcho> {
        self.echoes.capture()
    }

    /// The write side of those two: both halves of the engine's memory of
    /// sound, replaced at once by the restorer.
    ///
    /// One door for the pair on purpose. The pool and the echo book are one
    /// state — an appointment in the book fires INTO the pool — so a
    /// restore that set one and not the other would leave the world
    /// scheduling reflections of waves it no longer remembers.
    pub(crate) fn restore_state(&mut self, pool: PulsePool, echoes: EchoQueue) {
        self.pool = pool;
        self.echoes = echoes;
    }

    /// The one door into the pool: forwards to the pure emit and maps its
    /// refusal value onto the exact `push_error` the GDScript pool raised,
    /// so the observable refusal (loud message, no slot taken) survives
    /// the migration byte for byte.
    #[expect(
        clippy::too_many_arguments,
        reason = "the same mirrored signature as the public emit()"
    )]
    fn emit_or_refuse(
        &mut self,
        kind: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
        beam_dir: Vector3,
        cos_half: f64,
    ) {
        let refused = self
            .pool
            .emit(kind as i32, at, max_r, speed, gain, now, beam_dir, cos_half)
            .is_err();
        if refused {
            godot_error!("{REFUSAL_MESSAGE}");
        }
    }
}

/// Cast one golden-angle reflection fan into `space`, and report how many
/// rays were cast alongside what they struck.
///
/// THE ONLY reflection cast in the codebase, deliberately. The game's
/// `emit_reflecting` and the debug observer's `explain_reflection` both
/// come here, so a collision mask, an exclusion list, or any other change
/// to how the fan samples the world lands on both at once. Two copies of
/// this loop would drift silently and the explainer would start describing
/// a fan the engine no longer casts — telling an agent a wall answered
/// when it did not, which is the one failure this whole layer exists to
/// prevent.
///
/// PHYSICS CONTEXT: a space state may only be touched inside the physics
/// tick. Both callers guarantee it, by different means — the game's call
/// sites run in `_physics_process`, and the observer answers from its own.
///
/// The count is the rays actually CAST, which is not `ray_fan::RAYS`: the
/// fan is culled to the hemisphere in front of the birth normal, and a
/// query the physics server refuses to build was never cast at all.
pub(crate) fn cast_reflection_fan(
    request: &ReflectionRequest,
    mut space: Gd<PhysicsDirectSpaceState3D>,
) -> (usize, Vec<RayHit>) {
    let origin = request.ray_origin();
    // f64 reach like the GDScript minf; narrowed once, where the ray
    // vector is scaled — exactly where the original narrowed.
    let reach = request.reach() as f32;
    let mut rays_cast = 0usize;
    let mut hits: Vec<RayHit> = Vec::with_capacity(ray_fan::RAYS);
    for dir in request.directions() {
        let Some(query) = PhysicsRayQueryParameters3D::create(origin, origin + dir * reach) else {
            continue; // the engine refused to build a query: skip the ray
        };
        rays_cast += 1;
        let struck = space.intersect_ray(&query);
        let (Some(position), Some(normal)) = (
            struck
                .get("position")
                .and_then(|v| v.try_to::<Vector3>().ok()),
            struck
                .get("normal")
                .and_then(|v| v.try_to::<Vector3>().ok()),
        ) else {
            continue; // empty dictionary: the ray struck nothing
        };
        hits.push(RayHit {
            position,
            normal,
            dist: (position - origin).length(),
        });
    }
    (rays_cast, hits)
}
