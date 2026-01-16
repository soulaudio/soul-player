# Local build testing script for Windows (PowerShell)
# Emulates CI build pipeline using Docker

param(
    [Parameter(Position=0)]
    [ValidateSet("linux", "windows", "all")]
    [string]$Platform = "all",

    [Parameter(Position=1)]
    [ValidateSet("check", "test", "build", "packages")]
    [string]$BuildType = "packages",

    [switch]$SkipDockerBuild,
    [switch]$KeepContainer,
    [switch]$Verbose,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

function Print-Header {
    param([string]$Message)
    Write-Host ""
    Write-Host "============================================" -ForegroundColor Blue
    Write-Host $Message -ForegroundColor Blue
    Write-Host "============================================" -ForegroundColor Blue
    Write-Host ""
}

function Print-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Print-Error {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

function Print-Warning {
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
}

function Show-Help {
    @"
Usage: .\local-build-test.ps1 [PLATFORM] [BUILD_TYPE] [OPTIONS]

PLATFORM:
  linux           Build Linux packages only
  windows         Build Windows packages only (cross-compilation)
  all             Build all platforms (default)

BUILD_TYPE:
  check           Run cargo check and clippy only
  test            Run cargo test
  build           Build Rust binaries only
  packages        Build full installers/packages (default)

OPTIONS:
  -SkipDockerBuild    Skip rebuilding Docker images
  -KeepContainer      Don't remove container after build
  -Verbose            Show detailed output
  -Help               Show this help message

Examples:
  .\local-build-test.ps1 linux check
  .\local-build-test.ps1 windows build
  .\local-build-test.ps1 all packages

"@
    exit 0
}

if ($Help) {
    Show-Help
}

# Get script directory and project root
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
$DockerDir = Join-Path $ProjectRoot "docker\build-env"

# Check if Docker is installed
if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Print-Error "Docker is not installed. Please install Docker Desktop."
    exit 1
}

# Check if Docker is running
try {
    docker ps | Out-Null
} catch {
    Print-Error "Docker is not running. Please start Docker Desktop."
    exit 1
}

function Build-DockerImages {
    param([string]$TargetPlatform)

    Print-Header "Building Docker Images for $TargetPlatform"

    if ($SkipDockerBuild) {
        Print-Warning "Skipping Docker build (-SkipDockerBuild)"
        return
    }

    Push-Location $DockerDir

    try {
        if ($TargetPlatform -eq "linux" -or $TargetPlatform -eq "all") {
            docker compose build linux-builder
            Print-Success "Linux builder image ready"
        }

        if ($TargetPlatform -eq "windows" -or $TargetPlatform -eq "all") {
            docker compose build windows-cross-builder
            Print-Success "Windows cross-builder image ready"
        }
    } finally {
        Pop-Location
    }
}

function Invoke-Build {
    param(
        [string]$TargetPlatform,
        [string]$TargetBuildType
    )

    $serviceName = switch ($TargetPlatform) {
        "linux" { "linux-builder" }
        "windows" { "windows-cross-builder" }
        default {
            Print-Error "Unknown platform: $TargetPlatform"
            return $false
        }
    }

    Print-Header "Building $TargetPlatform ($TargetBuildType)"

    $dockerOpts = @()
    if (-not $KeepContainer) {
        $dockerOpts += "--rm"
    }

    $buildCmd = switch ($TargetBuildType) {
        "check" { "./scripts/ci-build-check.sh" }
        "test" { "./scripts/ci-build-test.sh" }
        "build" { "./scripts/ci-build-binary.sh $TargetPlatform" }
        "packages" { "./scripts/ci-build-packages.sh $TargetPlatform" }
        default {
            Print-Error "Unknown build type: $TargetBuildType"
            return $false
        }
    }

    Push-Location $DockerDir

    try {
        $profile = $TargetPlatform.Split('-')[0]

        if ($Verbose) {
            docker compose run @dockerOpts --profile $profile $serviceName bash -c $buildCmd
        } else {
            $logFile = Join-Path $ProjectRoot "build-$TargetPlatform.log"
            docker compose run @dockerOpts --profile $profile $serviceName bash -c $buildCmd 2>&1 | Tee-Object -FilePath $logFile
        }

        if ($LASTEXITCODE -eq 0) {
            Print-Success "$TargetPlatform build completed successfully"
            return $true
        } else {
            Print-Error "$TargetPlatform build failed (exit code: $LASTEXITCODE)"
            if (-not $Verbose) {
                Print-Warning "Check build-$TargetPlatform.log for details"
            }
            return $false
        }
    } finally {
        Pop-Location
    }
}

# Main execution
Print-Header "Soul Player - Local Build Testing"
Write-Host "Platform: $Platform"
Write-Host "Build Type: $BuildType"
Write-Host ""

# Build Docker images
Build-DockerImages $Platform

# Run builds
$failed = 0

if ($Platform -eq "linux" -or $Platform -eq "all") {
    if (-not (Invoke-Build "linux" $BuildType)) {
        $failed++
    }
}

if ($Platform -eq "windows" -or $Platform -eq "all") {
    if (-not (Invoke-Build "windows" $BuildType)) {
        $failed++
    }
}

# Summary
Print-Header "Build Summary"
if ($failed -eq 0) {
    Print-Success "All builds completed successfully!"
    exit 0
} else {
    Print-Error "$failed build(s) failed"
    exit 1
}
