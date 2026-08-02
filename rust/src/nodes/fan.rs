//! The oscillating pedestal fan as an engine node — the first Rust class
//! a designer places instead of a script. A constant sound source: a
//! blind person FEELS a steady source even from another room, so the
//! fan's hum is pulse kind 3 ("source hum"): its wave shells pass through
//! walls muffled instead of being cut like every player-made sound — but
//! its waves REVEAL only the fan's own room (the shader clips them to
//! u_hum_room). The fan blows a steady DIRECTED wash: a cone of waves out
//! of the pivoting head, born so often they read as one continuous stream
//! sweeping the room like a lighthouse. The head carries a real collider
//! that pivots with it, so the cane and echo rays strike the fan like
//! anything else in the world.
//!
//! The motion curves and the whoosh cadence live in the pure
//! [`fan_wave`] module; this file only builds the scene limbs and carries
//! each beat into the injected pulse pool.

use std::f32::consts::PI;

use godot::classes::{
    AnimatableBody3D, BoxMesh, CollisionShape3D, CylinderMesh, CylinderShape3D, INode3D, Material,
    Mesh, MeshInstance3D, Node3D, StaticBody3D, TorusMesh,
};
use godot::prelude::*;

use crate::fan_wave::{self, WhooshScheduler};

/// Hub height in meters: within the cane's reach. A build dimension the
/// pivot and the collider both hang from — not a tuning knob.
pub const HEAD_H: f32 = 1.15;

/// The pedestal fan node. Scene limbs are built in `_ready` from the
/// injected data-pass material; `update(t)` — driven by the composition
/// root with the simulated clock, like every animated thing — rides the
/// pure motion curves and fires the whoosh cadence into the injected
/// pulse pool. The acoustic voice (cadence, range, speed, gain, cone) is
/// a set of designer `#[export]` knobs defaulting to the shipped
/// constants; the knobs are read when the fan enters the tree.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct SoundFan {
    /// The wave pool every sound enters — today the GDScript `Pulses`
    /// shim, tomorrow the `WaveCore` itself; the fan only asks it to
    /// `emit`, dynamically, so both answer.
    #[var]
    pulses: Option<Gd<RefCounted>>,
    /// The data-pass material every limb renders through — the world is
    /// outline-only, and only this pass makes anything visible.
    #[var]
    data_mat: Option<Gd<Material>>,
    /// Whoosh cadence in seconds — so frequent the wash reads as one
    /// continuous stream.
    #[export]
    #[init(val = fan_wave::WHOOSH_EVERY)]
    whoosh_every: f64,
    /// Meters a hum travels.
    #[export]
    #[init(val = fan_wave::HUM_RANGE)]
    hum_range: f64,
    /// Hum wavefront speed, m/s — slower than a cane tap: a big lazy
    /// source.
    #[export]
    #[init(val = fan_wave::HUM_SPEED)]
    hum_speed: f64,
    /// Hum loudness — steady but never as sharp as the hero's own tap.
    #[export]
    #[init(val = fan_wave::HUM_GAIN)]
    hum_gain: f64,
    /// cos of the wash cone's half-angle (~32° at the shipped default).
    #[export]
    #[init(val = fan_wave::BEAM_COS)]
    beam_cos: f64,
    pivot: Option<Gd<Node3D>>,
    spinner: Option<Gd<Node3D>>,
    scheduler: WhooshScheduler,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for SoundFan {
    fn ready(&mut self) {
        // no silent nulls: without the pool and the data-pass material the
        // fan can neither sound nor be seen — refuse to build instead of
        // crashing later
        if self.pulses.is_none() || self.data_mat.is_none() {
            godot_error!("SoundFan: pulses/data_mat not injected — fan disabled");
            return;
        }
        self.scheduler = WhooshScheduler::with_cadence(self.whoosh_every);
        self.build_pedestal();
        self.build_head();
        self.build_blades();
    }
}

#[godot_api]
impl SoundFan {
    /// The head's oscillation at time `t` — the pure curve, kept callable
    /// for the headless tests.
    #[func]
    fn pivot_angle(t: f64) -> f64 {
        fan_wave::pivot_angle(t)
    }

    /// The blades' rotation at time `t` — the pure curve, kept callable
    /// for the headless tests.
    #[func]
    fn spin_angle(t: f64) -> f64 {
        fan_wave::spin_angle(t)
    }

    /// Radians the head pivots each way from its mounting yaw. A build
    /// constant (the collider rides the same curve), served as a static
    /// method: ClassDB constants are integers only, so a float constant
    /// cannot cross the boundary as one.
    #[func]
    fn pivot_range() -> f64 {
        fan_wave::PIVOT_RANGE
    }

    /// Hub height in meters — a build dimension, served as a static
    /// method for the same integer-only-constants reason.
    #[func]
    fn head_h() -> f64 {
        f64::from(HEAD_H)
    }

    /// Driven by the composition root with the simulated clock, like
    /// every animated thing — movie-maker runs and time scaling stay
    /// correct.
    #[func]
    fn update(&mut self, t: f64) {
        let (Some(mut pivot), Some(mut spinner)) = (self.pivot.clone(), self.spinner.clone())
        else {
            return; // _ready refused to build: nothing to animate, nothing to emit
        };
        let mut pivot_rot = pivot.get_rotation();
        pivot_rot.y = fan_wave::pivot_angle(t) as f32;
        pivot.set_rotation(pivot_rot);
        let mut spin_rot = spinner.get_rotation();
        spin_rot.z = fan_wave::spin_angle(t) as f32;
        spinner.set_rotation(spin_rot);
        let Some(whoosh) = self.scheduler.update(t) else {
            return;
        };
        // the wash blows where the head points right now: a sweeping beam
        let fwd = -pivot.get_global_transform().basis.col_c();
        let hub = spinner.get_global_position();
        let Some(pulses) = self.pulses.as_mut() else {
            return; // unreachable past the _ready guard; total anyway
        };
        pulses.call(
            "emit",
            &[
                3_i64.to_variant(),
                hub.to_variant(),
                self.hum_range.to_variant(),
                self.hum_speed.to_variant(),
                self.hum_gain.to_variant(),
                whoosh.at.to_variant(),
                fwd.to_variant(),
                self.beam_cos.to_variant(),
            ],
        );
    }

