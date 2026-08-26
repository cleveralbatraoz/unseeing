# hp-local Development Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provision `hp-local` reproducibly, prove Unseeing's native, Web,
Linux-player, actual Godot-editor and structured live-MCP paths, and publish a
complete tracked onboarding/change ledger.

**Architecture:** Treat the repository's pinned bootstrap, pipeline and MCP
configuration as behavioral authority; keep host dependencies at system or
user scope according to their owner; and build one clean, commit-pinned clone
whose sole submodule is verified before initialization. Prove the GUI/MCP path
only in a separately bootstrapped remote task worktree, restore every temporary
tracked editor change, and keep its evidence outside the Task-2/5 ledger. Store
no generated artifact in Git; preserve only tracked documentation in the
isolated local task worktree, while drafting the separate wiki update without
pushing it.

**Tech Stack:** Debian 13, POSIX shell, Git, Godot 4.7.1, rustup/Rust,
gdtoolkit, Emscripten 4.0.20, Chromium, Brotli, Node 20/npm 9,
`@satelliteoflove/godot-mcp@4.1.0`, Godot GDExtension and Web export.

**Spec:** `docs/superpowers/specs/2026-08-24-hp-local-development-setup-design.md`

## Global Constraints

- Propagation remains exact and perception remains explicitly authored; this task changes no acoustic, perception, physics, wave, rendering, or gameplay constant.
- The silhouette/reveal/detail laws remain untouched: reveal composes by `max`; occlusion is a `{0,1}` geometry gate; `sight::visible_air` composes ring cuts as distances with `min()`; `DetailKnee` gates only `seen_walled` fragments.
- The superface law remains owned by `rust/src/render/superface.rs`: `COPLANAR_EPS = 2e-3`, `PATCH_EPS = 1e-3`. Separate touching solids remain at least `MIN_SEP = 0.08` apart, owned by `rust/src/render/labels.rs`; new labels remain in `[0.15, 0.96]` except the grandfathered `Role::Case = 0.05`; labels are never assigned by cycling a list.
- Waves remain stopped by geometry through `level_plan::spans_the_corridor`, never node class; prop silhouette clarity remains CPU-only through `level_plan::prop_through`.
- `game/` remains the sole Godot 4.7 project. Supported outputs remain Web, universal macOS, Windows x86_64/arm64 and Linux x86_64/arm64; Rust remains portable across the declared desktop targets and wasm32.
- Law 1 remains: every designer-facing gameplay entity is a registered Rust tool node or plain `.tscn` composition, with the complete object checklist. Law 2 remains: every other behavior lives in pure Rust and registered nodes are thin boundary adapters.
- All functions remain total on their declared domains; domain logic remains pure; dependencies remain explicit; global mutable state and new unsafe Rust remain forbidden.
- No shipped GDScript, platform-specific game implementation, texture/fill/material noise, unapproved technology, or additional wasm GDExtension may be introduced.
- Strict TDD applies to any repository behavior/configuration change: named failing test, observed RED, minimal change, observed GREEN, refactor, and realistic mutation evidence. Documentation-only edits use the repository hygiene/link/build gates and do not invent a production behavior test.
- Build output, exports, `.pck`, `.wasm`, `target/`, reports and rendered frames are never committed. `.import`/`.uid` sidecars are committed when legitimately created; developer-agent tooling never enters a deployment archive.
- Every task is developed in the existing isolated worktree. Reviewers are read-only and never move HEAD in the checkout under review.
- Commits are small, self-contained and green, use repository identity `Dmitrii Galchenko <dggrus@gmail.com>`, have an evocative narrative subject and explanatory body, and contain no `Co-Authored-By`, `Generated with`, or other assistant attribution.
- Integration stops for the user's finish-branch choice. Never merge or push the repository branch or the separate wiki without explicit authorization; merging `main` is the automatic Web-deploy gate.

---

### Task 1: Freeze and review the operational contract

**Files:**
- Create: `docs/superpowers/specs/2026-08-24-hp-local-development-setup-design.md`
- Create: `docs/superpowers/plans/2026-08-24-hp-local-development-setup.md`

**Interfaces:**
- Consumes: the user request, read-only hp-local audit, current `origin/main`, repository pins, wiki `Mechanics-Overview.md`, its four linked mechanics pages, `Engineering-Setup.md`, and `Engineering-Build-Test-Deploy.md`.
- Produces: the approved scope, exact install locations, proof boundary, rollback requirement, and task sequence every later task follows.

- [ ] **Step 1: Confirm isolated and current input state**

Run:

```sh
git_dir=$(cd "$(git rev-parse --git-dir)" && pwd -P)
git_common=$(cd "$(git rev-parse --git-common-dir)" && pwd -P)
test "$git_dir" != "$git_common"
git status --short --branch
git rev-parse HEAD origin/main
git submodule status tools/superpowers
git worktree list --porcelain
git -C /Users/dmgalchenko/unseeing status --short --branch
```

Expected: branch `chore/hp-local-development-setup`, matching full SHAs for
`HEAD` and `origin/main`, a clean tree before these two files, and Superpowers
pin `b36e0829c6d0140e93cfef2ca599b1b07d4a7797`. The durable primary is recorded
as `main` with the unrelated untracked `out`; it is not modified.

- [ ] **Step 2: Confirm the required reading was completed**

Record that the controller read the live wiki's `Mechanics-Overview.md`, all
four linked mechanics pages, `Engineering-Setup.md` and
`Engineering-Build-Test-Deploy.md`, then reconciled them against current code
and pins. Record the stale statements found; do not copy them into this task.

- [ ] **Step 3: Self-review the spec and plan**

Check every spec section has an implementation task, scan for placeholders,
confirm commands use `/home/galchenko/src/unseeing`, and confirm all Global
Constraints above are present.

- [ ] **Step 4: Request an independent read-only review**

Give the reviewer this spec, this plan, `AGENTS.md`, the audited facts and the
existing cross-platform bootstrap spec. Resolve every Critical or Important
finding against repository reality.

- [ ] **Step 5: Verify and commit the task contract**

Run:

```sh
git diff --check
test/repo_hygiene.sh
```

Stage only the two task artifacts and make one documentation commit. Do not use
a literal commit message copied from this plan.

### Task 2: Capture the machine baseline and install external prerequisites

**Files:**
- Remote persistent: Debian package database and package-index cache; `/home/galchenko/.ssh/known_hosts`; `/home/galchenko/.local/bin/godot`; `/home/galchenko/.local/bin/Godot_v4.7.1-stable_linux.x86_64`; `/home/galchenko/.local/share/godot/export_templates/4.7.1.stable/`; rustup user directories; pipx user directories; `/home/galchenko/emsdk/`; `/home/galchenko/.local/state/unseeing/setup/2026-08-24/`.
- Remote temporary: `/home/galchenko/.local/state/unseeing/setup/2026-08-24/downloads/`, removed only after its recorded canonical-path/owner/mode guard passes.

**Interfaces:**
- Consumes: exact pins and install scope from the spec.
- Produces: native linker plus Godot, format/lint, Web compiler, export and browser dependencies available to the unchanged repository scripts; exact before/after evidence for Task 5.

- [ ] **Step 1: Capture fail-first and package baselines**

