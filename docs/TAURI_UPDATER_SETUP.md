# Tauri Updater Setup Guide

This guide explains how to set up cryptographic signing for Tauri auto-updates.

## Overview

Soul Player uses Tauri's built-in updater system to automatically deliver updates to users. The updater requires:

1. **Signing keypair** - Public/private key for cryptographic signatures
2. **GitHub Secrets** - Private key stored securely in CI/CD
3. **Public key** - Embedded in the application for signature verification
4. **latest.json** - Manifest file describing available updates

## Current Status

✅ **Already Configured:**
- `tauri.conf.json` configured with GitHub endpoint
- `createUpdaterArtifacts: true` enabled in bundle config
- GitHub Actions workflow updated to:
  - Sign all artifacts during build
  - Generate `.sig` signature files
  - Create `latest.json` manifest
  - Upload everything to GitHub releases
- `.gitignore` configured to exclude private keys

❌ **Manual Setup Required:**
- Generate signing keypair (one-time setup)
- Add public key to `tauri.conf.json`
- Add private key to GitHub Secrets

## Setup Instructions

### Step 1: Generate Signing Keys

Run this command from the project root:

```bash
cd applications/desktop
npx @tauri-apps/cli signer generate -w ../../.tauri-keys/soul-player.key
```

**Important:**
- You'll be prompted to set a password - use a strong password and save it
- This creates two keys:
  - **Private key:** `.tauri-keys/soul-player.key` (never commit!)
  - **Public key:** Will be displayed in the terminal output

The output will look like:

```
Your keypair was generated successfully
Private: dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5...
Public key: dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6...
```

**Save both keys immediately** - you'll need them in the next steps.

### Step 2: Update tauri.conf.json

Copy the public key from the output above and replace the placeholder in `applications/desktop/src-tauri/tauri.conf.json`:

```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/soulaudio/soul-player/releases/latest/download/latest.json"
      ],
      "pubkey": "PASTE_YOUR_PUBLIC_KEY_HERE"
    }
  }
}
```

Commit this change - the public key is safe to commit.

### Step 3: Add Private Key to GitHub Secrets

1. Go to your repository on GitHub
2. Navigate to **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Add two secrets:

**Secret 1: TAURI_SIGNING_PRIVATE_KEY**
- Name: `TAURI_SIGNING_PRIVATE_KEY`
- Value: Paste the entire private key from Step 1

**Secret 2: TAURI_SIGNING_PRIVATE_KEY_PASSWORD**
- Name: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- Value: The password you set when generating the key

### Step 4: Verify Setup

To verify everything is working:

1. Create a new tag and push it:
   ```bash
   git tag v0.0.2
   git push origin v0.0.2
   ```

2. The workflow will automatically:
   - Build all platform binaries
   - Sign each binary with your private key
   - Generate `.sig` files
   - Create `latest.json` with signatures
   - Upload everything to GitHub releases

3. Check the release at: `https://github.com/soulaudio/soul-player/releases/tag/v0.0.2`

4. Verify these files exist:
   - `Soul Player_0.0.2_x64-setup.exe`
   - `Soul Player_0.0.2_x64-setup.exe.sig`
   - `soul-player_0.0.2_amd64.deb`
   - `soul-player_0.0.2_amd64.deb.sig`
   - `Soul Player_0.0.2_aarch64.dmg`
   - `Soul Player_0.0.2_aarch64.dmg.sig`
   - `latest.json`

## How Auto-Updates Work

### User Experience

1. User installs Soul Player v0.0.1
2. Application periodically checks for updates (configurable)
3. When v0.0.2 is released:
   - App downloads `latest.json`
   - Verifies signature matches public key
   - Shows update notification to user
   - Downloads new installer
   - Installs update in background
   - Restarts app (Windows) or prompts user (macOS/Linux)

### Security

- All artifacts are cryptographically signed
- Signatures verified using public key embedded in app
- Tampering with files = signature verification fails
- Man-in-the-middle attacks prevented
- Only updates signed with your private key will be accepted

## Implementing Update Checks in the App

The updater is configured but not yet implemented in the frontend. To add update checking:

### 1. Add Updater Permission

The updater permission is already configured in `tauri.conf.json`.

### 2. Implement Update Check in React

Create a new hook `useAutoUpdater.ts`:

