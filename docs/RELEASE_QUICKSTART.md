# Release Quick Start Guide

## TL;DR - Create a Release in 5 Steps

```bash
# 1. Update version numbers
./scripts/bump-version.sh 0.1.0

# 2. Run pre-release checks
moon run :ci-check

# 3. Commit changes
git add .
git commit -m "chore: bump version to v0.1.0"
git push

# 4. Create and push tag
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0

# 5. Watch GitHub Actions → Release Pipeline
# Wait ~30-40 minutes → Release publishes automatically if all tests pass
```

## Prerequisites

Before creating a release, ensure:

- [ ] All CI checks passing on `main` branch
- [ ] No open P0 or P1 bugs
- [ ] CHANGELOG.md updated with release notes
- [ ] Version numbers bumped in all necessary files

## Step-by-Step Guide

### 1. Pre-Release Checklist

Complete this checklist before starting the release process:

#### Code Quality
- [ ] All tests passing: `cargo test --all`
- [ ] No clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Code formatted: `cargo fmt --all -- --check`
- [ ] Security audit clean: `cargo audit`
- [ ] Moon CI checks pass: `moon run :ci-check`

#### Documentation
- [ ] CHANGELOG.md updated with release notes
- [ ] README.md up to date
- [ ] Breaking changes documented
- [ ] Migration guide created (if needed)

#### Version Numbers
- [ ] Workspace `Cargo.toml` version updated
- [ ] All crate `Cargo.toml` versions updated
- [ ] `package.json` version updated (Tauri desktop)
- [ ] Tauri config version updated (`tauri.conf.json`)

**Pro Tip**: Use the version bumping script to automate this:
```bash
./scripts/bump-version.sh 0.1.0
```

### 2. Update CHANGELOG.md

Follow this format:

```markdown
## [0.1.0] - 2025-01-10

### Added
- Desktop audio playback with CPAL
- Linux package dependencies auto-detection
- Comprehensive release workflow with installation testing

### Changed
- Folder structure reorganized (libraries/ and applications/)
- Moon task orchestration integrated

### Fixed
- CPAL thread safety issues
- Audio buffer underruns in resampler

### Breaking Changes
- None

### Known Issues
- ESP32 firmware requires manual hardware testing
- iOS simulator testing requires signing certificates
```

### 3. Commit Version Changes

```bash
# Add all version files
git add Cargo.toml
git add */Cargo.toml
git add applications/desktop/package.json
git add applications/desktop/src-tauri/tauri.conf.json
git add CHANGELOG.md

# Commit with conventional commit message
git commit -m "chore: bump version to v0.1.0"

# Push to main
git push origin main

# Wait for CI to pass
```

### 4. Create and Push Git Tag

Tags must follow semantic versioning: `v<major>.<minor>.<patch>`

```bash
# Create annotated tag
git tag -a v0.1.0 -m "Release v0.1.0: Desktop Foundation Complete"

# Verify tag created
git tag -l v0.1.0

# Push tag to GitHub (this triggers the release workflow)
git push origin v0.1.0
```

**Tag Naming**:
- `v0.1.0` - Initial release
- `v0.1.1` - Patch release (bug fixes)
- `v0.2.0` - Minor release (new features)
- `v1.0.0` - Major release (breaking changes)

### 5. Monitor the Release Workflow

Go to: `https://github.com/yourusername/soul-player/actions`

You'll see the **Release - Desktop First** workflow running:

```
Stage 1: Build All Platforms (20-30 minutes)
├─ Build Windows (MSI + EXE)
├─ Build Linux (DEB + RPM + AppImage)
├─ Build macOS (DMG)
└─ Build Docker (Server image)

Stage 2: Create Draft Release (2-3 minutes)
└─ Upload all artifacts to GitHub Release (draft)

Stage 3: Test Installations (15-25 minutes, parallel)
├─ Test Windows MSI install/uninstall
├─ Test Linux DEB install/uninstall
├─ Test Linux RPM install/uninstall
├─ Test Linux AppImage execution
├─ Test macOS DMG mount
└─ Test Docker container start

Stage 4: Publish Release (1-2 minutes)
└─ If all tests pass → Publish draft release
```

