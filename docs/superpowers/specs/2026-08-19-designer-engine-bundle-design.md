# Designer Engine Bundles — Design

**Status:** approved by the user (Dmitrii), following a full clarification pass
covering cadence, platform scope, build hosts, and artifact shape. Completes
the half of #38 the 2026-08-11 campaign spec deliberately deferred.

## Lineage — why this is not a duplicate of the 2026-08-13 bootstrap work

`docs/superpowers/specs/2026-08-11-editor-authoring-campaign-design.md`
scoped Sub-project 1 to close "#38 (as scoped)" via a one-time build
bootstrap, explicitly stating *"CI-published binaries are out of scope for
this campaign"* and *"the designer is a technical friend"*. That sub-project
landed on `main` 2026-08-13 as `tools/bootstrap.sh` /
`tools/bootstrap.cmd` (spec:
`2026-08-13-cross-platform-bootstrap-design.md`).

Issue #38 is still open, with no closing comment — unlike #17, #27, and
#43, which each got an explicit "Fixed and live on `main`" comment when a
past campaign actually closed them. The bootstrap script requires a
terminal and the ability to install a Rust toolchain; it does not, and was
never meant to, serve a collaborator who has neither. This spec builds the
deferred remainder: a path that requires no terminal, no toolchain, and no
git.

Both paths coexist. `tools/bootstrap.sh`/`.cmd` remain the entry point for
contributors with a working shell. Nothing here modifies them.

## Goal

A designer with no terminal, no Rust toolchain, and no git obtains a
working Godot editor for this project by downloading one file, unzipping
it, and opening `project.godot`. Nothing they do is a script, a command
line, or a build.

This is designer-facing distribution only. It does not touch the
player-facing export/deploy pipeline (`deploy.sh`, the export presets),
which already works and ships a different artifact for a different
audience.

## Decision

A new GitHub Actions workflow builds, verifies, and publishes a ready-to-open
editor bundle for every supported desktop platform, on every push to `main`,
entirely on GitHub-hosted runners.

### Build matrix — all on GitHub-hosted runners, no self-hosted machines

| platform | runner | native execution? |
|---|---|---|
| Linux x86_64 | `ubuntu-24.04` | yes |
| Linux arm64 | `ubuntu-24.04-arm` (GA, free on public repos) | yes — real ARM64 hardware |
| Windows x86_64 | `windows-latest` | yes |
| Windows arm64 | `windows-11-arm` (GA, free on public repos) | yes — real ARM64 hardware |
| macOS universal | `macos-latest` (Apple Silicon host) | arm64 slice: yes. x86_64 slice: cross-compiled, same as `tools/build_macos_core.sh` does today; not independently execution-verified (Rosetta 2 availability on the hosted image unconfirmed) |

Runner labels for the arm64 images move fast (GitHub GA'd them in August
2025 and has already rolled a `windows-11-vs2026-arm` variant into preview
in 2026) — confirm the current label at implementation time rather than
trusting this table blindly.

This closes a gap the 2026-08-13 bootstrap spec explicitly left open: *"An
actual ARM64 editor load remains unverified until an ARM64 Windows runner
is available."* One is now available, GA, for free, on public repos.

