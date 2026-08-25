//! The hero's visible body as an engine node — cane, arm, legs, torso.
//! Everything renders through the data pass, so the body is OUTLINE-ONLY
//! like the world:
//! - the cane and the arm holding it carry a standing reveal (u_base):
//!   the hero always knows their own grip;
//! - legs and torso are revealed ONLY while a wave sweeps them — each
//!   footstep ripple makes your own feet blink into outline.
//!
//! The arm is a classical first-person viewmodel: anchored in CAMERA
//! space with a figure-eight walk bob, look-sway lag, and a strike kick
//! that reaches the cane tip out to the actual tap target and eases back.
//! The cane is BODY-anchored in yaw, so it doubles as a pitch indicator.
//!
//! No raycasts here: the cane rest comes pre-computed from the player's
//! physics tick (`cane_rest`), and footsteps are queued to the player so
//! their reflection rays also run in physics context. All animation MATH
//! lives in the pure [`crate::hero_visual`] owner; this file only samples
//! the engine once, calls the pure preparation, and commits its returned
//! frame atomically — meshes, viewmodel, shoes, bob, cane sweep, latch
//! and the optional footstep either all install or none do.

use godot::classes::{ArrayMesh, Camera3D, INode3D, MeshInstance3D, Node3D, ShaderMaterial};
use godot::prelude::*;

use super::player::{UnseeingPlayer, support_elevation_at};
use crate::hero_visual::{
    CheckedFootstepPreparer, HeroVisualSample, PreparedLastTap, prepare_hero_visual,
};
use crate::limbs::LimbBuf;
use crate::render;
use crate::support_motion::{
    ActorPosition, ActorTransform, ActorVelocity, FiniteRotation, PosePoint, StepDuration,
};
use crate::temporal::prepare_time;
use crate::viewmodel::{PlanarAxes, PreparedViewmodel, Viewmodel, ViewmodelCapture};

/// The hero's body node. Injected with the player it dresses, the camera
/// the arm anchors to, the wave pool (held for injection parity — waves
/// go through the player), and the two data-pass materials; driven by the
/// composition root through `update(now, dt)` every rendered frame.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct HeroBody {
    /// The player this body dresses — velocity, look, tap clock, cane
    /// rest and the wave queue all read from here.
    #[var]
    player: Option<Gd<UnseeingPlayer>>,
    /// The eye the arm viewmodel anchors to.
    #[var]
    camera: Option<Gd<Camera3D>>,
    /// The wave pool, injected like everywhere else; the body itself
    /// queues its waves through the player so they emit in physics
    /// context.
    #[var]
    pulses: Option<Gd<RefCounted>>,
    /// The cane/arm layer's material: carries the standing reveal.
    #[var]
    cane_mat: Option<Gd<ShaderMaterial>>,
    /// The legs/torso layer's material: revealed only by waves.
    #[var]
    body_mat: Option<Gd<ShaderMaterial>>,
    /// The current walk head-bob (world offset from the base eye height),
    /// prepared by the pure owner, applied through the player's one
    /// visual commit door — the camera's one owner.
    #[var]
    bob_offset: f64,
    #[init(val = ArrayMesh::new_gd())]
    cane_mesh: Gd<ArrayMesh>,
    #[init(val = ArrayMesh::new_gd())]
    body_mesh: Gd<ArrayMesh>,
    /// The installed triangle geometry for each layer, plus the next
    /// frame's scratch owners: the pure preparation fills the scratch
    /// pair, a successful commit swaps them with the installed pair, and
    /// the old installed allocations become the next frame's scratch —
    /// steady-state capacity with no per-frame allocation, and a refused
    /// frame returns both scratch owners untouched.
    #[init(val = Vec::new())]
    cane_buf: LimbBuf,
    #[init(val = Vec::new())]
    body_buf: LimbBuf,
    #[init(val = Vec::new())]
    next_cane_buf: LimbBuf,
    #[init(val = Vec::new())]
    next_body_buf: LimbBuf,
    vm: Option<Viewmodel>,
    shoes: [Vector3; 2],
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for HeroBody {
    fn ready(&mut self) {
        // no silent nulls: a body without its player, eye, pool and
        // materials can neither pose nor be seen — refuse to animate
        // instead of crashing later (the script's asserts, as refusals)
        let (Some(player), Some(camera)) = (self.player.clone(), self.camera.clone()) else {
            godot_error!("hero_body: player/camera not injected");
            return;
        };
        if self.pulses.is_none() || self.cane_mat.is_none() || self.body_mat.is_none() {
            godot_error!("hero_body: pulses/materials not injected");
            return;
        }
        let cane_mesh = self.cane_mesh.clone();
        let cane_mat = self.cane_mat.clone();
        self.add_layer(&cane_mesh, cane_mat.as_ref());
        let body_mesh = self.body_mesh.clone();
        let body_mat = self.body_mat.clone();
        self.add_layer(&body_mesh, body_mat.as_ref());
        self.vm = Some(Viewmodel::new(
            f64::from(player.get_rotation().y),
            f64::from(camera.get_rotation().x),
        ));
    }
}

