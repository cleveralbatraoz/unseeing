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
as the first-tick probe after construction: an actor authored over empty space
gets one zero-Y discovery move, then becomes `Airborne` with the actual planar
trajectory it achieved. The existing explicit player-relocation door also
resets the player to this probe state. Cats have no general runtime teleport
API in this version: scene construction initializes them and capture restore
installs their exact captured phase. This avoids a third ambient engine phase
and preserves the existing first controlled tick without inventing a new cat
behavior.

`Airborne` owns the launch trajectory. The player adapter does not replace it
with current input. The cat adapter does not advance `CatBrain` to obtain a new
direction. Collision-adjusted planar velocity returned by Godot becomes the
next stored trajectory, so a wall removes its blocked component without a
bounce.

All motion operations consume validated finite scalar/vector types. Raw Godot
positions are narrowed through `ActorPosition`, whose finite world-coordinate
domain is `[-1_000_000, 1_000_000] m` on every lane; raw headings use an
`ActorYaw` that is finite and representable in Godot's f32 rotation lane, and
non-negative distance/speed observations use a validated measure. The coordinate envelope is a numerical safety boundary, not a level
or perception constant: it leaves more than five orders of magnitude beyond
the shipped level while proving that offsets and differences in the cat pose
law cannot overflow an f32 lane. Derived gait, skeleton, and tail points use a
separate `PosePoint` envelope of `±1_000_002 m`, reserving more than the proved
sub-1 m maximum authored cat offset around an extreme valid root. Captured
gait/pose/tail points are validated against that derived-point door before
restore. Raw `dt` enters through
the pure total constructor `StepDuration::from_raw(f64)`: zero, negative, NaN,
and infinite values become a zero-duration step, while a finite positive value
is capped at `MAX_ACCEL_DT_S = 1.0 / 15.0 s`. `prepare` accepts only that
validated `StepDuration`, so a stalled debugger cannot create a non-finite
velocity. Acceleration is downward and terminal speed is a magnitude. The
configured terminal is narrowed once to an effective positive f32 lane; both
integration and restore validation use that same lane, so a decimal such as
`0.6 m/s` cannot produce a state that its own restore door rejects. Invalid
configuration or actor samples produce an explicit validation error, never a
panic or NaN.

An adapter validates its complete pre-move global `Transform3D` (origin and
all nine basis lanes), complete Euler rotation, stored prior position, and
desired physical vector before advancing any pure actor state or mutating the
scene. It validates the complete post-move transform and rotation, velocity,
and collision facts before advancing gait, pose, or sound. A poisoned
post-move result restores the exact saved transform bits rather than
reconstructing position/yaw, zeros velocity, disables processing, and emits
one error. Disabling processing makes the report one-shot without adding an
uncaptured “already reported” latch. Capture refuses a runtime actor disabled
by this boundary; it never serializes the actor as if it were healthy.

Player visual code is transactional too. The `HeroBody` boundary first proves
that the injected player and camera are live and that the camera is the same
instance the player owns, then validates one complete value-only
`HeroVisualSample`, including the tap clock against the same prepared frame
time. The existing pure limb builder moves from `nodes/limbs.rs` to top-level
`limbs.rs` without geometry changes, so the top-level cargo-tested
`prepare_hero_visual` owner depends only on pure modules. The operation advances
a copy of the `Viewmodel`, computes both triangle buffers, both shoe points,
bob/sweep commands, a typed optional `PreparedFootstepRequest`, and the next
`FootstepSuppression` value off to the side. Its candidate camera transform is
the validated camera-local transform with the next bob applied, composed with
the validated player transform; it never reads a pre-bob global transform as
the new frame's arm anchor. The complete next VM, bob/sweep scalars, shoes,
every buffer position/normal/label lane, and prepared request are validated
before any installed value changes. Only a completely valid `HeroVisualNext`
is installed through one Rust-only typed player door; the old separately
callable raw bob and cane-sweep setters do not remain as bypasses. A refusal
retains the prior VM, mesh buffers, shoes, bob, queue, cane request, and
suppression bit. No partial brain, gait, tail, visual, wave, or landing state is
installed on any boundary error path.

