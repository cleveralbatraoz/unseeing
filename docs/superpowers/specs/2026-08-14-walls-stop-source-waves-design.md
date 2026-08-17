# Walls stop every source wave

Date: 2026-08-14
Status: implemented (reveal 2026-08-14; shell corrected 2026-08-17 — see
"The shell")

## Problem

A sound source's wave reveals geometry on the far side of a wall. In
`level_01`, the fan at `(8.6, 4.4)` lights the spawn area at `(3, 4)`
through the `Divider` wall at `x = 6.4`; the straight line between them
crosses that wall at `z ~= 4.24`, well clear of the doorway at
`z in [8, 12.4]`.

This is not a regression. The wall table is complete (19 walls against the
32 slots of `sight::MAXW`, so `level_plan::wall_budget` files no
complaint), `sight.rs` counts crossings correctly, and the GLSL
transliteration in `pulse_pool.gdshaderinc` matches it. The reveal is
produced deliberately by `data_core.gdshaderinc:81-86`:

```glsl
float source_reveal_vis(float typ, vec3 src, vec3 world) {
    int blocked = wall_crossings_from(src, world);
    if (blocked == 0) { return 1.0; }
    if (typ > 2.5) { return pow(HUM_THROUGH, float(blocked)); }
    return 0.0;  // tap, echo, footstep: cut crisp at a wall
}
```

`HUM_THROUGH = 0.55` grants pulse kind 3 — every world sound source — a
transmission privilege the hero's own sounds do not have. One wall leaves
55% of the reveal, two leave 30%. The privilege is spent again in
`hearing_post.gdshader:129`, where a source's shell passes the world
muffled rather than being cut.

The behaviour is unwanted. A wall must be a barrier to sound waves, and
that must hold for every sound source rather than being configured per
source.

## Decision

**A wall stops a sound wave, whatever made it.** Pulse kind stops
mattering at a wall: taps, echoes, footsteps and world sources are all cut
crisp. `HUM_THROUGH` ceases to exist in both languages.

Two consequences follow, and both are intended:

- A source no longer reveals anything behind a wall. It still lights the
  next room *through a doorway*, because reveal has always been a
  straight-line test from source to lit point and never a diffusion — a
  point reachable only around a corner already receives nothing. "Same
  room or doorway" is therefore the existing geometry, not new work.
- A source's shell no longer passes a wall in the air, so a source in
  another room is silent.

Out of scope, deliberately: the source *silhouette*. `SOURCE_THROUGH =
0.3` (`level_plan.rs:42`, applied by `WaveLevel::source_muffle` at
`nodes/level.rs:688-692`) is untouched here. A source's body stays visible
through walls exactly as it ships today. Gating that silhouette on having
once heard the source is a separate, approved follow-up spec.

## Scope

### The law

`source_reveal_vis` loses its `typ` parameter. Kind no longer changes the
answer, so carrying it would be a lie about the domain:

```glsl
float source_reveal_vis(vec3 src, vec3 world) {
    return wall_crossings_from(src, world) == 0 ? 1.0 : 0.0;
}
```

The birth-wall skip in `wall_crossings_from` stays exactly as it is: a
sound born flush inside a wall still lights that wall's own near face,
which is what keeps a tap struck on a wall from erasing itself. That
subtlety is orthogonal to this change and must survive it.

`reveal_at`'s `bound` early-out at `data_core.gdshaderinc:125-139` remains
valid — `source_reveal_vis` still returns a value in `[0, 1]`, now only
ever `1.0` or `0.0` — but its comment names the old three-valued range and
must be rewritten.

### The shell

`hearing_post.gdshader:122-130` currently branches on kind:

```glsl
float mute = 1.0;
if (typ < 2.5) {
    if (t >= scene_d || seen_walled) { continue; }
} else if (t >= scene_d) {
    mute = HUM_THROUGH;
}
```

The intended shape is one rule for every kind, with `mute` deleted:

```glsl
if (t >= scene_d || seen_walled) { continue; }
```

**Revised 2026-08-17 — the shape above does not bar anything.** Both of its
terms are keyed to the CAMERA: `scene_d` is the packed depth of the surface
visible at this pixel, and `seen_walled` asks whether *that surface* lies
behind a wall. Neither mentions the walls between the SOUND and the air it
paints, so this rule does not stop a source's shell from crossing a wall —
it only stops the eye from seeing air the world hides. The two coincide
solely while the sound was made on the camera's side of the world, which is
true of a tap, a footstep and their echoes and never of a world source in
another room: once such a source's sphere has grown past the wall, the view
ray meets it at a NEAR root in FRONT of that wall, inside the hero's own
air, where the depth test passes it. The part of the shell this rule still
draws is precisely the part that leaked through.

The shipped shape therefore adds the wave's own law beside the camera's,
asked of the pulse's origin through the same predicate the reveal asks:

```glsl
if (t >= scene_d || seen_walled) { continue; }
...
if (wall_blocked_from(u_ppos[i], hp)) { continue; }
```

placed after the cone rejection, because it is a per-fragment wall walk per
live pulse per sphere root. `sight::blocked_from` and its GLSL twin answer
the source occluder as a bool with an early exit, which is what pays for it;
no reader needs a count once a wall is a barrier rather than a fade.