/// Report one boundary refusal and surrender the value — every invalid
/// engine sample fact uses the same door, so the frame refuses atomically
/// before any installed state has moved.
fn sampled<T, E: std::fmt::Display>(result: Result<T, E>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            godot_error!("hero_body: visual sample refused: {error}");
            None
        }
    }
}

#[godot_api]
impl HeroBody {
    /// Called by the composition root every frame after movement has
    /// settled. One checked sample in, one infallible commit out.
    #[func]
    pub(super) fn update(&mut self, now: f64, dt: f64) {
        let (Some(player_ref), Some(camera_ref)) = (self.player.as_ref(), self.camera.as_ref())
        else {
            return; // _ready refused: nothing to pose
        };
        // liveness before the first clone: cloning a freed Gd panics, so
        // the freed handle must be refused while still borrowed
        if !player_ref.is_instance_valid() || !camera_ref.is_instance_valid() {
            godot_error!("hero_body: visual camera refused — not the player's live eye");
            return;
        }
        let mut player = player_ref.clone();
        let camera = camera_ref.clone();
        if !player.bind().owns_visual_camera(&camera) {
            godot_error!("hero_body: visual camera refused — not the player's live eye");
            return;
        }
        let Some(prior_vm) = self.vm else {
            return; // _ready refused: no viewmodel was ever built
        };

        // read each engine fact exactly once
        let raw_transform = player.get_global_transform();
        let raw_rotation = player.get_global_rotation();
        let raw_velocity = player.get_velocity();
        let raw_camera_local = camera.get_transform();
        let raw_camera_rotation = camera.get_rotation();
        let (raw_last_tap, raw_tap_target, rest_tip, rest_supported, controlled, suppression) = {
            let fields = player.bind();
            let (tip, supported) =
                fields
                    .cane_rest
                    .as_ref()
                    .map_or((Vector3::ZERO, false), |rest| {
                        let rest = rest.bind();
                        (rest.tip, rest.supported)
                    });
            (
                fields.last_tap,
                fields.tap_target,
                tip,
                supported,
                fields.motion_state().accepts_control(),
                fields.footstep_suppression(),
            )
        };

        let Some(now) = sampled(prepare_time(now).map_err(|error| error.rule)) else {
            return;
        };
        let dt = StepDuration::from_raw(dt);
        let Some(player_transform) = sampled(ActorTransform::try_new(raw_transform)) else {
            return;
        };
        let Some(position) = sampled(ActorPosition::try_new(raw_transform.origin)) else {
            return;
        };
        // support derives from the same validated position — never a
        // second ambient position read
        let Some(support) = sampled(support_elevation_at(position.world())) else {
            return;
        };
        let Some(player_rotation) = sampled(FiniteRotation::try_new(raw_rotation)) else {
            return;
        };
        let Some(velocity) = sampled(ActorVelocity::try_new(raw_velocity)) else {
            return;
        };
        let Some(camera_local_transform) = sampled(ActorTransform::try_new(raw_camera_local))
        else {
            return;
        };
        let Some(camera_rotation) = sampled(FiniteRotation::try_new(raw_camera_rotation)) else {
            return;
        };
        let basis = raw_transform.basis;
        let Some(axes) = sampled(PlanarAxes::try_new(-basis.col_c(), basis.col_a())) else {
            return;
        };
        let Some(tap_target) = sampled(PosePoint::try_new(raw_tap_target)) else {
            return;
        };
        let Some(cane_rest_tip) = sampled(PosePoint::try_new(rest_tip)) else {
            return;
        };
        let Some(last_tap) = sampled(PreparedLastTap::try_new(raw_last_tap, now)) else {
            return;
        };
        let Some(sample) = sampled(HeroVisualSample::try_new(
            now,
            dt,
            player_transform,
            player_rotation,
            position,
            support,
            velocity,
            camera_local_transform,
            camera_rotation,
            axes,
            tap_target,
            cane_rest_tip,
            rest_supported,
            last_tap,
            controlled,
        )) else {
            return;
        };

        let cane_scratch = std::mem::take(&mut self.next_cane_buf);
        let body_scratch = std::mem::take(&mut self.next_body_buf);
        match prepare_hero_visual(
            sample,
            prior_vm,
            suppression,
            cane_scratch,
            body_scratch,
            CheckedFootstepPreparer,
        ) {
            Err(refusal) => {
                let (reason, cane_scratch, body_scratch, _preparer) = refusal.into_recovery();
                self.next_cane_buf = cane_scratch;
                self.next_body_buf = body_scratch;
                godot_error!("hero_body: visual sample refused: {reason}");
            }
            Ok((next, _preparer)) => {
                // the infallible commit: swap candidate buffers into the
                // installed slots, resize both meshes, install viewmodel
                // and shoes, then hand the player its one prepared frame
                let (vm, suppression, bob, cane_sweep, shoes, cane_vertices, body_vertices, step) =
                    next.into_commit_parts();
                self.next_cane_buf = std::mem::replace(&mut self.cane_buf, cane_vertices);
                self.next_body_buf = std::mem::replace(&mut self.body_buf, body_vertices);
                render::paint::resize_triangle_surface(&mut self.cane_mesh, &self.cane_buf);
                render::paint::resize_triangle_surface(&mut self.body_mesh, &self.body_buf);
                self.vm = Some(vm);
                self.shoes = shoes;
                self.bob_offset = bob;
                player
                    .bind_mut()
                    .commit_hero_frame(bob, cane_sweep, suppression, step);
            }
        }
    }

