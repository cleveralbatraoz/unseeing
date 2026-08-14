[CmdletBinding()]
param(
    [string]$SubjectUnderTest = ""
)

# Behavioral contract for tools\run_game.ps1 — build the engine, then play the
# GAME, never the editor.
#
# The POSIX half has test/run_game_test.sh; this side had nothing at all, which
# left the two hazards its own source calls out — the -Godot/-Passthrough prefix
# collision, and bootstrap.ps1's param() block resetting borrowed parameters when
# dot-sourced — resting on a single manual run.
#
# Everything below runs against a COPY of a checkout with a recording fake
# engine, so no case can build a real core, open a real window, or write into
# the developer's tree.
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
$Subject = if ([string]::IsNullOrWhiteSpace($SubjectUnderTest)) {
    Join-Path $Root "tools/run_game.ps1"
} else {
    $SubjectUnderTest
}
$Failed = 0

function Pass([string]$What) { Write-Output "run-game-windows: OK   $What" }
function Fail([string]$What) { Write-Output "run-game-windows: FAIL $What"; $script:Failed = 1 }
function Require([bool]$Condition, [string]$What) {
    if ($Condition) { Pass $What } else { Fail $What }
}

Require (Test-Path -LiteralPath $Subject -PathType Leaf) "the PowerShell run tool exists"
if ($Failed -ne 0) { exit $Failed }

