# hp-local Development Setup — Design

**Status:** operational scope approved by the user's 2026-08-24 request to set
up `hp-local`, record every resulting change, provide a complete onboarding
guide, and build the game, then amended by the user's 2026-08-25 request to
validate the actual Godot editor UI and repository-pinned Godot MCP loop. The
dependency and build design is inherited from the already-approved
cross-platform bootstrap specifications below; no new game or product design
is introduced here.

## Goal

Turn the existing Debian workstation reached through `ssh hp-local` into a
reproducible Unseeing development host, prove both the native and Web build
paths plus the actual graphical editor and structured live-debugging path on
the repository's current `main`, and leave a tracked guide that can recreate
or reverse every machine change without relying on shell history.

This is developer tooling and host provisioning only. It changes no game law,
Godot scene, shader, Rust domain module, export preset, or deployment path.

## Existing contracts

This setup executes, rather than replaces, the approved contracts in:

- `docs/superpowers/specs/2026-08-13-cross-platform-bootstrap-design.md`;
- `docs/superpowers/specs/2026-08-14-engine-selection-design.md`;
- `.godot-version`;
- `rust/rust-toolchain.toml`;
- `rust/build-wasm.sh`;
- `tools/bootstrap.sh`;
- `ci/pipeline.sh`;
- `.mcp.json` and `tools/setup-mcp.sh`;
- `docs/superpowers/mcp/godot-mcp-loop.md`.

Code and pins win over prose. The live wiki is required reading, but its stale
counts or tool descriptions are not installation authority.

## Audited starting point

The read-only audit on 2026-08-24 found:

- SSH alias `hp-local`, hostname `antisleep`;
- Debian 13 (`trixie`), kernel `6.12.101+deb13-amd64`, x86_64;
- AMD Ryzen 7 5800U, 7.1 GiB RAM, 7.3 GiB swap, 421 GiB free;
- a graphical GNOME 48 session and AMD Cezanne `/dev/dri` devices;
- Git, GitHub CLI, `build-essential`, GCC, Clang, Python 3, pipx, Node 20,
  ShellCheck, archive tools, and content hashers already installed;
- no Unseeing checkout, Godot, export templates, rustup, pinned Rust
  toolchains, gdtoolkit, Chromium, Brotli, or emsdk;
- GitHub CLI 2.46.0 installed, but neither `gh` nor GitHub SSH is authenticated;
- passwordless non-interactive `sudo` available to the user `galchenko`.

The 2026-08-25 MCP preflight further found an active uid-1000 Wayland user
manager with `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`,
`XDG_RUNTIME_DIR=/run/user/1000` and its user D-Bus address; official Godot
`4.7.1.stable.official.a13da4feb`; Node `20.19.2`; npm/npx `9.2.0`; and a clean
durable `main`. The target-side Godot MCP addon/state was absent, only vendored
gdUnit4 existed under `game/addons/`, and proof-only TCP port `16550` was unused
on target and controller. Seven unrelated controller-side 4.1.0 clients were
reconnecting to the default `127.0.0.1:6550`; they are outside this task and
must remain untouched. Their pre-existing default-on
`~/.godot-mcp/usage.log` baseline was 594 lines / 92,992 bytes with SHA-256
`4a470e3854b12fdb0db7915ffc6940c1b6332d77f14f570cfaadeb15a1ff7929`
and exact descriptor `mtime_ns` `1787340988551255243`. The rounded
`1787340988000000000` Phase-A prose transcription is rejected. The isolated
proof disables usage logging and requires that shared file's path, device,
inode, UID, mode, link count, size, line count, SHA-256, and `mtime_ns` to
remain unchanged.

The failed GitHub SSH authentication probe accepted GitHub's ED25519 host key
into `/home/galchenko/.ssh/known_hosts`; that persistent result is part of the
ledger even though authentication itself remained unavailable.

## Decisions

### Checkout and Git scope

For the dated `hp-local` run, clone the public repository to
`/home/galchenko/src/unseeing` over HTTPS only after `refs/heads/main` is
confirmed at the reviewed full SHA
`d6285b0bba84dd29846a9613c2e8081191e46cfd`. Verify the top-level origin,
`.gitmodules`, sole gitlink and `ci/superpowers.lock` before initializing the
submodule. Build that exact commit and record its full SHA. Configure the
mandated identity and `.githooks` repository-locally, never globally:

