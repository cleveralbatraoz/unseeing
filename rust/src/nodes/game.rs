//! `UnseeingGame` — the composition root, carried from `game/scripts/main.gd`
//! into the engine layer. It wires every system together and owns the
//! per-frame globals (clock, flicker) both shader passes consume; systems
//! never reach into each other, they meet here.
//!
//! [`INode3D::ready`] reproduces `main.gd:64-134`'s order EXACTLY —
//! materials, the wave core, the level (injected before it enters the
//! tree), the demo tap's aim, the player, the hero, the post quad, the
//! observer, the restorer, and the settings overlay LAST.
//!
//! [`INode3D::process`] reproduces `main.gd:150-168`'s order EXACTLY: the
//! clock, the player's own clock, the flicker draw pushed to all five
//! materials, the post quad's breath/grain mood, every source's clockwork
//! (`level.tick_sources`, eye = the CAMERA, never the player's body — the
//! two differ by the head-bob offset and, in a scripted test, by however
//! far a caller has moved one and not the other), every cat's clock, the
//! apply loop (`WaveCore::tick` draining reflections, then the pool's
//! shader lanes pushed to all five materials), the hero's viewmodel, and
//! last the dev-only demo tap's one-shot arming and cadence. Defining
//! `process()` on the `INode3D` impl auto-enables per-frame processing —
//! desired here, so there is no `set_process` dance to add.
//!
//! The env trio owns the nine environment fields (clock, demo schedule,
//! flicker envelope and RNG state). `restore_blob` freezes the tree, asks
//! the restorer for a complete read-only prepared transaction, then installs
//! its [`PreparedEnv`] and commits the remaining prepared owners. Invalid
//! artifact syntax, hash, environment, handles or subsystem values therefore
//! return before any write or repair warning; the legacy `apply_env` surface
//! remains only for its direct boundary tests.
//!
//! `main.tscn` boots `UnseeingGame` as its root — this class IS the
//! boot root of the shipped path. The retired GDScript composition root is
//! gone, so this node's `_ready()` and `process()` are the single owner of
//! every system's wiring.
//!
//! Loud totality on every resource load: a shader or the level scene that
//! fails to load prints `"UnseeingGame: …"` and `ready()` returns, wiring
//! nothing further — the same refuse-rather-than-limp law every other
//! composition-root child already keeps.
//!
//! WIRING COMPLETE: ready() has no "what is and is not wired yet" stage —
//! if it reaches its end rather than taking one of the explicit resource-load
//! refusals above, every system is live and the game is ready to tick.

use godot::classes::{
    Camera3D, Engine, INode3D, Material, MeshInstance3D, Node3D, Os, PackedScene, QuadMesh,
    RandomNumberGenerator, RefCounted, Shader, ShaderMaterial,
};
use godot::prelude::*;

use super::cat::WaveCat;
use super::hero::HeroBody;
use super::level::WaveLevel;
use super::observer::{WaveObserver, unavailable};
use super::player::UnseeingPlayer;
use super::restorer::{PreparedEnv, WaveRestorer};
use super::settings::SettingsMenu;
use crate::demo_tap::DemoTap;
use crate::ffi::WaveCore;
use crate::flicker::{Flicker, FlickerState};
use crate::temporal::{advance_clock, valid_time_or_zero};

/// The perceptual ladder's world layer — real depth, everything but the
/// sources. `main.gd`'s `PRIORITY_WORLD`.
const PRIORITY_WORLD: i32 = 0;

/// The perceptual ladder's source layer — always-on-top, drawn over the
/// world. `main.gd`'s `PRIORITY_SOURCES`.
const PRIORITY_SOURCES: i32 = 20;

/// The deterministic-run seed every armed switch shares.
const SEED: u64 = 0x5EED;

/// `restore_blob` was called before `ready()` wired a restorer — there is
/// no writer to hand the parsed blob to.
const NO_RESTORER: &str = "the root holds no restorer — restore_blob has nothing to write through";
const DEAD_RESTORER: &str =
    "the root restorer has been freed — restore_blob has no live transaction owner";
const NO_RNG: &str =
    "the root RNG is absent or has been freed — restore_blob has no exact stream target";

