# Closing the renderer's last three gaps — design

Frozen 2026-08-18. Follows the design audit recorded in
`docs/superpowers/handoffs/2026-08-18-rendering-design-audit-state.md`, whose
"What remains" section this supersedes.

Three items were carried rather than fixed. Researched together because two of
them turned out to be different problems from the ones recorded, and the third
turned out to be largely already solved.

---

## Finding 0 — the platform gate is cleared

Both remaining designs depended on a number nobody had measured: how many
levels one data channel preserves on the WEB target.
`tools/measure_web_platform.sh` now answers it. **1024, the same as the
desktop**, with the depth texture live there too (0.9490 against an analytic
0.9508). Measured under SwiftShader, so it is the floor a browser guarantees
rather than a ceiling — and a driver giving FEWER levels is the direction that
would break things, which is what the measurement rules out.

> **Corrected 2026-08-18.** The last sentence is exactly inverted, and the
> paragraph counts the wrong thing. A power-of-two ladder read at seventeen
> bases cannot see a channel that resolves a nominal code at 99.7% of bases
> and not the rest — which is what densifying it found. The web needs **1.02
> nominal codes** to separate (SwiftShader and ANGLE/Apple Metal agree, so
> the SwiftShader caveat is retired); Mesa/AMD desktop GL needs **1.25**.
> The DESKTOP is the worse target, the driver giving fewer usable levels is
> the one already in the room, and it broke the reconstruction guard by
> 4.4 mm at the shipped range — `sight::RECT_SHRINK` is 0.03 m for that
> reason. The depth-texture half of the paragraph stands, and has since been
> confirmed on a GPU-backed browser.

This matters most for Gap 2 below, whose whole design turns on the band
holding hundreds of distinguishable codes rather than eleven.

---

## Gap 1 — the label melt. The recorded diagnosis was wrong.

### What was recorded

> The label allocator's adjacency is a physical-contact relation
> (`TOUCH_EPS = 0.01`) while the image needs a screen-adjacency one … Fixing
> it properly means moving the silhouette test to a scale-relative predicate
> AND growing the adjacency relation together — neither works alone.

### What is actually true

**The magnitude encoding of G buys nothing on screen, and costs the entire
palette.** Every label the game ships is a rung of a ladder 0.09 apart, so
`nrm` is always either exactly 0 or at least 0.09, and
`smoothstep(0.04, 0.08, nrm)` therefore returns exactly 0 or exactly 1 in
every shipped frame. The crease is **already binary**. The eleven-label
ceiling, the graph colouring, the palette starvation reports and the
`MIN_SEP` separation law are all machinery for making a magnitude comparison
behave like an equality test that the shader is already performing.

**The melt is worse than recorded.** Forty shipped solid pairs share a label
and present a depth step under the 0.48 m silhouette knee. The worst is
`EastBarrelA`/`EastBarrelB` at **0.036 m** — smaller than one B quantum
(39.1 mm), so it is not merely under the knee, it is not representable in the
channel at all. `PipeA/B/C` read as one fat pipe. Every four-legged prop in
the game — 2 chairs, 2 tables, 2 benches, a workbench — merges its legs
pairwise whenever you sight along an axis of the leg square.

**"Screen adjacency" is not a well-posed law.** From *some* camera any two
mutually visible solids overlap, so the exact relation is the complete graph.
Restricting it to overlaps that *melt* does not help: at the boundary of any
screen overlap the depth step goes continuously to zero, so every overlapping
pair has a sub-knee sliver somewhere. Any implementable version is a distance
heuristic — `TOUCH_EPS` grown — with no law behind the choice of distance.
And the exact form would need the camera, which the derive-time planner does
not have and cannot get: `CUSTOM0` is baked into `ArrayMesh` vertices once.

**A scale-relative silhouette predicate cannot reach the worst cases either.**
Each B tap carries up to half a code of error, so the Laplacian carries up to
4 codes = 0.156 m of pure noise. The knee can come down perhaps 2×, to
~0.24 m, and no further. The four worst pairs (0.036–0.17 m) are inside the
noise band. The information is not in B and no predicate over B can recover
it.

> **Re-derived 2026-08-18, and the conclusion holds harder.** A tap carries
> up to half of the WORST gap, 0.625 nominal codes, not half of a nominal
> one — so the five-tap Laplacian carries up to 5 codes, **0.196 m** of
> noise rather than 0.156. Everything this paragraph concludes gets worse,
> not better: the knee has less room and the worst pairs sit further inside
> the noise band.


### Decision — re-encode G as an identity

```glsl
float nrm = step(0.5 / 1023.0, abs(c_c.g - c_l.g))
          + step(0.5 / 1023.0, abs(c_c.g - c_r.g))
          + ... ;   // any neighbour differing is a crease
```

Equality is **exactly correct here**, and by construction rather than by luck:

- `screen_tex` is `filter_nearest` (pinned by `shader_contract_test.gd`).
- `CUSTOM0` is piecewise constant per face by construction — a box is built
  with 24 vertices, 4 per face, `vertex_ordinal = vertex / 4`; a column's 12
  vertices per segment split rim/rim/flank so a flank never shares a vertex
  with a rim (`render/paint.rs`). No interpolation gradient exists inside a
  face.
- Nothing can smear it: `msaa_3d = 0`, `screen_space_aa = 0`, `use_taa =
  false`, `use_debanding = false`, `scaling_3d/scale = 1.0`, measured live.

A half-code epsilon rather than a literal `!=`, so future bit noise cannot
manufacture a crease. Centre-vs-neighbour rather than left-vs-right, which
fixes a pre-existing miss: the current form is blind to a one-pixel-wide
sliver of a differing label.

**What identity unlocks.** Labels need only be *distinct*, not `MIN_SEP`
apart. The band `[0.15, 0.96]` at 1024 levels holds ~828 codes, or ~414 at a
safe two-code spacing — against **five** today. The shipped map has 179
superface classes, so every class can hold its own label and the melt
inventory goes to zero **without touching the Laplacian at all**, because the
melts are precisely the small depth steps where B is blind and G is where the
information already was.

> **Corrected 2026-08-18, after the ladder was densified.** The channel does
> not deliver a nominal code everywhere: `render::channel::WORST_STEP_CODES`
> records a widest measured gap of 1.25 nominal codes (Mesa/AMD desktop GL;
> 1.02 on SwiftShader and ANGLE/Metal). So the band holds ~662 reliably
> distinct codes, not ~828, and ~331 at two-code spacing rather than ~414.
> The conclusion is unchanged — 179 classes fit inside 331 with room to
> spare — but any future plan that sizes a palette against 828 is sizing it
> against a number no driver measured.

**And the adjacency question dissolves.** Two distinct classes draw a seam
wherever they meet on screen — touching or 0.4 m apart, it makes no
difference — while coplanar-merged faces keep bit-identical labels and still
melt by construction. The merge law in `render/superface.rs` is untouched.

So the recorded "pair that must land together" is half right: the re-encoding
is the fix, and growing adjacency is not needed at all once it lands. The
scale-relative predicate stays worth doing, but as an independent quality item
for *large* steps between same-class surfaces at distance — not as a
co-requisite.

### Rejected alternatives

