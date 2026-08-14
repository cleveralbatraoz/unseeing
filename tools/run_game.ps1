# NO param() block, deliberately. PowerShell resolves a leading `--` the same as
# `-`, so a declared -Scene parameter swallows `--scene`, -Seed swallows
# `--seed`, -Windowed swallows `--windowed` — and `--scene --demo` dies with
# "Missing an argument for parameter 'Scene'" before a line of this file runs.
# Declaring the parameters is precisely what made the POSIX spellings
# unreachable. Parsing $args by hand is what lets both spellings mean the same
# thing, which is what README.md and tools/run_game.sh promise Windows.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Stop-Run([int]$Code, [string]$Message) {
    Write-Output "run-game: FAILED $Message"
    exit $Code
}
function Show-Usage() {
    Write-Output "usage: tools\run_game.cmd [--windowed [WxH]] [--scene res://path.tscn]"
    Write-Output "                          [--seed <n>] [--demo] [--skip-build]"
    Write-Output "                          [-Godot <path>] [-- <godot args>]"
}

$RequestedGodot = ""
$RequestedArchitecture = "Auto"
$RepositoryRoot = ""
$Geometry = "1280x720"
$Scene = ""
$Seed = ""
$UseWindow = $false
$SkipTheBuild = $false
$DemoMode = $false
$Extra = New-Object System.Collections.Generic.List[string]

$tokens = @($args)
$index = 0
$verbatim = $false
while ($index -lt $tokens.Count) {
    $token = [string]$tokens[$index]
    if ($verbatim) { [void]$Extra.Add($token); $index++; continue }
    $value = $null
    $needsValue = $token -match '^(--scene|-Scene|--seed|-Seed|--geometry|-Geometry|--godot|-Godot|--repository-root|-RepositoryRoot|--architecture|-Architecture)$'
    if ($needsValue) {
        if ($index + 1 -ge $tokens.Count) { Stop-Run 2 "$token needs a value" }
        $index++
        $value = [string]$tokens[$index]
    }
    switch -Regex ($token) {
        '^--$' { $verbatim = $true }
        '^(--windowed|-Windowed)$' {
            $UseWindow = $true
            # Only consume the next token when it IS a size, so `--windowed
            # --demo` does not lose --demo, and `--windowed 1280x720p` is not
            # split blindly into a non-numeric viewport.
            if ($index + 1 -lt $tokens.Count -and ([string]$tokens[$index + 1]) -match '^[0-9]+x[0-9]+$') {
                $Geometry = [string]$tokens[$index + 1]
                $index++
            }
        }
        '^(--skip-build|-SkipBuild)$' { $SkipTheBuild = $true }
        '^(--demo|-Demo)$' { $DemoMode = $true }
        '^(--scene|-Scene)$' {
            if ($value.StartsWith("-")) { Stop-Run 2 "--scene needs a res:// path" }
            $Scene = $value
        }
        '^(--seed|-Seed)$' {
            if ($value -notmatch '^[0-9]+$') { Stop-Run 2 "--seed needs a whole number" }
            $Seed = $value
        }
        '^(--geometry|-Geometry)$' { $Geometry = $value }
        '^(--godot|-Godot)$' { $RequestedGodot = $value }
        '^(--repository-root|-RepositoryRoot)$' { $RepositoryRoot = $value }
        '^(--architecture|-Architecture)$' { $RequestedArchitecture = $value }
        '^(-h|--help|-Help)$' { Show-Usage; exit 0 }
        default { [void]$Extra.Add($token) }
    }
    $index++
}

if (@("Auto", "X64", "Arm64") -notcontains $RequestedArchitecture) {
    Stop-Run 2 "the architecture must be Auto, X64 or Arm64 (got '$RequestedArchitecture')"
}
if ($Geometry -notmatch '^[0-9]+x[0-9]+$') {
    Stop-Run 2 "the window size must look like 1280x720 (got '$Geometry')"
}

$Root = if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
    (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
} else {
    try { (Resolve-Path -LiteralPath $RepositoryRoot).Path }
    catch { Stop-Run 2 "repository root '$RepositoryRoot' does not exist" }
}

# Build the engine and play the GAME - not the editor. The POSIX half is
# tools/run_game.sh; this is the same contract on Windows.
#
# Engine discovery, the pinned-version predicate, the PE architecture read and
# the streaming subprocess runner all belong to tools/bootstrap.ps1 already, so
# they are borrowed rather than written twice: -NoRun defines its functions
# without running any of it. Dot-sourcing also executes bootstrap's own param()
# block in this scope, which is a second reason nothing above shares its names.
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

# The engine gate first, for the same reason the bootstrap does it first: it
# costs milliseconds, and rebuilding the core for an editor that will be
# refused afterwards is time spent for nothing.
Write-Output "run-game: locating Godot"
$GodotPath = Find-Godot $RequestedGodot $Root $Want
Write-Output "run-game: godot OK ($(Get-GodotVersion $GodotPath))"

$Target = Get-WindowsTarget $GodotPath $RequestedArchitecture
$Artifact = Join-Path $RustDirectory "target\$Target\release\unseeing_core.dll"

if (-not $SkipTheBuild) {
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
if ($UseWindow -and (Test-Path -LiteralPath $Override)) {
    Write-Output "run-game:        remove it if it is a leftover, or wait for the run that owns it to finish."
    Stop-Bootstrap 2 "game\override.cfg already exists - a windowed run would overwrite and then delete it."
}

$exitCode = 0
$PriorSeed = $env:UNSEEING_SEED
$PriorDemo = $env:UNSEEING_DEMO
try {
    if ($UseWindow) {
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
    if ($DemoMode) { $env:UNSEEING_DEMO = "1" }

    $launch = New-Object System.Collections.Generic.List[string]
    [void]$launch.Add("--path")
    [void]$launch.Add($GameDirectory)
    foreach ($extra in $Extra) { [void]$launch.Add($extra) }
    if (-not [string]::IsNullOrWhiteSpace($Scene)) { [void]$launch.Add($Scene) }

    # Built up rather than interpolated: Windows PowerShell 5.1 cannot parse a
    # double-quoted string containing $( ... ) that itself contains a
    # double-quoted string, and the whole file fails to load if it tries.
    $announce = "run-game: playing"
    if (-not [string]::IsNullOrWhiteSpace($Scene)) { $announce += " $Scene" }
    if ($UseWindow) { $announce += " (windowed $Geometry)" }
    Write-Output $announce
    # No -e and no --editor: this is the world, not the authoring environment.
    & $GodotPath @launch
    $exitCode = $LASTEXITCODE
} finally {
    if ($UseWindow) {
        Remove-Item -LiteralPath $Override -Force -ErrorAction SilentlyContinue
    }
    # A .ps1 runs IN the caller's session, so $env: assignments outlive it —
    # unlike the POSIX half, where export dies with the script's own process.
    # Without this, one -Seed 42 quietly seeded every later run in that window.
    if ($null -eq $PriorSeed) {
        Remove-Item Env:UNSEEING_SEED -ErrorAction SilentlyContinue
    } else { $env:UNSEEING_SEED = $PriorSeed }
    if ($null -eq $PriorDemo) {
        Remove-Item Env:UNSEEING_DEMO -ErrorAction SilentlyContinue
    } else { $env:UNSEEING_DEMO = $PriorDemo }
}
exit $exitCode
