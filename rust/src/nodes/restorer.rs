//! The write side of reproduction — `WaveRestorer`.
//!
//! Sibling of [`super::observer`] and its exact opposite: the observer
//! reads every system and drives none, this one drives every system and
//! reads none. It adds no law either. It parses a blob with the observer's
//! own parser, walks the subsystem restore doors in one fixed order, and
//! then asks the OBSERVER to capture the world it just wrote and compares
//! the two states with the pure functions in [`crate::reproduce`].
//!
//! ── WHY THE PROOF RE-USES THE OBSERVER ──
//!
//! There is exactly one implementation of "what this world is", and it
//! lives on the observer. A restorer that read the world back its own way
//! would be proving that its writer agrees with its reader — a closed
//! loop that passes with both of them wrong about the same field. Going
//! back through `capture_state` means the proof is taken by the same code
//! the blob itself came out of, so agreement means the world really is the
//! captured instant.
//!
//! ── THE ORDER, AND WHY IT IS THAT ORDER ──
//!
//! 1. Parse — a malformed blob is refused with the parser's own dotted
//!    path, before any handle is even fetched.
//! 2. Header — format version, then the level scene. Both are cheap, both
//!    are fatal, and both are checked before a single field is written.
//! 3. Pool and echo book, through one core handle: they are one state.
//! 4. The hero — body, eye, cane clocks, wave out-tray, viewmodel. The
//!    CLOCK lands here, with `tick`.
//! 5. The cats, positionally: the blob encodes them in scene order.
//! 6. The sources — AFTER the clock, which is the whole point. A cadence
//!    gate re-pinned before `now` moved would be measured against the old
//!    instant, and the jumped-clock law would buy each source one
//!    spurious beat on the very next frame.
//! 7. The proof.
//!
//! It has no per-frame life of its own — no `_process`, no
//! `_physics_process`, nothing to gate — so it needs no process mode: a
//! paused tree stops the engine's callbacks and never a direct call, and
//! being called on a frozen world is the only way this node is ever meant
//! to work.
//!
//! ── WHAT A REFUSAL MEANS ──
//!
//! Everything through step 2 refuses with the world untouched. From step 3
//! on, a refusal means the world has been partly written and is NOT the
//! blob: the transaction cannot roll the engine back (undoing a restore
//! would need the same doors that just failed), so a refused verdict is
//! fatal to the run, never a warning to carry on past. The composition
//! root does roll back the half it owns — see
//! [`UnseeingGame::restore_blob`](super::game::UnseeingGame).

use godot::classes::Node;
use godot::prelude::*;

use super::cat::WaveCat;
use super::hero::HeroBody;
use super::level::WaveLevel;
use super::observer::{NO_POOL, WaveObserver, hex64, parse_blob, pulse_core, unavailable};
use super::player::UnseeingPlayer;
use super::source::SoundSource;
use crate::echo_queue::EchoQueue;
use crate::pulse_pool::PulsePool;
use crate::reproduce::{CaptureState, FORMAT_VERSION, first_divergence, state_hash};

/// No level: there is no world to write into, and no scene name to check
/// the blob's own against.
const NO_LEVEL: &str = "restorer was never injected a level";

/// The level was injected and has since been freed. A scene reload leaves
/// the handle looking perfectly valid, and writing through it would take
/// the game down with the restore.
const DEAD_LEVEL: &str = "the injected level has been freed";

/// No hero: a blob carries the hero whole, so there is nothing partial to
/// usefully apply without one.
const NO_PLAYER: &str = "restorer was never injected the hero";

const DEAD_PLAYER: &str = "the injected hero has been freed";

/// No hero body: the viewmodel — the footstep clock included — lives
/// there and on no other node, exactly as the capture side has it.
const NO_BODY: &str = "restorer was never injected the hero body — the viewmodel clocks live there";

const DEAD_BODY: &str = "the injected hero body has been freed";

/// No observer: then there is no way to take the second capture, and a
/// restore that cannot prove itself must not claim to have happened.
const NO_OBSERVER: &str = "restorer was never injected the observer — the proof is its capture";

const DEAD_OBSERVER: &str = "the injected observer has been freed";

