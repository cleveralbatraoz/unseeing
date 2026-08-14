# Engine Selection — Design

**Status:** approved by the user's instruction to fix the cross-machine tooling
defects found by the 2026-08-14 audit on Debian 13 and Windows 11.

## Problem

"Select the Godot editor this repository is pinned to" is one decision. It was
implemented as two separable steps — discover a binary, then check its version
— and only the first step was copied around the tree. Measured at `e6e5511`:

- The loop `for g in godot "$HOME/bin/godot" /opt/homebrew/bin/godot` appears
  verbatim in **ten** POSIX files.
- Two more (`tools/determinism_probe.sh`, `tools/restore_probe.sh`) dropped
  discovery entirely for `GODOT="${GODOT:-godot}"`.
- A thirteenth, structurally unrelated implementation lives in
  `tools/bootstrap.ps1`.
- The version gate travels with only **three** of the twelve POSIX copies.
- **No test exercises discovery on either platform.** Both self-test suites
  always supply an explicit engine, as does `.github/workflows/test.yml`, which
  additionally globs `*_console.exe` — a hand-written workaround for the very
  Windows discovery gap this design closes.

Observed consequences on real machines:

- Debian with Godot installed as `godot-4` (snap): `tools/bootstrap.sh` runs a
  full 45 s release build and only then reports `godot not found`.
- Windows with the official `4.7.1-stable` editor on `PATH`: `Find-Godot`
  reports `godot not found`, because the official archive ships
  `Godot_v4.7.1-stable_win64_console.exe` and the candidate list knows only
  `godot.exe`, `godot.console.exe` and `godot_console.exe`.
- Any Godot .NET/Mono build of the pinned version is rejected, because the gate
  prefix-matches a string carrying a build-flavour infix
  (`4.7.1.stable.mono.official.…` does not start with `4.7.1.stable.official`).

## Decision

One POSIX shell library, `tools/lib/engine.sh`, sourced by every POSIX caller.
`tools/bootstrap.ps1` remains the single Windows implementation and gains the
same contract, expressed in PowerShell.

### Why not Rust

Law 2 says everything that is not designer-facing lives in Rust, and the
version predicate is genuinely pure. It still does not belong there:

- **The bootstrap is what installs Rust.** The engine gate must run before the
  build (that is the whole point of moving it earlier), so it cannot depend on
  a Rust artifact. Committing a prebuilt helper violates the source-and-asset
  policy; building a helper crate first reintroduces the exact "expensive work
  before a cheap precondition" defect being removed; a shell fallback restores
  the duplication.
- **The droplet cannot satisfy it.** `ci/pipeline.sh` runs there from a
  `git archive` tar extract with no git metadata, and per `deploy.sh` that host
  cannot compile the core at all — `PREBUILT_RUST=1` exists for that reason.
- **It is not domain logic.** Locating a host binary is I/O against the
  machine. The purity law puts that in a thin adapter, never in a pure module.
  Moving it to Rust would satisfy the letter of Law 2 while breaking the purity
  law it serves.

This is the one place in the repository where "everything else in Rust"
deliberately does not apply. Recorded here so it is not relitigated.

No PowerShell module either: there is exactly one PowerShell consumer, and
`tools/bootstrap.cmd` only delegates. What is shared is the **contract**, and
it is enforced by both self-test suites asserting the same named behaviours.

## The contract

`tools/lib/engine.sh` is sourced, never executed. It returns status and never
calls `exit`, so each caller keeps its own exit code and its own message prefix
(`bootstrap:`, `ci:`, `probe:`, `vendor:`, `export-macos:`) — those prefixes are
grepped by the suites.

1. **`unseeing_engine_pin <root>`** — print the trimmed `.godot-version`.
   Return 2 with a named reason when the file is missing, unreadable or blank.
   A missing pin is a defined refusal, not a silently disabled gate.

2. **`unseeing_engine_accepts <have> <want>`** — the pure predicate, and the
   only home for flavour normalisation. `<have>` is accepted when, after
   removing a `mono` or `double` build-flavour field, it begins with `<want>`
   followed by end-of-string or a `.` separator. Empty `<have>` is refused.

3. **`unseeing_engine_select <root> [explicit]`** — the whole law. An explicit
   engine (argument, else `GODOT`) is used **and gated**: if it fails the pin,
   selection fails; it is never silently replaced by a search hit. Otherwise
   walk the candidate list and return **the first candidate that satisfies the
   pin**, never merely the first that exists. On exhaustion, fail with a
   message naming the pin and the `GODOT=` escape.

**Discovery is version-aware.** That is the load-bearing decision. Widening the
candidate list is only safe once selection is gated, because a machine may hold
several engines: on the audited Debian host, `godot-4` (4.7 mono) and
`~/bin/godot` (4.7.1) coexist, and a first-that-exists list picks the wrong one.
Version-aware selection also closes the "probes accept any engine" class for all
eight probes at once, without writing the gate eight more times.

### Candidate list

Ordered most-likely-first; non-existent candidates cost nothing, and the walk
short-circuits on the first accepted engine. `UNSEEING_ENGINE_CANDIDATES` (a
newline-separated list) replaces it entirely, which is how the tests point
discovery at a fixture directory instead of the host. That injection is what
makes discovery testable at all.

```
godot  godot4  godot-4  godot-editor  Godot
$HOME/bin/godot
/opt/homebrew/bin/godot  /usr/local/bin/godot  /usr/bin/godot
/Applications/Godot.app/Contents/MacOS/Godot
$HOME/Applications/Godot.app/Contents/MacOS/Godot
<repo>/godot-bin/godot
Godot_v*-stable_linux.<uname -m> and Godot_v*-stable_macos.universal in each PATH entry
```

### Windows candidates

`Find-Godot` keeps Scoop, WinGet, `%LOCALAPPDATA%` and `godot-bin`, and gains
the official release filenames — `Godot_v*_console.exe` and `Godot_v*.exe` —
searched in `godot-bin\`, the repository root and every `PATH` entry.
`Prefer-ConsoleGodot` already maps a GUI executable to its console sibling
correctly for official naming; only the candidate walk was blind.

## Deliberate behaviour changes

- **An explicit `GODOT` that fails the pin now fails.** Previously it was
  trusted without a gate in nine of twelve callers. Announced, not smuggled.
- **A Mono/.NET editor of the pinned version is now accepted.** It was rejected
  for its build flavour, which the pin does not constrain.
- **A missing `.godot-version` is now a refusal.** It previously disabled the
  gate and then killed `bootstrap.sh` on an unbound `$WANT` after the run had
  otherwise succeeded.
- **Every probe now applies the pin.** A probe run by hand against an engine
  `ci/pipeline.sh` would reject no longer emits an authoritative verdict.

## Amends the 2026-08-13 bootstrap spec

Parity contract item 5 read: *reject any Godot version whose output does not
begin with the complete `.godot-version` value*. That wording is what makes a
Mono build of the pinned version fail. It is replaced by:

> 5. reject any Godot whose reported version, after build-flavour
>    normalisation, does not begin with the complete `.godot-version` value at
>    a field boundary;

and two items are added:

> 9. locate and version-gate the engine before any Rust work;
> 10. never select an engine that fails the pin, including an explicitly
>     supplied one.

## Out of scope

The `19 checks` literal, the Rust pin duplicated in the suites, the Superpowers
pin constants, and every non-engine point defect from the audit. Same shape
(an unowned constant), different root cause, separate commits.