/// Unseeing — the complete shipped composition root.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct UnseeingGame {
    /// Optional authored world selected in the Inspector. Empty preserves the
    /// exact shipped level-01 fallback; a set scene is the only scene tried.
    #[export]
    #[var]
    level_scene: Option<Gd<PackedScene>>,
    /// The world skin — real depth, every solid and the hero's own body.
    #[init(val = ShaderMaterial::new_gd())]
    #[var]
    data_mat: Gd<ShaderMaterial>,
    /// The acoustic image of sound sources — always-on-top, one per-instance
    /// standing floor per source.
    #[init(val = ShaderMaterial::new_gd())]
    #[var]
    source_mat: Gd<ShaderMaterial>,
    /// The cane/arm layer — a standing reveal (`u_base` 0.85): the hero
    /// always knows their own grip.
    #[init(val = ShaderMaterial::new_gd())]
    #[var]
    cane_mat: Gd<ShaderMaterial>,
    /// The legs/torso layer — revealed only by a passing wave.
    #[init(val = ShaderMaterial::new_gd())]
    #[var]
    body_mat: Gd<ShaderMaterial>,
    /// The hearing pass — the fullscreen quad glued to the camera.
    #[init(val = ShaderMaterial::new_gd())]
    #[var]
    post_mat: Gd<ShaderMaterial>,
    /// The 64-slot pulse pool and echo book every system shares — the ONE
    /// handle that flows to the level, the player and the hero.
    #[var]
    wave_core: Option<Gd<WaveCore>>,
    /// The editor-authored world: walls, props, sound sources, the cat, the
    /// typed spawn datum.
    #[var]
    level: Option<Gd<WaveLevel>>,
    /// The blind hero's body-and-input node.
    #[var]
    player: Option<Gd<UnseeingPlayer>>,
    /// The hero's visible viewmodel — cane, arm, legs, torso.
    #[var]
    hero: Option<Gd<HeroBody>>,
    /// The eye every source's standing image and the demo tap's own frame
    /// of reference are measured from — the player's camera, cached once
    /// at `ready()` rather than re-fetched by name every frame. NOT the
    /// player's own body: the two differ by the head-bob offset, and
    /// `process()` must feed `tick_sources` this one, never the body's.
    camera: Option<Gd<Camera3D>>,
    /// The settings overlay — added LAST, so unhandled input sees Escape
    /// before the world does.
    #[var]
    settings: Option<Gd<SettingsMenu>>,
    /// The debug window: reads every system, drives none.
    #[var]
    observer: Option<Gd<WaveObserver>>,
    /// The write side of reproduction: drives every system, reads none —
    /// proves each restore by asking the observer to read the world back.
    #[var]
    restorer: Option<Gd<WaveRestorer>>,
    /// The level's companion creatures, handed over so a later `process()`
    /// can drive their clocks — the composition root's own copy of
    /// `level.cats()`.
    cat_children: Array<Gd<WaveCat>>,
    /// The game clock: simulated seconds accumulated from frame deltas —
    /// NOT wall time. Writable: `seed_test`'s contract.
    #[var]
    now: f64,
    /// The flicker's seeded stream. `None` only before `ready()` runs.
    rng: Option<Gd<RandomNumberGenerator>>,
    /// Nervous light: the reveal intensity wavers, with rare brief
    /// dropouts — `main.gd`'s `_flicker`, drawn from [`Self::rng`] every
    /// `process()` frame.
    #[init(val = Flicker::new())]
    flicker: Flicker,
    /// Dev-only demo tap: fires a wall strike every few seconds so an
    /// input-less run can verify the renderer — armed once `process()`
    /// sees `now >= 0.5`, aimed here at the level's own tap plan.
    #[init(val = DemoTap::new(Vector3::ZERO, Vector3::UP))]
    demo: DemoTap,
    /// Whether the one-shot demo-arming check has already run —
    /// `main.gd`'s `_demo_checked`.
    demo_checked: bool,
    /// Invalid temporal input can repeat every frame. Report it once per root
    /// lifetime, then repair silently so a bad caller cannot flood the log.
    temporal_fault_reported: bool,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for UnseeingGame {
    fn ready(&mut self) {
        UnseeingPlayer::ensure_actions();

        // Deterministic flicker for offline frame-comparison runs — armed
        // by ANY deterministic-run switch, not only the demo (see
        // `Self::seed_armed`).
        let mut rng = RandomNumberGenerator::new_gd();
        if self.seed_armed() {
            rng.set_seed(SEED);
        }
        self.rng = Some(rng);

        let Some(data_shader) = self.load_shader("res://shaders/data_pass.gdshader") else {
            return;
        };
        let Some(xray_shader) = self.load_shader("res://shaders/data_xray.gdshader") else {
            return;
        };
        let Some(post_shader) = self.load_shader("res://shaders/hearing_post.gdshader") else {
            return;
        };

        self.data_mat.set_shader(&data_shader);
        self.data_mat.set_render_priority(PRIORITY_WORLD);
        self.post_mat.set_shader(&post_shader);
        // the source image is LIVE: a source is always heard, its standing
        // image pushed per instance by the level (u_source_volume/u_source_muffle)
        self.source_mat.set_shader(&xray_shader);
        self.source_mat.set_render_priority(PRIORITY_SOURCES);
        // the hero's cane and body render at real depth like the world
        self.cane_mat.set_shader(&data_shader);
        self.body_mat.set_shader(&data_shader);
        self.cane_mat
            .set_shader_parameter("u_base", &0.85_f64.to_variant());
        // The crease knee, DERIVED from the one MIN_SEP the colouring
        // allocates against (render::crease) rather than retyped in GLSL.
        // The allocator and the renderer are the same law seen from two
        // ends — how far apart two labels must be, and how brightly the gap
        // between them draws — and nothing compared them: lowering MIN_SEP
        // to make a starved band fit kept every cargo test green while the
        // shader went on fading over a knee it no longer matched.
        //
        // Validated before it reaches the GPU: GLSL's smoothstep divides by
        // (hi - lo), so an equal pair is a division by zero and an inverted
        // one fades a bright seam dark. CreaseKnee refuses both, so this
        // push cannot deliver one.
        let knee = crate::render::crease::CreaseKnee::shipped();
        self.post_mat.set_shader_parameter(
            "u_crease_knee",
            &Vector2::new(knee.lo() as f32, knee.hi() as f32).to_variant(),
        );

        // Settled law 1's floor, pushed to the acoustic-image skin the way
        // the crease knee is pushed to the post pass, and for the identical
        // reason: it is DERIVED from the film grain's amplitude, and a
        // derivation whose two ends live in different languages is exactly
        // how MIN_SEP and the crease knee drifted apart while every cargo
        // test stayed green.
        self.source_mat.set_shader_parameter(
            "u_presence",
            &(crate::render::reveal::PRESENCE as f32).to_variant(),
        );

        // The second knee: SHAPE and DETAIL are two laws now, and this is
        // the one that decides how much a swept surface tells you. Same
        // validated-before-the-GPU contract as the crease knee — DetailKnee
        // refuses an equal or inverted pair, so this push cannot deliver a
        // division by zero or an inverted fade.
        let detail = crate::render::detail::DetailKnee::shipped();
        self.post_mat.set_shader_parameter(
            "u_detail_knee",
            &Vector2::new(detail.lo() as f32, detail.hi() as f32).to_variant(),
        );

        // The third knee: SHAPE. Stated in METRES of depth step rather than
        // in channel units, so that raising DIST_PACK_RANGE — which
        // level_plan::pack_range_budget actively tells a designer to do when
        // the map outgrows it — can no longer retune the outline behind their
        // back. Same validated-before-the-GPU contract as the other two.
        let sil = crate::render::silhouette::SilhouetteKnee::shipped();
        self.post_mat.set_shader_parameter(
            "u_sil_knee",
            &Vector2::new(sil.lo() as f32, sil.hi() as f32).to_variant(),
        );

        // The grain's own amplitude, and the reason it is pushed rather than
        // mirrored: `reveal::PRESENCE` is DERIVED from it, and settled law 1
        // — a sound source is always visible — is that derivation. The
        // constant used to live only as the shader's uniform default while
        // `render::grain`'s doc claimed the composition root pushed it, so
        // Rust was the mirror and the GLSL was the original, in the one
        // place the ordering matters. Now the push makes the doc true and
        // the shader's default is only what an unpushed material shows.
        self.post_mat.set_shader_parameter(
            "u_grain_amp",
            &(crate::render::grain::GRAIN_AMP as f32).to_variant(),
        );

        let core = WaveCore::new_gd();
        self.wave_core = Some(core.clone());

        // the world: an editor-authored scene. Injected BEFORE entering the
        // tree (children run _ready first, and a source refuses to build
        // uninjected); the root distributes the materials and pool, then
        // reads back the derived contracts below.
        let level_scene = if let Some(scene) = self.level_scene.clone() {
            scene
        } else {
            let Ok(scene) = try_load::<PackedScene>("res://scenes/level_01.tscn") else {
                godot_error!("UnseeingGame: failed to load res://scenes/level_01.tscn");
                return;
            };
            scene
        };
        let path = level_scene.get_path().to_string();
        let Some(instance) = level_scene.instantiate() else {
            godot_error!("UnseeingGame: {} could not be instantiated", path);
            return;
        };
        let mut level = match instance.try_cast::<WaveLevel>() {
            Ok(level) => level,
            Err(wrong_root) => {
                wrong_root.free();
                godot_error!(
                    "UnseeingGame: {} did not instantiate as a WaveLevel — check the scene's root type",
                    path
                );
                return;
            }
        };
        level.bind_mut().inject(
            self.data_mat.clone().upcast::<Material>(),
            self.source_mat.clone().upcast::<Material>(),
            core.clone().upcast::<RefCounted>(),
        );
        self.base_mut().add_child(&level);
        self.level = Some(level.clone());

        self.demo = DemoTap::new(level.bind().demo_tap(), level.bind().demo_tap_normal());

        // Every material the ROOT owns that consults the wall table gets it
        // here, the way the level hands it to the two it owns
        // (`WaveLevel::push_wall_table`). Three, not one:
        //
        //   - the hearing pass cuts every shell by the walls, the hero's and
        //     a world source's alike;
        //   - the cane and the body wear `data_pass.gdshader`, which runs
        //     `reveal_at` -> `source_reveal_vis` -> `wall_blocked_from`
        //     exactly like the world skin does.
        //
        // The last two are why this is a loop. Without a table `u_wall_count`
        // keeps its shader default of 0, the wall loop breaks on its first
        // iteration and `wall_blocked_from` answers `false` for every line —
        // so the barrier law simply did not run on the two surfaces the
        // player always has in frame, and a source in the next room lit the
        // hero's own body through the wall. Held by
        // `game/tests/wiring_test.gd::test_wall_table_reaches_every_occluding_skin`,
        // which reads the count back off all five materials.
        // REGISTERED, not pushed. The level owns its wall table and rebuilds
        // it on every derive; a push from out here would be correct only
        // because a runtime level happens to derive exactly once, before
        // this line runs. `WaveLevel::rederive` is a #[func] and anything
        // may call it, after which these three would be carrying last
        // derivation's walls while the level's own two carried this one's.
        for mat in [&self.post_mat, &self.cane_mat, &self.body_mat] {
            level
                .bind_mut()
                .add_occluding_skin(mat.clone().upcast::<Material>());
        }

        // the level's companion creatures — a later process() drives each
        // one's clock from this same handle.
        self.cat_children = level.bind().cats();

        let mut player = UnseeingPlayer::new_alloc();
        player.set("pulses", &core.clone().upcast::<RefCounted>().to_variant());
        player.set_position(level.bind().spawn_pos());
        let mut rotation = player.get_rotation();
        rotation.y = level.bind().spawn_yaw() as f32;
        player.set_rotation(rotation);
        self.base_mut().add_child(&player);
        self.player = Some(player.clone());

        let mut hero = HeroBody::new_alloc();
        hero.set("player", &player.to_variant());
        hero.set("camera", &player.get("camera"));
        hero.set("pulses", &core.clone().upcast::<RefCounted>().to_variant());
        hero.set("cane_mat", &self.cane_mat.to_variant());
        hero.set("body_mat", &self.body_mat.to_variant());
        self.base_mut().add_child(&hero);
        self.hero = Some(hero.clone());

        // the player builds its camera in its own _ready, which already ran
        // (add_child above): a null camera here means the player refused to
        // wire itself, which it already reported through its own refusal.
        let Ok(camera) = player.get("camera").try_to::<Gd<Camera3D>>() else {
            godot_error!("UnseeingGame: player built no camera — cannot attach the post quad");
            return;
        };
        self.camera = Some(camera.clone());
        self.setup_post_quad(camera.clone());

        // the debug window: the level (which already holds the wave pool)
        // and the hero's own eye, because how many walls stand between the
        // hero and a source is measured from there.
        let mut observer = WaveObserver::new_alloc();
        observer
            .bind_mut()
            .inject(Some(level.clone()), Some(camera));
        observer.bind_mut().inject_hero(Some(player.clone()));
        // the hero's BODY, injected separately from the hero: the
        // viewmodel — footstep clock and all — lives here and on no other
        // node.
        observer.bind_mut().inject_body(Some(hero.clone()));
        self.base_mut().add_child(&observer);
        self.observer = Some(observer.clone());

        // the write side: the same three systems the observer reads, plus
        // the observer itself — a restore proves itself by asking the
        // READER what the world now holds, never by reading its own writes
        // back.
        let mut restorer = WaveRestorer::new_alloc();
        restorer.bind_mut().inject(
            Some(level.clone()),
            Some(player.clone()),
            Some(hero.clone()),
            Some(observer.clone()),
        );
        self.base_mut().add_child(&restorer);
        self.restorer = Some(restorer.clone());

        // the settings overlay, added LAST on purpose: unhandled input
        // walks the tree bottom-up, so the overlay sees Escape before the
        // world does and can swallow every key it takes.
        let settings = SettingsMenu::new_alloc();
        self.base_mut().add_child(&settings);
        self.settings = Some(settings);
    }

    /// The per-frame loop — `main.gd:150-168` verbatim. Defining this
    /// override auto-enables processing (gdext's own rule, the same one
    /// [`super::level::WaveLevel`] documents turning back OFF at run
    /// time): the runtime root is meant to process every frame it lives,
    /// so nothing here opts back out of it.
    fn process(&mut self, delta: f64) {
        let (now, elapsed, repaired) = advance_clock(self.now, delta);
        self.now = now;
        if repaired {
            self.report_temporal_repair();
        }
        if let Some(mut player) = self.player.clone() {
            player.bind_mut().tick(self.now);
        }

        // the mood: drawn from the seeded stream, pushed to every material
        // that renders waves — a flicker one draw out of step desyncs a
        // shared seeded stream from the very next frame
        let Some(rng) = self.rng.as_mut() else {
            return; // ready() refused before the RNG was ever wired
        };
        let flick = self.flicker.next(elapsed, rng);
        for mut mat in self.wave_mats().iter_shared() {
            mat.set_shader_parameter("u_time", &self.now.to_variant());
            mat.set_shader_parameter("u_flick", &flick.to_variant());
        }
        self.post_mat.set_shader_parameter(
            "u_breath",
            &(1.0 + (self.now * 0.5).sin() * 0.045).to_variant(),
        );
        self.post_mat
            .set_shader_parameter("u_grain_t", &((self.now % 1.0) * 61.7).to_variant());

        // every world sound source, driven by the level itself: it
        // advances each one's clockwork on the simulated clock and dims
        // each one's standing image by the walls between the EYE — the
        // camera, never the player's body, which differs from it by the
        // head-bob offset — and THAT source's hub. A silent level is
        // legal; the loop simply finds nothing.
        if let (Some(mut level), Some(camera)) = (self.level.clone(), self.camera.clone()) {
            level
                .bind_mut()
                .tick_sources(self.now, camera.get_global_position());
        }
        for mut cat in self.cat_children.iter_shared() {
            cat.bind_mut().tick(self.now); // a catless level is legal too
        }

        // the apply loop — `Pulses.apply` verbatim: fire every echo whose
        // appointment has come, then push the pool's shader lanes to all
        // five materials. MUST run after the sources and the cats above:
        // this is the one frame their fresh emissions first reach the
        // screen, and a reorder ahead of them would push last frame's pool
        // instead.
        if let Some(mut core) = self.wave_core.clone() {
            let (count, positions, pdat, pdir) = {
                let mut bound = core.bind_mut();
                bound.tick(self.now);
                (
                    bound.live_count(self.now),
                    bound.positions(),
                    bound.pulse_data(),
                    bound.pulse_dirs(),
                )
            };
            for mut mat in self.wave_mats().iter_shared() {
                mat.set_shader_parameter("u_count", &count.to_variant());
                mat.set_shader_parameter("u_ppos", &positions.to_variant());
                mat.set_shader_parameter("u_pdat", &pdat.to_variant());
                mat.set_shader_parameter("u_pdir", &pdir.to_variant());
            }
        }

        if let Some(mut hero) = self.hero.clone() {
            hero.bind_mut().update(self.now, elapsed);
        }

        self.fire_demo_tap();
    }
}