```text
Dmitrii Galchenko <dggrus@gmail.com>
```

Do not manufacture credentials. Read-only clone/fetch works; the guide must
state that `gh auth login` or an SSH key remains a human-owned onboarding step
before the host can push.

The reusable fresh-host procedure is not a historical-SHA assertion against a
moving ref. It resolves current public `main`, clones it, proves the clone
matches that one observation, records the full SHA, and consumes Godot, Rust,
Web and tooling pins from checked-in files in that clone. Exact historical
reproduction is a separately labelled dated-ledger operation.

### Installation scope

Use the narrowest practical owner for each dependency:

| Dependency | Scope and location | Authority |
| --- | --- | --- |
| Chromium, Brotli | Debian packages | `test/web_smoke.sh`, `ci/pipeline.sh` |
| Godot editor | user executable under `~/.local/bin` | `.godot-version` |
| Godot export templates | user data under `~/.local/share/godot/export_templates` | the same Godot release |
| rustup and Rust toolchains | user state under `~/.cargo` and `~/.rustup` | `rust/rust-toolchain.toml`, `rust/build-wasm.sh` |
| gdtoolkit | isolated pipx environment | README and `ci/pipeline.sh` |
| emsdk | versioned Git checkout at `~/emsdk` | `rust/build-wasm.sh` |
| Godot MCP | ignored addon inside one isolated task worktree; pinned local stdio client | `.mcp.json`, `tools/setup-mcp.sh` |

Use the official Godot 4.7.1 release assets and verify both downloads against
that release's `SHA512-SUMS.txt` before installing them. The selected official
hashes are:

- editor ZIP:
  `4ccdab7a48eeccbe8819a2fc1f6262f8d72065d98601bcb3743fcbd7ebd39f373758a788ee3293a05ec5b2c48538266c437404312e372225cd2df273945a2de9`;
- export-template TPZ:
  `afcc83d8d3d298038f19c58744a0d660fa75dd4baa33cb55d1011bb2565a2a8c2381728924564cb909e37c205a23f21b521b23bd057993afd43ae4da0b2f9d47`.

Check out the official emsdk tag `4.0.20` at peeled commit
`e4fe26ef59168ff44f4c23c466e497bf60b3411e` before installing and activating
the SDK.

Do not install PowerShell, a second Godot package, or an agent CLI. Godot MCP
is a separately proven developer-only editor aid: install exact addon/server
version `4.1.0` only in the isolated Task-4A worktree. It remains ignored,
untracked, export-excluded, and absent from every build/test/runtime dependency.

### Rust toolchains

Download the current official `rustup-init` executable and its published
SHA-256 sidecar over TLS, verify the sidecar, and record the resolved installer
hash. Install rustup without an unpinned default toolchain. Install:

- stable `1.97.1`, `rustfmt`, `clippy`, and every desktop target declared in
  `rust/rust-toolchain.toml`;
- `nightly-2026-05-25`, `rust-src`, and
  `wasm32-unknown-emscripten` for Web only.

The Debian `rustc`/`cargo` packages remain installed and untouched. Commands
inside the repository use rustup's proxy after sourcing `~/.cargo/env` and the
repository pin selects the compiler.

### Proof boundary

Run four independent gates on the cloned SHA:

1. `tools/bootstrap.sh` — release editor GDExtension plus exact 19-class
   census;
2. `SKIP_EXPORT=1 ci/pipeline.sh` — complete checks-only native gate;
3. `ci/pipeline.sh` — wasm build, Web export, precompression, and Chromium
   smoke test in addition to the native gate;
4. `tools/export_linux.sh "Linux x86_64" build/linux/unseeing` — compiled
   Linux player artifact.

Use `CARGO_BUILD_JOBS=4` only as a command-local resource limit on this
7.1-GiB machine. It is not a project setting and must not be written into the
repository or shell startup files.

