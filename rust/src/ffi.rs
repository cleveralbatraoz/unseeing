//! The wave core's engine boundary — one of the thin modules where Godot
//! engine classes may appear. [`WaveCore`] owns the pure core's pulse pool
//! and echo book and translates between the Godot-facing API and the pure
//! modules, adding no law of its own. Every rule it enforces — packing,
//! eviction, clustering, appointments — lives below in the pure crate;
//! this file only carries values across.

use godot::classes::{
    PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D, RandomNumberGenerator,
};
use godot::prelude::*;

use crate::clustering::{self, RayHit};
use crate::echo_queue::{
    ECHO_KIND, ECHO_MAX_R, ECHO_SPEED, EchoAppointment, EchoQueue, PendingEcho, PreparedEchoQueue,
};
use crate::flicker::Randf;
use crate::level_plan;
use crate::observe::reflect::{
    CheckedRayHit, CheckedReflectionRequest, REFUSED_HIT, ReflectionRequest,
};
use crate::pulse_pool::{
    CheckedWave, MAXP, PreparedPulsePool, PulsePool, REFUSAL_MESSAGE, SlotCapture,
};
use crate::ray_fan;
use crate::render;
use crate::temporal::prepare_time;

/// The flicker law's randomness adapter: Godot's `randf()` returns f32,
/// widened to f64 at the exact point the GDScript law implicitly did (every
/// GDScript float is already f64, so `_rng.randf()` widens the instant it
/// enters an expression). This impl is the ONLY place `Gd<RandomNumberGenerator>`
/// meets the pure `Randf` trait — [`crate::flicker`] itself stays free of
/// Godot types. Needed by `game.rs` to adapt the RNG for the flicker law.
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