#[godot_api]
impl UnseeingGame {
    /// Every material that renders waves, in the perceptual-ladder order —
    /// `main.gd`'s `wave_mats` array. Each entry is the SAME object the
    /// named getter above returns (`is_same` holds): this and `data_mat`
    /// etc. are two ways of reading one set of fields, never two copies of
    /// it.
    #[func]
    fn wave_mats(&self) -> Array<Gd<ShaderMaterial>> {
        [
            self.data_mat.clone(),
            self.source_mat.clone(),
            self.cane_mat.clone(),
            self.body_mat.clone(),
            self.post_mat.clone(),
        ]
        .into_iter()
        .collect()
    }

    /// The level's companion creatures, in scene order — the composition
    /// root's own copy of `level.cats()`, exposed for the same reason
    /// [`super::level::WaveLevel::cats`] is.
    #[func]
    fn cats(&self) -> Array<Gd<WaveCat>> {
        self.cat_children.clone()
    }

    /// The RNG's own seed, widened to the wire-friendly integer width —
    /// `main.gd`'s `_flicker._rng.state` neighbour, not itself: this is
    /// the SEED, the fixed starting point, while the capture's
    /// `flicker_rng_state` (Task 5) is the stream's current position.
    #[func]
    fn flicker_seed(&self) -> i64 {
        self.rng.as_ref().map_or(0, |rng| rng.get_seed() as i64)
    }

