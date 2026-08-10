//! The oscillating pedestal fan — a [`SoundSource`] with a body. Everything
//! that makes it a source (kind-3 waves that pass walls muffled, the
//! always-on-top acoustic image, the volume ladder, the cadence gate) it
//! shares with every other source through [`super::source`]; everything in
//! this file is what makes it a FAN and nothing else:
//!
//! - a pedestal, a pivoting head and a guard ring, built as scene limbs;
//! - a head that OSCILLATES, carrying a real collider with it, so the cane
//!   and echo rays strike the fan where it actually points;
//! - and therefore a DIRECTED wash — each wave is aimed wherever the head
//!   points at the instant of birth, so the room is swept like a lighthouse
//!   rather than filled.
//!
//! The motion curves and the shipped voice live in the pure [`fan_wave`]
//! module; this file only builds the limbs and carries each beat into the
//! injected pulse pool.

use std::f32::consts::PI;

use godot::classes::{
    AnimatableBody3D, BoxMesh, CollisionShape3D, CylinderMesh, CylinderShape3D, INode3D, Material,
    Mesh, Node3D, RefCounted, StaticBody3D, TorusMesh,
};
use godot::prelude::*;

use super::source::{SoundSource, SourceRig, sound};
use crate::fan_wave;
use crate::sound_source::{Spread, Voice, Volume};

/// Hub height in meters: within the cane's reach. A build dimension the
/// pivot and the collider both hang from — not a tuning knob.
pub const HEAD_H: f32 = 1.15;

/// The fan housing's flat object id: pedestal, head and guard ring read as
/// one silhouette. The source band's SHELL id (see `oid_palette`).
const FAN_OID: f64 = 0.33;

/// The spinning blades' flat object id — the source band's MOVING id,
/// distinct from the housing so the blades stay legible instead of merging
/// into it.
const FAN_BLADE_OID: f64 = 0.63;

/// The guard ring's outer radius — the widest part of the swinging head,
/// and so what decides how far the housing reaches as it sweeps.
const GUARD_R: f32 = 0.44;

/// Both ids the fan paints itself with, so the level can keep the walls and
/// props it colours clear of whichever one they stand against.
const OIDS: [f64; 2] = [FAN_OID, FAN_BLADE_OID];