Record exit status, meaningful terminal verdict, duration, artifact paths,
sizes, and SHA-256 hashes for every raw and compressed export file. A command
that fails is evidence, not permission to weaken a pin or skip a gate; diagnose
one hypothesis at a time with the pinned systematic-debugging workflow.

### Actual editor and Godot MCP proof boundary

Never enable the addon or launch its editor session from the durable primary
clone. Create the exact task worktree
`/home/galchenko/src/unseeing/.worktrees/hp-local-mcp-setup` from reviewed
`main` after proving the parent is ignored, and ledger the resulting shared
Git common-directory worktree/branch/admin state. Ignored output is local to a
worktree: run that worktree's own checked-in `tools/bootstrap.sh` with
`CARGO_BUILD_JOBS=4`, `CARGO_NET_OFFLINE=true` and `RUSTUP_AUTO_INSTALL=0`,
and prove its release GDExtension plus 19-class census before editor launch.
Capture complete before/after Cargo and Rustup manifests and complete manifests
for every ignored worktree output, including `rust/target`, `game/.godot`, the
addon and override boundary. Every future worktree likewise needs its own
verified bootstrap and project-relative addon install.

Keep this proof outside the pending Task-2/5 evidence ledger. Its new
owner-only root is
`/home/galchenko/.local/state/unseeing/mcp-setup/2026-08-25`. Capture complete
before/after/delta manifests for the worktree MCP boundary, `~/.npm`, and the
mutable Godot cache/config/user-data roots. Capture only the four whitelisted
GUI session values, never an environment dump. On the controller, bind the
pre-existing `~/.godot-mcp/usage.log` by owner/mode/inode/size/hash/line count
and require that full boundary unchanged; neither its contents nor a line delta
enters target evidence or tracked documentation.

Both `.mcp.json` and `tools/setup-mcp.sh` pin
`@satelliteoflove/godot-mcp@4.1.0`. Invoke the checked-in installer with any
inherited `GODOT_MCP_VERSION` removed. Bind the installed `plugin.cfg` and
complete addon manifest to registry integrity
`sha512-uq3Gh5n7fos8vIoXpr32/K7r9tL9eYLbERr+Tolksg3Y+FC5coYEkRkbJ1JktMMhoH/BnGWsWhE5E+XJ/nMEPg==`,
record the npm-cache delta, and prove the addon remains ignored while both
tracked checkouts remain clean.

Enabling addon 4.1.0 changes tracked `game/project.godot`: besides its enabled
plugin row, `_enter_tree` installs the `MCPGameBridge` autoload and four
`godot_mcp/*` settings and saves the project. Disabling leaves the autoload and
settings behind. A fixed, test-driven and independently reviewed session
supervisor must therefore capture the initially HEAD-identical file's exact
bytes/hash/owner/mode/device/inode, add only the required plugin row and the
proof's `godot_mcp/port_override_enabled=true` and
`godot_mcp/port_override=16550` settings for this otherwise unclickable remote
first boot, and own cleanup through an always-run bounded `finally` path. It
also creates only this exact ignored `game/override.cfg`:

```text
[display]

window/size/mode=0
window/size/viewport_width=1280
window/size/viewport_height=720
```

The dated automated cleanup closes the controller first, then stops only the
supervisor's editor child while the plugin remains enabled. It accepts only the
complete expected plugin-row, `MCPGameBridge` and four-setting diff, removes
only that exact override, and restores the captured preimage directly. It does
not disable the plugin and has no post-disable phase. Unrelated bytes are
preserved as a blocking recovery artifact, never overwritten. Neither
`assume-unchanged` nor `skip-worktree` may conceal the temporary diff.

Start the supervisor as uid 1000 through one named transient user-systemd unit
in the existing GNOME Wayland session. Poll its child process, exact loopback
`127.0.0.1:16550` WebSocket listener and unit state without an arbitrary sleep.
Prove official Godot `4.7.1.stable.official.a13da4feb`, editor mode, the exact
task-worktree project path and that the listener owner is the supervisor's
exact Godot child; record only that unit's journal.

