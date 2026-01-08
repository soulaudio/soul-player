# Release Guide for Soul Player

This document explains how to create a new release of Soul Player.

## TL;DR - Quick Release

```bash
# 1. Bump version (updates all version files)
./scripts/bump-version.sh 0.2.0

# 2. Commit and push
git add -A
git commit -m "chore: bump version to v0.2.0"
git push origin main

# 3. Done! Automation handles the rest
# Monitor at: https://github.com/soulaudio/soul-player/actions
```

---

## How Releases Work

Soul Player uses **automated releases** triggered by version bumps:

1. Developer runs `bump-version.sh` to update version numbers
2. Commits and pushes to `main`
3. GitHub workflow detects the version change
4. Workflow creates a git tag (e.g., `v0.2.0`)
5. Tag triggers the release workflow
6. Release workflow builds, tests, and publishes

**No manual tagging required!**

---

## Step-by-Step Release Process

### 1. Prepare for Release

**Check the build status:**
- Ensure CI is passing on `main`: https://github.com/soulaudio/soul-player/actions
- All tests should be green
- No blocking issues

**Decide on version number:**
- **Patch** (0.1.0 → 0.1.1): Bug fixes, minor improvements
- **Minor** (0.1.0 → 0.2.0): New features, backwards-compatible
- **Major** (0.1.0 → 1.0.0): Breaking changes

**Pre-release versions (optional):**
- Alpha: `0.2.0-alpha.1` (early testing)
- Beta: `0.2.0-beta.1` (feature complete, testing)
- RC: `0.2.0-rc.1` (release candidate)

### 2. Bump Version

Run the version bump script:

```bash
./scripts/bump-version.sh 0.2.0
```

**What this script does:**
- Validates version format (semver)
- Updates version in ALL project files:
  - `Cargo.toml` (workspace root)
  - All library `Cargo.toml` files in `libraries/`
  - All application `Cargo.toml` files in `applications/`
  - `applications/desktop/package.json`
  - `applications/desktop/src-tauri/tauri.conf.json`
- Prompts for confirmation
- Shows summary of changes

**Files updated:**
```
✓ Cargo.toml (workspace root)
✓ libraries/soul-core/Cargo.toml
✓ libraries/soul-storage/Cargo.toml
✓ libraries/soul-audio/Cargo.toml
✓ libraries/soul-playback/Cargo.toml
✓ libraries/soul-metadata/Cargo.toml
✓ libraries/soul-importer/Cargo.toml
✓ libraries/soul-discovery/Cargo.toml
✓ libraries/soul-sync/Cargo.toml
✓ libraries/soul-artwork/Cargo.toml
✓ libraries/soul-audio-desktop/Cargo.toml
✓ libraries/soul-audio-mobile/Cargo.toml
✓ libraries/soul-audio-embedded/Cargo.toml
✓ applications/desktop/src-tauri/Cargo.toml
✓ package.json (root)
✓ applications/desktop/package.json
✓ applications/marketing/package.json
✓ applications/mobile/package.json
✓ applications/shared/package.json
✓ applications/desktop/src-tauri/tauri.conf.json
```

### 3. Review Changes

Review the version bump changes:

```bash
git diff
```

**What to check:**
- All version fields show the new version
- No unexpected changes
- Version format is correct (no typos)

### 4. Commit Changes

Commit the version bump:

```bash
git add -A
git commit -m "chore: bump version to v0.2.0"
```

**Commit message format:**
- Use conventional commit format: `chore: bump version to vX.Y.Z`
- Keep it simple and consistent

### 5. Push to Main

Push the commit to trigger automation:

```bash
git push origin main
```

**This triggers the auto-release workflow** (`.github/workflows/auto-release-on-version-bump.yml`):
- Detects version change in `Cargo.toml`
- Compares with latest git tag
- Creates new tag `v0.2.0`
- Pushes tag to GitHub

### 6. Monitor Release Progress

**The tag push triggers the desktop release workflow** (`.github/workflows/release-desktop.yml`):

