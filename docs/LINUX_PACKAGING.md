# Linux Packaging Guide

This document explains all Linux packaging options for Soul Player.

## Current Support

✅ **Implemented:**
- Debian/Ubuntu (.deb)
- Fedora/RHEL (.rpm)

📋 **Planned:**
- Arch Linux (AUR)
- AppImage (universal binary)
- Flatpak (universal package)

---

## Arch Linux (AUR)

### Overview

The **Arch User Repository (AUR)** is the standard way to distribute community packages for Arch Linux. Users are very familiar with this approach.

### Package Types

We'll provide two AUR packages:

1. **soul-player-bin** (Binary) - Downloads pre-built DEB, extracts, and installs
   - Fast installation
   - No compilation needed
   - Recommended for most users

2. **soul-player** (Source) - Builds from source code
   - Builds everything from scratch
   - For users who prefer building from source
   - Requires Rust, Node.js, build tools

### Setup Instructions

See `.aur/README.md` for detailed PKGBUILD files and publishing instructions.

**Quick Start:**
```bash
# 1. Create AUR account at https://aur.archlinux.org/register
# 2. Add SSH key to your AUR account
# 3. Clone the AUR repository
git clone ssh://aur@aur.archlinux.org/soul-player-bin.git

# 4. Copy PKGBUILD
cp .aur/PKGBUILD-bin soul-player-bin/PKGBUILD

# 5. Generate .SRCINFO
cd soul-player-bin
makepkg --printsrcinfo > .SRCINFO

# 6. Test build
makepkg -si

# 7. Publish
git add PKGBUILD .SRCINFO
git commit -m "Initial release: 0.1.1"
git push
```

### User Installation (After Publishing)

```bash
# Using yay
yay -S soul-player-bin

# Using paru
paru -S soul-player-bin

# Manual
git clone https://aur.archlinux.org/soul-player-bin.git
cd soul-player-bin
makepkg -si
```

---

## AppImage (Universal)

### Overview

**AppImage** is a universal Linux binary format that works on all distributions. Users can download a single file and run it without installation.

### Advantages
- ✅ Works on all distros (Arch, Debian, Fedora, etc.)
- ✅ No installation required
- ✅ Portable (can run from USB drive)
- ✅ Sandboxed

### Disadvantages
- ❌ Larger file size (includes all dependencies)
- ❌ No automatic updates (users must download new versions)
- ❌ Doesn't integrate as well with system (desktop files, etc.)

### Implementation

Tauri doesn't natively support AppImage, but we can create one using `linuxdeploy`:

```bash
# Install linuxdeploy
wget https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
chmod +x linuxdeploy-x86_64.AppImage

# Build AppImage
./linuxdeploy-x86_64.AppImage \
  --executable target/release/soul-player \
  --desktop-file applications/desktop/src-tauri/soul-player.desktop \
  --icon-file applications/desktop/src-tauri/icons/128x128.png \
  --appdir AppDir \
  --output appimage
```

**GitHub Actions Integration:**
```yaml
- name: Build AppImage
  run: |
    # Install dependencies
    sudo apt-get install -y libfuse2

    # Download linuxdeploy
    wget https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
    chmod +x linuxdeploy-x86_64.AppImage

    # Build AppImage
    ./linuxdeploy-x86_64.AppImage \
      --executable target/release/soul-player \
      --desktop-file applications/desktop/src-tauri/soul-player.desktop \
      --icon-file applications/desktop/src-tauri/icons/128x128.png \
      --appdir AppDir \
      --output appimage

    # Rename for consistency
    mv Soul_Player-*.AppImage soul_player_${VERSION}_x86_64.AppImage
```

**User Installation:**
```bash
# Download AppImage
wget https://github.com/soulaudio/soul-player/releases/download/v0.1.1/soul_player_0.1.1_x86_64.AppImage

# Make executable
chmod +x soul_player_0.1.1_x86_64.AppImage

# Run
./soul_player_0.1.1_x86_64.AppImage
```

---

## Flatpak (Universal)

### Overview

**Flatpak** is a universal package format with sandboxing. It's integrated into many Linux distributions and provides automatic updates.

### Advantages
- ✅ Works on all distros
- ✅ Sandboxed security
- ✅ Automatic updates via Flathub
- ✅ Good system integration
- ✅ Can be distributed via Flathub

### Disadvantages
- ❌ Requires Flatpak runtime
- ❌ More complex setup
- ❌ Sandbox can cause compatibility issues

### Implementation

Create a Flatpak manifest:

**`soul-player.yml`:**
```yaml
app-id: io.github.soulaudio.SoulPlayer
runtime: org.gnome.Platform
runtime-version: '45'
sdk: org.gnome.Sdk
command: soul-player

finish-args:
  # X11 + Wayland access
  - --socket=x11
  - --socket=wayland
  # Audio access
  - --socket=pulseaudio
  # File access
  - --filesystem=home
  # Network (for metadata fetching)
  - --share=network

modules:
  - name: soul-player
    buildsystem: simple
    build-commands:
      # Build commands here
      - cargo build --release
      - install -Dm755 target/release/soul-player /app/bin/soul-player
    sources:
      - type: archive
        url: https://github.com/soulaudio/soul-player/archive/v0.1.1.tar.gz
        sha256: ...
```

**Build Flatpak:**
```bash
# Install flatpak-builder
sudo apt-get install flatpak-builder

# Build
flatpak-builder --repo=repo --force-clean build-dir soul-player.yml

# Install locally for testing
flatpak-builder --user --install --force-clean build-dir soul-player.yml

# Run
flatpak run io.github.soulaudio.SoulPlayer
```

**Publish to Flathub:**
1. Fork https://github.com/flathub/flathub
2. Add your app manifest
3. Submit pull request
4. After approval, users can install with:
   ```bash
   flatpak install flathub io.github.soulaudio.SoulPlayer
   ```

---

## Comparison

| Format | Setup Effort | User Experience | Updates | Distro Support |
|--------|-------------|----------------|---------|---------------|
| **DEB** | Low | Excellent (Debian) | Manual | Debian/Ubuntu |
| **RPM** | Low | Excellent (RHEL) | Manual | Fedora/RHEL |
| **AUR** | Medium | Excellent (Arch) | AUR helpers | Arch Linux |
| **AppImage** | Medium | Good | Manual download | All distros |
| **Flatpak** | High | Excellent | Automatic | All distros |

## Recommendations

### Phase 1 (Current)
- ✅ DEB (Debian/Ubuntu)
- ✅ RPM (Fedora/RHEL)

### Phase 2 (Short-term)
- 🎯 **AUR** (Arch Linux) - Easy to implement, big user base
  - Publish `soul-player-bin` (binary package)
  - Optionally publish `soul-player` (source package)

### Phase 3 (Medium-term)
- 🎯 **AppImage** - Universal binary for all other distros
  - Works immediately on any distro
  - Good fallback for unsupported distributions

### Phase 4 (Long-term)
- 🎯 **Flatpak** - Polished universal package
  - Publish to Flathub for discoverability
  - Automatic updates
  - Better sandboxing

## Resources

- **AUR Guidelines**: https://wiki.archlinux.org/title/AUR_submission_guidelines
- **AppImage Documentation**: https://docs.appimage.org/
- **Flatpak Documentation**: https://docs.flatpak.org/
- **Flathub Submission**: https://github.com/flathub/flathub
