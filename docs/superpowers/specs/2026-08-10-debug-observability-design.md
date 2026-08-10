# Debug observability — structured state instead of screenshots

*Design frozen 2026-08-10. What we decided to build and why. How the shipped
thing works belongs on the wiki, not here.*

## The problem

Agents debugging this game reach for a screenshot, because a screenshot is
the only observable that exists. It is the worst possible one: slow to
produce, expensive in context, and mute about causes. A picture can show
that a wall's seam did not draw. It cannot say that two solids were handed
the same object id.

Four question classes send an agent to a picture today, and all four are in
scope:

1. **Is this visible?** — reveal reaching or leaking to the wrong place.
2. **Does it look right?** — outlines, seams, silhouettes.
3. **What happened over time?** — wave timing, echo scheduling, eviction.
4. **Where is everything?** — transforms, derivation, placement.

Three of the four are answerable from state alone. Only the second needs
pixels, and even then it needs *facts about* pixels, not the pixels.

## What makes this cheap here

The rendered image is a pure function of a small state vector: 64 pulse
slots, a wall table of at most 32 rects, the camera, and two per-frame
globals. The clock is simulated (`game/scripts/main.gd`, `now += dt`), the
flicker stream is seeded, and the pure modules are deterministic by
construction. A frame is a few kilobytes of JSON, and it replays.

`rust/src/ffi.rs` already exposes `positions()`, `pulse_data()`,
`pulse_dirs()` and `pending_echoes()` to tests. The crate contains no
`dump`, no `trace`, no `godot_print`. The raw material is there; the
vocabulary is not.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Pixel truth | **Framebuffer digest** | Reduce a readback to structured facts. Extends what `test/web_probe.py` already does. No shader maths duplicated. |
| Driver | **godot-mcp** (satelliteoflove, MIT) | Live in-session driving: freeze the clock, step frames, inject input, query state. Reaches the Rust core through `godot_exec`. |
| Transports | **Three, one source of truth** | Pure Rust functions exposed once as `#[func]`, consumed by godot-mcp live, by gdUnit4 in the gate, and by an on-demand dump scene. |
| Shape | **Snapshot / Diff / Explain / Digest** | "Why" is a question you ask, not a stream you record. Nothing touches the hot path. |
| Serialization | **`VarDictionary` + `JSON.stringify`** | Zero new crate dependencies. No serde, no wasm bloat. Matches `pending_echoes()`. |
| godot-mcp addon | **Gitignored, dev-only** | `deploy.sh` ships the tree by `git archive`; an addon committed under `game/addons/` reaches the web export and collides with the `ci/gdunit4.lock` regime. Reproducibility traded for a clean shipped artifact. |

### Approaches rejected

**Snapshot only.** Blind to anything computed and discarded within a frame.
A cane tap casts a golden-angle ray fan and clusters ~200 hits down to a
handful of echo points; no snapshot ever contains the 200, so "why did that
wall answer nothing?" stays unanswerable — which is precisely the class of
bug that sends someone to a picture.

**Snapshot plus an event ring buffer.** Captures causality, but puts
emission calls in the renderer's hot path that the wave cost model is
careful about, needs discipline to stay zero-cost when disabled, and is a
firehose: record everything, hope some of it matters, pay context to read
past the rest.

**Per-pixel CPU oracle as the primary mechanism.** Powerful, but duplicates
shader maths wholesale. `explain_ray` below keeps the useful half — the
occlusion oracle — without replicating the renderer.

## Architecture

```
rust/src/observe/
  mod.rs      Observation types + the frame composer      (pure)
  explain.rs  the "why" re-computations                   (pure)
  digest.rs   pixel buffer + oid table -> structured facts (pure)
rust/src/nodes/
  observer.rs WaveObserver — the one registered class     (boundary)
```

`WaveObserver` is a `Node`, constructed and injected by `main.gd` exactly as
`HeroBody` and `WaveLevel` are — handed the `Pulses` shim, the `WaveLevel`,
the player and the camera. No global state, no singleton, no autoload. It
owns nothing. It reads the systems it was given and calls pure functions on
them, and every `#[func]` returns a `VarDictionary`.

This respects the layering law: all law stays in pure modules under
`rust/src/`, and `nodes/observer.rs` carries values across and adds nothing.

## The four verbs

### Snapshot

`observe::frame(...) -> FrameObservation`, one pure function fed by
references.

| group | contents |
|---|---|
| pool | per slot: index, kind, origin, birth, `max_r`, speed, gain, beam dir + `cos_half`, current ring radius, age, remaining life, live/expired, rank in the eviction order |
| echoes | each pending appointment: `at_t`, `pos`, `gain`, seconds until it fires |
| sources | per source: position, volume, reach, cadence, next emit time, walls between camera and hub, resulting `u_source_floor`, `Voice::slot_pressure()` |
| level | wall rects with the truncation flag if over 32, oid per solid, spawn pos and yaw |
| view | camera transform, fov; the globals `now`, `flick`, `breath` |

