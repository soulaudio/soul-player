# Version Management Commands

This document describes the version management commands in the xtask automation system.

## Overview

The version management system handles semantic versioning across all project files:
- Workspace `Cargo.toml`
- All library `Cargo.toml` files
- All application `Cargo.toml` files  
- All `package.json` files
- All `tauri.conf.json` files
- `.github/release-config.json`

It includes pre-flight checks, backup/rollback capabilities, and optional git automation.

## Commands

### Show Current Version

Display the current version from the workspace `Cargo.toml`:

```bash
cargo xtask version current
```

**Output:**
```
Current Version
  ✓ Workspace version: 0.1.10
```

### Validate Version

Validate a semantic version format:

```bash
cargo xtask version validate 1.0.0
cargo xtask version validate 1.0.0-beta.1
```

**Output:**
```
Validating Version
  ℹ Version: 1.0.0
  ✓ Valid semantic version format

  ℹ   Major: 1
  ℹ   Minor: 0
  ℹ   Patch: 0
```

### Bump Version

Bump the version across all project files:

```bash
# Preview changes (dry-run)
cargo xtask version bump 0.1.11 --dry-run

# Bump version with git operations (commit, tag, push)
cargo xtask version bump 0.1.11

# Bump version without git operations
cargo xtask version bump 0.1.11 --skip-git

# Bump version on non-main branch (forced)
cargo xtask version bump 0.1.11 --force
```

**Options:**
- `--dry-run`: Preview changes without modifying files
- `--skip-git`: Skip git commit/tag/push operations
- `--force`: Allow version bump on non-main branch

## Pre-flight Checks

Before bumping, the system performs these checks (skipped in dry-run mode):

1. **Git Status**: Working directory must be clean
2. **Git Branch**: Must be on `main` or `master` (unless `--force`)

**Example failure:**
```
Pre-flight Checks
  ✗ Git working directory is not clean

Uncommitted changes:
  .cargo/config.toml
  applications/desktop/src-tauri/Cargo.toml

  ℹ Please commit or stash your changes before bumping version
```

## Version Validation

The system validates semantic versioning:

**Valid formats:**
- `1.0.0` (release)
- `1.0.0-beta.1` (pre-release)
- `1.0.0-alpha.2` (pre-release)
- `1.0.0-rc.1` (release candidate)

**Invalid formats:**
- `1.0` (missing patch)
- `1` (missing minor and patch)
- `invalid` (not a number)

## Backup and Rollback

The system automatically creates backups before modifying files:

1. **Before Update**: Creates `.backup` files for all target files
2. **On Success**: Deletes all backup files
3. **On Error**: Restores all files from backups and reports error

**Files backed up:**
- Workspace `Cargo.toml`
- All `libraries/*/Cargo.toml`
- All `applications/*/Cargo.toml`
- All `package.json` files
- All `tauri.conf.json` files
- `.github/release-config.json`

## Git Operations

When not using `--skip-git`, the system performs these git operations:

1. **Stage Files**: Stages all modified files
2. **Create Commit**: Creates commit with message:
   ```
   chore(release): bump version to X.Y.Z
   
   Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
   ```
3. **Create Tag**: Creates annotated tag `vX.Y.Z`
4. **Push Commits**: Pushes to `origin/main`
5. **Push Tag**: Pushes tag to `origin`

## File Updates

The system updates these files:

### Cargo.toml Files

**Workspace:**
```toml
[workspace.package]
version = "0.1.11"  # Updated
```

**Libraries/Applications:**
```toml
[package]
version = "0.1.11"  # Updated
```

### package.json Files

```json
{
  "version": "0.1.11"  // Updated
}
```

### tauri.conf.json Files

```json
{
  "version": "0.1.11"  // Updated (Tauri 2.0 format)
}
```

### .github/release-config.json

```json
{
  "version": "0.1.11"  // Updated
}
```

## Validation

After updating files, the system validates:

1. **Workspace Cargo.toml**: Version matches expected
2. **tauri.conf.json**: Version matches expected (UI display)
3. **release-config.json**: Version matches expected (auto-updater)

**Example validation failure:**
```
Validation
  ✗ tauri.conf.json version mismatch! Expected: 0.1.11, Actual: 0.1.10
  ⚠ This will cause UI to show wrong version!
```

## Common Workflows

### Standard Release

```bash
# 1. Ensure clean working tree
git status

# 2. Preview changes
cargo xtask version bump 0.2.0 --dry-run

# 3. Bump version (with git operations)
cargo xtask version bump 0.2.0

# GitHub Actions will automatically:
# - Detect tag v0.2.0
# - Trigger release workflow
# - Build installers
# - Create GitHub release
# - Generate auto-updater manifest
```

### Development/Testing

```bash
# 1. Bump version without git operations
cargo xtask version bump 0.1.11-dev.1 --skip-git

# 2. Test changes
cargo build --all

# 3. Manually commit if satisfied
git add -A
git commit -m "chore: bump version to 0.1.11-dev.1"
```

### Hotfix on Branch

```bash
# 1. Create hotfix branch
git checkout -b hotfix/critical-bug

# 2. Bump version with force flag
cargo xtask version bump 0.1.10-hotfix.1 --force --skip-git

# 3. Test and commit manually
git add -A
git commit -m "chore: bump version to 0.1.10-hotfix.1"
```

## Error Handling

The system handles errors gracefully:

**Same Version:**
```
Error: New version 0.1.10 is the SAME as current 0.1.10
```

**Invalid Format:**
```
Error: Invalid semantic version: invalid
```

**Dirty Working Tree:**
```
Error: Git working tree is not clean
```

**Update Failure:**
If any file update fails, the system automatically:
1. Prints error message
2. Rolls back all changes from backups
3. Exits with error code

## Comparison with Node.js Script

This Rust implementation replaces `scripts/bump-version.mjs` with these improvements:

**Maintained:**
- ✅ Same pre-flight checks
- ✅ Same validation logic
- ✅ Same file update patterns
- ✅ Same git operations
- ✅ Backup/rollback capability
- ✅ Dry-run mode

**Improvements:**
- ✅ Type-safe with Rust
- ✅ Better error messages
- ✅ Integrated with xtask workflow
- ✅ Uses `toml_edit` for format-preserving TOML updates
- ✅ Cross-platform git operations via `git2`

## Future Enhancements

Potential future improvements:

- [ ] Interactive mode for selecting version bump type (major/minor/patch)
- [ ] Changelog generation from git commits
- [ ] Version bump suggestions based on conventional commits
- [ ] Dependency version synchronization checks
- [ ] Pre-release version management workflow

## Troubleshooting

### Backup Files Left Behind

If the script crashes and leaves `.backup` files:

```bash
find . -name "*.backup" -delete
```

### Git Push Fails

If git push fails after local changes:

```bash
# Check git status
git status

# Push manually
git push origin main
git push origin v0.1.11
```

### Version Mismatch

If validation fails, manually check files:

```bash
# Check workspace version
grep "^version" Cargo.toml

# Check tauri config
jq .version applications/desktop/src-tauri/tauri.conf.json

# Check release config
jq .version .github/release-config.json
```
