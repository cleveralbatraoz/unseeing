# Deployment Stability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the gated deployment repeatable across complete checkouts and git-exported production trees, then deploy the current game successfully.

**Architecture:** Three narrow POSIX boundaries own host preflight, optional developer-tool testing, and production push selection. The versioned post-receive hook consumes ordinary main pushes and transient retry refs through one unchanged build pipeline.

**Tech Stack:** POSIX shell, Git archives/hooks, SSH/SCP, cargo-zigbuild/Zig, pinned Rust/Emscripten/Godot, gdUnit4, Chromium smoke probe.

**Spec:** `docs/superpowers/specs/2026-08-15-deployment-stability-design.md`

## Global Constraints

- Render black and white with thin outlines only; no textures, fills, materials, or visual noise.
- Same-facing coplanar overlaps merge under `COPLANAR_EPS` and `PATCH_EPS` in `rust/src/render/superface.rs`; separate touching solids and source semantic roles remain at least `MIN_SEP = 0.08` apart under `rust/src/render/labels.rs`.
- New labels stay in `[0.15, 0.96]`; the radio preview's existing `Role::Case = 0.05` exception is not extended.
- `game/` remains the sole Godot 4.7 project for Web, universal macOS, Windows x86_64/arm64, Rust desktop targets, and wasm32.
- Designer-facing objects remain Godot objects; all laws remain pure engine-free Rust with thin registered boundary adapters.
- Functions remain total, dependencies explicit, mutable global state forbidden, and `#![deny(unsafe_code)]` unchanged except the existing gdext entry point.
- No build output, export, `.pck`, `.wasm`, `target/`, frame, or report is committed.
- Commits are small, self-contained, green, authored as `Dmitrii Galchenko <dggrus@gmail.com>`, use narrative subjects/bodies, and contain no assistant attribution.
- Developer-agent tooling remains excluded from deployment archives and from game/build/deploy runtime dependencies.
- Every behavior change follows red-green-refactor and realistic branch/side-effect mutation checks.

---

### Task 1: Exact host preflight

**Files:**
- Create: `ci/deploy_host_preflight.sh`
- Create: `test/deploy_host_preflight_test.sh`
- Modify: `deploy.sh`

**Interfaces:**
- Consumes: repository root argument; executable boundaries from `PATH`; the configured `production` remote and `vpn` SSH alias.
- Produces: exit 0 plus `deploy: preflight OK`, or exit 2 plus a complete missing-capability list.

- [ ] **Step 1: Write the failing behavioral test**

  Build a temporary fake `PATH` whose `cargo-zigbuild` accepts only
  top-level `--version`, whose `zig` accepts `version`, and whose remaining
  commands record calls. Assert the shipped preflight does not yet exist.

- [ ] **Step 2: Run the test and verify RED**

  Run `test/deploy_host_preflight_test.sh`. Expect nonzero with the missing
  `ci/deploy_host_preflight.sh` named.

- [ ] **Step 3: Implement the minimal component and wire it**

  Move only host-capability checks out of `deploy.sh`. Require
  `cargo-zigbuild --version` and `zig version`; retain `git remote get-url`,
  batch SSH, wasm recipe, SCP, and curl checks. Leave clean-main provenance in
  `deploy.sh` and invoke the component before local checks.

- [ ] **Step 4: Verify GREEN and mutations**

  Run `test/deploy_host_preflight_test.sh`. Then make the fake
  cargo-zigbuild reject top-level `--version`, remove fake Zig, fail its
  `version` call, remove the production remote answer, and fail SSH one at a
  time; every mutation must make the named assertion fail.

- [ ] **Step 5: Commit the green behavior**

  Commit the test, component, and deploy wiring together with a narrative
  subject and a body explaining why Cargo's subcommand protocol was the wrong
  version boundary.

### Task 2: Archive-safe developer tooling gate

**Files:**
- Create: `ci/run_agent_tooling_self_test.sh`
- Create: `test/ci_agent_tooling_gate_test.sh`
- Create: `test/deployment_archive_test.sh`
- Modify: `ci/pipeline.sh`
- Modify: `test/repo_hygiene.sh`

**Interfaces:**
- Consumes: repository root argument and `.gitmodules` as the complete-checkout marker.
- Produces: executed setup-agent test in a checkout, explicit skip in a valid archive, or a named refusal for inconsistent contents.

