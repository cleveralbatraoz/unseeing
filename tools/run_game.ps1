[CmdletBinding()]
param(
    [string]$Godot = "",
    [switch]$Windowed,
    [string]$Geometry = "1280x720",
    [string]$Scene = "",
    [string]$Seed = "",
    [switch]$Demo,
    [switch]$SkipBuild,
    [ValidateSet("Auto", "X64", "Arm64")]
    [string]$Architecture = "Auto",
    [string]$RepositoryRoot = "",
    # NOT named -GodotArguments: -Godot is a prefix of it, and a remaining-
    # arguments parameter swallows the ambiguous -Godot along with its value,
    # so an explicitly named editor silently never arrived.
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Passthrough = @()
)

# Build the engine and play the GAME — not the editor. The POSIX half is
# tools/run_game.sh; this is the same contract on Windows.
#
# Engine discovery, the pinned-version predicate, the PE architecture read and
# the streaming subprocess runner all belong to tools/bootstrap.ps1 already, so
# they are borrowed rather than written twice: -NoRun defines its functions
# without running any of it.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
    (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
} else {
    try { (Resolve-Path -LiteralPath $RepositoryRoot).Path }
    catch { Write-Output "run-game: FAILED repository root '$RepositoryRoot' does not exist"; exit 2 }
}

# Dot-sourcing runs bootstrap.ps1's own param() block IN THIS SCOPE, so every
# parameter it shares a name with — Godot, Architecture, RepositoryRoot — is
# reset to its default the moment the functions are borrowed. Keep what was
# asked for first; an explicitly named editor was otherwise thrown away and the
# tool went hunting for one instead.
$RequestedGodot = $Godot
$RequestedArchitecture = $Architecture

. (Join-Path $PSScriptRoot "bootstrap.ps1") -NoRun

# Borrowed code refuses through Stop-Bootstrap, and a message reading
# "bootstrap: FAILED" from a command nobody called bootstrap sends the reader to
# the wrong script. Same behaviour, this tool's name on it.
function Stop-Bootstrap([int]$Code, [string]$Message) {
    Write-Output "run-game: FAILED $Message"
    exit $Code
}

$VersionFile = Join-Path $Root ".godot-version"
$RustDirectory = Join-Path $Root "rust"
$GameDirectory = Join-Path $Root "game"
if (-not (Test-Path -LiteralPath $VersionFile -PathType Leaf) -or
    -not (Test-Path -LiteralPath $GameDirectory -PathType Container)) {
    Stop-Bootstrap 2 "'$Root' is not an Unseeing checkout"
}
$Want = (Get-Content -LiteralPath $VersionFile -Raw).Trim()
if ([string]::IsNullOrWhiteSpace($Want)) {
    Stop-Bootstrap 2 ".godot-version is blank; it must carry the pinned Godot release"
}

if ($Geometry -notmatch '^[0-9]+x[0-9]+$') {
    Stop-Bootstrap 2 "-Geometry must look like 1280x720 (got '$Geometry')"
}
if (-not [string]::IsNullOrWhiteSpace($Seed) -and $Seed -notmatch '^[0-9]+$') {
    Stop-Bootstrap 2 "-Seed must be a whole number (got '$Seed')"
}

# The engine gate first, for the same reason the bootstrap does it first: it
# costs milliseconds, and rebuilding the core for an editor that will be
# refused afterwards is time spent for nothing.
Write-Output "run-game: locating Godot"
$GodotPath = Find-Godot $RequestedGodot $Root $Want
Write-Output "run-game: godot OK ($(Get-GodotVersion $GodotPath))"

$Target = Get-WindowsTarget $GodotPath $RequestedArchitecture
$Artifact = Join-Path $RustDirectory "target\$Target\release\unseeing_core.dll"

