#!/bin/bash
# Script to delete obsolete scripts replaced by cargo xtask
# Run with --dry-run to preview, --backup to create backup

set -e

BACKUP=false
DRY_RUN=false
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Parse arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --backup) BACKUP=true; shift ;;
    --dry-run) DRY_RUN=true; shift ;;
    -h|--help)
      echo "Usage: cleanup-old-scripts.sh [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  --dry-run    Preview what will be deleted without actually deleting"
      echo "  --backup     Create backup before deletion"
      echo "  -h, --help   Show this help message"
      exit 0
      ;;
    *) echo "Unknown option: $1"; echo "Use -h for help"; exit 1 ;;
  esac
done

echo "🗑️  Cleanup Old Scripts (Replaced by cargo xtask)"
echo "=================================================="
echo ""

# List of scripts to delete with their xtask equivalents
declare -A SCRIPTS_TO_DELETE=(
  # Build scripts
  ["scripts/build.sh"]="cargo xtask build"
  ["scripts/pre-build.sh"]="cargo xtask build (integrated)"

  # Setup scripts
  ["scripts/setup-sqlx.sh"]="cargo xtask setup sqlx"
  ["scripts/install-deps.sh"]="cargo xtask setup deps"
  ["scripts/install-deps.ps1"]="cargo xtask setup deps"

  # Pre-commit scripts
  ["scripts/pre-commit-check.sh"]="cargo xtask check precommit"
  ["scripts/pre-commit-check.ps1"]="cargo xtask check precommit"

  # Development scripts
  ["scripts/clean-dev.sh"]="cargo xtask clean dev"
  ["scripts/clean-dev.ps1"]="cargo xtask clean dev"

  # Version management
  ["scripts/bump-version.sh"]="cargo xtask version bump"
  ["scripts/bump-version.mjs"]="cargo xtask version bump (core logic)"

  # WASM scripts
  ["scripts/build-wasm.mjs"]="cargo xtask wasm build"
  ["scripts/watch-wasm.mjs"]="cargo xtask wasm watch"

  # Test scripts
  ["scripts/test-cache-e2e.sh"]="cargo xtask test cache"
  ["scripts/test-import-e2e.sh"]="cargo xtask test import"
  ["scripts/validate-e2e-setup.sh"]="cargo xtask test validate"
  ["scripts/validate-e2e-setup.ps1"]="cargo xtask test validate"
  ["scripts/validate-workflows.sh"]="cargo xtask test workflows"

  # Audio test scripts
  ["scripts/generate-test-audio.sh"]="cargo xtask test audio generate"
  ["scripts/generate-test-audio.ps1"]="cargo xtask test audio generate"
  ["scripts/setup-virtual-audio.sh"]="cargo xtask test audio setup"
  ["scripts/setup-virtual-audio.ps1"]="cargo xtask test audio setup"

  # Obsolete build artifacts
  ["scripts/generate_test_audio_rust.exe"]="(generated file, not needed)"
  ["scripts/generate_test_audio_rust.pdb"]="(generated file, not needed)"
  ["scripts/generate_test_audio_rust.rs"]="(migrated to xtask)"

  # Old migration/fix scripts (one-time use)
  ["scripts/fix_all_sqlx.py"]="(one-time migration, obsolete)"
  ["scripts/fix_fetch_one.py"]="(one-time migration, obsolete)"
  ["scripts/fix_soul_sync.py"]="(one-time migration, obsolete)"
  ["scripts/fix_sqlx_types.py"]="(one-time migration, obsolete)"
  ["scripts/fix_sqlx_types_v2.py"]="(one-time migration, obsolete)"
  ["scripts/replace-console-logs.sh"]="(one-time migration, obsolete)"
  ["scripts/seed-test-data.ts"]="(one-time migration, obsolete)"
)

# Scripts to KEEP (explicitly excluded)
KEEP_SCRIPTS=(
  "scripts/seed-test-data.js"     # Uses better-sqlite3, complex to port
  "scripts/inspect-demo.mjs"      # Low priority utility
  "scripts/cleanup-old-scripts.sh" # This script
  "scripts/cleanup-old-scripts.ps1" # Windows version
  "scripts/cleanup-obsolete-files.ps1" # Different cleanup script
)

# Directories to delete
DIRS_TO_DELETE=(
  "scripts/tests"  # Replaced by xtask test commands
)

cd "$PROJECT_ROOT"

echo "📋 Scripts to be deleted (with xtask equivalents):"
echo ""

FOUND_COUNT=0
MISSING_COUNT=0

for script in "${!SCRIPTS_TO_DELETE[@]}"; do
  xtask_cmd="${SCRIPTS_TO_DELETE[$script]}"
  if [ -f "$script" ] || [ -d "$script" ]; then
    echo "  ✓ $script"
    echo "    → $xtask_cmd"
    ((FOUND_COUNT++))
  else
    echo "  ⊗ $script (already deleted)"
    ((MISSING_COUNT++))
  fi
done

echo ""
echo "📁 Directories to be deleted:"
for dir in "${DIRS_TO_DELETE[@]}"; do
  if [ -d "$dir" ]; then
    file_count=$(find "$dir" -type f | wc -l)
    echo "  ✓ $dir ($file_count files)"
    ((FOUND_COUNT++))
  else
    echo "  ⊗ $dir (already deleted)"
    ((MISSING_COUNT++))
  fi
done

