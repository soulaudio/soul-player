# CLAUDE.md

Instructions for Claude Code when working with Soul Player.

---

## Project Overview

**Soul Player**: Cross-platform music player (Desktop/Server/Mobile)
- **Stack**: Cargo workspace + Yarn workspace + Tauri (Moon tasks optional for local dev)
- **Storage**: SQLite with multi-user schema from day 1
- **Audio**: Symphonia decoder + platform-specific output
- **Languages**: Rust (backend/libs) + TypeScript/React (frontend)

**Directory Structure**:
```
applications/     # Platform-specific apps (desktop/server/mobile)
libraries/        # Shared Rust libraries (soul-core, soul-storage, etc.)
```

---

## Critical Rules (MUST Follow)

### 1. Multi-User Always
Every database query MUST include `user_id` context:
- Desktop: `user_id = 1` (default user)
- Server: authenticated user ID
- Never query playlists/settings without user context

```rust
// ✅ CORRECT
pub async fn get_playlists(pool: &SqlitePool, user_id: i64) -> Result<Vec<Playlist>> {
    sqlx::query_as!(
        Playlist,
        "SELECT id, owner_id, name, created_at, updated_at
         FROM playlists WHERE owner_id = ?",
        user_id
    )
    .fetch_all(pool).await.map_err(Into::into)
}

// ❌ WRONG: No user context
pub async fn get_all_playlists(pool: &SqlitePool) -> Result<Vec<Playlist>> { ... }
```

### 2. Database: Compile-Time Queries Only
ALL queries MUST use `query!` / `query_as!` macros (not `query().bind()`):
- Typos = compile error
- Schema changes = immediate feedback
- Type safety enforced

```rust
// ✅ CORRECT
sqlx::query_as!(Track, "SELECT id, title FROM tracks WHERE id = ?", id)

// ❌ WRONG
sqlx::query("SELECT * FROM tracks WHERE id = ?").bind(id)
```

**Setup**: See [docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md)

### 3. Platform-Agnostic Core
Libraries (`libraries/*`) MUST NOT depend on platform-specific crates:
- Use traits for abstraction
- Platform code in `applications/` only
- Dependency flow: core → libraries → platform crates → applications

### 4. Audio Safety: No Allocations
Audio callback paths MUST NOT allocate:
- No `Vec::new()`, `Box::new()`, `String::from()` in `process()` methods
- Pre-allocate buffers in constructors

```rust
// ✅ CORRECT
pub struct Compressor {
    envelope: Vec<f32>,  // Pre-allocated in new()
}
impl AudioEffect for Compressor {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        for (i, sample) in buffer.iter_mut().enumerate() {
            self.envelope[i] = (*sample).abs();  // No allocation
        }
    }
}

// ❌ WRONG
fn process(&mut self, buffer: &mut [f32]) {
    let envelope = buffer.iter().map(|s| s.abs()).collect::<Vec<f32>>();
}
```

### 5. Test Quality: No Shallow Tests
- ✅ DO test: business logic, edge cases, error paths
- ❌ DON'T test: getters, setters, trivial constructors
- Use testcontainers with real SQLite (never in-memory)
- Target: 50-60% meaningful coverage

### 6. Error Handling
- Libraries: `thiserror` + `Result`, no `.unwrap()` in public APIs
- Applications: `.expect()` with clear messages is fine

### 7. Always Localize UI Strings
ALL user-facing strings MUST use localization - NEVER hardcode text:
- Desktop: Use i18n framework (e.g., `react-i18next`, `fluent`)
- Mobile: Platform localization APIs
- Server/Web: Use i18n framework with server-side rendering support
- Applies to: buttons, labels, messages, tooltips, errors

```typescript
// ✅ CORRECT (React)
<button>{t('playback.play')}</button>
<div className="error">{t('errors.file_not_found', { filename })}</div>

// ❌ WRONG
<button>Play</button>
<div className="error">File not found: {filename}</div>
```

**Why**: Enables internationalization from day 1, easier to maintain, professional UX.

### 8. Structured Logging Only
ALL logging MUST use the `tracing` crate - NEVER use `println!`, `eprintln!`, or `dbg!()`:
- Desktop/Server: Use `tracing::info!()`, `tracing::warn!()`, `tracing::error!()`, `tracing::debug!()`
- Logs are captured by the tracing subscriber and written to both console and file (when `--logs` flag is enabled)
- ONLY exception: `init_logging()` function itself may use `eprintln!` before logging is initialized
- Use appropriate log levels: `debug!` for verbose details, `info!` for normal operation, `warn!` for recoverable issues, `error!` for failures

