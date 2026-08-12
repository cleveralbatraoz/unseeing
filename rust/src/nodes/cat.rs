//! The companion cat as an engine node — the second creature the world
//! carries, and the first with a mind of its own. A designer drops a
//! WaveCat into a scene, picks a seed and a roam size, and the cat
//! lives: wandering its patch of floor, pausing, sitting, losing
//! interest in blocked paths — all of it deterministic under the seed,
//! replaying bit-for-bit.
//!
//! Like every creature here it is OUTLINE-ONLY, and revealed only while
//! waves sweep it. Its fore paws speak as it walks — soft kind-2 pulses,
//! the least precious slot class — so a walking cat paints its own faint
//! footprints of light and blinks its own body into outline: a little
//! walking lantern the blind hero can hear coming.
//!
//! All laws live in the pure modules — [`cat_brain`] decides,
//! [`cat_gait`] steps, [`cat_body`] poses — cargo-tested without a
//! Godot runtime. This file only carries values across the boundary:
//! physics context for movement and emission, immediate-mesh rebuilds
//! for the silhouette. The clock is handed, never poked: the composition
//! root will advance `tick(now)` like it does the player's.

use godot::classes::{
    ArrayMesh, CapsuleShape3D, CharacterBody3D, CollisionShape3D, Engine, ICharacterBody3D,
    Material, MeshInstance3D,
};
use godot::prelude::*;

use super::limbs::{LimbBuf, sphere, sphere_lod, tube, tube_res};
use super::solid::clear_limbs;
use crate::cat_body::{self, CatPose, Tail};
use crate::cat_brain::{CatBrain, RoamRect};
use crate::cat_gait::{self, CatGait};
use crate::render::{self, Role};
use crate::reproduce::blob::CatCapture;
use crate::sound_source::Cadence;

/// Collider radius — small enough to slip between furniture legs.
const COL_RADIUS: f32 = 0.11;

/// Collider height; its bottom floats a hair above the floor, like the
/// player's capsule — the flat map means nothing ever presses down.
const COL_HEIGHT: f32 = 0.34;

/// The sit blend's ease rate, 1/s — a cat settles, it does not snap.
const SIT_EASE: f64 = 3.0;

/// The two built limbs, named so a rebuilding ready() can free the ghosts a
/// Ctrl+D duplicate carries in (names are the only handle — a duplicate
/// reaches _ready as a fresh Rust object). Both the editor blueprint build
/// and the runtime build use these same two names.
const LIMBS: [&str; 2] = ["CatCollider", "CatSkin"];

/// The companion cat. Inject `pulses` and `data_mat` before adding to
/// the tree (children run `_ready` first, and the cat refuses to build
/// uninjected); the seed and roam size are designer knobs.
#[derive(GodotClass)]
#[class(tool, init, base=CharacterBody3D)]
pub struct WaveCat {
    /// The wave pool every sound enters — the GDScript `Pulses` shim
    /// today, a direct `WaveCore` tomorrow; the cat only asks it to
    /// `emit`, dynamically, so both answer.
    #[var]
    pulses: Option<Gd<RefCounted>>,
    /// The data-pass material — the world is outline-only, and only
    /// this pass makes anything visible.
    #[var]
    data_mat: Option<Gd<Material>>,
    /// The whimsy seed: same seed, same life. Two cats want two seeds.
    #[export(range = (0.0, 999999.0))]
    #[init(val = 7)]
    seed: i64,
    /// Full extents of the floor rectangle the cat roams, centered on
    /// where it stands when it enters the tree.
    #[export(range = (1.0, 30.0, 0.5, suffix = " m"))]
    #[init(val = Vector2::new(6.0, 6.0))]
    roam_size: Vector2,
    #[init(val = ArrayMesh::new_gd())]
    mesh: Gd<ArrayMesh>,
    /// The frame's raw triangle geometry — cleared and refilled every
    /// rebuild rather than allocated fresh: `Vec::clear` keeps its
    /// capacity, so once this has grown to the cat's steady-state
    /// vertex count it never allocates again for the rest of its life.
    #[init(val = Vec::new())]
    tri_buf: LimbBuf,
    brain: Option<CatBrain>,
    gait: Option<CatGait>,
    tail: Option<Tail>,
    pose: Option<CatPose>,
    /// The idle-presence cadence gate — fires the cat's slow heartbeat
    /// pulse so a standing cat never sinks into full black.
    presence: Cadence,
    sit: f64,
    now: f64,
    sim_t: f64,
    /// The body position at the START of the last physics tick, before
    /// move_and_slide — so this tick's `pos - last_pos` is the planar
    /// distance the body ACTUALLY covered last tick, the brain's honest
    /// progress feed (never zero-across-the-wrong-interval).
    last_pos: Vector3,
    /// The pose changes only on a physics tick (60 Hz); this marks a
    /// fresh pose so `process()` rebuilds the silhouette once per tick,
    /// not once per rendered frame — no wasted rebuilds above 60 Hz.
    mesh_dirty: bool,
    base: Base<CharacterBody3D>,
}

