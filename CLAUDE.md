# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

---

## Project Overview

**Soul Player** is a local-first, cross-platform music player with multi-user server streaming and embedded hardware support (ESP32-S3).

**Key Characteristics**:
- Monorepo using Cargo workspaces + Moon task runner
- Shared Rust core logic across desktop, server, and embedded platforms
- Multi-user database schema from the start (reused across all modes)
- Trait-based architecture for platform-specific adapters
- Quality-focused testing (50-60% coverage, no shallow tests)

---

## Essential Commands

### Development
```bash
# Build all workspace crates
cargo build --all

# Run desktop app (Tauri)
cd crates/soul-player-desktop && cargo tauri dev

# Run server
cargo run -p soul-server

# ESP32-S3 build
cd crates/soul-player-esp32 && cargo build --release

# Format all code
cargo fmt --all

# Lint with clippy
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all

# Integration tests with testcontainers
cargo test --all --features testcontainers

# Security audit
cargo audit
```

### Moon Tasks
```bash
# Run via Moon (task orchestration)
moon run :build
moon run :test
moon run :lint
```

---

## Architecture Quick Reference

### Crate Dependency Graph
```
soul-core (traits & types)
    ↓
├─→ soul-storage (SQLite, multi-user schema)
├─→ soul-audio (Symphonia decoder + CPAL/ESP32 output)
├─→ soul-metadata (tag reading, library scanning)
├─→ soul-discovery (Bandcamp, Discogs - Phase 4)
└─→ soul-sync (client-server sync protocol)
    ↓
├─→ soul-player-desktop (Tauri app)
├─→ soul-server (Axum server, multi-user auth)
└─→ soul-player-esp32 (Embassy firmware)
```

### Key Design Decisions

1. **Storage**: SQLite everywhere (desktop, server, ESP32 SD card)
   - Multi-user schema from the start
   - Users table + user-owned playlists
   - Shared library of tracks
   - Server uses same schema, same crate

2. **Audio Pipeline**:
   - Decoder: Symphonia (MP3, FLAC, OGG, WAV, AAC, OPUS)
   - Desktop output: CPAL
   - ESP32 output: awedio_esp32 (uses Symphonia internally)
   - Effect chain: Trait-based (EQ + Compressor for MVP)

3. **Platforms**:
   - Desktop: Tauri v2 (Windows, macOS, Linux)
   - Server: Docker container, one-click setup
   - Embedded: ESP32-S3 (NOT STM32 - needs std support)

4. **No DSD support in MVP** (add later if needed)

---

## Critical Architecture Constraints

### Embedded-First Design
- Core logic must work on ESP32-S3 (std, not no_std)
- Vertical slicing: Features work across all platforms when implemented
- Platform-specific code isolated via traits or `cfg` flags

### Multi-User from Day 1
- Database schema supports multiple users natively
- Desktop app uses "default user" (user_id = 1)
- Server mode: Real multi-user with authentication
- Same storage crate, same schema, reused everywhere

### Effect Chain Architecture
```rust
// Real-time audio processing (NO allocations in callback)
trait AudioEffect: Send {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32);
}

// Chain multiple effects
struct EffectChain {
    effects: Vec<Box<dyn AudioEffect>>,
}
```

---

## Testing Philosophy

**Quality over quantity** - See [`docs/TESTING.md`](docs/TESTING.md) for full details.

### Key Principles
- ❌ NO shallow tests (getters, setters, trivial constructors)
- ✅ Focus on business logic, edge cases, integration points
- ✅ Use testcontainers with **real SQLite** (NOT in-memory)
- ✅ Target 50-60% coverage (meaningful, not line-counting)

### Test Commands
```bash
# Unit tests
cargo test --lib --all

# Integration tests
cargo test --test '*' --all

# With testcontainers (storage, server)
cargo test --features testcontainers

# Property-based tests (proptest for audio/algorithms)
cargo test --all -- --ignored
```

---

## CI/CD Requirements

### Blocking PR Checks
- ❌ Clippy warnings (`-D warnings`)
- ❌ Rustfmt differences
- ❌ Test failures
- ❌ Security vulnerabilities (`cargo audit`)

### Non-Blocking
- ⚠️ Code coverage report (target 50-60%, don't block)

### Build Matrix
- **Platforms**: Linux (x64, ARM64), macOS (Intel, Apple Silicon), Windows (x64)
- **Targets**: Desktop binaries, Server binary, ESP32-S3 firmware

---

## Common Patterns

### Adding a New Crate
1. Create in `crates/` directory
2. Add to workspace `Cargo.toml`
3. Update `moon.yml` task configuration
4. Follow structure: `src/lib.rs`, `error.rs`, `types/`, `services/`
5. Add tests in `tests/` directory

### Platform-Specific Code
```rust
// Preferred: Trait abstraction
pub trait AudioOutput {
    fn play(&mut self, buffer: &AudioBuffer) -> Result<()>;
}

// Fallback: Conditional compilation
#[cfg(target_os = "espidf")]
use awedio_esp32;

#[cfg(not(target_os = "espidf"))]
use cpal;
```

### Database Queries (Multi-User)
```rust
// ALWAYS include user context for playlists
async fn get_user_playlists(
    &self,
    user_id: UserId
) -> Result<Vec<Playlist>> {
    sqlx::query_as!(
        Playlist,
        "SELECT * FROM playlists WHERE owner_id = ? OR id IN (
            SELECT playlist_id FROM playlist_shares WHERE shared_with_user_id = ?
        )",
        user_id,
        user_id
    )
    .fetch_all(&self.pool)
    .await
}
```

---

## Important Files

### Documentation
- [`ROADMAP.md`](ROADMAP.md) - Implementation phases and timeline
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) - System design and crate details
- [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md) - Coding standards and best practices
- [`docs/TESTING.md`](docs/TESTING.md) - Testing strategy and patterns