/// The pedestal fan node. Scene limbs are built in `_ready` from the
/// injected acoustic-image skin; `update(t)` — driven by the level with the
/// simulated clock, like every animated thing — rides the pure motion
/// curves and fires the cadence into the injected pulse pool. The acoustic
/// voice is a set of designer `#[export]` knobs defaulting to the shipped
/// constants; the knobs are read when the fan enters the tree.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct SoundFan {
    /// The wave pool every sound enters — today the GDScript `Pulses`
    /// shim, tomorrow the `WaveCore` itself; the fan only asks it to
    /// `emit`, dynamically, so both answer.
    #[var]
    pulses: Option<Gd<RefCounted>>,
    /// The acoustic-image skin every limb renders through — the world is
    /// outline-only, and only this pass makes anything visible.
    #[var]
    data_mat: Option<Gd<Material>>,
    /// Loudness in (0, 1]: the ONE knob the fan's carrying power hangs
    /// from. By the volume law it is also how far its waves reach
    /// (`FULL_REACH` × volume) and how strongly its silhouette is felt
    /// through a wall — quieter than the radio, deliberately.
    #[export(range = (0.0, 1.0))]
    #[init(val = fan_wave::FAN_VOLUME)]
    volume: f64,
    /// Seconds between whooshes — so frequent the wash reads as one
    /// continuous stream.
    #[export(range = (0.05, 10.0, 0.05, or_greater))]
    #[init(val = fan_wave::FAN_CADENCE)]
    cadence: f64,
    /// Wavefront speed, m/s — slower than a cane tap: a big lazy source.
    #[export(range = (0.5, 20.0, 0.1, or_greater))]
    #[init(val = fan_wave::FAN_SPEED)]
    wave_speed: f64,
    /// cos of the wash cone's half-angle (~32° at the shipped default).
    /// The fan's defining property: it AIMS, where a radio does not.
    #[export(range = (-1.0, 1.0))]
    #[init(val = fan_wave::FAN_BEAM_COS)]
    beam_cos: f64,
    pivot: Option<Gd<Node3D>>,
    spinner: Option<Gd<Node3D>>,
    rig: SourceRig,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for SoundFan {
    fn ready(&mut self) {
        // no silent nulls: without the pool and the acoustic-image skin the
        // fan can neither sound nor be seen — refuse to build instead of
        // crashing later
        if self.pulses.is_none() || self.data_mat.is_none() {
            godot_error!("SoundFan: pulses/data_mat not injected — fan disabled");
            return;
        }
        let voice = self.voice();
        self.rig.tune(&voice);
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

    /// Meters this fan's waves reach, derived from its volume — the
    /// designer's read-back of the volume law, and what the suites hold
    /// the slot budget against.
    #[func]
    fn reach(&self) -> f64 {
        self.voice().volume.reach()
    }

    /// Slots this fan's voice occupies in the 64-slot pool at steady
    /// state — the budget a designer turning knobs must not blow.
    #[func]
    fn slot_pressure(&self) -> f64 {
        self.voice().slot_pressure()
    }

    /// Driven by the level with the simulated clock — the engine-facing
    /// name for [`SoundSource::advance`], kept so GDScript and the suites
    /// call the fan the way they always have.
    #[func]
    fn update(&mut self, t: f64) {
        SoundSource::advance(self, t);
    }

    /// Pedestal: base disc + pole, as static as the walls.
    fn build_pedestal(&mut self) {
        let mut pedestal = StaticBody3D::new_alloc();
        self.base_mut().add_child(&pedestal);
        let mut body = pedestal.clone().upcast::<Node3D>();
        let skin = self.data_mat.clone();
        self.rig.limb(
            &mut body,
            &cyl(0.22, 0.06),
            Vector3::new(0.0, 0.03, 0.0),
            Vector3::ZERO,
            FAN_OID,
            skin.as_ref(),
        );
        self.rig.limb(
            &mut body,
            &cyl(0.03, HEAD_H),
            Vector3::new(0.0, HEAD_H * 0.5, 0.0),
            Vector3::ZERO,
            FAN_OID,
            skin.as_ref(),
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
        let skin = self.data_mat.clone();
        self.rig.limb(
            &mut head_node,
            &boxm(0.16, 0.16, 0.24),
            Vector3::new(0.0, 0.0, 0.10),
            Vector3::ZERO,
            FAN_OID,
            skin.as_ref(),
        );
        let mut torus = TorusMesh::new_gd();
        torus.set_inner_radius(0.40);
        torus.set_outer_radius(GUARD_R);
        self.rig.limb(
            &mut head_node,
            &torus.upcast::<Mesh>(),
            Vector3::new(0.0, 0.0, -0.10),
            Vector3::new(PI * 0.5, 0.0, 0.0),
            FAN_OID,
            skin.as_ref(),
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
        let skin = self.data_mat.clone();
        self.rig.limb(
            &mut spinner,
            &cyl(0.045, 0.08),
            Vector3::ZERO,
            Vector3::new(PI * 0.5, 0.0, 0.0),
            FAN_BLADE_OID,
            skin.as_ref(),
        );
        for k in 0..3_i32 {
            let mut arm = Node3D::new_alloc();
            let mut arm_rot = arm.get_rotation();
            arm_rot.z = std::f32::consts::TAU * k as f32 / 3.0;
            arm.set_rotation(arm_rot);
            spinner.add_child(&arm);
            self.rig.limb(
                &mut arm,
                &boxm(0.32, 0.11, 0.016),
                Vector3::new(0.24, 0.0, 0.0),
                Vector3::ZERO,
                FAN_BLADE_OID,
                skin.as_ref(),
            );
        }
        self.spinner = Some(spinner);
    }
}

#[godot_dyn]
impl SoundSource for SoundFan {
    /// The spinning hub, wherever the head has swept it to.
    fn hub(&self) -> Vector3 {
        self.spinner.as_ref().map_or_else(
            || self.base().get_global_position(),
            |s| s.get_global_position(),
        )
    }

    fn voice(&self) -> Voice {
        Voice {
            volume: Volume::new(self.volume),
            cadence: self.cadence,
            speed: self.wave_speed,
            spread: Spread::cone(self.beam_cos),
        }
    }

    fn oids(&self) -> &'static [f64] {
        &OIDS
    }

    /// The head swings `PIVOT_RANGE` each way, carrying a 0.44 m guard ring
    /// mounted 0.10 m off the pivot — so the housing reaches this much
    /// further out than whatever single pose the level samples.
    fn sweep_margin(&self) -> f64 {
        f64::from(GUARD_R) * fan_wave::PIVOT_RANGE.sin().abs()
    }

    fn next_emit(&self) -> Option<f64> {
        self.rig.next_beat()
    }

    fn inject(&mut self, pulses: Gd<RefCounted>, skin: Gd<Material>) {
        self.pulses = Some(pulses);
        self.data_mat = Some(skin);
    }

    /// Ride the motion curves, then fire the cadence: the wash blows where
    /// the head points RIGHT NOW, which is what makes it a sweeping beam
    /// rather than a fixed one.
    fn advance(&mut self, t: f64) {
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
        let voice = self.voice();
        let Some(at) = self.rig.beat(t, &voice) else {
            return;
        };
        let aim = -pivot.get_global_transform().basis.col_c();
        let hub = spinner.get_global_position();
        let Some(pulses) = self.pulses.as_mut() else {
            return; // unreachable past the _ready guard; total anyway
        };
        sound(pulses, &voice, hub, aim, at);
    }

    fn set_image(&mut self, image: f64) {
        self.rig.set_image(image);
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
