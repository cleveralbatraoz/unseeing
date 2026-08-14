# Wiki updates — walls stop every source wave

The wiki is a separate GitHub wiki with no local checkout in this repository
(the only submodule is `tools/superpowers`, which is developer-only). The prose
below is ready to paste into the named pages. Each section says what it
replaces.

---

## Page: Waves — replace whatever describes the per-kind wall rule

### A wall stops a sound wave

A wall is a barrier, not a filter. Every wave is extinguished by the first wall
between its source and the surface it would light — a cane tap, its echoes, a
footstep, and a world sound source's hum alike. Pulse kind does not enter the
decision.

The law lives in one pure function, `reveal_visibility` in
`rust/src/sight.rs`: it answers `1.0` when no wall stands between a wave's
source and the lit point, and `0.0` once any wall does. It takes a crossing
count and nothing else — no kind, no distance, no attenuation — because none of
those can change the answer.

The shipped renderer transliterates that function as `source_reveal_vis` in
`game/shaders/data_core.gdshaderinc`, which owns the reveal law as it actually
draws. `game/tests/shader_contract_test.gd` holds the GLSL to the Rust
reference.

**Waves still travel through doorways.** Reveal is a straight-line test from the
source to the lit point, never a diffusion, so a source lights the next room
through an open doorway and nothing at all through the wall beside it. "Same
room or through a doorway" is a consequence of that geometry, not a separate
rule.

The crossing count comes from `crossings_from` in `rust/src/sight.rs`, which
skips the wall a source is born inside. That is what lets a tap struck flush on
a wall light that wall's own near face instead of erasing itself.

### What changed, and what it replaced

World sound sources — the fan, the radio, anything of pulse kind 3 — used to
hold a privilege the hero's own sounds never had: a wave kept `HUM_THROUGH =
0.55` of its reveal per wall crossed, so a source lit rooms behind walls at 55%
through one wall and 30% through two, while a player's tap was cut to zero. In
`level_01` that made the fan light the spawn room straight through the `Divider`
wall.

`HUM_THROUGH` no longer exists in any Rust or shader file. There is no per-kind
transmission constant to tune, and adding one back would fail
`game/tests/data_skins_test.gd`, which asserts the identifier is absent from all
three shader files.

### Shells in the air

The same rule governs the travelling ring, not just the surfaces it lights.
`game/shaders/hearing_post.gdshader` cuts every shell at the world with a single
kind-independent test. A source in another room is therefore silent.

Kind still changes how a shell *looks* once it is drawn — a hum has a diffuse
body where a tap is a thin grazing ring — but never whether a wall stops it.

---

## Page: Sound sources — replace the "felt through a wall" description

A sound source's wave now stops at a wall exactly like the hero's own sounds.
The behaviour is native to every source: it is a property of the wave law, not a
per-source setting, so a new source class inherits it with no configuration and
there is no knob to get wrong.

**What deliberately did not change: the silhouette.** A source's own body is
still drawn through walls, dimmed by `SOURCE_THROUGH = 0.3` per wall
(`rust/src/level_plan.rs`, applied by `WaveLevel::source_muffle` in
`rust/src/nodes/level.rs`). That constant is keyed to the *camera* occluder —
every wall between the eye and the source counts — whereas the reveal law is
keyed to the *source* occluder.

The consequence is worth stating plainly, because it is a real gap in the
perception fiction rather than an oversight: **a source in another room is
visible but silent.** You see the fan's shape through the wall while nothing it
emits reaches you. Gating that silhouette on having once heard the source — so
it appears only after its wave has genuinely reached the hero, then persists as
a remembered position — is a separate, already-approved design, specified but
not yet implemented.

---

## Page: Build, test and deploy — add this under the test-gate description

### The automated suite cannot see a shader-reveal leak

This is a known and deliberate hole, and anyone changing the reveal or shell law
must work around it by hand.

Every automated occlusion test asserts one of two things: a crossing count
computed in Rust, or the presence of a substring in shader *source text*.
Neither executes GLSL. A flipped comparison inside the shader's wall test would
still contain every asserted substring and would still pass the whole suite
while the game visibly leaked light through walls.

`WaveObserver.explain_ray` reports what Rust believes and cannot prove the GPU
agrees. Its `wave_transmission` field (renamed from `hum_transmission`, whose
0.55-per-wall law no longer exists) reports `1.0` or `0.0` — the same gate
`source_reveal_vis` applies.

The only test that reads real pixels is `game/tests/probe/occlusion_probe.gd`,
run by `tools/probe_visibility.sh`. It is excluded from `ci/pipeline.sh` on
purpose because a headless run renders nothing and would report a false pass.
**Run it by hand after touching the reveal law, the shell law, or the wall
table.** It boots the game twice and requires both verdicts to agree, because
the first boot after a shader edit compiles different GL programs than every
boot after it.

Two traps in that probe are worth knowing before trusting a green result:

- `_peak_r` clamps an off-screen sample point to the image border, which is
  black. A mis-aimed check therefore passes while measuring nothing. Prove a new
  check can FAIL — against a deliberately broken law — before believing that it
  passes.
- The probe's source-reveal case is an *absolute* reading with no before/after
  subtraction to cancel a stray wave, so the run must contain no sound the probe
  did not itself queue. It seeds with `UNSEEING_SEED` for that reason and must
  never be switched to `UNSEEING_DEMO`, which also arms an automatic tap every
  four seconds (`rust/src/demo_tap.rs`).

Recorded evidence for the current law, measured on Apple A18 Pro / OpenGL 4.1
Metal / Godot 4.7.1: with the old muffling law restored the spawn-room check
fails at a leak of 0.263 and 0.165 in two separate sessions; with the shipped
law it reads 0.000, reproduced across a cold and a warm boot.
