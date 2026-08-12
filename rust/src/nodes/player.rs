//! The blind hero as an engine node: first-person movement, mouse look,
//! cane taps — player.gd carried into the Rust layer verbatim.
//!
//! The cane is the ONLY deliberate instrument. A tap picks its mode by
//! what a real ~1.7 m arm-plus-cane could actually touch:
//! - aimed strike — the 3D gaze ray connects within reach: the wave is
//!   born exactly where the player looked (wall, furniture, floor);
//! - rest tap — no aimed hit: the tap lands wherever the cane tip is
//!   physically resting (tabletop, chair seat, or — when the player is
//!   looking down — the floor);
//! - air swish — the cane rests on nothing raised and the player is not
//!   aiming down: NO wave. Air reflects nothing.
//!
//! PHYSICS CONTEXT: every raycast in the game runs inside the physics
//! tick. Input handlers only queue intent; the hero body and the
//! composition root queue wave requests. This keeps all space queries
//! inside Godot's supported physics window.

use godot::classes::{
    Camera3D, CapsuleShape3D, CharacterBody3D, CollisionShape3D, ICharacterBody3D, Input,
    InputEvent, InputEventKey, InputEventMouseButton, InputEventMouseMotion, InputMap, Os,
    PhysicsDirectSpaceState3D, PhysicsRayQueryParameters3D, input,
};
use godot::global::{Key, MouseButton};
use godot::prelude::*;

use crate::observe::QueuedWave;

/// Eye height above the floor.
pub const EYE: f64 = 1.6;

/// Camera rest height in capsule-local space.
pub const CAM_BASE_Y: f64 = EYE - 0.9;

/// Walk speed, m/s — a careful walk, not a run.
pub const SPEED: f64 = 2.1;

/// Arm + white cane: what can truly be touched.
pub const CANE_REACH: f64 = 1.7;

/// Seconds a too-eager second tap is swallowed for.
pub const TAP_COOLDOWN: f64 = 0.15;

/// Radians per pixel of mouse motion, both axes.
pub const MOUSE_SENS: f64 = 0.0026;

/// Radians the eye may pitch up or down.
pub const PITCH_LIMIT: f64 = 1.35;

/// Wall-detection ray height (below tabletops).
pub const CANE_SCAN_HEIGHT: f32 = 0.85;

/// Wall-detection ray length.
pub const CANE_SCAN_LENGTH: f64 = 3.4;

/// The tip stops this far short of a struck wall face.
pub const WALL_BACKOFF: f64 = 0.06;

/// Move actions bind PHYSICAL keycodes so WASD works on any keyboard
/// layout (ЦФЫВ on Russian, ZQSD keys on AZERTY, etc.).
const MOVE_KEYS: [(&str, Key); 4] = [
    ("move_forward", Key::W),
    ("move_left", Key::A),
    ("move_back", Key::S),
    ("move_right", Key::D),
];

/// Where the cane tip naturally rests, and whether any surface actually
/// holds it up (false over open air at floor level). A registered class:
/// the hero body and the suites read it straight off the player.
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct CaneRest {
    /// The resting tip position, settled 0.02 m above its support.
    #[var]
    pub(crate) tip: Vector3,
    /// True when a surface holds the tip up — bare floor included;
    /// "unsupported" is reserved for true open air.
    #[var]
    pub(crate) supported: bool,
    base: Base<RefCounted>,
}

/// What a tap or footstep asks of the wave pool — carried whole from the
/// input/frame context into the physics tick where raycasts may run.
struct WaveRequest {
    kind: i64,
    at: Vector3,
    max_r: f64,
    speed: f64,
    gain: f64,
    echoes: i64,
    normal: Vector3,
}

/// A cane-rest probe before it is published: the raw physics answer.
struct RestProbe {
    tip: Vector3,
    supported: bool,
}

