# CLAUDE.md — Unseeing

## What this game is

Unseeing is a mystic/horror game about a **blind hero**. The whole point is to
visualize how a blind person *feels* the world — and the hero is blind, so the
visualization is the central design dilemma: we render a world its protagonist
cannot see. A blind person can't see, but they feel *more* than a sighted one.

Perception laws (non-negotiable):
- Black & white, **thin outlines only**. No textures, no fills, no materials,
  no bloat, no modern-game visual noise. Contours and vibe.
- The world is revealed by **waves** (sound, touch, wind). If nothing emits,
  nothing is visible.
- **One outline per object, and every seam between two objects draws.** The
  hearing pass draws two kinds of line: silhouettes, from a Laplacian of
  packed distance, and creases, from differences in a flat object id. Where
  two things interpenetrate there is no depth step, so the id difference is
  the *only* thing that can draw their seam — two touching objects sharing
  an id have no line between them and melt into one shape. So anything new
  that can touch something else needs an id at least 0.08 clear of it. The
  id budget and the graph colouring that hands them out live in
  `rust/src/oid_palette.rs`; never assign ids by cycling a list.
- UI/UX is simple and minimalistic.
- Inspiration: modern codebases and games in the blind-protagonist /
  echolocation genre — *Perception*, *Dark Echo*, *Stifled*.

## Platforms and stack

**One source of truth: `game/`, the Godot 4.7 project.** Supported platforms
are **Windows, macOS, and web** — all produced by *exporting* that one
project. Never write a separate implementation per platform.

- **Web** ships continuously — the wasm export, live at
  https://206.223.241.165, deployed through the test-gated `deploy.sh`
  pipeline: headless test runner → strict web export → browser smoke gate
  (`test/`, headless-Chrome DevTools probes) → atomic deploy → HTTPS
  byte-verify.
- **Windows and macOS** are exported on demand (when the user asks), not on
  every push.
- **Architecture independence: the game must work on both x86_64 and
  arm64.** Never rely on a particular architecture — no arch-specific code
  paths, intrinsics, or assumptions. macOS ships a universal binary; Windows
  ships both x86_64 and arm64 exports; the Rust core must build for
  x86_64 + aarch64 on every desktop platform, plus wasm32 for web.

Keep the technology stack deliberately small. Approved: Godot, typed
GDScript, GDExtension Rust (the wave/physics core), wasm, and the tooling
listed below. Before introducing any other technology, ask the user.

## Documentation: the wiki is the reference — read it first, write it back

Deep reference documentation lives in the project's **GitHub wiki**
(`https://github.com/cleveralbatraoz/unseeing/wiki`, cloneable as
`unseeing.wiki.git`). It exists for exactly one reason: a session opening a
technical task should not have to re-derive this system from the source
every time. That stays true only if every task reads it and pays it back:

1. **Read the wiki before implementing or researching anything.** It is the
   tutorial on how the mechanics work *today* — open it first, and let it
   collapse most of the research phase before you go near the code. This is
   not optional politeness to the docs; it is the cheapest step in the task.
2. **Update the wiki when the task is done, before you call it done.** Every
   mechanic you changed or added, and every research result you produced,
   lands on a page — rewritten to describe the new behaviour, not appended
   as a changelog. **If no page covers it, create the page and fill it.**
   Research with no wiki page is research the next session will have to
   redo, so it does not count as finished work.

Six pages, each naming the file that owns every constant it quotes:

- **Mechanics Overview** — the one idea, the two layers, the file map, the
  frame end to end, and the five laws that break everything if violated.
  *Start here; it links onward.*
- **Mechanics — Rendering** — the two passes, the R/G/B channel protocol,
  the outline maths, the acoustic-image depth band, the mood layer.
- **Mechanics — Waves** — the 64-slot pulse pool, pulse kinds and their
  privileges, reflections, and the wall occlusion Rust and GLSL share.
- **Mechanics — Sound Sources** — the source abstraction, the volume law,
  the fan against the radio, and how to add a third.