    /// Whether the dev-only demo tap is armed — `main.gd`'s
    /// `_demo.armed`, read back rather than mirrored.
    #[func]
    fn demo_armed(&self) -> bool {
        self.demo.armed
    }

    /// Whether ANY deterministic-run switch armed the seeded flicker
    /// stream at boot — `main.gd`'s `_seed_armed()`, read back off the
    /// RNG it actually seeded rather than re-derived from the environment
    /// a second time.
    #[func]
    fn seeded(&self) -> bool {
        self.rng.as_ref().is_some_and(|rng| rng.get_seed() == SEED)
    }

    /// The env group of a capture blob: everything about this instant that
    /// lives on this Rust node's own fields and in no other subsystem, so
    /// the observer cannot derive it from its injected nodes — the clock,
    /// demo tap's one-shot check and schedule, and the flicker's whole
    /// envelope, RNG stream position included. `main.gd::capture_env`
    /// verbatim: exactly nine keys, real (native) values — the blob's own
    /// TEXT spelling of the same nine fields is
    /// [`super::observer::WaveObserver::env_of`]'s job,
    /// never this one's.
    #[func]
    fn capture_env(&self) -> VarDictionary {
        let flicker = self.flicker.state();
        let mut env = VarDictionary::new();
        env.set("now", self.now);
        env.set("demo_checked", self.demo_checked);
        env.set("demo_armed", self.demo.armed);
        env.set("demo_next", self.demo.next_at());
        env.set("flicker_t", flicker.t);
        env.set("flicker_level", flicker.level);
        env.set("flicker_drop_until", flicker.drop_until);
        env.set("flicker_next_drop", flicker.next_drop);
        env.set(
            "flicker_rng_state",
            self.rng
                .as_ref()
                .map_or(0_i64, |rng| rng.get_state() as i64),
        );
        env
    }

