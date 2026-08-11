# The reproduction loop — capture a game, launch it again, act, and prove it

*Design frozen 2026-08-11. What we decided to build and why. How the shipped
thing works belongs on the wiki, not here.*

Builds on the state layer
(`docs/superpowers/specs/2026-08-10-debug-observability-design.md`, shipped).
Sequenced **before** the pixel oracle gate
(`docs/superpowers/specs/2026-08-11-pixel-oracle-gate-design.md`), whose
seed prerequisite this work delivers early.

## The problem

The state layer made the game *observable*: `snapshot()` reads the frame's
whole state vector as JSON, and the `explain_*` family answers "why". But
observation is one-way. An agent that finds a bad state cannot **get back to
it**: there is no way to capture a running game as data, launch a fresh
process *into* that state, perform actions, and read the state again. So a
reproduction today is a prose recipe — "walk to the divider, tap twice,
wait for the fan" — replayed by hand, at real-time speed, with no proof the
replay reached the same state. That gap, not any missing observable, is what
still makes debugging feel incomplete: the loop *trace → relaunch from trace
→ act → trace → compare* has only its first verb.

Screenshots are already demoted and stay demoted. Everything below is
structured data end to end; no pixel is read anywhere in this design.

## What the audit found (2026-08-11, at `05839f8`)

- **Capture is close but holey.** The snapshot binds pool + echoes +
  sources + walls + camera at one instant, but the hero has no block in it
  (position, velocity, yaw, pitch, tap clocks are 8+ separate property
  reads; `tap_queued` — `rust/src/nodes/player.rs:134` — has no accessor at
  all). The pool's f64 shadow (`t0`/`end`, `rust/src/pulse_pool.rs:70-77`) —
  what eviction actually compares (`pulse_pool.rs:162-178`) — is
  unreachable; capture sees only the f32 lanes, and one ULP is exactly where
  "oldest" tiebreaks flip. The cat's future is private (PCG32 state, roam
  target, timers, gait phase — `rust/src/cat_brain.rs:166-174`,
  `rust/src/cat_gait.rs:161-171`), as is the viewmodel's footstep clock
  (`step_t`/`step_side`, `rust/src/viewmodel.rs:142-153`).
- **Restore does not exist.** No load/deserialize surface anywhere in
  `rust/src` or `game/scripts`. `emit` picks its own slot
  (`pulse_pool.rs:157-178`), so hole layout cannot be rebuilt by re-emitting;
  `EchoQueue::schedule` is pub in Rust with no `#[func]` door
  (`rust/src/echo_queue.rs:61`); a cadence gate's booked appointment cannot
  be set (`rust/src/sound_source.rs:229-311`); the cat's brain is built only
  in `ready()` (`rust/src/nodes/cat.rs:139`).
- **Acting is half-built.** Movement works headless through
  runtime-registered actions (`player.rs:241-253`, proven by
  `game/tests/movement_test.gd:68-96`). But the cane tap is mouse-event-only
  (`player.rs:173-202` → private `cane_tap`, `player.rs:389-447`; the public
  `queue_wave` bypasses the whole decision tree), there is no input-shaped
  look primitive, and the vendored gdUnit4 SceneRunner's input simulation is
  used by zero tests.
- **Nothing pins determinism.** No `--fixed-fps` anywhere in the repo, so
  `now += dt` (`game/scripts/main.gd:134-135`) is a function of real frame
  deltas and two runs never agree. Seeding is coupled to the demo tap: the
  only way to seed the game's only RNG (`main.gd:67-69`) also arms a
  4-second wave emitter that contaminates the pool. And a restored clock
  fires one spurious beat per source, because a jumped clock legally buys
  one beat (`sound_source.rs:289-311`).
- **The substrate is good.** The full game loop runs headless today
  (`ci/pipeline.sh` boots it for 30 frames; `game/tests/observer_test.gd`
  loads `main.tscn` headless and snapshots it live). The Rust core is
  deterministic by construction (BTreeMap clustering, pinned drain order),
  and Godot-physics dependence is thin — nearest-hit raycasts plus the
  character controllers.

## What the industry says (research, 2026-08-11)

The full survey lands on the wiki; the load-bearing findings:

