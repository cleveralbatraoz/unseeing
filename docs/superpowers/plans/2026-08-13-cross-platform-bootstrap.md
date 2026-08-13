# Cross-Platform Designer Bootstrap — Implementation Plan

**Goal:** give Windows x86_64 and ARM64 the same one-command, census-proven
authoring setup already available on macOS and Linux.

## Task 1: Pin the Windows behavior fail-first

- Add `test/bootstrap_windows_test.ps1` using recording Cargo and Godot
  executables in an isolated temporary directory.
- Require both Windows target routes, release `editor-docs`, import then census,
  exact-version refusal, and nonzero build/census propagation.
- Run it and record RED because the Windows entry point does not exist.

## Task 2: Implement the native Windows path

- Add `tools/bootstrap.ps1` with total architecture mapping, official rustup
  installation, current-process PATH refresh, build diagnostics, Godot
  discovery/version pinning, import, and census.
- Add `tools/bootstrap.cmd` as the execution-policy-safe one-command wrapper.
- Update `tools/bootstrap.sh` so unsupported hosts point to the Windows entry
  point instead of the retired manual Cargo recipe.
- Run the PowerShell boundary suite and both real POSIX bootstrap paths.

## Task 3: Make portability durable

- Update the root README, Godot project README, complete opening tutorial,
  active handoff, and wiki-debt record.
- Add a Windows CI job that runs the boundary suite and the real bootstrap
  against the pinned official Godot editor.
- Integrate the cheap PowerShell suite into the POSIX pipeline when `pwsh` is
  available, while the Windows job always runs it.

## Task 4: Verify and review

- Mutation-check architecture selection, `--target`, `editor-docs`, Godot pin
  comparison, import/census order, and nonzero propagation.
- Run shell syntax/ShellCheck, the PowerShell suite, real macOS bootstrap,
  repository hygiene, and the complete checks-only pipeline. The user's live
  `game/project.godot` and `game/scenes/level_01.tscn` edits remain unstaged and
  untouched.
- Obtain independent specification and quality reviews, fix findings, rerun
  the relevant gates, update the handoff decision record, and commit only the
  bootstrap task files.