| option | why not |
|---|---|
| Grow `TOUCH_EPS` to ~0.45 m | Needs 6–7 world colours against a palette that can gain at most one. At eps > 0.30 a chair becomes a K6 and starves. |
| `TOUCH_EPS` ≈ 0.20 as a stopgap | Fits in 4 colours and does catch the three unfixable-by-B pairs. A heuristic the next authored level breaks; kept only as a fallback if identity is deferred. |
| Lower the absolute silhouette knee to (0.006, 0.015) | Catches 34 of 40 pairs for one constant, but 0.24 m is only 1.5× the worst-case quantisation noise and ~2× the grazing-floor Laplacian. Resolution-dependent, so a knee tuned on a 4K monitor speckles a 720p one. |
| Shrink `DIST_PACK_RANGE` to buy B precision | Unavailable: 39.71 m of required slab diagonal against 40. |

### Residual risk

Graph colouring is **kept**, not deleted — with a large palette and two-code
spacing it never starves (demand is ≤ 9 even at `TOUCH_EPS = 1.0`), and it is
the graceful-degradation path if a platform ever reports fewer codes. The
change is an encoding change, not a removal of the allocator.

---

## Gap 2 — the prop ring cut. The recorded design was the expensive one.

### What is actually true

Measured over the shipped scene on a 0.1 m grid of reachable eye positions:
**1.34 m² of standing area** where the Radio is prop-occluded but wall-clear —
a contiguous wedge you walk through entering the radio room — and **9.66 m²**
for the Fan. At 3.3–4.0 m the pillar covers **100%** of the radio's
silhouette.

The artifact is sharper than "a wash": at those pixels `scene_d` is the
*radio's* 3.3 m, not the pillar's 0.9 m, so every shell root between them is
admitted. A ring interrupted by the pillar everywhere else **continues
through the radio-shaped patch**, and the radio's dim ghost flashes bright as
the ring sweeps. The discontinuity is what reads as broken. Do the same with a
*wall* between you and the radio and the ring is cut dead — props alone leak.

**Nothing the post pass can read answers it.** Exhaustively: B and G hold the
source's values (the source's fragment won the pixel), R is a wall-count
exponent, A is never written but is a write opportunity rather than a read,
and neighbour taps are different rays — useless where the pillar covers the
source completely, which is the whole of the near region.

**The blocker table is the wrong shape.** 106 props against `MAXW = 32`;
125 entries would be **6.6× the hottest loop in the game** — 259 M near-tests
per frame at 1080p — on a dynamically-indexed uniform-array loop, the
construct WebGL2 and mobile drivers handle worst. And a column's bounding
square over-approximates by 41.4% radially, which at 2.3 m is a **35 px false
notch on each side** of every pillar: trading a corner-case leak for a new
artifact everywhere.

### Decision — a per-source CPU visibility flag

`WaveLevel::tick_sources` already walks every source every frame with the eye
in hand and computes a per-source CPU sight answer (`source_muffle`). It
computes one more: is this source occluded from the eye by a prop? The blocked
sources' world boxes go to `post_mat` alone as `u_blocked_src[MAXS]`
(MAXS = 8 → **256 bytes**), and the ring cut becomes
`seen_walled || seen_blocked_source`.

Why this wins:

- **The cost leaves the hot path.** CPU work is bounded by *sources* × *props*
  = 212 tests per frame, unmeasurable. GPU work is bounded by the **source**
  count (2), not the prop count (106, and growing with every crate a designer
  drags in). The table option's cost grows with content; this does not.
- **The shape problem evaporates.** On the CPU the exact cylinder and the
  exact wedge hull are available (`prop_shape::wedge_hull` is right there).
  No circumscribed-vs-inscribed trade, no 35 px notches.
- **It cannot touch `source_muffle`.** Props never enter `self.occluders`, so
  the silhouette muffle law is untouched — which putting props in the shared
  table would have silently broken, dimming every prop-shadowed source to 0.3.
- **It is where this project already puts such decisions**: a pure function
  over immutable inputs in a cargo-tested module, applied by a thin boundary
  adapter, with the verdict pushed as data.
- **ORed, so it cannot regress.** A level that pushes nothing degrades to
  exactly today's behaviour — the same argument the outline cap already banked.

### What it does not fix, stated

- **All-or-nothing per source.** A source half behind a pillar edge flips
  entirely on or off. `source_muffle` already accepts precisely this,
  deliberately, so the precedent stands — but the boundary is where it will
  pop. Mitigation if a playtest dislikes it: test the hub plus a few body
  points and require a majority.
- **The per-pixel over-kill it inherits.** `seen_walled` is fragment-constant,
  so it kills *all* roots at that pixel including a shell a metre in front of
  the player's face. That is a **pre-existing** defect of the wall path, found
  during this research and worth recording: a source seen through a wall
  punches a source-shaped hole in near rings. Fixing it properly means testing
  per-root against the occluder, which is a separate and larger change.

### Taste, not correctness — needs a playtest

Whether a prop-hidden source should cut the ring **completely**, matching a
wall, or merely attenuate it. A wall is a barrier; a pillar is not, and the
fully-cut version may itself read as a hole. **Planned as a knob, not a
constant.**

---

## Gap 3 — the executable oracle. Mostly already built, and misnamed.

### What is actually true

`rust/src/observe/oracle.rs` is not missing so much as **obsolete**. The
2026-08-11 spec designed `expect_lit(walls_between, kind) -> Verdict` on the
premise that player sounds are cut at a wall while a world source is muffled
but not silenced. The barrier campaign deleted `HUM_THROUGH` and made a wall
absolute for every kind, so that function now degenerates to `walls == 0` with
a dead `kind` parameter, and its constants no longer exist.

The comparison it was designed to make — Rust's belief against real pixels —
**is already implemented**, inline, at `occlusion_probe.gd`'s checks 10 and
11: `explain_ray(...)["camera_crossings"] == 1`, then the pixel held to a
hand-derived window `[0.13, 0.30]`. That is the oracle pattern, shipped.

**What is genuinely missing is a scheduler and a host, not an oracle.** The
rot is measurable: two of the eleven checks had silently become unfailable and
were caught only because a human went looking.

### Decision — automate the probe, drop `oracle.rs`

The stated obstacle ("headless CI cannot see shader-reveal leaks") is true of
`--headless`, which admits only the `dummy` driver. It is **not** true of
CI-without-a-GPU. This repository's own `Research-Linux-CI-and-Shipping.md`
already documents the recipe:

```sh
LIBGL_ALWAYS_SOFTWARE=1 dbus-run-session xvfb-run -a godot --path game …
```

and the project pins `gl_compatibility`, so llvmpipe suffices — no Vulkan
stack. `test/web_smoke.sh` already proves a software rasteriser is affordable
here: it runs SwiftShader on every deploy, executing every fragment shader and
screenshotting the result.

Wall-clock cost is roughly fixed rather than proportional to rasteriser speed,
because the probe's windows are durations on the **simulated** clock: a slower
rasteriser collects fewer samples inside the same window, it does not stretch
it.

Two real residual obstacles, neither the stated one: the GitHub workflow runs
`SKIP_EXPORT=1` with a 10-minute cap, and sampling density under llvmpipe
needs the noise floor and the warm-boot-pair law re-validated.

`rust/src/observe/oracle.rs` is **not built**, and the 2026-08-11 spec is
marked superseded.

---

## What this spec does not cover

- Per-root ring occlusion (the pre-existing fragment-constant over-kill).
- The scale-relative silhouette predicate, now independent of Gap 1.
- Re-measuring the web on a GPU-backed browser rather than SwiftShader.

---

## The scale-relative silhouette predicate (design, 2026-08-18)

