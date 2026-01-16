# Pre-commit quality checks (PowerShell version)
# Run this before committing to ensure CI will pass

$ErrorActionPreference = "Stop"

Write-Host "====================" -ForegroundColor Cyan
Write-Host "Pre-Commit Checks" -ForegroundColor Cyan
Write-Host "====================" -ForegroundColor Cyan
Write-Host ""

# Rust checks
Write-Host "→ Checking Rust formatting..." -ForegroundColor Yellow
cargo fmt --all --check
Write-Host "✓ Rust formatting OK" -ForegroundColor Green
Write-Host ""

Write-Host "→ Running Clippy..." -ForegroundColor Yellow
cargo clippy --workspace --lib --bins --release -- -D warnings
Write-Host "✓ Clippy OK" -ForegroundColor Green
Write-Host ""

Write-Host "→ Running Rust tests..." -ForegroundColor Yellow
cargo test --all --quiet
Write-Host "✓ Rust tests OK" -ForegroundColor Green
Write-Host ""

# TypeScript checks
Write-Host "→ TypeScript check - Desktop..." -ForegroundColor Yellow
yarn workspace @soul-player/desktop run tsc --noEmit
Write-Host "✓ Desktop TypeScript OK" -ForegroundColor Green
Write-Host ""

Write-Host "→ TypeScript check - Shared..." -ForegroundColor Yellow
yarn workspace @soul-player/shared run tsc --noEmit
Write-Host "✓ Shared TypeScript OK" -ForegroundColor Green
Write-Host ""

Write-Host "→ TypeScript check - Marketing..." -ForegroundColor Yellow
yarn workspace @soul-player/marketing run tsc --noEmit
Write-Host "✓ Marketing TypeScript OK" -ForegroundColor Green
Write-Host ""

Write-Host "→ ESLint - Desktop..." -ForegroundColor Yellow
yarn workspace @soul-player/desktop run lint
Write-Host "✓ Desktop ESLint OK" -ForegroundColor Green
Write-Host ""

Write-Host "→ ESLint - Shared..." -ForegroundColor Yellow
yarn workspace @soul-player/shared run lint
Write-Host "✓ Shared ESLint OK" -ForegroundColor Green
Write-Host ""

Write-Host "====================" -ForegroundColor Cyan
Write-Host "✓ All checks passed!" -ForegroundColor Green
Write-Host "====================" -ForegroundColor Cyan
Write-Host "Safe to commit." -ForegroundColor Green