- **Mechanics — Level and Objects** — the four solid shapes, what the level
  derives from an authored scene, the object-id budget, the shipped map.
- **Engineering — Build, Test, Deploy** — the two toolchains and why, the
  gate, how to read a failure, vendoring, the binary policy, deploy traps.

Plus the platform research reports (gdext reliability, Linux CI, Steam).

**The wiki is a description of the code, never a second source of truth.**
Where the two disagree the code wins — and updating the wiki in the same
session is part of the change, exactly like updating this file.

**Specs and plans are a different artifact and live in the repo**, under
`docs/superpowers/specs/` and `docs/superpowers/plans/` (see the workflow
below). A spec records what we decided to build and why, frozen at the
moment of the decision; a wiki page describes how the shipped thing works
now. Neither replaces the other, and a task that produces a spec still owes
the wiki its page.

## Your role

Act as a **principal game developer engineer**. Follow modern industry
conventions and technologies. Don't hesitate to write complex, smart code —
apply difficult concepts when the problem calls for them. Don't overload the
code, but don't dumb it down either. Avoid global state.

The technical bar: **ideal**. Every decision must be proved, not assumed.
Physics and sound-wave propagation are the main technical challenge here and
must work perfectly.

## Workflow: superpowers governs

The **superpowers** plugin (v6.2.0, official marketplace, installed at user
scope) owns the development process here. Its fourteen skills are the
method; this file is the project's specifics. **Where a skill and this file
disagree, the skill wins** — a deliberate 2026-08-10 decision, and the rules
below were rewritten to match rather than left to contradict.

That direction has to be stated explicitly, because superpowers points the
other way by default: `using-superpowers` ends "User instructions (CLAUDE.md,
AGENTS.md, …) take precedence over skills." This file *is* such an
instruction, and it uses that precedence to hand authority back. **The
delegation is the instruction** — do not cite that clause to reverse it.
The skills set the floor, never the ceiling: where this file asks for more
than a skill does (below), that is an addition, not a conflict.

The spine, in order:

1. **brainstorming** — before any feature, component, behaviour change or
   other creative work. Its hard gate holds: no implementation until a
   design has been presented and approved. Its first step, "explore project
   context", is where this project's research rule lives — **start from the
   wiki** (above), then the code, then the internet. Never suppose; verify
   by running code, tests and traces. Most problems are already solved
   elsewhere: search GitHub/GitLab/the web for existing solutions and
   research the physics/maths itself when needed. Copy whatever works —
   license risk is accepted on this project.
2. **writing-plans** — the approved spec becomes a bite-sized, TDD-shaped
   plan whose Global Constraints carry this file's non-negotiables verbatim:
   the perception laws, the **object-id clearance**, the platform set, the
   two layers, **the commit rules and the attribution ban**. That last pair
   is not decoration. An implementer subagent is dispatched with its task
   brief and the Global Constraints and nothing else — never the whole plan,
   never this file — so a rule absent from that block does not exist as far
   as the agent doing the work is concerned. Related: a plan's commit step
   states *that* the task commits, never a literal `git commit -m "feat: …"`
   line. The skill's template shows one as illustration; transcribed into a
   brief it becomes the commit message this project spent a whole history
   rewrite to make impossible.
3. **subagent-driven-development** (preferred) or **executing-plans** —
   execution, with a fresh implementer per task, a task review after each,
   and the ledger under `.superpowers/` that survives compaction.
4. **finishing-a-development-branch** — the integration decision.

Alongside them, always: **systematic-debugging** for any bug, test failure
or unexpected behaviour before proposing a fix;
**verification-before-completion** before any claim that something works;
**requesting-code-review** after each task and before merge;
**using-git-worktrees** at the start (see below).

**What this changed, deliberately:**

