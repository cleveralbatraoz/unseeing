# Editor Authoring SP1 — Wiki Debt

**Push at campaign merge, not before.** The wiki describes *shipped*
behaviour, and `worktree-editor-authoring-campaign` is unmerged — editing
the wiki now would describe code nobody on `main` can run yet. This file is
the ledger so none of it gets lost between now and merge: every claim below
carries the file:line that makes it true today, on this branch, at
`93f4140` (SP1's last commit, `2ff5bdf..93f4140` over the plan's twelve
tasks). It also folds in a second, older debt: nine claims in
*Research — Editor Authoring* went stale on 2026-08-11 when the 15-issue
campaign landed on `main` at `3f376cf` — before this branch's first commit
— and were never written back either. Whoever merges this campaign should
do one pass over the wiki, not two. Sections explicitly labelled as dated
addenda were recorded after `93f4140` and name their newer evidence boundary;
they do not retroactively change the historical SP1 snapshot.

Scope check against the campaign spec
(`docs/superpowers/specs/2026-08-11-editor-authoring-campaign-design.md`):
SP1 closes the "Blind placement" blocker (#30–#34) and #38-as-scoped
(binary delivery) in full, plus #44. It does **not** touch #22
(census-pinned gate), #16/#35/#36/#45 (source-seam and nesting hazards), or
#39/#41/#42 (run/ship, hand arithmetic, vocabulary) — those stay open,
sub-projects 2–4's scope, and nothing below should be read as resolving
them.

---

## 1. Mechanics — Sound Sources and Mechanics — Level and Objects

### Blueprint mode (Mechanics — Sound Sources)

`SoundFan`, `SoundRadio` and `WaveCat` are now `#[class(tool, init, …)]`
(`rust/src/nodes/fan.rs:64`, `radio.rs:79`, `cat.rs:59`) — where before only
the four solid shapes and `WaveLevel` ran their editor lifecycle, a source
or the cat placed in the Godot editor now builds the same limb geometry the
game outlines, skinless, with no injection and nothing ticking. Each class
names its own top-level built children in a `LIMBS` const so a rebuilding
`ready()` can free exactly the stale ones by name before rebuilding —
`fan.rs:55` `const LIMBS: [&str; 2] = ["FanPedestal", "FanPivot"]`;
`radio.rs:65-72`, six names (`RadioCase`, `RadioGrille`, `RadioTuner`,
`RadioDialA`/`RadioDialB`, `RadioAntenna`); `cat.rs:53`
`const LIMBS: [&str; 2] = ["CatCollider", "CatSkin"]`. The free happens
through `clear_limbs` (`rust/src/nodes/solid.rs:79-96`, already the solids'
own ghost-duplicate mechanism, now shared) and — because names are the only
handle a Ctrl+D duplicate reaches `_ready` with, ownership never being
serialized — this is also what makes a duplicated fan or radio safe to
build again rather than pile up orphaned meshes.

`ready()` on all three now branches on `Engine::singleton().is_editor_hint()`
first (`fan.rs:104-129`, `radio.rs:113-136`, `cat.rs:105-126`): the editor
branch clears, builds the visible limbs, and returns; the runtime branch —
byte-identical to before this sub-project, same uninjected-guard error
string, same build order — follows unchanged. `SourceRig::clear`
(`rust/src/nodes/source.rs:193`) forgets the rig's limb handles at the top
of every rebuild so it never holds a pointer into a node `clear_limbs` just
freed.

`WaveCat` needed one more thing the other two didn't: a `tool` node really
runs `process`/`physics_process` in the editor, and `cat.rs`'s runtime
`physics_process` calls `move_and_slide()` and writes the node's own yaw —
an owned, ticking cat would wander the viewport and a Ctrl+S would bake its
drift into the scene. The editor branch disables both
(`cat.rs:114-115`, `set_physics_process(false)` / `set_process(false)`)
before calling `build_editor_pose()` (`cat.rs:369-399`), which builds the
same two limbs as the runtime path but in **local** space around the
origin — no material override (no `data_mat` in the editor), no
`set_as_top_level(true)`, no cull margin — so the frozen silhouette rides
the node when a designer drags it, unlike the runtime mesh's
world-space/top-level placement.

Wiki edit: a new subsection in *Mechanics — Sound Sources* (after §7 "The
two shipped sources" reads best) describing blueprint mode as above; a line
in §9 Traps noting that named limbs and `clear_limbs` now also gate the
editor rebuild, not only the runtime one.

### Editor derive, per-node warnings, the signature watch (Mechanics — Level and Objects)

The single biggest fact to correct: *Mechanics — Level and Objects* §2
opens "`WaveLevel::_ready` (skipped in the editor, which wants shapes to
drag, not contracts)" — that sentence is now **false**. `ready()`
(`rust/src/nodes/level.rs:194-224`) no longer early-returns under the
editor hint at all: `build_slabs()` and `derive()` run in every mode. The
uninjected-materials `godot_error!` is now the only thing still gated on
`!editor` (`level.rs:203-207`) — an editor scene is legitimately
uninjected, and printing that on every scene open would be noise a
designer learns to ignore.

`derive()` (`level.rs:650-679`) reads
`Engine::singleton().is_editor_hint()` once into a local `editor`
(`level.rs:651`) and clears both fault stores at the top
(`level.rs:652-653`, `self.level_faults.clear(); self.node_faults.clear();`
— the "rewritten from scratch on every derivation" law already documented
on the fields, `level.rs:170-182`). Every fault site now does two jobs
unconditionally: file its text into `level_faults` or `node_faults`, and
print through `godot_error!`/`godot_warn!` only when `!editor` — text
byte-identical to what the boot gate already pinned. `derive()` ends by
telling the Scene dock to repaint: `self.base_mut().call_deferred(
"update_configuration_warnings", &[])` on itself (`level.rs:665-669`), then
the same deferred call on every censused solid (`level.rs:670-679`) so a
cleared fault stops showing and a new one starts on the node it belongs to.
Both are **deferred**, not synchronous — a solid's own warning override
walks back up to the same `WaveLevel` and re-binds it
(`solid::warnings_from_level` → `WaveLevel::faults_for`), and a synchronous
call during `derive()` (which already holds `self` exclusively) is a
reentrant bind on an object already bound, which panics. Deferring moves
both reads to idle time, after `derive()`'s own bind releases.

`get_configuration_warnings()` exists twice, deliberately: the `INode3D`
trait override (`level.rs:258-260`) is what Godot's editor actually calls
— a pure GDVIRTUAL, never bound to `ClassDB`, so **no script, static or
dynamic typing, `has_method`, or `.call()`, on any class engine or
extension, can reach it any other way** (measured, not assumed) — and an
inherent `#[func]` of the identical name (`level.rs:349-352`) forwards to
it via UFCS purely so a test or probe can read the same data, the same
shadowing trick `WaveWall::oid()` already used against `WaveSolid::oid()`.
`#[func] fn rederive(&mut self)` (`level.rs:335-338`) is the manual,
in-tree-only refresh; the starved-oid path also gained
`level_plan::starved_census_indices` (`rust/src/level_plan.rs:520`), a pure
function translating `oid_palette::assign`'s starved-slot indices (indices
into the *coloured subset*) back into `census.solids` indices, called from
`level.rs:831` inside `assign_oids` — one `PlacementFault` per starved
slot (`level.rs:831-838`), on top of the level-wide starvation count that
already printed (`level.rs:819-829`).

