# Test script for Phase 2 check commands
Write-Host "Testing Phase 2 Check Commands" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

$commands = @(
    @{ Name = "fmt"; Command = "cargo xtask check fmt" },
    @{ Name = "clippy"; Command = "cargo xtask check clippy"; Skip = $true },  # Takes too long
    @{ Name = "test (single package)"; Command = "cargo xtask check test -p soul-storage"; Skip = $true },  # Takes too long
    @{ Name = "typescript"; Command = "cargo xtask check typescript"; NeedsYarn = $true },
    @{ Name = "lint"; Command = "cargo xtask check lint"; NeedsYarn = $true }
)

foreach ($cmd in $commands) {
    Write-Host "Testing: $($cmd.Name)" -ForegroundColor Yellow
    
    if ($cmd.Skip) {
        Write-Host "  [SKIPPED - Run manually to verify]" -ForegroundColor Gray
        continue
    }
    
    if ($cmd.NeedsYarn -and -not (Test-Path "node_modules")) {
        Write-Host "  [SKIPPED - Requires 'yarn install' first]" -ForegroundColor Gray
        continue
    }
    
    $result = Invoke-Expression "$($cmd.Command) 2>&1"
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "  [PASS]" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL]" -ForegroundColor Red
        Write-Host "  Output: $result" -ForegroundColor Red
    }
    Write-Host ""
}

Write-Host "Test Complete!" -ForegroundColor Cyan