On `hp-local`, first require the complete dated path
`/home/galchenko/.local/state/unseeing/setup/2026-08-24` to be absent. If it
already exists, stop and preserve it for inspection instead of reusing,
renaming or deleting it. Create the parent if needed, then atomically create
the dated directory and its `downloads` child under `umask 077`; require both
to resolve to their exact expected paths, be owned by `galchenko`, and have
mode `700`. This prevents a retry from overwriting or mixing audit evidence.
Save:

```sh
dpkg-query -W -f='${binary:Package}\t${Version}\n'
apt-mark showmanual
for path in /etc/apt/sources.list /etc/apt/sources.list.d; do
  if [ -f "$path" ]; then
    sha256sum "$path"
  elif [ -d "$path" ]; then
    find "$path" -type f -print0 | sort -z | xargs -0 -r sha256sum
  else
    printf 'ABSENT\t%s\n' "$path"
  fi
done
for path in "$HOME/.profile" "$HOME/.bashrc"; do
  if [ -f "$path" ]; then sha256sum "$path"; else printf 'ABSENT\t%s\n' "$path"; fi
done
ssh-keygen -F github.com -f "$HOME/.ssh/known_hosts"
```

as `before-*.txt`, together with absence/version output for Godot, rustup,
gdformat/gdlint, Chromium, Brotli, emsdk and templates. Store no environment
values, credentials, private keys or tokens. The already-added public GitHub
entry is stored separately and fingerprinted with `ssh-keygen -lf`.

Expected RED: the audited development commands are absent exactly as stated in
the design.

- [ ] **Step 2: Install the two missing Debian packages**

Run:

```sh
sudo apt-get update
sudo apt-get install -y chromium brotli
```

Save the matching `/var/log/apt/history.log` transaction, after-package list,
after-manual list and their diffs. Hash the APT source files again to prove no
repository was added. Rollback may remove only packages proven new by that
diff and must restore the prior manual markings; package indexes/cache are
recreated metadata and are documented rather than destructively rewound.

- [ ] **Step 3: Install checksum-verified Godot and templates**

Use these exact official URLs inside the `downloads` child:

```sh
base=https://github.com/godotengine/godot/releases/download/4.7.1-stable
curl --fail --location --proto '=https' --tlsv1.2 --remote-name \
  "$base/Godot_v4.7.1-stable_linux.x86_64.zip"
curl --fail --location --proto '=https' --tlsv1.2 --remote-name \
  "$base/Godot_v4.7.1-stable_export_templates.tpz"
curl --fail --location --proto '=https' --tlsv1.2 --remote-name \
  "$base/SHA512-SUMS.txt"
grep -E ' Godot_v4\.7\.1-stable_(linux\.x86_64\.zip|export_templates\.tpz)$' \
  SHA512-SUMS.txt > godot-selected.sha512
test "$(wc -l < godot-selected.sha512)" -eq 2
sha512sum --check godot-selected.sha512
```

Require the selected lines to equal the two SHA-512 values in the spec. Reject
absolute or `..` archive members. Require the editor ZIP to contain exactly
`Godot_v4.7.1-stable_linux.x86_64`. Extract to a scratch child, install that
file mode 0755 under `~/.local/bin`, and create `~/.local/bin/godot` as a
relative symlink to it. Extract the TPZ to a separate scratch child, require
its sole top-level directory to be `templates/`, then copy `templates/*`—not
the wrapper directory—into the previously absent
`~/.local/share/godot/export_templates/4.7.1.stable/`. Require at least
`version.txt`, `web_release.zip` and `linux_release.x86_64` in the final
directory.

Immediately before installation, explicitly require all three persistent
targets to be absent:

```sh
test ! -e "$HOME/.local/bin/Godot_v4.7.1-stable_linux.x86_64"
test ! -e "$HOME/.local/bin/godot" && test ! -L "$HOME/.local/bin/godot"
test ! -e "$HOME/.local/share/godot/export_templates/4.7.1.stable"
```

Refuse rather than overwrite if any target exists, even if the earlier audit
reported Godot absent.

Use `python3`'s standard-library `zipfile` before either extraction: reject an
empty archive, any absolute member, any member whose `PurePosixPath.parts`
contains `..`, and any top-level component other than the expected executable
name for the editor or `templates` for the TPZ. Extract only after this check;
then require the scratch paths with `test -f` before `install`/`cp`.

Expected:

```text
4.7.1.stable.official
```

- [ ] **Step 4: Install rustup and the two pinned Rust lanes**

From the controlled `downloads` directory, download and verify the official
x86_64 GNU installer exactly:

```sh
rustup_base=https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu
curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
  --output rustup-init "$rustup_base/rustup-init"
curl --fail --location --silent --show-error --proto '=https' --tlsv1.2 \
  --output rustup-init.sha256 "$rustup_base/rustup-init.sha256"
test -f rustup-init
test -f rustup-init.sha256
grep -Eq '^[0-9a-f]{64} \*\./rustup-init$' rustup-init.sha256
sha256sum --check rustup-init.sha256
chmod 0700 rustup-init
sha256sum rustup-init
```

Record the resolved executable hash, then run:

```sh
./rustup-init -y --profile minimal --default-toolchain none
. "$HOME/.cargo/env"
rustup toolchain install 1.97.1 --profile minimal
rustup component add rustfmt clippy --toolchain 1.97.1
rustup target add \
  aarch64-apple-darwin x86_64-apple-darwin \
  x86_64-pc-windows-msvc aarch64-pc-windows-msvc \
  aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu \
  --toolchain 1.97.1
rustup toolchain install nightly-2026-05-25 --profile minimal
rustup component add rust-src --toolchain nightly-2026-05-25
rustup target add wasm32-unknown-emscripten --toolchain nightly-2026-05-25
```

Compare the six stable targets literally to `rust/rust-toolchain.toml`, then
verify them with `rustup target list --installed --toolchain 1.97.1`. Record
`rustup toolchain list`, nightly components/target, and actual startup-file
hash/installer-owned-line changes. The mutable installer distribution is a
documented trust boundary; its resolved verified hash makes this historical
installation auditable.

- [ ] **Step 5: Install gdtoolkit through pipx**

Run the repository-prescribed:

```sh
pipx install 'gdtoolkit==4.*'
```

Record the resolved package version, venv location and console applications;
verify both `gdformat` and `gdlint` execute from `~/.local/bin`. Download the
resolved wheel without dependencies into the evidence directory and record its
SHA-256. The guide distinguishes the repository-supported `4.*` range from the
exact version/hash installed on 2026-08-24 and shows both reproduction forms.

- [ ] **Step 6: Install the pinned emsdk**

First require:

```sh
git ls-remote --tags https://github.com/emscripten-core/emsdk.git \
  refs/tags/4.0.20
```

to return `e4fe26ef59168ff44f4c23c466e497bf60b3411e`, then run:

```sh
git clone --branch 4.0.20 --depth 1 \
  https://github.com/emscripten-core/emsdk.git /home/galchenko/emsdk
cd /home/galchenko/emsdk
test "$(git remote get-url origin)" = \
  https://github.com/emscripten-core/emsdk.git
test "$(git rev-parse HEAD)" = \
  e4fe26ef59168ff44f4c23c466e497bf60b3411e
./emsdk install 4.0.20
./emsdk activate 4.0.20
```

