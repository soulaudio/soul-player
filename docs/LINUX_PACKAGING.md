# Linux Packaging and Dependencies

## System Dependencies

Soul Player requires the following system libraries on Linux:

- **ALSA** - Audio output (`libasound2`)
- **pkg-config** - Build tool for finding libraries

## Distribution Strategies

### 1. Tauri Bundler (Recommended for .deb/.rpm)

Tauri's bundler can automatically declare system dependencies in package metadata.

#### Configure in `applications/desktop/tauri.conf.json`:

```json
{
  "bundle": {
    "identifier": "com.soulplayer.app",
    "linux": {
      "deb": {
        "depends": [
          "libasound2",
          "libgtk-3-0",
          "libwebkit2gtk-4.1-0"
        ]
      },
      "rpm": {
        "depends": [
          "alsa-lib",
          "gtk3",
          "webkit2gtk4.1"
        ]
      }
    }
  }
}
```

**How it works:**
- When user installs `.deb` → `apt` automatically installs `libasound2`
- When user installs `.rpm` → `yum`/`dnf` automatically installs `alsa-lib`

**Build commands:**

```bash
# Build .deb package (Debian/Ubuntu)
cargo tauri build --target x86_64-unknown-linux-gnu --bundles deb

# Build .rpm package (Fedora/RHEL)
cargo tauri build --target x86_64-unknown-linux-gnu --bundles rpm
```

**Installation:**

```bash
# Debian/Ubuntu - dependencies auto-installed
sudo dpkg -i soul-player_0.1.0_amd64.deb
sudo apt-get install -f  # Fix any missing dependencies

# Fedora/RHEL
sudo rpm -i soul-player-0.1.0.x86_64.rpm
```

---

### 2. AppImage (Zero Dependencies - Recommended for Portability)

AppImage bundles **everything** including ALSA libraries, so no system dependencies needed.

#### Configure in `tauri.conf.json`:

```json
{
  "bundle": {
    "linux": {
      "appimage": {
        "bundleMediaFramework": true,
        "files": {
          "/usr/lib/x86_64-linux-gnu/libasound.so.2": "lib/libasound.so.2"
        }
      }
    }
  }
}
```

**Build:**

```bash
cargo tauri build --bundles appimage
```

**User experience:**

```bash
# No installation needed, just run
chmod +x soul-player_0.1.0_amd64.AppImage
./soul-player_0.1.0_amd64.AppImage
```

**Pros:**
- ✅ Works on any Linux distro (Ubuntu, Fedora, Arch, etc.)
- ✅ No root/sudo required
- ✅ No dependency conflicts

**Cons:**
- ❌ Larger file size (~100-150MB vs ~10-20MB)
- ❌ No automatic updates via system package manager

---

### 3. Flatpak (Sandboxed with Declared Dependencies)

Flatpak uses a manifest to declare dependencies, which Flathub handles automatically.

#### Create `com.soulplayer.App.yml`:

```yaml
id: com.soulplayer.App
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable

command: soul-player

finish-args:
  # Audio access
  - --socket=pulseaudio
  - --device=all
  # Filesystem access for music library
  - --filesystem=xdg-music
  - --filesystem=home

modules:
  - name: soul-player
    buildsystem: simple
    build-commands:
      - cargo tauri build --bundles deb
      - install -Dm755 target/release/soul-player /app/bin/soul-player
    sources:
      - type: dir
        path: .
```

**Build:**

```bash
flatpak-builder build-dir com.soulplayer.App.yml
flatpak build-export export build-dir
flatpak build-bundle export soul-player.flatpak com.soulplayer.App
```

**User installation:**

```bash
flatpak install soul-player.flatpak
flatpak run com.soulplayer.App
```

**Pros:**
- ✅ Sandboxed (better security)
- ✅ Automatic dependency resolution
- ✅ Works across distros

**Cons:**
- ❌ Larger initial runtime download
- ❌ Sandbox may restrict file access

---

### 4. Snap (Ubuntu's Native Format)

