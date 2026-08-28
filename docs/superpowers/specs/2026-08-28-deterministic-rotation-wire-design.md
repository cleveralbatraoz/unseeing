# Deterministic rotation wire law — design

**Date:** 2026-08-28
**Status:** Approved by the user (both decisions below chosen explicitly)
**Fixes:** CI failure on PR #81 (3 cargo tests red on Linux); resolves issue #82
**Supersedes:** the `GodotRotation` trig-roundtrip canonicalization introduced by
the character-elevation campaign (#64/#74)

## Problem

`GodotRotation` (`rust/src/support_motion.rs`) defines "canonical rotation" as
the bit-exact fixed point of `Basis::from_euler(YXZ, v).get_euler_with(YXZ)`,
iterated up to 16 times and compared by `to_bits()`. Both halves of that
roundtrip reach the platform's libm through Rust `std` float trig, which the
Rust documentation explicitly declares **non-deterministic**: "The precision of
this function is non-deterministic. This means it varies by platform, Rust
version, and can even differ within the same execution from one invocation to
the next" (`f32::sin` et al.). Apple's libm and glibc converge to fixed points
one ULP apart (`0x3E000000` vs `0x3DFFFFFF` for the same input).

Consequences, from most to least visible:

1. **CI red.** Three cargo tests pass on macOS and fail on Linux
   (`support_motion.rs:1597` bit mismatch, `support_motion.rs:1650` and
   `nodes/cat.rs:2759` `InconsistentState`), because which inputs are fixed
   points at all is platform-local. One test already derived its bits at
   runtime specifically to dodge this and still failed — no test-side
   discipline can compensate for a platform-local law.
2. **Cross-platform restore refusal (issue #82).** Restore validation
   (`try_replacing_yaw`/`try_replacing_pitch`, and the cross-owner guard in
   `prepare_cat_snapshot_links` at `cat.rs:1810-1821`) recomputes the canonical
   image **on the reading platform** and demands bit equality with what the
   writing platform stored. A macOS-written blob refuses on Linux/wasm/Windows.
3. **A hidden same-platform coupling.** The cat installs and reads rotation
   through global-rotation APIs. Godot never caches global rotation — every
   `get_global_rotation()` re-derives euler from the basis via engine C++ trig
   — so the restore-verify postcondition (`install → re-capture → hash-compare`)
   only holds because the Rust fixed-point iteration happens to solve the
   engine's own roundtrip equation *when Rust std and the engine link the same
   libm*. That is a coincidence of platform, not a law.

## Evidence (verified against primary sources)

- **Godot 4.7.1 `Node3D` stores euler verbatim.** `set_rotation` writes
  `data.euler_rotation = p_euler_rad` with no trig and replaces the dirty mask
  with `DIRTY_LOCAL_TRANSFORM`; `get_rotation` returns those bits verbatim
  unless `DIRTY_EULER_ROTATION_AND_SCALE` is set (`scene/3d/node_3d.cpp`).
- **What dirties euler:** `set_transform`/`set_basis`/`set_global_transform`/
  `set_global_rotation`/`look_at`. `CharacterBody3D::move_and_slide` mutates
  only `gt.origin` but still calls `set_global_transform`, which
  unconditionally marks euler dirty — so the first `get_rotation()` after any
  physics step re-derives euler via `orthonormalized()` + `asin`/`atan2`
  (engine C++ libm). `get_global_rotation()` is never cached and always
  re-derives.
- **This repo writes orientation to hero body, eye, and cat body exclusively
  through euler APIs** (`set_rotation`, `set_global_rotation`, `rotate_y`).
  There is no `look_at` or `set_basis` anywhere in `rust/src`. The only
  `set_global_transform` calls on these nodes are same-tick rollback
  round-trips of a transform read moments earlier.
- **The wire is not the problem.** Blob floats cross as Rust-written decimal
  text (`Floats::Text`), which round-trips f64 losslessly. The platform-local
  bits are manufactured *before* serialization, by `canonicalize` itself.
- **gdext 0.5.4's Basis math is a pure-Rust port of the engine's algorithm**
  (same YXZ branch structure) calling `f32::sin/cos/asin/atan2` — platform
  libm again — with one divergence: its gimbal-lock threshold (`CMP_EPSILON`
  1e-5) is ~40× looser than the engine's own `get_euler` epsilon (2.5e-7).
- **Shipped content already satisfies the placement constraint below:** the
  cat sits at level root (`game/scenes/level_01.tscn:85`), the level under a
  transform-less `UnseeingGame` root. The cat's authored spawn basis derives
  its euler through exact IEEE special cases (`atan2(0,-1) = π` exactly), so
  even scene-load is deterministic.

## Decision 1 — the arithmetic wire law (user-approved)

Canonical rotation is redefined **arithmetically**, with no trig anywhere in
the law:

- **Domain:** a rotation is wire-canonical iff every lane is finite, lies in
  the closed interval `[-PI_F32, PI_F32]` (`PI_F32 = f32::consts::PI =
  0x40490FDB`, the f32 nearest π — note it is *greater* than π, and `atan2`
  can return exactly `±PI_F32`, so the interval is closed at both ends), and
  any zero is spelled `+0.0`.
- **`canonicalize`:** per-lane. A lane equal to `0.0` becomes `+0.0`. A lane
  already in the domain is returned bit-identically (identity on everything
  the engine can produce in range). A lane outside the domain is wrapped in
  f64: `m = (lane as f64) % TAU` (IEEE `fmod` — **exact** by specification,
  hence bit-identical on every conforming platform), then one conditional
  `±TAU` adjustment into `[-π₆₄, π₆₄]`, then cast to f32 (correctly rounded;
  lands inside the closed f32 domain because `PI_F32 > π₆₄`), then zero
  normalization. Every operation is an IEEE 754 basic operation — add,
  subtract, multiply, remainder, comparison, conversion — all exactly
  specified and deterministic across x86_64, arm64, and wasm32.
- **The fixed-point iteration and the `Basis::from_euler`/`get_euler_with`
  roundtrip are deleted.** `canonicalize` becomes infallible past the
  finiteness gate; the 16-iteration `InconsistentState` exit disappears.
- **Public API signatures are unchanged** (`canonicalize`, `try_canonical`,
  `try_replacing_yaw/pitch`, `canonicalize_replacing_yaw/pitch`). The strict
  `try_*` variants keep their refusal semantics: input must bit-equal its
  canonical image. The refusal surface keeps the same shape (out-of-domain
  values and `-0.0` spellings refused on the wire).

This is a **normal form over engine euler states (per-triple), not over SO(3)
elements**. Godot stores the euler triple verbatim; two triples that denote
the same 3D rotation are two distinct engine states, and the wire's job is
faithful engine state. `-PI_F32` and `+PI_F32` are therefore both admitted.
The old law normalized to the wrong equivalence class — a basis roundtrip the
engine never performs on these nodes — and paid platform dependence for it.

Rejected alternatives:

- **Deterministic trig dependency** (`libm` crate, keep the roundtrip
  definition): new dependency, machine-derived constants that cannot be
  hand-derived, and it still breaks the cat's same-platform restore-verify
  against the *engine's* trig unless the seam below is also changed — at which
  point the trig serves nothing.
- **Tolerance validation** (±1 ULP): abandons the bit-exact capture/restore
  contract the reproduction campaign proved, and weakens state-hash divergence
  detection.

## Decision 2 — cat rotation seam goes verbatim-euler, with a placement law (user-approved)

The cat's rotation seam switches from global to local euler APIs, which Godot
stores and returns verbatim:

- per-tick yaw write (`cat.rs:281`): `set_global_rotation` → `set_rotation`;
- capture read (`cat.rs:1379`): `get_global_rotation` → `get_rotation`;
- restore install (`cat.rs:1518`): `set_global_rotation` → `set_rotation`.

Because `set_rotation` replaces the dirty mask, the tick-order (move, then
write yaw) leaves the euler cache authoritative, so capture reads back the
pure state's own `ActorYaw` lane bit-for-bit and restore-verify compares
verbatim bits — the engine becomes a pure pass-through for rotation, on every
platform.

This is sound only under an **untransformed parent chain**, which becomes an
explicit placement law (same family as the wall-at-root rule): every ancestor
basis between a `WaveCat` and the world must be exactly identity (origin
offsets are irrelevant — translation never touches rotation composition; the
check is on the basis alone, exact equality, because any deviation breaks the
verbatim-bits guarantee). Violations produce the standard dual-channel
warning (stored editor warning + runtime print, virtual and callable
forwarder). Shipped content already complies.

Hero body and eye need no seam change: install/verify already use
`set_rotation`/`get_rotation` with no physics step between (tree paused), and
mid-gameplay captured bits — engine-derived after `move_and_slide` dirties
euler — are in-domain values the arithmetic law accepts as identity. They are
platform-flavored *data*, faithfully captured and faithfully restored; the
*law* no longer cares which platform produced them.

## What must be true when this ships (acceptance)

1. `cargo test` green on macOS **and** on Linux CI with identical
   hand-derived literals — no platform-conditional tests, no
   machine-copied bits.
2. A capture blob fixture authored bit-exactly in a test (text lanes written
   by hand) restores cleanly on both macOS (local run) and Linux (CI) — two
   different libms proving the wire contract is platform-free. wasm follows
   by construction: no platform-dependent operation remains in the law.
3. The cat's restore-verify postcondition passes with the verbatim seam.
4. The placement law warns (both channels) for a cat under a rotated parent
   and stays silent at root placement.
5. Mutation evidence: realistic mutations of the wrap arithmetic (domain
   boundary, TAU adjustment direction, zero normalization, f64/f32 cast
   order) each fail a named test.
6. Wiki and issue #82 updated; #82's "pre-existing" claim corrected (the law
   is branch-new; main never contained `support_motion.rs`).