The tag is not treated as a cryptographic signature: the trust boundary is the
official HTTPS remote plus the reviewed commit pin. Do not make emsdk global or
add it permanently to shell startup; `rust/build-wasm.sh` sources it.

- [ ] **Step 7: Verify the installed dependency boundary**

Read back versions and paths for every installed tool and compare package,
manual-package, APT-source and startup-file snapshots. Generate sorted
recursive type/mode/size/symlink manifests plus regular-file SHA-256 manifests
for the exact Godot, template, Rust, pipx-gdtoolkit and emsdk roots. Store the
manifests and their own SHA-256 hashes in the persistent evidence directory.
Keep it after documentation is complete.

- [ ] **Step 8: Request read-only host-change review**

Give a reviewer the spec, before/after evidence directory, package transaction,
installed version output and exact persistent paths. Resolve every verified
Critical or Important omission before using the toolchain.

### Task 3: Clone and configure the durable Unseeing checkout

**Files:**
- Remote create: `/home/galchenko/src/unseeing/`
- Remote repository config: `.git/config` in that checkout
- Remote generated/ignored: `tools/superpowers/` submodule checkout

**Interfaces:**
- Consumes: public GitHub read access, installed Git and the repository's exact gitlink.
- Produces: one durable primary checkout with correct local identity, hooks and verified Superpowers pin for Tasks 4–5.

- [ ] **Step 1: Pin and clone the reviewed main commit over HTTPS**

Require the remote before creating the destination, then clone without running
the submodule checkout:

```sh
expected=d6285b0bba84dd29846a9613c2e8081191e46cfd
actual=$(git ls-remote https://github.com/cleveralbatraoz/unseeing.git \
  refs/heads/main | awk 'NR == 1 { print $1 }')
test "$actual" = "$expected"
if [ ! -e /home/galchenko/src ]; then mkdir -m 0755 /home/galchenko/src; fi
test "$(realpath /home/galchenko/src)" = /home/galchenko/src
test "$(stat -c %U /home/galchenko/src)" = galchenko
test ! -e /home/galchenko/src/unseeing
git clone --branch main --no-recurse-submodules \
  https://github.com/cleveralbatraoz/unseeing.git \
  /home/galchenko/src/unseeing
test "$(git -C /home/galchenko/src/unseeing rev-parse HEAD)" = "$expected"
```

Any SHA mismatch stops the task for a deliberate baseline update; it is not
silently replaced with a moving `main`.

- [ ] **Step 2: Verify parent metadata before initializing Superpowers**

Inside the clone require `origin` to be the canonical HTTPS URL, run
`ci/verify-superpowers.sh metadata`, and independently require the sole
`.gitmodules` URL, mode-160000 path, gitlink and `ci/superpowers.lock` values to
match `https://github.com/obra/superpowers.git`, `tools/superpowers`,
`b36e0829c6d0140e93cfef2ca599b1b07d4a7797` and v6.3.0. Only then run:

```sh
git submodule update --init --depth 1 -- tools/superpowers
ci/verify-superpowers.sh full
```

- [ ] **Step 3: Pin repository-local Git configuration**

Inside the clone configure the mandated name, email and
`core.hooksPath=.githooks`. Do not set global identity or hooks.

- [ ] **Step 4: Verify checkout and developer-tool integrity**

Run:

```sh
git status --short --branch
git rev-parse HEAD origin/main
git submodule status tools/superpowers
ci/verify-superpowers.sh full
git config --local --get-regexp '^(user\.(name|email)|core\.hooksPath)$'
```

Expected: clean `main`, matching SHA, Superpowers v6.3.0 pin, and the three
repository-local settings. Record that GitHub push authentication remains
unconfigured and is not a build blocker.

- [ ] **Step 5: Request read-only checkout review**

Give a reviewer the remote checkout path, expected SHA, local Git config and
Superpowers verification output. Resolve every verified Critical or Important
finding without moving the primary checkout away from `main`.

### Task 4: Build and test every requested host path

**Files:**
- Remote generated/ignored: `/home/galchenko/src/unseeing/rust/target/`
- Remote generated/ignored: `/home/galchenko/src/unseeing/game/.godot/`
- Remote generated/ignored: `/home/galchenko/src/unseeing/game/build/web/`
- Remote generated/ignored: `/home/galchenko/src/unseeing/game/build/linux/`
- Remote generated/ignored: `/home/galchenko/src/unseeing/game/reports/`

**Interfaces:**
- Consumes: configured clean clone and every prerequisite from Tasks 2–3.
- Produces: fresh command output proving native engine registration, all checks, Web export/browser render, and a Linux x86_64 player artifact.

- [ ] **Step 1: Bootstrap the authoring engine**

Run from the clone with `CARGO_BUILD_JOBS=4` and a sourced `~/.cargo/env`:

```sh
. "$HOME/.cargo/env"
CARGO_BUILD_JOBS=4 tools/bootstrap.sh
```

Expected terminal evidence: `probe: PASS (19 checks)` followed by
`bootstrap: OK`.

- [ ] **Step 2: Run the checks-only gate**

Run:

```sh
. "$HOME/.cargo/env"
CARGO_BUILD_JOBS=4 SKIP_EXPORT=1 ci/pipeline.sh
```

Expected terminal evidence: `ci: SKIP_EXPORT=1` and `ci: OK`. PowerShell
boundary tests may explicitly report SKIP because PowerShell is intentionally
not installed; every other stage must pass.

- [ ] **Step 3: Build and browser-test the Web game**

Run:

```sh
. "$HOME/.cargo/env"
CARGO_BUILD_JOBS=4 ci/pipeline.sh
```

Expected terminal evidence includes the wasm build path, Web export success,
Chromium smoke-test success and final `ci: OK`. No `SKIP_EXPORT` or
`SKIP_SMOKE` is allowed.

- [ ] **Step 4: Build the Linux x86_64 player**

Run:

```sh
. "$HOME/.cargo/env"
CARGO_BUILD_JOBS=4 tools/export_linux.sh \
  "Linux x86_64" build/linux/unseeing
```

Expected terminal evidence: `export-linux: OK` with a non-empty executable,
adjacent `libunseeing_core.so`, and no loose `.pck`.

- [ ] **Step 5: Record artifact evidence and tracked cleanliness**

Record one sorted artifact manifest with size and SHA-256 for every regular
file under `game/build/web/` (including `.gz` and `.br`), and for the Linux
executable, adjacent library and retained sibling export log. Run
`git status --short --ignored` and prove all generated paths are ignored while
`git status --short` remains empty.

If any result is unexpected, invoke the pinned systematic-debugging skill,
capture the failing output, form one hypothesis, and do not change production
code before a named regression test observes the failure.

- [ ] **Step 6: Request read-only build-evidence review**

Give a reviewer the four full logs, exit statuses, artifact manifest and clean
tracked status. Resolve every verified Critical or Important evidence gap;
never substitute a skipped gate.

### Task 4A: Prove the actual Godot editor and repository-pinned MCP loop

