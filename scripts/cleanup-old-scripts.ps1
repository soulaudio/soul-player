#!/usr/bin/env pwsh
# Script to delete obsolete scripts replaced by cargo xtask
# Run with -DryRun to preview, -Backup to create backup

param(
    [switch]$Backup,
    [switch]$DryRun,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Usage: cleanup-old-scripts.ps1 [OPTIONS]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -DryRun    Preview what will be deleted without actually deleting"
    Write-Host "  -Backup    Create backup before deletion"
    Write-Host "  -Help      Show this help message"
    exit 0
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

Write-Host "🗑️  Cleanup Old Scripts (Replaced by cargo xtask)" -ForegroundColor Cyan
Write-Host "==================================================" -ForegroundColor Cyan
Write-Host ""

# List of scripts to delete with their xtask equivalents
$ScriptsToDelete = @{
    # Build scripts
    "scripts/build.sh" = "cargo xtask build"
    "scripts/pre-build.sh" = "cargo xtask build (integrated)"

    # Setup scripts
    "scripts/setup-sqlx.sh" = "cargo xtask setup sqlx"
    "scripts/install-deps.sh" = "cargo xtask setup deps"
    "scripts/install-deps.ps1" = "cargo xtask setup deps"

    # Pre-commit scripts
    "scripts/pre-commit-check.sh" = "cargo xtask check precommit"
    "scripts/pre-commit-check.ps1" = "cargo xtask check precommit"

    # Development scripts
    "scripts/clean-dev.sh" = "cargo xtask clean dev"
    "scripts/clean-dev.ps1" = "cargo xtask clean dev"

    # Version management
    "scripts/bump-version.sh" = "cargo xtask version bump"
    "scripts/bump-version.mjs" = "cargo xtask version bump (core logic)"

    # WASM scripts
    "scripts/build-wasm.mjs" = "cargo xtask wasm build"
    "scripts/watch-wasm.mjs" = "cargo xtask wasm watch"

    # Test scripts
    "scripts/test-cache-e2e.sh" = "cargo xtask test cache"
    "scripts/test-import-e2e.sh" = "cargo xtask test import"
    "scripts/validate-e2e-setup.sh" = "cargo xtask test validate"
    "scripts/validate-e2e-setup.ps1" = "cargo xtask test validate"
    "scripts/validate-workflows.sh" = "cargo xtask test workflows"

    # Audio test scripts
    "scripts/generate-test-audio.sh" = "cargo xtask test audio generate"
    "scripts/generate-test-audio.ps1" = "cargo xtask test audio generate"
    "scripts/setup-virtual-audio.sh" = "cargo xtask test audio setup"
    "scripts/setup-virtual-audio.ps1" = "cargo xtask test audio setup"

    # Obsolete build artifacts
    "scripts/generate_test_audio_rust.exe" = "(generated file, not needed)"
    "scripts/generate_test_audio_rust.pdb" = "(generated file, not needed)"
    "scripts/generate_test_audio_rust.rs" = "(migrated to xtask)"

    # Old migration/fix scripts (one-time use)
    "scripts/fix_all_sqlx.py" = "(one-time migration, obsolete)"
    "scripts/fix_fetch_one.py" = "(one-time migration, obsolete)"
    "scripts/fix_soul_sync.py" = "(one-time migration, obsolete)"
    "scripts/fix_sqlx_types.py" = "(one-time migration, obsolete)"
    "scripts/fix_sqlx_types_v2.py" = "(one-time migration, obsolete)"
    "scripts/replace-console-logs.sh" = "(one-time migration, obsolete)"
    "scripts/seed-test-data.ts" = "(one-time migration, obsolete)"
}

# Scripts to KEEP (explicitly excluded)
$KeepScripts = @(
    "scripts/seed-test-data.js"     # Uses better-sqlite3, complex to port
    "scripts/inspect-demo.mjs"      # Low priority utility
    "scripts/cleanup-old-scripts.sh" # Unix version
    "scripts/cleanup-old-scripts.ps1" # This script
    "scripts/cleanup-obsolete-files.ps1" # Different cleanup script
)

# Directories to delete
$DirsToDelete = @(
    "scripts/tests"  # Replaced by xtask test commands
)

Set-Location $ProjectRoot

Write-Host "📋 Scripts to be deleted (with xtask equivalents):" -ForegroundColor Yellow
Write-Host ""

$FoundCount = 0
$MissingCount = 0

foreach ($script in $ScriptsToDelete.Keys | Sort-Object) {
    $xtaskCmd = $ScriptsToDelete[$script]
    $path = Join-Path $ProjectRoot $script

    if (Test-Path $path) {
        Write-Host "  ✓ $script" -ForegroundColor Green
        Write-Host "    → $xtaskCmd" -ForegroundColor Gray
        $FoundCount++
    } else {
        Write-Host "  ⊗ $script (already deleted)" -ForegroundColor DarkGray
        $MissingCount++
    }
}

Write-Host ""
Write-Host "📁 Directories to be deleted:" -ForegroundColor Yellow
foreach ($dir in $DirsToDelete) {
    $path = Join-Path $ProjectRoot $dir

    if (Test-Path $path) {
        $fileCount = (Get-ChildItem -Path $path -Recurse -File).Count
        Write-Host "  ✓ $dir ($fileCount files)" -ForegroundColor Green
        $FoundCount++
    } else {
        Write-Host "  ⊗ $dir (already deleted)" -ForegroundColor DarkGray
        $MissingCount++
    }
}

Write-Host ""
Write-Host "📌 Scripts being KEPT:" -ForegroundColor Cyan
foreach ($script in $KeepScripts) {
    $path = Join-Path $ProjectRoot $script

    if (Test-Path $path) {
        Write-Host "  ✓ $script" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "Summary:" -ForegroundColor White
Write-Host "  - Files/dirs to delete: $FoundCount" -ForegroundColor Yellow
Write-Host "  - Already deleted: $MissingCount" -ForegroundColor DarkGray

if ($FoundCount -eq 0) {
    Write-Host ""
    Write-Host "✨ No files to delete. Cleanup already complete!" -ForegroundColor Green
    exit 0
}

if ($DryRun) {
    Write-Host ""
    Write-Host "🔍 DRY RUN: No files will be deleted" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Run without -DryRun to actually delete files" -ForegroundColor Yellow
    Write-Host "Use -Backup to create a backup before deletion" -ForegroundColor Yellow
    exit 0
}

Write-Host ""
Write-Host "⚠️  WARNING: This will permanently delete $FoundCount files/directories" -ForegroundColor Red

if ($Backup) {
    Write-Host "   A backup will be created in .backup/old-scripts-$(Get-Date -Format 'yyyyMMdd')/" -ForegroundColor Yellow
}

Write-Host ""
$response = Read-Host "Continue? (yes/no)"
if ($response -notmatch '^[Yy][Ee][Ss]$') {
    Write-Host "Cancelled." -ForegroundColor Yellow
    exit 0
}

# Create backup if requested
if ($Backup) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $BackupDir = Join-Path $ProjectRoot ".backup/old-scripts-$timestamp"

    Write-Host ""
    Write-Host "📦 Creating backup in $BackupDir..." -ForegroundColor Cyan
    New-Item -ItemType Directory -Path $BackupDir -Force | Out-Null

    foreach ($script in $ScriptsToDelete.Keys) {
        $sourcePath = Join-Path $ProjectRoot $script

        if (Test-Path $sourcePath) {
            $scriptDir = Split-Path -Parent $script
            $backupScriptDir = Join-Path $BackupDir $scriptDir
            New-Item -ItemType Directory -Path $backupScriptDir -Force | Out-Null

            Copy-Item -Path $sourcePath -Destination (Join-Path $BackupDir $script) -Force
            Write-Host "  ✓ Backed up: $script" -ForegroundColor Green
        }
    }

    foreach ($dir in $DirsToDelete) {
        $sourcePath = Join-Path $ProjectRoot $dir

        if (Test-Path $sourcePath) {
            $dirParent = Split-Path -Parent $dir
            $backupDirParent = Join-Path $BackupDir $dirParent
            New-Item -ItemType Directory -Path $backupDirParent -Force | Out-Null

            Copy-Item -Path $sourcePath -Destination (Join-Path $BackupDir $dir) -Recurse -Force
            Write-Host "  ✓ Backed up: $dir/" -ForegroundColor Green
        }
    }

    Write-Host "  Backup complete!" -ForegroundColor Green
}

# Delete files
Write-Host ""
Write-Host "🗑️  Deleting obsolete scripts..." -ForegroundColor Cyan
$DeletedCount = 0

foreach ($script in $ScriptsToDelete.Keys) {
    $path = Join-Path $ProjectRoot $script

    if (Test-Path $path) {
        Remove-Item -Path $path -Force
        Write-Host "  ✓ Deleted: $script" -ForegroundColor Green
        $DeletedCount++
    }
}

foreach ($dir in $DirsToDelete) {
    $path = Join-Path $ProjectRoot $dir

    if (Test-Path $path) {
        Remove-Item -Path $path -Recurse -Force
        Write-Host "  ✓ Deleted: $dir/" -ForegroundColor Green
        $DeletedCount++
    }
}

Write-Host ""
Write-Host "✅ Deleted $DeletedCount files/directories" -ForegroundColor Green

# Verification
Write-Host ""
Write-Host "🔍 Running verification checks..." -ForegroundColor Cyan
Write-Host ""

$VerificationFailed = $false

# Check 1: xtask is available
Write-Host "1. Checking cargo xtask is available..." -ForegroundColor White
try {
    $null = cargo xtask --help 2>&1
    Write-Host "   ✓ cargo xtask is available" -ForegroundColor Green
} catch {
    Write-Host "   ✗ cargo xtask not found!" -ForegroundColor Red
    $VerificationFailed = $true
}

# Check 2: Version command works
Write-Host "2. Checking version command..." -ForegroundColor White
try {
    $versionOutput = cargo xtask version current 2>&1 | Out-String
    if ($versionOutput -match '\d+\.\d+\.\d+') {
        $version = $matches[0]
        Write-Host "   ✓ Version command works (current: $version)" -ForegroundColor Green
    } else {
        throw "No version found"
    }
} catch {
    Write-Host "   ✗ Version command failed!" -ForegroundColor Red
    $VerificationFailed = $true
}

# Check 3: Check command works
Write-Host "3. Checking precommit check..." -ForegroundColor White
try {
    $null = cargo xtask check precommit --help 2>&1
    Write-Host "   ✓ Precommit check available" -ForegroundColor Green
} catch {
    Write-Host "   ✗ Precommit check failed!" -ForegroundColor Red
    $VerificationFailed = $true
}

# Check 4: Clean command works
Write-Host "4. Checking clean command..." -ForegroundColor White
try {
    $null = cargo xtask clean dev --help 2>&1
    Write-Host "   ✓ Clean command available" -ForegroundColor Green
} catch {
    Write-Host "   ✗ Clean command failed!" -ForegroundColor Red
    $VerificationFailed = $true
}

# Check 5: WASM commands work
Write-Host "5. Checking WASM commands..." -ForegroundColor White
try {
    $null = cargo xtask wasm build --help 2>&1
    Write-Host "   ✓ WASM commands available" -ForegroundColor Green
} catch {
    Write-Host "   ✗ WASM commands failed!" -ForegroundColor Red
    $VerificationFailed = $true
}

# Check 6: Test commands work
Write-Host "6. Checking test commands..." -ForegroundColor White
try {
    $null = cargo xtask test --help 2>&1
    Write-Host "   ✓ Test commands available" -ForegroundColor Green
} catch {
    Write-Host "   ✗ Test commands failed!" -ForegroundColor Red
    $VerificationFailed = $true
}

Write-Host ""

if ($VerificationFailed) {
    Write-Host "❌ Verification FAILED!" -ForegroundColor Red
    if ($Backup) {
        Write-Host ""
        Write-Host "To restore from backup:" -ForegroundColor Yellow
        Write-Host "  Copy-Item -Path $BackupDir\* -Destination . -Recurse -Force" -ForegroundColor Yellow
    }
    exit 1
} else {
    Write-Host "✅ All verification checks passed!" -ForegroundColor Green
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "🎉 Cleanup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Old scripts deleted: $DeletedCount" -ForegroundColor Yellow
if ($Backup) {
    Write-Host "Backup location: $BackupDir" -ForegroundColor Cyan
}
Write-Host ""
Write-Host "Use 'cargo xtask --help' to see all available commands" -ForegroundColor White
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
