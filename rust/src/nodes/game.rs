//! `UnseeingGame` — the composition root, carried from `game/scripts/main.gd`
//! into the engine layer. It wires every system together and owns the
//! per-frame globals (clock, flicker) both shader passes consume; systems
//! never reach into each other, they meet here.
//!
//! READY-SIDE ONLY, for now. [`INode3D::ready`] reproduces
//! `main.gd:64-134`'s order EXACTLY — materials, the wave core, the level
//! (injected before it enters the tree), the demo tap's aim, the player,
//! the hero, the post quad, the observer, the restorer, and the settings
//! overlay LAST. There is no `process()` override here yet: the per-frame
//! loop (the clock, the flicker draw, `tick_sources`, the cats' clocks, the
//! demo tap's cadence) and the capture/restore doors are a later migration
//! step, laid onto this same struct without reshaping it — which is why
//! `wave_core`, `rng` and `demo` already exist as fields, wired in `ready()`
//! and readable through this task's own observability surface. `flicker`
//! and `demo_checked` are deliberately NOT fields yet: nothing in this
//! ready-side task ever reads or writes either one (both are purely a
//! `process()` concern), and a field neither read nor written anywhere is
//! dead code `cargo clippy -D warnings` catches — so they land with the
//! `process()` migration step that first gives them a reader.
//!
//! `main.tscn` still boots `main.gd`, unchanged: this class exists in the
//! tree of registered classes but nothing in the shipped path instantiates
//! it. That switch is a later step too.
//!
//! Loud totality on every resource load: a shader or the level scene that
//! fails to load prints `"UnseeingGame: …"` and `ready()` returns, wiring
//! nothing further — the same refuse-rather-than-limp law every other
//! composition-root child already keeps.

use godot::classes::{
    Camera3D, Engine, INode3D, Material, MeshInstance3D, Node3D, Os, PackedScene, QuadMesh,
    RandomNumberGenerator, RefCounted, Shader, ShaderMaterial,
};
use godot::prelude::*;

use super::cat::WaveCat;
use super::hero::HeroBody;
use super::level::WaveLevel;
use super::observer::WaveObserver;
use super::player::UnseeingPlayer;
use super::restorer::WaveRestorer;
use super::settings::SettingsMenu;
use crate::demo_tap::DemoTap;
use crate::ffi::WaveCore;
use crate::level_plan;

/// The perceptual ladder's world layer — real depth, everything but the
/// sources. `main.gd`'s `PRIORITY_WORLD`.
const PRIORITY_WORLD: i32 = 0;

/// The perceptual ladder's source layer — always-on-top, drawn over the
/// world. `main.gd`'s `PRIORITY_SOURCES`.
const PRIORITY_SOURCES: i32 = 20;

/// The deterministic-run seed every armed switch shares.
const SEED: u64 = 0x5EED;

/// Unseeing — composition root. See the module docs for what is and is not
/// wired yet.
#[derive(GodotClass)]
#[class(init, base=Node3D)]
pub struct UnseeingGame {
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
    /// spawn marker.
    #[var]
    level: Option<Gd<WaveLevel>>,
    /// The blind hero's body-and-input node.
    #[var]
    player: Option<Gd<UnseeingPlayer>>,
    /// The hero's visible viewmodel — cane, arm, legs, torso.
    #[var]
    hero: Option<Gd<HeroBody>>,
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
    /// `Flicker` itself (`main.gd`'s nervous-light envelope, drawn from
    /// this stream) is not a field yet — nothing reads or writes one until
    /// `process()` exists; see the module docs.
    rng: Option<Gd<RandomNumberGenerator>>,
    /// Dev-only demo tap: fires a wall strike every few seconds so an
    /// input-less run can verify the renderer — armed by a later
    /// `process()`, aimed here at the level's own tap plan.
    #[init(val = DemoTap::new(Vector3::ZERO, Vector3::UP))]
    demo: DemoTap,
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
        // floor pushed per instance by the level (u_source_floor)
        self.source_mat.set_shader(&xray_shader);
        self.source_mat.set_render_priority(PRIORITY_SOURCES);
        // the hero's cane and body render at real depth like the world
        self.cane_mat.set_shader(&data_shader);
        self.body_mat.set_shader(&data_shader);
        self.cane_mat
            .set_shader_parameter("u_base", &0.85_f64.to_variant());

        let core = WaveCore::new_gd();
        self.wave_core = Some(core.clone());

        // the world: an editor-authored scene. Injected BEFORE entering the
        // tree (children run _ready first, and a source refuses to build
        // uninjected); the root distributes the materials and pool, then
        // reads back the derived contracts below.
        let Ok(level_scene) = try_load::<PackedScene>("res://scenes/level_01.tscn") else {
            godot_error!("UnseeingGame: failed to load res://scenes/level_01.tscn");
            return;
        };
        let Some(mut level) = level_scene.try_instantiate_as::<WaveLevel>() else {
            godot_error!(
                "UnseeingGame: res://scenes/level_01.tscn did not instantiate as a WaveLevel"
            );
            return;
        };
        level.bind_mut().inject(
            self.data_mat.clone().upcast::<Material>(),
            self.source_mat.clone().upcast::<Material>(),
            core.clone().upcast::<RefCounted>(),
        );
        self.base_mut().add_child(&level);
        self.level = Some(level.clone());

        self.demo = DemoTap::new(level.bind().demo_tap(), level.bind().demo_tap_normal());

        // the hearing pass cuts player-sound shells by the walls too: hand
        // it the same wall table the data skins occlude by.
        let rects = level.bind().wall_rects();
        self.post_mat
            .set_shader_parameter("u_walls", &rects.to_variant());
        self.post_mat
            .set_shader_parameter("u_wall_count", &(rects.len() as i64).to_variant());
        self.post_mat
            .set_shader_parameter("u_wall_top", &level_plan::WALL_H.to_variant());

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
}

impl UnseeingGame {
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
    /// `UNSEEING_DEMO` / `?demo` (a later `process()`'s job) — seed and
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

    /// The "hearing" pass: a fullscreen quad glued to the camera. It
    /// edge-detects the data the world pass wrote and ray-traces the wave
    /// shells — the only two ways anything becomes visible.
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
        camera.add_child(&quad);
    }
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