Per-node warnings on the four solid shapes: `solid::warnings_from_level`
(`rust/src/nodes/solid.rs:125-134`) walks a node's ancestors for the
owning `WaveLevel` and reads `WaveLevel::faults_for(node)`
(`level.rs:424-432`) — matched by `root.get_path_to(node)`, **not by
name**, so two identically-named nodes under different parents are told
apart. (Contrast the spawn marker's own name-only matching from before
SP1, §3 below — this is the more careful version of the same problem,
arrived at one sub-project later.) A solid outside any level — a prefab edited
standalone — walks to the scene root and wears no warning at all, which is
legal, not a fault. All four solid classes carry the identical pair: the
real `get_configuration_warnings` override in their base-interface impl
(`wall.rs:78-80`; `props.rs:102` `WaveProp`, `:248` `WaveColumn`, `:426`
`WaveWedge`), and the inherent `#[func]` forwarder for testability
(`wall.rs:125-128`; `props.rs:143-144`, `:293-294`, `:458-459`).

The condition-watch that makes all of this live while dragging, not just on
scene open: `WaveLevel::process` (`INode3D` impl, `level.rs:241-250`)
no-ops outside the editor and otherwise folds `scene_signature()`
(`level.rs:986-1022`) every frame, re-deriving only when it differs from
`last_signature` (field, `level.rs:188`, seeded from the first derive in
`ready()` at `level.rs:213` so a freshly opened scene doesn't re-derive on
its own first frame). The pure fold lives in
`level_plan::scene_signature` — an FNV-1a hash over the level's own
`extents` knob (folded first, its own boundary byte, `level_plan.rs`,
added by a review-driven fix after the first cut of this task shipped
without it — `derive()` genuinely reads `extents` through
`report_placement`'s floor box and `assign_oids`'s slab anchors, so a
signature blind to it would leave a placement warning stale after an
extents resize) plus, per censused node, its path, its 12 global-transform
floats, and — for a solid — its skin mesh's local AABB, which is what
captures a knob drag without a bespoke setter hook per class. Two separate
census walks exist on purpose (`process`/`scene_signature` vs. `derive()`
each call `census()` themselves) rather than one shared object threaded
through both: `derive()` mutates far more state than a signature fold
needs, and the walk is microseconds at ~130 nodes.

Wiki edit: replace *Mechanics — Level and Objects* §2's opening sentence
(now false); replace the whole closing subsection "### What the recipe does
not yet survive (measured 2026-08-10)" — every bullet in it is a hole SP1
closed — with a short paragraph pointing at the new behaviour above and
the still-open §3 items below; add a line to the authoring recipe
(currently §6) noting sources/cat now render in the editor, a yellow
triangle names a fault, and every shape knob carries a range; add one
sentence to §3 THE OBJECT-ID BUDGET noting a starved slot is now reported
per-node, not only as a level-wide count.

