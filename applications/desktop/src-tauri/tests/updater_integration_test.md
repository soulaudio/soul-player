# Updater Integration Tests

## Overview

The updater module (`src/updater.rs`) integrates with Tauri's updater plugin to provide automatic and manual update functionality. Due to the external dependency on Tauri's UpdaterExt trait, most testing must be done through integration tests rather than unit tests.

## Test Scenarios

### 1. Background Update Checker

**Scenario**: Verify background checker starts and runs on schedule

**Test Steps**:
1. Start the application
2. Verify log message: `[UPDATER] Starting initial update check on startup`
3. Wait 3 seconds (initial delay)
4. Verify log message: `[UPDATER] Running scheduled update check`
5. Verify update check occurs again after 1 hour

**Expected Behavior**:
- Initial check runs after 3-second delay
- Subsequent checks run hourly
- Checker respects `app.auto_update_enabled` setting

### 2. Manual Update Check - No Updates Available

**Scenario**: User clicks "Check Now" button when already on latest version

**Test Steps**:
1. Navigate to Settings > General > Updates
2. Click "Check Now" button
3. Wait for response

**Expected Behavior**:
- Button shows "Checking..." while in progress
- Button is disabled during check
- Toast message: "You're on the latest version!"
- Button returns to "Check Now" state

### 3. Manual Update Check - Update Available

**Scenario**: User clicks "Check Now" and update is available

**Test Steps**:
1. Navigate to Settings > General > Updates
2. Ensure `auto_update_enabled` is true
3. Click "Check Now" button
4. Wait for update detection

**Expected Behavior**:
- UpdateDialog opens automatically
- Dialog shows version number (e.g., "v1.5.0")
- Dialog shows release notes if available
- "Install Now" button is visible

### 4. Auto-Update Disabled

**Scenario**: Verify background checker respects settings

**Test Steps**:
1. Set `app.auto_update_enabled` to `false` in settings
2. Wait for next scheduled check
3. Check logs

**Expected Behavior**:
- Log message: `[UPDATER] Auto-update disabled in settings, skipping check`
- No update check is performed
- No update dialog appears

### 5. Silent Update Installation

**Scenario**: Update installs automatically without user interaction

**Test Steps**:
1. Set `app.auto_update_enabled` to `true`
2. Set `app.auto_update_silent` to `true`
3. Trigger an update detection (or wait for background check)

**Expected Behavior**:
- Update downloads automatically
- Log message: `[UPDATER] Starting silent install`
- Log message: `[UPDATER] Silent install completed successfully`
- Log message: `[UPDATER] Restarting app to apply update`
- Application restarts automatically
- New version is running after restart

### 6. Manual Update Installation

**Scenario**: User manually installs update via dialog

**Test Steps**:
1. Trigger update-available event
2. UpdateDialog appears
3. Click "Install Now" button
4. Monitor progress bar

**Expected Behavior**:
- Progress bar shows download progress (0-100%)
- "Install Now" button changes to "Installing..." and becomes disabled
- "Later" button becomes disabled
- Close button becomes disabled
- Dialog cannot be closed during installation
- Application restarts when installation completes

### 7. Update Progress Tracking

**Scenario**: Verify progress events are emitted and handled

**Test Steps**:
1. Start manual update installation
2. Monitor progress bar updates

**Expected Behavior**:
- Progress starts at 0%
- Progress updates incrementally
- Progress reaches 100% before installation completes
- UI updates reflect progress changes

### 8. Update Installation Error Handling

**Scenario**: Handle update installation failures gracefully

**Test Steps**:
1. Simulate network interruption during download
2. Or simulate disk full error during download

**Expected Behavior**:
- Log message: `[UPDATER] Silent install failed: <error>`
- Or log message: `[UPDATER] Manual install completed, restarting app` doesn't appear
- User can retry or dismiss dialog
- Application remains functional

### 9. Package Manager Detection (Linux)

**Scenario**: Detect installation method and show appropriate update instructions

**Test Steps**:
1. Install via DEB package
2. Open UpdateDialog when update is available

**Expected Behavior**:
- Dialog shows "Package Manager Update Required" section
- Update command is displayed: `sudo apt update && sudo apt upgrade soul-player`
- "Copy" button copies command to clipboard
- "View Release" link opens GitHub release page
- "Install Now" button is NOT shown

**Installation Methods to Test**:
- AppImage: Shows "Install Now" button
- DEB: Shows apt command
- RPM: Shows dnf command
- Flatpak: Shows flatpak command
- Snap: Shows snap command
- AUR: Shows yay command

### 10. GitHub Release Link Extraction

**Scenario**: Extract and display GitHub release link from release notes

**Test Steps**:
1. Trigger update with release notes containing GitHub URL
2. Example body: `"New features:\n- Feature 1\n\nFull changelog: https://github.com/soulaudio/soul-player/releases/tag/v1.5.0"`

**Expected Behavior**:
- "View full release notes →" link appears
- Link opens GitHub release page in browser
- Link opens in new tab (target="_blank")