Snap packages declare dependencies in `snapcraft.yaml`.

#### Create `snapcraft.yaml`:

```yaml
name: soul-player
version: '0.1.0'
summary: Local-first music player
description: |
  Soul Player is a cross-platform music player with server streaming
  and hardware support.

grade: stable
confinement: strict
base: core22

apps:
  soul-player:
    command: bin/soul-player
    plugs:
      - audio-playback
      - home
      - removable-media
    desktop: usr/share/applications/soul-player.desktop

parts:
  soul-player:
    plugin: rust
    source: .
    build-packages:
      - pkg-config
      - libasound2-dev
      - libgtk-3-dev
    stage-packages:
      - libasound2
      - libgtk-3-0
    override-build: |
      cd applications/desktop
      cargo tauri build
      install -Dm755 target/release/soul-player $SNAPCRAFT_PART_INSTALL/bin/soul-player
```

**Build:**

```bash
snapcraft
```

**User installation:**

```bash
sudo snap install soul-player_0.1.0_amd64.snap --dangerous
```

**Pros:**
- ✅ Native Ubuntu integration
- ✅ Automatic updates
- ✅ Sandboxed

**Cons:**
- ❌ Mainly Ubuntu-focused
- ❌ Requires snapd daemon

---

## Comparison Matrix

| Method | Size | Deps Auto-Install | Cross-Distro | No Root Needed | Sandboxed |
|--------|------|-------------------|--------------|----------------|-----------|
| `.deb`/`.rpm` | Small | ✅ Yes | ❌ No | ❌ No | ❌ No |
| AppImage | Large | ✅ Bundled | ✅ Yes | ✅ Yes | ❌ No |
| Flatpak | Medium | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| Snap | Medium | ✅ Yes | ⚠️ Mostly | ✅ Yes | ✅ Yes |

---

## Recommended Approach

### For Initial Release

**Provide multiple formats:**

1. **AppImage** - Universal, no-hassle option for all users
2. **`.deb`** - Native Debian/Ubuntu package with auto-dependencies
3. **`.rpm`** - Native Fedora/RHEL package with auto-dependencies

**Build script:**

```bash
#!/bin/bash
# build-linux-packages.sh

cd applications/desktop

echo "Building .deb package..."
cargo tauri build --bundles deb

echo "Building .rpm package..."
cargo tauri build --bundles rpm

echo "Building AppImage..."
cargo tauri build --bundles appimage

echo "Packages built:"
ls -lh target/release/bundle/deb/
ls -lh target/release/bundle/rpm/
ls -lh target/release/bundle/appimage/
```

### For Production/Distribution

**Add Flatpak to Flathub** (most user-friendly):

1. Submit to Flathub (free hosting + discovery)
2. Users can install with: `flatpak install flathub com.soulplayer.App`
3. Automatic updates via Flatpak

---

## Tauri Configuration Example

Here's a complete `tauri.conf.json` configuration:

```json
{
  "productName": "Soul Player",
  "version": "0.1.0",
  "identifier": "com.soulplayer.app",
  "bundle": {
    "active": true,
    "targets": ["deb", "rpm", "appimage"],
    "linux": {
      "deb": {
        "depends": [
          "libasound2",
          "libgtk-3-0",
          "libwebkit2gtk-4.1-0",
          "libayatana-appindicator3-1"
        ],
        "section": "sound"
      },
      "rpm": {
        "depends": [
          "alsa-lib",
          "gtk3",
          "webkit2gtk4.1"
        ]
      },
      "appimage": {
        "bundleMediaFramework": true,
        "files": {}
      }
    },
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "resources": [],
    "copyright": "Copyright © 2026 Soul Player Contributors",
    "category": "AudioVideo",
    "shortDescription": "Local-first music player",
    "longDescription": "Soul Player is a cross-platform music player with multi-user server streaming and embedded hardware support."
  }
}
```

---

## Installation Instructions for Users

### Debian/Ubuntu

**Option 1: .deb package (recommended)**

