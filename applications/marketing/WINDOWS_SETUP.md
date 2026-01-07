# Windows Setup Guide

If you're running the dev server on Windows and seeing SWC binary errors, here are the solutions:

## Problem

```
⚠ Failed to load SWC binary for win32/x64
Error: Failed to get registry from "yarn"
```

This happens because Next.js tries to download platform-specific native binaries.

## Solution 1: Clear Cache and Reinstall (Recommended)

```bash
# From workspace root
rm -rf node_modules .next .yarn/cache
yarn install
cd applications/marketing
yarn dev
```

## Solution 2: Use Node 20+ Directly on Windows

Instead of running through WSL, run the dev server directly in Windows PowerShell:

```powershell
# In Windows PowerShell (not WSL)
cd D:\dev\soulaudio\soul-player
yarn dev:marketing
```

This ensures the correct Windows SWC binary is downloaded.

## Solution 3: Install SWC Binary Manually (If needed)

If the above doesn't work, install the Windows SWC binary:

```bash
# From workspace root
yarn add -D -W @next/swc-win32-x64-msvc
```

Then try again:

```bash
yarn dev:marketing
```

## Solution 4: Use WSL 2 with Proper Node Setup

If you prefer using WSL:

1. Make sure you're on WSL 2 (not WSL 1)
2. Install Node.js inside WSL (not using Windows Node)
3. Clone the repo inside WSL filesystem (e.g., `~/projects/` not `/mnt/d/`)

```bash
# Check WSL version
wsl --list --verbose

# If using WSL 1, upgrade to WSL 2
wsl --set-version Ubuntu 2
```

## Verify Setup

After applying a solution, verify it works:

```bash
yarn dev:marketing
```

Should output:
```
✓ Ready in X seconds
○ Local:    http://localhost:3001
```

Visit http://localhost:3001 to see the site!

## Current Package Versions

✅ **Next.js**: 16.1.1 (latest, with security fixes)
✅ **Nextra**: 4.6.1 (latest)
✅ **React**: 18.3.1

## Alternative: Docker Development

If you continue having issues, you can use Docker:

```bash
cd applications/marketing
docker build -t soul-player-marketing .
docker run -p 3001:3000 soul-player-marketing
```

Visit http://localhost:3001

## Still Having Issues?

1. **Check Node version**: Must be v20+
   ```bash
   node --version  # Should be v20.x.x or v22.x.x
   ```

2. **Clear all caches**:
   ```bash
   yarn cache clean
   rm -rf node_modules
   rm -rf .next
   yarn install
   ```

3. **Try running in Windows native terminal** (PowerShell/CMD) instead of WSL

4. **Check firewall**: Make sure port 3001 isn't blocked

---

✅ Once working, you should see the landing page with:
- Hero section with download buttons
- Demo showcase
- Features comparison
- Coming soon section (Mobile + DAP)