#[godot_api]
impl ICharacterBody3D for WaveCat {
    fn ready(&mut self) {
        clear_limbs(self, &LIMBS);
        if Engine::singleton().is_editor_hint() {
            // blueprint mode: one standing pose, frozen. The mesh is built
            // in LOCAL space (pose seeded at the origin) so the silhouette
            // rides the node when the designer drags it; the runtime mesh
            // stays world-space + top_level as before. No brain, no clock:
            // an editor-ticking cat would walk the viewport and Ctrl+S
            // would save its drift into the scene.
            self.base_mut().set_physics_process(false);
            self.base_mut().set_process(false);
            self.build_editor_pose();
            return;
        }
        // no silent nulls: without the pool and the data-pass material
        // the cat can neither sound nor be seen — refuse to build
        // instead of crashing later
        if self.pulses.is_none() || self.data_mat.is_none() {
            godot_error!("WaveCat: pulses/data_mat not injected — cat disabled");
            self.base_mut().set_physics_process(false);
            self.base_mut().set_process(false);
            return;
        }
        let mut col = CollisionShape3D::new_alloc();
        col.set_name("CatCollider");
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(COL_RADIUS);
        capsule.set_height(COL_HEIGHT);
        col.set_shape(&capsule);
        col.set_position(Vector3::new(0.0, COL_HEIGHT * 0.5 + 0.02, 0.0));
        self.base_mut().add_child(&col);

        let mut mi = MeshInstance3D::new_alloc();
        mi.set_name("CatSkin");
        mi.set_mesh(&self.mesh.clone());
        mi.set_material_override(self.data_mat.as_ref());
        // one flat label for the whole cat: the outline post-pass draws it
        // as a single unified silhouette, never a pile of joint circles.
        // CUSTOM0 (baked below, every rebuild) is what the shader reads for
        // G directly — no per-instance bridge to keep in step any more.
        // the mesh mutates every frame in world space; never frustum-cull
        // it by its stale local bounds
        mi.set_extra_cull_margin(16384.0);
        mi.set_as_top_level(true);
        self.base_mut().add_child(&mi);

        // the brain, gait and mesh all work in WORLD space (the roam rect
        // and velocity are world), so read the world heading — a cat under
        // a rotated room or grouping folder still faces where the designer
        // aimed it
        let pos = self.base().get_global_position();
        let yaw = f64::from(self.base().get_global_rotation().y);
        let rect = RoamRect::around(
            pos,
            f64::from(self.roam_size.x),
            f64::from(self.roam_size.y),
        );
        self.brain = Some(CatBrain::new(self.seed as u64, rect, yaw));
        let mut gait = CatGait::new(pos, yaw);
        let frame = gait.advance(0.0, pos, yaw, 0.0);
        let pose = CatPose::from_gait(pos, yaw, &frame, 0.0);
        let sk = cat_body::skeleton(&pose);
        let tail = Tail::new(sk.tail_root, sk.tail_back, rightward(yaw));
        self.tail = Some(tail);
        self.gait = Some(gait);
        self.pose = Some(pose);
        self.presence = Cadence::every(cat_gait::PRESENCE_EVERY);
        self.last_pos = pos;
        // built HERE rather than left for the first process() tick: the
        // mesh's CUSTOM0 is the shader's own G-channel source now (no
        // per-instance uniform to carry the label in the meantime), so a
        // census or an observer reading this cat before a frame has ever
        // ticked must already find a real, painted silhouette.
        self.build_mesh(&pose, &tail);
        self.mesh_dirty = false;
    }