if (-not $SkipBuild) {
    $RustupPath = Find-Rustup ""
    if ($null -eq $RustupPath) {
        Stop-Bootstrap 2 "rustup not found - run tools\bootstrap.cmd first, it installs the toolchain"
    }
    $toolchainFile = Join-Path $RustDirectory "rust-toolchain.toml"
    $pinMatch = [regex]::Match((Get-Content -LiteralPath $toolchainFile -Raw), '(?m)^\s*channel\s*=\s*"([^"]+)"')
    if (-not $pinMatch.Success) {
        Stop-Bootstrap 2 "rust/rust-toolchain.toml carries no channel pin"
    }
    $RustPin = $pinMatch.Groups[1].Value
    Write-Output "run-game: building the engine ($Target)"
    Push-Location $RustDirectory
    try {
        # editor-docs matches what tools\bootstrap.cmd builds into this same
        # path, so alternating between the editor and the game does not rebuild
        # the world each time. Streamed, so a cold build is watchable.
        $build = Invoke-Streamed $RustupPath @(
            "run", $RustPin, "cargo", "build", "--release", "--features", "editor-docs",
            "--target", $Target, "--target-dir", (Join-Path $RustDirectory "target")
        )
    } finally {
        Pop-Location
    }
    if ($build.ExitCode -ne 0) {
        Stop-Bootstrap 1 "rust build failed (exit $($build.ExitCode); see output above)"
    }
}

if (-not (Test-Path -LiteralPath $Artifact -PathType Leaf)) {
    Write-Output "run-game: drop -SkipBuild, or run tools\bootstrap.cmd"
    Stop-Bootstrap 1 "no engine core at '$Artifact'"
}

# The game boots full screen because the PROJECT says so, and Godot's own window
# flags lose to the project setting. override.cfg is the documented escape
# hatch, merged over project.godot before the window exists. Written before the
# launch and removed however this script ends — the finally block covers a
# failure and Ctrl-C alike, because the repository forbids shipping this file.
$Override = Join-Path $GameDirectory "override.cfg"
if ($Windowed -and (Test-Path -LiteralPath $Override)) {
    Write-Output "run-game:        remove it if it is a leftover, or wait for the run that owns it to finish."
    Stop-Bootstrap 2 "game\override.cfg already exists - a windowed run would overwrite and then delete it."
}

$exitCode = 0
try {
    if ($Windowed) {
        $size = $Geometry -split "x"
        Set-Content -LiteralPath $Override -Encoding ascii -Value @(
            "[display]"
            ""
            "window/size/mode=0"
            "window/size/viewport_width=$($size[0])"
            "window/size/viewport_height=$($size[1])"
        )
    }

    # After any build, never before: a failed extension load is recorded in
    # .godot/extension_list.cfg at import time and never retried, so a play that
    # runs first gets a world with no engine classes in it at all.
    Invoke-Captured $GodotPath @("--headless", "--path", $GameDirectory, "--import") | Out-Null

    if (-not [string]::IsNullOrWhiteSpace($Seed)) { $env:UNSEEING_SEED = $Seed }
    if ($Demo) { $env:UNSEEING_DEMO = "1" }

    $launch = New-Object System.Collections.Generic.List[string]
    [void]$launch.Add("--path")
    [void]$launch.Add($GameDirectory)
    foreach ($extra in $Passthrough) { [void]$launch.Add($extra) }
    if (-not [string]::IsNullOrWhiteSpace($Scene)) { [void]$launch.Add($Scene) }

    # Built up rather than interpolated: Windows PowerShell 5.1 cannot parse a
    # double-quoted string containing $( ... ) that itself contains a
    # double-quoted string, and the whole file fails to load if it tries.
    $announce = "run-game: playing"
    if (-not [string]::IsNullOrWhiteSpace($Scene)) { $announce += " $Scene" }
    if ($Windowed) { $announce += " (windowed $Geometry)" }
    Write-Output $announce
    # No -e and no --editor: this is the world, not the authoring environment.
    & $GodotPath @launch
    $exitCode = $LASTEXITCODE
} finally {
    if ($Windowed) {
        Remove-Item -LiteralPath $Override -Force -ErrorAction SilentlyContinue
    }
}
exit $exitCode