**Total Time**: ~40-60 minutes

### 6. Verify Release Published

Once complete, check:

1. **GitHub Release Page**: `https://github.com/yourusername/soul-player/releases`
   - Release is published (not draft)
   - All assets are uploaded
   - Release notes are correct

2. **Download and Test** (spot check):
   ```bash
   # Download one artifact from release
   gh release download v0.1.0

   # Test installation on your platform
   # Windows: Install the MSI
   # Linux: sudo dpkg -i soul-player-*.deb
   # macOS: Open the DMG
   ```

3. **Announce Release**:
   - Update project website/docs
   - Post to Discord/Slack
   - Tweet/social media (optional)

## What Gets Built

### Desktop Applications

| Platform | Files | Size (approx) |
|----------|-------|---------------|
| Windows | `soul-player-v0.1.0-x64.msi` | ~30-50 MB |
| Windows | `soul-player-v0.1.0-x64.exe` | ~30-50 MB |
| Linux | `soul-player-v0.1.0-amd64.deb` | ~20-40 MB |
| Linux | `soul-player-v0.1.0.x86_64.rpm` | ~20-40 MB |
| Linux | `soul-player-v0.1.0.AppImage` | ~30-50 MB |
| macOS | `soul-player-v0.1.0-apple-silicon.dmg` | ~25-45 MB |
| macOS | `soul-player-v0.1.0-intel.dmg` | ~25-45 MB |

### Server

| Platform | Files | Size (approx) |
|----------|-------|---------------|
| Docker | `soul-server-v0.1.0.tar.gz` | ~100-200 MB |

**Note**: Exact sizes depend on build configuration and included dependencies.

## Troubleshooting

### Release Workflow Fails

#### Build Fails

**Check**: Which platform failed?
```bash
# View workflow logs
gh run list --workflow=release-desktop.yml
gh run view <run-id> --log
```

**Common Issues**:
- **Windows**: Tauri dependencies missing → Check Node.js version
- **Linux**: System libraries missing → Check apt-get install step
- **macOS**: Xcode version mismatch → Update Xcode in workflow
- **Docker**: Dockerfile not found → Check path in workflow

**Fix**:
1. Fix the issue locally first
2. Test build locally: `cd applications/desktop && npm run tauri build`
3. Commit fix and push
4. Delete failed tag: `git tag -d v0.1.0 && git push origin :refs/tags/v0.1.0`
5. Create new tag: `git tag -a v0.1.1 -m "Release v0.1.1" && git push origin v0.1.1`

#### Installation Tests Fail

**Check**: Which platform's test failed?

**Common Issues**:
- **Windows MSI**: Silent install fails → Check WiX configuration
- **Linux DEB**: Dependency issues → Check package metadata
- **Linux RPM**: Install fails on Fedora → Test in Fedora container locally
- **macOS DMG**: Mount fails → DMG creation issues
- **Docker**: Container won't start → Check Dockerfile CMD/ENTRYPOINT

**Fix**:
1. Download artifact from failed run
2. Test installation manually on that platform
3. Fix issue and create new release

#### Release Stays in Draft

This means **one or more tests failed**. The workflow is working correctly!

**What to do**:
1. Review test logs to identify failure
2. Fix the issue
3. Create new tag with patch version (e.g., v0.1.1)
4. Delete old draft release (optional)

**Manual Override** (use with caution):
```bash
# If you're confident tests should pass
# Go to GitHub → Releases → Edit draft → Publish manually
```

### Accidentally Created Wrong Tag

**Delete local and remote tag**:
```bash
# Delete local tag
git tag -d v0.1.0

# Delete remote tag
git push origin :refs/tags/v0.1.0

# This will stop the workflow if it hasn't completed
```

**Delete draft release**:
```bash
# Using GitHub CLI
gh release delete v0.1.0 --yes

# Or manually: Go to Releases → Edit → Delete
```

