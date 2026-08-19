# The pixel oracle gate — pinning Rust's beliefs against real pixels

*Design frozen 2026-08-11. Plan 2 of the debug observability work. What we
decided and why; the wiki describes the shipped behaviour once it exists.*

Plan 1 shipped in
`docs/superpowers/specs/2026-08-10-debug-observability-design.md`.

## The problem

Plan 1's layer answers questions about engine state, and it is trustworthy
about Rust. It is silent about the GPU, and it does not say so loudly enough
to be safe.

`rust/src/sight.rs` is the cargo-pinned occlusion reference. The GLSL in
`game/shaders/pulse_pool.gdshaderinc` is a **hand transliteration** of it.
Nothing holds them together. The only pin is `game/tests/data_skins_test.gd`,
which does `String.contains` on eight literal substrings of the shader source —
structurally blind to *added* code, and covering none of: the `k < 3` slab loop
bound, the `t0 > t1` early return, the `lo`/`hi` rect packing, the Z half of
`wall_near`, the axis-parallel branch body, or the `i >= u_wall_count` breaks.

Concretely: narrow the slab loop's `k < 3` to `k < 2` and every wall becomes
infinite along Z. The screen over-occludes badly. 220 cargo tests stay green,
167 gdUnit4 cases stay green, and `explain_ray` cheerfully reports the correct
crossing count — **so the layer sides with Rust and tells you the shader is
fine.** That is the exact failure this whole effort exists to prevent, sitting
inside the tool built to prevent it.

Recorded as the OPEN item in
`docs/superpowers/plans/2026-08-10-debug-observability-state-layer.md`. This
spec closes it.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Scope | **The oracle comparison only.** No digest. | The digest describes a picture without knowing what it should be. It answers a different question class, and it does not close this gap. Build it later if it earns its keep. |
| Host | **The browser smoke gate** (`test/web_smoke.sh`) | It already executes every shader under SwiftShader against the real wasm export, and already runs in `ci/pipeline.sh` and `deploy.sh`. A shader-only edit then fails the build automatically. |
| Pixel transport | **Godot reads its own framebuffer and publishes numbers** | `get_viewport().get_texture().get_image()`, exactly as `game/tests/probe/occlusion_probe.gd` does. Python compares numbers to numbers. |
| Existing machinery | **Extend, never rebuild** | The readback, the windowed `override.cfg` dance and the warm-boot-pair law already exist in `occlusion_probe.gd`, `tools/probe_visibility.sh` and `test/web_probe.py`. |

### Rejected: capturing a screenshot and decoding it in Python

`web_probe.py` already does this for its boot canary, and reusing it was the
first design. It is wrong here: encoding a 1280×720 PNG, shipping it over the
DevTools socket and decoding it in Python to read **four pixels** is absurd, and
it drags a screenshot pipeline into a gate whose whole thesis is that pixels
should arrive as numbers.

The existing `count_lit` check stays exactly as it is. It answers a different
question — did the engine boot and draw anything at all — and it is the canary
for a shader that fails to *compile*.

### Rejected: reading the canvas from JavaScript

`readPixels` on Godot's web canvas needs `preserveDrawingBuffer`, which the
export does not set, so it would return empty or stale data. Letting Godot do
its own readback sidesteps this entirely and reuses a path already proven on
desktop.

### Rejected: the windowed desktop probe as the home for this

Real GPU and faithful, but outside `ci/pipeline.sh` — it catches a regression
only when a human runs it, which is precisely how this gap persisted. It keeps
its existing job (below).

## Architecture

Three sides, each doing only what it can.

**Rust decides the law.** New pure module `rust/src/observe/oracle.rs`:

```rust
pub enum Verdict { Lit, Dark }
pub fn expect_lit(walls_between: u32, kind: i32) -> Verdict
```

A two-state verdict, deliberately — not a predicted brightness. Predicting a
number would mean reimplementing the shader's whole reveal chain in Rust, which
is the duplication this design exists to avoid; and it would make the gate fail
on any harmless tuning change. Lit-or-dark is the property that a broken
occluder actually violates.

Player sounds (kinds 0, 1, 2) are cut crisp at a wall — any crossing means
`Dark`. A world source (kind 3) is muffled, not silenced, so it stays `Lit`
whatever the crossing count. The constants are `level_plan::HUM_THROUGH` and
`SOURCE_THROUGH`. This is a real law; it belongs in a pure module and is
cargo-testable with no GPU.

**GDScript projects and samples.** A probe scene, armed only by `?probe`:
poses the hero at a fixed spot, waits for the demo tap, settles, and for each
sample point calls `explain_ray(eye, point)`, asks the oracle for a verdict,
and reads its own framebuffer at `unproject_position(point)`.

`brightness` is the **peak red channel over a ±2 pixel neighbourhood, taken
across a span of frames** — the same measurement `occlusion_probe.gd::_peak_r`
already makes, and for the same two reasons: the image is greyscale so red
carries it, a thin outline can fall between sample points, and a peak across
frames survives a flicker dropout that a single frame would read as darkness.
The probe requires `?demo` as well as `?probe`, since the tap is what lights
anything at all.

Publishes one JSON payload through `JavaScriptBridge`:

```json
[{"name": "...", "walls_between": 1, "expect_lit": false, "brightness": 0.03}]
```

**Python compares.** `test/web_probe.py` pulls that payload with
`Runtime.evaluate` and asserts each point. Numbers to numbers; no image crosses
a process boundary.

## Sample points

