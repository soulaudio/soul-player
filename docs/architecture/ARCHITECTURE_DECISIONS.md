# Architecture Decisions

## Overview

This document summarizes key architectural decisions made for Soul Player's multi-source design.

---

## Decision 1: SQLx Over Full ORM

**Date**: 2026-01-05

**Context**: Need to choose between SQLx, Diesel, and SeaORM for database access.

**Decision**: Use **SQLx** with raw SQL queries.

**Rationale**:
- ✅ Async-first (critical for Tauri + server)
- ✅ Compile-time checked queries (safety without ORM overhead)
- ✅ Works well with vertical slicing (each feature owns its queries)
- ✅ Lighter than full ORMs
- ✅ Better for explicit, debuggable SQL
- ✅ Built-in migration system with binary embedding

**Alternatives Considered**:
- **SeaORM**: Too heavy, adds abstraction that fights vertical slicing
- **Diesel**: Primarily synchronous (async in beta), more boilerplate

**Sources**:
- [A Guide to Rust ORMs in 2025 | Shuttle](https://www.shuttle.dev/blog/2024/01/16/best-orm-rust)
- [SQLx vs Diesel vs SeaORM Comparison](https://medium.com/@vishwajitpatil1224/3-rust-database-libraries-compared-sqlx-vs-diesel-vs-seaorm-4b978f96e1af)

---

## Decision 2: Vertical Slicing Over Repository Pattern

**Date**: 2026-01-05

**Context**: How to structure storage layer code.

**Decision**: Use **vertical slicing** instead of repository pattern.

**Structure**:
```
soul-storage/
  src/
    tracks/
      get_all.rs
      get_by_id.rs
      create.rs
    playlists/
      get_user_playlists.rs
      create_playlist.rs
```

**Rationale**:
- ✅ Each feature is self-contained (query + types + logic)
- ✅ Adding features doesn't touch existing code
- ✅ Each slice can optimize its own query
- ✅ No shared abstractions that couple features
- ✅ Easier to understand (explicit vs. abstracted)

**Trade-offs**:
- ❌ Some code duplication (acceptable for independence)
- ❌ Less DRY (but more explicit and maintainable)

**Alternatives Considered**:
- **Repository Pattern**: Creates shared abstractions that couple features together
- **Service Layer**: Similar issues to repository pattern

**Sources**:
- [Vertical Slice Architecture with Rust](https://github.com/akachida/vertical-slice-rust)
- [Vertical Slice Architecture | Milan Jovanović](https://www.milanjovanovic.tech/blog/vertical-slice-architecture)

---

## Decision 3: Trait-Based Storage Context

**Date**: 2026-01-05

**Context**: Support both local SQLite and remote server as data sources.

**Decision**: Define `StorageContext` trait with two implementations:
- `LocalStorageContext` - SQLite queries
- `RemoteStorageContext` - HTTP API calls

**Rationale**:
- ✅ Application code is source-agnostic
- ✅ Context always carries `user_id`
- ✅ Easy to add caching or hybrid modes later
- ✅ Testable (mock implementations)
- ✅ Clean dependency injection

**Example**:
```rust
#[async_trait]
pub trait StorageContext: Send + Sync {
    fn user_id(&self) -> UserId;
    async fn get_all_tracks(&self) -> Result<Vec<Track>>;
    // ... other operations
}
```

**Trade-offs**:
- Requires trait objects (`dyn StorageContext`)
- Small runtime cost (virtual dispatch)

**Alternatives Considered**:
- **Enum-based**: `Storage::Local | Storage::Remote` - more boilerplate at call sites
- **Feature flags**: Compile-time choice - can't switch sources at runtime

---

## Decision 4: Multi-Source Model

**Date**: 2026-01-05

**Context**: Users want to access music from multiple locations (local files, home server, cloud server).

**Decision**: Support **multiple sources** that coexist:
- Local source (always present, id=1)
- Multiple server sources (user-configured)
- One active server at a time
- Track availability tracked per source

**Schema**:
```sql
CREATE TABLE sources (...);
CREATE TABLE track_sources (
    track_id INTEGER,
    source_id INTEGER,
    status TEXT -- 'local_file', 'cached', 'stream_only'
);
```

**Rationale**:
- ✅ Flexible: add/remove servers as needed
- ✅ Offline-first: local files always work
- ✅ Deduplication: same track from multiple sources shows once
- ✅ Choice: stream vs. download per track

**Trade-offs**:
- More complex than single source
- Need sync protocol and conflict resolution

**Alternatives Considered**:
- **Single source toggle**: Either local OR server - too limiting
- **Cloud-only**: Doesn't work offline

---

## Decision 5: Offline-First with Sync Queue

**Date**: 2026-01-05

**Context**: App must work without internet connection.

**Decision**: Queue operations when offline, sync when reconnected.

**Implementation**:
```sql
CREATE TABLE offline_queue (
    operation TEXT,
    payload TEXT,
    retries INTEGER
);
```

**Rationale**:
- ✅ Never block user actions
- ✅ Graceful degradation
- ✅ Automatic retry on reconnect
- ✅ User sees pending changes

**Trade-offs**:
- Need conflict resolution (server changed while offline)
- Queue can grow large if offline for days

---

## Decision 6: Embedded Migrations

**Date**: 2026-01-05

**Context**: How to manage database migrations.

**Decision**: Use SQLx's `migrate!()` macro to embed migrations in binary.

**Implementation**:
```rust
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
```

**Rationale**:
- ✅ No runtime file dependencies
- ✅ Migrations always match code version
- ✅ Easy deployment (single binary)
- ✅ Automatic tracking via `_sqlx_migrations` table

**Build Script Required**:
```bash
sqlx migrate build-script
```
(Tells Cargo to rebuild when migrations change)

**Sources**:
- [SQLx migrate! macro documentation](https://docs.rs/sqlx/latest/sqlx/macro.migrate.html)
- [Database migrations with rocket and sqlx](https://wtjungle.com/blog/db-migrations-rocket-sqlx/)

---

## Decision 7: Content-Addressable Cache

**Date**: 2026-01-05

**Context**: Where to store downloaded tracks from servers.

**Decision**: Use content-addressable file storage.

**Structure**:
```
~/.soul-player/cache/tracks/{source_id}/{track_hash}.mp3
```

**Rationale**:
- ✅ Deduplication (same file = same hash)
- ✅ No filename conflicts
- ✅ Easy to verify integrity
- ✅ Can share cache across sources

**Hash Function**: SHA-256 of file content

**Trade-offs**:
- Requires rehashing on download
- Filenames not human-readable

**Alternatives Considered**:
- **Original filenames**: Conflicts and security issues
- **Database BLOB**: Slow, inflexible

---

## Decision 8: soul-import for Intelligent Import

**Date**: 2026-01-05

**Context**: Users import music with messy metadata.

**Decision**: Create `soul-import` crate with:
- Artist name normalization
- Fuzzy matching
- Proposal system (ask user to approve changes)

**Workflow**:
```
1. Scan files, read tags
2. Normalize ("the beatles" → "Beatles, The")
3. Match against existing artists (fuzzy)
4. Generate proposals ("Merge 'Beatles' and 'The Beatles'?")
5. User approves
6. Import with clean data
```

**Rationale**:
- ✅ Prevents duplicate artists
- ✅ Fixes capitalization automatically
- ✅ User retains control (proposals)
- ✅ Better than auto-merge (less risky)

**Trade-offs**:
- More complex import flow
- User must review proposals

---

## Decision 9: Deduplication via MusicBrainz + Fingerprinting

**Date**: 2026-01-05

**Context**: Same track may exist in multiple sources (local + server).

**Decision**: Use multi-stage deduplication:
1. MusicBrainz Recording ID (if available)
2. Acoustic fingerprint (Chromaprint)
3. Fuzzy metadata match (title + artist + duration)

**Rationale**:
- ✅ High accuracy (MusicBrainz is canonical)
- ✅ Fallback to fingerprint (works for untagged files)
- ✅ Last resort fuzzy match (catches most cases)

**Result**: User sees one track with multiple sources:
```
Song A
  🏠 Local: /music/song_a.mp3
  ☁️ Home Server: Available to stream
```

---

## Decision 10: Transaction Support via StorageContext

**Date**: 2026-01-05

**Context**: Some operations need atomicity (e.g., create artist + album + tracks).

**Decision**: Add optional transaction API to `StorageContext`:

```rust
async fn transaction<F, T>(&self, f: F) -> Result<T>
where
    F: FnOnce(&mut Transaction) -> Future<Output = Result<T>>;
```

**Rationale**:
- ✅ Needed for import operations (all-or-nothing)
- ✅ Prevents partial imports on error
- ✅ Keeps API simple (optional, not everywhere)

**Usage**:
```rust
storage.transaction(|tx| async {
    let artist_id = tx.create_artist(artist).await?;
    let album_id = tx.create_album(album, artist_id).await?;
    tx.create_tracks(tracks, album_id).await?;
    Ok(())
}).await?;
```

---

## Summary

These decisions prioritize:
- **Simplicity**: Explicit over clever
- **Offline-first**: Local-first architecture
- **Flexibility**: Multi-source support
- **Quality**: Meaningful tests, clean code
- **Performance**: Async, compile-time checks, efficient queries

## References

- [Multi-Source Architecture](./MULTI_SOURCE_ARCHITECTURE.md)
- [Sync Strategy](./SYNC_STRATEGY.md)
- [Offline Mode](./OFFLINE_MODE.md)
- [ROADMAP](../../ROADMAP.md)
