//! The write side of reproduction: validate everything, then write once.
//!
//! [`WaveRestorer::preflight`] is read-only. It parses the complete format-2
//! artifact, verifies the artifact's exact stored hash, resolves every live
//! target, and asks each state owner to construct a checked prepared value.
//! No scene, environment, pool, actor, source or warning state is changed on
//! that path. [`WaveRestorer::commit`] consumes only those prepared values;
//! after its first assignment there is no artifact-validation or repair path.
//! The observer's final recapture is an internal postcondition over code that
//! was already admitted, not a late opportunity to reject the artifact.

use godot::classes::Node;
use godot::prelude::*;

use super::cat::{PreparedCatState, WaveCat};
use super::hero::HeroBody;
use super::level::WaveLevel;
use super::observer::{NO_POOL, WaveObserver, hex64, parse_blob, pulse_core, unavailable};
use super::player::{PreparedPlayerState, UnseeingPlayer};
use super::source::SoundSource;
use crate::cat_body::{CatPose, PreparedCatPose, PreparedTail, Tail};
use crate::cat_brain::{CatBrain, PreparedCatBrain};
use crate::cat_gait::{CatGait, PreparedCatGait};
use crate::demo_tap::{DemoTap, PreparedDemoTap};
use crate::echo_queue::{EchoQueue, PreparedEchoQueue};
use crate::ffi::WaveCore;
use crate::flicker::{Flicker, FlickerState, PreparedFlicker};
use crate::pulse_pool::{PreparedPulsePool, PulsePool};
use crate::reproduce::{
    CaptureState, FORMAT_VERSION, RestoreValueError, first_divergence, state_hash,
};
use crate::sound_source::{Cadence, PreparedCadence};
use crate::temporal::{PreparedTime, prepare_time};
use crate::viewmodel::{PreparedViewmodel, Viewmodel};

const NO_LEVEL: &str = "restorer was never injected a level";
const DEAD_LEVEL: &str = "the injected level has been freed";
const NO_PLAYER: &str = "restorer was never injected the hero";
const DEAD_PLAYER: &str = "the injected hero has been freed";
const NO_BODY: &str = "restorer was never injected the hero body — the viewmodel clocks live there";
const DEAD_BODY: &str = "the injected hero body has been freed";
const NO_OBSERVER: &str = "restorer was never injected the observer — the proof is its capture";
const DEAD_OBSERVER: &str = "the injected observer has been freed";

/// The composition root's environment after each temporal owner has checked
/// its own complete domain. Fields are visible only to the sibling game node,
/// which owns the corresponding live values and performs the assignments.
#[derive(Debug, Clone)]
pub(super) struct PreparedEnv {
    pub(super) now: PreparedTime,
    pub(super) demo_checked: bool,
    pub(super) demo_armed: bool,
    pub(super) demo: PreparedDemoTap,
    pub(super) flicker: PreparedFlicker,
    pub(super) flicker_rng_state: u64,
}

struct PreparedWaveState {
    pool: PreparedPulsePool,
    echoes: PreparedEchoQueue,
}

struct PreparedHeroRestore {
    player: PreparedPlayerState,
    viewmodel: PreparedViewmodel,
}

struct PreparedCatRestore {
    cat: PreparedCatState,
}

struct PreparedSourceRestore {
    handle: DynGd<Node, dyn SoundSource>,
    name: String,
    cadence: PreparedCadence,
}

struct RestoreTargets {
    core: Gd<WaveCore>,
    player: Gd<UnseeingPlayer>,
    body: Gd<HeroBody>,
    cats: Vec<Gd<WaveCat>>,
    sources: Vec<DynGd<Node, dyn SoundSource>>,
    observer: Gd<WaveObserver>,
}