echo ""
echo "📌 Scripts being KEPT:"
for script in "${KEEP_SCRIPTS[@]}"; do
  if [ -f "$script" ]; then
    echo "  ✓ $script"
  fi
done

echo ""
echo "Summary:"
echo "  - Files/dirs to delete: $FOUND_COUNT"
echo "  - Already deleted: $MISSING_COUNT"

if [ $FOUND_COUNT -eq 0 ]; then
  echo ""
  echo "✨ No files to delete. Cleanup already complete!"
  exit 0
fi

if [ "$DRY_RUN" = true ]; then
  echo ""
  echo "🔍 DRY RUN: No files will be deleted"
  echo ""
  echo "Run without --dry-run to actually delete files"
  echo "Use --backup to create a backup before deletion"
  exit 0
fi

echo ""
echo "⚠️  WARNING: This will permanently delete $FOUND_COUNT files/directories"

if [ "$BACKUP" = true ]; then
  echo "   A backup will be created in .backup/old-scripts-$(date +%Y%m%d)/"
fi

echo ""
read -p "Continue? (yes/no): " -r
if [[ ! $REPLY =~ ^[Yy][Ee][Ss]$ ]]; then
  echo "Cancelled."
  exit 0
fi

# Create backup if requested
if [ "$BACKUP" = true ]; then
  BACKUP_DIR=".backup/old-scripts-$(date +%Y%m%d-%H%M%S)"
  echo ""
  echo "📦 Creating backup in $BACKUP_DIR..."
  mkdir -p "$BACKUP_DIR"

  for script in "${!SCRIPTS_TO_DELETE[@]}"; do
    if [ -f "$script" ]; then
      script_dir=$(dirname "$script")
      mkdir -p "$BACKUP_DIR/$script_dir"
      cp "$script" "$BACKUP_DIR/$script"
      echo "  ✓ Backed up: $script"
    fi
  done

  for dir in "${DIRS_TO_DELETE[@]}"; do
    if [ -d "$dir" ]; then
      dir_parent=$(dirname "$dir")
      mkdir -p "$BACKUP_DIR/$dir_parent"
      cp -r "$dir" "$BACKUP_DIR/$dir"
      echo "  ✓ Backed up: $dir/"
    fi
  done

  echo "  Backup complete!"
fi

# Delete files
echo ""
echo "🗑️  Deleting obsolete scripts..."
DELETED_COUNT=0

for script in "${!SCRIPTS_TO_DELETE[@]}"; do
  if [ -f "$script" ]; then
    rm "$script"
    echo "  ✓ Deleted: $script"
    ((DELETED_COUNT++))
  elif [ -d "$script" ]; then
    rm -rf "$script"
    echo "  ✓ Deleted: $script/"
    ((DELETED_COUNT++))
  fi
done

for dir in "${DIRS_TO_DELETE[@]}"; do
  if [ -d "$dir" ]; then
    rm -rf "$dir"
    echo "  ✓ Deleted: $dir/"
    ((DELETED_COUNT++))
  fi
done

echo ""
echo "✅ Deleted $DELETED_COUNT files/directories"

# Verification
echo ""
echo "🔍 Running verification checks..."
echo ""

VERIFICATION_FAILED=false

# Check 1: xtask is available
echo "1. Checking cargo xtask is available..."
if cargo xtask --help &> /dev/null; then
  echo "   ✓ cargo xtask is available"
else
  echo "   ✗ cargo xtask not found!"
  VERIFICATION_FAILED=true
fi

# Check 2: Version command works
echo "2. Checking version command..."
if cargo xtask version current &> /dev/null; then
  VERSION=$(cargo xtask version current 2>/dev/null | grep -oP '\d+\.\d+\.\d+' | head -1)
  echo "   ✓ Version command works (current: $VERSION)"
else
  echo "   ✗ Version command failed!"
  VERIFICATION_FAILED=true
fi

# Check 3: Check command works
echo "3. Checking precommit check..."
if cargo xtask check precommit --help &> /dev/null; then
  echo "   ✓ Precommit check available"
else
  echo "   ✗ Precommit check failed!"
  VERIFICATION_FAILED=true
fi

# Check 4: Clean command works
echo "4. Checking clean command..."
if cargo xtask clean dev --help &> /dev/null; then
  echo "   ✓ Clean command available"
else
  echo "   ✗ Clean command failed!"
  VERIFICATION_FAILED=true
fi

# Check 5: WASM commands work
echo "5. Checking WASM commands..."
if cargo xtask wasm build --help &> /dev/null; then
  echo "   ✓ WASM commands available"
else
  echo "   ✗ WASM commands failed!"
  VERIFICATION_FAILED=true
fi

# Check 6: Test commands work
echo "6. Checking test commands..."
if cargo xtask test --help &> /dev/null; then
  echo "   ✓ Test commands available"
else
  echo "   ✗ Test commands failed!"
  VERIFICATION_FAILED=true
fi

echo ""

if [ "$VERIFICATION_FAILED" = true ]; then
  echo "❌ Verification FAILED!"
  if [ "$BACKUP" = true ]; then
    echo ""
    echo "To restore from backup:"
    echo "  cp -r $BACKUP_DIR/* ."
  fi
  exit 1
else
  echo "✅ All verification checks passed!"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🎉 Cleanup complete!"
echo ""
echo "Old scripts deleted: $DELETED_COUNT"
if [ "$BACKUP" = true ]; then
  echo "Backup location: $BACKUP_DIR"
fi
echo ""
echo "Use 'cargo xtask --help' to see all available commands"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