### Knobs get ranges and metres

All 15 designer `#[export]` knobs across the eight node classes now carry
`#[export(range = (…))]`, with a `" m"` suffix wherever the value is a
length: `WaveWall.length` (`wall.rs:42`, `(0.3, 30.0, 0.1, or_greater,
suffix = " m")`); `WaveProp.size` / `WaveWedge.size` — both `Vector3`
(`props.rs:66`, `:359`, `(0.05, 20.0, 0.05, or_greater, suffix = " m")`);
`WaveColumn.radius`/`height` (`props.rs:172`, `:177`); `WaveLevel.extents`
— `Vector2` (`level.rs:135`, `(4.0, 60.0, 1.0, or_greater, suffix = " m")`);
`WaveCat.seed` (`cat.rs:71`, `(0.0, 999999.0)`, no suffix — not a length)
and `roam_size` (`cat.rs:76`, `(1.0, 30.0, 0.5, suffix = " m")` — the one
length knob in this set with **no** `or_greater`, a literal choice from the
brief rather than an oversight, so the Inspector hard-clamps a cat's roam
at 30 m with no escape hatch). Measured and worth recording as a fact
about the toolchain: `#[export(range = …)]` and `#[var(get =, set =)]`
stack cleanly with no fallback in gdext 0.5.4, on both scalar and
`Vector2`/`Vector3` fields — confirmed by macro-expansion tracing, not
just observed behaviour, so a range hint provably cannot bypass a solid's
`SignFold` setter machinery. Pinned by `game/tests/knob_hint_test.gd`
(7 cases, 3 scalar + 4 vector).

### Every class gets a face

`game/unseeing.gdextension` gained an `[icons]` block
(`game/unseeing.gdextension:23-32`) naming eight 16×16 SVGs under
`game/icons/` — thin `#e0e0e0` outlines, `stroke-width="1.2"`, no fill —
one each for `WaveLevel`, `WaveWall`, `WaveProp`, `WaveColumn`,
`WaveWedge`, `SoundFan`, `SoundRadio`, `WaveCat`. Pinned by
`game/tests/icon_manifest_test.gd` (3 cases: the section exists, names
exactly eight classes, every referenced file exists on disk). Rendering
inside the Create Node dialog itself is **unverified** — no headless probe
reaches it; carry that into §3's unverified-items update below rather than
claim it visually confirmed.

### In-editor docs, honestly gated

`rust/Cargo.toml:21` adds a non-default feature,
`editor-docs = ["godot/register-docs"]`. A sweep of all 15 `#[export]`
knobs across the eight classes found every one already carried a
designer-facing `///` doc line before this task — nothing was missing, so
no doc text changed; `editor-docs` only makes those comments reach the
Inspector tooltip for whoever builds with the feature enabled. A default
build carries zero bytes of it. Kept alive against rot by
`cargo check --features editor-docs` in `ci/pipeline.sh`'s rust stage
(`ci/pipeline.sh:69`, right after `cargo test` and before the release
build) — see §2 below. CLAUDE.md's own matching claim is corrected **in
place**, not deferred to the wiki: `CLAUDE.md:404-406` now reads
"in-editor docs behind the non-default `editor-docs` cargo feature (the
designer bootstrap build enables it; shipped exports never do)". That
closes #44 outright; nothing further is owed the wiki for it beyond noting
it resolved in §3.

---

## 2. Engineering — Build, Test, Deploy

`ci/pipeline.sh` gained four new gates. Inside the existing rust stage,
right after `cargo test` and before the release build, `cargo check
--features editor-docs` (`ci/pipeline.sh:66-69`) — a feature nobody
compiles by default rots unnoticed, so this line is what stops it from
doing that. After the pre-existing editor-mode slab probe
(`ci/pipeline.sh:149-150`) and before the `SKIP_EXPORT` cutoff
(`ci/pipeline.sh:168`), three more probes run in sequence: the
**editor-source probe** (`tools/probe_editor_sources.sh`, wired at
`ci/pipeline.sh:152-153`) proving `SoundFan`/`SoundRadio`/`WaveCat` build
their blueprint limbs under `-e` and build nothing at all uninjected at
run time; the **editor-level probe** (`tools/probe_editor_level.sh`,
`ci/pipeline.sh:155-156`) proving `WaveLevel` derives and reports
configuration warnings at edit time, that fixing an arrangement clears the
warning, and that the scene-signature watch re-derives with no manual
`rederive()` call after a wall move, a re-sunk crate, or a shrunk `extents`
knob; and the **engine census probe**
(`game/tests/probe/engine_census_probe.gd`, invoked directly at
`ci/pipeline.sh:164-166` with no wrapper script, since — unlike its three
siblings — it has no editor/run duality to prove) reproducing the same
15-class hand-written roster `game/tests/engine_binary_test.gd:25-41`
already carries, deliberately duplicated rather than derived from source
(a roster regenerated from `rust/src` would drift together with the exact
bug class it exists to catch).

