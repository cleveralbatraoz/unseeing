# Debug Observability — State Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give agents a structured, queryable view of the wave engine's state so debugging stops depending on screenshots.

**Architecture:** Pure Rust modules under `rust/src/observe/` compute observations and explanations from data they are handed; one registered node, `WaveObserver` in `rust/src/nodes/observer.rs`, is injected by `main.gd` with the systems to read and exposes everything as `#[func]` methods returning `VarDictionary`. Three transports consume that one surface: godot-mcp live in-session, gdUnit4 in the headless gate, and (in Plan 2) a windowed dump scene.

**Tech Stack:** Rust (gdext 0.5.4, stable channel per `rust/rust-toolchain.toml`), Godot 4.7, typed GDScript, gdUnit4, godot-mcp (Node.js 20+, dev-only).

**Source spec:** `docs/superpowers/specs/2026-08-10-debug-observability-design.md`. This plan is Plan 1 of 2. Plan 2 (the pixel layer: `observe/digest.rs`, viewport readback, windowed dump scene) is out of scope here.

## Global Constraints

These apply to every task. Copied from `CLAUDE.md` and the spec.

- **No new crate dependencies.** Serialization is `VarDictionary` + GDScript `JSON.stringify`. No serde. The wasm export must not grow.
- **No `unsafe` Rust.** The crate is `#![deny(unsafe_code)]`; the only permitted exception is the existing `unsafe impl ExtensionLibrary` in `ffi.rs`. Never add another.
- **One Rust GDExtension per wasm export.** Everything joins the single `unseeing-core` crate. Never create a second crate.
- **The two layers.** All law lives in pure modules (`rust/src/*.rs`) that compile and test without a Godot runtime. Engine types (`Gd<T>`, `Node`, `VarDictionary`) appear ONLY in `rust/src/ffi.rs` and `rust/src/nodes/*.rs`. A boundary module carries values and adds no law.
- **Architecture independence.** No arch-specific code paths, intrinsics, or assumptions. Must build for x86_64, aarch64, and wasm32.
- **Observation never mutates.** Every `observe`/`explain` entry point takes `&self` or plain values. Nothing may emit a pulse, schedule an echo, or move a node.
- **A vacuous pass is worse than a failure.** Every observation that cannot be computed returns `{"unavailable": "<reason>"}` with no data fields. An empty pool and an unobservable pool must never serialise to the same JSON.
- **Object id law:** two touching objects need ids at least `oid_palette::MIN_SEP` (0.08) apart. Never assign ids by cycling a list.
- **Perception laws are untouched by this work.** This plan adds no rendering, no geometry, no light, no fill. If a task seems to need one, stop and ask.
- **Commits:** small, self-contained, each one green. Narrative subject line, technical body. **No `Co-Authored-By`, no "Generated with", no mention of Claude, AI, or any assistant anywhere in the repository.** Repo identity is `Dmitrii Galchenko <dggrus@gmail.com>`.
- **Tooling before every commit:** `cargo fmt`, `cargo clippy` (warnings are errors), `cargo test` for Rust; `gdformat` + `gdlint` for GDScript.
- **TDD is mandatory:** write the test, watch it fail *for the right reason*, write minimal code, watch it pass. Production code written before its test gets deleted, not retrofitted.

---

## File Structure

**Created:**

| File | Responsibility |
|---|---|
| `rust/src/observe/mod.rs` | Observation types, the `frame()` composer, module re-exports |
| `rust/src/observe/pool.rs` | Pulse-slot observation, from the public lanes |
| `rust/src/observe/evict.rs` | `explain_eviction` — the next slot and the rule that claims it |
| `rust/src/observe/ray.rs` | `explain_ray` — per-wall occlusion detail and transmission |
| `rust/src/observe/oids.rs` | `explain_oids` — touch graph, colouring, separation violations |
| `rust/src/observe/reflect.rs` | `explain_clustering` + the request/answer ledger for the reflection fan |
| `rust/src/nodes/observer.rs` | `WaveObserver` — the one registered class (boundary only) |
| `game/tests/observer_test.gd` | gdUnit4 suite driving `WaveObserver` against a built level |
| `docs/superpowers/mcp/godot-mcp-loop.md` | The live debugging loop, written for a fresh agent |

**Six modules shipped, not four.** The plan drafted `pool.rs` as "slot observation *and* the eviction rule" and gave the reflection explainer no file at all. Both split during execution and both splits are kept: eviction is a re-derivation of `emit`'s selection loop that must never call `emit`, which is a different job from decoding lanes, and `reflect.rs` carries a request/answer ledger that pool observation has no business knowing about. Task 2 and Task 7 name the files they actually create; this table is the corrected record.

**Modified:**

| File | Change |
|---|---|
| `rust/src/lib.rs` | Add `mod observe;` to the module list |
| `rust/src/nodes/mod.rs` | Add `mod observer;` and register the class |
| `game/scripts/main.gd` | Construct and inject `WaveObserver` |
| `game/tests/data_skins_test.gd` | Extend to pin `explain_ray` against the GLSL |
| `.gitignore` | Ignore `game/addons/godot_mcp/` |
| `test/repo_hygiene.sh` | Assert the godot-mcp addon is never committed |
| `CLAUDE.md` | Document the debugging loop in the tooling section |

**Why this split:** each module answers one question class and is independently testable; `mod.rs` only composes them. None exceeds a few hundred lines, matching the crate's existing file sizes.

---

### Task 1: Pulse-slot observation

The pool's `t0`, `end` and `kind` fields are private. Every one of them is derivable from the public `dat` lane — but a virgin slot holds `dat = (-1, 0, 0, 0)`, so reconstructing `end = birth + max_r/speed + fade_tail(kind)` computes `0.0/0.0` and yields `NaN`. Handle the sentinel first or the observable lies.

Deriving from the lanes rather than adding accessors is deliberate: it reports **what the shader sees** (f32-narrowed), which is what you want when the renderer and the CPU disagree.

**Files:**
- Create: `rust/src/observe/mod.rs`
- Create: `rust/src/observe/pool.rs`
- Modify: `rust/src/lib.rs`

**Interfaces:**
- Consumes: `crate::pulse_pool::{PulsePool, MAXP, fade_tail}` (all public today).
- Produces:
  ```rust
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
      pub ring_radius: f64,
      pub age: f64,
      pub remaining: f64,
      pub end: f64,
  }
  pub enum SlotState { Never, Expired, Live }
  pub fn slots(pool: &PulsePool, now: f64) -> Vec<SlotObservation>;
  ```

- [ ] **Step 1: Write the failing test**

Create `rust/src/observe/pool.rs` containing only this test module (no implementation yet):

```rust
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
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0).unwrap();
        let s = &slots(&pool, 0.5)[0];
        assert_eq!(s.state, SlotState::Live);
        assert_eq!(s.kind, 0);
        assert!((s.ring_radius - 2.75).abs() < 1e-5, "got {}", s.ring_radius);
        assert!((s.remaining - 6.590_909_090_9).abs() < 1e-5, "got {}", s.remaining);
        assert!((s.age - 0.5).abs() < 1e-5);
    }

    /// The ring stops growing at max_r; it does not run away with the clock.
    /// max_r 6.0 at speed 5.5 reaches full radius at t = 1.0909…, so at
    /// t = 3.0 the radius is still exactly 6.0 while the slot is alive on
    /// its 6-second fade tail.
    #[test]
    fn the_ring_is_capped_at_max_radius() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0).unwrap();
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
        pool.emit(3, Vector3::ZERO, 9.0, 4.5, 0.75, 0.0, beam, 0.85).unwrap();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0).unwrap();
        let obs = slots(&pool, 0.1);
        assert_eq!(obs[0].beam, beam);
        assert!((obs[0].cos_half - 0.85).abs() < 1e-5);
        assert_eq!(obs[1].cos_half, -2.0);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test --lib observe::pool 2>&1 | tail -20`

Expected: FAIL — compile error, `cannot find function 'slots' in this scope` and `cannot find type 'SlotState'`. That is the right failure: the tests are calling an API that does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Prepend to `rust/src/observe/pool.rs`, above the test module:

```rust
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
```

Create `rust/src/observe/mod.rs`:

```rust
//! Debug observability — the wave engine described to an agent as data.
//!
//! Four verbs, per `docs/superpowers/specs/2026-08-10-debug-observability-design.md`:
//! SNAPSHOT (state now), DIFF (the caller's job — sample and compare),
//! EXPLAIN (pure re-computations that answer "why"), and DIGEST (the pixel
//! reduction, Plan 2).
//!
//! Everything here is pure and engine-free. The boundary that hands these
//! results to Godot is `crate::nodes::observer`.

pub mod pool;
```

Add to `rust/src/lib.rs`'s module list, in alphabetical position among the existing `mod` lines:

```rust
mod observe;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test --lib observe::pool 2>&1 | tail -20`

Expected: PASS, 6 tests.

- [ ] **Step 5: Run the mutation check**

This is required by `CLAUDE.md`, not optional. Make each edit, confirm at least one test fails, then revert it:

1. In `slot()`, change `(age * speed).clamp(0.0, max_r)` to `age * speed` → `the_ring_is_capped_at_max_radius` must fail.
2. Change `if dat.x < 0.0` to `if false` → `virgin_slots_report_never_not_nan` must fail (on NaN, not on a wrong enum).
3. Change `end >= now` to `end > now` … this may pass; that is fine and expected — it is a boundary the tests do not pin, and pinning `end == now` exactly would be a change detector. Note it and move on.
4. Change the gain decode `% 10.0` to `% 100.0` → `kind_and_gain_are_decoded_not_raw` must fail.

