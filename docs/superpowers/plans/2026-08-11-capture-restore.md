# Capture and Restore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. The user's standing execution choice is subagent-driven — do not offer the inline option.

**Goal:** An agent captures a running game as one restorable blob, launches a fresh boot *into* that state, and can prove the restore is exact — the launch-from-state heart of the reproduction loop.

**Architecture:** Pure capture/restore law in `rust/src/reproduce/blob.rs` (state structs, canonical bytes, FNV-1a hash, first-divergence); one new subsystem door per module that owns private state (pool, echo book, cadence, cat brain/gait/tail, viewmodel, player); `capture()` joins `WaveObserver` (a read, wider than `snapshot()`); one new write-side node `WaveRestorer` applies a blob as a refuse-or-succeed transaction; the gates (round-trip, advance-and-compare, deliberate break) make omission — serialization's silent killer — loud.

**Tech Stack:** Rust (gdext 0.5.4, stable channel per `rust/rust-toolchain.toml`), Godot 4.7, typed GDScript, gdUnit4 (vendored), bash.

**Source spec:** `docs/superpowers/specs/2026-08-11-reproduction-loop-design.md`. This is Plan 2 of 3. Plan 1 (the determinism substrate) is merged and live at `18c09e4`: `UNSEEING_SEED` seeds without arming the demo tap, `tap()`/`look()` drive real input paths, the snapshot carries a `hero` group, and `tools/determinism_probe.sh` gates two seeded `--fixed-fps 60` boots on one hash. Plan 3 (tape, primitives, `tools/reproduce.sh`, the diff verb) comes after this and is out of scope here.