- **Autonomy is now bounded at both ends.** It used to run from "questions
  answered" straight through to deploy. Brainstorming's gate now stands at
  the front — a written, approved design before code — and
  finishing-a-development-branch stands at the back: merge, PR or keep is
  the user's choice, presented as that skill's menu, never assumed. Between
  those two gates autonomy is unchanged and total: install anything, run
  anything, spawn agents, use the server. **Deploying is the exception,
  and it always was** — it *follows* the integration decision rather than
  running inside the autonomous stretch, because `deploy.sh` ships
  `git push production main` and refuses to run from any other branch or a
  dirty tree (`deploy.sh:16-26`): a core built from a feature branch would
  not be the core that ships, and no gate downstream could tell. So the
  order is merge first, then deploy — and since `main` can be checked out
  in only one worktree, that deploy runs in the shared checkout too, the
  second sanctioned action there after the merge itself.
- **Questions come one at a time now, not all upfront.** The old rule was
  to surface every open question before implementation and keep asking until
  none remained. Brainstorming's is the opposite — one question per message,
  multiple choice where possible, refining as it goes — and it is the more
  visible day-to-day change in this whole adoption.
- **Subagents are part of the method, not an escalation.** Implementers,
  reviewers and parallel investigators get dispatched as the skills
  prescribe, without asking first.
- **Specs and plans are committed to this repo**, at superpowers' paths:
  `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md` and
  `docs/superpowers/plans/YYYY-MM-DD-<feature>.md`. This does not demote the
  wiki — the two hold different things. A spec is what we decided to build
  and why, frozen at decision time; the wiki is how the shipped thing works
  today. Both get written.
- **Review is mandatory, not proportional to size.** Every task and every
  merge, dispatched to a subagent with superpowers'
  `requesting-code-review/code-reviewer.md` rubric. That raised the floor —
  trivial fixes no longer get a self-review — and it removed no ceiling:
  **physics and wave work still gets its full multi-agent design and
  performance review, with redesign if it fails.** A diff review asks
  whether the design decisions were sound; it is not the same instrument
  and does not replace it. Algorithmic complexity and performance remain
  part of every review.