Any mutation that no test catches marks that behaviour as unprotected — add a test rather than shrugging.

- [ ] **Step 6: Format, lint, commit**

```bash
cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && git add rust/src/observe/mod.rs rust/src/observe/pool.rs rust/src/lib.rs
git commit -F - <<'MSG'
The pool learns to describe itself, sentinel first

Every pulse slot decoded from the public lanes an agent can now read as
data: kind, gain, ring radius, age, remaining life, and whether the slot
never lived, has expired, or is still drawing.

Derived from pos/dat/dir rather than the private CPU shadow on purpose —
the lanes are what the shaders consume, f32-narrowed, so an observation
built from them reports what the renderer sees. That is the useful side of
a CPU/GPU disagreement.

The virgin sentinel is checked before any arithmetic. A slot that never
lived holds dat = (-1, 0, 0, 0), so reconstructing its end time computes
0.0/0.0 — and a NaN end compares false against every clock, reporting the
slot as neither live nor expired. An observable that cannot say which is
worse than no observable at all.
MSG
```

---

### Task 2: The eviction rule, explained

Answers "which slot would the next sound of this kind claim, and why?" — a question no snapshot can answer, because eviction happens between frames and overwrites its own evidence.

**Files:**
- Create: `rust/src/observe/evict.rs`
- Modify: `rust/src/observe/mod.rs`

**Interfaces:**
- Consumes: `crate::observe::pool::{SlotObservation, SlotState, slots}` from Task 1.
- Produces:
  ```rust
  pub enum EvictionRule { Expired, OldestRecurring, OldestOverall, Fallback }
  pub struct EvictionPlan { pub slot: usize, pub rule: EvictionRule, pub victim_kind: i32 }
  pub fn explain_eviction(pool: &PulsePool, now: f64) -> EvictionPlan;
  ```

The rule mirrors `PulsePool::emit`'s selection loop: first expired slot wins and breaks; otherwise the oldest slot of `kind >= 2` (footsteps and source hums, both of which recur and are therefore cheap); otherwise the oldest of anything; otherwise `MAXP - 1`.

**This must NOT call `emit()`.** It re-derives the rule independently, and the test below pins the two against each other by emitting for real and comparing the slot that actually changed.

- [ ] **Step 1: Write the failing test**

Create `rust/src/observe/evict.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pulse_pool::{MAXP, PulsePool};
    use godot::builtin::Vector3;

    /// The prediction is worthless unless it matches what emit() actually
    /// does. Fill the pool so the interesting rule fires, predict, then
    /// emit for real and check the slot that changed is the predicted one.
    /// This compares two independent implementations, so it is not a
    /// mirror assertion — the prediction never calls emit().
    fn assert_prediction_matches_reality(mut pool: PulsePool, now: f64) {
        let plan = explain_eviction(&pool, now);
        let before = *pool.pos();
        let marker = Vector3::new(999.0, 999.0, 999.0);
        pool.emit_omni(0, marker, 6.0, 5.5, 1.0, now).unwrap();
        let changed: Vec<usize> = (0..MAXP)
            .filter(|i| pool.pos()[*i] != before[*i])
            .collect();
        assert_eq!(changed, vec![plan.slot], "predicted {:?}", plan);
    }

    /// An expired slot is claimed before anything living is touched.
    #[test]
    fn expired_slots_are_claimed_first() {
        let mut pool = PulsePool::new();
        pool.emit_omni(2, Vector3::new(1.0, 0.0, 0.0), 1.6, 4.0, 0.8, 0.0).unwrap();
        pool.emit_omni(0, Vector3::new(2.0, 0.0, 0.0), 6.0, 5.5, 1.0, 0.0).unwrap();
        let plan = explain_eviction(&pool, 5.0);
        assert_eq!(plan.slot, 0);
        assert_eq!(plan.rule, EvictionRule::Expired);
        assert_prediction_matches_reality(pool, 5.0);
    }

    /// A full pool of live taps with one hum: the hum goes, because it
    /// recurs and a cane tap does not. Slot 7 holds the hum and is NOT the
    /// oldest slot — so this distinguishes the recurring rule from a plain
    /// oldest-wins rule.
    #[test]
    fn a_recurring_hum_is_sacrificed_before_any_tap() {
        let mut pool = PulsePool::new();
        for i in 0..MAXP {
            let kind = if i == 7 { 3 } else { 0 };
            let at = Vector3::new(i as f32, 0.0, 0.0);
            pool.emit_omni(kind, at, 6.0, 5.5, 1.0, 100.0 + i as f64 * 0.001).unwrap();
        }
        let plan = explain_eviction(&pool, 100.1);
        assert_eq!(plan.slot, 7);
        assert_eq!(plan.rule, EvictionRule::OldestRecurring);
        assert_eq!(plan.victim_kind, 3);
        assert_prediction_matches_reality(pool, 100.1);
    }

    /// Nothing cheap to sacrifice: 64 live taps, so the oldest tap goes.
    #[test]
    fn a_full_tap_pool_gives_up_its_oldest() {
        let mut pool = PulsePool::new();
        for i in 0..MAXP {
            let at = Vector3::new(i as f32, 0.0, 0.0);
            pool.emit_omni(0, at, 6.0, 5.5, 1.0, 100.0 + i as f64 * 0.001).unwrap();
        }
        let plan = explain_eviction(&pool, 100.1);
        assert_eq!(plan.slot, 0);
        assert_eq!(plan.rule, EvictionRule::OldestOverall);
        assert_eq!(plan.victim_kind, 0);
        assert_prediction_matches_reality(pool, 100.1);
    }

    /// A virgin pool: slot 0 has never lived, so it is Expired-by-sentinel
    /// and claimed first.
    #[test]
    fn a_virgin_pool_claims_slot_zero() {
        let pool = PulsePool::new();
        let plan = explain_eviction(&pool, 0.0);
        assert_eq!(plan.slot, 0);
        assert_eq!(plan.rule, EvictionRule::Expired);
        assert_prediction_matches_reality(pool, 0.0);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test --lib observe::evict 2>&1 | tail -20`

Expected: FAIL — `cannot find function 'explain_eviction'`.

- [ ] **Step 3: Write the minimal implementation**

Prepend to `rust/src/observe/evict.rs`:

```rust
//! Which slot the next sound would claim, and by which rule.
//!
//! Eviction happens between frames and overwrites its own evidence, so no
//! snapshot can show it. This re-derives `PulsePool::emit`'s selection
//! independently — it must never CALL emit, or asking the question would
//! answer it by changing it.

use crate::observe::pool::{SlotState, slots};
use crate::pulse_pool::{MAXP, PulsePool};

/// Why a slot was chosen. Mirrors the three-branch preference in
/// `PulsePool::emit`, plus the unreachable landing spot it falls back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvictionRule {
    /// A slot past its end time (or one that never lived).
    Expired,
    /// The oldest slot of kind >= 2 — footsteps and source hums, which
    /// recur and are therefore the cheapest live thing to lose.
    OldestRecurring,
    /// The oldest slot of any kind: nothing cheap was available.
    OldestOverall,
    /// Unreachable unless every birth time is non-finite; `emit` lands on
    /// the last slot, so this reports the same.
    Fallback,
}

/// The slot the next `emit` would take, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvictionPlan {
    pub slot: usize,
    pub rule: EvictionRule,
    /// The kind currently occupying that slot — what would be lost.
    pub victim_kind: i32,
}

/// Predict the next eviction as of `now`.
///
/// Total on any pool state. Scans in slot order and stops at the first
/// expired slot, exactly as `emit` does — the ORDER matters, because an
/// expired slot later in the pool must not win over an earlier one.
#[must_use]
pub fn explain_eviction(pool: &PulsePool, now: f64) -> EvictionPlan {
    let obs = slots(pool, now);
    let mut oldest_recurring: Option<usize> = None;
    let mut oldest_overall: Option<usize> = None;
    let mut t_recurring = f64::INFINITY;
    let mut t_overall = f64::INFINITY;
    for s in &obs {
        if s.state != SlotState::Live {
            return plan(&obs, s.index, EvictionRule::Expired);
        }
        if s.kind >= 2 && s.birth < t_recurring {
            t_recurring = s.birth;
            oldest_recurring = Some(s.index);
        }
        if s.birth < t_overall {
            t_overall = s.birth;
            oldest_overall = Some(s.index);
        }
    }
    if let Some(i) = oldest_recurring {
        return plan(&obs, i, EvictionRule::OldestRecurring);
    }
    if let Some(i) = oldest_overall {
        return plan(&obs, i, EvictionRule::OldestOverall);
    }
    plan(&obs, MAXP - 1, EvictionRule::Fallback)
}

fn plan(obs: &[super::pool::SlotObservation], slot: usize, rule: EvictionRule) -> EvictionPlan {
    EvictionPlan {
        slot,
        rule,
        victim_kind: obs[slot].kind,
    }
}
```

Add to `rust/src/observe/mod.rs`:

```rust
pub mod evict;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test --lib observe:: 2>&1 | tail -20`

Expected: PASS, 10 tests (6 from Task 1 plus 4).

- [ ] **Step 5: Run the mutation check**

1. Change `s.kind >= 2` to `s.kind >= 3` → `a_recurring_hum_is_sacrificed_before_any_tap` must still pass (the hum IS kind 3), but change it to `s.kind >= 4` and that test must fail. Use the `>= 4` form as the real check.
2. Remove the early `return` inside the loop so the scan continues past the first expired slot → `expired_slots_are_claimed_first` must fail.
3. Swap `oldest_recurring` and `oldest_overall` in the fallback order → `a_recurring_hum_is_sacrificed_before_any_tap` must fail.

