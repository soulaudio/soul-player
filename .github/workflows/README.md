# GitHub Actions Workflows

## Quick Reference

### Workflows in This Directory

| Workflow | Trigger | Purpose | Status |
|----------|---------|---------|--------|
| `ci.yml` | Push, PR | Continuous integration (build, test, lint) | ✅ Active |
| `security.yml` | Daily, Deps change | Security audits and vulnerability scanning | ✅ Active |
| `docs.yml` | Push to main | Build and deploy documentation | ✅ Active |
| `release.yml` | Git tags (v*.*.*) | Multi-platform release with testing | 🚧 Ready to use |

### Release Workflow Overview

```
Tag v0.1.0 →  Build (7 platforms) →  Draft Release →  Test Install →  Publish
                    ↓                      ↓               ↓             ↓
               20-30 min             Upload all      15-25 min     If all pass
                                     artifacts       (parallel)
```

## Using the Release Workflow

### Triggering a Release

**Option 1: Git Tag (Recommended)**
```bash
# Create and push tag
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0

# Workflow automatically triggers
```

**Option 2: Manual Trigger**
```yaml
# Go to GitHub Actions → Release Pipeline → Run workflow
# Enter version: 0.1.0
```

### What Gets Built

#### Desktop
- **Windows**: `soul-player-v0.1.0-x64.msi`, `soul-player-v0.1.0-x64.exe`
- **Linux**: `soul-player-v0.1.0-amd64.deb`, `soul-player-v0.1.0.x86_64.rpm`, `soul-player-v0.1.0.AppImage`
- **macOS**: `soul-player-v0.1.0-intel.dmg`, `soul-player-v0.1.0-apple-silicon.dmg`

#### Mobile
- **Android**: `soul-player-v0.1.0.apk`
- **iOS**: `soul-player-v0.1.0.ipa` (requires signing)

#### Firmware
- **ESP32-S3**: `soul-player-esp32-v0.1.0.bin`

#### Server
- **Docker**: `soul-server:v0.1.0`, `soul-server:latest`

### What Gets Tested

| Platform | Test Coverage | Time | Fail = Block Release |
|----------|---------------|------|----------------------|
| Windows MSI | Install, upgrade, uninstall | ~3 min | ✅ Yes |
| Windows EXE | Install, uninstall | ~2 min | ✅ Yes |
| Linux DEB | Install, upgrade, uninstall | ~2 min | ✅ Yes |
| Linux RPM | Install, upgrade, uninstall | ~2 min | ✅ Yes |
| Linux AppImage | Execute, version check | ~1 min | ✅ Yes |
| macOS DMG | Install, launch, uninstall | ~3 min | ✅ Yes |
| Android APK | Install, launch, uninstall (API 30, 33) | ~8 min | ✅ Yes |
| iOS IPA | Install, launch (simulator) | ~5 min | ⚠️ Optional |
| ESP32 Firmware | QEMU unit tests | ~3 min | ⚠️ Optional |
| Docker | Container start, health check | ~2 min | ✅ Yes |

**Total Test Time**: ~15-25 minutes (parallel execution)

### Monitoring a Release

1. **Go to**: GitHub Actions → Release Pipeline
2. **Watch stages**:
   - ⏳ Building (7 jobs in parallel)
   - ⏳ Creating draft release
   - ⏳ Testing installations (10 jobs in parallel)
   - ✅ Publishing (if all pass) OR ❌ Staying draft (if any fail)

3. **Check artifacts**: Each build uploads artifacts you can download for manual testing

### If Tests Fail

**What happens**:
- ❌ Release stays in DRAFT mode
- 📝 Test logs uploaded as artifacts
- 🔍 Review logs to identify issue

**How to fix**:
1. Download test logs from failed job
2. Fix the issue in code
3. Create new tag (e.g., `v0.1.1`)
4. Workflow runs again

**Manual override** (if needed):
1. Go to GitHub Releases
2. Find draft release
3. Click "Edit"
4. Manually publish (use with caution!)

### Cost Monitoring

Check your Actions usage:
- Settings → Billing → Actions minutes used
- Free tier: 2000 minutes/month
- Multipliers: Linux 1x, Windows 2x, macOS 10x

**Per release estimate**: ~465 minute-equivalents

## Configuration

### Customizing Tests

Edit `.github/workflows/release.yml`:

**Skip Android testing**:
```yaml
# Comment out in "needs" section of publish-release job
needs: [
  # ... other jobs ...
  # test-android-install,  # Commented out
]
```

**Change Linux distros**:
```yaml
matrix:
  distro: [ubuntu, fedora, debian]  # Add more
```

**Change API levels**:
```yaml
matrix:
  api-level: [30, 33, 34]  # Test more versions
```

### Secrets Required

| Secret | Purpose | Required |
|--------|---------|----------|
| `GITHUB_TOKEN` | Automatic (create releases) | ✅ Auto-provided |
| `APPLE_CERTIFICATE` | iOS code signing | ⚠️ Optional |
| `APPLE_PROVISIONING_PROFILE` | iOS provisioning | ⚠️ Optional |
| `ANDROID_KEYSTORE` | Android APK signing | ⚠️ Optional |