**Write it back.** Unchanged and still the last step: the task ends in
documentation, not in a green test. Update the **wiki** pages for every
mechanic you touched and every research result you produced (creating the
page if there isn't one), record the *crucial* facts — a decision,
constraint or gotcha that changes future work — in persistent memory, and
update this file when the rules or stack evolve.

## Parallel sessions: one worktree each

**Every session/task works in its own git worktree.** Multiple sessions and
agents modify this repo in parallel; never work directly in the shared main
checkout. Follow **using-git-worktrees**: detect existing isolation first
(`git rev-parse --git-dir` against `--git-common-dir`, minus the submodule
case), and if you are already in a linked worktree do not create a second
one.

**Where a native worktree tool exists, use it and never `git worktree
add`.** That is the skill's own priority order, and it matters here: this
project's worktrees live under `.claude/worktrees/`, which the native tool
owns end to end — placement, branch, and cleanup. A hand-rolled `git
worktree add` alongside it leaves state the harness cannot see or remove.
**With no native tool**, take the skill's fallback but keep this repo's
layout: `git worktree add .claude/worktrees/<branch> -b <branch>`. That
path is already ignored, which satisfies the skill's safety check; its
`.worktrees/` default is not this repo's layout and would need a new ignore
rule to be safe.

Cleanup follows the same ownership rule, from
**finishing-a-development-branch**: it claims only `.worktrees/` and
`worktrees/`, and everything else — `.claude/worktrees/` included — belongs
to the host, so release it with the harness's exit tool rather than `git
worktree remove`. Work lands back as the usual small green commits.

**The one sanctioned exception to "never the shared checkout":** that
skill's merge option runs `git checkout <base>; git pull; git merge` in the
main checkout, because that is where `main` lives. Merging is an
integration action, not work, so it is allowed there — but only against a
clean tree. If `/Users/dmgalchenko/unseeing` has uncommitted changes or has
been left on another branch, stop and say so rather than checking out over
someone's work; other sessions may be live in it.

## Strict TDD

The **test-driven-development** skill is the procedure; this is what it
means here.

- Every behavior change starts with a test: **write the test → watch it fail
  for the right reason → minimal code → watch it pass → refactor.** The
  middle step is not a formality: a test you never saw fail has not been
  shown to test anything.
- **Production code written before its test gets deleted, not retrofitted.**
  Not kept as reference, not adapted while the test is written. Exploration
  is allowed and thrown away.
- **Every test names the break it catches** (`writing-good-tests.md`), and
  the wave core is exactly where that discipline pays:
  - **No mirror assertions.** An expected value computed by the code under
    test — or by its helpers — passes no matter what that code does.
    Propagation maths is the natural home of this bug. Hand-derive the
    literal, or use a checked fixture.
  - **No change detectors.** Not `assert_eq!(REACH_PER_VOLUME, 12.0)` but
    the behaviour that depends on it: a source at volume 0.5 reaches 6 m and
    not 7.
  - **Run the mutation check before finishing.** Flip a constant, a branch,
    a side effect, an early return — each realistic mutation must fail at
    least one test. A mutation nothing catches marks the behaviour as
    unprotected.
- Test *everything* — features, physics, edge cases. Tests pin behavior down
  so anything can be changed fearlessly.
- **Debugging is its own procedure**, not an improvisation:
  **systematic-debugging**. Root cause before any fix, traced backward to
  the original trigger; one hypothesis at a time; the failing test comes
  before the fix. And the rule this project did not have — **after three
  failed fixes, stop and question the architecture.** Three fixes that each
  reveal a new problem somewhere else is not a run of bad hypotheses, it is
  a wrong design, and the fourth attempt will not find it.
- **Arbitrary waits are a defect** (`condition-based-waiting.md`): poll for
  the condition, never guess the duration. A `sleep` inside the deploy gate
  either blocks a good deploy or waves a bad one through. **Standing debt,
  and exactly two lines of it:** `test/web_smoke.sh:33` sleeps 3 s for
  Chrome's DevTools port — redundantly, since `web_probe.py:20-30` already
  polls `/json/list` thirty times for that very target — and
  `test/web_probe.py:92` sleeps `SMOKE_WAIT` (22 s by default) before
  evaluating, with no condition behind it. Those two are the debt. The 1 s
  inside the probe's polling loop is condition-based waiting and is correct;
  the 1 s in the smoke script's EXIT trap is cleanup, not a gate wait.
- **The skill's exceptions are declined here.** test-driven-development
  permits skipping TDD for throwaway prototypes, generated code and config
  files after asking. This project doesn't: prototypes get thrown away and
  rewritten test-first, and the generated code that matters — the Rust
  core's registered node classes — is exactly the code the physics rides
  on. Stricter than the skill on purpose.
- Godot: **gdUnit4** is the test framework — suites in `game/tests/`, run
  headless from `ci/pipeline.sh` (the old custom runner is gone; that
  migration is done). Browser-level behavior: the headless-Chrome smoke
  suite in `test/`. Visual verification: movie-maker frame rendering
  (`godot --write-movie`, run under `caffeinate -dis` on macOS).

### Display defaults, and how to run windowed anyway

The game **boots full screen at the monitor's own resolution**
(`display/window/size/mode=3`), with the web exempted by feature tag
(`mode.web=0` — a browser grants the Fullscreen API only inside a user
gesture, and Godot's web display server swallows the rejected promise, so
a boot request fails *silently*). No content scale is set, so the viewport
IS the window and "native resolution" needs no machinery. Escape opens the
settings overlay (`SettingsMenu`), which freezes the world, frees the
mouse, and can toggle full screen or pick a resolution. Nothing is
persisted — every launch starts from these defaults, by design.

**`--windowed`, `-w` and `--resolution` cannot override this.** Measured,
and confirmed in `main/main.cpp`: the flags are parsed into `window_mode`,
then that variable is overwritten wholesale from
`display/window/size/mode` a thousand lines later — and the flags are
consumed on the way through, so `OS.get_cmdline_args()` never sees them
and no script can compensate.

To run windowed — rendered probes, `--write-movie`, any run whose frame
size must be identical on every machine — write a **`game/override.cfg`**,
the engine's documented escape hatch (merged over `project.godot` before
the window is created):

```
[display]

window/size/mode=0
window/size/viewport_width=1280
window/size/viewport_height=720
```

`tools/probe_visibility.sh` does exactly this and removes the file however
it exits. `game/override.cfg` is gitignored and `test/repo_hygiene.sh`
pins that: committed, it would silently un-fullscreen the shipped game.

## Commits

- Split every job into small, self-contained commits — one behavior per
  commit. Anyone (human or agent) reading `git log` must fully understand
  what changed and why.
- Each commit is **green**: the new test and the code that makes it pass land
  together.
- Style: narrative, evocative subject line (matching the existing history,
  e.g. "The fan blows a real wind: a sweeping cone of waves, walled into its
  room"), with a body carrying the precise technical what/why.
- **All work is authored on behalf of the user, never the assistant.** No
  Co-Authored-By or "Generated with" trailers; no mention of Claude, AI, or
  any assistant anywhere in the repository — commits, code, comments, docs,
  or PRs. Repo-local git identity: `Dmitrii Galchenko <dggrus@gmail.com>`.
- **What that ban is, precisely: authorship attribution.** It was never a
  ban on the words. This file is named for a tool, `.gitignore` describes
  "per-session agent worktrees", and specs and plans now committed under
  `docs/superpowers/` carry the header writing-plans mandates verbatim,
  naming its own sub-skill — all fine, because none of them claims an
  assistant wrote the work. The line is: **nothing in this repository may
  credit, sign, or present an assistant as an author or collaborator.**
  Tool identifiers, agent-facing instructions and process documentation are
  not attribution. Commits, code comments and PR bodies stay clean of both.

## Binary assets: keep them out, and know when that stops being free

**The repo is source-only, and the perception laws are what make that
possible.** Black and white, thin outlines only, no textures or materials,
geometry built in Rust as `ImmediateMesh`, sound synthesised rather than
sampled — the art direction *is* an asset budget of zero. As of the
2026-08-04 audit the entire tracked binary payload is 10 PNGs / 700 KB
(one README screenshot plus vendored gdUnit4 icons), every one at exactly
one revision, and none of it reaches a shipped Windows, macOS, or web
artifact.

- **Never commit build output.** Exports, `.pck`, `.wasm`, `target/`,
  `--write-movie` frames, reports. `.githooks/pre-commit` rejects any staged
  file over 5 MiB (`ALLOW_BIG=1` to override deliberately);
  `test/repo_hygiene.sh` holds the standing invariants and runs first in
  `ci/pipeline.sh`.
- **Do commit** `.import` and `.uid` sidecars — Godot's docs require it, and
  the project tracks one per script with zero orphans.
- **git-lfs is rejected, not deferred.** The cost of a binary in git is
  size × revisions, and this repo's churn is 1. LFS would break the deploy
  (`deploy.sh` pushes to a *bare* repo whose post-receive hook does
  `git archive | tar -x`; with no filter configured it would ship 130-byte
  pointer files, and the hook returns tar's status, so it would fail
  silently), break the README image on GitHub raw, add a required dependency
  to a droplet that cannot run `sudo` unattended, and cost a second history
  rewrite to adopt or leave. Same verdict for git-annex, DVC, and external
  object storage.
- **Reopen the question only on a real trigger**: a single file over ~50 MiB,
  `size-pack` over ~500 MB, or — the one that will actually happen first —
  **any binary reaching its 5th revision**. The likely path there is
  committing golden frames for visual regression; store perceptual hashes or
  downsampled baselines instead of full frames.

## Code style

- **Small, total functions**: every function handles any input it can
  receive; think in functional-programming terms — clear inputs, clear
  outputs, no hidden state.
- **Dependency injection / independent components**: split code into pieces
  that don't rely on unclear implicit guarantees of each other.
- **The two layers (the engine/content split — 2026-08-02 decision).**
  The game is built so non-technical collaborators can create and modify
  content in the Godot editor without ever meeting the machinery:
  - **Rust (`rust/`, godot-rust / `gdext`) = the engine, hidden**: wave
    simulation, echo physics, player kinematics, viewmodel animation math,
    perception/graphics internals. Exposed to Godot only as registered
    node classes with designer-meaningful `#[export]` knobs, typed
    signals, and in-editor docs (`register-docs`). All logic lives here,
    pure modules cargo-tested; native on desktop, wasm on web — one
    source of truth on every platform.
  - **Godot = the game, visible**: editor-authored `.tscn` scenes (levels,
    placements, sound sources) and thin statically-typed GDScript for
    game-facing scripting only — triggers, sequences, tuning — written
    against the Rust nodes' API. Levels are authored in the editor, never
    in code; technical contracts (hum rooms, level data) are derived from
    the scene by the engine layer.
  - Rust won for safety and testing culture — it directly serves the
    total-functions and fearless-refactoring doctrine. (C# was rejected:
    Godot 4.x .NET cannot export to web. C++ was chosen briefly, then
    replaced.) The wasm export allows exactly ONE Rust extension: every
    native system joins the single crate.
- **No unsafe Rust.** The crate is `#![deny(unsafe_code)]`; the single
  permitted exception is the `unsafe impl ExtensionLibrary` entry point
  that gdext's API mandates (a targeted `#[allow]` in `ffi.rs`). Never
  add another exception — if a problem seems to need `unsafe`, redesign
  or ask the user.
- **Rust web-export constraints** (as of 2026, gdext wasm rides the
  bleeding edge). Two toolchains, pinned in two places, and the split is
  deliberate:
  - `rust/rust-toolchain.toml` pins the **stable** channel every native
    build, test and clippy run uses, so laptop, droplet and CI compile
    with the identical compiler.
  - `rust/build-wasm.sh` pins the **nightly** used for wasm *only* —
    gdext's web build needs `-Zbuild-std` and `-Zemscripten-wasm-eh`,
    both nightly-only. Never promote that nightly into
    `rust-toolchain.toml`: it would put desktop and CI on a moving
    compiler and forfeit the reproducibility the stable pin exists for.

  Also pin the Emscripten version to match the Godot build, keep link
  flags in sync with Godot's web export settings, and remember only ONE
  Rust GDExtension can live in a wasm export. Verify the web build in CI
  on every core change — toolchain churn is the known risk we accepted.

## Tooling

Formatters and analyzers are mandatory, run before every commit:
- **GDScript**: `gdformat` + `gdlint` (godot-gdscript-toolkit).
- **Rust (GDExtension)**: `cargo fmt` + `cargo clippy` (warnings are
  errors) + `cargo test`; reach for Miri or `cargo-fuzz` on the trickiest
  wave-math when it earns its keep.
- Adopt further instruments (fuzzers, static analysis, profilers) whenever
  they earn their keep — but they must not grow the shipped stack.

### Vendored dependencies

Godot resolves addons as project resources, so a third-party framework has to
live in the tree (`game/addons/`) — **never a submodule**: upstream ships no
`.uid` sidecars, Godot mints hundreds of them on import, and inside a
submodule they are permanently dirty and uncommittable. The copy is pinned
instead, and is never hand-edited:

- `ci/gdunit4.lock` records the upstream repo, tag, commit, and two
  fingerprints — upstream's shipped source and our resulting tree (content
  plus executable bits).