/// The write side of [`WaveObserver`]: it applies a captured blob to the
/// running game as one refuse-or-succeed transaction, and proves the fit
/// by re-capturing.
#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct WaveRestorer {
    level: Option<Gd<WaveLevel>>,
    player: Option<Gd<UnseeingPlayer>>,
    /// The hero's BODY, a separate handle because it is a separate node —
    /// the same split the observer's capture side has.
    body: Option<Gd<HeroBody>>,
    /// The reader, held by the writer: the proof is the observer's own
    /// capture of the world this node just wrote.
    observer: Option<Gd<WaveObserver>>,
    base: Base<Node>,
}

#[godot_api]
impl WaveRestorer {
    /// Hand the restorer the systems to write, and the observer to prove
    /// against. Called once by the composition root; nothing is owned,
    /// only borrowed — and every handle is re-checked for validity on each
    /// call, because a freed node leaves a handle that still looks fine.
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

    /// Apply `blob` to the running world, then prove it.
    ///
    /// `env_after` is the composition root's own env group, taken AFTER it
    /// applied the blob's env half — the clock, the demo tap's schedule
    /// and the flicker envelope all live in GDScript, where no Rust node
    /// can write them. It is passed in rather than read back because the
    /// second capture needs the same env the first one had, and only the
    /// caller can say what it now holds.
    ///
    /// `{"restored": true, "hash": "<16 hex>"}` on success, where the hash
    /// is the restored world's own — computed from the second capture, not
    /// copied from the blob, so a caller comparing it against the blob's is
    /// comparing two measurements rather than one number with itself.
    /// Otherwise the one-key refusal every boundary here speaks.
    #[func]
    pub(super) fn restore(
        &mut self,
        blob: VarDictionary,
        env_after: VarDictionary,
    ) -> VarDictionary {
        match self.transact(&blob, &env_after) {
            Ok(hash) => {
                let mut verdict = VarDictionary::new();
                verdict.set("restored", true);
                verdict.set("hash", hash.as_str());
                verdict
            }
            Err(reason) => unavailable(&reason),
        }
    }
}

impl WaveRestorer {
    /// The transaction, in the module docs' order.
    fn transact(
        &mut self,
        blob: &VarDictionary,
        env_after: &VarDictionary,
    ) -> Result<String, String> {
        let state = parse_blob(blob)?;
        self.check_header(&state)?;
        self.restore_waves(&state)?;
        self.restore_hero(&state)?;
        self.restore_cats(&state)?;
        self.restore_sources(&state)?;
        self.prove(&state, env_after)
    }

    /// The two facts that decide whether this blob belongs to this build
    /// and this map at all. Both name BOTH sides: "wrong version" without
    /// the two numbers sends the reader to find out which is which.
    fn check_header(&self, state: &CaptureState) -> Result<(), String> {
        if state.format_version != FORMAT_VERSION {
            return Err(format!(
                "the blob is capture format {}, and this build restores format {FORMAT_VERSION}",
                state.format_version
            ));
        }
        let scene = self.live_level()?.get_scene_file_path().to_string();
        if state.level_scene != scene {
            return Err(format!(
                "the blob was captured in {} and this game is running {scene}",
                state.level_scene
            ));
        }
        Ok(())
    }

    /// The pool and the echo book, through one core handle and one door.
    fn restore_waves(&mut self, state: &CaptureState) -> Result<(), String> {
        let level = self.live_level()?.clone();
        // the level's borrow ends here: the core is a different object,
        // and binding it mutably while the level is bound would be two
        // live borrows across one restore
        let core = pulse_core(&level.bind());
        let Some(mut core) = core else {
            return Err(NO_POOL.to_string());
        };
        core.bind_mut().restore_state(
            PulsePool::from_slots(&state.slots),
            EchoQueue::from_pending(state.echoes.clone()),
        );
        Ok(())
    }

