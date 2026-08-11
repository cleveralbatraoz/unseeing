//! The blob: one instant of the world as a value, its canonical bytes,
//! and the single number that stands for all of them.
//!
//! [`CaptureState`] is a composition and nothing else — every group is a
//! value type some subsystem already produced ([`SlotCapture`],
//! [`PendingEcho`], [`ViewmodelCapture`], [`CatCapture`]'s three
//! sub-captures), gathered at one instant. No law lives here; the law is
//! in the modules that filled it.
//!
//! What DOES live here is the format. [`canonical_bytes`] writes the
//! state as fixed-width little-endian bytes with every float as its BIT
//! PATTERN — `to_bits`, never `to_string`. The distinction is not
//! theoretical: the determinism probe once compared vectors through
//! Godot's pretty-printer, which rounds, and two states that differed by
//! a ULP hashed the same. A hash that can miss a difference is worse
//! than no hash, because it is trusted.
//!
//! The bytes are never parsed back — only hashed and compared — so the
//! format owes nothing to a reader: no tags, no separators, no
//! self-description. It owes exactly two things. That every field
//! reaches the bytes is pinned field by field, by
//! `every_field_reaches_both_walks`. That no two different states reach
//! the same bytes is a property of the LAYOUT rather than of any one
//! test: a u32 length before every variable-length run and a u32
//! discriminant before every enum payload mean no boundary inside the
//! stream can shift, so the same bytes can only have come from the same
//! state.

use godot::builtin::{Vector3, Vector4};

use crate::cat_body::{CatPose, TAIL_N};
use crate::cat_brain::{BrainCapture, BrainState, RoamRect};
use crate::cat_gait::GaitCapture;
use crate::echo_queue::PendingEcho;
use crate::observe::QueuedWave;
use crate::pulse_pool::{MAXP, SlotCapture};
use crate::viewmodel::ViewmodelCapture;

/// FNV-1a, 64-bit. Hand-rolled because the hash must be identical on
/// every run and every platform of the same build — std's DefaultHasher
/// is neither. Not cryptographic and not meant to be: it detects drift,
/// not adversaries.
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The world around the simulation: the clock every subsystem reads, and
/// the two mood machines the composition root owns in GDScript — the
/// dev demo tap's schedule and the flicker envelope, RNG word included.
///
/// The flicker is not decoration for this purpose: it draws from its own
/// stream every frame, so a restored world whose flicker RNG is one draw
/// behind diverges from the original on the very next frame.
#[derive(Debug, Clone, Copy)]
pub struct EnvCapture {
    /// The simulated clock — the absolute `now` every pulse, cadence and
    /// appointment in the blob is dated against.
    pub now: f64,
    /// Whether the demo arming check has already run (it runs once, at
    /// 0.5 s), and what it decided.
    pub demo_checked: bool,
    pub demo_armed: bool,
    /// The demo tap's next appointment.
    pub demo_next: f64,
    /// The flicker envelope: its own elapsed clock, the current level,
    /// the instant the running dropout ends, and the countdown to the
    /// next one.
    pub flicker_t: f64,
    pub flicker_level: f64,
    pub flicker_drop_until: f64,
    pub flicker_next_drop: f64,
    /// Godot's `RandomNumberGenerator.state`, carried verbatim as the
    /// 64-bit int the Variant boundary gives us. Never arithmetic, only
    /// storage — the bit pattern is the whole value.
    pub flicker_rng_state: i64,
}

/// The hero as data: the body, the eye, the cane's clocks, the waves
/// already asked for but not yet emitted, and the viewmodel's own state
/// machine.
///
/// `queued_waves` matters as much as the pool does: a wave requested
/// this frame and emitted on the physics tick is a sound that WILL
/// happen. Dropping the out-tray would silently swallow it.
#[derive(Debug, Clone)]
pub struct HeroCapture {
    pub position: Vector3,
    pub velocity: Vector3,
    /// Body yaw and eye pitch, radians, as the look law last left them.
    pub yaw: f64,
    pub pitch: f64,
    /// The clock reading of the last ACCEPTED tap, and where it landed —
    /// the cane's cooldown and the viewmodel's strike animation both
    /// read them.
    pub last_tap: f64,
    pub tap_target: Vector3,
    pub tap_queued: bool,
    /// Waves requested and still waiting for the physics tick.
    pub queued_waves: Vec<QueuedWave>,
    /// The viewmodel's whole state — footstep clock included.
    pub viewmodel: ViewmodelCapture,
}

/// One world sound source's appointment book. The name is the identity
/// the restore matches against (sources are found by name in the scene,
/// never by index), and `next_emit` is the only mutable state a source
/// carries — everything else is designer-authored and already in the
/// scene.
#[derive(Debug, Clone)]
pub struct SourceCapture {
    pub name: String,
    pub next_emit: f64,
}

/// The cat as data — a whole life in one value, so a whim mid-stride
/// moves whole or not at all.
#[derive(Debug, Clone, Copy)]
pub struct CatCapture {
    /// The body's transform and momentum — world position, world-space
    /// yaw, and the CharacterBody3D velocity move_and_slide reads next.
    pub position: Vector3,
    pub yaw: f64,
    pub velocity: Vector3,
    /// The mind's whole state — RNG words included, or the restored cat
    /// diverges at its first whim.
    pub brain: BrainCapture,
    /// The stride's whole state — every planted paw and swing aim.
    pub gait: GaitCapture,
    /// The tail's exact curve — a settled cat and a mid-sway cat are
    /// different tails, and only the exact one is the truth.
    pub tail: [Vector3; TAIL_N],
    /// The last-built skeleton pose — carried verbatim so `paw_positions`
    /// and `mood` answer correctly between the restore and the first tick.
    pub pose: CatPose,
    /// The idle-presence cadence's next appointment, or NaN when the cat
    /// never beat (see `WaveCat::restore_state`, which round-trips that
    /// NaN back into a cadence with no appointment).
    pub presence_next: f64,
    /// The eased sit blend and the elapsed sim clock the tail's idle
    /// breath rides.
    pub sit: f64,
    pub sim_t: f64,
    /// The body position at the start of the last physics tick — the
    /// brain's honest-progress feed depends on this exact value.
    pub last_pos: Vector3,
}