### Two-phase tick

Godot collision facts are authoritative only after motion, while velocity is
required before it. The adapter therefore uses two pure calls around exactly
one existing `move_and_slide()`:

The player narrows the current simulation time to `PreparedTime` together with
the pre-move boundary sample and retains that exact value until any landing
command is prepared. An invalid time refuses before velocity write or body
move; it is never repaired or resampled after contact.

1. Obtain desired planar motion only when the state is `Controlled`.
2. `prepare(state, desired_planar, duration, config)` returns a finite
   world-space velocity command. Controlled Y is exactly positive zero.
   Airborne Y applies the bounded acceleration law.
3. Set the one velocity and call `move_and_slide()` once.
4. Read the post-move position and collision-adjusted velocity. When Godot
   reports `is_on_floor()`, read every bounded contact in its public motion
   ledger. If that complete ledger contains no floor contact, run the one
   conditional preallocated snap-fact probe defined below; it is a read-only
   query, not a second body move.
5. Convert those facts to a narrow `MotionOutcome` value and call
   `reconcile(prepared, outcome)`.
6. Return the pre-move phase and the fresh `LandingEvent` beside the new state;
   store that state and apply only that returned event command.

The private player tick-success value carries `phase_before`, the reconciled
state, and `landing: Option<LandingEvent>` without collapsing the event into
history. `MotionState::last_landing` is retained observation only. No adapter
may infer a fresh landing from it: doing so would replay an old event after
restore or on every later controlled tick.

A controlled move with no accepted support becomes airborne and captures the
actual planar movement at the edge. An airborne move with accepted support
becomes controlled. Only the latter transition can produce `LandingEvent`.
An airborne collision that is not accepted support retains the downward
command while accepting Godot's collision-adjusted planar velocity.

The pure output contains data, not effects:

```rust
struct LandingEvent {
    impact_speed: FiniteSpeed,
    support: SupportContact,
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

The support adapter validates each contact point and normal before classifying
it, then reads every floor collider's collision layer from its collision RID
through `PhysicsServer3D`. One `KinematicCollision3D` contains multiple contact
facts, so the adapter scans both bounded levels: at most six public motion
results and at most six contacts in each result. It accumulates the first
world-support candidate in ledger order but validates the complete bounded
geometry sample and every floor collider fact before returning it. A later
poisoned point, normal, or floor RID can therefore never be hidden by an
earlier valid floor. Collider identity for a contact already proven non-floor
is neither read nor part of the support domain.

Godot 4.7.1's internal floor snap is a special case that the public slide
ledger cannot represent. The engine computes a private `MotionResult`, may set
`is_on_floor()` and move the body, but does not append that result to the
public `motion_results` list. No public `CharacterBody3D` method exposes that
snap collider's RID. This is pinned to Godot 4.7.1's
[`apply_floor_snap` implementation](https://github.com/godotengine/godot/blob/4.7.1-stable/scene/3d/physics/character_body_3d.cpp#L459-L501)
and its public
[`motion_results` accessors](https://github.com/godotengine/godot/blob/4.7.1-stable/scene/3d/physics/character_body_3d.cpp#L716-L738),
not inferred from node classes. Therefore, only when `is_on_floor()` is true and the
complete ordinary ledger contains no floorish contact, the adapter runs one
cached, read-only `PhysicsServer3D::body_test_motion` from the validated
post-move transform down `FLOOR_SNAP_M`. Its parameters mirror the engine's
snap law: `SAFE_MARGIN_M`, four maximum contacts,
`recovery_as_collision = true`, and `collide_separation_ray = true`. The
parameter and result objects are allocated once per actor; the physics tick
only rewrites and reads them. A false query result is valid no-support and the
adapter does not read stale reusable result data. A successful result with an
out-of-domain count, an invalid RID, or a poisoned contact is an explicit
transaction refusal. If the ordinary ledger contains floorish actor contacts
but no world contact, it is already a complete actor-only floor fact and no
fallback runs. This conditional query recovers collision facts only; it never
changes the actor's transform or velocity and is not a second solver.

For either source, the adapter rejects a floor collision whose layer occupies
either named actor bit; every other valid floor collision remains
geometry-classified by the explicit slope settings below, including
server-backed geometry whose collider object cannot be cast to
`CollisionObject3D`. It finishes validating all bounded contact geometry and
every floor collider fact before using the first accepted world candidate.
Actor-only floor contacts are valid
no-support. An invalid RID or poisoned contact is an explicit adapter refusal,
not an invented edge. Collider object ID zero means absent observation
identity, not invalid support.

The adapter derives its own layer and mask entirely from captured
`MotionPhase`: construction applies the controlled pair before the first move;
explicit player relocation changes the phase and collision pair synchronously
before it returns; every reconciliation and restore applies the pair for the
resulting phase. The boundary writes layer/mask only when the derived pair
differs from the current one, avoiding needless broadphase churn. They are not
additional mutable state.
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
| motion mode | `MotionMode::GROUNDED` | floor and wall classification uses world up |
| platform floor layers | `0` | no ambient moving-floor following enters the actor |
| platform wall layers | `0` | no ambient moving-wall following enters the actor |
| platform on leave | `PlatformOnLeave::DO_NOTHING` | Godot never injects platform velocity into the held trajectory |

Moving-platform following is not introduced by this change. Future moving
support must enter `MotionOutcome` as explicit captured data rather than rely
on Godot's ambient platform history. The zero platform-layer masks and
`DO_NOTHING` leave policy make that absence an explicit solver setting rather
than an assumption about the kinds of bodies present in today's level.

## Physical and visual coordinate law

### Player

The non-class pure `rust/src/hero_visual.rs` owner defines the shared
player eye, standing-root, contact-birth, and derived camera-local datums;
`rust/src/nodes/player.rs` imports those exact constants for the physical
adapter and its existing Godot-facing accessors. It does not define copies.
The explicit standing-root datum is 0.9 m.
The 1.7 m player capsule moves to local Y = -0.05 m, putting its bottom at
root-relative -0.9 m. Existing scene roots at Y = 0.9 m therefore remain
exactly where they are, with the capsule touching floor Y = 0.

The player's support elevation is:

```text
support_y = player_world_y - PLAYER_STANDING_ROOT_Y
```

That one value enters the pure body-pose boundary exactly once, as a
transport (decision, 2026-08-25, superseding the earlier joint-threaded
wording: a ULP audit showed threading support through the joint chain
drifts a raised silhouette up to several f32 roundings away from the
translated flat silhouette, while a single transport add bounds every lane
at half an output ULP by construction — the same law the cat already ships
through `translate_skeleton_y`/`transport_y`):

- `viewmodel::leg_pose` is the flat leg law — hip height 0.90 m, ankle
  floor 0.07 m, shoe floor 0.065 m, no support parameter — and is total
  over validated actor positions because bounded joint offsets near 1.25 m
  cannot cross the 2 m margin between the actor and pose envelopes;
- `hero_visual` builds torso, pelvis, and legs flat, then adds `support_y`
  once to every emitted body vertex and both shoes in one transport pass;
- queued footstep origins use `support_y + 0.04 m`, preserving the exact flat
  birth height;
- the cane rest scan, floor/raised classification, fallback target, and air
  swish use player-relative elevation rather than absolute scan heights.

The camera remains a child of the physical player at local
`CAM_BASE_Y`. It already inherits root elevation and receives no support
translation. Head bob remains camera-local. This is the explicit guard against
double-lifting the eye. Visual preparation does not mutate the camera to obtain
the same-frame arm anchor: it replaces only the validated local camera Y with
`CAM_BASE_Y + next_bob`, composes that prospective local transform with the
validated player transform, and uses the result for both hand and elbow. The
successful commit later applies the same bob to that same live camera instance.

While airborne, the viewmodel receives zero walking speed for pose/footstep
purposes. It may settle through its existing neutral easing; no new fall pose
is introduced. Looking and cane animation remain live.

The cane's physics boundary is total independently of the render sample. One
narrow `CaneQueryPort` is the production path for both the Godot adapter and
the cargo fake; it exposes only raw player/camera samples and bounded ray
answers, never scene mutation or emission. One generic boundary coordinator
sequences that explicit dependency while value-only helper operations derive
one checked `support_y`, validate the complete camera transform/rotation and
every translated query endpoint before asking the port to query, then validate
every returned hit position and normal before comparison, state assignment, or
emission. Cane-rest preparation returns a value and publishes it only on
success; cane-tap preparation first validates the current time and prior tap
clock, then stages queued-intent consumption, the next tap clock, target, and
optional prepared reflecting request together. A malformed time, camera,
endpoint, or physics hit retains the queued intent, prior cane rest,
`last_tap`, and `tap_target`, emits nothing, and reports an explicit refusal.
This adds no query: the existing aim, wall, and downward cane rays are the
complete query set.

A render frame can queue a shoe contact while the actor is still controlled,
then the next physics move can leave the edge before that request is drained.
The queue therefore carries an explicit captured `QueuedWaveGate`:
`Always` for the existing demo/general requests and `ControlledContact` for a
shoe step. After reconciliation, a controlled-contact request emits only when
both the pre-move and post-move phases are controlled and no landing occurred;
otherwise it is consumed silently. This is provenance carried as data, not a
guess based on pulse kind or numeric voice constants, and it prevents a
one-frame-old shoe request from sounding in air or on the landing tick.

`QueuedWaveGate::allows(before, after, landing)` is a pure total policy shared
by queued shoes and immediate cat-paw contacts. The Godot adapters only apply
its emit-or-suppress answer; neither reimplements the transition as callback
logic.

The shoe producer cannot assign that provenance through the general registered
queue API. It prepares a Rust-only `PreparedFootstepRequest` whose fixed fields
are kind 2, range 1.6 m, speed 4.0 m/s, gain 0.8, two echoes, `Vector3::UP`, and
`ControlledContact`; only its checked origin and prepared time vary. It owns a
`CheckedWave` proof for kind/origin/range/speed/gain/time and a distinct
`CheckedReflectionRequest` proof for origin/normal/range/speed/echoes/time plus
derived fan geometry; neither proof substitutes for the other. The player
commit door accepts that type, never raw voice parameters, and appends it
without another fallible validation. General/demo requests continue to enter
through `queue_wave` as `Always`. A pure state-in/state-out `FootstepPreparer`
is the only request/reflection allocation door and is called only when cadence
actually yields a contact. Its cargo fake returns an explicit call count, so a
no-footfall frame proves zero calls without a global allocator or hidden test
state. Such frames allocate nothing for requests or reflection; retained
visual scratch buffers reuse capacity and no per-frame proof or temporary
request `Vec` is introduced.

Physics can run more than once before `HeroBody::update()` renders. A
one-physics-tick flag could therefore disappear before the viewmodel consumes
it. Instead the player owns a captured pure `FootstepSuppression` value. Its
`on_transition` operation sets the pending bit on every
airborne-to-controlled transition, and its `acknowledge` operation returns the
old bit plus the cleared next value. `HeroBody` acknowledges it through one
narrow method when it next evaluates footsteps, passing `moving = false` to
the existing cadence for that frame. The latch persists
across any number of physics ticks, cannot emit a wave itself, and prevents a
regular footstep from doubling the landing voice. Every fresh landing arms it,
including a silent landing and a landing whose authored maximum gain or range
is zero; suppression follows the transition event, never the optional audible
voice.

### Cat

`rust/src/nodes/cat.rs` keeps the cat root as the support datum. The 0.34 m
capsule centre moves from local Y = 0.19 m to 0.17 m, placing its bottom at the
root. The editor blueprint and runtime collider use the same named constant.

The cat's support elevation is simply its validated world root Y. `CatGait`
stores that single scalar beside its existing world-space `planted` and `aim`
arrays. At the start of every gait advance it computes the bounded
`delta_y = new_root_y - prior_support_y` for tail transport, assigns every
stored planted-paw and swing-aim Y lane directly to the exact new root-Y bits,
then records those same bits as `new_root_y`. X/Z lanes are untouched.
`anchor`, swing, and `settle` use the stored support Y instead of world zero.
This is a uniform vertical transport, not per-paw terrain sampling. Direct Y
assignment also makes the temporary format-1 datum exactly recoverable from a
planted point; subtraction/addition rounding cannot create a one-ULP restore
shift before format 2 begins carrying the scalar explicitly.

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
single actor support elevation. When the validated new Y is the existing
positive zero, the assignment writes the same bits and every flat output
remains bit-for-bit unchanged. Public gait restoration validates every stored
point and its recoverable/explicit support datum before constructing state, so
malformed capture data cannot poison the transport.

When airborne, `CatBrain` is not advanced and yaw is not replaced. Its state is
preserved exactly until accepted support returns. `last_pos` is maintained so
the first resumed brain tick does not receive the whole flight as fictitious
walking progress. The existing gait continues from the actual achieved planar
displacement while airborne, which preserves its phase and avoids inventing a
fall pose or resetting planted state. Gait contacts produced in air or on the
landing tick are withheld from the pulse pool; the next controlled tick
resumes ordinary paw voice. Tail animation and presence cadence continue as
they do today.

The cat's pure owners accept typed inputs rather than trusting the node:
`CatBrain` consumes `ActorPosition`, `ActorYaw`, `StepDuration`, and a finite
non-negative progress measure; `CatGait` consumes those same motion values;
`CatPose`, `Skeleton`, and `Tail` validate every derived/captured point through
`PosePoint`. A zero-duration cat tick takes an explicit zero-speed branch and
performs no division. The airborne branch produces no yaw command at all, so
the adapter does not invoke a yaw setter merely to write the old value back.

`RoamRect` is a validated value. `try_around` computes all four min/max edges
in f64, requires finite positive authored extents in `1.0..=30.0 m`, and
rejects a rectangle whose edge would leave the `ActorPosition` coordinate
envelope. Restore validates the same edges/order and requires a captured roam
target to remain inside the raw stored rectangle and the actor coordinate
envelope. It deliberately does not reapply the pre-rounding wall-margin
interval: target selection samples there and then rounds to the 0.1 m grid, so
a lawful self-produced rounded target can sit just outside that inner interval
while remaining inside the raw rectangle. A cat spawned at the last exactly
safe centre is valid; the adjacent f32 centre that would put an edge outside
the envelope is not.

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

Every exported scalar has an explicit initializer equal to the table default;
the tool class never inherits numeric zero from `#[class(init)]`. Each actor
also exposes a read-only six-f64 active-configuration snapshot in constructor
order for tests and observability. That snapshot proves which validated config
the running adapter uses; reading the root's staged Inspector fields is not
accepted as proof of injection.