**Files:**
- Modify: `docs/superpowers/mcp/godot-mcp-loop.md`
- Remote persistent: `/home/galchenko/src/unseeing/.worktrees/hp-local-mcp-setup/`, its dedicated branch/worktree admin state in the durable clone's common Git directory, ignored worktree-local release output, and ignored `game/addons/godot_mcp/`
- Remote persistent: `/home/galchenko/.local/state/unseeing/mcp-setup/2026-08-25/`
- Remote generated/shared: `~/.npm/` delta and Godot cache/config/user-data deltas, retained rather than broadly removed
- Remote temporary: the task worktree's MCP-only `game/project.godot` diff, ignored `game/override.cfg`, and one named transient user-systemd unit; all restored/removed before completion
- Controller shared: pre-existing `/Users/dmgalchenko/.godot-mcp/usage.log`, retained and required byte-unchanged
- Controller temporary: one loopback-only leased SSH forward and one ignored owned MCP-controller directory/process pair, always signal-cleaned
- Controller read-only source: exact existing NPX tree `/Users/dmgalchenko/.npm/_npx/e9af8ac9cd94a1c8/`
- Ignored only: reviewed session-supervisor, forward-lease and owned-MCP-controller helpers; real-filesystem/process tests; mutation harnesses; and Task-4A execution report under this plan's SDD workspace

**Interfaces:**
- Consumes: the reviewed clean remote `main`, official Godot `4.7.1.stable.official.a13da4feb`, checked-in bootstrap and MCP pins, active uid-1000 Wayland user-manager session, ordinary target Node `20.19.2` and npm/npx `9.2.0`, exact controller Node `22.23.2`, the validated controller cache containing exact Godot MCP 4.1.0 and MCP SDK 1.30.0, and Task 4's build proof.
- Produces: an isolated, separately sealed proof of the actual editor UI and owned structured MCP loop on proof-only port 16550, an exact tracked-file restoration verdict, unchanged unrelated controller MCP state, a retained ignored 4.1.0 addon for this worktree only, and factual evidence for Task 5.

- [ ] **Step 1: Build and review the bounded lifecycle helpers before host mutation**

In the ignored local SDD workspace, use strict TDD with Python's standard
library plus the installed Node runtime only where the MCP SDK boundary
requires it; use real temporary files/symlinks/processes, no mock and no
network. Build one fixed-path session supervisor, one fixed loopback-forward
lease wrapper and one owned MCP controller. Their live modes accept no caller-
controlled project, deletion target, executable, unit, port, evidence path,
package tree or tool sequence.

The supervisor must refuse a wrong canonical worktree/project, dirty initial
`game/project.godot`, wrong owner/type/mode/device/inode/hash, unexpected addon
version, output collision or live diff. It captures the exact tracked preimage,
adds only the reviewed 4.1.0 enabled row and proof settings
`godot_mcp/port_override_enabled=true` and
`godot_mcp/port_override=16550`, creates only the exact ignored 1280x720
override, launches only the exact editor child, and owns a bounded always-run
cleanup path. Dated automated cleanup stops only that editor child while the
plugin is still enabled, accepts only the enabled row plus `MCPGameBridge`
autoload and four `godot_mcp/*` settings, removes only that override, and
restores the preimage directly. It never disables the plugin or creates a
post-disable validation phase. If unrelated bytes appear, it preserves them and
emits a blocking recovery artifact rather than overwriting them.

The three helpers share one absolute monotonic 1200-second deadline, a
1170-second mutation-capable work cutoff, and a final fixed 30-second cleanup
reserve. No mutation-capable work may start after the cutoff. The clock begins
before any supervisor/editor, tunnel or controller startup and ends only after
cleanup, project restoration and listener-absence proof. No helper may mint,
restart or extend either endpoint. The lease wrapper fixes both 16550 loopback endpoints,
the complete SSH argument vector and that deadline, owns the PID, and closes it
on normal exit and every handled signal. It validates both base and full
`ssh -G` effective configurations, exact listener owners, and refuses inherited
forwards, control sockets, fork-after-authentication, proxies and local
commands. The owned MCP controller validates the fixed cached NPX tree's lock,
package files, Godot MCP integrity and resolved SDK integrity without executing
npm or using network. Ordinary addon installation still requires Node 20 or
newer; the dated controller requires exact Node `22.23.2` and its direct
executable identity. It copies descriptor-held bytes into a private execution
capsule, imports the SDK from a held `Buffer` through `registerHooks()`,
gives the child the same held-byte preload, and checks sealed parent and child
resolution ledgers before `process.execPath` starts the staged `dist/cli.js`.
It owns that child and closes it on every exit. It permits only the SDK's
present safe inherited environment
names (`HOME`, `LOGNAME`, `PATH`, `SHELL`, `TERM`, `USER`) plus fixed
`GODOT_HOST`, `GODOT_PORT` and `GODOT_MCP_USAGE_LOG` overrides.

Observe named RED before minimal production for every branch, then full GREEN.
Mutation-check live paths/commands, project-diff classification, restore and
refusal branches, override bytes/target, editor child/unit identity, base/full
SSH effective configuration, tunnel bind/options/listener owner, lease expiry
and signal cleanup, cached package/SDK identity and integrity, safe-plus-fixed
environment, stdio child ownership/close-on-failure, exact tool sequence and
assertions, the single 1200-second startup-through-cleanup deadline, failure
records and last-write seal. Request independent read-only
review of the amended design/plan/MCP-loop and all three helper/test/mutation
pairs. No `hp-local` write is authorized before that review accepts every
Critical/Important finding.

- [ ] **Step 2: Create the separate evidence boundary and isolated worktree**

Require the evidence path
`/home/galchenko/.local/state/unseeing/mcp-setup/2026-08-25` and exact worktree
path `/home/galchenko/src/unseeing/.worktrees/hp-local-mcp-setup` to be absent,
not symlinks. Revalidate the durable clone's canonical path, clean `main`,
reviewed HEAD/origin and owner; prove its `.worktrees/` parent is ignored; and
require the dedicated branch name to be unused. Under `umask 077`, exclusively
create the owner-only evidence root and create the named task branch/worktree
from reviewed `main`. Record the full before/after shared common-Git worktree,
branch and admin delta even though the durable tracked tree stays unchanged.

Capture deterministic complete before manifests for the task-worktree MCP
boundary, `~/.npm`, Cargo, Rustup and mutable Godot cache/config/user-data
roots. Cover every ignored worktree output, including `rust/target`,
`game/.godot`, the addon and temporary override boundary. Record only
`DISPLAY`, `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR` and the user D-Bus address from
the uid-1000 user manager, never a raw environment. On the controller, capture
the existing usage log's path, device, inode, UID, mode, link count, 594-line /
92,992-byte baseline, SHA-256
`4a470e3854b12fdb0db7915ffc6940c1b6332d77f14f570cfaadeb15a1ff7929`,
and exact descriptor `mtime_ns` `1787340988551255243`; reject the rounded
`1787340988000000000` transcription;
do not copy its contents to the host or tracked documentation, and require the
same complete metadata/content boundary after the proof. Capture the exact NPX
tree's canonical path, owner/type/mode, package lock and complete regular-file
manifest before any controller child starts; no task process may modify it.

- [ ] **Step 3: Bootstrap this worktree and install exact addon 4.1.0**

Ignored native output does not cross worktrees. From the new task worktree,
source `~/.cargo/env` and run the checked-in bootstrap with the same bounded
resource setting:

```sh
. "$HOME/.cargo/env"
CARGO_BUILD_JOBS=4 CARGO_NET_OFFLINE=true RUSTUP_AUTO_INSTALL=0 \
  tools/bootstrap.sh
```