    /// Put a captured env group back — the write side of
    /// [`Self::capture_env`], the exact nine fields it reads and the only
    /// half of a blob no other Rust node can write.
    /// This legacy callable deliberately retains its repairing boundary law
    /// for direct engine tests. Artifact restore never calls it: preflight
    /// constructs [`PreparedEnv`] and the private assignment-only door below
    /// consumes that value without warning or repair.
    #[func]
    fn apply_env(&mut self, env: VarDictionary) {
        let (now, repaired_now) = valid_time_or_zero(dict_f64(&env, "now"));
        self.now = now;
        self.demo_checked = dict_bool(&env, "demo_checked");
        self.demo.armed = dict_bool(&env, "demo_armed");
        let repaired_demo = self.demo.restore_next(dict_f64(&env, "demo_next"));
        let repaired_flicker = self.flicker.restore(FlickerState {
            t: dict_f64(&env, "flicker_t"),
            level: dict_f64(&env, "flicker_level"),
            drop_until: dict_f64(&env, "flicker_drop_until"),
            next_drop: dict_f64(&env, "flicker_next_drop"),
        });
        if repaired_now || repaired_demo || repaired_flicker {
            self.report_temporal_repair();
        }
        if let Some(rng) = self.rng.as_mut() {
            rng.set_state(dict_i64(&env, "flicker_rng_state") as u64);
        }
    }