The remote proof never contends with the controller's built-in MCP processes
on default port 6550. Before starting SSH, capture base `ssh -G hp-local` and
refuse inherited local/remote/dynamic forwards, control master/path/persistence,
fork-after-authentication, proxy command/jump or local command. Capture the full
proposed command through `ssh -G` too and require exactly the sole
`127.0.0.1:16550:127.0.0.1:16550` local forward,
`ForkAfterAuthentication=no`, and no forbidden setting.
Both base and full effective configurations reject `ClearAllForwardings=yes`
because it would also discard this required command-line `-L`. The fixed,
signal-cleaned, maximum-lease wrapper
uses literal `-N -T`, `BatchMode=yes`, `ExitOnForwardFailure=yes`,
`ForkAfterAuthentication=no`, `ControlMaster=no`, `ControlPath=none`, `ControlPersist=no`,
`PermitLocalCommand=no`, `UpdateHostKeys=no`, `StrictHostKeyChecking=yes`,
`ServerAliveInterval=15`, `ServerAliveCountMax=3`, and only that forward.
Require its exact SSH PID to own the controller listener and never bind a
non-loopback address.

The supervisor, forward lease and controller share one absolute monotonic
1200-second deadline, a 1170-second mutation-capable work cutoff, and a final
fixed 30-second cleanup reserve. No mutation-capable work may start after the
cutoff. The clock starts before the first editor/supervisor, tunnel or
controller startup and includes every proof call, `finally` path, child stop,
override removal, project restoration, tunnel close and listener-absence
check. No component may mint, restart or extend either endpoint; unfinished
cleanup at the deadline is a failed proof.

Before any editor/game mutation, an owned bounded Node controller in an
ignored controller directory validates exact existing NPX root
`/Users/dmgalchenko/.npm/_npx/e9af8ac9cd94a1c8`, its lock, package files and
registry integrities for `@satelliteoflove/godot-mcp@4.1.0` and resolved
`@modelcontextprotocol/sdk@1.30.0` without executing npm or contacting a
registry. Ordinary addon installation still requires Node 20 or newer; this
dated controller requires exact Node `22.23.2` and its direct executable
identity. Descriptor-held reviewed bytes are copied into a private execution
capsule. The parent imports from a held `Buffer` through `registerHooks()`
and the child preload applies the same held-byte rule, so neither can reopen a
reviewed module pathname. Sealed parent and child resolution ledgers bind every
request, target and format before `process.execPath` spawns the staged
`dist/cli.js`. SDK transport adds only present names from its safe
inherited allow-list—`HOME`, `LOGNAME`, `PATH`, `SHELL`, `TERM`, `USER`—plus
fixed `GODOT_HOST=127.0.0.1`, `GODOT_PORT=16550`, and
`GODOT_MCP_USAGE_LOG=0`; reject every other inherited child-environment name.
Record the permitted names/values, parent/child PIDs, argv, executable
identities, package lock and integrities, then close only that owned child in
`finally`. The package source and fixed overrides prove the endpoint;
`addon_status` is not an endpoint diagnostic.

List the owned connection's available tools before calling them. Prove
server/addon compatibility, exact project/version/main-scene identity and
editor state. Open `res://scenes/level_02.tscn`, inspect its editor tree,
select and confirm one returned child, and take the one user-requested
640-pixel 3D editor capture. Do not save or edit a scene, node or resource.
Run configured main frozen under the supervisor-owned override, inspect the
generated input map, step exactly two initialization frames and then a bounded
30-frame window with one `move_forward` input, and compare structured
`WaveObserver.snapshot(main.now)` results. The snapshot's own hero/eye position
must move by more than `0.25 m`; source/time evolution cannot satisfy the
control. The movement request is `frames:30`; Godot MCP 4.1.0's accepted reply
is `frames:31`, comprising the 30 requested frames plus one input-settle frame,
and the proof requires that exact distinction. Godot MCP 4.1.0 returns the
clean mesh verdict as plain text, not
`structuredContent`; require the complete result anchored to exactly:

```text
Checked 144 meshes (144 surfaces) — no integrity problems. This rules out winding, dropped triangles, degenerate UVs/tangents, and NaN data. If rendering still looks wrong, the cause is lighting or materials, not mesh data — note that SDFGI replaces constant ambient light, so shadow-side fill must come from a shadowless fill DirectionalLight rather than ambient_light_energy.
```

