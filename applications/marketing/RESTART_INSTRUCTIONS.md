# COMPLETE RESTART - Fix Caching Issues

## The Problem
Your browser and dev server are serving OLD CACHED CODE. The file has the correct code but you're seeing the old version.

## The Solution
Complete cache clear and restart.

---

## Step-by-Step Instructions

### 1. Stop Dev Server
In your terminal where the dev server is running:
- Press **Ctrl+C** to stop it
- Wait for it to fully stop

### 2. Clear Caches

**On Linux/Mac/WSL:**
```bash
cd applications/marketing
rm -rf .next
rm -rf node_modules/.cache
```

**On Windows PowerShell:**
```powershell
cd applications/marketing
Remove-Item -Recurse -Force .next
Remove-Item -Recurse -Force node_modules/.cache
```

### 3. Restart Dev Server
```bash
yarn dev
```

### 4. Clear Browser Cache
**CRITICAL - You MUST do this:**

- **Chrome/Edge:** Press **Ctrl+Shift+Delete** (Windows) or **Cmd+Shift+Delete** (Mac)
  - Select "Cached images and files"
  - Click "Clear data"
  - OR just do a hard refresh: **Ctrl+Shift+R** (Windows) / **Cmd+Shift+R** (Mac)

- **Firefox:** Press **Ctrl+Shift+Delete** (Windows) or **Cmd+Shift+Delete** (Mac)
  - Select "Cache"
  - Click "Clear Now"
  - OR hard refresh: **Ctrl+F5** (Windows) / **Cmd+Shift+R** (Mac)

### 5. Navigate Fresh
- Go to http://localhost:3001
- Scroll to "Why Soul Player?" section
- The LocalFirstShowcase should now work

---

## Quick Script (Linux/Mac/WSL Only)

I've created a script that does steps 1-3 automatically:

```bash
cd applications/marketing
./restart-dev.sh
```

---

## Verify the Fix Worked

Open browser console (F12) and check:

### Should SEE:
```
[WebPlaybackProvider] WASM manager initialized
[PlaybackContextProvider] Loaded X contexts
```

### Should NOT see:
```
usePlaybackContext must be used within PlaybackContextProvider
```

---

## If Still Not Working

1. **Check browser console (F12)** - Tell me exact error messages
2. **Check terminal** - Any errors when dev server starts?
3. **Try different browser** - Maybe Chrome cache is stuck
4. **Check file saved:**
   ```bash
   grep "WebPlaybackProvider" applications/marketing/src/components/features/LocalFirstShowcase.tsx
   ```
   Should show: Line 15 and Line 181

---

## Why This Happened

Next.js dev server caches compiled pages in `.next/` directory. When we changed providers, the cache wasn't invalidated. The browser also caches JavaScript bundles.

**Solution:** Clear both caches (server + browser) and restart fresh.

---

**Last Updated:** 2026-01-24
**Status:** File is correct ✅ - Just need cache clear
