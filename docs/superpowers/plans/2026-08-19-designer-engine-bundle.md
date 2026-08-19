# Designer Engine Bundle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A designer with no terminal, no Rust toolchain, and no git downloads
one zip per platform from a GitHub Release, extracts it, and opens
`project.godot` in a working Godot editor — closing the half of #38 the
2026-08-11 campaign deliberately deferred.

**Architecture:** A new GitHub Actions workflow (`push: branches: [main]` +
manual `workflow_dispatch`), entirely on GitHub-hosted runners, builds the
release core for five platform targets, packages each into a ready-to-open
bundle via a new testable POSIX-sh script, verifies each packaged bundle by
actually loading it in a headless Godot import + the existing engine census
probe, then — only if every platform succeeded — publishes all five as
assets on a single rolling `engine-latest` GitHub Release.

**Tech Stack:** GitHub Actions (`ubuntu-24.04`, `ubuntu-24.04-arm`,
`windows-latest`, `windows-11-arm`, `macos-latest`), POSIX sh, `gh` CLI,
Rust (pinned toolchain, auto-provisioned from `rust/rust-toolchain.toml`),
Godot 4.7.1.stable.official (headless).

**Spec:** `docs/superpowers/specs/2026-08-19-designer-engine-bundle-design.md`

## Global Constraints

- **Perception laws, label clearance, superface merge law** (`AGENTS.md`):
  inherited standing constraints, not exercised — this plan touches no
  rendering, wave, or audio code.
- **Two code layers** (Law 1: designer-facing = Godot objects; Law 2:
  everything else = Rust): not exercised — this plan adds no designer-facing
  classes and no gameplay Rust. It adds build/release tooling (POSIX sh) and
  CI configuration (YAML), which are developer/CI infrastructure, outside
  both laws' scope.
- **Supported platforms:** macOS universal (arm64+x86_64 fused via `lipo`),
  Windows x86_64 + arm64, Linux x86_64 + arm64. Web/wasm32 is untouched by
  this plan (designer editor bundles are desktop-only; the web export is a
  player-facing artifact `deploy.sh` already ships).
- **Strict TDD:** every behavior change gets a failing test first, watched
  fail for the right reason, then minimal code, then green. No exceptions
  for tooling or CI configuration.
- **Commits:** small, self-contained, green. Evocative narrative subject
  matching repository history, body carrying the precise what/why. Never
  `Co-Authored-By`, `Generated with`, or any assistant attribution anywhere.
  Repository identity: `Dmitrii Galchenko <dggrus@gmail.com>`.
- **Isolation:** already done — this plan executes in the worktree at
  `.claude/worktrees/designer-engine-bundle`, branch `designer-engine-bundle`.
- **Never push to any remote, or merge, without the user's explicit
  go-ahead at that exact point** — this includes pushing this feature
  branch to `origin` purely to let the new workflow run for real (Tasks 2
  and 3 below each hit this and say so explicitly), not only the eventual
  merge to `main`.
- **A new `test/*_test.sh` file is worthless unless wired into
  `ci/pipeline.sh`.** This repository already has one recorded case of a
  test committed and never invoked (`test/macos_universal_test.sh` — see
  its own header comment). Task 1 wires its test in explicitly for this
  reason, not as an afterthought.

---

## File Structure

- `tools/package_engine_bundle.sh` (new) — given an already-built engine
  library and its destination path under `rust/target/`, assembles and zips
  one designer-ready bundle. Builds nothing itself.
- `test/package_engine_bundle_test.sh` (new) — behavioral contract for the
  script above, against a synthetic fixture repo (never the real 300+-file
  `game/` tree).
- `.github/workflows/release-engine.yml` (new) — five platform build jobs +
  one publish job.
- `ci/pipeline.sh` (modify) — wires the new test into the self-test section.
- Wiki page `Engineering-Build-Test-Deploy.md` (external repo, not this
  checkout) — documents the new pipeline.

---

### Task 1: The packaging script

**Files:**
- Create: `tools/package_engine_bundle.sh`
- Create: `test/package_engine_bundle_test.sh`
- Modify: `ci/pipeline.sh` (add self-test invocation)

**Interfaces:**
- Consumes: nothing from earlier tasks (first task). Reuses the
  `DIR="$(cd "$(dirname "$0")/.." && pwd)"` self-location pattern every
  other `tools/*.sh` script in this repo uses (see `tools/bootstrap.sh`,
  `tools/build_macos_core.sh`), and the fixture-copy testing pattern
  `test/bootstrap_posix_test.sh` already established (copy the subject
  script into a synthetic fake repo tree so `$(dirname "$0")/..` resolves
  inside the fixture, never the real checkout).
- Produces: `tools/package_engine_bundle.sh <platform-label>
  <source-library> <dest-relative-path> <output-zip>` — a POSIX-sh
  executable, exit 0 on success, exit 2 on a bad invocation or missing
  input, exit 1 if zip creation fails. Task 2's workflow calls this
  directly, once per platform job, with these exact four positional
  arguments.