    fn physics_process(&mut self, dt: f64) {
        let (Some(mut brain), Some(mut gait), Some(mut tail)) =
            (self.brain.take(), self.gait.take(), self.tail.take())
        else {
            return; // _ready refused: nothing to think with
        };
        let pos = self.base().get_global_position();
        // progress = |pos_now - pos_at_last_tick_start| = the planar
        // distance actually covered last tick (last_pos is stored PRE-move
        // below), so a wall-blocked cat honestly reads as making none
        let progress =
            f64::from(Vector2::new(pos.x - self.last_pos.x, pos.z - self.last_pos.z).length());
        self.last_pos = pos;
        let drive = brain.advance(dt, pos, progress);

        // command a WORLD heading: velocity and the world-space silhouette
        // both read drive.yaw as world, so the body's yaw must be world too
        let mut grot = self.base().get_global_rotation();
        grot.y = drive.yaw as f32;
        self.base_mut().set_global_rotation(grot);
        let fw = forward(drive.yaw);
        self.base_mut().set_velocity(fw * (drive.speed as f32));
        self.base_mut().move_and_slide();

        let new_pos = self.base().get_global_position();
        let moved = f64::from(Vector2::new(new_pos.x - pos.x, new_pos.z - pos.z).length());
        let actual_speed = if dt > 0.0 { moved / dt } else { 0.0 };
        let frame = gait.advance(dt, new_pos, drive.yaw, actual_speed);

        self.sit += ((if drive.sitting { 1.0 } else { 0.0 }) - self.sit) * (dt * SIT_EASE).min(1.0);
        self.sim_t += dt;
        // tail sway: riding the stride while walking, a slow breath while
        // still
        let sway = 0.22 * (frame.phase * std::f64::consts::TAU).sin() * frame.amp
            + 0.10 * (self.sim_t * 0.9).sin() * (1.0 - frame.amp);

        let pose = CatPose::from_gait(new_pos, drive.yaw, &frame, self.sit);
        let sk = cat_body::skeleton(&pose);
        tail.advance(
            dt,
            sk.tail_root,
            sk.tail_back,
            rightward(drive.yaw),
            self.sit,
            sway,
        );

        // the lead fore paw speaks each stride; the others are silent
        let now = self.now;
        for c in frame
            .contacts
            .iter()
            .filter(|c| cat_gait::paw_sounds(c.leg))
        {
            self.emit_wave(
                Vector3::new(c.at.x, 0.02, c.at.z),
                cat_gait::PAW_RANGE,
                cat_gait::PAW_GAIN,
                now,
            );
        }
        // the idle heartbeat: a faint bloom from the chest on a slow beat,
        // walking or still, so the hero can always find the cat
        if self.presence.beat(now).is_some() {
            let chest = Vector3::new(new_pos.x, cat_gait::PRESENCE_HEIGHT as f32, new_pos.z);
            self.emit_wave(
                chest,
                cat_gait::PRESENCE_RANGE,
                cat_gait::PRESENCE_GAIN,
                now,
            );
        }

        self.pose = Some(pose);
        self.brain = Some(brain);
        self.gait = Some(gait);
        self.tail = Some(tail);
        self.mesh_dirty = true;
    }

    fn process(&mut self, _dt: f64) {
        if !self.mesh_dirty {
            return; // pose unchanged since the last rebuild — no wasted work
        }
        let (Some(pose), Some(tail)) = (self.pose, self.tail) else {
            return; // no physics tick yet: nothing to draw
        };
        self.build_mesh(&pose, &tail);
        self.mesh_dirty = false;
    }
}

#[godot_api]
impl WaveCat {
    /// The clock is handed, never poked: the composition root advances
    /// the simulated time here every frame, exactly like the player's.
    ///
    /// `pub(super)`: the root's own `process()` drives every cat's clock
    /// through a typed handle, the same precedent
    /// `UnseeingPlayer::tick`/`HeroBody::update` already set.
    #[func]
    pub(super) fn tick(&mut self, now_t: f64) {
        self.now = now_t;
    }

    /// Paw wave reach in meters — the voice constant, served as a static
    /// method: ClassDB registers integer constants only.
    #[func]
    fn paw_range() -> f64 {
        cat_gait::PAW_RANGE
    }

    /// Paw wavefront speed, m/s — static-method constant, same reason.
    #[func]
    fn paw_speed() -> f64 {
        cat_gait::PAW_SPEED
    }

    /// Paw wave loudness — static-method constant, same reason.
    #[func]
    fn paw_gain() -> f64 {
        cat_gait::PAW_GAIN
    }

    /// Idle-presence wave reach in meters — static-method constant.
    #[func]
    fn presence_range() -> f64 {
        cat_gait::PRESENCE_RANGE
    }

    /// Idle-presence loudness — static-method constant.
    #[func]
    fn presence_gain() -> f64 {
        cat_gait::PRESENCE_GAIN
    }