/// One instant of the running world, whole.
///
/// ALL-OR-NOTHING: a capture that cannot answer for one subsystem is a
/// refusal at the boundary, never a `CaptureState` with an empty group.
/// A blob missing its cats and a blob whose cats had all gone home look
/// identical from here, and only one of them is the truth.
///
/// Deliberately NOT `PartialEq`: two states are compared by
/// [`state_hash`] and [`first_divergence`], which compare float BIT
/// PATTERNS. A derived `==` would disagree with both, in both
/// directions — `NaN != NaN` would report an unchanged cat cadence as a
/// divergence, and `-0.0 == 0.0` would report a flipped sign as no
/// change at all.
#[derive(Debug, Clone)]
pub struct CaptureState {
    /// [`super::FORMAT_VERSION`] as of the capture, and the first field
    /// hashed.
    pub format_version: u32,
    /// The level scene the instant belongs to — restoring into a
    /// different map is a refusal, not a divergence.
    pub level_scene: String,
    pub env: EnvCapture,
    /// All 64 pool slots, live and dead alike: a dead slot's bytes are
    /// part of the state, because a restore that left a stale pulse
    /// behind must not hash the same as one that did not.
    pub slots: Box<[SlotCapture; MAXP]>,
    pub echoes: Vec<PendingEcho>,
    pub sources: Vec<SourceCapture>,
    pub hero: HeroCapture,
    pub cats: Vec<CatCapture>,
}

/// The canonical bytes of a state: fixed width, little-endian, floats as
/// bits. Same build, same platform, same state → same bytes, always.
#[must_use]
pub fn canonical_bytes(state: &CaptureState) -> Vec<u8> {
    let mut enc = Enc::default();
    encode(state, &mut enc);
    enc.out
}

/// The one number that stands for the whole instant.
#[must_use]
pub fn state_hash(state: &CaptureState) -> u64 {
    fnv1a64(&canonical_bytes(state))
}

/// The byte writer. Every method is total and fixed-width; none of them
/// can fail, so the encoding walk has no error path to get wrong.
#[derive(Default)]
struct Enc {
    out: Vec<u8>,
}

impl Enc {
    fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    fn i32(&mut self, v: i32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    fn u64(&mut self, v: u64) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    fn i64(&mut self, v: i64) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }

    /// A float as its BITS, never its decimal text — the whole point of
    /// this module. `to_string` rounds, and a rounded hash agrees with
    /// states it should have caught.
    fn f64(&mut self, v: f64) {
        self.u64(v.to_bits());
    }

    /// Vector lanes are f32 (godot's `Vector3`/`Vector4` are
    /// single-precision), and are hashed at their real width — widening
    /// to f64 first would be a second representation to keep in sync.
    fn f32(&mut self, v: f32) {
        self.u32(v.to_bits());
    }

    fn bool(&mut self, v: bool) {
        self.out.push(u8::from(v));
    }

    /// A string as its byte length then its UTF-8 — so "ab" + "c" and
    /// "a" + "bc" cannot encode alike.
    fn str(&mut self, v: &str) {
        self.len(v.len());
        self.out.extend_from_slice(v.as_bytes());
    }

    /// Any variable-length run's element count. Saturating rather than
    /// panicking keeps the writer total; the real counts here are the
    /// pool's 64, a handful of echoes, and single-digit source and cat
    /// lists.
    fn len(&mut self, n: usize) {
        self.u32(u32::try_from(n).unwrap_or(u32::MAX));
    }

    fn v3(&mut self, v: Vector3) {
        self.f32(v.x);
        self.f32(v.y);
        self.f32(v.z);
    }

    fn v4(&mut self, v: Vector4) {
        self.f32(v.x);
        self.f32(v.y);
        self.f32(v.z);
        self.f32(v.w);
    }
}

// ─────────────────────────────────────────────────────────────────────
// THE FIELD LIST, WALKED TWICE
//
// `encode` below and `first_divergence` under it are twin walks over the
// SAME fields in the SAME order: one turns them into bytes, the other
// names the first one that differs. They are kept adjacent because they
// are one contract wearing two faces.
//
// A field added to `CaptureState` must be added to BOTH walks. In the
// encoder alone it would be hashed but never named, so a mismatch would
// report the wrong field or none; in the divergence walk alone it would
// be named but never hashed, so a restore that got it wrong would pass
// the hash gate. `every_field_reaches_both_walks` is the net that makes
// either omission a red test, with `identical_states_agree_completely`,
// `one_ulp_in_one_slot_changes_the_hash`, and the deliberate-break test
// in the restore gate behind it.
// ─────────────────────────────────────────────────────────────────────

fn encode(state: &CaptureState, enc: &mut Enc) {
    enc.u32(state.format_version);
    enc.str(&state.level_scene);
    encode_env(&state.env, enc);
    // fixed 64 by the pool's own contract, so no length prefix
    for slot in state.slots.iter() {
        encode_slot(slot, enc);
    }
    enc.len(state.echoes.len());
    for echo in &state.echoes {
        enc.f64(echo.at_t);
        enc.v3(echo.pos);
        enc.f64(echo.gain);
    }
    enc.len(state.sources.len());
    for source in &state.sources {
        enc.str(&source.name);
        enc.f64(source.next_emit);
    }
    encode_hero(&state.hero, enc);
    enc.len(state.cats.len());
    for cat in &state.cats {
        encode_cat(cat, enc);
    }
}

fn encode_env(env: &EnvCapture, enc: &mut Enc) {
    enc.f64(env.now);
    enc.bool(env.demo_checked);
    enc.bool(env.demo_armed);
    enc.f64(env.demo_next);
    enc.f64(env.flicker_t);
    enc.f64(env.flicker_level);
    enc.f64(env.flicker_drop_until);
    enc.f64(env.flicker_next_drop);
    enc.i64(env.flicker_rng_state);
}

fn encode_slot(slot: &SlotCapture, enc: &mut Enc) {
    enc.v3(slot.pos);
    enc.v4(slot.dat);
    enc.v4(slot.dir);
    enc.f64(slot.t0);
    enc.f64(slot.end);
    enc.i32(slot.kind);
}

fn encode_hero(hero: &HeroCapture, enc: &mut Enc) {
    enc.v3(hero.position);
    enc.v3(hero.velocity);
    enc.f64(hero.yaw);
    enc.f64(hero.pitch);
    enc.f64(hero.last_tap);
    enc.v3(hero.tap_target);
    enc.bool(hero.tap_queued);
    enc.len(hero.queued_waves.len());
    for wave in &hero.queued_waves {
        enc.i64(wave.kind);
        enc.v3(wave.at);
        enc.f64(wave.max_r);
        enc.f64(wave.speed);
        enc.f64(wave.gain);
        enc.i64(wave.echoes);
        enc.v3(wave.normal);
    }
    let vm = &hero.viewmodel;
    enc.f64(vm.walk_amp);
    enc.f64(vm.leg_phase);
    enc.f64(vm.swing_phase);
    enc.f64(vm.cane_swing);
    enc.f64(vm.sway_x);
    enc.f64(vm.sway_y);
    enc.f64(vm.last_yaw);
    enc.f64(vm.last_pitch);
    enc.f64(vm.step_t);
    enc.i32(vm.step_side);
}

