#!/usr/bin/env pwsh
# Cleanup obsolete temporary files from past development sessions

Write-Host "[CLEANUP] Removing obsolete files..." -ForegroundColor Cyan

$filesToRemove = @(
    # Root-level temporary fix/debug files from past sessions
    "ASYNC_TASK_ERROR_HANDLING_FIXES.md",
    "CI_TESTING.md",
    "COMPILATION_FIXES_SUMMARY.md",
    "COMPILATION_STATUS.md",
    "DEAD_CODE_CLEANUP.md",
    "DEVICE_EVENT_DEDUPLICATION.md",
    "DEVICE_MONITOR_CANCELLATION_FIXES.md",
    "DEVICE_MONITORING_FINAL.md",
    "HOVER_EFFECTS_GUIDE.md",
    "LINUX_DEFAULT_DEVICE_FIX.md",
    "MACOS_USE_AFTER_FREE_FIX.md",
    "OPTIMIZATION_SUMMARY.md",
    "PRODUCTION_DEVICE_MONITORING.md",
    "SILENT_ERROR_SUPPRESSION_FIX.md",
    "SILENT_MODE_AND_LOGGING_IMPLEMENTATION.md",
    "ZERO_DEVICE_SILENT_MODE.md",

    # Marketing app temporary session notes
    "applications/marketing/AUDIOPHILE_SHOWCASE.md",
    "applications/marketing/FEATURE_SHOWCASES.md",
    "applications/marketing/PLAYBACK_ARCHITECTURE.md",
    "applications/marketing/PLAYBACK_FIXED.md",
    "applications/marketing/PROVIDERS_FIXED.md",
    "applications/marketing/QUICK_REFERENCE.md",
    "applications/marketing/RESTART_INSTRUCTIONS.md",
    "applications/marketing/SCROLL_ROTATE_3D_INTEGRATION.md",
    "applications/marketing/SCROLL_ROTATE_3D_SUMMARY.md",
    "applications/marketing/SHOWCASES_COMPLETE.md",
    "applications/marketing/SHOWCASES_READY.md",
    "applications/marketing/TESTING.md",
    "applications/marketing/WASM_BUILD_INTEGRATION.md",
    "applications/marketing/src/components/features/LocalFirstShowcase.md",

    # One-off Python migration scripts (no longer needed)
    "scripts/fix_all_sqlx.py",
    "scripts/fix_fetch_one.py",
    "scripts/fix_soul_sync.py",
    "scripts/fix_sqlx_types.py",
    "scripts/fix_sqlx_types_v2.py"
)

$dirsToRemove = @(
    # Stale WSL build artifacts
    "libraries/soul-audio-desktop/target-wsl"
)

$removedCount = 0

Write-Host ""
Write-Host "Removing files..." -ForegroundColor Yellow

foreach ($file in $filesToRemove) {
    if (Test-Path $file) {
        Remove-Item -Path $file -Force -ErrorAction SilentlyContinue
        Write-Host "   [OK] Removed $file" -ForegroundColor Green
        $removedCount++
    }
}

Write-Host ""
Write-Host "Removing directories..." -ForegroundColor Yellow

foreach ($dir in $dirsToRemove) {
    if (Test-Path $dir) {
        Remove-Item -Path $dir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "   [OK] Removed $dir/" -ForegroundColor Green
        $removedCount++
    }
}

Write-Host ""
if ($removedCount -gt 0) {
    Write-Host "[DONE] Removed $removedCount obsolete files/directories" -ForegroundColor Green
} else {
    Write-Host "[INFO] No obsolete files found (already clean)" -ForegroundColor Blue
}

Write-Host ""
Write-Host "Kept important documentation:" -ForegroundColor Blue
Write-Host "   - CLAUDE.md (project instructions)" -ForegroundColor White
Write-Host "   - README.md (project overview)" -ForegroundColor White
Write-Host "   - CONTRIBUTING.md (contribution guide)" -ForegroundColor White
Write-Host "   - RELEASING.md (release process)" -ForegroundColor White
Write-Host "   - ROADMAP.md (future plans)" -ForegroundColor White
Write-Host "   - docs/ (architecture & guides)" -ForegroundColor White
Write-Host "   - docs/archive/session-notes/ (archived session notes)" -ForegroundColor White
Write-Host ""