Require no editor or game error. Before SDK import, a fresh supervisor contract
must bind the previously absent named unit, journal cursor, unit/Godot start
identities, project/evidence roots and pending gate. Controller success is only
`controller_lane_status:"passed"` with
`integrated_proof_status:"pending_game_log_gate"`. After game stop, the
supervisor-owned game-process journal finalizer inspects only that unit and
cursor interval, sanitizes/hashes it, requires zero game-process errors, and
binds its terminal result back to the contract. Only then is integrated proof
successful.

Teardown ordering is exact: stop the game through the owned MCP client; close
that controller and only its stdio child; stop the transient editor while its
plugin remains enabled; validate exactly the full enabled-row,
`MCPGameBridge` and four-setting diff; remove the override and restore exact
`project.godot` directly without a disable/post-disable phase; close the leased
SSH forward; then prove both worktrees tracked-clean and shared usage-log
identity/content unchanged before the same 1200-second deadline. Retain the
ignored addon in the Task-4A worktree and retain shared npm/Godot caches. Seal
only canonical summaries, manifests, commands, statuses and durations—including
every failed attempt and cleanup disposition—plus tool-result hashes and
rollback facts: exclude exactly the final manifest and digest from a
bytewise-sorted relative SHA-256 manifest, fsync the digest last, and only read
afterward. Raw environment, credentials, private user content, screenshots and
build output never enter the evidence. Any unexpected result stops
state-changing work, but every cleanup owner still runs; no retry follows until
systematic debugging and a failing regression fixture prove the correction.
Strict real-filesystem/process TDD and realistic mutations cover the session
supervisor, tunnel wrapper and owned MCP controller: paths, identities,
project/override semantics, child/signal cleanup, base/full SSH effective
configuration, tunnel argv/lease, the absolute 1200-second startup-through-
cleanup deadline, cached package/SDK integrity, safe-plus-fixed environment,
tool sequence/assertions, failure records and last-write seal.

Ordinary manual addon rollback first stops the game and client, Disables the
plugin, and closes the editor. It then requires exactly the post-disable
residue—`MCPGameBridge` plus four settings, with no enabled row—and restores the
exact clean `project.godot`, comparing restored output bytes/SHA-256/UID/GID/mode.
Device plus original and replacement inode identities are recorded as facts
without requiring equality; timestamps are outside the restoration contract.
Only after that proof may it remove the canonical
ignored `game/addons/godot_mcp` tree, deleting only entries in its complete
installed manifest after path/type/owner/mode/hash validation. Shared
`~/.npm`, Godot caches and controller usage history are retained absent a
separate complete ownership proof. Removing the addon tree alone is not an
uninstall.

Removing the retained proof worktree is a separate guarded rollback. Require
its exact canonical path, branch, reviewed HEAD, clean tracked state, restored
project, stopped editor/unit/client/tunnel and only recorded ignored outputs.
Remove each exact manifest-authorized ignored output only after canonical
path/type/owner/mode/file-hash verification—`rust/target`, `game/.godot`,
`game/addons/godot_mcp`, `game/override.cfg`, and any reviewed controller
directory—and stop on residue or mismatch. Only then may the durable checkout
use non-forced `git worktree remove`, prove both path and common-Git admin entry
absent, and delete a no-unique-commit proof branch with safe `git branch -d`.
Never use `--force` or recursive manual deletion.

### Dated editor-only disposition

The eventual Task-4A live result is narrower than the full runtime protocol
designed above. Attempt 7 passed the editor-only boundary after six guarded,
fully cleaned failures. Remediation installed and enabled Debian
`systemd-timesyncd` `257.13-1~deb13u1` after excessive clock skew, corrected
the real `0600` project-mode contract, totalized and diagnosed the launch
callback failure, and replaced the incompatible held-`/dev/fd` SSH lease plus
inode-bound supervisor close with direct-owned Godot/SSH and guarded
content/metadata restoration.

