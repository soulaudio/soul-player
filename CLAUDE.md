# CLAUDE.md

Instructions for Claude Code when working with Soul Player codebase.

---

## Project Identity

**Soul Player**: Cross-platform music player (Desktop/Server/ESP32-S3)
- **Structure**: Cargo workspace monorepo + Moon tasks + Tauri desktop
- **Storage**: SQLite everywhere, multi-user schema from day 1
- **Audio**: Symphonia decoder + platform-specific output (CPAL/ESP32)
- **Languages**: Rust (backend/libs) + TypeScript/React (Tauri frontend)

---

## Critical Constraints

### MUST Follow

1. **Multi-User Always**: Every database query MUST respect user context
   - Desktop = `user_id = 1` (default user)
   - Server = authenticated user ID
   - Never query playlists/settings without user context

2. **Platform-Agnostic Core**: Libraries under `libraries/` MUST NOT depend on platform-specific crates
   - Use traits for platform abstraction
   - Platform code in `applications/` only

3. **Real-Time Audio Safety**: Audio callback paths MUST NOT allocate
   - No `Vec::new()`, `Box::new()`, `String::from()` in `process()` methods
   - Pre-allocate buffers, use fixed-size arrays

4. **Test Quality**: NO shallow tests
   - ❌ Don't test: getters, setters, constructors, `Default` impls
   - ✅ DO test: business logic, edge cases, error paths, integration points
   - Use testcontainers with real SQLite (never in-memory)

5. **Error Handling**: Libraries use `Result<T>`, applications can panic
   - Libraries: `thiserror` for errors, no `.unwrap()` in public APIs
   - Applications: `.expect()` with clear messages is fine

### Directory Structure

```
applications/
  ├── desktop/        # Tauri app (Rust + TypeScript/React)
  ├── server/         # Axum HTTP server
  └── shared/         # Shared TypeScript components/hooks

libraries/
  ├── soul-core/      # Core traits & types (no platform deps)
  ├── soul-storage/   # SQLite + sqlx (multi-user schema)
  ├── soul-audio/     # Decoder + effects (Symphonia)
  ├── soul-audio-desktop/   # CPAL output
  ├── soul-audio-mobile/    # Mobile audio output
  ├── soul-audio-embedded/  # ESP32 I2S output
  ├── soul-metadata/  # Tag reading (MP3/FLAC/etc)
  ├── soul-playback/  # Queue/shuffle/history logic
  └── soul-importer/  # Library scanning & import
```

---

## Essential Commands

```bash
# First-time setup
corepack enable                  # Enable Yarn 4.x (first time only)
yarn install                     # Install all dependencies
./scripts/setup-sqlx.sh          # Setup SQLx offline mode

# Build & Test
cargo build --all
cargo test --all
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all

# Desktop app
yarn dev:desktop

# Server
cargo run -p soul-server

# Database migrations (after schema changes)
sqlx migrate run --source libraries/soul-storage/migrations
cd libraries/soul-storage && cargo sqlx prepare -- --lib

# Moon tasks
moon run :build
moon run :test
moon run :lint
```

**For SQLx issues, configuration, or troubleshooting: See [docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md)**

---

## Code Patterns & Best Practices

### Database: Compile-Time Query Verification (REQUIRED)

**ALL database queries MUST use compile-time macros (`query!` / `query_as!`).**

```rust
// ✅ CORRECT: Compile-time verified query
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Track>> {
    let row = sqlx::query_as!(
        Track,
        "SELECT id, title, artist_id, album_id, duration_ms, file_hash
         FROM tracks
         WHERE id = ?",
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

// ✅ CORRECT: Insert with query!
pub async fn create(pool: &SqlitePool, name: &str) -> Result<i64> {
    let result = sqlx::query!(
        "INSERT INTO artists (name, sort_name) VALUES (?, ?)",
        name,
        name
    )
    .execute(pool)
    .await?;

    Ok(result.last_insert_rowid())
}

// ❌ WRONG: Runtime query (not type-safe)
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<Option<Track>> {
    let row = sqlx::query("SELECT * FROM tracks WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    // ❌ No compile-time verification, error-prone
}
```

**Why compile-time queries:**
- ✅ Typos in column names = compile error (not runtime crash)
- ✅ Schema changes = immediate feedback on what broke
- ✅ Type mismatches = compile error
- ✅ Better IDE support (auto-completion, type hints)
- ✅ Refactoring safety

**Setup required:** See [docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md)

### Database: Multi-User Queries

```rust
// ✅ CORRECT: Always filter by user
pub async fn get_playlists(pool: &SqlitePool, user_id: i64) -> Result<Vec<Playlist>> {
    sqlx::query_as!(
        Playlist,
        "SELECT id, owner_id, name, created_at, updated_at
         FROM playlists
         WHERE owner_id = ?",
        user_id
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

// ❌ WRONG: Global query without user context
pub async fn get_all_playlists(pool: &SqlitePool) -> Result<Vec<Playlist>> {
    sqlx::query_as!(Playlist, "SELECT * FROM playlists")
        .fetch_all(pool).await
        .map_err(Into::into)
}
```

