# WASM Field Naming Guide

## The Problem

WASM (Rust) uses **snake_case** while TypeScript uses **camelCase**. Serde (Rust serialization) expects exact field names.

## Critical Field Names

When sending data to WASM, use **snake_case**:

| TypeScript (Wrong) | WASM Expected (Correct) | Type |
|-------------------|------------------------|------|
| `duration` | `duration_secs` | `number` |
| `trackNumber` | `track_number` | `number?` |
| `durationSecs` | `duration_secs` | `number` |

## Where This Matters

### 1. Demo Data JSON (`/public/demo-data.json`)
```json
{
  "id": "1",
  "title": "Dark",
  "duration": 42,        ← Just "duration" in JSON (stays as-is)
  "trackNumber": 1       ← camelCase in JSON (stays as-is)
}
```

### 2. TypeScript → WASM Conversion
```typescript
// ❌ WRONG - Will fail with "missing field `duration_secs`"
const track = {
  id: "1",
  path: "/demo-audio/dark.flac",
  title: "Dark",
  duration: 42,           // Wrong field name
  trackNumber: 1          // Wrong field name
}

// ✅ CORRECT - WASM can deserialize this
const track = {
  id: "1",
  path: "/demo-audio/dark.flac",
  title: "Dark",
  duration_secs: 42,      // Correct field name with underscore
  track_number: 1         // Correct field name with underscore
}
```

### 3. WASM → TypeScript Conversion

When WASM returns data, it uses **camelCase** in JavaScript getters:

```typescript
// WASM returns (via wasm_bindgen getters):
wasmTrack.durationSecs    // ✅ camelCase getter
wasmTrack.trackNumber     // ✅ camelCase getter

// But we convert to TypeScript using snake_case:
const track: QueueTrack = {
  duration_secs: wasmTrack.durationSecs,  // Map to snake_case
  track_number: wasmTrack.trackNumber     // Map to snake_case
}
```

## Why This Split?

**Rust Struct** (in WASM):
```rust
#[derive(Serialize, Deserialize)]
pub struct WasmQueueTrack {
    duration_secs: f64,    // Serde expects this exact name
    track_number: Option<u32>,
}

// But wasm_bindgen generates camelCase getters:
#[wasm_bindgen(getter, js_name = durationSecs)]
pub fn duration_secs(&self) -> f64 { ... }
```

So:
- **Sending to WASM (JSON → Rust)**: Use `duration_secs` (serde deserialization)
- **Getting from WASM (Rust → JS)**: Use `durationSecs` (wasm_bindgen getter)
- **TypeScript internal**: Use `duration_secs` (matches Rust struct)

## Files Updated

### 1. Type Definition
**File**: `applications/marketing/src/lib/demo/types.ts`
```typescript
export interface QueueTrack {
  duration_secs: number  // ✅ Now matches WASM
  track_number?: number  // ✅ Now matches WASM
}
```

### 2. Storage Conversion
**File**: `applications/marketing/src/lib/demo/storage.ts`
```typescript
toQueueTrack(track: DemoTrack): QueueTrack {
  return {
    duration_secs: track.duration,  // ✅ Convert to snake_case
    track_number: track.trackNumber // ✅ Convert to snake_case
  }
}
```

### 3. Provider Conversion
**File**: `applications/marketing/src/providers/DemoPlayerCommandsProvider.tsx`
```typescript
const demoQueue = queue.map(track => ({
  duration_secs: track.durationSeconds || 0,  // ✅ Use snake_case
  track_number: track.trackNumber || undefined // ✅ Use snake_case
}))
```

### 4. WASM Adapter
**File**: `applications/marketing/src/lib/demo/wasm-playback-adapter.ts`
```typescript
// Creating WASM track
new WasmQueueTrack(id, path, title, artist, track.duration_secs) // ✅

// Mapping from WASM
{
  duration_secs: wasmTrack.durationSecs,  // ✅ camelCase getter → snake_case field
  track_number: wasmTrack.trackNumber     // ✅ camelCase getter → snake_case field
}
```