The passed scope proved the complete addon/editor handshake, opened level 02,
selected its Room node, inspected one transient editor capture, observed no
new editor errors, preserved the complete usage-log boundary, restored the
project exactly, and removed every owned process/listener/override. The proof
worktree and ignored addon/build caches are retained clean for development.
No runtime-game MCP claim is made: the game, movement snapshots, running mesh
validator, and supervisor game-journal terminal gate were not exercised.
Earlier native/Web run and export results remain separate evidence. The guide
owns the exact attempt chronology, hashes, package transaction, and retained
worktree roster.

## Durable record

Create `docs/hp-local-development-setup.md` as both:

1. a copy-paste onboarding guide for a fresh Debian 13 workstation; and
2. a factual 2026-08-24 change ledger for this run.

It must distinguish pre-existing state, installed packages, user-level files,
repository-local configuration, generated ignored artifacts, temporary files
removed, failed attempts that left state, and intentionally unperformed
optional steps. Every persistent change gets an owner/path, observed version,
verification, and rollback command. Never include tokens, credentials, private
keys, or a raw environment dump.

Build/editor/browser effects outside the checkout are part of that accounting:
Cargo registry/cache/database state, any rustup verifier residue, Godot editor
cache/settings/user data, gdtoolkit parser cache, Chromium configuration and
PKI metadata. They remain distinct from Task 2's installation manifests.
Mutable/shared state is never broadly removed; rollback either proves a
complete unchanged setup-owned boundary or retains it explicitly.

Task 4A's guide record separately covers the actual editor/MCP proof: isolated
worktree and shared Git-admin delta; worktree-local bootstrap and ignored addon;
4.1.0 registry integrity; GUI unit and leased loopback-forward lifecycle;
temporary `project.godot`/`override.cfg` state and exact restoration; structured
tool results; generated npm/Godot deltas and unchanged controller log; evidence
seal; update, troubleshooting and guarded rollback. It distinguishes local
default-port host use (no SSH forward) from an isolated-port owned-client remote
controller. It must never present addon-directory
deletion as a complete uninstall because disabling 4.1.0 leaves its autoload
and four project settings.

Keep redacted raw evidence under the mode-700 host directory
`/home/galchenko/.local/state/unseeing/setup/2026-08-24/`: before/after Debian
package and manual-package lists, APT source hashes and transaction excerpt,
startup-file hashes plus only the installer-owned changed lines, the exact
public GitHub known-host entry and fingerprint, tool version output, and sorted
recursive type/mode/size/symlink and SHA-256 manifests for the installation
roots this task owns. The tracked guide records every material item, the raw
manifest filenames, counts and hashes. Download scratch is a separately named
child removed only after its canonical path, owner and mode are revalidated;
the evidence directory survives. Tool-managed trees are reversed by their
own uninstallers or by exact recorded roots, never broad globs.

For later fresh-host runs, the operator explicitly selects one unused
`YYYY-MM-DD` evidence key; no automatic date or reuse is allowed. The guide
records concrete before/after package and manual-package deltas, APT-source and
startup hashes, resolved versions, checked-in pin hashes, complete
installed-root metadata/file manifests, status-preserving logs for all four
gates, and a complete artifact manifest. Its guarded generic scratch cleanup
does not reuse Task 5's historical helper. It writes a retained download
inventory, proves only the selected download child absent, and creates a
non-self-referential final seal that excludes exactly its manifest/digest and
writes the digest last.

The partial `stable-x86_64-unknown-linux-gnu` state attributed to Task 4's
timed-out redundant verifier was not repaired inside this setup ledger. One
separately approved reviewed-helper attempt removed it successfully on
2026-08-25. Its proof and 25-entry/27-file final seal use the distinct guarded
remediation root
`/home/galchenko/.local/state/unseeing/remediation/2026-08-25-stable-timeout`;
the final-manifest SHA-256 is
`98865ea6f93416f0916ede7ce2f8b0a0bbbb116cd57b379f5db32be72178d0f2`.
Both pinned toolchains remained exact, the original Task-2 checkpoint remained
byte-current, and the guarded transfer directory was removed. The Task-2/5
evidence root was never reopened for the remediation.

### Historical checkpoint and final-seal semantics

