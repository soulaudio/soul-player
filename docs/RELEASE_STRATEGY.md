# Release Strategy and CI/CD Pipeline

## Overview

Comprehensive multi-platform release pipeline with automated installation testing before publishing.

## Pipeline Stages

```
1. Build All Platforms → 2. Create Draft Release → 3. Test Installations → 4. Publish Release
                                                    ↓
                                           (Fail = Keep Draft)
```

## Platforms and Artifacts

### Desktop Applications

| Platform | Artifacts | Package Managers | Installation Methods |
|----------|-----------|------------------|---------------------|
| **Windows** | `.exe`, `.msi`, `.msix` | winget, chocolatey, scoop | MSI installer, MSIX bundle, portable .exe |
| **Linux** | `.deb`, `.rpm`, `.AppImage` | apt, yum/dnf, snap, flatpak | Native package managers, AppImage |
| **macOS** | `.dmg`, `.pkg`, `.app` | Homebrew, MacPorts | DMG installer, PKG installer |

### Mobile Applications

| Platform | Artifacts | Distribution | Testing Method |
|----------|-----------|--------------|----------------|
| **Android** | `.apk`, `.aab` | Google Play, F-Droid | Hardware-accelerated emulator |
| **iOS** | `.ipa` | App Store, TestFlight | Xcode simulator (requires signing) |

### Firmware

| Platform | Artifacts | Distribution | Testing Method |
|----------|-----------|--------------|----------------|
| **ESP32-S3** | `.bin`, `.elf` | GitHub Releases, OTA | QEMU simulation (unit tests only) |

### Server

| Platform | Artifacts | Distribution | Testing Method |
|----------|-----------|--------------|----------------|
| **Docker** | Docker image | Docker Hub, GHCR | Container smoke test |

## Installation Testing Strategy

### 1. Desktop Testing (Full Coverage)

#### Windows
```yaml
Test Scenarios:
  - Fresh install (.msi)
  - Upgrade from previous version
  - Uninstall and cleanup verification
  - Silent install (for enterprise)
  - Per-user vs system-wide install
  - Registry key verification
  - Start menu shortcut verification
  - File association verification
```

**Tools**:
- PowerShell scripts for automated testing
- `msiexec` for MSI testing
- WiX Toolset for installer validation
- Windows Package Manager (`winget`) validation

#### Linux
```yaml
Test Scenarios (per format):
  .deb:
    - dpkg install
    - apt install (with dependencies)
    - Upgrade
    - Uninstall with dependency cleanup
  .rpm:
    - rpm install
    - yum/dnf install
    - Upgrade
    - Uninstall
  .AppImage:
    - Permission verification
    - Direct execution
    - Desktop integration (optional)
```

**Distributions Tested**:
- Ubuntu 22.04 LTS, 24.04 LTS
- Fedora latest
- Debian stable

#### macOS
```yaml
Test Scenarios:
  - DMG mount and app installation
  - PKG installer execution
  - Code signature verification
  - Gatekeeper approval
  - Launch and smoke test
  - Uninstall (drag to trash)
  - Homebrew cask installation
```

### 2. Mobile Testing

#### Android
```yaml
Test Scenarios:
  - APK installation on emulator (API 30, 33, 34)
  - App launch and basic functionality
  - Permission grants
  - Upgrade from previous version
  - Uninstall
```

**Emulator Configuration**:
- Hardware acceleration (KVM)
- Multiple API levels
- Different screen sizes

**Limitations**:
- No Play Store testing (would need Firebase App Distribution)
- No real device testing

#### iOS
```yaml
Test Scenarios:
  - IPA installation on simulator
  - App launch and basic functionality
  - Upgrade simulation
  - Uninstall
```

**Limitations**:
- Requires code signing (can use development certificates)
- Simulator only (no real device in CI)
- Cannot test App Store installation flow
- Limited to iOS versions available in GitHub Actions (currently up to 18.1)

### 3. Firmware Testing (Limited)

#### ESP32-S3
```yaml
Test Scenarios:
  - Firmware build verification
  - QEMU-based unit tests
  - Binary size check
  - OTA update package generation
```

**Limitations**:
- **Cannot test hardware I/O** (audio output, SD card, display, WiFi)
- QEMU simulation for CPU/memory tests only
- **Recommendation**: Leave placeholder for hardware testing