**Monitor at:** https://github.com/soulaudio/soul-player/actions

**Release stages:**
1. **Build Stage** (20-30 min) - Builds installers for all platforms
   - Windows: MSI and NSIS installers
   - macOS: DMG for Apple Silicon and Intel
   - Linux: DEB, RPM, and AppImage packages
   - Docker: Server image tarball

2. **Draft Release** (1 min) - Creates GitHub release draft with all artifacts

3. **Test Stage** (15-25 min) - Runs installation tests
   - Windows: Silent install/uninstall tests
   - macOS: DMG mount/unmount tests
   - Linux: Package installation tests for DEB, RPM, AppImage
   - Docker: Image load and verification

4. **Publish Stage** (1 min) - Publishes release if all tests pass
   - Release becomes public on GitHub
   - Artifacts available for download

**Total time: ~40-60 minutes**

### 7. Verify Release

Once published, verify the release:

**GitHub Release Page:**
- Go to: https://github.com/soulaudio/soul-player/releases/latest
- Check all artifacts are present
- Verify release notes are correct

**Marketing Page:**
- Visit: https://player.soulaudio.co
- Download button should show new version
- Test download for your platform

**Artifacts to verify:**
- [ ] `soul-player-v0.2.0-x64.msi` (Windows)
- [ ] `soul-player-v0.2.0-x64.exe` (Windows NSIS)
- [ ] `soul-player-v0.2.0-apple-silicon.dmg` (macOS ARM)
- [ ] `soul-player-v0.2.0-intel.dmg` (macOS Intel)
- [ ] `soul-player-v0.2.0-amd64.deb` (Linux Debian/Ubuntu)
- [ ] `soul-player-v0.2.0.x86_64.rpm` (Linux Fedora/RHEL)
- [ ] `soul-player-v0.2.0.AppImage` (Linux Universal)
- [ ] `soul-server-v0.2.0.tar.gz` (Docker)

### 8. Announce Release (Optional)

- Update documentation if needed
- Announce on social media, Discord, etc.
- Notify users of new features/fixes

---

## Troubleshooting

### Version Bump Failed

**Problem:** Script fails to update files

**Solution:**
```bash
# Check file permissions
chmod +x scripts/bump-version.sh

# Ensure jq is installed (optional but recommended)
# macOS:
brew install jq

# Ubuntu/Debian:
sudo apt-get install jq

# Run script again
./scripts/bump-version.sh 0.2.0
```

### Tag Already Exists

**Problem:** Workflow fails with "Tag already exists"

**Solution:**
```bash
# Option 1: Bump to a newer version
./scripts/bump-version.sh 0.2.1

# Option 2: Delete existing tag (if this was a mistake)
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0
```

### Release Workflow Failed

**Problem:** Tests fail or builds fail during release

**Solution:**
1. Check the failed workflow logs: https://github.com/soulaudio/soul-player/actions
2. Fix the issue in a new PR
3. Bump version to 0.2.1 and re-release

**Note:** Failed releases create a DRAFT release, so no broken artifacts are published.

### Marketing Page Not Showing New Version

**Problem:** Website still shows old version after release

**Solution:**
- Wait 5 minutes (sessionStorage cache TTL)
- Hard refresh the page (Ctrl+Shift+R or Cmd+Shift+R)
- Check GitHub API: https://api.github.com/repos/soulaudio/soul-player/releases/latest

---

## Emergency/Hotfix Release

For urgent bug fixes, you can manually trigger a release:

```bash
# 1. Bump version
./scripts/bump-version.sh 0.1.1

# 2. Commit and push
git add -A && git commit -m "chore: hotfix release v0.1.1"
git push origin main

# 3. Automation handles the rest
```

Alternatively, bypass automation and create tag manually:

```bash
# Create tag directly (skips auto-release workflow)
git tag -a v0.1.1 -m "Hotfix: Critical bug fix"
git push origin v0.1.1

# This triggers release-desktop.yml immediately
```

---

## Pre-Release (Alpha/Beta/RC)