- [ ] **Step 6: Format, lint, commit**

```bash
cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && git add rust/src/observe/evict.rs rust/src/observe/mod.rs
git commit -F - <<'MSG'
Eviction stops covering its tracks

Which slot the next sound will claim, and by which of the three rules —
a question no snapshot can answer, because eviction happens between frames
and the evidence it leaves is the thing it overwrote.

The rule is re-derived here rather than borrowed from emit(). Calling emit
to find out what emit would do would answer the question by changing it,
and a prediction that mutates the pool is not an observation. The two
implementations are pinned against each other instead: the tests predict,
then emit for real, then assert that the slot which actually changed is
the one predicted.

Scan order is load-bearing and tested. emit stops at the FIRST expired
slot, so a scan that keeps looking would hand back a later one and quietly
disagree with the engine it claims to describe.
MSG
```

---

### Task 3: `explain_ray` — occlusion, wall by wall

The GLSL in `pulse_pool.gdshaderinc` is a transliteration of `sight.rs`. This exposes the Rust side as an answerable question, which makes it the oracle: when the picture disagrees with `explain_ray`, the bug is in the shader.

**Files:**
- Create: `rust/src/observe/ray.rs`
- Modify: `rust/src/observe/mod.rs`

**Interfaces:**
- Consumes: `crate::sight::{crosses, contains, crossings, crossings_from}`, `crate::level_plan::{HUM_THROUGH, SOURCE_THROUGH, WALL_H}` (all public today).
- Produces:
  ```rust
  pub struct WallVerdict { pub index: usize, pub rect: Vector4, pub crossed: bool, pub contains_origin: bool }
  pub struct RayExplanation {
      pub from: Vector3, pub to: Vector3, pub wall_top: f32,
      pub walls: Vec<WallVerdict>,
      pub camera_crossings: u32, pub source_crossings: u32,
      pub hum_transmission: f64, pub source_transmission: f64,
  }
  pub fn explain_ray(from: Vector3, to: Vector3, rects: &[Vector4], wall_top: f32) -> RayExplanation;
  ```

- [ ] **Step 1: Write the failing test**

Create `rust/src/observe/ray.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::level_plan;
    use crate::sight::wall_rect;
    use godot::builtin::{Vector3, Vector4};

    const WALL_TOP: f32 = level_plan::WALL_H as f32;

    /// A RETIRED 20×20/10-wall map, not the shipped 28×28/19-wall scene —
    /// the same fixture sight.rs pins itself against, under the name it was
    /// corrected to in Task 8.
    fn retired_map_rects() -> Vec<Vector4> {
        [
            Vector4::new(0.6, 0.6, 19.4, 0.6),
            Vector4::new(19.4, 0.6, 19.4, 19.4),
            Vector4::new(19.4, 19.4, 0.6, 19.4),
            Vector4::new(0.6, 19.4, 0.6, 0.6),
            Vector4::new(6.4, 0.6, 6.4, 8.0),
            Vector4::new(6.4, 12.4, 6.4, 19.4),
            Vector4::new(6.4, 8.0, 14.0, 8.0),
            Vector4::new(14.0, 8.0, 14.0, 15.6),
            Vector4::new(9.0, 15.6, 14.0, 15.6),
            Vector4::new(0.6, 13.0, 4.0, 13.0),
        ]
        .iter()
        .map(|s| wall_rect(*s))
        .collect()
    }

    /// Spawn to fan head crosses exactly one wall — and the explanation
    /// must NAME it, not merely count it. Wall index 4 is DividerNorth.
    /// Transmission is then 0.55^1 for the wave and 0.30^1 for the
    /// silhouette, hand-derived from the constants in level_plan.
    #[test]
    fn one_wall_is_named_and_its_transmission_derived() {
        let e = explain_ray(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(8.6, 1.15, 4.4),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert_eq!(e.camera_crossings, 1);
        let crossed: Vec<usize> = e.walls.iter().filter(|w| w.crossed).map(|w| w.index).collect();
        assert_eq!(crossed, vec![4]);
        assert!((e.hum_transmission - 0.55).abs() < 1e-9);
        assert!((e.source_transmission - 0.30).abs() < 1e-9);
    }

    /// Two walls compose as k^2 — the composition law, hand-derived:
    /// 0.55^2 = 0.3025 and 0.30^2 = 0.09.
    #[test]
    fn two_walls_compose_their_transmission() {
        let e = explain_ray(
            Vector3::new(3.0, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 10.0),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert_eq!(e.camera_crossings, 2);
        assert!((e.hum_transmission - 0.3025).abs() < 1e-9, "got {}", e.hum_transmission);
        assert!((e.source_transmission - 0.09).abs() < 1e-9, "got {}", e.source_transmission);
    }

    /// A clear line reports full transmission and every wall verdict false
    /// — not an empty list. An agent must be able to see that the walls
    /// were considered and refused.
    #[test]
    fn a_clear_line_still_reports_every_wall() {
        let e = explain_ray(
            Vector3::new(8.0, 1.0, 4.0),
            Vector3::new(12.0, 1.5, 6.0),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert_eq!(e.camera_crossings, 0);
        assert_eq!(e.walls.len(), 10);
        assert!(e.walls.iter().all(|w| !w.crossed));
        assert!((e.hum_transmission - 1.0).abs() < 1e-9);
    }

    /// The birth-wall asymmetry, made visible. A source standing on the
    /// divider centerline lighting an open point: the camera occluder
    /// counts the wall it exits, the source occluder skips the wall it was
    /// born in, and the explanation reports BOTH plus which wall contained
    /// the origin.
    #[test]
    fn the_birth_wall_asymmetry_is_reported_not_hidden() {
        let e = explain_ray(
            Vector3::new(6.4, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 4.0),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert_eq!(e.camera_crossings, 1);
        assert_eq!(e.source_crossings, 0);
        let held: Vec<usize> = e
            .walls
            .iter()
            .filter(|w| w.contains_origin)
            .map(|w| w.index)
            .collect();
        assert_eq!(held, vec![4]);
    }

    /// The two transmissions must be keyed to their own occluder, not both
    /// to the camera's. On the birth-wall geometry the SOURCE occluder
    /// (`source_crossings`) sees zero walls, so `hum_transmission` — the
    /// exponent `source_reveal_vis` in the shader actually applies — is
    /// full at 1.0 (`0.55^0`); the CAMERA occluder still sees the one wall
    /// it exits, so `source_transmission` — the exponent `source_muffle`
    /// applies — is dimmed to 0.30 (`0.30^1`). A version that exponentiates
    /// both by `camera_crossings` would report 0.55 for `hum_transmission`
    /// here instead of 1.0, and no other test in this module can tell the
    /// two exponent bases apart.
    #[test]
    fn the_two_transmissions_use_their_own_occluder() {
        let e = explain_ray(
            Vector3::new(6.4, 0.9, 4.0),
            Vector3::new(10.0, 0.9, 4.0),
            &retired_map_rects(),
            WALL_TOP,
        );
        assert!(
            (e.hum_transmission - 1.0).abs() < 1e-9,
            "got {}",
            e.hum_transmission
        );
        assert!(
            (e.source_transmission - 0.30).abs() < 1e-9,
            "got {}",
            e.source_transmission
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test --lib observe::ray 2>&1 | tail -20`

Expected: FAIL — `cannot find function 'explain_ray'`.

- [ ] **Step 3: Write the minimal implementation**

Prepend to `rust/src/observe/ray.rs`:

```rust
//! Occlusion, wall by wall — the oracle.
//!
//! `crate::sight` is the cargo-pinned reference that the GLSL in
//! `pulse_pool.gdshaderinc` transliterates. Exposing it as an answerable
//! question turns "the picture looks wrong" into "the Rust says one
//! crossing and the shader drew none", which localises the bug to the
//! shader without a single pixel being inspected.

use godot::builtin::{Vector3, Vector4};

use crate::level_plan::{HUM_THROUGH, SOURCE_THROUGH};
use crate::sight::{contains, crosses, crossings, crossings_from};

/// One wall's answer for one sight line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallVerdict {
    pub index: usize,
    pub rect: Vector4,
    /// Does the segment pierce this wall's box?
    pub crossed: bool,
    /// Does this wall contain the origin? Such a wall is skipped by the
    /// SOURCE occluder — a sound born flush on a wall lights its own face.
    pub contains_origin: bool,
}

/// Everything the occlusion tests say about one sight line.
#[derive(Debug, Clone, PartialEq)]
pub struct RayExplanation {
    pub from: Vector3,
    pub to: Vector3,
    pub wall_top: f32,
    /// Every wall considered, in table order — including the ones that
    /// refused. An empty verdict list and a clear line are different facts.
    pub walls: Vec<WallVerdict>,
    /// Eye to lit point: every wall counts.
    pub camera_crossings: u32,
    /// Source to lit point: the birth wall is skipped.
    pub source_crossings: u32,
    /// `HUM_THROUGH ^ source_crossings` — how much of a source's WAVE
    /// survives (the shader's `source_reveal_vis`, keyed to the SOURCE
    /// occluder so a sound born flush on a wall still lights its own face).
    pub hum_transmission: f64,
    /// `SOURCE_THROUGH ^ camera_crossings` — how much of its SILHOUETTE
    /// survives (the engine's `source_muffle`, keyed to the CAMERA
    /// occluder — every wall between the eye and the source counts).
    pub source_transmission: f64,
}

/// Explain what the walls do to the sight line `from -> to`.
///
/// Total on any input, including a degenerate segment (`from == to` still
/// runs every wall test — a point can lie inside a wall's occluder box, so
/// it is not guaranteed to cross nothing). The counts come from `sight`'s
/// own functions rather than from the per-wall verdicts, so a disagreement
/// between the two would surface as a failing test here rather than as a
/// plausible-looking wrong answer in the field.
#[must_use]
pub fn explain_ray(
    from: Vector3,
    to: Vector3,
    rects: &[Vector4],
    wall_top: f32,
) -> RayExplanation {
    let walls = rects
        .iter()
        .enumerate()
        .map(|(index, rect)| WallVerdict {
            index,
            rect: *rect,
            crossed: crosses(from, to, *rect, wall_top),
            contains_origin: contains(*rect, from, wall_top),
        })
        .collect();
    let camera_crossings = crossings(from, to, rects, wall_top);
    let source_crossings = crossings_from(from, to, rects, wall_top);
    RayExplanation {
        from,
        to,
        wall_top,
        walls,
        camera_crossings,
        source_crossings,
        // HUM_THROUGH is the source_reveal_vis exponent base
        // (data_core.gdshaderinc), which reads off wall_crossings_from —
        // the SOURCE occluder that skips the wall a source is born inside.
        hum_transmission: HUM_THROUGH.powi(source_crossings as i32),
        // SOURCE_THROUGH is the source_muffle exponent base
        // (nodes/level.rs), which reads off sight::crossings — the CAMERA
        // occluder, every wall counted.
        source_transmission: SOURCE_THROUGH.powi(camera_crossings as i32),
    }
}
```