`seen_walled` extends to source shells on purpose. It exists because the
always-on-top source skins corrupt packed depth at their own pixels, so
`scene_d` alone cannot be trusted on a ray that reaches an x-rayed source
through a wall. A player ring already needed that backstop; once a source
shell is also barred from crossing walls, it needs the identical one.
This is the one part of the change to derive from a failing test rather
than assert: if a test shows `seen_walled` wrongly suppresses a shell in
the hero's own room, the fallback is `t >= scene_d` alone for kind 3 and
the reason gets recorded here.

### Deletions

`HUM_THROUGH` is removed from every site:

| Site | Action |
| --- | --- |
| `game/shaders/pulse_pool.gdshaderinc:18-24` | delete constant and its comment |
| `game/shaders/data_core.gdshaderinc:65-86` | rewrite law and its comment block |
| `game/shaders/data_core.gdshaderinc:127` | rewrite `bound` comment |
| `game/shaders/hearing_post.gdshader:122-130` | collapse to one rule |
| `rust/src/level_plan.rs:31-34` | delete constant |
| `rust/src/level_plan.rs:1367` | drop the reference in the neighbouring doc |
| `rust/src/observe/ray.rs:11,39-42,78-81` | see below |

### Observability

`RayExplanation::hum_transmission` is defined as
`HUM_THROUGH ^ source_crossings`. With the hum's law gone the name states
a law that no longer exists, so the field is renamed `wave_transmission`
and defined as the new one: `1.0` when `source_crossings == 0`, `0.0`
otherwise — the exact value `source_reveal_vis` returns, for any kind.

`source_transmission` is unchanged: it reports the silhouette law, keyed
to the camera occluder, and Spec A does not touch the silhouette.

Renaming a field of the observer's public answer is a breaking change to
a designer-facing surface, and `game/tests/observer_test.gd:730` reads the
old key. Both move together in one commit.

## Testing

The existing gate cannot catch this class of bug and that is the second
finding worth recording. Coverage today is:

- `sight.rs` and `observe/ray.rs` cargo tests prove the *crossing count*,
  never what is drawn.
- `shader_contract_test.gd:96` and `data_skins_test.gd:79-85` assert that
  the GLSL *source text contains* certain substrings. A swapped
  comparison inside `wall_crosses` still contains every substring and
  still passes.
- `game/tests/probe/occlusion_probe.gd` is the only test that reads real
  pixels (`flare < 0.12`, `reveal < 0.08`), and
  `tools/probe_visibility.sh:5-6` deliberately excludes it from CI because
  headless cannot see shader-reveal leaks. It appears in no CI job.

So the suite is green while the game visibly leaks. The plan adds, in TDD
order:

1. **A value test, not a text test.** A cargo test over the new
   `wave_transmission` pinning the shipped `level_01` geometry: from the
   fan hub to the spawn point, across the real `Divider`, transmission is
   exactly `0.0`; within the fan's own room it is exactly `1.0`; through
   the doorway line it is `1.0`. Literals hand-derived from the scene
   coordinates quoted above, not read back from the code under test.
2. **A kind-independence test.** The same geometry evaluated for kinds 0
   through 3 must give one identical answer. This is the test that fails
   today and the one that would have caught the bug: it breaks the moment
   any kind regains a transmission privilege.
3. **Shader-text contracts updated** to pin the *absence* of
   `HUM_THROUGH` and the presence of the new signature, in
   `shader_contract_test.gd` and `data_skins_test.gd`. These remain weak
   by nature and are not counted as proof.
4. **The rendered probe tightened and actually run.** `occlusion_probe.gd`
   gets a spawn-side assertion — with the hero at the spawn point and the
   fan running across the `Divider`, revealed pixels in the spawn room
   must be at the black floor. It stays human-run, but the plan requires
   running it once and recording the numbers, since it is the only
   evidence that GLSL agrees with Rust.
5. **Mutation checks.** Restoring `pow(0.55, blocked)`, flipping the
   `== 0` comparison, and deleting the `seen_walled` term must each fail
   at least one test.

`explain_ray` reports Rust's belief and cannot prove GLSL agrees; item 4
is the only item that closes that gap, and the plan says so where it
would otherwise be tempting to claim item 1 did.

## Risks

- **The level goes quieter than intended.** Cutting both the reveal and
  the shell removes every long-range cue from a source. If playtesting
  says the level reads dead, the recorded fallback is to restore the
  shell in `hearing_post` alone while the reveal stays cut — the two are
  independent after this change, which is a reason to keep them as
  separate edits within the branch.
- **`seen_walled` over-suppression**, handled above by test.
- **No architecture change.** Nothing here adds state, so the purity,
  totality and no-global-state laws are unaffected. `source_reveal_vis`
  becomes more total, not less: it loses a parameter whose whole domain it
  no longer needs to interpret.

## Documentation

The wiki pages describing wave propagation and sound sources both state
the muffled-hum rule and must be rewritten to the new law, naming
`data_core.gdshaderinc` as the file that owns it, before the branch is
declared complete.