**Future Enhancement**:
- Consider hardware-in-the-loop (HIL) testing with real ESP32 devices
- Self-hosted runners with USB-connected ESP32 boards

### 4. Server Testing

```yaml
Test Scenarios:
  - Docker image build
  - Container startup
  - Health check endpoint
  - Environment variable configuration
  - Volume mounting
  - Port exposure
  - Container stop/cleanup
```

## Test Pass Criteria

### Critical (Must Pass)

- ✅ All desktop installers install successfully
- ✅ All desktop apps launch without crashes
- ✅ Upgrade preserves user data/settings
- ✅ Uninstall removes all files (no leftovers)
- ✅ Android APK installs and launches
- ✅ iOS IPA installs and launches (if signed)
- ✅ Docker container starts and responds

### Non-Critical (Can Fail)

- ⚠️ ESP32 firmware QEMU tests (hardware limitations)
- ⚠️ iOS tests if signing unavailable
- ⚠️ Package manager installations (winget, brew) - optional

## Workflow Structure

### Stage 1: Build Matrix
```
Jobs:
  - build-windows
  - build-linux
  - build-macos
  - build-android
  - build-ios
  - build-esp32
  - build-server
```

### Stage 2: Create Draft Release
```
Job: create-release
  - Upload all artifacts to draft release
  - Generate release notes
  - Tag version
```

### Stage 3: Installation Tests (Parallel)
```
Jobs:
  - test-windows-install
  - test-linux-install (matrix: ubuntu, fedora)
  - test-macos-install
  - test-android-install
  - test-ios-install
  - test-esp32-qemu
  - test-server-docker
```

### Stage 4: Publish Release
```
Job: publish-release
  needs: [all-test-jobs]
  if: success()
  - Publish draft release
  - Update latest tag
  - Notify distribution channels
```

## Best Practices Applied

### 1. Clean Test Environments

**Windows**:
- Use fresh Windows Server 2022 runners
- Reset registry hives between tests
- Clear Program Files directories

**Linux**:
- Use Docker containers for each test
- Fresh Ubuntu/Fedora images
- Clean /opt, /usr/local, ~/.local

**macOS**:
- Use macos-latest runners
- Clear /Applications before tests
- Reset LaunchServices database

### 2. Idempotent Tests

All tests must be repeatable:
```bash
# Example: Linux test structure
setup() {
  # Clean environment
  sudo apt-get purge -y soul-player || true
  rm -rf ~/.config/soul-player
}

install_test() {
  sudo dpkg -i soul-player.deb
  soul-player --version
}

upgrade_test() {
  # Install old version first
  sudo dpkg -i soul-player-v0.0.9.deb
  # Upgrade to new
  sudo dpkg -i soul-player-v0.1.0.deb
  soul-player --version | grep "0.1.0"
}

uninstall_test() {
  sudo apt-get purge -y soul-player
  ! command -v soul-player  # Should not exist
  ! [ -d "/opt/soul-player" ]  # Should be gone
}

teardown() {
  # Cleanup
  sudo apt-get purge -y soul-player || true
}
```

### 3. Version Matrix Testing

Test against multiple versions:
- **Windows**: Windows 10, Windows 11, Windows Server 2022
- **Linux**: Ubuntu 22.04, 24.04, Fedora latest, Debian stable
- **macOS**: macOS 13 (Ventura), 14 (Sonoma), 15 (Sequoia)
- **Android**: API 30, 33, 34 (representative of market share)
- **iOS**: iOS 16, 17, 18 (if available)

### 4. Artifact Verification

Before publishing:
```yaml
verify:
  - Check file signatures (Windows Authenticode, macOS codesign)
  - Verify checksums (SHA256)
  - Scan for malware (VirusTotal API)
  - Validate package metadata
  - Check binary sizes (flag anomalies)
```

### 5. Rollback Strategy

If tests fail:
- Keep draft release unpublished
- Add failure comments to draft
- Notify developers via GitHub issues
- **Do not auto-publish failed releases**

## Installation Methods to Test

### Windows

1. **MSI Installer** (Primary)
   ```powershell
   msiexec /i soul-player.msi /qn  # Silent install
   msiexec /i soul-player.msi /qb  # Basic UI
   ```

2. **MSIX Package** (Microsoft Store)
   ```powershell
   Add-AppxPackage soul-player.msix
   ```

