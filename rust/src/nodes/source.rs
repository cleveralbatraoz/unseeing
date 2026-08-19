//! The sound-source abstraction at the engine boundary — the layer the fan
//! stopped being a special case in.
//!
//! The world's own sounds (the ones the hero did NOT make) all behave the
//! same way and differ only in their [`Voice`]: they are born as pulse kind
//! [`SOURCE_KIND`], their waves die at a wall like any other sound, and
//! they wear the always-on-top acoustic-image skin so their silhouette is
//! still felt through a wall as a dimmed ghost. A fan and a radio are two
//! voices, not two systems.
//!
//! WHY A TRAIT AND NOT A BASE CLASS. gdext cannot derive one registered
//! class from another, so "a fan IS a source" cannot be inheritance. It is
//! [`SoundSource`], a plain Rust trait, published to the engine by
//! `#[godot_dyn]`: that registers a `Class -> dyn SoundSource` upcast, and
//! [`godot::obj::Gd::try_dynify`] then turns any child node whose DYNAMIC
//! class implements it into a `DynGd<Node3D, dyn SoundSource>`. So the level
//! walks its children once and collects every source there is without ever
//! naming a concrete class — adding a third source class means writing that
//! class and nothing else. Dependency injection runs the same way: the level
//! hands each source the wave pool and the acoustic-image skin through the
//! trait, and a source never reaches out for either.
//!
//! WHAT LIVES HERE. Only the machinery every source shares: the cadence gate
//! that decides when a wave is born, the limb builder that tags each mesh
//! with its semantic role, the push of the per-object
//! standing image, and the one call into the wave pool. The LAWS are pure
//! and live in [`crate::sound_source`]; the BODIES — what a fan looks like,
//! what a radio looks like — live in the node files. Nothing in between.

use godot::classes::{Material, MeshInstance3D, Node3D, RefCounted};
use godot::prelude::*;

use super::solid::mesh_first_label;
use crate::render;
use crate::sound_source::{Cadence, SOURCE_KIND, Voice};

/// The two per-instance shader parameters carrying a source's STANDING
/// acoustic image. Both are INSTANCE uniforms, not material uniforms,
/// because all sources share one acoustic-image material — a material
/// uniform would make the quiet fan and the loud radio dim and brighten as
/// one. `data_xray.gdshader` declares them.
///
/// A source's STANDING loudness before any wall is considered — its
/// `Volume::image()`.
pub(crate) const VOLUME_PARAM: &str = "u_source_volume";

/// What survives of that image across the walls between the source and the
/// EYE: `SOURCE_THROUGH` per crossing. The other half, and it is delivered
/// separately on purpose — as one product it could only ever be a FLOOR
/// under the source's own wave reveal, and a floor loses. See
/// [`render::reveal::source_image`] for the law both halves now feed.
pub(crate) const MUFFLE_PARAM: &str = "u_source_muffle";

/// What the level needs of a sound source, whatever the thing actually is.
/// Implemented by every source node through `#[godot_dyn]`, which is what
/// lets the level hold them all as one list.
pub trait SoundSource {
    /// Where this source's waves are born, in world space, right now — the
    /// fan's spinning hub as the head sweeps, the radio's speaker cone. The
    /// level also measures the walls between the eye and THIS point to
    /// decide how muffled the source's image is.
    fn hub(&self) -> Vector3;

    /// This source's acoustic identity: volume, cadence, speed, spread.
    /// Read from the node's designer knobs, so a fan turned down in the
    /// Inspector answers differently.
    fn voice(&self) -> Voice;

    /// How many semantic limb roles this source paints. A fan has shell and
    /// moving roles; a radio has case and fascia roles. They enter the level's
    /// colourable separation graph as distinct classes so two touching
    /// same-class sources cannot melt merely because their role names match.
    fn role_count(&self) -> u8;

    /// Bake the level's chosen label for each semantic role back onto every
    /// limb in that role. Missing or non-finite labels retain that role's
    /// prior assignment (or its preview default before any assignment); extra
    /// labels are retained for a later generation but paint no absent limb.
    fn set_role_labels(&mut self, labels: &[f64]);

    /// Read one role's current label from an actual limb mesh. `None` when
    /// the role has no built limb or the mesh has no readable CUSTOM0 data.
    fn role_label(&self, role: usize) -> Option<f64>;

    /// The single injection point: the wave pool every sound enters and the
    /// acoustic-image skin every limb renders through. Called by the level
    /// BEFORE the source enters the tree, because a source that cannot
    /// sound or be seen must refuse to build rather than fail later.
    fn inject(&mut self, pulses: Gd<RefCounted>, skin: Gd<Material>);