Add to `rust/src/observe/mod.rs`:

```rust
pub mod ray;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test --lib observe:: 2>&1 | tail -20`

Expected: PASS, 15 tests.

- [ ] **Step 5: Run the mutation check**

1. Change `HUM_THROUGH.powi(...)` to `HUM_THROUGH * f64::from(source_crossings)` → `two_walls_compose_their_transmission` must fail.
2. Change `crossings_from` to `crossings` for `source_crossings` → `the_birth_wall_asymmetry_is_reported_not_hidden` must fail.
3. Change the `walls` collect to filter on `crossed` → `a_clear_line_still_reports_every_wall` must fail.

- [ ] **Step 6: Format, lint, commit**

```bash
cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && git add rust/src/observe/ray.rs rust/src/observe/mod.rs
git commit -F - <<'MSG'
The walls answer for themselves: occlusion becomes a question you can ask

sight.rs is the cargo-pinned reference the fragment shader transliterates.
Exposing it as an answerable question makes it an oracle: when a source
shows through a wall on screen, the Rust can be asked how many walls the
sight line pierces, and a disagreement localises the bug to the GLSL
without anyone inspecting a pixel.

Every wall is reported, including the ones that refused. A clear line and
an unconsidered wall table are different facts, and an explanation that
returned only the crossings could not tell them apart.

Both occluders are reported side by side, because they differ on exactly
one wall — the one a sound is born inside, which blocks the eye behind it
but never that sound's own near face. That asymmetry is deliberate, and
now it is visible instead of merely tested.
MSG
```

---

### Task 4: `explain_oids` — the touch graph and the 0.08 law

**Files:**
- Create: `rust/src/observe/oids.rs`
- Modify: `rust/src/observe/mod.rs`

**Interfaces:**
- Consumes: `crate::oid_palette::{Box3, MIN_SEP, separated}` (all public today).
- Produces:
  ```rust
  pub struct TouchPair { pub a: usize, pub b: usize, pub oid_a: f64, pub oid_b: f64, pub delta: f64, pub draws: bool }
  pub struct OidExplanation { pub pairs: Vec<TouchPair>, pub violations: Vec<usize>, pub min_sep: f64 }
  pub fn explain_oids(boxes: &[Box3], oids: &[f64]) -> OidExplanation;
  ```

`violations` holds indices into `pairs`, not into `boxes` — a pair is what can violate the law, not a box.

- [ ] **Step 1: Write the failing test**

Create `rust/src/observe/oids.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid_palette::Box3;

    fn unit_at(x: f64) -> Box3 {
        Box3::from_center_size([x, 0.5, 0.0], [1.0, 1.0, 1.0])
    }

    /// Two touching boxes with identical ids melt into one silhouette:
    /// the crease is the ONLY line between interpenetrating solids, and it
    /// comes from a difference in the flat object id. Delta 0 means no
    /// line, and the explanation must say so as a violation.
    #[test]
    fn touching_boxes_with_equal_ids_are_a_violation() {
        let boxes = [unit_at(0.0), unit_at(0.9)];
        let e = explain_oids(&boxes, &[0.24, 0.24]);
        assert_eq!(e.pairs.len(), 1);
        assert_eq!(e.pairs[0].delta, 0.0);
        assert!(!e.pairs[0].draws);
        assert_eq!(e.violations, vec![0]);
    }

    /// Exactly at the minimum separation the seam draws — the law is
    /// "at least 0.08", not "more than". Hand-derived: 0.32 - 0.24 = 0.08.
    #[test]
    fn the_minimum_separation_itself_draws() {
        let boxes = [unit_at(0.0), unit_at(0.9)];
        let e = explain_oids(&boxes, &[0.24, 0.32]);
        assert!((e.pairs[0].delta - 0.08).abs() < 1e-9);
        assert!(e.pairs[0].draws);
        assert!(e.violations.is_empty());
    }

    /// Boxes that do not touch are not pairs at all. Two solids across the
    /// room share an id harmlessly — the budget would be unusable
    /// otherwise, and reporting them would bury the real violations.
    #[test]
    fn distant_boxes_with_equal_ids_are_not_reported() {
        let boxes = [unit_at(0.0), unit_at(50.0)];
        let e = explain_oids(&boxes, &[0.24, 0.24]);
        assert!(e.pairs.is_empty());
        assert!(e.violations.is_empty());
    }

    /// Every touching pair is reported once, not twice — a-b and b-a are
    /// the same seam. Three mutually touching boxes give three pairs.
    #[test]
    fn each_seam_is_reported_once() {
        // Spacing 0.4 keeps every pair — including the two-hop 0-2 pair —
        // inside the unit boxes' 1.0-wide touch range, so all three are
        // genuinely mutually touching. A 0.9 spacing does NOT: box0 spans
        // [-0.5, 0.5] and box2 spans [1.3, 2.3], and `touches` would need
        // 1.29 <= 0.5. The assertion below cannot pass at 0.9.
        let boxes = [unit_at(0.0), unit_at(0.4), unit_at(0.8)];
        let e = explain_oids(&boxes, &[0.0, 0.16, 0.32]);
        let ids: Vec<(usize, usize)> = e.pairs.iter().map(|p| (p.a, p.b)).collect();
        assert_eq!(ids, vec![(0, 1), (0, 2), (1, 2)]);
    }

    /// A short oid list cannot be explained. Reporting zero pairs here
    /// would be a vacuous pass — the caller would read "no violations"
    /// from an input that was never checked.
    #[test]
    fn a_short_oid_list_is_refused_not_silently_truncated() {
        let boxes = [unit_at(0.0), unit_at(0.9)];
        assert!(explain_oids_checked(&boxes, &[0.24]).is_none());
        assert!(explain_oids_checked(&boxes, &[0.24, 0.32]).is_some());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test --lib observe::oids 2>&1 | tail -20`

Expected: FAIL — `cannot find function 'explain_oids'`.

- [ ] **Step 3: Write the minimal implementation**

Prepend to `rust/src/observe/oids.rs`:

```rust
//! The object-id budget, checked and explained.
//!
//! Law #2: where two objects interpenetrate there is no depth step, so a
//! difference in the flat object id is the ONLY thing that can draw their
//! seam. Two touching solids sharing an id melt into one shape. This
//! reports the touch graph, the id handed to each solid, and every pair
//! closer than `oid_palette::MIN_SEP`.

use crate::oid_palette::{Box3, MIN_SEP, separated};

/// Two solids that touch, and whether the seam between them draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchPair {
    pub a: usize,
    pub b: usize,
    pub oid_a: f64,
    pub oid_b: f64,
    pub delta: f64,
    /// True when the ids are at least `MIN_SEP` apart.
    pub draws: bool,
}

/// The touch graph with its colouring checked.
#[derive(Debug, Clone, PartialEq)]
pub struct OidExplanation {
    /// Every touching pair, each reported once, in ascending (a, b) order.
    pub pairs: Vec<TouchPair>,
    /// Indices INTO `pairs` whose seam does not draw.
    pub violations: Vec<usize>,
    pub min_sep: f64,
}

/// Explain the colouring, or refuse.
///
/// Returns `None` when `oids` is shorter than `boxes`: a truncated check
/// that reported no violations would be a vacuous pass, and the caller
/// could not tell it apart from a clean level.
#[must_use]
pub fn explain_oids_checked(boxes: &[Box3], oids: &[f64]) -> Option<OidExplanation> {
    if oids.len() < boxes.len() {
        return None;
    }
    Some(explain_oids(boxes, oids))
}

/// Explain the colouring of a level whose ids are known to be complete.
///
/// # Panics
///
/// If `oids` is shorter than `boxes`. Callers crossing a boundary should
/// use [`explain_oids_checked`].
#[must_use]
pub fn explain_oids(boxes: &[Box3], oids: &[f64]) -> OidExplanation {
    assert!(oids.len() >= boxes.len(), "one oid per box is required");
    let mut pairs = Vec::new();
    for a in 0..boxes.len() {
        for b in (a + 1)..boxes.len() {
            if !boxes[a].touches(&boxes[b]) {
                continue;
            }
            pairs.push(TouchPair {
                a,
                b,
                oid_a: oids[a],
                oid_b: oids[b],
                delta: (oids[a] - oids[b]).abs(),
                draws: separated(oids[a], oids[b]),
            });
        }
    }
    let violations = pairs
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.draws)
        .map(|(i, _)| i)
        .collect();
    OidExplanation {
        pairs,
        violations,
        min_sep: MIN_SEP,
    }
}
```