Preserve the complete log, literal status and duration. Require the exact
release GDExtension in this worktree, `probe: PASS (19 checks)`, and
`bootstrap: OK`; never point it at another checkout's binary.

Verify Node `20.19.2`, npm/npx `9.2.0`, the matching exact 4.1.0 literals in
`.mcp.json` and `tools/setup-mcp.sh`, absent addon path and no target listener
on proof-only port 16550. From the task worktree run exactly:

```sh
/usr/bin/env -u GODOT_MCP_VERSION ./tools/setup-mcp.sh
```

Record stdout/stderr/status/duration, complete installed-addon metadata and
regular-file hashes, `plugin.cfg` version `4.1.0`, registry integrity
`sha512-uq3Gh5n7fos8vIoXpr32/K7r9tL9eYLbERr+Tolksg3Y+FC5coYEkRkbJ1JktMMhoH/BnGWsWhE5E+XJ/nMEPg==`,
the complete npm-cache delta, ignored verdict and clean tracked status. Install
does not authorize an enabled row or any other tracked project change.

- [ ] **Step 4: Start the owned graphical editor session**

Transfer and re-hash the reviewed supervisor through one new guarded mode-0700
temporary directory. It must capture a byte-for-byte HEAD-identical
`game/project.godot`, add only the required plugin row plus
`godot_mcp/port_override_enabled=true` and
`godot_mcp/port_override=16550`, and create this exact ignored override before
editor boot:

```text
[display]

window/size/mode=0
window/size/viewport_width=1280
window/size/viewport_height=720
```

Start it as uid 1000 through one uniquely
named transient user-systemd unit in the already active GNOME Wayland session.
Poll—never sleep—for the owned child process, unit state, and target
`127.0.0.1:16550` listener. Prove the child is official Godot
`4.7.1.stable.official.a13da4feb` in editor mode with project path
`/home/galchenko/src/unseeing/.worktrees/hp-local-mcp-setup/game`; record only
the named unit's journal. Require that exact child to own the listener. Verify
the resulting complete live project diff is still limited to the enabled row,
autoload and four settings.

- [ ] **Step 5: Prove SSH configuration and lease the isolated forward**

Consume Step 3's recorded pre-editor absence proof. Now require controller port
16550 unused and target port 16550 owned only by the supervisor's Godot child.
Capture base `ssh -G hp-local` and stop if its effective output
contains any local, remote or dynamic forward; control path/master/persistence;
fork-after-authentication; proxy command/jump; or local command. Capture the
complete proposed command through `ssh -G` too; require exactly one local
forward with the literal mapping below, `ForkAfterAuthentication=no`, and no
other forward or forbidden setting. Both base and full effective configurations
reject `ClearAllForwardings=yes`: OpenSSH would also discard the required
`-L`.

Start the reviewed foreground lease wrapper around exactly this transport:

```sh
ssh -N -T \
  -o BatchMode=yes \
  -o ExitOnForwardFailure=yes \
  -o ForkAfterAuthentication=no \
  -o ControlMaster=no \
  -o ControlPath=none \
  -o ControlPersist=no \
  -o PermitLocalCommand=no \
  -o UpdateHostKeys=no \
  -o StrictHostKeyChecking=yes \
  -o ServerAliveInterval=15 \
  -o ServerAliveCountMax=3 \
  -L 127.0.0.1:16550:127.0.0.1:16550 \
  hp-local
```

Record its exact command, PID, endpoints and the shared absolute deadline
locally. Prove both listeners loopback-only, the exact SSH PID owns the
controller listener, and the supervisor's exact Godot child owns the target
listener. Never bind a wildcard/non-loopback address, extend the 1200-second
startup-through-cleanup deadline or touch unrelated default-port clients.

- [ ] **Step 6: Start one owned offline MCP client/server pair**

From a new ignored controller directory, validate exact existing NPX root
`/Users/dmgalchenko/.npm/_npx/e9af8ac9cd94a1c8`, its package lock and complete
package-file manifest. Require `@satelliteoflove/godot-mcp@4.1.0` integrity
`sha512-uq3Gh5n7fos8vIoXpr32/K7r9tL9eYLbERr+Tolksg3Y+FC5coYEkRkbJ1JktMMhoH/BnGWsWhE5E+XJ/nMEPg==`
and resolved `@modelcontextprotocol/sdk@1.30.0` integrity
`sha512-xKd8OIzlqNzcqcNumGAa6g+PW2kjD5vrpcKOnfldAUPP3j7lnqMPwlTXQm8gF+UwH72z0lqaRbjr9hqGz0eITA==`.
Do not run npm/npx or contact a registry.

Run the reviewed Node controller through the exact Node/capsule/held-byte and
resolution-ledger boundary above; never fall back to ordinary pathname module
loading. Its transport uses `process.execPath` with the staged Godot MCP
`dist/cli.js`. Set fixed
`GODOT_HOST=127.0.0.1`, `GODOT_PORT=16550`, and
`GODOT_MCP_USAGE_LOG=0`. Accept only present SDK-safe inherited names `HOME`,
`LOGNAME`, `PATH`, `SHELL`, `TERM`, and `USER` plus those three fixed values;
reject every other child-environment name. Record parent/child PIDs, exact
argv/executable identities and the whitelisted environment. Require the shared
usage-log boundary unchanged. List available tools and verify the expected
Godot tool surface before editor/game mutation. Package source and fixed
overrides prove the endpoint; never treat `addon_status` as an endpoint
diagnostic.

- [ ] **Step 7: Exercise the actual UI and structured MCP boundary**

Through only the owned MCP client/server pair, require exact server/addon
version compatibility, project path, Godot version, configured main scene and
editor state. Open `res://scenes/level_02.tscn`; inspect the edited-scene tree;
select one returned child and read back that selection. Capture exactly one
640-pixel-wide 3D editor viewport because the user explicitly requested actual
UI validation. Inspect it transiently and retain only canonical dimensions and
tool-result hash, not screenshot bytes, in evidence.

Run configured main frozen under the supervisor-owned 1280x720 override.
After the game starts, call exactly `godot_input {action:"get_map"}`—not
`godot_project get_settings`—and require the running game's returned
`move_forward` action to contain at least one event. Step two initialization
frames, take a structured snapshot through the running current scene's
`WaveObserver.snapshot(main.now)`, then make one `godot_game_time` request with
`action:"step"`, `frames:30`, and
`inputs:[{action_name:"move_forward",start_ms:0,duration_ms:500}]`; take the
matching after snapshot. This request is `frames:30`; Godot MCP 4.1.0's
accepted reply is `frames:31`, comprising the 30 requested frames plus one
input-settle frame, and both values are required exactly. A separate
frozen-time input injection is forbidden
because its edge is not consumed by the stepped frames. Require the snapshot's
own hero/eye position to move by more than `0.25 m`; source or time evolution
is not a valid control. Run `godot_validate_meshes` against configured main.
Version 4.1.0 returns a plain text value, not `structuredContent`;
require the complete value anchored to exactly:

```text
Checked 144 meshes (144 surfaces) — no integrity problems. This rules out winding, dropped triangles, degenerate UVs/tangents, and NaN data. If rendering still looks wrong, the cause is lighting or materials, not mesh data — note that SDFGI replaces constant ambient light, so shadow-side fill must come from a shadowless fill DirectionalLight rather than ambient_light_energy.
```