`hearing_post` decides SHAPE with `smoothstep(0.012, 0.03, lap)`, where `lap` is a
five-tap Laplacian of packed camera distance and `0.012` of full B scale is
`0.012 × DIST_PACK_RANGE` = 0.48 m of world depth step — the same 0.48 m at two
metres and at thirty-five. A silhouette is a screen-space event and a fixed
world-metre threshold is not, so the knee is recorded as wrong in both
directions: too coarse near the hand, too fine far away. Both literals are bare
GLSL that nothing in the tree derives, compares, or guards. This section replaces
them with a knee Rust owns, and replaces the absolute threshold with one that
scales — not with distance, which is the variable the complaint names and the
wrong one, but with the surface's own depth ramp across the stencil, which is the
variable that actually separates a real discontinuity from a smooth surface seen
edge-on.

### The two halves of the complaint are not symmetric

**"Too coarse near the hand" is unreachable, and the reason is arithmetic.** Each
B tap carries up to half a reliable channel gap (`channel::recon_eps`, 0.024438 m
at the shipped range), and the five-tap stencil's weights sum to 8 in absolute
value, so a perfectly flat wall can hand `lap` **0.0048876 of full scale =
0.19550 m** out of rounding alone, at every distance, because quantisation noise
is constant in metres. The shipped 0.012 clears that by 2.4552×. A floor below
about 0.39 m would spend the entire margin; below 0.196 m the predicate
manufactures creases out of rounding. There is no fourth data channel
(RGB10_A2's alpha is 2 bits, and naming `ALPHA` ejects the material to the alpha
queue where the screen texture is gone), and `DIST_PACK_RANGE` cannot shrink
(39.73 m of slab diagonal against 40). **No predicate over B can resolve a step
under ~0.2 m anywhere in the frame.** The near half of the complaint is closed as
unreachable, not addressed.

