# Character Elevation Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the player and cat fall, follow ordinary supported slopes, keep their colliders, silhouettes, cane, and wave origins at one physical elevation, and emit one configurable footstep-class landing wave without changing static objects or the cat's established movement behavior.

**Architecture:** `rust/src/support_motion.rs` owns one pure, total, two-phase CharacterBody transition and landing-response law. `UnseeingPlayer` and `WaveCat` remain separate thin Godot adapters around one `move_and_slide()` each; each owns its input/brain, pose, sound, and Inspector policy, while `rust/src/nodes/support.rs` shares only named solver/layer constants and phase-to-layer mapping. Capture, restore, and structured observation carry the resulting explicit motion state without capturing transient collider IDs or scene-authored configuration.

**Tech Stack:** Rust 2024 on the repository-pinned stable toolchain, godot-rust `0.5.4` with Godot `4.7.1`, typed GDScript gdUnit4 fixtures, the existing native/wasm build scripts, and the separate GitHub wiki repository.

**Spec:** `docs/superpowers/specs/2026-08-21-character-elevation-support-design.md`

## Global Constraints

- Work only in `/Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation`; the durable `main` checkout and all concurrent worktrees remain untouched.
- Strict TDD applies to every behavior: add the named test, witness the intended failure, write the smallest production change, witness the pass, then refactor. Production written before its test is deleted.
- Execute the checklist in dependency order `Task 1 → Task 6 → Task 4 → Task 2 → Task 3 → Task 5 → Task 7 → Task 8`; related prose remains grouped by actor, so the numbering is the authority. Every task ends with targeted mutation evidence, a read-only reviewer gate against the actual diff, all relevant green tests, and one small coherent commit. Task 6 lands the format-2 schema before behavior: player/cat motion fields are `Controlled`/unsupported, suppression is clear, and existing queued waves are `Always`; preflight rejects non-dormant actor states until the corresponding adapter task lifts only that semantic restriction. The wire layout never changes again. Tasks 4, 2, 3, and 5 then activate already-captured state one behavior at a time, so every reachable commit remains complete and restorable without a mega-commit or version churn. All commits use `Dmitrii Galchenko <dggrus@gmail.com>`; subjects/bodies are narrative and contain no assistant attribution.
- `game/` remains the sole Godot 4.7 project. GDScript remains test/probe-only; all shipped kinematics, animation math, landing response, capture, and perception behavior live in Rust.
- The same pure Rust law must run on native x86_64/arm64 and wasm32. Add no architecture conditionals, threads, global caches, mutable statics, custom Resources, generated configuration files, dependencies, raycasts, shape casts, or a second `move_and_slide()`.
- All public pure functions are total on their declared domain. Godot values are validated at the adapter boundary; invalid configuration/state produces an explicit error and never a panic, NaN, Infinity, invented support, or partial restore.
- Per actor, steady-state work remains O(1), allocation-free pure arithmetic around the existing one body move. The cat's existing contact `Vec` and wave activity remain its only relevant allocation behavior.
- Only `UnseeingPlayer` and `WaveCat` acquire support motion. `WaveProp`, `WaveColumn`, `WaveWedge`, `WaveWall`, `WaveRun`, floor/ceiling slabs, `SoundFan`, and `SoundRadio` remain authored static content with their current coupled mesh/collider transforms.
- There is no jump, fall damage, recovery, landing pause, fall pose, moving-platform state, steep-slope sliding, per-foot/per-paw terrain IK, or actor-to-actor airborne response in this version.
- The cat brain and yaw freeze while airborne, but `CatGait`, the silhouette, tail animation, and presence cadence continue from actual movement. Do not reset or freeze the gait.
- Player/cat collision uses named layers: controlled layer `1 << 1` (Godot layer 2) and airborne layer `1 << 2` (Godot layer 3). Controlled actors collide with each other; if either is airborne, the pair ignores each other. Any floor contact on either actor layer is rejected as support.
- Solver values are exact: grounded motion mode, up `(0,1,0)`, snap `0.10 m`, floor angle `π/4 rad`, safe margin `0.001 m`, `max_slides = 6`, stop-on-slope `true`, constant-slope-speed `false`, platform floor/wall layers `0`, and platform-on-leave `DO_NOTHING`.
- Default actor values are exact: acceleration `9.8 m/s²`, terminal descent `20.0 m/s`, silent impact `1.5 m/s`, full impact `4.0 m/s`; maximum player landing gain/range `0.85 / 5.0 m`, maximum cat landing gain/range `0.60 / 2.5 m`.
- Acceleration is authored kinematics. Landing reach and gain are authored perception controls. Do not justify either from acoustics: the engine has no frequency axis, waves travel at `4.0–5.5 m/s`, reveal composes by `max`, and occlusion is a `{0,1}` gate.
- Both landing voices remain pulse kind 2 at `4.0 m/s`. Player landings use the existing reflecting footstep policy and support normal; cat landings remain direct omnidirectional pulses. Zero severity, zero resulting gain, or zero resulting range invokes neither emitter and consumes no pulse or echo capacity.
- Preserve black/white thin-outline rendering, the visible-air distance/min law, geometry-based `level_plan::spans_the_corridor` occlusion, source semantics, all role labels, the `[0.15, 0.96]` label band (except the existing radio preview), `MIN_SEP = 0.08`, and `render::superface::{COPLANAR_EPS,PATCH_EPS}`. Actor origin Y is the only perception input changed here.
- Never commit `target/`, `.wasm`, exports, `.pck`, rendered frames, reports, or other build output. Commit generated `.uid` sidecars for new GDScript files.
- Use bounded condition polling in Godot fixtures; add no sleeps. Structured observer/body/mesh/pulse reads are primary evidence; movie frames are final visual evidence, not the oracle.
- Numeric checks use one of four named contracts: `to_bits()` for canonical lanes and positive zero; at most one hand-derived f32 ULP for pure translations/static datums; `SAFE_MARGIN_M + one ULP` for a settled body contact; and one ULP of `terminal_fall_speed_lane_mps()` for terminal descent. Do not use bare `is_equal_approx` or copy a production constant into the expected side.
- Before completion run Rust fmt/clippy/tests/editor-docs/release, gdformat/gdlint, full census-checked gdUnit4, boot/error, editor, restore/determinism, native target, wasm, and repository hygiene gates.
- Update the five relevant wiki pages in a fresh external wiki clone and commit that repository separately. Do not push the code branch, wiki branch, merge to `main`, close issues, or deploy without the user's finish-branch choice; deployment follows a later push to `main` automatically.

## File Structure

| File | Responsibility in this change |
| --- | --- |
| `rust/src/support_motion.rs` (new) | Pure finite value types, validated actor configuration, support phase transition, landing event and authored landing voice. |
| `rust/src/nodes/support.rs` (new) | Shared Godot solver constants, named actor layer bits/masks, and pure phase-to-layer helpers; no node state or queries. |
| `rust/src/lib.rs` | Publish the pure support-motion module and document it in the crate architecture map. |
| `rust/src/nodes/mod.rs` | Include the non-class support boundary helper. |
| `rust/src/nodes/player.rs` | Player body datum, two-phase motion adapter, layer/support extraction, landing emission, latch, relocation/restore doors, and elevation-relative cane. |
| `rust/src/nodes/game.rs` | Six scene-authored player Inspector knobs, validated setters, and pre-tree player configuration injection. |
| `rust/src/viewmodel.rs` | Pure support-relative player leg placement. |
| `rust/src/nodes/hero.rs` | Whole-body elevation, airborne neutral pose/step gate, and acknowledged landing-step suppression. |
| `rust/src/cat_gait.rs` | One support-Y datum transported through planted/aim/contact state and capture. |
| `rust/src/cat_body.rs` | Support-relative cat skeleton and exact vertical tail transport. |
| `rust/src/nodes/cat.rs` | Cat Inspector knobs, body datum, motion adapter, brain freeze, layer/support extraction, elevated voices, landing, and exact restore door; no general cat teleport API. |
| `rust/src/observe/mod.rs` | Pure structured actor-motion observation values. |
| `rust/src/nodes/observer.rs` | Live player/cat motion facts plus canonical capture dictionary writer/parser. |
| `rust/src/reproduce/blob.rs` | Motion-aware Hero/Cat capture types, canonical bytes, bitwise diff, fixtures, and exhaustive mutation table. |
| `rust/src/reproduce/mod.rs` | Capture format version 2. |
| `rust/src/nodes/restorer.rs` | All-read-only preflight before any restore write, then exact actor phase/latch/layer restoration. |
| `rust/src/{pulse_pool,echo_queue,viewmodel,cat_brain,cat_gait,cat_body,sound_source,temporal,demo_tap,flicker}.rs` | Owner-defined checked/prepared restore values; the restorer composes these contracts and never reimplements them. |
| `game/project.godot` | Human-readable names for physics layers 2 and 3. |
| `game/tests/character_elevation_fixture.gd` (new) | Checked floor, platform, ramp, wall, player, cat, and bounded-poll helpers shared only by tests. |
| `game/tests/player_elevation_test.gd` (new) | #64 player fall/ramp/visual/cane/landing/config regressions. |
| `game/tests/cat_elevation_test.gd` (new) | #74 floor/table/bed/fall/brain/gait/voice regressions. |
| `game/tests/actor_support_test.gd` (new) | Controlled blocking, airborne pass-through, and actor-contact support rejection. |
| `game/tests/scenes/character_elevation_movie.tscn` and `game/tests/probe/character_elevation_movie.gd` (new) | Deterministic test-only visual sequence and structured state marks for movie evidence. |
| Existing `game/tests/{movement,viewmodel,footsteps,cane,cat,knob_hint,game_root,observer,restore,restore_transaction}_test.gd` | Preserve flat behavior and extend established public/capture contracts. |
| `game/tests/probe/restore_probe.gd` | Exercise the new format/state through the existing deterministic future probe. |

---

### Task 1: Pure Support Motion and Landing Response

**Files:**
- Create: `rust/src/support_motion.rs`
- Modify: `rust/src/lib.rs:14-75,89-114`

**Interfaces:**
- Consumes: only `godot::builtin::{Transform3D, Vector3}` as engine-free math values.
- Produces: `MotionConfigField`, `MotionConfigError`, `MotionValueProblem`, `MotionValueError`, `MotionRestoreError`, `ActorPosition`, `PosePoint`, `ActorTransform`, `FiniteRotation`, `SupportElevation`, `ActorYaw`, `FiniteMeasure`, `ActorVelocity`, `StepDuration`, `PlanarVelocity`, `FiniteVelocity`, `FiniteSpeed`, `SupportContact`, `LandingEvent`, `MotionPhase`, `MotionState`, `SupportMotionConfig`, `MotionCommand`, `PreparedMotion`, `MotionOutcome`, `MotionTransition`, `LandingVoice`, `QueuedWaveGate`, `FootstepSuppression`, `prepare`, `reconcile`, `landing_voice`, and `validate_restore` with the exact signatures below.