**Read before implementing** (a fresh session must not skip these): the spec above; the wiki pages *Engineering — Debugging and Observability* (the observer's contracts this plan extends) and *Mechanics — Waves* (the pool contract every slot capture encodes).

## Global Constraints

These apply to every task. Copied from `CLAUDE.md` and the spec.

- **No new crate dependencies.** Serialization is `VarDictionary` + GDScript `JSON.stringify`; the hash is a hand-rolled FNV-1a (std `DefaultHasher` is not run-stable). No serde. The wasm export must not grow.
- **No `unsafe` Rust.** The crate is `#![deny(unsafe_code)]`; the only permitted exception is the existing `unsafe impl ExtensionLibrary` in `ffi.rs`. Never add another.
- **One Rust GDExtension per wasm export.** Everything joins the single `unseeing-core` crate.
- **The two layers.** All law lives in pure modules (`rust/src/*.rs`) that compile and test without a Godot runtime. Engine types (`Gd<T>`, `Node`, `VarDictionary`) appear ONLY in `rust/src/ffi.rs` and `rust/src/nodes/*.rs`. A boundary module carries values and adds no law. (`godot::builtin` value types — `Vector3`, `Vector4` — are fine in pure modules; the existing pure modules already use them.)
- **Architecture independence.** No arch-specific code. Must build for x86_64, aarch64, wasm32. Blob and hash are same-build, same-platform artifacts by declared scope — but the *code* stays portable.
- **Observation never mutates.** `capture()` and everything it calls take `&self`. Nothing may emit a pulse, schedule an echo, advance a cadence, or move a node. Restore mutates on purpose — on `WaveRestorer` and the doors it calls, never on the observer.
- **A vacuous pass is worse than a failure.** The blob is ALL-OR-NOTHING: any subsystem that cannot answer → one-key `{"unavailable": reason}` refusal, never a blob missing a group. Restore that cannot prove itself (post-restore hash mismatch) refuses naming the first divergent field. A partial blob or a silent partial restore is the defect this whole plan exists to prevent.
- **Perception laws are untouched.** No rendering, geometry, light, or fill changes. If a task seems to need one, stop and ask.
- **Object id law:** untouched by this plan; nothing here paints.
- **Commits:** small, self-contained, each one green. Narrative subject line, technical body. **No `Co-Authored-By`, no "Generated with", no mention of Claude, AI, or any assistant anywhere in the repository.** Repo identity is `Dmitrii Galchenko <dggrus@gmail.com>`.
- **Tooling before every commit:** `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; `gdformat` + `gdlint` for GDScript.
- **TDD is mandatory:** write the test, watch it fail *for the right reason* (capture real output), minimal code, watch it pass. Production code written before its test gets deleted, not retrofitted.
- **Test literals are hand-derived from contracts, never mirrored from the code under test.** The pool contract (Waves wiki page), FNV-1a's published test vectors, and the capture structs' own definitions are the sources.

## Environment facts the implementer needs

- Godot binary: `/opt/homebrew/bin/godot` (or `GODOT` env; `ci/pipeline.sh` resolves it).
- Run one suite: `/opt/homebrew/bin/godot --headless --path game -s res://addons/gdUnit4/bin/GdUnitCmdTool.gd -a res://tests/<file>.gd`
- **Gate trap:** the gdUnit runner can exit 0 on a suite that fails to PARSE ("No test cases found") and prints green PASSED on lines carrying failures. Trust ONLY the exit code and the executed case count (198 cases / 25 suites before this plan).
- Rebuild the extension before Godot sees Rust changes: `cd rust && cargo build`.
- Deterministic runs: `UNSEEING_SEED=1` + `--fixed-fps 60` (Plan 1's substrate; `tools/determinism_probe.sh` is the reference invocation).

---

## File Structure

**Created:**

| File | Responsibility |
|---|---|
| `rust/src/reproduce/mod.rs` | module root: re-exports, `FORMAT_VERSION` |
| `rust/src/reproduce/blob.rs` | `CaptureState` + group structs, canonical bytes, FNV-1a, `first_divergence` (pure) |
| `rust/src/nodes/restorer.rs` | `WaveRestorer` — the one write-side node (boundary) |
| `game/tests/rng_state_test.gd` | pins Godot's `RandomNumberGenerator.state` round-trip semantics |
| `game/tests/restore_test.gd` | the restore transaction against the live scene |
| `game/tests/probe/restore_probe.gd` | headless A/B probe for advance-and-compare |
| `tools/restore_probe.sh` | the advance-and-compare gate |

**Modified:**

| File | Change |
|---|---|
| `rust/src/cat_brain.rs` | `Pcg32::capture/restore`; `BrainState` (pub mirror of `State`); `CatBrain::capture/restore` |
| `rust/src/cat_gait.rs` | `GaitCapture`; `CatGait::capture/restore` |
| `rust/src/cat_body.rs` | `Tail::restore(nodes)` |
| `rust/src/pulse_pool.rs` | `SlotCapture`; `PulsePool::capture_slots/from_slots` |
| `rust/src/echo_queue.rs` | `EchoQueue::capture/from_pending` |
| `rust/src/sound_source.rs` | `Cadence::restore(interval, next)` |
| `rust/src/nodes/source.rs` | `SourceRig::restore_cadence`; `SoundSource` trait gains **required** `restore_appointment` |
| `rust/src/nodes/fan.rs`, `rust/src/nodes/radio.rs` | implement `restore_appointment` |
| `rust/src/nodes/cat.rs` | `WaveCat::capture_state/restore_state` (`pub(crate)`) |
| `rust/src/viewmodel.rs` | `ViewmodelCapture`; `Viewmodel::capture/restore` |
| `rust/src/nodes/hero.rs` | `HeroBody::capture_vm/restore_vm` (`pub(crate)`) |
| `rust/src/nodes/player.rs` | `pub(crate) set_eye_pitch`, `pub(crate) clear_wave_queue` (restore rebuilds via existing `queue_wave`/`tap`) |
| `rust/src/ffi.rs` | `WaveCore::capture_pool/capture_echoes/restore_state` (`pub(crate)`) |
| `rust/src/nodes/level.rs` | `pub(super) cat_handles()` |
| `rust/src/nodes/observer.rs` | `inject_body`; `#[func] capture(now, env)`; `pulse_core` becomes `pub(super)` |
| `rust/src/lib.rs` | `mod reproduce;` |
| `rust/src/nodes/mod.rs` | `mod restorer;` |
| `game/scripts/main.gd` | `capture_env()`/`apply_env()`; construct + inject `WaveRestorer`; blob file helpers |
| `ci/pipeline.sh` | new stage: `tools/restore_probe.sh` after the determinism probe |

**Task order:** 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10. Tasks 2–6 are the subsystem doors (independent of each other, but keep the order — later tasks' tests reuse earlier doors). Task 7 composes them; 8 reads; 9 writes; 10 proves.

**Naming rule used throughout:** every capture struct is the owning module's own type (`SlotCapture` in `pulse_pool.rs`, `BrainCapture` in `cat_brain.rs`, …), so each module still owns its privacy — `reproduce/blob.rs` composes them and adds no reach-ins.

---

### Task 1: The RNG doors — Pcg32 opens, and Godot's RNG is pinned

Two RNGs hold hidden stream state. The cat's `Pcg32` (`rust/src/cat_brain.rs:60-101`) has two private `u64`s (`state`, `inc`) and only fresh-stream construction — a restored cat diverges on its next whim without them. The flicker's `RandomNumberGenerator` has a documented read/write `state` property, but that engine claim has never been verified in this repo — and the whole env restore rests on it.

**Files:**
- Modify: `rust/src/cat_brain.rs` (two methods on `Pcg32`, next to `new` at line 72)
- Create: `game/tests/rng_state_test.gd`

**Interfaces:**
- Produces: `Pcg32::capture(&self) -> (u64, u64)` (state, inc) and `Pcg32::restore(state: u64, inc: u64) -> Pcg32`. Task 5's `BrainCapture` carries the pair; Task 9's env restore relies on the gdUnit-pinned `RandomNumberGenerator.state` semantics.

- [ ] **Step 1: Write the failing cargo test**

In `rust/src/cat_brain.rs`'s test module:

```rust
    /// A captured stream, restored, continues EXACTLY where the original
    /// would have — the property every cat restore rests on. Literals are
    /// the module's own pinned reference stream (srandom(42, 54)), not
    /// values read back from the code under test.
    #[test]
    fn a_restored_stream_continues_where_the_original_left_off() {
        let mut original = Pcg32::new(42, 54);
        let _ = original.next_u32(); // 0xa15c02b7, per the pinned stream
        let (state, inc) = original.capture();
        let mut restored = Pcg32::restore(state, inc);
        // both must produce the identical next five draws
        for _ in 0..5 {
            assert_eq!(restored.next_u32(), original.clone().next_u32());
            let _ = original.next_u32();
        }
    }
```

Note the shape: `original.clone()` peeks the next draw without advancing past it, then `original` advances — restored and original walk in lockstep.

- [ ] **Step 2: Run it, watch it fail for the right reason**

Run: `cd rust && cargo test a_restored_stream` — Expected: compile error, no method `capture` on `Pcg32`.

- [ ] **Step 3: Implement the pair**

In `impl Pcg32`, after `new`:

```rust
    /// The raw stream words, for capture. An advanced PCG32 cannot be
    /// rebuilt from its seed — the draws already taken are gone — so the
    /// capture is the two words themselves.
    #[must_use]
    pub fn capture(&self) -> (u64, u64) {
        (self.state, self.inc)
    }

    /// Rebuild a stream at an exact position, from a capture. Total: any
    /// two words form a valid PCG32 state (inc's low bit being set is a
    /// property `new` guarantees and `capture` preserves).
    #[must_use]
    pub fn restore(state: u64, inc: u64) -> Self {
        Self { state, inc }
    }
```

- [ ] **Step 4: Run it, watch it pass** — `cargo test a_restored_stream`, then the whole module: `cargo test cat_brain`.

- [ ] **Step 5: Write the failing gdUnit pin for Godot's RNG**

Create `game/tests/rng_state_test.gd`:

```gdscript
extends GdUnitTestSuite
## Pins the engine claim the env restore rests on: RandomNumberGenerator's
## `state` property is the complete stream position — read it, draw, write
## it back, and the stream REPLAYS the same draw. If a Godot upgrade ever
## breaks this, the flicker restore silently diverges; this suite is the
## tripwire. (`seed` is NOT sufficient: its getter returns the last seed
## assigned, not the current position — also pinned here.)


func test_state_round_trip_replays_the_stream() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 0x5EED
	rng.randf()  # advance somewhere mid-stream
	var mark: int = rng.state
	var expected := rng.randf()
	rng.state = mark
	assert_float(rng.randf()).is_equal(expected)


func test_seed_alone_does_not_carry_the_position() -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 0x5EED
	var first := rng.randf()
	rng.randf()
	# seed reads back as assigned even though the stream has moved on
	assert_int(rng.seed).is_equal(0x5EED)
	var again := RandomNumberGenerator.new()
	again.seed = 0x5EED
	assert_float(again.randf()).is_equal(first)
```

- [ ] **Step 6: Run the suite** — Expected: both PASS immediately (they pin engine behaviour, not new code). If either FAILS, STOP: the env-restore design is wrong for this engine build — report BLOCKED with the output; do not proceed to Task 9 on a broken assumption. This is a pin, not a red step — the red discipline applies to the cargo test above, which you watched fail in Step 2.

- [ ] **Step 7: Tooling + commit**

`cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test`; `gdformat`/`gdlint` the new suite; full gdUnit gate (case count grows by 2). Commit both files. Subject: *"Two streams learn to stand still: the cat's words open, the engine's are pinned"*. Body: why an advanced PCG32 cannot be rebuilt from its seed, and what the gdUnit tripwire protects.

---

### Task 2: The pool door — slots move out and back with their f64 shadow

`PulsePool`'s six arrays are private (`rust/src/pulse_pool.rs:69-77`); the f64 shadow (`t0`/`end`/`kind`) has **no accessor at all**, and eviction compares it at full width. Restore-by-emit is structurally impossible (the slot scan always fills the first expired slot — holes are unreachable; `emit` re-derives `end` and clamps gain). So the door is verbatim: capture copies all six lanes per slot; restore writes them back bit-identically, holes, expired lanes, virgin asymmetry (`dat.x = −1` but `t0 = 0.0`) and all.

**Files:**
- Modify: `rust/src/pulse_pool.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct SlotCapture {   // all fields pub, Copy
      pub pos: Vector3, pub dat: Vector4, pub dir: Vector4,
      pub t0: f64, pub end: f64, pub kind: i32,
  }
  impl PulsePool {
      pub fn capture_slots(&self) -> Box<[SlotCapture; MAXP]>;
      pub fn from_slots(slots: &[SlotCapture; MAXP]) -> PulsePool;
  }
  ```
  Task 7's blob carries `Box<[SlotCapture; MAXP]>`; Task 9 restores through `from_slots`. (Boxed: 64 slots × ~72 bytes stays off the stack in composed structs.)

- [ ] **Step 1: Write the failing tests**

In `pulse_pool.rs`'s test module:

```rust
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
        pool.emit_omni(0, Vector3::ZERO, 6.0, 5.5, 1.0, 0.0).unwrap();
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
        pool.emit_omni(2, Vector3::ZERO, 60.0, 4.0, 0.8, base).unwrap();
        assert_eq!(pool.dat()[0].x, pool.dat()[1].x); // f32 cannot tell
        let mut restored = PulsePool::from_slots(&pool.capture_slots());
        // pool full of live waves? No — 62 virgin slots remain; fill
        // rule (1) takes the first expired/virgin slot for both pools.
        // The discriminating emit: claim every remaining slot first...
        for i in 0..62 {
            let t = base + 1.0 + f64::from(i) * 1e-6;
            pool.emit_omni(0, Vector3::ZERO, 600.0, 4.0, 1.0, t).unwrap();
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
```

- [ ] **Step 2: RED** — `cargo test a_captured_pool` → compile error, no `SlotCapture`/`capture_slots`.

- [ ] **Step 3: Implement**

In `pulse_pool.rs`, below the accessors:

```rust
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

impl PulsePool {
    /// Every slot, verbatim — holes, expired lanes and virgin sentinels
    /// included, because slot_scan_limit and future eviction read them.
    #[must_use]
    pub fn capture_slots(&self) -> Box<[SlotCapture; MAXP]> {
        let mut slots = Box::new([SlotCapture {
            pos: Vector3::ZERO,
            dat: Vector4::ZERO,
            dir: Vector4::ZERO,
            t0: 0.0,
            end: 0.0,
            kind: 0,
        }; MAXP]);
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
    /// values are legal — the capture is trusted verbatim, and the hash
    /// gate (reproduce/blob.rs) is where tampering shows.
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
}
```

- [ ] **Step 4: GREEN** — both tests, then `cargo test pulse_pool`.

- [ ] **Step 5: Mutation check** — temporarily make `from_slots` skip the `t0` copy: the eviction-equivalence test must fail (this is exactly the omission it exists to catch). Revert. Record the observed failure in your report.

- [ ] **Step 6: Tooling + commit** — subject: *"The pool's shadow learns to travel"*. Body: why verbatim beats decode-reencode, and the ULP demonstration.

---

### Task 3: The echo book door — appointments move in discovery order

`EchoQueue.pending` is private; `schedule()` always applies its two transforms (`at_t = now + d/speed`, `gain × 0.55/(1+0.4d)`), so restoring a captured appointment through it would require inverting both through an f32 — not bit-exact. `PendingEcho`'s fields are all pub and the drain order is load-bearing ("Pool slot assignment … depends on this order — it must not drift", `echo_queue.rs:76-79`). The door is a verbatim `Vec` move.

**Files:**
- Modify: `rust/src/echo_queue.rs`

**Interfaces:**
- Produces: `EchoQueue::capture(&self) -> Vec<PendingEcho>` and `EchoQueue::from_pending(pending: Vec<PendingEcho>) -> EchoQueue`. Task 7 carries the Vec; Task 9 restores.

- [ ] **Step 1: Write the failing test**

```rust
    /// A restored book drains in the ORIGINAL's order — the pinned
    /// reverse-index walk over discovery order, which pool slot
    /// assignment depends on. Appointments deliberately NOT in at_t
    /// order, so an implementation that sorted would be caught.
    #[test]
    fn a_restored_book_drains_in_the_original_order() {
        let mut book = EchoQueue::new();
        book.schedule(0.0, 5.5, Vector3::new(1.0, 0.0, 0.0), 1.0, 5.5);
        book.schedule(0.0, 2.2, Vector3::new(2.0, 0.0, 0.0), 1.0, 5.5);
        book.schedule(0.0, 4.4, Vector3::new(3.0, 0.0, 0.0), 1.0, 5.5);
        let mut restored = EchoQueue::from_pending(book.capture());
        assert_eq!(restored.pending(), book.pending());
        let fired_original = book.drain(2.0);
        let fired_restored = restored.drain(2.0);
        assert_eq!(fired_restored, fired_original);
        assert_eq!(restored.pending(), book.pending()); // survivors too
    }
```

- [ ] **Step 2: RED** — compile error, no `capture`/`from_pending`.

- [ ] **Step 3: Implement**

```rust
    /// The whole book, verbatim, in discovery order — the order the
    /// pinned drain walks. Never sorted: slot assignment depends on it.
    #[must_use]
    pub fn capture(&self) -> Vec<PendingEcho> {
        self.pending.clone()
    }

    /// A book rebuilt from a capture. The Vec is taken as-is — restoring
    /// through schedule() would re-apply the falloff and re-narrow the
    /// distance through f32, neither of which round-trips.
    #[must_use]
    pub fn from_pending(pending: Vec<PendingEcho>) -> Self {
        Self { pending }
    }
```

- [ ] **Step 4: GREEN**, then `cargo test echo_queue`.

- [ ] **Step 5: Tooling + commit** — subject: *"The echo book copies over, appointment for appointment"*.

---

### Task 4: The cadence door — an appointment can finally be re-booked

`Cadence`'s two fields are private; `Cadence::every(interval)` hard-codes `next = interval`; `retune` keeps the standing appointment; nothing can express "mid-flight, next beat at T". The trap this closes (spec: "restoring the clock fires one spurious beat from every source"): after a clock jump, `beat(t)` fires immediately for any stale appointment. Restore must re-pin each gate to its captured `next` **after** the clock lands, so nothing fires spuriously and an *overdue* captured appointment stays overdue.

**Files:**
- Modify: `rust/src/sound_source.rs` (one constructor on `Cadence`)
- Modify: `rust/src/nodes/source.rs` (`SourceRig::restore_cadence`; `SoundSource` trait gains **required** `fn restore_appointment(&mut self, next: f64)`)
- Modify: `rust/src/nodes/fan.rs`, `rust/src/nodes/radio.rs` (implement it)

**Interfaces:**
- Produces:
  ```rust
  impl Cadence { pub fn restore(interval: f64, next: f64) -> Cadence; }
  impl SourceRig { pub(crate) fn restore_cadence(&mut self, cadence: Cadence); }
  trait SoundSource { fn restore_appointment(&mut self, next: f64); /* REQUIRED — no default */ }
  ```
  Task 9 iterates `source_handles()` and calls `restore_appointment(next)` per source, in scene order. **Required, not defaulted, on purpose:** a future source that forgot to implement it would otherwise silently keep a stale gate — the compiler is the reminder.
- Consumes: `Cadence::next_at()` (the read side, already shipped via `SoundSource::next_emit`).

- [ ] **Step 1: Write the failing cargo tests** (in `sound_source.rs`)

```rust
    /// A restored gate holds EXACTLY the captured appointment: nothing
    /// fires before it, the appointment fires on time, and the next
    /// rebooking runs on the restored interval. Literals hand-picked:
    /// interval 4.0, appointment at 10.0, clock restored to 9.0.
    #[test]
    fn a_restored_appointment_stands_and_nothing_fires_early() {
        let mut gate = Cadence::restore(4.0, 10.0);
        assert_eq!(gate.next_at(), Some(10.0));
        assert_eq!(gate.beat(9.0), None); // the restore instant: silence
        assert_eq!(gate.beat(10.0), Some(10.0)); // fires on the dot
        assert_eq!(gate.next_at(), Some(14.0)); // rebooks from t
    }

    /// An OVERDUE captured appointment stays overdue and fires on the
    /// very next beat — exactly as it would have in the original run.
    #[test]
    fn an_overdue_restored_appointment_fires_at_once() {
        let mut gate = Cadence::restore(4.0, 8.0);
        assert_eq!(gate.beat(9.0), Some(9.0));
        assert_eq!(gate.next_at(), Some(13.0));
    }
```

- [ ] **Step 2: RED** — no `restore` on `Cadence`.

- [ ] **Step 3: Implement `Cadence::restore`**

Next to `every`:

```rust
    /// A gate rebuilt mid-flight: the interval AND the standing
    /// appointment, exactly as captured. `every` cannot express this (it
    /// books one interval out) and `retune` deliberately keeps the old
    /// date — this is the one door for a restored clock, and re-pinning
    /// through it AFTER the clock lands is what keeps a jumped clock
    /// from buying its one spurious beat per source.
    #[must_use]
    pub fn restore(interval: f64, next: f64) -> Self {
        Self {
            every: interval,
            next,
        }
    }
```

- [ ] **Step 4: GREEN**, then the rig and trait plumbing (compiler-driven — this is the step where fan and radio stop compiling until they implement the method):

`rust/src/nodes/source.rs`:

```rust
    /// Replace the rig's gate wholesale — the restore door. The limbs are
    /// untouched: geometry is derived from the scene, only the clock is
    /// state.
    pub(crate) fn restore_cadence(&mut self, cadence: Cadence) {
        self.cadence = cadence;
    }
```

On the `SoundSource` trait (no default body — see Interfaces):

```rust
    /// Re-pin this source's beat appointment to a captured date. Called
    /// by the restorer AFTER the clock lands, so the jumped-clock law
    /// (one beat per jump) never fires on a restore. Required, not
    /// defaulted: a source that cannot restore its gate is a source a
    /// blob cannot carry, and the compiler says so at the source.
    fn restore_appointment(&mut self, next: f64);
```

`fan.rs` and `radio.rs`, identical bodies in their `impl SoundSource` blocks:

```rust
    fn restore_appointment(&mut self, next: f64) {
        let interval = self.voice().cadence;
        self.rig.restore_cadence(Cadence::restore(interval, next));
    }
```

(Both files import `Cadence` from `crate::sound_source` — `fan.rs` and `radio.rs` already import `Voice` from there; extend the use line.)

- [ ] **Step 5: Full cargo suite** — the trait change must break nothing else (only fan and radio implement `SoundSource`; the compiler proves it).

- [ ] **Step 6: Tooling + commit** — subject: *"A beat appointment survives the journey"*. Body: the spurious-beat law and why the trait method is required rather than defaulted.

---

### Task 5: The cat doors — a whole life moves mid-whim

The cat is the deepest capture: `CatBrain` (private `Pcg32`, private `State` enum with `Pause/Sit { left }` payloads, `rect`, `yaw`, eased `speed`, `blocked`), `CatGait` (`phase`, `amp`, `planted[4]`, `aim[4]`, `in_swing[4]`, hysteretic `moving` — unrecoverable from outputs), `Tail` (5 nodes; `Tail::new` *settles* by running 120 iterations — an arbitrary mid-sway chain needs a verbatim door), and `WaveCat`'s node fields (`presence` gate, `sit` blend, `sim_t`, `last_pos`, `pose`). All of it moves, or the cat's future diverges at the first whim.

**Files:**
- Modify: `rust/src/cat_brain.rs` (`BrainState` pub enum, `BrainCapture`, `CatBrain::capture/restore`)
- Modify: `rust/src/cat_gait.rs` (`GaitCapture`, `CatGait::capture/restore`)
- Modify: `rust/src/cat_body.rs` (`Tail::restore`)
- Modify: `rust/src/nodes/cat.rs` (`CatCapture`, `WaveCat::capture_state/restore_state`, both `pub(crate)`)

**Interfaces:**
- Produces (pure, all fields pub):
  ```rust
  // cat_brain.rs
  pub enum BrainState { Roam { tx: f64, tz: f64 }, Pause { left: f64 }, Sit { left: f64 } }
  pub struct BrainCapture { pub rng_state: u64, pub rng_inc: u64, pub rect: RoamRect,
                            pub state: BrainState, pub yaw: f64, pub speed: f64, pub blocked: f64 }
  impl CatBrain { pub fn capture(&self) -> BrainCapture;
                  pub fn restore(capture: BrainCapture) -> CatBrain; }
  // cat_gait.rs
  pub struct GaitCapture { pub phase: f64, pub amp: f64, pub planted: [Vector3; LEGS],
                           pub aim: [Vector3; LEGS], pub in_swing: [bool; LEGS], pub moving: bool }
  impl CatGait { pub fn capture(&self) -> GaitCapture;
                 pub fn restore(capture: GaitCapture) -> CatGait; }
  // cat_body.rs
  impl Tail { pub fn restore(nodes: [Vector3; TAIL_N]) -> Tail; }  // capture via existing nodes()
  ```
- Produces (boundary, `rust/src/nodes/cat.rs`):
  ```rust
  pub(crate) struct CatCapture {
      pub position: Vector3, pub yaw: f64, pub velocity: Vector3,
      pub brain: BrainCapture, pub gait: GaitCapture,
      pub tail: [Vector3; TAIL_N], pub pose: CatPose,
      pub presence_next: f64, pub sit: f64, pub sim_t: f64, pub last_pos: Vector3,
  }
  impl WaveCat {
      pub(crate) fn capture_state(&self) -> Option<CatCapture>;  // None = never built (_ready refused)
      pub(crate) fn restore_state(&mut self, capture: &CatCapture);
  }
  ```
  `CatPose` is already all-pub (`cat_body.rs:105-118`) — carried verbatim so `paw_positions()`/`mood()` answer correctly between the restore and the first physics tick. (If `CatPose` lacks a `Clone` derive, add it — a pure value struct, the derive is the fix, not a design question.) `presence_next` re-enters through `Cadence::restore(cat_gait::PRESENCE_EVERY, next)` (Task 4's door).
- Consumes: `Pcg32::capture/restore` (Task 1), `Cadence::restore` (Task 4), `Cadence::next_at`.

- [ ] **Step 1: Write the failing cargo tests**

`cat_brain.rs`:

```rust
    /// The restored brain lives the SAME future: drive a brain to
    /// mid-life, capture, then advance both original and restored with
    /// identical inputs — every Drive must match. This is the
    /// same-seed-same-life law, lifted to same-capture-same-future.
    #[test]
    fn a_restored_brain_lives_the_same_future() {
        let rect = RoamRect::around(Vector3::ZERO, 6.0, 6.0);
        let mut original = CatBrain::new(7, rect, 0.3);
        let mut pos = Vector3::ZERO;
        for _ in 0..120 {
            let drive = original.advance(0.1, pos, 0.05);
            pos += Vector3::new(0.05, 0.0, 0.02) * (drive.speed as f32);
        }
        let mut restored = CatBrain::restore(original.capture());
        assert_eq!(restored, original); // PartialEq on the whole brain
        for _ in 0..200 {
            let a = original.advance(0.1, pos, 0.04);
            let b = restored.advance(0.1, pos, 0.04);
            assert_eq!(a.speed, b.speed);
            assert_eq!(a.yaw, b.yaw);
            assert_eq!(a.sitting, b.sitting);
            pos += Vector3::new(0.03, 0.0, 0.01);
        }
    }
```

`cat_gait.rs` (same shape — drive to mid-stride, capture, `assert_eq!(restored, original)`, then 100 lockstep `advance(dt, pos, yaw, speed)` calls comparing whole `GaitFrame`s; use a moving position so swings and touchdowns occur; hand-pick dt 0.05, speed 0.4):

```rust
    #[test]
    fn a_restored_gait_walks_the_same_stride() {
        let mut pos = Vector3::ZERO;
        let mut original = CatGait::new(pos, 0.0);
        for _ in 0..40 {
            pos += Vector3::new(0.02, 0.0, 0.0);
            let _ = original.advance(0.05, pos, 0.0, 0.4);
        }
        let mut restored = CatGait::restore(original.capture());
        assert_eq!(restored, original);
        for _ in 0..100 {
            pos += Vector3::new(0.02, 0.0, 0.0);
            let a = original.advance(0.05, pos, 0.0, 0.4);
            let b = restored.advance(0.05, pos, 0.0, 0.4);
            assert_eq!(a, b);
        }
    }
```

`cat_body.rs`:

```rust
    /// A mid-sway tail restores verbatim — Tail::new SETTLES (120
    /// iterations toward rest), so a restore door must bypass it.
    #[test]
    fn a_restored_tail_holds_its_exact_curve() {
        let rv = Vector3::new(1.0, 0.0, 0.0);
        let mut tail = Tail::new(Vector3::ZERO, Vector3::new(0.0, 0.1, -0.2), rv);
        for i in 0..30 {
            let root = Vector3::new(f32::from(i as u8) * 0.01, 0.0, 0.0);
            tail.advance(0.05, root, root + Vector3::new(0.0, 0.1, -0.2), rv, 0.2, 0.1);
        }
        let restored = Tail::restore(*tail.nodes());
        assert_eq!(restored.nodes(), tail.nodes());
    }
```

- [ ] **Step 2: RED** — compile errors for the missing types/methods, one module at a time.

- [ ] **Step 3: Implement the pure doors**

`cat_brain.rs` — the pub mirror and the pair (place `BrainState` beside the private `State`; conversion is a straight 1:1 match both ways):

```rust
/// The private state machine's public mirror, for capture. One variant
/// per State, payloads included — the Pause/Sit countdown is exactly the
/// state a restored cat resumes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrainState {
    Roam { tx: f64, tz: f64 },
    Pause { left: f64 },
    Sit { left: f64 },
}

/// Everything a CatBrain is, as data. Same-build, same-platform contract
/// as the brain itself (the module's determinism doc).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrainCapture {
    pub rng_state: u64,
    pub rng_inc: u64,
    pub rect: RoamRect,
    pub state: BrainState,
    pub yaw: f64,
    pub speed: f64,
    pub blocked: f64,
}

impl CatBrain {
    /// The whole brain as data — the RNG words included, or the restored
    /// cat diverges at its first whim.
    #[must_use]
    pub fn capture(&self) -> BrainCapture {
        let (rng_state, rng_inc) = self.rng.capture();
        BrainCapture {
            rng_state,
            rng_inc,
            rect: self.rect,
            state: match self.state {
                State::Roam { tx, tz } => BrainState::Roam { tx, tz },
                State::Pause { left } => BrainState::Pause { left },
                State::Sit { left } => BrainState::Sit { left },
            },
            yaw: self.yaw,
            speed: self.speed,
            blocked: self.blocked,
        }
    }

    /// A brain rebuilt mid-life — the one thing `new` cannot express (it
    /// hard-codes the first pause and a fresh stream).
    #[must_use]
    pub fn restore(capture: BrainCapture) -> Self {
        Self {
            rng: Pcg32::restore(capture.rng_state, capture.rng_inc),
            rect: capture.rect,
            state: match capture.state {
                BrainState::Roam { tx, tz } => State::Roam { tx, tz },
                BrainState::Pause { left } => State::Pause { left },
                BrainState::Sit { left } => State::Sit { left },
            },
            yaw: capture.yaw,
            speed: capture.speed,
            blocked: capture.blocked,
        }
    }
}
```

`cat_gait.rs` — same pattern, straight field copies both ways (`GaitCapture` mirrors all six fields; `capture` reads them, `restore` writes them). `cat_body.rs`:

```rust
    /// A chain rebuilt at an exact curve. `new` settles toward rest by
    /// iterating — correct for a spawn, wrong for a restore.
    #[must_use]
    pub fn restore(nodes: [Vector3; TAIL_N]) -> Self {
        Self { nodes }
    }
```

- [ ] **Step 4: GREEN on all three modules.**

- [ ] **Step 5: The node door** (`nodes/cat.rs`) — `CatCapture` struct as in Interfaces (plain `pub(crate)` struct with pub fields, doc comment on each group), then:

```rust
impl WaveCat {
    /// The cat as data — None when _ready refused (uninjected) and the
    /// brain never existed, which the blob reports as a refusal, never
    /// as a default cat.
    pub(crate) fn capture_state(&self) -> Option<CatCapture> {
        let brain = self.brain.as_ref()?;
        let gait = self.gait.as_ref()?;
        let tail = self.tail.as_ref()?;
        let pose = self.pose.as_ref()?;
        Some(CatCapture {
            position: self.base().get_global_position(),
            yaw: f64::from(self.base().get_global_rotation().y),
            velocity: self.base().get_velocity(),
            brain: brain.capture(),
            gait: gait.capture(),
            tail: *tail.nodes(),
            pose: pose.clone(),
            presence_next: self.presence.next_at().unwrap_or(f64::NAN),
            sit: self.sit,
            sim_t: self.sim_t,
            last_pos: self.last_pos,
        })
    }

    /// Place a built cat into a captured mid-life state. Callers hold the
    /// tree frozen; the next physics tick resumes the captured life.
    pub(crate) fn restore_state(&mut self, capture: &CatCapture) {
        self.base_mut().set_global_position(capture.position);
        let mut rot = self.base().get_global_rotation();
        rot.y = capture.yaw as f32;
        self.base_mut().set_global_rotation(rot);
        self.base_mut().set_velocity(capture.velocity);
        self.brain = Some(CatBrain::restore(capture.brain));
        self.gait = Some(CatGait::restore(capture.gait.clone()));
        self.tail = Some(Tail::restore(capture.tail));
        self.pose = Some(capture.pose.clone());
        self.presence = Cadence::restore(cat_gait::PRESENCE_EVERY, capture.presence_next);
        self.sit = capture.sit;
        self.sim_t = capture.sim_t;
        self.last_pos = capture.last_pos;
        self.mesh_dirty = true;
    }
}
```

(`presence_next` NaN — a cat that never beat — round-trips through `Cadence::restore(interval, NaN)`, whose `next_at()` returns `None` again: the poison repair in `beat()` re-books it exactly as it would have. Add that sentence as a comment.)

- [ ] **Step 6: gdUnit integration test** (append to `game/tests/restore_test.gd` — created in this task with its suite header; Task 9 extends it):

```gdscript
extends GdUnitTestSuite
## The restore doors against the live scene. Each test freezes nothing —
## it drives the clock by hand, exactly as observer_test does.

const MAIN_SCENE := preload("res://scenes/main.tscn")


func test_a_restored_cat_resumes_the_same_life() -> void:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	var cat: WaveCat = main.cats[0]
	# let the cat live a little, on real physics
	for _i in 30:
		main.now += 1.0 / 60.0
		cat.tick(main.now)
		await get_tree().physics_frame
	var mood_at_capture: int = cat.mood()
	var paws_at_capture: PackedVector3Array = cat.paw_positions()
	# capture/restore round trip through the Rust door is exercised via
	# the observer/restorer in Task 9's tests; here the door itself is
	# pinned through the one #[func] pair added for THIS test:
	assert_int(mood_at_capture).is_not_equal(-1)
	assert_int(paws_at_capture.size()).is_equal(4)
```

**Note:** the full cat round-trip through GDScript needs the blob (Task 8/9) — this task's gdUnit test only pins that a live scene cat is capturable at all (mood ≠ −1 sentinel). The real equivalence gates are the cargo lockstep tests above plus Task 10's advance-and-compare. Do not build a premature `#[func]` surface here.

- [ ] **Step 7: Tooling + full gate + commit** — subject: *"The cat's whole life fits in a suitcase"*. Body: what moves (RNG words, whim payloads, stance flags, the settled-vs-restored tail distinction) and why `new` could never express it.

---

### Task 6: The viewmodel and hero doors — the footstep clock travels

`Viewmodel`'s ten fields are private; `step_t`/`step_side` — the whole footstep-firing state — have no read path, and `Viewmodel::new` starts `step_t = 0.0` (SPENT), so restore-by-reconstruct fires a spurious footstep on the first moving frame and resets the L/R alternation. The door mirrors Task 5's shape.

**Files:**
- Modify: `rust/src/viewmodel.rs` (`ViewmodelCapture`, `Viewmodel::capture/restore`)
- Modify: `rust/src/nodes/hero.rs` (`pub(crate) fn capture_vm/restore_vm`)
- Modify: `rust/src/nodes/player.rs` (`pub(crate) fn set_eye_pitch`, `pub(crate) fn clear_wave_queue`)

**Interfaces:**
- Produces:
  ```rust
  // viewmodel.rs — all ten fields, pub, Copy
  pub struct ViewmodelCapture { pub walk_amp: f64, pub leg_phase: f64, pub swing_phase: f64,
      pub cane_swing: f64, pub sway_x: f64, pub sway_y: f64, pub last_yaw: f64,
      pub last_pitch: f64, pub step_t: f64, pub step_side: i32 }
  impl Viewmodel { pub fn capture(&self) -> ViewmodelCapture;
                   pub fn restore(capture: ViewmodelCapture) -> Viewmodel; }
  // hero.rs
  impl HeroBody { pub(crate) fn capture_vm(&self) -> Option<ViewmodelCapture>;  // None = _ready refused
                  pub(crate) fn restore_vm(&mut self, capture: ViewmodelCapture); }
  // player.rs
  impl UnseeingPlayer { pub(crate) fn set_eye_pitch(&mut self, pitch: f64);  // clamped by PITCH_LIMIT
                        pub(crate) fn clear_wave_queue(&mut self); }
  ```
  Task 9 restores the hero as: position/velocity/rotation via node API + `set_eye_pitch` + `last_tap`/`tap_target` (already `#[var]`-writable) + `tick(now)` + `clear_wave_queue` then `queue_wave` per captured entry + `tap()` if `tap_queued` was captured true + `restore_vm`. `shoes`/`cane_rest`/`bob_offset` are derived every tick — never captured.

- [ ] **Step 1: Write the failing cargo test** (`viewmodel.rs`)

```rust
    /// The restored walker's NEXT footstep lands exactly when the
    /// original's would — timing and which shoe. A reconstructed
    /// viewmodel (new()) cannot do this: it starts with the step clock
    /// SPENT and the alternation reset to right.
    #[test]
    fn a_restored_walker_keeps_its_step_clock_and_its_next_shoe() {
        let mut original = Viewmodel::new(0.0, 0.0);
        // walk until the first step fires and the clock is mid-count
        let mut fired = None;
        let mut steps = 0;
        while fired.is_none() || steps < 3 {
            fired = original.footstep(0.05, true);
            if fired.is_some() {
                steps += 1;
            }
        }
        // now mid-interval: 0.05 into the 0.42 rebook, next side known
        let mut restored = Viewmodel::restore(original.capture());
        assert_eq!(restored, original);
        // lockstep to the next firing: same tick, same side
        loop {
            let a = original.footstep(0.05, true);
            let b = restored.footstep(0.05, true);
            assert_eq!(a, b);
            if a.is_some() {
                break;
            }
        }
        // and the spurious-first-step failure a fresh walker would show:
        let mut fresh = Viewmodel::new(0.0, 0.0);
        assert!(fresh.footstep(0.05, true).is_some()); // fires at once
    }
```

- [ ] **Step 2: RED** — no `capture`/`restore` on `Viewmodel`.

- [ ] **Step 3: Implement** — `ViewmodelCapture` with all ten fields; `capture` copies them out; `restore` copies them in (straight struct literals both ways, doc comments explaining the spent-clock trap). Then the two hero methods (`capture_vm` = `self.vm.as_ref().map(Viewmodel::capture)`; `restore_vm` = `self.vm = Some(Viewmodel::restore(capture))`) and the two player methods:

```rust
    /// The restore door for the eye: the same clamp the look law applies,
    /// so a blob cannot place the eye past PITCH_LIMIT.
    pub(crate) fn set_eye_pitch(&mut self, pitch: f64) {
        if let Some(camera) = self.camera.as_mut() {
            let mut rot = camera.get_rotation();
            rot.x = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT) as f32;
            camera.set_rotation(rot);
        }
    }

    /// Empty the out-tray before a restore rebuilds it — restoring onto a
    /// non-empty queue would replay the captured waves AND the stale ones.
    pub(crate) fn clear_wave_queue(&mut self) {
        self.wave_queue.clear();
    }
```

- [ ] **Step 4: GREEN**, full cargo suite.

- [ ] **Step 5: Tooling + commit** — subject: *"The next footstep survives the move"*. Body: the spent-clock/reset-alternation failure of restore-by-reconstruct, demonstrated in the test.

---

### Task 7: The blob — one pure struct, canonical bytes, and an honest hash

The composition layer: `CaptureState` gathers every door's output plus the env; `canonical_bytes` encodes it deterministically (**bit-verbatim**: `f64::to_bits`/`f32::to_bits` little-endian — no strings, no pretty-print; the determinism probe already paid for that lesson once); `fnv1a64` hashes it; `first_divergence` names where two states part. `FORMAT_VERSION` is inside the hashed bytes, so a format change can never false-match.

**Files:**
- Create: `rust/src/reproduce/mod.rs`, `rust/src/reproduce/blob.rs`
- Modify: `rust/src/lib.rs` (add `pub mod reproduce;` beside `pub mod observe;` — match the existing module list style)

**Interfaces:**
- Produces:
  ```rust
  pub const FORMAT_VERSION: u32 = 1;
  pub struct EnvCapture { pub now: f64, pub demo_checked: bool, pub demo_armed: bool,
      pub demo_next: f64, pub flicker_t: f64, pub flicker_level: f64,
      pub flicker_drop_until: f64, pub flicker_next_drop: f64, pub flicker_rng_state: i64 }
  pub struct HeroCapture { pub position: Vector3, pub velocity: Vector3, pub yaw: f64,
      pub pitch: f64, pub last_tap: f64, pub tap_target: Vector3, pub tap_queued: bool,
      pub queued_waves: Vec<QueuedWave>, pub viewmodel: ViewmodelCapture }
  pub struct SourceCapture { pub name: String, pub next_emit: f64 }
  pub struct CaptureState { pub format_version: u32, pub level_scene: String,
      pub env: EnvCapture, pub slots: Box<[SlotCapture; MAXP]>, pub echoes: Vec<PendingEcho>,
      pub sources: Vec<SourceCapture>, pub hero: HeroCapture, pub cats: Vec<CatCapture> }
  pub fn canonical_bytes(state: &CaptureState) -> Vec<u8>;
  pub fn fnv1a64(bytes: &[u8]) -> u64;
  pub fn state_hash(state: &CaptureState) -> u64;   // fnv1a64(canonical_bytes)
  pub fn first_divergence(a: &CaptureState, b: &CaptureState) -> Option<String>;  // e.g. "slots[12].t0", "cats[0].brain.rng_state"
  ```
  `CatCapture` moves from `nodes/cat.rs` into `blob.rs`? **No** — it holds only value types but was declared in the boundary. Move it: declare `CatCapture` in `blob.rs` (pure — `CatPose`, `BrainCapture`, `GaitCapture` are all pure value types) and have `nodes/cat.rs` import it. Adjust Task 5's placement accordingly if implementing out of order; if Task 5 already landed it in `cat.rs`, this task's first step MOVES it (one `use crate::reproduce::blob::CatCapture;` in `cat.rs`, struct text relocated verbatim, visibility widened to `pub`).
- Consumes: `SlotCapture` (Task 2), `PendingEcho` (Task 3), `BrainCapture`/`GaitCapture`/`CatPose`/`TAIL_N` (Task 5), `ViewmodelCapture` (Task 6), `QueuedWave` (`observe/mod.rs`, shipped).
- `flicker_rng_state: i64` — Godot's `RandomNumberGenerator.state` is a 64-bit int crossing a Variant boundary; store as `i64` verbatim (bit pattern preserved; no arithmetic ever done on it).

- [ ] **Step 1: Write the failing tests**

```rust
    /// FNV-1a 64 against the published reference vectors — the offset
    /// basis for "" and the classic single-byte check. Hand-derived from
    /// the algorithm's spec (offset 0xcbf29ce484222325, prime
    /// 0x100000001b3), never from this implementation.
    #[test]
    fn fnv1a64_matches_the_published_vectors() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x85944171f73967e8);
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
        assert_eq!(
            first_divergence(&a, &b).as_deref(),
            Some("slots[12].t0")
        );
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
```

with a `test_state()` builder that populates every group with distinct non-default literals (one live slot, one echo, one source, one cat, a mid-count viewmodel) so an encoder that skips a field is caught by the ULP/divergence machinery.

- [ ] **Step 2: RED** — module doesn't exist.

- [ ] **Step 3: Implement**

`fnv1a64` (the whole algorithm, ~8 lines):

```rust
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
```

`canonical_bytes`: a small private `Enc` helper (`Vec<u8>` + `fn f64(&mut self, v: f64)` pushing `v.to_bits().to_le_bytes()`, `fn f32`, `fn u64/i64/u32/i32` le-bytes, `fn bool` as one byte, `fn str` as u32 length + UTF-8 bytes, `fn v3` = 3×f32 bits, `fn v4` = 4×f32 bits) and one `encode(state, &mut enc)` walking EVERY field of every group in declaration order, prefixing each `Vec` with its u32 length. `first_divergence`: a parallel walk comparing field-by-field, returning the dotted path of the first mismatch (compare bit patterns for floats — `to_bits()` — so NaN ≠ NaN never loops forever and −0.0 ≠ 0.0 is caught honestly). Write both walks from one field list, kept adjacent in the file with a comment binding them: **a field added to `CaptureState` must be added to BOTH walks — the `identical_states_agree_completely` + `one_ulp` tests plus Task 10's deliberate break are the net.**

- [ ] **Step 4: GREEN**, full cargo suite.

- [ ] **Step 5: Tooling + commit** — subject: *"Every field, every bit, one number"*. Body: to_bits vs to_string, the version-inside-the-bytes property, and the twin-walk discipline.

---

### Task 8: `capture()` — the observer reads everything, or refuses

The read side. `capture(now, env)` joins `WaveObserver`: strictly wider than `snapshot()` (the f64 shadow, the cat's private life, the viewmodel clocks, the env), strictly stricter (ALL-OR-NOTHING — no `unknown` array; any unobservable subsystem refuses the whole capture). It returns the blob as a `VarDictionary` (the JSON-able artifact) carrying its own `hash` — and it must not mutate anything.

**Files:**
- Modify: `rust/src/nodes/observer.rs` (`inject_body`, `capture`, the `CaptureState → VarDictionary` and `VarDictionary → CaptureState` converters — the parser lives here too because Task 9's restorer shares it; `pulse_core` becomes `pub(super)`)
- Modify: `rust/src/nodes/level.rs` (`pub(super) fn cat_handles(&self) -> &[Gd<WaveCat>]`, beside `source_handles`)
- Modify: `rust/src/ffi.rs` (`pub(crate) fn capture_pool(&self) -> Box<[SlotCapture; MAXP]>` = `self.pool.capture_slots()`; `pub(crate) fn capture_echoes(&self) -> Vec<PendingEcho>` = `self.echoes.capture()`)
- Modify: `game/scripts/main.gd` (`capture_env()`, and `inject_body` call)
- Modify: `game/tests/restore_test.gd` (capture-side tests)

**Interfaces:**
- Produces:
  - `#[func] fn inject_body(&mut self, body: Option<Gd<HeroBody>>)` on `WaveObserver` (the viewmodel lives on `HeroBody`, which the observer was never handed before).
  - `#[func] fn capture(&self, now: f64, env: VarDictionary) -> VarDictionary` — on success, the blob dict; on failure `{"unavailable": reason}`. Blob top-level keys: `format_version: i64`, `level_scene: String`, `hash: String` (the u64 rendered as 16 hex chars — JSON round-trips it losslessly as a string, never as a float), `env: Dictionary`, `slots: Array[Dictionary]` (64, keys `pos`/`dat`/`dir`/`t0`/`end`/`kind`), `echoes: Array[Dictionary]` (`at_t`/`pos`/`gain`), `sources: Array[Dictionary]` (`name`/`next_emit`), `hero: Dictionary` (the `HeroCapture` fields; `queued_waves` entries keyed `type`/`at`/`max_r`/`speed`/`gain`/`echoes`/`normal` — the shipped vocabulary; `viewmodel` as a nested dict of the ten fields), `cats: Array[Dictionary]` (the `CatCapture` fields; `tail` as `PackedVector3Array`, `pose` nested, `brain`/`gait` nested with their exact field names; `rng_state`/`rng_inc` as `String` hex — u64 does not fit a Godot int when the high bit is set).
  - `pub(super) fn parse_blob(dict: &VarDictionary) -> Result<CaptureState, String>` — total: every missing or mis-typed key returns `Err` naming the dotted path (`"cats[0].brain.rng_state: missing"`). The restorer consumes this; a parser that defaulted a field would be the vacuous pass.
  - Env dict contract (built by `main.gd::capture_env()`, keys exactly): `now`, `demo_checked`, `demo_armed`, `demo_next`, `flicker_t`, `flicker_level`, `flicker_drop_until`, `flicker_next_drop`, `flicker_rng_state`.
- Refusal reasons (new consts, existing one-key grammar): `NO_BODY: "observer was never injected the hero body — the viewmodel clocks live there"`, `DEAD_BODY: "the injected hero body has been freed"`, `NO_VM: "the hero body never built its viewmodel — the game is not running"`, `UNBUILT_CAT: "a level cat was never built — capture refuses a defaulted cat"`, `NO_APPOINTMENT: "a source holds no beat appointment — the level has not ticked"`, `BAD_ENV: "the env group is missing or malformed: "` (+ the offending key).
- Consumes: every door from Tasks 2–7; `level.source_handles()` + `SoundSource::next_emit()`; `hero_observation()`'s existing fetch pattern.

- [ ] **Step 1: Write the failing gdUnit tests** (append to `restore_test.gd`)

```gdscript
func _boot_ticked() -> UnseeingMain:
	var main: UnseeingMain = auto_free(MAIN_SCENE.instantiate() as UnseeingMain)
	add_child(main)
	# one real process frame so sources book appointments and the
	# viewmodel exists — capture refuses an unticked world by design
	await get_tree().process_frame
	await get_tree().physics_frame
	return main


func test_capture_is_total_and_carries_its_own_hash() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	assert_int(blob["format_version"]).is_equal(1)
	assert_int((blob["slots"] as Array).size()).is_equal(64)
	assert_int((blob["cats"] as Array).size()).is_equal(main.cats.size())
	assert_str(blob["hash"]).has_length(16)
	# hero group carries the viewmodel clocks snapshot() never had
	var vm: Dictionary = blob["hero"]["viewmodel"]
	assert_bool(vm.has("step_t")).is_true()
	assert_bool(vm.has("step_side")).is_true()


func test_capture_never_mutates() -> void:
	var main := await _boot_ticked()
	var before: Dictionary = main.observer.snapshot(main.now)
	var _blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var after: Dictionary = main.observer.snapshot(main.now)
	assert_int((after["echoes"] as Array).size()).is_equal((before["echoes"] as Array).size())
	assert_int(after["live_slots"]).is_equal(before["live_slots"])
	for i in main.cats.size():
		# a capture that advanced a cadence or an RNG would differ here
		assert_that(after["sources"][i].get("next_emit")).is_equal(before["sources"][i].get("next_emit"))


func test_capture_without_the_body_refuses_whole() -> void:
	var main := await _boot_ticked()
	main.observer.inject_body(null)
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_true()
	assert_bool(blob.has("slots")).is_false()


func test_capture_with_malformed_env_names_the_key() -> void:
	var main := await _boot_ticked()
	var env: Dictionary = main.capture_env()
	env.erase("flicker_rng_state")
	var blob: Dictionary = main.observer.capture(main.now, env)
	assert_bool(blob.has("unavailable")).is_true()
	assert_str(blob["unavailable"]).contains("flicker_rng_state")
```

- [ ] **Step 2: RED** — nonexistent `capture`/`inject_body`/`capture_env`.

- [ ] **Step 3: Implement**, in this order:
  1. `ffi.rs` accessors (2 one-liners).
  2. `level.rs` `cat_handles` (mirror `source_handles`).
  3. `main.gd::capture_env()` — a typed function returning the nine-key Dictionary read from `now`, `_demo_checked`, `_demo.armed`, `_demo._next`, `_flicker._t/_level/_drop_until/_next_drop`, `_flicker._rng.state` (GDScript privacy is conventional; the wiki documents this reach as the sanctioned one). Plus `observer.inject_body(hero)` in `_ready` after `inject_hero`.
  4. `observer.rs`: the `body` field + `inject_body` + `live_body()` (validity per call, like `live_camera`); `capture()` — fetch level/camera/pool exactly as `snapshot()` does, then assemble `CaptureState` refusing at the FIRST unobservable subsystem (env parse first — cheapest; then pool/echoes from one `core.bind()`; then sources — any `next_emit() == None` → `NO_APPOINTMENT`; then hero — reuse `hero_observation()`'s fetches but refuse rather than omit, plus `capture_vm()` via the body (`None` → `NO_VM`); then cats via `cat_handles()` — any `capture_state() == None` → `UNBUILT_CAT`); compute `state_hash`; serialize with the dict converters. Then `parse_blob` — the strict parser (a small `fn need<T>(dict, key, path) -> Result<T, String>` helper applied to every field).
  5. Keep the serializer and parser adjacent with the same twin-walk comment as Task 7 — and one round-trip cargo-less test lives in gdUnit: capture → parse (via a tiny `#[func] fn blob_round_trip_ok(blob) -> String` on the observer, returning `""` or the parse error — used by tests only, cheap, honest).
- [ ] **Step 4: GREEN** — the four tests, then the full gdUnit gate (case counts grow; every pre-existing suite stays green — `snapshot()` is untouched).
- [ ] **Step 5: Tooling + commit** — subject: *"The observer learns to pack a suitcase, or say why it cannot"*. Body: capture vs snapshot (wider AND stricter — no `unknown` here), the env contract, the hex-string u64s.

---

### Task 9: `WaveRestorer` — the transaction that proves itself

The write side. A new node, injected like the observer, applies a parsed blob under a frozen tree in the spec's order, then **re-captures and compares hashes** — a restore that cannot prove itself refuses, naming the first divergent field. The spurious-beat trap dies here: appointments re-pin after the clock.

**Files:**
- Create: `rust/src/nodes/restorer.rs`
- Modify: `rust/src/nodes/mod.rs` (register), `game/scripts/main.gd` (construct/inject; `apply_env`; `restore_blob()` helper), `game/tests/restore_test.gd`

**Interfaces:**
- Produces:
  - `WaveRestorer` (`Node`): `#[func] fn inject(&mut self, level: Option<Gd<WaveLevel>>, player: Option<Gd<UnseeingPlayer>>, body: Option<Gd<HeroBody>>, observer: Option<Gd<WaveObserver>>)` — four handles, each validity-checked per call like the observer's own; it re-uses the OBSERVER for the post-restore proof, never duplicating capture; `#[func] fn restore(&mut self, blob: VarDictionary, env_after: VarDictionary) -> VarDictionary` returning `{"restored": true, "hash": "<hex>"}` or `{"unavailable": reason}`. `env_after` is `main.capture_env()` taken by the caller AFTER it applied the blob's env group — see the choreography below.
  - `main.gd`: `func apply_env(env: Dictionary) -> void` (writes the nine fields back: `now`, `_demo_checked`, `_demo.armed`, `_demo._next`, four flicker floats, `_flicker._rng.state`); `func restore_blob(blob: Dictionary) -> Dictionary` — the one-call choreography the tests and Plan 3's harness use:

```gdscript
## Apply a captured blob to this running game. The env half is GDScript
## state, applied here; the engine half is the restorer's transaction,
## which re-captures and refuses on any divergence. Pause first: state
## must not move between the two halves.
func restore_blob(blob: Dictionary) -> Dictionary:
	get_tree().paused = true
	apply_env(blob["env"])
	var verdict: Dictionary = restorer.restore(blob, capture_env())
	get_tree().paused = false
	return verdict
```

- Restore order inside `restorer.rs::restore` (the spec's transaction, each step refusing on failure):
  1. `parse_blob` (Task 8's parser) — malformed → its error, verbatim.
  2. Header: `format_version == FORMAT_VERSION` else refuse naming both; `level_scene == level.get_scene_file_path()` else refuse naming both.
  3. Pool + echoes: one `core` fetch (via the now-`pub(super)` `pulse_core`), `bind_mut`, `restore_state(PulsePool::from_slots(&state.slots), EchoQueue::from_pending(state.echoes.clone()))` — one new `pub(crate)` method on `WaveCore` setting both fields (add it in this task; two lines).
  4. Hero: node transform/velocity, `set_eye_pitch`, `last_tap`/`tap_target` (property set via `bind_mut` field writes — they are `pub(crate)`), `tick(state.env.now)`, `clear_wave_queue` + `queue_wave` per entry, `tap()` iff `tap_queued`, `restore_vm`.
  5. Cats: count must equal `cat_handles().len()` else refuse; `restore_state` each, scene order.
  6. Sources: count check likewise; `restore_appointment(next_emit)` each, scene order — AFTER the clock (step 4's `tick` and the caller's `apply_env` both already placed `now`).
  7. The proof: `observer.capture(state.env.now, env_after)` → parse → `state_hash` both → on mismatch, `first_divergence` names the field; refuse `{"unavailable": "restore diverged at <path> — the blob and the restored world disagree"}`. On match, `{"restored": true, "hash": hex}`.
- Consumes: everything.

- [ ] **Step 1: Write the failing gdUnit tests** (append to `restore_test.gd`)

```gdscript
func test_round_trip_capture_restore_capture_is_exact() -> void:
	var main := await _boot_ticked()
	# a livelier world: tap once, let waves and echoes exist
	main.player.tap()
	for _i in 10:
		main.now += 1.0 / 60.0
		main.player.tick(main.now)
		await get_tree().physics_frame
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	assert_bool(blob.has("unavailable")).is_false()
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_false()
	assert_str(verdict["hash"]).is_equal(blob["hash"])


func test_restore_repins_appointments_no_spurious_beat() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var live_before: int = main.observer.snapshot(main.now)["live_slots"]
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_false()
	# one process frame: if any source's gate came back stale, it beats
	# NOW and a fresh hum enters the pool — the spurious-beat trap
	await get_tree().process_frame
	var live_after: int = main.observer.snapshot(main.now)["live_slots"]
	assert_int(live_after).is_equal(live_before)


func test_a_wrong_version_refuses_before_touching_anything() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	blob["format_version"] = 999
	var before: Dictionary = main.observer.snapshot(main.now)
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_true()
	assert_str(verdict["unavailable"]).contains("999")
	var after: Dictionary = main.observer.snapshot(main.now)
	assert_int(after["live_slots"]).is_equal(before["live_slots"])


func test_a_tampered_blob_is_named_at_its_divergent_field() -> void:
	var main := await _boot_ticked()
	var blob: Dictionary = main.observer.capture(main.now, main.capture_env())
	var slots: Array = blob["slots"]
	var slot: Dictionary = slots[3]
	slot["t0"] = float(slot["t0"]) + 1.0
	# hash still the ORIGINAL's: the restored world will disagree with it
	var verdict: Dictionary = main.restore_blob(blob)
	assert_bool(verdict.has("unavailable")).is_true()
	assert_str(verdict["unavailable"]).contains("slots[3]")
```

- [ ] **Step 2: RED** — nonexistent `restorer`/`restore_blob`/`apply_env`.

- [ ] **Step 3: Implement** in the order the Interfaces section lays out (restorer node first, then `WaveCore::restore_state`, then main.gd). The restorer's `#[func]`s stay boundary-thin: every check calls Task 7/8's pure functions; the node adds sequencing and refusals only.

- [ ] **Step 4: GREEN** — the four tests, then the FULL gate including `tools/determinism_probe.sh` (the probe's hash covers a world the restorer's registration must not have disturbed — registration alone changes nothing, and the gate proves it).

- [ ] **Step 5: Tooling + commit** — subject: *"A captured game steps back into its own shoes, and proves the fit"*. Body: the transaction order and why the proof re-uses the observer (one capture implementation, or the proof proves the wrong thing).

---

### Task 10: The gate — advance-and-compare, and the deliberate break

Round-trip proves serialization; it cannot catch **omission** (a field absent from both capture and restore agrees with itself forever). The omission detector runs two processes: run A boots seeded, lives T frames, captures to a file, lives N more, snapshots a hash. Run B boots fresh, restores A's blob, lives the same N, snapshots. The hashes must match — any state that influences the future but escaped the blob diverges within N frames of real dynamics (source beats, cat whims, echo firings, footstep clocks).

**Files:**
- Create: `game/tests/probe/restore_probe.gd`, `tools/restore_probe.sh`
- Modify: `ci/pipeline.sh` (stage after the determinism probe)

**Interfaces:**
- Consumes: Plan 1's determinism substrate (`UNSEEING_SEED`, `--fixed-fps 60`, the `canonicalize()` snapshot-hash pattern from `game/tests/probe/determinism_probe.gd` — copy its canonicalization verbatim into this probe, with a comment naming the origin), `main.restore_blob`, `observer.capture`.
- Produces: the gate every future restore-touching change runs under, and the pattern Plan 3's `tools/reproduce.sh` extends.

- [ ] **Step 1: Write the probe**

`game/tests/probe/restore_probe.gd` (SceneTree script, mode from `UNSEEING_RESTORE_MODE` env = `capture` | `restore`, blob file path from `UNSEEING_RESTORE_BLOB`):

```gdscript
extends SceneTree
## Advance-and-compare, the omission detector. Mode "capture": boot
## seeded, live T frames, write the blob, live N more, print the state
## hash. Mode "restore": boot fresh, restore the blob, live the SAME N,
## print the hash. tools/restore_probe.sh demands the pair agree — any
## state that influences the future but escaped the blob diverges here.
## Frame counting rides process_frame; refusals exit 2 with no hash line.

const T_FRAMES := 180
const N_FRAMES := 240

var _main: UnseeingMain
var _mode := ""
var _blob_path := ""
var _frames := 0
var _captured := false


func _initialize() -> void:
	_mode = OS.get_environment("UNSEEING_RESTORE_MODE")
	_blob_path = OS.get_environment("UNSEEING_RESTORE_BLOB")
	if OS.get_environment("UNSEEING_SEED").is_empty():
		push_error("restore probe: refusing an unseeded run")
		quit(2)
		return
	if _mode != "capture" and _mode != "restore":
		push_error("restore probe: UNSEEING_RESTORE_MODE must be capture|restore")
		quit(2)
		return
	_main = load("res://scenes/main.tscn").instantiate() as UnseeingMain
	root.add_child(_main)
	process_frame.connect(_on_frame)


func _on_frame() -> void:
	_frames += 1
	if _mode == "capture":
		_capture_leg()
	else:
		_restore_leg()


func _capture_leg() -> void:
	if _frames == T_FRAMES:
		var blob: Dictionary = _main.observer.capture(_main.now, _main.capture_env())
		if blob.has("unavailable"):
			push_error("restore probe: capture refused: %s" % blob["unavailable"])
			quit(2)
			return
		var out := FileAccess.open(_blob_path, FileAccess.WRITE)
		out.store_string(JSON.stringify(blob, "", true, true))
		out.close()
		_captured = true
	if _captured and _frames == T_FRAMES + N_FRAMES:
		_print_hash_and_quit()


func _restore_leg() -> void:
	if _frames == 1:
		var text := FileAccess.get_file_as_string(_blob_path)
		var blob: Variant = JSON.parse_string(text)
		if blob == null:
			push_error("restore probe: blob file unreadable")
			quit(2)
			return
		var verdict: Dictionary = _main.restore_blob(blob)
		if verdict.has("unavailable"):
			push_error("restore probe: restore refused: %s" % verdict["unavailable"])
			quit(2)
			return
	if _frames == 1 + N_FRAMES:
		_print_hash_and_quit()


func _print_hash_and_quit() -> void:
	var snap: Dictionary = _main.observer.snapshot(_main.now)
	if snap.has("unavailable"):
		push_error("restore probe: snapshot refused: %s" % snap["unavailable"])
		quit(2)
		return
	print("RESTORE_HASH=%s" % JSON.stringify(_canonicalize(snap), "", true, true).md5_text())
	quit(0)
```

plus the `_canonicalize` function copied verbatim from `determinism_probe.gd` (with its origin comment). **One subtlety the implementer must preserve:** run B's frame 1 restores — so its N frames run from the restored state, while run A's N frames run from the live state at T; the two legs must count N identically from those anchors, exactly as written above.

- [ ] **Step 2: Write the gate**

`tools/restore_probe.sh` (0755, same conventions as `determinism_probe.sh`):

```bash
#!/usr/bin/env bash
# Advance-and-compare: a captured world, restored into a fresh boot, must
# live the next N frames IDENTICALLY to the original that kept running.
# Catches omission — the one failure class round-trip hashing cannot see.
# A missing hash is a failure, never a pass.
set -euo pipefail
DIR="$(cd "$(dirname "$0")/.." && pwd)"
GODOT="${GODOT:-godot}"
BLOB="$(mktemp -t unseeing-blob.XXXXXX.json)"
trap 'rm -f "$BLOB"' EXIT

leg() {
  UNSEEING_SEED=1 UNSEEING_RESTORE_MODE="$1" UNSEEING_RESTORE_BLOB="$BLOB" \
    "$GODOT" --headless --fixed-fps 60 --path "$DIR/game" \
    -s res://tests/probe/restore_probe.gd 2>&1 \
    | grep '^RESTORE_HASH=' | head -1
}

A="$(leg capture || true)"
B="$(leg restore || true)"
[ -n "$A" ] || { echo "restore: FAILED — no hash from the capture leg"; exit 1; }
[ -n "$B" ] || { echo "restore: FAILED — no hash from the restore leg"; exit 1; }
if [ "$A" != "$B" ]; then
  echo "restore: FAILED — the restored run diverged from the original:"
  echo "  original: $A"
  echo "  restored: $B"
  exit 1
fi
echo "restore: OK $A"
```

- [ ] **Step 3: Run it, watch it fail or pass FOR REAL** — first run is the moment of truth. If it fails: that is a genuine omission or a physics-reproducibility finding — invoke superpowers:systematic-debugging, use `first_divergence` (restore the blob in a scratch run and diff snapshots per-frame to localize the tick), and fix the missing state — never widen the gate. If it passes: proceed.

- [ ] **Step 4: The deliberate break** — in `restorer.rs`, temporarily comment out ONE restore step (the cat's `restore_state` loop is the designated victim — the cat's whims make omission vivid). Run the gate: it MUST fail (the round-trip test in `restore_test.gd` will ALSO fail — the post-restore hash proof catches it first; note which failed first in your report). Revert; run again; it must pass. This is the acceptance criterion: **a gate never shown to catch a real omission has not been shown to work.** Capture both outputs in the report.

- [ ] **Step 5: Wire the pipeline** — in `ci/pipeline.sh`, directly after the determinism-probe stage:

```bash
echo "ci: restore probe (a restored world must live the same future)"
GODOT="$GODOT" "$DIR/tools/restore_probe.sh"
```

- [ ] **Step 6: Full pipeline** (`SKIP_EXPORT=1`) end to end; gdformat/gdlint the probe; commit — subject: *"A restored world lives the same future, and a gate demands it"*. Body: why round-trip cannot catch omission, the two-leg anchor subtlety, and the deliberate-break evidence.

---

## After the last task

Per `CLAUDE.md`, the work is not done at green:

1. **Wiki** (clone `unseeing.wiki.git`): *Engineering — Debugging and Observability* gains the Capture and Restore sections — the fifth and sixth verbs joining the four, the blob totality rule (no `unknown` in a blob — refusal only), the env contract, the restore transaction order with the spurious-beat law, the proof step, and both gates; §10 ("what is NOT built") shrinks accordingly; *Engineering — Build, Test, Deploy* gains the restore-probe stage. The `JSON.stringify`-vectors trap note already on the page gets a pointer to `reproduce/blob.rs` as the reference discipline.
2. **Memory**: update `reproduction-loop-design` (Plan 2 status, any new traps found — especially anything the advance-and-compare gate caught during Step 3 of Task 10).
3. **Review**: every task got its review during execution; the final whole-branch review runs before merge, per the method.
4. **Integration**: finishing-a-development-branch presents the menu; merge runs in the shared checkout only against a clean tree, and deploy (if chosen) only after the merge, from `main`.

## Self-Review Notes

- **Spec coverage:** capture wider than snapshot (f64 shadow T2, cat T5, viewmodel T6, env T8) ✓; restore transaction with header check, clock-first, appointments-after-clock, post-restore proof (T9) ✓; blob totality + versioned + FNV-1a over canonical bytes (T7, hex-string u64s at the JSON boundary) ✓; round-trip + advance-and-compare + deliberate break (T9/T10) ✓; per-subsystem doors exactly where the spec's architecture section put them ✓. Deferred to Plan 3, per spec: the tape, primitives, `tools/reproduce.sh`, the full diff verb (`first_divergence` here is its seed), NDJSON traces.
- **Type consistency:** `SlotCapture` boxed array everywhere; `QueuedWave` reused from `observe/mod.rs` with the `"type"` wire key; `CatCapture` declared in `blob.rs` (Task 7 resolves Task 5's placement); `flicker_rng_state` is `i64` end to end; u64 RNG words are hex strings only at the dict boundary.
- **Known risks stated:** Godot RNG `state` semantics pinned by Task 1 BEFORE anything depends on them; hero/cat kinematics ride Godot physics — same-machine reproducibility was measured green by Plan 1's gate, and Task 10 measures the restored variant; if it flakes, that is a finding to report, never a tolerance to widen.