### Need to Update Release After Publishing

**Option 1: Edit Release Notes Only**
```bash
# Update release body
gh release edit v0.1.0 --notes "Updated release notes"
```

**Option 2: Create Patch Release**
```bash
# Fix the issue
git commit -am "fix: critical bug in v0.1.0"
git push

# Create patch release
git tag -a v0.1.1 -m "Release v0.1.1: Fix critical bug"
git push origin v0.1.1
```

**Never**: Replace binaries in an existing release. Always create a new release.

## Manual Trigger

You can manually trigger a release without creating a tag:

1. Go to: `Actions → Release - Desktop First → Run workflow`
2. Enter version: `0.1.0` (without the `v` prefix)
3. Click "Run workflow"

**Use Cases**:
- Testing the workflow before official release
- Creating a pre-release or beta
- Re-running failed release without deleting tag

## Creating Pre-Releases (Beta, RC)

For pre-release versions:

```bash
# Tag as pre-release
git tag -a v0.1.0-beta.1 -m "Beta release"
git push origin v0.1.0-beta.1

# Edit release on GitHub after workflow completes
gh release edit v0.1.0-beta.1 --prerelease
```

**Pre-release naming**:
- `v0.1.0-alpha.1` - Alpha release
- `v0.1.0-beta.1` - Beta release
- `v0.1.0-rc.1` - Release candidate

## Testing a Release Locally

Before pushing the tag, test locally:

```bash
# Build desktop app
cd applications/desktop
npm run tauri build

# Test installers
# Windows: target/release/bundle/msi/*.msi
# Linux: target/release/bundle/deb/*.deb
# macOS: target/release/bundle/dmg/*.dmg

# Build server (if applicable)
cd applications/server
cargo build --release

# Build Docker image
docker build -t soul-server:test -f Dockerfile ../..
docker run -d -p 8080:8080 soul-server:test
```

## Post-Release Tasks

After a successful release:

### Immediate (Day 1)

- [ ] Verify all download links work
- [ ] Test installation on at least one platform per OS
- [ ] Update project website with new version
- [ ] Announce on social channels
- [ ] Monitor GitHub issues for installation problems

### Week 1

- [ ] Check crash reports (if telemetry enabled)
- [ ] Address any critical bugs with patch release
- [ ] Update package manager submissions (winget, homebrew, AUR)
- [ ] Write blog post about release (optional)

### ESP32 Firmware (Manual Testing Required)

Since ESP32 firmware testing is limited in CI, you **must** manually test before release:

1. Follow checklist in [`docs/ESP32_MANUAL_TESTING.md`](ESP32_MANUAL_TESTING.md)
2. Document test results
3. Include in release notes if hardware testing completed

**Example Release Note**:
```markdown
### ESP32 Firmware

**⚠️ Manual Testing Required**: ESP32 firmware has been built and passed QEMU tests, but requires manual hardware testing before production use.

See [ESP32 Manual Testing Guide](docs/ESP32_MANUAL_TESTING.md) for complete testing checklist.

**Tested on**: ESP32-S3 DevKit with PCM5102 DAC (2025-01-10)
**Result**: ✅ All core features working
**Known Issues**: Occasional e-ink ghosting after 50+ partial updates
```

## Release Cadence

**Recommended schedule**:

- **Patch releases** (0.1.x): As needed for critical bugs
- **Minor releases** (0.x.0): Every 4-6 weeks
- **Major releases** (x.0.0): When breaking changes accumulated

**Exception**: Early development (0.x.y) can release more frequently.

## Version Numbering Guide

