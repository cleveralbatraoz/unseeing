[CmdletBinding()]
param(
    [string]$Godot = "",
    [string]$Rustup = "",
    [string]$RustupInstaller = "",
    [ValidateSet("Auto", "X64", "Arm64")]
    [string]$Architecture = "Auto",
    [string]$RepositoryRoot = "",
    [switch]$NoRun
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Stop-Bootstrap([int]$Code, [string]$Message) {
    Write-Output "bootstrap: FAILED $Message"
    exit $Code
}

function Resolve-CommandPath([string]$Candidate) {
    if ([string]::IsNullOrWhiteSpace($Candidate)) {
        return $null
    }
    if (Test-Path -LiteralPath $Candidate -PathType Leaf) {
        return (Resolve-Path -LiteralPath $Candidate).Path
    }
    $command = Get-Command $Candidate -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -eq $command) {
        return $null
    }
    if (-not [string]::IsNullOrWhiteSpace($command.Path)) {
        return $command.Path
    }
    return $command.Source
}

function Prefer-ConsoleGodot([string]$Path) {
    if (-not $Path.EndsWith(".exe", [StringComparison]::OrdinalIgnoreCase)) {
        return $Path
    }
    if ($Path.EndsWith("_console.exe", [StringComparison]::OrdinalIgnoreCase) -or
        $Path.EndsWith(".console.exe", [StringComparison]::OrdinalIgnoreCase)) {
        return $Path
    }
    $directory = Split-Path -Parent $Path
    $stem = [IO.Path]::GetFileNameWithoutExtension($Path)
    $console = Join-Path $directory ($stem + "_console.exe")
    if (Test-Path -LiteralPath $console -PathType Leaf) {
        return (Resolve-Path -LiteralPath $console).Path
    }
    $dotConsole = Join-Path $directory ($stem + ".console.exe")
    if (Test-Path -LiteralPath $dotConsole -PathType Leaf) {
        return (Resolve-Path -LiteralPath $dotConsole).Path
    }
    return $Path
}

function Find-Godot([string]$Requested, [string]$Root) {
    if ([string]::IsNullOrWhiteSpace($Requested)) {
        $Requested = $env:GODOT
    }
    $resolved = Resolve-CommandPath $Requested
    if ($null -ne $resolved) {
        return Prefer-ConsoleGodot $resolved
    }
    if (-not [string]::IsNullOrWhiteSpace($Requested)) {
        Stop-Bootstrap 2 "godot not found at '$Requested'"
    }

    $candidates = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($env:SCOOP)) {
        [void]$candidates.Add((Join-Path $env:SCOOP "apps\godot\current\godot.console.exe"))
    }
    [void]$candidates.Add((Join-Path $HOME "scoop\apps\godot\current\godot.console.exe"))
    [void]$candidates.Add((Join-Path $Root "godot-bin\godot.exe"))
    [void]$candidates.Add((Join-Path $HOME "bin\godot.exe"))
    if (-not [string]::IsNullOrWhiteSpace($env:SCOOP)) {
        [void]$candidates.Add((Join-Path $env:SCOOP "apps\godot\current\godot.exe"))
    }
    [void]$candidates.Add((Join-Path $HOME "scoop\apps\godot\current\godot.exe"))
    if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        [void]$candidates.Add((Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links\godot.exe"))
        [void]$candidates.Add((Join-Path $env:LOCALAPPDATA "Programs\Godot\Godot.exe"))
    }
    foreach ($candidate in $candidates) {
        $resolved = Resolve-CommandPath $candidate
        if ($null -ne $resolved) {
            return Prefer-ConsoleGodot $resolved
        }
    }

    foreach ($name in @("godot.console.exe", "godot_console.exe", "godot.exe", "godot")) {
        $resolved = Resolve-CommandPath $name
        if ($null -ne $resolved) {
            return Prefer-ConsoleGodot $resolved
        }
    }

    Stop-Bootstrap 2 (
        "godot not found; install the pinned standard editor (for example, " +
        "'scoop install godot'), put it on PATH, or run " +
        "'tools\bootstrap.cmd -Godot C:\path\to\Godot_console.exe'"
    )
}

