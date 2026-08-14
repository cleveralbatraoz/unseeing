[CmdletBinding()]
param(
    [string]$BootstrapUnderTest = "",
    [string]$CmdEntryUnderTest = ""
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
$Bootstrap = if ([string]::IsNullOrWhiteSpace($BootstrapUnderTest)) {
    Join-Path $Root "tools/bootstrap.ps1"
} else {
    $BootstrapUnderTest
}
$CmdEntry = if ([string]::IsNullOrWhiteSpace($CmdEntryUnderTest)) {
    Join-Path $Root "tools/bootstrap.cmd"
} else {
    $CmdEntryUnderTest
}
$Failed = 0

function Pass([string]$What) {
    Write-Output "bootstrap-windows: OK   $What"
}

function Fail([string]$What) {
    Write-Output "bootstrap-windows: FAIL $What"
    $script:Failed = 1
}

function Require([bool]$Condition, [string]$What) {
    if ($Condition) {
        Pass $What
    } else {
        Fail $What
    }
}

Require (Test-Path -LiteralPath $Bootstrap -PathType Leaf) "PowerShell bootstrap exists"
Require (Test-Path -LiteralPath $CmdEntry -PathType Leaf) "CMD entry point exists"
if ($Failed -ne 0) {
    exit $Failed
}

$Sandbox = Join-Path ([IO.Path]::GetTempPath()) ("unseeing-bootstrap-{0}" -f [guid]::NewGuid())
$Log = Join-Path $Sandbox "calls.log"
$Stdout = Join-Path $Sandbox "stdout.log"
$Stderr = Join-Path $Sandbox "stderr.log"
$FakeRustup = Join-Path $Sandbox "rustup fake.ps1"
$FakeRustupInstaller = Join-Path $Sandbox "rustup installer fake.ps1"
$FakeGodot = Join-Path $Sandbox "godot fake.ps1"
$Repo = Join-Path $Sandbox "repo with spaces"
$Shell = (Get-Process -Id $PID).Path

New-Item -ItemType Directory -Path $Sandbox | Out-Null
New-Item -ItemType Directory -Path (Join-Path $Repo "rust") | Out-Null
New-Item -ItemType Directory -Path (Join-Path $Repo "game") | Out-Null
# The fixture carries its OWN pin, deliberately not the project's. Copying the
# live rust-toolchain.toml and then asserting a hardcoded version made this a
# constant-change detector: bumping Rust broke a dozen assertions with nothing
# to say about the bump. A pin the project does not use also proves something
# stronger — that the bootstrap READS the file rather than knowing a number.
$FixturePin = "1.90.7"
Set-Content -LiteralPath (Join-Path $Repo "rust/rust-toolchain.toml") -Encoding ascii -Value @(
    "[toolchain]"
    "channel = `"$FixturePin`""
    "components = [`"rustfmt`", `"clippy`"]"
)
Set-Content -LiteralPath (Join-Path $Repo ".godot-version") `
    -Value "4.7.1.stable.official" -Encoding ascii
# The count the fixture pretends the roster has — again not the project's, so a
# real class being added or removed cannot break these assertions.
$FixtureClasses = 7
New-Item -ItemType Directory -Path (Join-Path $Repo "ci") -Force | Out-Null
Set-Content -LiteralPath (Join-Path $Repo "ci/engine_class_count") `
    -Value "$FixtureClasses" -Encoding ascii

try {
    . $Bootstrap -NoRun

    function New-PeFixture([string]$Path, [uint16]$Machine) {
        $bytes = New-Object byte[] 256
        $bytes[0] = 0x4D
        $bytes[1] = 0x5A
        [BitConverter]::GetBytes([int32]128).CopyTo($bytes, 0x3C)
        $bytes[128] = 0x50
        $bytes[129] = 0x45
        [BitConverter]::GetBytes($Machine).CopyTo($bytes, 132)
        [IO.File]::WriteAllBytes($Path, $bytes)
    }

    $peX64 = Join-Path $Sandbox "godot-x64.exe"
    $peArm64 = Join-Path $Sandbox "godot-arm64.exe"
    New-PeFixture $peX64 0x8664
    New-PeFixture $peArm64 0xAA64
    Require ((Read-PeArchitecture $peX64) -eq "X64") `
        "the real PE reader recognizes an x86_64 Godot editor"
    Require ((Read-PeArchitecture $peArm64) -eq "Arm64") `
        "the real PE reader recognizes an ARM64 Godot editor"
    $scoopGui = Join-Path $Sandbox "godot.exe"
    $scoopConsole = Join-Path $Sandbox "godot.console.exe"
    Set-Content -LiteralPath $scoopGui -Value "gui fixture"
    Set-Content -LiteralPath $scoopConsole -Value "console fixture"
    Require ((Prefer-ConsoleGodot $scoopGui) -eq (Resolve-Path $scoopConsole).Path) `
        "Scoop's dot-console executable is preferred over its GUI binary"

    # The official archive ships exactly these two names and nobody renames
    # them. Discovery knew neither, so an editor that was installed, on PATH,
    # and the right version reported "godot not found" — the failure both
    # audited machines hit, and the one .github/workflows/test.yml works around
    # by globbing for '*_console.exe' before it ever calls this script.
    $archiveDir = Join-Path $Sandbox "downloads"
    New-Item -ItemType Directory -Path $archiveDir | Out-Null
    $archiveGui = Join-Path $archiveDir "Godot_v4.7.1-stable_win64.exe"
    $archiveConsole = Join-Path $archiveDir "Godot_v4.7.1-stable_win64_console.exe"
    Set-Content -LiteralPath $archiveGui -Value "gui fixture"
    Set-Content -LiteralPath $archiveConsole -Value "console fixture"
    Require ((Prefer-ConsoleGodot $archiveGui) -eq (Resolve-Path $archiveConsole).Path) `
        "the official archive's GUI executable maps to its console sibling"

    $oldPathForArchive = $env:Path
    try {
        $env:Path = $archiveDir + [IO.Path]::PathSeparator + $env:Path
        $found = Get-GodotCandidates $Repo
        Require ($found -contains (Resolve-Path $archiveConsole).Path -or $found -contains $archiveConsole) `
            "the official archive name on PATH becomes a discovery candidate"
    } finally {
        $env:Path = $oldPathForArchive
    }

    $binDir = Join-Path $Repo "godot-bin"
    New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    $binArchive = Join-Path $binDir "Godot_v4.7.1-stable_win64_console.exe"
    Set-Content -LiteralPath $binArchive -Value "console fixture"
    $foundInBin = Get-GodotCandidates $Repo
    Require ($foundInBin -contains $binArchive -or
        $foundInBin -contains (Resolve-Path $binArchive).Path) `
        "an archive unpacked into godot-bin becomes a discovery candidate"
    Remove-Item -LiteralPath $binDir -Recurse -Force

    # The pure predicate, hand-derived from .godot-version = 4.7.1.stable.official
    # and the strings real editors print. Mirrors test/engine_select_test.sh.
    Require (Test-EngineVersion "4.7.1.stable.official.a13da4feb" "4.7.1.stable.official") `
        "the exact pinned build satisfies the pin"
    Require (Test-EngineVersion "4.7.1.stable.mono.official.a13da4feb" "4.7.1.stable.official") `
        "a Mono/.NET build of the pinned version satisfies the pin"
    Require (-not (Test-EngineVersion "4.7.0.stable.official.a13da4feb" "4.7.1.stable.official")) `
        "a nearby patch release does not"
    Require (-not (Test-EngineVersion "4.7.stable.mono.official.5b4e0cb0f" "4.7.1.stable.official")) `
        "a Mono build of the wrong version is still refused"
    Require (-not (Test-EngineVersion "4.7.10.stable.official.abc" "4.7.1")) `
        "a longer numeric field is not a prefix match"
    Require (-not (Test-EngineVersion "4.7.1.stable.monolithic.official.abc" "4.7.1.stable.official")) `
        "the flavour field is dropped whole, not as a substring"
    Require (-not (Test-EngineVersion "" "4.7.1.stable.official")) `
        "an empty version satisfies nothing"

    # Discovery must take the first candidate that SATISFIES the pin, not the
    # first that exists — a machine can hold several editors, and the wrong one
    # sorting earlier is the ordinary case, not a corner one.
    $engineDir = Join-Path $Sandbox "engines"
    New-Item -ItemType Directory -Path $engineDir | Out-Null
    function New-FixtureEngine([string]$Path, [string]$Version) {
        if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
            Set-Content -LiteralPath $Path -Encoding ascii -Value @(
                "@echo off"
                "if /I `"%~1`"==`"--version`" echo $Version"
                "exit /b 0"
            )
        } else {
            Set-Content -LiteralPath $Path -Encoding utf8 -Value @(
                "#!/bin/sh"
                "[ `"`$1`" = --version ] && printf '%s\n' '$Version'"
                "exit 0"
            )
            & /bin/chmod +x $Path
        }
    }
    $ext = if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) { ".cmd" } else { "" }
    $engineWrong = Join-Path $engineDir ("wrong" + $ext)
    $engineRight = Join-Path $engineDir ("right" + $ext)
    New-FixtureEngine $engineWrong "4.7.stable.mono.official.5b4e0cb0f"
    New-FixtureEngine $engineRight "4.7.1.stable.official.a13da4feb"
    $oldCandidates = $env:UNSEEING_ENGINE_CANDIDATES
    try {
        $env:UNSEEING_ENGINE_CANDIDATES = "$engineWrong`n$engineRight"
        $picked = Find-Godot "" $Repo "4.7.1.stable.official"
        Require ($picked -eq (Resolve-Path $engineRight).Path) `
            "discovery skips an engine that fails the pin and takes the one that passes"
    } finally {
        if ($null -eq $oldCandidates) {
            Remove-Item Env:UNSEEING_ENGINE_CANDIDATES -ErrorAction SilentlyContinue
        } else {
            $env:UNSEEING_ENGINE_CANDIDATES = $oldCandidates
        }
    }

    $oldArch = $env:PROCESSOR_ARCHITECTURE
    $oldWow = $env:PROCESSOR_ARCHITEW6432
    try {
        $env:PROCESSOR_ARCHITECTURE = "AMD64"
        Remove-Item Env:PROCESSOR_ARCHITEW6432 -ErrorAction SilentlyContinue
        Require ((Get-RustupHost) -eq "x86_64-pc-windows-msvc") `
            "a fresh x86_64 Windows host downloads x86_64 rustup-init"
        $env:PROCESSOR_ARCHITECTURE = "x86"
        $env:PROCESSOR_ARCHITEW6432 = "ARM64"
        Require ((Get-RustupHost) -eq "aarch64-pc-windows-msvc") `
            "native ARM64 wins over an emulated 32-bit PowerShell process"
    } finally {
        $env:PROCESSOR_ARCHITECTURE = $oldArch
        if ($null -eq $oldWow) {
            Remove-Item Env:PROCESSOR_ARCHITEW6432 -ErrorAction SilentlyContinue
        } else {
            $env:PROCESSOR_ARCHITEW6432 = $oldWow
        }
    }
    if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
        $nativeStderrProbe = Invoke-Captured "$env:ComSpec" @(
            "/d", "/c", "echo fixture-native-progress 1>&2 & exit /b 0"
        )
        Require ($nativeStderrProbe.ExitCode -eq 0 -and
            (($nativeStderrProbe.Lines -join "`n").Contains("fixture-native-progress"))) `
            "Windows PowerShell 5.1 captures successful native stderr"

        $wrapperSandbox = Join-Path $Sandbox "wrapper path with spaces"
        New-Item -ItemType Directory -Path $wrapperSandbox | Out-Null
        $wrapperCmd = Join-Path $wrapperSandbox "bootstrap.cmd"
        $wrapperPs = Join-Path $wrapperSandbox "bootstrap.ps1"
        Copy-Item -LiteralPath $CmdEntry -Destination $wrapperCmd
        @'
param([string]$Marker = "")
if ($Marker -ne "value with spaces") { exit 41 }
exit 37
'@ | Set-Content -LiteralPath $wrapperPs -Encoding ascii
        & $wrapperCmd -Marker "value with spaces"
        Require ($LASTEXITCODE -eq 37) `
            "the CMD wrapper forwards spaced arguments and propagates failure"
    }

    @'
Add-Content -LiteralPath $env:BOOTSTRAP_TEST_LOG -Value ("rustup " + ($args -join " "))
Add-Content -LiteralPath $env:BOOTSTRAP_TEST_LOG -Value ("rustup-cwd " + (Get-Location).Path + " " + ($args -join " "))
if ($env:BOOTSTRAP_TEST_NATIVE_STDERR -eq "1") {
    [Console]::Error.WriteLine("fixture native progress on stderr")
}
if ($args.Count -eq 1 -and $args[0] -eq "--version") {
    Write-Output "rustup 1.28.2 (fixture)"
    exit 0
}
if (($args -join " ") -eq "run $env:BOOTSTRAP_TEST_PIN rustc --version") {
    $requiredCwd = if ($env:BOOTSTRAP_TEST_REQUIRED_RUST_CWD) {
        (Resolve-Path -LiteralPath $env:BOOTSTRAP_TEST_REQUIRED_RUST_CWD).Path
    } else {
        ""
    }
    $actualCwd = (Resolve-Path -LiteralPath (Get-Location).Path).Path
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
        $requiredCwd = $requiredCwd -replace '^/private/var/', '/var/'
        $actualCwd = $actualCwd -replace '^/private/var/', '/var/'
    }
    if ($requiredCwd -and $actualCwd -ne $requiredCwd) {
        Write-Error "fixture toolchain selection happened outside rust/"
        exit 31
    }
    if (-not (Test-Path -LiteralPath $env:BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL)) {
        exit 1
    }
    if ($env:BOOTSTRAP_TEST_RUSTC_VERSION) {
        Write-Output $env:BOOTSTRAP_TEST_RUSTC_VERSION
    } else {
        Write-Output "rustc $env:BOOTSTRAP_TEST_PIN (fixture)"
    }
    exit 0
}
if (($args -join " ") -eq "toolchain install $env:BOOTSTRAP_TEST_PIN --profile minimal") {
    Set-Content -LiteralPath $env:BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL -Value "installed"
    exit 0
}
if (($args -join " ") -eq "run $env:BOOTSTRAP_TEST_PIN cargo --version") {
    Write-Output "cargo $env:BOOTSTRAP_TEST_PIN (fixture)"
    exit 0
}
if (($args -join " ").StartsWith("target add ")) {
    exit 0
}
if ($args.Count -gt 3 -and $args[0] -eq "run" -and $args[2] -eq "cargo" -and
    $args[3] -eq "build" -and $env:BOOTSTRAP_TEST_CARGO_FAIL -eq "1") {
    exit 19
}
# A build that announces itself and then refuses to finish until the test says
# so. It is the only way to tell streaming from buffering: with buffering the
# marker cannot appear until this process has already exited.
if ($args.Count -gt 3 -and $args[0] -eq "run" -and $args[2] -eq "cargo" -and
    $args[3] -eq "build" -and $env:BOOTSTRAP_TEST_STREAM_GATE) {
    Write-Output "FIXTURE-STREAM-MARKER"
    while (-not (Test-Path -LiteralPath $env:BOOTSTRAP_TEST_STREAM_GATE)) {
        Start-Sleep -Milliseconds 25
    }
}
if ($args.Count -gt 3 -and $args[0] -eq "run" -and $args[2] -eq "cargo" -and
    $args[3] -eq "build" -and
    $env:BOOTSTRAP_TEST_ARTIFACT -and $env:BOOTSTRAP_TEST_SKIP_ARTIFACT -ne "1") {
    $parent = Split-Path -Parent $env:BOOTSTRAP_TEST_ARTIFACT
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    Set-Content -LiteralPath $env:BOOTSTRAP_TEST_ARTIFACT -Value "fixture dll"
}
exit 0
'@ | Set-Content -LiteralPath $FakeRustup -Encoding utf8

    @'
Add-Content -LiteralPath $env:BOOTSTRAP_TEST_LOG -Value ("godot " + ($args -join " "))
if ($args.Count -eq 1 -and $args[0] -eq "--version") {
    if ($env:BOOTSTRAP_TEST_GODOT_SILENT -eq "1") {
        exit 0
    }
    if ($env:BOOTSTRAP_TEST_GODOT_VERSION) {
        Write-Output $env:BOOTSTRAP_TEST_GODOT_VERSION
    } else {
        Write-Output "4.7.1.stable.official.fixture"
    }
    exit 0
}
if (($args -join " ").Contains("engine_census_probe.gd") -and $env:BOOTSTRAP_TEST_CENSUS_FAIL -eq "1") {
    exit 23
}
if (($args -join " ").Contains("--import") -and $env:BOOTSTRAP_TEST_IMPORT_FAIL -eq "1") {
    exit 17
}
if (($args -join " ").Contains("engine_census_probe.gd")) {
    if ($env:BOOTSTRAP_TEST_WRONG_CENSUS -eq "1") {
        Write-Output "probe: PASS ($([int]$env:BOOTSTRAP_TEST_CLASSES - 1) checks)"
    } else {
        Write-Output "probe: PASS ($env:BOOTSTRAP_TEST_CLASSES checks)"
    }
}
exit 0
'@ | Set-Content -LiteralPath $FakeGodot -Encoding utf8

    @'
Add-Content -LiteralPath $env:BOOTSTRAP_TEST_LOG -Value "rustup installer invoked"
$installBin = Join-Path $env:CARGO_HOME "bin"
New-Item -ItemType Directory -Path $installBin -Force | Out-Null
if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
    $wrapper = Join-Path $installBin "rustup.cmd"
    $body = @(
        "@echo off"
        ('"{0}" -NoProfile -File "{1}" %*' -f `
            $env:BOOTSTRAP_TEST_SHELL, $env:BOOTSTRAP_TEST_FAKE_RUSTUP)
        "set fixture_status=%ERRORLEVEL%"
        'echo rustup-wrapper-exit %fixture_status% %*>>"%BOOTSTRAP_TEST_LOG%"'
        "exit /b %fixture_status%"
    ) -join "`r`n"
    Set-Content -LiteralPath $wrapper -Value $body -Encoding ascii
} else {
    $wrapper = Join-Path $installBin "rustup"
    $body = @(
        "#!/bin/sh"
        ('"{0}" -NoProfile -File "{1}" "$@"' -f `
            $env:BOOTSTRAP_TEST_SHELL, $env:BOOTSTRAP_TEST_FAKE_RUSTUP)
        "fixture_status=`$?"
        'printf ''rustup-wrapper-exit %s %s\n'' "$fixture_status" "$*" >>"$BOOTSTRAP_TEST_LOG"'
        "exit `$fixture_status"
    ) -join "`n"
    Set-Content -LiteralPath $wrapper -Value $body -Encoding utf8
    & /bin/chmod +x $wrapper
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
$global:LASTEXITCODE = 0
'@ | Set-Content -LiteralPath $FakeRustupInstaller -Encoding utf8

    function Invoke-Fixture([string]$Architecture, [bool]$InstallRustup = $false) {
        if (Test-Path -LiteralPath $Log) {
            Remove-Item -LiteralPath $Log -Force
        }
        $env:BOOTSTRAP_TEST_LOG = $Log
        $env:BOOTSTRAP_TEST_PIN = $FixturePin
        $env:BOOTSTRAP_TEST_CLASSES = $FixtureClasses
        $env:BOOTSTRAP_TEST_REQUIRED_RUST_CWD = Join-Path $Repo "rust"
        $env:BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL = Join-Path $Sandbox "toolchain-installed"
        Remove-Item -LiteralPath $env:BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL `
            -Force -ErrorAction SilentlyContinue
        $target = if ($Architecture -eq "X64") {
            "x86_64-pc-windows-msvc"
        } else {
            "aarch64-pc-windows-msvc"
        }
        $env:BOOTSTRAP_TEST_ARTIFACT = Join-Path $Repo "rust/target/$target/release/unseeing_core.dll"
        $artifactParent = Split-Path -Parent $env:BOOTSTRAP_TEST_ARTIFACT
        New-Item -ItemType Directory -Path $artifactParent -Force | Out-Null
        Set-Content -LiteralPath $env:BOOTSTRAP_TEST_ARTIFACT -Value "stale fixture dll"

        $arguments = @(
            "-NoProfile"
            "-ExecutionPolicy Bypass"
            "-File `"$Bootstrap`""
            "-Godot `"$FakeGodot`""
            "-Architecture $Architecture"
            "-RepositoryRoot `"$Repo`""
        )
        if ($InstallRustup) {
            $arguments += "-RustupInstaller `"$FakeRustupInstaller`""
        } else {
            $arguments += "-Rustup `"$FakeRustup`""
        }
        $argumentLine = $arguments -join " "

        $oldPath = $env:Path
        $oldUpperPath = $env:PATH
        $oldCargoHome = $env:CARGO_HOME
        try {
            if ($InstallRustup) {
                $env:BOOTSTRAP_TEST_SHELL = $Shell
                $env:BOOTSTRAP_TEST_FAKE_RUSTUP = $FakeRustup
                $env:CARGO_HOME = Join-Path $Sandbox "empty cargo home"
                $isolatedPath = if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
                    Join-Path $env:SystemRoot "System32"
                } else {
                    "/usr/bin:/bin"
                }
                $env:Path = $isolatedPath
                $env:PATH = $isolatedPath
            }
            $process = Start-Process -FilePath $Shell -ArgumentList $argumentLine -Wait -PassThru `
                -RedirectStandardOutput $Stdout -RedirectStandardError $Stderr
        } finally {
            $env:Path = $oldPath
            $env:PATH = $oldUpperPath
            if ($null -eq $oldCargoHome) {
                Remove-Item Env:CARGO_HOME -ErrorAction SilentlyContinue
            } else {
                $env:CARGO_HOME = $oldCargoHome
            }
        }
        $output = ""
        if (Test-Path -LiteralPath $Stdout) {
            $output += Get-Content -LiteralPath $Stdout -Raw
        }
        if (Test-Path -LiteralPath $Stderr) {
            $output += Get-Content -LiteralPath $Stderr -Raw
        }
        return @{
            ExitCode = $process.ExitCode
            Output = $output
            Calls = if (Test-Path -LiteralPath $Log) {
                @(Get-Content -LiteralPath $Log)
            } else {
                @()
            }
        }
    }

    function Calls-Contain([hashtable]$Run, [string]$Needle) {
        return (($Run.Calls -join "`n").Contains($Needle))
    }

    $x64 = Invoke-Fixture "X64"
    Require ($x64.ExitCode -eq 0) "x86_64 fixture completes"
    Require (Calls-Contain $x64 (
        "rustup-cwd " + (Join-Path $Repo "rust") + " run $FixturePin rustc --version"
    )) "the exact compiler pin is selected through rustup inside rust/"
    Require (Calls-Contain $x64 "rustup toolchain install $FixturePin --profile minimal") `
        "a fresh rustup receives the pinned toolchain without a second command"
    Require (Calls-Contain $x64 "rustup target add x86_64-pc-windows-msvc --toolchain $FixturePin") `
        "x86_64 standard library is installed for the selected Godot target"
    Require (Calls-Contain $x64 (
        "rustup run $FixturePin cargo build --release --features editor-docs " +
        "--target x86_64-pc-windows-msvc " +
        "--target-dir " + (Join-Path $Repo "rust/target")
    )) `
        "x86_64 builds the release editor artifact at its declared target"
    Require ($x64.Output.Contains("bootstrap: OK")) "success is announced only after the fixture census"

    $freshRustup = Invoke-Fixture "X64" $true
    if ($freshRustup.ExitCode -ne 0) {
        Write-Output "bootstrap-windows: installer fixture output follows"
        Write-Output $freshRustup.Output
        Write-Output ($freshRustup.Calls -join "`n")
    }
    Require ($freshRustup.ExitCode -eq 0 -and
        (Calls-Contain $freshRustup "rustup installer invoked")) `
        "rustup absence invokes the installer and completes in one command"
    Require (Calls-Contain $freshRustup "rustup run $FixturePin cargo build") `
        "the newly installed rustup is discovered in the current process"

    $arm64 = Invoke-Fixture "Arm64"
    Require ($arm64.ExitCode -eq 0) "ARM64 fixture completes"
    Require (Calls-Contain $arm64 (
        "rustup run $FixturePin cargo build --release --features editor-docs " +
        "--target aarch64-pc-windows-msvc " +
        "--target-dir " + (Join-Path $Repo "rust/target")
    )) `
        "ARM64 builds the release editor artifact at its declared target"
    Require (Calls-Contain $arm64 "rustup target add aarch64-pc-windows-msvc --toolchain $FixturePin") `
        "ARM64 standard library is installed for the selected Godot target"

    $calls = $arm64.Calls -join "`n"
    $importAt = $calls.IndexOf("--import")
    $censusAt = $calls.IndexOf("engine_census_probe.gd")
    Require ($importAt -ge 0 -and $censusAt -gt $importAt) "project import happens before the class census"

    $env:BOOTSTRAP_TEST_IMPORT_FAIL = "1"
    $noisyImport = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_IMPORT_FAIL
    Require ($noisyImport.ExitCode -eq 0) "a nonzero cache import yields to a successful class census"
    Require ($noisyImport.Output.Contains("authoritative class census")) `
        "the tolerated import failure is explained rather than hidden"

    $env:BOOTSTRAP_TEST_NATIVE_STDERR = "1"
    $nativeStderr = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_NATIVE_STDERR
    Require ($nativeStderr.ExitCode -eq 0) `
        "successful native stderr is output, not a PowerShell 5.1 terminating error"

    $env:BOOTSTRAP_TEST_SKIP_ARTIFACT = "1"
    $noArtifact = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_SKIP_ARTIFACT
    Require ($noArtifact.ExitCode -eq 1) `
        "a no-op build cannot reuse the DLL left by an earlier checkout"
    Require (-not (Calls-Contain $noArtifact "--import")) `
        "a missing fresh DLL never reaches import"

    $env:BOOTSTRAP_TEST_RUSTC_VERSION = "rustc 1.90.6 (fixture)"
    $wrongRust = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_RUSTC_VERSION
    Require ($wrongRust.ExitCode -eq 2) `
        "a rustup toolchain with the wrong compiler is refused"
    Require (-not (Calls-Contain $wrongRust "cargo build")) `
        "a compiler-pin refusal never builds"

    $env:BOOTSTRAP_TEST_GODOT_VERSION = "4.7.0.stable.official.fixture"
    $wrongVersion = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_GODOT_VERSION
    Require ($wrongVersion.ExitCode -eq 2) "a nearby Godot version is refused as an environment failure"
    Require ($wrongVersion.Output.Contains("!= pinned '4.7.1.stable.official'")) `
        "the version refusal names the complete pin"
    Require (-not (Calls-Contain $wrongVersion "--import")) `
        "a version refusal never imports with a mismatched editor"
    Require (-not (Calls-Contain $wrongVersion "cargo build")) `
        "a version refusal never builds"

    # .godot-version pins a version, not a build flavour. Refusing every .NET
    # editor made the most convenient install on several platforms unusable.
    $env:BOOTSTRAP_TEST_GODOT_VERSION = "4.7.1.stable.mono.official.fixture"
    $monoVersion = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_GODOT_VERSION
    Require ($monoVersion.ExitCode -eq 0) "a Mono build of the pinned version is accepted"
    Require (Calls-Contain $monoVersion "engine_census_probe.gd") `
        "the accepted Mono build reaches the census"

    # A GUI-subsystem editor has no console to answer on. Calling that a
    # version mismatch sends the reader hunting for a version problem that does
    # not exist; the fix is to point at the _console.exe sibling.
    $env:BOOTSTRAP_TEST_GODOT_SILENT = "1"
    $silent = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_GODOT_SILENT
    Require ($silent.ExitCode -eq 2) "an editor that answers with silence is refused"
    Require ($silent.Output.Contains("reported no version")) `
        "silence is diagnosed as a missing version, not a version mismatch"
    Require ($silent.Output.Contains("_console.exe")) `
        "the silence diagnosis names the console executable as the remedy"

    $env:BOOTSTRAP_TEST_CARGO_FAIL = "1"
    $buildFailure = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_CARGO_FAIL
    Require ($buildFailure.ExitCode -eq 1) "a failed Rust release build propagates"
    Require (-not (Calls-Contain $buildFailure "--import")) "a failed build never imports a stale extension"
    Require (-not $buildFailure.Output.Contains("bootstrap: OK")) "a failed build never announces success"

    $env:BOOTSTRAP_TEST_CENSUS_FAIL = "1"
    $censusFailure = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_CENSUS_FAIL
    Require ($censusFailure.ExitCode -eq 1) "a failed class census propagates"
    Require (-not $censusFailure.Output.Contains("bootstrap: OK")) "a failed census never announces success"

    $env:BOOTSTRAP_TEST_WRONG_CENSUS = "1"
    $wrongCensus = Invoke-Fixture "X64"
    Remove-Item Env:BOOTSTRAP_TEST_WRONG_CENSUS
    Require ($wrongCensus.ExitCode -eq 1) `
        "a successful process with the wrong class count is refused"
    Require (-not $wrongCensus.Output.Contains("bootstrap: OK")) `
        "the wrong census never announces success"

    # A cold Windows bootstrap spends four minutes compiling godot-core. When
    # that output is collected and replayed afterwards, the terminal shows
    # nothing at all for four minutes and an interrupt loses the entire log.
    # The fixture build prints a marker and then parks; if the marker reaches
    # stdout while the process is STILL RUNNING, the output is streaming. With
    # buffering it cannot appear until after the process is gone.
    $streamGate = Join-Path $Sandbox "stream-gate"
    Remove-Item -LiteralPath $streamGate -Force -ErrorAction SilentlyContinue
    $streamOut = Join-Path $Sandbox "stream-stdout.log"
    $streamErr = Join-Path $Sandbox "stream-stderr.log"
    Remove-Item -LiteralPath $Log -Force -ErrorAction SilentlyContinue
    $env:BOOTSTRAP_TEST_LOG = $Log
    $env:BOOTSTRAP_TEST_REQUIRED_RUST_CWD = Join-Path $Repo "rust"
    $env:BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL = Join-Path $Sandbox "toolchain-installed"
    Set-Content -LiteralPath $env:BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL -Value "installed"
    $env:BOOTSTRAP_TEST_ARTIFACT = `
        Join-Path $Repo "rust/target/x86_64-pc-windows-msvc/release/unseeing_core.dll"
    $env:BOOTSTRAP_TEST_STREAM_GATE = $streamGate
    $streamArguments = @(
        "-NoProfile"
        "-ExecutionPolicy Bypass"
        "-File `"$Bootstrap`""
        "-Godot `"$FakeGodot`""
        "-Architecture X64"
        "-RepositoryRoot `"$Repo`""
        "-Rustup `"$FakeRustup`""
    ) -join " "
    $streamProcess = Start-Process -FilePath $Shell -ArgumentList $streamArguments -PassThru `
        -RedirectStandardOutput $streamOut -RedirectStandardError $streamErr
    $sawMarkerWhileRunning = $false
    # Polling for the condition, not waiting a guessed interval: the loop ends
    # the moment the marker lands, and the bound only exists so a broken build
    # cannot hang the suite.
    $deadline = [DateTime]::UtcNow.AddSeconds(90)
    while ([DateTime]::UtcNow -lt $deadline -and -not $streamProcess.HasExited) {
        $soFar = ""
        try {
            $handle = [IO.File]::Open($streamOut, [IO.FileMode]::Open,
                [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
            $reader = New-Object IO.StreamReader($handle)
            $soFar = $reader.ReadToEnd()
            $reader.Dispose()
        } catch {
            $soFar = ""
        }
        if ($soFar.Contains("FIXTURE-STREAM-MARKER")) {
            $sawMarkerWhileRunning = $true
            break
        }
        Start-Sleep -Milliseconds 50
    }
    Set-Content -LiteralPath $streamGate -Value "go"
    $streamProcess.WaitForExit()
    Remove-Item Env:BOOTSTRAP_TEST_STREAM_GATE -ErrorAction SilentlyContinue
    Require $sawMarkerWhileRunning `
        "build output reaches the console while the build is still running"
    $streamFinal = Get-Content -LiteralPath $streamOut -Raw
    Require (([regex]::Matches($streamFinal, "FIXTURE-STREAM-MARKER")).Count -eq 1) `
        "streamed build output is printed once, not collected and replayed"
} finally {
    Remove-Item Env:BOOTSTRAP_TEST_STREAM_GATE -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_GODOT_SILENT -ErrorAction SilentlyContinue
    Remove-Item Env:UNSEEING_ENGINE_CANDIDATES -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_LOG -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_PIN -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_CLASSES -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_GODOT_VERSION -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_CARGO_FAIL -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_CENSUS_FAIL -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_IMPORT_FAIL -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_WRONG_CENSUS -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_NATIVE_STDERR -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_SKIP_ARTIFACT -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_ARTIFACT -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_REQUIRED_RUST_CWD -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_RUSTC_VERSION -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_SHELL -ErrorAction SilentlyContinue
    Remove-Item Env:BOOTSTRAP_TEST_FAKE_RUSTUP -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $Sandbox -Recurse -Force -ErrorAction SilentlyContinue
}

exit $Failed
