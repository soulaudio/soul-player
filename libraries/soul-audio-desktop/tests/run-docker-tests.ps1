# Run Docker-based E2E tests for device switching (Windows PowerShell)

$ErrorActionPreference = "Stop"

Write-Host "========================================"
Write-Host "Soul Audio Desktop - Docker E2E Tests"
Write-Host "========================================"
Write-Host ""

# Check if Docker is available
try {
    $null = Get-Command docker -ErrorAction Stop
    Write-Host "✓ Docker is available"
} catch {
    Write-Host "ERROR: Docker is not installed or not in PATH" -ForegroundColor Red
    Write-Host "Please install Docker Desktop and try again"
    exit 1
}

# Check if Docker daemon is running
try {
    $null = docker info 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Docker daemon not running"
    }
} catch {
    Write-Host "ERROR: Docker daemon is not running" -ForegroundColor Red
    Write-Host "Please start Docker Desktop and try again"
    exit 1
}

Write-Host ""

# Build the audio test Docker image
Write-Host "Building audio test Docker image..."
$dockerPath = Join-Path $PSScriptRoot "docker"
Push-Location $dockerPath
docker build -t soul-audio-test:latest -f Dockerfile.audio-test .
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Failed to build Docker image" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location
Write-Host "✓ Docker image built successfully"
Write-Host ""

# Go to library root
$libRoot = Split-Path -Parent $PSScriptRoot

# Run the Docker-based E2E tests
Write-Host "Running Docker E2E tests..."
Write-Host ""
Push-Location $libRoot
cargo test --features docker-tests --test e2e_device_switching_docker -- --nocapture
$testResult = $LASTEXITCODE
Pop-Location

Write-Host ""
if ($testResult -eq 0) {
    Write-Host "========================================"
    Write-Host "All Docker E2E tests completed!"
    Write-Host "========================================"
} else {
    Write-Host "========================================"
    Write-Host "Some tests failed - see output above"
    Write-Host "========================================"
    exit $testResult
}