    /// Advance this source's clockwork to `t` and emit whatever waves fall
    /// due. Driven by the level with the SIMULATED clock, like every
    /// animated thing, so movie-maker runs and time scaling stay correct.
    fn advance(&mut self, t: f64);

    /// How far this source's MOVING parts swing beyond the pose the level
    /// samples when it colours the world, in metres on the horizontal axes.
    ///
    /// The level connects every source-role class to the world and source
    /// roles touching this swept box. For a source that swings, one sampled
    /// pose is too small: a prop just outside it could meet the source for
    /// part of every cycle while a single-pose seam check reported green.
    /// Zero for a source that does not move.
    fn sweep_margin(&self) -> f64 {
        0.0
    }

    /// When this source's next wave is due, on the same simulated clock
    /// [`Self::advance`] is driven with — `None` when no appointment is
    /// being kept (a source that never built, or one whose cadence knob
    /// cannot fire). Read-only; nothing in the game asks, and the debug
    /// observer would otherwise have to guess a source's clockwork from its
    /// interval and the wall clock.
    ///
    /// The default answers `None` rather than inventing a date, so a source
    /// class that keeps its clock somewhere other than a [`SourceRig`] is
    /// honest about it until it says otherwise.
    fn next_emit(&self) -> Option<f64> {
        None
    }

    /// Set how strongly this source's standing image is felt: its own
    /// volume and the muffle of the walls between it and the eye, computed
    /// once per frame by the level and delivered SEPARATELY, as
    /// [`VOLUME_PARAM`] and [`MUFFLE_PARAM`].
    ///
    /// Separately because their product is not what the skin needs. A
    /// single pre-multiplied number can only enter the fragment as a floor
    /// under the source's own wave reveal, and that floor always loses: a
    /// source's hub is unwalled from its own body by construction, so the
    /// wave washing it is near full strength however many walls stand
    /// between it and the player. Kept apart, the muffle multiplies the
    /// whole image instead of competing with half of it.
    fn set_image(&mut self, image: render::reveal::SourceImage);

    /// Re-pin this source's beat appointment to a captured date. Called
    /// by the restorer AFTER the clock lands, so the jumped-clock law
    /// (one beat per jump) never fires on a restore. Required, not
    /// defaulted: a source that cannot restore its gate is a source a
    /// blob cannot carry, and the compiler says so at the source.
    fn restore_appointment(&mut self, next: f64);
}

/// The organs every sound source has, whatever its body looks like: the
/// cadence gate its clock runs through, and the mesh limbs it draws itself
/// with (kept so the standing image can be pushed to each one).
///
/// A source node owns one of these and its own `pulses`/`skin` handles; the
/// rig deliberately does NOT own those, so a node's injection surface stays
/// visible in its own fields where a designer and a test can see it.
#[derive(Default)]
pub(crate) struct SourceRig {
    limbs: Vec<(Gd<MeshInstance3D>, usize)>,
    assigned_labels: Vec<Option<f64>>,
    cadence: Cadence,
}

impl SourceRig {
    /// Book the first wave one interval out, when the source enters the
    /// tree and the designer's knobs are first known.
    pub(crate) fn tune(&mut self, voice: &Voice) {
        self.cadence = Cadence::every(voice.cadence);
    }

    /// Has a wave's moment come? One beat per cadence, never a backfilled
    /// burst after a stalled clock — the law lives in [`Cadence`].
    ///
    /// The voice is re-read on every beat, so the cadence knob is as live as
    /// volume, speed and cone width. Nothing about a running source is
    /// frozen at build time, and `slot_pressure` therefore always describes
    /// the source that is actually running.
    pub(crate) fn beat(&mut self, t: f64, voice: &Voice) -> Option<f64> {
        self.cadence.retune(voice.cadence);
        self.cadence.beat(t)
    }

    /// The appointment the gate is holding — see [`Cadence::next_at`], which
    /// owns the rule about when there is no appointment at all.
    pub(crate) fn next_beat(&self) -> Option<f64> {
        self.cadence.next_at()
    }

    /// Replace the rig's gate wholesale — the restore door. The limbs are
    /// untouched: geometry is derived from the scene, only the clock is
    /// state.
    pub(crate) fn restore_cadence(&mut self, cadence: Cadence) {
        self.cadence = cadence;
    }