For testing versions before stable release:

```bash
# Alpha release (early testing)
./scripts/bump-version.sh 0.2.0-alpha.1
git add -A && git commit -m "chore: alpha release v0.2.0-alpha.1"
git push origin main

# Beta release (feature complete)
./scripts/bump-version.sh 0.2.0-beta.1
git add -A && git commit -m "chore: beta release v0.2.0-beta.1"
git push origin main

# Release candidate
./scripts/bump-version.sh 0.2.0-rc.1
git add -A && git commit -m "chore: release candidate v0.2.0-rc.1"
git push origin main

# Final stable release
./scripts/bump-version.sh 0.2.0
git add -A && git commit -m "chore: stable release v0.2.0"
git push origin main
```

---

## Release Checklist

Before releasing, ensure:

- [ ] All CI checks passing on `main`
- [ ] No known critical bugs
- [ ] Documentation is up-to-date
- [ ] CHANGELOG updated (if applicable)
- [ ] Version number follows semver
- [ ] All tests pass locally: `cargo test --all`

After releasing, verify:

- [ ] GitHub release published successfully
- [ ] All platform installers present
- [ ] Marketing page shows new version
- [ ] Download links work
- [ ] Installation tested on at least one platform

---

## Rollback

If a release has critical issues:

### Delete Release and Tag

```bash
# 1. Delete the GitHub release manually
# Go to: https://github.com/soulaudio/soul-player/releases
# Find the release → Delete release

# 2. Delete the tag
git tag -d v0.2.0
git push origin :refs/tags/v0.2.0

# 3. Revert version bump commit
git revert HEAD
git push origin main
```

### Create Hotfix Release

```bash
# Fix the issue, then release a patch version
./scripts/bump-version.sh 0.2.1
git add -A && git commit -m "chore: hotfix release v0.2.1"
git push origin main
```

---

## Technical Details

### Automated Workflows

**`.github/workflows/auto-release-on-version-bump.yml`**
- Triggers: Push to `main` when `Cargo.toml` changes
- Detects version bumps
- Creates and pushes git tags

**`.github/workflows/release-desktop.yml`**
- Triggers: Tag push matching `v*.*.*`
- Builds multi-platform installers
- Runs installation tests
- Publishes GitHub release

### Version Source of Truth

**Primary:** `/Cargo.toml` (workspace version)

All Rust crates inherit from workspace:
```toml
[package]
version.workspace = true
```

**Synchronized files:**
- `package.json` (Node/Tauri)
- `tauri.conf.json` (Tauri configuration)

### Artifact Naming Convention

Format: `{app}-v{VERSION}-{platform}.{ext}`

Examples:
- `soul-player-v0.2.0-x64.msi`
- `soul-player-v0.2.0-apple-silicon.dmg`
- `soul-player-v0.2.0.AppImage`
- `soul-server-v0.2.0.tar.gz`

---

## FAQ

**Q: Do I need to manually create git tags?**
A: No! The automation creates tags for you when you push a version bump to `main`.

**Q: Can I skip the automation and tag manually?**
A: Yes, you can manually create and push a tag to trigger the release workflow directly:
```bash
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

**Q: What if I push to main but don't change the version?**
A: The workflow runs but exits early with "No release needed - version has not changed".

**Q: Can I release from a branch other than main?**
A: Not automatically. The auto-release workflow only runs on `main`. For manual releases, create a tag on any branch and push it.

**Q: How long does a release take?**
A: Approximately 40-60 minutes from push to published release.

**Q: What if the release fails?**
A: The workflow creates a DRAFT release, so nothing is published if tests fail. Fix the issue and bump to a patch version.

**Q: Can I release multiple versions in parallel?**
A: No, releases should be sequential. Wait for one release to complete before starting another.

---

## Support

If you encounter issues:
- Check workflow logs: https://github.com/soulaudio/soul-player/actions
- Review this guide
- Check GitHub Issues: https://github.com/soulaudio/soul-player/issues
- Ask in Discord (if available)
