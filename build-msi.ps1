#!/usr/bin/env pwsh
# Build MSI with verbose output

Write-Host "=== Starting MSI Build ===" -ForegroundColor Green
Write-Host "Time: $(Get-Date)" -ForegroundColor Cyan

Set-Location applications/desktop

Write-Host "`nStep 1: Building frontend..." -ForegroundColor Yellow
yarn build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Frontend build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "`nStep 2: Copying dist to src-tauri..." -ForegroundColor Yellow
node copy-dist.cjs
if ($LASTEXITCODE -ne 0) {
    Write-Host "Copy dist failed!" -ForegroundColor Red
    exit 1
}

Write-Host "`nStep 3: Building Tauri MSI (this will take 10-15 minutes)..." -ForegroundColor Yellow
yarn tauri build --bundles msi
if ($LASTEXITCODE -ne 0) {
    Write-Host "Tauri build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "`n=== Build Complete ===" -ForegroundColor Green
Write-Host "Looking for MSI..." -ForegroundColor Cyan

$msi = Get-ChildItem -Path "src-tauri/target/release/bundle/msi" -Filter "*.msi" -ErrorAction SilentlyContinue | Select-Object -First 1

if ($msi) {
    Write-Host "`nMSI Location: $($msi.FullName)" -ForegroundColor Green
    Write-Host "Size: $([math]::Round($msi.Length / 1MB, 2)) MB" -ForegroundColor Cyan
} else {
    Write-Host "MSI not found!" -ForegroundColor Red
}