    /// Idle-presence cadence in seconds — static-method constant.
    #[func]
    fn presence_every() -> f64 {
        cat_gait::PRESENCE_EVERY
    }

    /// The four paw world positions, LF RF LH RH — the suites' observable.
    #[func]
    fn paw_positions(&self) -> PackedVector3Array {
        self.pose
            .map(|p| PackedVector3Array::from(&p.paws[..]))
            .unwrap_or_default()
    }

    /// The current mood as an integer: 0 roaming, 1 pausing, 2 sitting.
    #[func]
    fn mood(&self) -> i64 {
        use crate::cat_brain::Mood;
        match self.brain.as_ref().map(CatBrain::mood) {
            Some(Mood::Roam) => 0,
            Some(Mood::Pause) => 1,
            Some(Mood::Sit) => 2,
            None => -1,
        }
    }

    /// The silhouette's baked mesh — observable for mesh-sanity pins.
    #[func]
    fn cat_mesh(&self) -> Gd<ArrayMesh> {
        self.mesh.clone()
    }

    /// The cat as data — None when _ready refused (uninjected) and the
    /// brain never existed, which the blob reports as a refusal, never
    /// as a default cat.
    pub(crate) fn capture_state(&self) -> Option<CatCapture> {
        let brain = self.brain.as_ref()?;
        let gait = self.gait.as_ref()?;
        let tail = self.tail.as_ref()?;
        let pose = self.pose.as_ref()?;
        Some(CatCapture {
            position: self.base().get_global_position(),
            yaw: f64::from(self.base().get_global_rotation().y),
            velocity: self.base().get_velocity(),
            brain: brain.capture(),
            gait: gait.capture(),
            tail: *tail.nodes(),
            pose: *pose,
            presence_next: self.presence.next_at().unwrap_or(f64::NAN),
            sit: self.sit,
            sim_t: self.sim_t,
            last_pos: self.last_pos,
        })
    }

    /// Place a built cat into a captured mid-life state. Callers hold the
    /// tree frozen; the next physics tick resumes the captured life.
    pub(crate) fn restore_state(&mut self, capture: &CatCapture) {
        self.base_mut().set_global_position(capture.position);
        let mut rot = self.base().get_global_rotation();
        rot.y = capture.yaw as f32;
        self.base_mut().set_global_rotation(rot);
        self.base_mut().set_velocity(capture.velocity);
        self.brain = Some(CatBrain::restore(capture.brain));
        self.gait = Some(CatGait::restore(capture.gait));
        self.tail = Some(Tail::restore(capture.tail));
        self.pose = Some(capture.pose);
        // presence_next NaN — a cat that never beat — round-trips through
        // Cadence::restore(interval, NaN), whose next_at() returns None
        // again: the poison repair in beat() re-books it exactly as it
        // would have.
        self.presence = Cadence::restore(cat_gait::PRESENCE_EVERY, capture.presence_next);
        self.sit = capture.sit;
        self.sim_t = capture.sim_t;
        self.last_pos = capture.last_pos;
        self.mesh_dirty = true;
    }

    /// One of the cat's own waves into the pool: kind 2 (footstep — the
    /// least precious slot class), omnidirectional, no reflections — a
    /// whisper that reveals the cat and a small circle of floor, not the
    /// room. Both the paw steps and the idle heartbeat speak through here,
    /// differing only in reach and loudness.
    fn emit_wave(&mut self, at: Vector3, range: f64, gain: f64, now: f64) {
        let Some(pulses) = self.pulses.as_mut() else {
            return; // unreachable past the _ready guard; total anyway
        };
        pulses.call(
            "emit",
            &[
                2_i64.to_variant(),
                at.to_variant(),
                range.to_variant(),
                cat_gait::PAW_SPEED.to_variant(),
                gain.to_variant(),
                now.to_variant(),
                Vector3::ZERO.to_variant(),
                (-2.0_f64).to_variant(),
            ],
        );
    }

