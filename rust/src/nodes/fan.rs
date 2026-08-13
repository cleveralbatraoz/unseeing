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
    AnimatableBody3D, ArrayMesh, CollisionShape3D, CylinderShape3D, Engine, INode3D, Material,
    Mesh, Node3D, RefCounted, StaticBody3D,
};
use godot::prelude::*;

use super::solid::{clear_limbs, warnings_from_level};
use super::source::{SoundSource, SourceRig, sound};
use crate::fan_wave;
use crate::render::{self, Role};
use crate::sound_source::{Cadence, Spread, Voice, Volume};
use crate::{prop_shape, source_shape};

/// Hub height in meters: within the cane's reach. A build dimension the
/// pivot and the collider both hang from — not a tuning knob.
pub const HEAD_H: f32 = 1.15;

/// The guard ring's outer radius — the widest part of the swinging head,
/// and so what decides how far the housing reaches as it sweeps.
const GUARD_R: f32 = 0.44;

/// Standalone blueprint defaults for the fan's two semantic roles. Inside a
/// WaveLevel the grouping remains but numeric labels are derived per instance,
/// so two touching fans cannot melt merely because both have a shell.
const PREVIEW_ROLE_LABELS: [f64; 2] = [
    render::role_label(Role::Shell),
    render::role_label(Role::Moving),
];
const SHELL_ROLE: usize = 0;
const MOVING_ROLE: usize = 1;

/// The two built subtrees, named so a rebuilding ready() can free the
/// ghosts a Ctrl+D duplicate carries in (names are the only handle —
/// a duplicate reaches _ready as a fresh Rust object).
const LIMBS: [&str; 2] = ["FanPedestal", "FanPivot"];

/// The pedestal fan node. Scene limbs are built in `_ready` from the
/// injected acoustic-image skin; `update(t)` — driven by the level with the
/// simulated clock, like every animated thing — rides the pure motion
/// curves and fires the cadence into the injected pulse pool. The acoustic
/// voice is a set of designer `#[export]` knobs defaulting to the shipped
/// constants; the knobs are read when the fan enters the tree.
#[derive(GodotClass)]
#[class(tool, init, base=Node3D)]
pub struct SoundFan {
    /// The wave pool every sound enters — the `WaveCore` itself, upcast to
    /// `RefCounted`. The GDScript `Pulses` shim survives only in
    /// `game/tests/`. The fan only asks it to `emit`, dynamically.
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
        clear_limbs(self, &LIMBS);
        self.rig.clear();
        if Engine::singleton().is_editor_hint() {
            // blueprint mode: the same geometry the game outlines, skinless
            // (SourceRig::limb skips the override while data_mat is None).
            // Nothing ticks, emits, or registers here — advance() is only
            // ever called by the level at run time.
            self.build_pedestal();
            self.build_head();
            self.build_blades();
            return;
        }
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

    fn get_configuration_warnings(&self) -> PackedStringArray {
        warnings_from_level(&self.base().clone().upcast::<Node>())
    }
}

#[godot_api]
impl SoundFan {
    /// Callable mirror of the editor-only warning virtual.
    #[func]
    fn get_configuration_warnings(&self) -> PackedStringArray {
        INode3D::get_configuration_warnings(self)
    }

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

    /// Driven by the Rust level/composition root with the simulated clock —
    /// the engine-facing name for [`SoundSource::advance`]. It remains
    /// callable so boundary suites can exercise the same typed door.
    #[func]
    fn update(&mut self, t: f64) {
        SoundSource::advance(self, t);
    }

