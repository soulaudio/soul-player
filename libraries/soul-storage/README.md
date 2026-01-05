# Soul Storage

Multi-user, multi-source SQLite database layer for Soul Player.

## Quick Start

### First Time Setup

1. **Copy the environment file:**
   ```bash
   cp .env.example .env
   ```

2. **Create the development database:**
   ```bash
   cd libraries/soul-storage
   mkdir -p .tmp
   DATABASE_URL="sqlite:.tmp/dev.db" sqlx database create
   DATABASE_URL="sqlite:.tmp/dev.db" sqlx migrate run
   ```

3. **Build the crate:**
   ```bash
   cargo build -p soul-storage
   ```

## Architecture

This crate uses **SQLx with runtime queries** (not compile-time checked queries). This approach:

- ✅ Simple developer setup - no database required during compilation
- ✅ Works immediately without complex configuration
- ✅ SQL errors caught by integration tests (using testcontainers with real SQLite)
- ❌ No compile-time type safety for SQL queries

### Why Runtime Queries?

We initially tried SQLx's compile-time query checking (`sqlx::query!()` macros), but encountered issues:

- SQLx macros require a database connection **during compilation**
- Setting up `DATABASE_URL` for every developer was fragile
- CI/CD needed special configuration
- Offline mode (`.sqlx/` metadata) required regeneration on every query change

By using runtime queries (`sqlx::query()`), we get:
- Simpler setup
- Faster builds
- Better CI/CD compatibility
- Tests still catch SQL errors (we use testcontainers with real SQLite)

## Database Schema

See `migrations/` directory for the complete schema. Key tables:

- `users` - User accounts (multi-user from day 1)
- `sources` - Local and remote server sources
- `tracks` - Shared library of tracks
- `track_sources` - Many-to-many (tracks can exist in multiple sources)
- `playlists` - User-owned playlists
- `playlist_tracks` - Playlist contents
- `playlist_shares` - Collaborative playlists

## Testing

Run tests with:

```bash
# Unit tests
cargo test --lib -p soul-storage

# Integration tests (requires Docker for testcontainers)
cargo test -p soul-storage
```

Integration tests use **real SQLite databases** via testcontainers - never in-memory databases. This ensures tests match production behavior.

## Migrations

Add new migrations:

```bash
cd libraries/soul-storage
sqlx migrate add <migration_name>
```

Edit the generated file in `migrations/`, then run:

```bash
DATABASE_URL="sqlite:.tmp/dev.db" sqlx migrate run
```

## Multi-User Design

All queries are designed for multi-user from the start:

- Desktop app uses `user_id = 1` (default local user)
- Server mode uses real user authentication
- Playlists support ownership and sharing permissions
- Same schema works across desktop, server, and embedded (ESP32-S3)

## Performance Notes

- All queries use prepared statements (via `.bind()`)
- Indexes on foreign keys and frequently queried columns
- Transactions for multi-step operations (see `playlists::remove_track`)
- Connection pooling via `SqlitePool`
