# Linux Uninstallation Behavior

## Problem
After uninstalling Soul Player on Linux with `yay -Rns soul-player` and reinstalling, the app does not show the onboarding screen - it loads the existing library data from the previous installation.

## Root Cause
This is **intentional Linux behavior**, not a bug. Linux package managers follow the XDG Base Directory Specification, which separates system files (managed by the package manager) from user data (owned by the user).

**What gets removed on uninstall:**
- `/usr/bin/soul-player` (binary)
- `/usr/share/applications/soul-player.desktop` (desktop entry)
- `/usr/share/icons/hicolor/*/apps/soul-player.png` (icons)

**What is preserved on uninstall:**
- `~/.config/soul-player/soul-player.db` (database)
- `~/.config/soul-player/logs/` (log files)
- `~/.config/soul-player/config.json` (app configuration)
- `~/.config/soul-player/library/` (imported music files)

This matches the behavior of Firefox, VS Code, Docker, and other Linux applications.

## Solutions Implemented

### Solution 1: In-App Factory Reset (Natural UX)

Added **Settings > Data Management** with a "Reset to Factory Settings" button.

**Features:**
- ✅ Confirmation dialog requires typing "reset"
- ✅ Shows exactly what will be deleted
- ✅ Cross-platform (works on Windows/macOS/Linux)
- ✅ Gracefully stops playback and closes database before deletion
- ✅ App exits after reset (user must manually relaunch)
- ✅ Next launch shows onboarding screen

**Implementation:**
- Frontend: `applications/shared/src/components/settings/DataManagementSettingsPage.tsx`
- Backend: `applications/desktop/src-tauri/src/main.rs` (`reset_to_factory_settings` command)
- Routing: `applications/desktop/src/pages/SettingsRouter.tsx`

### Solution 2: CLI Cleanup Script (For Manual Cleanup)

Created `scripts/cleanup-linux-userdata.sh` for users who want to clean up after uninstallation.

**Features:**
- ✅ Shows what will be deleted and total size
- ✅ Interactive confirmation (or `--force` flag)
- ✅ Supports custom `XDG_CONFIG_HOME`
- ✅ Safe error handling

**Usage:**
```bash
# Interactive mode
./scripts/cleanup-linux-userdata.sh

# Silent mode (for scripts)
./scripts/cleanup-linux-userdata.sh --force
```

### Solution 3: Package Manager Messages

Updated `.aur/PKGBUILD` to show post-removal instructions:

**post_remove hook:**
```bash
echo "Your user data is preserved at: ~/.config/soul-player/"
echo "To completely remove all data:"
echo "  rm -rf ~/.config/soul-player"
```

Users see this message every time they uninstall, guiding them on how to clean up if desired.

## Best Practices Comparison

| Application | Behavior on Uninstall |
|-------------|----------------------|
| Firefox     | Preserves `~/.mozilla/firefox/` profiles |
| VS Code     | Preserves `~/.config/Code/` settings and extensions |
| Docker      | Preserves `/var/lib/docker/` volumes (separate `docker system prune`) |
| Steam       | Separate "Uninstall game" vs "Delete game data" |
| **Soul Player** | Preserves `~/.config/soul-player/` + in-app reset button |

## User Experience Flow

### Normal Uninstall/Reinstall
```bash
yay -Rns soul-player   # Package removed, data preserved
# App data: ~/.config/soul-player/ still exists
yay -S soul-player     # Reinstall
# App loads with existing library intact ✓
```

### Fresh Start (Option A: In-App)
1. Open Soul Player
2. Settings > Data Management
3. Click "Reset to Factory Settings"
4. Type "reset" to confirm
5. App closes automatically
6. Relaunch app → Onboarding screen ✓

### Fresh Start (Option B: Manual)
```bash
yay -Rns soul-player
./scripts/cleanup-linux-userdata.sh  # Or: rm -rf ~/.config/soul-player
yay -S soul-player
# App shows onboarding screen ✓
```

## Documentation Updates

1. **LINUX_INSTALLATION.md** - Added full uninstallation guide
2. **scripts/cleanup-linux-userdata.sh** - CLI cleanup tool
3. **.aur/soul-player.install** - Post-removal message
4. **Settings UI** - Data Management page with factory reset

## Why This Approach?

**Prevents Accidental Data Loss:**
- Users upgrading packages don't lose their library
- System updates don't wipe user data
- Matches user expectations from other Linux apps

**Provides Control:**
- Users who want to keep data: do nothing (default)
- Users who want fresh start: use in-app reset or script
- Clear documentation for both paths

**Cross-Platform Consistency:**
- macOS: Users manually delete `~/Library/Application Support/soul-player/`
- Windows: Users manually delete `%APPDATA%\Soul Player\`
- Linux: Users manually delete `~/.config/soul-player/` OR use in-app reset
- **In-app reset works on all platforms** ✓

## Files Modified

```
applications/
  ├── shared/
  │   ├── src/
  │   │   ├── components/settings/
  │   │   │   ├── DataManagementSettingsPage.tsx  (NEW)
  │   │   │   └── SettingsSidebar.tsx  (UPDATED)
  │   │   └── i18n/en-US.json  (UPDATED - added translations)
  │   └── src/index.ts  (UPDATED - export DataManagementSettingsPage)
  └── desktop/
      ├── src/
      │   ├── App.tsx  (UPDATED - use SettingsRouter)
      │   └── pages/
      │       └── SettingsRouter.tsx  (NEW - nested settings routes)
      └── src-tauri/src/
          └── main.rs  (UPDATED - added reset_to_factory_settings command)

scripts/
  └── cleanup-linux-userdata.sh  (NEW)

.aur/
  ├── PKGBUILD  (UPDATED - added install script)
  └── soul-player.install  (NEW)

docs/
  ├── LINUX_INSTALLATION.md  (NEW)
  └── LINUX_UNINSTALL.md  (THIS FILE)
```

## Testing

### Test 1: In-App Reset
```bash
# 1. Start with populated library
# 2. Settings > Data Management > Reset to Factory Settings
# 3. Type "reset" and confirm
# 4. App should close
# 5. Relaunch app
# Expected: Onboarding screen appears ✓
```

### Test 2: Manual Cleanup
```bash
# 1. Install and use app
yay -Rns soul-player
ls ~/.config/soul-player/  # Should exist with data
./scripts/cleanup-linux-userdata.sh
ls ~/.config/soul-player/  # Should not exist
yay -S soul-player
# Expected: Onboarding screen appears ✓
```

### Test 3: Upgrade Scenario
```bash
# 1. Install v0.1.6 and add music
yay -S soul-player  # v0.1.7
# Expected: Library data preserved, no onboarding ✓
```

## Summary

The Linux uninstallation behavior is **correct and intentional**. We've added:

1. **In-app factory reset** (Settings > Data Management) - most user-friendly
2. **CLI cleanup script** - for manual cleanup after uninstall
3. **Package manager messages** - guides users on data cleanup
4. **Documentation** - clear explanation of behavior

Users now have full control over their data with clear paths for both preservation and deletion.