- [ ] **Step 1: Write failing fixture and real-archive tests**

  Cover a full checkout, a valid export, a broken checkout missing its setup
  subject, and an invalid export leaking that subject. Compose `git archive`
  with the gate and assert the executable gate/test are present while
  `.gitmodules` and `tools/setup-agents.sh` are absent.

- [ ] **Step 2: Run both tests and verify RED**

  Run `test/ci_agent_tooling_gate_test.sh` and
  `test/deployment_archive_test.sh`. Expect failures because the gate does not
  exist and the pipeline still invokes the omitted subject directly.

- [ ] **Step 3: Implement the minimal gate and pipeline wiring**

  In a checkout, require and execute both subject and test. In an archive,
  refuse a leaked subject and otherwise print an explicit skip. Replace the
  direct pipeline test call with the new gate. Extend hygiene to pin the
  exported runtime gate and test.

- [ ] **Step 4: Verify GREEN and mutations**

  Run both focused tests and `test/repo_hygiene.sh`. Mutate each context
  branch, remove each required archive entry, and leak the setup subject; each
  mutation must turn a focused test red.

- [ ] **Step 5: Commit the green behavior**

  Commit one self-contained archive-boundary behavior with a narrative subject
  and a body explaining why developer tooling must stay absent in production.

### Task 3: Retryable production trigger

**Files:**
- Create: `ci/push_production.sh`
- Create: `test/push_production_test.sh`
- Create: `test/post_receive_test.sh`
- Modify: `deploy.sh`
- Modify: `infra/post-receive`
- Modify: `infra/README.md`

**Interfaces:**
- Consumes: repository root and exact full commit SHA.
- Produces: an ordinary `production main` push when remote main differs, or a unique `main:refs/heads/deploy-retry/...` push when it matches.

- [ ] **Step 1: Write failing push-selection and hook tests**

  Fake `git ls-remote`/`git push` to pin equal and unequal branches. Build a
  temporary bare repository with the real hook and a recording pipeline; prove
  an identical retry commit is built twice, its transient ref is removed, and
  unrelated refs do not run the pipeline.

- [ ] **Step 2: Run both tests and verify RED**

  Run `test/push_production_test.sh` and `test/post_receive_test.sh`. Expect
  failures because retry selection and retry-ref hook handling do not exist.

- [ ] **Step 3: Implement selection and hook cleanup**

  Add the narrow push component, call it from `deploy.sh`, accept the retry
  namespace in the hook, and delete retry refs after either pipeline outcome.
  Preserve the identical archive, stamp, environment scrub, and pipeline path.

- [ ] **Step 4: Verify GREEN and mutations**

  Run both focused tests. Reverse the equality branch, stop accepting retry
  refs, stop deleting them, and allow an unrelated ref one at a time; each
  mutation must fail an exact assertion.

- [ ] **Step 5: Commit the green behavior**

  Commit the retry behavior with its tests and infra documentation, explaining
  why post-receive cannot reject an already advanced ref.

### Task 4: Documentation and complete verification

**Files:**
- Modify: `README.md`
- Modify in wiki checkout: `Engineering-Build-Test-Deploy.md`

**Interfaces:**
- Consumes: the final executable behavior from Tasks 1-3.
- Produces: current operator documentation and deployment evidence.

- [ ] **Step 1: Update tracked and live documentation**

  Describe the exact preflight commands, checkout/archive gate distinction,
  retry-ref behavior, and the rule that live-stamp verification precedes
  origin synchronization. Name each owning file.

- [ ] **Step 2: Run focused and full verification**

  Run every new test, `test/repo_hygiene.sh`, `ci/verify-superpowers.sh
  metadata`, and `SKIP_EXPORT=1 ci/pipeline.sh`; require exact zero-failure
  summaries. Run shell syntax checks over all changed shell files.

- [ ] **Step 3: Commit project documentation and code**

  Commit any final tracked documentation with a narrative subject/body. Commit
  and publish the wiki separately from its clean isolated checkout.

- [ ] **Step 4: Integrate and install versioned infrastructure**

  Merge the verified branch into clean `main`, copy the reviewed
  `infra/post-receive` to the documented server hook path, verify its bytes and
  executable mode remotely, and push `main` to origin as authorized.

- [ ] **Step 5: Deploy and prove the definition of done**

  Run unmodified `deploy.sh` from clean `main`. Require local checks, exact
  native/wasm core builds, stamped server pipeline, export, browser smoke,
  served-byte verification, and exit 0. Independently curl the live page and
  require `UNSEEING_BUILD` to equal final `git rev-parse --short=9 HEAD`.