Follow [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH

MAJOR: Breaking changes (0.x.y → 1.0.0)
MINOR: New features, backward compatible (0.1.0 → 0.2.0)
PATCH: Bug fixes, backward compatible (0.1.0 → 0.1.1)
```

**Pre-1.0.0 releases**:
- Breaking changes allowed in minor versions
- API is not yet stable

**Post-1.0.0 releases**:
- Strict semver rules
- Breaking changes require MAJOR bump

## GitHub CLI Cheat Sheet

Useful `gh` commands for releases:

```bash
# List all releases
gh release list

# View specific release
gh release view v0.1.0

# Download release assets
gh release download v0.1.0

# Create release manually
gh release create v0.1.0 --title "Soul Player v0.1.0" --notes "Release notes"

# Edit release
gh release edit v0.1.0 --notes "Updated notes"

# Delete release
gh release delete v0.1.0 --yes

# View workflow runs
gh run list --workflow=release-desktop.yml

# View specific run logs
gh run view <run-id> --log

# Re-run failed workflow
gh run rerun <run-id>
```

## Help and Support

**Workflow issues**: Check `.github/workflows/release-desktop.yml` and logs

**Build issues**: See `docs/LINUX_BUILD_SETUP.md` for platform-specific dependencies

**Installation testing**: See `docs/RELEASE_TESTING_MATRIX.md` for what's tested

**ESP32 testing**: See `docs/ESP32_MANUAL_TESTING.md` for hardware checklist

**Questions**: Open a GitHub issue with `[release]` prefix

## Quick Reference Card

```
┌─────────────────────────────────────────────────────────┐
│              Soul Player Release Process                │
├─────────────────────────────────────────────────────────┤
│ 1. Pre-checks:  moon run :ci-check                     │
│ 2. Bump:        ./scripts/bump-version.sh 0.1.0        │
│ 3. Commit:      git commit -am "chore: bump to v0.1.0" │
│ 4. Tag:         git tag -a v0.1.0 -m "Release v0.1.0"  │
│ 5. Push:        git push origin v0.1.0                 │
│ 6. Wait:        ~40-60 minutes                          │
│ 7. Verify:      gh release view v0.1.0                 │
├─────────────────────────────────────────────────────────┤
│ If workflow fails:                                      │
│   - Check logs:  gh run list --workflow=release-*.yml  │
│   - Fix issue                                           │
│   - Delete tag:  git push origin :refs/tags/v0.1.0     │
│   - Re-tag:      git tag -a v0.1.1 -m "..."            │
└─────────────────────────────────────────────────────────┘
```

## Example: Complete Release Session

Real example of creating v0.1.0:

```bash
# Starting point: main branch, all CI passing

# 1. Check everything is good
moon run :ci-check
# ✓ All checks passed

# 2. Update version numbers
./scripts/bump-version.sh 0.1.0
# ✓ Updated 12 files

# 3. Update CHANGELOG
vim CHANGELOG.md
# (Add release notes)

# 4. Commit version bump
git add .
git commit -m "chore: bump version to v0.1.0"
git push origin main
# Wait for CI... ✓ Passed

# 5. Create and push tag
git tag -a v0.1.0 -m "Release v0.1.0: Desktop Foundation Complete

Desktop audio playback with CPAL, comprehensive CI/CD pipeline with installation testing, ESP32 manual testing checklist."

git push origin v0.1.0
# ✓ Tag pushed

# 6. Watch workflow
gh run watch
# [15:30] Build Windows... ✓
# [15:35] Build Linux... ✓
# [15:40] Build macOS... ✓
# [15:42] Build Docker... ✓
# [15:45] Create Draft... ✓
# [15:50] Test Windows... ✓
# [15:52] Test Linux DEB... ✓
# [15:53] Test Linux RPM... ✓
# [15:54] Test Linux AppImage... ✓
# [15:56] Test macOS... ✓
# [15:57] Test Docker... ✓
# [16:00] Publish Release... ✓
# ✅ Release v0.1.0 published!

# 7. Verify release
gh release view v0.1.0
# ✓ Published, all assets present

# 8. Test installation (spot check)
gh release download v0.1.0
sudo dpkg -i soul-player-v0.1.0-amd64.deb
soul-player --version
# ✓ v0.1.0

# 9. Announce
echo "🎉 Soul Player v0.1.0 is now available!"

# Done! ✅
```

---

**Remember**: Releases are permanent. Take your time, follow the checklist, and test thoroughly!