    /// Restore a blob through a complete read-only preflight, followed by
    /// assignment-only environment and subsystem installs. The tree stays
    /// frozen across both phases and its incoming pause state is preserved.
    /// Every artifact refusal returns before the environment, world or
    /// warning latch is touched; any post-write refusal is necessarily an
    /// internal prepared-commit defect rather than late validation.
    #[func]
    fn restore_blob(&mut self, blob: VarDictionary) -> VarDictionary {
        let was_paused = self.is_paused();
        self.set_paused(true);

        let Some(mut rng) = self
            .rng
            .as_ref()
            .filter(|rng| rng.is_instance_valid())
            .cloned()
        else {
            self.set_paused(was_paused);
            return unavailable(NO_RNG);
        };
        let Some(restorer) = self.restorer.as_ref() else {
            self.set_paused(was_paused);
            return unavailable(NO_RESTORER);
        };
        if !restorer.is_instance_valid() {
            self.set_paused(was_paused);
            return unavailable(DEAD_RESTORER);
        }
        let mut restorer = restorer.clone();
        let prepared = match restorer.bind().preflight(&blob) {
            Ok(prepared) => prepared,
            Err(reason) => {
                self.set_paused(was_paused);
                return unavailable(&reason);
            }
        };
        self.apply_prepared_env(prepared.env(), &mut rng);
        let committed = restorer.bind_mut().commit(prepared);
        let live_now = self.now;
        let live_env = self.capture_env();
        let verdict = committed.verify(live_now, &live_env);
        self.set_paused(was_paused);
        verdict
    }
}