`tools/bootstrap.sh` (new, 109 lines) is the one command
`game/README.md`'s authoring step 1 now names (`game/README.md:50-71`):
checks for `rustup`/`cargo`, installing rustup non-interactively if
missing (`bootstrap.sh:26-44`) with every failure path — including the
rustup-install-failed branch, `bootstrap.sh:37-42` — naming a concrete
remedy, not just restating the symptom; checks for a C linker
(`bootstrap.sh:46-56`); builds with `cargo build --release --features
editor-docs` (`bootstrap.sh:58-69`); discovers and version-checks Godot
against `.godot-version` (`bootstrap.sh:71-95`, the same prefix-match
pattern `ci/pipeline.sh` already uses); imports the project
(`bootstrap.sh:100-101`), deliberately **after** the build — a pre-build
import records a failed extension load in `.godot/extension_list.cfg`
that a running editor never retries, only a fresh import after the dylib
exists will; and verifies with the engine census probe
(`bootstrap.sh:103-107`) before printing `bootstrap: OK`. macOS/Linux
only — Windows prints the per-triple `cargo build --release --features
editor-docs --target x86_64-pc-windows-msvc` command and exits 2
(`bootstrap.sh:17-24`), because the gdextension's Windows keys are
per-triple and a single host-arch build can never satisfy them, unlike the
macOS/Linux keys, which both point at the one host-native
`rust/target/release/` artifact.

Test counts as of this branch's HEAD (`93f4140`): **286 cargo tests**
(`rust/`, `cargo test`), **231 gdUnit4 cases across 28 suites**
(`game/tests/`, including the two new suites this sub-project added,
`knob_hint_test.gd` and `icon_manifest_test.gd`). Both figures are already
higher than *Engineering — Build, Test, Deploy*'s currently-published
213 cargo / 158 gdUnit-cases / 23 suites, which predates even the
15-issue campaign merged at `3f376cf` before this branch started (that
campaign's own baseline was 275/221/26 — see the wiki-debt note already
recorded against it, still unpushed). Whoever writes this back should
re-read the count at merge time rather than trust either number here: the
remaining sub-projects (2–4) will move it again before the campaign as a
whole lands.

Wiki edit: add the four new gates to *Engineering — Build, Test, Deploy*
§2's numbered stage list (a new sub-bullet under stage 3 for the
`editor-docs` check, three new items after stage 5 for the probes); update
§3's cargo/gdUnit case counts (and note the 15-issue campaign's own
still-outstanding 275/221/26 update while doing it, so this doesn't become
a third unpushed revision of the same two numbers).

### 2026-08-13 addendum — native bootstrap on every desktop

This addendum records the cross-platform bootstrap follow-up on the completed
campaign branch; it is intentionally newer than the `93f4140` SP1 snapshot
above. The native bootstrap pair is the one-command contract:
`tools/bootstrap.sh` on macOS/Linux and `tools\bootstrap.cmd` (delegating to
`tools/bootstrap.ps1`) on Windows. The POSIX path `game/README.md`'s authoring
step 1 now names (`game/README.md:55-78`) checks for rustup, installing it
non-interactively when absent (`bootstrap.sh:23-50`) with every failure path
naming a concrete remedy, not just restating the symptom; installs and selects
the exact `rust-toolchain.toml` channel (`bootstrap.sh:51-80`); checks for a C
linker (`bootstrap.sh:82-92`); deletes the expected artifact before building
and requires that exact path to be recreated by `cargo build --release
--features editor-docs` (`bootstrap.sh:94-119`), so a stale library or
redirected Cargo target cannot masquerade as success; discovers and
version-checks Godot against `.godot-version` (`bootstrap.sh:121-145`, the same
prefix-match pattern `ci/pipeline.sh` already uses); imports the project
(`bootstrap.sh:150-151`), deliberately **after** the build — a pre-build import
records a failed extension load in `.godot/extension_list.cfg` that a running
editor never retries, only a fresh import after the library exists will; and
verifies with the engine census probe (`bootstrap.sh:153-170`) before printing
`bootstrap: OK`.

The Windows path holds the same ordering and verdict, reads the Godot
executable's PE architecture, and selects `x86_64-pc-windows-msvc` or
`aarch64-pc-windows-msvc` so the DLL lands at the target-specific path the
GDExtension declares. It installs official rustup when needed, refreshes the
current process's search path, and gives an actionable MSVC Build Tools remedy.
Both paths have behavioral fake-boundary suites; Windows CI also runs the real
x86_64 build/import/19-class census. Linux ARM64 now has an explicit manifest
route and pinned Rust target alongside x86_64.

---

## 3. Research — Editor Authoring

### Claims resolved by SP1 (this branch, `2ff5bdf..93f4140`)