Add to `rust/src/observe/mod.rs`:

```rust
pub mod oids;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test --lib observe:: 2>&1 | tail -20`

Expected: PASS, 19 tests.

- [ ] **Step 5: Verify against the shipped level, then mutation check**

First confirm the shipped map is clean — a checker that reports violations on a known-good level is worse than none:

Run: `cd rust && cargo test --lib oid_palette 2>&1 | tail -5` and confirm the existing palette suites still pass.

Then the mutations:
1. Change `separated(oids[a], oids[b])` to `oids[a] != oids[b]` → `the_minimum_separation_itself_draws` still passes, but add a temporary case with delta 0.04 to confirm it fails; if it does not, the `draws` field is not actually testing the law.
2. Change `(a + 1)..boxes.len()` to `0..boxes.len()` → `each_seam_is_reported_once` must fail.
3. Remove the `touches` guard → `distant_boxes_with_equal_ids_are_not_reported` must fail.

- [ ] **Step 6: Format, lint, commit**

```bash
cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && git add rust/src/observe/oids.rs rust/src/observe/mod.rs
git commit -F - <<'MSG'
The touch graph explains why a seam went missing

Where two objects interpenetrate there is no depth step, so a difference
in the flat object id is the only thing that can draw their seam. Two
touching solids sharing an id melt into a single silhouette, and until now
the only way to notice was to look at the picture and feel that something
was wrong.

The graph is now reportable: every touching pair, the id each solid was
handed, the delta between them, and whether that delta clears the minimum
separation. Boxes that do not touch are not pairs — solids across the room
share ids harmlessly, and reporting them would bury the real violations
under the budget working as intended.

A short id list is refused rather than truncated. A check that ran over
half its input and reported nothing wrong is indistinguishable from a
clean level, which is the failure mode worth engineering against.
MSG
```

---

### Task 5: The frame composer

Assembles the pieces into one snapshot. Pure: it takes plain data, never a `Gd<T>`.

**Files:**
- Modify: `rust/src/observe/mod.rs`

**Interfaces:**
- Consumes: `slots` (Task 1), `explain_eviction` (Task 2).
- Produces:
  ```rust
  pub struct SourceObservation { pub name: String, pub position: Vector3, pub volume: f64, pub reach: f64, pub walls_to_eye: u32, pub source_floor: f64, pub slot_pressure: f64 }
  pub struct FrameObservation { pub now: f64, pub flick: f64, pub live_count: usize, pub slots: Vec<SlotObservation>, pub next_eviction: EvictionPlan, pub sources: Vec<SourceObservation>, pub wall_rects: Vec<Vector4>, pub wall_truncated: bool, pub camera: Vector3, pub camera_basis: Basis }
  pub fn frame(pool: &PulsePool, now: f64, flick: f64, sources: Vec<SourceObservation>, wall_rects: Vec<Vector4>, camera: Vector3, camera_basis: Basis) -> FrameObservation;
  ```

`wall_truncated` is `wall_rects.len() >= sight::MAXW` — the level truncates at 32 and must say so loudly.

- [ ] **Step 1: Write the failing test**

Append to `rust/src/observe/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::evict::EvictionRule;
    use crate::pulse_pool::PulsePool;
    use godot::builtin::{Basis, Vector3, Vector4};

    fn empty_frame(pool: &PulsePool, now: f64) -> FrameObservation {
        frame(pool, now, 1.0, Vec::new(), Vec::new(), Vector3::ZERO, Basis::IDENTITY)
    }

    /// The composer carries the pieces through without recomputing them:
    /// live_count agrees with the pool, and the eviction plan is present.
    #[test]
    fn a_frame_carries_pool_state_and_the_next_eviction() {
        let mut pool = PulsePool::new();
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0).unwrap();
        let f = empty_frame(&pool, 0.5);
        assert_eq!(f.now, 0.5);
        assert_eq!(f.live_count, 1);
        assert_eq!(f.slots.len(), 64);
        assert_eq!(f.next_eviction.rule, EvictionRule::Expired);
        assert_eq!(f.next_eviction.slot, 1);
    }

    /// A wall table at the shader's ceiling is flagged. The level
    /// truncates at MAXW and must say so — a silently clipped table
    /// occludes with walls the level does not have.
    #[test]
    fn a_full_wall_table_is_flagged_as_truncated() {
        let pool = PulsePool::new();
        let rect = Vector4::new(0.0, 0.0, 1.0, 1.0);
        let short = frame(&pool, 0.0, 1.0, Vec::new(), vec![rect; 31], Vector3::ZERO, Basis::IDENTITY);
        let full = frame(&pool, 0.0, 1.0, Vec::new(), vec![rect; 32], Vector3::ZERO, Basis::IDENTITY);
        assert!(!short.wall_truncated);
        assert!(full.wall_truncated);
    }

    /// A level with no sources is legal and reports an empty list — not
    /// an error, and not an absence of the field.
    #[test]
    fn a_silent_level_is_legal() {
        let pool = PulsePool::new();
        assert!(empty_frame(&pool, 0.0).sources.is_empty());
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd rust && cargo test --lib observe 2>&1 | tail -20`

Expected: FAIL — `cannot find function 'frame'`.

- [ ] **Step 3: Write the minimal implementation**

Insert into `rust/src/observe/mod.rs`, above the test module and below the `pub mod` lines:

```rust
use godot::builtin::{Basis, Vector3, Vector4};

use crate::evict::{EvictionPlan, explain_eviction};
use crate::pool::{SlotObservation, slots};
use crate::pulse_pool::PulsePool;
use crate::sight::MAXW;

/// One sound source as an agent reads it. Built at the boundary, where
/// the source nodes live; carried through here unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceObservation {
    pub name: String,
    pub position: Vector3,
    pub volume: f64,
    pub reach: f64,
    /// Walls between the eye and this source's hub.
    pub walls_to_eye: u32,
    /// The standing image floor after muffling — the `u_source_floor`
    /// instance uniform this source is pushed.
    pub source_floor: f64,
    pub slot_pressure: f64,
}

/// The whole state vector for one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameObservation {
    pub now: f64,
    pub flick: f64,
    pub live_count: usize,
    pub slots: Vec<SlotObservation>,
    pub next_eviction: EvictionPlan,
    pub sources: Vec<SourceObservation>,
    pub wall_rects: Vec<Vector4>,
    /// True when the table has reached the shader's ceiling, so walls may
    /// have been dropped. Loud by construction.
    pub wall_truncated: bool,
    pub camera: Vector3,
    pub camera_basis: Basis,
}

/// Compose one frame's observation from parts the boundary supplies.
///
/// Pure: every argument is plain data. The boundary
/// (`crate::nodes::observer`) is what knows how to obtain them.
#[must_use]
pub fn frame(
    pool: &PulsePool,
    now: f64,
    flick: f64,
    sources: Vec<SourceObservation>,
    wall_rects: Vec<Vector4>,
    camera: Vector3,
    camera_basis: Basis,
) -> FrameObservation {
    FrameObservation {
        now,
        flick,
        live_count: pool.live_count(now),
        slots: slots(pool, now),
        next_eviction: explain_eviction(pool, now),
        sources,
        wall_truncated: wall_rects.len() >= MAXW,
        wall_rects,
        camera,
        camera_basis,
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd rust && cargo test --lib observe 2>&1 | tail -20`

Expected: PASS, 22 tests.

- [ ] **Step 5: Mutation check and commit**

1. Change `>= MAXW` to `> MAXW` → `a_full_wall_table_is_flagged_as_truncated` must fail.
2. Change `pool.live_count(now)` to `slots.len()` → `a_frame_carries_pool_state_and_the_next_eviction` must fail.

```bash
cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && git add rust/src/observe/mod.rs
git commit -F - <<'MSG'
One frame, one state vector

The composer that assembles a whole frame's observation: the pool, the
next eviction, the sources with their muffled standing floors, the wall
table, and where the eye was standing when all of it was true.

It stays pure — every argument is plain data, and the boundary is what
knows how to obtain them. That keeps the entire snapshot cargo-testable
with no Godot runtime, which is the property that lets the headless gate
assert on it later.

A wall table at the shader's ceiling is flagged rather than trusted. The
level truncates at MAXW, and a table that quietly clipped would occlude
the world with walls the level does not actually have — the kind of fault
that reads as a rendering bug for a day and a half.
MSG
```

---

### Task 6: `WaveObserver` — the boundary

The one registered class. It adds no law: it reads the systems it was injected with, calls the pure functions, and converts to `VarDictionary`.

**Files:**
- Create: `rust/src/nodes/observer.rs`
- Modify: `rust/src/nodes/mod.rs`
- Modify: `game/scripts/main.gd`
- Create: `game/tests/observer_test.gd`