### Diff

No code. The driver samples snapshots and compares. A stored history would
only be state to invalidate.

### Explain

Pure re-computations over modules that are already deterministic.

- **`explain_ray(from, to)`** — every wall the sight line pierces, where,
  the `crossings` vs `crossings_from` asymmetry, and the resulting
  `HUM_THROUGH^n` and `SOURCE_THROUGH^n`. This is the GLSL oracle:
  `rust/src/sight.rs` is already the transliteration reference for
  `game/shaders/pulse_pool.gdshaderinc`, so a disagreement localises the
  bug to the shader without a single pixel.
- **`explain_reflection(origin, normal, max_r, max_echoes)`** — re-runs the
  golden-angle fan and reports all `ray_fan::RAYS` directions, hit or miss,
  the 0.9 m cluster cell each hit fell into, its rank, and every point
  dropped past `echo_budget` with the reason. **Needs physics context**:
  `space.intersect_ray` is legal only inside the physics tick, which is why
  `UnseeingPlayer` queues waves. So this queues too and answers on the next
  physics frame. It is not a synchronous call, and the surface says so: it
  splits into `request_explain_reflection(...) -> request_id` and
  `take_explanation(request_id)`, which returns `{"pending": true}` until
  the physics frame has run and the explanation exactly once thereafter.
  Every transport already round-trips — an MCP call, a test frame step —
  so a request/collect pair costs nothing and hides no latency.
- **`explain_oids()`** — the touch graph, the colouring, Δoid for every
  touching pair, and any pair under the 0.08 law.
- **`explain_eviction(kind)`** — which slot the next emit of that kind would
  claim, and by which of the three rules.

### Digest

`digest::reduce(pixels, width, height, oid_table, grid) -> Digest`, pure over
a `PackedByteArray` read back from `viewport.get_texture().get_image()`.

`oid_table` is not derived from the pixels — it is the `level.oid per solid`
map the same snapshot reports, passed in so the digest can name a region
`table` rather than `0.24`, and so an id present in the level but absent
from the frame is reported as **expected but unseen** rather than silently
omitted. `grid` is the coarse-downsample resolution, caller-chosen, and
echoed back in the output so a reader always knows what they are looking at.

Reports the lit fraction; per-oid pixel count and bounding box; silhouette
runs and crease runs counted separately; shared-boundary pairs with zero
crease pixels flagged as melted; and the coarse grid for cheap eyeballing.

The same function reads the data pass (R = reveal, G = object id,
B = distance) and the hearing output, differing only in how channels are
named.

## Data flow

### Live loop — godot-mcp, the primary one

Editor open, game running. A fixed cycle:

```
godot_game_time  freeze                    clock stops; observation cannot race logic
godot_input      tap / walk                the hero makes a sound
godot_game_time  step 30 frames            advance exactly, deterministically
godot_exec       observer.snapshot(now)    the state vector, as JSON
godot_exec       observer.explain_ray(a,b) why, when the snapshot is not enough
godot_editor     screenshot                last resort, when a digest disagrees with itself
```

`godot_exec` returns the value of the GDScript it runs, so each call is a
one-liner against `WaveObserver`. Freeze-then-step is what makes the loop
trustworthy: without it the state moves between the tap and the snapshot.

### Headless gate — gdUnit4, in `ci/pipeline.sh`

Suites in `game/tests/` build a level, drive the clock by hand, and assert
on structured facts. Snapshot and Explain run headless. **Digest does not** —
headless renders no frames, the same reason `tools/probe_visibility.sh` sits
deliberately outside the pipeline.

### On-demand dump — windowed, GPU

A probe scene shaped like `tools/probe_visibility.sh`: write
`game/override.cfg` for a fixed 1280×720, boot with `UNSEEING_DEMO=1` so the
flicker stream seeds to `0x5EED`, run a scripted scenario, write NDJSON
snapshots and per-capture digests into `game/reports/` — already gitignored,
so nothing leaks into a commit. The warm-boot law applies unchanged: two
runs, and only an agreeing pair counts.

### The determinism gap this closes

`main.gd` accumulates `now += dt` from real frame deltas, so two dump runs
do not produce bit-identical clocks. Trace runs therefore pass
`--fixed-fps`, which makes Godot report a constant delta and turns the
simulated clock into an exact function of the frame index. Without it,
diffing snapshots across runs compares a frame against a slightly different
frame.

## Error handling

The governing rule is the one `test/repo_hygiene.sh` already states about
itself: **a vacuous pass is worse than a failure.** An observable that
returns zeros when it cannot see is more dangerous than one that does not
exist, because the agent will believe it and debug the wrong thing.