- **The action log is the canonical truth; a snapshot is a cache that must
  be re-derivable.** libTAS refuses a savestate whose input prefix doesn't
  match the movie; OpenTTD replays a command log into savegame anchors;
  GGPO snapshots every frame *and still* demands bit-exact determinism;
  Riot's Chronobreak chose reconstruction-by-replay for exact server-state
  restore. A serialized state is correct only if nothing was omitted, and
  **omission is silent** (bevy_ggrs's top documented pitfall; GGPO's rule:
  RNG state is part of game state).
- **Restore-to-state is what upgrades a driving loop into a debugging
  loop.** ALE's `cloneState(include_rng)`/`restoreState` on a deterministic
  emulator enables branching and retry-from-state — and `include_rng` exists
  because forgotten RNG was the last silent divergence.
- **Determinism scope is honest: same build, same platform.** Cross-platform
  float determinism is effectively unattainable (Gaffer on Games; Jolt
  guarantees same-binary only). Artifacts pin commit + platform in a header
  (BizHawk BK2's `SyncSettings` is the model) and are short-lived repro
  artifacts, not archives.
- **Record at action level.** `Input.parse_input_event` does not propagate
  under `--headless` (godotengine/godot#73557, open); the proven transport —
  in this repo's own vendored gdUnit4
  (`game/addons/gdUnit4/src/core/GdUnitSceneRunnerImpl.gd:554-589`) — is
  `Input.action_press`/`action_release` plus direct handler delivery.
- **The loop contract is Gym's**: `reset(seed, state?) → obs`,
  `step(action) → obs`. Divergence is caught by per-tick state hashes
  compared between runs (AoE, SupCom), which localize it to the first bad
  tick instead of minutes later.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Launch-from-state | **True state restore** (ALE-style blob), verified by re-simulation | User's call, 2026-08-11. Enables branching and retry-from-state without replaying from boot. The known omission risk is answered by the verification gate, not accepted. |
| Cat | **Full state in the blob** | The loop must reproduce cat-involved bugs (eviction pressure, creature-band seams) exactly. Per-platform determinism caveat declared in the header, never hidden. |
| Actions | **Per-tick action tape + primitives that compile to it** | The tape drives the real input paths (reproduction-faithful); primitives (`walk_to`, `turn_to`, `tap`, `wait_ticks`, `wait_until`) spare agents the hand-arithmetic. One artifact, two authoring levels. |
| Read/write split | **`capture()` on `WaveObserver`; restore on a new `WaveRestorer` node** | "Observation never mutates" survives untouched. Capture is a read; restore is a write and gets its own boundary node. |
| Blob totality | **All-or-nothing** | A partial blob is the omission trap with a file extension. Any subsystem that cannot answer → one-key refusal, never a blob missing a group. |
| Serialization | **Versioned JSON `VarDictionary`**, hash via hand-rolled FNV-1a over canonical bytes | Zero new crates (no serde; std `DefaultHasher` is not run-stable). A state-format version constant, bumped on every format change, checked by `restore`. |
| Diff | **Gets code**: one pure `diff(a, b)` module | Revises the 2026-08-10 spec's "no code, by design" — that ruling was about stored history, which this adds none of. Verification needs canonical serialization anyway, and "hashes differ" without naming the field is a vacuous failure. |
| Sequencing | **This program first; pixel oracle gate second** | User's call, 2026-08-11. The seed decoupling both need lands here. |

### Approaches rejected

**Replay-from-boot as the only mechanism.** The industry default (Riot,
libTAS, OpenTTD) and the recommended option — but rejected by decision: a
pure-replay loop cannot branch from a state without re-simulating from
boot each time, and the agent workflow this exists for (reproduce, restore,
try a different action, diff) is branching. Replay discipline is kept
anyway, demoted from mechanism to **verifier**: a blob is trusted because
re-simulation from it matches the original run, not because serialization
looks complete.

**Scenario anchors (approximate restore).** Cannot reproduce pool /
eviction / echo-timing bugs — the class this engine actually suffers from —
and a plausible-but-wrong state is the exact failure the refusal contract
exists to prevent.

**Restoring the pool by re-emitting.** `emit` chooses slots by its own
scan, so hole layout and expired-slot lanes — which feed `slot_scan_limit`
and future eviction — cannot be reproduced. Restore writes slots directly.

## Architecture

```
rust/src/reproduce/
  blob.rs     capture/restore law: the state groups, totality,
              format version, canonical bytes + FNV-1a hash     (pure)
  tape.rs     tape format: header, per-tick action lines,
              parse/emit, primitive compilation                 (pure)
  diff.rs     canonical snapshot/blob comparison:
              first divergent field, field-level deltas         (pure)
rust/src/nodes/
  restorer.rs WaveRestorer — the one write-side node            (boundary)
```

`WaveRestorer` is a `Node`, constructed and injected by `main.gd` exactly
as `WaveObserver` is — handed the level and the player. It owns nothing and
holds no law. New doors it needs, each a thin `pub(super)` or `#[func]`
passage with the law kept pure: `PulsePool::load_slots` (writes lanes *and*
f64 shadow, preserving indices and holes), an `EchoQueue` injection door, a
`Cadence` appointment setter, a composed hero placement, a
`CatBrain`/`CatGait` construct-from-state, and viewmodel clock setters.

`capture()` joins `WaveObserver` — it is a read, wider than `snapshot()`:
everything the snapshot reports **plus** the f64 shadow, the cat's full
state, the viewmodel clocks, the hero's tap clocks and wave queue, and the
game-side environment (clock `now`, flicker fields + RNG state, demo-tap
schedule), fetched through `main.gd`'s thin `capture_env()`/`apply_env()`
pair. Presentation state (settings overlay, resolution) is not sim state
and is excluded, GGPO-style.

### The restore transaction

Under a frozen tree (`SettingsMenu` already owns the pause plumbing,
`rust/src/nodes/settings.rs:314-341`):

1. Check header: format version, level scene path. Mismatch → refusal.
2. Apply the clock and environment first.
3. Apply pool slots directly; echo book; hero; cat; viewmodel.
4. Re-pin every cadence appointment to its captured `next_emit` **after**
   the clock, so the one-beat-per-jump law fires nothing. An overdue
   captured appointment stays overdue and fires on the next tick, exactly
   as it would have in the original run.
5. Re-capture and compare hashes. Mismatch → the restore **refuses loudly**
   (one-key grammar) and reports the first divergent field via `diff`.
6. Unfreeze (or hand control to the tape runner).

Restore never partially succeeds silently: any step that cannot apply
aborts with the refusal naming the subsystem.

## The action tape

A BK2-shaped artifact. Header: state-format version, git commit, platform,
level scene, seed, fixed dt, tick count, and the **anchor** — power-on
(boot with this seed) or the hash of the blob it starts from, BizHawk's
`StartsFromSavestate` distinction made explicit — everything sync-relevant,
so a tape that outlives its build or loses its blob refuses instead of
desyncing. Body: one line
per physics tick — held movement actions, look delta (consumed by a new
input-shaped look entry that applies `MOUSE_SENS`/`PITCH_LIMIT` through the
same code a mouse event reaches), and tap events routed through a new
`#[func] tap()` that runs the **real** cane decision tree (aimed raycast,
rest/swish arbitration), queued to the physics tick like every other wave
decision. `queue_wave` stays what it is — a wave faker for probes — and is
never what the tape uses.

Primitives compile to tape by running in-sim while recording: `walk_to`
drives the actions closed-loop once, the resulting tape replays open-loop
forever after. Replay transport: `action_press`/`action_release` plus
direct handler delivery — never `parse_input_event` alone.

## Determinism substrate

Lands first, as its own plan:

- **Seed decoupling.** `UNSEEING_SEED` (env, and a `?seed` URL param on
  web) seeds the flicker without arming the demo tap; `?demo`/
  `UNSEEING_DEMO` now seeds too — delivering the pixel-oracle spec's
  prerequisite early. Seeding the only RNG must not cost a contaminated
  pool.
- **The fixed-timestep recipe.** `godot --headless --fixed-fps 60` wrapped
  in the harness, making `now` an exact function of the frame index and
  pinning the process/physics interleave that wave birth times ride on.
- **The hero block joins `snapshot()`** (diagnosis, not just capture):
  position, velocity, yaw, pitch, tap clocks, queued waves — one instant,
  one call. `tap_queued` gets its accessor.

**Stated honestly:** hero and cat kinematics flow through Godot's
character controller, and Godot Physics carries no determinism guarantee of
any kind (godotengine/godot#112976). This game's scenes are simple — one
capsule, one cat, static boxes — and the gate below *measures* whether that
is reproducible in practice, warm-boot-pair style, rather than assuming it.
If it flakes, that is a finding with an escalation path (quantized branch
inputs, Rust-side kinematics), not a surprise. Blob and tape are valid
same-commit, same-platform only; the cat's contract is already
per-platform (`cat_brain.rs:12-16`).

## Verification — what makes restore trustworthy

The governing rule is unchanged: a vacuous pass is worse than a failure.
Restore is the one place the whole design could quietly lie, so it gets
three instruments in the headless gate:

1. **Round-trip.** `capture → restore → capture`; the two hashes must be
   equal. Catches serialization bugs, not omission.
2. **Advance-and-compare — the omission detector.** Run A: boot, drive to
   state T, capture, advance N input-free ticks, snapshot. Run B: boot
   fresh, restore A's blob, advance the same N ticks, snapshot. The two
   snapshots must be identical (hash, then `diff` on failure). Input-free
   advancement already exercises pool expiry, cadence beats, echo firing,
   flicker, and the cat's whims; once the tape exists the same gate runs
   with actions. Any state that influences the future but escaped the blob
   fails here at the first tick it matters.
3. **The deliberate break.** Omit one field from restore — the cat's RNG,
   one cadence appointment — and watch advance-and-compare fail; restore it
   and watch it pass. A gate never shown to catch a real omission has not
   been shown to work.

Then the mutation check, as ever: flip a restored field, flip the format
version check, flip the hash — each must fail at least one test.

## Testing

- **Pure cargo tests** for `blob`, `tape`, `diff`: hand-derived literals
  (a slot captured at `t = 0.5` with `max_r 6.0`, `speed 5.5` restores to
  ring radius **2.75**, from the contract, not the code); tape parse/emit
  round-trips; diff names the planted divergent field; FNV-1a against
  known vectors.
- **gdUnit4 integration**: the restore transaction against a built level;
  the spurious-beat trap (restore, tick once, assert no source fired early);
  slot-hole fidelity (capture a pool with an expired slot under a live one,
  restore, assert `slot_scan_limit` identical); the cat's next roam target
  identical after restore; refusal wording for version/scene/hash
  mismatches.
- **The harness gate**: round-trip, advance-and-compare, and the deliberate
  break, run from `ci/pipeline.sh`.

## Delivery — three plans

1. **Substrate.** Seed decoupling, the fixed-fps recipe, the hero snapshot
   block, `tap()` and the look entry point. Small, immediately useful,
   independently shippable; unblocks the oracle gate.
2. **Capture and restore.** `reproduce/blob.rs`, the subsystem doors,
   `WaveRestorer`, `capture()`, round-trip and advance-and-compare
   (input-free) gates, the deliberate break.
3. **Tape, primitives, harness, diff.** `reproduce/tape.rs`,
   `reproduce/diff.rs`, primitive compilation, `tools/reproduce.sh`
   (boot headless from a blob, replay a tape, emit tick-keyed NDJSON
   snapshots, diff two runs), advance-and-compare with actions, and the
   godot-mcp capture recipe documented.

The wiki page is owed by whichever plan lands last; each plan updates the
sections it ships.

## Error handling

- Every new surface speaks the existing one-key refusal grammar; blob
  totality means capture refuses whole, never emits a partial blob.
- `restore` onto a wrong format version, wrong scene, or failing
  post-restore hash refuses naming the cause and the first divergent field.
- A tape whose header pins a different commit or platform refuses before
  the first tick.
- Truncation stays loud: NDJSON trace files carry their decimation
  interval in the header, exactly as the 2026-08-10 spec prescribed.

## Out of scope

- The pixel oracle gate (next program; spec frozen 2026-08-11) and the
  framebuffer digest.
- `+watch` event streams and a self-describing schema — noted from the
  research as the two verbs Bevy BRP and Unreal Remote Control both ship;
  revisit when the loop is in daily use.
- Cross-platform blob/tape portability (desktop ↔ wasm), and any
  desktop-vs-wasm equivalence gate for the cat.
- MCP packaging of the primitives; `godot_exec` one-liners against
  `WaveObserver`/`WaveRestorer` suffice.
- Settings/presentation state in the blob.

## Documentation owed

Per `CLAUDE.md`: the wiki's **Engineering — Debugging and Observability**
page gains Capture / Restore / Tape / Verify sections and loses its four
audited drifts (the godot-mcp preconditions it calls unmet landed in
`750d161`; its Plan-2 description predates the pixel-oracle spec; "Today's
two members" heads a three-row table; test counts moved to 233 cargo / 185
gdUnit at `05839f8`). The external survey lands as a new wiki research page
(**Research — Agent-First Debugging**). **Engineering — Build, Test,
Deploy** gains the new gate entries; persistent memory records the
decisions; `CLAUDE.md`'s debugging section points at the loop when it
ships.