**"Too fine far away" is real but mis-attributed.** Sweeping the reachable
geometry, the only false positive above the floor that is not quantisation is
*perspective curvature on a grazing plane*: a flat surface at perpendicular
standoff `p`, seen at distance `d`, hands the stencil a Laplacian of
`2α²d³/p²` where `α = 2·tan(fov/2)/rows`. At 720 rows, fov 66°, and the closest a
wall plane can be grazed (the player capsule's 0.35 m radius), that crosses the
0.48 m floor at **20.75 m** and reaches **3.55 m** of apparent depth step at
40 m — a full-strength white band converging on the vanishing point with no
geometry in it. Floors and ceilings never reach it (0.213 m at 40 m for the
1.4 m ceiling clearance, 4.5× under the floor), and neither does the curvature of
any convex prop: a smooth body's own Laplacian never reaches the floor before its
limb, where the limb is a genuine discontinuity.

So the far-field defect is not "things far away are judged too finely". It is
"surfaces raked near-tangent are judged as if they were face-on". Distance is a
poor proxy for that — it is bad at both ends, dimming face-on far geometry that
has no defect while under-tracking the grazing artefact, which grows as `d³`
against a linear arm's `d¹`.

### The predicate

Replaces `hearing_post.gdshader` lines 158–168 and the x-ray cap gate at 176.
`lap`, `nrm`, `crease`, `reveal`, `detail` and the final composition are
untouched.

```glsl
// SHAPE's knee. rust/src/render/silhouette.rs owns all four numbers.
//   .x floor — the smallest step B can PROVE is real, in packed units.
//              Under it the Laplacian is quantisation, and any relative rule
//              that fired there would draw rounding as geometry.
//   .y cap   — the most of the local depth ramp a step may be asked to beat.
//              Bounded ABOVE so an isolated step can never raise the knee
//              past its own height, and BELOW so a raked plane cannot slip
//              under it.
//   .z plane — the margin on the plane identity below.
//   .w span  — hi = lo * span; held as a RATIO, which is what makes raising
//              the knee provably unable to invent a line (theorem below).
// The default is UNREACHABLE rather than merely wrong: lap <= 4.0 exactly,
// because B is clamp(vd/DIST_PACK_RANGE, 0, 1) and the stencil's positive
// weights sum to 4. An unpushed post material therefore draws no shape at
// all while creases and rings carry on — visibly broken, in the dim
// direction, exactly as u_crease_knee's and u_detail_knee's defaults are.
uniform vec4 u_sil_knee = vec4(4.0, 0.0, 0.0, 2.5);

float lap  = abs(c_l.b + c_r.b + c_u.b + c_d.b - 4.0 * c_c.b);
float nrm  = abs(c_l.g - c_r.g) + abs(c_u.g - c_d.g);
// The same stencil as nrm, on B: the surface's own depth ramp across the
// stencil, in packed units. Zero on anything locally symmetric — a thin
// bar, a symmetric corner — and equal to lap on an isolated step.
float ramp = abs(c_l.b - c_r.b) + abs(c_u.b - c_d.b);
// Nearest of the five taps, clamped by the floor. The clamp needs only a
// validated positive lower bound and the uniform already carries one; a
// surface nearer than the floor's 0.48 m only ever gets a GENTLER arm, and
// the division can never see zero. B == 0 is the cleared background, and
// clamping it here puts the void on the permissive branch, which is what an
// object outlined against nothing needs.
float bnc  = max(min(min(min(c_l.b, c_r.b), min(c_u.b, c_d.b)), c_c.b), u_sil_knee.x);
// A plane's Laplacian is exactly ramp*ramp/(2*bnc) in packed units, to
// within 2-4% over the whole reachable domain, so that expression IS the
// arm: it raises the knee to what a smooth surface would have produced
// here. u_sil_knee.y caps it, because for an isolated step ramp == lap and
// an uncapped quadratic arm would suppress every step larger than about
// twice its viewing distance.
float sil_lo = max(u_sil_knee.x,
        ramp * min(u_sil_knee.y, u_sil_knee.z * ramp / (2.0 * bnc)));
float sil    = smoothstep(sil_lo, sil_lo * u_sil_knee.w, lap);
float crease = smoothstep(u_crease_knee.x, u_crease_knee.y, nrm);
```

`float edge = max(sil, crease);` is **deleted** and the cap's gate written out
against the UNGROWN floor, so the x-ray cap fires on exactly the pixels it fires
on today (`smoothstep(lo, hi, x) > 0` iff `x > lo`, and the ungrown low end is
`u_sil_knee.x`):

```glsl
if (lap > u_sil_knee.x || nrm > u_crease_knee.x) {
```

Asking the *grown* `sil` would silently narrow the cap on raked surfaces, where an
x-rayed source's outline would start borrowing the brightness of the lit wall
behind it again — the defect the cap exists to close.

Cost on the hot path: one four-way `min`, two `abs`-differences, one `min`, three
multiplies and one division, on taps already in registers. No new texture fetch,
no `VIEWPORT_SIZE`, no `PROJECTION_MATRIX`, no camera term.

**The safety theorem.** For `L ≥ F > 0`, `S > 1`, `x ≥ 0`:
`smoothstep(L, LS, x) ≤ smoothstep(F, FS, x)`, because `(x−L)/(L(S−1)) ≤
(x−F)/(F(S−1))` reduces to `Fx ≤ Lx`. `sil_lo ≥ floor` for every possible tap
tuple (the arm is a product of non-negatives), and `lap ≥ 0` always, so **the
change can only ever erase a silhouette, never manufacture one** — on any driver,
at any resolution, at any FOV, under any uniform the validated type admits. No new
speckle is reachable by construction. That property is why the span is held as a
ratio rather than an additive width, and it is the property the pure-relative and
gradient-normalised families cannot offer.

**The invariance theorem.** For an isolated step of height `s` at distance `d`,
the operator gives `ramp = lap = s/R` exactly, on both sides of the edge and for
horizontal, vertical and diagonal edges alike. So the arm is at most `K·lap`, and
the response is unchanged iff either the floor still governs (`K·lap ≤ floor`) or
the grown knee still saturates (`lap ≥ span·K·lap`, i.e. `K ≤ 1/span`). Carrying
worst-case quantisation through both arms, the two conditions cover the whole
domain iff

> `K ≤ floor / (lap_noise + floor·span + ramp_noise)` = **0.321445**

which is where `SIL_CAP`'s upper bound comes from. Swept exhaustively over 160 000
(distance, step) pairs from 0.4 m to 40 m and 0.1 m to 40 m, **zero isolated steps
change response**. The same holds for a one-pixel bar (`ramp = 0`), a symmetric
corner (`ramp = 0`), and any object outlined against the cleared background.
Every pixel this design costs is on a monotone depth ramp; nothing else can move.

### Constants

| symbol | value | units | hand-derivation |
|---|---|---|---|
| `SIL_FLOOR` | 0.012 | fraction of full B scale (0.480 m at range 40) | Grandfathered from the shipped literal so no pixel off a raked surface moves — a change of ownership and a change of behaviour in one commit is two things to debug at once. Now **checked** rather than chosen: `SIL_FLOOR / laplacian_noise_fraction()` = 0.012 × 1023 / 5 = **2.4552**, so the knee opens at 2.46× the widest Laplacian a flat wall can manufacture. Range-independent: floor and noise both scale with `DIST_PACK_RANGE`. |
| `SIL_SPAN` | 2.5 | dimensionless | Grandfathered: 0.03 / 0.012. The invariant is the *ratio*, not the width — the arm multiplies both ends, so the fade keeps its proportion instead of becoming a sliver, and the safety theorem needs exactly this. |
| `channel::LAP_WEIGHT_SUM` | 8 | dimensionless | `|−4| + 4·|1|`, the L1 norm of the stencil the pass evaluates. |
| `channel::RAMP_WEIGHT_SUM` | 4 | dimensionless | `2 × |±1|` per axis, two axes — the `ramp` stencil, which is the `nrm` stencil applied to B. |
| `channel::laplacian_noise_fraction()` | 4.887586e-3 (= 5/1023) | fraction of full B scale; **0.195503 m** at range 40 | `LAP_WEIGHT_SUM × recon_eps(1)` = 8 × 0.5 × `WORST_STEP_CODES` / (`CHANNEL_LEVELS` − 1) = 8 × 0.5 × 1.25 / 1023. Reuses the existing `recon_eps`, so the reconstruction guard and the silhouette guard cannot drift apart. One nominal code is 40/1023 = 39.10 mm; one reliable gap is 48.89 mm. |
| `channel::ramp_noise_fraction()` | 2.443793e-3 | fraction of full B scale; 0.097752 m | `RAMP_WEIGHT_SUM × recon_eps(1)`. Half the Laplacian's, because half the weight. |
| `channel::max_worst_gap(SIL_FLOOR)` | 3.0690 | nominal 10-bit codes | `floor × 2 × (LEVELS−1) / LAP_WEIGHT_SUM` = 0.012 × 2 × 1023 / 8. The widest channel gap the floor survives, against **1.25** measured on Mesa/AMD desktop GL and 1.02 on SwiftShader and ANGLE/Apple Metal. |
| `REF_ROWS` | 720 | viewport lines | `game/project.godot` `window/size/viewport_height`. The lowest resolution the project declares, and therefore the one the cap is derived at: the artefact it must reject shrinks as 1/rows², so deriving at the smallest declared height makes every larger display strictly safer. |
| `REF_FOV` | 66.0 | degrees, VERTICAL | `nodes::player` `camera.set_fov(66.0)`. Godot's default KEEP_HEIGHT makes it vertical, which is why aspect never enters the derivation; square pixels make the horizontal step identical. |
| `PX_TAN` | 1.8039100e-3 | tangent per pixel | `2·tan(REF_FOV/2)/REF_ROWS` = 1.2988152 / 720. |
| `CAPSULE_R` | 0.35 | m | `nodes::player` `capsule.set_radius(0.35)`. The camera rides the capsule axis, so this is the closest the eye can be to any collidable plane — the standoff that maximises the grazing Laplacian. Props are **not** bounded by it; see limits. |
| worst reachable plane | lap 0.0887676 B (3.5507 m), ramp 0.4305747 B (17.2230 m) | B units | Exact three-tap second and first differences of the plane distance field `d(x) = p·√(1+x²)/x` at `x = p/√(d²−p²)`, step `PX_TAN`, evaluated at `d` = `DIST_PACK_RANGE` = 40 m, `p` = `CAPSULE_R`. The closed forms `2α²d³/p²` and `2αd²/p` agree to 4.4% and 0.1%; the exact discrete values are the ones the law uses. |
| `SIL_CAP` lower bound | 0.218754 | dimensionless | `(lap + lap_noise) / (ramp − ramp_noise)` at the worst reachable plane. Below this the cap lets the vanishing-point band through. |
| `SIL_CAP` upper bound | 0.321445 | dimensionless | `floor / (lap_noise + floor·span + ramp_noise)` = 0.012 / 0.0373314, the closed form of the invariance theorem. Above this an isolated step can be dimmed by adversarial quantisation. |
| **`SIL_CAP`** | **0.265174** | dimensionless | Geometric mean of the two bounds, so the two failure modes — which are multiplicative in the cap — carry **equal margins of 1.2122×**. The balance rule is AUTHORED and says so; the two bounds are not. |
| `SIL_PLANE` (M) | 2.0 | dimensionless | Margin on the plane identity. The requirement is closed form: at the floor's own boundary a plane's true Laplacian is `floor − lap_noise`, and worst-case quantisation can present it as `floor`, so `M ≥ floor/(floor − lap_noise)` = 0.012 / 0.0071124 = **1.687191**. A sweep of the exact discrete operator over rows ∈ {480…2160}, `p` ∈ [0.05, 4.0] m and `d` ∈ (p, 40] m peaks at 1.6504, below the closed form because the discrete Laplacian under-runs `ramp²/(2b)` by 2–4%. Shipped at 2.0, a margin of **1.1854×**. |
| minimum viewport | **559.8 → 560** | rows at fov 66 | Solve `plane_ratio(40 m, 0.35 m, α) = SIL_CAP` for α, then `rows = 2·tan(33°)/α`. Below it the grazing band partially survives. `project.godot` ships 720 (1.29× margin) and desktop boots at native resolution; the web canvas is the browser window and is not bounded — hence `viewport_budget`, read at boot. |
| shader default | `vec4(4.0, 0.0, 0.0, 2.5)` | — | `lap ≤ 4.0` exactly (B ∈ [0,1]; positive stencil weights sum to 4), so `smoothstep(4.0, 10.0, lap)` is 0 for every reachable tuple — no shape at all, dim direction. Deliberately a tuple `SilKnee::new` *accepts*, so Rust models the broken default exactly rather than having no reading of it. |

### What ships, in numbers

Grazing plane, response today → response after, at 720 rows:

| standoff | 19.4 m | 22 m | 26.8 m | 30 m | 40 m |
|---|---|---|---|---|---|
| 0.35 m (capsule) | 0.000 → 0.000 | 0.046 → **0.000** | 0.877 → **0.000** | 1.000 → **0.000** | 1.000 → **0.000** |
| 0.70 m | 0.000 → 0.000 | 0.000 → 0.000 | 0.000 → 0.000 | 0.000 → 0.000 | 0.539 → **0.000** |
| 1.60 m (floor) | 0.000 → 0.000 | 0.000 → 0.000 | 0.000 → 0.000 | 0.000 → 0.000 | 0.000 → 0.000 |

Isolated steps: **unchanged at every distance and every size** (160 000-pair
sweep, zero changes). Thin bars, symmetric corners and anything outlined against
the cleared background: unchanged.

The entire measured cost, over the same sweep — a step sitting *on* a wall raked
at 0.35 m, at 720 rows:

| | 0.5 m step | 1.0 m step | ≥ 2 m step |
|---|---|---|---|
| at 10 m, 15 m | unchanged | unchanged | unchanged |
| at 19.4 m | 0.607 → 0.164 | unchanged | unchanged |
| at 26.8 m | 1.000 → 0.000 | 1.000 → 0.038 | unchanged |

The 26.8 m row is misleading in the design's favour and is worth stating plainly:
at that point today's law already paints the whole wall at 1.000 out of its own
curvature, so those two "losses" are lines that carried no information. **The one
honest cost in the shipped content is the 19.4 m / 0.5 m cell: 0.607 → 0.164.**
That surface's own depth ramp there is 3.92 m across the stencil, so a 0.5 m step
is an eighth of what one stencil already spans on the surface it interrupts — at
the sampling limit, and the same magnitude as the false line the same law draws
2 m further along.

### Where the Rust law lives

**Module.** `rust/src/render/silhouette.rs`, a peer of `channel`, `crease`,
`detail` and `depth` — `depth.rs` and `crease.rs` are the established precedent
for a module owning a shader-facing derived quantity together with its
derivation. Registered in `render/mod.rs`. It depends on `render::channel`,
`level_plan::{DIST_PACK_RANGE, Budget, Severity}`; nothing depends on it but the
composition root.

**Type.** `pub struct SilKnee { floor: f32, cap: f32, plane: f32, span: f32 }` —
narrowed to f32 at construction because f32 is what reaches the GPU, the same
reason `CreaseKnee` and `DetailKnee` store narrowed lanes and judge ordering
*after* narrowing.

**Total functions and their complete input domains.**

- `SilKnee::new(floor: f64, cap: f64, plane: f64, span: f64) -> Option<Self>`.
  Domain: the whole of f64⁴, NaN and both infinities included. Narrows all four
  to f32 **first**, then `Some` only if: all finite; `floor > 0` (a zero floor
  makes the near clamp zero and lets a future form divide by it); `cap ≥ 0` and
  `plane ≥ 0` (zero is legal for both and *is* today's absolute law, which makes
  the old behaviour a constructible fixture and the degradation target);
  `span > 1` after narrowing (`span == 1` is GLSL's division by zero, `< 1`
  inverts the fade); and the **reachable-maximum guard** — `ramp ≤ 2.0` exactly
  and `bnc ≥ floor`, so the largest low end the shader can compute is
  `lo_max = max(floor, 2·min(cap, plane/floor))`, and `(lo_max × span) as f32`
  must be finite. Without that last clause a pair like `(2e38, …, 3e38)` passes
  every other test in f64 while GLSL computes `inf − inf` and paints NaN across
  the far field. The f32-collapse case is load-bearing too: `(0.012, …, 1.0 +
  1e-9)` is strictly ordered in f64 and lands in one f32 lane.
- `SilKnee::from_geometry(floor, span, range, rows, fov_deg, standoff, gap_codes)
  -> Option<Self>` — the shipped derivation: `cap = sqrt(cap_lower × cap_upper)`,
  `plane = PLANE_MARGIN`. Domain: the whole of its argument space; `None` for any
  non-finite, `rows == 0`, `fov ∉ (0, 180)`, `standoff ≤ 0`, `range ≤ standoff`,
  `gap_codes ≤ 0`, or an empty bracket (`cap_lower ≥ cap_upper`).
- `SilKnee::shipped() -> Self` = `from_geometry(...)` with the **valid** fallback
  `{ floor: 0.012, cap: 0.0, plane: 0.0, span: 2.5 }` rather than a panic — which
  is exactly today's absolute law, so a degenerate derivation degrades to the
  known-safe shipped picture and not to a broken one. `CreaseKnee::shipped`'s
  precedent; this crate does not panic.
- `SilKnee::lo_at(self, ramp: f64, bnear: f64) -> f64` and
  `SilKnee::response(self, lap: f64, ramp: f64, bnear: f64) -> f64` — the GLSL
  written in Rust, and the mirror the rendered probe holds real pixels against.
  Total over the whole of f64³: non-finite inputs answer the floor pair and 0.0
  respectively, `ramp` is clamped to [0, 2] and `bnear` to [0, 1] (the domains
  `data_core::pack_data` can actually deliver), and the smoothstep parameter is
  clamped with explicit comparisons rather than `f64::clamp`, which propagates
  NaN. Neither can return NaN, an infinity, or a value outside [0, 1].
- `plane_taps(distance, standoff, alpha) -> Option<[f64; 3]>` — the exact three
  samples of `p·√(1+x²)/x`. Uses `hypot` and refuses `distance ≤ standoff`, so
  the `d² − p²` cancellation that produces `inf/inf = NaN` one ulp above the
  standoff is unrepresentable. `None`, never NaN: a guard built on it must FAIL
  rather than go silent, and NaN makes every comparison false.
- `pixel_tan(fov_deg, rows) -> Option<f64>`, `plane_ratio(...) -> Option<f64>`,
  `cap_lower(...) -> Option<f64>`, `cap_upper(floor, span, range, gap_codes)
  -> f64` (the closed form), `plane_margin_required(floor, range, gap_codes)
  -> f64` (`floor/(floor − laplacian_noise)`), `min_rows(...) -> Option<f64>`.
- `floor_budget(floor, range) -> Option<Budget>` — peer of
  `channel::reconstruction_budget` and pointed the other way: that one is about
  the range outgrowing the channel, this one about the knee sinking into it.
  `Severity::Error` if the floor is non-finite, ≤ 0, or ≤ `laplacian_noise`;
  `Warn` below twice it; `None` otherwise. The shipped 0.012 is silent at 2.4552×.
- `cap_budget(...) -> Option<Budget>` — `Error` when the bracket is empty, `Warn`
  when either margin falls under 1.1×, naming both bounds and the constant that
  closed them.
- `viewport_budget(rows: u32) -> Option<Budget>` — `Warn` below `min_rows` at the
  shipped fov, range and standoff. Domain: every `u32`; `rows == 0` warns rather
  than going silent. Emitted **at boot from the real viewport height**, because
  the derive-time planner does not know the monitor and the web canvas is the
  browser window; this is the runtime portability check the new-object checklist
  asks for, not a compile-time one.

**Purity and where dependencies enter.** Nothing above reads the scene tree, a
clock, a viewport, a random source, or a static. The FOV, reference height and
capsule radius enter as constants declared here and cross-checked against their
owners by test rather than read from Godot at runtime — deliberately, so that what
the hero perceives is an authored property of the game and not a property of the
window manager. The only boundary is
`nodes::game::UnseeingGame::ready`, the same composition root that already pushes
`u_crease_knee`, `u_detail_knee` and `u_presence`, doing one push of
`Vector4::new(k.floor() as f32, k.cap() as f32, k.plane() as f32, k.span() as
f32)`. Validated before it reaches the GPU, so the push cannot deliver a division
by zero, an inverted fade, or an overflowing product. `WaveCore` gains
`#[func] fn silhouette_knee() -> Vector4` and `#[func] fn max_worst_gap() -> f64`
beside the existing `crease_knee()`, so the gdUnit suites hold the pushed uniform
against the derivation rather than a retyped literal. No per-frame push, no new
state.

### The test plan

1. **cargo `a_knee_the_shader_cannot_evaluate_is_refused`.** BREAK: an
   unevaluable knee reaching the GPU, where GLSL divides by `hi − lo`, an
   inverted pair fades a bright edge dark, and an overflowing product paints NaN
   across the far field. Cases: `span` 1.0 / 0.5, `floor` 0.0 / −0.012, `cap`
   −0.01, `plane` −1.0, NaN and both infinities in each of the four slots, the
   f32-collapse `span = 1.0 + 1e-9`, and the overflow case
   `(2e38, 0.3, 2.0, 3e38)` — finite in f64, `inf − inf` in GLSL.
2. **cargo `the_floor_clears_the_laplacians_own_quantisation_noise`.** BREAK:
   someone lowering the floor to catch the 0.036 m `EastBarrelA`/`B` pair, after
   which rounding draws as creases on every flat wall. Hand-derived, not read
   back: 8 × 0.5 × 1.25 / 1023 = 5/1023 = 4.887586e-3 = 0.195503 m; ratio 2.4552;
   `max_worst_gap(0.012)` = 3.0690 codes. `floor_budget` is `None` at 0.012,
   `Warn` at 0.0090, `Error` at 0.0048, 0.0 and NaN. Mutating `LAP_WEIGHT_SUM`
   8 → 4 or `WORST_STEP_CODES` 1.25 → 1.0 moves both literals.
3. **cargo `the_arm_can_only_ever_raise_the_knee`.** BREAK: a future edit — a
   `min` for the `max`, a subtraction, an additive span, a negative cap — letting
   the arm lower the knee somewhere, which would put new speckle on a platform
   nobody tested. The safety theorem swept rather than argued: over `ramp` on a
   0..=200 grid of [0, 2], `bnear` on a 0..=100 grid of [0, 1] and `lap` on a
   0..=400 grid of [0, 4], `shipped().response(...) ≤ new(0.012, 0.0, 0.0,
   2.5).response(...)` and `lo_at(...) ≥ floor()`. Two different laws compared
   across their complete domain — not a mirror assertion.
4. **cargo `an_isolated_step_is_untouched_at_every_distance_and_size`.** BREAK:
   the cap drifting above its bound, after which the change stops being confined
   to raked surfaces and starts deleting ordinary far shape — the failure that
   sank the distance-keyed family. Asserts `ramp == lap` exactly for the ideal
   step tuple at horizontal, vertical and diagonal edges; asserts the closed form
   `cap_upper = floor/(lap_noise + floor·span + ramp_noise)` = 0.321445
   hand-derived; then sweeps 160 000 (d, s) pairs from 0.4–40 m and 0.1–40 m and
   requires **zero** response changes. Mutating `SIL_CAP` to 0.35 fails it.
5. **cargo `the_worst_reachable_grazing_plane_is_rejected_inside_the_range`.**
   BREAK: the artefact this design exists to remove coming back, quietly, when
   the range, the FOV or the reference height moves. Hand-derived at 720 rows,
   fov 66, `p` = 0.35 m: lap 0.0887676 B, ramp 0.4305747 B, ratio-with-noise
   0.218754; response 0.877 → 0.000 at 26.8 m and 1.000 → 0.000 at 30 and 40 m;
   and the milder standoffs 0.7 m and 1.6 m unchanged or improved.
6. **cargo `neither_rejected_family_is_reachable_from_here`.** BREAK: the two
   wrong shapes being re-proposed, each of which passes casual inspection. Kept
   as executable counter-examples on the same tuples: an **uncapped** quadratic
   arm suppresses a 5 m step at 2 m (a doorway with a room behind it) while the
   shipped composed arm does not; and a **distance-keyed** arm `max(floor,
   floor·grow·bnear)` drops an isolated 1.0 m step at 35 m from 1.000 to 0.044
   while the shipped law leaves it at 1.000.
7. **cargo `a_bar_a_corner_and_the_void_never_raise_the_knee`.** BREAK: the
   `ramp` stencil being rewritten as `abs(l + r − 2c)` or the clamp being
   dropped, after which the most fragile features in the game — a one-pixel
   sliver, a symmetric corner, an object against cleared black — start being
   judged against a threshold built from their own signal. Asserts `ramp == 0`
   exactly for the bar and the symmetric corner, `ratio == 2.0` for an asymmetric
   bend, and unchanged response for all three plus the void tuple.
8. **cargo `a_degenerate_derivation_is_refused_rather_than_waved_through`.**
   BREAK: a NaN or infinity reaching the GPU from a degenerate constant, where
   GLSL's `smoothstep` on a NaN is implementation-defined and one bad pixel can
   draw a line across the frame. `pixel_tan` at fov 0, 180, NaN and rows 0;
   `plane_taps` at `distance == standoff`, one ulp above it, at
   `x > 1.34e154` where `1 + x²` overflows, and at every non-finite; `response`
   and `lo_at` on NaN, ±∞ and out-of-range in each slot, asserting finite results
   in [0, 1]; and `shipped()` valid under every one of them, degrading to
   `cap = plane = 0` — today's law — rather than to a broken one.
9. **cargo `the_bracket_has_room_and_the_module_says_where_it_closes`.** BREAK:
   the bracket silently closing when a constant moves, leaving a `cap` that
   satisfies neither bound. Hand-derived: bracket [0.218754, 0.321445] at the
   shipped settings, margins 1.2122× each way; `min_rows` = 559.8; the bracket
   closes below 560 rows and above a worst gap of 4.2380 codes, while the floor
   itself fails at 3.0690 — so the **floor**, not the arm, is the binding
   platform constraint. `cap_budget` silent at 720 rows, `Warn` at 600,
   `Error` at 480; `viewport_budget` silent at 720 and 1080, `Warn` at 480 and 360.
10. **gdUnit `shader_contract_test.gd::test_the_silhouette_knee_is_the_one_rust_derived`.**
    BREAK: the GLSL drifting from the Rust law, or a literal knee being
    re-hardcoded, with every cargo test green because Rust would go on deriving a
    number nothing reads — bit for bit the `MIN_SEP`/crease-knee drift this
    repository already shipped once. Pins `uniform vec4 u_sil_knee = vec4(4.0,
    0.0, 0.0, 2.5);` and the four predicate lines (`ramp`, `bnc`, `sil_lo`,
    `sil`) as source text; asserts the retired `smoothstep(0.012, 0.03, lap)` is
    **absent** (it is currently pinned at line 436 and that assertion must move);
    asserts `vec3 col = vec3(max(sil * reveal, crease * detail * reveal));` is
    untouched so the SHAPE/DETAIL split survives; and compares
    `WaveCore.silhouette_knee()` against the hand-derived
    `Vector4(0.012, 0.265174, 2.0, 2.5)`.
11. **gdUnit `shader_contract_test.gd::test_the_outline_cap_still_fires_on_the_pixels_it_always_did`.**
    BREAK: the x-ray cap narrowing on raked surfaces because it started asking the
    grown `sil`, after which a far source's outline borrows the brightness of the
    lit wall behind it again. Pins `if (lap > u_sil_knee.x || nrm >
    u_crease_knee.x) {` and the absence of `float edge = max(sil, crease);`.
12. **gdUnit `wiring_test.gd::test_the_silhouette_knee_reaches_the_post_pass`.**
    BREAK: the knee derived in Rust and never pushed, so the deliberately-wrong
    default ships and the game draws no shape at all while every source-text test
    passes. Reads `main.post_mat.get_shader_parameter("u_sil_knee")` back, holds
    it against `WaveCore.silhouette_knee()`, and asserts it is not
    `Vector4(4.0, 0.0, 0.0, 2.5)`.
13. **gdUnit probe `game/tests/probe/silhouette_knee_probe.tscn`.** BREAK:
    everything above being true of Rust's belief and false of the shipped shader —
    a driver that ignores `filter_nearest`, a compiler that reassociated the
    `min` chain, a knee that never reached the material. `explain_ray` reports
    Rust's belief and cannot prove GLSL agrees. One frame, grain forced to 0 and
    the reveal held at a known standing floor: (a) a quad raked to the worst
    reachable plane geometry — zero pixels above the outline threshold anywhere in
    its interior, where today's law paints a band; (b) an isolated 1.0 m step at
    25 m reading within tolerance of `SilKnee::response`'s hand-derived value.
    **Both quads carry the same label bit-for-bit**, so the crease term cannot
    supply the line the probe is attributing to `sil`, and every tolerance is
    widened by the channel's own worst gap (a nominal 0.60 m step is delivered as
    0.55–0.65 m) rather than asserted to a precision B cannot carry.