3. **Portable Executable**
   ```powershell
   .\soul-player.exe --version
   ```

4. **Winget** (Windows Package Manager)
   ```powershell
   winget install SoulPlayer
   ```

5. **Chocolatey**
   ```powershell
   choco install soul-player
   ```

### Linux

1. **Debian Package**
   ```bash
   sudo dpkg -i soul-player.deb
   sudo apt-get install -f  # Fix dependencies
   ```

2. **RPM Package**
   ```bash
   sudo rpm -i soul-player.rpm
   # or
   sudo dnf install soul-player.rpm
   ```

3. **AppImage**
   ```bash
   chmod +x soul-player.AppImage
   ./soul-player.AppImage
   ```

4. **Snap**
   ```bash
   sudo snap install soul-player.snap --dangerous
   ```

5. **Flatpak**
   ```bash
   flatpak install soul-player.flatpak
   ```

### macOS

1. **DMG**
   ```bash
   hdiutil attach soul-player.dmg
   cp -R "/Volumes/Soul Player/Soul Player.app" /Applications/
   hdiutil detach "/Volumes/Soul Player"
   ```

2. **PKG Installer**
   ```bash
   sudo installer -pkg soul-player.pkg -target /
   ```

3. **Homebrew**
   ```bash
   brew install --cask soul-player
   ```

### Android

1. **APK Direct Install**
   ```bash
   adb install soul-player.apk
   ```

2. **Via Emulator**
   ```yaml
   - uses: reactivecircus/android-emulator-runner@v2
     with:
       api-level: 30
       script: |
         adb install soul-player.apk
         adb shell monkey -p com.soulplayer.app -c android.intent.category.LAUNCHER 1
   ```

### iOS

1. **Simulator**
   ```bash
   xcrun simctl install booted soul-player.app
   xcrun simctl launch booted com.soulplayer.app
   ```

2. **TestFlight** (Manual, not CI)
   - Upload via App Store Connect API
   - External tester installation

## Notification Strategy

### On Success
- ✅ GitHub Release published
- ✅ Discord/Slack notification
- ✅ Update download page
- ✅ Trigger distribution pipeline (winget, homebrew PRs)

### On Failure
- ❌ Keep draft release
- ❌ Create GitHub issue with failure details
- ❌ Tag @maintainers
- ❌ Include test logs

## Timeline Estimate

```
Build Stage:         20-30 minutes
Draft Creation:      2-3 minutes
Testing Stage:       15-25 minutes (parallel)
Publishing:          2-3 minutes
---
Total:              ~40-60 minutes per release
```

## Cost Considerations

GitHub Actions minutes (free tier: 2000 min/month):
- Linux runners: 1x multiplier
- macOS runners: 10x multiplier (expensive!)
- Windows runners: 2x multiplier

**Optimization**:
- Use self-hosted runners for Linux
- Limit macOS matrix (test only latest 2 versions)
- Cache build dependencies aggressively

## Future Enhancements

### Phase 2: Real Device Testing

1. **Android**: Firebase Test Lab integration
2. **iOS**: Real device farm (BrowserStack, Sauce Labs)
3. **ESP32**: Self-hosted runner with USB-connected boards

### Phase 3: Performance Testing

- Installation time benchmarks
- App startup time measurement
- Memory usage profiling
- Cold start vs warm start

### Phase 4: Security Testing

- Code signing verification
- Binary integrity checks
- Malware scanning
- Dependency vulnerability scanning

## References

- [Android Emulator Runner](https://github.com/marketplace/actions/android-emulator-runner)
- [GitHub Actions Hardware Acceleration](https://github.blog/changelog/2024-04-02-github-actions-hardware-accelerated-android-virtualization-now-available/)
- [iOS Testing with GitHub Actions](https://brightinventions.pl/blog/ios-build-run-tests-github-actions/)
- [ESP32 QEMU Runner](https://github.com/marketplace/actions/esp32-qemu-runner)
- [Desktop App CI/CD Best Practices](https://www.frugaltesting.com/blog/desktop-app-testing-2025-in-depth-tools-breakdown-pro-best-practices)
- [.NET CI/CD with GitHub Actions](https://devblogs.microsoft.com/dotnet/continuous-integration-and-deployment-for-desktop-apps-with-github-actions/)
