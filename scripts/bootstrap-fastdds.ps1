param(
    [Parameter(Mandatory = $true)]
    [string] $CachePath
)
$ErrorActionPreference = "Stop"
$VcpkgCommit = "08289410db1d8b9d7cc31def3decfc0831e20f4e"
$GeneratorTag = "v4.3.0"
$Triplet = "x64-windows"
New-Item -ItemType Directory -Path $CachePath -Force | Out-Null
$Cache = (Resolve-Path -LiteralPath $CachePath).Path
$Vcpkg = Join-Path $Cache "vcpkg"
$Install = Join-Path $Cache "installed"
$Prefix = Join-Path $Install $Triplet
$Generator = Join-Path $Cache "sources\fastddsgen"
$GeneratorCommand = Join-Path $Generator "scripts\fastddsgen.bat"
$Log = Join-Path $Cache "bootstrap-windows.log"
$Marker = Join-Path $Cache ".ready-windows-v1"
$Expected = "vcpkg=$VcpkgCommit fastdds=3.6.2 generator=$GeneratorTag triplet=$Triplet"

# A lock guards against parallel Cargo builds writing to a single vcpkg tree.
$LockPath = Join-Path $Cache ".bootstrap.lock"
$Lock = $null
for ($attempt = 0; $attempt -lt 3600; $attempt++) {
    try {
        $Lock = [System.IO.File]::Open(
            $LockPath, [System.IO.FileMode]::OpenOrCreate,
            [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None
        )
        break
    } catch [System.IO.IOException] {
        Start-Sleep -Milliseconds 500
    }
}
if ($null -eq $Lock) { throw "Timed out acquiring bootstrap lock at $LockPath" }

function Invoke-Logged {
    param([string]$Executable, [string[]]$Arguments, [string]$WorkingDirectory = "")
    $previous = (Get-Location).Path
    try {
        if ($WorkingDirectory) { Set-Location -LiteralPath $WorkingDirectory }
        # Windows PowerShell converts native stderr into non-terminating
        # NativeCommandError records. Do not let harmless git/vcpkg progress
        # messages abort the bootstrap; check the real native exit code.
        $ErrorActionPreference = "Continue"
        & $Executable @Arguments *>> $Log
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = "Stop"
        if ($exitCode -ne 0) {
            $tail = (Get-Content -LiteralPath $Log -Tail 80 -ErrorAction SilentlyContinue) -join [Environment]::NewLine
            throw ("Command failed (exit " + $exitCode + "): " + $Executable + " " + ($Arguments -join " ") + [Environment]::NewLine + "Log: " + $Log + [Environment]::NewLine + $tail)
        }
    } finally {
        $ErrorActionPreference = "Stop"
        Set-Location -LiteralPath $previous
    }
}

function Test-Ready {
    if (!(Test-Path -LiteralPath $Marker)) { return $false }
    if ((Get-Content -LiteralPath $Marker -Raw).Trim() -ne $Expected) { return $false }
    foreach ($file in @(
        (Join-Path $Prefix "include\fastdds\dds\domain\DomainParticipantFactory.hpp"),
        (Join-Path $Prefix "lib\fastdds.lib"),
        (Join-Path $Prefix "lib\fastcdr.lib"),
        $GeneratorCommand,
        (Join-Path $Generator "share\fastddsgen\java\fastddsgen.jar")
    )) {
        if (!(Test-Path -LiteralPath $file)) { return $false }
    }
    if (@(Get-ChildItem -LiteralPath (Join-Path $Prefix "bin") -Filter "fastdds*.dll" -ErrorAction SilentlyContinue).Count -eq 0) { return $false }
    return $true
}

try {
    if (!(Test-Ready)) {
        foreach ($tool in @("git.exe", "cmake.exe", "java.exe")) {
            if (!(Get-Command $tool -ErrorAction SilentlyContinue)) {
                throw "Missing prerequisite: $tool. Install Git, CMake and JDK 17+."
            }
        }
        if (!(Test-Path (Join-Path $Vcpkg ".git"))) {
            New-Item -ItemType Directory -Path $Vcpkg -Force | Out-Null
            Invoke-Logged "git.exe" @("init", $Vcpkg)
            Invoke-Logged "git.exe" @("-C", $Vcpkg, "remote", "add", "origin", "https://github.com/microsoft/vcpkg.git")
        }
        Invoke-Logged "git.exe" @("-C", $Vcpkg, "fetch", "--depth", "1", "origin", $VcpkgCommit)
        Invoke-Logged "git.exe" @("-C", $Vcpkg, "checkout", "--detach", "--force", $VcpkgCommit)

        $VcpkgExe = Join-Path $Vcpkg "vcpkg.exe"
        if (!(Test-Path -LiteralPath $VcpkgExe)) {
            Invoke-Logged (Join-Path $Vcpkg "bootstrap-vcpkg.bat") @("-disableMetrics") $Vcpkg
        }
        # vcpkg installs Fast DDS v3.6.2 and all its native dependencies.
        Invoke-Logged $VcpkgExe @(
            "install", "fastdds:$Triplet",
            "--x-install-root=$Install", "--disable-metrics"
        ) $Vcpkg

        if (!(Test-Path (Join-Path $Generator ".git"))) {
            $Sources = Join-Path $Cache "sources"
            New-Item -ItemType Directory -Path $Sources -Force | Out-Null
            Invoke-Logged "git.exe" @(
                "clone", "--recursive", "--depth", "1", "--branch", $GeneratorTag,
                "https://github.com/eProsima/Fast-DDS-Gen.git", $Generator
            )
        }
        Invoke-Logged "git.exe" @("-C", $Generator, "submodule", "update", "--init", "--recursive")
        Invoke-Logged (Join-Path $Generator "gradlew.bat") @("--no-daemon", "assemble") $Generator

        if (!(Test-Path (Join-Path $Generator "share\fastddsgen\java\fastddsgen.jar"))) {
            throw "Fast DDS-Gen jar not produced. See $Log"
        }
        Set-Content -LiteralPath $Marker -Value $Expected -Encoding ascii
    }

    if (!(Test-Ready)) { throw "Fast DDS bootstrap is incomplete. Inspect $Log" }
    Write-Output "D2S_PREFIX=$Prefix"
    Write-Output "D2S_FASTDDSGEN=$GeneratorCommand"
    Write-Output "D2S_BINDIR=$(Join-Path $Prefix 'bin')"
} finally {
    if ($null -ne $Lock) { $Lock.Dispose() }
}