```rust
// ✅ CORRECT
tracing::info!("[SCAN] Processing: {}", file_path.display());
tracing::error!("[SCAN] TIMEOUT on file: {}", file_path.display());
tracing::debug!("[play_queue] Calling stop()...");

// ❌ WRONG
eprintln!("[SCAN] Processing: {}", file_path.display());
println!("Processing file: {}", file_path);
dbg!(file_path);
```

**Why**: Ensures all logs are captured in log files for debugging, provides consistent log formatting, enables filtering by log level, allows structured logging with key-value pairs.

**Desktop Logging**: Run with `yarn dev:desktop:logs` or `--logs` flag to enable file logging. Logs are saved to:
- **Windows**: `%APPDATA%\Soul Player\logs\soul-player.log.YYYY-MM-DD`
- **macOS**: `~/Library/Application Support/soul-player/logs/soul-player.log.YYYY-MM-DD`
- **Linux**: `~/.config/soul-player/logs/soul-player.log.YYYY-MM-DD`

See [LOGGING.md](./LOGGING.md) for detailed logging documentation.

---

## Essential Commands

### First-Time Setup
```bash
corepack enable              # Enable Yarn 4.x (run in root)
yarn                         # Install all dependencies (run in root)
```

**SQLx Setup (Windows PowerShell):**
```powershell
cargo install sqlx-cli --no-default-features --features sqlite
copy .env.example .env
mkdir libraries\soul-storage\.tmp
sqlx database create
sqlx migrate run --source libraries/soul-storage/migrations
```

**SQLx Setup (Unix/Linux/macOS):**
```bash
./scripts/setup-sqlx.sh      # Handles everything automatically
```

### Development
```bash
cargo build --all
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all

yarn dev:desktop             # Run desktop app
cargo run -p soul-server     # Run server
```

### Database Migrations

**Prerequisites**: Ensure `.env` exists with `DATABASE_URL` set (copy from `.env.example`).

**Creating a new migration:**
```bash
cd libraries/soul-storage
sqlx migrate add your_migration_name
# Edit the generated file in migrations/
```

**Applying migrations:**
```bash
sqlx migrate run --source libraries/soul-storage/migrations
```

**Updating SQLx offline data (required after schema changes):**
```bash
cd libraries/soul-storage
cargo sqlx prepare -- --lib
git add .sqlx/
```

**Standard SQLx Commands** (these are pre-approved in CI allowlist):
```bash
# Database commands
sqlx database create
sqlx database drop
sqlx migrate run --source libraries/soul-storage/migrations

# SQLx prepare (from libraries/soul-storage directory)
cargo sqlx prepare -- --lib

# Build/check with SQLX_OFFLINE
SQLX_OFFLINE=true cargo build --all
SQLX_OFFLINE=true cargo check -p soul-storage
```

### WASM Development (Marketing Demo)
WASM modules build **automatically** via npm lifecycle hooks:
```bash
cd applications/marketing
yarn dev              # Auto-builds WASM before starting
yarn build            # Auto-builds WASM before production build
yarn build:wasm       # Manual WASM build only
yarn dev:wasm-watch   # Optional: Watch Rust files and auto-rebuild
```

**Requirements**: `wasm-pack` must be installed
```bash
cargo install wasm-pack
```

**Note**: All WASM builds are cross-platform (Windows/macOS/Linux). See [applications/marketing/WASM_BUILD_INTEGRATION.md](./applications/marketing/WASM_BUILD_INTEGRATION.md) for details.

### Version Management (CRITICAL)

**IMPORTANT**: Version numbers MUST be synchronized across all configuration files. Use the automated script to prevent mismatches.

**Version Bump Procedure:**
```bash
# 1. Run the version bump script (handles all files automatically)
./scripts/bump-version.sh 0.1.3

# 2. Review the changes
git diff

# 3. Commit and push to main
git add -A
git commit -m "chore: bump version to v0.1.3"
git push origin main

# 4. CI will automatically:
#    - Detect the version bump in Cargo.toml
#    - Create and push the tag v0.1.3
#    - Trigger the release workflow
#    - Build installers for all platforms
#    - Publish the release to GitHub
```

