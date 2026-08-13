# Cross-Platform Designer Bootstrap — Design

**Status:** approved by the user's request for one authoring bootstrap across
macOS, Linux, and Windows.

## Goal

A fresh desktop checkout must reach the same working Godot editor boundary on
all supported operating systems: the pinned Rust engine is built as a release
artifact with Inspector documentation, the exact pinned Godot editor imports
the project, and the 19-class census proves the GDExtension loaded before the
script says it is safe to author a level.

This is developer tooling only. It does not change the Godot/Rust content
boundary, any shipped artifact, or deployment.

## Decision

Keep native entry points instead of replacing the workflow with Python:

- `tools/bootstrap.sh` remains the POSIX entry point for macOS and Linux.
- `tools/bootstrap.cmd` is the one-command Windows entry point and delegates
  to `tools/bootstrap.ps1` with a process-local execution-policy bypass.
- `tools/bootstrap.ps1` owns Windows architecture detection and the real work.

Python is already present in some contributor tooling, but it is not guaranteed
on a new Windows designer machine. Requiring it in order to install the Rust
engine would move the bootstrap gap instead of closing it. Windows PowerShell
is the native control plane for supported Windows versions, while CMD provides
an entry point that works without changing a user's execution policy.

## Parity contract

Both implementations must:

1. locate or install rustup without requiring a new terminal;
2. use `rust/rust-toolchain.toml`, including its exact compiler pin;
3. build `--release --features editor-docs` into the path declared by
   `game/unseeing.gdextension`;
4. accept an explicit Godot executable and otherwise discover one locally;
5. reject any Godot version whose output does not begin with the complete
   `.godot-version` value;
6. import only after the release library exists;
7. run `engine_census_probe.gd` and propagate a nonzero verdict;
8. print `bootstrap: OK` only after all 19 classes pass.

Preflight/discovery failures use exit 2. A failed build (including a missing
MSVC component discovered by Cargo) or census uses exit 1.
The import command may return nonzero because its only purpose is to warm the
cache; the following census is the authoritative load verdict.

## Windows boundary

Windows supports the two architectures already declared by the project:

| Godot editor architecture | Cargo target | GDExtension artifact |
| --- | --- | --- |
| x86_64 | `x86_64-pc-windows-msvc` | `rust/target/x86_64-pc-windows-msvc/release/unseeing_core.dll` |
| ARM64 | `aarch64-pc-windows-msvc` | `rust/target/aarch64-pc-windows-msvc/release/unseeing_core.dll` |

Other Windows architectures are refused before download or build. When rustup
is absent, the script downloads the architecture-matched official
`rustup-init.exe` from Rust's static distribution, removes the temporary
installer in a `finally` block, refreshes the current process's Cargo path, and
then installs/selects the exact toolchain pin and target. Rust's MSVC linker and Windows SDK remain required;
build failure names the Visual Studio Build Tools remedy.

Godot discovery precedence is an explicit `-Godot` argument, `GODOT`, direct
Scoop/WinGet/program locations, then commands on `PATH`. Direct Scoop discovery
avoids inspecting the architecture of its launcher shim. Paths are always
invoked as values so spaces are safe.

## Verification

- A PowerShell boundary test runs the production script against recording
  rustup/Godot fakes. It proves x86_64 and ARM64 target selection, release
  editor-docs arguments, import-before-census ordering, exact pin refusal, and
  propagation of build/census failures. An installer fake also starts with no
  discoverable rustup and proves current-process PATH refresh.
- A Windows GitHub Actions job downloads the pinned official Godot editor and
  runs the real `tools/bootstrap.cmd`, proving the x86_64 release DLL loads and
  all 19 classes register on an actual Windows host.
- ARM64 target selection, standard-library installation, and artifact location
  are boundary-tested, and Windows CI cross-checks the Rust engine for ARM64.
  An actual ARM64 editor load remains unverified until an ARM64 Windows runner
  is available.
- The POSIX boundary suite independently starts with no discoverable rustup,
  crosses its installer boundary, and proves current-process discovery. The
  real POSIX bootstrap runs locally and remains in the ordinary Linux pipeline
  contract.

The tests inject executable paths and an architecture only at the process
boundary. Production derives the DLL target from the actual Godot PE, not the
host OS, so an emulated editor receives a matching extension architecture.