```typescript
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { useState, useEffect } from 'react';

export function useAutoUpdater() {
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<any>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);

  // Check for updates on mount
  useEffect(() => {
    checkForUpdates();
  }, []);

  async function checkForUpdates() {
    try {
      const update = await check();
      if (update) {
        setUpdateAvailable(true);
        setUpdateInfo({
          version: update.version,
          date: update.date,
          body: update.body
        });
      }
    } catch (error) {
      console.error('Update check failed:', error);
    }
  }

  async function downloadAndInstall() {
    try {
      const update = await check();
      if (!update) return;

      setDownloading(true);

      // Listen to download progress
      update.onEvent((event) => {
        if (event.status === 'DOWNLOADING') {
          const { downloaded, contentLength } = event.data;
          const percent = Math.round((downloaded / contentLength) * 100);
          setDownloadProgress(percent);
        }
      });

      // Download and install
      await update.downloadAndInstall();

      // Restart app
      await relaunch();
    } catch (error) {
      console.error('Update installation failed:', error);
      setDownloading(false);
    }
  }

  return {
    updateAvailable,
    updateInfo,
    downloading,
    downloadProgress,
    checkForUpdates,
    downloadAndInstall
  };
}
```

### 3. Add Update Notification UI

In your main layout or settings page:

```tsx
import { useAutoUpdater } from './hooks/useAutoUpdater';

function UpdateNotification() {
  const { updateAvailable, updateInfo, downloading, downloadProgress, downloadAndInstall } = useAutoUpdater();

  if (!updateAvailable) return null;

  return (
    <div className="update-notification">
      <h3>Update Available: v{updateInfo.version}</h3>
      <p>{updateInfo.body}</p>

      {downloading ? (
        <div>
          <progress value={downloadProgress} max={100} />
          <span>Downloading: {downloadProgress}%</span>
        </div>
      ) : (
        <button onClick={downloadAndInstall}>
          Update Now
        </button>
      )}
    </div>
  );
}
```

### 4. Automatic Update Checks

For periodic background checks (every 6 hours):

```typescript
useEffect(() => {
  // Check immediately
  checkForUpdates();

  // Check every 6 hours
  const interval = setInterval(() => {
    checkForUpdates();
  }, 6 * 60 * 60 * 1000);

  return () => clearInterval(interval);
}, []);
```

## Troubleshooting

### "Public key verification failed"
- Ensure `pubkey` in `tauri.conf.json` matches the public key used to sign
- Verify you copied the entire public key string

### "No update available" (but you know there is)
- Check `latest.json` exists at the endpoint
- Verify version number in `latest.json` is higher than current version
- Check console for network errors

### Build fails with signature error
- Verify `TAURI_SIGNING_PRIVATE_KEY` is set in GitHub Secrets
- Verify `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` is correct
- Check the private key format is correct (should start with `dW50cnVzdGVkIGNvbW1lbnQ6`)

### Update downloads but won't install
- Windows: Check if installer requires admin rights (NSIS currentUser mode doesn't)
- macOS: User must approve security prompt on first launch
- Linux: Package manager permissions may be required

## Key Management Best Practices

1. **Backup your private key** - Store it securely (password manager, encrypted backup)
2. **Never commit private key** - It's in `.gitignore` but double-check
3. **Use strong password** - Protects the private key file
4. **Rotate keys periodically** - Generate new keypair every 1-2 years
5. **Document key rotation** - Create migration plan for existing users

## Key Rotation Process

If you need to rotate keys (lost key, security breach, etc.):

1. Generate new keypair
2. Update `tauri.conf.json` with new public key
3. Update GitHub Secrets with new private key
4. Release new version signed with NEW key
5. Old versions will need manual update (can't auto-update with new key)

## References

- [Tauri v2 Updater Documentation](https://v2.tauri.app/plugin/updater/)
- [Tauri Signing Documentation](https://v2.tauri.app/distribute/sign/)
- [GitHub Actions Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)

## Summary

Once setup is complete:

1. ✅ All releases will be automatically signed
2. ✅ `latest.json` will be generated with each release
3. ✅ Users will be able to auto-update securely
4. ✅ Signatures prevent tampering and MITM attacks

The only manual step required for future releases is creating a git tag - everything else is automated.