**What the script updates:**
- ✅ Workspace `Cargo.toml` (line 31: `version = "X.Y.Z"`)
- ✅ All library `Cargo.toml` files in `libraries/*/Cargo.toml`
- ✅ All application `Cargo.toml` files in `applications/*/src-tauri/Cargo.toml`
- ✅ Root `package.json` and all `applications/*/package.json`
- ✅ **CRITICAL**: `applications/desktop/src-tauri/tauri.conf.json` (line 3: `"version": "X.Y.Z"`)

**Version Resolution in Tauri:**
- Tauri's `getVersion()` API reads from **`tauri.conf.json` first** (primary source)
- Falls back to `Cargo.toml` only if `tauri.conf.json` has no version
- The bump script ensures both are synchronized to prevent UI version mismatch

**Manual Version Updates (NOT RECOMMENDED):**
If you must update versions manually, you MUST update ALL files listed above. Missing even one file will cause version mismatches in the UI or build artifacts.

**Validation:**
After running the bump script:
```bash
# Verify all versions match
grep -r "\"version\":" package.json applications/*/package.json applications/desktop/src-tauri/tauri.conf.json
grep "^version = " Cargo.toml libraries/*/Cargo.toml
```

### Windows Installer Caching Issues (TROUBLESHOOTING)

**Known Issue:** Windows may show old version after installing new release due to caching or incomplete uninstall.

**Common Causes:**
- AppData directories not cleaned during uninstall (Tauri preserves user data by default)
- Background processes still running after uninstall
- Installer using `quiet` mode without admin privileges (silent failure)

**Solutions:**

1. **Clean Installation (Recommended):**
   ```powershell
   # Stop all processes
   Get-Process -Name "soul-player" -ErrorAction SilentlyContinue | Stop-Process -Force

   # Uninstall via Windows Settings
   # Settings > Apps > Installed apps > Soul Player > Uninstall

   # Clean AppData cache
   Remove-Item "$env:APPDATA\Soul Player" -Recurse -Force -ErrorAction SilentlyContinue
   Remove-Item "$env:LOCALAPPDATA\Soul Player" -Recurse -Force -ErrorAction SilentlyContinue

   # Reinstall fresh installer
   # Right-click Soul.Player_X.X.X_x64-setup.exe > Run as Administrator
   ```

2. **Automated Cleanup Script:**
   - Run `cleanup-soul-player.ps1` (generated during development)
   - Handles process termination, cache cleanup, and verification

3. **Verify Installation:**
   ```powershell
   # Check installed version
   & "C:\Program Files\Soul Player\Soul Player.exe" --version

   # Or check in app: Settings > About
   ```

**Prevention (Configuration):**
- ✅ `installMode: "currentUser"` - Default, no admin required, avoids silent failures
- ✅ WebView2 embedded bootstrapper - Ensures consistent runtime
- ✅ Proper uninstall hooks - Cleans up application data when user opts in