**Interfaces:**
- Consumes: `crate::observe::{frame, FrameObservation, ray::explain_ray, oids::explain_oids_checked, evict::explain_eviction}`.
- Produces (GDScript-visible):
  ```
  WaveObserver.inject(level: WaveLevel, pulses: RefCounted, camera: Camera3D) -> void
  WaveObserver.snapshot(now: float) -> Dictionary
  WaveObserver.explain_ray(from: Vector3, to: Vector3) -> Dictionary
  WaveObserver.explain_oids() -> Dictionary
  WaveObserver.explain_eviction(now: float) -> Dictionary
  ```

Every one of these returns `{"unavailable": "<reason>"}` when it was never injected.

- [ ] **Step 1: Write the failing test**

Create `game/tests/observer_test.gd`:

```gdscript
extends GdUnitTestSuite
## WaveObserver — the debug observability boundary.
##
## These pin the CONTRACT the live debugging loop depends on: an
## uninjected observer refuses loudly, an injected one reports the state
## an agent reads. The maths itself is pinned by cargo tests; what is
## tested here is that the boundary carries it across without inventing
## anything.


func test_uninjected_observer_refuses_rather_than_reporting_zeros() -> void:
	var obs := WaveObserver.new()
	var snap: Dictionary = obs.snapshot(0.0)
	assert_bool(snap.has("unavailable")).is_true()
	assert_bool(snap.has("slots")).is_false()


func test_uninjected_explainers_refuse_too() -> void:
	var obs := WaveObserver.new()
	assert_bool(obs.explain_ray(Vector3.ZERO, Vector3.ONE).has("unavailable")).is_true()
	assert_bool(obs.explain_oids().has("unavailable")).is_true()


func test_snapshot_reports_a_tap_that_was_emitted() -> void:
	var level := _built_level()
	var obs := WaveObserver.new()
	obs.inject(level, level.pulses_for_debug(), null)
	level.pulses_for_debug().emit(0, Vector3.ZERO, 6.0, 5.5, 1.0, 0.0, Vector3.ZERO, -2.0)
	var snap: Dictionary = obs.snapshot(0.5)
	assert_int(snap["live_count"]).is_equal(1)
	var slot: Dictionary = snap["slots"][0]
	assert_int(slot["kind"]).is_equal(0)
	assert_float(slot["ring_radius"]).is_equal_approx(2.75, 0.001)
	assert_str(slot["state"]).is_equal("Live")
	level.free()


func test_the_shipped_level_has_no_object_id_violations() -> void:
	var level := _built_level()
	var obs := WaveObserver.new()
	obs.inject(level, level.pulses_for_debug(), null)
	var e: Dictionary = obs.explain_oids()
	assert_bool(e.has("unavailable")).is_false()
	assert_array(e["violations"]).is_empty()
	assert_array(e["pairs"]).is_not_empty()
	level.free()


func test_explain_ray_names_the_wall_between_spawn_and_fan() -> void:
	var level := _built_level()
	var obs := WaveObserver.new()
	obs.inject(level, level.pulses_for_debug(), null)
	var e: Dictionary = obs.explain_ray(Vector3(3.0, 0.9, 4.0), Vector3(8.6, 1.15, 4.4))
	assert_int(e["camera_crossings"]).is_equal(1)
	assert_float(e["hum_transmission"]).is_equal_approx(0.55, 0.0001)
	level.free()


func _built_level() -> WaveLevel:
	var scene: PackedScene = load("res://scenes/level_01.tscn")
	var level: WaveLevel = scene.instantiate()
	add_child(level)
	return level
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `./ci/pipeline.sh 2>&1 | tail -30` (or the gdUnit4 headless invocation it uses for a single suite).

Expected: FAIL — `Identifier "WaveObserver" not declared in the current scope`.

Note: `pulses_for_debug()` does not exist on `WaveLevel` yet either. If exposing the pool through the level proves awkward, inject the `Pulses` shim from the test directly instead — but keep the observer's `inject` signature as specified, because `main.gd` and the MCP loop both depend on it.

- [ ] **Step 3: Write the minimal implementation**

Create `rust/src/nodes/observer.rs`:

```rust
//! The debug observability boundary — `WaveObserver`.
//!
//! Adds no law. It holds references to the systems it was injected with,
//! calls the pure functions in `crate::observe`, and converts the results
//! to `VarDictionary` so GDScript's `JSON.stringify` can encode them. That
//! is the whole job.
//!
//! Every entry point refuses loudly when uninjected: a snapshot of nothing
//! and a snapshot of an empty world must never serialise the same.

use godot::classes::Camera3D;
use godot::prelude::*;

use crate::nodes::level::WaveLevel;
use crate::observe::{self, oids, ray};

/// The agent's window into the running wave engine.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct WaveObserver {
    level: Option<Gd<WaveLevel>>,
    camera: Option<Gd<Camera3D>>,
    base: Base<Node>,
}

#[godot_api]
impl WaveObserver {
    /// Hand the observer the systems to read. Called once by the
    /// composition root; nothing is owned, only borrowed.
    #[func]
    fn inject(&mut self, level: Option<Gd<WaveLevel>>, camera: Option<Gd<Camera3D>>) {
        self.level = level;
        self.camera = camera;
    }

    /// The whole state vector as of `now`.
    #[func]
    fn snapshot(&self, now: f64) -> VarDictionary {
        let Some(level) = self.level.as_ref() else {
            return unavailable("observer was never injected a level");
        };
        // …compose from level.bind() state, observe::frame(), and convert.
        // The implementer fills this in against the frame() signature from
        // Task 5; the shape is fixed by the tests above:
        //   { now, flick, live_count, slots: [...], next_eviction: {...},
        //     sources: [...], wall_rects: [...], wall_truncated, camera }
        let _ = level;
        todo!("compose per Task 5's frame() and convert to VarDictionary")
    }

    /// What the walls do to one sight line.
    #[func]
    fn explain_ray(&self, from: Vector3, to: Vector3) -> VarDictionary {
        let Some(level) = self.level.as_ref() else {
            return unavailable("observer was never injected a level");
        };
        let rects: Vec<Vector4> = level.bind().wall_rects().as_slice().to_vec();
        let top = WaveLevel::wall_height() as f32;
        let e = ray::explain_ray(from, to, &rects, top);
        let mut d = VarDictionary::new();
        d.set("camera_crossings", e.camera_crossings);
        d.set("source_crossings", e.source_crossings);
        d.set("hum_transmission", e.hum_transmission);
        d.set("source_transmission", e.source_transmission);
        d.set("walls", walls_to_array(&e));
        d
    }

    /// The touch graph and its colouring.
    #[func]
    fn explain_oids(&self) -> VarDictionary {
        let Some(level) = self.level.as_ref() else {
            return unavailable("observer was never injected a level");
        };
        let _ = level;
        todo!("read the level's boxes and oids, call oids::explain_oids_checked")
    }
}

/// The one refusal shape. A dictionary carrying only this key is how an
/// agent learns it asked a question that could not be answered — as
/// opposed to one whose answer happens to be empty.
fn unavailable(reason: &str) -> VarDictionary {
    let mut d = VarDictionary::new();
    d.set("unavailable", reason);
    d
}

fn walls_to_array(e: &ray::RayExplanation) -> Array<VarDictionary> {
    e.walls
        .iter()
        .map(|w| {
            let mut d = VarDictionary::new();
            d.set("index", w.index as i64);
            d.set("rect", w.rect);
            d.set("crossed", w.crossed);
            d.set("contains_origin", w.contains_origin);
            d
        })
        .collect()
}
```

**The two `todo!()` calls above are the implementer's work, not placeholders in the plan** — their exact shape depends on which accessors `WaveLevel` ends up exposing for its census boxes and oid assignments, which is a decision best made with `rust/src/nodes/level.rs` open. `explain_ray` is written out in full as the worked example to follow. If `WaveLevel` needs a new accessor, add it in this task and keep it `#[func]`-free unless GDScript needs it.

Register the class in `rust/src/nodes/mod.rs` alongside the existing modules:

```rust
mod observer;
```

Wire it in `game/scripts/main.gd`. Add the field beside the other systems:

```gdscript
## The agent's window into the engine — reads every system, drives none.
var observer: WaveObserver
```

and at the end of `_ready()`, after `settings` is added:

```gdscript
	observer = WaveObserver.new()
	observer.inject(level, player.camera)
	add_child(observer)
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `./ci/pipeline.sh 2>&1 | tail -30`

Expected: the full gate green, including the five new `observer_test.gd` cases.

- [ ] **Step 5: Verify the observer changed nothing**

The observer must be inert. Confirm the suite count for every pre-existing suite is unchanged and no existing test's timing shifted:

Run: `./ci/pipeline.sh 2>&1 | grep -E "tests?|passed|failed" | tail -20`

Expected: every previously passing suite still passes, with the new suite added and nothing else altered.

- [ ] **Step 6: Format, lint, commit**

```bash
cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && gdformat game/scripts/main.gd game/tests/observer_test.gd
gdlint game/scripts/main.gd game/tests/observer_test.gd
git add rust/src/nodes/observer.rs rust/src/nodes/mod.rs game/scripts/main.gd game/tests/observer_test.gd
git commit -F - <<'MSG'
A window into the engine that opens only outward

WaveObserver, the one registered class of the observability layer: it is
handed the level and the camera by the composition root, reads them, calls
the pure functions, and converts the results into dictionaries GDScript
can stringify. It adds no law and owns nothing.

Refusal is the part worth naming. Every entry point on an uninjected
observer returns a dictionary carrying only "unavailable" and a reason —
never zeros, never an empty slot list. A snapshot of nothing and a
snapshot of an empty world are different facts, and an agent that cannot
tell them apart will debug the wrong one for an hour.

