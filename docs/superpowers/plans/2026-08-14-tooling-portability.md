# Tooling portability — plan and closeout

Design: [2026-08-14-engine-selection-design](../specs/2026-08-14-engine-selection-design.md).

Driven by an audit run on two real machines at `e6e5511`: Debian 13 x86_64 with
Godot installed as `godot-4` (a 4.7 mono snap) and a 4.7.1 in `~/bin`, and
Windows 11 26200 AMD64 reached over `ssh swift-local` with the official 4.7.1
archive on `PATH`. Neither could bootstrap. 44 findings survived adversarial
verification; the ones below are the ones that were fixed, each with the
evidence that fixed it.

## Global constraints carried into every step

The perception laws, the label clearance and superface merge law, the supported
platforms (x86_64 and arm64 across macOS, Windows and web), the two code layers
(Law 1 designer-facing Godot objects, Law 2 everything else in Rust), small
green commits with an evocative subject, and no assistant attribution anywhere.

None of this work touches the game. It is developer tooling only: no shipped
artifact, no scene, no Rust behaviour change, no deployment change beyond a
preflight and a gate that stops skipping itself.

## Commits, in order

1. **`8f6aa4d` One place decides which engine the world runs on** — the spec,
   `tools/lib/engine.sh`, `test/engine_select_test.sh` (29 cases), wired into
   the pipeline. Eight mutations of the library, all killed.
2. **`bbe635f` Every tool that wakes the engine now asks the same question** —
   eleven callers migrated, `test/engine_callers_test.sh` asking each both
   questions against copies of the checkout.
3. **`eb7a74f` The bootstrap looks for the editor before it earns the right
   to** — POSIX ordering, Mono acceptance, missing-pin refusal, `CARGO_HOME`.
4. **`26db0dd` Windows learns the name the editor actually arrives under** —
   official archive names, version-aware discovery, silence diagnosed as
   silence, `%USERPROFILE%`, streamed build output proven by a parked build.
5. **`f0f21f9` A missing formatter stops claiming the guards are broken** —
   gdtoolkit check moved to the top; the hygiene suite names the real cause.
6. **`7e8c24d` Scratch files stop landing where their reader cannot follow** —
   restore blob out of a sandboxed `/tmp`, export log under `TMPDIR`,
   `override.cfg` refused rather than clobbered, HUP in the traps.
7. **`7e69926` An absent hasher can no longer vouch for a tampered tree** —
   `tools/lib/digest.sh`, python3 preflight, `.DS_Store` no longer drift.
8. **`380cfe7` The Apple gate asks the toolchain that will actually do the
   work** — cwd-correct target gate, rustup preflight, `lipo` host-vs-artifact,
   and the orphaned universal suite wired in.
9. **`5fff24f` The gate that proves the world renders stops disappearing
   quietly** — smoke test refuses instead of exiting 0, kernel-assigned ports,
   readiness polling replacing the sleep, CDP errors diagnosed, deploy
   preflight.
10. **`1fa4778` Two constants stop living in four files at once** —
    `ci/engine_class_count`; both suites given their own fixture pin and class
    count, which also removes a constant-change detector.
11. **`2070cb4` The agent pin gets a lock, and the upgrade path actually moves
    it** — `ci/superpowers.lock`, and developer-agent scripts export-ignored.
12. **`63ddaa1` One command from a source change to the world running** —
    `tools/run_game.sh`, `tools/run_game.cmd`, `tools/run_game.ps1`,
    `test/run_game_test.sh` (28 cases).
13. **this commit** — README, `docs/opening-in-godot.md`, and the wiki.

## Review round

A five-dimension review of the finished branch, each finding challenged by two
independent skeptics, returned 20 that survived. All 20 are fixed; the ones
worth naming because they were real defects the work itself introduced or
claimed falsely to have fixed:

- **`tools/export_macos.sh` could not be parsed.** The migration deleted an
  `if` and left its `fi`, and nothing noticed: the script's own `uname` guard
  exits before the shell reaches the bad line, so on Linux the file is never
  parsed that far and the pipeline stayed green. The only machine that would
  have found it is a Mac, at the moment someone cut a release.
  `test/shell_syntax_test.sh` now parses every tracked shell script, and was
  confirmed to catch this exact fault when it is put back.
- **The digest refusal never reached its caller.** `tools/lib/digest.sh`
  correctly refuses without a hasher, but `setup-agents.sh` compared two
  command substitutions — which discard exit status — so two failed digests
  compared equal and a tampered cache still passed. The comparison moved into
  the library as a three-answer call (identical / differ / cannot tell).
  Measured both ways: the old form certified a tampered tree, the new one
  refuses.
- **`test/web_smoke.sh`'s cleanup killed neither child.** `kill "" "$SRV"`
  aborts on the empty first argument in dash, and `CHR` is unset for the whole
  window between starting the server and starting the browser — which is where
  the readiness poll can fail. It leaked the HTTP server on the failure path.