/// The blind hero. Movement and look happen on the engine's
/// CharacterBody3D; the cane's three voices and every wave request drain
/// through the physics tick. The clock is handed, never poked: the
/// composition root advances the simulated time via `tick`, and the
/// player never reads a wall clock of its own.
#[derive(GodotClass)]
#[class(init, base=CharacterBody3D)]
pub struct UnseeingPlayer {
    /// The wave pool every sound enters — the `WaveCore` itself, upcast to
    /// `RefCounted`. The GDScript `Pulses` shim survives only in
    /// `game/tests/`. The player only asks it to `emit_reflecting`, dynamically.
    #[var]
    pulses: Option<Gd<RefCounted>>,
    /// The eye. Built by `_ready` at the fixed base height; the player
    /// alone moves it (mouse pitch, head-bob).
    #[var]
    camera: Option<Gd<Camera3D>>,
    /// The tap clock reading of the last accepted tap — drives the cane
    /// strike animation.
    #[var]
    #[init(val = -10.0)]
    pub(crate) last_tap: f64,
    /// Where the last tap landed (wall/floor/air) — the strike target
    /// the viewmodel reaches toward.
    #[var]
    pub(crate) tap_target: Vector3,
    /// Cached cane rest, recomputed every physics tick at the sweep
    /// offset the viewmodel requested — the hero body reads this instead
    /// of raycasting itself.
    #[var]
    #[init(val = Some(CaneRest::new_gd()))]
    pub(crate) cane_rest: Option<Gd<CaneRest>>,
    cane_rest_offset: f64,
    now: f64,
    tap_queued: bool,
    wave_queue: Vec<WaveRequest>,
    base: Base<CharacterBody3D>,
}

#[godot_api]
impl ICharacterBody3D for UnseeingPlayer {
    fn ready(&mut self) {
        // the body and the eye, exactly the script's _init limbs: a
        // capsule collider and a camera at the fixed base height
        let mut col = CollisionShape3D::new_alloc();
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(0.35);
        capsule.set_height(1.7);
        col.set_shape(&capsule);
        self.base_mut().add_child(&col);
        let mut camera = Camera3D::new_alloc();
        camera.set_position(Vector3::new(0.0, CAM_BASE_Y as f32, 0.0));
        camera.set_near(0.05);
        camera.set_far(60.0);
        camera.set_fov(66.0); // ~1.15 rad vertical, the validated design FOV
        self.base_mut().add_child(&camera);
        self.camera = Some(camera);
        Self::ensure_actions();
        // no silent nulls: without its pulse pool the player cannot voice a
        // single tap or footstep — refuse to run instead of crashing later
        if self.pulses.is_none() {
            godot_error!("UnseeingPlayer: pulses not injected — physics disabled");
            self.base_mut().set_physics_process(false);
            return;
        }
        // on web the browser only grants capture on a user gesture; the
        // click handler below recaptures, so skip the doomed attempt and
        // console noise
        if !Os::singleton().has_feature("web") {
            Input::singleton().set_mouse_mode(input::MouseMode::CAPTURED);
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            if Input::singleton().get_mouse_mode() == input::MouseMode::CAPTURED {
                self.apply_look(motion.get_relative());
            }
            return;
        }
        // Escape belongs to the settings overlay, which raises itself,
        // frees the mouse and freezes the world — all three at once. The
        // player used to release the mouse here and leave the world
        // running; two owners of the cursor is one too many.
        if let Ok(click) = event.try_cast::<InputEventMouseButton>() {
            if !click.is_pressed() {
                return;
            }
            if Input::singleton().get_mouse_mode() != input::MouseMode::CAPTURED {
                Input::singleton().set_mouse_mode(input::MouseMode::CAPTURED);
            }
            if click.get_button_index() == MouseButton::LEFT {
                self.tap_queued = true; // executed next physics tick, in physics context
            }
        }
    }

