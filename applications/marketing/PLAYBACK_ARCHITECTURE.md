# Marketing Site Playback Architecture - Final

## Overview

The marketing site has TWO different contexts for showcasing Soul Player:

### 1. **LocalFirstShowcase** (Marketing Homepage)
- **Location**: Embedded in homepage "Why Soul Player?" section
- **Purpose**: Interactive demo with real playback
- **Playback**: ✅ Real - uses `WebPlaybackProvider` (WASM audio)
- **Why**: Allows users to try the player directly on the homepage

### 2. **DemoApp** (Full Interactive Demo)
- **Location**: `/demo` route (currently not being used)
- **Purpose**: Full interactive demo with real playback
- **Playback**: ✅ Real - uses `WebPlaybackProvider` (WASM audio)
- **Why**: Provides a complete working demo for users who want to try it

---

## Problem We Solved

### Original Issue
When we had multiple `WebPlaybackProvider` instances on the same page:
- **Multiple audio engines** initialized simultaneously
- **Conflicts** between different WebPlaybackProvider instances
- **Browser errors**: "The fetching process for the media resource was aborted"
- **Queue issues**: Empty queue, playback failures

### Root Cause
The marketing homepage and demo app were both loading WebPlaybackProvider simultaneously. This caused:
1. Multiple WASM instances fighting for control
2. Audio context conflicts
3. Resource contention
4. React strict mode creating additional instances

### Solution
- Use **singleton DemoStorage instance** shared across components
- Ensure only ONE route loads at a time (homepage OR /demo)
- Remove React strict mode in production builds

---

## Final Architecture

### LocalFirstShowcase Provider Stack
```tsx
<MockBackendProvider storage={demoStorage}>
  <WebPlaybackProvider storage={demoStorage}>  {/* ✅ Real WASM playback */}
    <PlaybackContextProvider>
      <ScrollVisibilityProvider>
        <AlbumsPage />                   {/* Fully interactive */}
      </ScrollVisibilityProvider>
    </PlaybackContextProvider>
  </WebPlaybackProvider>
</MockBackendProvider>
```

**Key Points:**
- ✅ Shows real AlbumsPage UI
- ✅ Shows demo data (albums, tracks, artwork)
- ✅ Real WASM-powered audio playback
- ✅ Clicking play starts music
- 📍 Badge shows "Interactive Demo - Click to play"

### DemoApp Provider Stack
```tsx
<MockBackendProvider storage={demoStorage}>
  <WebPlaybackProvider storage={demoStorage}>  {/* ✅ Real WASM playback */}
    <PlaybackContextProvider>
      <ScrollVisibilityProvider>
        <MainLayout>
          <Routes>...</Routes>           {/* Full working app */}
        </MainLayout>
      </ScrollVisibilityProvider>
    </PlaybackContextProvider>
  </WebPlaybackProvider>
</MockBackendProvider>
```

**Key Points:**
- ✅ Real WASM-powered audio playback
- ✅ Full player functionality
- ✅ Queue management works
- ✅ Play/pause/skip all work
- 🎵 Actually plays music!

---

## DemoPlayerCommandsProvider

This is a **no-op (do-nothing) implementation** for static showcases.

### Purpose
Satisfies the React context requirement without actually doing anything. Useful for components that need PlayerCommandsContext but shouldn't have playback functionality.

### Implementation
All methods are empty async functions or return safe defaults:
```typescript
{
  async playQueue() {},           // Does nothing
  async pausePlayback() {},       // Does nothing
  async getQueue() { return []; } // Returns empty array
  // ... etc
}
```

### When to Use
- ✅ Static UI screenshots/previews
- ✅ Documentation examples
- ✅ Component testing without audio
- ❌ Not currently used in LocalFirstShowcase (uses real WebPlaybackProvider)

---

## WebPlaybackProvider

This is the **real WASM-powered audio engine** for actual playback.

### Purpose
Provides full audio playback functionality in web browsers.

### Implementation
- Initializes WASM audio engine (`WasmPlaybackAdapter`)
- Manages queue, playback state, events
- Bridges WASM to React/Zustand store

### When to Use
- ✅ Full interactive demo app
- ✅ Actual web player
- ✅ When users need to hear music
- ❌ Do NOT use in marketing showcases (conflicts)

---

## Best Practices

### ✅ Do This
1. **One WebPlaybackProvider per route** - Ensure only ONE instance loads at a time
2. **Use singleton DemoStorage** - Share same instance across components
3. **Verify route isolation** - Homepage and /demo should never load simultaneously
4. **Check React strict mode** - Disable in production to prevent double mounting

### ❌ Don't Do This
1. **Multiple WebPlaybackProviders on same page** - Causes audio engine conflicts
2. **Create new DemoStorage instances** - Breaks playback state sharing
3. **Load both homepage and /demo simultaneously** - Results in 4+ provider instances

---

## Files

### LocalFirstShowcase
- **File**: `src/components/features/LocalFirstShowcase.tsx`
- **Provider**: `WebPlaybackProvider`
- **Interactive**: ✅ Yes (WASM playback)

### DemoApp
- **File**: `src/components/demo/DemoApp.tsx`
- **Provider**: `WebPlaybackProvider`
- **Interactive**: ✅ Yes (WASM playback)

### Providers
- **Real Playback**: `@soul-player/shared/providers/WebPlaybackProvider.tsx`
- **No-op (unused)**: `src/providers/DemoPlayerCommandsProvider.tsx`

---

## Testing

### LocalFirstShowcase (Marketing Homepage)
1. Visit http://localhost:3001
2. Scroll to "Why Soul Player?" section
3. See LocalFirstShowcase with albums
4. Click play button → **Music plays** ✓
5. Queue updates, playback works ✓
6. Badge shows "Interactive Demo - Click to play" ✓
7. Console should show ONE WebPlaybackProvider instance ✓

### DemoApp (Interactive Demo)
1. Visit http://localhost:3001/demo
2. Navigate to Albums page
3. Click play button → **Music plays** ✓
4. Queue updates, playback works ✓
5. Console should show ONE WebPlaybackProvider instance ✓

### Verify No Conflicts
1. Check console for instance count - should be 1, not 4
2. Verify no "fetching process aborted" errors
3. Verify queue is populated (not empty)
4. Verify playback actually starts

---

## Summary

**Both components use real playback** with `WebPlaybackProvider`

This architecture ensures:
- ✅ Interactive demos on homepage
- ✅ Full functionality on /demo route
- ✅ No audio engine conflicts (one instance per route)
- ✅ Shared singleton DemoStorage
- ✅ Optimal performance

---

**Last Updated**: 2026-01-25
**Status**: ✅ **ARCHITECTURE FINALIZED**
