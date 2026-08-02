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
//! lives in the pure [`viewmodel`] module; this file only reads the
//! player, poses the pure curves, and rebuilds the immediate meshes.

use std::f64::consts::{PI, TAU};

use godot::classes::mesh::PrimitiveType;
use godot::classes::{Camera3D, INode3D, ImmediateMesh, MeshInstance3D, Node3D, ShaderMaterial};
use godot::prelude::*;

use super::player::UnseeingPlayer;
use crate::viewmodel::{self, Pose, Viewmodel};

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
    /// computed here, applied by the player — the camera's one owner.
    #[var]
    bob_offset: f64,
    #[init(val = ImmediateMesh::new_gd())]
    cane_mesh: Gd<ImmediateMesh>,
    #[init(val = ImmediateMesh::new_gd())]
    body_mesh: Gd<ImmediateMesh>,
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

#[godot_api]
impl HeroBody {
    /// Called by the composition root every frame after movement has
    /// settled.
    #[func]
    fn update(&mut self, now: f64, dt: f64) {
        let (Some(mut player), Some(camera)) = (self.player.clone(), self.camera.clone()) else {
            return; // _ready refused: nothing to pose
        };
        let velocity = player.get_velocity();
        let planar_speed = f64::from(Vector2::new(velocity.x, velocity.z).length());
        let yaw = f64::from(player.get_rotation().y);
        let pitch = f64::from(camera.get_rotation().x);
        let last_tap = player.bind().last_tap;
        let pose = {
            let Some(vm) = self.vm.as_mut() else {
                return;
            };
            vm.advance(now, dt, planar_speed, yaw, pitch, last_tap)
        };

        // classic head-bob while walking: computed here, applied by the
        // player before the arm anchors below read the camera transform
        self.bob_offset = pose.bob;
        player.bind_mut().set_head_bob(pose.bob);

        // ask the player's next physics tick to compute the rest at our
        // sweep angle
        player
            .bind_mut()
            .request_cane_sweep(pose.cane_swing * (1.0 - pose.thrust));

        self.build_cane(&player, &camera, pose);
        self.build_body(&player, pose);

        // each footfall: a small wave rippling out around that very shoe,
        // queued to the player so its reflection rays run in physics tick
        let fired = {
            let Some(vm) = self.vm.as_mut() else {
                return;
            };
            vm.footstep(dt, pose.moving)
        };
        if let Some(side) = fired {
            let shoe = self.shoes[if side < 0 { 0 } else { 1 }];
            player.bind_mut().queue_wave(
                2,
                Vector3::new(shoe.x, 0.04, shoe.z),
                1.6,
                4.0,
                0.8,
                2,
                Vector3::UP,
            );
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

    /// The cane/arm immediate mesh — observable for the mesh-sanity pins.
    #[func]
    fn cane_mesh(&self) -> Gd<ImmediateMesh> {
        self.cane_mesh.clone()
    }

    /// The legs/torso immediate mesh — observable for the same pins.
    #[func]
    fn body_mesh(&self) -> Gd<ImmediateMesh> {
        self.body_mesh.clone()
    }

    /// One render layer of the body: an immediate mesh drawn through the
    /// given data-pass material, never frustum-culled (the mesh mutates
    /// every frame).
    fn add_layer(&mut self, mesh: &Gd<ImmediateMesh>, mat: Option<&Gd<ShaderMaterial>>) {
        let mut mi = MeshInstance3D::new_alloc();
        mi.set_mesh(mesh);
        if let Some(mat) = mat {
            mi.set_material_override(mat);
        }
        mi.set_extra_cull_margin(16384.0);
        self.base_mut().add_child(&mi);
    }

    /// The cane and the arm holding it, rebuilt for this frame's pose.
    fn build_cane(&mut self, player: &Gd<UnseeingPlayer>, camera: &Gd<Camera3D>, pose: Pose) {
        let bx = 0.016 * pose.leg_phase.sin() * pose.walk_amp + pose.sway_x;
        let by = 0.012 * (pose.leg_phase * 2.0).sin() * pose.walk_amp + pose.sway_y;
        let hand = view_to_world(
            camera,
            0.30 + bx,
            -0.40 + by - 0.03 * pose.thrust,
            0.55 + 0.16 * pose.thrust,
        );
        let elbow = view_to_world(camera, 0.48 + bx * 0.5, -0.64 + by * 0.5, 0.26);

        // rest: the tip lies on whatever surface the cane reaches — floor,
        // table, chair seat — pre-computed by the player's physics tick; a
        // small hover animates the sweep so the tip touches down at the
        // extremes
        let fields = player.bind();
        let mut rest_tip = fields
            .cane_rest
            .as_ref()
            .map_or(Vector3::ZERO, |rest| rest.bind().tip);
        let target = fields.tap_target;
        drop(fields);
        let moving = pose.walk_amp > 0.5;
        let lift = viewmodel::cane_lift(moving, pose.cane_swing);
        rest_tip.y = (f64::from(rest_tip.y) + 0.12 * lift * (1.0 - pose.thrust)) as f32;
        let tip = rest_tip.lerp(target, pose.thrust.clamp(0.0, 1.0) as f32);

        let mut mesh = self.cane_mesh.clone();
        mesh.clear_surfaces();
        mesh.surface_begin(PrimitiveType::TRIANGLES);
        tube(&mut mesh, elbow, hand, 0.055, 0.045);
        sphere(&mut mesh, hand, 0.055);
        tube(&mut mesh, hand, tip, 0.013, 0.010);
        sphere(&mut mesh, tip, 0.040);
        mesh.surface_end();
    }

    /// The torso and both legs, rebuilt for this frame's walk phase; the
    /// shoes are cached for the footstep waves.
    fn build_body(&mut self, player: &Gd<UnseeingPlayer>, pose: Pose) {
        let p = player.get_global_position();
        let basis = player.get_global_transform().basis;
        let fw_raw = -basis.col_c();
        let fw = Vector3::new(fw_raw.x, 0.0, fw_raw.z).normalized();
        let rv_raw = basis.col_a();
        let rv = Vector3::new(rv_raw.x, 0.0, rv_raw.z).normalized();

        let mut mesh = self.body_mesh.clone();
        mesh.clear_surfaces();
        mesh.surface_begin(PrimitiveType::TRIANGLES);
        // small slim torso ending in a pelvis the legs grow out of
        let tc = Vector3::new(p.x, 0.0, p.z) - fw * 0.20;
        tube(
            &mut mesh,
            Vector3::new(tc.x, 0.90, tc.z),
            Vector3::new(tc.x, 1.28, tc.z),
            0.11,
            0.10,
        );
        sphere(&mut mesh, Vector3::new(tc.x, 1.28, tc.z), 0.10);
        sphere(&mut mesh, Vector3::new(tc.x, 0.90, tc.z), 0.13);
        // full legs: thigh, knee, shin, round shoe — phase-mirrored walk
        // cycle from the pure module
        for s in [-1, 1] {
            let leg = viewmodel::leg_pose(p, fw, rv, pose.leg_phase, pose.walk_amp, s);
            self.shoes[if s < 0 { 0 } else { 1 }] = leg.shoe;
            tube(&mut mesh, leg.hip, leg.knee, 0.06, 0.05);
            sphere(&mut mesh, leg.knee, 0.055);
            tube(&mut mesh, leg.knee, leg.ankle, 0.05, 0.04);
            sphere(&mut mesh, leg.shoe, 0.08);
        }
        mesh.surface_end();
    }
}

/// A classic viewmodel anchor: camera-space offsets (x right, y up, z
/// depth into the view) to a world point.
fn view_to_world(camera: &Gd<Camera3D>, x: f64, y: f64, z: f64) -> Vector3 {
    let cb = camera.get_global_transform().basis;
    camera.get_global_position() + cb.col_a() * x as f32 + cb.col_b() * y as f32
        - cb.col_c() * z as f32
}

// --- smooth cartoon geometry: per-vertex normals mean the edge detector
// --- draws only one clean silhouette per shape, never facet lines

/// A latitude/longitude sphere fan, tessellated exactly like the script's
/// `_sphere` (6 x 12, two triangles per cell, normal per vertex).
fn sphere(mesh: &mut Gd<ImmediateMesh>, c: Vector3, r: f32) {
    const LA: i32 = 6;
    const LO: i32 = 12;
    for i in 0..LA {
        let t0 = f64::from(i) / f64::from(LA) * PI;
        let t1 = f64::from(i + 1) / f64::from(LA) * PI;
        for j in 0..LO {
            let p0 = f64::from(j) / f64::from(LO) * TAU;
            let p1 = f64::from(j + 1) / f64::from(LO) * TAU;
            let n00 = lat_lon_normal(t0, p0);
            let n01 = lat_lon_normal(t0, p1);
            let n10 = lat_lon_normal(t1, p0);
            let n11 = lat_lon_normal(t1, p1);
            for n in [n00, n10, n11, n00, n11, n01] {
                mesh.surface_set_normal(n);
                mesh.surface_add_vertex(c + n * r);
            }
        }
    }
}

/// The unit normal at sphere coordinates (theta from the pole, phi around
/// the equator) — the script's inline Vector3, narrowed per component.
fn lat_lon_normal(theta: f64, phi: f64) -> Vector3 {
    Vector3::new(
        (theta.sin() * phi.cos()) as f32,
        theta.cos() as f32,
        (theta.sin() * phi.sin()) as f32,
    )
}

/// A tapered tube between two points, tessellated exactly like the
/// script's `_tube` (10 segments, quad split into two triangles, the
/// radial direction as the normal).
fn tube(mesh: &mut Gd<ImmediateMesh>, a: Vector3, b: Vector3, r1: f32, r2: f32) {
    const SEG: i32 = 10;
    let span = b - a;
    let len = span.length();
    let axis = if f64::from(len) > 0.0001 {
        span / len
    } else {
        Vector3::UP
    };
    let reference = if f64::from(axis.y.abs()) > 0.9 {
        Vector3::new(1.0, 0.0, 0.0)
    } else {
        Vector3::new(0.0, 1.0, 0.0)
    };
    let u = axis.cross(reference).normalized();
    let v = axis.cross(u);
    for k in 0..SEG {
        let a0 = f64::from(k) / f64::from(SEG) * TAU;
        let a1 = f64::from(k + 1) / f64::from(SEG) * TAU;
        let d0 = u * (a0.cos() as f32) + v * (a0.sin() as f32);
        let d1 = u * (a1.cos() as f32) + v * (a1.sin() as f32);
        let p00 = a + d0 * r1;
        let p01 = a + d1 * r1;
        let p10 = b + d0 * r2;
        let p11 = b + d1 * r2;
        for (vertex, normal) in [
            (p00, d0),
            (p10, d0),
            (p11, d1),
            (p00, d0),
            (p11, d1),
            (p01, d1),
        ] {
            mesh.surface_set_normal(normal);
            mesh.surface_add_vertex(vertex);
        }
    }
}