- **#30 sources visible** — §3(ii)'s "`SoundFan` (`fan.rs:58`), `SoundRadio`
  (`radio.rs:63`) and `WaveCat` (`cat.rs:52`) are `#[class(init, …)]` with
  **no `tool`**... Measured: `Fan children=0`, `Radio children=0`,
  `Cat children=0`" is now **false**: all three are `#[class(tool, …)]`
  (`fan.rs:64`, `radio.rs:79`, `cat.rs:59`) and build their limbs in the
  editor (`fan.rs:104-129`, `radio.rs:113-136`, `cat.rs:105-126`).
- **#31 editor warnings** — §3(ii)'s
  "`grep -rn "configuration_warning|Gizmo|EditorPlugin…" → 0 hits" is now
  **false**: `WaveLevel` (`level.rs:258-260`, `:349-352`), `WaveWall`
  (`wall.rs:78-80`, `:125-128`) and `WaveProp`/`WaveColumn`/`WaveWedge`
  (`props.rs`, three matching pairs) all override
  `get_configuration_warnings`. §5's "All three report only at runtime, in
  the Output panel, because `WaveLevel::ready` returns before `derive()`
  under `is_editor_hint()`" is also now **false** — that early return is
  deleted (`level.rs:194-224`); every fault fires into `level_faults`/
  `node_faults` in both modes (`level.rs:650-654`), printed to the log
  only at run time.
- **#32 icons** — §3(ii)'s "No class has an icon" is now **false**: eight
  are wired (`game/unseeing.gdextension:23-32`).
- **#33 docs** — §3(ii)'s "`register-docs` is absent from
  `rust/Cargo.toml:13`" is now **false**: present as the non-default
  `editor-docs` feature (`rust/Cargo.toml:21`); CLAUDE.md's own matching
  claim, which this same wiki section calls premature, is corrected
  (`CLAUDE.md:404-406`).
- **#34 ranges** — §3(ii)'s "Of 15 `#[export]`s, 7 carry ranges and all 7
  are on the sound sources; every shape knob a designer actually places is
  `hint=0`" is now **false**: all 15 carry a range hint (§1 above).
- **#38-as-scoped bootstrap** — §3(i)'s "Unblocking it needs rustup, the
  pinned `1.97.1` toolchain, five targets and `cargo build --release` —
  the three things the premise excludes" is now answered by one native command
  on every desktop: `tools/bootstrap.sh` on macOS/Linux and
  `tools\bootstrap.cmd` on Windows. Per-triple Windows GDExtension keys are
  selected automatically from the editor architecture rather than handed to a
  designer as a manual build recipe.
- **#44 CLAUDE.md** — corrected in place, not deferred
  (`CLAUDE.md:404-406`); no further wiki action beyond noting it resolved,
  since CLAUDE.md is not a wiki page.

### Claims already stale before SP1 started

These went false on 2026-08-11 when the 15-issue campaign landed on `main`
at `3f376cf` — SP1's own first commit's parent — so *Research — Editor
Authoring* (researched 2026-08-10 at `b01632e`) has been carrying them as
live obstacles for a full day longer than it needed to, independent of
anything SP1 did:

- **"macOS ships arm64-only... no `lipo` step exists anywhere in `ci/`,
  `tools/` or `deploy.sh`"** (§3.i) — **false**: `tools/build_macos_core.sh`
  builds both `aarch64-apple-darwin` and `x86_64-apple-darwin` slices and
  fuses them (`build_macos_core.sh:103`, `lipo -create`), then reads the
  resulting Mach-O's own architectures back to prove the fusion actually
  happened (`build_macos_core.sh:113`, `tools/check_universal.sh`).
- **"A stale dylib is worse because it half-works... Nothing detects it"**
  (§3.i) — **false**: `build_macos_core.sh`'s `build_slice()` deletes each
  target-triple artifact before building it (`build_macos_core.sh:60-77`)
  so the file's mere existence afterward proves this run produced it, not
  merely that something sits at the conventional path — the exact
  distinction the old claim said nothing caught.
- **"a 28 × 28 m opaque ceiling is drawn over the whole map at edit
  time... Being unowned they have no Scene-dock row and no eye icon"**
  (§3.ii), restated in *Mechanics — Level and Objects*' "recipe does not
  yet survive" note — **false**, twice over: first fixed by hiding the lid
  under `is_editor_hint()` rather than skipping it
  (`level_plan::slab_drawn`, `rust/src/level_plan.rs:94`, read at
  `level.rs:940`), then SP1 deleted the early return that made any of this
  conditional on editor-vs-run at all — `build_slabs()` now runs
  identically in both modes and the level derives configuration warnings
  live regardless.
- **"the cheap gate is deaf to every loud thing `WaveLevel` says...
  Measured with exit 0 and 0 hits"** (§3.iii) — **false**:
  `ci/boot_error_pattern.sh:38` now matches `ERROR: WaveLevel`,
  `ERROR: SoundFan`, `ERROR: SoundRadio`, `ERROR: WaveCat`,
  `ERROR: UnseeingPlayer`, `ERROR: WaveWall`, `ERROR: hero_body` in
  addition to the original generic patterns.
- **"a third source placed first in scene order... 10 assertions, incl.
  3 nil crashes... `sources()[0] as SoundFan`"** (§3.iii table) —
  **false**: sources are no longer read off a fixed scene-order slot; the
  suite finds them by name.
