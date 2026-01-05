# Storage Layer Integration Tests

Comprehensive test suite for the soul-storage layer following Rust best practices.

## Test Organization

- `test_helpers.rs` - Test fixtures and database setup utilities
- `sources_tests.rs` - Multi-source functionality tests
- `artists_albums_tests.rs` - Artist and album CRUD tests
- `tracks_tests.rs` - Track operations with availability tracking
- `playlists_tests.rs` - Playlist system with permissions and ordering

## Test Philosophy

Following the project's testing philosophy from `docs/TESTING.md`:

- **Quality over quantity**: No shallow tests (getters, setters)
- **Real databases**: Uses file-based SQLite (NOT in-memory) via tempfile
- **Business logic focus**: Tests complex operations, edge cases, and integrations
- **Transaction correctness**: Verifies atomicity of multi-step operations
- **Constraint enforcement**: Tests foreign keys, unique constraints, cascades

## Running Tests

### Prerequisites

Set up DATABASE_URL for SQLx compile-time query checking:

```bash
# Create a temporary database for SQLx
cd libraries/soul-storage
mkdir -p .tmp
DATABASE_URL="sqlite://.tmp/sqlx-check.db" cargo sqlx database create
DATABASE_URL="sqlite://.tmp/sqlx-check.db" cargo sqlx migrate run
```

### Run all tests

```bash
# From workspace root
cargo test --package soul-storage

# With output
cargo test --package soul-storage -- --nocapture

# Specific test file
cargo test --package soul-storage --test sources_tests

# Single test
cargo test --package soul-storage test_create_local_source
```

### Generate offline mode cache

For CI/CD or offline development:

```bash
cd libraries/soul-storage
DATABASE_URL="sqlite://.tmp/sqlx-check.db" cargo sqlx prepare
```

This creates `.sqlx/query-*.json` files for offline compilation.

## Test Coverage

The test suite covers:

### Sources (12 tests)
- Creating local and server sources
- Active server management (only one active)
- Online/offline status tracking
- Constraint enforcement

### Artists & Albums (12 tests)
- Artist creation with sort names and MusicBrainz IDs
- Finding by name (case-sensitive)
- Album creation with artist relationships
- Proper ordering (sort_name, year DESC)
- Foreign key cascades (ON DELETE SET NULL)
- Unique constraint enforcement

### Tracks (18 tests)
- CRUD operations with denormalized data
- Multi-source availability tracking
- Filtering by artist, album, source
- Partial updates (dynamic SQL)
- Play history recording
- Statistics aggregation (play_count, skip_count)
- Recently played queries
- Transaction correctness (create + track_sources)
- Cascade deletes

### Playlists (18 tests)
- User ownership and permissions
- Track ordering and reordering
- Adding/removing tracks with position management
- Sharing with read/write permissions
- Permission enforcement (read-only cannot modify)
- Only owner can delete
- Favorite playlist sorting
- updated_at timestamp updates
- Transaction correctness (remove + reorder)
- Cascade deletes (playlist_tracks)

**Total: 60 meaningful integration tests**

## Design Decisions

### Why file-based SQLite?

In-memory SQLite (`:memory:`) behaves differently from production:
- No actual file I/O
- Different locking behavior
- Can't test migrations properly
- Different constraint enforcement timing

Using `tempfile::tempdir()` creates real database files that match production.

### Why no testcontainers for SQLite?

Testcontainers is designed for Docker containers (Postgres, MySQL, etc.).
For SQLite, we use `tempfile` which is:
- Faster (no container startup)
- Simpler (no Docker dependency)
- Cross-platform
- Sufficient for file-based databases

### Test Isolation

Each test gets a fresh database via `TestDb::new()`:
- No shared state between tests
- Tests can run in parallel
- Temp directories auto-cleanup on drop

### Transaction Testing

Tests verify atomicity:
- Create track + track_sources (must both succeed or both fail)
- Record play + update stats (transactional)
- Remove track from playlist + reorder (transactional)

These use `.begin()` / `.commit()` / `.rollback()` patterns.
