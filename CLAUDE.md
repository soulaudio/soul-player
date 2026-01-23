# CLAUDE.md

Instructions for Claude Code when working with Soul Player.

---

## Project Overview

**Soul Player**: Cross-platform music player (Desktop/Server/Mobile)
- Stack: Cargo + Yarn workspaces + Tauri
- Storage: SQLite multi-user schema
- Audio: Symphonia decoder + platform-specific output
- Languages: Rust (backend/libs) + TypeScript/React (frontend)
- Structure: `applications/` (platform apps), `libraries/` (shared Rust libs)

---

## Critical Rules (MUST Follow)

### 1. Multi-User Always
ALL database queries MUST include `user_id`. Desktop uses `user_id = 1`, server uses authenticated ID.

```rust
// ✅ CORRECT
pub async fn get_playlists(pool: &SqlitePool, user_id: i64) -> Result<Vec<Playlist>> {
    sqlx::query_as!(Playlist, "SELECT * FROM playlists WHERE owner_id = ?", user_id)
        .fetch_all(pool).await.map_err(Into::into)
}
```

### 2. Database: Compile-Time Queries Only
Use `query!`/`query_as!` macros ONLY. Never `query().bind()`. Setup: [docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md)

### 3. Platform-Agnostic Core
Libraries (`libraries/*`) MUST NOT depend on platform crates. Use traits. Platform code in `applications/` only.

### 4. Audio Safety: No Allocations
Audio callbacks MUST NOT allocate. Pre-allocate buffers in constructors.

### 5. Test Quality: No Shallow Tests
- Test: business logic, edge cases, error paths
- Skip: getters, setters, trivial constructors
- Use testcontainers with real SQLite (never in-memory)
- Target: 50-60% meaningful coverage

### 6. Error Handling
- Libraries: `thiserror` + `Result`, no `.unwrap()` in public APIs
- Applications: `.expect()` with clear messages is fine

### 7. Always Localize UI Strings
ALL user-facing strings MUST use localization. NEVER hardcode text.

```typescript
// ✅ CORRECT
<button>{t('playback.play')}</button>
<div>{t('errors.file_not_found', { filename })}</div>
```

### 8. Structured Logging Only
Use `tracing` crate ONLY. NEVER `println!`, `eprintln!`, `dbg!()`. Exception: `init_logging()` may use `eprintln!` before initialization.

```rust
// ✅ CORRECT
tracing::info!("[SCAN] Processing: {}", file_path.display());
tracing::error!("[SCAN] TIMEOUT on file: {}", file_path.display());
```

**Log locations**: `%APPDATA%\Soul Player\logs\` (Win), `~/Library/Application Support/soul-player/logs/` (Mac), `~/.config/soul-player/logs/` (Linux)

### 9. Frontend Styling: CSS-First (Tailwind v4)

**Core Principles**:
1. CSS variables for theming (in `:root`)
2. Data attributes for state (`data-state`, `aria-current`) - NOT class-based
3. Separate hover and selected states
4. Opacity-based hover effects - NOT color changes
5. Avoid custom Tailwind plugins

**Standard Patterns**:

```tsx
// Navigation Item (Mutually Exclusive)
<button
  data-state={isActive ? 'active' : 'inactive'}
  aria-current={isActive ? 'page' : undefined}
  className={cn(
    'transition-all',
    isActive
      ? 'text-primary bg-accent/20'
      : 'text-muted-foreground hover:opacity-80 hover:bg-foreground/10'
  )}
/>

// Toggle Button (Active + Hover)
<button
  data-state={isEnabled ? 'on' : 'off'}
  aria-pressed={isEnabled}
  className={cn(
    'transition-opacity hover:opacity-80',
    isEnabled ? 'text-primary' : 'text-muted-foreground'
  )}
