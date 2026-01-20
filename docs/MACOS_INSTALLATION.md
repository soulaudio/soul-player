# macOS Installation Guide

## Installing Soul Player on macOS

Soul Player uses **ad-hoc code signing** for macOS builds, which means the app is signed but not notarized by Apple. This is a free alternative to paid Apple Developer Program membership ($99/year) and is suitable for open-source projects.

## Installation Steps

### 1. Download the DMG

Download the appropriate DMG for your Mac:
- **Apple Silicon (M1/M2/M3)**: `soul_player_*_aarch64.dmg`
- **Intel Mac**: `soul_player_*_x64.dmg`

### 2. Mount the DMG

Double-click the downloaded DMG file to mount it. A Finder window will open showing the Soul Player app.

### 3. Install the App

Drag the **Soul Player.app** to your **Applications** folder.

### 4. First Launch

When launching Soul Player for the first time, macOS Gatekeeper may show a warning:

#### Option A: "App cannot be opened" or "App is damaged"

If you see this error:

1. Open **Terminal** (Applications → Utilities → Terminal)
2. Run this command:
   ```bash
   xattr -cr "/Applications/Soul Player.app"
   ```
3. Try opening the app again

This removes the quarantine attribute that macOS adds to downloaded files.

#### Option B: "App from unidentified developer"

If you see this warning:

1. Click **Cancel** (don't move to Trash)
2. Open **System Settings** → **Privacy & Security**
3. Scroll down to the **Security** section
4. Click **Open Anyway** next to the Soul Player message
5. Click **Open** in the confirmation dialog

Alternatively:
1. Right-click (or Control-click) on **Soul Player** in Applications
2. Select **Open** from the context menu
3. Click **Open** in the dialog

### 5. Normal Usage

After the first successful launch, Soul Player will open normally like any other app. You won't need to repeat these steps.

## Why These Steps Are Needed

Soul Player uses **ad-hoc code signing**, which means:
- ✅ The app is signed and verified
- ✅ The app won't show "damaged" errors (after removing quarantine)
- ❌ The app is not notarized by Apple (requires $99/year developer account)

This is the same approach used by many open-source macOS applications. It's safe, but macOS requires the extra confirmation steps on first launch.

## Technical Details

- **Minimum macOS Version**: macOS 10.15 (Catalina)
- **Code Signature Type**: Ad-hoc (local signing identity `-`)
- **Hardened Runtime**: Enabled
- **Entitlements**: Includes necessary permissions for audio playback and file access

## Upgrading to Notarized Builds (Future)

If Soul Player gains funding or sponsorship, we may upgrade to full notarization, which would:
- Remove the "unidentified developer" warning
- Provide automatic app updates through macOS
- Offer a smoother installation experience

For now, ad-hoc signing provides a balance between security and cost for an open-source project.

## Troubleshooting

### "App is damaged and can't be opened"

**Solution**: Remove quarantine attribute:
```bash
xattr -cr "/Applications/Soul Player.app"
```

### "App can't be opened because Apple cannot check it for malicious software"

**Solution**:
1. System Settings → Privacy & Security
2. Click "Open Anyway"

Or:
1. Right-click app → Open
2. Click "Open" in dialog

### Verify Code Signature

To verify the app is properly signed:
```bash
codesign -dv --verbose=4 "/Applications/Soul Player.app"
```

You should see `Signature=adhoc` in the output, confirming the ad-hoc signature is present.

## Need Help?

- GitHub Issues: https://github.com/soulaudio/soul-player/issues
- Documentation: https://github.com/soulaudio/soul-player/tree/main/docs

---

**Note**: These instructions are specific to Soul Player v0.1.x and later, which uses Tauri v2 with ad-hoc code signing configured in `tauri.conf.json`.
