//! The radio — a [`SoundSource`] with a body, and the proof that "sound
//! source" is an abstraction rather than a word for "the fan". Every law it
//! obeys it obeys through [`super::source`]: kind-3 waves that pass walls
//! muffled, the always-on-top acoustic image, the volume ladder, the
//! cadence gate. Not one of them is written twice.
//!
//! What makes it a RADIO and not a fan is two things, and they are exactly
//! the two the pure [`radio_wave`] module pins:
//!
//! - **It is the loudest thing in the world** (full volume against the
//!   fan's three quarters), so by the volume law it reaches further and is
//!   felt more strongly through a wall.
//! - **It does not aim.** Nothing on a radio moves and nothing points: its
//!   waves are an even sphere, so walking around it changes nothing, where
//!   walking around a fan changes everything.
//!
//! The body is a tabletop wireless set: a case, a speaker grille, a tuning
//! scale, two dials and a telescopic antenna. The case carries the collider
//! — the cane and the echo rays strike a radio like anything else in the
//! world — and stands on whatever the designer put under it.

use std::f32::consts::PI;

use godot::classes::{
    ArrayMesh, BoxShape3D, CollisionShape3D, Engine, INode3D, Material, Mesh, Node3D, RefCounted,
    StaticBody3D,
};
use godot::prelude::*;

use super::solid::{clear_limbs, warnings_from_level};
use super::source::{SoundSource, SourceRig, sound};
use crate::radio_wave;
use crate::render::{self, Role};
use crate::sound_source::{Cadence, Spread, Voice, Volume};
use crate::{prop_shape, source_shape};

/// The case's full extent in meters: a set you could carry with one hand.
const CASE: Vector3 = Vector3::new(0.44, 0.26, 0.20);

/// Where the waves are born, in the node's own space: the middle of the
/// speaker cone, on the front face. An even spread makes the exact point
/// cosmetic for the waves themselves — but it is also the point the level
/// counts walls to when it dims the standing image, so it must be ON the
/// radio, not at its feet.
const HUB: Vector3 = Vector3::new(-0.11, 0.14, -0.10);

/// Standalone blueprint defaults for the case and fascia semantic roles.
/// Inside a WaveLevel the grouping stays fixed while numeric labels are
/// derived per instance, so a copied radio may touch its twin and keep a seam.
const PREVIEW_ROLE_LABELS: [f64; 2] = [
    render::role_label(Role::Case),
    render::role_label(Role::Shell),
];
const CASE_ROLE: usize = 0;
const FASCIA_ROLE: usize = 1;

/// The six built subtrees, named so a rebuilding ready() can free the
/// ghosts a Ctrl+D duplicate carries in (names are the only handle — a
/// duplicate reaches _ready as a fresh Rust object). The case is a
/// StaticBody3D child of the radio node; the five fascia limbs are
/// MeshInstance3D children of the radio node itself (`build_fascia`
/// parents them straight to `self.base()`, not to a wrapper group).
const LIMBS: [&str; 6] = [
    "RadioCase",
    "RadioGrille",
    "RadioTuner",
    "RadioDialA",
    "RadioDialB",
    "RadioAntenna",
];