All ranges carry the unit suffix shown in the table. No custom Resource,
singleton, or configuration file is introduced. Programmatic values are still
validated because Inspector ranges do not narrow the runtime type domain.
Each setter rejects and retains the prior value for a non-finite or
out-of-range scalar. The silent/full pair is different: Godot deserializes
properties one at a time, and no fixed assignment order can load both a valid
pair above the defaults and a valid pair below them. Therefore each
range-valid scalar is staged as authored data; the complete six-field value is
validated atomically before actor construction and whenever a staged edit
again forms a valid pair. During a cross-field-invalid intermediate edit, an
already-running cat keeps its last valid active `SupportMotionConfig`; a game
that reaches `ready` with an invalid final pair refuses before constructing the
player, and a cat that reaches runtime `ready` with one refuses before enabling
motion. The error names both thresholds. Packed-scene round-trip tests cover
valid pairs on both sides of the defaults so serialization order cannot reject
valid authored content.

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
For an audible player landing, a pure preparation door constructs the complete
reflecting command before the first emitter call: the current validated frame
time, kind 2, the accepted support point plus `(0, 0.04, 0)`, independently
authored range and gain, speed 4.0 m/s, two echoes, and the accepted support
normal. The command owns both the complete `CheckedWave` and
`CheckedReflectionRequest` proofs. The origin must pass the shared `WaveOrigin`
envelope and the normal plus derived fan geometry must pass the existing
checked reflection request. A preparation refusal invokes no
emitter and cannot install a partial landing effect. Player preparation happens
while the exact saved pre-move transform is still owned; a refusal restores it,
zeros velocity, disables processing, and returns before state, collision pair,
latch, queue, or emitter changes. The suppression latch is still derived
directly from the fresh transition event, not from whether this optional
audible command exists.

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