### Configuration
- `Cargo.toml` - Workspace configuration
- `moon.yml` - Task orchestration (build, test, lint)
- `.github/workflows/` - CI/CD pipelines

---

## Database Schema (Multi-User)

Core tables (see `soul-storage/migrations/`):
```sql
users              -- User accounts
tracks             -- Shared library of tracks
playlists          -- User-owned playlists (owner_id FK)
playlist_tracks    -- Many-to-many (playlists ↔ tracks)
playlist_shares    -- Playlist collaboration (shared playlists)
```

**Key Insight**: Same schema used in desktop (single user), server (multi-user), and ESP32 (offline user).

---

## Audio Formats Support

| Format | Desktop | Server | ESP32-S3 |
|--------|---------|--------|----------|
| MP3    | ✅      | ✅     | ✅       |
| FLAC   | ✅      | ✅     | ✅       |
| OGG    | ✅      | ✅     | ✅       |
| WAV    | ✅      | ✅     | ✅       |
| AAC    | ✅      | ✅     | ✅       |
| OPUS   | ✅      | ✅     | ✅       |
| DSD    | ❌ (future) | ❌ | ❌  |

All use **Symphonia** decoder (pure Rust, cross-platform).

---

## Server Deployment

```bash
# Build Docker image
docker build -t soul-server .

# Run container
docker run -d \
  -p 8080:8080 \
  -v /path/to/music:/music \
  -v /path/to/data:/data \
  -e JWT_SECRET=your-secret \
  soul-server

# One-click setup script (future)
./scripts/setup-server.sh
```

---

## ESP32-S3 Development

```bash
# Install ESP toolchain
espup install

# Build firmware
cd crates/soul-player-esp32
cargo build --release

# Flash to device
espflash flash target/xtensa-esp32s3-espidf/release/soul-player-esp32

# Monitor serial output
espflash monitor
```

**Hardware**: ESP32-S3 with:
- I2S DAC for audio output
- SD card (SDMMC interface)
- E-ink display (SPI)
- WiFi for server sync

---

## Troubleshooting

### Build Issues
- **Tauri build fails**: Check Node.js version (16+), install system dependencies
- **ESP32 build fails**: Ensure `espup` toolchain installed, set `LIBCLANG_PATH`
- **SQLite errors**: Check migrations ran, verify schema version

### Test Issues
- **Testcontainers failing**: Ensure Docker running, check port conflicts
- **Audio tests failing**: Check audio device availability, run headless tests with dummy output
- **Integration tests timeout**: Increase timeout, check database cleanup

---

## Dependencies to Know

### Core
- `symphonia` - Audio decoding (all formats)
- `sqlx` - SQLite database (compile-time checked queries)
- `serde` - Serialization (JSON, TOML)
- `thiserror` - Error handling (libraries)

### Desktop
- `tauri` - Desktop app framework
- `cpal` - Cross-platform audio output

### Server
- `axum` - HTTP server
- `tokio` - Async runtime
- `jsonwebtoken` - JWT authentication

### Embedded
- `embassy-executor` - Async embedded runtime
- `embassy-stm32` - Wait, this is ESP32! Use `esp-idf-hal`
- `awedio_esp32` - Audio output for ESP32

### Testing
- `testcontainers` - Database integration tests
- `proptest` - Property-based testing
- `criterion` - Benchmarking

---

## Development Workflow

1. **Start with tests** (TDD encouraged for complex logic)
2. **Implement core logic** in trait-based way
3. **Add platform-specific adapters** (desktop, server, ESP32)
4. **Verify cross-platform** compilation
5. **Run full test suite** (unit + integration)
6. **Update documentation** if architecture changes

---

## Code Review Checklist

Before submitting PR:
- [ ] Tests added for new functionality (no shallow tests!)
- [ ] Multi-user implications considered (if touching storage)
- [ ] Platform-specific code isolated (traits or `cfg`)
- [ ] Documentation updated (if public API changed)
- [ ] Error handling complete (no unwraps in library code)
- [ ] Performance implications reviewed (audio hot path?)
- [ ] Security reviewed (user input validation, SQL injection?)

---

## Need Help?

1. **Architecture questions**: See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
2. **Coding standards**: See [`docs/CONVENTIONS.md`](docs/CONVENTIONS.md)
3. **Testing approach**: See [`docs/TESTING.md`](docs/TESTING.md)
4. **Implementation plan**: See [`ROADMAP.md`](ROADMAP.md)

---

## Quick Start for New Contributors

```bash
# Clone and setup
git clone https://github.com/yourusername/soul-player.git
cd soul-player

# Install Rust toolchain
rustup update stable

# Install Moon task runner
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Build everything
cargo build --all

# Run tests
cargo test --all

# Start desktop app
cd crates/soul-player-desktop && cargo tauri dev
```

---

## Project Status

**Current Phase**: Phase 1 - Desktop Foundation (see ROADMAP.md)

**Next Milestones**:
1. Complete storage layer with multi-user schema
2. Implement audio engine with effect chain
3. Build Tauri desktop UI
4. Set up CI/CD pipelines

---

## Important Notes for Claude Code

- This project uses **late 2025/early 2026 Rust best practices**
- Embedded target is **ESP32-S3** (std, not no_std) - NOT STM32
- Multi-user database schema is **fundamental** - not bolted on later
- Testing uses **real databases** (testcontainers) - never in-memory SQLite
- Code coverage targets are **guidelines** (50-60%) - quality over quantity
- Always **vertically slice** features (desktop + server + ESP32 when possible)