/// The radio node. Scene limbs are built in `_ready` from the injected
/// acoustic-image skin; `update(t)` — driven by the level with the
/// simulated clock — fires the cadence into the injected pulse pool. There
/// are no motion curves: a radio sits still and sounds.
#[derive(GodotClass)]
#[class(tool, init, base=Node3D)]
pub struct SoundRadio {
    /// The wave pool every sound enters — asked only to `emit`, and only
    /// dynamically, so the GDScript shim and the Rust core both answer.
    #[var]
    pulses: Option<Gd<RefCounted>>,
    /// The acoustic-image skin every limb renders through.
    #[var]
    data_mat: Option<Gd<Material>>,
    /// Loudness in (0, 1]: the ONE knob the radio's carrying power hangs
    /// from. By the volume law it is also how far its waves reach
    /// (`FULL_REACH` × volume) and how strongly its silhouette is felt
    /// through a wall. Shipped at the top of the ladder — a radio is
    /// louder than a fan, and the hero must be able to hear which is which.
    #[export(range = (0.0, 1.0))]
    #[init(val = radio_wave::RADIO_VOLUME)]
    volume: f64,
    /// Seconds between waves. Lazier than the fan's sweep: an even sphere
    /// fills every direction at once, so fewer births still read as one
    /// continuous presence — and an even wave that reaches twelve meters
    /// is the most expensive thing in the pool.
    #[export(range = (0.05, 10.0, 0.05, or_greater))]
    #[init(val = radio_wave::RADIO_CADENCE)]
    cadence: f64,
    /// Wavefront speed, m/s.
    #[export(range = (0.5, 20.0, 0.1, or_greater))]
    #[init(val = radio_wave::RADIO_SPEED)]
    wave_speed: f64,
    rig: SourceRig,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for SoundRadio {
    fn ready(&mut self) {
        clear_limbs(self, &LIMBS);
        self.rig.clear();
        if Engine::singleton().is_editor_hint() {
            // blueprint mode: the same geometry the game outlines, skinless
            // (SourceRig::limb skips the override while data_mat is None).
            // Nothing ticks, emits, or registers here — advance() is only
            // ever called by the level at run time.
            self.build_case();
            self.build_fascia();
            return;
        }
        // no silent nulls: without the pool and the acoustic-image skin the
        // radio can neither sound nor be seen — refuse to build instead of
        // crashing later
        if self.pulses.is_none() || self.data_mat.is_none() {
            godot_error!("SoundRadio: pulses/data_mat not injected — radio disabled");
            return;
        }
        let voice = self.voice();
        self.rig.tune(&voice);
        self.build_case();
        self.build_fascia();
    }

    fn get_configuration_warnings(&self) -> PackedStringArray {
        warnings_from_level(&self.base().clone().upcast::<Node>())
    }
}

#[godot_api]
impl SoundRadio {
    /// Callable mirror of the editor-only warning virtual.
    #[func]
    fn get_configuration_warnings(&self) -> PackedStringArray {
        INode3D::get_configuration_warnings(self)
    }

    /// Where the waves are born in the node's own space — a build
    /// dimension, served as a static method because ClassDB registers
    /// integer constants only.
    #[func]
    fn hub_offset() -> Vector3 {
        HUB
    }

    /// Meters this radio's waves reach, derived from its volume — the
    /// designer's read-back of the volume law.
    #[func]
    fn reach(&self) -> f64 {
        self.voice().volume.reach()
    }

    /// Slots this radio's voice occupies in the 64-slot pool at steady
    /// state — the budget a designer turning knobs must not blow.
    #[func]
    fn slot_pressure(&self) -> f64 {
        self.voice().slot_pressure()
    }

    /// Driven by the level with the simulated clock — the engine-facing
    /// name for [`SoundSource::advance`], matching the fan's.
    #[func]
    fn update(&mut self, t: f64) {
        SoundSource::advance(self, t);
    }

    /// The case: one box, with the collider that lets the cane find it.
    fn build_case(&mut self) {
        let mut body = StaticBody3D::new_alloc();
        body.set_name("RadioCase");
        self.base_mut().add_child(&body);
        let mut node = body.clone().upcast::<Node3D>();
        let skin = self.data_mat.clone();
        let lift = Vector3::new(0.0, CASE.y * 0.5, 0.0);
        let case_label = render::role_label(Role::Case);
        self.rig.limb(
            &mut node,
            &labelled_boxm(CASE, case_label),
            CASE_ROLE,
            lift,
            Vector3::ZERO,
            skin.as_ref(),
        );
        let mut shape = BoxShape3D::new_gd();
        shape.set_size(CASE);
        let mut collider = CollisionShape3D::new_alloc();
        collider.set_shape(&shape);
        collider.set_position(lift);
        body.add_child(&collider);
    }