/>
```

**Interaction Values**:
- Hover (text): opacity 0.8
- Hover (background): `foreground/10`
- Selected: opacity 1.0, `accent/20` background
- Disabled: opacity 0.5, no hover

**Related Files**: `applications/shared/src/styles/globals.css`, `applications/shared/src/theme/ThemeManager.ts`

---

## Essential Commands

### First-Time Setup
```bash
corepack enable && yarn
./scripts/setup-sqlx.sh      # Unix/Linux/macOS
# Windows: see docs/SQLX_SETUP.md
```

### Development
```bash
cargo build --all && cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all
yarn dev:desktop             # Desktop app
cargo run -p soul-server     # Server
```

### Database Migrations
```bash
cd libraries/soul-storage
sqlx migrate add your_migration_name
sqlx migrate run --source libraries/soul-storage/migrations
cargo sqlx prepare -- --lib
git add .sqlx/
```

### WASM Development
Auto-builds via npm hooks. Requires `cargo install wasm-pack`.
```bash
cd applications/marketing
yarn dev              # Auto-builds WASM
yarn build:wasm       # Manual build
```

### Version Management (CRITICAL)
ALWAYS use `./scripts/bump-version.sh` - NEVER manually edit versions.

```bash
./scripts/bump-version.sh 0.1.5              # Standard release
./scripts/bump-version.sh 0.1.5 --dry-run    # Preview changes
./scripts/bump-version.sh 1.0.0-beta.1       # Pre-release
```

**Updates**: Workspace `Cargo.toml`, all lib/app `Cargo.toml`, all `package.json`, `tauri.conf.json`, `.github/release-config.json`

**Windows Installer Caching**: If UI shows old version after update, clean AppData cache:
```powershell
Get-Process -Name "soul-player" -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item "$env:APPDATA\Soul Player" -Recurse -Force -ErrorAction SilentlyContinue
# Reinstall as Administrator
```

---

## Pre-Commit Requirements (MANDATORY)

Husky runs these automatically. ALL must pass before commit.

```bash
# Rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --all