- **PowerShell was claiming the POSIX flags.** A leading `--` resolves the same
  as `-`, so a declared `-Scene` parameter swallowed `--scene` and
  `--scene --demo` died before the script ran. `run_game.ps1` declares no
  parameters at all now and parses `$args` itself, so both spellings work and
  `--verbose` reaches Godot instead of PowerShell.
- **Assertions that could not fail.** "The run never opens the editor" searched
  a joined string for `' -e '`, which cannot see a launch whose last argument is
  `-e`; the windowed cases never read `override.cfg`, so writing full-screen
  mode passed; the trap's signal arms and the bare-name-on-PATH branch were both
  surviving mutations. Each is now killed by a named case.

Ten mutations of `tools/lib/engine.sh`, all killed.

## Found by running it

Three defects surfaced only when something other than a developer's machine ran
the work, and all three were the same shape — a check that passed for a reason
other than the one it claimed.

- **CI, round one.** `test/engine_select_test.sh` was not hermetic against an
  inherited `GODOT`. `.github/workflows/test.yml` exports it for the whole Linux
  pipeline, an inherited `GODOT` is an *explicit* engine to the library, and an
  explicit engine skips the candidate walk — so nine discovery cases were asking
  about the runner's editor instead of their fixtures.
- **CI, round two.** The identical defect in `test/bootstrap_windows_test.ps1`,
  which passed on the Windows job (no `GODOT`) and failed on the Linux one. The
  first fix was applied to one language and not generalised. Both PowerShell
  suites carry the guard now. `pwsh` is installed on the development machine so
  `ci/pipeline.sh` runs them locally rather than skipping them, which is what
  let round two be reproduced instead of inferred from a log.
- **A real `--migrate` run.** `tools/setup-agents.sh` uninstalled with a
  hardcoded `--scope user`, and the CLI refuses that against a `local` install,
  naming the scope it actually wants. Worse, the plugin in question was
  `local`-scoped to an unrelated checkout: it cannot load here, so blocking
  setup over it was wrong and removing it would have deleted a plugin out of
  somebody else's project. Scope now decides — `user` competes, `local`/`project`
  competes only when it belongs to this repository, an unreported scope is
  treated as competing — and the uninstall uses the scope the plugin reports.
  `test/setup_agents_test.sh` drives a recording fake CLI and asserts both the
  scope on the wire and that another project's plugin survives `--migrate`
  untouched.

## Evidence

| Check | Before | After |
|---|---|---|
| `tools/bootstrap.sh` on Debian, no `GODOT` | 45 s build, then `godot not found` | `bootstrap: OK`, `PASS (19 checks)` |
| the same with a wrong engine | 45 s build, then refusal | refused in 0.172 s, naming the engine and the pin |
| `tools\bootstrap.cmd` on Windows, editor on `PATH` | `godot not found` | `bootstrap: OK`, `PASS (19 checks)` |
| `test/repo_hygiene.sh` without gdtoolkit | exit 1, 20 misleading failures | one failure naming gdtoolkit |
| `setup-agents` integrity gate without `shasum` | passes a tampered tree | refuses |
| Apple target gate from the repo root | no apple targets (default toolchain) | both (pinned toolchain) |
| `ci/pipeline.sh SKIP_EXPORT=1` | could not start on either machine | exit 0, 34 stages, 31 suites / 328 cases |
| `tools/run_game.sh` | did not exist | windowed on Linux against OpenGL; headless on Windows |

Windows suite: 57 green. POSIX bootstrap suite: 30 green. New suites: 29 + 42 +
8 + 28.

## Deliberate behaviour changes

Announced rather than smuggled, and spec'd in the design:

- An explicit `GODOT` that fails the pin now **fails** instead of running.
- A Mono/.NET editor of the pinned version is now **accepted**.
- A missing `.godot-version` is now a **refusal**, not a disabled gate.
- Every probe now applies the pin, so a hand-run probe against a rejected
  engine no longer prints an authoritative verdict.
- `test/web_smoke.sh` **fails** without a browser instead of exiting 0.
  `SKIP_SMOKE=1` is the deliberate opt-out, and `ci/pipeline.sh` already
  honours it. **This one can block a deploy on a host with no Chrome** — which
  is the point, since that host was shipping without ever rendering a frame.

## Not done

Findings from the audit that were judged not to reproduce, and are recorded
here so they are not re-investigated: shallow submodule clones do fetch the
`v6.3.0` tag today and `ci/verify-superpowers.sh full` passes; exit codes
propagate correctly through `tools/bootstrap.cmd`; `pwsh -NoProfile -File` runs
the Windows suite without an execution-policy problem.

Left alone deliberately: `test/bootstrap_posix_test.sh` produces misleading
failures when run under Git Bash on Windows, where `tools/bootstrap.sh` refuses
by design. The production script has a platform guard; its harness does not.
Low value, and the suite is not meant to run there.
