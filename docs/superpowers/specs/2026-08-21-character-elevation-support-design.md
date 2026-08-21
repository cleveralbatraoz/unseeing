# Characters Keep Their Height — Design

**Date:** 2026-08-21

**Status:** approved by Dmitrii after a full desired-result clarification pass

**Issues:** [#64](https://github.com/cleveralbatraoz/unseeing/issues/64),
[#74](https://github.com/cleveralbatraoz/unseeing/issues/74)

## Problem and reproduced evidence

The two moving actors have two incompatible ideas of height.

- `UnseeingPlayer` and `WaveCat` are `CharacterBody3D` nodes whose colliders
  follow their physical roots.
- Their physics adapters overwrite Y velocity with planar motion, so neither
  body can fall from unsupported authored positions or after leaving an edge.
- `HeroBody`, `viewmodel::leg_pose`, `cat_body::skeleton`, `CatGait`, and the
  actor wave origins independently rebuild their visuals and contact points
  against absolute world Y values near zero.

A disposable gdUnit fixture placed a real player, hero body, cat, static prop,
and gravity-driven `RigidBody3D` control at Y = 3 m above a floor. After 120
physics ticks on pinned Godot 4.7.1:

```text
player root_y=3.0000 velocity_y=0.0000 collider_y=3.0000 visual_y=-0.0150..1.3800
cat    root_y=3.0000 velocity_y=0.0000 collider_y=3.1900 visual_y=-0.0210..0.3593
prop   root_y=3.0000 collider_y=3.0000 visual_y=2.7500..3.2500
rigid  root_y=0.1900 visual_y=-0.0100..0.3900
```

Godot gravity and collision were therefore working. The two
`CharacterBody3D` adapters discarded vertical motion, while their pure pose
laws discarded physical elevation. The static prop kept its mesh and collider
aligned and correctly remained authored in place.

A second disposable contact fixture compared the current capsule offsets with
capsules whose bottoms meet the actors' authored standing datums. The aligned
player stayed exactly at root Y = 0.9 and the aligned cat exactly at root Y =
0.0 through zero, downward, and held-contact moves; `is_on_floor()` remained
true. The current 5 cm player clearance and 2 cm cat clearance reported no
floor and began drifting once downward motion was supplied.

The defect is consequently limited to the moving player and cat. It is not a
general request to turn static furniture, walls, slabs, fans, radios, or props
into simulated falling bodies.

## Desired shipped behaviour

- Only the player and cat fall. Authored static objects do not.
- Falling has an ordinary Earth-like feel. There is no jump action.
- Walking off an edge freezes the actor's actual world-space trajectory.
  Player movement input and new cat-brain decisions cannot steer the actor in
  air.
- Looking and cane tapping remain available while the player falls.
- A wall removes the blocked horizontal component without bounce. It does not
  emit an impact wave.
- Lower floor-like geometry catches the actor. With no lower geometry the
  actor continues at bounded terminal descent; there is no damage, death, or
  automatic recovery.
- Whole silhouettes follow their physical bodies. This version does not add
  per-foot or per-paw slope placement, slope sliding, or a dedicated falling
  pose.
- Ordinary ramps work in both directions without false landing events.
- Player footsteps and cat paw-contact waves stop in air. The cat's existing
  presence voice remains unchanged except that its origin follows the cat's
  height.
- A sufficiently strong landing emits the actor's familiar footstep-class
  wave once. Its strength grows with impact speed and saturates. The player
  can reach the floor-cane voice; the cat remains quieter.
- On landing, ordinary control and cat behaviour resume without an authored
  pause.
- Player and cat are solid obstacles to each other while both are controlled
  on support. If either actor is airborne, they do not collide with each
  other; only world geometry can catch a fall. Full actor-to-actor airborne
  collision is deferred.

## Decision: one pure support-motion law, two actor adapters

Add one engine-free module, `rust/src/support_motion.rs`. It owns only the
state transition shared by controlled `CharacterBody3D` actors. It does not
know about Godot nodes, input, cat mood, gait, rendering, pulse pools,
scene-tree order, clocks, collision queries, or object classes.

The player and cat retain separate configurations and separate boundary
adapters. Sharing a transition does not make their gait, pose, input, sound,
or Inspector policy interchangeable.

### State and value contracts

The dynamic state has two policies:

```rust
enum MotionPhase {
    Controlled,
    Airborne {
        planar_velocity_mps: PlanarVelocity,
        vertical_velocity_mps: FiniteVelocity,
    },
}

struct MotionState {
    phase: MotionPhase,
    support: Option<SupportContact>,
    last_landing: Option<LandingEvent>,
}
```

`SupportContact` is the finite support datum point and normal from the most
recent move; it is `None` whenever that move had no accepted support.
`last_landing` is inert observation and does not make a second command.

`Controlled` means that designer/player intent may command planar motion and
that the next move is allowed to establish or retain support. It also serves
as the first-tick probe after construction or explicit relocation: an actor
authored over empty space gets one zero-Y discovery move, then becomes
`Airborne` with the actual planar trajectory it achieved. This avoids a third
ambient engine phase and preserves the existing first controlled tick.

`Airborne` owns the launch trajectory. The player adapter does not replace it
with current input. The cat adapter does not advance `CatBrain` to obtain a new
direction. Collision-adjusted planar velocity returned by Godot becomes the
next stored trajectory, so a wall removes its blocked component without a
bounce.

All motion operations consume validated finite scalar/vector types. Raw Godot
`Vector3` values are checked at the adapter boundary. Raw `dt` enters through
the pure total constructor `StepDuration::from_raw(f64)`: zero, negative, NaN,
and infinite values become a zero-duration step, while a finite positive value
is capped at `MAX_ACCEL_DT_S = 1.0 / 15.0 s`. `prepare` accepts only that
validated `StepDuration`, so a stalled debugger cannot create a non-finite
velocity. Acceleration is downward and terminal speed is a magnitude; invalid
configuration produces an explicit validation error, never a panic or NaN.

### Two-phase tick

Godot collision facts are authoritative only after motion, while velocity is
required before it. The adapter therefore uses two pure calls around exactly
one existing `move_and_slide()`:

1. Obtain desired planar motion only when the state is `Controlled`.
2. `prepare(state, desired_planar, duration, config)` returns a finite
   world-space velocity command. Controlled Y is exactly positive zero.
   Airborne Y applies the bounded acceleration law.
3. Set the one velocity and call `move_and_slide()` once.
4. Read the post-move position, collision-adjusted velocity, and floor contact
   facts at the Godot boundary.
5. Convert those facts to a narrow `MotionOutcome` value and call
   `reconcile(prepared, outcome)`.
6. Store the returned state and apply any returned `LandingEvent` command.

A controlled move with no accepted support becomes airborne and captures the
actual planar movement at the edge. An airborne move with accepted support
becomes controlled. Only the latter transition can produce `LandingEvent`.
An airborne collision that is not accepted support retains the downward
command while accepting Godot's collision-adjusted planar velocity.

The pure output contains data, not effects:

```rust
struct LandingEvent {
    impact_speed_mps: FiniteSpeed,
    support_point: FinitePoint,
    support_normal: FiniteDirection,
}
```

Today the actor adapter turns this event into one wave. A later feature may
consume the same value for animation, damage, material response, or camera
feedback without changing the kinematic transition or reaching into it.

### Accepted support

Support is a post-move geometry fact supplied by the adapter. It requires a
Godot floor contact under explicit world-up, floor-angle, and snap settings.
It never depends on a Rust gameplay class name, group, scene-tree path, or
object label.

Godot cannot make the same capsule an always-solid obstacle and also guarantee
that a perfectly centred falling capsule never balances on its top. The
approved simple version makes that tradeoff explicit through two named 3D
physics layers in `game/project.godot`:

- `CONTROLLED_ACTOR_LAYER = 1 << 1` (Godot layer 2);
- `AIRBORNE_ACTOR_LAYER = 1 << 2` (Godot layer 3).

A controlled player/cat occupies layer 2 and uses a mask containing every
layer except airborne actor layer 3. An airborne player/cat occupies layer 3
and uses a mask excluding both actor layers while retaining every other world
layer. Therefore two controlled actors remain solid obstacles, while either
airborne actor makes the pair ignore each other symmetrically. A centred fall
passes through instead of balancing, and only non-actor world geometry can
produce accepted support or a landing event.

The support adapter rejects any floor collision whose collider occupies either
named actor layer; every other floor collision remains geometry-classified by
the explicit slope settings below. The adapter derives its own layer and mask
entirely from captured `MotionPhase`: construction applies the controlled pair
before the first move; explicit relocation changes the phase and collision
pair synchronously before it returns; every reconciliation and restore applies
the pair for the resulting phase. The boundary writes layer/mask only when the
derived pair differs from the current one, avoiding needless broadphase churn.
They are not additional mutable state.
Default all-layer ray queries, including touch/cane observation, can still see
both named actor layers. A future actor-to-actor airborne response must replace
this explicit layer contract rather than add an order-dependent second move.

Normal authored ramps are accepted floors and use an explicit, named floor
snap distance so descending them does not manufacture one-tick airborne gaps.
Deliberate steep-slope sliding is deferred. The current version promises only
the shipped ramp envelope, pinned by the issue fixture and tests.

The adapter writes its solver assumptions instead of inheriting ambient Godot
defaults:

| Setting | Value | Contract |
| --- | ---: | --- |
| up direction | `(0, 1, 0)` | Y is the one vertical axis |
| floor snap | `FLOOR_SNAP_M = 0.10 m` | keeps ordinary descending ramps supported |
| maximum floor angle | `FLOOR_MAX_ANGLE_RAD = π / 4 rad` | covers the shipped ramp envelope |
| safe margin | `SAFE_MARGIN_M = 0.001 m` | bounded collision recovery tolerance |
| maximum slides | `MAX_SLIDES = 6` | preserves Godot 4.7.1's current default |
| stop on slope | `FLOOR_STOP_ON_SLOPE = true` | no gravity-driven creep while controlled |
| constant slope speed | `FLOOR_CONSTANT_SPEED = false` | preserves current walk response |

Moving-platform following is not introduced by this change. Future moving
support must enter `MotionOutcome` as explicit captured data rather than rely
on Godot's ambient platform history.

## Physical and visual coordinate law

### Player

`rust/src/nodes/player.rs` owns an explicit standing-root datum of 0.9 m.
The 1.7 m player capsule moves to local Y = -0.05 m, putting its bottom at
root-relative -0.9 m. Existing scene roots at Y = 0.9 m therefore remain
exactly where they are, with the capsule touching floor Y = 0.

The player's support elevation is:

```text
support_y = player_world_y - PLAYER_STANDING_ROOT_Y
```

That one value enters the pure body-pose boundary:

- `HeroBody` torso, pelvis, and leg roots add it once;
- `viewmodel::leg_pose` clamps ankle and shoe against heights relative to
  `support_y`, not absolute world zero;
- queued footstep origins use `support_y + 0.04 m`, preserving the exact flat
  birth height;
- the cane rest scan, floor/raised classification, fallback target, and air
  swish use player-relative elevation rather than absolute scan heights.

The camera remains a child of the physical player at local
`CAM_BASE_Y`. It already inherits root elevation and receives no support
translation. Head bob remains camera-local. This is the explicit guard against
double-lifting the eye.

While airborne, the viewmodel receives zero walking speed for pose/footstep
purposes. It may settle through its existing neutral easing; no new fall pose
is introduced. Looking and cane animation remain live.

Physics can run more than once before `HeroBody::update()` renders. A
one-physics-tick flag could therefore disappear before the viewmodel consumes
it. Instead the player sets a captured `footstep_suppression_pending` latch on
every airborne-to-controlled transition. `HeroBody` consumes and clears it
through one narrow method when it next evaluates footsteps, passing
`moving = false` to the existing cadence for that frame. The latch persists
across any number of physics ticks, cannot emit a wave itself, and prevents a
regular footstep from doubling the landing voice.

### Cat

`rust/src/nodes/cat.rs` keeps the cat root as the support datum. The 0.34 m
capsule centre moves from local Y = 0.19 m to 0.17 m, placing its bottom at the
root. The editor blueprint and runtime collider use the same named constant.

The cat's support elevation is simply its world root Y. `CatGait` stores that
single scalar beside its existing world-space `planted` and `aim` arrays. At
the start of every gait advance it computes
`delta_y = new_root_y - prior_support_y`, adds exactly that delta to every
stored planted paw and swing aim, then records `new_root_y`. `anchor`, swing,
and `settle` use the stored support Y instead of world zero. This is a uniform
vertical transport, not per-paw terrain sampling.

The remaining consumers use the same transport:

- `cat_body::skeleton` builds its ground plane at `pose.pos.y` rather than
  zero;
- the current `CatPose` paws come from the elevation-aware gait frame;
- before its existing follow law, every stored tail node receives
  `(0, delta_y, 0)`, so height changes do not leave the chain behind while its
  intentional horizontal turning lag remains intact;
- paw origins use contact Y + 0.02 m;
- the presence origin uses root Y + `PRESENCE_HEIGHT`.

No per-paw terrain IK is added. On a ramp all four paw baselines follow the
single actor support elevation. When `delta_y` is positive zero, the existing
flat arithmetic path and outputs remain bit-for-bit unchanged.

When airborne, `CatBrain` is not advanced and yaw is not replaced. Its state is
preserved exactly until accepted support returns. `last_pos` is maintained so
the first resumed brain tick does not receive the whole flight as fictitious
walking progress. The existing gait continues from the actual achieved planar
displacement while airborne, which preserves its phase and avoids inventing a
fall pose or resetting planted state. Gait contacts produced in air or on the
landing tick are withheld from the pulse pool; the next controlled tick
resumes ordinary paw voice. Tail animation and presence cadence continue as
they do today.

## Authored falling and landing voice

These defaults are gameplay stylisation in their stated units. They are not
derived from acoustics. Acceleration is selected for the approved Earth-like
feel; wave reach and gain are independent authored perception controls.

| Setting | Player default | Cat default | Inspector range |
| --- | ---: | ---: | --- |
| fall acceleration | 9.8 m/s² | 9.8 m/s² | `0.1..30.0`, step `0.1 m/s²` |
| terminal fall speed | 20.0 m/s | 20.0 m/s | `0.5..50.0`, step `0.5 m/s` |
| silent impact speed | 1.5 m/s | 1.5 m/s | `0.0..10.0`, step `0.1 m/s` |
| full landing speed | 4.0 m/s | 4.0 m/s | `0.1..20.0`, step `0.1 m/s` |
| maximum landing gain | 0.85 | 0.60 | `0.0..1.0`, step `0.01` |
| maximum landing range | 5.0 m | 2.5 m | `0.0..10.0`, step `0.1 m` |

`WaveCat` is scene-authored, so it owns these six typed `#[export]` fields as
`fall_acceleration`, `terminal_fall_speed`, `landing_silent_speed`,
`landing_full_speed`, `landing_max_gain`, and `landing_max_range`.

`UnseeingPlayer` is constructed at runtime and cannot honestly expose
selectable Inspector fields. The scene-authored `UnseeingGame` root therefore
owns the same six fields with a `player_` prefix. In `UnseeingGame::ready`, it
constructs a validated pure configuration and injects it through a typed Rust
method before adding the player to the tree. This keeps the Inspector usable
and makes the runtime dependency explicit.

All ranges carry the unit suffix shown in the table. No custom Resource,
singleton, or configuration file is introduced. Programmatic values are still
validated because Inspector ranges do not narrow the runtime type domain. A
property setter rejects a non-finite or cross-field-invalid value, retains the
last valid value, and reports the error; the pure configuration constructor
independently validates the complete set. Invalid input is therefore explicit
without freezing an otherwise valid actor or silently installing a different
designer value.

For impact speed `v`, pure landing severity is:

```text
0                                      when v <= silent_speed
(v - silent_speed) / (full_speed - silent_speed)
                                       between the thresholds
1                                      when v >= full_speed
```

The finite result multiplies maximum gain and maximum range independently.
There is no energy or frequency claim. Both actors use pulse kind 2 and the
existing footstep/paw wavefront speed of 4.0 m/s. Player landings retain the
player footstep's reflection policy; cat landings retain the cat's direct
non-reflecting voice. The origin is the actor's support datum plus the existing
flat contact birth height. `LandingEvent` and the observer retain the accepted
floor normal. Only the player's reflecting emission consumes that normal; the
cat's existing omnidirectional direct pulse has no surface-normal field and is
not expanded for this feature.

Every airborne-to-controlled transition retains its `LandingEvent`, including
a silent one. If severity is zero, or if either resulting gain or resulting
range is zero, the adapter deliberately produces no wave command and does not
call `emit`/`emit_reflecting`; no pulse slot or echo appointment is consumed.

The 1.5 m/s silence threshold separates ordinary snap/ramp corrections and a
small curb from a real drop. A chair-height drop is above it. The 4.0 m/s full
threshold makes larger level falls saturate rather than claim unbounded
perception. All four thresholds/maxima are designer-adjustable because their
correct values are authored feel, not physical truth.

## Capture, restore, and observability

`MotionPhase`, including held airborne planar and vertical velocity, is part
of `HeroCapture` and `CatCapture`. The current accepted support point/normal
(absent while unsupported) and most recent `LandingEvent` are captured
alongside it as inert observation.
That event remains available until another landing replaces it; only a fresh
airborne-to-controlled transition returns a new wave command, so restoring an
old observation can never re-emit an already captured wave.

The player capture also carries `footstep_suppression_pending`. Cat capture
carries the gait's support Y with its planted/aim state. Physical position and
body velocity remain captured as boundary observations. Compatibility is
phase-specific rather than an invalid blanket equality:

- every physical and pure scalar/vector must be finite;
- controlled state imposes no equality on Godot's post-slope velocity because
  the next `prepare` authoritatively writes the controlled command;
- airborne held planar X/Z must equal the collision-adjusted physical X/Z
  stored by the same reconciliation;
- airborne pure Y must be non-positive and within the configured terminal
  bound, but may differ from physical body Y when a rejected actor collision
  made Godot report zero. The pure Y is authoritative for the next command.

Restore installs both observations and the pure state before processing
resumes. Support and last-landing history do not feed the transition; the
footstep latch has only its explicit one-consumer suppression effect. A
malformed or contradictory blob returns an explicit transaction error before
the scene tree is mutated.

The reproduction blob encoder, parser, equality/diff surface, snapshot hash,
and mutation fixtures move together, including the required
`FORMAT_VERSION` bump for the changed canonical byte layout. A normal restore
never silently converts an airborne actor to controlled. A deliberate
relocation starts controlled so the following move probes authored support.

The observer adds structured actor motion facts:

- phase (`controlled` or `airborne`);
- actual velocity and held airborne trajectory;
- accepted support presence, point, normal, and collider identity when
  available;
- most recent landing impact speed, with explicit absence before the actor has
  ever landed.

Contact identity is observation only. It is not captured and does not enter
the pure transition, so behavior cannot depend on unstable instance IDs. The
observer lets a fixture prove what Rust accepted as support; mesh-vertex and
wave-queue reads independently prove that the visible/perception boundaries
received the same elevation.

## TDD and verification

Every production behavior starts with the named test that fails against the
current implementation. Tests use checked fixtures and hand-derived expected
values, not mirrors of production constants.

### Pure cargo tests

- controlled support, edge departure, airborne integration, terminal clamp,
  landing, and post-landing control;
- movement input ignored in air and collision-adjusted trajectory retained;
- wall collision removes a planar component without bounce;
- `StepDuration::from_raw` maps zero, negative, non-finite, and oversized `dt`
  to the specified finite bounded duration;
- degenerate/non-finite configuration is rejected;
- landing severity below, exactly at, between, exactly at full, and above the
  thresholds;
- zero/max gain and range remain total;
- support/elevation transformations preserve the exact zero-elevation pose.

### Godot fixtures

- The exact #64 wedge/box/player fixture: root climbs the ramp and platform;
  torso, shoes, collider, and footstep origins share that elevation; flat
  vertices and birth heights remain exact; camera has one lift only.
- Player drop: unsupported start, edge departure, fixed trajectory, wall
  collision, lower-floor landing, no-floor bounded descent, and immediate
  resumed control.
- #74 cats on floor, table, and bed; stationary elevated silhouette/collider;
  walking elevated; edge fall; lower landing; player/cat collision at matching
  and differing elevations.
- Two controlled actors block each other on the same floor; a centred airborne
  actor passes through the other capsule and lands on world geometry without
  becoming controlled or emitting a premature landing wave.
- A controlled actor walking directly off world onto a grounded actor rejects
  that floor collision, switches to airborne collision state in the same
  reconciliation, and passes through on the next move.
- A normal ramp in both directions stays supported and emits no landing wave.
- Airborne movement input and cat-brain time do not steer; look/cane and cat
  presence remain active.
- Paw/footstep queues stay empty in air and on the landing tick. Exactly one
  landing wave has the correct origin, kind, strength, range, and cap; the
  player reflection uses the observed normal while the cat remains
  omnidirectional.
- Multiple physics ticks before one `HeroBody::update()` cannot lose the
  pending footstep suppression or duplicate a landing voice.
- The `UnseeingGame` Inspector values reach its runtime-created player before
  tree entry; each authored cat uses its own Inspector values.
- Small-drop silence, chair-height audible landing, and high-drop saturation.
- Zero configured landing gain or range keeps the landing observation but
  calls neither player nor cat emitter and consumes no pulse/echo capacity.
- Capture/restore in controlled, just-left-edge, mid-fall, wall-adjusted, and
  pre-landing states reproduces position, velocity, phase, cat state, observer
  facts, and queued waves.

The retained fixture is test content under `game/tests/`, not a shipped level.
Movie-maker frames provide final visual evidence for flat, elevated, airborne,
and landed actor/collider alignment. Structured state remains the primary
diagnostic.

### Mutation evidence

At minimum, each mutation must make a named test fail:

- restore either unconditional `velocity.y = 0` path;
- reverse or zero acceleration;
- remove the terminal clamp or `dt` bound;
- accept movement input or a new cat decision in air;
- discard collision-adjusted planar velocity;
- let airborne actor layers collide or accept actor contact as support;
- remove either capsule datum correction;
- discard player/cat elevation in each pose boundary;
- double-apply elevation to the camera or tail;
- emit steps in air or duplicate the landing tick;
- clear the player footstep-suppression latch before `HeroBody` acknowledges
  it;
- flip landing threshold/cap branches;
- call either emitter for zero resulting gain or range;
- ignore the root-injected player configuration or one cat's own configuration;
- restore absolute foot, paw, presence, or cane Y origins;
- omit motion phase/velocity from capture, diff, or restore validation.

### Full gates

- `cargo fmt --check`, pinned `cargo test`, Clippy with warnings denied, and
  release build;
- gdformat/gdlint for changed test GDScript, gdUnit4 full census, editor probe,
  restore/determinism probes, and boot error gate;
- native x86_64/arm64 desktop and wasm32 build checks from the existing pinned
  toolchains, including the repository's macOS-universal and Windows target
  contracts; no architecture conditional code;
- targeted movie-maker frames and probe evidence recorded without committing
  reports, exports, frames, `.wasm`, or `target/` output;
- implementation, architecture, physics/performance, and final code reviews
  against the actual diff.

The per-actor steady-state cost remains O(1), allocation-free pure arithmetic
around the one existing `move_and_slide()`. No support raycast, shape cast,
global cache, worker, or platform-specific path is added. Player cane rays are
unchanged except for elevation-relative endpoints.

## Documentation and delivery

After implementation, rewrite the current behavior and evidence in the
separate project wiki, at minimum:

- Mechanics Overview;
- Mechanics — Level and Objects;
- Mechanics — Waves;
- Engineering — Debugging and Observability;
- Engineering — Build, Test, Deploy.

The wiki must name every owning file and constant, state units, distinguish
physics from authored perception, and record actual probe/test limits. Wiki
work uses a fresh external clone and its own commit; pushing it requires the
user's explicit choice.

This frozen spec and its later implementation plan remain tracked under
`docs/superpowers/`. Production work stays in the isolated
`issue-64-hero-elevation` worktree. Completion ends at the branch-choice gate:
no merge to `main`, push, issue close, or wiki push happens without the user's
explicit integration choice.

## Rejected and deferred alternatives

- **RigidBody actors.** Rejected: it hands controlled movement, timing, and
  capture semantics to a different engine body instead of fixing the thin
  `CharacterBody3D` adapters.
- **Unconditional downward velocity every supported tick.** Rejected as the
  primary law: it hides support phase, weakens exact-flat/capture evidence, and
  gives consumers no explicit air/landing contract.
- **Support ray or shape-cast solver.** Rejected: it duplicates Godot's capsule
  sweep, disagrees at edges, adds queries per actor, and makes moving-platform
  evolution harder.
- **Per-paw/per-foot terrain IK, steep-slope sliding, moving-platform state,
  actor-to-actor airborne response, fall damage, recovery, landing pause, and
  fall animation.** Deliberately deferred. The explicit `MotionOutcome` and
  `LandingEvent` seams are where those future laws may enter.
- **Falling static content.** Rejected: those nodes intentionally author fixed
  level geometry, and their meshes/colliders are already coupled.

## Architecture constraints preserved

- Kinematics and landing response are pure, total Rust laws.
- Godot nodes remain thin adapters around explicit inputs and commands.
- All mutable state has one actor owner and is captured; no global state or
  ambient collaborator is introduced.
- The native and wasm paths call the same pure code.
- Pulse propagation, occlusion, visible-air distance cuts, labels, source
  roles, `MIN_SEP`, and superface merge behavior are untouched. Only actor
  origin Y changes to the physical contact height.
- Falling acceleration is a kinematic gameplay choice. Landing reach/gain are
  named perception authorship. Neither is justified by an acoustic derivation
  the engine cannot represent.