14. **gdUnit `platform_probe.tscn` gains `measured worst gap <
    WaveCore.max_worst_gap()`.** BREAK: a driver whose worst local gap exceeds
    3.0690 nominal codes, where flat walls speckle and not one cargo test can see
    it, because `floor_budget` checks the constants and never the hardware. The
    ladder already measures the gap at every base of a swept column; this is the
    assertion that turns the measurement into a gate.
15. **gdUnit `platform_probe.tscn` gains a flat-wall Laplacian measurement.**
    BREAK: the half-gap-per-tap model being wrong — see *Unresolved* below. Reads
    the actual five-tap Laplacian and two-axis ramp off a large flat wall at
    several distances, through the amplification trick `channel_probe` already
    uses, and asserts them under `channel::laplacian_noise` and
    `channel::ramp_noise`. This is the one measurement that converts the
    campaign's central inherited assumption into a fact.

**Deliberate mutation evidence**, each mutation named with the test that must
fail: `SIL_CAP` 0.2652 → 0.35 fails 4; → 0.15 fails 5; `SIL_PLANE` 2.0 → 1.0
fails 5; `max` → `min` in `sil_lo` fails 3; `ramp` → `lap` fails 4; the `bnc`
clamp dropped fails 8 (division by zero on the cleared background); `2.0 * bnc` →
`bnc` fails 5 and the probe; `SIL_FLOOR` → 0.0060 fails 2; `LAP_WEIGHT_SUM`
8 → 4 fails 2; `WORST_STEP_CODES` → 1.0 fails 2 and 9; `REF_ROWS` 720 → 1080 or
`CAPSULE_R` 0.35 → 1.0 moves the bracket and fails 9; the shader default set to
the shipped tuple fails 10 and 12.

