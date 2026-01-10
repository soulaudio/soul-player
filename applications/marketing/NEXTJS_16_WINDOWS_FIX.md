# Next.js 16.1 on Windows - SWC Binary Fix

## Why Upgrade to Next.js 16.1.1?

**CRITICAL SECURITY VULNERABILITIES** patched in 16.1.1:

### CVE-2025-55182 & CVE-2025-66478 (CVSS 10.0 - Critical)
- **Unauthenticated Remote Code Execution (RCE)**
- Affects React Server Components (RSC) "Flight" protocol
- Actively exploited by RondoDox botnet (90,000+ exposed systems)
- Allows attackers to execute arbitrary code on your server

### CVE-2025-55184 (CVSS 7.5 - High)
- Denial of Service attack vector
- Can crash your application server

### CVE-2025-55183 (CVSS 5.3 - Medium)
- Source code exposure vulnerability
- Attackers can read your server-side code

### CVE-2025-29927 (Middleware Bypass)
- Authorization bypass in Next.js middleware
- Can allow unauthorized access to protected routes

**DO NOT use Next.js 15.1.4 or earlier versions - they are vulnerable!**

## The Problem: SWC Binary Loading on Windows

When running Next.js 16 on Windows, you may see:

```
⨯ Failed to load SWC binary for win32/x64
Error: Failed to load SWC binary for win32/x64
```

This happens because:
1. Next.js uses SWC (Speedy Web Compiler) written in Rust
2. Windows requires a native binary (`@next/swc-win32-x64-msvc`)
3. The binary may fail to install or load due to:
   - Missing Visual C++ redistributables
   - Corrupted node_modules
   - WSL/Windows cross-environment issues
   - Yarn workspace monorepo path resolution

## Solution: Clean Reinstall + Verification

### Step 1: Prerequisites (Run in PowerShell as Administrator)

```powershell
# Check Node.js version (must be 64-bit, v20.9+)
node --version
node -p "process.arch"  # Should output: x64

# If not 64-bit or < v20.9, download from: https://nodejs.org/
```

### Step 2: Install Visual C++ Redistributables

Download and install **Microsoft Visual C++ Redistributable** (if not already installed):

**Download:** https://aka.ms/vs/17/release/vc_redist.x64.exe

Or install via `winget`:
```powershell
winget install Microsoft.VCRedist.2015+.x64
```

### Step 3: Clean Reinstall (Run in PowerShell)

```powershell
# Navigate to project root
cd D:\dev\soulaudio\soul-player

# Delete node_modules and lock files
Remove-Item -Recurse -Force node_modules -ErrorAction SilentlyContinue
Remove-Item -Force yarn.lock -ErrorAction SilentlyContinue

# Clean Yarn cache (optional but recommended)
yarn cache clean

# Reinstall dependencies
yarn install

# Verify SWC binary was installed
Test-Path "node_modules\@next\swc-win32-x64-msvc\next-swc.win32-x64-msvc.node"
# Should output: True
```

### Step 4: Verify Installation

```powershell
# Check if SWC binary exists
ls node_modules\@next\swc-win32-x64-msvc\

# Should see: next-swc.win32-x64-msvc.node (size ~120MB)
```

### Step 5: Test Marketing App

```powershell
cd D:\dev\soulaudio\soul-player

# Clear Next.js cache
Remove-Item -Recurse -Force applications\marketing\.next -ErrorAction SilentlyContinue

# Start dev server
yarn dev:marketing
```

If it works, you should see:
```
▲ Next.js 16.1.1 (Turbopack)
- Local: http://localhost:3001
✓ Starting...
✓ Ready in 3s
```

## Troubleshooting

### Issue 1: "Attempted to load @next/swc-win32-x64-msvc, but it was not installed"

**Cause**: Optional dependencies not installed

**Fix**:
```powershell
# Manually install the Windows SWC binary
yarn add @next/swc-win32-x64-msvc@16.1.1 -D

# Or with npm
npm install @next/swc-win32-x64-msvc@16.1.1 --save-dev
```

### Issue 2: "The system cannot find the path specified"

**Cause**: WSL/Windows path issues

**Fix**: Run all commands from **PowerShell** (not WSL). WSL and Windows share files but have different path resolution, causing lock issues.

### Issue 3: Binary Exists But Still Fails

**Cause**: Corrupted binary or permission issues

**Fix**:
```powershell
# Delete and reinstall just the SWC package
Remove-Item -Recurse -Force node_modules\@next\swc-win32-x64-msvc
yarn install --check-files
```