- `ci/vendor-gdunit4.sh update <tag>` is the only sanctioned way to change
  the addon; `check-upstream` re-verifies the pin against GitHub; the bare
  `verify` runs inside `ci/pipeline.sh` on every build, so drift cannot land
  silently — the pre-commit hook deliberately skips `game/addons/`.
- In-editor self-updaters stay **off** (`game/project.godot`). gdUnit4's
  would poll GitHub on project open and, on one click, delete the addon and
  unpack an unreviewed release over it. Version bumps are reviewed commits.

Godot itself is not vendored: `.godot-version` pins it, CI downloads that
exact release, and `ci/pipeline.sh` refuses a mismatched binary.

## Agents, reviews, tooling

- Use subagents freely — most tasks want a mix of tester / critic / software
  architect / programmer / product / plain-gamer perspectives on the *same*
  question. Run as many as needed; that is unaffected by anything below.
- **dispatching-parallel-agents governs how work is divided**, which is a
  different question: one agent per *independent* problem domain, each with
  a scope, a constraint and a named return, and related failures
  investigated together rather than split. Many lenses on one problem is
  review; many problems in parallel is division. Don't let the second rule
  eat the first.
- **Review is mandatory after every task and before every merge** — not
  proportional to size, which is the one place this project's old rule gave
  way. Dispatch it to a subagent with superpowers'
  `requesting-code-review/code-reviewer.md` rubric, and hand it a diff file
  rather than your session's history. **Physics and wave work keeps its
  full multi-agent design and performance review on top of that**, with
  redesign if it fails — the diff rubric asks whether design decisions were
  sound, which is not the same instrument. Algorithmic complexity and
  performance remain part of every review. Reviewers are read-only: never
  move HEAD on the checkout under review — with this many live worktrees
  that is not a theoretical concern.