Serialization stays at the boundary as VarDictionary plus JSON.stringify,
so the crate takes on no dependency and the wasm export does not grow to
carry a debugging tool it never runs.
MSG
```

---

### Task 7: Reflection, explained — the physics-context pair

`space.intersect_ray` is legal only inside the physics tick, so this cannot be a synchronous call. It splits into request and collect, exactly as `UnseeingPlayer` queues waves.

**Files:**
- Create: `rust/src/observe/reflect.rs`
- Modify: `rust/src/nodes/observer.rs`
- Modify: `game/tests/observer_test.gd`

**Interfaces:**
- Consumes: `crate::ray_fan::{RAYS, fan_directions}`, `crate::clustering::{RayHit, cluster_hits, echo_budget, ray_length, RAY_ORIGIN_LIFT}`.
- Produces:
  ```
  WaveObserver.request_explain_reflection(origin: Vector3, normal: Vector3, max_r: float, max_echoes: int) -> int
  WaveObserver.take_explanation(request_id: int) -> Dictionary
  ```
  `take_explanation` returns `{"pending": true}` until the physics frame has run, then the explanation exactly once, then `{"unavailable": "no such request"}`.

**The hard rule:** the fan's hits go into a scratch buffer. Nothing may reach the real `EchoQueue`. Asking why a wall did not answer must not schedule echoes.

- [ ] **Step 1: Write the failing test**

Add to `game/tests/observer_test.gd`:

```gdscript
func test_explaining_a_reflection_schedules_no_echoes() -> void:
	var level := _built_level()
	var obs := WaveObserver.new()
	add_child(obs)
	obs.inject(level, null)
	var pulses := level.pulses_for_debug()
	var before: int = pulses.pending_echo_count()
	var id: int = obs.request_explain_reflection(Vector3(3.0, 0.9, 4.0), Vector3.UP, 6.0, 6)
	await await_millis(100)
	var e: Dictionary = obs.take_explanation(id)
	assert_bool(e.has("pending")).is_false()
	assert_int(pulses.pending_echo_count()).is_equal(before)
	level.free()


func test_an_explanation_is_pending_before_the_physics_frame_runs() -> void:
	var level := _built_level()
	var obs := WaveObserver.new()
	add_child(obs)
	obs.inject(level, null)
	var id: int = obs.request_explain_reflection(Vector3(3.0, 0.9, 4.0), Vector3.UP, 6.0, 6)
	assert_bool(obs.take_explanation(id)["pending"]).is_true()
	level.free()


func test_an_unknown_request_id_is_refused() -> void:
	var obs := WaveObserver.new()
	assert_bool(obs.take_explanation(9999).has("unavailable")).is_true()


func test_the_explanation_reports_every_ray_not_only_the_hits() -> void:
	var level := _built_level()
	var obs := WaveObserver.new()
	add_child(obs)
	obs.inject(level, null)
	var id: int = obs.request_explain_reflection(Vector3(3.0, 0.9, 4.0), Vector3.UP, 6.0, 6)
	await await_millis(100)
	var e: Dictionary = obs.take_explanation(id)
	assert_int(e["rays_cast"]).is_equal(obs.ray_fan_size())
	assert_int(e["clusters_kept"]).is_less_equal(6)
	level.free()
```

- [ ] **Step 2: Run to verify it fails**

Run: `./ci/pipeline.sh 2>&1 | tail -30`

Expected: FAIL — `request_explain_reflection` not found.

- [ ] **Step 3: Implement**

The pure half in `rust/src/observe/reflect.rs` takes the already-gathered hits (so it stays engine-free and cargo-testable) and reports what clustering did with them:

```rust
//! Why a surface answered, or did not.
//!
//! The golden-angle fan and the clustering that follows it are computed
//! and discarded inside a single frame, so no snapshot can contain them.
//! This re-runs the same pure functions on demand and reports every ray —
//! including the ones that struck nothing, because absence of echo is
//! information and a report of only the hits would hide it.

use godot::builtin::Vector3;

use crate::clustering::{RayHit, cluster_hits, echo_budget};