- **Every observation is a sum type.** Not injected a level, no camera,
  digest called on a headless run, `explain_reflection` with no physics
  space — the dictionary carries `{"unavailable": "<reason>"}` and no data
  fields at all. An empty pool and an unobservable pool are never the same
  JSON.
- **Observation never mutates.** `snapshot` and every `explain_*` take
  `&self`. `explain_reflection` writes its hits into a scratch buffer, never
  into the real `EchoQueue` — otherwise asking why a wall did not answer
  would itself schedule echoes and change what is being measured.
- **Truncation is loud.** `WaveLevel` already says so past 32 walls; the
  digest names the count it dropped; the dump scene writes its decimation
  interval into the file header, so a sampled trace cannot be mistaken for a
  complete one.
- **No silent fallback.** Editor closed, or another MCP client holding the
  session: the agent does not degrade to guessing from source. It falls back
  to the dump scene — the same observables through a different transport —
  and says which one it used.

## Testing

The trap here is the mirror assertion. Asserting `snapshot().pool[3].kind`
against `pool.dat()[3].w` passes no matter what either does. Every
observable test hand-derives its literals instead: emit a kind-0 tap at the
origin with `max_r 6.0`, `speed 5.5`, at `t = 0`; at `t = 0.5` the snapshot
must report ring radius **2.75** and remaining life
**6.0/5.5 + 6.0 − 0.5**. Those numbers come from the contract, not from the
code under test.

1. **Pure cargo tests** cover `observe`, `explain` and `digest` with no
   Godot. `digest::reduce` gets a hand-built 8×8 pixel buffer with two known
   oid regions sharing a known boundary; the melted-pair check must fire at
   Δoid 0.00 and stay silent at 0.08.
2. **The oracle contract.** `game/tests/data_skins_test.gd` already pins
   `sight.rs` against `pulse_pool.gdshaderinc` line by line. Extend it so
   `explain_ray` is what gets pinned, promoting the oracle from a
   convenience to a tested contract — a shader edit that drifts from the
   Rust reference then fails the gate instead of misleading an agent later.
3. **gdUnit4 integration** drives a built level: a source two walls away
   reports `SOURCE_THROUGH²`, eviction claims the footstep and not the cane
   tap, `explain_reflection` leaves `pending_echo_count()` unchanged.
4. **The acceptance criterion — inject a known bug, confirm the observable
   catches it.** Give a solid a colliding oid and `explain_oids()` must
   report the violation. Put the fan behind a wall and the snapshot's
   `u_source_floor` must be `SOURCE_THROUGH¹`. An observable never shown to
   detect a real fault has not been shown to work; this is the same
   discipline as watching a test fail for the right reason first.

Then the mutation check: flip `HUM_THROUGH`, flip the 0.08 threshold, flip
the eviction order — each must fail at least one observable test. Anything
nothing catches marks that behaviour as unobserved.

## Delivery — two plans

This spec is one coherent subsystem but too large for one plan. It splits on
the line the architecture already draws: everything that runs without a GPU,
then everything that needs one.

**Plan 1 — the state layer.** `observe/mod.rs`, `observe/explain.rs`,
`nodes/observer.rs`, the pure cargo tests, the gdUnit4 suites, the
`explain_ray` oracle contract, and the godot-mcp live loop. Answers three of
the four question classes — visibility, timing, placement — and runs
entirely in the headless gate. This lands first and starts paying off on its
own; nothing in it waits on the digest.

**Plan 2 — the pixel layer.** `observe/digest.rs`, the viewport readback,
the windowed dump scene, and `--fixed-fps` NDJSON capture. Answers the
fourth class, "does it look right", and inherits the vocabulary Plan 1
established — the digest is keyed by the object-id table Plan 1's snapshot
already reports, so building it second is the cheaper order, not merely the
safer one.

The wiki page is owed by whichever plan lands last; Plan 1 creates it, Plan
2 rewrites it to describe the whole shipped layer.

## Out of scope

- A per-pixel CPU oracle replicating the renderer. `explain_ray` keeps the
  occlusion half; the rest is not worth the duplication.
- Committed digest baselines as a regression gate. It collides with the
  binary policy, and baselines would have to be perceptual hashes or
  structured digests, never frames. Revisit only on the documented trigger.
- An event ring buffer. Rejected above; revisit if Explain proves unable to
  answer a real causality question.
- Any observation of the agent itself (OpenTelemetry and friends). That
  instruments the assistant, not the game.

## Documentation owed

Per `CLAUDE.md`, this work is not done when the tests are green. It is done
when the wiki describes the shipped behaviour: a new **Engineering —
Debugging and Observability** page covering the four verbs, the three
transports, the godot-mcp loop, and the determinism requirements — plus an
update to **Engineering — Build, Test, Deploy** for the new gate entries and
the gitignored addon.
