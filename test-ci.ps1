#!/usr/bin/env pwsh
# test-ci.ps1 - Run CI tests locally in Docker
# This script replicates the exact CI environment from .github/workflows/ci.yml

param(
    [switch]$Build,         # Force rebuild Docker image
    [switch]$Shell,         # Open interactive shell instead of running tests
    [switch]$Clean,         # Clean Docker volumes (cargo cache, target)
    [switch]$Clippy,        # Run clippy instead of tests
    [switch]$Format,        # Run format check instead of tests
    [string]$Package = "",  # Run tests for specific package only
    [switch]$Help           # Show help
)

$ErrorActionPreference = "Stop"

function Show-Help {
    Write-Host @"
Soul Player CI Test Runner (Docker)
====================================

Run integration tests locally in an environment that exactly matches GitHub Actions CI.

USAGE:
    .\test-ci.ps1 [OPTIONS]

OPTIONS:
    -Build          Force rebuild Docker image (use after Dockerfile.ci changes)
    -Shell          Open interactive bash shell in CI container
    -Clean          Remove all Docker volumes (cargo cache, target directory)
    -Clippy         Run clippy lints instead of tests
    -Format         Run format check instead of tests
    -Package <name> Run tests for specific package only (e.g., -Package soul-playback)
    -Help           Show this help message

EXAMPLES:
    # Run all integration tests (like CI)
    .\test-ci.ps1

    # Run tests for a specific package
    .\test-ci.ps1 -Package soul-playback

    # Run clippy (lint check)
    .\test-ci.ps1 -Clippy

    # Run format check
    .\test-ci.ps1 -Format

    # Open interactive shell for debugging
    .\test-ci.ps1 -Shell

    # Rebuild image after Dockerfile changes
    .\test-ci.ps1 -Build

    # Clean all caches and rebuild
    .\test-ci.ps1 -Clean -Build

PERFORMANCE:
    First run: ~5-10 minutes (downloads dependencies, compiles)
    Subsequent runs: ~1-3 minutes (cached dependencies and target)

VOLUMES (persistent caches):
    - cargo-registry:   Downloaded crates
    - cargo-git:        Git dependencies
    - cargo-target-ci:  Compiled artifacts (target/ directory)

To clean volumes: .\test-ci.ps1 -Clean

"@
    exit 0
}

if ($Help) {
    Show-Help
}

# Ensure Docker is running
Write-Host "Checking Docker..." -ForegroundColor Blue
try {
    docker info | Out-Null
} catch {
    Write-Host "ERROR: Docker is not running. Please start Docker Desktop." -ForegroundColor Red
    exit 1
}

# Clean volumes if requested
if ($Clean) {
    Write-Host "Cleaning Docker volumes..." -ForegroundColor Yellow
    docker compose -f docker-compose.ci.yml down -v
    Write-Host "Volumes cleaned." -ForegroundColor Green
    if (-not $Build) {
        exit 0
    }
}

# Build image if requested or if it doesn't exist
$imageName = "soul-player-ci-test"
$imageExists = docker images -q $imageName
if ($Build -or -not $imageExists) {
    Write-Host "Building Docker image..." -ForegroundColor Blue
    docker compose -f docker-compose.ci.yml build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Docker build failed." -ForegroundColor Red
        exit 1
    }
    Write-Host "Image built successfully." -ForegroundColor Green
}

# Open interactive shell if requested
if ($Shell) {
    Write-Host "Opening interactive shell..." -ForegroundColor Blue
    docker compose -f docker-compose.ci.yml run --rm ci-test bash
    exit $LASTEXITCODE
}

# Determine command to run
$command = ""
if ($Clippy) {
    Write-Host "Running clippy..." -ForegroundColor Blue
    $command = @"
mkdir -p applications/desktop/dist && \
echo '<!DOCTYPE html><html><head><title>Soul Player</title></head><body><div id=\"root\">Loading...</div></body></html>' > applications/desktop/dist/index.html && \
cargo clippy --workspace --lib --bins --release -- -D warnings
"@
} elseif ($Format) {
    Write-Host "Running format check..." -ForegroundColor Blue
    $command = "cargo fmt --all --check"
} elseif ($Package) {
    Write-Host "Running tests for package: $Package" -ForegroundColor Blue
    $command = @"
mkdir -p applications/desktop/dist && \
echo '<!DOCTYPE html><html><head><title>Soul Player</title></head><body><div id=\"root\">Loading...</div></body></html>' > applications/desktop/dist/index.html && \
cargo test --tests --release -p $Package -- --test-threads=1
"@
} else {
    Write-Host "Running all integration tests..." -ForegroundColor Blue
    # Use default command from Dockerfile
    $command = ""
}

# Run the container
if ($command) {
    docker compose -f docker-compose.ci.yml run --rm ci-test bash -c $command
} else {
    docker compose -f docker-compose.ci.yml up --abort-on-container-exit
}

$exitCode = $LASTEXITCODE

# Show result
if ($exitCode -eq 0) {
    Write-Host "`n✅ Success! All checks passed." -ForegroundColor Green
} else {
    Write-Host "`n❌ Failed. Exit code: $exitCode" -ForegroundColor Red
}

exit $exitCode