### 11. Update Settings Persistence

**Scenario**: Verify settings persist across app restarts

**Test Steps**:
1. Navigate to Settings > General > Updates
2. Disable "Automatically check for updates"
3. Enable "Install updates silently"
4. Restart application
5. Return to Settings > General > Updates

**Expected Behavior**:
- "Automatically check for updates" remains unchecked
- "Install updates silently" remains checked (but disabled due to auto-update being off)
- Settings persist in SQLite database

### 12. Event Listener Cleanup

**Scenario**: Verify event listeners are cleaned up on unmount

**Test Steps**:
1. Navigate to Settings page
2. Navigate away from Settings page
3. Return to Settings page
4. Trigger update-available event

**Expected Behavior**:
- No duplicate event handlers
- Only one UpdateDialog appears
- No memory leaks from event listeners

## Test Environment Setup

### Prerequisites

```bash
# Install dependencies
yarn install

# Build Rust backend
cargo build --release

# Run app in development mode
yarn dev:desktop
```

### Enable Logging

Start the app with logging enabled to see updater messages:

```bash
# Set RUST_LOG environment variable
RUST_LOG=info yarn dev:desktop

# Or check log files (after enabling in settings)
# Windows: %APPDATA%\Soul Player\logs\
# macOS: ~/Library/Application Support/soul-player/logs/
# Linux: ~/.config/soul-player/logs/
```

### Mock Update Server (Optional)

For testing without actual releases, you can mock the update server:

1. Create a local `latest.json` file:
```json
{
  "version": "99.99.99",
  "notes": "Test update with mock release notes",
  "pub_date": "2024-01-15T00:00:00Z",
  "platforms": {
    "linux-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6...",
      "url": "https://github.com/soulaudio/soul-player/releases/download/v99.99.99/soul-player_99.99.99_amd64.AppImage"
    }
  }
}
```

2. Update `tauri.conf.json` updater endpoints to point to local server

3. Serve the file locally:
```bash
python3 -m http.server 8000
```

## Automated Testing

### Unit Tests

Run Rust unit tests:
```bash
cargo test --package soul-player --lib installation
```

### Frontend Tests

Run TypeScript tests:
```bash
# Run all tests
yarn test

# Run specific test file
yarn test UpdateDialog.test.tsx

# Run with coverage
yarn test --coverage
```

### Integration Tests

Integration tests require a running application and are best performed manually or via E2E testing frameworks.

**Future Enhancement**: Add Playwright or Tauri's test framework for automated integration tests.

## CI/CD Considerations

### Pre-Release Checklist

Before releasing a new version:

1. ✅ Verify updater endpoint is configured correctly
2. ✅ Generate and sign update artifacts
3. ✅ Create `latest.json` with correct signatures
4. ✅ Test update flow on all platforms (Windows, macOS, Linux)
5. ✅ Test all installation methods (AppImage, DEB, RPM, Flatpak, etc.)
6. ✅ Verify update rollback works if installation fails
7. ✅ Test silent update mode
8. ✅ Test manual update mode
9. ✅ Verify release notes display correctly

### Known Limitations

1. **Tauri Updater Plugin**: Cannot easily mock in unit tests
2. **Network Dependency**: Requires internet connection for real tests
3. **Platform-Specific**: Some features only work on specific platforms
4. **Signing Required**: Updates must be properly signed to install

## Debugging Tips

### Common Issues

**Issue**: Update check fails immediately

**Solution**:
- Check network connectivity
- Verify updater endpoint URL in `tauri.conf.json`
- Check if `latest.json` is accessible at the endpoint
- Review logs for error messages

**Issue**: Progress events not firing

**Solution**:
- Verify `update-progress` event listener is registered
- Check that event emission is not throttled
- Ensure handler updates state correctly

**Issue**: Installation fails silently

**Solution**:
- Check update signatures match
- Verify file permissions
- Check disk space
- Review error logs

**Issue**: Application doesn't restart after update

**Solution**:
- Check logs for `restart()` call
- Verify no modal dialogs are blocking
- Check OS-specific restart behavior

## Performance Metrics

Track these metrics during testing:

- **Initial Check Delay**: Should be ~3 seconds
- **Check Frequency**: Should be ~1 hour
- **Download Speed**: Varies by network
- **Progress Update Frequency**: Should update at least every 5%
- **Installation Time**: Should complete within 1-2 minutes
- **Restart Time**: Should occur immediately after installation

## Security Considerations

- ✅ Updates are signed with private key
- ✅ Signatures are verified before installation
- ✅ HTTPS is used for update endpoint
- ✅ No sensitive data is logged
- ✅ User can opt out of auto-updates
- ✅ Silent updates can be disabled

## Future Improvements

1. Add E2E tests with Playwright/Tauri test framework
2. Add unit tests with mocked Tauri APIs
3. Add performance benchmarks for update downloads
4. Add telemetry for update success/failure rates
5. Add update rollback mechanism
6. Add delta updates for faster downloads
7. Add bandwidth throttling for downloads
8. Add resume capability for interrupted downloads
