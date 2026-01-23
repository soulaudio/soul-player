# Installation Method Detection

Soul Player automatically detects how it was installed on Linux and provides appropriate update instructions based on the installation method.

---

## Overview

On Linux, Soul Player can be installed via multiple package formats:
- **AppImage** - Self-contained, supports in-app auto-updates ✅
- **DEB** - Debian/Ubuntu package, updates via `apt` ❌
- **RPM** - Fedora/RHEL package, updates via `dnf` ❌
- **Flatpak** - Sandboxed app, updates via `flatpak` ❌
- **Snap** - Confined app, updates via `snap` ❌
- **AUR** - Arch User Repository, updates via AUR helper ❌

Only **AppImage** supports Tauri's built-in auto-updater. Other formats must be updated through their respective package managers.

---

## Architecture

### Backend (Rust)

**File**: `applications/desktop/src-tauri/src/installation.rs`

The detection logic runs on the Rust backend and uses multiple strategies:

1. **Environment Variables** (most reliable)
   - `APPIMAGE` - Set when running as AppImage
   - `FLATPAK_ID` - Set when running in Flatpak sandbox
   - `SNAP` / `SNAP_NAME` - Set when running in Snap confinement

2. **Process Execution Path** (`/proc/self/exe`)
   - AppImage: `/tmp/.mount_*` or contains `appimage`
   - Flatpak: `/app/` or `/.var/app/`
   - Snap: `/snap/`

3. **Package Manager Detection**
   - DEB: Check `/var/lib/dpkg/info/soul-player.list` or `dpkg -l`
   - RPM: Check `/etc/redhat-release` + `rpm -q`
   - AUR: Check `/etc/arch-release`

4. **Fallback**: Unknown installation method

**Detection Flow**:
```
Check APPIMAGE env var
  ↓ (if not found)
Check FLATPAK_ID env var
  ↓ (if not found)
Check SNAP env vars
  ↓ (if not found)
Read /proc/self/exe symlink
  ↓ (check path hints)
Check package manager databases
  ↓ (dpkg, rpm, pacman)
Return Unknown
```

**Tauri Command**:
```rust
#[tauri::command]
pub fn get_installation_info() -> InstallationInfo {
    InstallationInfo {
        method: InstallationMethod::AppImage,  // or Deb, Rpm, etc.
        update_command: Some("sudo apt upgrade soul-player".to_string()),
        supports_auto_update: false,
    }
}
```

### Frontend (React)

**File**: `applications/desktop/src/components/UpdateDialog.tsx`

When the update dialog opens, it:

1. **Fetches installation info** via Tauri command:
   ```typescript
   invoke<InstallationInfo>('get_installation_info')
   ```

2. **Conditionally renders** based on `supports_auto_update`:
   - **AppImage**: Shows "Install Now" button (uses Tauri updater)
   - **Other formats**: Shows package manager command with "Copy" button

3. **Displays update instructions**:
   ```typescript
   {!supportsAutoUpdate && updateCommand && (
     <div className="package-manager-instructions">
       <code>{updateCommand}</code>
       <button onClick={handleCopyCommand}>Copy</button>
     </div>
   )}
   ```

---

## User Experience

### AppImage Users

When an update is available:

```
┌─────────────────────────────────────┐
│ Update Available                 ✕  │
├─────────────────────────────────────┤
│ A new version is available          │
│ v0.1.8                              │
│                                     │
│ What's New                          │
│ - Bug fixes                         │
│ - Performance improvements          │
│                                     │
│         [Later]   [Install Now]     │
└─────────────────────────────────────┘
```

**Action**: Click "Install Now" → Download + install automatically

### DEB/RPM/Flatpak Users

When an update is available:

```
┌─────────────────────────────────────┐
│ Update Available                 ✕  │
├─────────────────────────────────────┤
│ A new version is available          │
│ v0.1.8                              │
│                                     │
│ What's New                          │
│ - Bug fixes                         │
│ - Performance improvements          │
│                                     │
│ ⚠ Package Manager Update Required   │
│ Your installation requires updating │
│ via your system package manager:    │
│                                     │
│ sudo apt upgrade soul-player [Copy] │
│                                     │
│       [Later]   [View Release]      │
└─────────────────────────────────────┘
```