/// A whole restore transaction whose artifact and runtime dependencies were
/// proven usable without a write.
pub(super) struct PreparedRestore {
    expected: CaptureState,
    expected_hash: u64,
    env: PreparedEnv,
    waves: PreparedWaveState,
    hero: PreparedHeroRestore,
    cats: Vec<PreparedCatRestore>,
    sources: Vec<PreparedSourceRestore>,
    targets: RestoreTargets,
}

/// An assignment-complete transaction awaiting only an independent live
/// readback. It cannot reject or repair input: every artifact law was proven
/// before the first write.
pub(super) struct CommittedRestore {
    expected: CaptureState,
    expected_hash: u64,
    observer: Gd<WaveObserver>,
}

impl CommittedRestore {
    pub(super) fn verify(self, live_now: f64, live_env: &VarDictionary) -> VarDictionary {
        let fresh = match self.observer.bind().capture_state(live_now, live_env) {
            Ok(fresh) => fresh,
            Err(reason) => {
                return unavailable(&format!(
                    "internal restore defect: postcondition capture failed: {reason}"
                ));
            }
        };
        if let Some(field) = first_divergence(&self.expected, &fresh) {
            return unavailable(&format!(
                "internal restore defect: prepared commit diverged at {field}"
            ));
        }
        let fresh_hash = state_hash(&fresh);
        if fresh_hash != self.expected_hash {
            return unavailable(&format!(
                "internal restore defect: prepared commit hash {} differs from expected {}",
                hex64(fresh_hash),
                hex64(self.expected_hash)
            ));
        }
        let mut verdict = VarDictionary::new();
        verdict.set("restored", true);
        verdict.set("hash", hex64(fresh_hash).as_str());
        verdict
    }
}

impl PreparedRestore {
    pub(super) fn env(&self) -> &PreparedEnv {
        &self.env
    }
}

/// The observer's opposite: a dormant node invoked only while the game root
/// holds the tree paused.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct WaveRestorer {
    level: Option<Gd<WaveLevel>>,
    player: Option<Gd<UnseeingPlayer>>,
    body: Option<Gd<HeroBody>>,
    observer: Option<Gd<WaveObserver>>,
    base: Base<Node>,
}

#[godot_api]
impl WaveRestorer {
    #[func]
    pub(super) fn inject(
        &mut self,
        level: Option<Gd<WaveLevel>>,
        player: Option<Gd<UnseeingPlayer>>,
        body: Option<Gd<HeroBody>>,
        observer: Option<Gd<WaveObserver>>,
    ) {
        self.level = level;
        self.player = player;
        self.body = body;
        self.observer = observer;
    }
}