fn encode_cat(cat: &CatCapture, enc: &mut Enc) {
    enc.v3(cat.position);
    enc.f64(cat.yaw);
    enc.v3(cat.velocity);
    encode_brain(&cat.brain, enc);
    encode_gait(&cat.gait, enc);
    for node in &cat.tail {
        enc.v3(*node);
    }
    encode_pose(&cat.pose, enc);
    enc.f64(cat.presence_next);
    enc.f64(cat.sit);
    enc.f64(cat.sim_t);
    enc.v3(cat.last_pos);
}

fn encode_brain(brain: &BrainCapture, enc: &mut Enc) {
    enc.u64(brain.rng_state);
    enc.u64(brain.rng_inc);
    encode_rect(&brain.rect, enc);
    encode_brain_state(brain.state, enc);
    enc.f64(brain.yaw);
    enc.f64(brain.speed);
    enc.f64(brain.blocked);
}

fn encode_rect(rect: &RoamRect, enc: &mut Enc) {
    enc.f64(rect.min_x);
    enc.f64(rect.min_z);
    enc.f64(rect.max_x);
    enc.f64(rect.max_z);
}

/// The brain's state machine as bytes: a u32 discriminant then that
/// variant's payload — **Roam = 0** (tx, tz), **Pause = 1** (left),
/// **Sit = 2** (left). The discriminant comes first and fixes the
/// payload's width, so no two variants can encode alike (a paused cat
/// and a sitting cat with the same countdown are different cats).
///
/// The numbering is part of the canonical format: renumbering it is a
/// [`super::FORMAT_VERSION`] bump, not a refactor.
fn encode_brain_state(state: BrainState, enc: &mut Enc) {
    match state {
        BrainState::Roam { tx, tz } => {
            enc.u32(0);
            enc.f64(tx);
            enc.f64(tz);
        }
        BrainState::Pause { left } => {
            enc.u32(1);
            enc.f64(left);
        }
        BrainState::Sit { left } => {
            enc.u32(2);
            enc.f64(left);
        }
    }
}

fn encode_gait(gait: &GaitCapture, enc: &mut Enc) {
    enc.f64(gait.phase);
    enc.f64(gait.amp);
    for paw in &gait.planted {
        enc.v3(*paw);
    }
    for aim in &gait.aim {
        enc.v3(*aim);
    }
    for swinging in &gait.in_swing {
        enc.bool(*swinging);
    }
    enc.bool(gait.moving);
}

fn encode_pose(pose: &CatPose, enc: &mut Enc) {
    enc.v3(pose.pos);
    enc.f64(pose.yaw);
    for paw in &pose.paws {
        enc.v3(*paw);
    }
    enc.f64(pose.bob);
    enc.f64(pose.amp);
    enc.f64(pose.sit);
}

/// Where two states part, as a dotted field path — `"slots[12].t0"`,
/// `"cats[0].brain.rng_state"` — or `None` when they are the same
/// instant. The first mismatch in the encoder's own walk order, so the
/// answer is the earliest thing that went wrong, not an arbitrary one.
///
/// A mismatched hash without this is a shrug; with it, it is a bug
/// report.
#[must_use]
pub fn first_divergence(a: &CaptureState, b: &CaptureState) -> Option<String> {
    if a.format_version != b.format_version {
        return Some("format_version".to_string());
    }
    if a.level_scene != b.level_scene {
        return Some("level_scene".to_string());
    }
    if let Some(field) = diff_env(&a.env, &b.env) {
        return Some(format!("env.{field}"));
    }
    for (i, (sa, sb)) in a.slots.iter().zip(b.slots.iter()).enumerate() {
        if let Some(field) = diff_slot(sa, sb) {
            return Some(format!("slots[{i}].{field}"));
        }
    }
    if a.echoes.len() != b.echoes.len() {
        return Some("echoes.len".to_string());
    }
    for (i, (ea, eb)) in a.echoes.iter().zip(&b.echoes).enumerate() {
        if !same_f64(ea.at_t, eb.at_t) {
            return Some(format!("echoes[{i}].at_t"));
        }
        if let Some(c) = diff_v3(ea.pos, eb.pos) {
            return Some(format!("echoes[{i}].pos.{c}"));
        }
        if !same_f64(ea.gain, eb.gain) {
            return Some(format!("echoes[{i}].gain"));
        }
    }
    if a.sources.len() != b.sources.len() {
        return Some("sources.len".to_string());
    }
    for (i, (sa, sb)) in a.sources.iter().zip(&b.sources).enumerate() {
        if let Some(field) = first_mismatch(&[
            ("name", sa.name == sb.name),
            ("next_emit", same_f64(sa.next_emit, sb.next_emit)),
        ]) {
            return Some(format!("sources[{i}].{field}"));
        }
    }
    if let Some(field) = diff_hero(&a.hero, &b.hero) {
        return Some(format!("hero.{field}"));
    }
    if a.cats.len() != b.cats.len() {
        return Some("cats.len".to_string());
    }
    for (i, (ca, cb)) in a.cats.iter().zip(&b.cats).enumerate() {
        if let Some(field) = diff_cat(ca, cb) {
            return Some(format!("cats[{i}].{field}"));
        }
    }
    None
}