```bash
# Download from releases page
wget https://github.com/yourusername/soul-player/releases/download/v0.1.0/soul-player_0.1.0_amd64.deb

# Install (dependencies auto-installed)
sudo dpkg -i soul-player_0.1.0_amd64.deb
sudo apt-get install -f

# Run
soul-player
```

**Option 2: AppImage (no installation)**

```bash
# Download from releases page
wget https://github.com/yourusername/soul-player/releases/download/v0.1.0/soul-player_0.1.0_amd64.AppImage

# Make executable and run
chmod +x soul-player_0.1.0_amd64.AppImage
./soul-player_0.1.0_amd64.AppImage
```

### Fedora/RHEL

```bash
# Download from releases page
wget https://github.com/yourusername/soul-player/releases/download/v0.1.0/soul-player-0.1.0.x86_64.rpm

# Install (dependencies auto-installed)
sudo rpm -i soul-player-0.1.0.x86_64.rpm

# Run
soul-player
```

### Arch Linux

**Option 1: AUR package (create PKGBUILD)**

```bash
# Create AUR package
yay -S soul-player-bin

# Or use AppImage
```

**Option 2: AppImage**

```bash
chmod +x soul-player_0.1.0_amd64.AppImage
./soul-player_0.1.0_amd64.AppImage
```

### Any Linux Distribution

**AppImage (universal)**

```bash
# Download
wget https://github.com/yourusername/soul-player/releases/download/v0.1.0/soul-player_0.1.0_amd64.AppImage

# Run
chmod +x soul-player_0.1.0_amd64.AppImage
./soul-player_0.1.0_amd64.AppImage

# Optional: Integrate with system
# Use AppImageLauncher or manually create .desktop file
```

---

## Checking Dependencies at Runtime

You can also add a runtime check in your app to verify dependencies and provide helpful error messages:

```rust
// In applications/desktop/src/main.rs

fn check_audio_dependencies() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        // Check if ALSA is available
        use std::process::Command;

        let output = Command::new("ldconfig")
            .arg("-p")
            .output()
            .map_err(|e| format!("Failed to check dependencies: {}", e))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        if !output_str.contains("libasound.so") {
            return Err(
                "Missing audio dependency: libasound2\n\n\
                 Please install it:\n\
                 - Debian/Ubuntu: sudo apt-get install libasound2\n\
                 - Fedora/RHEL: sudo dnf install alsa-lib\n\
                 - Arch: sudo pacman -S alsa-lib".to_string()
            );
        }
    }

    Ok(())
}

fn main() {
    // Check dependencies before starting
    if let Err(e) = check_audio_dependencies() {
        eprintln!("Dependency check failed:\n{}", e);
        std::process::exit(1);
    }

    // Start Tauri app
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## GitHub Actions CI/CD

Add build jobs for all Linux formats:

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            pkg-config \
            libasound2-dev \
            libgtk-3-dev \
            libwebkit2gtk-4.1-dev \
            libayatana-appindicator3-dev

      - uses: dtolnay/rust-toolchain@stable

      - name: Install Tauri CLI
        run: cargo install tauri-cli --locked

      - name: Build packages
        run: |
          cd applications/desktop
          cargo tauri build --bundles deb,rpm,appimage

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: linux-packages
          path: |
            applications/desktop/target/release/bundle/deb/*.deb
            applications/desktop/target/release/bundle/rpm/*.rpm
            applications/desktop/target/release/bundle/appimage/*.AppImage
```

---

## Summary

**Best approach for Soul Player:**

1. **Build all three formats** (.deb, .rpm, AppImage) on each release
2. **Recommend AppImage** in README for easiest installation
3. **Provide native packages** (.deb/.rpm) for users who prefer system integration
4. **Consider Flatpak** for Flathub submission once stable

**Dependencies are handled automatically** when users install via:
- `.deb` → apt installs libasound2
- `.rpm` → dnf installs alsa-lib
- AppImage → everything bundled (no system deps)
- Flatpak → runtime provides dependencies

**No manual dependency installation required** for end users! 🎉