    /// Build one limb: a mesh instance drawn through the injected skin,
    /// positioned and rotated in its parent. Remembered, so the standing
    /// image reaches it every frame. Returns the built instance so a caller
    /// that needs a stable handle (a rebuilding `ready()` freeing it by
    /// name) can name it.
    ///
    /// The caller's mesh carries a standalone preview default in CUSTOM0.
    /// If a level already assigned this role, a generation rebuild reapplies
    /// the retained derived value before the limb is exposed.
    pub(crate) fn limb(
        &mut self,
        parent: &mut Gd<Node3D>,
        mesh: &Gd<godot::classes::Mesh>,
        role: usize,
        at: Vector3,
        rotation: Vector3,
        skin: Option<&Gd<Material>>,
    ) -> Gd<MeshInstance3D> {
        let mut mi = MeshInstance3D::new_alloc();
        mi.set_mesh(mesh);
        if let Some(label) = self.assigned_labels.get(role).and_then(|label| *label)
            && let Some(current) = mi.get_mesh()
            && let Ok(mut current) = current.try_cast::<godot::classes::ArrayMesh>()
        {
            render::paint::relabel_constant(&mut current, label as f32);
            mi.set_mesh(&current);
        }
        if let Some(skin) = skin {
            mi.set_material_override(skin);
        }
        mi.set_position(at);
        mi.set_rotation(rotation);
        parent.add_child(&mi);
        self.limbs.push((mi.clone(), role));
        mi
    }

    /// Repaint each built limb with the graph-coloured label chosen for its
    /// semantic role. The node remains the sole owner of its mesh resources;
    /// the level supplies values and never reaches into limb structure.
    pub(crate) fn set_role_labels(&mut self, labels: &[f64]) {
        self.assigned_labels =
            render::paint_plan::update_role_labels(&self.assigned_labels, labels);
        for (limb, role) in &mut self.limbs {
            let Some(label) = self.assigned_labels.get(*role).and_then(|label| *label) else {
                continue;
            };
            let Some(mesh) = limb.get_mesh() else {
                continue;
            };
            let Ok(mut mesh) = mesh.try_cast::<godot::classes::ArrayMesh>() else {
                continue;
            };
            render::paint::relabel_constant(&mut mesh, label as f32);
            limb.set_mesh(&mesh);
        }
    }

    /// The label an actual built limb in `role` carries, never a mirrored
    /// copy of what the allocator intended to write.
    pub(crate) fn role_label(&self, role: usize) -> Option<f64> {
        self.limbs.iter().find_map(|(limb, own_role)| {
            (*own_role == role)
                .then(|| mesh_first_label(limb))
                .flatten()
        })
    }

    /// Push the standing acoustic image onto every limb this source built,
    /// as its two independent halves. Per instance, not per material: the
    /// world's sources share one skin, and each must dim by its OWN volume
    /// and its OWN walls.
    pub(crate) fn set_image(&mut self, image: render::reveal::SourceImage) {
        for (limb, _) in &mut self.limbs {
            limb.set_instance_shader_parameter(VOLUME_PARAM, &image.volume.to_variant());
            limb.set_instance_shader_parameter(MUFFLE_PARAM, &image.muffle.to_variant());
        }
    }

    /// Did this rig ever build anything? False for a source that refused to
    /// build uninjected — its `advance` must then be a harmless no-op.
    pub(crate) fn is_built(&self) -> bool {
        !self.limbs.is_empty()
    }

    /// Forget every limb handle. A rebuilding `ready()` frees the old
    /// limbs by name first; the rig must not keep pointers into them.
    pub(crate) fn clear(&mut self) {
        self.limbs.clear();
    }
}

/// Put one wave into the pool: a source's voice, born at `at`, aimed along
/// `aim` (which an even spread ignores), at time `t`.
///
/// The pool is reached dynamically, by name — the `WaveCore` itself,
/// upcast to `RefCounted`. The GDScript `Pulses` shim survives only in
/// `game/tests/`, and a source only ever asks it to `emit`, so both answer.
/// A silent voice is not asked at all: the pool rightly refuses a
/// zero-radius wave, and asking every cadence would be a steady drip of
/// refusals in the log.
pub(crate) fn sound(pulses: &mut Gd<RefCounted>, voice: &Voice, at: Vector3, aim: Vector3, t: f64) {
    if !voice.volume.audible() {
        return;
    }
    let wave = voice.wave(aim);
    pulses.call(
        "emit",
        &[
            i64::from(SOURCE_KIND).to_variant(),
            at.to_variant(),
            wave.range.to_variant(),
            wave.speed.to_variant(),
            wave.gain.to_variant(),
            t.to_variant(),
            wave.beam.to_variant(),
            wave.cos_half.to_variant(),
        ],
    );
}