- **Feedback gets verified, not performed** (**receiving-code-review**).
  Check every suggestion against this codebase before implementing it, push
  back with technical reasoning when it is wrong for the stack, and clarify
  the whole list before implementing any of it.
- **A subagent's success report is not evidence**
  (**verification-before-completion**). Check the diff. This project has
  already paid for the general form of that lesson once: `git push` reported
  success while the post-receive hook failed, which is why `deploy.sh` reads
  `UNSEEING_BUILD` back off the live page instead of trusting an exit code.
- Connect any MCP server that helps (e.g. tools that can actually *play* the
  game for UI/UX testing), or write one if it doesn't exist.

### The installed skill set

`superpowers@claude-plugins-official` v6.2.0, user scope, upstream commit
`44c9b2d`. Update with `claude plugin install`/`marketplace update`.

**It is deliberately unpinned, and that is a real exception to the rule two
sections up.** gdUnit4 gets a lock file, a `verify` step in every build, and
a disabled self-updater, because it is a project resource that ships inside
the tested artifact — drift there changes what the game does. This ships
nothing: it is user-scope tooling that never reaches a Windows, macOS or web
export, so there is no lock file, no `game/addons/` entry and no CI check.
The cost of that is honest: a `marketplace update` run in an unrelated
project changes the method governing this one, silently. The version and
commit above are therefore informational, and **a version bump is a reviewed
edit to this section** — the same standard as gdUnit4's, enforced by
attention rather than by CI.

