# Marketing Demo: Setup and Test

Quick guide to get the marketing demo running with the playback fixes.

## Prerequisites

1. **Install wasm-pack:**
   ```bash
   cargo install wasm-pack
   ```

2. **Verify installation:**
   ```bash
   wasm-pack --version
   # Should print: wasm-pack 0.x.x
   ```

## First-Time Setup

```bash
# From project root
cd applications/marketing

# Install dependencies (if not already done)
yarn install

# Start development server
# (WASM builds automatically on first run)
yarn dev
```

**Expected output:**
```
[WASM] Building soul-playback WASM module...
[WASM] Source: /path/to/libraries/soul-playback
[WASM] Output: /path/to/applications/marketing/src/wasm/soul-playback
[info] Checking for the Wasm target...
[info] Compiling to Wasm...
... (Rust compilation output)
[WASM] WASM build complete!
[WASM] Output: /path/to/applications/marketing/src/wasm/soul-playback
  ▲ Next.js 16.1.1
  - Local:        http://localhost:3001
```

**First build takes ~10-30 seconds.** Subsequent `yarn dev` runs are faster (~5-15 seconds) as Rust caches incremental builds.

## Testing Playback

1. **Open browser:** http://localhost:3001

2. **Navigate to Library**

3. **Click any track** (Track #3 "BOY IN THE MIRROR" recommended - has valid audio file)

4. **Open browser console** and verify logs:
   ```
   [LibraryPage] buildQueue called: { totalTracks: 33, clickedTrack: "BOY IN THE MIRROR", ... }
   [DemoPlayerCommandsProvider] Loading playlist to WASM, starting track: BOY IN THE MIRROR
   [WasmPlaybackAdapter] loadPlaylist called with 33 tracks
   [WasmPlaybackAdapter] play() called, current state: stopped
   [WasmPlaybackAdapter] *** onTrackChange callback invoked *** with track  ← KEY EVENT!
   [WasmPlaybackAdapter] Track change: { title: "BOY IN THE MIRROR", ... }
   [WebAudioPlayer] Loading track: /demo-audio/...
   [WebAudioPlayer] Playback started
   ```

5. **Verify:**
   - ✅ Audio plays
   - ✅ Track info displays at bottom
   - ✅ Queue sidebar shows all tracks with cover art
   - ✅ Next/Previous buttons work
   - ✅ Progress bar moves

## What Was Fixed

### 1. WASM Event Bug (Rust)
**File:** `libraries/soul-playback/src/wasm/manager.rs:49`

The `play()` method wasn't emitting track change events. Fixed by adding:
```rust
self.emit_track_change(); // Notify JavaScript about current track
```

### 2. Automatic Build Integration (JavaScript)
**Files:**
- `scripts/build-wasm.mjs` - Cross-platform build script
- `applications/marketing/package.json` - predev/prebuild hooks

WASM now builds automatically before `yarn dev` and `yarn build`. No manual commands needed!

### 3. Cover Art Retrieval (JavaScript)
**File:** `applications/marketing/src/providers/DemoPlayerCommandsProvider.tsx:100-114`

Queue items now look up cover art from demo storage (WASM doesn't store it).

### 4. Field Name Fixes (JavaScript)
**File:** `applications/marketing/src/providers/DemoPlayerCommandsProvider.tsx:105-113`

Fixed field names to match WASM expectations:
- `duration` → `duration_secs`
- `trackNumber` → `track_number`

## Development Workflow

### Standard (Recommended)

```bash
cd applications/marketing
yarn dev
```

WASM rebuilds automatically each time you run `yarn dev`.

### With WASM Auto-Rebuild (Advanced)

If actively developing Rust code:

```bash
# Terminal 1: Watch Rust files
cd applications/marketing
yarn dev:wasm-watch

# Terminal 2: Run dev server
yarn dev
```

Now editing Rust files auto-triggers WASM rebuild. Refresh browser to see changes.

## Troubleshooting

### "wasm-pack is not installed"

```bash
cargo install wasm-pack
```

### Audio doesn't play

Check console for errors. Common issues:
- **File not found:** Tracks 1-2 reference non-existent files. Use track #3+
- **CORS error:** Ensure files are in `public/demo-audio/`
- **Browser autoplay policy:** Click play button to start

### Changes don't appear

1. Stop dev server (`Ctrl+C`)
2. Clear Next.js cache: `rm -rf .next`
3. Restart: `yarn dev`

### WASM build fails

- Check Rust installation: `rustc --version`
- Check wasm-pack: `wasm-pack --version`
- Check Cargo.toml for dependency issues

## Production Build

```bash
cd applications/marketing
yarn build
```

WASM builds automatically, then Next.js builds the static site.

## Next Steps

- See [WASM_BUILD_INTEGRATION.md](./WASM_BUILD_INTEGRATION.md) for detailed build system docs
- See [WASM_PLAYBACK_FIX.md](./WASM_PLAYBACK_FIX.md) for technical details on the bug fix
- See [PLAYBACK_DEBUG_FIXES.md](./PLAYBACK_DEBUG_FIXES.md) for all related fixes

## Summary

✅ **Automatic WASM builds** - No manual commands
✅ **Cross-platform** - Works on Windows, macOS, Linux
✅ **Playback working** - Track change events now fire correctly
✅ **Cover art working** - Queue items display album art
✅ **Seamless DevX** - Just run `yarn dev` and everything works

Happy coding! 🎵
