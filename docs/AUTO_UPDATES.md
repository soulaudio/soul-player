# Auto-Update System

Soul Player uses [Tauri's built-in updater](https://v2.tauri.app/plugin/updater/) to deliver seamless updates across all platforms.

---

## Overview

The auto-update system works on the following platforms with proper signature verification:

- ✅ **Windows** - NSIS installer (.exe) - **Auto-update supported**
- ✅ **Linux** - AppImage (.AppImage) - **Auto-update supported**
- ⚠️ **Linux** - DEB/RPM packages - **Auto-update NOT supported** (use package manager)
- ✅ **macOS** - .app.tar.gz (both Intel and Apple Silicon) - **Auto-update supported**

### Important: Linux Package Manager Updates

According to [Tauri documentation](https://v2.tauri.app/plugin/updater/), **only AppImage supports auto-updates on Linux**. If you installed Soul Player via:

- **DEB package** (Debian, Ubuntu, Mint, Pop!_OS): Use `sudo apt update && sudo apt upgrade soul-player`
- **RPM package** (Fedora, RHEL, CentOS, openSUSE): Use `sudo dnf upgrade soul-player` or `sudo yum upgrade soul-player`
- **Flatpak**: Use `flatpak update io.github.soulaudio.SoulPlayer`
- **AUR** (Arch Linux): Use `yay -Syu soul-player`

---

## Update Manifest

The CI/CD pipeline generates a **single update manifest** following Tauri standards:

### `latest.json` (Tauri Updater Manifest)

**URL**: `https://github.com/soulaudio/soul-player/releases/latest/download/latest.json`

**Supported Platforms**:
- **Windows x64**: NSIS installer
- **Linux x64**: AppImage (universal, works on all distros)
- **macOS ARM64**: .app.tar.gz (Apple Silicon)
- **macOS x64**: .app.tar.gz (Intel)

---

## How Auto-Updates Work

### User Experience

1. **Update Check**: App checks for updates periodically (or manually via Settings)
2. **Download**: If available, update is downloaded in background
3. **Verification**: Signature is verified using Tauri's public key
4. **Installation Prompt**: User is notified when update is ready
5. **Seamless Update**: User clicks "Install", app restarts with new version

### Security

All auto-update artifacts are **cryptographically signed** using Tauri's signing mechanism:

- **Windows**: NSIS installer + `.sig` file
- **Linux**: AppImage + `.sig` file
- **macOS**: .app.tar.gz + `.sig` file (both architectures)

The updater **verifies signatures** before applying updates, preventing tampering.

**Note**: DEB/RPM packages are also signed (available in releases) but are not used by the Tauri updater - they rely on system package manager verification instead.

---

## Configuration

### Tauri Updater Config

The updater is configured in `applications/desktop/src-tauri/tauri.conf.json`:

```json
{
  "updater": {
    "active": true,
    "endpoints": [
      "https://github.com/soulaudio/soul-player/releases/latest/download/latest.json"
    ],
    "dialog": true,
    "pubkey": "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDJGQ0M4OUZEN0M5NEMyNzcKUldRc1o3bXVuV2V6WVlTU09wckhnOFFoR2lWRmFWQnJ2bERNVWIzK0VmZ2FxeHJ3V3p2Z015VWwK"
  }
}
```

### Dynamic Update Endpoints (Advanced)

For advanced scenarios like release channels, Tauri supports [dynamic variables](https://v2.tauri.app/plugin/updater/) in endpoint URLs:

- `{{current_version}}` - Current app version
- `{{target}}` - OS (linux, windows, darwin)
- `{{arch}}` - Architecture (x86_64, aarch64, etc.)

**Example** (multi-channel setup):
```json
{
  "updater": {
    "endpoints": [
      "https://releases.myserver.com/{{target}}-{{arch}}/{{current_version}}/latest.json"
    ]
  }
}
```

Your server can respond with **204 No Content** if no update is available for that platform/version.

---

## Build Process

### Signature Generation

All auto-update packages are signed during the build process using GitHub Secrets:

- `TAURI_SIGNING_PRIVATE_KEY` - Private key for signing
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` - Key password

**Windows & macOS**: Tauri automatically signs during build (NSIS, .app.tar.gz)
**Linux AppImage**: Custom signing step in `.appimage/build-appimage.sh` using `tauri signer sign`
**Linux DEB/RPM**: Tauri automatically signs during build (for package manager verification, not updater)

### Update Manifest Generation

The `.github/workflows/release.yml` workflow automatically generates the update manifest:

1. **Builds all platforms** - Windows (NSIS), Linux (AppImage, DEB, RPM), macOS (.app.tar.gz, DMG)
2. **Signs packages** - Generates `.sig` files for updater-supported formats
3. **Collects signatures** - Extracts signatures for Windows, Linux AppImage, macOS
4. **Generates `latest.json`** - Creates Tauri updater manifest with all supported platforms
5. **Uploads to release** - Publishes manifest to GitHub Release

**Manifest Contents**:
```json
{
  "version": "0.1.7",
  "notes": "Release notes...",
  "pub_date": "2026-01-23T12:00:00Z",
  "platforms": {
    "windows-x86_64": { "signature": "...", "url": "..." },
    "linux-x86_64": { "signature": "...", "url": "..." },
    "darwin-aarch64": { "signature": "...", "url": "..." },
    "darwin-x86_64": { "signature": "...", "url": "..." }
  }
}
```

---

## Manual Update Check

Users can manually check for updates from the app:

1. Open Soul Player
2. Go to **Settings** > **About**
3. Click **Check for Updates**

---

## Troubleshooting

### Update Check Fails

**Symptoms**: "Failed to check for updates" error

**Causes**:
- No internet connection
- GitHub API rate limit exceeded
- Invalid update manifest URL

**Fix**: Check internet connection, wait a few minutes, try again

### Signature Verification Fails

**Symptoms**: "Update signature verification failed" error

**Causes**:
- Corrupted download
- Manifest was modified (security risk)
- Public key mismatch

**Fix**:
1. Try again (may be network issue)
2. If persists, file a bug report - do NOT install the update

### Update Installed But App Crashes

**Symptoms**: App crashes after update installation

**Causes**:
- Database migration failed
- Incompatible config format
- Missing dependencies

**Fix**:
1. Check logs in app data directory
2. Reinstall manually from GitHub Releases
3. File a bug report with logs

---

## Developer Notes

### Testing Auto-Updates Locally

You cannot test auto-updates locally without publishing a release. To test:

1. Create a draft release with test artifacts
2. Update `tauri.conf.json` to point to your draft release
3. Build and run the app
4. Trigger update check

### Generating Signing Keys

If you need to rotate signing keys:

```bash
# Generate new key pair
tauri signer generate

# Update GitHub Secrets
# - TAURI_SIGNING_PRIVATE_KEY (from .tauri-key)
# - TAURI_SIGNING_PRIVATE_KEY_PASSWORD

# Update tauri.conf.json with new public key
```

⚠️ **WARNING**: Changing signing keys will break auto-updates for existing users. Only do this if absolutely necessary (key compromise).

---

## Related Documentation

- [Tauri Updater Guide](https://v2.tauri.app/plugin/updater/)
- [Release Pipeline](.github/workflows/release.yml)
- [Build Configuration](../applications/desktop/src-tauri/tauri.conf.json)
- [AppImage Build Script](../.appimage/build-appimage.sh)

---

## Why AppImage Only for Linux Auto-Updates?

According to [Tauri's updater documentation](https://v2.tauri.app/plugin/updater/), the built-in updater only supports formats that can be updated **in-place** without requiring system-level package manager integration:

- ✅ **AppImage** - Self-contained, can replace itself without root/sudo
- ❌ **DEB** - Requires `dpkg` and system-level installation (use `apt upgrade`)
- ❌ **RPM** - Requires `rpm` and system-level installation (use `dnf upgrade`)
- ❌ **Flatpak** - Requires `flatpak` runtime (use `flatpak update`)

This design ensures auto-updates work **without requiring elevated privileges** and remain consistent across all Linux distributions.

---

## References

- [Tauri Updater Plugin](https://v2.tauri.app/plugin/updater/) - Official documentation
- [AppImage Distribution](https://v2.tauri.app/distribute/appimage/) - AppImage packaging guide
- [Debian Packages](https://v2.tauri.app/distribute/debian/) - DEB packaging guide
- [RPM Packages](https://v2.tauri.app/distribute/rpm/) - RPM packaging guide
- [GitHub Actions Pipeline](https://v2.tauri.app/distribute/pipelines/github/) - CI/CD setup

---

**Last Updated**: 2026-01-23