/// The wave core the shipped Rust composition root owns directly: the 64
/// pulse slots and echo appointment book, one instance per game world. The
/// test-only GDScript `Pulses` shim mirrors this Godot-facing surface so the
/// suites ported from pulses.gd retain the same semantics, refusal message,
/// slot order and drain order.
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
    /// taken. Omnidirectional requests use the one exact shader tuple:
    /// `beam_dir = ZERO`, `cos_half = -2`.
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
    /// inside the physics tick; shipped Rust player call sites enforce that,
    /// and external or test callers carry the same precondition.
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
        let Some(checked) =
            Self::prepare_wave_or_refuse(kind, at, max_r, speed, gain, now, Vector3::ZERO, -2.0)
        else {
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
            speed: checked.raw_speed(),
            max_echoes,
            now,
        };
        let checked_reflection = match CheckedReflectionRequest::prepare(request) {
            Ok(checked) => checked,
            Err(error) => {
                godot_error!(
                    "WaveCore.emit_reflecting: field {}: {}",
                    error.field(),
                    error.reason()
                );
                return;
            }
        };
        self.pool.install_checked(checked);
        let Some(space) = space else {
            return;
        };
        let hits = match cast_reflection_fan(&checked_reflection, space) {
            Ok((_, hits)) => hits,
            Err(reason) => {
                godot_error!("{reason}");
                return;
            }
        };
        let appointments: Vec<EchoAppointment> =
            clustering::cluster_hits(hits, clustering::echo_budget(max_echoes))
                .into_iter()
                .map(|hit| EchoAppointment {
                    now,
                    dist: hit.dist,
                    pos: hit.point,
                    gain: checked.effective_gain(),
                    speed: checked.raw_speed(),
                })
                .collect();
        match self.echoes.prepare_schedule_batch(&appointments) {
            Ok(prepared) => self.echoes = EchoQueue::from_prepared(prepared),
            Err(error) => {
                godot_error!(
                    "WaveCore.emit_reflecting: echo {} is not representable — refused",
                    error.field()
                );
            }
        }
    }

    /// Fire every echo whose appointment has come — the drain half of
    /// `Pulses.apply`: each fired reflection re-enters the pool as an
    /// ECHO pulse born at drain time, in the pinned reverse-index order.
    ///
    /// `pub(crate)`: the composition root's own `process()` calls this
    /// directly through a typed `Gd<WaveCore>` — the shim's `apply()` loop
    /// moved into `UnseeingGame`, and the handle it drives is the typed
    /// core, never the GDScript shim, so the call is typed rather than
    /// the stringly `.call()` a dynamic `pulses` handle would need.
    #[func]
    pub(crate) fn tick(&mut self, now: f64) {
        let now = match prepare_time(now) {
            Ok(now) => now,
            Err(error) => {
                godot_error!("WaveCore.tick: field now: {} — tick refused", error.rule);
                return;
            }
        };
        for echo in self.echoes.drain(now.value()) {
            // ECHO_MAX_R and ECHO_SPEED are positive constants: refusal
            // is impossible, so the Result carries nothing here.
            let _ = self.pool.emit_omni(
                i64::from(ECHO_KIND),
                echo.pos,
                ECHO_MAX_R,
                ECHO_SPEED,
                echo.gain,
                now.value(),
            );
        }
    }

    /// Highest live slot + 1 — the shaders' loop bound, holes spanned.
    ///
    /// `pub(crate)`, for the same typed-call reason as [`Self::tick`].
    #[func]
    pub(crate) fn live_count(&self, now: f64) -> i64 {
        self.pool.live_count(now) as i64
    }

    /// World origins of the pool's sounds — the `u_ppos` uniform lane.
    ///
    /// `pub(crate)`, for the same typed-call reason as [`Self::tick`].
    #[func]
    pub(crate) fn positions(&self) -> PackedVector3Array {
        PackedVector3Array::from(&self.pool.pos()[..])
    }

    /// Packed pulse data (birth, max radius, speed, kind*10 + gain*9) —
    /// the `u_pdat` uniform lane.
    ///
    /// `pub(crate)`, for the same typed-call reason as [`Self::tick`].
    #[func]
    pub(crate) fn pulse_data(&self) -> PackedVector4Array {
        PackedVector4Array::from(&self.pool.dat()[..])
    }

    /// Beam directions + cos(half-angle), w = -2 for omni — the `u_pdir`
    /// uniform lane.
    ///
    /// `pub(crate)`, for the same typed-call reason as [`Self::tick`].
    #[func]
    pub(crate) fn pulse_dirs(&self) -> PackedVector4Array {
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

    /// Seconds a wave of `kind` keeps revealing a surface after its front
    /// passed — the pool's own slot lifetime, and the end of the reveal
    /// envelope (`render::reveal`).
    ///
    /// Exposed so a suite can hold `pulse_pool.gdshaderinc`'s
    /// `pulse_fade_tail` chain against the Rust table branch by branch. The
    /// GLSL is the copy that renders and Rust is the copy that reasons; the
    /// two are joined by nothing else, and a shader-side edit to one arm of
    /// that chain would otherwise silently give one kind of sound a
    /// different life on screen than the CPU budgeted its slot for.
    ///
    /// Total over every i64, including the kinds `emit` cannot currently
    /// pack: anything outside 0..=3 takes the tap's long tail, exactly as
    /// the Rust `match`'s wildcard arm and the GLSL chain's fallthrough do.
    #[func]
    fn wave_fade_tail(&self, kind: i64) -> f64 {
        render::reveal::reveal_tail(i32::try_from(kind).unwrap_or(i32::MAX))
    }

    /// The acoustic-image band's width, and the two derived numbers that
    /// bracket it — exposed so a suite can hold the shipped GLSL literal
    /// against `render::depth`'s derivation instead of against itself.
    ///
    /// The assertion these replace was `1.0e-5 < 1.0 - 0.999999 + 1.0e-5`,
    /// which reduces to `x < 1e-6 + x` and is true for every x. It passed
    /// happily while the band was a hundred times too narrow to order one
    /// source's own limbs.
    #[func]
    fn source_band(&self) -> f64 {
        render::depth::SOURCE_BAND
    }

    /// Metres of camera distance per distinguishable depth code inside the
    /// band. Two source surfaces closer than this resolve by opaque draw
    /// order rather than by distance.
    #[func]
    fn source_band_resolution(&self) -> f64 {
        render::depth::band_resolution(render::depth::SOURCE_BAND, level_plan::DIST_PACK_RANGE)
    }

    /// The tightest gap between two surfaces of one shipped source that the
    /// band must still order — the fan's guard-to-blade separation.
    #[func]
    fn min_source_limb_gap(&self) -> f64 {
        render::depth::MIN_SOURCE_LIMB_GAP
    }

    /// How close to the eye a WORLD surface would have to stand before it
    /// reached into the band. Beyond this distance nothing in the world can
    /// compete with the acoustic image drawn over it.
    #[func]
    fn deepest_world_fragment_in_band(&self) -> f64 {
        render::depth::deepest_world_fragment_in_band(
            render::depth::SOURCE_BAND,
            render::depth::CAM_NEAR,
            render::depth::CAM_FAR,
        )
    }

    /// The eye's near plane — the other half of the derivation above, and
    /// the value `UnseeingPlayer` builds its camera with.
    #[func]
    fn camera_near(&self) -> f64 {
        render::depth::CAM_NEAR
    }

    /// The whole role table, by name — `render::labels::role_label` served
    /// to the suites so they read the one table instead of transcribing it.
    ///
    /// Every GDScript case that checks a baked label used to carry its own
    /// copy of the number, which is how the table drifted out of its own
    /// separation law and stayed there: the tests agreed with whatever it
    /// said. Reading it here means a re-spacing moves one place and every
    /// suite follows, while the law itself — that no two labels able to
    /// share a frame land within MIN_SEP — is cargo-tested where the whole
    /// label universe is visible at once.
    #[func]
    fn role_labels(&self) -> VarDictionary {
        let mut table = VarDictionary::new();
        for role in render::labels::ALL_ROLES {
            table.set(
                render::labels::role_name(role),
                render::labels::role_label(role),
            );
        }
        table
    }

    /// The palette every wall, prop and source instance is coloured from.
    #[func]
    fn world_palette(&self) -> PackedFloat64Array {
        PackedFloat64Array::from(&render::labels::WORLD_PALETTE[..])
    }

    /// The reveal envelope at `since_front` seconds past the wavefront, for
    /// a wave granted `tail` seconds — `render::reveal::flare`.
    ///
    /// Exposed so a suite can evaluate the SHAPE across its domain rather
    /// than substring-match it. The four constants in
    /// `data_core.gdshaderinc`'s `pulse_flare` are the whole look of the
    /// game and a `contains()` assertion cannot tell 1.3 from 1.0 or a 3.0
    /// time constant from a 4.0 one.
    #[func]
    fn wave_flare(&self, since_front: f64, tail: f64) -> f64 {
        render::reveal::flare(since_front, tail)
    }

    /// The fraction of a wave's tail spent closing its envelope out —
    /// `render::reveal::CLOSE_FRACTION`.
    #[func]
    fn wave_close_fraction(&self) -> f64 {
        render::reveal::CLOSE_FRACTION
    }

    /// When a point `dist` metres from a sound of `kind` travelling at
    /// `speed` stops being revealed by it, measured from the sound's birth.
    ///
    /// This is the one number that pins the shader's time coordinate. The
    /// reveal law is written against seconds-since-the-front-passed
    /// (`ga = age - dist / speed`), and nothing else in the tree asserts
    /// that `ga` is that and not simply `age`: with `ga = age` the fan,
    /// whose ring time is exactly its own 2.0 s tail, would stop revealing
    /// the outer metre of its wash at the instant its front arrived there,
    /// while the hearing pass kept drawing the ring — a ring in the air over
    /// unlit surfaces, and every existing test still green.
    ///
    /// Total over every input: a non-positive or non-finite speed or a
    /// negative distance answers [`f64::NAN`], which fails every assertion
    /// rather than inventing a date.
    #[func]
    fn wave_death_time(&self, kind: i64, dist: f64, speed: f64) -> f64 {
        if !dist.is_finite() || !speed.is_finite() || speed <= 0.0 || dist < 0.0 {
            return f64::NAN;
        }
        dist / speed + render::reveal::reveal_tail(i32::try_from(kind).unwrap_or(i32::MAX))
    }

    /// A sound source's composed acoustic image — `render::reveal::source_image`,
    /// the law `data_xray.gdshader` transliterates.
    ///
    /// The composition is a Rust law with a Godot caller rather than an
    /// expression a suite reconstructs for itself: a test that multiplies
    /// volume by muffle on its own asserts a product the shader may well
    /// have stopped forming.
    #[func]
    fn compose_source_image(&self, wave: f64, volume: f64, muffle: f64) -> f64 {
        render::reveal::source_image(wave, render::reveal::SourceImage { volume, muffle })
    }

    /// The crease knee the hearing pass fades a seam over, `(lo, hi)` —
    /// `render::crease::CreaseKnee::shipped`, derived from `MIN_SEP`.
    ///
    /// Exposed so a suite can hold the pushed uniform against the
    /// derivation rather than against a retyped literal: the allocator and
    /// the renderer are the same law seen from two ends, and they used to
    /// be two numbers that never met.
    #[func]
    fn crease_knee(&self) -> Vector2 {
        let knee = render::crease::CreaseKnee::shipped();
        Vector2::new(knee.lo() as f32, knee.hi() as f32)
    }

    /// The separation the shader's crease knee demands between two labels
    /// that must draw a seam — `render::labels::MIN_SEP`.
    #[func]
    fn min_label_separation(&self) -> f64 {
        render::labels::MIN_SEP
    }

    /// The silhouette knee, in METRES of depth step: `u_sil_knee`, derived by
    /// `render::silhouette` and pushed by the composition root.
    ///
    /// In metres and not in channel units, which is the whole point of the
    /// type: the pair used to be a GLSL literal that silently meant a
    /// different depth step whenever `DIST_PACK_RANGE` moved.
    #[func]
    fn silhouette_knee(&self) -> Vector2 {
        let knee = render::silhouette::SilhouetteKnee::shipped();
        Vector2::new(knee.lo() as f32, knee.hi() as f32)
    }

    /// The bottom of the band camera distance is packed into —
    /// `render::channel::SAFE_FLOOR`.
    ///
    /// Exposed for the reason [`Self::source_band`] is: the shipped GLSL
    /// carries this number too, and a drift between the two would put the
    /// write end and the read end of the channel on different affine maps —
    /// every visible surface at the wrong distance, smoothly and plausibly.
    #[func]
    fn dist_safe_floor(&self) -> f64 {
        render::channel::SAFE_FLOOR
    }

    /// One whole reconstruction gap at the shipped packing range — the bias
    /// `probe_distance` pulls every reconstructed wall-test endpoint toward
    /// the eye by, `render::channel::quantum(DIST_PACK_RANGE)`.
    ///
    /// Exposed so `shader_contract_test` can hold the GLSL twin's
    /// `DIST_RECON_BIAS` derivation against the cargo one: a bias smaller
    /// than the measured gap readmits the false wall crossing that flared a
    /// fan through the wall it hides behind.
    #[func]
    fn dist_recon_bias(&self) -> f64 {
        render::channel::quantum(level_plan::DIST_PACK_RANGE)
    }

    /// The composed reconstruction guard in nominal codes —
    /// `render::channel::guard_codes` at the shipped range. The one number
    /// `tap_error_probe` holds its live in-band error measurement against,
    /// exposed so the probe gates on the derived guard instead of a pasted
    /// literal that goes stale when the bias or the shrink is retuned.
    #[func]
    fn recon_guard_codes(&self) -> f64 {
        render::channel::guard_codes(level_plan::DIST_PACK_RANGE)
    }

    /// The widest gap the data channel was measured to leave, as a fraction
    /// of full scale — `WORST_STEP_CODES` over the nominal step count.
    ///
    /// Exposed for `game/tests/probe/channel_probe.gd`, which writes a pair
    /// of values exactly this far apart and demands the screen texture keep
    /// them distinct. The probe is a PLATFORM gate, so it has to hold the
    /// hardware against the number the renderer derives its tolerance from,
    /// not against a second copy of it that can drift.
    #[func]
    fn channel_worst_step(&self) -> f64 {
        render::channel::WORST_STEP_CODES / f64::from(render::channel::CHANNEL_LEVELS - 1)
    }
}