- [ ] **Step 1: Write the failing test**

Create `test/package_engine_bundle_test.sh`:

```sh
#!/bin/sh
# Behavioral contract for the designer-bundle packaging script: given an
# already-built library and a git checkout, it must produce a zip whose
# single top-level folder contains the tracked game/ tree, the library at
# the exact rust/target/<dest> path .gdextension expects (a SIBLING of
# game/, not nested inside it -- the bug this test exists to catch), and an
# ENGINE_COMMIT file naming the real commit. Runs against a synthetic
# fixture repo, never the real 300+-file game/ tree, so it stays fast and
# is not a change detector on real content.
set -eu
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUBJECT="${PACKAGE_BUNDLE_SUBJECT:-$ROOT/tools/package_engine_bundle.sh}"
FAIL=0

ok() { echo "package-bundle: OK   $1"; }
bad() { echo "package-bundle: FAIL $1"; FAIL=1; }
require() {
  label="$1"
  shift
  if "$@"; then ok "$label"; else bad "$label"; fi
}

command -v git >/dev/null 2>&1 || { echo "package-bundle: SKIP git not found"; exit 0; }
command -v python3 >/dev/null 2>&1 || { echo "package-bundle: SKIP python3 not found"; exit 0; }
command -v unzip >/dev/null 2>&1 || { echo "package-bundle: SKIP unzip not found"; exit 0; }

T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM
REPO="$T/repo"
mkdir -p "$REPO/tools" "$REPO/game/scenes" "$REPO/rust"
cp "$SUBJECT" "$REPO/tools/package_engine_bundle.sh"
chmod +x "$REPO/tools/package_engine_bundle.sh"
printf 'config_version=5\n' >"$REPO/game/project.godot"
printf 'fixture level content\n' >"$REPO/game/scenes/level_01.tscn"
# Only game/ is ever archived, so a file outside it proves the script does
# not silently ship the whole checkout.
printf 'not shipped\n' >"$REPO/rust/should-not-appear.txt"

( cd "$REPO" \
  && git init -q \
  && git config user.email "fixture@example.invalid" \
  && git config user.name "fixture" \
  && git add game \
  && git commit -q -m "fixture game tree" )
COMMIT="$(cd "$REPO" && git rev-parse HEAD)"

printf 'fake engine library\n' >"$T/libunseeing_core.so"
OUT_ZIP="$T/out/unseeing-editor-linux-x86_64.zip"

STATUS=0
"$REPO/tools/package_engine_bundle.sh" linux-x86_64 "$T/libunseeing_core.so" \
  release/libunseeing_core.so "$OUT_ZIP" >"$T/run.log" 2>&1 || STATUS=$?
require "the script exits 0 on a valid invocation" test "$STATUS" -eq 0
[ "$STATUS" -eq 0 ] || sed 's/^/package-bundle:      /' "$T/run.log"

require "the zip was created" test -s "$OUT_ZIP"

EXTRACT="$T/extract"
mkdir -p "$EXTRACT"
unzip -q "$OUT_ZIP" -d "$EXTRACT"

require "the zip has exactly one top-level entry" \
  test "$(find "$EXTRACT" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')" = "1"
require "the top-level entry is named unseeing-editor-linux-x86_64" \
  test -d "$EXTRACT/unseeing-editor-linux-x86_64"

BUNDLE="$EXTRACT/unseeing-editor-linux-x86_64"
require "game/project.godot is present" test -f "$BUNDLE/game/project.godot"
require "game/scenes/level_01.tscn is present" \
  test -f "$BUNDLE/game/scenes/level_01.tscn"
require "the library sits at rust/target/release/libunseeing_core.so" \
  test -f "$BUNDLE/rust/target/release/libunseeing_core.so"
# THE bug this test exists to catch: unseeing.gdextension resolves
# res://../rust/target/..., one level ABOVE game/ -- so rust/ must be a
# SIBLING of game/, never nested inside it.
require "rust/ is a sibling of game/, not nested inside it" \
  test ! -e "$BUNDLE/game/rust"
require "the shipped library is the one that was given, byte for byte" \
  cmp -s "$BUNDLE/rust/target/release/libunseeing_core.so" "$T/libunseeing_core.so"
require "ENGINE_COMMIT names the real commit" \
  test "$(cat "$BUNDLE/ENGINE_COMMIT")" = "$COMMIT"
require "rust/should-not-appear.txt is not shipped (only game/ is archived)" \
  test ! -e "$BUNDLE/rust/should-not-appear.txt"
require "no .git reaches the bundle" test ! -e "$BUNDLE/.git"

# Preconditions: a missing source library must refuse before touching disk.
STATUS=0
"$REPO/tools/package_engine_bundle.sh" linux-x86_64 "$T/does-not-exist.so" \
  release/libunseeing_core.so "$T/out/should-not-exist.zip" >"$T/refuse.log" 2>&1 || STATUS=$?
require "a missing source library refuses with exit 2" test "$STATUS" -eq 2
require "the refusal names the missing file" \
  grep -q "does-not-exist.so" "$T/refuse.log"
require "no zip was written on refusal" test ! -e "$T/out/should-not-exist.zip"

exit "$FAIL"
```