$IsWindowsHost = [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT
$Sandbox = Join-Path ([IO.Path]::GetTempPath()) ("unseeing-run-{0}" -f [guid]::NewGuid())
$Repo = Join-Path $Sandbox "repo with spaces"
$Log = Join-Path $Sandbox "calls.log"
$Out = Join-Path $Sandbox "stdout.log"
$Err = Join-Path $Sandbox "stderr.log"
$Shell = (Get-Process -Id $PID).Path
$Pin = "4.7.1.stable.official"

New-Item -ItemType Directory -Path $Sandbox | Out-Null
New-Item -ItemType Directory -Path (Join-Path $Repo "game") | Out-Null
New-Item -ItemType Directory -Path (Join-Path $Repo "rust") | Out-Null
New-Item -ItemType Directory -Path (Join-Path $Repo "tools") | Out-Null
Copy-Item -LiteralPath $Subject -Destination (Join-Path $Repo "tools/run_game.ps1")
# run_game.ps1 borrows its engine discovery from bootstrap.ps1, so the copy is
# part of the subject.
Copy-Item -LiteralPath (Join-Path $Root "tools/bootstrap.ps1") `
    -Destination (Join-Path $Repo "tools/bootstrap.ps1")
Set-Content -LiteralPath (Join-Path $Repo ".godot-version") -Value $Pin -Encoding ascii
Set-Content -LiteralPath (Join-Path $Repo "rust/rust-toolchain.toml") -Encoding ascii -Value @(
    "[toolchain]"
    'channel = "1.90.7"'
)
$Target = if ($IsWindowsHost) { "x86_64-pc-windows-msvc" } else { "x86_64-pc-windows-msvc" }
$ArtifactDir = Join-Path $Repo "rust/target/$Target/release"
New-Item -ItemType Directory -Path $ArtifactDir -Force | Out-Null
Set-Content -LiteralPath (Join-Path $ArtifactDir "unseeing_core.dll") -Value "fixture core"

$Override = Join-Path $Repo "game/override.cfg"

try {
    # A recording engine: it logs the whole command line, then each argument on
    # its own line so an assertion can match a WHOLE argument. Searching a joined
    # string for " -e " cannot see a launch whose LAST argument is -e, which is
    # exactly the mutation that opens the editor.
    $FakeEngine = if ($IsWindowsHost) {
        Join-Path $Sandbox "godot fake.cmd"
    } else {
        Join-Path $Sandbox "godot-fake"
    }
    if ($IsWindowsHost) {
        # The version answer comes FIRST: the argument loop below consumes %1
        # with shift, so a check placed after it always sees an empty argument.
        Set-Content -LiteralPath $FakeEngine -Encoding ascii -Value @(
            '@echo off'
            'echo engine %* >>"%RUN_GAME_TEST_LOG%"'
            'if /I "%~1"=="--version" ('
            '  echo 4.7.1.stable.official.fixture'
            '  exit /b 0'
            ')'
            ':loop'
            'if "%~1"=="" goto done'
            'echo arg %~1 >>"%RUN_GAME_TEST_LOG%"'
            'shift'
            'goto loop'
            ':done'
            'if exist "%RUN_GAME_TEST_OVERRIDE%" type "%RUN_GAME_TEST_OVERRIDE%" >>"%RUN_GAME_TEST_LOG%"'
            'echo seed=%UNSEEING_SEED% demo=%UNSEEING_DEMO% >>"%RUN_GAME_TEST_LOG%"'
            'exit /b 0'
        )
    } else {
        Set-Content -LiteralPath $FakeEngine -Encoding utf8 -Value @(
            '#!/bin/sh'
            'printf "engine %s\n" "$*" >>"$RUN_GAME_TEST_LOG"'
            'for a in "$@"; do printf "arg %s\n" "$a" >>"$RUN_GAME_TEST_LOG"; done'
            '[ ! -f "$RUN_GAME_TEST_OVERRIDE" ] || cat "$RUN_GAME_TEST_OVERRIDE" >>"$RUN_GAME_TEST_LOG"'
            'printf "seed=%s demo=%s\n" "${UNSEEING_SEED:-}" "${UNSEEING_DEMO:-}" >>"$RUN_GAME_TEST_LOG"'
            '[ "$1" = "--version" ] && printf "%s\n" "4.7.1.stable.official.fixture"'
            'exit 0'
        )
        & /bin/chmod +x $FakeEngine
    }

    # The version fixture answers --version on its own; the recording one above
    # answers it too, so a single fake serves both roles.
    function Invoke-Run([string[]]$Arguments) {
        if (Test-Path -LiteralPath $Log) { Remove-Item -LiteralPath $Log -Force }
        $env:RUN_GAME_TEST_LOG = $Log
        $env:RUN_GAME_TEST_OVERRIDE = $Override
        $env:UNSEEING_ENGINE_CANDIDATES = $FakeEngine
        $line = @(
            "-NoProfile"
            "-ExecutionPolicy Bypass"
            "-File `"$(Join-Path $Repo 'tools/run_game.ps1')`""
            "-RepositoryRoot `"$Repo`""
            "-Architecture X64"
        ) + $Arguments -join " "
        $process = Start-Process -FilePath $Shell -ArgumentList $line -Wait -PassThru `
            -RedirectStandardOutput $Out -RedirectStandardError $Err
        $text = ""
        if (Test-Path -LiteralPath $Out) { $text += Get-Content -LiteralPath $Out -Raw }
        if (Test-Path -LiteralPath $Err) { $text += Get-Content -LiteralPath $Err -Raw }
        return @{
            ExitCode = $process.ExitCode
            Output = $text
            Calls = if (Test-Path -LiteralPath $Log) { @(Get-Content -LiteralPath $Log) } else { @() }
        }
    }
    function HasArg([hashtable]$Run, [string]$Value) {
        return [bool]($Run.Calls | Where-Object { $_.Trim() -eq "arg $Value" })
    }
    function Mentions([hashtable]$Run, [string]$Needle) {
        return ($Run.Calls -join "`n").Contains($Needle)
    }

    $plain = Invoke-Run @("-SkipBuild")
    Require ($plain.ExitCode -eq 0) "a default run completes"
    Require (HasArg $plain "--path") "the engine is launched against game/"
    Require (-not (HasArg $plain "-e")) "the run never opens the editor"
    Require (-not (HasArg $plain "--editor")) "the run never opens the editor by long flag"
    Require (-not (Mentions $plain "cargo build")) "-SkipBuild does not build"

    # README.md and tools/run_game.sh both promise Windows "the same contract",
    # and a designer reading either types --skip-build, not -SkipBuild. Before
    # this, the POSIX spelling was forwarded to Godot as an unknown argument.
    $posix = Invoke-Run @("--skip-build")
    Require ($posix.ExitCode -eq 0) "the POSIX spelling --skip-build is accepted"
    Require (-not (Mentions $posix "cargo build")) "--skip-build actually skips the build"
    Require (-not (HasArg $posix "--skip-build")) "--skip-build is consumed, not handed to Godot"

    # An advanced script binds -Verbose, -Debug and their unique prefixes as
    # COMMON parameters before any remaining-arguments parameter sees them, so
    # these were swallowed here instead of reaching the engine.
    $verbose = Invoke-Run @("--skip-build", "--", "--verbose")
    Require (HasArg $verbose "--verbose") "--verbose reaches the engine instead of PowerShell"
    $dashv = Invoke-Run @("--skip-build", "--", "-v")
    Require (HasArg $dashv "-v") "-v reaches the engine instead of PowerShell"

    # -Godot is a prefix of the old -GodotArguments, and a remaining-arguments
    # parameter swallowed it whole along with its value.
    $named = Invoke-Run @("-Godot `"$FakeEngine`"", "-SkipBuild")
    Require ($named.ExitCode -eq 0) "an explicitly named editor is used, not swallowed"
    Require ($named.Output.Contains("godot OK")) "the named editor passes the pin"

    $scene = Invoke-Run @("--skip-build", "--scene", "res://scenes/level_02.tscn")
    Require (HasArg $scene "res://scenes/level_02.tscn") "--scene reaches the engine"
    $badScene = Invoke-Run @("--skip-build", "--scene", "--demo")
    Require ($badScene.ExitCode -eq 2) "--scene refuses an option as its value"

    $seeded = Invoke-Run @("--skip-build", "--seed", "42", "--demo")
    Require (Mentions $seeded "seed=42") "--seed reaches the game as its environment"
    Require (Mentions $seeded "demo=1") "--demo reaches the game as its environment"
    $badSeed = Invoke-Run @("--skip-build", "--seed", "abc")
    Require ($badSeed.ExitCode -eq 2) "--seed refuses a non-number"

    # The window is configured through override.cfg because Godot's own window
    # flags lose to the project setting; what the file SAYS is the contract.
    $windowed = Invoke-Run @("--skip-build", "--windowed", "640x480")
    Require ($windowed.ExitCode -eq 0) "--windowed completes"
    Require (Mentions $windowed "window/size/mode=0") "the override asks for windowed mode"
    Require (Mentions $windowed "window/size/viewport_width=640") "the override carries the requested width"
    Require (Mentions $windowed "window/size/viewport_height=480") "the override carries the requested height"
    Require (-not (Test-Path -LiteralPath $Override)) "--windowed leaves no override.cfg behind"

    $defaultSize = Invoke-Run @("--skip-build", "--windowed")
    Require (Mentions $defaultSize "window/size/viewport_width=1280") "the default size is 1280x720"
    $notASize = Invoke-Run @("--skip-build", "--windowed", "--demo")
    Require (Mentions $notASize "demo=1") "--windowed does not swallow the option after it"

    # A pre-existing override.cfg belongs to whoever wrote it.
    Set-Content -LiteralPath $Override -Value "someone else" -Encoding ascii
    $occupied = Invoke-Run @("--skip-build", "--windowed")
    Require ($occupied.ExitCode -eq 2) "a pre-existing override.cfg is refused, not clobbered"
    Require ((Get-Content -LiteralPath $Override -Raw).Contains("someone else")) `
        "the refused run leaves the other file untouched"
    Remove-Item -LiteralPath $Override -Force

    # A .ps1 runs IN the caller's session, so $env: assignments outlive it.
    $env:UNSEEING_SEED = $null
    Remove-Item Env:UNSEEING_SEED -ErrorAction SilentlyContinue
    & (Join-Path $Repo "tools/run_game.ps1") -RepositoryRoot $Repo -Architecture X64 `
        -SkipBuild --seed 7 2>&1 | Out-Null
    Require ([string]::IsNullOrEmpty($env:UNSEEING_SEED)) `
        "a seeded run does not leave UNSEEING_SEED set in the caller's session"

    $missingCore = Join-Path $ArtifactDir "unseeing_core.dll"
    Remove-Item -LiteralPath $missingCore -Force
    $noCore = Invoke-Run @("--skip-build")
    Require ($noCore.ExitCode -eq 1) "playing with no built core is refused"
    Require (-not (HasArg $noCore "--path")) "a missing core never launches the game"
    Set-Content -LiteralPath $missingCore -Value "fixture core"
} finally {
    Remove-Item Env:RUN_GAME_TEST_LOG -ErrorAction SilentlyContinue
    Remove-Item Env:RUN_GAME_TEST_OVERRIDE -ErrorAction SilentlyContinue
    Remove-Item Env:UNSEEING_ENGINE_CANDIDATES -ErrorAction SilentlyContinue
    Remove-Item Env:UNSEEING_SEED -ErrorAction SilentlyContinue
    Remove-Item Env:UNSEEING_DEMO -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Sandbox -Recurse -Force -ErrorAction SilentlyContinue
}

exit $Failed