impl UnseeingGame {
    /// Install only owner-checked environment values. Unlike the public
    /// legacy `apply_env` test surface, this door cannot repair or warn.
    fn apply_prepared_env(&mut self, env: &PreparedEnv, rng: &mut Gd<RandomNumberGenerator>) {
        self.now = env.now.value();
        self.demo_checked = env.demo_checked;
        self.demo.armed = env.demo_armed;
        self.demo.install_prepared(env.demo);
        self.flicker = Flicker::from_prepared(env.flicker);
        rng.set_state(env.flicker_rng_state);
    }

    /// One warning per node lifetime is enough to expose a repaired engine or
    /// restore boundary without turning a repeated bad delta into log spam.
    fn report_temporal_repair(&mut self) {
        if !self.temporal_fault_reported {
            godot_warn!(
                "UnseeingGame: repaired invalid temporal input; further temporal repairs are silent"
            );
            self.temporal_fault_reported = true;
        }
    }

    /// `try_load` a shader, refusing loudly and returning `None` on
    /// failure — the one place `ready()`'s three shader loads share their
    /// refusal wording.
    fn load_shader(&self, path: &str) -> Option<Gd<Shader>> {
        match try_load::<Shader>(path) {
            Ok(shader) => Some(shader),
            Err(err) => {
                godot_error!("UnseeingGame: failed to load shader '{path}' — {err}");
                None
            }
        }
    }

    /// Deterministic runs arm the seed three ways: `UNSEEING_SEED` (seed
    /// alone), `UNSEEING_DEMO` (a demo run must also be reproducible), or
    /// `?seed` / `?demo` in a web URL. The demo TAP itself arms only from
    /// `UNSEEING_DEMO` / `?demo`, in [`Self::fire_demo_tap`] — seed and
    /// demo are separate switches, and this helper owns only the seed.
    fn seed_armed(&self) -> bool {
        let os = Os::singleton();
        if !os.get_environment("UNSEEING_SEED").is_empty() {
            return true;
        }
        if !os.get_environment("UNSEEING_DEMO").is_empty() {
            return true;
        }
        if os.has_feature("web") {
            return web_location_search()
                .is_some_and(|search| search.contains("seed") || search.contains("demo"));
        }
        false
    }

    /// Dev-only: fires a wall tap every few seconds so an input-less run
    /// can verify the renderer visually — movie-maker locally
    /// (`UNSEEING_DEMO=1` env), or the deployed web build (`?demo` in the
    /// URL). Queued through the player so its reflection raycasts run in
    /// physics context. `main.gd::_demo_tap` verbatim, one-shot arming
    /// check included — the web search read is [`Self::seed_armed`]'s own
    /// `web_location_search` helper, not a second `JavaScriptBridge.eval`.
    fn fire_demo_tap(&mut self) {
        if !self.demo_checked && self.now >= 0.5 {
            self.demo_checked = true;
            self.demo.armed = !Os::singleton().get_environment("UNSEEING_DEMO").is_empty();
            if Os::singleton().has_feature("web") {
                self.demo.armed = self.demo.armed
                    || web_location_search().is_some_and(|search| search.contains("demo"));
            }
        }
        if self.demo.fire_due(self.now)
            && let Some(mut player) = self.player.clone()
        {
            player
                .bind_mut()
                .queue_wave(0, self.demo.point, 6.0, 5.5, 1.0, 6, self.demo.normal);
        }
    }