The player capture also carries the pending bit of `FootstepSuppression`, and every
queued wave carries its `QueuedWaveGate`, so a restored airborne out-tray
cannot turn a suppressed shoe contact into a general wave. Cat capture carries
the gait's support Y with its planted/aim state. Physical position and body
velocity remain captured as boundary observations. Compatibility is
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

That read-only preflight is complete rather than actor-only: it parses and
domain-validates the environment clock/demo/flicker state without invoking a
repairing setter, validates the exact stored 16-hex hash against the parsed
canonical state, resolves all live targets, validates every actor and
configuration contract, and checks the cat lockstep invariant that captured
body position and `CatPose.pos` have identical X/Y/Z f32 bits and that
`CatGait.support_y` has the same Y bits. Only
the prepared native values may then be committed. An invalid environment or
hash therefore cannot warn, repair, or partly write the pool before refusal;
the old post-write “good restore, bad label” exception is removed.

The reproduction blob encoder, parser, equality/diff surface, snapshot hash,
and mutation fixtures move together, including the required
`FORMAT_VERSION` bump for the changed canonical byte layout. A normal restore
never silently converts an airborne actor to controlled. A deliberate player
relocation starts controlled so the following move probes authored support.

Format 2 lands before the new behavior. In that independently green schema
commit, both live actors still capture only `Controlled`/unsupported motion,
player suppression is clear, every live queued wave is `Always`, and gait
support is the existing common planted/aim Y datum. The parser and pure blob
fixtures already understand the final variants, but restore rejects a
non-dormant live value until the task owning that behavior activates it. The
wire layout and version do not change again as player motion, player effects,
cat pose, and cat motion are enabled in separate green commits.