**Trigger is `push: branches: [main]` only** — never `pull_request`, never a
form of `workflow_dispatch` reachable by a fork. This isn't the self-hosted
runner risk (we're not using self-hosted runners here), but it's still the
right discipline: nothing about producing a designer-facing binary should be
triggerable by someone else's PR.

### Per-platform job

1. Build `--release` from the pinned toolchain (`rust/rust-toolchain.toml`),
   same target triples `unseeing.gdextension` already names. macOS builds
   both `aarch64-apple-darwin` and `x86_64-apple-darwin`, then fuses them
   with the existing `tools/build_macos_core.sh` (`lipo -create`) and
   verifies with `tools/check_universal.sh` — no new macOS logic, straight
   reuse of the #17 fix.
2. Import the built library into a pinned headless Godot and run
   `game/tests/probe/engine_census_probe.gd` — the exact probe
   `tools/bootstrap.sh` already uses to print `bootstrap: OK`. **A platform
   whose census fails does not get packaged.** Shipping a designer a bundle
   that fails to load would be worse than not shipping one; the whole point
   of this feature is a binary that is proven to work before it leaves CI.
3. Package the bundle (layout below) and upload it as a same-run internal
   Actions artifact (not yet public).

### Publish job — atomic, all-or-nothing

A final job, gated with `needs: [<all five build jobs>]` (GitHub Actions'
default `if: success()` on `needs` means it only runs if every platform
job succeeded), downloads all five internal artifacts and uploads them
together to a single rolling GitHub Release tagged `engine-latest`
(a **prerelease**, so it never shadows the player-facing `vX.Y.Z` "Latest"
release), using `gh release upload engine-latest <files> --clobber`.

If any one platform fails, **nothing is published for that push** — the
release keeps whichever assets it last successfully published, atomically.
A designer never receives a set where, say, Windows matches commit X and
Linux is still three commits behind: exactly the class of confusion #27 was
about, just moved one layer up the pipeline instead of eliminated.

Each bundle also carries an `ENGINE_COMMIT` text file at its root
containing the full commit SHA it was built from — the same stamping
pattern `deploy.sh` already uses (`core.commit`) for the identical reason.

### Artifact shape — the layout the `.gdextension` paths actually need

`game/unseeing.gdextension` resolves its library paths as
`res://../rust/target/...` — **one level above** the Godot project root.
The bundle is therefore not "just `game/`" but `game/` plus the single
matching binary, laid out as siblings from a common root. The zip contains
exactly one top-level folder (named e.g.
`unseeing-editor-linux-x86_64/`), so extracting it never splatters files
into whatever directory the designer happened to unzip in — everything
lands inside that one named folder, `game/` and `rust/` as its immediate
children:

```
unseeing-editor-<platform>/                (the zip's single top-level entry)
  ENGINE_COMMIT
  game/                                    (the full Godot project — no changes)
  rust/
    target/
      release/libunseeing_core.so          (Linux, both arches)
      release/libunseeing_core.dylib       (macOS, universal)
      x86_64-pc-windows-msvc/release/unseeing_core.dll
      aarch64-pc-windows-msvc/release/unseeing_core.dll  (only the arch matching this bundle)
```

No `rust/src`, no `Cargo.toml`, no `docs/`, no `test/`, no `.git`, no CI
scripts — a designer editing content in the Godot UI never needs any of it.
The designer's entire workflow: download the zip for their OS, extract it
(the wrapping folder keeps `game/` and `rust/` together automatically),
open `unseeing-editor-<platform>/game/project.godot` in Godot.

## Rejected / deferred, and why

- **A `tools/fetch-engine` download-and-unzip script.** Rejected mid-design:
  even a double-clickable script is still a script — OS security prompts
  (Gatekeeper, SmartScreen), and it presumes the designer trusts and
  understands running one. A plain file download replaces it entirely.
- **Self-hosted build/test machines (`mac-local`, `swift-local`) or a
  QEMU/Parallels VM for arm64 verification.** Considered when it looked
  like arm64 execution testing would need emulated or virtualized
  hardware. Superseded once GitHub's own hosted `windows-11-arm` and
  `ubuntu-24.04-arm` runners turned out to already give real native
  execution, for free, on this public repo — simpler and requires no
  infrastructure of ours to maintain.
- **Tag-only or dual (tag + rolling-prerelease) cadence.** Considered for
  lower CI cost. Rejected in favor of rolling-on-every-push specifically so
  a designer's download always matches `main`'s HEAD, eliminating the
  stale-binary risk class #27 already proved this project is vulnerable to.
- **Folding in adjacent Stage-1 work** (e.g. #30 — `SoundFan`/`SoundRadio`/
  `WaveCat` becoming visible `tool` classes). Deliberately out of scope;
  this spec is binary delivery only.
- **A git-based return path for a designer's edits.** Explicitly deferred,
  not designed here. The bundle has no `.git` and no way to contribute
  changes back; that remains a separate, future problem.

## Testing

- A packaging-layout assertion (shell-level, alongside the existing
  `test/` suites): given a built `game/` tree and a fake release library,
  the packaging step must place the library as a **sibling** of `game/`,
  never nested inside it — this is exactly the kind of relative-path detail
  that is silent and easy to get backwards, and a designer hitting it would
  just see "still broken" with no diagnostic pointing at why.
- Per-platform `engine_census_probe.gd` gates packaging, per job — reusing
  existing, already-verified tooling rather than inventing a new census.
- Before calling this feature done: manually download the actual published
  Linux bundle (the platform this machine can fully exercise) and open it
  in a real Godot editor, confirming the census passes and the level scene
  loads with real geometry, not `MissingNode` — the same failure mode
  originally reproduced against issue #38.

## Non-goals, explicit

- Does not change what players receive (`deploy.sh`, exports, the droplet).
- Does not change `tools/bootstrap.sh`/`.cmd` or their contract.
- Does not verify the macOS x86_64 slice by execution (matches today's
  status quo for that slice; only regresses nothing).
- Does not design a way for a designer's edits to reach the repository.