impl WaveCore {
    /// The pool itself, for READING only — the debug observer decodes it
    /// into an agent-facing snapshot. Deliberately not a `#[func]`: the
    /// Godot-callable surface stays limited to the mirrored compatibility
    /// methods, and nothing outside this crate can reach the slots at all.
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
    pub(crate) fn install_prepared_state(
        &mut self,
        pool: PreparedPulsePool,
        echoes: PreparedEchoQueue,
    ) {
        self.pool = PulsePool::from_prepared(pool);
        self.echoes = EchoQueue::from_prepared(echoes);
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
        let Some(checked) =
            Self::prepare_wave_or_refuse(kind, at, max_r, speed, gain, now, beam_dir, cos_half)
        else {
            return;
        };
        self.pool.install_checked(checked);
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the same mirrored signature as the public emit()"
    )]
    fn prepare_wave_or_refuse(
        kind: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
        beam_dir: Vector3,
        cos_half: f64,
    ) -> Option<CheckedWave> {
        let now = match prepare_time(now) {
            Ok(now) => now,
            Err(error) => {
                godot_error!("Pulses.emit: field now: {} — wave refused", error.rule);
                return None;
            }
        };
        match CheckedWave::prepare(kind, at, max_r, speed, gain, now, beam_dir, cos_half) {
            Ok(checked) => Some(checked),
            Err(error)
                if matches!(error.field(), "speed" | "max_r")
                    && error.rule() == "must be strictly positive" =>
            {
                // These are the original GDScript refusal cases. Keep their
                // established text; newer domains name their exact lane.
                godot_error!("{REFUSAL_MESSAGE}");
                None
            }
            Err(error) => {
                godot_error!(
                    "Pulses.emit: field {}: {} — wave refused",
                    error.field(),
                    error.rule()
                );
                None
            }
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
    request: &CheckedReflectionRequest,
    mut space: Gd<PhysicsDirectSpaceState3D>,
) -> Result<(usize, Vec<RayHit>), &'static str> {
    let origin = request.ray_origin();
    let reach = request.reach_lane();
    let mut rays_cast = 0usize;
    let mut hits: Vec<RayHit> = Vec::with_capacity(ray_fan::RAYS);
    for dir in request.directions() {
        let query =
            PhysicsRayQueryParameters3D::create(origin, origin + dir * reach).ok_or(REFUSED_HIT)?;
        rays_cast += 1;
        let struck = space.intersect_ray(&query);
        if struck.is_empty() {
            continue;
        }
        let (Some(position), Some(normal)) = (
            struck
                .get("position")
                .and_then(|v| v.try_to::<Vector3>().ok()),
            struck
                .get("normal")
                .and_then(|v| v.try_to::<Vector3>().ok()),
        ) else {
            return Err(REFUSED_HIT);
        };
        let hit = CheckedRayHit::prepare(position, normal, request).map_err(|_| REFUSED_HIT)?;
        hits.push(hit.ray_hit());
    }
    Ok((rays_cast, hits))
}
