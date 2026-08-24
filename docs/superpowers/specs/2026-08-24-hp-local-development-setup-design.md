# hp-local Development Setup — Design

**Status:** operational scope approved by the user's 2026-08-24 request to set
up `hp-local`, record every resulting change, provide a complete onboarding
guide, and build the game. The dependency and build design is inherited from
the already-approved cross-platform bootstrap specifications below; no new game
or product design is introduced here.

## Goal

Turn the existing Debian workstation reached through `ssh hp-local` into a
reproducible Unseeing development host, prove both the native and Web build
paths on the repository's current `main`, and leave a tracked guide that can
recreate or reverse every machine change without relying on shell history.

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
- `ci/pipeline.sh`.

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

The failed GitHub SSH authentication probe accepted GitHub's ED25519 host key
into `/home/galchenko/.ssh/known_hosts`; that persistent result is part of the
ledger even though authentication itself remained unavailable.

## Decisions

### Checkout and Git scope

Clone the public repository to `/home/galchenko/src/unseeing` over HTTPS only
after `refs/heads/main` is confirmed at the reviewed full SHA
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

Do not install PowerShell, a second Godot package, an agent CLI, or the optional
Godot MCP addon. They are not needed to build or test this Linux host. Node 20
is already present if the user elects to install the ignored addon later.

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

Link the guide once from the root README. Update the live wiki's existing
`Engineering-Setup.md` with the generic, shipped workflow and dated hp-local
evidence; do not duplicate the full host transcript there. Wiki publication is
a separate public push and requires explicit user authorization.

## Repository and artifact boundaries

- Build outputs stay ignored and uncommitted: `rust/target/`,
  `game/build/`, wasm, exports, reports, and rendered frames.
- `game/` remains the sole Godot 4.7 project for every platform.
- `tools/superpowers` remains the sole developer-tool submodule and never
  enters a deployment archive.
- The setup introduces no new shipped technology or runtime dependency.
- Repository documentation commits use the mandated identity, small green
  commits, evocative narrative subjects and bodies explaining what and why,
  with no assistant attribution.

## Completion

Completion requires fresh proof of the four gates, a clean tracked diff, an
independent review of the spec/plan/guide/README changes, a separate review of
the wiki draft, and the Superpowers finish-branch choice. Neither the main
repository branch nor the wiki may be pushed or merged without the user's
explicit integration choice.