impl WaveRestorer {
    /// Read, resolve and validate the entire transaction. Every `bind()` here
    /// is immutable; every constructor called returns a prepared owner value.
    pub(super) fn preflight(&self, blob: &VarDictionary) -> Result<PreparedRestore, String> {
        let state = parse_blob(blob)?;
        let expected_hash = exact_stored_hash(blob)?;
        let canonical_hash = state_hash(&state);
        if expected_hash != canonical_hash {
            return Err(format!(
                "the blob's stored hash disagrees with its canonical state: stored {}, canonical {} — the artifact was edited or corrupted",
                hex64(expected_hash),
                hex64(canonical_hash)
            ));
        }
        if state.format_version != FORMAT_VERSION {
            return Err(format!(
                "the blob is capture format {}, and this build restores format {FORMAT_VERSION}",
                state.format_version
            ));
        }

        let level = self.live_level()?.clone();
        let scene = level.get_scene_file_path().to_string();
        if state.level_scene != scene {
            return Err(format!(
                "the blob was captured in {} and this game is running {scene}",
                state.level_scene
            ));
        }
        let player = self.live_player()?.clone();
        let body = self.live_body()?.clone();
        let observer = self.live_observer()?.clone();
        observer
            .bind()
            .validate_restore_graph(&level, &player, &body)?;
        let (core, cats, live_sources) = {
            let level = level.bind();
            let core = pulse_core(&level).ok_or_else(|| NO_POOL.to_string())?;
            for (index, cat) in level.cat_handles().iter().enumerate() {
                if !cat.is_instance_valid() {
                    return Err(format!(
                        "the level's cat restore target at index {index} has been freed"
                    ));
                }
            }
            for (index, source) in level.source_handles().iter().enumerate() {
                if !source.is_instance_valid() {
                    return Err(format!(
                        "the level's source restore target at index {index} has been freed"
                    ));
                }
            }
            (
                core,
                level.cat_handles().to_vec(),
                level.source_handles().to_vec(),
            )
        };

        if cats.len() != state.cats.len() {
            return Err(format!(
                "the blob carries {} cats and this level has {}",
                state.cats.len(),
                cats.len()
            ));
        }
        if live_sources.len() != state.sources.len() {
            return Err(format!(
                "the blob carries {} sound sources and this level has {}",
                state.sources.len(),
                live_sources.len()
            ));
        }

        let env = prepare_env(&state)?;
        let waves = PreparedWaveState {
            pool: PulsePool::prepare_restore(&state.slots, env.now).map_err(string_error)?,
            echoes: EchoQueue::prepare_restore(state.echoes.clone()).map_err(string_error)?,
        };
        let hero = PreparedHeroRestore {
            player: player
                .bind()
                .prepare_restore(&state.hero, env.now)
                .map_err(string_error)?,
            viewmodel: Viewmodel::prepare_restore(state.hero.viewmodel).map_err(string_error)?,
        };
        if body.bind().capture_vm().is_none() {
            return Err("hero.viewmodel: the runtime hero body is not built".to_string());
        }

        let mut prepared_cats = Vec::with_capacity(cats.len());
        for (index, (cat, capture)) in cats.iter().zip(&state.cats).enumerate() {
            let prefix = format!("cats[{index}]");
            let brain: PreparedCatBrain = CatBrain::prepare_restore(capture.brain)
                .map_err(|error| string_error(error.prefixed(&prefix)))?;
            let gait: PreparedCatGait = CatGait::prepare_restore(capture.gait)
                .map_err(|error| string_error(error.prefixed(&prefix)))?;
            let pose: PreparedCatPose = CatPose::prepare_restore(capture.pose)
                .map_err(|error| string_error(error.prefixed(&prefix)))?;
            let tail: PreparedTail = Tail::prepare_restore(capture.tail)
                .map_err(|error| string_error(error.prefixed(&prefix)))?;
            let presence = Cadence::prepare_restore(
                crate::cat_gait::PRESENCE_EVERY,
                capture.presence_next,
                true,
            )
            .map_err(|error| string_error(error.prefixed(&prefix)))?;
            let prepared = cat
                .bind()
                .prepare_restore(capture, brain, gait, pose, tail, presence, env.now)
                .map_err(|error| string_error(error.prefixed(&prefix)))?;
            prepared_cats.push(PreparedCatRestore { cat: prepared });
        }

        let mut prepared_sources = Vec::with_capacity(live_sources.len());
        for (index, (source, capture)) in live_sources.iter().zip(&state.sources).enumerate() {
            let name = source.clone().into_gd().get_name().to_string();
            if name != capture.name {
                return Err(format!(
                    "the blob's source {} stands where this level has {name}",
                    capture.name
                ));
            }
            let prefix = format!("sources[{index}]");
            let cadence = source
                .dyn_bind()
                .prepare_appointment(capture.next_emit)
                .map_err(|error| string_error(error.prefixed(&prefix)))?;
            prepared_sources.push(PreparedSourceRestore {
                handle: source.clone(),
                name,
                cadence,
            });
        }

        Ok(PreparedRestore {
            expected: state,
            expected_hash,
            env,
            waves,
            hero,
            cats: prepared_cats,
            sources: prepared_sources,
            targets: RestoreTargets {
                core,
                player,
                body,
                cats,
                sources: live_sources,
                observer,
            },
        })
    }