### Environment Variables

Set in workflow or repository secrets:

```yaml
env:
  TAURI_PRIVATE_KEY: ${{ secrets.TAURI_PRIVATE_KEY }}  # For auto-updates
  TAURI_KEY_PASSWORD: ${{ secrets.TAURI_KEY_PASSWORD }}
```

## Troubleshooting

### Build Fails

**Windows**:
- Check Tauri configuration in `src-tauri/tauri.conf.json`
- Verify WiX Toolset configuration for MSI

**Linux**:
- Ensure dependencies installed (see job logs)
- Check `.deb`/`.rpm` metadata in `tauri.conf.json`

**macOS**:
- Verify Xcode version compatibility
- Check code signing (if enabled)

**Android**:
- Verify Android SDK installed
- Check Gradle build in `mobile/android/`

**iOS**:
- Verify Xcode version
- Check Xcode project in `mobile/ios/`
- Ensure provisioning profiles (if signing)

**ESP32**:
- Verify ESP-IDF version
- Check CMakeLists.txt in `firmware/`

### Test Fails

**Windows install test**:
- Check MSI can install silently
- Verify registry keys created
- Check start menu shortcuts

**Linux install test**:
- Verify dependencies declared in package
- Check binary installed to correct location
- Verify desktop file

**macOS install test**:
- Check code signature
- Verify Gatekeeper approval
- Check app bundle structure

**Android install test**:
- Verify APK is signed
- Check package name matches
- Ensure app has launcher activity

**iOS install test**:
- Verify app is signed (even for simulator)
- Check bundle ID
- Ensure Info.plist is valid

**ESP32 QEMU test**:
- Verify firmware format (.elf required)
- Check unit test output format
- Ensure tests complete within timeout

**Docker test**:
- Verify Dockerfile builds
- Check health endpoint implemented
- Ensure port 8080 exposed

### Performance Issues

**Slow builds**:
```yaml
# Add caching
- uses: Swatinem/rust-cache@v2
- uses: actions/cache@v4
  with:
    path: ~/.cargo
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

**Hitting Actions limits**:
- Use self-hosted runners for Linux
- Limit macOS matrix to latest 2 versions only
- Skip optional tests (iOS, ESP32)

### Release Not Publishing

**Check**:
1. Did all test jobs complete successfully?
2. Check `publish-release` job logs
3. Verify `needs: [...]` lists all test jobs

**Manual publish**:
```bash
# If automated publish fails, manually:
# 1. Go to Releases
# 2. Edit draft
# 3. Uncheck "Draft"
# 4. Publish
```

## Best Practices

### Pre-Release Checklist

- [ ] Update CHANGELOG.md
- [ ] Bump version in Cargo.toml
- [ ] Update version in package.json (Tauri)
- [ ] Test locally: `cargo build --release --workspace`
- [ ] Run CI checks: `moon run :ci-check`
- [ ] Create and push tag

### Version Numbering

Follow Semantic Versioning (semver):
- `v0.1.0` - Initial release
- `v0.1.1` - Patch (bug fixes)
- `v0.2.0` - Minor (new features, backward compatible)
- `v1.0.0` - Major (breaking changes)

### Release Notes

Auto-generated, but you can customize:

Edit `release.yml` → `Generate release notes` step:
```yaml
- name: Generate release notes
  run: |
    # Your custom logic here
    # Can pull from CHANGELOG.md
    # Can use GitHub API to get PRs merged since last release
```

## Advanced Usage

### Testing Without Publishing

Create a test tag:
```bash
git tag v0.0.1-test
git push origin v0.0.1-test
```

This will:
- ✅ Build all platforms
- ✅ Run all tests
- ✅ Create draft release
- ❌ Not publish (you can delete draft after)

### Building Specific Platforms Only

Comment out platforms in workflow:
```yaml
needs: [
  build-windows,
  # build-linux,  # Skip
  build-macos,
  # ... etc
]
```

### Adding New Platforms

1. Add build job (e.g., `build-arm-linux`)
2. Add to `create-draft-release` needs
3. Add test job (e.g., `test-arm-linux-install`)
4. Add to `publish-release` needs

## Help and Support

- **Workflow fails**: Check job logs in Actions tab
- **Test fails**: Download test artifacts for debugging
- **Questions**: Open issue with `[release]` prefix
- **Documentation**: See `docs/RELEASE_STRATEGY.md`

## Quick Commands

```bash
# Create release
git tag -a v0.1.0 -m "Release v0.1.0" && git push origin v0.1.0

# Delete tag (if mistake)
git tag -d v0.1.0 && git push origin :refs/tags/v0.1.0

# List all releases
gh release list

# Download release assets
gh release download v0.1.0

# View workflow runs
gh run list --workflow=release.yml

# View specific run
gh run view <run-id>
```

## Related Documentation

- [Release Strategy](../docs/RELEASE_STRATEGY.md) - Overall strategy
- [Release Workflow Discussion](../docs/RELEASE_WORKFLOW_DISCUSSION.md) - Detailed analysis
- [Linux Packaging](../docs/LINUX_PACKAGING.md) - Linux-specific packaging
- [Contributing](../CONTRIBUTING.md) - Contribution guidelines