The public error contract is also explicit:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionConfigField {
    FallAcceleration,
    TerminalFallSpeed,
    LandingSilentSpeed,
    LandingFullSpeed,
    LandingMaxGain,
    LandingMaxRange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionConfigError {
    NonFinite { field: MotionConfigField, value: f64 },
    OutOfRange { field: MotionConfigField, value: f64, min: f64, max: f64 },
    ThresholdOrder { silent_speed_mps: f64, full_speed_mps: f64 },
}
impl MotionConfigError {
    pub fn field(self) -> Option<MotionConfigField>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionValueProblem { NonFinite, Negative, OutOfRange, ZeroVector, InconsistentState }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionValueError {
    field: &'static str,
    problem: MotionValueProblem,
}
impl MotionValueError {
    pub fn non_finite(field: &'static str) -> Self;
    pub fn negative(field: &'static str) -> Self;
    pub fn out_of_range(field: &'static str) -> Self;
    pub fn zero_vector(field: &'static str) -> Self;
    pub fn inconsistent_state(field: &'static str) -> Self;
    pub fn field(self) -> &'static str;
    pub fn problem(self) -> MotionValueProblem;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionRestoreError {
    Physical(MotionValueError),
    AirbornePlanarMismatch { axis: &'static str },
    AirborneTerminalExceeded,
}
```

All three error types implement `Display` and `std::error::Error`; their messages name the field and violated rule, and `ThresholdOrder` prints both supplied silent/full thresholds. `MotionConfigError::field()` returns `Some` for the two field-specific variants and `None` for `ThresholdOrder`. Error payloads are diagnostic values only and never re-enter arithmetic.

- [ ] **Step 1: Add the failing total-domain and transition tests**

Create the test module first, importing the complete math boundary explicitly; only then add `pub mod support_motion;` to `rust/src/lib.rs` so these tests are discovered. Do not add production definitions yet: the wired tests must be what makes the crate fail to compile.

```rust
#[cfg(test)]
mod tests {
use super::*;
use godot::builtin::{Basis, Transform3D, Vector3};

#[test]
fn malformed_durations_are_zero_and_large_steps_are_capped() {
    for raw in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(StepDuration::from_raw(raw).seconds().to_bits(), 0.0_f64.to_bits());
    }
    assert_eq!(StepDuration::from_raw(0.5).seconds(), 1.0 / 15.0);
}

#[test]
fn airborne_acceleration_is_bounded_by_dt_and_terminal_speed() {
    let config = SupportMotionConfig::PLAYER_DEFAULT;
    let state = MotionState::restore(
        MotionPhase::Airborne {
            planar_velocity_mps: PlanarVelocity::ZERO,
            vertical_velocity_mps: FiniteVelocity::try_new(0.0).unwrap(),
        },
        None,
        None,
    ).unwrap();
    let first = prepare(state, PlanarVelocity::ZERO, StepDuration::from_raw(1.0 / 60.0), config);
    assert_eq!(first.command().world_velocity().y, -0.163_333_33_f32);
    let near_terminal = MotionState::restore(
        MotionPhase::Airborne {
            planar_velocity_mps: PlanarVelocity::ZERO,
            vertical_velocity_mps: FiniteVelocity::try_new(-19.9).unwrap(),
        },
        None,
        None,
    ).unwrap();
    let capped = prepare(near_terminal, PlanarVelocity::ZERO, StepDuration::from_raw(0.5), config);
    assert_eq!(capped.command().world_velocity().y, -20.0);
}

#[test]
fn landing_voice_is_silent_linear_and_capped() {
    let support = SupportContact::try_new(Vector3::ZERO, Vector3::UP).unwrap();
    assert!(landing_voice(LandingEvent::try_new(1.5, support).unwrap(), SupportMotionConfig::PLAYER_DEFAULT).is_none());
    let half = landing_voice(LandingEvent::try_new(2.75, support).unwrap(), SupportMotionConfig::PLAYER_DEFAULT).unwrap();
    assert_eq!(half.gain(), 0.425);
    assert_eq!(half.range_m(), 2.5);
    let full = landing_voice(LandingEvent::try_new(9.0, support).unwrap(), SupportMotionConfig::PLAYER_DEFAULT).unwrap();
    assert_eq!((full.gain(), full.range_m()), (0.85, 5.0));
}
}
```

Add the remaining named cases listed below as compact table-driven tests. The table is the expected side—do not derive it through production getters:

| Test | Literal rows/assertion |
| --- | --- |
| `defaults_are_valid_and_raw_config_obeys_the_authored_ranges` | player `(9.8,20,1.5,4,0.85,5)`, cat `(9.8,20,1.5,4,0.60,2.5)`; each inclusive endpoint succeeds and its adjacent outside decimal fails with the matching field. |
| `actor_position_rejects_each_poisoned_or_out_of_envelope_lane` | put `NaN`, `+∞`, `-∞`, and the adjacent f32 above `1_000_000.0` into X/Y/Z one lane at a time; `±1_000_000.0` succeeds bit-exactly. |
| `actor_transform_rejects_each_poisoned_origin_or_basis_lane_and_preserves_valid_bits` | mutate each of the 12 origin/basis f32 lanes of `Transform3D::IDENTITY`; every poison refuses and an untouched transform round-trips all lane bits. |
| `finite_rotation_rejects_each_poisoned_lane` | poison X/Y/Z separately; finite `(-f32::MAX,0,f32::MAX)` succeeds bit-exactly. |
| `actor_yaw_and_measure_reject_poison_and_unrepresentable_or_negative_values` | yaw rejects nonfinite and the next f64 above `f32::MAX as f64`; measure rejects nonfinite/negative and accepts `-0.0`, `0.0`, and `2_000_000.0`. |
| `prepared_terminal_state_validates_for_nonrepresentable_decimal_config` | config terminal `0.6_f64`; integrate to its effective f32 lane, then `validate_restore` must accept that exact state and reject the adjacent-more-negative f32 lane. |
| `controlled_contact_gate_requires_two_controlled_phases_without_landing` | `Always` is true for all rows; `ControlledContact` is true only for `(Controlled,Controlled,None)` and false for pre-air, post-air, or `Some(landing)`. |
| `footstep_suppression_persists_until_acknowledged` | `CLEAR → on_transition(None) = CLEAR`; landing sets pending; any later `None` preserves pending; acknowledge returns `(CLEAR,true)` once, then `(CLEAR,false)`. |

Also implement `every_config_field_reports_its_exact_error_variant_and_display`, `threshold_order_reports_both_supplied_values`, `derived_pose_and_player_support_envelopes_admit_both_extreme_roots`, `actor_velocity_rejects_each_poisoned_lane`, `controlled_support_never_creates_a_landing`, `an_edge_captures_actual_trajectory_and_air_ignores_new_intent`, `a_wall_keeps_the_collision_adjusted_planar_trajectory`, `landing_changes_phase_once_and_keeps_the_event_as_observation`, `relocation_retains_only_inert_landing_history`, `restore_validation_rejects_poison_mismatch_and_terminal_violations`, `signed_zero_planar_restore_is_bit_exact`, and `zero_gain_or_range_produces_no_voice` using the literal state/voice values already shown in this task. Each test asserts the named error field/problem or complete returned state, not merely `is_err()`.

- [ ] **Step 2: Run the new tests and record the intended red result**

Run:

```bash
cd rust
cargo test support_motion::tests
```

Expected: compilation fails because the module API is not implemented. This is the required red evidence; a failure in an unrelated pre-existing test is not acceptable.

- [ ] **Step 3: Implement the finite value/configuration door**

Add `support_motion` to `lib.rs`'s pure-core architecture map, then implement these exact public shapes and names. `SupportContact::try_new` validates a finite point and finite nonzero normal but preserves every supplied f32 lane bit; it must not normalize the normal. Test nonzero component lanes directly rather than via squared length, so a finite subnormal direction is not accidentally classified as the zero vector through underflow and `(±0, ±0, ±0)` remains the only zero normal.

```rust
pub const MAX_ACCEL_DT_S: f64 = 1.0 / 15.0;
pub const MAX_ACTOR_COORD_M: f32 = 1_000_000.0;
pub const MAX_POSE_COORD_M: f32 = 1_000_002.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepDuration(f64);
impl StepDuration {
    pub fn from_raw(raw_seconds: f64) -> Self {
        Self(if raw_seconds.is_finite() && raw_seconds > 0.0 {
            raw_seconds.min(MAX_ACCEL_DT_S)
        } else {
            0.0
        })
    }
    pub fn seconds(self) -> f64 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorPosition(Vector3);
impl ActorPosition {
    pub fn try_new(world: Vector3) -> Result<Self, MotionValueError>;
    pub fn world(self) -> Vector3 { self.0 }
    pub fn planar_distance(self, prior: Self) -> FiniteMeasure;
    pub fn elevation(self) -> SupportElevation;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PosePoint(Vector3);
impl PosePoint {
    pub fn try_new(world: Vector3) -> Result<Self, MotionValueError>;
    pub fn world(self) -> Vector3 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorTransform(Transform3D);
impl ActorTransform {
    pub fn try_new(world: Transform3D) -> Result<Self, MotionValueError>;
    pub fn world(self) -> Transform3D { self.0 }
    pub fn position(self) -> ActorPosition;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteRotation(Vector3);
impl FiniteRotation {
    pub fn try_new(euler_radians: Vector3) -> Result<Self, MotionValueError>;
    pub fn world(self) -> Vector3 { self.0 }
    pub fn yaw(self) -> ActorYaw;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportElevation(f32);
impl SupportElevation {
    pub fn try_new(y: f32) -> Result<Self, MotionValueError>;
    pub fn y(self) -> f32 { self.0 }
    pub fn delta_from(self, prior: Self) -> f32;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorYaw(f64);
impl ActorYaw {
    pub fn try_new(radians: f64) -> Result<Self, MotionValueError>;
    pub fn radians(self) -> f64 { self.0 }
    pub fn godot_lane(self) -> f32 { self.0 as f32 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteMeasure(f64);
impl FiniteMeasure {
    pub const ZERO: Self = Self(0.0);
    pub fn try_new(value: f64, field: &'static str) -> Result<Self, MotionValueError>;
    pub fn value(self) -> f64 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorVelocity(Vector3);
impl ActorVelocity {
    pub fn try_new(world_mps: Vector3) -> Result<Self, MotionValueError>;
    pub fn world(self) -> Vector3 { self.0 }
    pub fn planar(self) -> PlanarVelocity;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanarVelocity { x_mps: f32, z_mps: f32 }
impl PlanarVelocity {
    pub const ZERO: Self = Self { x_mps: 0.0, z_mps: 0.0 };
    pub fn try_new(x_mps: f32, z_mps: f32) -> Result<Self, MotionValueError>;
    pub fn try_from_world(raw: Vector3) -> Result<Self, MotionValueError>;
    pub fn x_mps(self) -> f32 { self.x_mps }
    pub fn z_mps(self) -> f32 { self.z_mps }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionPhase {
    Controlled,
    Airborne {
        planar_velocity_mps: PlanarVelocity,
        vertical_velocity_mps: FiniteVelocity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionState {
    phase: MotionPhase,
    support: Option<SupportContact>,
    last_landing: Option<LandingEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteVelocity(f32);
impl FiniteVelocity {
    pub fn try_new(mps: f32) -> Result<Self, MotionValueError>;
    pub fn mps(self) -> f32 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteSpeed(f32);
impl FiniteSpeed {
    pub fn try_new(mps: f32) -> Result<Self, MotionValueError>;
    pub fn mps(self) -> f32 { self.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportContact { point: Vector3, normal: Vector3 }
impl SupportContact {
    pub fn try_new(point: Vector3, normal: Vector3) -> Result<Self, MotionValueError>;
    pub fn point(self) -> Vector3 { self.point }
    pub fn normal(self) -> Vector3 { self.normal }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LandingEvent { impact_speed: FiniteSpeed, support: SupportContact }
impl LandingEvent {
    pub fn try_new(impact_speed_mps: f32, support: SupportContact) -> Result<Self, MotionValueError>;
    pub fn impact_speed(self) -> FiniteSpeed { self.impact_speed }
    pub fn support(self) -> SupportContact { self.support }
}
```

`ActorPosition::try_new` rejects each non-finite lane and every magnitude above `MAX_ACTOR_COORD_M`. `PosePoint` validates derived joints/paws/tail nodes against `MAX_POSE_COORD_M`, reserving a proved 2 m envelope around an extreme valid root; the cat's farthest authored joint/tail offset is below 1 m. `ActorTransform::try_new` validates its origin through `ActorPosition` plus all nine basis lanes as finite, and preserves every bit for exact rollback. `FiniteRotation` validates the three Euler lanes without treating radians as meter coordinates; its f32 lanes are representable by construction, and `yaw()` widens Y exactly. `planar_distance` converts each X/Z lane to f64 before subtracting and squaring and returns the proved finite non-negative measure. `SupportElevation` uses the derived `MAX_POSE_COORD_M` bound so the player value `root_y - 0.9 m` is admitted at both root extremes; `delta_from` widens before subtraction, then narrows a magnitude no greater than `2_000_004.0`, so it is structurally finite and infallible. `ActorYaw` accepts finite f64 values only when their magnitude is at most `f32::MAX as f64`; `godot_lane` is therefore always finite. `FiniteMeasure` accepts finite non-negative values. `ActorVelocity::try_new` validates all three physical lanes once and retains them; `PlanarVelocity::try_new` rejects either non-finite retained lane. `try_from_world` remains a convenience door but rejects a non-finite X, Y, or Z observation before retaining X/Z. `FiniteVelocity::try_new` accepts every finite signed velocity; `FiniteSpeed::try_new` accepts only finite values greater than or equal to zero. `MotionOutcome::new` consumes `ActorVelocity`, so no raw Godot velocity can reach reconciliation.

Add `MotionState::initial() -> Self`, `restore(MotionPhase, Option<SupportContact>, Option<LandingEvent>) -> Result<Self, MotionValueError>`, `relocated(self) -> Self`, `phase(self) -> MotionPhase`, `support(self) -> Option<SupportContact>`, `last_landing(self) -> Option<LandingEvent>`, and `accepts_control(self) -> bool`. `restore` rejects positive airborne Y and airborne state with support. `relocated` returns controlled/no-support while retaining inert `last_landing`.

Define the configuration completely:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupportMotionConfig {
    fall_acceleration_mps2: f64,
    terminal_fall_speed_mps: f64,
    terminal_fall_speed_lane_mps: f32,
    landing_silent_speed_mps: f64,
    landing_full_speed_mps: f64,
    landing_max_gain: f64,
    landing_max_range_m: f64,
}
```

Implement `SupportMotionConfig::try_new(fall_acceleration_mps2: f64, terminal_fall_speed_mps: f64, landing_silent_speed_mps: f64, landing_full_speed_mps: f64, landing_max_gain: f64, landing_max_range_m: f64) -> Result<Self, MotionConfigError>` plus by-value getters `fall_acceleration_mps2()`, `terminal_fall_speed_mps()`, `terminal_fall_speed_lane_mps()`, `landing_silent_speed_mps()`, `landing_full_speed_mps()`, `landing_max_gain()`, and `landing_max_range_m()`. The authored getters return f64 and the effective lane getter returns the one positive f32 produced by the constructor. Validate finite acceleration `0.1..=30.0`, terminal speed `0.5..=50.0`, silent speed `0.0..=10.0`, full speed `0.1..=20.0`, maximum gain `0.0..=1.0`, maximum range `0.0..=10.0`, and `full > silent`. Provide associated constants `PLAYER_DEFAULT` and `CAT_DEFAULT` through a private `const fn from_validated_constants`; do not use `Default`, `unwrap`, or an Option in node initialization.

Add these remaining exact result interfaces:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionCommand { world_velocity_mps: Vector3 }
impl MotionCommand { pub fn world_velocity(self) -> Vector3 { self.world_velocity_mps } }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedMotion { prior: MotionState, command: MotionCommand }
impl PreparedMotion { pub fn command(self) -> MotionCommand { self.command } }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionOutcome {
    actual_planar_velocity_mps: PlanarVelocity,
    accepted_support: Option<SupportContact>,
}
impl MotionOutcome {
    pub fn new(
        actual_velocity_mps: ActorVelocity,
        accepted_support: Option<SupportContact>,
    ) -> Self;
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionTransition {
    pub state: MotionState,
    pub landing: Option<LandingEvent>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LandingVoice { gain: f64, range_m: f64 }
impl LandingVoice {
    pub fn gain(self) -> f64;
    pub fn range_m(self) -> f64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuedWaveGate { Always, ControlledContact }
impl QueuedWaveGate {
    pub fn allows(
        self,
        before: MotionPhase,
        after: MotionPhase,
        landing: Option<LandingEvent>,
    ) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FootstepSuppression { pending: bool }
impl FootstepSuppression {
    pub const CLEAR: Self = Self { pending: false };
    pub fn restore(pending: bool) -> Self;
    pub fn pending(self) -> bool;
    pub fn on_transition(self, landing: Option<LandingEvent>) -> Self;
    pub fn acknowledge(self) -> (Self, bool);
}
pub fn prepare(
    state: MotionState,
    desired_planar: PlanarVelocity,
    duration: StepDuration,
    config: SupportMotionConfig,
) -> PreparedMotion;
pub fn reconcile(prepared: PreparedMotion, outcome: MotionOutcome) -> MotionTransition;
pub fn landing_voice(
    event: LandingEvent,
    config: SupportMotionConfig,
) -> Option<LandingVoice>;
pub fn validate_restore(
    state: MotionState,
    physical_velocity_mps: ActorVelocity,
    config: SupportMotionConfig,
) -> Result<(), MotionRestoreError>;
```

`MotionCommand` also has a private `from_finite_parts(planar: PlanarVelocity, vertical: FiniteVelocity) -> Self`; `PreparedMotion` is built directly inside this module from its private `prior` and `command` fields. Neither construction path revalidates or panics because its arguments are already validated finite value types.

- [ ] **Step 4: Implement the two-phase transition and authored voice**

Use one prepared command and never read desired intent in the airborne branch:

```rust
pub fn prepare(
    state: MotionState,
    desired_planar: PlanarVelocity,
    duration: StepDuration,
    config: SupportMotionConfig,
) -> PreparedMotion {
    let command = match state.phase() {
        MotionPhase::Controlled => MotionCommand::from_finite_parts(
            desired_planar,
            FiniteVelocity::ZERO,
        ),
        MotionPhase::Airborne { planar_velocity_mps, vertical_velocity_mps } => {
            let next_y = (f64::from(vertical_velocity_mps.mps())
                - config.fall_acceleration_mps2() * duration.seconds())
                .max(-f64::from(config.terminal_fall_speed_lane_mps())) as f32;
            MotionCommand::from_finite_parts(
                planar_velocity_mps,
                FiniteVelocity::from_finite(next_y),
            )
        }
    };
    PreparedMotion { prior: state, command }
}
```

`FiniteVelocity::ZERO` and the private `from_finite(f32)` are available only inside the module. `from_finite` is called here only after finite validated operands, a bounded finite duration, and finite configuration arithmetic; document that invariant next to the private constructor. The public function therefore remains visibly total without a panic site.

`reconcile` implements exactly four cases: controlled/support stays controlled with no event; controlled/no-support captures post-slide X/Z and starts airborne at negative zero Y; airborne/no-support keeps prepared Y and post-slide X/Z; airborne/support creates/stores/returns one event from the magnitude of the prepared downward command. Every non-landing branch carries the prior `last_landing` unchanged as inert history, while support is always replaced by the current accepted-support result. `landing_voice` uses the approved piecewise linear severity and returns `None` at/below the silence threshold or when resulting gain/range is zero. `QueuedWaveGate::Always` returns true; `ControlledContact` returns true only when both phases are `Controlled` and `landing.is_none()`. `FootstepSuppression::on_transition` sets pending exactly when `landing.is_some()` and otherwise preserves it; `acknowledge` returns `(CLEAR, old_pending)`. `validate_restore` uses bit-equal airborne X/Z, permits controlled slope velocity differences, and compares the pure airborne f32 Y directly with `-config.terminal_fall_speed_lane_mps()..=0.0`, the same effective lane used by `prepare`.

- [ ] **Step 5: Run pure green gates**

Run:

```bash
cd rust
cargo fmt --check
cargo test support_motion::tests
cargo clippy --all-targets -- -D warnings
```

Expected: all new tests pass; existing tests remain green.

- [ ] **Step 6: Perform the focused mutation campaign**

Temporarily mutate one item at a time, run its named test, witness failure, then restore the green line: zero/reverse acceleration; remove dt cap; remove terminal clamp; use fresh desired input in air; retain launch rather than post-collision X/Z; keep controlled off an edge; emit landing from controlled support; change `<= silent` to `<`; remove saturation; couple gain/range; allow a zero output; compare signed zero numerically. Record command/result in the task notes; do not commit mutations.

- [ ] **Step 7: Review and commit the pure component**

Request read-only spec and code review of this task's diff, verify every finding against the source, fix with a new red/green cycle, rerun Step 5, then stage only `rust/src/support_motion.rs` and `rust/src/lib.rs` and make one small narrative commit with a precise what/why body.

---

### Task 2: Player Physical Support, Layers, and Inspector Injection

**Files:**
- Create: `rust/src/nodes/support.rs`
- Create: `game/tests/character_elevation_fixture.gd`
- Create: `game/tests/player_elevation_test.gd`
- Modify: `rust/src/nodes/mod.rs:17-33`
- Modify: `rust/src/nodes/player.rs:20-236,600-660`
- Modify: `rust/src/nodes/game.rs:86-175,349-360,499-681`
- Modify: `game/project.godot:59-63`
- Modify: `game/tests/movement_test.gd:1-97`
- Modify: `game/tests/game_root_test.gd`
- Modify: `game/tests/knob_hint_test.gd`

**Interfaces:**
- Consumes: all Task 1 types/functions.
- Produces: shared `nodes::support` constants/helpers; `UnseeingPlayer::{inject_motion_config,motion_state,motion_config,support_collider_id,support_elevation_y,try_relocate,validate_motion_restore}` plus registered relocation/config diagnostics; six `UnseeingGame.player_*` exports. Task 6 already owns the capture fields; Task 3 activates landing/suppression/gated contacts.

- [ ] **Step 1: Add checked fixture builders and failing player cases**

Create a non-suite `RefCounted` fixture with exact helpers:

```gdscript
extends RefCounted

const DT := 1.0 / 60.0

static func add_box(parent: Node, centre: Vector3, size: Vector3, name: String) -> StaticBody3D:
	var body := StaticBody3D.new()
	body.name = name
	body.position = centre
	var collision := CollisionShape3D.new()
	var shape := BoxShape3D.new()
	shape.size = size
	collision.shape = shape
	body.add_child(collision)
	parent.add_child(body)
	return body

static func add_floor(parent: Node, top_y := 0.0, size := Vector2(20, 20)) -> StaticBody3D:
	return add_box(parent, Vector3(0, top_y - 0.05, 0), Vector3(size.x, 0.1, size.y), "Floor")

static func add_ramp(parent: Node, datum: Vector3, size := Vector3(1.4, 0.45, 1.0)) -> WaveWedge:
	var ramp := WaveWedge.new()
	ramp.name = "Ramp"
	ramp.size = size
	ramp.position = datum
	parent.add_child(ramp)
	return ramp
```

The wedge follows the shipped `HallRamp` orientation: its node is the horizontal centre/datum, its lowest edge stands at `datum.y`, and local +X climbs. Pair it with a `WaveProp` platform of size `(1.2, 0.45, 1.0)`, centre `datum + (1.3, 0.225, 0)`, so the wedge high edge and platform top are both Y `datum.y + 0.45`. Read both generated collision shapes back in the fixture before using them.

Add `add_ramp`, `add_player`, `add_cat`, and a bounded `poll_physics(tree, predicate, max_ticks)` that returns false after the fixed bound rather than sleeping. The fixture also owns checked content constants and builders:

```gdscript
const TABLE_SCENE := preload("res://scenes/props/table.tscn")
const TABLE_TOP_Y := 0.75  # Top centre 0.725 + half-height 0.025.
const BED_TOP_Y := 0.48  # BedFrame centre 0.42 + half-height 0.06.

static func add_table(parent: Node, at: Vector3) -> Node3D:
	var table := TABLE_SCENE.instantiate() as Node3D
	assert(table != null, "table.tscn must retain its Node3D root")
	table.position = at
	parent.add_child(table)
	return table

static func add_bed(parent: Node, at: Vector3) -> WaveProp:
	var bed := WaveProp.new()
	bed.name = "BedFrame"
	bed.size = Vector3(1.9, 0.12, 0.9)
	bed.position = at + Vector3(0, 0.42, 0)
	parent.add_child(bed)
	return bed
```

These literals come from `game/scenes/props/table.tscn` and `game/scenes/level_01.tscn`; tests assert the resulting collider top rather than trusting only the comments.

In `player_elevation_test.gd`, add exact cases `test_player_capsule_bottom_meets_the_authored_flat_datum`, `test_unsupported_player_falls_and_stops_at_terminal_speed`, `test_airborne_input_cannot_reverse_the_departure_trajectory`, `test_airborne_wall_contact_removes_only_the_blocked_planar_component_without_a_wave`, `test_player_returns_to_control_on_lower_world_geometry_once`, `test_player_knobs_reach_the_runtime_player_before_tree_entry`, `test_out_of_range_player_knob_retains_the_prior_scalar`, `test_valid_player_threshold_pairs_round_trip_above_and_below_defaults`, `test_invalid_final_player_threshold_pair_refuses_before_player_construction`, `test_player_solver_contract_is_explicit_on_every_property`, `test_player_solver_disables_ambient_platform_motion`, `test_actor_layers_are_named_and_phase_derived`, `test_player_rejects_actor_layer_floor_before_cat_adapter_exists`, `test_server_backed_world_body_without_node_is_accepted_support`, `test_server_backed_zero_object_id_is_accepted_with_null_identity`, `test_poisoned_player_pre_move_transform_or_rotation_refuses_without_move_or_wave`, `test_poisoned_player_post_move_transform_or_rotation_rolls_back_exactly`, and `test_nonfinite_player_relocation_is_atomic`. Until Task 7 exposes structured phase, the return-to-control test observes the layer-3 to layer-2 transition and accepted `is_on_floor()` state; Task 3 separately pins exactly one landing wave. The solver-contract case reads and hand-asserts motion mode, up direction, snap, maximum floor angle, safe margin, maximum slides, both slope booleans, both platform masks, and platform-on-leave. The actor-floor case uses a static dummy on layer 2 beneath the player; the server-backed cases create a body/shape directly through `PhysicsServer3D` and prove its valid RID remains support without a `CollisionObject3D` node; an object ID of zero is observed as NIL, not support refusal.

Before adapter production, add Rust trace tests around a narrow private `PlayerMotionPort` used by the callback: `post_move_poison_writes_exact_saved_transform_then_zero_velocity_then_disables` supplies a valid pre-transform and a post-transform/rotation with each lane poisoned in turn, and asserts that exact three-command trace plus no state/layer/wave command; `valid_tick_calls_move_and_slide_once` asserts one and only one move command. The production port forwards to `CharacterBody3D`; the fake is test-only and contains no scene tree.

Add the exact #64 ramp test in this red step, before the adapter exists: a `1.4 × 0.45 × 1.0 m` wedge leading to the checked platform described above. Poll the player up and back down; assert the settled platform root against `1.35_f32` with tolerance `SAFE_MARGIN_M + 1.192_092_895_507_812_5e-7 m` (one f32 ULP at 1.35), controlled layer/support throughout, no airborne layer, and no landing observation/wave on either direction. The old planar-only body must fail this elevation result.

Update `movement_test.gd::before_test` to add a checked wide floor before adding its root-Y `0.9` player. The expected grounded velocity Y remains exactly zero; replace the obsolete “flat map” wording with “controlled supported motion”.

- [ ] **Step 2: Run player tests and witness the old defect**

Run:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test valid_tick_calls_move_and_slide_once)
(cd rust && cargo test post_move_poison_writes_exact_saved_transform_then_zero_velocity_then_disables)
(cd rust && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
"$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a res://tests/player_elevation_test.gd
```

Expected: the unsupported body remains at its authored Y with `velocity.y == 0`, capsule datum/layer/config assertions fail, and no landing transition exists.

- [ ] **Step 3: Add shared solver/layer constants and project names**

Implement `rust/src/nodes/support.rs` exactly as a stateless boundary value module:

```rust
pub const CONTROLLED_ACTOR_LAYER: u32 = 1 << 1;
pub const AIRBORNE_ACTOR_LAYER: u32 = 1 << 2;
pub const ALL_LAYERS: u32 = u32::MAX;
pub const FLOOR_SNAP_M: f32 = 0.10;
pub const FLOOR_MAX_ANGLE_RAD: f32 = std::f32::consts::FRAC_PI_4;
pub const SAFE_MARGIN_M: f32 = 0.001;
pub const MAX_SLIDES: i32 = 6;
pub const PLATFORM_LAYERS: u32 = 0;

pub fn collision_pair(phase: MotionPhase) -> (u32, u32) {
    match phase {
        MotionPhase::Controlled => (
            CONTROLLED_ACTOR_LAYER,
            ALL_LAYERS & !AIRBORNE_ACTOR_LAYER,
        ),
        MotionPhase::Airborne { .. } => (
            AIRBORNE_ACTOR_LAYER,
            ALL_LAYERS & !(CONTROLLED_ACTOR_LAYER | AIRBORNE_ACTOR_LAYER),
        ),
    }
}

pub fn is_actor_layer(layer: u32) -> bool {
    layer & (CONTROLLED_ACTOR_LAYER | AIRBORNE_ACTOR_LAYER) != 0
}
```

Name the layers in `game/project.godot`:

```ini
[layer_names]

3d_physics/layer_2="Controlled Actor"
3d_physics/layer_3="Airborne Actor"
```

- [ ] **Step 4: Add the player datum and one-move adapter**

Define:

```rust
pub const PLAYER_STANDING_ROOT_Y: f64 = 0.9;
pub const CONTACT_BIRTH_HEIGHT_M: f32 = 0.04;
const PLAYER_CAPSULE_CENTER_Y_M: f32 = -0.05;
pub const CAM_BASE_Y: f64 = EYE - PLAYER_STANDING_ROOT_Y;
```

`support_elevation_y()` performs `global_position.y - PLAYER_STANDING_ROOT_Y as f32`, entirely in the `Vector3` lane type. Thus an authored flat root whose Y is the same `0.9` f32 returns exact positive zero, while `CAM_BASE_Y` retains the existing f64 expression and camera bit pattern; do not widen the world Y before subtraction or add the datum to the camera twice.

Store `motion_config: SupportMotionConfig::PLAYER_DEFAULT`, `motion_state: MotionState::initial()`, and `support_collider_id: Option<u64>`. In `ready`, set the 1.7 m capsule local Y to `-0.05`, write every solver setting explicitly—including `MotionMode::GROUNDED`, both platform-layer masks `0`, and `PlatformOnLeave::DO_NOTHING`—and apply the controlled pair before the first move. Add no “fault already reported” state: the refusal door below disables processing after its one error.

The player-owned function has the exact signature `fn post_move_support(&self) -> Result<(Option<SupportContact>, Option<u64>), SupportReadError>`. If `is_on_floor()` is false it returns `(None,None)`. Otherwise it scans `0..get_slide_collision_count()` in ledger order; `get_slide_collision(index) == None` is `SupportReadError::MissingCollision(index)`. For each entry it constructs `SupportContact::try_new(collision.get_position(), collision.get_normal())?` before angle arithmetic, widens the validated normal lanes, and accepts the floor predicate `acos(clamp(dot(normal, UP)/(length(normal)), -1, 1)) <= f64::from(FLOOR_MAX_ANGLE_RAD)`. Non-floor entries continue. A floorish entry must have `collision.get_collider_rid().is_valid()` or returns `InvalidRid(index)`; query `PhysicsServer3D::singleton().body_get_collision_layer(rid)`, continue on either actor bit, and otherwise return the contact plus `NonZeroU64::new(collision.get_collider_id()).map(NonZeroU64::get)`. Exhausting the ledger returns `(None,None)`. `SupportReadError` is a private `Display` enum with `MissingCollision(i32)`, `InvalidRid(i32)`, and `InvalidValue(MotionValueError)`; `From<MotionValueError>` maps the last variant. It never silently turns poisoned facts into air. A rejected actor collision cannot hide a later world floor, while server-backed geometry needs no object cast and a zero object ID becomes `None`. Collider identity never enters behavior or capture.

`desired_planar_velocity` returns `Result<PlanarVelocity, MotionValueError>` and validates the complete transformed world vector. `support_elevation_y` returns `Result<SupportElevation, MotionValueError>` after validating the player root as `ActorPosition`. Replace `physics_process` motion with this exact ordering (the small `refuse_motion(error)` helper logs once, zeros velocity, disables physics processing, and has no mutable latch):

```rust
let transform_before = match ActorTransform::try_new(self.base().get_global_transform()) {
    Ok(value) => value,
    Err(error) => { self.refuse_motion(error); return; }
};
let _rotation_before = match FiniteRotation::try_new(self.base().get_global_rotation()) {
    Ok(value) => value,
    Err(error) => { self.refuse_motion(error); return; }
};
let desired = if self.motion_state.accepts_control() {
    match self.desired_planar_velocity() {
        Ok(value) => value,
        Err(error) => { self.refuse_motion(error); return; }
    }
} else {
    PlanarVelocity::ZERO
};
let prepared = prepare(
    self.motion_state,
    desired,
    StepDuration::from_raw(dt),
    self.motion_config,
);
self.base_mut().set_velocity(prepared.command().world_velocity());
self.base_mut().move_and_slide();
let transform_after = match ActorTransform::try_new(self.base().get_global_transform()) {
    Ok(value) => value,
    Err(error) => {
        self.base_mut().set_global_transform(transform_before.world());
        self.refuse_motion(error);
        return;
    }
};
let rotation_after = match FiniteRotation::try_new(self.base().get_global_rotation()) {
    Ok(value) => value,
    Err(error) => {
        self.base_mut().set_global_transform(transform_before.world());
        self.refuse_motion(error);
        return;
    }
};
let _validated_post = (transform_after.position(), rotation_after);
let (support, collider_id) = match self.post_move_support() {
    Ok(value) => value,
    Err(error) => {
        self.base_mut().set_global_transform(transform_before.world());
        self.refuse_motion(error);
        return;
    }
};
let actual_velocity = match ActorVelocity::try_new(self.base().get_velocity()) {
    Ok(value) => value,
    Err(error) => {
        self.base_mut().set_global_transform(transform_before.world());
        self.refuse_motion(error);
        return;
    }
};
let outcome = MotionOutcome::new(actual_velocity, support);
let transition = reconcile(prepared, outcome);
self.motion_state = transition.state;
self.support_collider_id = self.motion_state.support().and(collider_id);
self.apply_collision_pair();
```

Do not sample movement input when airborne. Keep look and queued cane handling after the validated body move. The `MotionState` itself retains any landing observation returned by reconciliation; Task 3 adds its tested effect. Rust-only `try_relocate(world_position: Vector3) -> Result<(), MotionValueError>` first constructs `ActorPosition`; on error it changes no transform, velocity, phase, layer, or support. On success it synchronously installs the validated position, calls `motion_state.relocated()`, clears support identity, writes zero body velocity, and applies controlled layers before returning `Ok(())`. A registered `#[func] relocate(world_position) -> VarDictionary` is the thin Godot test/game door: it returns `{ "relocated": true }` on success or `{ "unavailable": error.to_string() }` on refusal, using the repository's existing verdict convention rather than exposing Rust `Result` through godot-rust. The gdUnit atomicity test calls this wrapper and asserts the exact dictionary plus unchanged body observations.

- [ ] **Step 5: Add validated player Inspector fields and inject before tree entry**

Add the exact six `player_` exports to `UnseeingGame` with custom getters/setters. Use these hints:

```rust
#[export(range = (0.1, 30.0, 0.1, suffix = " m/s²"))]
#[export(range = (0.5, 50.0, 0.5, suffix = " m/s"))]
#[export(range = (0.0, 10.0, 0.1, suffix = " m/s"))]
#[export(range = (0.1, 20.0, 0.1, suffix = " m/s"))]
#[export(range = (0.0, 1.0, 0.01))]
#[export(range = (0.0, 10.0, 0.1, suffix = " m"))]
```

Each field also has an explicit `#[init(val = ...)]` in this order: `9.8`, `20.0`, `1.5`, `4.0`, `0.85`, `5.0`; `#[class(init)]`'s numeric zero is not an acceptable implicit default. The created player field is initialized directly with `SupportMotionConfig::PLAYER_DEFAULT` before injection.

Each setter constructs the complete candidate via `SupportMotionConfig::try_new`. `NonFinite` or `OutOfRange` for the requested `MotionConfigField` rejects that scalar and retains its old value. `ThresholdOrder` silently stages the requested individually range-valid scalar; this is required so Godot can deserialize valid pairs both above and below the defaults in either assignment order. A later complementary setter may make the full candidate valid, while a pair that is still invalid at `ready` produces one error naming both thresholds. Give every exported field an editor-docs comment naming its units, authored purpose, cross-field rule, and construction-time application; extend the existing `editor-docs` tests to assert those descriptions reach the generated XML. Validate the final six staged values before allocating `UnseeingPlayer` and before `add_child`; only the success branch allocates, injects, and inserts the player:

```rust
let player_config = SupportMotionConfig::try_new(
    self.player_fall_acceleration,
    self.player_terminal_fall_speed,
    self.player_landing_silent_speed,
    self.player_landing_full_speed,
    self.player_landing_max_gain,
    self.player_landing_max_range,
).map_err(|error| error.to_string());
let player_config = match player_config {
    Ok(config) => config,
    Err(error) => {
        godot_error!("UnseeingGame: invalid player motion configuration — {error}");
        return;
    }
};
let mut player = UnseeingPlayer::new_alloc();
player.bind_mut().inject_motion_config(player_config);
```

This explicit callback refusal must remain the only invalid-injection branch; do not panic or silently substitute defaults. Bare test players retain the valid associated default. The PackedScene round-trip test packs/instantiates one high pair (`silent = 8`, `full = 9`) and one low pair (`silent = 0.1`, `full = 0.2`), sets them in both orders, and proves all four instances inject those exact values.

Add registered read-only `motion_config_snapshot() -> PackedFloat64Array` on both actor classes (player here, cat in Task 5). It returns exactly six active pure-config authored f64 getters in constructor order; it performs no mutation and is explicitly an observability/test door, not designer configuration. The player/cat PackedScene tests compare this array to hand-written expected values, so reading the root's staged properties cannot masquerade as proof of injection.

- [ ] **Step 6: Run player/config green tests and exact #64 ramp fixture**

Use the already-red #64 geometry from Step 1; do not add or weaken assertions here. Its `1.2 × 0.45 × 1.0 m` platform is centred `1.3 m` farther along the travel axis.

Run exactly:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo test --features editor-docs editor_docs && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
for suite in player_elevation_test movement_test game_root_test knob_hint_test; do
  "$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode -c -a "res://tests/${suite}.gd"
done
```

Expected: all green against the freshly built dylib, one body move per tick, no input steering in air, and exact property documentation.

- [ ] **Step 7: Mutate, review, and commit physical player support**

Prove the named tests fail when restoring `velocity.y = 0`, sampling input in air, retaining desired rather than post-slide X/Z, omitting either post-transform/rotation validation, reconstructing instead of restoring the saved transform, treating collider ID zero as refusal, leaving controlled layers in air, accepting actor support, restoring capsule Y to zero, ignoring injected config, changing each solver/platform property, or adding a second move. Restore green after each mutation. Rebuild release Rust, import, rerun the Task 2 Godot gates, request physics/performance plus code review, and fix verified findings. Lift Task 6 preflight's dormant-player restriction in the same diff, then commit the physical player behavior with its existing format-2 capture fields.

---

### Task 3: Player Silhouette, Footsteps, Landing Voice, and Cane Elevation

**Files:**
- Modify: `rust/src/viewmodel.rs:91-136,340-550`
- Modify: `rust/src/nodes/hero.rs:103-157,245-305`
- Modify: `rust/src/nodes/player.rs:30-40,236-600`
- Modify: `rust/src/observe/mod.rs:105-128`
- Modify: `game/tests/viewmodel_test.gd`
- Modify: `game/tests/footsteps_test.gd`
- Modify: `game/tests/cane_test.gd`
- Modify: `game/tests/player_elevation_test.gd`

**Interfaces:**
- Consumes: Task 2 motion state/config/support datum and Task 1 landing voice.
- Produces: support-relative `viewmodel::leg_pose`; one-consumer player latch; `QueuedWaveGate` and a controlled-contact queue door; support-relative body/cane/footstep origins; reflecting landing emission.

- [ ] **Step 1: Add failing pure and Godot elevation tests**

Change the planned pure signature and add a translation-equivariance test before production:

```rust
pub fn leg_pose(
    p: ActorPosition,
    support: SupportElevation,
    axes: PlanarAxes,
    leg_phase: f64,
    walk_amp: f64,
    side: LegSide,
) -> Result<LegPose, MotionValueError>;
```

Add `PlanarAxes::try_new(forward, right) -> Result<Self, MotionValueError>`; it validates every lane and rejects either zero horizontal direction before normalizing in widened lanes. `LegSide` is the closed `Left | Right` domain. `leg_pose` rejects non-finite phase/amplitude and validates every result as `PosePoint`. The translation test compares support `0.0_f32` and `0.45_f32`: every joint's X/Z bits remain equal and each translated Y is within `2.980_232_238_769_531_25e-8 m` (one f32 ULP at 0.45) of its hand-computed expected Y; existing support-zero cases preserve flat outputs bit-for-bit. Add `leg_pose_rejects_poisoned_axes_phase_and_amplitude_without_output` and the actor-envelope boundary case.

Add Godot cases `test_raised_support_translates_every_player_body_vertex_once`, `test_shoes_and_footstep_origin_follow_platform_height`, `test_camera_inherits_root_height_exactly_once`, `test_airborne_planar_trajectory_does_not_drive_walk_or_steps`, `test_footstep_queued_before_edge_is_consumed_without_emission`, `test_always_wave_queued_before_edge_still_emits`, `test_small_player_drop_retains_landing_but_emits_nothing`, `test_audible_player_landing_uses_support_normal_and_relative_origin_once`, `test_high_player_drop_caps_gain_and_range`, `test_zero_player_landing_gain_consumes_no_pulse_or_echo`, `test_zero_player_landing_range_consumes_no_pulse_or_echo`, `test_landing_tick_has_one_landing_voice_and_no_regular_step`, `test_suppression_survives_multiple_physics_ticks_before_hero_update`, `test_landing_acknowledgement_allows_next_controlled_footstep`, `test_poisoned_player_visual_sample_retains_mesh_vm_queue_and_suppression`, `test_cane_rest_follows_an_elevated_player`, `test_elevated_table_is_classified_relative_to_the_player`, `test_air_sweeping_target_follows_the_falling_player`, and `test_look_and_tap_remain_live_while_airborne`.

Give legacy cane fixtures a small pedestal only under the capsule; it must not extend to the cane tip 1.7 m ahead, preserving their intentional unsupported-tip cases.

- [ ] **Step 2: Witness the visual/cane/voice red failures**

Run:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test viewmodel && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
for suite in player_elevation_test viewmodel_test footsteps_test cane_test; do
  "$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode -c -a "res://tests/${suite}.gd"
done
```

Expected: player root/collider move but body legs, waves, and cane still use absolute world zero; landing suppression/voice tests fail.

- [ ] **Step 3: Make the player body support-relative exactly once**

In `leg_pose`, use `support.y() + 0.90`, ankle floor `support.y() + 0.07`, and shoe floor `support.y() + 0.065`. At the very start of `HeroBody::update`, before mutating `Viewmodel`, bob, sweep, buffers, meshes, shoes, queue, or suppression, build a private `HeroVisualSample`. It validates `now`, capped `dt`, player `ActorPosition`, `SupportElevation`, `ActorVelocity`, complete player/camera transforms and rotations, flattened `PlanarAxes`, tap target, cane-rest tip, and finite `last_tap`.

Add retained `next_cane_buf` and `next_body_buf` scratch buffers beside the installed buffers and compute this complete next value off-copy:

```rust
struct HeroVisualNext {
    vm: Viewmodel,
    suppression: FootstepSuppression,
    bob: f64,
    cane_sweep: f64,
    shoes: [Vector3; 2],
    cane_vertices: LimbBuf,
    body_vertices: LimbBuf,
    queue_footstep_at: Option<Vector3>,
}
```

Move the two retained scratch buffers into the candidate (leaving their prior capacities available), copy the current VM and suppression, advance/acknowledge only those copies, and calculate the complete `HeroVisualNext`. Validate every derived point before touching an installed buffer, mesh, player command, or queue. On success, one infallible commit swaps both candidate buffers into the installed slots (the old installed allocations become the next scratch buffers), resizes both meshes, installs VM/shoes **and `HeroVisualNext.suppression`**, and calls one player door that installs bob/cane sweep and appends the optional prevalidated footstep. On error, return the work buffers to the scratch slots and change nothing else; VM, meshes, shoes, bob, cane request, queue, and suppression remain bit-identical. Thus VM, suppression, geometry, and the complete queue delta are one prepared next state, with no fallible operation after the first write and no steady-state allocation. The prewritten acknowledgement regression proves pending clears exactly once and a later ordinary controlled footstep emits normally. `build_body` uses the sample's support, puts torso/pelvis at `support.y() + 0.90/1.28`, and passes typed position/axes/support into both leg calls. Do not change camera local Y.

Add `footstep_suppression: FootstepSuppression` initialized to `CLEAR` to `UnseeingPlayer`. In the physics callback, save `transition.landing`, store `transition.state`, update accepted support identity, apply the resulting collision pair, then replace the suppression value with `footstep_suppression.on_transition(transition.landing)`; only a fresh landing calls `emit_landing`. At `HeroBody::update`, feed zero planar animation speed while airborne. Acknowledge the persistent value only after a real `Viewmodel` has reached its footstep evaluation:

```rust
let controlled = player.bind().motion_state().accepts_control();
let planar_speed = if controlled {
    f64::from(Vector2::new(velocity.x, velocity.z).length())
} else {
    0.0
};
// Viewmodel advance/look/cane remain live, but only the copies change here.
let Some(mut next_vm) = self.vm else { return; };
let suppression = player.bind().footstep_suppression();
let (next_suppression, suppress_landing_step) = suppression.acknowledge();
let fired = next_vm.footstep(dt, pose.moving && !suppress_landing_step);
```

Consume Task 1's `QueuedWaveGate`; do not define a node-local duplicate. `WaveRequest` stores this gate. Existing `queue_wave` uses `Always`; add a narrow `queue_footstep(at, ...)` used only by `HeroBody`, which stores `ControlledContact`. Queue the ordinary footstep at `Vector3::new(shoe.x, support_y + 0.04, shoe.z)` through that door. After the physics move/reconciliation, drain the queue with:

```rust
for request in std::mem::take(&mut self.wave_queue) {
    if !request.gate.allows(
        phase_before,
        transition.state.phase(),
        transition.landing,
    ) {
        continue;
    }
    self.emit_request(request, now, &space);
}
```

This consumes a stale pre-edge shoe request silently while preserving general/demo requests; never infer provenance from kind/range/gain. The player suppression value is captured through `pending()` and is never acknowledged by a physics tick. The earlier Task 6 schema already carries the gate through every canonical/wire/restore path.

- [ ] **Step 4: Emit one authored player landing voice**

`emit_landing` first calls `landing_voice`. On `None`, return before acquiring physics space or calling either emitter. Otherwise call the existing `emit_reflecting` in the landing physics tick with kind 2, support contact point plus `(0,0.04,0)`, range/gain from `LandingVoice`, speed `4.0`, two echoes, and the event support normal. The transition's inert `last_landing` remains present even when silent.

Pin small-drop silence, chair-height audibility, high-drop cap, exact one emission, and zero configured gain/range consuming neither pulse nor echo capacity. Configure player variants through a fixture `UnseeingGame` root, not an artificial player Inspector.

- [ ] **Step 5: Translate every cane vertical law by the player datum**

Use `support_y = global_player_y - PLAYER_STANDING_ROOT_Y as f32` at the boundary and replace all absolute tests/endpoints:

```text
wall scan                 support_y + 0.85
down probe top/bottom     support_y + 1.05 / support_y - 0.10
unsupported fallback      support_y + 0.02
raised classification     rest.tip.y > support_y + 0.15
floorish aimed hit        hit.y < support_y + 0.20
swish target              support_y + clamp(EYE + tan(pitch)*1.5, 0.3, 1.7)
```

The camera-derived aim ray is already world-elevated and remains unchanged. Cane rays remain the only existing player support-related queries; add no query for body motion.

- [ ] **Step 6: Run green regressions, mutate, review, and commit player effects**

Run exactly:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
for suite in player_elevation_test viewmodel_test footsteps_test cane_test; do
  "$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode -c -a "res://tests/${suite}.gd"
done
```

Mutate separately: visual sample guard; mutate the copied VM before a forced derived-point refusal; clear/swap one installed buffer before validation; acknowledge suppression early; zero torso support; zero leg support/clamps; double-lift camera; feed airborne planar speed; restore absolute footstep Y; emit a pre-edge queued contact in air; restore each cane Y literal; emit a silent landing. Each named test must fail. Request implementation plus wave/performance review and restore all green gates. Lift Task 6 preflight's dormant suppression/gate restriction in the same diff, then commit the player visual/contact/landing behavior.

---

### Task 4: Cat Gait, Skeleton, and Tail Share One Elevation

**Files:**
- Modify: `rust/src/cat_brain.rs:121-170,208-365,430-680`
- Modify: `rust/src/cat_gait.rs:128-353,355-700`
- Modify: `rust/src/cat_body.rs:102-237,284-365,400-700`
- Modify: `rust/src/nodes/cat.rs:120-230,350-405`
- Modify: `rust/src/nodes/restorer.rs:245-280`
- Modify: `game/tests/cat_test.gd`
- Modify: `game/tests/restore_transaction_test.gd`

**Interfaces:**
- Consumes: Task 1 `ActorPosition`, `SupportElevation`, `ActorYaw`, `FiniteMeasure`, and `StepDuration`; malformed raw vectors never enter the gait/body law.
- Produces: validated `RoamRect`/typed brain, support-aware `CatGait`, `GaitFrame.support_delta_y: f32`, support-relative `skeleton`, and total `Tail::transport_y(f32) -> Result<(), MotionValueError>`. The earlier Task 6 schema already carries explicit `GaitCapture.support_y`.

- [ ] **Step 1: Add failing gait transport/capture tests**

Add `height_change_transports_every_planted_paw_and_swing_aim`, `elevated_swing_and_settle_never_return_to_world_zero`, `captured_gait_restores_elevated_state_in_lockstep`, `adversarial_fractional_height_round_trip_has_no_future_ulp_shift`, `extreme_valid_root_produces_pose_points_inside_the_derived_envelope`, `roam_rect_accepts_last_exact_safe_edge_and_rejects_adjacent_excess`, `rounded_brain_target_round_trips_after_margin_sampling`, `brain_restore_rejects_out_of_envelope_rect_or_target`, `malformed_captured_point_phase_or_amplitude_refuses_gait_restore`, `poisoned_brain_capture_refuses_checked_restore`, `poisoned_tail_capture_refuses_checked_restore`, `brain_and_tail_typed_steps_reject_invalid_inputs_without_mutating_prior_state`, and `zero_support_delta_preserves_flat_lane_bits`.

The first test creates a gait at Y `0`, advances to a mid-swing capture, advances at the same X/Z and Y `0.75`, then asserts all `planted`/`aim` Y lanes equal the new `0.75_f32` bits and X/Z bits did not change. The adversarial case uses old Y `0.146_820_16_f32` and new Y `-0.440_136_85_f32`, captures/restores the format-2 explicit support scalar, advances again at the same new Y, and requires every lane bit to remain unchanged.

- [ ] **Step 2: Run every changed pure domain red**

Run `(cd rust && cargo test cat_gait::tests)`, `(cd rust && cargo test cat_brain::tests)`, and `(cd rust && cargo test cat_body::tests)`. Expected: elevation cases return world zero/omit the datum and the new typed/checked `new`, `advance`, and `restore` APIs do not yet exist. A failure in unrelated existing behavior is not the intended red.

- [ ] **Step 3: Implement one f32 support datum and exact transport**

Change `CatGait::new/advance` to consume `ActorPosition` and `ActorYaw`; `advance` also consumes `StepDuration` and `FiniteMeasure`. Change `CatBrain::new/advance` to the same typed position/yaw/duration/progress doors and make `Drive` carry `ActorYaw` plus `FiniteMeasure` speed. Replace public-field `RoamRect` construction with private fields and exact checked doors:

```rust
impl RoamRect {
    pub fn try_around(center: ActorPosition, size: Vector2) -> Result<Self, MotionValueError>;
    pub fn try_restore(min_x: f64, min_z: f64, max_x: f64, max_z: f64)
        -> Result<Self, MotionValueError>;
    pub fn contains_target(self, tx: f64, tz: f64) -> bool;
}
```

Both size lanes must be finite in `1.0..=30.0`; compute min/max in f64 and require ordered edges inside `±MAX_ACTOR_COORD_M`. `contains_target` requires finite X/Z inside both the actor envelope and the raw stored rectangle, inclusive. It deliberately does not reapply `WALL_MARGIN`: `pick_target` samples inside that margin and then rounds to a `0.1 m` grid, so a lawful self-produced target may sit just beyond the pre-rounding inner interval while remaining inside the raw rectangle. The exact-safe test uses centre X `MAX_ACTOR_COORD_M - 15.0` with size X `30.0`; the adjacent higher f32 centre rejects. The prewritten seeded case uses centre X `0.08`, size X `1.0`, and a sampled value near `0.16` that rounds to `0.20`; its self-produced capture must prepare/restore. Restore accepts either raw-rectangle edge and rejects the adjacent value outside it or the actor envelope. `CatBrain::prepare_restore` validates the rect plus every `BrainState::Roam` target through this policy.

Change `Tail::new/advance` to validated `PosePoint`/`ActorYaw`/`StepDuration` inputs and return `Result` before mutating nodes; change `Tail::restore` to validate every node as `PosePoint`; `transport_y` returns `Result` and rejects an invalid raw delta before mutation. Add `support: SupportElevation` to `CatGait`. Task 6 has already added `GaitCapture.support_y`; change `CatGait::restore` to validate every stored point, phase in `[0,1)`, amplitude in `[0,1]`, swing/moving consistency, and the explicit datum before constructing state. At the start of every advance:

```rust
fn transport_support(&mut self, new_position: ActorPosition) -> f32 {
    let new_support = new_position.elevation();
    let delta_y = new_support.delta_from(self.support);
    let new_support_y = new_support.y();
    for point in &mut self.planted { point.y = new_support_y; }
    for point in &mut self.aim { point.y = new_support_y; }
    self.support = new_support;
    delta_y
}
```

The implementation has no panic site. Return the delta in `GaitFrame`. Build anchors at the validated position Y, swing at `support.y() + lift`, and settle at `support.y()`; direct Y assignment preserves the exact new datum and a positive-zero datum keeps the existing flat lane bits. Consume Task 6's existing `GaitCapture.support_y`: `CatGait::restore` constructs `SupportElevation::try_new(capture.support_y)`, requires every planted/aim Y lane to match it, and returns `Result` on malformed or contradictory points.

- [ ] **Step 4: Add failing skeleton/tail translation tests**

Add `elevated_skeleton_is_the_flat_skeleton_translated_once`, `elevated_sit_keeps_every_joint_above_its_support`, `tail_transport_preserves_the_curve_before_following`, `tail_follow_is_translation_equivariant`, `tail_transport_rejects_nonfinite_delta_without_poisoning_nodes`, `tail_transport_rejects_finite_boundary_overflow_without_mutation`, `cat_ready_rejects_poisoned_position_yaw_and_roam_size_before_brain_construction`, `cat_physics_rejects_poisoned_pre_or_post_move_sample_without_advancing_brain_gait_tail_or_waves`, and restore-transaction `test_cat_body_y_and_gait_support_y_mismatch_leaves_world_untouched`.

Compare every chest/hip/head/ear/whisker/leg/tail-root point at support Y `0` and `0.75`: X/Z bit-equal, Y difference exactly `0.75` within one f32 ULP.

Run exactly to witness independent reds before Step 5:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test cat_body::tests && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
for suite in cat_test restore_transaction_test; do
  "$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode -c -a "res://tests/${suite}.gd"
done
```

The poisoned ready/physics cases and `test_cat_body_y_and_gait_support_y_mismatch_leaves_world_untouched` must fail for their named reasons before boundary/preflight production changes.

- [ ] **Step 5: Implement support-relative skeleton and bounded tail transport**

Keep `CatPose`'s existing public raw capture fields for format-2 wire compatibility, but construct runtime values through checked `try_from_gait(ActorPosition, ActorYaw, &GaitFrame, sit) -> Result<Self, MotionValueError>`. It validates every paw/contact as `PosePoint`, phase in `[0,1)`, amplitude/sit in `[0,1]`, and finite bob before output. Make `skeleton(&CatPose) -> Result<Skeleton, MotionValueError>` total over the still-public input: repeat the pose-domain validation before arithmetic and validate every derived joint against `MAX_POSE_COORD_M`. The cat adapter handles the result before installing pose/mesh; Task 6's prepared parser already validates the raw capture representation. Use `let ground = pose.pos`; seated paw targets use `ground.y`, never zero. Add:

```rust
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
```

The prewritten finite-boundary test places one valid node at `MAX_POSE_COORD_M`, applies the adjacent positive finite delta, and proves `Err` with every original node bit unchanged. The future adapter calls transport exactly once before the existing tail follow law and propagates an error to the same exact rollback door. Use the checked `RoamRect` at the cat boundary. In `WaveCat::ready`, validate position, complete global transform/rotation, and roam size before constructing brain/gait/tail. In the existing planar physics callback, validate the full pre-move transform/rotation, `last_pos`, and capped duration before taking or advancing components; validate the full post-move transform/rotation and velocity before gait/pose/tail/waves. A pre-move refusal disables physics/process with one error and changes no pure component. A poisoned post-move sample restores the exact saved `Transform3D` bits, zeroes velocity, restores the pre-advance brain/gait/tail values, disables processing, and emits no wave. Task 6 already made restore commit consume owner-prepared values; strengthen preflight in this same Task 4 commit to require cat body position and `CatPose.pos` bit equality on X/Y/Z, exact body-Y/`GaitCapture.support_y` bits, every planted/aim Y equal to the same datum, and a prepared brain whose rect/target passes the actor envelope. The mismatch test must be red before this Task 4 strengthening and green before commit. Task 6 validates only the dormant gait datum against its own planted/aim lanes, because an elevated pre-Task-4 cat lawfully has body Y above the old world-zero gait datum.

- [ ] **Step 6: Green, mutate, review, and commit the cat-pose behavior**

Run exactly:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test cat_gait::tests && cargo test cat_body::tests && cargo test cat_brain::tests && cargo test && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
for suite in cat_test restore_transaction_test; do
  "$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode -c -a "res://tests/${suite}.gd"
done
```

Mutate planted/aim Y assignment, swing/settle Y, support-bit checks, root/pose/roam envelope, captured target, skeleton ground, either seated paw, tail translation count, pre/post transform or rotation guard, exact rollback, and NaN guard one at a time; witness the named failure. Request pure-component/boundary/code review, fix verified findings, rerun every command, then commit only these Task 4 files; Task 6's already-landed schema captures the changed support datum.

---

### Task 5: Cat Physical Adapter, Inspector Controls, and Elevated Voices

**Files:**
- Create: `game/tests/cat_elevation_test.gd`
- Modify: `rust/src/nodes/cat.rs:21-275,277-455`
- Modify: `game/tests/cat_test.gd`
- Modify: `game/tests/knob_hint_test.gd`
- Modify: `game/tests/character_elevation_fixture.gd`
- Create: `game/tests/actor_support_test.gd`
- Modify: `game/tests/probe/editor_source_probe.gd`
- Modify: `tools/probe_editor_sources.sh`

**Interfaces:**
- Consumes: Tasks 1, 2 shared layer constants, and Task 4 gait/body/tail outputs.
- Produces: cat-owned solver/layer/support adapter; six per-cat exported settings; brain-safe airborne motion; elevated paw/presence/landing voices; cat motion and exact restore doors. Scene construction and restore are the only cat placement paths in this version.

- [ ] **Step 1: Add failing Inspector/datum/elevation tests**

Add exact ClassDB hint assertions for all six `WaveCat` fields and cases for independent cats, nonfinite/out-of-range scalar rejection, high/low silent-full pairs packed and instantiated in both assignment orders, an invalid final pair refusing before motion, `test_invalid_cat_threshold_pair_reaches_virtual_and_callable_warning_channels`, `test_valid_complementary_threshold_edit_clears_both_warning_channels`, the runtime capsule bottom at root, `test_cat_solver_contract_is_explicit_on_every_property`, and flat root staying exactly Y zero with `is_on_floor()` true. The solver case hand-asserts all eleven properties listed by the spec, not only platform masks. Before adapter production, add private `CatMotionPort` trace tests: `cat_valid_tick_calls_move_and_slide_once` records exactly one move command, while `cat_post_move_poison_writes_exact_saved_transform_then_zero_velocity_then_disables` poisons every post-transform/rotation lane in turn and records that exact rollback trace with no component/layer/wave command. The production port forwards to `CharacterBody3D`; the fake remains Rust-test-only.

Extend `editor_source_probe.gd` before production so its editor branch casts `CatCollider` to `CollisionShape3D`, casts its shape to `CapsuleShape3D`, checks `collider.position.y - capsule.height * 0.5 == 0.0` within one f32 ULP, stages an invalid threshold pair, and checks the exact same warning text through both `get_configuration_warnings()` and the registered callable forwarder before a complementary valid edit clears both. Raise the editor expected count in `probe_editor_sources.sh` from 11 to 15 (datum, virtual warning, callable warning, clear); run mode retains its three checks. Run the probe now: editor mode must fail at the old `+0.02 m` datum and missing warning contract.

In `cat_elevation_test.gd`, add `test_floor_cat_keeps_root_collider_paws_and_skin_together`, `test_stationary_cat_stands_on_table_support`, `test_stationary_cat_stands_on_bed_support`, `test_walking_elevated_cat_transports_paws_tail_and_voices`, `test_cat_walks_off_a_platform_with_fixed_trajectory`, `test_airborne_cat_keeps_brain_and_yaw_frozen`, `test_airborne_cat_policy_produces_no_yaw_command`, `test_airborne_cat_gait_uses_achieved_displacement`, `test_first_resumed_brain_tick_receives_zero_flight_progress`, `test_zero_negative_and_nonfinite_cat_dt_keep_zero_actual_speed_without_fault`, `test_airborne_presence_origin_follows_root_height`, `test_airborne_and_landing_ticks_emit_no_paw_voice`, `test_cat_wall_contact_removes_only_blocked_trajectory_without_a_wave`, `test_no_floor_cat_stays_finite_at_terminal_speed`, `test_cat_ramp_up_and_down_never_lands`, `test_poisoned_cat_pre_move_sample_disables_without_advancing_any_component`, `test_poisoned_cat_post_move_sample_rolls_back_without_wave_or_partial_component`, `test_cat_landing_is_silent_at_threshold`, `test_cat_landing_is_audible_above_threshold`, `test_cat_landing_caps_gain_and_range`, `test_zero_cat_landing_gain_emits_nothing`, and `test_zero_cat_landing_range_emits_nothing`.

Create `actor_support_test.gd` in this same red step with `test_two_controlled_actors_block_each_other_on_world_floor`, `test_controlled_actors_at_different_elevations_do_not_create_contact`, `test_centred_airborne_cat_passes_through_player_and_lands_on_world`, and `test_controlled_player_walking_off_world_onto_cat_rejects_actor_support`. The current planar cat/default layers must fail these before the cat adapter completes the pair.

- [ ] **Step 2: Witness old cat red behavior**

Run:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test cat_valid_tick_calls_move_and_slide_once)
(cd rust && cargo test cat_post_move_poison_writes_exact_saved_transform_then_zero_velocity_then_disables)
(cd rust && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
for suite in cat_elevation_test cat_test knob_hint_test actor_support_test; do
  "$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode -c -a "res://tests/${suite}.gd"
done
GODOT="$GODOT" tools/probe_editor_sources.sh
```

Expected: cat Y velocity/root stay elevated, voices/collider motion remain old, threshold/config/landing behavior does not exist, and actor-pair cases fail against the planar/default-layer cat. These are the required reds for Task 5.

- [ ] **Step 3: Add per-cat validated exported controls and exact collider datum**

Add fields/getter-setter pairs named `fall_acceleration`, `terminal_fall_speed`, `landing_silent_speed`, `landing_full_speed`, `landing_max_gain`, and `landing_max_range`, with the same ranges/suffixes as player fields and cat defaults `9.8, 20.0, 1.5, 4.0, 0.60, 2.5`.

Put explicit `#[init(val = ...)]` on those six f64 fields in exactly that order and initialize private active `motion_config` with `SupportMotionConfig::CAT_DEFAULT`; never rely on `#[class(init)]` zeroes.

Keep the six exported authored scalars separate from the always-valid active `motion_config`. Each setter tries the complete candidate: a `NonFinite`/`OutOfRange` for the requested field rejects that scalar; `ThresholdOrder` stages the individually valid scalar, retains the active config, stores an editor configuration warning naming both thresholds, and calls `update_configuration_warnings()`; a valid candidate stages the scalar, installs the new active config, and clears that warning. Add the `ICharacterBody3D::get_configuration_warnings` virtual plus the callable forwarder so the existing tool node shows the triangle. At runtime `ready`, validate the final authored six before enabling motion; an invalid final pair reports `WaveCat: invalid motion configuration — {error}` and disables physics/process without silently using the last active default. Editor mode may still build its nonmoving blueprint while showing the warning.

Give every field an editor-docs comment naming its units, authored purpose, cross-field rule, and active-vs-authored staging behavior, and assert those descriptions in generated XML. The PackedScene tests use the same high/low pairs and both assignment orders as the player tests, and prove independent cats install the exact final active configs. Both runtime and editor blueprint colliders use `COLLIDER_CENTER_Y = COL_HEIGHT * 0.5` (`0.17 m`), removing the old `+0.02`.

Implement the same registered read-only `motion_config_snapshot() -> PackedFloat64Array` contract defined in Task 2 and assert the six active cat values through it; exported authored getters alone are not injection evidence.

- [ ] **Step 4: Implement the cat-owned two-phase Godot adapter**

Store `motion_state: MotionState::initial()` and `support_collider_id: Option<u64>`; add no report latch. Configure all eleven solver values from the global table before editor/runtime branches and apply the phase-derived pair only when it changes.

The cat-owned support door is `fn post_move_support(&self) -> Result<(Option<SupportContact>, Option<u64>), SupportReadError>`. False `is_on_floor()` returns `(None,None)`. Otherwise scan every slide entry in ledger order; missing entry is `MissingCollision(index)`. Validate point/normal, compare the widened normalized dot angle with `FLOOR_MAX_ANGLE_RAD`, continue on a non-floor entry, require a valid collider RID, and obtain its layer through `PhysicsServer3D::body_get_collision_layer`. Continue on either actor bit; otherwise return the contact and `NonZeroU64::new(get_collider_id()).map(NonZeroU64::get)`. Exhaustion returns `(None,None)`. The private `Display` error variants are `MissingCollision(i32)`, `InvalidRid(i32)`, and `InvalidValue(MotionValueError)`. No object-class cast or poisoned-fact skip is permitted; a zero object ID remains valid support with absent observation identity.

Add a private pure `CatControlPolicy::{AdvanceBrain,Frozen { yaw: ActorYaw, sitting: bool }}` selector over `(MotionPhase, ActorYaw, Mood)`. Only `AdvanceBrain` may call `CatBrain::advance` and produce `Some(yaw)`; `Frozen` always produces `PlanarVelocity::ZERO` and `yaw_command = None`. The named policy test pins that no airborne path can invoke a yaw setter even when writing the old yaw would appear observationally harmless.

Physics ordering is exact:

```rust
let duration = StepDuration::from_raw(dt);
let transform_before = match ActorTransform::try_new(self.base().get_global_transform()) {
    Ok(value) => value,
    Err(error) => { self.refuse_motion(error); return; }
};
let before = transform_before.position();
let prior = match ActorPosition::try_new(self.last_pos) {
    Ok(value) => value,
    Err(error) => { self.refuse_motion(error); return; }
};
let rotation_before = match FiniteRotation::try_new(self.base().get_global_rotation()) {
    Ok(value) => value,
    Err(error) => { self.refuse_motion(error); return; }
};
let body_yaw = rotation_before.yaw();
let (mut brain, mut gait, mut tail) = match
    (self.brain.take(), self.gait.take(), self.tail.take())
{
    (Some(brain), Some(gait), Some(tail)) => (brain, gait, tail),
    (brain, gait, tail) => {
        self.brain = brain;
        self.gait = gait;
        self.tail = tail;
        self.refuse_motion(MotionValueError::inconsistent_state("cat.components"));
        return;
    }
};
let phase_before = self.motion_state.phase();
let brain_before = brain;
let gait_before = gait.clone();
let tail_before = tail;

let policy = cat_control_policy(self.motion_state.phase(), body_yaw, brain.mood());
let (desired, yaw, sitting, yaw_command) = match policy {
CatControlPolicy::AdvanceBrain => {
    let drive = brain.advance(duration, before, before.planar_distance(prior));
    let yaw = drive.yaw;
    let speed = drive.speed;
    let desired = match PlanarVelocity::try_new(
        (-yaw.radians().sin() * speed.value()) as f32,
        (-yaw.radians().cos() * speed.value()) as f32,
    ) {
        Ok(value) => value,
        Err(error) => {
            self.restore_components(brain_before, gait_before, tail_before);
            self.refuse_motion(error);
            return;
        }
    };
    (desired, yaw, drive.sitting, Some(yaw))
}
CatControlPolicy::Frozen { yaw, sitting } => {
    (PlanarVelocity::ZERO, yaw, sitting, None)
}
};

// Every fallible pre-move conversion is complete. Only now may yaw/body mutate.
if let Some(command) = yaw_command {
    self.set_world_yaw(command.godot_lane());
}

let prepared = prepare(self.motion_state, desired, duration, self.motion_config);
self.base_mut().set_velocity(prepared.command().world_velocity());
self.base_mut().move_and_slide();
let transform_after = match ActorTransform::try_new(self.base().get_global_transform()) {
    Ok(value) => value,
    Err(error) => {
        self.rollback_motion(transform_before, brain_before, gait_before, tail_before, error);
        return;
    }
};
let rotation_after = match FiniteRotation::try_new(self.base().get_global_rotation()) {
    Ok(value) => value,
    Err(error) => {
        self.rollback_motion(transform_before, brain_before, gait_before, tail_before, error);
        return;
    }
};
let new_position = transform_after.position();
let _validated_post_rotation = rotation_after;
let (support, collider_id) = match self.post_move_support() {
    Ok(value) => value,
    Err(error) => {
        self.rollback_motion(transform_before, brain_before, gait_before, tail_before, error);
        return;
    }
};
let actual_velocity = match ActorVelocity::try_new(self.base().get_velocity()) {
    Ok(value) => value,
    Err(error) => {
        self.rollback_motion(transform_before, brain_before, gait_before, tail_before, error);
        return;
    }
};
let outcome = MotionOutcome::new(actual_velocity, support);
let transition = reconcile(prepared, outcome);
```

Continue with an explicit zero-duration branch; the named test was already red in Step 1:

```rust
let actual_speed = if duration.seconds() == 0.0 {
    FiniteMeasure::ZERO
} else {
    match FiniteMeasure::try_new(
        new_position.planar_distance(before).value() / duration.seconds(),
        "cat.actual_speed",
    ) {
        Ok(value) => value,
        Err(error) => {
            self.rollback_motion(transform_before, brain_before, gait_before, tail_before, error);
            return;
        }
    }
};
```

Compute a local gait frame, pose, skeleton, translated/advanced tail, sit, and `sim_t` through checked pure APIs. Any error takes the same rollback before pulse/cadence/field/layer mutation. Only after all succeed install transition state, identity, layers, components, pose/sit/time/last position, then apply voices and mark the mesh dirty. `rollback_motion` restores exact pre-move `Transform3D` bits, zeros velocity, restores components, disables physics/process, and logs once by virtue of disabling; capture of a disabled runtime actor refuses. Do not call `CatBrain::advance` or any yaw setter in air. Advance gait from achieved displacement in both phases, transport tail once, and set `last_pos = new_position.world()` throughout air/landing so resumed brain progress excludes flight.

Use the same `StepDuration`/`duration.seconds()` for `CatBrain::advance`, achieved-speed division, `CatGait::advance`, sit easing, checked `sim_t`, and `Tail::advance`. When elapsed is zero, achieved speed is exactly zero; invalid/raw oversized dt never enters those pure components. The airborne brain-freeze test boots a full `UnseeingGame` cat fixture and compares the existing canonical `blob.cats[0].brain` dictionary on every airborne tick, so advancing only a hidden countdown or RNG word is caught even when mood/yaw happen not to change.

- [ ] **Step 5: Gate and elevate cat voices, then emit one landing**

For each paw contact, call `QueuedWaveGate::ControlledContact.allows(phase_before, transition.state.phase(), transition.landing)`; emit only on true. Do not duplicate the phase boolean law in the adapter. Paw origin is `contact.at + (0,0.02,0)`. Presence continues in all phases at `new_position.world() + (0,PRESENCE_HEIGHT,0)`.

For `transition.landing`, call `landing_voice`; on `Some`, call the existing direct `emit` once with kind 2, support point plus `(0,0.02,0)`, returned range/gain, speed `4.0`, omni direction/cos-half. A silent/zero output keeps `last_landing` but never calls `emit` and schedules no echo.

- [ ] **Step 6: Run green cat behavior and preserve existing movement**

Run exactly:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test cat_gait::tests && cargo test cat_body::tests && cargo test cat_brain::tests && cargo test --features editor-docs editor_docs && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
for suite in cat_test cat_elevation_test actor_support_test knob_hint_test; do
  "$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode -c -a "res://tests/${suite}.gd"
done
GODOT="$GODOT" tools/probe_editor_sources.sh
```

Specifically compare brain capture/yaw each airborne tick, prove gait continues from actual displacement, prove the first supported brain tick sees no whole-flight progress, preserve the existing flat paw cadence/voice constants, and make all four cross-actor fixtures green against the fresh dylib.

- [ ] **Step 7: Mutate, review, and commit the cat adapter**

Mutate planar-only velocity, brain advance in air, airborne yaw replacement, desired-air steering, discarded post-slide X/Z, stale `last_pos`, actor support acceptance, every solver/platform property, runtime collider `+0.02`, editor collider `+0.02`, threshold staging/order, warning virtual/forwarder/clear paths, absolute paw/presence Y, gated contacts, severity cap/threshold, zero emitter, and player defaults substituted for cat defaults. Restore each after its named failure. Rebuild release Rust, import, rerun every Task 5 suite/probe, request cat-movement, physics/performance, and code review, and fix verified findings. Lift Task 6 preflight's dormant-cat restriction in the same diff, then commit the cat motion/perception behavior against the already-final format-2 schema.

---

### Task 6: Land the Motion-Aware Format and Atomic Restore Before Behavior

**Files:**
- Modify: `rust/src/reproduce/blob.rs:38-190,198-910,1000-1610`
- Modify: `rust/src/reproduce/mod.rs:18-36`
- Modify: `rust/src/pulse_pool.rs:72-190`
- Modify: `rust/src/echo_queue.rs:25-125`
- Modify: `rust/src/viewmodel.rs:140-310`
- Modify: `rust/src/cat_brain.rs:185-370`
- Modify: `rust/src/cat_body.rs:100-330`
- Modify: `rust/src/sound_source.rs:220-330`
- Modify: `rust/src/temporal.rs:1-55`
- Modify: `rust/src/demo_tap.rs:20-90`
- Modify: `rust/src/flicker.rs:50-165`
- Modify: `rust/src/observe/mod.rs:105-128`
- Modify: `rust/src/ffi.rs:490-515`
- Modify: `rust/src/nodes/source.rs:105-200`
- Modify: `rust/src/nodes/fan.rs:311-395`
- Modify: `rust/src/nodes/radio.rs:270-340`
- Modify: `rust/src/nodes/hero.rs:282-305`
- Modify: `rust/src/cat_gait.rs:173-230`
- Modify: `rust/src/nodes/observer.rs:32-60,642-702,1280-2280`
- Modify: `rust/src/nodes/restorer.rs:130-320`
- Modify: `rust/src/nodes/game.rs:612-670`
- Modify: `rust/src/nodes/player.rs:600-680`
- Modify: `rust/src/nodes/cat.rs:350-430`
- Modify: `game/tests/restore_test.gd`
- Modify: `game/tests/restore_transaction_test.gd`
- Modify: `game/tests/probe/restore_probe.gd`

**Interfaces:**
- Consumes: Task 1 `MotionState`, `QueuedWaveGate`, `FootstepSuppression`, value validators/default configs, plus the existing observer canonical JSON and restorer transaction.
- Produces: final `FORMAT_VERSION = 2`; dormant initial actor fields; exhaustive motion/suppression/queued-gate/gait-support capture bytes/diff/wire; complete read-only `PreparedRestore` (including environment and stored hash) performed before any write. At this commit both adapters still move exactly as before and preflight admits only their self-produced dormant state; later actor tasks lift their own restriction without changing the wire layout.

- [ ] **Step 1: Change capture fixtures/tests first and witness red**

Change only the exhaustive fixture constructors/mutations to require this expected final shape; do not add the production fields yet. Their compile failure is the red, and Step 2 adds the fields only after that red is recorded:

```rust
pub struct HeroCapture {
    pub position: Vector3,
    pub velocity: Vector3,
    pub motion: MotionState,
    pub yaw: f64,
    pub pitch: f64,
    pub last_tap: f64,
    pub tap_target: Vector3,
    pub tap_queued: bool,
    pub queued_waves: Vec<QueuedWave>,
    pub footstep_suppression_pending: bool,
    pub viewmodel: ViewmodelCapture,
}

pub struct CatCapture {
    pub position: Vector3,
    pub yaw: f64,
    pub velocity: Vector3,
    pub motion: MotionState,
    pub brain: BrainCapture,
    pub gait: GaitCapture,
    pub tail: [Vector3; TAIL_N],
    pub pose: CatPose,
    pub presence_next: f64,
    pub sit: f64,
    pub sim_t: f64,
    pub last_pos: Vector3,
}
```

Add the currently missing `hero.velocity.y` mutation and rows for every phase payload lane, support point/normal lane, last-landing speed/support lane, player suppression bit, each queued wave gate, each cat motion variant, and `gait.support_y`. Add `gate: QueuedWaveGate` to `QueuedWave`; the exhaustive pure fixture deliberately carries one `Always` and one `ControlledContact`, while every live queue door still writes `Always` until Task 3 activates shoe provenance. Reconstruct states through `MotionState::restore` inside mutation closures; add no mutation-only production setters.

Run `cargo test reproduce::blob::tests::every_field_reaches_both_walks`; expected red until encoder and diff both carry each field.

- [ ] **Step 2: Encode/diff the exact canonical layout and bump the version**

After the witnessed compile-red, add the final fields to `HeroCapture`, `CatCapture`, `GaitCapture`, and `QueuedWave`; add the dormant node-owned `motion_state`, `support_collider_id`, and player `footstep_suppression` values. Update every existing Rust constructor/capture door so the tree compiles: live actors write `MotionState::initial()`, clear suppression, and `Always` gates; pre-Task-4 gait capture writes `planted[0].y` and requires planted/aim Y agreement. Existing JSON parse construction temporarily supplies those same dormant values without accepting new keys; this is an explicit uncommitted scaffold whose missing-key behavior is tested red in Step 3 and replaced in Step 4. No format-2 commit exists at this intermediate point.

Use u32 discriminants: phase controlled `0`, airborne `1`; Option none `0`, some `1`. Encode airborne X/Z/Y as f32, support as six f32 lanes, and landing as impact f32 plus its support. Add paired `encode_motion/diff_motion`, `encode_support/diff_support`, and `encode_landing/diff_landing` helpers.

Use fixture variants: airborne hero/no support/old landing; controlled cat/support/no landing; airborne cat/no support/old landing. Encode `QueuedWaveGate` as u32 (`Always = 0`, `ControlledContact = 1`) for each of the fixture's two queued waves. Update the hand-derived layout comment and checked whole fixture length from `5407` to `5564` bytes (`+8` queued-gate bytes, `+53` hero motion/latch bytes, `+40` controlled-cat motion/gait bytes, `+56` airborne-cat motion/gait bytes), then set `FORMAT_VERSION` to `2`. Divergence paths must name `queued_waves[i].gate`, `motion.phase`, `motion.phase.planar_velocity.{x,z}`, `motion.phase.vertical_velocity`, `motion.support...`, `motion.last_landing...`, `footstep_suppression_pending`, and `gait.support_y`.

Run `cargo test reproduce::blob::tests::every_field_reaches_both_walks` again and require it green before Step 3. The binary encoder/diff now has real final fields to consume; JSON is deliberately still incomplete and uncommitted.

- [ ] **Step 3: Add failing JSON round-trip and malformed-state tests**

Canonical dictionaries use:

```json
{"motion":{"phase":{"kind":"controlled"},"support":null,"last_landing":null}}
{"motion":{"phase":{"kind":"airborne","planar_velocity":["-0.0","1.25"],"vertical_velocity":"-3.5"},"support":null,"last_landing":{"impact_speed":"3.5","support":{"point":["1.0","0.45","2.0"],"normal":["0.0","1.0","0.0"]}}}}
{"gate":"always"}
{"gate":"controlled_contact"}
```

All floats are Rust decimal text. Add `Group::f32(key)` that parses directly to f32, rejects nonfinite values, and preserves signed-zero/round-trip lane bits; add an exact two-string `Group::planar_velocity(key)`. Add `Group::optional_group(key)` accepting only NIL or DICTIONARY. Test missing/wrong fields, wrong planar arity/type, unknown phase, an unknown/missing queued gate, nonfinite values, zero normal, positive airborne Y, airborne-with-support, negative landing speed, missing suppression bit, and support-Y poison with exact dotted error paths.

Before any writer/preflight production change, add named restore-transaction reds: `test_invalid_environment_only_leaves_env_pool_actors_sources_and_warning_state_untouched`, `test_wrong_or_malformed_stored_hash_only_leaves_world_untouched`, `test_clamped_hero_pitch_and_lossy_hero_yaw_refuse_before_writes`, `test_lossy_cat_yaw_and_poisoned_brain_refuse_before_writes`, `test_cat_pose_position_mismatch_on_x_or_y_or_z_refuses_before_writes`, `test_cat_gait_internal_support_mismatch_refuses_before_writes`, `test_disabled_runtime_actor_refuses_capture`, and `test_dormant_schema_refuses_airborne_pending_or_controlled_contact_state`. Add a read-only observer diagnostic `canonical_hash_of(blob) -> VarDictionary` that performs syntax parsing plus `state_hash` only, returns the canonical 16-hex value or a dotted parse refusal, and never runs semantic restore validation or writes. Give it a red unit/Godot test. Each atomicity case starts from a valid capture, mutates only the named field, uses this independent diagnostic to install a valid recomputed hash except in the hash test, emits an extra live pulse, records `capture_env()` and the full live hash, calls restore under the exact expected error/no-warning assertion, then proves all recorded observations are bit-identical. These tests are separate: do not hide an environment or hash defect behind an earlier hero mismatch.

Witness the JSON/wire tests red before implementing their parser/writer pairs:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test reproduce::blob::tests)
```

Expected: the binary exhaustive walk remains green, while the new JSON cases fail on the deliberately dormant/defaulting parser or missing writer keys, never on fixture setup.

- [ ] **Step 4: Implement writer/parser pairs and actor capture doors**

Add `motion_dict/parse_motion`, `phase_dict/parse_phase`, `support_dict/parse_support`, `landing_dict/parse_landing`, and queued-gate writer/parser pairs. Constructors enforce invariants and preserve normal/signed-zero bits. Implement the registered read-only `canonical_hash_of(blob)` here as a thin syntax-parse plus `state_hash` wrapper: it returns either the exact lowercase 16-hex hash or the parser's dotted error dictionary, never calls preflight/restore, and never mutates or warns. Replace Step 2's temporary dormant JSON defaults: player capture/parser now carries `motion_state`, `footstep_suppression.pending()`, and every gate; cat capture/parser carries motion; gait carries explicit `support_y` and requires every planted/aim Y lane to equal it. Restorer rebuilds through gate/suppression-preserving doors. Task 4 later replaces the derived runtime gait datum with the typed stored value while leaving this layout untouched. Config and collider ID remain absent.

Add actor restore-capability validation, not an uncaptured flag: at this schema commit, player and cat admit only `MotionState::initial()` bit-for-bit, clear suppression, and live `Always` gates. `capture_state` refuses if either runtime actor has physics/process disabled after a boundary/config fault. The pure fixture/parser still cover every variant. Task 2 replaces only the player-state restriction with `validate_restore`; Task 3 admits pending suppression/controlled-contact gates; Task 5 replaces only the cat-state restriction. Thus every self-produced blob restores at each commit, while a hand-edited future phase cannot be installed before its adapter exists.

Run `(cd rust && cargo test reproduce::blob::tests)` and require every binary/JSON/parser case green before witnessing the independent transaction reds in Step 5.

- [ ] **Step 5: Witness every independent atomicity failure**

Run:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
"$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a res://tests/restore_transaction_test.gd
```

The current implementation must show each intended red independently: invalid env repairs/warns; bad hash is late; narrowed actor values diverge late; owner poison/coupled cat contradictions are late or absent; disabled capture and dormant restrictions are absent. Record each named failure before Step 6.

- [ ] **Step 6: Add a complete read-only preflight before any mutation**

Before composing preflight, land each checked owner contract through its own strict micro-cycle. Add one named test at a time: `prepared_restore_rejects_nonfinite_pool_slot`, `prepared_restore_rejects_poisoned_echo_appointment`, `prepared_restore_rejects_invalid_viewmodel_side_or_blend`, `prepared_restore_rejects_invalid_brain_rect_target_or_rng`, `prepared_restore_rejects_invalid_gait_phase_point_or_support`, `prepared_restore_rejects_invalid_pose_or_tail_point`, `prepared_restore_rejects_invalid_cadence_interval_or_appointment`, `prepared_restore_rejects_invalid_time_or_demo_appointment`, and `prepared_restore_rejects_invalid_flicker_state`. For each owner, run its exact test first and record the targeted missing-API compile-red; add only the prepared type/signature plus an unchecked scaffold and rerun to record the named assertion-red; then implement that owner's complete validation and rerun green before starting the next owner. Finally run:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test prepared_restore_rejects_)
```

All nine owner tests must be green before `WaveRestorer::preflight` is written. The unchecked scaffolds are never committed and no validator is implemented before its named red.

Introduce a Rust-only prepared transaction:

```rust
pub(super) struct PreparedRestore {
    expected: CaptureState,
    expected_hash: u64,
    env: PreparedEnv,
    waves: PreparedWaveState,
    hero: PreparedHeroRestore,
    cats: Vec<PreparedCatRestore>,
    sources: Vec<PreparedSourceRestore>,
    targets: RestoreTargets,
}

#[derive(Debug, Clone)]
pub(super) struct PreparedEnv {
    now: PreparedTime,
    demo_checked: bool,
    demo_armed: bool,
    demo: PreparedDemoTap,
    flicker: PreparedFlicker,
    flicker_rng_state: u64,
}

struct RestoreTargets {
    core: Gd<WaveCore>,
    player: Gd<UnseeingPlayer>,
    body: Gd<HeroBody>,
    cats: Vec<Gd<WaveCat>>,
    sources: Vec<DynGd<Node, dyn SoundSource>>,
    observer: Gd<WaveObserver>,
}
```

`RestoreValueError` is the shared diagnostic carrier only—`{ path: String, rule: &'static str }` with `Display`/`Error`; it owns no validation law. Prepared composition has these exact shapes:

```rust
struct PreparedWaveState { pool: PreparedPulsePool, echoes: PreparedEchoQueue }
struct PreparedHeroRestore {
    player: PreparedPlayerState,
    viewmodel: PreparedViewmodel,
}
struct PreparedCatRestore { cat: PreparedCatState }
struct PreparedSourceRestore {
    handle: DynGd<Node, dyn SoundSource>,
    name: String,
    cadence: PreparedCadence,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PreparedCadence {
    // Both lanes are finite and interval_s is strictly positive. `None` is
    // the sole typed representation of the admitted legacy absent appointment.
    interval_s: f64,
    next_s: Option<f64>,
}
```

`PreparedCadence` is owned by `sound_source.rs`. Its fields stay private; only crate-visible checked owner methods construct, inspect, or install it across `sound_source.rs`, `nodes/source.rs`, `nodes/cat.rs`, and `nodes/restorer.rs`.

Task 6 review hardening is part of this same independently green schema
commit. Add one `WaveOrigin` checked value beside `CheckedWave`; every direct,
queued, restored-slot, scheduled-echo, and restored-echo origin must be finite
and lie in the closed `+-MAX_POSE_COORD_M` numerical envelope. This is an
origin admission invariant, not an acoustic constant or a claim that arbitrary
corrupted camera/matrix/wall inputs are safe. Make the unchecked raw
`EchoQueue::from_pending` constructor owner-private. Add the f32-max direct
red, each closed/adjacent origin boundary, rehashed-slot atomic red, and
scheduled/restored echo reds before production validation.

The same hardening gives reflected requests one prepared geometry contract.
Before player queue append, restored-queue acceptance, reflected-primary pool
installation, or raycast, prove the origin, zero-or-finite/nondegenerate
normal, lifted ray origin, fan dot products, f32 reach, and every retained ray
endpoint. Malformed caller geometry makes no primary, queue entry, raycast, or
echo. Validate every physics-server hit before clustering; a malformed hit
keeps the independently valid primary but refuses the complete echo fan before
any echo mutation. Reflection explanations propagate that same refusal rather
than filtering a kept cluster. Write and witness NaN and mixed-sign f32-max
normal reds, invalid-hit/batch-atomicity reds, and mutations for every check
and ordering edge.

Actor rotation preparation preserves configuration absent from format 2.
Capture canonicalizes each complete live player-local, eye-local, or
cat-global YXZ vector, requires omitted axes bit-identical, and serializes only
the owned yaw/pitch lane. Use separate pure operations for (a) canonicalizing
a live/brain lane replacement while preserving omitted axes and returning a
possibly one-ULP-adjusted owned lane, and (b) strict artifact installation
whose full requested vector is already canonical. Restore replaces only the
captured lane in the current complete live vector and carries that full target
to the install door; it never synthesizes zero axes. Treat omitted `+0`/`-0`
as the existing zero-equivalent class but return the original omitted sign
bits; every nonzero omitted lane stays bit-exact and the owned wire zero is
canonical `+0`. Extend the exact external
round trip with compatible canonical nonzero player X/Z, eye Y/Z, and cat X/Z;
record those live omitted bits immediately before restore and prove them again
immediately after restore. Record one normal-tick control future from the same
complete configuration, reset to that configuration, restore, advance once,
and require the identical future rather than freezing Godot's ordinary Euler
cache evolution.

Finally, treat the one GLSL packed/decode result as the effective gain. Keep
the raw finite `[0,1]` clamp and exact packed slot bits, but widen the decoded
f32 gain to f64 and use only that value for echo scheduling. Pin kind
`1_000_000`, raw gain `0.5`, packed `10_000_004.0_f32`, and effective gain
`4.0_f32 / 9.0_f32`; mutation-check any return to raw/clamped echo gain.
Validate cached hero/restorer liveness before first dereference, validate tick
time before echo drain, and pass registered `queue_wave` through the shared
checked request before append.

`UnseeingPlayer::prepare_restore(&HeroCapture) -> Result<PreparedPlayerState, RestoreValueError>` precomputes the exact target transform/eye lane, `ActorVelocity`, motion/suppression/gates, tap values, and validated queue requests; `install_prepared(PreparedPlayerState)` only assigns and clears transient collider identity. The restorer pairs it with `Viewmodel::prepare_restore` in `PreparedHeroRestore`. `WaveCat::prepare_restore(&CatCapture, PreparedCatBrain, PreparedCatGait, PreparedCatPose, PreparedTail, PreparedCadence) -> Result<PreparedCatState, RestoreValueError>` precomputes exact transform/rotation, velocity, motion, cadence, pose/time/last position, and lockstep checks; its install door only assigns and clears identity. These boundary owners, rather than the restorer, know their private fields and authored configs.

The owner APIs are exact and are introduced with failing owner-local tests before `WaveRestorer::preflight` calls them:

```rust
impl PulsePool {
    pub fn prepare_restore(slots: &[SlotCapture; MAXP]) -> Result<PreparedPulsePool, RestoreValueError>;
    pub fn from_prepared(value: PreparedPulsePool) -> Self;
}
impl EchoQueue {
    pub fn prepare_restore(pending: Vec<PendingEcho>) -> Result<PreparedEchoQueue, RestoreValueError>;
    pub fn from_prepared(value: PreparedEchoQueue) -> Self;
}
impl Viewmodel {
    pub fn prepare_restore(capture: ViewmodelCapture) -> Result<PreparedViewmodel, RestoreValueError>;
    pub fn from_prepared(value: PreparedViewmodel) -> Self;
}
impl CatBrain {
    pub fn prepare_restore(c: BrainCapture) -> Result<PreparedCatBrain, RestoreValueError>;
    pub fn from_prepared(c: PreparedCatBrain) -> Self;
}
impl CatGait {
    pub fn prepare_restore(c: GaitCapture) -> Result<PreparedCatGait, RestoreValueError>;
    pub fn from_prepared(c: PreparedCatGait) -> Self;
}
impl CatPose {
    pub fn prepare_restore(c: CatPose) -> Result<PreparedCatPose, RestoreValueError>;
    pub fn from_prepared(c: PreparedCatPose) -> Self;
}
impl Tail {
    pub fn prepare_restore(c: [Vector3; TAIL_N]) -> Result<PreparedTail, RestoreValueError>;
    pub fn from_prepared(c: PreparedTail) -> Self;
}
impl Cadence {
    pub(crate) fn prepare_restore(interval: f64, next: f64, allow_absent_nan: bool)
        -> Result<PreparedCadence, RestoreValueError>;
    pub(crate) fn from_prepared(value: PreparedCadence) -> Self;
}
pub fn prepare_time(value: f64) -> Result<PreparedTime, RestoreValueError>;
impl PreparedTime { pub fn value(self) -> f64; }
impl DemoTap {
    pub fn prepare_restore(next: f64) -> Result<PreparedDemoTap, RestoreValueError>;
    pub fn install_prepared(&mut self, value: PreparedDemoTap);
}
impl Flicker {
    pub fn prepare_restore(s: FlickerState) -> Result<PreparedFlicker, RestoreValueError>;
    pub fn from_prepared(value: PreparedFlicker) -> Self;
}
```

Each prepared type has private fields and can only be installed by its owner. The owner validates the complete domain it later reads: every pool/echo vector/scalar lane; viewmodel finite fields, closed side and bounded blends; brain enum/rect/target/countdowns/RNG increment; gait enum/phase/blends/points/support; pose/tail points; cadence interval/appointment; renderer-visible time; demo appointment; and flicker bounds. `presence_next` alone calls cadence preparation with `allow_absent_nan = true`, which converts that legacy wire sentinel into a typed absent appointment; no prepared/runtime arithmetic receives NaN. Source appointments pass false. Add required `SoundSource::prepare_appointment(&self, next) -> Result<PreparedCadence, RestoreValueError>` and `install_prepared_appointment(&mut self, PreparedCadence)` trait methods. `SoundFan` and `SoundRadio` implement them by asking `Cadence` to prepare against `self.voice().cadence` and by replacing their private `SourceRig` cadence with `Cadence::from_prepared`; the trait has no unrealistic default access to a concrete rig. `PreparedSourceRestore` carries the matched handle, name, and prepared cadence. `PreparedWaveState` carries a prepared pool and queue. Hero/cat prepared values carry exact target transforms/rotations, checked velocities and pure owner values, so commit performs assignment only. `apply_prepared_env` reads `PreparedTime::value`, calls `DemoTap::install_prepared`, replaces flicker through `Flicker::from_prepared`, and installs the already-validated RNG word; it invokes no legacy repairing restore door.

`WaveRestorer::preflight(&VarDictionary) -> Result<PreparedRestore, String>` parses the complete blob; requires an exact 16-lowercase-hex hash equal to `state_hash(parsed)`; validates version/level and every live handle/count/source name/order; asks the environment and every pure owner above to prepare its values; requires hero pitch inside `±PITCH_LIMIT` and engine yaw/pitch to round-trip `f64 -> f32 -> f64` bit-exactly; prepares exact hero/cat target transforms from checked positions/rotations; requires cat body/pose X/Y/Z bits to agree and, at this dormant schema commit, requires only `GaitCapture.support_y` to match every planted/aim Y lane. It does not yet equate body Y with that old world-zero gait datum, because a self-produced elevated pre-Task-4 cat legitimately has those different frames; Task 4 activates support transport and adds the final body-Y/gait-Y bit invariant. Preflight validates dormant capability and refuses fault-disabled actors. The restorer performs no owner arithmetic or repair itself. Task 2/5 later add live scene-authored motion-config validation when their airborne phases become admissible.

Preflight performs no writes and never calls `env_of` or reparses after validation. `UnseeingGame::restore_blob` freezes the tree, resolves the restorer, calls full preflight, and returns before any environment/pool/node write on failure. `apply_prepared_env(&PreparedEnv)` installs its already-validated time/demo/flicker/RNG values without repair or warning; then `WaveRestorer::commit(prepared)` consumes exact prepared handles/values. Commit has no semantic `Result` branch after its first write. The final recapture must equal `prepared.expected` and `prepared.expected_hash`; mismatch is an internal defect verdict, not artifact validation. Update module/order documentation accordingly.

Commit installs the dormant physical observation plus pure state/suppression/gates and clears transient collider identity; it preserves the actors' current default layer/mask because `nodes::support` does not exist until Task 2. Task 2 and Task 5 synchronously apply their controlled pair when each adapter activates. Historical landing never becomes a command. The old post-write stored-hash exception is deleted because hash validity is known before mutation. Every parse/header/hash/env/handle/count/source/actor/config/coupled-state contradiction leaves env, pool, hero, cats, sources, process flags, and transient warning state untouched.

- [ ] **Step 7: Run dormant-format/restore green gates**

Run exactly:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test reproduce::blob::tests && cargo test && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
for suite in restore_test restore_transaction_test; do
  "$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
    --ignoreHeadlessMode -c -a "res://tests/${suite}.gd"
done
GODOT="$GODOT" tools/restore_probe.sh
```

Expected: format-2 controlled dormant captures have identical hashes/futures, every invalid case refuses before writes/warnings, historical events do not emit, and future actor states are rejected until their owner task activates them. Transient identity remains private here; Task 7 observes its refresh.

- [ ] **Step 8: Mutate, review, and commit dormant capture/restore**

Remove each encoder/diff/parser/restore field in turn; leave version 1; normalize a support normal; coerce a pure fixture phase; re-emit history; capture collider ID; accept pending/gated/airborne live state prematurely; move preflight below env application; permit an env repair; accept wrong hash, lossy pitch/yaw, brain poison, or cat pose/gait mismatch. Witness each named failure, restore green, and request reproduction/architecture/code review. Rerun Step 7, stage only Task 6 files, and make one narrative format/preflight commit. Subsequent actor commits may activate these fields but may not change the version-2 wire layout or weaken complete preflight.

---

### Task 7: Structured Actor Motion Observability and End-to-End Fixtures

**Files:**
- Modify: `rust/src/observe/mod.rs:111-225,247-380`
- Modify: `rust/src/nodes/observer.rs:232-280,581-702,851-865,900-1052`
- Modify: `game/tests/observer_test.gd`
- Modify: `game/tests/player_elevation_test.gd`
- Modify: `game/tests/cat_elevation_test.gd`
- Modify: `game/tests/actor_support_test.gd`
- Create: `game/tests/scenes/character_elevation_movie.tscn`
- Create: `game/tests/probe/character_elevation_movie.gd`

**Interfaces:**
- Consumes: both actor state/support/identity/event observations and completed capture format.
- Produces: `ActorMotionObservation`, hero motion dictionary, ordered cat motion dictionaries, and independent structured proof used by retained fixtures.

- [ ] **Step 1: Add failing observer shape tests**

Define the expected pure value:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActorMotionObservation {
    state: MotionState,
    actual_velocity_mps: ActorVelocity,
    support_collider_id: Option<NonZeroU64>,
}
impl ActorMotionObservation {
    pub fn try_new(
        state: MotionState,
        raw_velocity_mps: Vector3,
        support_collider_id: Option<u64>,
    ) -> Result<Self, MotionValueError>;
    pub fn phase(self) -> MotionPhase;
    pub fn actual_velocity(self) -> ActorVelocity;
    pub fn support(self) -> Option<SupportContact>;
    pub fn support_collider_id(self) -> Option<u64>;
    pub fn last_landing(self) -> Option<LandingEvent>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatMotionObservation {
    pub name: String,
    pub position: ActorPosition,
    pub motion: ActorMotionObservation,
}
```

`try_new` validates the physical velocity first, then constructs the identity
with the following total branch:

```rust
let support_collider_id = match support_collider_id {
    None => None,
    Some(raw) => Some(
        NonZeroU64::new(raw)
            .ok_or_else(|| MotionValueError::out_of_range("support_collider_id"))?,
    ),
};
```

The public getter maps the private nonzero value back to `Option<u64>`. Thus
`None` means unavailable identity and can never be confused with an invalid
zero ID.

`try_new` first constructs `ActorVelocity`; it rejects `Some(collider_id)` when `state.support()` is absent, while accepted server-backed support may legitimately have no ID. Every phase/support/landing getter projects the one private `MotionState`; callers cannot supply contradictory copies.

Add player cases for controlled/airborne/never-landed/landed, poisoned physical velocity, identity-without-support refusal, and ordered cat entries including poisoned position refusal. Remove the raw `velocity: Vector3` field from internal `HeroObservation`; its existing Godot `hero.velocity` key and new `hero.motion.actual_velocity` are both projected from `motion.actual_velocity()`. Store hero/cat positions internally as `ActorPosition`. `SceneObservation` and `FrameObservation` gain `cat_motion: Vec<CatMotionObservation>` in recursive census order. The Godot snapshot uses `hero.motion` and `cats_motion`, where each cat dictionary is `{ "name": String, "position": Vector3, "motion": Dictionary }`. Motion exposes `phase`, `actual_velocity`, airborne-only nullable held velocities, nullable support `{point,normal,collider_id}`, and nullable landing `{impact_speed,point,normal}`. Collider identity is stable 16-lowercase-hex; absence is NIL, never empty/zero.

Name the pure cases `actor_motion_observation_rejects_poisoned_velocity`, `actor_motion_observation_rejects_zero_or_unsupported_identity`, and `cat_motion_observations_preserve_census_order`; add `test_actor_motion_snapshot_exposes_checked_player_and_ordered_cats` plus poisoned-observer refusal to `observer_test.gd`. Witness them red before production:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test actor_motion_observation)
(cd rust && cargo test cat_motion_observations_preserve_census_order)
(cd rust && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
"$GODOT" --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a res://tests/observer_test.gd
```

Expected: the Rust tests fail because the checked observation types do not exist, and the Godot suite fails only on the missing motion dictionaries/refusal boundary.

- [ ] **Step 2: Implement pure carry-through and boundary dictionaries**

For each actor, read its private `MotionState` once, its physical velocity once, and its position once; immediately construct the checked observation before reading another actor. A failed position/velocity/identity invariant refuses the whole snapshot with an actor-index/name path. Do not derive support from position or `is_on_floor`, duplicate phase/support/landing fields, or capture identity.

- [ ] **Step 3: Make integration fixtures assert independent boundaries**

For every elevated/fall/landing case, use three independent observations:

1. actor root/collision shape and observer phase/support;
2. mesh vertex/shoe/paw/cane Y;
3. pulse slot/queued-wave origin, type, range, gain, and reflection policy.

Pin floor, platform, table, bed, ramp up/down, edge, wall, no-floor terminal, silent/audible/capped landing, zero gain/range, delayed HeroBody latch, controlled blocking, centred airborne pass-through, actor-support rejection, and capture/restore future.

Create the test-only movie scene/probe now, before the green gate. It uses `CharacterElevationFixture`, a fixed camera, and labelled flat, `0.45 m` supported, unsupported `3 m`, and lower-floor lanes. At fixed physics frames it prints exactly one `ELEVATION_STATE <flat|elevated|airborne|landed> <dictionary>` record containing actor root, collider bottom, mesh Y extrema, and the Task 7 motion dictionary; it exits nonzero if any mark cannot be built. The `.tscn` references only this probe and test fixtures, never ships in a level, and its `.uid` is committed.

- [ ] **Step 4: Run observer/end-to-end green gates**

Run exactly:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo test observe && cargo test && cargo build --release)
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
ci/run_gdunit.sh "$PWD/game" "$GODOT" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd --ignoreHeadlessMode -c -a tests
```

Expected: full source census is exact; structured facts agree with body/mesh/pulse evidence; no unrelated perception/render fields differ.

- [ ] **Step 5: Mutate, review, and commit observability**

Omit each phase/held/support/identity/landing dictionary branch, report Godot `is_on_floor` instead of accepted support, capture collider identity, or reorder cats. Named tests must fail. Restore green, request observability/code review, and commit the structured surface and retained integration evidence.

---

### Task 8: Mutation Audit, Portability Gates, Visual Evidence, Wiki, and Branch Handoff

**Files:**
- Modify only for verified defects found by gates: files already owned by Tasks 1–7
- Update externally: fresh clone of `https://github.com/cleveralbatraoz/unseeing.wiki.git`
- Verify/update if implementation decisions changed: `docs/superpowers/specs/2026-08-21-character-elevation-support-design.md`
- Track execution checkboxes/results here: `docs/superpowers/plans/2026-08-21-character-elevation-support.md`

**Interfaces:**
- Consumes: the reviewed green implementation and all retained fixtures.
- Produces: complete mutation/portability/performance evidence, current wiki prose in a separate commit, final code reviews, and the user's explicit finish-branch choice.

- [ ] **Step 1: Re-run the complete mutation matrix**

Execute every mutation listed in the spec and Tasks 1–7 one at a time. Include both old unconditional-Y-zero paths, acceleration direction/value, terminal/dt caps, air steering/brain advance, post-collision X/Z, actor layers/support rejection, both capsule data, every visual/elevation boundary, camera/tail double lift, step/paw gates, persistent latch, severity branches, zero emitters, per-actor config injection, absolute wave/cane origins, and every capture/diff/restore field. Record each failing test and restore the reviewed implementation after each mutation.

- [ ] **Step 2: Format, lint, and run all Rust gates**

Run:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
(cd rust && cargo fmt --check)
(cd rust && cargo clippy --all-targets -- -D warnings)
(cd rust && cargo test)
(cd rust && cargo check --features editor-docs)
(cd rust && cargo test --features editor-docs editor_docs)
(cd rust && cargo build --release)
```

Expected: all green with no warnings. Confirm the test count increased from the 568-test baseline by the exact newly discovered cargo cases; do not hard-code a count into production.

- [ ] **Step 3: Format/lint GDScript and run full Godot gates**

Run exactly; the first block formats/lints only changed GDScript and refuses a missing tool:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
GDFORMAT="$(command -v gdformat)"
GDLINT="$(command -v gdlint)"
test -n "$GDFORMAT" && test -n "$GDLINT"
GDSCRIPT_FILES="$({ git diff --name-only "$(git merge-base HEAD main)" -- game; git ls-files --others --exclude-standard -- game; } | awk '/\.gd$/' | LC_ALL=C sort -u)"
if test -n "$GDSCRIPT_FILES"; then
  # File names in this repository contain no whitespace.
  "$GDFORMAT" $GDSCRIPT_FILES
  "$GDLINT" $GDSCRIPT_FILES
fi
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
"$GODOT" --headless --path game --import
ci/run_gdunit.sh "$PWD/game" "$GODOT" --headless --path "$PWD/game" \
  -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd \
  --ignoreHeadlessMode -c -a tests
GODOT="$GODOT" tools/determinism_probe.sh
GODOT="$GODOT" tools/restore_probe.sh
GODOT="$GODOT" tools/probe_editor_slabs.sh
GODOT="$GODOT" tools/probe_editor_sources.sh
GODOT="$GODOT" tools/probe_editor_level.sh
GODOT="$GODOT" tools/probe_editor_prefabs.sh
```

Expected: source-derived suite/case census is exact, zero errors/failures/skips, both deterministic probes agree, editor blueprints retain the corrected cat datum, and boot logs contain no new refusal/error pattern. Commit generated `.uid` sidecars; leave reports untracked/ignored.

- [ ] **Step 4: Run repository and platform gates**

Run:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
test/repo_hygiene.sh
ci/vendor-gdunit4.sh verify
tools/build_macos_core.sh
(cd rust && cargo check --features editor-docs --target x86_64-pc-windows-msvc)
(cd rust && cargo check --features editor-docs --target aarch64-pc-windows-msvc)
rust/build-wasm.sh
GODOT="$GODOT" SKIP_EXPORT=1 ci/pipeline.sh
```

`tools/build_macos_core.sh` builds and verifies both `aarch64-apple-darwin` and `x86_64-apple-darwin` slices. The two Windows `cargo check` commands compile the exact x86_64/arm64 targets pinned in `rust/rust-toolchain.toml` without attempting a foreign Godot export; the existing Windows CI remains the executable/import boundary. If an installed SDK/toolchain prevents a command, preserve its complete error and report that exact environmental limit rather than claiming a pass. Confirm no architecture conditional and exactly one Rust GDExtension in web.

- [ ] **Step 5: Capture visual/performance evidence without committing output**

Run exactly. `tools/run_game.sh --windowed 1280x720` owns the repository's checked `[display]` override (`mode=0`, viewport `1280×720`) and removes it on EXIT/INT/TERM/HUP. The evidence directory has one explicit, validated path so a later `view_image` call can inspect it before an explicit cleanup:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
GODOT=/tmp/unseeing-godot-4.7.1.VYRXsi/Godot.app/Contents/MacOS/Godot
ELEVATION_EVIDENCE_DIR=/tmp/unseeing-elevation-movie-issue64
test ! -e "$ELEVATION_EVIDENCE_DIR"
mkdir "$ELEVATION_EVIDENCE_DIR"
test ! -e game/override.cfg
GODOT="$GODOT" tools/run_game.sh --windowed 1280x720 --skip-build \
  --scene res://tests/scenes/character_elevation_movie.tscn -- \
  --fixed-fps 60 --quit-after 180 \
  --write-movie "$ELEVATION_EVIDENCE_DIR/elevation.png" \
  --log-file "$ELEVATION_EVIDENCE_DIR/movie.log"
for state in flat elevated airborne landed; do
  grep -q "^ELEVATION_STATE $state " "$ELEVATION_EVIDENCE_DIR/movie.log"
done
test "$(find "$ELEVATION_EVIDENCE_DIR" -name '*.png' -type f | wc -l | tr -d ' ')" -eq 180
(cd rust && cargo test valid_tick_calls_move_and_slide_once)
(cd rust && cargo test cat_valid_tick_calls_move_and_slide_once)
if git diff --unified=0 "$(git merge-base HEAD main)" -- rust/src | \
  grep -E '^\+.*(intersect_ray|intersect_shape|cast_motion)'; then
  echo 'unexpected new body-support query' >&2
  exit 1
fi
```

Use `view_image` on the generated frames nearest the four logged marks under
`/tmp/unseeing-elevation-movie-issue64` and compare them with the structured
dictionaries. Record frame names/results in the task notes, then run exactly:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
rm -rf /tmp/unseeing-elevation-movie-issue64
test ! -e game/override.cfg
```

Review the pure/adapters for allocation sites and global state; the trace tests prove one move, and the diff gate proves no new body-support query.

- [ ] **Step 6: Rewrite the current wiki in a fresh external clone**

Clone the wiki afresh and verify the real page names/branch:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
WIKI_DIR=/tmp/unseeing-wiki-issue64-final
test ! -e "$WIKI_DIR"
git clone --single-branch https://github.com/cleveralbatraoz/unseeing.wiki.git "$WIKI_DIR"
test "$(git -C "$WIKI_DIR" branch --show-current)" = master
test "$(git -C "$WIKI_DIR" rev-parse HEAD)" = "$(git -C "$WIKI_DIR" rev-parse origin/master)"
test -z "$(git -C "$WIKI_DIR" status --porcelain)"
for page in Mechanics-Overview.md Mechanics-Level-and-Objects.md Mechanics-Waves.md Engineering-Debugging-and-Observability.md Engineering-Build-Test-Deploy.md; do
  test -f "$WIKI_DIR/$page"
done
printf 'WIKI_DIR=%s\n' "$WIKI_DIR"
```

Update these files with `apply_patch` using the printed absolute clone path:

```text
Mechanics-Overview.md
Mechanics-Level-and-Objects.md
Mechanics-Waves.md
Engineering-Debugging-and-Observability.md
Engineering-Build-Test-Deploy.md
```

Name `support_motion.rs`, both node adapters, `MotionPhase`, `LandingEvent`, solver/layer constants, actor data, Inspector owners/ranges/units, origin heights, capture format 2, tests/probes, exact evidence, and limitations. State explicitly that acceleration is authored kinematics and reach/gain authored perception, not acoustics; static content remains fixed. Then run:

```bash
cd /Users/dmgalchenko/unseeing/.worktrees/issue-64-hero-elevation
WIKI_DIR=/tmp/unseeing-wiki-issue64-final
test -d "$WIKI_DIR/.git"
git -C "$WIKI_DIR" diff --check
git -C "$WIKI_DIR" config user.name 'Dmitrii Galchenko'
git -C "$WIKI_DIR" config user.email 'dggrus@gmail.com'
git -C "$WIKI_DIR" add Mechanics-Overview.md Mechanics-Level-and-Objects.md Mechanics-Waves.md Engineering-Debugging-and-Observability.md Engineering-Build-Test-Deploy.md
git -C "$WIKI_DIR" diff --cached --check
git -C "$WIKI_DIR" commit
```

Write a narrative subject/body in the editor, keep the clone for handoff, and do not push.

- [ ] **Step 7: Final reviews and clean-state proof**

Request implementation, architecture, physics/performance, wave/perception, and final code reviews against the complete diff. Verify every finding, fix only confirmed issues through a new failing test, rerun proportional gates and then Steps 2–4. Show `git status --short`, `git diff --check`, branch log, and absence of build artifacts/reports. Make a final docs-only commit if tracked spec/plan execution facts changed.

- [ ] **Step 8: Present the finish-branch choice and stop**

Using `finishing-a-development-branch`, summarize shipped behavior, tests/mutations/platform limits, code commits, and the separate unpushed wiki commit. Offer the user's allowed integration choices. Do not merge, push, close #64/#74, push wiki, or trigger deployment until the user explicitly chooses.