**At least one point Rust says is lit and one it says is dark, in the same
frame, from the same tap.** A gate where every point agrees trivially would
pass a shader that ignores walls entirely.

The straddling pair is the load-bearing one: a point on the near side of the
divider and a point behind it, both within the tap's reach. Narrowing the slab
loop turns the lit one dark and the assertion fires.

## Two thresholds, with a gap

Lit points must exceed a floor; dark points must stay under a ceiling; **the
two must not meet.** Noise then cannot flip a verdict. The gap is reported on
every run, so a shrinking margin is visible before it becomes a flake.

## A determinism defect this depends on, fixed first

`game/scripts/main.gd:66` seeds the flicker only when the `UNSEEING_DEMO`
environment variable is set — which never happens in a browser. On web the
flicker therefore runs **unseeded**, and it gates reveal intensity and can drop
out entirely. Sampled brightness would vary run to run and the gate would
flake.

Fix: seed whenever the demo is armed, whatever armed it. Small, and it makes
the whole web path reproducible.

## Error handling

- **The probe must not touch a normal visit.** It arms only on `?probe`, and
  the bridge is published only then. A player loading the site gets the game,
  unchanged.
- **A missing payload is a failure, never a pass.** If `Runtime.evaluate`
  returns nothing — probe never armed, bridge never published, engine never
  booted — the gate fails loudly. A vacuous pass is worse than a failure, and
  an empty sample list satisfies "every sample agreed".
- **The sample count is asserted**, so a probe that silently published fewer
  points than it was written to publish fails rather than passing on a subset.
- **Refusals propagate.** If `explain_ray` returns `unavailable`, the payload
  carries the reason and the gate fails naming it, rather than treating the
  point as agreeing.

## Testing

- **`oracle.rs`** gets cargo tests with hand-derived verdicts: a kind-0 tap
  behind one wall is dark; a kind-3 source behind one wall is lit but dimmer;
  zero walls is lit for every kind. Literals derived from `HUM_THROUGH = 0.55`
  and `SOURCE_THROUGH = 0.30`, never read back from the code under test.
- **The acceptance criterion is the deliberate break.** Narrow the slab loop's
  `k < 3` to `k < 2` in `game/shaders/pulse_pool.gdshaderinc`, rebuild the
  wasm, and **watch the gate fail**. Then revert and watch it pass. This is
  expensive — a full wasm rebuild — and it is the entire point. If it does not
  fail, the gate is decoration, and we learn that before shipping it.
- **The threshold gap is verified**, not assumed: record the measured lit and
  dark brightnesses on a passing run so the margin is a number in the log.

## Stated limits

- **SwiftShader is a software rasterizer.** This gate covers occlusion, which
  is geometry-driven and should reproduce faithfully. It does **not** cover
  depth-fight-dependent artifacts, which may behave differently there than on a
  real GPU. Those remain the windowed probe's job
  (`tools/probe_visibility.sh`), which keeps its warm-boot-pair law.
- **This closes the occlusion half of the shader gap, not all of it.** The six
  uncovered constructs all live in the occlusion functions, so the coverage is
  meaningful — but a shader edit elsewhere (the outline maths, the shell
  raytrace, the mood layer) remains unpinned. Say so on the wiki; do not let
  this gate read as a general shader contract.

## Out of scope

- The framebuffer digest — per-oid pixel counts, crease/silhouette run
  classification, melted-pair detection. A separate question class.
- Object-id readback and data-pass access. The final composited image is enough
  for an occlusion oracle.
- Hiding the hearing quad in the browser.
- Committed baseline images of any kind. The binary policy stands.

## Documentation owed

The wiki page *Engineering — Debugging and Observability* currently states that
no gate asserts shader correctness. When this lands, that section must be
rewritten to say what is now pinned and what still is not — and the plan's OPEN
item closed with a pointer to the gate rather than deleted.


---

## Superseded, 2026-08-18

**This spec's law no longer exists.** It designed

```rust
pub enum Verdict { Lit, Dark }
pub fn expect_lit(walls_between: u32, kind: i32) -> Verdict
```

on the premise that player sounds are cut crisp at a wall while a world
source is muffled but not silenced, with `HUM_THROUGH` and `SOURCE_THROUGH`
as its constants. The 2026-08-14 barrier campaign deleted `HUM_THROUGH` from
both languages and made a wall absolute for every kind, so the function
degenerates to `walls_between == 0` with `kind` a dead parameter, and the
cargo tests this spec planned around those constants are untestable as
written.

**Its comparison is nonetheless shipped.** "At least one point Rust says is
lit and one it says is dark, in the same frame, from the same tap" is
implemented — in a stronger form, since it reads one wall from both sides
across one voice change and so cancels every other emitter — by
`game/tests/probe/occlusion_probe.gd`. Its checks 10 and 11 are the oracle
pattern outright: ask `explain_ray(...)["camera_crossings"]`, then hold the
pixel to a hand-derived window.

**`rust/src/observe/oracle.rs` is deliberately not built.** What was missing
was never an oracle but a scheduler and a host — see
`docs/superpowers/specs/2026-08-18-closing-the-renderers-last-gaps-design.md`,
which chooses software GL under `xvfb` in `ci/pipeline.sh` instead, on a
recipe this repository had already researched and written down.

The spec's other decisions have aged well and are worth keeping in view: the
two-threshold form with a reported gap, so a shrinking margin is visible
before it flakes; and the acceptance criterion — narrow the slab loop's
`k < 3` to `k < 2` and watch the gate fail — which remains the right way to
prove any rendered gate is not decoration.