    fn physics_process(&mut self, _dt: f64) {
        let input =
            Input::singleton().get_vector("move_left", "move_right", "move_forward", "move_back");
        let dir3 = self.base().get_transform().basis * Vector3::new(input.x, 0.0, input.y);
        let mut velocity = self.base().get_velocity();
        velocity.x = (f64::from(dir3.x) * SPEED) as f32;
        velocity.z = (f64::from(dir3.z) * SPEED) as f32;
        velocity.y = 0.0; // flat map: no gravity, no jumping — walking is the verb
        self.base_mut().set_velocity(velocity);
        self.base_mut().move_and_slide();

        let probe = self.compute_cane_rest(self.cane_rest_offset);
        self.publish_cane_rest(&probe);
        if self.tap_queued {
            self.tap_queued = false;
            self.cane_tap();
        }
        // other systems' queued waves: emitted here so reflection raycasts
        // run in physics context
        let space = self.space_state().to_variant();
        let now = self.now;
        let requests = std::mem::take(&mut self.wave_queue);
        for w in requests {
            self.emit_reflecting(
                w.kind, w.at, w.max_r, w.speed, w.gain, now, &space, w.echoes, w.normal,
            );
        }
    }
}

#[godot_api]
impl UnseeingPlayer {
    /// The player registers its own senses: idempotent, so a bare
    /// instance in a test scene polls input without the root's help, and
    /// the boot-time call plus every player `_ready` leave exactly one
    /// key event per action.
    #[func]
    pub(super) fn ensure_actions() {
        let mut map = InputMap::singleton();
        for (action, key) in MOVE_KEYS {
            if map.has_action(action) {
                continue;
            }
            map.add_action(action);
            let mut ev = InputEventKey::new_gd();
            ev.set_physical_keycode(key);
            map.action_add_event(action, &ev);
        }
    }

    /// The registered move actions, in binding order — the observable
    /// face of MOVE_KEYS for the suites (a Dictionary constant cannot
    /// cross the boundary; the physical keycodes stay an engine detail).
    #[func]
    fn move_keys() -> Array<GString> {
        MOVE_KEYS
            .iter()
            .map(|(action, _)| GString::from(*action))
            .collect()
    }

    /// Camera rest height — a float constant served as a static method:
    /// ClassDB registers integer constants only.
    #[func]
    fn cam_base_y() -> f64 {
        CAM_BASE_Y
    }

    /// Walk speed, m/s — static-method constant, same reason.
    #[func]
    fn speed() -> f64 {
        SPEED
    }

    /// Arm + cane reach in meters — static-method constant, same reason.
    #[func]
    fn cane_reach() -> f64 {
        CANE_REACH
    }

    /// The wall backoff in meters — static-method constant, same reason.
    #[func]
    fn wall_backoff() -> f64 {
        WALL_BACKOFF
    }

    /// The pitch clamp in radians — static-method constant, same reason.
    #[func]
    fn pitch_limit() -> f64 {
        PITCH_LIMIT
    }

    /// Mouse sensitivity, radians per pixel — static-method constant,
    /// same reason.
    #[func]
    fn mouse_sens() -> f64 {
        MOUSE_SENS
    }

    /// The clock is handed, never poked: the composition root advances
    /// the simulated time here every frame — and the restorer places it
    /// back on the captured instant before the cane's own clocks land.
    #[func]
    pub(super) fn tick(&mut self, now_t: f64) {
        self.now = now_t;
    }

    /// The cane speaks on command: the scripted twin of the left click,
    /// riding the SAME queued-intent path — executed next physics tick,
    /// in physics context, through the full aimed/rest/swish decision
    /// tree and the [`TAP_COOLDOWN`]. `queue_wave` fakes a wave; this
    /// taps the cane.
    #[func]
    pub fn tap(&mut self) {
        self.tap_queued = true;
    }

    /// One mouse-motion's worth of look, as data: yaw by -x, pitch by -y,
    /// both scaled by [`MOUSE_SENS`], pitch clamped to [`PITCH_LIMIT`] —
    /// the exact law the captured-mouse handler applies, callable without
    /// a mouse so a scripted run turns the hero through the player's real
    /// look path instead of teleporting the rotation around it.
    #[func]
    pub fn look(&mut self, relative: Vector2) {
        self.apply_look(relative);
    }