/// Two floats are the same when their BITS are the same. That is
/// stricter than `==` on purpose, in both directions: NaN equals itself
/// (a cat that never beat carries one, and it is not a divergence), and
/// −0.0 differs from 0.0 (a sign that flipped IS a different world).
fn same_f64(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

fn same_f32(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

/// The first named check that reports a mismatch, in walk order — the
/// flat-field idiom, so a field list reads as a field list.
fn first_mismatch(fields: &[(&'static str, bool)]) -> Option<&'static str> {
    fields
        .iter()
        .find(|(_, same)| !*same)
        .map(|(name, _)| *name)
}

fn diff_v3(a: Vector3, b: Vector3) -> Option<&'static str> {
    first_mismatch(&[
        ("x", same_f32(a.x, b.x)),
        ("y", same_f32(a.y, b.y)),
        ("z", same_f32(a.z, b.z)),
    ])
}

fn diff_v4(a: Vector4, b: Vector4) -> Option<&'static str> {
    first_mismatch(&[
        ("x", same_f32(a.x, b.x)),
        ("y", same_f32(a.y, b.y)),
        ("z", same_f32(a.z, b.z)),
        ("w", same_f32(a.w, b.w)),
    ])
}

fn diff_env(a: &EnvCapture, b: &EnvCapture) -> Option<&'static str> {
    first_mismatch(&[
        ("now", same_f64(a.now, b.now)),
        ("demo_checked", a.demo_checked == b.demo_checked),
        ("demo_armed", a.demo_armed == b.demo_armed),
        ("demo_next", same_f64(a.demo_next, b.demo_next)),
        ("flicker_t", same_f64(a.flicker_t, b.flicker_t)),
        ("flicker_level", same_f64(a.flicker_level, b.flicker_level)),
        (
            "flicker_drop_until",
            same_f64(a.flicker_drop_until, b.flicker_drop_until),
        ),
        (
            "flicker_next_drop",
            same_f64(a.flicker_next_drop, b.flicker_next_drop),
        ),
        (
            "flicker_rng_state",
            a.flicker_rng_state == b.flicker_rng_state,
        ),
    ])
}

fn diff_slot(a: &SlotCapture, b: &SlotCapture) -> Option<String> {
    if let Some(c) = diff_v3(a.pos, b.pos) {
        return Some(format!("pos.{c}"));
    }
    if let Some(c) = diff_v4(a.dat, b.dat) {
        return Some(format!("dat.{c}"));
    }
    if let Some(c) = diff_v4(a.dir, b.dir) {
        return Some(format!("dir.{c}"));
    }
    first_mismatch(&[
        ("t0", same_f64(a.t0, b.t0)),
        ("end", same_f64(a.end, b.end)),
        ("kind", a.kind == b.kind),
    ])
    .map(String::from)
}

fn diff_hero(a: &HeroCapture, b: &HeroCapture) -> Option<String> {
    if let Some(c) = diff_v3(a.position, b.position) {
        return Some(format!("position.{c}"));
    }
    if let Some(c) = diff_v3(a.velocity, b.velocity) {
        return Some(format!("velocity.{c}"));
    }
    if let Some(field) = first_mismatch(&[
        ("yaw", same_f64(a.yaw, b.yaw)),
        ("pitch", same_f64(a.pitch, b.pitch)),
        ("last_tap", same_f64(a.last_tap, b.last_tap)),
    ]) {
        return Some(field.to_string());
    }
    if let Some(c) = diff_v3(a.tap_target, b.tap_target) {
        return Some(format!("tap_target.{c}"));
    }
    if a.tap_queued != b.tap_queued {
        return Some("tap_queued".to_string());
    }
    if a.queued_waves.len() != b.queued_waves.len() {
        return Some("queued_waves.len".to_string());
    }
    for (i, (wa, wb)) in a.queued_waves.iter().zip(&b.queued_waves).enumerate() {
        if let Some(field) = diff_wave(wa, wb) {
            return Some(format!("queued_waves[{i}].{field}"));
        }
    }
    diff_viewmodel(&a.viewmodel, &b.viewmodel).map(|field| format!("viewmodel.{field}"))
}

fn diff_wave(a: &QueuedWave, b: &QueuedWave) -> Option<String> {
    if a.kind != b.kind {
        return Some("kind".to_string());
    }
    if let Some(c) = diff_v3(a.at, b.at) {
        return Some(format!("at.{c}"));
    }
    if let Some(field) = first_mismatch(&[
        ("max_r", same_f64(a.max_r, b.max_r)),
        ("speed", same_f64(a.speed, b.speed)),
        ("gain", same_f64(a.gain, b.gain)),
        ("echoes", a.echoes == b.echoes),
    ]) {
        return Some(field.to_string());
    }
    diff_v3(a.normal, b.normal).map(|c| format!("normal.{c}"))
}

fn diff_viewmodel(a: &ViewmodelCapture, b: &ViewmodelCapture) -> Option<&'static str> {
    first_mismatch(&[
        ("walk_amp", same_f64(a.walk_amp, b.walk_amp)),
        ("leg_phase", same_f64(a.leg_phase, b.leg_phase)),
        ("swing_phase", same_f64(a.swing_phase, b.swing_phase)),
        ("cane_swing", same_f64(a.cane_swing, b.cane_swing)),
        ("sway_x", same_f64(a.sway_x, b.sway_x)),
        ("sway_y", same_f64(a.sway_y, b.sway_y)),
        ("last_yaw", same_f64(a.last_yaw, b.last_yaw)),
        ("last_pitch", same_f64(a.last_pitch, b.last_pitch)),
        ("step_t", same_f64(a.step_t, b.step_t)),
        ("step_side", a.step_side == b.step_side),
    ])
}

fn diff_cat(a: &CatCapture, b: &CatCapture) -> Option<String> {
    if let Some(c) = diff_v3(a.position, b.position) {
        return Some(format!("position.{c}"));
    }
    if !same_f64(a.yaw, b.yaw) {
        return Some("yaw".to_string());
    }
    if let Some(c) = diff_v3(a.velocity, b.velocity) {
        return Some(format!("velocity.{c}"));
    }
    if let Some(field) = diff_brain(&a.brain, &b.brain) {
        return Some(format!("brain.{field}"));
    }
    if let Some(field) = diff_gait(&a.gait, &b.gait) {
        return Some(format!("gait.{field}"));
    }
    for (i, (na, nb)) in a.tail.iter().zip(&b.tail).enumerate() {
        if let Some(c) = diff_v3(*na, *nb) {
            return Some(format!("tail[{i}].{c}"));
        }
    }
    if let Some(field) = diff_pose(&a.pose, &b.pose) {
        return Some(format!("pose.{field}"));
    }
    if let Some(field) = first_mismatch(&[
        ("presence_next", same_f64(a.presence_next, b.presence_next)),
        ("sit", same_f64(a.sit, b.sit)),
        ("sim_t", same_f64(a.sim_t, b.sim_t)),
    ]) {
        return Some(field.to_string());
    }
    diff_v3(a.last_pos, b.last_pos).map(|c| format!("last_pos.{c}"))
}

fn diff_brain(a: &BrainCapture, b: &BrainCapture) -> Option<String> {
    if a.rng_state != b.rng_state {
        return Some("rng_state".to_string());
    }
    if a.rng_inc != b.rng_inc {
        return Some("rng_inc".to_string());
    }
    if let Some(field) = first_mismatch(&[
        ("min_x", same_f64(a.rect.min_x, b.rect.min_x)),
        ("min_z", same_f64(a.rect.min_z, b.rect.min_z)),
        ("max_x", same_f64(a.rect.max_x, b.rect.max_x)),
        ("max_z", same_f64(a.rect.max_z, b.rect.max_z)),
    ]) {
        return Some(format!("rect.{field}"));
    }
    if let Some(field) = diff_brain_state(a.state, b.state) {
        return Some(field);
    }
    first_mismatch(&[
        ("yaw", same_f64(a.yaw, b.yaw)),
        ("speed", same_f64(a.speed, b.speed)),
        ("blocked", same_f64(a.blocked, b.blocked)),
    ])
    .map(String::from)
}

/// A different variant is named as the state itself (`"state"`); the
/// same variant with a different countdown or target is named down to
/// the payload (`"state.left"`, `"state.tx"`).
fn diff_brain_state(a: BrainState, b: BrainState) -> Option<String> {
    match (a, b) {
        (BrainState::Roam { tx: ax, tz: az }, BrainState::Roam { tx: bx, tz: bz }) => {
            first_mismatch(&[
                ("state.tx", same_f64(ax, bx)),
                ("state.tz", same_f64(az, bz)),
            ])
            .map(String::from)
        }
        (BrainState::Pause { left: al }, BrainState::Pause { left: bl })
        | (BrainState::Sit { left: al }, BrainState::Sit { left: bl }) => {
            (!same_f64(al, bl)).then(|| "state.left".to_string())
        }
        _ => Some("state".to_string()),
    }
}

fn diff_gait(a: &GaitCapture, b: &GaitCapture) -> Option<String> {
    if let Some(field) = first_mismatch(&[
        ("phase", same_f64(a.phase, b.phase)),
        ("amp", same_f64(a.amp, b.amp)),
    ]) {
        return Some(field.to_string());
    }
    for (i, (pa, pb)) in a.planted.iter().zip(&b.planted).enumerate() {
        if let Some(c) = diff_v3(*pa, *pb) {
            return Some(format!("planted[{i}].{c}"));
        }
    }
    for (i, (pa, pb)) in a.aim.iter().zip(&b.aim).enumerate() {
        if let Some(c) = diff_v3(*pa, *pb) {
            return Some(format!("aim[{i}].{c}"));
        }
    }
    for (i, (sa, sb)) in a.in_swing.iter().zip(&b.in_swing).enumerate() {
        if sa != sb {
            return Some(format!("in_swing[{i}]"));
        }
    }
    (a.moving != b.moving).then(|| "moving".to_string())
}

fn diff_pose(a: &CatPose, b: &CatPose) -> Option<String> {
    if let Some(c) = diff_v3(a.pos, b.pos) {
        return Some(format!("pos.{c}"));
    }
    if !same_f64(a.yaw, b.yaw) {
        return Some("yaw".to_string());
    }
    for (i, (pa, pb)) in a.paws.iter().zip(&b.paws).enumerate() {
        if let Some(c) = diff_v3(*pa, *pb) {
            return Some(format!("paws[{i}].{c}"));
        }
    }
    first_mismatch(&[
        ("bob", same_f64(a.bob, b.bob)),
        ("amp", same_f64(a.amp, b.amp)),
        ("sit", same_f64(a.sit, b.sit)),
    ])
    .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reproduce::FORMAT_VERSION;

    /// One cat's whole life, every literal shifted by `k` so two cats in
    /// the same blob are never confusable and a transposed pair fails
    /// loudly. These are encoder fixtures, not physically constrained
    /// poses — the blob carries bits verbatim and validates nothing.
    fn test_cat(k: f64, rng_state: u64, state: BrainState, presence_next: f64) -> CatCapture {
        let f = k as f32;
        CatCapture {
            position: Vector3::new(1.25 + f, 0.5 + f, -2.5 + f),
            yaw: 0.625 + k,
            velocity: Vector3::new(-0.375 + f, 0.25 + f, 1.125 + f),
            brain: BrainCapture {
                rng_state,
                rng_inc: rng_state ^ 0x00ca_7000_0000_0ca7,
                rect: RoamRect {
                    min_x: -3.5 + k,
                    min_z: -4.25 + k,
                    max_x: 5.75 + k,
                    max_z: 6.125 + k,
                },
                state,
                yaw: 0.875 + k,
                speed: 0.4375 + k,
                blocked: 0.3125 + k,
            },
            gait: GaitCapture {
                phase: 0.28125 + k,
                amp: 0.75 + k,
                planted: [
                    Vector3::new(1.5 + f, 0.125 + f, 2.75 + f),
                    Vector3::new(3.25 + f, 0.1875 + f, 4.5 + f),
                    Vector3::new(-5.75 + f, 0.21875 + f, 6.375 + f),
                    Vector3::new(7.125 + f, 0.25 + f, -8.5 + f),
                ],
                aim: [
                    Vector3::new(9.25 + f, 0.3125 + f, 10.5 + f),
                    Vector3::new(-11.75 + f, 0.375 + f, 12.125 + f),
                    Vector3::new(13.5 + f, 0.4375 + f, -14.25 + f),
                    Vector3::new(15.625 + f, 0.5 + f, 16.75 + f),
                ],
                in_swing: [true, false, false, true],
                moving: true,
            },
            tail: [
                Vector3::new(0.75 + f, 0.5625 + f, -1.25 + f),
                Vector3::new(1.875 + f, 0.625 + f, -2.375 + f),
                Vector3::new(2.5 + f, 0.6875 + f, -3.125 + f),
                Vector3::new(3.625 + f, 0.75 + f, -4.875 + f),
                Vector3::new(4.25 + f, 0.8125 + f, -5.5 + f),
            ],
            pose: CatPose {
                pos: Vector3::new(17.5 + f, 0.875 + f, -18.25 + f),
                yaw: 0.9375 + k,
                paws: [
                    Vector3::new(19.125 + f, 0.03125 + f, 20.5 + f),
                    Vector3::new(-21.75 + f, 0.0625 + f, 22.25 + f),
                    Vector3::new(23.375 + f, 0.09375 + f, -24.5 + f),
                    Vector3::new(25.625 + f, 0.15625 + f, 26.125 + f),
                ],
                bob: 0.046875 + k,
                amp: 0.6875 + k,
                sit: 0.125 + k,
            },
            presence_next,
            sit: 0.34375 + k,
            sim_t: 27.5 + k,
            last_pos: Vector3::new(28.25 + f, 0.40625 + f, -29.75 + f),
        }
    }

    /// A dead pool slot, as [`crate::pulse_pool::PulsePool::new`] leaves
    /// one: the `-1` birth-time sentinel in `dat.x` and the `-1` end.
    fn dead_slot() -> SlotCapture {
        SlotCapture {
            pos: Vector3::ZERO,
            dat: Vector4::new(-1.0, 0.0, 0.0, 0.0),
            dir: Vector4::ZERO,
            t0: 0.0,
            end: -1.0,
            kind: 0,
        }
    }

    /// A state with every group populated and every field a DISTINCT
    /// non-default literal. Distinctness is the point: two fields that
    /// happen to share a value hide both an omission (the hash still
    /// moves when the twin moves) and a transposition (yaw written into
    /// pitch). Every `Vec` carries two elements, so a walk that reads
    /// only the first is caught; the live pool slots sit at 12 and 63,
    /// so a walk that reads only slot 0 — or stops one short — is caught
    /// too; and the first cat's presence cadence is NaN, the shipped
    /// "never beat" value, so the bit comparison is exercised by the
    /// fixture itself.
    fn test_state() -> CaptureState {
        let mut slots = Box::new([dead_slot(); MAXP]);
        slots[12] = SlotCapture {
            pos: Vector3::new(1.5, 2.25, -3.75),
            dat: Vector4::new(4.5, 5.125, 6.25, 7.375),
            dir: Vector4::new(-0.25, 0.5, -0.75, 8.625),
            t0: 9.5,
            end: 10.75,
            kind: 2,
        };
        slots[63] = SlotCapture {
            pos: Vector3::new(-11.5, 12.25, 13.125),
            dat: Vector4::new(14.5, 15.25, 16.125, 17.0),
            dir: Vector4::new(0.125, -0.25, 0.375, 18.5),
            t0: 19.25,
            end: 20.75,
            kind: 1,
        };
        CaptureState {
            format_version: FORMAT_VERSION,
            level_scene: "res://levels/level_01.tscn".to_string(),
            env: EnvCapture {
                now: 12.5,
                demo_checked: true,
                demo_armed: false,
                demo_next: 16.5,
                flicker_t: 12.375,
                flicker_level: 0.875,
                flicker_drop_until: 13.25,
                flicker_next_drop: 6.125,
                flicker_rng_state: 0x0123_4567_89ab_cdef,
            },
            slots,
            echoes: vec![
                PendingEcho {
                    at_t: 21.5,
                    pos: Vector3::new(-22.25, 23.125, 24.75),
                    gain: 0.6875,
                },
                PendingEcho {
                    at_t: 25.5,
                    pos: Vector3::new(26.25, -27.125, 28.75),
                    gain: 0.4375,
                },
            ],
            sources: vec![
                SourceCapture {
                    name: "Fan".to_string(),
                    next_emit: 29.5,
                },
                SourceCapture {
                    name: "Radio".to_string(),
                    next_emit: 30.25,
                },
            ],
            hero: HeroCapture {
                position: Vector3::new(3.5, 1.25, -4.75),
                velocity: Vector3::new(-0.75, 0.375, 2.5),
                yaw: 1.125,
                pitch: -0.375,
                last_tap: 31.5,
                tap_target: Vector3::new(5.25, 1.75, -6.125),
                tap_queued: true,
                queued_waves: vec![
                    QueuedWave {
                        kind: 0,
                        at: Vector3::new(7.5, 1.5, -8.25),
                        max_r: 6.0,
                        speed: 5.5,
                        gain: 1.0,
                        echoes: 6,
                        normal: Vector3::new(0.0, 0.0, 1.0),
                    },
                    QueuedWave {
                        kind: 2,
                        at: Vector3::new(-9.75, 0.625, 10.125),
                        max_r: 2.2,
                        speed: 4.25,
                        gain: 0.5625,
                        echoes: 3,
                        normal: Vector3::new(1.0, 0.0, 0.0),
                    },
                ],
                viewmodel: ViewmodelCapture {
                    walk_amp: 0.375,
                    leg_phase: 1.125,
                    swing_phase: 2.25,
                    cane_swing: -0.5,
                    sway_x: 0.0625,
                    sway_y: -0.03125,
                    last_yaw: 1.75,
                    last_pitch: -0.25,
                    step_t: 0.140625,
                    step_side: -1,
                },
            },
            cats: vec![
                test_cat(
                    0.0,
                    0x1234_5678_9abc_def0,
                    BrainState::Roam {
                        tx: 4.25,
                        tz: -5.75,
                    },
                    f64::NAN,
                ),
                test_cat(
                    1.0,
                    0x0fed_cba9_8765_4321,
                    BrainState::Pause { left: 0.875 },
                    32.5,
                ),
            ],
        }
    }

    /// One leaf field's path and a perturbation that touches that field
    /// and nothing else.
    type Mutation = (&'static str, fn(&mut CaptureState));

    /// Every leaf field of the state, each with a perturbation that
    /// touches it and nothing else, and the path the divergence walk
    /// must name for it. Hand-derived from the capture structs'
    /// definitions — the contract — never read back off the encoder.
    fn mutations() -> Vec<Mutation> {
        vec![
            ("format_version", |s| s.format_version += 7),
            ("level_scene", |s| s.level_scene.push('x')),
            // env
            ("env.now", |s| s.env.now += 1.0),
            ("env.demo_checked", |s| s.env.demo_checked = false),
            ("env.demo_armed", |s| s.env.demo_armed = true),
            ("env.demo_next", |s| s.env.demo_next += 1.0),
            ("env.flicker_t", |s| s.env.flicker_t += 1.0),
            ("env.flicker_level", |s| s.env.flicker_level += 0.5),
            ("env.flicker_drop_until", |s| {
                s.env.flicker_drop_until += 1.0
            }),
            ("env.flicker_next_drop", |s| s.env.flicker_next_drop += 1.0),
            ("env.flicker_rng_state", |s| s.env.flicker_rng_state ^= 1),
            // slots
            ("slots[0].t0", |s| s.slots[0].t0 += 3.0),
            ("slots[12].pos.x", |s| s.slots[12].pos.x += 1.0),
            ("slots[12].pos.y", |s| s.slots[12].pos.y += 1.0),
            ("slots[12].pos.z", |s| s.slots[12].pos.z += 1.0),
            ("slots[12].dat.x", |s| s.slots[12].dat.x += 1.0),
            ("slots[12].dat.y", |s| s.slots[12].dat.y += 1.0),
            ("slots[12].dat.z", |s| s.slots[12].dat.z += 1.0),
            ("slots[12].dat.w", |s| s.slots[12].dat.w += 1.0),
            ("slots[12].dir.x", |s| s.slots[12].dir.x += 1.0),
            ("slots[12].dir.y", |s| s.slots[12].dir.y += 1.0),
            ("slots[12].dir.z", |s| s.slots[12].dir.z += 1.0),
            ("slots[12].dir.w", |s| s.slots[12].dir.w += 1.0),
            ("slots[12].t0", |s| s.slots[12].t0 += 1.0),
            ("slots[12].end", |s| s.slots[12].end += 1.0),
            ("slots[12].kind", |s| s.slots[12].kind += 1),
            ("slots[63].pos.z", |s| s.slots[63].pos.z += 1.0),
            ("slots[63].kind", |s| s.slots[63].kind += 1),
            // echoes
            ("echoes.len", |s| {
                s.echoes.pop();
            }),
            ("echoes[0].at_t", |s| s.echoes[0].at_t += 1.0),
            ("echoes[0].pos.x", |s| s.echoes[0].pos.x += 1.0),
            ("echoes[0].pos.y", |s| s.echoes[0].pos.y += 1.0),
            ("echoes[0].pos.z", |s| s.echoes[0].pos.z += 1.0),
            ("echoes[0].gain", |s| s.echoes[0].gain += 0.25),
            ("echoes[1].gain", |s| s.echoes[1].gain += 0.25),
            // sources
            ("sources.len", |s| {
                s.sources.pop();
            }),
            ("sources[0].name", |s| s.sources[0].name.push('x')),
            ("sources[0].next_emit", |s| s.sources[0].next_emit += 1.0),
            ("sources[1].next_emit", |s| s.sources[1].next_emit += 1.0),
            // hero
            ("hero.position.x", |s| s.hero.position.x += 1.0),
            ("hero.position.y", |s| s.hero.position.y += 1.0),
            ("hero.position.z", |s| s.hero.position.z += 1.0),
            ("hero.velocity.x", |s| s.hero.velocity.x += 1.0),
            ("hero.velocity.z", |s| s.hero.velocity.z += 1.0),
            ("hero.yaw", |s| s.hero.yaw += 0.25),
            ("hero.pitch", |s| s.hero.pitch += 0.25),
            ("hero.last_tap", |s| s.hero.last_tap += 1.0),
            ("hero.tap_target.y", |s| s.hero.tap_target.y += 1.0),
            ("hero.tap_queued", |s| s.hero.tap_queued = false),
            ("hero.queued_waves.len", |s| {
                s.hero.queued_waves.pop();
            }),
            ("hero.queued_waves[0].kind", |s| {
                s.hero.queued_waves[0].kind += 1;
            }),
            ("hero.queued_waves[0].at.x", |s| {
                s.hero.queued_waves[0].at.x += 1.0;
            }),
            ("hero.queued_waves[0].max_r", |s| {
                s.hero.queued_waves[0].max_r += 1.0;
            }),
            ("hero.queued_waves[0].speed", |s| {
                s.hero.queued_waves[0].speed += 1.0;
            }),
            ("hero.queued_waves[0].gain", |s| {
                s.hero.queued_waves[0].gain += 0.25;
            }),
            ("hero.queued_waves[0].echoes", |s| {
                s.hero.queued_waves[0].echoes += 1;
            }),
            ("hero.queued_waves[0].normal.z", |s| {
                s.hero.queued_waves[0].normal.z += 1.0;
            }),
            ("hero.queued_waves[1].normal.x", |s| {
                s.hero.queued_waves[1].normal.x += 1.0;
            }),
            ("hero.queued_waves[1].speed", |s| {
                s.hero.queued_waves[1].speed += 1.0;
            }),
            // hero.viewmodel
            ("hero.viewmodel.walk_amp", |s| {
                s.hero.viewmodel.walk_amp += 0.25;
            }),
            ("hero.viewmodel.leg_phase", |s| {
                s.hero.viewmodel.leg_phase += 0.25;
            }),
            ("hero.viewmodel.swing_phase", |s| {
                s.hero.viewmodel.swing_phase += 0.25;
            }),
            ("hero.viewmodel.cane_swing", |s| {
                s.hero.viewmodel.cane_swing += 0.25;
            }),
            ("hero.viewmodel.sway_x", |s| s.hero.viewmodel.sway_x += 0.25),
            ("hero.viewmodel.sway_y", |s| s.hero.viewmodel.sway_y += 0.25),
            ("hero.viewmodel.last_yaw", |s| {
                s.hero.viewmodel.last_yaw += 0.25;
            }),
            ("hero.viewmodel.last_pitch", |s| {
                s.hero.viewmodel.last_pitch += 0.25;
            }),
            ("hero.viewmodel.step_t", |s| s.hero.viewmodel.step_t += 0.25),
            ("hero.viewmodel.step_side", |s| {
                s.hero.viewmodel.step_side = 1;
            }),
            // cats
            ("cats.len", |s| {
                s.cats.pop();
            }),
            ("cats[0].position.x", |s| s.cats[0].position.x += 1.0),
            ("cats[0].position.y", |s| s.cats[0].position.y += 1.0),
            ("cats[0].position.z", |s| s.cats[0].position.z += 1.0),
            ("cats[0].yaw", |s| s.cats[0].yaw += 0.25),
            ("cats[0].velocity.y", |s| s.cats[0].velocity.y += 1.0),
            ("cats[0].brain.rng_state", |s| {
                s.cats[0].brain.rng_state ^= 1
            }),
            ("cats[0].brain.rng_inc", |s| s.cats[0].brain.rng_inc ^= 2),
            ("cats[0].brain.rect.min_x", |s| {
                s.cats[0].brain.rect.min_x += 1.0;
            }),
            ("cats[0].brain.rect.min_z", |s| {
                s.cats[0].brain.rect.min_z += 1.0;
            }),
            ("cats[0].brain.rect.max_x", |s| {
                s.cats[0].brain.rect.max_x += 1.0;
            }),
            ("cats[0].brain.rect.max_z", |s| {
                s.cats[0].brain.rect.max_z += 1.0;
            }),
            ("cats[0].brain.state", |s| {
                s.cats[0].brain.state = BrainState::Sit { left: 0.5 };
            }),
            ("cats[0].brain.state.tx", |s| {
                if let BrainState::Roam { tx, .. } = &mut s.cats[0].brain.state {
                    *tx += 1.0;
                }
            }),
            ("cats[0].brain.state.tz", |s| {
                if let BrainState::Roam { tz, .. } = &mut s.cats[0].brain.state {
                    *tz += 1.0;
                }
            }),
            ("cats[0].brain.yaw", |s| s.cats[0].brain.yaw += 0.25),
            ("cats[0].brain.speed", |s| s.cats[0].brain.speed += 0.25),
            ("cats[0].brain.blocked", |s| s.cats[0].brain.blocked += 0.25),
            ("cats[0].gait.phase", |s| s.cats[0].gait.phase += 0.25),
            ("cats[0].gait.amp", |s| s.cats[0].gait.amp += 0.25),
            ("cats[0].gait.planted[0].x", |s| {
                s.cats[0].gait.planted[0].x += 1.0;
            }),
            ("cats[0].gait.planted[3].z", |s| {
                s.cats[0].gait.planted[3].z += 1.0;
            }),
            ("cats[0].gait.aim[1].y", |s| s.cats[0].gait.aim[1].y += 1.0),
            ("cats[0].gait.in_swing[2]", |s| {
                s.cats[0].gait.in_swing[2] = true;
            }),
            ("cats[0].gait.moving", |s| s.cats[0].gait.moving = false),
            ("cats[0].tail[0].x", |s| s.cats[0].tail[0].x += 1.0),
            ("cats[0].tail[4].z", |s| s.cats[0].tail[4].z += 1.0),
            ("cats[0].pose.pos.y", |s| s.cats[0].pose.pos.y += 1.0),
            ("cats[0].pose.yaw", |s| s.cats[0].pose.yaw += 0.25),
            ("cats[0].pose.paws[2].x", |s| {
                s.cats[0].pose.paws[2].x += 1.0;
            }),
            ("cats[0].pose.bob", |s| s.cats[0].pose.bob += 0.25),
            ("cats[0].pose.amp", |s| s.cats[0].pose.amp += 0.25),
            ("cats[0].pose.sit", |s| s.cats[0].pose.sit += 0.25),
            // NaN in, number out: `+=` on a NaN would be a no-op mutation
            ("cats[0].presence_next", |s| s.cats[0].presence_next = 3.5),
            ("cats[0].sit", |s| s.cats[0].sit += 0.25),
            ("cats[0].sim_t", |s| s.cats[0].sim_t += 1.0),
            ("cats[0].last_pos.z", |s| s.cats[0].last_pos.z += 1.0),
            ("cats[1].brain.rng_state", |s| {
                s.cats[1].brain.rng_state ^= 1
            }),
            ("cats[1].brain.state.left", |s| {
                if let BrainState::Pause { left } = &mut s.cats[1].brain.state {
                    *left += 1.0;
                }
            }),
            ("cats[1].presence_next", |s| s.cats[1].presence_next += 1.0),
            ("cats[1].pose.sit", |s| s.cats[1].pose.sit += 0.25),
        ]
    }

    /// FNV-1a 64 against the published reference vectors — the offset
    /// basis for "" and the classic single-byte check. Hand-derived from
    /// the algorithm's spec (offset 0xcbf29ce484222325, prime
    /// 0x100000001b3), never from this implementation.
    #[test]
    fn fnv1a64_matches_the_published_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    /// One-ULP anywhere flips the hash — the property the determinism
    /// probe's vector canonicalization had to fight for. Here it is by
    /// construction: to_bits, never to_string.
    #[test]
    fn one_ulp_in_one_slot_changes_the_hash() {
        let a = test_state();
        let mut b = test_state();
        b.slots[12].t0 = f64::from_bits(b.slots[12].t0.to_bits() + 1);
        assert_ne!(state_hash(&a), state_hash(&b));
        assert_eq!(first_divergence(&a, &b).as_deref(), Some("slots[12].t0"));
    }

    /// The format version lives INSIDE the hashed bytes: two states
    /// identical except for version never match.
    #[test]
    fn a_version_bump_can_never_false_match() {
        let a = test_state();
        let mut b = test_state();
        b.format_version += 1;
        assert_ne!(state_hash(&a), state_hash(&b));
    }

    /// Identical states hash identically and diverge nowhere.
    #[test]
    fn identical_states_agree_completely() {
        assert_eq!(state_hash(&test_state()), state_hash(&test_state()));
        assert_eq!(first_divergence(&test_state(), &test_state()), None);
    }

    /// The net under the twin walks: EVERY leaf field, perturbed one at
    /// a time, must move the hash (or the encoder never saw it) and must
    /// be named by the divergence walk under exactly that path (or the
    /// two walks read different field lists). This is what catches a
    /// field added to one walk and forgotten in the other — the four
    /// tests above only ever touch four fields.
    #[test]
    fn every_field_reaches_both_walks() {
        let table = mutations();
        let mut paths: Vec<&str> = table.iter().map(|(path, _)| *path).collect();
        let listed = paths.len();
        paths.sort_unstable();
        paths.dedup();
        // a duplicated path is a copy-paste that quietly tests one field
        // twice and another never — the table's own vacuity guard
        assert_eq!(paths.len(), listed, "duplicate path in the mutation table");

        for (path, mutate) in table {
            let a = test_state();
            let mut b = test_state();
            mutate(&mut b);
            assert_ne!(
                state_hash(&a),
                state_hash(&b),
                "the encoder never hashed {path}"
            );
            assert_eq!(
                first_divergence(&a, &b).as_deref(),
                Some(path),
                "the divergence walk misnamed {path}"
            );
        }
    }

    /// Sign of zero and NaN are exactly where a float `==` walk and a
    /// bit walk part company, and the blob answers to bits: −0.0 is a
    /// real divergence from 0.0 (a velocity that flipped sign IS a
    /// different world), while a NaN field — the cat that never beat
    /// carries one — is identical to itself rather than eternally
    /// divergent.
    #[test]
    fn zeros_and_nans_are_compared_by_bits() {
        let mut a = test_state();
        let mut b = test_state();
        a.hero.yaw = 0.0;
        b.hero.yaw = -0.0;
        assert_ne!(state_hash(&a), state_hash(&b));
        assert_eq!(first_divergence(&a, &b).as_deref(), Some("hero.yaw"));

        assert!(
            test_state().cats[0].presence_next.is_nan(),
            "the fixture must carry the shipped NaN cadence"
        );
        assert_eq!(first_divergence(&test_state(), &test_state()), None);
    }

    /// A shorter list is a divergence, never a silent match. The
    /// divergence walk pairs list elements up, and a pairing walk that
    /// forgets to compare the lengths first simply runs out of pairs:
    /// an empty echo book would read as identical to a book with one
    /// appointment in it, and the restore gate would call that a
    /// success. The element here is all-default on purpose — the
    /// difference is its EXISTENCE.
    #[test]
    fn a_dropped_element_cannot_hide_behind_the_bytes_that_remain() {
        let mut a = test_state();
        a.echoes.clear();
        let mut b = test_state();
        b.echoes = vec![PendingEcho {
            at_t: 0.0,
            pos: Vector3::ZERO,
            gain: 0.0,
        }];
        assert_ne!(state_hash(&a), state_hash(&b));
        assert_eq!(first_divergence(&a, &b).as_deref(), Some("echoes.len"));
    }
}
