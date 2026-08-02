//! The engine boundary — the ONE module where Godot engine classes may
//! appear. [`WaveCore`] is the wave system's engine-facing organ: it owns
//! the pure core's pulse pool and echo book and translates between the
//! GDScript surface and the pure modules, adding no law of its own. Every
//! rule it enforces — packing, eviction, clustering, appointments — lives
//! below in the pure crate; this file only carries values across.

use godot::classes::{PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D};
use godot::prelude::*;

use crate::clustering::{self, RayHit};
use crate::echo_queue::{ECHO_KIND, ECHO_MAX_R, ECHO_SPEED, EchoQueue};
use crate::pulse_pool::{PulsePool, REFUSAL_MESSAGE};
use crate::ray_fan;

struct UnseeingCore;

#[gdextension]
unsafe impl ExtensionLibrary for UnseeingCore {}

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
        let Some(mut space) = space else {
            return;
        };
        let origin = at + origin_normal * clustering::RAY_ORIGIN_LIFT;
        // f64 min like the GDScript minf; narrowed once, where the ray
        // vector is scaled — exactly where the original narrowed.
        let reach = clustering::ray_length(max_r) as f32;
        let mut hits: Vec<RayHit> = Vec::with_capacity(ray_fan::RAYS);
        for dir in ray_fan::fan_directions(origin_normal) {
            let Some(query) = PhysicsRayQueryParameters3D::create(origin, origin + dir * reach)
            else {
                continue; // the engine refused to build a query: skip the ray
            };
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
}

impl WaveCore {
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
