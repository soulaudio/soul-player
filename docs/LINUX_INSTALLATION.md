# Linux Installation Guide

## Installing Soul Player on Linux

Soul Player provides multiple installation methods for Linux users:

### 1. Arch Linux (AUR)

Install from the Arch User Repository using an AUR helper:

```bash
# Using yay
yay -S soul-player

# Using paru
paru -S soul-player
```

### 2. AppImage (Universal)

Download the AppImage for your architecture:
- **x86_64**: `soul-player_*_amd64.AppImage`
- **ARM64**: `soul-player_*_arm64.AppImage`

Make it executable and run:

```bash
chmod +x soul-player_*.AppImage
./soul-player_*.AppImage
```

**Optional**: Integrate with your desktop environment using [AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher).

### 3. Debian/Ubuntu (DEB)

```bash
sudo dpkg -i soul-player_*_amd64.deb
sudo apt-get install -f  # Install dependencies if needed
```

## Data Storage Location

Soul Player stores all user data in the XDG config directory:

```
~/.config/soul-player/
├── soul-player.db          # SQLite database (library, playlists, settings)
├── config.json             # App configuration cache
├── logs/                   # Log files (if enabled)
└── library/                # Managed music files (if using import feature)
```

Or if you have `XDG_CONFIG_HOME` set:
```
$XDG_CONFIG_HOME/soul-player/
```

## Uninstalling Soul Player

### Standard Uninstall (Preserves User Data)

```bash
# AUR (using yay)
yay -Rns soul-player

# DEB package
sudo apt remove soul-player
```

**Important**: This removes the application binary but **preserves your user data** (database, settings, logs). This is standard Linux behavior to prevent accidental data loss.

### Complete Uninstall (Remove All Data)

To completely remove Soul Player including all user data:

#### Option 1: Use the cleanup script (Recommended)

```bash
# Step 1: Uninstall the package
yay -Rns soul-player  # or sudo apt remove soul-player

# Step 2: Run the cleanup script
./scripts/cleanup-linux-userdata.sh
```

The script will:
- Show you exactly what will be deleted
- Display the total size of data
- Ask for confirmation before proceeding
- Remove `~/.config/soul-player/` and all its contents

#### Option 2: Manual cleanup

```bash
# Step 1: Uninstall the package
yay -Rns soul-player

# Step 2: Manually remove user data
rm -rf ~/.config/soul-player
```

### Fresh Reinstall

If you want to start fresh after updating or encountering issues:

```bash
# 1. Uninstall
yay -Rns soul-player

# 2. Remove user data
./scripts/cleanup-linux-userdata.sh

# 3. Reinstall
yay -S soul-player
```

The app will now show the onboarding screen and start with a clean slate.

## Why User Data Is Preserved

Linux package managers follow the **XDG Base Directory Specification** and **Filesystem Hierarchy Standard (FHS)**, which distinguish between:

- **System files** (managed by package manager):
  - `/usr/bin/soul-player` - application binary
  - `/usr/share/applications/soul-player.desktop` - desktop entry
  - `/usr/share/icons/hicolor/*/apps/soul-player.png` - icons

- **User data** (owned by user, NOT managed by package manager):
  - `~/.config/soul-player/` - user-specific configuration and data
  - This is NOT removed during uninstall to prevent accidental data loss

This is the same behavior as Firefox, VS Code, and other Linux applications - uninstalling the package does not delete your profile data.

## Troubleshooting

### "Database is locked" error

If you see database lock errors:

1. Close all running instances of Soul Player
2. Check for background processes:
   ```bash
   ps aux | grep soul-player
   killall soul-player
   ```
3. Restart the app

### Clean install not showing onboarding

If you uninstalled and reinstalled but the app still shows your old library:

- User data was not removed during uninstall (this is intentional)
- Run the cleanup script: `./scripts/cleanup-linux-userdata.sh`
- Or manually delete: `rm -rf ~/.config/soul-player`

### Different data location on your system

Check where your data is stored:

```bash
# If XDG_CONFIG_HOME is set
echo ${XDG_CONFIG_HOME:-~/.config}/soul-player

# List contents
ls -la ${XDG_CONFIG_HOME:-~/.config}/soul-player/
```

## Technical Details

- **Minimum Requirements**: WebKit2GTK 2.40+, GTK3
- **Architecture**: x86_64 (amd64), ARM64 (aarch64)
- **Desktop Integration**: Follows XDG Desktop Entry specification
- **File Associations**: Registers handlers for MP3, FLAC, WAV, OGG, M4A, etc.

## Need Help?

- GitHub Issues: https://github.com/soulaudio/soul-player/issues
- Documentation: https://github.com/soulaudio/soul-player/tree/main/docs

---

**Note**: These instructions are for Soul Player v0.1.x and later using Tauri v2.