    /// Consume prepared values in a fixed assignment-only order. Any verdict
    /// after the writes diagnoses an internal postcondition failure; artifact
    /// validity was completely decided by `preflight`.
    pub(super) fn commit(&mut self, prepared: PreparedRestore) -> CommittedRestore {
        let PreparedRestore {
            expected,
            expected_hash,
            env: _,
            waves,
            hero,
            cats,
            sources,
            targets,
        } = prepared;
        let RestoreTargets {
            mut core,
            mut player,
            mut body,
            cats: live_cats,
            sources: resolved_sources,
            observer,
        } = targets;

        core.bind_mut()
            .install_prepared_state(waves.pool, waves.echoes);
        {
            let mut player = player.bind_mut();
            player.install_prepared(hero.player);
        }
        body.bind_mut().install_prepared_vm(hero.viewmodel);
        for (mut cat, prepared_cat) in live_cats.into_iter().zip(cats) {
            cat.bind_mut().install_prepared(prepared_cat.cat);
        }
        // The duplicated resolved list is deliberately consumed here: it is
        // the preflight census pinned into the transaction, while each source
        // prepared value carries the exact matched typed handle it installs.
        drop(resolved_sources);
        for mut source in sources {
            let _matched_name = source.name;
            source
                .handle
                .dyn_bind_mut()
                .install_prepared_appointment(source.cadence);
        }

        CommittedRestore {
            expected,
            expected_hash,
            observer,
        }
    }

    fn live_level(&self) -> Result<&Gd<WaveLevel>, &'static str> {
        live(self.level.as_ref(), NO_LEVEL, DEAD_LEVEL)
    }

    fn live_player(&self) -> Result<&Gd<UnseeingPlayer>, &'static str> {
        live(self.player.as_ref(), NO_PLAYER, DEAD_PLAYER)
    }

    fn live_body(&self) -> Result<&Gd<HeroBody>, &'static str> {
        live(self.body.as_ref(), NO_BODY, DEAD_BODY)
    }

    fn live_observer(&self) -> Result<&Gd<WaveObserver>, &'static str> {
        live(self.observer.as_ref(), NO_OBSERVER, DEAD_OBSERVER)
    }
}

fn prepare_env(state: &CaptureState) -> Result<PreparedEnv, String> {
    let env = state.env;
    Ok(PreparedEnv {
        now: prepare_time(env.now).map_err(string_error)?,
        demo_checked: env.demo_checked,
        demo_armed: env.demo_armed,
        demo: DemoTap::prepare_restore(env.demo_next).map_err(string_error)?,
        flicker: Flicker::prepare_restore(FlickerState {
            t: env.flicker_t,
            level: env.flicker_level,
            drop_until: env.flicker_drop_until,
            next_drop: env.flicker_next_drop,
        })
        .map_err(string_error)?,
        flicker_rng_state: env.flicker_rng_state as u64,
    })
}

fn exact_stored_hash(blob: &VarDictionary) -> Result<u64, String> {
    let value = blob
        .get("hash")
        .ok_or_else(|| "field hash: missing".to_string())?;
    let text = value
        .try_to::<GString>()
        .map_err(|_| "field hash: expected a string".to_string())?
        .to_string();
    if text.len() != 16
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("field hash: expected exactly 16 lowercase hex characters".to_string());
    }
    u64::from_str_radix(&text, 16)
        .map_err(|_| "field hash: expected exactly 16 lowercase hex characters".to_string())
}

fn string_error(error: RestoreValueError) -> String {
    error.to_string()
}

fn live<'a, T: GodotClass>(
    handle: Option<&'a Gd<T>>,
    missing: &'static str,
    dead: &'static str,
) -> Result<&'a Gd<T>, &'static str> {
    match handle {
        None => Err(missing),
        Some(node) if !node.is_instance_valid() => Err(dead),
        Some(node) => Ok(node),
    }
}
