# =============================================================================
# Test Docker Build for Soul Server (PowerShell)
# =============================================================================
# This script emulates the CI Docker build environment locally.
# It runs the same Docker build command that GitHub Actions uses.
#
# Usage:
#   .\scripts\test-docker-build.ps1 [-NoCache] [-Platform PLATFORM]
#
# Options:
#   -NoCache          Build without using Docker cache
#   -Platform         Build for specific platform (e.g., linux/amd64, linux/arm64)
#   -Help             Show this help message
# =============================================================================

param(
    [switch]$NoCache,
    [string]$Platform = "",
    [switch]$Help
)

# Show help if requested
if ($Help) {
    Get-Content $PSCommandPath | Select-Object -First 15 | Select-Object -Skip 2 | ForEach-Object { $_ -replace "^# ", "" }
    exit 0
}

# Check if we're in the project root
if (-not (Test-Path "Cargo.toml") -or -not (Test-Path "applications\server")) {
    Write-Host "Error: This script must be run from the project root directory" -ForegroundColor Red
    exit 1
}

$ImageName = "soul-server:local-test"

Write-Host "==============================================================================" -ForegroundColor Blue
Write-Host "Soul Player Server - Local Docker Build Test" -ForegroundColor Blue
Write-Host "==============================================================================" -ForegroundColor Blue
Write-Host ""
Write-Host "This will build the Docker image exactly as CI does." -ForegroundColor Yellow
Write-Host "Build options:" -ForegroundColor Yellow
Write-Host "  Image name: " -NoNewline; Write-Host $ImageName -ForegroundColor Green
Write-Host "  No cache:   " -NoNewline; Write-Host $(if ($NoCache) { "Yes" } else { "No" }) -ForegroundColor Green
Write-Host "  Platform:   " -NoNewline; Write-Host $(if ($Platform) { $Platform } else { "Default (current architecture)" }) -ForegroundColor Green
Write-Host ""
Write-Host "Starting build..." -ForegroundColor Yellow
Write-Host ""

# Build the Docker command
$dockerArgs = @(
    "build"
    "-f", "applications/server/Dockerfile"
    "-t", $ImageName
)

if ($NoCache) {
    $dockerArgs += "--no-cache"
}

if ($Platform) {
    $dockerArgs += "--platform", $Platform
}

$dockerArgs += "."

# Run the Docker build
$buildSuccess = $false
try {
    docker @dockerArgs
    if ($LASTEXITCODE -eq 0) {
        $buildSuccess = $true
    }
} catch {
    $buildSuccess = $false
}

Write-Host ""

if ($buildSuccess) {
    Write-Host "==============================================================================" -ForegroundColor Green
    Write-Host "✓ Docker build completed successfully!" -ForegroundColor Green
    Write-Host "==============================================================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "Image details:" -ForegroundColor Blue
    docker images $ImageName --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedAt}}"
    Write-Host ""
    Write-Host "To run the server:" -ForegroundColor Yellow
    Write-Host "  docker run -p 8080:8080 -v `${PWD}/data:/app/data $ImageName" -ForegroundColor Green
    Write-Host ""
    Write-Host "To test the build for multiple platforms (as CI does):" -ForegroundColor Yellow
    Write-Host "  .\scripts\test-docker-build.ps1 -Platform linux/amd64" -ForegroundColor Green
    Write-Host "  .\scripts\test-docker-build.ps1 -Platform linux/arm64" -ForegroundColor Green
    Write-Host ""
} else {
    Write-Host "==============================================================================" -ForegroundColor Red
    Write-Host "✗ Docker build failed!" -ForegroundColor Red
    Write-Host "==============================================================================" -ForegroundColor Red
    Write-Host ""
    Write-Host "Common issues:" -ForegroundColor Yellow
    Write-Host "  1. Ensure Docker is running"
    Write-Host "  2. Check that you have enough disk space"
    Write-Host "  3. Try running with -NoCache if you suspect cache issues"
    Write-Host "  4. Ensure all required files exist (check .dockerignore)"
    Write-Host ""
    exit 1
}