The player-effects activation removes only the temporary format-2 preflight
restrictions on a pending player suppression bit and a
`ControlledContact` queued-wave gate. It neither changes the parser, canonical
bytes, hash, field order, nor `FORMAT_VERSION`. Restore prepares and installs
both values exactly, a later unrelated preflight failure remains all-read-only,
and the restored future consumes them through the same acknowledgement and
gate laws as an uninterrupted run. Restored `last_landing` remains inert and
cannot create a fresh landing command.

Restore validation is delegated to the owners of the laws. Pulse-pool slots,
echo appointments, viewmodel state, cat brain/gait/pose/tail, cadence/source
appointments, renderer time, demo schedule, and flicker state each expose a
checked prepared constructor. `PreparedRestore` contains those validated
native values plus exact live targets, prepared environment, and the verified
stored hash. The restorer does not duplicate their bounds or call a repairing,
clamping, narrowing, or fallible constructor after the first write. Applying
the environment and committing every owner are infallible installs of the
prepared values; the final recapture/hash is an internal postcondition, not a
late artifact-validation path.

Prepared wave state also has a renderer-numerical domain. Every direct,
queued, reflected, restored-slot, scheduled-echo, and restored-echo origin is
admitted through one checked `WaveOrigin`, whose f32 lanes are finite and lie
in the closed interval `[-MAX_POSE_COORD_M, MAX_POSE_COORD_M]`, with
`MAX_POSE_COORD_M = 1_000_002 m`. This is the already-authored coordinate
safety envelope, not an acoustic reach or a level-size limit. It closes the
specific producer/artifact path by which an extreme finite origin could enter
`u_ppos`; it is not presented as a theorem over an arbitrary corrupted Godot
camera, matrix, wall uniform, or authored transform. Origins are never clamped
or rewritten, and format 2 stores the accepted bits verbatim.