### Database: Query Patterns

```rust
// Pattern 1: SELECT with specific columns (preferred)
sqlx::query_as!(
    Artist,
    "SELECT id, name, sort_name, musicbrainz_id, created_at, updated_at
     FROM artists
     WHERE id = ?",
    id
)

// Pattern 2: INSERT and return ID
let result = sqlx::query!(
    "INSERT INTO tracks (title, duration_ms) VALUES (?, ?)",
    title,
    duration_ms
)
.execute(pool)
.await?;

let id = result.last_insert_rowid();

// Pattern 3: UPDATE with multiple columns
sqlx::query!(
    "UPDATE tracks SET title = ?, updated_at = datetime('now')
     WHERE id = ?",
    new_title,
    track_id
)
.execute(pool)
.await?;

// Pattern 4: DELETE
sqlx::query!("DELETE FROM tracks WHERE id = ?", track_id)
    .execute(pool)
    .await?;

// Pattern 5: JOIN queries
sqlx::query_as!(
    Album,
    "SELECT a.id, a.title, a.artist_id, ar.name as artist_name,
            a.year, a.cover_art_path, a.musicbrainz_id,
            a.created_at, a.updated_at
     FROM albums a
     LEFT JOIN artists ar ON a.artist_id = ar.id
     WHERE a.id = ?",
    id
)
```

### Platform Abstraction: Traits

```rust
// ✅ CORRECT: Trait in soul-core, impls in platform crates
pub trait AudioOutput: Send {
    fn play(&mut self, buffer: &[f32]) -> Result<()>;
}

// Desktop: soul-audio-desktop
impl AudioOutput for CpalOutput { ... }

// ESP32: soul-audio-embedded
impl AudioOutput for I2sOutput { ... }
```

### Audio Processing: No Allocations

```rust
// ✅ CORRECT: Pre-allocated buffer
pub struct Compressor {
    envelope: Vec<f32>,  // Pre-allocated in new()
}

impl AudioEffect for Compressor {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32) {
        // Only indexing, no allocations
        for (i, sample) in buffer.iter_mut().enumerate() {
            self.envelope[i] = (*sample).abs();
        }
    }
}

// ❌ WRONG: Allocates in hot path
fn process(&mut self, buffer: &mut [f32]) {
    let envelope = buffer.iter().map(|s| s.abs()).collect::<Vec<f32>>();
}
```

### Error Types: Library vs Application

```rust
// ✅ Libraries: thiserror + Result
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Track not found: {0}")]
    TrackNotFound(i64),
}

pub fn get_track(&self, id: i64) -> Result<Track, StorageError> { ... }

// ✅ Applications: can use expect/unwrap with context
let track = storage.get_track(id)
    .expect("Failed to load track from database");
```

### Testing: Meaningful Tests Only

```rust
// ✅ GOOD: Tests business logic
#[test]
fn shuffle_respects_no_repeat_within_window() {
    let mut shuffle = Shuffle::new(10, 3); // window_size = 3
    let mut seen = Vec::new();

    for _ in 0..20 {
        let idx = shuffle.next();
        assert!(!seen[seen.len().saturating_sub(3)..].contains(&idx));
        seen.push(idx);
    }
}

// ❌ BAD: Shallow test
#[test]
fn shuffle_new_returns_instance() {
    let shuffle = Shuffle::new(10, 3);
    assert_eq!(shuffle.size(), 10);  // Just testing a getter
}
```

---

## Architecture Rules

### Dependency Flow (MUST Enforce)

```
libraries/soul-core
    ↓ (traits & types only)
libraries/soul-{storage,audio,metadata,playback,importer}
    ↓ (business logic, platform-agnostic)
libraries/soul-audio-{desktop,mobile,embedded}
    ↓ (platform-specific implementations)
applications/{desktop,server}
    ↓ (composition & UI)
```

**Never reverse this flow**: Applications can't be dependencies of libraries.

### Feature Flags: Minimal Usage

Only use features for:
- Test utilities (`testcontainers`)
- Optional external integrations (future: Bandcamp API)

**Don't use features for**: Platform selection (use separate crates instead)

### Async Runtime: Tokio Everywhere

- Desktop: Tokio (via Tauri)
- Server: Tokio (via Axum)
- Libraries: Runtime-agnostic where possible (accept `&Pool` not `Runtime`)

---

## Database Schema (Core Tables)