### What this design cannot do

1. **It does not fix the near field, and nothing can.** Below the floor the knee
   is bit-for-bit today's 0.48 m, which at 2 m is 24% of the viewing distance. A
   0.2 m crate lip at arm's length draws no silhouette after this change, exactly
   as before it. The Laplacian's quantisation noise is 0.195503 m *at every
   distance*, so the lowest honest floor is about 0.39 m — a 1.23× improvement at
   most, bought by spending the entire margin. The information is not in B.
2. **It catches none of the melt inventory.** The 40 same-label pairs with
   sub-knee depth steps are untouched; the worst, `EastBarrelA`/`B` at 0.036 m,
   is under one nominal code (0.0391 m) and so is not representable in B at all.
   The arm only ever raises the threshold, so it catches exactly zero of them.
   Gap 1's identity re-encoding of G remains the only fix and this must never be
   quoted as a substitute for it.
3. **It cannot separate a step from a ramp when the two are comparable.** A step
   of `s` on a surface whose stencil already spans `ρ` of depth gives
   `lap = |s − ρ/2|` and `ramp = ρ + s`; when `s` approaches `ρ/2` the Laplacian
   vanishes and *no* second-difference operator can see the step, this one or
   today's. Where `s` is above that but still a fraction of `ρ`, this design
   answers "ramp" and today's answers "step". That is a choice between two errors,
   not a repair: today's law pays with a full-strength false line on every raked
   surface past 20.75 m, and this one pays with the 19.4 m / 0.5 m cell above.