- **"The spawn is matched by the magic string `"SpawnPoint"`
  (`level.rs:499-501`). Rename it → the hero wakes at `(0, 0.9, 0)`,
  sealed outside the border walls. Duplicate it → the copy is silently
  ignored"** and **"The demo tap is taken from `source_children.first()`
  (`level.rs:328-335`), so reordering the Scene dock re-aims it"** (§3.iv)
  — **false** on every count: `level_plan::choose_spawn`
  (`level_plan.rs:226-263`) takes every `Marker3D` whose name reads as
  `SpawnPoint` or `SpawnPoint<digits>` (`spawn_name`,
  `level_plan.rs:199-207`) and complains, loudly and by PATH (not name —
  two markers can share a name under different parents), about every
  marker that is not the winner, about auto-numbered Ctrl+D copies, and
  about a level with no exact match at all (three distinct sentences,
  `level_plan.rs:234-263`); the demo tap now aims at the sound source
  **nearest** the spawn (`level_plan::nearest_source`, read at
  `level.rs:367-369`'s own doc comment), not the first one in scene order.
- **§5's ceilings, "report only at runtime, in the Output panel"** — the
  wall-slot and pack-range budgets are now asserted in gdUnit, not only
  eyeballed: `game/tests/level_test.gd:593` ("warns with its headroom"),
  `:614` ("errors and counts the dropped walls"), and the pack-range case
  following `:632`, backed by cargo tests in `rust/src/level_plan.rs`
  (`wall_budget`, the fixtures around `:1787-1843`).

### Unverified items (§8), status update

- **Item 1** (icon rendering in the Create Node dialog) — still
  unverified; the icons exist and are wired (§1 above) but no headless
  probe reaches the dialog itself, per the task that landed them.
- **Item 2** (`register-docs` reaching the Inspector tooltip) — still
  unverified for the identical structural reason; the feature and the
  doc sweep are both landed (§1), only the visual end-to-end is open.
- **Item 5** (`test/repo_hygiene.sh` with eight new SVG + `.import`
  pairs) — **resolved**: run as part of the full pipeline when the icons
  landed, no blob-size violation (8 SVGs, ≈1.5 KB each).
- **Item 7** (cold-machine install cost) — partially addressed, not
  closed: `tools/bootstrap.sh` exists and every one of its failure paths
  was exercised, including the cargo-less branch (forced for real, via a
  scratch `HOME`, a `PATH` stripped to `/usr/bin:/bin`, and a black-holed
  proxy so the real rustup installer never launches but `curl` still
  fails at the network layer as the script would see it) — but a machine
  with genuinely nothing installed, run end to end from a truly clean
  state, has still never happened. The original number (1m19s–1m43s,
  warm) remains unmeasured cold.

Everything else in §7 (Refuted) and the remaining §8 items (3, 4, 6, 8, 9,
10) is untouched by this sub-project and needs no edit here.

---

## 4. SP4 — The Rust composition root

Scope check against the campaign spec: SP4 absorbs `game/scripts/main.gd`
into the registered `UnseeingGame` node (`rust/src/nodes/game.rs`) — level
instancing, injection order, player/hero/observer/restorer wiring, the
settings-menu construction (added LAST — unchanged law), the per-frame
globals (clock, flicker), the demo tap schedule, and the
capture_env/apply_env/restore_blob trio. Landed `e0c0250..c0ecba9`
(nine tasks, `6cc6c54`..`c0ecba9`). Closes the "`main.gd` is gone;
GDScript in the repo is designer-facing only; the razor is stated in
CLAUDE.md" success criterion. Does **not** touch #22, #16/#35/#36/#45, or
#39/#41/#42 — unchanged from SP1's own scope note above.

Measured at `c0ecba9`: `game/scripts/` does not exist — git tracks no
such path at all (`git ls-tree -r HEAD --name-only game/ | grep -c
'^game/scripts/'` → 0), the same fact CLAUDE.md's own phrasing already
carries precisely ("`game/scripts/` carries nothing," not "is empty").
The only GDScript left in the repository lives under `game/tests/`
(suites, probes, and the relocated `pulses.gd` test shim) — 7,690 lines
total, all of it test- and probe-facing (`find game -name '*.gd' -not
-path 'game/addons/*' | xargs wc -l`), export-excluded from every
shipped build
(`game/export_presets.cfg`, `exclude_filter="tests/*,addons/*,reports/*"`,
repeated per platform preset, e.g. line 11). `game/scenes/main.tscn` is
now `[node name="Main" type="UnseeingGame"]` — no script attached, because
there is nothing left for one to do.

### Mechanics Overview

§3's file map (`Mechanics-Overview.md:100-114`) still lists
`scenes/main.tscn — one node: UnseeingMain` and four `scripts/*.gd`
bullets (`main.gd` the composition root, `pulses.gd`, `flicker.gd`,
`demo_tap.gd`) — none of which exist any more. Replace with: `main.tscn`
— one node, `Main`, of type `UnseeingGame` (`rust/src/nodes/game.rs`); no
`scripts/` entries at all — flicker (`rust/src/flicker.rs`), the demo tap
(`rust/src/demo_tap.rs`) and the pool shim (`WaveCore`,
`rust/src/ffi.rs`) are Rust now, reached straight off the root's own
fields rather than through a GDScript intermediary.

§4 "The frame, end to end" (`Mechanics-Overview.md:117-131`) opens
"`game/scripts/main.gd::_process` is the whole game loop" and walks seven
numbered steps. The steps are still correct in order and substance —
only the anchor needs replacing: it is now `UnseeingGame::process`
(`INode3D` impl, `rust/src/nodes/game.rs:312-381`), with `ready()`
(`game.rs:166-305`) as the boot-time sibling the section doesn't name at
all today. New line citations for the same seven steps: 1 (`now += dt`)
→ `game.rs:313`; 2 (`player.tick`) → `game.rs:314-316`; 3 (push
`u_time`/`u_flick`) → `game.rs:324-328`; 4 (`level.tick_sources`) →
`game.rs:342-346`; 5 (cat ticks) → `game.rs:347-349`; 6 (the apply loop —
tick, live_count, positions/pulse_data/pulse_dirs pushed to all five
materials) → `game.rs:357-374`, now inlined rather than calling
`Pulses.apply`; 7 (`hero.update`) → `game.rs:376-378`. Worth one added
sentence: `fire_demo_tap()` (`game.rs:380`, body at `game.rs:604-620`)
runs last, after the frame's own state has settled — present in
`main.gd` too (`_demo_tap`), just never itemised among the seven.

### Engineering — Build, Test, Deploy

No literal `main.gd` citation on this page, but two things it implies are
now imprecise. Stage 5 in the numbered list
(`Engineering-Build-Test-Deploy.md:57-58`, "gdformat --check and gdlint
over `game/scripts` and `game/tests`") still runs correctly —
`gdscript_files` walks whatever exists — but `game/scripts` does not
exist at all any more (git tracks no such path), so the sentence should
say so rather than imply a populated, or even an empty, directory;
the regression this exact drift would cause is guarded by the lint-scope
sentinel, `test/ci_gdscript_lint_scope.sh:74-91`, which proves
`game/tests/` coverage through two named files (`pulses.gd`,
`wiring_test.gd`) precisely because `main.gd` stopped being available as
a sentinel. Stages 8 and 9 ("headless boots of the real main scene") stay
literally true — `main.tscn` is still THE main scene — but a reader could
infer a GDScript boot from the surrounding prose; worth one clause noting
the boot is now the registered `UnseeingGame` node, not a `main.gd`
script, so the determinism and restore probes exercise Rust
`ready()`/`process()` end to end, not a script calling into Rust.

### Mechanics — Sound Sources / Mechanics — Level and Objects: checked, nothing to do

Verified directly against a clean wiki checkout (`e2a36e8`): `grep -n
"main\.gd\|Pulses\|pulses\.gd" Mechanics-Sound-Sources.md
Mechanics-Level-and-Objects.md` → **zero hits in both files.** Neither
page has ever named `main.gd` or `Pulses`. Recording this so the next
reader doesn't go looking for a citation that was never there — the plan
that named these two pages for this pass was working from an assumption,
not a re-check.

### Where `main.gd`/`Pulses` actually still linger

Two pages the SP4 plan didn't name, found by the same grep, do carry
stale citations and belong in the same wiki pass:

- **Mechanics — Rendering**, three hits: `Mechanics-Rendering.md:10`,
  sources-of-truth line, "`game/scripts/main.gd` (wiring)"; `:176`,
  "`main.gd` owns five materials and pushes the same globals to all of
  them"; `:182`, inside the uniform table that follows, "`u_count, u_ppos,
  u_pdat, u_pdir ← every frame, from Pulses.apply`" — easy to miss because
  it sits inside a fenced code block rather than prose, which is exactly
  how a first pass over this page missed it too. All three repoint to
  `rust/src/nodes/game.rs` — `wave_mats()` (`game.rs:391-402`) is the same
  five-material array under a new name, `process()` (`game.rs:312-381`) is
  what pushes the globals every frame, and the `:182` row specifically is
  the inlined apply loop at `game.rs:357-374` (no more `Pulses.apply`
  call to name).
- **Mechanics — Waves** (`Mechanics-Waves.md:51`, "`now` advances once per
  frame in `game/scripts/main.gd`"; `:93`, "`Pulses.emit_reflecting`").
  The clock line becomes `rust/src/nodes/game.rs:313`. The reflection line
  is unchanged in substance — the reflection law itself never moved — but
  the call now reaches `WaveCore::emit_reflecting` (`rust/src/ffi.rs`)
  directly from whichever Rust node uses it (player, sources), not
  through the `Pulses` GDScript name; `Pulses.emit_reflecting` survives
  only inside `game/tests/pulses.gd`, the test-facing shim, which is what
  the suites that still call it through that name are exercising.
- **Engineering — Debugging and Observability** carries the heaviest
  concentration: 15 raw line hits across 14 citations (`:22`, `:119`,
  `:138-139` — two consecutive lines, one citation — `:207`, `:403`,
  `:443`, `:714`, `:759`, `:782`, `:955`, `:961`, `:1131`, `:1134`,
  `:1261`) — out of SP4's named scope but flagged here rather than left
  for a second unpushed pass to rediscover. The one that needs more than a
  path swap: its §11 header, "The GDScript half: `main.gd::restore_blob`"
  (line `:1131`), needs the *framing* changed, not just the citation —
  there is no GDScript half left. `restore_blob`
  (`rust/src/nodes/game.rs:505-560`), `capture_env` (`game.rs:446-464`)
  and `apply_env` (`game.rs:475-489`) are the same three functions under
  the same names, ported verbatim (module doc, `game.rs:23-32` says so
  explicitly), now all Rust. The seed/demo paragraph at `:780-783`
  ("`UNSEEING_SEED` (or `?seed` on web) seeds the flicker WITHOUT arming
  the demo tap that would contaminate the pool; `UNSEEING_DEMO`/`?demo`
  still seed too") is semantically **unchanged** — three arming paths
  exactly as before, same names, same behaviour — and needs only its
  citation moved: the switch itself is `UnseeingGame::seed_armed`
  (`game.rs:582-595`), the tap's one-shot arming check lives inside
  `fire_demo_tap` (`game.rs:604-620`), and the web-only `?seed`/`?demo`
  query read is `web_location_search` (`game.rs:660-667`), called from
  both.

Wiki edit: Mechanics Overview §3/§4 rewritten as above; Engineering —
Build, Test, Deploy stage 5 and stages 8-9 each get one clarifying
clause; Mechanics — Rendering (three citations) and Mechanics — Waves
(two citations) get every `main.gd`/`Pulses` mention repointed at
`rust/src/nodes/game.rs`; Engineering — Debugging and Observability gets
the largest pass of this sub-project's debt — path repoints throughout,
§11's header reframed away from "the GDScript half", the seed/demo
paragraph's citation moved — worth a full read-through rather than a
mechanical find-replace, since several of its fifteen line hits sit
inside prose written for a GDScript reader (e.g. "the reader who has to
fix it is looking at `main.gd::capture_env`").

---

## 5. SP3 — authored vocabulary and reusable composition

This section is debt only. Do not apply it to the wiki before the campaign's
merge gate.

### Mechanics — Levels and Objects

- Replace every magic-name spawn recipe with `WaveSpawn`. The first typed
  datum in depth-first scene order wins; an absent datum falls back to the
  level origin with a warning, and every duplicate loser is warned. Its global
  transform determines both position and facing, so a spawn nested in a
  rotated room works without copied angle data (`rust/src/nodes/spawn.rs`,
  `rust/src/nodes/level.rs`).
- Add the prefab doctrine: reusable content has a plain `Node3D` root and is
  composed from typed Rust nodes. Chair and table examples live under
  `game/scenes/props/`; the configured doorway and 16×16 room live under
  `game/scenes/rooms/`. Rust-generated preview limbs are ownerless derived
  data, never authored children (`game/tests/probe/editor_prefab_probe.gd`).
- Document `WaveRun.from`, `to`, and `openings`. Endpoints are parent-local
  X/Z coordinates. Godot displays each `Vector2` as `x` and `y`; in this
  planar authoring API that displayed `y` means world Z. Each opening is
  `(absolute start coordinate on the selected axis, width)`, with negative
  width treated by magnitude. Runs normalize reversed endpoints, choose the
  dominant axis with X winning ties, warn while folding diagonals, merge/clamp
  openings, and emit every positive residual as ownerless `RunSeg1…N` walls
  (`rust/src/level_plan.rs`, `rust/src/nodes/run.rs`).
- Add the level-selection recipe: `UnseeingGame.level_scene` is a PackedScene
  picker; empty means the exact level-01 fallback, while a selected scene is
  the only scene tried and must have a `WaveLevel` root. Level 02 demonstrates
  the reusable room, typed spawn, fan, chair, and interior run entirely as
  scene composition (`rust/src/nodes/game.rs`, `game/scenes/level_02.tscn`).
- Correct the run workflow: a raw `WaveLevel` tab is content without the
  player, hearing pass, materials, or wave pool and is not a playable F6
  target. Duplicate `main.tscn`, select its `UnseeingGame` root, assign the
  desired level to `level_scene`, and choose **Run Current Scene** or press F6
  from that runner tab. F5 runs the shipped main project scene and therefore
  follows its own `level_scene` picker. Do not call F6 “Run Custom Scene”.

### Research — Editor Authoring corrections

- Delete the stale `SpawnPoint` name-law passages and the old 16-class/eight-
  icon counts. The corrected SP3 close is 19 registered classes and ten icons,
  including the previously omitted `WaveRestorer`, `WaveSpawn`, and
  `WaveRun`.
- Replace the claim that room prefabs remain future scope: SP3 ships plain-root
  chair, table, doorway, and `room_16x16` scenes and exercises drag, rotation,
  repacking, recursive census, ownerless limbs, and inherited global heading in
  the editor probe.