# TypeScript
yarn workspace soul-player-desktop run tsc --noEmit
yarn workspace @soul-player/shared run tsc --noEmit
yarn workspace soul-player-desktop run lint
yarn workspace @soul-player/shared run lint
```

**Bypass** (WIP only): `git commit --no-verify`

---

## Quick Reference

### Key Docs
- **Architecture**: [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
- **Testing**: [docs/TESTING.md](./docs/TESTING.md)
- **SQLx Setup**: [docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md)

### Database Schema
See `libraries/soul-storage/migrations/*.sql`. Core: `users`, `tracks`, `albums`, `artists`, `playlists`, `playlist_tracks`

### Frontend Stack
React 18 + TypeScript + Zustand + TailwindCSS + Lucide React

### macOS Code Signing
Ad-hoc signing (`signingIdentity: "-"`). First launch requires right-click → Open. See `.github/workflows/release.yml` for notarization upgrade.

### Keyboard Shortcuts
- **App-level only** (NOT OS-level global)
- Input-aware (disabled in text fields)
- MUST use `PlayerCommandsContext` - NEVER `invoke()` directly
- Files: `libraries/soul-storage/src/shortcuts/mod.rs`, `applications/desktop/src/hooks/useKeyboardShortcuts.ts`

**Defaults**: Space (play/pause), Arrows (next/prev/volume), M (mute)

---

## Architecture Patterns

### Backend Abstraction (BackendContext)
Ensures desktop/marketing parity - same UI components on both platforms.

**Files**:
- `applications/shared/src/contexts/BackendContext.tsx` - Interface
- `applications/desktop/src/providers/TauriBackendProvider.tsx` - Desktop impl
- `applications/marketing/src/providers/DemoBackendProvider.tsx` - Demo impl

**Rules**:
1. NEVER `invoke()` directly in shared pages - use `useBackend()` hook
2. Shared pages must work on both platforms
3. Use `DesktopOnly`/`WebOnly`/`FeatureGate` for conditional rendering
4. New operations → add to `BackendInterface` first

```typescript
// ✅ CORRECT
const backend = useBackend()
const tracks = await backend.getAllTracks()
```

### Playback Architecture (CRITICAL)

**TWO SEPARATE CONTEXTS - NEVER MIX**:

1. **BackendContext** - Data fetching ONLY
   - Use for: `getAllTracks`, `getAlbumById`, `recordContext`
   - NEVER add: playback control methods

2. **PlayerCommandsContext** - Playback control ONLY
   - Use for: `playQueue`, `pausePlayback`, `skipNext`, `setVolume`
   - NEVER add: data fetching methods

**Command Flow**:
```
UI → useBackend() → fetch data → transform to queue
  → usePlayerCommands() → playQueue()
  → Tauri Events → Zustand Store → UI Update
```

**Playback Pattern**:
```typescript
// Step 1: Fetch data
const backend = useBackend()
const tracks = await backend.getAlbumTracks(albumId)

// Step 2: Transform to queue
const queue = tracks.map(t => ({ trackId: String(t.id), ... }))

// Step 3: Record context (optional)
await backend.recordContext({ contextType: 'album', contextId: String(albumId) })

// Step 4: Control playback
const commands = usePlayerCommands()
await commands.playQueue(queue, 0)
```

**Files**:
- `applications/shared/src/contexts/BackendContext.tsx` - Data interface
- `applications/shared/src/contexts/PlayerCommandsContext.tsx` - Control interface
- `applications/desktop/src-tauri/src/playback.rs` - Rust wrapper
- `libraries/soul-playback/src/manager.rs` - Core logic

### Web Playback Patterns

**Three-layer architecture**: Application → Shared (`WebPlaybackProvider`) → Library (`@soul-player/playback-web`)

**Critical Rules**:
1. Use `WebPlaybackProvider` from `@soul-player/shared/providers` - NEVER create app-specific providers
2. Events emitted AUTOMATICALLY by `WasmPlaybackAdapter` - NEVER manually emit
3. Implement `PlaybackDataStorage` interface for track lookup
4. Pass plain objects to WASM with `snake_case` fields (`duration_secs`, `track_number`)
5. Event bridge is internal to `WebPlaybackProvider` - don't expose to app layer

```typescript
// ✅ CORRECT - Plain objects with snake_case
const wasmQueue = queue.map(track => ({
  id: track.trackId,
  path: track.filePath,
  title: track.title,
  artist: track.artist || 'Unknown Artist',  // Never undefined
  duration_secs: track.durationSeconds || 0,  // snake_case
  track_number: track.trackNumber || undefined
}))
```

**Files**:
- `applications/shared/src/providers/WebPlaybackProvider.tsx` - Reusable provider
- `applications/shared/src/types/storage.ts` - `PlaybackDataStorage` interface
- `libraries/soul-playback-web/src/wasm-adapter.ts` - WASM bridge
- `libraries/soul-playback-web/src/types.ts` - Type definitions

---

## When in Doubt

1. **Multi-user**: Always require `user_id` parameter
2. **Database**: Use compile-time `query!` macros
3. **Platform code**: Use traits, isolate in `applications/`
4. **Tests**: Skip getters/setters, test business logic only
5. **Allocations**: Never in audio `process()` methods
6. **UI Strings**: Always localize, never hardcode
7. **Shortcuts**: App-level only, use `PlayerCommandsContext`
8. **Shared pages**: Use `useBackend()`, never `invoke()`
9. **Playback**: Data → BackendContext, Control → PlayerCommandsContext. NEVER mix.
10. **Versions**: ALWAYS use `./scripts/bump-version.sh`
11. **Styling**: CSS variables + data attributes + opacity hover (NOT color changes)
12. **Logging**: `tracing` only, never `println!`

---

**Last Updated**: 2026-01-23
**Rust Edition**: 2021
**Platforms**: Windows, macOS, Linux