    /// Blueprint mode: build the same two limbs the runtime path does, but
    /// in LOCAL space around the origin (no material, no top-level, no
    /// cull margin), then write one frozen standing pose into the mesh.
    /// The gait, tail and pose are thrown away the moment the mesh is
    /// written — the persistent `Option` fields stay `None`, exactly as
    /// they are for any node whose `_ready` refused to build, and nothing
    /// reads them because processing is disabled before this runs. No
    /// brain is built here at all: a frozen standing pose needs no roam
    /// decision to render.
    fn build_editor_pose(&mut self) {
        let mut col = CollisionShape3D::new_alloc();
        col.set_name("CatCollider");
        let mut capsule = CapsuleShape3D::new_gd();
        capsule.set_radius(COL_RADIUS);
        capsule.set_height(COL_HEIGHT);
        col.set_shape(&capsule);
        col.set_position(Vector3::new(0.0, COL_HEIGHT * 0.5 + 0.02, 0.0));
        self.base_mut().add_child(&col);

        let mut mi = MeshInstance3D::new_alloc();
        mi.set_name("CatSkin");
        mi.set_mesh(&self.mesh.clone());
        self.base_mut().add_child(&mi);

        let pos = Vector3::ZERO;
        let yaw = 0.0_f64;
        let mut gait = CatGait::new(pos, yaw);
        let frame = gait.advance(0.0, pos, yaw, 0.0);
        let pose = CatPose::from_gait(pos, yaw, &frame, 0.0);
        let sk = cat_body::skeleton(&pose);
        let tail = Tail::new(sk.tail_root, sk.tail_back, rightward(yaw));
        self.build_mesh(&pose, &tail);
    }

    /// The whole silhouette, rebuilt for this frame's skeleton: torso
    /// line, neck and head, ears, whiskers, four bent legs, the tail
    /// chain — smooth tubes and spheres, one clean outline per shape.
    /// Small joints use the radius-tiered [`sphere_lod`] and whiskers the
    /// low-segment [`tube_res`]: the pea-sized parts read identically at a
    /// fraction of the per-vertex FFI cost the wasm build feels.
    ///
    /// Every vertex carries the SAME [`Role::Cat`] label in `CUSTOM0` — one
    /// silhouette, exactly what the shader's G channel reads for the whole
    /// mesh instance. `tri_buf` is cleared and refilled here rather than
    /// rebuilt fresh, so a cat that has been alive a few frames allocates
    /// nothing more to keep drawing itself.
    fn build_mesh(&mut self, pose: &CatPose, tail: &Tail) {
        let sk = cat_body::skeleton(pose);
        let label = render::role_label(Role::Cat) as f32;
        self.tri_buf.clear();
        // the torso line, chest proud of hip — the big shapes stay full-res
        tube(&mut self.tri_buf, sk.chest, sk.hip, 0.068, 0.062, label);
        sphere(&mut self.tri_buf, sk.chest, 0.072, label);
        sphere(&mut self.tri_buf, sk.hip, 0.068, label);
        // neck and head
        tube(&mut self.tri_buf, sk.chest, sk.head, 0.045, 0.034, label);
        sphere(&mut self.tri_buf, sk.head, 0.052, label);
        sphere_lod(&mut self.tri_buf, sk.muzzle, 0.028, label);
        for (base, tip) in sk.ears {
            tube(&mut self.tri_buf, base, tip, 0.016, 0.002, label);
        }
        for (root, tip) in sk.whiskers {
            tube_res(&mut self.tri_buf, root, tip, 0.0012, 0.0006, 4, label);
        }
        for leg in sk.legs {
            tube(&mut self.tri_buf, leg.root, leg.mid, 0.030, 0.024, label);
            sphere_lod(&mut self.tri_buf, leg.mid, 0.026, label);
            tube(&mut self.tri_buf, leg.mid, leg.paw, 0.024, 0.020, label);
            // the paw pad, seated ON the shin's end — no lift offset, so an
            // occluded far paw can't survive as a free-floating ball
            sphere_lod(&mut self.tri_buf, leg.paw, 0.021, label);
        }
        let mut prev = sk.tail_root;
        for (i, node) in tail.nodes().iter().enumerate() {
            let r1 = 0.014 - 0.0018 * i as f32;
            let r2 = 0.014 - 0.0018 * (i + 1) as f32;
            tube(&mut self.tri_buf, prev, *node, r1, r2, label);
            sphere_lod(&mut self.tri_buf, *node, r2 * 0.9, label);
            prev = *node;
        }
        render::paint::resize_triangle_surface(&mut self.mesh, &self.tri_buf);
    }
}

/// The heading's forward vector — Godot yaw convention: yaw 0 faces -Z.
fn forward(yaw: f64) -> Vector3 {
    Vector3::new((-yaw.sin()) as f32, 0.0, (-yaw.cos()) as f32)
}

/// The heading's right vector.
fn rightward(yaw: f64) -> Vector3 {
    Vector3::new(yaw.cos() as f32, 0.0, (-yaw.sin()) as f32)
}