function Read-PeArchitecture([string]$Path) {
    $stream = $null
    $reader = $null
    try {
        $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
        if ($stream.Length -lt 64) {
            Stop-Bootstrap 2 "Godot executable '$Path' is too short to be a Windows PE file"
        }
        $reader = New-Object IO.BinaryReader($stream)
        if ($reader.ReadUInt16() -ne 0x5A4D) {
            Stop-Bootstrap 2 "Godot executable '$Path' has no MZ header"
        }
        $stream.Position = 0x3C
        $peOffset = $reader.ReadInt32()
        if ($peOffset -lt 0 -or ($peOffset + 6) -gt $stream.Length) {
            Stop-Bootstrap 2 "Godot executable '$Path' has an invalid PE header offset"
        }
        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            Stop-Bootstrap 2 "Godot executable '$Path' has no PE signature"
        }
        $machine = $reader.ReadUInt16()
        switch ($machine) {
            0x8664 { return "X64" }
            0xAA64 { return "Arm64" }
            default {
                Stop-Bootstrap 2 ("Godot executable '$Path' uses unsupported PE machine 0x{0:X4}" -f $machine)
            }
        }
    } catch {
        Stop-Bootstrap 2 "cannot inspect Godot executable '$Path': $($_.Exception.Message)"
    } finally {
        if ($null -ne $reader) {
            $reader.Dispose()
        } elseif ($null -ne $stream) {
            $stream.Dispose()
        }
    }
}

function Get-WindowsTarget([string]$GodotPath, [string]$RequestedArchitecture) {
    $selected = $RequestedArchitecture
    if ($selected -eq "Auto") {
        if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
            Stop-Bootstrap 2 "automatic architecture detection requires Windows"
        }
        $selected = Read-PeArchitecture $GodotPath
    }
    switch ($selected) {
        "X64" { return "x86_64-pc-windows-msvc" }
        "Arm64" { return "aarch64-pc-windows-msvc" }
        default { Stop-Bootstrap 2 "unsupported Windows architecture '$selected'" }
    }
}

function Get-RustupHost() {
    $native = $env:PROCESSOR_ARCHITEW6432
    if ([string]::IsNullOrWhiteSpace($native)) {
        $native = $env:PROCESSOR_ARCHITECTURE
    }
    switch -Regex ($native) {
        "^(AMD64|x86_64)$" { return "x86_64-pc-windows-msvc" }
        "^(ARM64|aarch64)$" { return "aarch64-pc-windows-msvc" }
        default { Stop-Bootstrap 2 "unsupported Windows host architecture '$native'" }
    }
}

function Find-Rustup([string]$Requested) {
    $resolved = Resolve-CommandPath $Requested
    if ($null -ne $resolved) {
        return $resolved
    }
    if (-not [string]::IsNullOrWhiteSpace($Requested)) {
        Stop-Bootstrap 2 "rustup not found at '$Requested'"
    }
    $resolved = Resolve-CommandPath "rustup.exe"
    if ($null -eq $resolved) {
        $resolved = Resolve-CommandPath "rustup"
    }
    if ($null -ne $resolved) {
        return $resolved
    }
    $cargoHome = $env:CARGO_HOME
    if ([string]::IsNullOrWhiteSpace($cargoHome)) {
        $cargoHome = Join-Path $HOME ".cargo"
    }
    return Resolve-CommandPath (Join-Path $cargoHome "bin\rustup.exe")
}

function Install-Rustup([string]$InstallerOverride) {
    if (-not [string]::IsNullOrWhiteSpace($InstallerOverride)) {
        & $InstallerOverride
        if ($LASTEXITCODE -ne 0) {
            Stop-Bootstrap 2 "rustup installer override failed (exit $LASTEXITCODE)"
        }
    } elseif ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        Stop-Bootstrap 2 "automatic rustup installation requires Windows"
    } else {
        $hostTriple = Get-RustupHost
        $temporary = Join-Path ([IO.Path]::GetTempPath()) ("unseeing-rustup-{0}" -f [guid]::NewGuid())
        $installer = Join-Path $temporary "rustup-init.exe"
        New-Item -ItemType Directory -Path $temporary | Out-Null
        try {
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
            $url = "https://static.rust-lang.org/rustup/dist/$hostTriple/rustup-init.exe"
            Write-Output "bootstrap: rustup not found - downloading official rustup-init for $hostTriple"
            Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $installer
            & $installer -y --profile minimal --default-toolchain none
            if ($LASTEXITCODE -ne 0) {
                Stop-Bootstrap 2 "rustup installation failed (exit $LASTEXITCODE)"
            }
        } catch {
            Stop-Bootstrap 2 "rustup installation failed: $($_.Exception.Message)"
        } finally {
            Remove-Item -LiteralPath $temporary -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    $cargoHome = $env:CARGO_HOME
    if ([string]::IsNullOrWhiteSpace($cargoHome)) {
        $cargoHome = Join-Path $HOME ".cargo"
    }
    $cargoBin = Join-Path $cargoHome "bin"
    $env:Path = $cargoBin + [IO.Path]::PathSeparator + $env:Path
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        # Keep the injectable installer boundary runnable under pwsh in the
        # macOS/Linux developer suites, whose environment names are case-sensitive.
        $env:PATH = $env:Path
    }
}