**References:**
- [Tauri Issue #5861](https://github.com/tauri-apps/tauri/issues/5861) - Old version after update
- [Tauri Issue #6113](https://github.com/tauri-apps/tauri/issues/6113) - AppData leftover after uninstall
- [Tauri Issue #8875](https://github.com/tauri-apps/tauri/issues/8875) - App continues running after uninstall
- [Windows Installer Docs](https://v2.tauri.app/distribute/windows-installer/)

---

## Pre-Commit Requirements (CRITICAL FOR AI AGENTS)

**MANDATORY**: Before committing ANY code changes, ALL checks below MUST pass. CI will reject commits that fail these checks.

**Automated Checks**: Husky is configured to automatically run all quality checks before each commit. If any check fails, the commit will be blocked until issues are fixed.

**Windows File Locking**: If Rust tests fail due to file locks (common when dev server or IDE is running), Husky will warn but allow the commit. Tests will still run in CI. Close running applications before committing to run tests locally.

### Rust Quality Checks
```bash
# 1. Format check (auto-fix: cargo fmt --all)
cargo fmt --all --check

# 2. Lint check (fix warnings manually)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 3. Tests (all must pass)
cargo test --all
```

### TypeScript/Frontend Quality Checks
```bash
# 4. TypeScript type checking (fix errors manually)
yarn workspace soul-player-desktop run tsc --noEmit
yarn workspace @soul-player/shared run tsc --noEmit
yarn workspace @soul-player/marketing run tsc --noEmit

# 5. ESLint (auto-fix: add --fix flag)
yarn workspace soul-player-desktop run lint
yarn workspace @soul-player/shared run lint
```

### AI Agent Workflow
1. Make code changes
2. **Run ALL pre-commit checks** (automated via Husky, or use script below for manual runs)
3. **Fix ANY errors before proceeding**
4. Only commit when ALL checks pass ✓

**Manual Check Scripts** (optional - Husky runs these automatically):
```bash
# Unix/Linux/macOS
./scripts/pre-commit-check.sh

# Windows (PowerShell)
.\scripts\pre-commit-check.ps1
```

**Note**: When you run `git commit`, Husky will automatically execute all checks. You can also run the scripts above manually to test before committing.

**Bypassing Hooks** (use sparingly for WIP commits):
```bash
git commit --no-verify -m "WIP: work in progress"
```

**Common Issues**:
- **TypeScript TS6133**: Unused variables → prefix with `_` or remove
- **TypeScript TS2722**: Possibly undefined → add `?.` optional chaining or null check
- **Clippy warnings**: Follow suggestions or use `#[allow(...)]` with justification
- **Format**: Run `cargo fmt --all` to auto-fix

---

## Quick Reference

### Key Documentation
- **Architecture**: [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
- **Testing**: [docs/TESTING.md](./docs/TESTING.md)
- **Local Build Testing**: [docs/LOCAL_BUILD_TESTING.md](./docs/LOCAL_BUILD_TESTING.md) - Test release builds locally with Docker
- **Conventions**: [docs/CONVENTIONS.md](./docs/CONVENTIONS.md)
- **SQLx Setup**: [docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md)
- **Contributing**: [CONTRIBUTING.md](./CONTRIBUTING.md)
- **Roadmap**: [ROADMAP.md](./ROADMAP.md)

### Database Schema
See `libraries/soul-storage/migrations/*.sql` for full schema.
Core tables: `users`, `tracks`, `albums`, `artists`, `playlists`, `playlist_tracks`

### Frontend Stack (Tauri Desktop)
- React 18 + TypeScript
- Zustand (state)
- TailwindCSS (styling)
- Lucide React (icons)

### macOS Code Signing (Ad-Hoc Signing)

Soul Player uses **ad-hoc code signing** for macOS builds - a free alternative to paid Apple Developer Program membership.

**Configuration Files:**
- `applications/desktop/src-tauri/tauri.conf.json` - Bundle configuration with macOS signing settings
- `applications/desktop/src-tauri/entitlements.plist` - App entitlements for hardened runtime

**Key Settings (tauri.conf.json):**
```json
{
  "bundle": {
    "macOS": {
      "minimumSystemVersion": "10.15",
      "signingIdentity": "-",
      "entitlements": "./entitlements.plist",
      "hardenedRuntime": true
    }
  }
}
```

**What This Provides:**
- ✅ Ad-hoc code signature (prevents "app is damaged" errors)
- ✅ Hardened runtime for macOS security
- ✅ Proper entitlements for audio playback, file access, network
- ❌ Not notarized (users see Gatekeeper warning on first launch)

**User Experience:**
- First launch: Users must right-click → Open (or run `xattr -cr "/Applications/Soul Player.app"`)
- Subsequent launches: App opens normally like any other app

**Upgrading to Notarization:**
To fully remove Gatekeeper warnings, add these GitHub secrets and update workflow:
- `APPLE_CERTIFICATE` - Developer ID certificate (.p12 base64 encoded)
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_ID` - Apple account email
- `APPLE_PASSWORD` - App-specific password
- `APPLE_TEAM_ID` - Team ID from App Store Connect

See `.github/workflows/release.yml` (macOS build section) for detailed upgrade instructions.

**Documentation:**
- User guide: `docs/MACOS_INSTALLATION.md`
- Tauri docs: https://v2.tauri.app/distribute/sign/macos/

### Keyboard Shortcuts System

The desktop app supports customizable keyboard shortcuts with these characteristics:
- **App-level only**: Shortcuts only work when the app window is focused (NOT OS-level global)
- **Input-aware**: Shortcuts are disabled when typing in input fields, textareas, or contenteditable elements
- **Customizable**: Users can configure shortcuts in Settings > Keyboard Shortcuts

**Architecture:**
```
┌─────────────────────────────────────────────────────────────────┐
│  Database (soul-storage)                                        │
│  └── global_shortcuts table (user_id, action, accelerator)      │
├─────────────────────────────────────────────────────────────────┤
│  Backend (Tauri)                                                │
│  └── shortcuts.rs                                               │
│      └── Tauri commands: get/set/reset_global_shortcuts         │
│          (storage only, no OS-level registration)               │
├─────────────────────────────────────────────────────────────────┤
│  Frontend (React)                                               │
│  ├── useKeyboardShortcuts.ts (hook)                             │
│  │   ├── Loads shortcuts from database                          │
│  │   ├── Listens for keydown events                             │
│  │   ├── Checks if active element is editable (skips if so)     │
│  │   └── Executes playback commands via Tauri invoke            │
│  ├── TauriPlayerCommandsProvider.tsx                            │
│  │   └── Initializes useKeyboardShortcuts hook                  │
│  └── ShortcutsSettings.tsx                                      │
│      └── UI for viewing/editing shortcuts                       │
└─────────────────────────────────────────────────────────────────┘
```

**Key Files:**
- `libraries/soul-storage/src/shortcuts/mod.rs` - Data model & DB queries
- `applications/desktop/src-tauri/src/shortcuts.rs` - Tauri commands (storage only)
- `applications/desktop/src/hooks/useKeyboardShortcuts.ts` - Keyboard event handling
- `applications/desktop/src/components/ShortcutsSettings.tsx` - Settings UI

**Important:**
- Do NOT use `tauri-plugin-global-shortcut` - we use app-level React shortcuts instead
- Do NOT add playback shortcuts to MainLayout - they are handled by useKeyboardShortcuts
- MainLayout only handles navigation shortcuts (Ctrl+K for search, etc.)
- **Shortcuts MUST use PlayerCommandsContext** - never call `invoke()` directly for playback commands
- This ensures shortcuts and UI buttons use the same code path and behave identically

**Default Shortcuts (Windows/Linux use Ctrl, macOS uses Cmd):**
| Action       | Shortcut              |
|--------------|----------------------|
| Play/Pause   | Ctrl/Cmd + Space     |
| Next Track   | Ctrl/Cmd + Right     |
| Previous     | Ctrl/Cmd + Left      |
| Volume Up    | Ctrl/Cmd + Up        |
| Volume Down  | Ctrl/Cmd + Down      |
| Mute         | Ctrl/Cmd + M         |

**Adding a New Shortcut Action:**
1. Add variant to `ShortcutAction` enum in `soul-storage/src/shortcuts/mod.rs`
2. Update `as_str()` and `from_str()` methods
3. Add to `default_shortcuts()` if it should have a default binding
4. Add case to `executeAction()` in `useKeyboardShortcuts.ts`
5. Add translation key in `i18n/*.json` files

### Backend Abstraction (BackendContext)

The frontend uses a `BackendContext` abstraction to ensure **zero parity** between desktop and marketing demo - both use the EXACT SAME UI components.

**Architecture:**
```
┌─────────────────────────────────────────────────────────────────┐
│  Shared Package (@soul-player/shared)                           │
│  ├── BackendContext.tsx - Interface definition                  │
│  │   └── BackendInterface: getAllTracks, getAlbumTracks, etc.   │
│  ├── PlatformContext.tsx - Platform detection                   │
│  │   └── DesktopOnly, WebOnly, FeatureGate components           │
│  └── Shared Pages (HomePage, LibraryPage, AlbumPage, etc.)      │
│      └── Use useBackend() - no direct invoke() calls            │
├─────────────────────────────────────────────────────────────────┤
│  Desktop (applications/desktop)                                  │
│  └── TauriBackendProvider.tsx                                   │
│      └── Implements BackendInterface using Tauri invoke()       │
├─────────────────────────────────────────────────────────────────┤
│  Marketing Demo (applications/marketing)                         │
│  └── DemoBackendProvider.tsx                                    │
│      └── Implements BackendInterface using DemoStorage + WASM   │
└─────────────────────────────────────────────────────────────────┘
```

**Key Files:**
- `applications/shared/src/contexts/BackendContext.tsx` - Core interface
- `applications/shared/src/contexts/PlatformContext.tsx` - Platform awareness
- `applications/desktop/src/providers/TauriBackendProvider.tsx` - Desktop implementation
- `applications/marketing/src/providers/DemoBackendProvider.tsx` - Demo implementation

**Critical Rules:**
1. **Never use `invoke()` directly in shared pages** - always use `useBackend()` hook
2. **Shared pages must work on both platforms** - no platform-specific imports
3. **Use `DesktopOnly`/`WebOnly`/`FeatureGate` for conditional rendering**
4. **New backend operations must be added to BackendInterface first**

```typescript
// ✅ CORRECT - Use BackendContext in shared pages
import { useBackend } from '../contexts/BackendContext'

function LibraryPage() {
  const backend = useBackend()
  const tracks = await backend.getAllTracks()
  await backend.playQueue(queue, 0)
}

// ❌ WRONG - Direct invoke() in shared code
import { invoke } from '@tauri-apps/api/core'

function LibraryPage() {
  const tracks = await invoke('get_all_tracks')  // Won't work in marketing!
}
```

**Adding a New Backend Operation:**
1. Add method signature to `BackendInterface` in `BackendContext.tsx`
2. Add types if needed (e.g., new data structures)
3. Implement in `TauriBackendProvider.tsx` using Tauri invoke
4. Implement in `DemoBackendProvider.tsx` using demo storage/WASM
5. Export new types from `applications/shared/src/index.ts`

### Playback Architecture (CRITICAL)

The playback system has **two separate, non-overlapping contexts** - mixing them causes bugs and duplicate code:

#### **Separation of Concerns:**

```
┌─────────────────────────────────────────────────────────────────┐
│  BackendContext - Library Data & Context Recording              │
│  Purpose: Data fetching, NOT playback control                   │
│  ├── getAllTracks, getAlbumTracks, getPlaylistTracks           │
│  ├── getAllAlbums, getAllArtists, getAllPlaylists              │
│  └── recordContext (for "Jump Back In")                         │
├─────────────────────────────────────────────────────────────────┤
│  PlayerCommandsContext - Playback Control Only                  │
│  Purpose: Audio control, NOT data fetching                      │
│  ├── playQueue, pausePlayback, resumePlayback                   │
│  ├── skipNext, skipPrevious, seek, setVolume                   │
│  ├── setShuffle, setRepeatMode, getQueue                        │
│  └── Event listeners: onStateChange, onTrackChange, etc.        │
└─────────────────────────────────────────────────────────────────┘
```

#### **Command Flow:**

```
UI Component / Keyboard Shortcut
  ↓ (fetch data)
BackendContext → useBackend() → TauriBackendProvider → invoke('get_*')
  ↓ (build queue from data)
  ↓ (playback control)
PlayerCommandsContext → usePlayerCommands() → TauriPlayerCommandsProvider → invoke('play_queue')
  ↓ (event emission)
Tauri Events → usePlaybackEvents() → Zustand Store → UI Update
```

**CRITICAL:** Both UI buttons AND keyboard shortcuts MUST use `PlayerCommandsContext`. Never call `invoke()` directly for playback commands.

#### **Critical Rules:**

1. **BackendContext is for DATA ONLY**
   - ✅ Use for: getAllTracks, getAlbumById, recordContext
   - ❌ NEVER add: playQueue, pause, skip, setVolume

2. **PlayerCommandsContext is for CONTROL ONLY**
   - ✅ Use for: playQueue, pause, skip, seek, setVolume
   - ❌ NEVER add: getAllTracks, getAlbumById, database queries

3. **Playback Pattern (UI → Audio):**
   ```typescript
   // Step 1: Fetch data via BackendContext
   const backend = useBackend()
   const tracks = await backend.getAlbumTracks(albumId)

   // Step 2: Transform to queue format
   const queue = tracks.map(t => ({
     trackId: String(t.id),
     title: t.title,
     filePath: t.file_path!,
     // ...
   }))

   // Step 3: Record playback context (optional)
   await backend.recordContext({
     contextType: 'album',
     contextId: String(albumId)
   })

   // Step 4: Control playback via PlayerCommandsContext
   const commands = usePlayerCommands()
   await commands.playQueue(queue, 0)
   ```

4. **NEVER Duplicate Methods Across Contexts**
   - If a method exists in PlayerCommandsContext, it should NOT be in BackendContext
   - If a method exists in BackendContext, it should NOT be in PlayerCommandsContext
   - Duplication leads to inconsistent behavior and maintenance burden

#### **Rust Backend Architecture:**

```
Tauri Commands (main.rs)
  ↓
PlaybackManager (playback.rs) - Wrapper for event emission
  ↓
DesktopPlayback (soul-audio-desktop) - Platform integration
  ↓
PlaybackManager (soul-playback) - Core orchestration
  ↓
AudioSource + Queue + Volume + Crossfade → Audio Output
```

**Event Emission:** All playback state changes emit Tauri events automatically:
- `playback:state-changed` - Playing/Paused/Stopped
- `playback:track-changed` - Current track info
- `playback:position-updated` - Playback position (every 250ms)
- `playback:volume-changed` - Volume level (0-100)
- `playback:queue-updated` - Queue modified

#### **Common Mistakes to Avoid:**

❌ **Adding playQueue to BackendContext**
```typescript
// WRONG - playQueue is playback control, not data
export interface BackendInterface {
  getAllTracks: () => Promise<Track[]>
  playQueue: (queue, index) => Promise<void>  // ❌ Don't do this!
}
```

❌ **Using backend.playQueue() in UI**
```typescript
// WRONG - use commands.playQueue() instead
await backend.playQueue(queue, 0)  // ❌
```

❌ **Direct invoke() in keyboard shortcuts**
```typescript
// WRONG - bypasses PlayerCommandsContext
await invoke('pause_playback')  // ❌
await invoke('next_track')       // ❌

// CORRECT - use PlayerCommandsContext
const commands = usePlayerCommands()
await commands.pausePlayback()  // ✅
await commands.skipNext()        // ✅
```

✅ **Correct Separation**
```typescript
// BackendContext - data only
export interface BackendInterface {
  getAllTracks: () => Promise<Track[]>
  getAlbumTracks: (id) => Promise<Track[]>
  recordContext: (ctx) => Promise<void>
}

// PlayerCommandsContext - control only
export interface PlayerCommandsInterface {
  playQueue: (queue, index) => Promise<void>
  pausePlayback: () => Promise<void>
  setShuffle: (enabled) => Promise<void>
}
```

#### **Key Files:**

- `applications/shared/src/contexts/BackendContext.tsx` - **Data interface**
- `applications/shared/src/contexts/PlayerCommandsContext.tsx` - **Control interface**
- `applications/desktop/src/providers/TauriBackendProvider.tsx` - Data implementation
- `applications/desktop/src/providers/TauriPlayerCommandsProvider.tsx` - Control implementation
- `applications/desktop/src-tauri/src/playback.rs` - Rust playback wrapper
- `libraries/soul-playback/src/manager.rs` - Core playback logic

**When Adding New Features:**
1. Ask: "Is this data fetching or playback control?"
2. Data → BackendContext, Control → PlayerCommandsContext
3. Update both desktop AND demo implementations
4. Test on both platforms

### Running Tests

Tests use isolated databases in system temp directories:
```bash
cargo test --all                    # Run all tests
cargo test -p soul-storage          # Run tests for specific crate
cargo test --test integration_test  # Run specific test file
```

**Test Database Strategy:**
- Tests create temporary databases automatically (cleaned up after)
- Compile-time verification uses `libraries/soul-storage/.tmp/dev.db`
- All `*.db` files are gitignored - never commit database files

### Before Committing
```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo check -p soul-storage         # Verify compile-time queries
```

---

## When in Doubt

1. **Multi-user**: Always require `user_id` parameter
2. **Database**: Use compile-time `query!` macros
3. **Platform code**: Use traits, isolate in `applications/`
4. **Tests**: Skip if just testing a getter/setter
5. **Allocations**: Never in audio `process()` methods
6. **Dependencies**: Libraries can't depend on applications
7. **UI Strings**: Always use localization, never hardcode text
8. **Keyboard shortcuts**: Add to app-level shortcuts in useKeyboardShortcuts.ts (NOT global/OS-level)
9. **Shared pages**: Use `useBackend()` hook, never direct `invoke()` calls - ensures desktop/marketing parity
10. **Playback architecture**: Data fetching → BackendContext, Playback control → PlayerCommandsContext. NEVER mix or duplicate.
11. **Version bumps**: ALWAYS use `./scripts/bump-version.sh` - never manually edit version numbers (prevents UI version mismatch)
12. **Windows version issues**: If UI shows old version after update, clean AppData cache and reinstall (see "Windows Installer Caching Issues" section)

---

**Last Updated**: 2026-01-20
**Rust Edition**: 2021
**Platforms**: Windows, macOS, Linux