```bash
chmod +x test/package_engine_bundle_test.sh
```

- [ ] **Step 2: Run it to verify it fails for the right reason**

```bash
./test/package_engine_bundle_test.sh
```

Expected: fails at the `cp "$SUBJECT" ...` line (or immediately after) with
something like `cp: cannot stat '.../tools/package_engine_bundle.sh': No
such file or directory` — the subject does not exist yet. This is the
correct failure; anything else (a passing run, or a failure inside the
assertions) means the test fixture itself is broken and must be fixed
before continuing.

- [ ] **Step 3: Write the implementation**

Create `tools/package_engine_bundle.sh`:

```sh
#!/bin/sh
# Assembles one designer-facing editor bundle: the tracked game/ tree plus a
# single already-built engine library, laid out exactly as
# game/unseeing.gdextension expects it (the library one level ABOVE game/,
# under rust/target/<dest-relative-path>), wrapped in one named top-level
# folder so extracting the zip never splatters files into wherever the
# designer unzipped it.
#
# Usage: package_engine_bundle.sh <platform-label> <source-library> <dest-relative-path> <output-zip>
#   platform-label      e.g. linux-x86_64 -- becomes the wrapping folder
#                        name "unseeing-editor-<platform-label>"
#   source-library       path to the already-built release library (this
#                        script never builds anything)
#   dest-relative-path   where that library lands under rust/target/, e.g.
#                        "release/libunseeing_core.so" or
#                        "aarch64-pc-windows-msvc/release/unseeing_core.dll"
#                        -- must match a key in game/unseeing.gdextension
#   output-zip            where to write the finished zip
#
# Env knobs: none. The commit stamped into ENGINE_COMMIT is always
# `git rev-parse HEAD` of the checkout this runs in -- there is exactly one
# honest answer to "which commit was this built from" and it is not a
# parameter.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"

[ $# -eq 4 ] || {
  echo "package-engine-bundle: usage: package_engine_bundle.sh <platform-label> <source-library> <dest-relative-path> <output-zip>"
  exit 2
}
PLATFORM="$1"
SRC_LIB="$2"
DEST_REL="$3"
OUT_ZIP="$4"
case "$OUT_ZIP" in
  /*) : ;;
  *) OUT_ZIP="$(pwd)/$OUT_ZIP" ;;
esac

[ -s "$SRC_LIB" ] || {
  echo "package-engine-bundle: FAILED source library missing or empty: $SRC_LIB"
  exit 2
}
command -v git >/dev/null 2>&1 || {
  echo "package-engine-bundle: FAILED git not found"
  exit 2
}
# Not the `zip` CLI: it is not reliably present on GitHub's windows-latest
# hosted runner (only 7z is guaranteed there, and even that has shipped
# with gaps). python3's stdlib zipfile module is already relied on
# elsewhere in this same release pipeline (the Godot-fetch steps extract
# with it), so it is already proven present on every runner this script
# needs to run on -- Linux, Windows (via `shell: bash`, which still runs
# whatever python3 the runner has on PATH), and macOS.
command -v python3 >/dev/null 2>&1 || {
  echo "package-engine-bundle: FAILED python3 not found (used to create the zip portably)"
  exit 2
}
COMMIT="$(cd "$DIR" && git rev-parse HEAD 2>/dev/null)" || {
  echo "package-engine-bundle: FAILED cannot resolve HEAD -- is $DIR a git checkout?"
  exit 2
}

ROOT="unseeing-editor-$PLATFORM"
T="$(mktemp -d)"
trap 'rm -rf "$T"' EXIT INT TERM
STAGE="$T/$ROOT"
mkdir -p "$STAGE"

echo "package-engine-bundle: archiving the tracked game/ tree"
( cd "$DIR" && git archive HEAD -- game ) | ( cd "$STAGE" && tar -x )
[ -f "$STAGE/game/project.godot" ] || {
  echo "package-engine-bundle: FAILED git archive did not produce game/project.godot"
  exit 1
}

echo "package-engine-bundle: placing the engine library at rust/target/$DEST_REL"
LIB_DEST="$STAGE/rust/target/$DEST_REL"
mkdir -p "$(dirname "$LIB_DEST")"
cp "$SRC_LIB" "$LIB_DEST"

printf '%s\n' "$COMMIT" > "$STAGE/ENGINE_COMMIT"

mkdir -p "$(dirname "$OUT_ZIP")"
rm -f "$OUT_ZIP"
python3 -c '
import os, sys, zipfile
stage, root, out_zip = sys.argv[1], sys.argv[2], sys.argv[3]
with zipfile.ZipFile(out_zip, "w", zipfile.ZIP_DEFLATED) as zf:
    for dirpath, _dirnames, filenames in os.walk(os.path.join(stage, root)):
        for name in filenames:
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, stage)
            zf.write(full, rel)
' "$T" "$ROOT" "$OUT_ZIP" || {
  echo "package-engine-bundle: FAILED could not create $OUT_ZIP"
  exit 1
}
echo "package-engine-bundle: OK   $OUT_ZIP ($(du -k "$OUT_ZIP" | cut -f1) KB)"
```

