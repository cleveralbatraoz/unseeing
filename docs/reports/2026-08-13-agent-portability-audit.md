# Agent portability audit — 2026-08-13

Scope: repository instructions and the pinned workflow used by Claude Code and
Codex App/CLI. Codex IDE and other agents are excluded because the IDE does not
currently expose Codex plugins. The audit reviewed repository policy, ignore
rules, CI, deploy/archive behaviour, and Superpowers host manifests.

## Addressed findings

- Project policy was owned by a 33 KiB Claude-specific file, above the desired
  instruction headroom and unavailable as a canonical contract to Codex. It is
  now a sub-24-KiB root `AGENTS.md`; `CLAUDE.md` is only the supported import
  adapter. Normative game, platform, workflow, TDD, commit, architecture,
  dependency, review, and documentation rules remain in the shared contract;
  historical explanation and setup detail moved to normal documentation. The
  Rust contract explicitly makes decoupling, total functions, pure domain
  logic, and absence of global state mandatory, defines each property, and
  confines engine effects to thin boundary adapters.
- Workflow precedence formerly delegated project authority back to an
  unpinned plugin. The contract now states: project policy owns project rules,
  pinned Superpowers owns generic procedure, stricter compatible rules apply,
  and genuine conflicts require user direction.
- Worktree and memory language assumed one host. Shared durable knowledge is
  tracked; native isolation is preferred, `.worktrees/<branch>` is the neutral
  fallback, and cleanup belongs to the tool that created the worktree.
  `.worktrees/`, `.claude/`, and `.superpowers/` are ignored.
- Superpowers was an unpinned user-global dependency. The sole repository
  submodule now uses the canonical HTTPS URL and pins v6.3.0 at peeled commit
  `b36e0829c6d0140e93cfef2ca599b1b07d4a7797`, without a branch or update
  override. Setup refuses competing installations unless migration is
  explicit and verifies enabled caches byte-for-byte against the pin.
- Supply-chain upgrades lacked a review boundary. The updater accepts only a
  release-shaped tag, fetches that tag, verifies all host/package versions,
  rejects unchanged-version code changes, and leaves a detached candidate for
  inspection without staging, committing, or changing user caches.
- The repository had no invariant for agent instructions or gitlinks. Hygiene,
  pre-commit, CI metadata/full verification, and failure fixtures now enforce
  the adapter, size ceiling, exact sole gitlink, canonical URL, clean pin,
  release/manifests, required skill set, and host integration files.
- Developer tooling could have entered the deployment archive. Both the
  gitmodules file and payload are `export-ignore`, and archive verification
  explicitly proves they are absent. Game and deployment code have no runtime
  dependency on the submodule.
- Active project/toolchain comments now point to `AGENTS.md`; the README and
  `docs/agent-workflow.md` describe cloning, activation, conflict diagnosis,
  migration, upgrades, restart requirements, and the deployment boundary.

## Deferred build and deployment recommendations

These broader observations are intentionally not implemented by this change:

- Replace the production bare-repository archive hook with a versioned,
  authenticated artifact promotion flow so the exact CI-tested export—not a
  freshly reconstructed server tree—is deployed.
- Remove the two documented fixed sleeps from browser smoke testing in favour
  of observable readiness conditions and retain bounded timeouts.
- Publish signed checksums or provenance attestations for desktop and web
  artifacts, including the Godot, Rust, Emscripten, and gdUnit pins.
- Add periodic clean-room builds for macOS universal and both Windows
  architectures. They remain on-demand today, so ordinary CI cannot detect
  platform-specific packaging drift.
- Evaluate a deployment identity that does not rely on a mutable workstation
  SSH remote, and separate production authorization from source integration.

No merge, push, deployment, user-global plugin mutation, or change to another
branch/worktree is part of this implementation.
