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
- UI/UX is simple and minimalistic.
- Inspiration: modern codebases and games in the blind-protagonist /
  echolocation genre — *Perception*, *Dark Echo*, *Stifled*.

## Platforms and stack

**One source of truth: `game/`, the Godot 4.7 project.** Supported platforms
are **Windows, macOS, and web** — all produced by *exporting* that one
project. Never write a separate implementation per platform.

- **Web** ships continuously — the wasm export, live at
  https://dggrus.hlab.kz, deployed through the test-gated `deploy.sh`
  pipeline: headless test runner → strict web export → browser smoke gate
  (`test/`, headless-Chrome DevTools probes) → atomic deploy → HTTPS
  byte-verify.
- **Windows and macOS** are exported on demand (when the user asks), not on
  every push.

Keep the technology stack deliberately small. Approved: Godot, typed
GDScript, GDExtension C++ (the wave/physics core), wasm, and the tooling
listed below. Before introducing any other technology, ask the user.

## Your role

Act as a **principal game developer engineer**. Follow modern industry
conventions and technologies. Don't hesitate to write complex, smart code —
apply difficult concepts when the problem calls for them. Don't overload the
code, but don't dumb it down either. Avoid global state.

The technical bar: **ideal**. Every decision must be proved, not assumed.
Physics and sound-wave propagation are the main technical challenge here and
must work perfectly.

## Workflow: research → questions → full autonomy

1. **Research first.** Before planning or asking anything, investigate the
   actual code. Never suppose — acknowledge by running code, tests, and
   tracing. Most problems are already solved by other projects: search GitHub/
   GitLab/the internet for existing solutions, and research the physics/math/
   mechanics itself when needed. Copy whatever works — license risk is
   accepted on this project.
2. **Ask every question upfront.** Surface all open questions *before*
   implementation starts, and keep asking until none remain.
3. **Then full autonomy.** Once questions are answered, proceed without
   further confirmation: install anything, run anything, use the server,
   spawn agents, consume whatever resources the task needs — including
   deploys.
4. **Keep memory and this file current.** When something new and *crucial*
   is figured out — a decision, constraint, or gotcha that changes future
   work — record it in persistent memory (crucial facts only, not
   everything) and update CLAUDE.md itself when the rules or stack evolve.

## Strict TDD

- Every behavior change starts with a test: **write the test → observe
  current behavior → change the code → test passes → done.**
- Trace every problem to its root. Never apply a fix naively; never touch
  code whose behavior you don't clearly understand.
- Test *everything* — features, physics, edge cases. Tests pin behavior down
  so anything can be changed fearlessly.
- Godot: **gdUnit4** is the test framework — migrate the custom headless
  runner (`game/tests/run_tests.gd`) into it. Browser-level behavior: the
  headless-Chrome smoke suite in `test/`. Visual verification: movie-maker
  frame rendering (`godot --write-movie`, run under `caffeinate -dis` on
  macOS).

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

## Code style

- **Small, total functions**: every function handles any input it can
  receive; think in functional-programming terms — clear inputs, clear
  outputs, no hidden state.
- **Dependency injection / independent components**: split code into pieces
  that don't rely on unclear implicit guarantees of each other.
- **Languages**: statically-typed GDScript for gameplay; the wave/physics
  core is a **GDExtension C++ module** — native speed on desktop, compiled
  to wasm via Emscripten for the web export, so the single source of truth
  holds on every platform. (C# was considered and rejected: Godot 4.x .NET
  builds cannot export to web.)

## Tooling

Formatters and analyzers are mandatory, run before every commit:
- **GDScript**: `gdformat` + `gdlint` (godot-gdscript-toolkit).
- **C++ (GDExtension)**: `clang-format` + `clang-tidy`; run tests with
  sanitizers (ASan/UBSan) where the harness allows.
- Adopt further instruments (fuzzers, static analysis, profilers) whenever
  they earn their keep — but they must not grow the shipped stack.

## Agents, reviews, tooling

- Use subagents freely — most tasks want a mix of tester / critic / software
  architect / programmer / product / plain-gamer perspectives. Run as many as
  needed.
- **Review is part of every task, proportional to its size**: features and
  physics/wave work get a full multi-agent design review + performance review
  (redesign if it fails); trivial fixes get a lightweight self-review.
  Algorithmic complexity and performance are always part of the review.
- Connect any MCP server that helps (e.g. tools that can actually *play* the
  game for UI/UX testing), or write one if it doesn't exist.