    /// The hero: where the body stands and how fast, where the eye looks,
    /// the cane's two clocks and its queued intent, the waves already
    /// asked for, and the viewmodel's whole state machine.
    ///
    /// The CLOCK lands here — `tick` — and everything dated against it
    /// (the sources, below) is placed afterwards.
    fn restore_hero(&mut self, state: &CaptureState) -> Result<(), String> {
        let hero = &state.hero;
        let mut player = self.live_player()?.clone();
        let mut body = self.live_body()?.clone();
        player.set_global_position(hero.position);
        player.set_velocity(hero.velocity);
        let mut rotation = player.get_rotation();
        rotation.y = hero.yaw as f32;
        player.set_rotation(rotation);
        {
            let mut player = player.bind_mut();
            player.set_eye_pitch(hero.pitch);
            player.last_tap = hero.last_tap;
            player.tap_target = hero.tap_target;
            player.tick(state.env.now);
            // the out-tray is rebuilt, never added to: a restore onto a
            // non-empty queue would emit the captured waves AND whatever
            // the live world had not drained yet
            player.clear_wave_queue();
            for wave in &hero.queued_waves {
                player.queue_wave(
                    wave.kind,
                    wave.at,
                    wave.max_r,
                    wave.speed,
                    wave.gain,
                    wave.echoes,
                    wave.normal,
                );
            }
            player.restore_tap_queued(hero.tap_queued);
        }
        body.bind_mut().restore_vm(hero.viewmodel);
        Ok(())
    }

    /// Every cat, positionally — the order the blob encodes them in.
    ///
    /// A cat's `now` is not in the blob and is not restored here: the
    /// composition root hands every cat the clock each frame before it
    /// ticks, and nothing reads it in between, so the field is re-supplied
    /// rather than carried (the capture side made the same call).
    fn restore_cats(&mut self, state: &CaptureState) -> Result<(), String> {
        let level = self.live_level()?.clone();
        let cats: Vec<Gd<WaveCat>> = level.bind().cat_handles().to_vec();
        if cats.len() != state.cats.len() {
            return Err(format!(
                "the blob carries {} cats and this level has {}",
                state.cats.len(),
                cats.len()
            ));
        }
        for (mut cat, capture) in cats.into_iter().zip(&state.cats) {
            cat.bind_mut().restore_state(capture);
        }
        Ok(())
    }

    /// Every source's beat appointment, re-pinned AFTER the clock landed.
    ///
    /// Identity is checked, not assumed: the blob names its sources, and a
    /// scene whose sources are the same in number but not in name would
    /// otherwise have the fan's appointment written onto the radio — two
    /// gates that then both look plausible and neither of which is the
    /// captured one.
    fn restore_sources(&mut self, state: &CaptureState) -> Result<(), String> {
        let level = self.live_level()?.clone();
        let sources: Vec<DynGd<Node, dyn SoundSource>> = level.bind().source_handles().to_vec();
        if sources.len() != state.sources.len() {
            return Err(format!(
                "the blob carries {} sound sources and this level has {}",
                state.sources.len(),
                sources.len()
            ));
        }
        for (mut source, capture) in sources.into_iter().zip(&state.sources) {
            let name = source.clone().into_gd().get_name().to_string();
            if name != capture.name {
                return Err(format!(
                    "the blob's source {} stands where this level has {name}",
                    capture.name
                ));
            }
            source.dyn_bind_mut().restore_appointment(capture.next_emit);
        }
        Ok(())
    }

    /// The proof: capture the world that was just written, through the
    /// observer, and compare it against the blob field by field.
    ///
    /// The hash handed back is the SECOND capture's. A verdict that echoed
    /// the blob's own hash back would agree with itself no matter what the
    /// world holds.
    fn prove(&self, state: &CaptureState, env_after: &VarDictionary) -> Result<String, String> {
        let observer = self.live_observer()?.clone();
        let fresh = observer.bind().capture_state(state.env.now, env_after)?;
        if let Some(field) = first_divergence(state, &fresh) {
            return Err(format!(
                "restore diverged at {field} — the blob and the restored world disagree"
            ));
        }
        Ok(hex64(state_hash(&fresh)))
    }

    /// The level, if there is one and it still exists — the same rule the
    /// observer applies to every handle it was given, for the same reason:
    /// a freed node's handle still looks valid, and the tool must refuse a
    /// torn-down scene rather than write through it.
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

/// One injected handle, or the reason it cannot be used. Never cloned
/// before the validity check: cloning a `Gd<T>` whose instance is gone
/// panics, so a freed node has to be caught on the borrowed reference.
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