4. **It has no normal, so it bounds rather than discriminates.** A predicate over
   B alone cannot distinguish a plane at 89° from a real step of the same
   magnitude; the plane identity `ramp²/(2·bnear)` is an upper bound on the
   former, deliberately conservative. A predicate that genuinely discriminates
   would fit a local plane before differencing — 8+ taps, a second threshold, and
   a composition that is **not** monotone in the current law, so it could light
   pixels today's law leaves black and could not be validated by argument at all.
   That is a different design and it should be written as one.
5. **`CAPSULE_R` bounds walls, not props or the viewmodel.** The capsule keeps the
   eye 0.35 m from any collidable plane; a prop face, or the hero's own body at
   0.277 m (`nodes::hero`'s torso cap sphere: 0.20 m back and 0.32 m down from the
   eye, radius 0.10), can be nearer. The plane identity itself is standoff- and
   resolution-**independent** (`lap`, and `ramp²/(2d)`, both scale as `α²/p²`), so
   the quadratic branch rejects a plane at any standoff; only the cap carries the
   0.35 m assumption, and only where `ramp` exceeds `2·cap·bnear/plane`. A plane
   at 0.1 m standoff seen beyond ~15 m at 720 rows is outside the guarantee.
6. **It does not touch anything else.** G (creases, `u_crease_knee`), R (detail,
   `u_detail_knee`), the superface merge law, `MIN_SEP`, the graph colouring, the
   ring cut and the prop table are all unchanged. `render::memory` does not
   currently compose into this term; if it later lands as
   `max(sil · max(reveal, trace), …)`, the knee applies to `sil` identically and
   nothing here reads `reveal`.
7. **The benefit's *content* is unmeasured even though its *mechanism* is not.**
   Nobody has traced how many pixels of the shipped map are raked hard enough to
   produce the band. The four border walls are 26.8 m and are crossed by interior
   walls; one review put the longest clear wall-hugging run at 19.4 m, which this
   pass did not independently confirm. At 19.4 m the artefact does not fire today
   (0.392 m against a 0.48 m floor); at 26.8 m it fires at 0.877. **This design
   does not depend on which is true** — it rejects the plane at every distance
   inside the packing range — which is precisely the property the distance-keyed
   alternative lacked, since that one needed the census to justify its constant.

### Platform and resolution degradation

**A wider channel gap.** Both noise terms are linear in `WORST_STEP_CODES`, so
both guards are derived predicates rather than remembered numbers. The floor
clears its own noise up to a worst gap of **3.0690** nominal codes; the cap's
bracket does not close until **4.2380**. So the floor is the binding constraint
and the arm is not the weak link: against 1.25 measured on the worst of three
drivers, the margin is 2.46×, and 3.01× against the 1.02 the web delivers. Past
3.0690 codes, flat walls speckle at every distance simultaneously (the noise is
constant in metres, so there is no "safe near field" to lose last), the safety
theorem still holds — the picture is still a subset of today's — and
`floor_budget` reports the crossing at level derivation. Nothing at runtime
detects a driver that has gone bad; test 14 is what turns the existing
`platform_probe` measurement into a gate, and without it a worse driver is
invisible to every test in the tree and shows up first as a player saying the
walls look grainy.