Require no editor or game errors. Before SDK import, consume a fresh supervisor
contract binding the absent named unit, journal cursor, unit/Godot start
identities, exact project/evidence roots and pending gate. Controller success
may report only `controller_lane_status:"passed"` with
`integrated_proof_status:"pending_game_log_gate"`. After game stop, the
supervisor-owned game-process journal finalizer inspects only that unit and
cursor interval, sanitizes/hashes the result, requires zero game-process errors,
and binds the terminal gate back to the contract. Do not call the integrated
proof successful before that result. Do not save or edit any scene, node,
resource, project setting or game source.

- [ ] **Step 8: Stop, restore and inventory every delta**

Stop the game through the owned MCP client. Close that client and require its
`finally` path to stop only its exact stdio child. Stop the named transient
supervisor unit while the addon remains enabled. Its bounded cleanup stops only
its editor child, validates the complete stopped-editor diff as exactly the
enabled row, `MCPGameBridge` autoload and four settings, removes only the exact
ignored override, and restores captured `project.godot` directly with exact
bytes/SHA-256/UID/GID/mode verification. Device plus original/replacement inode
identities are facts without an equality requirement; timestamps are outside
the restoration contract. It must not disable the plugin or create a
post-disable phase. Then close the leased SSH forward through its owner and
prove both controller and target port 16550 released before the one absolute
1200-second deadline. Prove the task worktree and durable primary tracked-
clean; neither path may use `assume-unchanged` or `skip-worktree`.

Retain the installed ignored addon and worktree-local build output for later
sessions in this task worktree. Retain generated npm/Godot caches and the
controller's pre-existing usage log; capture deterministic after/delta
manifests and prove its path/device/inode/UID/mode/link count/size/line
count/SHA-256/exact `mtime_ns` boundary unchanged. Remove only guarded
transfer/controller directories. Record that a
future task worktree must run both its own bootstrap and `tools/setup-mcp.sh`;
neither ignored state propagates.

Record, but do not invoke, the retained-worktree rollback. Require the exact
canonical path, dedicated branch, reviewed HEAD, clean tracked status,
restored `project.godot`, stopped editor/unit/client/tunnel and only
inventoried ignored outputs. Guardedly delete each exact manifest-authorized
ignored root—`rust/target`, `game/.godot`, `game/addons/godot_mcp`,
`game/override.cfg`, and any reviewed controller directory—after canonical
path/type/owner/mode/file-hash verification; stop on residue or mismatch. Only then may the
durable checkout use non-forced `git worktree remove`, prove path and common-
Git admin entry absent, and remove a no-unique-commit proof branch with safe
`git branch -d`. Never use `--force` or recursive manual deletion.

- [ ] **Step 9: Seal and independently review Task-4A evidence**

Write canonical summaries, before/after/delta manifests, exact commands,
statuses and durations—including every failed attempt and cleanup
disposition—plus tool-result hashes, restore/absence proofs and rollback facts
under the separate evidence root. Exclude exactly the final manifest and its
digest from a bytewise-sorted relative SHA-256 manifest, fsync the digest last,
then perform only read-only regeneration. Do not store credentials, raw
environment, screenshot bytes, build output, private user content or the
controller's historical usage log.

Request independent read-only review of the full evidence seal, structured
MCP results and anchored mesh text, exact project restoration, unchanged
controller usage log, both clean worktrees, retained ignored state and
controller/transfer/forward/unit cleanup. Resolve every verified Critical or
Important finding through systematic debugging; do not retry a state-changing
step ad hoc. Task 4A makes no repository commit and never pushes, merges,
publishes the wiki or changes any game/platform law.

#### Observed Task-4A disposition — 2026-08-26

Attempt 7 passed an explicitly editor-only fallback, not Steps 6--9's full
runtime game protocol. Attempts 1--6 were each cleaned before the next:
excessive clock skew led to exact Debian `systemd-timesyncd` installation;
the real `0600` worktree mode replaced a mistaken `0644` assumption; a launch
`TypeError` was totalized and then traced to the version-probe callback
boundary; and the held `/dev/fd/<ssh-fd>` transport plus Godot's atomic inode
replacement led to direct-owned Godot/SSH with guarded content/metadata
restoration.

The successful scope proved the full editor/addon handshake, opened level 02,
selected `/root/Level02/Room`, captured and retired one reviewed editor image,
kept editor errors and the complete usage-log boundary unchanged, restored the
project exactly, and left no owned unit/process/listener/override residue. The
isolated tracked-clean worktree and exact ignored development outputs remain
intentionally retained. No runtime-game MCP claim is made: no game, movement,
runtime snapshot, 144-mesh result, or supervisor game-journal terminal gate
ran. The earlier native/Web run and export proof is independent. Task 5 must
record this scoped result and all seven attempts without marking the unchecked
runtime steps complete.

### Task 5: Write the complete onboarding and change ledger

**Files:**
- Create: `docs/hp-local-development-setup.md`
- Modify: `README.md`
- Modify: `docs/superpowers/mcp/godot-mcp-loop.md`
- Modify: `docs/superpowers/specs/2026-08-24-hp-local-development-setup-design.md`
- Modify: `docs/superpowers/plans/2026-08-24-hp-local-development-setup.md`
- Modify: `tools/setup-mcp.sh` (behavior change: reject any present
  `GODOT_MCP_VERSION` before Node/npx and keep exact 4.1.0)
- Modify: `.gitignore` (comments only)
- Modify: `test/repo_hygiene.sh` (static guidance checks plus an executable
  fake-Node/npx regression proving override exit 2 and no npx invocation)
- Ignored only: `.superpowers/sdd/2026-08-24-hp-local-development-setup/task-5-cleanup-seal.py`
- Ignored only: `.superpowers/sdd/2026-08-24-hp-local-development-setup/task-5-cleanup-seal-test.py`
- Ignored only: `.superpowers/sdd/2026-08-24-hp-local-development-setup/task-5-cleanup-seal-mutations.py`
- Ignored only: `.superpowers/sdd/2026-08-24-hp-local-development-setup/task-5-phase-a-report.md`

**Interfaces:**
- Consumes: factual before/after package/filesystem evidence, fresh build results from Tasks 2–4, and the separately sealed actual-editor/MCP results from Task 4A.
- Produces: a self-contained fresh-host procedure, rollback guide, corrected live-MCP loop and dated proof record discoverable from the root README.

- [ ] **Step 1: Draft the tracked Phase-A contract and guide without remote mutation**

Amend the design with the historical pre-cleanup-checkpoint versus current
post-cleanup-final-seal distinction. Draft the guide from Tasks 2--4A's factual
reports and narrowly scoped read-only evidence checks. Include host role and
baseline SHA; pre-existing inventory; exact persistent changes and no-change
results; the exact APT transaction; user-level installs; repository-local
settings; removed temporary state; retained ignored artifacts; unperformed
optional steps; four build gates and complete artifact hashes; daily/editor,
update, troubleshooting and security workflows; guarded rollback; and the
GitHub-auth limitation. Name the checked-in owner of every quoted pin and
distinguish supported reproduction ranges from exact 2026-08-24 resolutions.
Account separately for every Task-4-generated Cargo/rustup, Godot, gdtoolkit,
Chromium and PKI user-state root without relabelling it as Task-2 installed
state. Rollback fences are independently fail-fast; APT removal requires an
exact reviewed simulation; later Cargo state forbids broad rustup uninstall;
and shared/sensitive-capable state has no deletion without a complete
unchanged-boundary proof.