    /// Pedestal: base disc + pole, as static as the walls.
    fn build_pedestal(&mut self) {
        let mut pedestal = StaticBody3D::new_alloc();
        self.base_mut().add_child(&pedestal);
        let mut body = pedestal.clone().upcast::<Node3D>();
        self.add_mesh(&mut body, &cyl(0.22, 0.06), Vector3::new(0.0, 0.03, 0.0));
        self.add_mesh(
            &mut body,
            &cyl(0.03, HEAD_H),
            Vector3::new(0.0, HEAD_H * 0.5, 0.0),
        );
        let mut base_col = CollisionShape3D::new_alloc();
        let mut pole = CylinderShape3D::new_gd();
        pole.set_radius(0.22);
        pole.set_height(HEAD_H);
        base_col.set_shape(&pole);
        base_col.set_position(Vector3::new(0.0, HEAD_H * 0.5, 0.0));
        pedestal.add_child(&base_col);
    }

    /// The pivoting head: motor, guard ring and a collider that swings
    /// along.
    fn build_head(&mut self) {
        let mut pivot = Node3D::new_alloc();
        pivot.set_position(Vector3::new(0.0, HEAD_H, 0.0));
        self.base_mut().add_child(&pivot);
        let mut head = AnimatableBody3D::new_alloc();
        pivot.add_child(&head);
        let mut head_node = head.clone().upcast::<Node3D>();
        self.add_mesh(
            &mut head_node,
            &boxm(0.16, 0.16, 0.24),
            Vector3::new(0.0, 0.0, 0.10),
        );
        let mut torus = TorusMesh::new_gd();
        torus.set_inner_radius(0.40);
        torus.set_outer_radius(0.44);
        self.add_mesh_rx(
            &mut head_node,
            &torus.upcast::<Mesh>(),
            Vector3::new(0.0, 0.0, -0.10),
            PI * 0.5,
        );
        let mut head_col = CollisionShape3D::new_alloc();
        let mut disc = CylinderShape3D::new_gd();
        disc.set_radius(0.45);
        disc.set_height(0.30);
        head_col.set_shape(&disc);
        // cylinder axis Y -> face along Z
        let mut col_rot = head_col.get_rotation();
        col_rot.x = PI * 0.5;
        head_col.set_rotation(col_rot);
        head_col.set_position(Vector3::new(0.0, 0.0, -0.06));
        head.add_child(&head_col);
        self.pivot = Some(pivot);
    }

    /// The blades: three flat paddles around a hub, spinning about the
    /// facing axis. Mounted on the pivot, so `build_head` must run first.
    fn build_blades(&mut self) {
        let Some(pivot) = self.pivot.as_mut() else {
            return; // build order broken: nothing to mount the blades on
        };
        let mut spinner = Node3D::new_alloc();
        spinner.set_position(Vector3::new(0.0, 0.0, -0.10));
        pivot.add_child(&spinner);
        self.add_mesh_rx(&mut spinner, &cyl(0.045, 0.08), Vector3::ZERO, PI * 0.5);
        for k in 0..3_i32 {
            let mut arm = Node3D::new_alloc();
            let mut arm_rot = arm.get_rotation();
            arm_rot.z = std::f32::consts::TAU * k as f32 / 3.0;
            arm.set_rotation(arm_rot);
            spinner.add_child(&arm);
            self.add_mesh(
                &mut arm,
                &boxm(0.32, 0.11, 0.016),
                Vector3::new(0.24, 0.0, 0.0),
            );
        }
        self.spinner = Some(spinner);
    }

    /// One limb: a mesh instance drawn through the injected data-pass
    /// material.
    fn add_mesh(&self, parent: &mut Gd<Node3D>, mesh: &Gd<Mesh>, at: Vector3) {
        self.add_mesh_rx(parent, mesh, at, 0.0);
    }

    /// [`Self::add_mesh`] with a rotation around X, for the guard ring
    /// and the blade hub.
    fn add_mesh_rx(&self, parent: &mut Gd<Node3D>, mesh: &Gd<Mesh>, at: Vector3, rx: f32) {
        let mut mi = MeshInstance3D::new_alloc();
        mi.set_mesh(mesh);
        mi.set_material_override(self.data_mat.as_ref());
        mi.set_position(at);
        let mut rot = mi.get_rotation();
        rot.x = rx;
        mi.set_rotation(rot);
        parent.add_child(&mi);
    }
}

/// A capped cylinder mesh with equal top and bottom radii.
fn cyl(radius: f32, height: f32) -> Gd<Mesh> {
    let mut c = CylinderMesh::new_gd();
    c.set_top_radius(radius);
    c.set_bottom_radius(radius);
    c.set_height(height);
    c.upcast()
}

/// A box mesh of the given size.
fn boxm(x: f32, y: f32, z: f32) -> Gd<Mesh> {
    let mut b = BoxMesh::new_gd();
    b.set_size(Vector3::new(x, y, z));
    b.upcast()
}