Task 2's recovery finalizer sealed the complete evidence tree while the
separately owned `downloads/` scratch child was still present. Its 490-entry
manifest has SHA-256
`ad0a3c2626a4c7c85e8a0f04a7f15bffa0fd5affe1d7065c1da8f4b5fd272385`.
That pair is immutable historical proof of the pre-cleanup tree. It is called
the **pre-cleanup checkpoint**, not the current or final seal, and remains in
the evidence root after cleanup.

Task 5 owns one planned transition from that checkpoint to the final current-
tree seal. Before any removal, it must verify the checkpoint digest, all 490
entries, and the exact reviewed cleanup-helper source/digest pair. It must
then reject every output collision; validate the literal evidence and download
paths, non-symlink directory types, owner `galchenko`, mode `0700`, held
directory identities and link counts; and fsync a complete cleanup and
supersession record. Only then may it execute the semantic equivalent of the
literal bounded command:

```sh
rm -rf -- /home/galchenko/.local/state/unseeing/setup/2026-08-24/downloads
```

The operation must prove that exact child absent and every non-download
checkpoint entry unchanged. It then writes a deterministic, bytewise-sorted,
relative-path manifest of every remaining regular evidence file excluding
exactly that new manifest and its digest, fsyncs the digest last, and performs
only read-only verification afterward. The pre-removal cleanup record retains
the checkpoint as historical proof and names the new pair only prospectively:
it becomes current truth after removal, absence, preservation and final-digest
regeneration all succeed. Without that verified digest it does not supersede
the checkpoint. Any failed pre-removal guard leaves `downloads/` intact; any
unexpected live result stops without retry or free-form repair.

The fixed absolute pathname process call has an irreducible interval between
the last identity check and `/usr/bin/rm`. This residual same-UID race is
accepted for this one reviewed operation because both path components are
fixed, the evidence/download directories are non-symlink mode-`0700` trees
owned by the one approved UID, held device/inode/link-count identities are
revalidated immediately before execution, and no caller can supply a target
or executable. A different owner or threat boundary would require a deletion
API operating relative to held descriptors instead.

The reviewed helper and tests live only in the ignored task SDD workspace.
Phase B copies the reviewed helper and a matching digest into the evidence root
so the final seal covers both, but neither helper artifact is tracked in Git.
After that final digest, no later task may add a `final/` child or any other
file to this evidence root. Exact-handoff rerun logs and manifests live only in
a separate mode-`0700` temporary verification tree; their hashes/results are
recorded in the ignored local SDD report, and the temporary tree is removed
after accepted review so it cannot become a second persistent unsealed ledger.

Link the guide once from the root README. Update the live wiki's existing
`Engineering-Setup.md` with the generic, shipped workflow and dated hp-local
evidence; do not duplicate the full host transcript there. Wiki publication is
a separate public push and requires explicit user authorization.

## Repository and artifact boundaries

- Build outputs stay ignored and uncommitted: `rust/target/`,
  `game/build/`, wasm, exports, reports, and rendered frames.
- The project-relative `game/addons/godot_mcp/` remains ignored, untracked,
  export-excluded and developer-only. Its presence cannot become a game,
  build, test or deployment prerequisite.
- MCP-owner prose in `tools/setup-mcp.sh`, `.gitignore`,
  `test/repo_hygiene.sh` and the tracked loop must describe the current
  GitHub-Pages/archive boundary, exact pin, rejected second-client behavior and
  complete project restore; it must not revive retired `deploy.sh`/droplet,
  unpinned/latest, queued-client or deletion-only-uninstall claims.
- `game/` remains the sole Godot 4.7 project for every platform.
- `tools/superpowers` remains the sole developer-tool submodule and never
  enters a deployment archive.
- The setup introduces no new shipped technology or runtime dependency.
- Repository documentation commits use the mandated identity, small green
  commits, evocative narrative subjects and bodies explaining what and why,
  with no assistant attribution.

## Completion

Completion requires fresh proof of the four build gates; the isolated actual
editor/MCP proof, restoration and separate evidence seal; clean durable and
task-worktree tracked state; an independent review of the
spec/plan/guide/README/MCP-loop changes; a separate review of the two wiki
pages; and the Superpowers finish-branch choice. Neither the main repository
branch nor the wiki may be pushed or merged without the user's explicit
integration choice.