Include Task 4A's actual Godot UI and MCP boundary without copying a raw tool
transcript: isolated worktree/common-Git delta and per-worktree bootstrap;
Node/npm and exact 4.1.0 pin/integrity; install versus per-session enable;
temporary `project.godot` autoload/settings and `override.cfg` lifecycle;
local default-port use versus remote owned-client, isolated-port and leased
loopback-SSH use; collision-free one-client-per-port behavior; validated cached
controller package/SDK with no live npm; exact editor-only UI/tool results and
an explicit absence of any runtime movement/snapshot/144-mesh claim; the dated
owned-controller's unchanged usage-log boundary (not a promise for ordinary clients);
generated ignored addon/build/npm/Godot state; separate evidence seal;
troubleshooting, update and guarded uninstall. Correct stale MCP-owner comments
and setup guidance in `tools/setup-mcp.sh`, `.gitignore` and
`test/repo_hygiene.sh`: no retired `deploy.sh`/droplet path, unpinned/latest
version, queued client, once-per-machine enablement, or deletion-only uninstall
may remain. Pin the per-worktree Enable and exact-restore guidance with the
static hygiene assertion. Never claim addon-tree deletion alone reverses the
tracked project settings.

Make ordinary manual use actionable as Disable, close editor, validate exactly
the post-disable autoload-plus-four-settings residue, then restore captured
bytes/SHA-256/UID/GID/mode while recording device plus original and replacement
inode identities as facts without requiring equality; timestamps are outside
the restoration contract. Addon removal deletes only exact entries from its
private install manifest after canonical path/type/owner/mode/file-hash checks;
no unbounded recursive deletion is permitted. Daily authoring and verification
run in an explicitly selected task worktree, never the durable primary.

The reusable path explicitly selects one unused evidence date, refuses reuse,
clones the then-current public `main`, records its observed SHA and consumes
pins from that clone. Every standalone fence reconstructs its date/root/PATH
and fails fast. It records concrete post-state/deltas, installed-root
manifests, status-preserving four-gate logs and artifacts, then performs its
own exact scratch cleanup and non-self-referential digest-last seal. It does
not assert that moving `main` equals the dated baseline or reuse Task 5's
historical helper.

The Phase-A guide may contain one clearly delimited final-seal table whose
values are awaiting the observed Phase-B run. It contains no other placeholder
or invented cleanup result.

- [ ] **Step 2: Add one README link**

Under `README.md` Setup, link the hp-local guide without duplicating its host
transcript or changing generic setup commands.

- [ ] **Step 3: Build the ignored cleanup/seal helper with strict TDD**

Use Python's standard library only, real temporary files/directories/symlinks,
and no mock or network. Observe a named RED before production code for every
required branch. The executable accepts only literal `--live`; its live paths,
owner, 490-entry checkpoint and checkpoint digest are hard-coded and it accepts
no caller-provided deletion target.

Before any removal it must reject a wrong canonical path, owner, mode, type,
symlink, directory identity/link count, checkpoint file/count/digest/content,
reviewed helper/digest pair, unsafe filename, or cleanup/final-seal output
collision. It must preserve and later re-hash every non-download checkpoint
entry, exclusively create and fsync the complete sorted JSON cleanup and
conditional/prospective supersession record, invoke only the exact bounded
downloads removal, and prove the target absent. It then creates a deterministic
sorted relative manifest of every remaining regular evidence file excluding
exactly its own
manifest and digest, fsyncs the digest last, and performs no evidence-root
write afterward. Every filesystem/process failure returns a bounded nonzero
result rather than an uncaught exception.

Mutate realistic path/type/owner/mode/checkpoint/digest/source/output-collision
guards, held identities/link counts, the removal target/result, preservation
check, exact live `/usr/bin/rm`, cleanup-record conditional semantics, absence
proof and both seal exclusions. Each mutation must fail its named test. Do not
copy or execute the helper on
`hp-local` in Phase A.

- [ ] **Step 4: Stop for independent Phase-A review**

Run the full helper suite, syntax check, helper mutations, `git diff --check`,
and inspect the complete uncommitted tracked diff. Write the ignored Phase-A
report with the tracked-file summary, guide source/evidence map, exact
RED/GREEN/mutation output, helper/test hashes, validation results, remote
non-mutation proof, and self-review. Request independent read-only review of
the amended design/plan, guide, README, helper, tests and report. Resolve every
verified Critical or Important issue before Phase B.

- [ ] **Step 5: Phase B preflight — verify the historical checkpoint**

Before any remote write, revalidate the current 490-entry pre-cleanup checkpoint
against every current evidence file except exactly its manifest/digest, and
require its manifest SHA-256 to remain
`ad0a3c2626a4c7c85e8a0f04a7f15bffa0fd5affe1d7065c1da8f4b5fd272385`.
Revalidate the exact canonical evidence/download paths, non-symlink directory
types, owner `galchenko`, mode `0700`, and absence of every planned Task-5
source, record and final-seal output. Any mismatch stops before mutation.

- [ ] **Step 6: Install the reviewed helper pair, clean once, and seal last**

Copy the reviewed helper to its hard-coded evidence-root path and exclusively
create its matching digest record, both owner `galchenko`, non-symlink regular
files with link count one and mode `0600`. Revalidate the copied source/digest
against the reviewed local hash. Execute its literal `--live` entry point
exactly once.

The helper ordering is exact: checkpoint/source/output guards; complete
cleanup/supersession record creation and fsync; final target/parent identity
guards; the one semantic operation

```sh
rm -rf -- /home/galchenko/.local/state/unseeing/setup/2026-08-24/downloads
```

then absence proof; complete non-download preservation proof; deterministic
final-manifest creation; final-digest creation and fsync; read-only seal
regeneration and digest verification. The retained historical checkpoint files
remain. No evidence-root write follows the final digest. Do not remove Task 4
build artifacts. If any live result is unexpected, stop without retry or ad
hoc repair and invoke systematic debugging.

- [ ] **Step 7: Complete the factual guide and review the final evidence**

Using read-only verification only, replace the guide's single Phase-A table
with the observed removal verdict, reviewed helper hash, cleanup-record
path/hash, final manifest entry count, total regular-file count and full final
digest. Put the final manifest's exact 64-hex SHA-256 as the first backticked
value in its completion-table result so the destructive evidence rollback can
consume that independently reviewed literal and fail closed while it is
absent. Recheck every installation, build, no-change and rollback fact against
its named evidence source. Request a second independent documentation/evidence
review and resolve every verified Critical or Important issue.

- [ ] **Step 8: Verify and commit the onboarding task**

Run at minimum:

```sh
git diff --check
test/repo_hygiene.sh
```

Stage only the design, plan, MCP-loop document, onboarding guide, README, and
the reviewed MCP guidance/comment/static-assertion corrections in
`tools/setup-mcp.sh`, `.gitignore` and `test/repo_hygiene.sh`. Inspect the
staged diff and staged file list, then make one documentation commit with no
generated output.