```bash
chmod +x tools/package_engine_bundle.sh
```

- [ ] **Step 4: Run the test again to verify it passes**

```bash
./test/package_engine_bundle_test.sh
```

Expected: every `OK` line, exit 0.

- [ ] **Step 5: Mutation-check the implementation**

Confirm each of these, one at a time, makes the test fail (then revert it):
- Change `git archive HEAD -- game` to `git archive HEAD` (whole repo) →
  `rust/should-not-appear.txt is not shipped` must fail.
- Remove the `mkdir -p "$STAGE"` + zip-from-`$T` structure so the zip is
  created from inside `$STAGE` itself instead of `$T` → `the zip has
  exactly one top-level entry` / `named unseeing-editor-linux-x86_64` must
  fail.
- Change `printf '%s\n' "$COMMIT"` to a hardcoded string → `ENGINE_COMMIT
  names the real commit` must fail.
- Remove the `[ -s "$SRC_LIB" ]` precondition → the missing-library case
  must stop refusing with exit 2 (it will instead fail later, or worse,
  succeed by copying nothing meaningful).

- [ ] **Step 6: Wire the test into the pipeline gate**

In `ci/pipeline.sh`, immediately after the existing `test/bootstrap_posix_test.sh`
invocation (around line 80-81, in the "self-tests for gates further down"
section), add:

```sh
echo "ci: designer engine-bundle packaging self-test"
"$DIR/test/package_engine_bundle_test.sh" || exit 1
```

- [ ] **Step 7: Run the full checks-only pipeline to confirm nothing broke**

```bash
GODOT=/home/albatraoz/bin/godot SKIP_EXPORT=1 ci/pipeline.sh
```

Expected: `ci: designer engine-bundle packaging self-test` prints its `OK`
lines, and the run still ends `ci: OK`.

- [ ] **Step 8: Commit**

```bash
git add tools/package_engine_bundle.sh test/package_engine_bundle_test.sh ci/pipeline.sh
git commit -m "<narrative subject in house style, e.g. describing that a bundle now assembles itself from a checkout and a library>"
```

---

### Task 2: The release-engine workflow

**Files:**
- Create: `.github/workflows/release-engine.yml`

**Interfaces:**
- Consumes: `tools/package_engine_bundle.sh <platform-label> <source-library>
  <dest-relative-path> <output-zip>` from Task 1. `tools/build_macos_core.sh`
  (existing, unmodified) for the macOS universal slice-and-lipo build.
  `game/tests/probe/engine_census_probe.gd`, invoked exactly as
  `tools/bootstrap.sh` and `ci/pipeline.sh` already do:
  `"$GODOT" --headless --path <project> --import` (exit ignored) then
  `"$GODOT" --headless --path <project> -s res://tests/probe/engine_census_probe.gd`
  (exit 0 = every class registered, nonzero = failure).
- Produces: five zip artifacts named `unseeing-editor-linux-x86_64`,
  `unseeing-editor-linux-arm64`, `unseeing-editor-windows-x86_64`,
  `unseeing-editor-windows-arm64`, `unseeing-editor-macos-universal`,
  published as assets on a rolling `engine-latest` prerelease. Later tasks
  (3, 4) consume that published release, not this workflow's internals.

Verified download URLs used below (checked live against the pinned
`4.7.1-stable` release before writing this plan — `curl -sIL` returned 200
for every one):
- Linux x86_64: `https://github.com/godotengine/godot/releases/download/${V}-stable/Godot_v${V}-stable_linux.x86_64.zip` → single file `Godot_v${V}-stable_linux.x86_64`
- Linux arm64: `https://github.com/godotengine/godot/releases/download/${V}-stable/Godot_v${V}-stable_linux.arm64.zip` → single file `Godot_v${V}-stable_linux.arm64`
- Windows x86_64: `https://github.com/godotengine/godot-builds/releases/download/${V}-stable/Godot_v${V}-stable_win64.exe.zip` → `Godot_v${V}-stable_win64.exe` + `Godot_v${V}-stable_win64_console.exe`
- Windows arm64: `https://github.com/godotengine/godot-builds/releases/download/${V}-stable/Godot_v${V}-stable_windows_arm64.exe.zip` → `Godot_v${V}-stable_windows_arm64.exe` + `Godot_v${V}-stable_windows_arm64_console.exe`
- macOS universal: `https://github.com/godotengine/godot/releases/download/${V}-stable/Godot_v${V}-stable_macos.universal.zip` → `Godot.app/Contents/MacOS/Godot`

- [ ] **Step 1: Write the workflow file**

Create `.github/workflows/release-engine.yml`:

```yaml
name: release-engine
on:
  push:
    branches: [main]
  workflow_dispatch: {}
concurrency:
  group: release-engine-main
  cancel-in-progress: false
permissions:
  contents: write

jobs:
  linux-x86_64:
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
          submodules: false
      - name: Read pinned Godot version
        id: godot
        run: echo "version=$(cut -d. -f1-3 .godot-version)" >> "$GITHUB_OUTPUT"
      - name: Cache Godot
        id: cache-godot
        uses: actions/cache@v4
        with:
          path: godot-bin
          key: godot-${{ steps.godot.outputs.version }}-linux-x86_64
      - name: Fetch Godot
        if: steps.cache-godot.outputs.cache-hit != 'true'
        run: |
          V="${{ steps.godot.outputs.version }}"
          curl -sL -o godot.zip "https://github.com/godotengine/godot/releases/download/${V}-stable/Godot_v${V}-stable_linux.x86_64.zip"
          python3 -m zipfile -e godot.zip .
          mkdir -p godot-bin
          mv "Godot_v${V}-stable_linux.x86_64" godot-bin/godot
          chmod +x godot-bin/godot
      - name: Cache cargo + rust target
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            rust/target
          key: release-rust-linux-x86_64-${{ hashFiles('rust/Cargo.lock', 'rust/rust-toolchain.toml') }}
      - name: Build the release core
        run: (cd rust && cargo build --release)
      - name: Package the designer bundle
        run: |
          mkdir -p bundle
          tools/package_engine_bundle.sh linux-x86_64 \
            rust/target/release/libunseeing_core.so \
            release/libunseeing_core.so \
            bundle/unseeing-editor-linux-x86_64.zip
      - name: Verify the packaged bundle loads (census)
        run: |
          mkdir -p verify
          unzip -q bundle/unseeing-editor-linux-x86_64.zip -d verify
          godot-bin/godot --headless --path verify/unseeing-editor-linux-x86_64/game --import >/dev/null 2>&1 || true
          godot-bin/godot --headless --path verify/unseeing-editor-linux-x86_64/game \
            -s res://tests/probe/engine_census_probe.gd
      - uses: actions/upload-artifact@v4
        with:
          name: unseeing-editor-linux-x86_64
          path: bundle/unseeing-editor-linux-x86_64.zip
          retention-days: 1

  linux-arm64:
    runs-on: ubuntu-24.04-arm
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
          submodules: false
      - name: Read pinned Godot version
        id: godot
        run: echo "version=$(cut -d. -f1-3 .godot-version)" >> "$GITHUB_OUTPUT"
      - name: Cache Godot
        id: cache-godot
        uses: actions/cache@v4
        with:
          path: godot-bin
          key: godot-${{ steps.godot.outputs.version }}-linux-arm64
      - name: Fetch Godot
        if: steps.cache-godot.outputs.cache-hit != 'true'
        run: |
          V="${{ steps.godot.outputs.version }}"
          curl -sL -o godot.zip "https://github.com/godotengine/godot/releases/download/${V}-stable/Godot_v${V}-stable_linux.arm64.zip"
          python3 -m zipfile -e godot.zip .
          mkdir -p godot-bin
          mv "Godot_v${V}-stable_linux.arm64" godot-bin/godot
          chmod +x godot-bin/godot
      - name: Cache cargo + rust target
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            rust/target
          key: release-rust-linux-arm64-${{ hashFiles('rust/Cargo.lock', 'rust/rust-toolchain.toml') }}
      - name: Build the release core
        run: (cd rust && cargo build --release)
      - name: Package the designer bundle
        run: |
          mkdir -p bundle
          tools/package_engine_bundle.sh linux-arm64 \
            rust/target/release/libunseeing_core.so \
            release/libunseeing_core.so \
            bundle/unseeing-editor-linux-arm64.zip
      - name: Verify the packaged bundle loads (census)
        run: |
          mkdir -p verify
          unzip -q bundle/unseeing-editor-linux-arm64.zip -d verify
          godot-bin/godot --headless --path verify/unseeing-editor-linux-arm64/game --import >/dev/null 2>&1 || true
          godot-bin/godot --headless --path verify/unseeing-editor-linux-arm64/game \
            -s res://tests/probe/engine_census_probe.gd
      - uses: actions/upload-artifact@v4
        with:
          name: unseeing-editor-linux-arm64
          path: bundle/unseeing-editor-linux-arm64.zip
          retention-days: 1

  windows-x86_64:
    runs-on: windows-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
          submodules: false
      - name: Read pinned Godot version
        id: godot
        shell: powershell
        run: |
          $pin = (Get-Content .godot-version -Raw).Trim()
          $version = (($pin -split '\.')[0..2] -join '.')
          $utf8 = New-Object System.Text.UTF8Encoding($false)
          [IO.File]::AppendAllText($env:GITHUB_OUTPUT, "version=$version`n", $utf8)
      - name: Cache Godot
        id: cache-godot
        uses: actions/cache@v4
        with:
          path: godot-bin
          key: godot-${{ steps.godot.outputs.version }}-windows-x86_64
      - name: Fetch Godot
        if: steps.cache-godot.outputs.cache-hit != 'true'
        shell: powershell
        run: |
          $version = "${{ steps.godot.outputs.version }}"
          $url = "https://github.com/godotengine/godot-builds/releases/download/$version-stable/Godot_v$version-stable_win64.exe.zip"
          Invoke-WebRequest -Uri $url -OutFile godot.zip
          Expand-Archive -LiteralPath godot.zip -DestinationPath godot-bin
      - name: Cache cargo + rust target
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            rust/target
          key: release-rust-windows-x86_64-${{ hashFiles('rust/Cargo.lock', 'rust/rust-toolchain.toml') }}
      - name: Build the release core
        shell: powershell
        run: |
          Push-Location rust
          cargo build --release --target x86_64-pc-windows-msvc
          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
          Pop-Location
      - name: Package the designer bundle
        shell: bash
        run: |
          mkdir -p bundle
          tools/package_engine_bundle.sh windows-x86_64 \
            rust/target/x86_64-pc-windows-msvc/release/unseeing_core.dll \
            x86_64-pc-windows-msvc/release/unseeing_core.dll \
            bundle/unseeing-editor-windows-x86_64.zip
      - name: Verify the packaged bundle loads (census)
        shell: powershell
        run: |
          $godotConsole = (Get-ChildItem godot-bin -Filter '*_console.exe' -Recurse | Select-Object -First 1).FullName
          if (-not $godotConsole) { throw 'Pinned Godot console executable was not found' }
          Expand-Archive -LiteralPath bundle/unseeing-editor-windows-x86_64.zip -DestinationPath verify
          & $godotConsole --headless --path verify/unseeing-editor-windows-x86_64/game --import
          & $godotConsole --headless --path verify/unseeing-editor-windows-x86_64/game -s res://tests/probe/engine_census_probe.gd
          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
      - uses: actions/upload-artifact@v4
        with:
          name: unseeing-editor-windows-x86_64
          path: bundle/unseeing-editor-windows-x86_64.zip
          retention-days: 1

  windows-arm64:
    runs-on: windows-11-arm
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
          submodules: false
      - name: Read pinned Godot version
        id: godot
        shell: powershell
        run: |
          $pin = (Get-Content .godot-version -Raw).Trim()
          $version = (($pin -split '\.')[0..2] -join '.')
          $utf8 = New-Object System.Text.UTF8Encoding($false)
          [IO.File]::AppendAllText($env:GITHUB_OUTPUT, "version=$version`n", $utf8)
      - name: Cache Godot
        id: cache-godot
        uses: actions/cache@v4
        with:
          path: godot-bin
          key: godot-${{ steps.godot.outputs.version }}-windows-arm64
      - name: Fetch Godot
        if: steps.cache-godot.outputs.cache-hit != 'true'
        shell: powershell
        run: |
          $version = "${{ steps.godot.outputs.version }}"
          $url = "https://github.com/godotengine/godot-builds/releases/download/$version-stable/Godot_v$version-stable_windows_arm64.exe.zip"
          Invoke-WebRequest -Uri $url -OutFile godot.zip
          Expand-Archive -LiteralPath godot.zip -DestinationPath godot-bin
      - name: Cache cargo + rust target
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            rust/target
          key: release-rust-windows-arm64-${{ hashFiles('rust/Cargo.lock', 'rust/rust-toolchain.toml') }}
      - name: Build the release core
        shell: powershell
        run: |
          Push-Location rust
          cargo build --release --target aarch64-pc-windows-msvc
          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
          Pop-Location
      - name: Package the designer bundle
        shell: bash
        run: |
          mkdir -p bundle
          tools/package_engine_bundle.sh windows-arm64 \
            rust/target/aarch64-pc-windows-msvc/release/unseeing_core.dll \
            aarch64-pc-windows-msvc/release/unseeing_core.dll \
            bundle/unseeing-editor-windows-arm64.zip
      - name: Verify the packaged bundle loads (census)
        shell: powershell
        run: |
          $godotConsole = (Get-ChildItem godot-bin -Filter '*_console.exe' -Recurse | Select-Object -First 1).FullName
          if (-not $godotConsole) { throw 'Pinned Godot console executable was not found' }
          Expand-Archive -LiteralPath bundle/unseeing-editor-windows-arm64.zip -DestinationPath verify
          & $godotConsole --headless --path verify/unseeing-editor-windows-arm64/game --import
          & $godotConsole --headless --path verify/unseeing-editor-windows-arm64/game -s res://tests/probe/engine_census_probe.gd
          if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
      - uses: actions/upload-artifact@v4
        with:
          name: unseeing-editor-windows-arm64
          path: bundle/unseeing-editor-windows-arm64.zip
          retention-days: 1

  macos-universal:
    runs-on: macos-latest
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
          submodules: false
      - name: Read pinned Godot version
        id: godot
        run: echo "version=$(cut -d. -f1-3 .godot-version)" >> "$GITHUB_OUTPUT"
      - name: Cache Godot
        id: cache-godot
        uses: actions/cache@v4
        with:
          path: godot-bin
          key: godot-${{ steps.godot.outputs.version }}-macos-universal
      - name: Fetch Godot
        if: steps.cache-godot.outputs.cache-hit != 'true'
        run: |
          V="${{ steps.godot.outputs.version }}"
          curl -sL -o godot.zip "https://github.com/godotengine/godot/releases/download/${V}-stable/Godot_v${V}-stable_macos.universal.zip"
          mkdir -p godot-bin
          python3 -m zipfile -e godot.zip godot-bin
          # zipfile's permission preservation is not trusted anywhere else in
          # this repo's CI either -- the Linux job re-asserts +x explicitly
          # right after extraction rather than relying on it; same here.
          chmod +x godot-bin/Godot.app/Contents/MacOS/Godot
      - name: Cache cargo + rust target
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            rust/target
          key: release-rust-macos-universal-${{ hashFiles('rust/Cargo.lock', 'rust/rust-toolchain.toml') }}
      - name: Build the universal release core
        run: tools/build_macos_core.sh
      - name: Package the designer bundle
        run: |
          mkdir -p bundle
          tools/package_engine_bundle.sh macos-universal \
            rust/target/release/libunseeing_core.dylib \
            release/libunseeing_core.dylib \
            bundle/unseeing-editor-macos-universal.zip
      - name: Verify the packaged bundle loads (census)
        run: |
          mkdir -p verify
          unzip -q bundle/unseeing-editor-macos-universal.zip -d verify
          GODOT=godot-bin/Godot.app/Contents/MacOS/Godot
          "$GODOT" --headless --path verify/unseeing-editor-macos-universal/game --import >/dev/null 2>&1 || true
          "$GODOT" --headless --path verify/unseeing-editor-macos-universal/game \
            -s res://tests/probe/engine_census_probe.gd
      - uses: actions/upload-artifact@v4
        with:
          name: unseeing-editor-macos-universal
          path: bundle/unseeing-editor-macos-universal.zip
          retention-days: 1

  publish:
    needs: [linux-x86_64, linux-arm64, windows-x86_64, windows-arm64, macos-universal]
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
          submodules: false
      - uses: actions/download-artifact@v4
        with:
          path: bundles
      - name: Publish to the rolling engine-latest release
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          set -eu
          SHA="${{ github.sha }}"
          SHORT="$(printf %.9s "$SHA")"
          if ! gh release view engine-latest --repo "${{ github.repository }}" >/dev/null 2>&1; then
            gh release create engine-latest --repo "${{ github.repository }}" \
              --prerelease --title "Designer engine bundles (rolling)" \
              --notes "Always matches main's HEAD. Currently built from commit $SHORT."
          else
            gh release edit engine-latest --repo "${{ github.repository }}" \
              --notes "Always matches main's HEAD. Currently built from commit $SHORT."
          fi
          gh release upload engine-latest --repo "${{ github.repository }}" --clobber \
            bundles/unseeing-editor-linux-x86_64/unseeing-editor-linux-x86_64.zip \
            bundles/unseeing-editor-linux-arm64/unseeing-editor-linux-arm64.zip \
            bundles/unseeing-editor-windows-x86_64/unseeing-editor-windows-x86_64.zip \
            bundles/unseeing-editor-windows-arm64/unseeing-editor-windows-arm64.zip \
            bundles/unseeing-editor-macos-universal/unseeing-editor-macos-universal.zip