**Fewer channel levels.** At 512 the Laplacian's noise is 9.775e-3 B, above the
shipped floor's fade start, and no knee exists that both clears the noise and
draws a room. That is a different renderer, not a degradation, and
`CHANNEL_LEVELS` is measured rather than assumed precisely so it is discovered by
a probe.

**Resolution.** The predicate is resolution-**independent** where it matters and
this is deliberate: for an isolated step `ramp == lap` at any pixel count, and the
plane identity `ramp²/(2·bnear)` scales as `α²` exactly as the plane's Laplacian
does, so the quadratic branch rejects a raked plane at **every** resolution. The
one resolution dependence is the cap, and it is one-sided: full rejection at the
worst reachable standoff needs **≥ 560 rows** at fov 66. `project.godot` ships
720 (1.29× margin) and desktop boots at native resolution, so more pixels is
strictly safer and the design cannot be over-resolved. Below 560 the band
partially survives — response 0.034 at 480 rows and 0.309 at 360, against 1.000
today, so even outside its bound the design is strictly better than what it
replaces. The two ways to reach that state are a small browser window
(`window/size/mode.web = 0`, resizable) and `scaling_3d/scale` dropping below
1.0; `viewport_budget` is read from the real viewport at boot for exactly that
reason. A knee validated at 720 can never speckle a 4K screen, which is the
failure mode the earlier "lower the absolute knee to (0.006, 0.015)" option was
rejected for.

**A wider FOV** invalidates the cap's lower bound: `PX_TAN ∝ tan(fov/2)`, so at
90° the bound rises by 1.54× to 0.3369 — above `cap_upper`, i.e. the bracket
closes and no cap satisfies both constraints at 720 rows. The FOV is a hardcoded
66.0 today, so nothing moves; the day a FOV option or a zoom lands,
`SilKnee::shipped()` must take it as an argument and the root must repush.
`REF_FOV` is declared here, and a gdUnit assertion against
`main.player.camera.fov` is the cheap guard that makes the drift impossible to
miss.

**Two things that do not degrade it.** An orthographic camera is harmless,
because there is no camera term at all. A dead depth texture is harmless, because
this predicate never reads it and the cap gate is written to fire on precisely
the pixels it fires on today.

### Unresolved, and stated rather than smoothed over

**The half-gap-per-tap model is inherited and unverified, and every noise number
in this section rests on it.** `WORST_STEP_CODES` is measured as a gap *width* —
the smallest step that survives at every base — and half of it is the per-tap
error only if the readback representative sits at the cell centre. Rounding to
nearest puts it there for a uniform quantiser, but the 1.25 figure exists
*because* the path from `ALBEDO` through the back-buffer copy is not a uniform
quantiser, and nothing has measured where the representative sits inside a
widened cell. If it sits at a cell edge, the per-tap error is a full gap: the
Laplacian's noise becomes 0.391007 m, the shipped floor's margin falls from
2.4552× to **1.2276×**, `cap_upper` falls to 0.1317, the bracket at 720 rows is
**empty**, and the arm is unavailable below roughly 1130 rows. That is a
fatal-if-true dependency for the arm and an uncomfortable one for the floor, and
no cargo test can settle it because the question is about the hardware. **Test 15
is the experiment and it should run before the arm is implemented.** If it lands
at a full gap, what ships is the module, the budgets, the probe gates and the
floor's ownership — `cap = plane = 0`, which is bit-for-bit today's law — and the
arm waits for a resolution bound the project can actually meet.

**`SIL_CAP`'s balance rule is authored.** The two bounds are derived and the
geometric mean is not: it encodes a decision that the two failure modes deserve
equal ratio margins. Weighting toward the lower bound buys resolution headroom
(at 0.30 the design holds to 495 rows) at the cost of the invariance margin
(1.07×, thin enough that adversarial quantisation would dim some steps).
Weighting the other way does the reverse. Per project law that is fine —
perception is authored and says so — but the honest label on the balance is
taste, and no test in the list above can make it anything else.

**The ramp arm has not been through the same adversarial review as the two
distance-keyed designs it replaces.** Its derivations and every table in this
section were produced in one pass and checked against the tree; they have not
been independently refuted. The three properties it rests on — `ramp == lap` on an
isolated step, `ramp == 0` on locally symmetric features, and
`lap == ramp²/(2·bnear)` on a plane to within 4% — are each pinned by a named
cargo test above precisely because they are the load-bearing claims and are the
first place to look if the picture disagrees with this document.

---

## Test 15, run: the absolute error is not the half-gap (measurement, 2026-08-18)

The design above names one fatal-if-true dependency — that a value read back
from the channel sits within half a measured gap of the value written — and
says the experiment should run before the arm is implemented. It has run:
`game/tests/probe/tap_error_probe.tscn`, on Mesa 25.0 / AMD Radeon, desktop GL,
1280x720, reproduced bit-for-bit across two boots.

**It writes one constant across the whole screen, reads one texel of it back
through `hint_screen_texture`, and takes the difference against the same
constant as a uniform** — inside the shader, where the precision still exists.
No spatial layout, no base swept down a column, no second tap. An earlier
version did sweep the base with y and reported a mean error of -0.578 codes
that was mostly the base moving 1.28 codes per screen row: a reading about
sampling wearing a reading about the channel's clothes. That version is gone.

| base | signed error, nominal codes |
|---|---|
| 0.050 | **-20.24** |
| 0.125 | -8.94 |
| 0.200 | -2.67 |
| 0.275 | -0.16 |
| 0.30 .. 0.95 | between **+1.65 and -1.02** |

**Two findings, and they do not contradict each other.**

*Resolution is intact.* `platform_probe` says a step of 1.25 nominal codes
separates at every one of 649 bases, and that stands: a slowly varying bias
moves both members of a pair together, so every step survives.

*Absolute accuracy is not what `recon_eps` assumes.* A single value comes back
up to **1.647 codes** from where it was written in the mid-range, against the
**0.625** a representative at the cell centre would give. Below B ≈ 0.2 — under
eight metres, at the shipped packing range — the error grows sharply, reaching
20 codes at B = 0.05. The 2026-08-12 platform note recorded that "values below
≈0.027 crush to zero"; this is the same curve, measured, and it starts much
higher than 0.027.

**What this does NOT settle, and why nothing was changed on it.** The
reconstruction guard compares a point rebuilt from B against a wall table
derived on the CPU, so a bias in B is a real displacement of that point — 1.647
codes is 64 mm at `DIST_PACK_RANGE`, against a `RECT_SHRINK` of 30 mm. Taken at
face value the guard needs roughly 86 mm and is hopeless within eight metres of
the eye. But the shipped renderer demonstrably does not misbehave that way:
`occlusion_probe` check 2 has a lit wall face reading as a lit wall face, at a
geometry inside that range, across two boots. So either the guard is less
sensitive to a bias than its own derivation suggests, or this probe is still
measuring something other than what the guard suffers.

Two facts make the probe itself suspect and are stated rather than smoothed
over. Its readings come back on a **0.25-code grid** that its own resolution
model does not predict (8-bit readback of a `±4`-code window should give
0.031), and at `400/1023` three values a quarter of a code apart return the
same stored value while the reported error tracks the offset exactly — which is
a cell at least half a code wide sitting a full code away from the nominal
grid. Both are consistent with a phase-shifted grid; neither is proven.

**So: the arm stays unimplemented, the constants stay where they are, and this
is the first item for the design review rather than a licence to move
`RECT_SHRINK` again.** The probe ships as a diagnostic that gates only on what
it can honestly assert — that the error is bounded by two codes — and prints
the open question every run so it cannot be forgotten.