    /// The viewmodel's sweep asks for the cane rest to be computed at
    /// this yaw offset. One frame of latency BY DESIGN: requested during
    /// the render frame, honored on the next physics tick, read back
    /// through `cane_rest` after that — raycasts stay in physics context,
    /// and the sweep is too slow to notice.
    #[func]
    pub(crate) fn request_cane_sweep(&mut self, offset: f64) {
        self.cane_rest_offset = offset;
    }

    /// The player owns its camera: the viewmodel reports the walk
    /// head-bob and the player alone moves the eye around its fixed base
    /// height. Called by the hero body mid-update, BEFORE the arm anchors
    /// read the camera transform, so the bob shapes the same frame's
    /// viewmodel — as it always has.
    #[func]
    pub(crate) fn set_head_bob(&mut self, offset: f64) {
        if let Some(camera) = self.camera.as_mut() {
            let mut pos = camera.get_position();
            pos.y = (CAM_BASE_Y + offset) as f32;
            camera.set_position(pos);
        }
    }

    /// Other systems (hero footsteps, the demo tap) request waves here;
    /// they are emitted next physics tick so reflection raycasts run
    /// in-context.
    #[func]
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the GDScript queue_wave() signature one to one, \
                  so every call site reads like the script it replaces"
    )]
    pub(crate) fn queue_wave(
        &mut self,
        wave_type: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        max_echoes: i64,
        origin_normal: Vector3,
    ) {
        self.wave_queue.push(WaveRequest {
            kind: wave_type,
            at,
            max_r,
            speed,
            gain,
            echoes: max_echoes,
            normal: origin_normal,
        });
    }

    /// The waves waiting for the next physics tick, copied out as
    /// dictionaries — the queue's observable face for the suites, which
    /// used to read the script's private array directly.
    #[func]
    fn queued_waves(&self) -> Array<VarDictionary> {
        self.wave_queue
            .iter()
            .map(|w| {
                let mut entry = VarDictionary::new();
                entry.set("type", w.kind);
                entry.set("at", w.at);
                entry.set("max_r", w.max_r);
                entry.set("speed", w.speed);
                entry.set("gain", w.gain);
                entry.set("echoes", w.echoes);
                entry.set("normal", w.normal);
                entry
            })
            .collect()
    }

    /// The look law, shared by the captured mouse and the scripted
    /// `look`: the capture GATE stays at the event handler — it is about
    /// who owns the cursor, not about how rotation works.
    fn apply_look(&mut self, relative: Vector2) {
        self.base_mut()
            .rotate_y((f64::from(-relative.x) * MOUSE_SENS) as f32);
        if let Some(camera) = self.camera.as_mut() {
            let mut rot = camera.get_rotation();
            rot.x = (f64::from(rot.x) - f64::from(relative.y) * MOUSE_SENS)
                .clamp(-PITCH_LIMIT, PITCH_LIMIT) as f32;
            camera.set_rotation(rot);
        }
    }

    /// The cane speaks. Executed inside the physics tick, one queued tap
    /// at a time, cooled down by [`TAP_COOLDOWN`] — the three voices of
    /// the decision tree in the module docs.
    fn cane_tap(&mut self) {
        if self.now - self.last_tap < TAP_COOLDOWN {
            return;
        }
        self.last_tap = self.now;
        let Some(camera) = self.camera.clone() else {
            return; // no eye: nothing to aim with (unreachable past _ready)
        };
        let pitch = f64::from(camera.get_rotation().x);
        let aim = -camera.get_global_transform().basis.col_c();
        let flat = Vector3::new(aim.x, 0.0, aim.z).normalized();
        let from = camera.get_global_position();
        let Some(mut space) = self.space_state() else {
            return; // outside a world: nothing to strike
        };
        let hit = PhysicsRayQueryParameters3D::create(from, from + aim * CANE_REACH as f32)
            .map(|query| space.intersect_ray(&query))
            .unwrap_or_default();
        let space_var = space.to_variant();
        let now = self.now;
        if let (Some(hit_pos), Some(hit_normal)) = (
            hit.get("position").and_then(|v| v.try_to::<Vector3>().ok()),
            hit.get("normal").and_then(|v| v.try_to::<Vector3>().ok()),
        ) {
            // aimed strike: the wave is born exactly where you looked
            self.tap_target = hit_pos;
            let floorish = f64::from(hit_normal.y) > 0.7 && f64::from(hit_pos.y) < 0.2;
            let max_r = if floorish { 5.0 } else { 6.0 };
            let gain = if floorish { 0.85 } else { 1.0 };
            self.emit_reflecting(0, hit_pos, max_r, 5.5, gain, now, &space_var, 6, hit_normal);
            return;
        }
        let rest = self.compute_cane_rest(0.0);
        let raised = rest.supported && f64::from(rest.tip.y) > 0.15;
        if raised || (rest.supported && pitch <= -0.12) {
            // no aim needed: tap whatever the cane is physically resting
            // on — tabletop, chair seat, or (when looking down) the floor
            self.tap_target = rest.tip;
            let max_r = if raised { 6.0 } else { 5.0 };
            let gain = if raised { 1.0 } else { 0.85 };
            self.emit_reflecting(
                0,
                rest.tip,
                max_r,
                5.5,
                gain,
                now,
                &space_var,
                6,
                Vector3::UP,
            );
        } else {
            // air swish: the cane sweeps up through nothing; air reflects
            // nothing — only the strike animation remembers the arc
            let swish_y = (EYE + pitch.tan() * 1.5).clamp(0.3, 1.7);
            let reach = from + flat * 1.5;
            self.tap_target = Vector3::new(reach.x, swish_y as f32, reach.z);
        }
    }

    /// Where the cane tip naturally rests for a given sweep offset: reach
    /// forward (walls shorten the reach at cane height), then settle onto
    /// the first supporting surface below — floor, tabletop, chair seat.
    /// This is the cane "touching" the world; the tap and the visuals
    /// both use it. Physics-context only: called from the physics tick.
    fn compute_cane_rest(&mut self, yaw_offset: f64) -> RestProbe {
        let fw = -self.base().get_global_transform().basis.col_c();
        let dir = Vector3::new(fw.x, 0.0, fw.z)
            .normalized()
            .rotated(Vector3::UP, yaw_offset as f32);
        let gp = self.base().get_global_position();
        let from = Vector3::new(gp.x, CANE_SCAN_HEIGHT, gp.z);
        let mut wall_d = CANE_SCAN_LENGTH;
        let space = self.space_state();
        if let (Some(space), Some(query)) = (
            space.clone(),
            PhysicsRayQueryParameters3D::create(from, from + dir * CANE_SCAN_LENGTH as f32),
        ) {
            let mut space = space;
            let wall = space.intersect_ray(&query);
            if let Some(wall_pos) = wall
                .get("position")
                .and_then(|v| v.try_to::<Vector3>().ok())
            {
                wall_d = f64::from((wall_pos - from).length());
            }
        }
        let reach = CANE_REACH.min(wall_d - WALL_BACKOFF);
        let px = f64::from(gp.x) + f64::from(dir.x) * reach;
        let pz = f64::from(gp.z) + f64::from(dir.z) * reach;
        let down = match (
            space,
            PhysicsRayQueryParameters3D::create(
                Vector3::new(px as f32, 1.05, pz as f32),
                Vector3::new(px as f32, -0.1, pz as f32),
            ),
        ) {
            (Some(mut space), Some(query)) => space.intersect_ray(&query),
            _ => VarDictionary::new(),
        };
        if let Some(down_pos) = down
            .get("position")
            .and_then(|v| v.try_to::<Vector3>().ok())
        {
            RestProbe {
                tip: Vector3::new(px as f32, (f64::from(down_pos.y) + 0.02) as f32, pz as f32),
                supported: true,
            }
        } else {
            RestProbe {
                tip: Vector3::new(px as f32, 0.02, pz as f32),
                supported: false,
            }
        }
    }

    /// Publish a probe as the frame's `cane_rest` — a fresh CaneRest per
    /// tick, exactly as the script rebuilt its own.
    fn publish_cane_rest(&mut self, probe: &RestProbe) {
        let mut rest = CaneRest::new_gd();
        {
            let mut fields = rest.bind_mut();
            fields.tip = probe.tip;
            fields.supported = probe.supported;
        }
        self.cane_rest = Some(rest);
    }

    /// The one door to the pool: a dynamic `emit_reflecting` on the
    /// injected object, so the GDScript shim and a future direct
    /// WaveCore both answer. PHYSICS CONTEXT: callers are all inside the
    /// physics tick, per the module law.
    #[expect(
        clippy::too_many_arguments,
        reason = "mirrors the pool's emit_reflecting signature one to one, \
                  so the call sites read like the GDScript they replace"
    )]
    fn emit_reflecting(
        &mut self,
        kind: i64,
        at: Vector3,
        max_r: f64,
        speed: f64,
        gain: f64,
        now: f64,
        space: &Variant,
        max_echoes: i64,
        origin_normal: Vector3,
    ) {
        let Some(pulses) = self.pulses.as_mut() else {
            return; // unreachable past the _ready guard; total anyway
        };
        pulses.call(
            "emit_reflecting",
            &[
                kind.to_variant(),
                at.to_variant(),
                max_r.to_variant(),
                speed.to_variant(),
                gain.to_variant(),
                now.to_variant(),
                space.clone(),
                max_echoes.to_variant(),
                origin_normal.to_variant(),
            ],
        );
    }

    /// The physics space of the player's world, if it stands in one.
    fn space_state(&self) -> Option<Gd<PhysicsDirectSpaceState3D>> {
        self.base()
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state())
    }
}