```

- [ ] **Step 2: Validate YAML syntax locally**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release-engine.yml'))" && echo "valid YAML"
```

Expected: `valid YAML`. This only proves the file parses — GitHub Actions'
own semantics (valid runner labels, job graph, action versions) can only be
proven by an actual run, which Step 5 does.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-engine.yml
git commit -m "<narrative subject in house style, e.g. describing that every push now proves a designer's binary loads>"
```

- [ ] **Step 4: Ask the user before pushing**

This is the point where the branch must reach `origin` for the workflow to
run for real — five cloud runners, one of them a brand-new `macos-latest`
job this repository has never used before. **Stop and ask the user for
explicit confirmation before running `git push`.** Do not push
autonomously.

- [ ] **Step 5: Push and dispatch a real run**

Once confirmed:

```bash
git push -u origin designer-engine-bundle
gh workflow run release-engine.yml --ref designer-engine-bundle
gh run watch "$(gh run list --workflow release-engine.yml --branch designer-engine-bundle --limit 1 --json databaseId --jq '.[0].databaseId')"
```

Expected: all six jobs (`linux-x86_64`, `linux-arm64`, `windows-x86_64`,
`windows-arm64`, `macos-universal`, `publish`) complete successfully.

- [ ] **Step 6: Confirm the release actually carries five assets**

```bash
gh release view engine-latest --json assets --jq '.assets[].name'
```

Expected: exactly `unseeing-editor-linux-x86_64.zip`,
`unseeing-editor-linux-arm64.zip`, `unseeing-editor-windows-x86_64.zip`,
`unseeing-editor-windows-arm64.zip`, `unseeing-editor-macos-universal.zip`.

- [ ] **Step 7: If any job failed, fix and re-verify**

Diagnose from the failing job's log (`gh run view --log-failed`), fix the
workflow or the script it calls, commit, push, and repeat Steps 5-6 until
every job is green. Common first-run failure modes to expect: a runner
label typo, a Godot download URL 404 (re-verify with `curl -sIL` against
the exact pinned `.godot-version`, not assumed), or a Windows path
separator issue in a `shell: bash` step.

---

### Task 3: Wiki write-back

**Files:**
- External: `Engineering-Build-Test-Deploy.md` in the repository's GitHub
  wiki (`git@github.com:cleveralbatraoz/unseeing.wiki.git`), not this
  checkout.

**Interfaces:**
- Consumes: the now-published `engine-latest` release from Task 2, to
  describe accurately (not aspirationally).
- Produces: an updated wiki page. No other task depends on this one.

- [ ] **Step 1: Clone the wiki fresh**

```bash
git clone git@github.com:cleveralbatraoz/unseeing.wiki.git /tmp/unseeing-wiki-writeback
```

- [ ] **Step 2: Add a new section to `Engineering-Build-Test-Deploy.md`**

Insert a new `## 9. The designer engine bundle` section (after the existing
`## 8. Movie-maker determinism` section) describing, as fact rather than
plan: what `.github/workflows/release-engine.yml` builds, on which
GitHub-hosted runners (naming the exact runner labels, since GitHub's arm64
runner naming has already moved once), the `game/` + sibling `rust/target/`
bundle layout and why (`res://../rust/target/...`), the rolling
`engine-latest` prerelease and its `--clobber` semantics, the per-platform
census verification, and that `tools/bootstrap.sh`/`.cmd` remain the
separate path for contributors with a toolchain. Reference
`docs/superpowers/specs/2026-08-19-designer-engine-bundle-design.md` by
name for the full rationale.