The shader's packed gain is authoritative at its actual precision. A finite
raw gain is clamped to `[0, 1]`, packed once with its kind, and decoded once by
the same f32 floor/remainder law as GLSL. That decoded f32 value, widened to
f64, is the checked effective gain used for reflection appointments. Thus a
primary ring and its echoes cannot disagree because the packed word lost
precision; no tolerance or acoustic attenuation model is introduced.

Yaw and pitch remain the only actor rotation lanes in the reproduction blob.
The omitted axes are live scene configuration, not zeroes. Capture
canonicalizes each complete live Godot YXZ rotation and refuses if doing so
would alter an omitted lane. Restore replaces only player/cat yaw or eye
pitch in the corresponding complete live rotation. Live/brain capture may
canonicalize the owned lane by one ULP while requiring every omitted lane
bit-identical; the sole exception is `+0`/`-0` equivalence, for which the
original omitted sign bit is retained while the serialized owned lane uses
canonical `+0`. Strict artifact installation otherwise requires the complete
requested target already canonical. A
successful restore therefore preserves player body X/Z, eye Y/Z, and cat
body X/Z at the commit boundary. Once processing resumes, those axes follow
the same Godot Euler-cache evolution as an un-restored actor started from the
same complete transform; restore does not freeze or repair ordinary engine
evolution.

The restore-critical handles hardened here are checked live before their first
clone, bind, or method call: the observer's cached player during canonical
capture and the composition root's restorer at transaction entry. This is not
a claim about unrelated legacy runtime caches. A non-finite public wave tick
refuses before draining echoes. The
registered player queue validates a request through the same checked-wave
door before appending it, while retaining next-physics-tick birth semantics.
Reflection explanations propagate an unrepresentable appointment as one
explicit refusal; they never silently drop a kept cluster and publish an
unbalanced ledger. The same checked reflection-geometry value validates a
zero-or-finite surface normal, lifted origin, fan-dot arithmetic, reach, and
ray endpoints before a caller can append a player request, install a reflected
primary, or cast. Malformed caller geometry is wholly atomic. Invalid facts
returned by the physics server keep the independently valid primary but
refuse the entire reflection fan before any echo is scheduled.

The observer adds structured actor motion facts from one validated source:

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

`ActorMotionObservation` stores one validated `MotionState`, one checked
physical velocity, and optional transient collider identity. Its phase,
support, and last landing are projections of that one state, never copied raw
fields that can contradict it. Hero and cat positions are checked
`ActorPosition` values; the existing hero velocity dictionary key, retained
for compatibility, is projected from the same checked velocity as
`hero.motion.actual_velocity`. Any invalid live position/velocity refuses the
whole snapshot instead of publishing a partial actor entry.

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
- Paw contacts and queued shoe contacts produce no pulse in air or on the
  landing tick, including a shoe request queued just before edge departure.
  Exactly one landing wave has the correct origin, kind, strength, range, and
  cap; the player reflection uses the observed normal while the cat remains
  omnidirectional.
- Multiple physics ticks before one `HeroBody::update()` cannot lose the
  pending footstep suppression or duplicate a landing voice. A silent or
  zero-configured landing also suppresses the cadence-ready regular step.
- A nonzero-bob visual frame uses the prospective same-frame camera transform
  for hand and elbow, while camera-local Y receives bob exactly once and no
  support translation. A missing, freed, or mismatched injected camera refuses
  before changing VM, either mesh, shoes, bob, cane request, queue, or latch.
- A deliberately late visual preparation refusal, after copied-VM advance,
  complete scratch-buffer construction, and optional footstep preparation,
  retains every installed VM lane, both triangle buffers including normals and
  labels, both shoes, bob, cane request, queue, and suppression bit.
- The `UnseeingGame` Inspector values reach its runtime-created player before
  tree entry; each authored cat uses its own Inspector values.
