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
Copy-Item -LiteralPath (Join-Path $Root "rust/rust-toolchain.toml") `
    -Destination (Join-Path $Repo "rust/rust-toolchain.toml")
Set-Content -LiteralPath (Join-Path $Repo ".godot-version") `
    -Value "4.7.1.stable.official" -Encoding ascii

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
if (($args -join " ") -eq "run 1.97.1 rustc --version") {
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
        Write-Output "rustc 1.97.1 (fixture)"
    }
    exit 0
}
if (($args -join " ") -eq "toolchain install 1.97.1 --profile minimal") {
    Set-Content -LiteralPath $env:BOOTSTRAP_TEST_TOOLCHAIN_SENTINEL -Value "installed"
    exit 0
}
if (($args -join " ") -eq "run 1.97.1 cargo --version") {
    Write-Output "cargo 1.97.1 (fixture)"
    exit 0
}
if (($args -join " ").StartsWith("target add ")) {
    exit 0
}
if ($args.Count -gt 3 -and $args[0] -eq "run" -and $args[2] -eq "cargo" -and
    $args[3] -eq "build" -and $env:BOOTSTRAP_TEST_CARGO_FAIL -eq "1") {
    exit 19
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
        Write-Output "probe: PASS (18 checks)"
    } else {
        Write-Output "probe: PASS (19 checks)"
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
        "rustup-cwd " + (Join-Path $Repo "rust") + " run 1.97.1 rustc --version"
    )) "the exact compiler pin is selected through rustup inside rust/"
    Require (Calls-Contain $x64 "rustup toolchain install 1.97.1 --profile minimal") `
        "a fresh rustup receives the pinned toolchain without a second command"
    Require (Calls-Contain $x64 "rustup target add x86_64-pc-windows-msvc --toolchain 1.97.1") `
        "x86_64 standard library is installed for the selected Godot target"
    Require (Calls-Contain $x64 (
        "rustup run 1.97.1 cargo build --release --features editor-docs " +
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
    Require (Calls-Contain $freshRustup "rustup run 1.97.1 cargo build") `
        "the newly installed rustup is discovered in the current process"

    $arm64 = Invoke-Fixture "Arm64"
    Require ($arm64.ExitCode -eq 0) "ARM64 fixture completes"
    Require (Calls-Contain $arm64 (
        "rustup run 1.97.1 cargo build --release --features editor-docs " +
        "--target aarch64-pc-windows-msvc " +
        "--target-dir " + (Join-Path $Repo "rust/target")
    )) `
        "ARM64 builds the release editor artifact at its declared target"
    Require (Calls-Contain $arm64 "rustup target add aarch64-pc-windows-msvc --toolchain 1.97.1") `
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

    $env:BOOTSTRAP_TEST_RUSTC_VERSION = "rustc 1.97.0 (fixture)"
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
} finally {
    Remove-Item Env:BOOTSTRAP_TEST_LOG -ErrorAction SilentlyContinue
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
