# hp-local Development Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provision `hp-local` reproducibly, prove Unseeing's native, Web and Linux-player builds, and publish a complete tracked onboarding/change ledger.

**Architecture:** Treat the repository's pinned bootstrap and pipeline as the behavioral authority, keep host dependencies at system or user scope according to their owner, and build one clean, commit-pinned clone whose sole submodule is verified before initialization. Store no generated artifact in Git; preserve only the design, plan, onboarding guide and README link in the isolated task worktree, while drafting the separate wiki update without pushing it.

**Tech Stack:** Debian 13, POSIX shell, Git, Godot 4.7.1, rustup/Rust, gdtoolkit, Emscripten 4.0.20, Chromium, Brotli, Godot GDExtension and Web export.

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

### Task 5: Write the complete onboarding and change ledger

**Files:**
- Create: `docs/hp-local-development-setup.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: factual before/after package and filesystem evidence plus fresh build results from Tasks 2–4.
- Produces: a self-contained fresh-host procedure, rollback guide and dated proof record discoverable from the root README.

- [ ] **Step 1: Write the guide from observed evidence**

Include host role and baseline SHA, pre-existing inventory, exact persistent
changes, apt dependency transaction, user-level installs, repository-local
settings, removed temporary state, ignored artifacts, unperformed optional
steps, four build gates, artifact hashes, daily workflow, update workflow,
troubleshooting, security, rollback and GitHub-auth limitation. Name the file
owning every quoted pin.

- [ ] **Step 2: Add one README link**

Under `README.md` Setup, link the hp-local guide without duplicating its host
transcript or changing generic setup commands.

- [ ] **Step 3: Review the onboarding diff before committing**

Request independent read-only review of the guide and README against the spec,
the retained host evidence and actual four build logs. Resolve every verified
Critical or Important issue.

- [ ] **Step 4: Remove only download scratch**

Resolve the recorded download directory with `realpath`, require it to equal
`/home/galchenko/.local/state/unseeing/setup/2026-08-24/downloads`, require
owner `galchenko` and mode `700`, then remove that one exact directory and
confirm it is absent. The evidence directory, its manifests and their hashes
remain. Do not remove build artifacts; they remain useful and ignored.

The removal command, after those guards, is exactly:

```sh
rm -rf -- /home/galchenko/.local/state/unseeing/setup/2026-08-24/downloads
```

- [ ] **Step 5: Verify and commit the onboarding task**

Run at minimum:

```sh
git diff --check
test/repo_hygiene.sh
```

Inspect the staged diff and staged file list, then make one documentation
commit with no generated output.

### Task 6: Draft and review the wiki write-back

**Files:**
- External modify: `Engineering-Setup.md` in a fresh clone of `https://github.com/cleveralbatraoz/unseeing.wiki.git`

**Interfaces:**
- Consumes: current wiki master, shipped setup scripts and verified hp-local evidence.
- Produces: one local wiki commit describing current generic setup and the dated successful host proof; nothing is pushed.

- [ ] **Step 1: Refresh a clean wiki clone**

Before cloning, require `git ls-remote` to report the canonical
`https://github.com/cleveralbatraoz/unseeing.wiki.git` `refs/heads/master` SHA.
Clone into a new mode-700 `mktemp -d` parent, confirm the clone's `origin`,
`master`, base SHA and clean status, and record that external path. It is a
separate disposable repository, never a worktree of Unseeing.

- [ ] **Step 2: Update only current setup behavior**

Update `Engineering-Setup.md` with any generic correction proven by the run,
the exact four proof commands and a concise 2026-08-24 hp-local evidence note.
Point to `docs/hp-local-development-setup.md` for the host transcript. Do not
copy stale test counts and do not rewrite unrelated mechanics pages.

- [ ] **Step 3: Review and commit locally**

Run `git diff --check`, request an independent read-only documentation review,
resolve Critical/Important findings, and commit the single wiki file locally
with the mandated identity and no attribution. Do not push.

- [ ] **Step 4: Retain the wiki clone for the publication choice**

Record the exact clone path, base/head commits and clean status in the handoff.
Do not delete it or push it. Cleanup happens only after the user decides how to
publish or preserve that independent commit.

### Task 7: Final verification, review and branch handoff

**Files:** all changed task files; no new files.

**Interfaces:**
- Consumes: repository commits, wiki draft commit, remote build evidence and the original user request.
- Produces: reviewed, reproducible work plus the required user-controlled integration choice.

- [ ] **Step 1: Verify the requirements line by line**

Re-read the spec and plan, inspect the repository and wiki diffs, compare the
onboarding ledger to the captured before/after evidence, and verify no secret or
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

From that remote standalone clone run, with `CARGO_BUILD_JOBS=4` and
`~/.cargo/env` sourced:

```sh
. "$HOME/.cargo/env"
CARGO_BUILD_JOBS=4 ci/pipeline.sh
CARGO_BUILD_JOBS=4 tools/export_linux.sh \
  "Linux x86_64" build/linux/unseeing
```

Do not set `SKIP_EXPORT` or `SKIP_SMOKE`. Confirm both the temporary clone and
durable `main` checkout remain tracked-clean, then regenerate and hash the
complete raw/compressed Web and Linux artifact manifest. Copy the final logs,
exit statuses, tree SHA and manifest into a new, non-overwriting `final/`
child of the persistent dated evidence directory. Keep this final rerun
evidence in the host ledger and cite it in the handoff; do not copy it back
into the tracked guide. The guide already records Task 4's full setup proof,
and editing it with Task 7's result would change the exact handoff tree just
verified and create a self-referential verification loop.

After the evidence is copied into the persistent host ledger, re-resolve the temporary
verification directory, require it to match
`/home/galchenko/.cache/unseeing-final.*`, require owner `galchenko` and mode
`700`, and remove that one exact directory with `rm -rf -- "$resolved"`.
Record its removal. The durable main checkout's Task 4 artifacts remain.

- [ ] **Step 4: Request final independent code/documentation review**

Give a read-only reviewer the requirements, base/head SHAs and a diff package.
Fix every verified Critical or Important issue, rerun affected gates, and
commit the fix separately if required.

- [ ] **Step 5: Invoke finishing-a-development-branch**

Detect worktree/base state, report the dirty/stale durable primary checkout if
it still blocks a local merge, and present the exact user choice for repository
integration. Separately report the local wiki commit and request explicit wiki
push authorization; never infer it from the repository choice.