impl UnseeingPlayer {
    /// The cane's queued-intent flag, for the observer: a tap accepted
    /// this frame that the physics tick has not yet executed.
    pub(crate) fn tap_queued(&self) -> bool {
        self.tap_queued
    }

    /// The eye's pitch, radians — `None` before `_ready` has built the
    /// camera, which is a different fact from a level gaze and must not
    /// be reported as one.
    pub(crate) fn eye_pitch(&self) -> Option<f64> {
        self.camera
            .as_ref()
            .map(|camera| f64::from(camera.get_rotation().x))
    }

    /// The wave queue as pure observations — the same content the
    /// `queued_waves` #[func] serialises for the suites.
    pub(crate) fn wave_queue(&self) -> Vec<QueuedWave> {
        self.wave_queue
            .iter()
            .map(|w| QueuedWave {
                kind: w.kind,
                at: w.at,
                max_r: w.max_r,
                speed: w.speed,
                gain: w.gain,
                echoes: w.echoes,
                normal: w.normal,
            })
            .collect()
    }

    /// The restore door for the eye: the same clamp the look law applies,
    /// so a blob cannot place the eye past `PITCH_LIMIT`.
    pub(crate) fn set_eye_pitch(&mut self, pitch: f64) {
        if let Some(camera) = self.camera.as_mut() {
            let mut rot = camera.get_rotation();
            rot.x = pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT) as f32;
            camera.set_rotation(rot);
        }
    }

    /// Empty the out-tray before a restore rebuilds it — restoring onto a
    /// non-empty queue would replay the captured waves AND the stale ones.
    pub(crate) fn clear_wave_queue(&mut self) {
        self.wave_queue.clear();
    }

    /// The restore door for the cane's queued intent — the flag as DATA,
    /// both ways.
    ///
    /// [`Self::tap`] cannot serve here, and that is the whole reason this
    /// exists: it only ever SETS the flag, so a blob captured with no tap
    /// pending could not clear one the live world was holding, and the
    /// transaction would refuse itself at `hero.tap_queued` over a
    /// difference it was able to fix. Nothing else about a tap is decided
    /// here — the cooldown, the aim and the three voices all still run in
    /// [`Self::cane_tap`], on the physics tick, exactly as a real click's
    /// would.
    pub(crate) fn restore_tap_queued(&mut self, queued: bool) {
        self.tap_queued = queued;
    }
}
