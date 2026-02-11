# Version Management - Quick Start

## Commands

### Show current version
```bash
cargo xtask version current
```

### Validate a version
```bash
cargo xtask version validate 1.0.0
cargo xtask version validate 1.0.0-beta.1
```

### Bump version

**Preview changes (safe):**
```bash
cargo xtask version bump 0.2.0 --dry-run
```

**Bump with full git automation:**
```bash
cargo xtask version bump 0.2.0
```

**Bump without git (manual commit):**
```bash
cargo xtask version bump 0.2.0 --skip-git
```

**Bump on non-main branch (forced):**
```bash
cargo xtask version bump 0.2.0 --force
```

## Workflow

### Standard Release
```bash
# 1. Ensure clean git tree
git status

# 2. Preview changes
cargo xtask version bump 0.2.0 --dry-run

# 3. Bump version (creates commit, tag, pushes)
cargo xtask version bump 0.2.0

# GitHub Actions will automatically build and release
```

### Pre-release
```bash
cargo xtask version bump 1.0.0-beta.1 --dry-run
cargo xtask version bump 1.0.0-beta.1
```

### Development
```bash
cargo xtask version bump 0.1.11-dev.1 --skip-git
# Test changes...
git add -A
git commit -m "chore: bump to 0.1.11-dev.1"
```

## Files Updated

The version bump updates 20+ files:
- Workspace `Cargo.toml`
- All `libraries/*/Cargo.toml`
- All `applications/*/Cargo.toml`
- All `package.json` files
- All `tauri.conf.json` files
- `.github/release-config.json`

## Pre-flight Checks

Before bumping (unless `--dry-run`):
- ✅ Git working tree must be clean
- ✅ Must be on `main` or `master` branch (unless `--force`)

## Validation

After updating:
- ✅ All files have matching version
- ✅ No version mismatches
- ✅ Automatic rollback on error

## Error Handling

**Same version:**
```
Error: New version 0.1.10 is the SAME as current 0.1.10
```

**Dirty git tree:**
```
Error: Git working directory is not clean
Uncommitted changes:
  .cargo/config.toml
  ...
```

**Invalid version:**
```
Error: Invalid semantic version: invalid
```

## Help

```bash
cargo xtask version --help
cargo xtask version bump --help
```

## Full Documentation

See `xtask/README_VERSION.md` for comprehensive documentation.
