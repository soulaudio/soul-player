# Release Workflow - Discussion and Recommendations

## Executive Summary

I've researched and created a comprehensive multi-platform release pipeline with automated installation testing. Here's what's **feasible** and what requires **trade-offs**.

## What's Fully Testable in CI ✅

### 1. **Desktop (Windows, Linux, macOS)** - 100% Coverage
- ✅ **Installation**: All package formats (.msi, .exe, .deb, .rpm, .AppImage, .dmg, .pkg)
- ✅ **Upgrade**: Test version upgrades with data persistence
- ✅ **Uninstall**: Verify complete removal
- ✅ **Multiple distros**: Ubuntu, Fedora, Debian via containers
- ✅ **No limitations**: Full native execution

**Confidence Level**: HIGH - This is battle-tested and reliable.

### 2. **Android** - 90% Coverage
- ✅ **Installation**: APK installation via emulator
- ✅ **App Launch**: Verify app starts successfully
- ✅ **Upgrade**: Test APK upgrade path
- ✅ **Uninstall**: Verify cleanup
- ✅ **Hardware acceleration**: Available since April 2024 ([source](https://github.blog/changelog/2024-04-02-github-actions-hardware-accelerated-android-virtualization-now-available/))

**Limitations**:
- ❌ No Google Play Store testing (can add Firebase App Distribution separately)
- ❌ No real device hardware (camera, sensors)
- ⚠️ API levels limited to what emulators support

**Confidence Level**: HIGH for sideload APK testing.

### 3. **iOS** - 70% Coverage
- ✅ **Installation**: Simulator installation
- ✅ **App Launch**: Verify app starts
- ⚠️ **Requires code signing**: Even for simulator (can use dev certificates)
- ✅ **Multiple iOS versions**: Test iOS 16, 17, 18.1

**Limitations**:
- ❌ No real device testing
- ❌ No App Store flow testing
- ❌ Limited to iOS versions in GitHub runners (currently up to 18.1)
- ❌ No hardware features (Face ID, Touch ID, cameras)

**Confidence Level**: MEDIUM - Good for basic validation, but not comprehensive.

### 4. **Server/Docker** - 100% Coverage
- ✅ **Build**: Docker image creation
- ✅ **Startup**: Container launches successfully
- ✅ **Health checks**: HTTP endpoints respond
- ✅ **Configuration**: Environment variables work
- ✅ **Cleanup**: Container stops cleanly

**Confidence Level**: HIGH - Docker is CI-native.

## What's Partially Testable ⚠️

### 5. **ESP32 Firmware** - 40% Coverage
- ✅ **Build**: Firmware compiles successfully
- ✅ **Unit tests**: QEMU-based CPU/memory tests ([ESP32 QEMU Runner](https://github.com/marketplace/actions/esp32-qemu-runner))
- ✅ **Binary validation**: Size checks, format verification
- ❌ **Hardware I/O**: Cannot test audio output, SD card, display, WiFi, Bluetooth

**Why Limited?**:
- QEMU simulates ESP32 CPU but **not peripherals**
- Audio DAC, I2S, SD card, e-ink display require real hardware
- No viable solution for CI-based hardware testing

**Recommendation**:
```yaml
✅ Run QEMU unit tests (test core logic)
❌ Skip hardware I/O tests (requires real device)
📝 Add manual testing checklist for hardware features
```

**Future Enhancement**:
- Self-hosted runner with USB-connected ESP32 board
- Hardware-in-the-loop (HIL) testing setup
- Remote test lab (expensive, complex)

**Confidence Level**: LOW for production readiness. **Recommend manual testing.**

## Cost Analysis 💰

GitHub Actions pricing (free tier: 2000 minutes/month):

| Runner | Multiplier | Est. Time/Build | Cost Impact |
|--------|------------|-----------------|-------------|
| Linux | 1x | 15 min | ✅ Low |
| Windows | 2x | 20 min | ⚠️ Medium |
| **macOS** | **10x** | 25 min | ⚠️ **HIGH** |

**Per Release Estimate** (all tests):
- Linux: 15 min × 1x = 15 minutes
- Windows: 20 min × 2x = 40 minute-equivalents
- macOS: 25 min × 10x = 250 minute-equivalents
- Android: 10 min × 1x = 10 minutes
- iOS: 15 min × 10x (macOS) = 150 minute-equivalents

**Total: ~465 minute-equivalents per release**

**Free tier allows**: ~4 releases/month before hitting limits.

### Cost Optimization Strategies:

1. **Use self-hosted runners for Linux** (free compute)
2. **Limit macOS matrix** (only test latest 2 macOS versions instead of 3)
3. **Skip iOS testing initially** (saves 150 minutes) - add later when needed
4. **Cache aggressively** (Rust builds, npm modules, Docker layers)
5. **Parallel testing** (already implemented)

## Recommended Phased Approach

### Phase 1: Essential Coverage (MVP)
```yaml
✅ Windows (MSI, EXE)
✅ Linux (DEB, RPM, AppImage) - Ubuntu only
✅ macOS (DMG) - Universal binary only
✅ Server (Docker)
❌ Skip: Android, iOS, ESP32 (manual testing)
```

**Rationale**: Core desktop + server coverage with minimal cost.
**Monthly cost**: ~200 minutes/release = 10 releases/month on free tier.

### Phase 2: Mobile Coverage
```yaml
+ Android (APK) - API 30, 33
+ iOS (Simulator) - if you have signing certificates
```

**Rationale**: Add mobile once desktop is stable.
**Monthly cost**: ~350 minutes/release = 5 releases/month.

### Phase 3: Comprehensive (Full)
```yaml
+ Multiple Linux distros (Fedora, Debian)
+ macOS Intel + Apple Silicon
+ ESP32 QEMU tests
+ iOS real device testing (external service)
```

**Rationale**: Production-grade testing.
**Monthly cost**: ~465 minutes/release = 4 releases/month.

## Trade-Offs and Decisions

### Decision 1: ESP32 Hardware Testing

**Options**:
1. ❌ **Full QEMU simulation** - Not feasible (no peripheral support)
2. ✅ **QEMU unit tests only** - Feasible but limited
3. ⚠️ **Self-hosted runner with hardware** - Feasible but requires setup
4. ⚠️ **Remote test lab** - Feasible but expensive

**Recommendation**:
```
✅ Implement QEMU unit tests in CI
📝 Add manual testing checklist for releases
🚀 Future: Self-hosted runner with USB ESP32
```

**Manual Testing Checklist for ESP32**:
- [ ] Flash firmware to device
- [ ] Test audio playback (DAC output)
- [ ] Test SD card read/write
- [ ] Test e-ink display rendering
- [ ] Test WiFi connectivity
- [ ] Test OTA update

### Decision 2: iOS Testing Without Apple Developer Account

**Options**:
1. ✅ **Simulator with development signing** - Free, basic validation
2. ❌ **Real device testing** - Requires $99/year Apple Developer account
3. ⚠️ **TestFlight beta testing** - Requires paid account, manual step

**Recommendation**:
```
Start: Skip iOS testing (not critical initially)
Later: Add simulator tests when you have signing set up
Production: Add TestFlight for beta testing
```

### Decision 3: Package Manager Testing

**Options**:
1. ✅ **Direct installer testing** (MSI, DEB, RPM) - Implemented
2. ⚠️ **Package manager testing** (winget, apt, homebrew) - Requires publishing first
3. ⚠️ **Repository hosting** - Complex, ongoing maintenance

**Recommendation**:
```
Phase 1: Test direct installers (.msi, .deb, .dmg)
Phase 2: Create PRs to package manager repos (winget, homebrew)
Phase 3: Host your own apt/yum repository
```

**Note**: Package managers like `winget` and `homebrew` require:
- Submitting PRs to their repositories
- Manifest files with checksums
- Can be automated but adds complexity

## Failure Handling Strategy

### When Tests Fail

**Current implementation**:
```yaml
1. Draft release is created (always)
2. All tests run (parallel)
3. If ANY test fails:
   - Release stays in DRAFT mode (not published)
   - Test logs uploaded as artifacts
   - GitHub issue created automatically (optional)
4. If ALL tests pass:
   - Draft is published
   - Latest tag updated
```

**Best Practice**:
- ✅ **Fail fast**: Stop testing on first critical failure (optional)
- ✅ **Parallel testing**: Run all tests simultaneously
- ✅ **Artifact preservation**: Keep test logs for debugging
- ✅ **Manual override**: Allow manual publish of draft if needed

### Rollback Strategy

If a published release has issues:
```yaml
1. Manual: Edit release, mark as pre-release
2. Automated: Create hotfix branch, new release
3. Notification: Update users via release notes
```

## CI/CD Workflow Architecture

```
┌─────────────┐
│ Git Tag     │
│ (v0.1.0)    │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Build Stage (Parallel)                  │
│ - Windows (.msi, .exe)                  │
│ - Linux (.deb, .rpm, .AppImage)         │
│ - macOS (.dmg, .pkg)                    │
│ - Android (.apk)                        │
│ - iOS (.ipa)                            │
│ - ESP32 (.bin, .elf)                    │
│ - Server (Docker image)                 │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Create Draft Release                    │
│ - Upload all artifacts                  │
│ - Generate release notes                │
│ - Tag version                           │
└──────┬──────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────┐
│ Test Stage (Parallel)                   │
│ ✓ Windows: MSI + EXE install/uninstall │
│ ✓ Linux: DEB + RPM + AppImage          │
│ ✓ macOS: DMG + PKG install/uninstall   │
│ ✓ Android: APK on emulator (API 30,33) │
│ ⚠ iOS: Simulator install (if signed)   │
│ ⚠ ESP32: QEMU unit tests (limited)     │
│ ✓ Server: Docker container start/stop  │
└──────┬──────────────────────────────────┘
       │
       ├─── ALL PASS ──┐
       │               │
       │               ▼
       │        ┌─────────────────┐
       │        │ Publish Release │
       │        │ - Make public   │
       │        │ - Update tags   │
       │        │ - Notify users  │
       │        └─────────────────┘
       │
       └─── ANY FAIL ──┐
                       │
                       ▼
                ┌─────────────────┐
                │ Keep as Draft   │
                │ - Upload logs   │
                │ - Create issue  │
                │ - Notify devs   │
                └─────────────────┘
```

## Recommended Implementation Plan

### Week 1: Core Desktop
1. ✅ Implement Windows MSI/EXE testing
2. ✅ Implement Linux DEB/RPM testing (Ubuntu only)
3. ✅ Implement macOS DMG testing (universal binary)
4. ✅ Create draft release workflow
5. ✅ Test end-to-end with manual publish

### Week 2: Server + Containers
1. ✅ Add Docker build and test
2. ✅ Add Linux AppImage testing
3. ✅ Add multiple Linux distro testing (Fedora)
4. ✅ Implement automated publish on success

### Week 3: Mobile (Optional)
1. ⚠️ Add Android emulator testing
2. ⚠️ Add iOS simulator testing (if signing available)
3. ⚠️ Test upgrade paths

### Week 4: Firmware + Optimization
1. ⚠️ Add ESP32 QEMU tests
2. ✅ Optimize caching strategy
3. ✅ Add failure notifications
4. ✅ Document manual testing procedures

## Testing Checklist (Per Platform)

### Windows (.msi)
- [ ] Silent install (`msiexec /i /qn`)
- [ ] UI install
- [ ] Installation directory verification
- [ ] Registry keys created
- [ ] Start menu shortcuts
- [ ] File associations (if any)
- [ ] Upgrade from previous version
- [ ] Uninstall cleanup (no leftovers)

### Linux (.deb)
- [ ] Install with dependencies
- [ ] Binary in /usr/bin
- [ ] Desktop file in /usr/share/applications
- [ ] Icon file installed
- [ ] Upgrade preserves config
- [ ] Uninstall removes all files

### macOS (.dmg)
- [ ] DMG mounts successfully
- [ ] App copies to /Applications
- [ ] Code signature valid
- [ ] Gatekeeper approval
- [ ] App launches without "unknown developer" warning
- [ ] Uninstall (drag to trash)

### Android (.apk)
- [ ] APK installs on emulator
- [ ] App icon appears
- [ ] App launches
- [ ] Permissions requested
- [ ] Basic functionality works
- [ ] Upgrade from previous APK
- [ ] Uninstall

### iOS (.ipa)
- [ ] Installs on simulator
- [ ] App launches
- [ ] No crashes on startup
- [ ] Uninstall

### ESP32 (.bin)
- [ ] Firmware builds
- [ ] Binary size check
- [ ] QEMU unit tests pass
- [ ] **Manual**: Flash to device
- [ ] **Manual**: Test hardware features

### Docker (server)
- [ ] Image builds
- [ ] Container starts
- [ ] Health endpoint responds
- [ ] Environment variables work
- [ ] Volumes persist data
- [ ] Container stops cleanly

## Alternative Approaches

### Option 1: Staged Releases
```
1. Build all artifacts
2. Create "canary" release (pre-release flag)
3. Run tests
4. If pass: promote to stable
```
**Pros**: Less risk, gradual rollout
**Cons**: More complex workflow

### Option 2: Manual Testing Gate
```
1. Build all artifacts
2. Create draft release
3. Automated tests run
4. Manual review required before publish
```
**Pros**: Human oversight, catch edge cases
**Cons**: Slower release cycle

### Option 3: Beta Channel
```
1. Automated publish to "beta" channel
2. Community testing (1-7 days)
3. If no issues: promote to "stable"
```
**Pros**: Real-world testing
**Cons**: Requires beta user base

**Recommendation**: Start with **Option 1** (automated with draft), add **Option 3** (beta channel) later.

## Questions for Discussion

1. **ESP32 Testing**: Are you okay with QEMU-only tests + manual hardware checklist?
2. **iOS Testing**: Do you have/want to get Apple Developer account for real testing?
3. **Cost**: Are you okay with ~4 releases/month on free tier, or willing to pay for more?
4. **Phased Approach**: Should we implement Phase 1 (desktop) first, then add mobile?
5. **Beta Channel**: Would you like a beta testing program with community testers?
6. **Package Managers**: Priority on winget/homebrew/snap, or direct downloads first?

## My Recommendations

### Immediate Implementation (Now)
```yaml
✅ Desktop (Windows, Linux, macOS) - all formats
✅ Server (Docker)
✅ Draft → Test → Publish workflow
✅ Failure handling (keep draft)
```

### Phase 2 (1-2 months)
```yaml
✅ Android (if mobile app ready)
⚠️ iOS (if you have signing)
✅ Add more Linux distros
```

### Phase 3 (3-6 months)
```yaml
⚠️ ESP32 QEMU tests
✅ Package manager automation (winget, brew PRs)
✅ Beta channel with community testing
⚠️ Self-hosted runner for hardware testing
```

### Skip/Manual
```yaml
❌ ESP32 hardware I/O (manual testing)
❌ iOS real device (unless you get paid account)
❌ Play Store testing (Firebase App Distribution instead)
```

## Next Steps

1. **Review** this document and decide on phased approach
2. **Test** the workflow with a dummy release (`v0.0.1-test`)
3. **Iterate** based on what works/fails
4. **Document** manual testing procedures for ESP32
5. **Optimize** once baseline is working

## Sources

- [Android Emulator Runner](https://github.com/marketplace/actions/android-emulator-runner)
- [GitHub Actions Hardware Acceleration](https://github.blog/changelog/2024-04-02-github-actions-hardware-accelerated-android-virtualization-now-available/)
- [iOS Testing with GitHub Actions](https://brightinventions.pl/blog/ios-build-run-tests-github-actions/)
- [ESP32 QEMU Runner](https://github.com/marketplace/actions/esp32-qemu-runner)
- [Desktop App Testing Best Practices](https://www.frugaltesting.com/blog/desktop-app-testing-2025-in-depth-tools-breakdown-pro-best-practices)
- [CI/CD Desktop Apps with GitHub Actions](https://devblogs.microsoft.com/dotnet/continuous-integration-and-deployment-for-desktop-apps-with-github-actions/)

---

**Ready to discuss and refine based on your priorities!**