- **Process:** using-superpowers, brainstorming, writing-plans,
  executing-plans, subagent-driven-development,
  finishing-a-development-branch.
- **Craft:** test-driven-development (+ `writing-good-tests.md`),
  systematic-debugging (+ root-cause-tracing, defense-in-depth,
  condition-based-waiting), verification-before-completion.
- **Collaboration:** requesting-code-review, receiving-code-review,
  dispatching-parallel-agents, using-git-worktrees.
- **Meta:** writing-skills — TDD for process docs. Use it for any
  project-specific skill we author (a deploy skill, a probe skill), and
  baseline an agent *without* the doc before writing it.

Three consequences worth stating outright, because they touch rules
elsewhere in this file:

- **brainstorming may offer a browser "visual companion"** — a local Node
  HTTP server for mockups and diagrams. That is outside the deliberately
  small stack, so it stays exactly as the skill defines it: offered to the
  user, run only on acceptance, and never a dependency of the game, the
  build, or the deploy.
- **The commit rules survive, but they are now reachable from a plan.**
  Superpowers has no commit-message doctrine of its own — the `feat:`
  strings in its plan template are illustration — so the narrative subject
  lines stand. What changed is who reads them: a plan's task steps become
  an implementer's entire brief, so the rules have to travel in Global
  Constraints (see the workflow above) or they will not be there when a
  commit is written.
- **Committed specs and plans do not breach the attribution ban.** They
  carry writing-plans' mandated header naming its own sub-skill, and
  brainstormed prose picks up the skills' phrasing. That is process
  documentation, not authorship — see the Commits section, which now draws
  that line explicitly rather than leaving it to be rediscovered.