- Small-drop silence, chair-height audible landing, and high-drop saturation.
- Zero configured landing gain or range keeps the landing observation but
  calls neither player nor cat emitter and consumes no pulse/echo capacity.
- Capture/restore in controlled, just-left-edge, mid-fall, wall-adjusted, and
  pre-landing states reproduces position, velocity, phase, cat state, observer
  facts, and queued waves.
- Format-2 restore accepts and bit-preserves a pending player suppression bit
  and a `ControlledContact` request, reproduces their next acknowledged/gated
  future, never re-emits a restored old landing, and still refuses every later
  malformed subsystem before any write. The same canonical layout and version
  remain in force.

The retained fixture is test content under `game/tests/`, not a shipped level.
Movie-maker frames provide final visual evidence for flat, elevated, airborne,
and landed actor/collider alignment. Structured state remains the primary
diagnostic.

Numeric assertions name their precision contract. Canonical/capture lanes and
positive zero compare by `to_bits`; pure f32 translations and authored static
datums permit at most one f32 ULP at the hand-written expected magnitude;
settled `CharacterBody3D` contact permits `SAFE_MARGIN_M` plus that one ULP;
and terminal velocity permits one ULP of the effective f32 terminal lane. No
test uses an unnamed `is_equal_approx` or a decimal epsilon copied from
production.

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
- anchor the arm to the sampled pre-bob camera, accept a freed/mismatched
  camera, or double-apply support/bob to the eye;
- emit steps in air, emit a pre-edge controlled contact after a wall/edge
  transition, or duplicate the landing tick;
- clear the player footstep-suppression latch before `HeroBody` acknowledges
  it, omit the candidate latch install, or arm it only for an audible landing;
- flip landing threshold/cap branches;
- call either emitter for zero resulting gain or range;
- infer a fresh landing from `last_landing`, change the player landing current
  time, kind, support-point `+0.04 m` origin, independent gain/range mapping,
  speed, echo budget, or accepted support normal, or omit either admission
  proof;
- append a raw/unprepared shoe request, change its time, fixed voice or gate,
  omit either admission proof, validate it after the first installed visual
  write, or call the request preparer on a no-footfall render frame;
- install a copied VM, shoe, bob, cane request, suppression bit, or either
  triangle buffer before a forced late refusal; omit validation of one buffer
  position, normal, or label lane;
- ignore the root-injected player configuration or one cat's own configuration;
- restore absolute foot, paw, presence, or one cane Y law; bypass the cane
  query port, accept a poisoned time, tap clock, camera, endpoint, hit position,
  or hit normal after consuming intent or mutating cane rest/tap state, change
  any cane request field, omit either proof, or change the bounded query
  order/count;
- omit motion phase/velocity from capture, diff, or restore validation;
- reject or rewrite a valid pending/gated player restore, re-emit its old
  landing, change format 2/canonical bytes, or mutate the scene before a later
  restore-preflight refusal.

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
around the one existing `move_and_slide()`. The ordinary support ledger is
bounded at six motion results times six contacts. A snap-only floor may add
exactly one read-only four-contact `body_test_motion` using per-actor scratch
objects allocated at construction; no support raycast, unbounded query, global
cache, worker, or platform-specific path is added. Player cane rays are
unchanged except for elevation-relative endpoints.
The player's two visual triangle scratch buffers retain capacity and swap with
the installed buffers; a no-footfall render frame performs no request/reflection
allocation. Allocation needed to prepare or emit an actual footstep or landing
is wave activity and occurs only for that request, never as an unconditional
per-frame proof.
This is an adapter work bound; it does not claim a constant instruction count
inside the selected PhysicsServer backend.

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
- **Independent support ray or shape-cast solver.** Rejected: it duplicates
  Godot's capsule sweep, disagrees at edges, adds an unconditional second
  support law, and makes moving-platform evolution harder. The accepted
  conditional `body_test_motion` is narrower: it runs only when Godot has
  already declared a floor but withheld the snap collision from its public
  ledger, mirrors the engine's four-contact snap parameters, and recovers the
  missing RID without moving the body.
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