### Issue 4: "EACCES: permission denied"

**Cause**: File locks from running processes

**Fix**:
```powershell
# Stop all Node processes
Stop-Process -Name "node" -Force -ErrorAction SilentlyContinue

# Close VS Code, browsers, and dev servers
# Then retry installation
yarn install
```

### Issue 5: Antivirus Blocking

**Cause**: Windows Defender or antivirus quarantining the .node file

**Fix**:
1. Temporarily disable real-time protection
2. Add exception for `node_modules\@next\swc-win32-x64-msvc\`
3. Reinstall dependencies

## Alternative: Use Babel (Not Recommended)

If SWC continues to fail, you can fall back to Babel (much slower):

**File: `applications/marketing/.babelrc`**
```json
{
  "presets": ["next/babel"]
}
```

This disables SWC and uses Babel instead. **Not recommended** because:
- Much slower build times
- Misses SWC-specific optimizations
- Not the default path, may have bugs

## Yarn Workspace Considerations

This is a monorepo using Yarn workspaces. The SWC binary is hoisted to the root `node_modules/@next/`. This can cause issues if:

1. **Running from WSL**: Paths like `/mnt/d/...` confuse Next.js on Windows
   - **Solution**: Always run `yarn dev:marketing` from PowerShell

2. **Multiple Node versions**: WSL and Windows have different Node.js installations
   - **Solution**: Use the same Node version (20.9+, 64-bit) in both

3. **File locks**: Running commands from both WSL and Windows simultaneously
   - **Solution**: Pick one environment (recommend PowerShell on Windows)

## Verification Checklist

- [ ] Node.js version 20.9+ (64-bit)
- [ ] Visual C++ Redistributable installed
- [ ] `node_modules/@next/swc-win32-x64-msvc/` exists (~120MB file)
- [ ] No processes locking node_modules
- [ ] Running from PowerShell (not WSL)
- [ ] `.next` folder deleted before starting
- [ ] Yarn install completed without errors

## Turbopack Configuration (Next.js 16 Default)

Next.js 16 uses **Turbopack** by default instead of Webpack. The configuration has been migrated:

**Old (Webpack):**
```javascript
webpack: (config) => {
  config.resolve.fallback = { fs: false }
}
```

**New (Turbopack):**
```javascript
turbopack: {
  resolveAlias: {
    fs: { browser: './empty.ts' }
  }
}
```

This tells Turbopack to use an empty module when client code tries to import Node.js modules like `fs`.

## Expected Output (Success)

```powershell
PS D:\dev\soulaudio\soul-player> yarn dev:marketing
▲ Next.js 16.1.1 (Turbopack)
- Local:        http://localhost:3001
- Network:      http://10.2.0.2:3001

✓ Starting...
✓ Ready in 908ms
```

Open http://localhost:3001 in your browser. The demo should load and you should be able to:
- See 33 tracks from "pressures, father I sober" album
- Click any track to play
- Use player controls (play/pause/skip)
- View queue in right sidebar

## Security Notice

**You are now running Next.js 16.1.1 with critical security patches applied.**

Previous versions (15.x and earlier) have **CRITICAL remote code execution vulnerabilities** that are actively being exploited in the wild. Do not downgrade.

---

## Sources

- [Next.js 16.1 Release](https://nextjs.org/blog/next-16-1)
- [Next.js Security Update: December 11, 2025](https://nextjs.org/blog/security-update-2025-12-11)
- [Security Advisory: CVE-2025-66478](https://nextjs.org/blog/CVE-2025-66478)
- [Security Bulletin: CVE-2025-55184 and CVE-2025-55183](https://vercel.com/kb/bulletin/security-bulletin-cve-2025-55184-and-cve-2025-55183)
- [SWC Failed to Load Fix Guide](https://nextjs.org/docs/messages/failed-loading-swc)
- [How to Fix SWC Binary Loading](https://www.geeksforgeeks.org/how-to-fix-swc-failed-to-load-in-next-js/)
- [Medium: Solved - Failed to load SWC binary](https://medium.com/@stanykhay29/solved-failed-to-load-swc-binary-for-win32-64-next-js-0f492c19ea56)

---

**Last Updated**: 2026-01-09
**Next.js Version**: 16.1.1
**Security Status**: ✅ Patched (CVE-2025-55182, CVE-2025-66478, CVE-2025-55184, CVE-2025-55183, CVE-2025-29927)