#[derive(Debug, Clone, PartialEq)]
pub struct ReflectionExplanation {
    pub origin: Vector3,
    pub normal: Vector3,
    pub rays_cast: usize,
    pub rays_struck: usize,
    pub clusters_kept: usize,
    pub budget: usize,
    pub points: Vec<ClusteredPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusteredPoint {
    pub point: Vector3,
    pub dist: f64,
    /// When the echo would fire, given the primary's speed.
    pub at_t: f64,
}

/// Explain the clustering of an already-cast fan.
///
/// Takes the hits rather than casting them, so this half stays pure: the
/// raycasting is the boundary's job, because a space state is an engine
/// object and only the physics tick may touch it.
#[must_use]
pub fn explain_clustering(
    origin: Vector3,
    normal: Vector3,
    rays_cast: usize,
    hits: Vec<RayHit>,
    max_echoes: i64,
    speed: f64,
    now: f64,
) -> ReflectionExplanation {
    let rays_struck = hits.len();
    let budget = echo_budget(max_echoes);
    let points = cluster_hits(hits, budget)
        .into_iter()
        .map(|h| ClusteredPoint {
            point: h.point,
            dist: h.dist,
            at_t: now + h.dist / speed,
        })
        .collect::<Vec<_>>();
    ReflectionExplanation {
        origin,
        normal,
        rays_cast,
        rays_struck,
        clusters_kept: points.len(),
        budget,
        points,
    }
}
```

Write cargo tests for `explain_clustering` with hand-built `RayHit` vectors — one where several hits share a 0.9 m cell and must collapse to one point, one where the budget truncates and `clusters_kept` reports the smaller number.

In `observer.rs`, add the request queue: a `Vec<(i64, PendingRequest)>` and a `#[func] fn _physics_process(&mut self, _dt: f64)` that drains pending requests, casts the fan through `self.base().get_world_3d()`'s space state into a **local** `Vec<RayHit>`, calls `explain_clustering`, and stores the result against the request id. `take_explanation` removes and returns it.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `./ci/pipeline.sh 2>&1 | tail -30`

Expected: green, including the four new cases.

- [ ] **Step 5: Prove the non-mutation directly**

The `schedules_no_echoes` test is the important one. Deliberately break it once: make `_physics_process` call `pulses.emit_reflecting` instead of the scratch path, confirm the test fails, then revert. An untested non-mutation guarantee is not a guarantee.

- [ ] **Step 6: Format, lint, commit**

```bash
cd rust && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
cd .. && gdformat game/tests/observer_test.gd && gdlint game/tests/observer_test.gd
git add rust/src/observe/reflect.rs rust/src/observe/mod.rs rust/src/nodes/observer.rs game/tests/observer_test.gd
git commit -F - <<'MSG'
Asking why a wall stayed silent, without making it speak

The golden-angle fan and the clustering behind every echo are computed and
thrown away inside one frame, so no snapshot can hold them. This re-runs
them on demand and reports the whole fan — every ray, including the ones
that struck nothing. Absence of echo is how this world communicates its
shape, and a report listing only the hits would hide exactly the case
worth investigating.

It answers across a physics frame rather than in place, because a space
state may only be touched inside the physics tick — the same reason the
player queues its waves. Request returns an id; collect returns pending
until the frame has run, then the explanation once.

The hits go to a scratch buffer and never reach the echo queue. A question
that scheduled echoes would answer itself by changing the thing it asked
about, and the test that pins this was watched failing before it was
trusted.
MSG
```

---

### Task 8: The oracle contract — pin `explain_ray` against the GLSL

`game/tests/data_skins_test.gd` already holds `sight.rs` against `pulse_pool.gdshaderinc` line by line. Extending it to pin `explain_ray` promotes the oracle from a convenience to a contract: a shader edit that drifts from the Rust then fails the gate instead of misleading an agent weeks later.

**Files:**
- Modify: `game/tests/data_skins_test.gd`

- [ ] **Step 1: Read the existing suite**

Run: `sed -n '1,80p' game/tests/data_skins_test.gd` and find how it currently compares the GLSL constants and the Rust results. Follow that pattern exactly — do not invent a second comparison style.

- [ ] **Step 2: Write the failing test**

Add a case asserting that for a set of sight lines spanning the shipped map, `WaveObserver.explain_ray(...)["camera_crossings"]` equals the count the suite already derives from the shader-side constants. Include at minimum: the spawn-to-fan line (1 crossing), a same-room line (0), the two-wall diagonal (2), and a line that grazes an endpoint (0).

- [ ] **Step 3: Run to verify it fails**

Temporarily change `RECT_SHRINK` in `rust/src/sight.rs` from `0.02` to `0.2`, rebuild, and confirm the new case fails. Revert. **This is the point of the task** — a contract test that cannot fail when the two sides drift is decoration.

- [ ] **Step 4: Implement and confirm green**

Run: `./ci/pipeline.sh 2>&1 | tail -30`

- [ ] **Step 5: Commit**

```bash
gdformat game/tests/data_skins_test.gd && gdlint game/tests/data_skins_test.gd
git add game/tests/data_skins_test.gd
git commit -F - <<'MSG'
The oracle signs a contract with the shader

explain_ray is only worth trusting if it agrees with the GLSL it claims to
predict. The suite that already holds sight.rs against the shader include
now holds the explanation too, over sight lines spanning the shipped map:
the spawn-to-fan crossing, a clear same-room line, the two-wall diagonal,
and an endpoint graze.

Verified by breaking it on purpose — widening the occluder shrink made the
new case fail before the change was reverted. A contract test that cannot
fail when the two sides drift apart is decoration, and this one was
watched failing for the right reason.
MSG
```

**OPEN — residual gap, not closed by this task.** `explain_ray` calls into
`sight.rs` only (`rust/src/nodes/observer.rs:173-182`); no gate in this
repo executes GLSL. So no test anywhere — not Task 8's, not
`test_pool_slab_test_mirrors_the_rust_reference`'s literal-text pin — can
catch a shader-only edit that leaves `sight.rs` untouched. The text pin
covers only what it names as a substring, which is structurally blind to
*inserted* code; concretely, in `pulse_pool.gdshaderinc`, none of these are
pinned:

- the `for (int k = 0; k < 3; k++)` loop bound (narrowing it silently drops
  an axis from every wall's slab test)
- the `t0 > t1` early return
- the `lo`/`hi` rect packing from `rect.xy`/`rect.zw`
- the Z half of `wall_near` (only the X half is pinned)
- the axis-parallel branch body (the `abs(d[k]) < 1e-6` case)
- the `i >= u_wall_count` loop breaks

Closing this needs either a rendered pixel probe (boot the real scene,
render, and diff against what `explain_ray` predicts for the same camera
ray — `game/tests/probe/occlusion_probe.gd` is the existing windowed-probe
pattern to extend) or a checksum-shaped pin on the GLSL (hash the include's
normalized source and pin the hash, so ANY edit — inserted, deleted, or
reordered — forces a deliberate re-pin rather than a silent pass). Until
one of those exists, `explain_ray` is an oracle for **what Rust believes**,
never for **what the screen draws**, and an agent using it for shader
debugging must be told that explicitly rather than left to infer it from a
passing gate.

---

### Task 9: Install godot-mcp and document the loop

**Files:**
- Modify: `.gitignore`
- Modify: `test/repo_hygiene.sh`
- Create: `docs/superpowers/mcp/godot-mcp-loop.md`
- Modify: `CLAUDE.md`

- [ ] **Step 1: Install the addon**

```bash
npx @satelliteoflove/godot-mcp --install-addon "$(pwd)/game"
```

Then enable the plugin in the Godot editor under Project Settings → Plugins. Requires Godot 4.5+ (this project pins 4.7 in `.godot-version`) and Node.js 20+.

- [ ] **Step 2: Ignore it before it can be committed**

Add to `.gitignore`, with the reasoning inline as the file's existing entries do:

```
# The godot-mcp editor addon — a developer tool, never a shipped dependency.
# deploy.sh ships the tree by `git archive` into a bare repo, so anything
# committed under game/addons/ reaches the droplet and the wasm export; and
# ci/vendor-gdunit4.sh governs that directory on the assumption that
# gdUnit4 is the only thing in it. Install it locally, never commit it.
game/addons/godot_mcp/
```

- [ ] **Step 3: Write the failing hygiene check**

Add to `test/repo_hygiene.sh`, following the file's existing check style (including its index-guard convention, which skips loudly rather than passing vacuously when run without an index):

A check asserting `git ls-files game/addons/godot_mcp/` returns nothing, and that the path IS covered by `git check-ignore`. Both halves matter: the first catches a commit that already happened, the second catches an ignore rule someone deleted.

- [ ] **Step 4: Verify it fails, then passes**

```bash
mkdir -p game/addons/godot_mcp && touch game/addons/godot_mcp/plugin.cfg
git add -f game/addons/godot_mcp/plugin.cfg
./test/repo_hygiene.sh          # expect FAIL on the new check
git rm --cached game/addons/godot_mcp/plugin.cfg
./test/repo_hygiene.sh          # expect PASS
```

- [ ] **Step 5: Write the loop documentation**

Create `docs/superpowers/mcp/godot-mcp-loop.md` covering: the install command, the editor requirement (the server has no headless mode), the one-client-at-a-time limit, the `freeze → input → step → snapshot → explain` cycle with a worked example calling `WaveObserver`, and the explicit rule that a screenshot is the last resort, taken only when a digest contradicts itself.

- [ ] **Step 6: Point `CLAUDE.md` at it**

Add to the tooling section a short paragraph naming the loop doc, the gitignore rule and why it exists, and the fallback when the editor is unavailable.

- [ ] **Step 7: Commit**

```bash
./test/repo_hygiene.sh && ./ci/pipeline.sh 2>&1 | tail -10
git add .gitignore test/repo_hygiene.sh docs/superpowers/mcp/godot-mcp-loop.md CLAUDE.md
git commit -F - <<'MSG'
The driver arrives, and the gate learns to keep it out of the ship

godot-mcp installed as the live debugging driver: freeze the clock, inject
input, step exact frames, then ask WaveObserver what happened. The loop is
written down for a session that has never seen it, because a tool nobody
can rediscover is a tool nobody uses twice.

Its addon is gitignored, and the hygiene suite now enforces that from both
directions — nothing tracked under the path, and the ignore rule itself
still present. deploy.sh ships the tree by git archive into a bare repo, so
a committed dev addon would ride to the droplet and into the wasm export;
and vendor-gdunit4.sh governs game/addons/ on the assumption that gdUnit4
is the only tenant.

Verified by staging the addon on purpose and watching the check fail
before it was removed.
MSG
```

---

### Task 10: Write the wiki page

Per `CLAUDE.md`, the task is not done when the tests are green. It is done when the wiki describes the shipped behaviour.

**Files:**
- Create: `Engineering-Debugging-and-Observability.md` in the wiki repo (`unseeing.wiki.git`)
- Modify: `Engineering-Build-Test-Deploy.md` (wiki) — the new gate entries and the gitignored addon
- Modify: `Mechanics-Overview.md` (wiki) — link the new page from section 6

- [ ] **Step 1: Clone the wiki**

```bash
git clone https://github.com/cleveralbatraoz/unseeing.wiki.git /tmp/unseeing-wiki
```

- [ ] **Step 2: Write the page**

It describes what shipped, not what was decided — the spec holds the decision. Name the file that owns every constant quoted, as the other pages do. Cover: the four verbs and their entry points; the three transports; the live loop; the physics-context split on `explain_reflection`; the refusal contract; the `--fixed-fps` determinism requirement; and the two things Plan 1 deliberately does not do (digest, dump scene), so the next session does not go looking for them.

- [ ] **Step 3: Link it**

Add the page to `Mechanics-Overview.md`'s "Where to go next", and note the new suites in `Engineering-Build-Test-Deploy.md`.

- [ ] **Step 4: Commit and push the wiki**

```bash
cd /tmp/unseeing-wiki
git add -A
git commit -m "Debugging and observability: the four verbs, the three transports"
git push
```

- [ ] **Step 5: Record the crucial facts in memory**

Per `CLAUDE.md`, persist what changes future work: that `WaveObserver` exists and is the first thing to reach for when debugging; that the godot-mcp addon is gitignored and why; that `explain_reflection` answers across a physics frame; and the `dat.x = -1` NaN trap from Task 1.

---

## Self-Review

**Spec coverage.** Snapshot → Tasks 1, 5, 6. Diff → no code by design, per the spec. Explain → Tasks 2, 3, 4, 7. Digest → Plan 2, out of scope. Three transports → Task 6 (live + gdUnit4), Plan 2 (dump scene). Refusal contract → Task 6. Non-mutation → Task 7. Oracle contract → Task 8. Addon policy → Task 9. Wiki → Task 10. **One gap found and accepted:** the spec's `--fixed-fps` determinism requirement has no task here, because nothing in Plan 1 writes a trace file — it belongs to Plan 2's dump scene and Task 10 documents it as pending.

**Snapshot groups: what shipped, and the one thing deliberately left out.** The final whole-branch review found four of the spec's snapshot groups missing with the omission recorded nowhere — which breaks this layer's own contract, since `unknown` promises that anything unobservable is *named*. Three were added rather than argued away: the whole **`echoes`** group (every pending appointment with the seconds until it fires), **`sources.cadence`** and **`sources.next_emit`**, **`view.fov`**, and **`spawn`** position and yaw. The spec's `level.oid per solid` lives in `explain_oids()` — `names` and `oids`, parallel and complete — rather than in the snapshot, because it is a table no frame changes and duplicating it per snapshot would invite the two copies to disagree.

**Deliberately not in the snapshot: `breath`.** `u_breath` (`game/scripts/main.gd`) is pushed to the hearing-pass material alone, which the observer is not injected with, and it is a pure function of `now` that scales the mood vignette. It changes no wave, no geometry, no occlusion and no timing, so no bug this layer exists to diagnose can be caused by it or hidden from it — where `flick`, which IS carried, gates reveal intensity and can drop to zero for a frame. Adding it would mean injecting a third material into the observer to report a number an agent can compute from `now`. Recorded here and in the wiki's snapshot section so the next session finds a decision rather than a hole.

**Placeholder scan.** The two `todo!()` calls in Task 6 are flagged in prose as the implementer's work with the worked example beside them, not as unstated requirements. Task 8's test bodies are specified by their assertions rather than transcribed, because they must follow the existing suite's comparison style, which the implementer reads in Step 1. Everything else carries its code.

**Type consistency.** `slots()` returns `Vec<SlotObservation>` and is consumed under that name in Tasks 2 and 5. `EvictionPlan`/`EvictionRule` are consistent between Tasks 2 and 5. `explain_oids_checked` (returns `Option`) versus `explain_oids` (panics) is used deliberately and consistently — the boundary in Task 6 calls the checked form. `RayExplanation` field names match between Tasks 3 and 6. `ray_fan_size()` in Task 7's test already exists on `WaveCore` and must be re-exposed on `WaveObserver` or read from the pulses shim — flagged for the implementer.
