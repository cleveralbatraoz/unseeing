# Rendering engine design audit — state at 2026-08-18

Branch `pr47-review`, worktree `.claude/worktrees/pr47-review`, rebased onto
`origin/main` (32 commits of tooling/deploy/docs; main had not touched
`rust/src`, `game/shaders`, `game/scenes`, `game/project.godot` or
`game/tests`, so all prior work applied cleanly).

**Nothing is merged, pushed or deployed.** The finish-branch choice has not
been presented, and integration is the user's call.

## What this branch is

It began as a review of PR 47 ("A wall becomes a barrier no sound may
cross") and became a design-level audit of the whole rendering engine, at
the user's direction: *"fix all gaps until the implementation is ideal /
your goal is to compose a technically perfect rendering engine, not ad-hoc
fixes application."*

The barrier-law work (PR 47 plus its repairs) is the first 18 commits.
Everything after is the design audit, in three passes: five structural fixes,
three commits repairing what an adversarial re-review found in THOSE, and
four more closing the gaps that pass had carried rather than fixed. This
document covers all three.

## Verification state of the tree

Reproduced at commit `71e9345`:

| gate | result |
|---|---|
| `cargo test` | 510 passed |
| `cargo fmt --check`, `clippy -D warnings` | clean |
| gdUnit4 | 338 cases / 31 suites, 0 failures |
| headless boot | no script/shader/engine errors |
| `tools/probe_visibility.sh` | PASS on all THREE scenes (11 occlusion checks), reproduced across a cold and a warm boot |
| `gdformat --check`, `gdlint` | clean |
| `test/repo_hygiene.sh`, `ci/check_gdscript_policy.sh` | pass |

Rendered probe readings (Mesa / Linux / Godot 4.7.1, AMD Radeon,
Compatibility). The full table with each row's bound is on the wiki's
Build, Test, Deploy page; the two most informative:

| check | reading | bound |
|---|---|---|
| the fan's own body, ABSOLUTE, through one wall | 0.192–0.196 | 0.13 … 0.30 |
| the same shell inside the fan's OWN room | 0.306–0.322 | > 0.05 |

The positive controls are markedly stronger than the pre-audit readings
(0.161–0.314 for the own-face case, now 0.373–0.396). That is the wave-death
gate: the "muted" baseline no longer carries an unexpired front, so the delta
stops understating.

Every row was proven able to FAIL by breaking the law it guards — see the
adversarial-pass section below, which is where two of them turned out not to
be.

**Do not add `--fixed-fps` to the probe.** See the note in
`tools/probe_visibility.sh`; at a fixed 60 fps a 12-frame baseline is 0.2 s
against the fan's own 0.4 s cadence and case 4 collapses from 0.329 to 0.000.

## The five landed design fixes

Each was verified against the code before it was touched — the audit was a
lead, never evidence — and each was written test-first, with the correct
failure observed and the realistic mutations confirmed to fail.

### 1. `84b53bf` — a sound now ends where its own slot ends

`reveal_at` had **no end condition**. `dist > min(radius, d.y)` freezes into
the static `dist > max_r` once the front has run its course, and the decay
`1.3·e^(-ga/0.25) + 0.5·e^(-ga/3)` is a sum of exponentials, so it never
reaches zero. Every point a wave ever swept kept **0.0677** of peak (a tap)
or 0.2568 (a hum) indefinitely, going dark only when an unrelated later
sound claimed the slot. The visible life of a sound was a property of the
slot allocator.

`rust/src/render/reveal.rs` is the new pure home of the law, beside
`sight`'s: sight owns *where* a wave reaches, this owns *when it stops*.
Both are written against seconds-since-the-front-passed, the only coordinate
under which the fade is the same law near the source and at full reach.

The envelope was first brought to zero by subtracting its own value at the
tail. **That was wrong and is fixed** — see the adversarial pass below; it
is now a closing window over the last quarter of each wave's life, which
leaves the shape identical for every kind until then.

**Design note worth keeping:** the audit proposed a new `vec4 u_plife[MAXP]`
lane (1 KB) to carry death times. It is not needed — death is already
derivable from `dat` as `t0 + max_r/speed + fade_tail(kind)`, so the
shader's death is now *exactly* the CPU's `end` with no new uniform.

`pulse_fade_tail` in `game/shaders/pulse_pool.gdshaderinc` now has a real
cross-language gate: `game/tests/shader_contract_test.gd` reads the arms out
of the shipped GLSL and evaluates them against `render::reveal` through
`WaveCore.wave_fade_tail`, kind by kind, including kinds outside the four
`emit` packs. Mutating one arm fails with
`"GLSL grants kind 3 a 2.5 s tail while Rust budgets its slot for 2.0 s"`.

The commit also corrects the early-out comment the barrier campaign
invalidated: with a 0/1 occlusion gate, `bound <= reveal` cannot fire
against a `reveal` still at exactly 0.0, so the fragments paying the full
wall walk are precisely the ones in the room next door. It is not the loop's
affordability.

### 2. `269ae74` — a wall may take something from a source at last

`SOURCE_THROUGH` is documented as the ladder a source's silhouette descends
through walls (0.30, 0.09, ...) and it descended nothing. The level
pre-multiplied volume by muffle into one `u_source_floor`, which reached
`data_xray` as a floor under `reveal_at` — and a floor only ever competes. A
source's hub is unwalled from its own body by construction, so `reveal_at`
reads near full strength there however many walls stand between the source
and the player; `max()` handed back the full strength.

Now delivered as `u_source_volume` and `u_source_muffle` and composed as
`muffle * max(wave, volume)`, with `render::reveal::source_image` as the
pure law. With no wall between, it is exactly the old `max`, so a source in
the player's own room reads bit-for-bit as before.

The debug snapshot reports the halves separately (`source_volume`,
`source_muffle`) because the renderer consumes them separately; their
product is no longer a quantity anything on screen forms.

### 3. `4ed321f` — the acoustic image gets a band it can order in

`SOURCE_BAND = 1.0e-5` against a 24-bit depth buffer — and f32's identical
`2^-24` ULP near 1.0 — carries **168 distinguishable values** across
`DIST_PACK_RANGE = 40`: one code every **0.238 m**. Every limb of the
shipped fan lies inside one code, so housing, guard and blades resolved by
opaque draw order, and since a blade's sort key moves as it spins the
creases on the fan head reshuffled instead of rotating.

`rust/src/render/depth.rs` now states **both** bounds as cargo-tested
functions:

- wide enough to order: `band_resolution(1e-3, 40) = 2.38 mm`, under the
  fan's 0.012 m guard-to-blade gap (`MIN_SOURCE_LIMB_GAP`);
- narrow enough to stay unreachable:
  `deepest_world_fragment_in_band(1e-3, 0.05, 60) = 0.050050 m`, i.e. a
  world surface must stand 0.05 mm past a near plane that already clips
  everything closer.

Four orders of magnitude separate the two bounds, which is why the wrong
value looked right in any screenshot of a single source. The camera's near
and far planes moved into the same module, since the second bound is a
statement about them.

**The gdUnit assertion guarding this was
`assert_bool(1.0e-5 < 1.0 - 0.999999 + 1.0e-5)`, which reduces to
`x < 1e-6 + x` and is true for every x.** It is replaced by the derivation,
read back through `WaveCore`.

### 4. `8a9a9dd` — an anchor names a surface, not a whole slab

One prop set flush into the floor could cost a level every outline it has.

An anchor was written onto every class its entry owned — correct only while
the entry stays a merge singleton. Any coplanar merge splits a slab's faces,
rule (a) separates the pairs sharing an edge, and every one of those classes
still carried the slab's own label, so the check compared 0.15 against 0.15
and rejected the whole request:
`AnchorSeparationConflict { first_class: 0, second_class: 2, first_entry: 0, second_entry: 0 }`.
`WaveLevel::paint_labels` answers a rejection with one warning and `return`,
so every solid keeps its unpainted `BOX_ORDINALS`, which `pack_data` writes
to G unclamped and the LDR target saturates 1..5 to white.

`PaintEntryInput.anchor` is now `Option<FaceAnchor>` — a label plus the
direction of the face it belongs to. Slabs anchor `[0, +1, 0]` (floor) and
`[0, -1, 0]` (ceiling). A lone slab is still a merge singleton, so all six
faces collapse into one class and all six carry the label; that behaviour is
pinned in `level_test.gd`.

Ties between two faces answering a direction equally well resolve in build
order (`faces` builds a box -X, +X, -Y, +Y, -Z, +Z), pinned and
mutation-checked, because the wasm and desktop builds must colour one level
identically.

### 5. `4420bfa` — every label that can share a frame stands on one ladder

The role table was held to no separation law and shipped breaking it twice.
Creatures, the viewmodel and the palette never enter `paint_entries`, so the
graph colouring that enforces `MIN_SEP` everywhere else was structurally
blind to them, and the only test over the table was a per-row mirror
assertion that would agree with any numbers at all.

- `HeroBody` 0.82 vs `Ceiling` 0.90 → **0.079999983** in f32, a hair under
  the knee;
- `Ceiling` 0.90 vs `HeroCane` 0.96 → **0.06**, and the cane reaches the
  ceiling (eye 1.6, `PITCH_LIMIT` 1.35 rad, `CANE_REACH` 1.7 → 3.26 m
  against a 3.0 m ceiling), where the distance Laplacian is dead and the
  label difference draws the seam alone at half strength.

**The repair could not be local, and that is the finding.** Ten labels must
coexist in one frame — floor, five palette entries, cat, hero body, ceiling,
cane — and nine gaps across the 0.81-wide band leave exactly 0.09 each.
Pushing the ceiling clear of the cane pushed the body into the ceiling;
pushing the cat clear of that pushed it into the palette's top entry.

The ladder is `0.15 + 0.09k`, `k = 0..9`:

| rung | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
|---|---|---|---|---|---|---|---|---|---|---|
| label | 0.15 | 0.24 | 0.33 | 0.42 | 0.51 | 0.60 | 0.69 | 0.78 | 0.87 | 0.96 |
| owner | Floor | palette | palette | palette | palette | palette | Cat | HeroBody | Ceiling | HeroCane |

`Role::Shell` (0.33) and `Role::Moving` (0.60) deliberately reuse palette
rungs: they are standalone blueprint preview defaults that a level never
renders beside a wall, and twelve distinct labels do not fit in a band that
holds eleven. `Role::Case` (0.05) remains the grandfathered exception below
the band.

`WORLD_OIDS` moved from `nodes/level.rs` into `render::labels::WORLD_PALETTE`,
because the law is only checkable where the whole label universe is visible
at once. The suites now read the table through `WaveCore.role_labels()`,
`WaveCore.world_palette()` and `WaveCore.min_label_separation()` instead of
transcribing it — including `map_test.gd`'s `MIN_OID_SEP`, a third
executable copy of `MIN_SEP` that let a Rust-side change keep every gdUnit
case green while the seams rendered weaker.

## New Rust modules and FFI surface

- `rust/src/render/reveal.rs` — `flare`, `reveal_tail`, `source_image`,
  `SourceImage`. The reveal envelope and its end, and a source's standing
  acoustic image.
- `rust/src/render/depth.rs` — `ALWAYS_ON_TOP`, `SOURCE_BAND`,
  `DEPTH_CODES`, `CAM_NEAR`, `CAM_FAR`, `MIN_SOURCE_LIMB_GAP`,
  `window_depth`, `deepest_world_fragment_in_band`, `source_depth`,
  `band_resolution`.
- `rust/src/render/channel.rs` — `CHANNEL_LEVELS` (measured), `quantum`,
  `recon_eps`, `max_safe_range`, `reconstruction_budget`. What one data
  channel holds, and the geometric tolerance that turns on it.
- `rust/src/render/crease.rs` — `CreaseKnee`, `LOW_KNEE_RATIO`. The rendered
  response to a label difference, derived from `MIN_SEP`.
- `rust/src/sight.rs` — gains `Occluder`, which carries a wall's rect and its
  world Y span together; `wall_top` leaves five signatures.
- `rust/src/render/labels.rs` — gains `LADDER_BASE`/`LADDER_STEP`/
  `LADDER_RUNGS`, `ladder_rung`, `WORLD_PALETTE`, `coexisting_labels`.
- `WaveCore` gains, for the suites to read rather than transcribe:
  `wave_fade_tail`, `source_band`, `source_band_resolution`,
  `min_source_limb_gap`, `deepest_world_fragment_in_band`, `camera_near`,
  `role_labels`, `world_palette`, `min_label_separation`.

## The adversarial pass, and the six repairs it forced

A ten-agent workflow re-reviewed the five landed fixes, each skeptic tasked
with REFUTING rather than confirming, with a second opinion on every
refutation. It found real defects in three of the five. All are now fixed
(commits `c667d0a`, `b932898`, `45f0536`).

**The worst was mine, and it was in a test.** Making the wall muffle
multiply the whole acoustic image shrank every reading on the fan by 3.3x,
while the rendered probe's two hand-derived leak floors (0.12 and 0.08)
stayed where they were. The largest delta a full-strength leak could then
produce was `0.867 x (0.30 - 0.225) = 0.065` — under both floors. **The two
checks that exist to catch a wave reaching through a wall could no longer
fail.** Two agents derived this independently and I confirmed the
arithmetic before touching anything. They are ratios of the fan's own
standing image now, which is scale-free, plus a new ABSOLUTE row that
catches the one failure no unit test can see: an instance uniform that never
reached the GPU, because the suites and the observer read those uniforms
back through the same names that would have been renamed. Verified by
deleting the muffle from the skin — 0.659 against a 0.30 ceiling.

**The envelope repair had over-reached.** Bringing the decay to zero by
subtracting its own value at the tail reached zero correctly and darkened
everything else by a kind-dependent amount: 17.3, 39.7, 55.4 and 65.6 codes
of 255 for kinds 0..3. A cane tap and its own echo striking one surface read
0.3144 and 0.2264 at the same age — a 22-code split with no cause in the
world, only in the slot allocator. The decay is the acoustic law and the
tail is only the budget, so a CLOSING WINDOW now ends the wave instead:
flat at 1.0 through the first three quarters of every wave's life, then a
smoothstep to exactly zero. `CLOSE_FRACTION = 0.25`.

**Two "cargo-pinned references" pinned nothing.** `render::reveal::flare`
and `::source_image` had no non-test caller, and the shipped GLSL was held
to them by substring alone — a `contains()` cannot tell 1.3 from 1.0, or a
3.0 time constant from a 4.0 one. And `ga = age - dist / speed`, the single
time coordinate the whole law is written against, was asserted nowhere: with
`ga = age` the fan (ring time exactly 2.0 s against kind 3's tail of exactly
2.0 s) would stop revealing the outer metre of its own wash at the instant
its front arrived, while the ring kept drawing. Both cross the language
boundary numerically now, through `WaveCore.wave_flare` /
`wave_close_fraction` / `wave_death_time`, with the shader's own constants
parsed out of the GLSL and evaluated. Mutation-checked: the slow time
constant, `CLOSE_FRACTION`, and the time coordinate all fail.

**The probe had a second, older lie.** Its sweep window was 200 frames,
about 3.3 s against the fan's 11.42 s oscillation, so it could land wholly
in a phase where the beam points elsewhere — a POSITIVE control read 0.000
on one boot and 0.322 and 0.310 on the next two. The window is a duration on
the simulated clock now (`SWEEP_SECONDS = 12.0`), because the probe's frame
rate is set by the full-framebuffer readback it does every frame and a fixed
frame count means something different on every machine. Widening it exposed
the second half: the hero is a `CharacterBody3D` running its own physics and
drifts out from under the sample points over a window that long, so
`_peak_r` now re-applies the POSE every sampled frame, not only the aim.

Smaller repairs in the same pass: `paint_plan`'s `LABEL_MIN`/`LABEL_MAX`
were a second executable copy of the ladder endpoints; the Godot-facing role
roster was a hand-written array that a new `Role` could silently miss (it is
paired with an exhaustive `role_name` match now); `SOURCE_THROUGH`'s own
file still documented the law the campaign replaced; `pack_data` wrote G
unclamped while clamping R and B; the anchor direction was a world-up
literal rather than the slab's own basis; and two anchor limits that survive
— a column's curved flank cannot be named by any direction, and two entries
pinned to the SAME label across a seam are still refused outright — are now
written down and tested as the unreachable edges they currently are.

**Two of the five fixes survived refutation intact**: the label ladder and
the face-scoped anchor drew only minor findings, both of which are fixed.

## The four carried gaps, now closed

All four were eliminated in a second pass. Two of them were blocked on
"unverified platform facts", and the first thing that pass did was stop
deferring them and measure.

### 1. `9c35cc1` — a wall occludes where it draws

`sight::crosses` hard-coded the occluder's vertical span as `[0, WALL_H]`,
with one global `u_wall_top`. Nothing constrains a wall's Y:
`plan_wall_transform` normalises the BASIS and carries `origin.y` through
untouched, `wall_segment` then discards the height, and `sunken` stays quiet
for anything resting on or standing clear above the floor. A lifted wall left
a phantom barrier beneath it and an unoccluded strip over it; a lifted level
ROOT put every occluder below the map and failed the barrier law open
everywhere, silently.

The design review found it worse than reported: **`contains` shared the bug
and failed in the dangerous direction.** A source in open air under a lifted
wall was judged born *inside* it, so the birth-wall skip disabled that wall
for that source in every direction at once.

`sight::Occluder` now carries rect and span together, because a table of
rects beside a table of heights is two things that can disagree. `wall_top`
leaves five signatures and one uniform. `is_empty` is a CHECKED guard, not an
emergent property — the slab arithmetic accepts an inverted interval — and is
written `!(a <= b)` so a NaN lane reads as empty rather than as ordered.

**Pixel-identical on the shipped map**, which was the acceptance criterion.

### 2. `6de2eb6` — the knee that draws a seam is the separation that allocated it

`MIN_SEP` governed allocation in Rust; `smoothstep(0.04, 0.08, nrm)` governed
the rendered response in GLSL; nothing compared them, and `labels.rs` said so
in its own doc comment while shipping anyway. Lowering `MIN_SEP` to fit a
starved band kept every `separated()` test green while the shader faded over
a knee it no longer matched.

`render::crease::CreaseKnee` is a validated type, not a pair of floats: GLSL's
`smoothstep` divides by `hi - lo`, so an equal pair divides by zero and an
inverted one fades a bright seam dark. Ordering is judged after narrowing to
f32. `(MIN_SEP/2, MIN_SEP)` reproduces `0.04 / 0.08` exactly, so no pixel
moved; the shader default is deliberately `(0.0, 1.0)`, loudly wrong.

### 3. `212f07d` — the channel stops being a story

**Measured: 1024 levels per channel (RGB10_A2).** The brief said 8-bit LDR
and an earlier probe claimed RGB10_A2; at 8 bits the B-channel reconstruction
guard is broken four times over.

The measurement took two attempts and the first was wrong in an instructive
way: a single test base sits at one arbitrary place on the quantisation grid,
and that alone moves the answer a full bit. `0.5 x 1023 = 511.5` lies exactly
between two codes so half a code still crosses a boundary there, while
`0.25 x 1023 = 255.75` does not — the same buffer reported 2^-11 at 0.5 and
2^-10 at 0.25, which I briefly read as float16. Swept across seventeen bases
and demanding every one separate, it is 1024.

`render::channel` derives what that implies: one B code is `40/1023 = 0.0391`
m, the worst reconstruction error is half of it, and `RECT_SHRINK` clears
that by **0.45 mm** — a 2.3% margin on a tolerance chosen for an unrelated
reason. `reconstruction_budget` refuses the range at which they cross,
**40.92 m**, which `pack_range_budget` actively walks designers toward.

> **Superseded 2026-08-18.** Seventeen bases was still a subsample. Read at
> every base of the swept column and laddered in multiples of a nominal code
> rather than in whole bits, the same buffer collapses a 1/1023 step at two
> bases in 649 on Mesa/AMD; the smallest step that always survives there is
> **1.25 nominal codes**, and 1.02 on SwiftShader and on ANGLE/Apple Metal.
> The real worst reconstruction error was therefore 24.4 mm against a 20 mm
> tolerance — the guard was in the red, and its own test passed because it
> compared the tolerance against the nominal gap rather than the measured
> one. `RECT_SHRINK` is now 0.03 m, the quantum carries
> `render::channel::WORST_STEP_CODES`, and the refused range is 49.10 m. The
> paragraph above is kept as written because the lesson it teaches — that a
> sparse base sweep answers confidently and wrongly — turned out to apply to
> itself.

### 4. `e67166f` — the hearing pass asks which layer a pixel is

Two designs were refuted before the third worked, and both refutations are
worth keeping.

**The ALPHA-lane sentinel is closed by documentation.** `ALPHA`, if read from
or written to, moves a material to the transparent pipeline, and transparent
materials cannot appear in `hint_screen_texture` — which IS this pass, so
every source would vanish from the only buffer that builds the image.

**The depth texture is NOT closed**, though `data_core` had claimed so since
before this suite existed, and two of my own early measurements agreed with
the claim. Both were wrong: one probe sampled off its geometry, and one
conflated a shader edit with an unrelated revert. Measured properly, the
depth texture is live in Compatibility and carries real reversed-Z depth —
0.0158 at three metres against an analytic 0.01585 — an always-on-top
fragment reads back at 1.0000 against a band floor of 0.9990, and declaring
it beside `hint_screen_texture` costs the screen read nothing.

So a source is identifiable by one exact comparison. It is **ORed** with the
old wall-table inference rather than replacing it: the measurement is desktop
GL, WebGL2 is unmeasured, and where the depth texture is dead the term is
false everywhere and the pass degrades to precisely its former behaviour.
Better where measured, never worse anywhere.

`depth_texture_probe` keeps all four facts as a rendered gate — including one
assertion that had to be rewritten because it was weak: at the 60x gain a
world fragment needs to be visible at all, everything above 1/60 saturates,
so the layer separation is read at unit gain where a dead depth texture would
fail the band assertion rather than pass it.

## The second audit, and the regression it caught

A second adversarial pass ran over the four gap closures. It found one real
behaviour regression, three unfailable checks, and a scatter of drifted
comments — all fixed (`a4b7e4b`, `ca6b4ff`, `71e9345`).

**The regression was mine and one commit old.** `hearing_post` asks two
different questions — "is something between the eye and this surface" and
"is this surface an acoustic image" — and I had fed the first the answer to
the second. Every source fragment is an acoustic image by definition, so the
ring cut began dropping player rings over EVERY source pixel in the game,
including a fan standing in the open. They are separate predicates now, and
the split is what the suite pins.

**Three checks could not fail.** The depth probe compared `c` against
`min(c + d, c)` — an algebraic identity. The occlusion probe's outline ratio
had no non-vacuity guard while a comment promised one "for both ratios".
And `reveal`'s tail test asserted `reveal_tail == fade_tail`, a one-line
delegation compared against itself.

**A fourth was found while writing its own fix**: the re-derive test
scribbled a ONE-entry wrong table onto every skin, and the fixture level
holds exactly one wall — so it compared 1 against 1 and passed against a
deliberately broken build. That is now the fourth time in this campaign that
a check which passed turned out to be unable to fail, and the reason the
mutation step is not optional.

Two other real findings: `wall_footprint` still stamped a global
`[0, WALL_H]` — the last global wall-height read in the crate — which made
the pack-range budget raise an Error against a level lifted 2.557 m whose
true diagonal had not moved; and the wall table had two owners across five
skins, refreshed by one derive only because a runtime level happens to
derive exactly once before the composition root pushes. Both fixed.

## What remains, and it is no longer a list of gaps

Three items, none of them a defect in shipped behaviour, and the first is a
deliberate partial.

1. **The prop gap is closed for the outline cap and OPEN for the ring cut.**
   A player's ring still washes over a source hidden behind a pillar. That
   needs to know something stands in FRONT of the source, and neither the
   depth buffer nor the wall table can say so — the depth buffer holds the
   source's own faked value there, and props are in no occluder table.
   Closing it means giving the CAMERA occluder a table that includes props,
   which is a different law from the wave occluder (props are transparent to
   waves, deliberately) and a per-fragment cost on the hottest path. It also
   trades one artifact for another: a column's circular footprint
   over-approximated by a rect produces false positives at its corners,
   which notch rings near pillars. That trade wants a playtest, not a
   unilateral call.
2. **The label allocator's adjacency is a physical-contact relation**
   (`TOUCH_EPS = 0.01`) while the image needs a screen-adjacency one, and the
   B Laplacian is identically zero below a 0.48 m depth step. Contact-free
   props 0.4 m apart can therefore share a label and melt on screen. Fixing
   it properly means moving the silhouette test to a scale-relative predicate
   AND growing the adjacency relation together — neither works alone, and the
   pair is a re-encoding of G, the largest change the audit identified.
3. **The renderer still has no executable oracle.**
   `docs/superpowers/specs/2026-08-11-pixel-oracle-gate-design.md` designed
   one; `rust/src/observe/oracle.rs` was never built. Five rendered-probe
   checks and three platform measurements now stand where there was none, but
   they are hand-run and outside `ci/pipeline.sh`.

**Unmeasured, and now measurable:** the WEB target's channel depth and depth
texture. Both probes exist and would answer it; running them under the web
export needs export templates and the emscripten toolchain.

## Documentation state

- The wiki checkout at `/home/albatraoz/unseeing.wiki` has **3 local commits
  that are not pushed** (`29302ef`, `03e976d`, `c199f4f`). The first two
  cover the barrier law; the third rewrites `Mechanics-Rendering.md`,
  `Mechanics-Waves.md`, `Mechanics-Sound-Sources.md` and
  `Engineering-Build-Test-Deploy.md` for this campaign — the derived depth
  band, the label ladder, the reveal's end, the muffle as a multiplier, and
  the probe's ten-row evidence table with the measured value that proves
  each row can still fail. **Pushing is outward-facing and is the user's
  call.**
- Two specs are amended in-repo rather than rewritten, since a spec freezes
  what was decided: `2026-08-12-superface-outline-rendering-design.md` (the
  label numbers it named broke their own separation law; the design it
  describes is unchanged apart from anchors becoming face-scoped) and
  `2026-08-10-debug-observability-design.md` (`u_source_floor` no longer
  exists). Plans are left untouched: they record what was planned.
- No NEW spec was written for the audit campaign. If this work continues,
  one under `docs/superpowers/specs/` should freeze the label ladder in
  particular — it is a forced arrangement rather than a taste, and the
  derivation deserves its own page.

## Where to pick up

1. The three items under "What remains", in that order. The first is a
   design trade that wants a playtest rather than a unilateral call.
2. Measure the WEB target with the two probes that now exist — it is the
   last unmeasured platform fact either audit named, and two derivations
   (`CHANNEL_LEVELS`, the depth-texture layer test) are desktop-only.
3. Both audits' journals survive under
   `~/.claude/projects/-home-albatraoz-unseeing/<session>/subagents/workflows/`.
   Verify anything either claims against the code before accepting it:
   between them they were wrong about a refuted platform fact that turned
   out true, and right about four checks that passed while being unable to
   fail.
4. Present the finish-branch choice. Do not merge, push or deploy without
   the user's explicit selection.