    /// The fascia: speaker grille, tuning scale, two dials and the antenna.
    /// All share one semantic role, whose numeric label the level derives for
    /// this radio instance, so the set reads as one face against the case.
    fn build_fascia(&mut self) {
        let skin = self.data_mat.clone();
        let mut node = self.base().clone().upcast::<Node3D>();
        let face = -CASE.z * 0.5 - 0.001; // a hair proud of the front face
        let flat = Vector3::new(PI * 0.5, 0.0, 0.0); // a disc facing front
        let shell = render::role_label(Role::Shell);

        self.rig
            .limb(
                &mut node,
                &labelled_torus(0.052, 0.086, shell),
                FASCIA_ROLE,
                Vector3::new(HUB.x, HUB.y, face),
                flat,
                skin.as_ref(),
            )
            .set_name("RadioGrille");
        self.rig
            .limb(
                &mut node,
                &labelled_boxm(Vector3::new(0.15, 0.05, 0.014), shell),
                FASCIA_ROLE,
                Vector3::new(0.11, 0.195, face),
                Vector3::ZERO,
                skin.as_ref(),
            )
            .set_name("RadioTuner");
        for (x, name) in [0.055_f32, 0.165]
            .into_iter()
            .zip(["RadioDialA", "RadioDialB"])
        {
            self.rig
                .limb(
                    &mut node,
                    &labelled_cyl(0.030, 0.026, shell),
                    FASCIA_ROLE,
                    Vector3::new(x, 0.075, face),
                    flat,
                    skin.as_ref(),
                )
                .set_name(name);
        }
        // the antenna, leaning back off the case's top corner
        let tilt = 0.32_f32;
        let half = 0.28_f32;
        self.rig
            .limb(
                &mut node,
                &labelled_cyl(0.008, half * 2.0, shell),
                FASCIA_ROLE,
                Vector3::new(
                    CASE.x * 0.5 - 0.04,
                    CASE.y + half * tilt.cos(),
                    half * tilt.sin(),
                ),
                Vector3::new(tilt, 0.0, 0.0),
                skin.as_ref(),
            )
            .set_name("RadioAntenna");
    }
}

#[godot_dyn]
impl SoundSource for SoundRadio {
    /// The speaker cone, carried into world space by whatever transform
    /// the designer gave the set.
    fn hub(&self) -> Vector3 {
        self.base().get_global_transform() * HUB
    }

    fn voice(&self) -> Voice {
        Voice {
            volume: Volume::new(self.volume),
            cadence: self.cadence,
            speed: self.wave_speed,
            // structural, not a knob: a radio has no front to aim
            spread: Spread::Even,
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

    fn next_emit(&self) -> Option<f64> {
        self.rig.next_beat()
    }

    fn inject(&mut self, pulses: Gd<RefCounted>, skin: Gd<Material>) {
        self.pulses = Some(pulses);
        self.data_mat = Some(skin);
    }

    /// Nothing to animate — just the cadence. The aim handed to the voice
    /// is the node's own facing, which an even spread throws away; it is
    /// passed anyway so the trait reads the same for every source and a
    /// future directed radio would need no new plumbing.
    fn advance(&mut self, t: f64) {
        if !self.rig.is_built() {
            return; // _ready refused to build: nothing to emit
        }
        let voice = self.voice();
        let Some(at) = self.rig.beat(t, &voice) else {
            return;
        };
        let hub = self.hub();
        let aim = -self.base().get_global_transform().basis.col_c();
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
fn labelled_boxm(size: Vector3, label: f64) -> Gd<Mesh> {
    let label = label as f32;
    render::paint::labelled_box(size, Vector3::ZERO, [label; 6]).upcast()
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
