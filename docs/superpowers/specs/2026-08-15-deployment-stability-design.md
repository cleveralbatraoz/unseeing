# Deployment Stability Design

**Date:** 2026-08-15

## Problem

Deployment of `d23fc174` exposed two independent false gates. The local host
preflight called `cargo zigbuild --version`, but cargo-zigbuild exposes its
version flag only as `cargo-zigbuild --version`. After a temporary host adapter
allowed the real build to proceed, the droplet's `git archive` checkout ran
`test/setup_agents_test.sh` even though its subject,
`tools/setup-agents.sh`, is deliberately `export-ignore`. The post-receive
failure still advanced `production/main`, leaving the live build unchanged and
making an identical retry a no-op.

These are deployment-boundary defects. Game behavior, rendering, physics,
sound propagation, and editor authoring are unchanged.

## Goals and definition of done

1. A deploy host is rejected before expensive work unless Cargo,
   cargo-zigbuild, Zig, the pinned wasm recipe, SSH/SCP, curl, the production
   remote, and the `vpn` SSH endpoint are actually usable.
2. The cargo-zigbuild probe uses the tool's real top-level CLI and separately
   executes Zig's version command; it never relies on a malformed Cargo
   subcommand invocation.
3. Developer-agent tests run in a complete checkout and skip, with an explicit
   reason, only in the deployment archive where `.gitmodules` and
   `tools/setup-agents.sh` are intentionally absent.
4. The exact `git archive` composition is regression-tested: the deployment
   gate and its test remain present, while developer-only tooling remains
   absent.
5. If `production/main` already names the requested commit, `deploy.sh` can
   trigger the versioned server hook through a unique retry ref. The hook
   builds that exact commit and deletes the transient ref after the attempt.
6. All existing exact-commit core stamps, full local checks, server-side full
   pipeline, browser smoke test, served-byte verification, live build-stamp
   verification, and origin/tag synchronization remain mandatory.
7. The current game is live only when the served `UNSEEING_BUILD` equals the
   final clean `main` commit and the deployment script exits zero.

## Design

### Host preflight

`ci/deploy_host_preflight.sh` owns host capability detection. It accepts the
repository root as its only argument, reads no mutable project state, and
returns either one `deploy: preflight OK` record or one complete refusal that
names every missing dependency. Executable discovery remains a boundary effect
through `PATH`; repository and endpoint checks remain explicit `git` and `ssh`
calls. `deploy.sh` retains branch/clean-tree provenance, then calls this single
component before any test, build, upload, or push.

The preflight requires both `cargo-zigbuild --version` and `zig version` to
succeed. `cargo zigbuild` is not a version API: Cargo inserts the subcommand
name as an argument, so the former probe asked the build subcommand to parse a
flag it does not own.

### Checkout versus deployment archive

`ci/run_agent_tooling_self_test.sh` owns the optional developer-tool gate.
`.gitmodules` is the explicit context marker because policy intentionally
exports it and all developer-agent entry points out together.

- With `.gitmodules` present, both `tools/setup-agents.sh` and its behavioral
  test must exist and the test must run.
- With `.gitmodules` absent, the setup tool must also be absent and the gate
  skips explicitly. A leaked setup tool is a refusal, not a reason to execute
  developer tooling in production.

`test/deployment_archive_test.sh` composes the real `git archive` mechanism
with that gate. It proves the archive contains the gate and regression test,
contains neither `.gitmodules` nor `tools/setup-agents.sh`, and produces the
explicit archive skip. This keeps developer tooling available to contributors
without making it a game/build/deploy dependency.

### Retry trigger

`ci/push_production.sh` owns production-ref selection. When remote
`production/main` differs from local `HEAD`, it performs the ordinary main
push. When they are identical, it pushes the same commit to a unique
`refs/heads/deploy-retry/...` ref instead of reporting `Everything up-to-date`.

The versioned `infra/post-receive` accepts `main` and the retry namespace,
builds the received commit through the same archive, prebuilt-core stamp, and
pipeline path, and deletes retry refs after success or failure. Other refs stay
ignored. A retry never changes what is built and never weakens a gate; it only
makes the existing gate callable again.

## Failure behavior

- Host failures exit 2 before expensive or remote work.
- Local or server pipeline failures exit 1 and deploy nothing.
- A failed post-receive may still advance `production/main`, but the next
  unchanged `deploy.sh` run uses a retry ref and reaches the hook again.
- A live build-stamp mismatch remains a hard failure. Origin and tags are
  pushed only after the site proves it serves the requested commit.
- Retry refs are transient server implementation details and are never pushed
  to origin.

## Tests and mutations

- A fake cargo-zigbuild that accepts only top-level `--version` catches any
  return to `cargo zigbuild --version`.
- Removing the Zig executable or making `zig version` fail turns the preflight
  test red.
- Removing `.gitmodules` from a checkout fixture selects archive skip; adding
  it back without the setup tool turns the gate red.
- The exact archive test fails if the developer tool leaks, the runtime gate
  disappears, or the archive tries to run the omitted subject.
- A bare-repository hook fixture proves main pushes build, retry pushes rebuild
  the identical commit, retry refs are removed, and unrelated refs are ignored.
- Push-selection tests mutate both the equal and unequal remote-main branches.
- The full pipeline runs before merge and again through the real deployment
  path on clean `main`.

## Project invariants

This change introduces no gameplay object and touches no Rust or Godot law.
The black-and-white outline-only perception model, same-facing coplanar
superface merge law in `rust/src/render/superface.rs`, `MIN_SEP = 0.08` and
sRGB-safe label band in `rust/src/render/labels.rs`, source/creature separation,
Godot-object/Rust-law split, total pure domain functions, no global state, and
x86_64/arm64/wasm32 support all remain unchanged. No new shipped dependency is
introduced. Developer tooling remains excluded from deployment archives.
