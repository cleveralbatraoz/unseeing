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
    BoxMesh, BoxShape3D, CollisionShape3D, CylinderMesh, INode3D, Material, Mesh, Node3D,
    RefCounted, StaticBody3D, TorusMesh,
};
use godot::prelude::*;

use super::source::{SoundSource, SourceRig, sound};
use crate::radio_wave;
use crate::sound_source::{Spread, Voice, Volume};

/// The case's full extent in meters: a set you could carry with one hand.
const CASE: Vector3 = Vector3::new(0.44, 0.26, 0.20);

/// Where the waves are born, in the node's own space: the middle of the
/// speaker cone, on the front face. An even spread makes the exact point
/// cosmetic for the waves themselves — but it is also the point the level
/// counts walls to when it dims the standing image, so it must be ON the
/// radio, not at its feet.
const HUB: Vector3 = Vector3::new(-0.11, 0.14, -0.10);

/// The case's flat object id — the source band's CASE id, the one worn by
/// the part that stands on world geometry (see `oid_palette`'s budget).
const RADIO_CASE_OID: f64 = 0.05;

/// The fascia's flat object id: grille, tuning scale, dials and antenna
/// read as one silhouette against the case. The source band's SHELL id,
/// reused from the fan's housing under the palette's own law — two objects
/// that can never touch may share an id, and these two stand rooms apart.
const RADIO_FACE_OID: f64 = 0.33;

/// Both ids the radio paints itself with, so the level can keep whatever
/// it stands on clear of them.
const OIDS: [f64; 2] = [RADIO_CASE_OID, RADIO_FACE_OID];

/// The radio node. Scene limbs are built in `_ready` from the injected
/// acoustic-image skin; `update(t)` — driven by the level with the
/// simulated clock — fires the cadence into the injected pulse pool. There
/// are no motion curves: a radio sits still and sounds.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
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
}

#[godot_api]
impl SoundRadio {
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
        self.base_mut().add_child(&body);
        let mut node = body.clone().upcast::<Node3D>();
        let skin = self.data_mat.clone();
        let lift = Vector3::new(0.0, CASE.y * 0.5, 0.0);
        self.rig.limb(
            &mut node,
            &boxm(CASE),
            lift,
            Vector3::ZERO,
            RADIO_CASE_OID,
            skin.as_ref(),
        );
        let mut shape = BoxShape3D::new_gd();
        shape.set_size(CASE);
        let mut collider = CollisionShape3D::new_alloc();
        collider.set_shape(&shape);
        collider.set_position(lift);
        body.add_child(&collider);
    }

    /// The fascia: speaker grille, tuning scale, two dials and the
    /// antenna. All one object id, so the set reads as a face against the
    /// case rather than as five loose parts.
    fn build_fascia(&mut self) {
        let skin = self.data_mat.clone();
        let mut node = self.base().clone().upcast::<Node3D>();
        let face = -CASE.z * 0.5 - 0.001; // a hair proud of the front face
        let flat = Vector3::new(PI * 0.5, 0.0, 0.0); // a disc facing front

        let mut grille = TorusMesh::new_gd();
        grille.set_inner_radius(0.052);
        grille.set_outer_radius(0.086);
        self.rig.limb(
            &mut node,
            &grille.upcast::<Mesh>(),
            Vector3::new(HUB.x, HUB.y, face),
            flat,
            RADIO_FACE_OID,
            skin.as_ref(),
        );
        self.rig.limb(
            &mut node,
            &boxm(Vector3::new(0.15, 0.05, 0.014)),
            Vector3::new(0.11, 0.195, face),
            Vector3::ZERO,
            RADIO_FACE_OID,
            skin.as_ref(),
        );
        for x in [0.055_f32, 0.165] {
            self.rig.limb(
                &mut node,
                &cyl(0.030, 0.026),
                Vector3::new(x, 0.075, face),
                flat,
                RADIO_FACE_OID,
                skin.as_ref(),
            );
        }
        // the antenna, leaning back off the case's top corner
        let tilt = 0.32_f32;
        let half = 0.28_f32;
        self.rig.limb(
            &mut node,
            &cyl(0.008, half * 2.0),
            Vector3::new(
                CASE.x * 0.5 - 0.04,
                CASE.y + half * tilt.cos(),
                half * tilt.sin(),
            ),
            Vector3::new(tilt, 0.0, 0.0),
            RADIO_FACE_OID,
            skin.as_ref(),
        );
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

    fn oids(&self) -> &'static [f64] {
        &OIDS
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
fn boxm(size: Vector3) -> Gd<Mesh> {
    let mut b = BoxMesh::new_gd();
    b.set_size(size);
    b.upcast()
}
