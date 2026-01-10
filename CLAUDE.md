# CLAUDE.md

Instructions for Claude Code when working with Soul Player.

---

## Project Overview

**Soul Player**: Cross-platform music player (Desktop/Server/ESP32-S3)
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
- Firmware (ESP32-S3): Minimal string tables for display text
- Applies to: buttons, labels, messages, tooltips, errors

```typescript
// ✅ CORRECT (React)
<button>{t('playback.play')}</button>
<div className="error">{t('errors.file_not_found', { filename })}</div>

// ❌ WRONG
<button>Play</button>
<div className="error">File not found: {filename}</div>
```

```rust
// ✅ CORRECT (Firmware/Embedded)
const STRINGS: &[&str] = &[
    "Play",      // en
    "Jouer",     // fr
    "Abspielen", // de
];
display.text(STRINGS[locale_index]);

// ❌ WRONG
display.text("Play");
```

**Why**: Enables internationalization from day 1, easier to maintain, professional UX.

---

## Essential Commands

### First-Time Setup
```bash
corepack enable              # Enable Yarn 4.x (run in root)
yarn                         # Install all dependencies (run in root)
./scripts/setup-sqlx.sh      # Setup SQLx offline mode
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
```bash
sqlx migrate run --source libraries/soul-storage/migrations
cd libraries/soul-storage && cargo sqlx prepare -- --lib
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

---

## Quick Reference

### Key Documentation
- **Architecture**: [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)
- **Testing**: [docs/TESTING.md](./docs/TESTING.md)
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

### Before Committing
```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
DATABASE_URL="sqlite:test.db" cargo check
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

---

**Last Updated**: 2026-01-08
**Rust Edition**: 2021
**Platforms**: Windows, macOS, Linux, ESP32-S3