### 5. Bridge
**File**: `applications/marketing/src/lib/demo/bridge.ts`
```typescript
const sharedTrack = {
  duration: Math.floor(track.duration_secs)  // ✅ Read snake_case field
}
```

## Testing

After these changes, clicking a track should:

1. ✅ Build queue with `duration_secs` fields
2. ✅ Serialize to JSON successfully
3. ✅ Send to WASM via `loadPlaylist()`
4. ✅ WASM deserializes without "missing field" errors
5. ✅ Audio starts playing

## Error Messages

**Before Fix**:
```
Failed to parse tracks: Error: missing field `duration_secs`
```

**After Fix**:
```
[DemoPlayerCommandsProvider] Loading playlist to WASM, starting track: Dark
[WasmPlaybackAdapter] Track change: Dark
[WebAudioPlayer] Loading track: /demo-audio/dark.flac
[WebAudioPlayer] Playback started
```

## Critical: WASM Objects vs Plain JavaScript Objects

### The Serialization Problem

WASM methods that accept **arrays** use `serde_wasm_bindgen::from_value()` which expects **plain JavaScript objects**, not WASM objects!

**Methods that take arrays**:
- `loadPlaylist(tracks: JsValue)` ← Uses serde deserialization
- `appendToQueue(tracks: JsValue)` ← Uses serde deserialization

**Methods that take single objects**:
- `addToQueueNext(track: WasmQueueTrack)` ← Takes WASM object directly
- `addToQueueEnd(track: WasmQueueTrack)` ← Takes WASM object directly

### Example: Wrong vs Right

```typescript
// ❌ WRONG - Creates WASM objects, serde can't deserialize them
const wasmTracks = tracks.map(t => new WasmQueueTrack(
  t.id, t.path, t.title, t.artist, t.duration_secs
))
this.wasmManager!.loadPlaylist(wasmTracks)  // ERROR: missing field `duration_secs`

// ✅ CORRECT - Plain JavaScript objects with exact field names
const plainTracks = tracks.map(t => ({
  id: t.id,
  path: t.path,
  title: t.title,
  artist: t.artist,
  album: t.album || null,
  duration_secs: t.duration_secs,  // Exact field name from Rust struct
  track_number: t.track_number !== undefined ? t.track_number : null,
}))
this.wasmManager!.loadPlaylist(plainTracks)  // ✅ Works!
```

### Why This Happens

1. **WASM Objects** (`new WasmQueueTrack(...)`):
   - Created by `wasm_bindgen`
   - Have private fields with public getters/setters
   - Serialize as `{ ... }` but serde can't access fields through getters

2. **Plain JS Objects** (`{ id: "1", duration_secs: 42 }`):
   - Regular JavaScript objects
   - Fields are directly accessible
   - Serde can deserialize them correctly

### Fixed Files

**`wasm-playback-adapter.ts`**:
```typescript
// Single track methods - use WASM objects (OK)
addToQueueNext(track: QueueTrack) {
  const wasmTrack = this.createWasmTrack(track)  // ✅ WASM object
  this.wasmManager!.addToQueueNext(wasmTrack)
}

// Array methods - use plain JS objects (REQUIRED)
loadPlaylist(tracks: QueueTrack[]) {
  const plainTracks = tracks.map(t => ({  // ✅ Plain objects
    id: t.id,
    duration_secs: t.duration_secs,  // Exact field names
    // ...
  }))
  this.wasmManager!.loadPlaylist(plainTracks)
}
```

---

**Key Takeaway**: When working with WASM:
1. **Always use snake_case field names** (`duration_secs`, `track_number`)
2. **For array methods**: Pass **plain JavaScript objects**, not WASM objects
3. **For single object methods**: You can use WASM objects (`new WasmQueueTrack(...)`)