function Invoke-Captured([string]$Executable, [string[]]$Arguments) {
    $global:LASTEXITCODE = 0
    $previousErrorAction = $ErrorActionPreference
    try {
        # Windows PowerShell 5.1 turns ordinary native stderr into
        # NativeCommandError records when Stop is active. Cargo writes progress
        # to stderr on success, so capture it under Continue and trust the real
        # native exit code below.
        $ErrorActionPreference = "Continue"
        $lines = @(& $Executable @Arguments 2>&1 | ForEach-Object { $_.ToString() })
        $nativeExit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorAction
    }
    return @{
        ExitCode = $nativeExit
        Lines = $lines
    }
}

if ($NoRun) {
    return
}

if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
    $RepositoryRoot = Join-Path $PSScriptRoot ".."
}
try {
    $Root = (Resolve-Path -LiteralPath $RepositoryRoot).Path
} catch {
    Stop-Bootstrap 2 "repository root '$RepositoryRoot' does not exist"
}
$RustDirectory = Join-Path $Root "rust"
$GameDirectory = Join-Path $Root "game"
$VersionFile = Join-Path $Root ".godot-version"
$ToolchainFile = Join-Path $RustDirectory "rust-toolchain.toml"
if (-not (Test-Path -LiteralPath $RustDirectory -PathType Container) -or
    -not (Test-Path -LiteralPath $GameDirectory -PathType Container) -or
    -not (Test-Path -LiteralPath $VersionFile -PathType Leaf) -or
    -not (Test-Path -LiteralPath $ToolchainFile -PathType Leaf)) {
    Stop-Bootstrap 2 "'$Root' is not an Unseeing checkout"
}

Write-Output "bootstrap: locating Godot"
$GodotPath = Find-Godot $Godot $Root
$version = Invoke-Captured $GodotPath @("--version")
if ($version.ExitCode -ne 0) {
    Stop-Bootstrap 2 "Godot --version failed (exit $($version.ExitCode))"
}
$Have = $version.Lines | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1
$Want = (Get-Content -LiteralPath $VersionFile -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($Have) -or
    -not $Have.StartsWith($Want, [StringComparison]::Ordinal)) {
    Stop-Bootstrap 2 "godot version '$Have' != pinned '$Want'"
}
Write-Output "bootstrap: godot OK ($Have)"

$Target = Get-WindowsTarget $GodotPath $Architecture
Write-Output "bootstrap: Windows target $Target"

