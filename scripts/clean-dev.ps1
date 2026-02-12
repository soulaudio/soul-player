#!/usr/bin/env pwsh
# Clean all development artifacts for Soul Player

Write-Host "[CLEAN] Cleaning Soul Player development artifacts..." -ForegroundColor Cyan

# Stop any running processes
Write-Host ""
Write-Host "1. Stopping running processes..." -ForegroundColor Yellow
Get-Process -Name "soul-player*" -ErrorAction SilentlyContinue | Stop-Process -Force
Get-Process -Name "vite" -ErrorAction SilentlyContinue | Stop-Process -Force

# Clean Rust build artifacts
Write-Host ""
Write-Host "2. Cleaning Rust target directory..." -ForegroundColor Yellow
if (Test-Path "target") {
    Remove-Item -Path "target" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "   [OK] Removed target/" -ForegroundColor Green
}

if (Test-Path "applications/desktop/src-tauri/target") {
    Remove-Item -Path "applications/desktop/src-tauri/target" -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "   [OK] Removed applications/desktop/src-tauri/target/" -ForegroundColor Green
}

# Clean frontend dist folders
Write-Host ""
Write-Host "3. Cleaning frontend dist folders..." -ForegroundColor Yellow
$distFolders = @(
    "applications/desktop/dist",
    "applications/desktop/src-tauri/dist",
    "applications/marketing/dist",
    "applications/web/dist",
    "applications/mobile/dist"
)

foreach ($folder in $distFolders) {
    if (Test-Path $folder) {
        Remove-Item -Path $folder -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "   [OK] Removed $folder" -ForegroundColor Green
    }
}

# Clean node_modules cache (optional - uncomment if needed)
# Write-Host ""
# Write-Host "4. Cleaning node_modules cache..." -ForegroundColor Yellow
# Remove-Item -Path "node_modules/.cache" -Recurse -Force -ErrorAction SilentlyContinue
# Remove-Item -Path "applications/*/node_modules/.cache" -Recurse -Force -ErrorAction SilentlyContinue

# Clean Yarn cache
Write-Host ""
Write-Host "4. Cleaning Yarn cache..." -ForegroundColor Yellow
yarn cache clean --all 2>$null

# Clean SQLx offline data (if regeneration needed)
Write-Host ""
Write-Host "5. Checking SQLx offline data..." -ForegroundColor Yellow
if (Test-Path "libraries/soul-storage/.sqlx") {
    Write-Host "   [WARN] SQLx offline data found. To regenerate, run:" -ForegroundColor Yellow
    Write-Host "   cargo sqlx prepare -- --lib" -ForegroundColor Cyan
}

Write-Host ""
Write-Host "[DONE] Cleanup complete! Run 'yarn dev:desktop' to start fresh." -ForegroundColor Green
Write-Host ""
Write-Host "[TIPS]" -ForegroundColor Blue
Write-Host "   - First start will be slower (rebuilding everything)" -ForegroundColor White
Write-Host "   - Frontend HMR should work after Vite starts" -ForegroundColor White
Write-Host "   - Rust changes require full rebuild" -ForegroundColor White
Write-Host ""