### Task 6: Draft and review the wiki write-back

**Files:**
- External modify: `Engineering-Setup.md` in a fresh clone of `https://github.com/cleveralbatraoz/unseeing.wiki.git`
- External modify: `Engineering-Debug-Tooling-Install.md` in the same fresh wiki clone

**Interfaces:**
- Consumes: current wiki master, shipped setup scripts and verified hp-local evidence.
- Produces: one local wiki commit describing current generic setup, safe MCP install/session/uninstall behavior and the dated successful host proof; nothing is pushed.

- [ ] **Step 1: Refresh a clean wiki clone**

Before cloning, require `git ls-remote` to report the canonical
`https://github.com/cleveralbatraoz/unseeing.wiki.git` `refs/heads/master` SHA.
Clone into a new mode-700 `mktemp -d` parent, confirm the clone's `origin`,
`master`, base SHA and clean status, and record that external path. It is a
separate disposable repository, never a worktree of Unseeing.

- [ ] **Step 2: Update only current setup behavior**

Update `Engineering-Setup.md` with any generic correction proven by the run,
the exact four build proof commands and concise 2026-08-24/25 hp-local evidence
notes. Update `Engineering-Debug-Tooling-Install.md` with the exact 4.1.0 pin,
isolated-worktree install, local versus loopback-forward connection, temporary
enable/project restore, one-client rule, generated-state ownership and guarded
uninstall. Point to `docs/hp-local-development-setup.md` for the host ledger and
the tracked MCP-loop document for interactive semantics. Do not copy stale test
counts and do not rewrite unrelated mechanics pages.

- [ ] **Step 3: Review and commit locally**

Run `git diff --check`, request an independent read-only documentation review,
resolve Critical/Important findings, and commit only the two wiki files locally
with the mandated identity and no attribution. Do not push.

- [ ] **Step 4: Retain the wiki clone for the publication choice**

Record the exact clone path, base/head commits and clean status in the handoff.
Do not delete it or push it. Cleanup happens only after the user decides how to
publish or preserve that independent commit.

### Task 7: Final verification, review and branch handoff

**Files:** all changed task files; no new files.

**Interfaces:**
- Consumes: repository commits, wiki draft commit, remote build and actual-editor/MCP evidence, and the original user request.
- Produces: reviewed, reproducible work plus the required user-controlled integration choice.

- [ ] **Step 1: Verify the requirements line by line**

Re-read the spec and plan, inspect the repository and wiki diffs, compare the
onboarding ledger to the captured before/after evidence, regenerate both the
Task-4A MCP seal and Task-5 setup seal read-only, confirm the retained MCP
worktree and durable primary are tracked-clean, and verify no secret or
assistant attribution appears.

- [ ] **Step 2: Transfer the exact handoff tree without a Git push**

Create and verify a local bundle with an explicit branch ref, then transfer it
to a newly created remote directory. Use this command shape, substituting only
the resolved temporary paths and captured SHA values:

```sh
branch=chore/hp-local-development-setup
handoff_head=$(git rev-parse "$branch")
bundle_parent=$(mktemp -d "${TMPDIR:-/tmp}/unseeing-bundle.XXXXXX")
bundle="$bundle_parent/hp-local-development-setup.bundle"
git bundle create "$bundle" "$branch"
git bundle verify "$bundle"
bundle_sha256=$(shasum -a 256 "$bundle" | awk '{print $1}')

remote_parent=$(ssh hp-local '
  if [ ! -e "$HOME/.cache" ]; then mkdir -m 700 "$HOME/.cache"; fi
  test -d "$HOME/.cache"
  test "$(realpath "$HOME/.cache")" = "/home/galchenko/.cache"
  test "$(stat -c %U "$HOME/.cache")" = galchenko
  mktemp -d "$HOME/.cache/unseeing-final.XXXXXX"
')
scp "$bundle" "hp-local:$remote_parent/hp-local-development-setup.bundle"
ssh hp-local "printf '%s  %s\\n' '$bundle_sha256' \
  '$remote_parent/hp-local-development-setup.bundle' | sha256sum --check"
ssh hp-local "git clone --branch '$branch' \
  '$remote_parent/hp-local-development-setup.bundle' '$remote_parent/checkout'"
remote_head=$(ssh hp-local \
  "git -C '$remote_parent/checkout' rev-parse HEAD")
test "$remote_head" = "$handoff_head"
```

Record the local and copied SHA-256 plus `handoff_head`. In the standalone
temporary clone, validate the parent Superpowers metadata before initializing
only `tools/superpowers`. This is not a Git linked worktree, push, merge,
published branch or change to the durable remote `main` checkout. Guard and
remove the local bundle parent after successful transfer, using the same
canonical-path, owner and mode checks applied to remote temporary cleanup.

- [ ] **Step 3: Run fresh final gates on the exact handoff tree**

Before running, verify the completed Task-5 digest and manifest read-only and
prove that no `final/` path or other post-seal output exists in the persistent
dated evidence root. Create a non-overwriting mode-`0700` `verification/`
directory under the already guarded `$remote_parent`; it is the sole remote
owner of Task-7 logs, statuses and manifests. Run the following from the remote
standalone clone with `CARGO_BUILD_JOBS=4` and `~/.cargo/env` sourced. The
implementation
must use a status-preserving wrapper that writes and fsyncs the complete log
and literal status before returning a command's real status; the block below
shows the two semantic commands, not permission to omit that wrapper:

```sh
. "$HOME/.cargo/env"
CARGO_BUILD_JOBS=4 ci/pipeline.sh
CARGO_BUILD_JOBS=4 tools/export_linux.sh \
  "Linux x86_64" build/linux/unseeing
```

Do not set `SKIP_EXPORT` or `SKIP_SMOKE`. Confirm both the temporary clone and
durable `main` checkout remain tracked-clean, then regenerate and hash the
complete raw/compressed Web and Linux artifact manifest. Preserve each
command's real status and full log directly in
`$remote_parent/verification/`; record its tree SHA and hashes in the ignored
local SDD handoff report. Do not copy any Task-7 output into the sealed Task-2/5
evidence root or tracked guide. The guide already records Task 4's full setup
proof, and editing it with Task 7's result would change the exact handoff tree
just verified and create a self-referential verification loop.

- [ ] **Step 4: Request final independent code/documentation review**

Give a read-only reviewer the requirements, base/head SHAs, diff package, and
read-only access to the still-temporary exact-tree logs, statuses and artifact
manifest. Fix every verified Critical or Important issue, rerun affected gates
in a new non-overwriting temporary tree, and commit the fix separately if
required. After review accepts the final run, re-resolve the one remote parent,
require it to match `/home/galchenko/.cache/unseeing-final.*`, owner
`galchenko`, mode `0700`, and the recorded inode, then remove that exact
temporary parent. Record its absence verdict and retained log/manifest hashes
in the ignored local SDD report. No second persistent remote ledger remains;
the durable main checkout's Task 4 artifacts remain.

- [ ] **Step 5: Invoke finishing-a-development-branch**

Detect worktree/base state, report the dirty/stale durable primary checkout if
it still blocks a local merge, and present the exact user choice for repository
integration. Separately report the local wiki commit and request explicit wiki
push authorization; never infer it from the repository choice.