    /// The lagging look-sway, horizontal — the suites' bounded-envelope
    /// observable (the script's once-private `_sway_x`).
    #[func]
    fn sway_x(&self) -> f64 {
        self.vm.as_ref().map_or(0.0, Viewmodel::sway_x)
    }

    /// The lagging look-sway, vertical.
    #[func]
    fn sway_y(&self) -> f64 {
        self.vm.as_ref().map_or(0.0, Viewmodel::sway_y)
    }

    /// Both shoes' world positions, left then right — the suites'
    /// shoes-on-the-floor observable (the script's once-private `_shoe`).
    #[func]
    fn shoes(&self) -> PackedVector3Array {
        PackedVector3Array::from(&self.shoes[..])
    }

    /// The cane/arm baked mesh — observable for the mesh-sanity pins.
    #[func]
    fn cane_mesh(&self) -> Gd<ArrayMesh> {
        self.cane_mesh.clone()
    }

    /// The legs/torso baked mesh — observable for the same pins.
    #[func]
    fn body_mesh(&self) -> Gd<ArrayMesh> {
        self.body_mesh.clone()
    }

    /// One render layer of the body: a baked mesh drawn through the given
    /// data-pass material, never frustum-culled (the mesh mutates every
    /// frame). The arm and the legs/torso each read as one silhouette, not
    /// a heap of tubes, because the pure builder bakes one constant label
    /// into every vertex of the mesh's own `CUSTOM0` — what the shader's
    /// G channel reads directly, with no per-instance uniform to keep in
    /// step.
    fn add_layer(&mut self, mesh: &Gd<ArrayMesh>, mat: Option<&Gd<ShaderMaterial>>) {
        let mut mi = MeshInstance3D::new_alloc();
        mi.set_mesh(mesh);
        if let Some(mat) = mat {
            mi.set_material_override(mat);
        }
        mi.set_extra_cull_margin(16384.0);
        self.base_mut().add_child(&mi);
    }

    /// The viewmodel as data — `None` when `_ready` refused (uninjected)
    /// and the viewmodel never existed, which the blob reports as a
    /// refusal, never as a default pose.
    pub(crate) fn capture_vm(&self) -> Option<ViewmodelCapture> {
        self.vm.as_ref().map(Viewmodel::capture)
    }

    /// Place a built viewmodel into a captured mid-stride state — the
    /// footstep clock and shoe alternation included, so the very next
    /// footfall lands exactly where the original's would have.
    pub(crate) fn install_prepared_vm(&mut self, capture: PreparedViewmodel) {
        self.vm = Some(Viewmodel::from_prepared(capture));
    }
}