```sql
-- Multi-user foundation
users (id, username, email, created_at)
user_settings (user_id, key, value)

-- Audio library (shared across users)
tracks (id, title, duration_ms, file_hash, ...)
albums (id, title, year, ...)
artists (id, name, ...)
genres (id, name)

-- User-owned data
playlists (id, owner_id, name, created_at)
playlist_tracks (playlist_id, track_id, position)

-- Multi-source support
sources (id, name, type)  -- local/http/webdav
track_sources (track_id, source_id, path)
```

**Migration Location**: `libraries/soul-storage/migrations/*.sql`

---

## Testing Strategy

### Test Organization

```
libraries/soul-*/
  ├── src/
  └── tests/
      ├── integration_test.rs      # End-to-end scenarios
      ├── property_test.rs         # Proptest for algorithms
      └── test_helpers.rs          # Shared test utilities
```

### Database Tests: Use Testcontainers

```rust
use soul_storage::test_helpers::create_test_context;

#[tokio::test]
async fn test_create_playlist() {
    let ctx = create_test_context().await;
    let user_id = 1;

    let playlist = ctx.playlists()
        .create(user_id, "My Playlist")
        .await
        .unwrap();

    assert_eq!(playlist.owner_id, user_id);
}
```

### Coverage Target: 50-60% (Quality, Not Quantity)

Focus coverage on:
- ✅ Business logic (shuffle, queue, effects)
- ✅ Error handling paths
- ✅ Database queries (user isolation)
- ✅ Audio processing (correctness)

Ignore:
- ❌ Generated code (Tauri commands)
- ❌ Simple getters/setters
- ❌ Type definitions

---

## Frontend (Tauri Desktop)

### Technology Stack

- **Framework**: React 18 + TypeScript
- **State**: Zustand (lightweight, no Redux)
- **Styling**: TailwindCSS
- **Icons**: Lucide React
- **Tauri**: v2 (invoke commands from Rust backend)

### Tauri Command Pattern

```rust
// Rust: applications/desktop/src-tauri/src/main.rs
#[tauri::command]
async fn create_playlist(
    state: State<'_, AppState>,
    name: String,
) -> Result<Playlist, String> {
    state.storage
        .playlists()
        .create(1, &name)  // user_id = 1 (desktop default)
        .await
        .map_err(|e| e.to_string())
}
```

```typescript
// TypeScript: applications/desktop/src/
import { invoke } from '@tauri-apps/api/core';

async function createPlaylist(name: string): Promise<Playlist> {
  return await invoke('create_playlist', { name });
}
```

### State Management: Zustand

```typescript
import { create } from 'zustand';

interface PlaybackState {
  isPlaying: boolean;
  currentTrack: Track | null;
  play: () => void;
  pause: () => void;
}

export const usePlayback = create<PlaybackState>((set) => ({
  isPlaying: false,
  currentTrack: null,
  play: () => set({ isPlaying: true }),
  pause: () => set({ isPlaying: false }),
}));
```

---

## Performance Guidelines

### Audio Hot Paths

- Pre-allocate all buffers in `new()`
- Use `&mut [f32]` slices, not `Vec<f32>`
- Profile with `cargo bench` (criterion)

### Database

- Use `sqlx::query!` macros (compile-time checked)
- Index foreign keys: `owner_id`, `track_id`, `playlist_id`
- Batch inserts for imports (use transactions)

### Desktop UI

- Virtualize long lists (react-window)
- Debounce search input (300ms)
- Cache album art (Tauri asset protocol)

---

## Security Checklist

When touching:

- **Auth code**: Validate JWT exp, check user_id in claims
- **File paths**: Sanitize input, prevent path traversal
- **SQL queries**: Use parameterized queries (sqlx handles this)
- **API endpoints**: Validate input schemas (serde)

---

## Before Committing

Run locally:
```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
DATABASE_URL="sqlite:test.db" cargo check  # Verify sqlx queries
```

CI will block PRs if:
- ❌ Rustfmt fails
- ❌ Clippy warnings present
- ❌ Tests fail
- ❌ `cargo audit` finds vulnerabilities

---

## Quick Reference: Key Files

- `Cargo.toml` - Workspace dependencies
- `.moon/workspace.yml` - Task runner config
- `applications/desktop/src-tauri/tauri.conf.json` - Tauri settings
- `libraries/soul-storage/migrations/` - Database schema
- `ROADMAP.md` - Implementation phases

---

## When in Doubt

1. **Multi-user**: Always require `user_id` parameter
2. **Platform code**: Use traits, isolate in applications/
3. **Tests**: Skip if it's just testing a getter
4. **Errors**: Return `Result` in libraries, `.expect()` in apps
5. **Allocations**: Never in audio `process()` methods
6. **Dependencies**: Libraries can't depend on applications

---

**Last Updated**: 2026-01-06
**Rust Edition**: 2021
**Target Platforms**: Windows, macOS, Linux, ESP32-S3