- [ ] **Step 3: Commit in the wiki repo**

```bash
cd /tmp/unseeing-wiki-writeback
git add Engineering-Build-Test-Deploy.md
git commit -m "Document the designer engine bundle pipeline"
```

- [ ] **Step 4: Ask the user before pushing**

The wiki is a separate, publicly-visible repository. **Stop and ask the
user for explicit confirmation before running `git push`** here too.

- [ ] **Step 5: Push once confirmed**

```bash
git push
```

---

### Task 4: End-to-end acceptance — reproduce the original issue's fix

**Files:** none created or modified — this is a verification-only task,
mirroring the exact reproduction already performed against issue #38 during
its investigation, now expected to succeed instead of fail.

**Interfaces:**
- Consumes: the published `unseeing-editor-linux-x86_64.zip` asset from the
  `engine-latest` release (Task 2).
- Produces: a pass/fail verdict recorded in the task's own commit-adjacent
  notes (no code artifact — this task's only output is confidence that #38
  is actually fixed).

- [ ] **Step 1: Download the published Linux bundle, exactly as a designer would**

```bash
mkdir -p /tmp/designer-acceptance && cd /tmp/designer-acceptance
gh release download engine-latest --repo cleveralbatraoz/unseeing \
  --pattern 'unseeing-editor-linux-x86_64.zip'
unzip -q unseeing-editor-linux-x86_64.zip
```