    /// Pedestal: base disc + pole, as static as the walls.
    fn build_pedestal(&mut self) {
        let mut pedestal = StaticBody3D::new_alloc();
        pedestal.set_name("FanPedestal");
        self.base_mut().add_child(&pedestal);
        let mut body = pedestal.clone().upcast::<Node3D>();
        let skin = self.data_mat.clone();
        let shell = render::role_label(Role::Shell);
        self.rig.limb(
            &mut body,
            &labelled_cyl(0.22, 0.06, shell),
            SHELL_ROLE,
            Vector3::new(0.0, 0.03, 0.0),
            Vector3::ZERO,
            skin.as_ref(),
        );
        self.rig.limb(
            &mut body,
            &labelled_cyl(0.03, HEAD_H, shell),
            SHELL_ROLE,
            Vector3::new(0.0, HEAD_H * 0.5, 0.0),
            Vector3::ZERO,
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
        pivot.set_name("FanPivot");
        pivot.set_position(Vector3::new(0.0, HEAD_H, 0.0));
        self.base_mut().add_child(&pivot);
        let mut head = AnimatableBody3D::new_alloc();
        pivot.add_child(&head);
        let mut head_node = head.clone().upcast::<Node3D>();
        let skin = self.data_mat.clone();
        let shell = render::role_label(Role::Shell);
        self.rig.limb(
            &mut head_node,
            &labelled_boxm(0.16, 0.16, 0.24, shell),
            SHELL_ROLE,
            Vector3::new(0.0, 0.0, 0.10),
            Vector3::ZERO,
            skin.as_ref(),
        );
        self.rig.limb(
            &mut head_node,
            &labelled_torus(0.40, GUARD_R, shell),
            SHELL_ROLE,
            Vector3::new(0.0, 0.0, -0.10),
            Vector3::new(PI * 0.5, 0.0, 0.0),
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
        let moving = render::role_label(Role::Moving);
        self.rig.limb(
            &mut spinner,
            &labelled_cyl(0.045, 0.08, moving),
            MOVING_ROLE,
            Vector3::ZERO,
            Vector3::new(PI * 0.5, 0.0, 0.0),
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
                &labelled_boxm(0.32, 0.11, 0.016, moving),
                MOVING_ROLE,
                Vector3::new(0.24, 0.0, 0.0),
                Vector3::ZERO,
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

    fn role_count(&self) -> u8 {
        PREVIEW_ROLE_LABELS.len() as u8
    }

    fn set_role_labels(&mut self, labels: &[f64]) {
        self.rig.set_role_labels(labels);
    }

    fn role_label(&self, role: usize) -> Option<f64> {
        self.rig.role_label(role)
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

    fn restore_appointment(&mut self, next: f64) {
        let interval = self.voice().cadence;
        self.rig.restore_cadence(Cadence::restore(interval, next));
    }
}

/// A capped cylinder, baked with a single constant label on every vertex —
/// replaces the `CylinderMesh` engine primitive (Task 7), which carries no
/// `CUSTOM0` channel to bake a label into. Reuses
/// [`prop_shape::column_triangles`]'s already-tested geometry outright,
/// discarding its bottom/top/flank ordinal (a source's own limb reads as
/// ONE label, not three). The pure triples wind counter-clockwise/outward;
/// the explicit ArrayMesh door converts them to Godot-clockwise order.
fn labelled_cyl(radius: f32, height: f32, label: f64) -> Gd<Mesh> {
    let label = label as f32;
    let triangles: Vec<(Vector3, Vector3, f32)> =
        prop_shape::column_triangles(radius, height * 0.5)
            .into_iter()
            .map(|(v, n, _ordinal)| (v, n, label))
            .collect();
    let mut mesh = ArrayMesh::new_gd();
    render::paint::resize_outward_triangle_surface(&mut mesh, &triangles);
    mesh.upcast()
}

/// A box, baked with a single constant label on every face — replaces the
/// `BoxMesh` engine primitive the same way, as a thin wrapper over
/// [`render::paint::labelled_box`] with all six faces given the one label.
fn labelled_boxm(x: f32, y: f32, z: f32, label: f64) -> Gd<Mesh> {
    let label = label as f32;
    render::paint::labelled_box(Vector3::new(x, y, z), Vector3::ZERO, [label; 6]).upcast()
}

/// A torus (donut), baked with a single constant label — replaces the
/// `TorusMesh` engine primitive: [`source_shape::torus_triangles`] is the
/// pure geometry, already proven to wind outward and converted at the
/// ArrayMesh boundary (load-bearing here, since
/// a source's limbs render through the acoustic-image skin's `cull_back`,
/// not the world skin's `cull_disabled` — see that module's own doc
/// comment).
fn labelled_torus(inner_radius: f32, outer_radius: f32, label: f64) -> Gd<Mesh> {
    let label = label as f32;
    let triangles: Vec<(Vector3, Vector3, f32)> =
        source_shape::torus_triangles(inner_radius, outer_radius)
            .into_iter()
            .map(|(v, n)| (v, n, label))
            .collect();
    let mut mesh = ArrayMesh::new_gd();
    render::paint::resize_outward_triangle_surface(&mut mesh, &triangles);
    mesh.upcast()
}