Write-Output "bootstrap: checking for rustup/cargo"
$RustupPath = if ([string]::IsNullOrWhiteSpace($RustupInstaller)) {
    Find-Rustup $Rustup
} else {
    # Supplying an installer is the deterministic boundary-test seam. Normal
    # designer runs leave it empty and reuse any rustup already present.
    $null
}
if ($null -eq $RustupPath) {
    Install-Rustup $RustupInstaller
    $RustupPath = Find-Rustup ""
}
if ($null -eq $RustupPath) {
    Stop-Bootstrap 2 (
        "the installer did not leave usable rustup in the current process; install Rust from " +
        "https://rustup.rs and rerun tools\bootstrap.cmd"
    )
}
$toolchainText = Get-Content -LiteralPath $ToolchainFile -Raw
$pinMatch = [regex]::Match($toolchainText, '(?m)^\s*channel\s*=\s*"([^"]+)"')
if (-not $pinMatch.Success) {
    Stop-Bootstrap 2 "rust/rust-toolchain.toml carries no channel pin"
}
$RustPin = $pinMatch.Groups[1].Value
Push-Location $RustDirectory
try {
    $rustcVersion = Invoke-Captured $RustupPath @("run", $RustPin, "rustc", "--version")
    if ($rustcVersion.ExitCode -ne 0) {
        Write-Output "bootstrap: installing pinned Rust $RustPin toolchain"
        $installToolchain = Invoke-Captured $RustupPath @(
            "toolchain", "install", $RustPin, "--profile", "minimal"
        )
        foreach ($line in $installToolchain.Lines) {
            Write-Output $line
        }
        if ($installToolchain.ExitCode -ne 0) {
            Stop-Bootstrap 2 "pinned Rust $RustPin toolchain install failed"
        }
        $rustcVersion = Invoke-Captured $RustupPath @("run", $RustPin, "rustc", "--version")
    }
    $targetInstall = Invoke-Captured $RustupPath @(
        "target", "add", $Target, "--toolchain", $RustPin
    )
    $cargoVersion = Invoke-Captured $RustupPath @("run", $RustPin, "cargo", "--version")
} finally {
    Pop-Location
}
if ($rustcVersion.ExitCode -ne 0) {
    Stop-Bootstrap 2 "rustup could not select pinned Rust $RustPin"
}
$RustcHave = $rustcVersion.Lines | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1
if ([string]::IsNullOrWhiteSpace($RustcHave) -or
    -not $RustcHave.StartsWith("rustc $RustPin ", [StringComparison]::Ordinal)) {
    Stop-Bootstrap 2 "rustc version '$RustcHave' != pinned '$RustPin'"
}
if ($targetInstall.ExitCode -ne 0) {
    Stop-Bootstrap 2 "Rust target $Target install failed"
}
if ($cargoVersion.ExitCode -ne 0) {
    Stop-Bootstrap 2 "cargo is unavailable in pinned Rust $RustPin"
}
$CargoHave = $cargoVersion.Lines | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -First 1
Write-Output "bootstrap: rustup/cargo OK ($RustcHave; $CargoHave)"

Write-Output "bootstrap: building the engine (cargo build --release --features editor-docs --target $Target)"
$Artifact = Join-Path $RustDirectory "target\$Target\release\unseeing_core.dll"
try {
    Remove-Item -LiteralPath $Artifact -Force -ErrorAction Stop
} catch [System.Management.Automation.ItemNotFoundException] {
    # A fresh checkout has nothing to remove.
} catch {
    Stop-Bootstrap 1 "cannot remove stale '$Artifact'; close every Godot process and retry"
}
if (Test-Path -LiteralPath $Artifact) {
    Stop-Bootstrap 1 "stale '$Artifact' survived deletion; close every Godot process and retry"
}
Push-Location $RustDirectory
try {
    $build = Invoke-Captured $RustupPath @(
        "run", $RustPin, "cargo", "build", "--release", "--features", "editor-docs", "--target", $Target,
        "--target-dir", (Join-Path $RustDirectory "target")
    )
} finally {
    Pop-Location
}
foreach ($line in $build.Lines) {
    Write-Output $line
}
if ($build.ExitCode -ne 0) {
    Write-Output "bootstrap: fix: close Godot, then install Visual Studio 2022 Build Tools with"
    Write-Output "bootstrap: fix: Desktop development with C++, the Windows SDK, and this target's C++ tools"
    Stop-Bootstrap 1 "rust build failed (exit $($build.ExitCode); see output above)"
}
if (-not (Test-Path -LiteralPath $Artifact -PathType Leaf)) {
    Stop-Bootstrap 1 "cargo reported success but '$Artifact' does not exist"
}
Write-Output "bootstrap: engine built ($Artifact)"

Write-Output "bootstrap: importing the project"
$import = Invoke-Captured $GodotPath @("--headless", "--path", $GameDirectory, "--import")
if ($import.ExitCode -ne 0) {
    Write-Output "bootstrap: import exited $($import.ExitCode); continuing to the authoritative class census"
}

Write-Output "bootstrap: verifying every engine class registered"
$census = Invoke-Captured $GodotPath @(
    "--headless", "--path", $GameDirectory,
    "-s", "res://tests/probe/engine_census_probe.gd"
)
foreach ($line in $census.Lines) {
    Write-Output $line
}
if ($census.ExitCode -ne 0) {
    Stop-Bootstrap 1 "the engine census probe failed (exit $($census.ExitCode); see output above)"
}
if (-not (($census.Lines -join "`n").Contains("probe: PASS (19 checks)"))) {
    Stop-Bootstrap 1 "the engine census returned success without the exact 19-class verdict"
}

Write-Output "bootstrap: OK - open game/project.godot in Godot $Want"
exit 0