**Action**: Click "Copy" → Paste in terminal → Run command

---

## Marketing Site

**File**: `applications/marketing/src/components/LinuxDownloadModal.tsx`

The download page clearly indicates which formats support auto-updates:

```
┌─────────────────────────────────────┐
│ Download for Linux                  │
├─────────────────────────────────────┤
│ Automatic Updates                   │
│                                     │
│ ✅ AppImage - In-app auto-updates   │
│ ❌ DEB - Use apt upgrade            │
│ ❌ RPM - Use dnf upgrade            │
│ ❌ Flatpak - Use flatpak update     │
│ ❌ AUR - Use yay -Syu               │
│                                     │
│ Note: Only AppImage supports in-app │
│ auto-updates.                       │
└─────────────────────────────────────┘
```

---

## Localization

All user-facing strings are localized in `applications/shared/src/i18n/en-US.json`:

```json
{
  "updateDialog": {
    "packageManagerUpdateRequired": "Package Manager Update Required",
    "packageManagerUpdateDescription": "Your installation method requires updating via your system package manager. Copy and run the command below:",
    "copy": "Copy",
    "copied": "Copied!",
    "viewRelease": "View Release"
  }
}
```

---

## Testing

### Manual Testing (Linux)

**AppImage**:
```bash
# Download AppImage
wget https://github.com/soulaudio/soul-player/releases/latest/download/soul-player_0.1.7_x86_64.AppImage
chmod +x soul-player_0.1.7_x86_64.AppImage

# Run
./soul-player_0.1.7_x86_64.AppImage

# Check Settings > About > Check for Updates
# Should show "Install Now" button
```

**DEB**:
```bash
# Install DEB
sudo dpkg -i Soul.Player_0.1.7_amd64.deb

# Run app
soul-player

# Check Settings > About > Check for Updates
# Should show "sudo apt upgrade soul-player" command
```

**RPM**:
```bash
# Install RPM
sudo dnf install Soul.Player-0.1.7-1.x86_64.rpm

# Run app
soul-player

# Check Settings > About > Check for Updates
# Should show "sudo dnf upgrade soul-player" command
```

### Unit Tests

The Rust detection logic includes unit tests:

```bash
cd applications/desktop/src-tauri
cargo test installation
```

**Test Coverage**:
- ✅ Update command generation
- ✅ Auto-update support detection
- ✅ Installation info creation
- ✅ Environment variable detection (when set)

---

## Cross-Platform Behavior

### Windows & macOS

On non-Linux platforms, `detect_installation_method()` always returns:

```rust
InstallationInfo {
    method: InstallationMethod::AppImage,  // Equivalent to "standard install"
    update_command: None,
    supports_auto_update: true,
}
```

This ensures auto-updates work seamlessly on Windows (NSIS installer) and macOS (.app.tar.gz bundles).

---

## Future Enhancements

### 1. Flatpak Permissions

If we publish to Flathub, we could detect Flatpak and offer to open the update in GNOME Software:

```typescript
if (installationInfo.method.type === 'flatpak') {
  // Open in GNOME Software
  window.open('gnome-software://update/io.github.soulaudio.SoulPlayer');
}
```

### 2. Package Repository Detection

For DEB/RPM, we could check if the package was installed from our own repository vs. GitHub release:

```rust
// Check if installed from PPA
if Path::new("/etc/apt/sources.list.d/soulaudio-ppa.list").exists() {
    // Recommend apt update first
}
```

### 3. Snap Auto-Updates

Snap packages auto-update by default. We could detect this and show a different message:

```
✅ Snap - Automatic updates via snapd (updates happen automatically in background)
```

---

## References

- [Tauri Updater Plugin](https://v2.tauri.app/plugin/updater/) - Auto-update documentation
- [Linux Installation Detection](https://wiki.archlinux.org/title/XDG_Base_Directory) - Environment variables
- [Flatpak Detection](https://docs.flatpak.org/en/latest/conventions.html#environment-variables)
- [AppImage Specification](https://docs.appimage.org/packaging-guide/environment-variables.html)

---

**Last Updated**: 2026-01-23