- [ ] **Step 2: Confirm the layout is exactly what the gdextension needs**

```bash
ls unseeing-editor-linux-x86_64/
test -f unseeing-editor-linux-x86_64/game/project.godot && echo "game/project.godot: OK"
test -f unseeing-editor-linux-x86_64/rust/target/release/libunseeing_core.so && echo "engine library: OK"
cat unseeing-editor-linux-x86_64/ENGINE_COMMIT
```

- [ ] **Step 3: Open it in the real, pinned Godot editor headlessly, and confirm real geometry loads (not `MissingNode`)**

This mirrors the exact original reproduction from the issue investigation,
against `tools/probe_editor_level.sh`'s own probe, but this time run
against the downloaded bundle instead of a clean `git archive` with no
`rust/target`:

```bash
cd unseeing-editor-linux-x86_64
/home/albatraoz/bin/godot --headless --editor --path game --quit-after 60 \
  2>&1 | grep -iE "Can't open|Cannot get class|MissingNode" || echo "NO ENGINE-LOAD ERRORS"
/home/albatraoz/bin/godot --headless --path game -s res://tests/probe/engine_census_probe.gd
```

Expected: `NO ENGINE-LOAD ERRORS`, and the census probe reports `probe:
PASS (19 checks)`, exit 0 — the exact opposite of the original
reproduction's `ERROR: Can't open dynamic library` /
`Cannot get class 'WaveWall'` cascade.

- [ ] **Step 4: Report the verdict to the user**

State plainly whether Task 4 passed or failed, and if it failed, treat that
as a new bug in the workflow (return to Task 2, do not paper over it).

---

## After all tasks

Present the finish-branch choice to the user per `AGENTS.md` — this plan
does not merge, and does not run `deploy.sh`, on its own authority.