    /// Whether the world is frozen right now — the same rule
    /// [`super::settings::SettingsMenu`] applies to its own pause bracket.
    fn is_paused(&self) -> bool {
        self.base()
            .get_tree_or_null()
            .is_some_and(|tree| tree.is_paused())
    }

    /// Freeze or thaw the world — the write side of [`Self::is_paused`].
    fn set_paused(&mut self, paused: bool) {
        if let Some(mut tree) = self.base().get_tree_or_null() {
            tree.set_pause(paused);
        }
    }

    /// The "hearing" pass: a fullscreen quad glued to the camera. It
    /// edge-detects the data the world pass wrote and ray-traces the wave
    /// shells — the only two ways anything becomes visible. The browser
    /// smoke's `?gprobe` switch hides this quad so its screenshot can read
    /// the raw data pass (reveal/label/distance), matching the native GPU
    /// probe's own temporary hide.
    fn setup_post_quad(&self, mut camera: Gd<Camera3D>) {
        let mut quad = MeshInstance3D::new_alloc();
        let mut mesh = QuadMesh::new_gd();
        mesh.set_size(Vector2::new(2.0, 2.0));
        quad.set_mesh(&mesh);
        quad.set_material_override(&self.post_mat);
        // The vertex shader pins the quad to the full screen; a huge cull
        // margin stops Godot from frustum-culling the tiny quad mesh it
        // thinks this is.
        quad.set_extra_cull_margin(16384.0);
        quad.set_position(Vector3::new(0.0, 0.0, -1.0));
        let web = Os::singleton().has_feature("web");
        let search = web.then(web_location_search).flatten();
        quad.set_visible(post_quad_visible(web, search.as_deref()));
        camera.add_child(&quad);
    }
}

/// Whether the hearing-pass quad should be shown for this launch. Only the
/// browser gate's explicit `gprobe` query hides it; the same spelling on a
/// native run is inert, and a missing bridge/search is the safe visible
/// default. Kept pure so deleting the switchover's old GDScript cannot
/// silently delete this web-only contract with it.
#[must_use]
fn post_quad_visible(web: bool, search: Option<&str>) -> bool {
    !web || !search.is_some_and(|query| query.contains("gprobe"))
}

/// `window.location.search`, read through the JavaScriptBridge singleton —
/// reached DYNAMICALLY (`Engine::get_singleton`, not the `JavaScriptBridge`
/// type) because that class does not exist in desktop bindings at all;
/// naming it directly would fail to compile there. `None` off the web
/// feature tag, where there is no bridge to ask.
fn web_location_search() -> Option<String> {
    let mut bridge = Engine::singleton().get_singleton("JavaScriptBridge")?;
    let result = bridge.call(
        "eval",
        &["window.location.search".to_variant(), true.to_variant()],
    );
    Some(result.to_string())
}

/// One field of a NATIVE env dictionary — [`UnseeingGame::apply_env`]'s
/// only reader. The dictionary always came straight back from
/// `WaveObserver::env_of` or `UnseeingGame::capture_env` itself, never
/// through a file, so a missing or mistyped key can only mean the caller
/// handed this function something that was never a real env group —
/// answered with the same total default every other reader in this crate
/// falls back to, rather than a panic.
fn dict_f64(env: &VarDictionary, key: &str) -> f64 {
    env.get(key)
        .and_then(|v| v.try_to::<f64>().ok())
        .unwrap_or(0.0)
}

/// The bool sibling of [`dict_f64`].
fn dict_bool(env: &VarDictionary, key: &str) -> bool {
    env.get(key)
        .and_then(|v| v.try_to::<bool>().ok())
        .unwrap_or(false)
}

/// The integer sibling of [`dict_f64`] — the flicker RNG's stream
/// position, the one field in the env group that is never a float.
fn dict_i64(env: &VarDictionary, key: &str) -> i64 {
    env.get(key)
        .and_then(|v| v.try_to::<i64>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::post_quad_visible;

    #[test]
    fn gprobe_hides_only_the_web_post_quad() {
        assert!(!post_quad_visible(true, Some("?demo&gprobe")));
        assert!(post_quad_visible(true, Some("?demo")));
        assert!(post_quad_visible(true, None));
        assert!(post_quad_visible(false, Some("?gprobe")));
    }
}
