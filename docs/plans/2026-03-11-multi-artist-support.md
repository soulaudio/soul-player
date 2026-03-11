# Multi-Artist Support Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split multiple artists from audio file tags, store them in a junction table, and display all artists as individual clickable links throughout the app.

**Architecture:** Add a `track_artists(track_id, artist_id, position)` junction table mirroring the existing `track_genres` pattern. Keep `tracks.artist_id` pointing to the primary (position 0) artist for backward-compat. Fetch artists via a batch function after track queries; populate `Track.artists: Vec<TrackArtist>`. Update the import pipeline to split artist strings by common delimiters (`,`, `;`, ` feat. `, ` ft. `, ` & `, ` x `).

**Tech Stack:** SQLite + sqlx compile-time macros, Rust (soul-storage, soul-importer, soul-core), TypeScript/React (shared components)

---

## Context & Conventions

- `tracks.artist_id` stays as the primary-artist shortcut — no existing queries need immediate changes
- `sqlx::query!` macros require running `cargo sqlx prepare -- --lib` after any query change
- Multi-user rule: all queries include `user_id` where applicable (artists/tracks are shared across users in current schema — no user_id on artists/tracks table itself, only on joins; this is unchanged)
- After each Rust change run `cargo clippy -p <crate> -- -D warnings` to catch issues early
- CLAUDE.md: all strings localized, no `println!`, structured logging via `tracing`

---

### Task 1: Database Migration — track_artists junction table

**Files:**
- Create: `libraries/soul-storage/migrations/20260311000001_create_track_artists.sql`

**Step 1: Write the migration**

```sql
-- Create track_artists junction table (many-to-many, ordered by position)
CREATE TABLE IF NOT EXISTS track_artists (
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    artist_id INTEGER NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,  -- 0 = primary artist
    PRIMARY KEY (track_id, artist_id)
);

CREATE INDEX IF NOT EXISTS idx_track_artists_track_id ON track_artists(track_id);
CREATE INDEX IF NOT EXISTS idx_track_artists_artist_id ON track_artists(artist_id);
```

**Step 2: Run migration**

```bash
cd libraries/soul-storage
sqlx migrate run --source migrations
```

Expected: `Applied 20260311000001/migrate create track artists`

**Step 3: Commit**

```bash
git add libraries/soul-storage/migrations/20260311000001_create_track_artists.sql
git commit -m "feat: add track_artists junction table for multi-artist support"
```

---

### Task 2: Add TrackArtist type to soul-core

**Files:**
- Modify: `libraries/soul-core/src/types/multisource_track.rs`
- Modify: `libraries/soul-core/src/types/mod.rs` (re-export if needed)

**Step 1: Add `TrackArtist` struct and `artists` field to `Track`**

In `multisource_track.rs`, add after the existing imports:

```rust
/// A single artist associated with a track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackArtist {
    pub id: ArtistId,
    pub name: String,
}
```

Then in the `Track` struct, add after `artist_name`:

```rust
/// All artists for this track (populated from track_artists junction; empty = not loaded)
#[serde(skip_serializing_if = "Vec::is_empty", default)]
pub artists: Vec<TrackArtist>,
```

**Step 2: Build to verify no compile errors**

```bash
cargo build -p soul-core 2>&1 | tail -5
```

Expected: `Finished` with no errors.

**Step 3: Commit**

```bash
git add libraries/soul-core/src/types/multisource_track.rs
git commit -m "feat: add TrackArtist type and artists field to Track"
```

---

### Task 3: Storage layer — add_artist_to_track and batch fetch

**Files:**
- Modify: `libraries/soul-storage/src/artists/mod.rs`
- Create new module or extend existing: `libraries/soul-storage/src/track_artists.rs`
- Modify: `libraries/soul-storage/src/lib.rs` (add pub mod track_artists)

**Step 1: Create `libraries/soul-storage/src/track_artists.rs`**

```rust
use soul_core::{error::Result, types::{ArtistId, TrackArtist, TrackId}};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Link an artist to a track at a given position.
/// Silently ignores duplicate (track_id, artist_id) pairs (ON CONFLICT DO NOTHING).
pub async fn add_to_track(
    pool: &SqlitePool,
    track_id: TrackId,
    artist_id: ArtistId,
    position: i64,
) -> Result<()> {
    sqlx::query!(
        "INSERT OR IGNORE INTO track_artists (track_id, artist_id, position)
         VALUES (?, ?, ?)",
        track_id,
        artist_id,
        position
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Remove all artist associations for a track (call before re-importing).
pub async fn clear_for_track(pool: &SqlitePool, track_id: TrackId) -> Result<()> {
    sqlx::query!(
        "DELETE FROM track_artists WHERE track_id = ?",
        track_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch all artists for multiple tracks in one query.
/// Returns a map from track_id to ordered Vec<TrackArtist>.
pub async fn get_for_tracks(
    pool: &SqlitePool,
    track_ids: &[TrackId],
) -> Result<HashMap<TrackId, Vec<TrackArtist>>> {
    if track_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // Build IN clause dynamically (sqlx doesn't support Vec binding in query!)
    let placeholders = track_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT ta.track_id, ta.artist_id as 'artist_id!', ar.name
         FROM track_artists ta
         JOIN artists ar ON ar.id = ta.artist_id
         WHERE ta.track_id IN ({})
         ORDER BY ta.track_id, ta.position",
        placeholders
    );

    let mut query = sqlx::query_as::<_, (String, i64, String)>(&sql);
    for id in track_ids {
        query = query.bind(id.as_str());
    }

    let rows = query.fetch_all(pool).await?;

    let mut map: HashMap<TrackId, Vec<TrackArtist>> = HashMap::new();
    for (track_id_str, artist_id, name) in rows {
        let track_id = TrackId::new(track_id_str);
        map.entry(track_id).or_default().push(TrackArtist {
            id: artist_id,
            name,
        });
    }
    Ok(map)
}
```

**Note:** `get_for_tracks` uses raw `query_as` (not the compile-time macro) because the IN list is dynamic. This is an accepted exception per CLAUDE.md since the query is constructed safely from typed data (no user strings interpolated).

**Step 2: Export from `libraries/soul-storage/src/lib.rs`**

Add `pub mod track_artists;` alongside the other mod declarations.

**Step 3: Update `get_track_counts` in `artists/mod.rs` to include junction artists**

Replace the existing `get_track_counts` function:

```rust
/// Get track counts for all artists (including featured/non-primary artists via junction)
pub async fn get_track_counts(
    pool: &SqlitePool,
) -> Result<std::collections::HashMap<ArtistId, i32>> {
    let rows: Vec<_> = sqlx::query!(
        "SELECT artist_id as 'artist_id!', COUNT(DISTINCT track_id) as count
         FROM track_artists
         GROUP BY artist_id"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.artist_id, row.count as i32))
        .collect())
}
```

**Step 4: Run sqlx prepare to regenerate compile-time query cache**

```bash
cd libraries/soul-storage
cargo sqlx prepare -- --lib
```

Expected: New `.sqlx/query-*.json` files generated; existing ones may be removed if queries changed.

**Step 5: Build**

```bash
cargo build -p soul-storage 2>&1 | tail -5
```

**Step 6: Commit**

```bash
git add libraries/soul-storage/src/track_artists.rs libraries/soul-storage/src/lib.rs libraries/soul-storage/src/artists/mod.rs libraries/soul-storage/.sqlx/
git commit -m "feat: add track_artists storage functions (add, clear, batch fetch)"
```

---

### Task 4: Artist page — query via junction table

**Files:**
- Modify: `libraries/soul-storage/src/artists/mod.rs` — `get_by_artist` function

**Context:** Currently `get_by_artist` queries `WHERE t.artist_id = ?`. This misses tracks where the artist is featured (not primary). After this change, querying `track_artists` returns all tracks the artist appears on.

**Step 1: Update `get_by_artist` (find its exact signature/query in artists/mod.rs or tracks/mod.rs)**

Search for the function:

```bash
grep -n "get_by_artist\|get_tracks_by_artist" libraries/soul-storage/src/ -r
```

Then update the WHERE clause from:
```sql
WHERE t.artist_id = ?
```

To use the junction:
```sql
JOIN track_artists ta ON ta.track_id = t.id
WHERE ta.artist_id = ?
```

Full updated query pattern:
```sql
SELECT t.id as 'id!', t.title as 'title!', t.artist_id, ...
FROM tracks t
JOIN track_artists ta ON ta.track_id = t.id
LEFT JOIN artists ar ON t.artist_id = ar.id
LEFT JOIN albums al ON t.album_id = al.id
WHERE ta.artist_id = ?
ORDER BY al.title COLLATE NOCASE, t.disc_number, t.track_number
```

**Step 2: Run sqlx prepare**

```bash
cd libraries/soul-storage && cargo sqlx prepare -- --lib
```

**Step 3: Build and verify**

```bash
cargo build -p soul-storage 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add libraries/soul-storage/src/ libraries/soul-storage/.sqlx/
git commit -m "feat: artist page shows all tracks (primary + featured) via track_artists junction"
```

---

### Task 5: Metadata parsing — split artist string into Vec<String>

**Files:**
- Modify: `libraries/soul-importer/src/metadata.rs`

**Step 1: Change `ExtractedMetadata.artist` from `Option<String>` to `Vec<String>`**

In the `ExtractedMetadata` struct, replace:
```rust
/// Artist name
pub artist: Option<String>,
```

With:
```rust
/// Artist names (split from tag; may be multiple)
pub artists: Vec<String>,
```

Update `is_sparse`:
```rust
pub fn is_sparse(&self) -> bool {
    self.artists.is_empty() && self.album.is_none() && self.genres.is_empty()
}
```

**Step 2: Add `split_artists` helper function**

Add before `extract_metadata`:

```rust
/// Split a raw artist tag string into individual artist names.
///
/// Handles common delimiters used in music metadata:
/// - `,` and `;` — Vorbis/FLAC multi-value, ID3 separation
/// - ` feat. `, ` feat `, ` ft. `, ` ft ` — featuring credits
/// - ` & ` — collaborative tracks ("Artist A & Artist B")
/// - ` x ` — DJ/electronic collab notation (lowercase x with spaces)
pub fn split_artists(raw: &str) -> Vec<String> {
    // Use a regex-free approach: iteratively split by each delimiter.
    // Order matters: check longer/more-specific patterns first to avoid
    // splitting "feat." inside a longer token.
    let tokens = vec![raw.to_string()];

    // Delimiters in order of specificity (longest first)
    let delimiters: &[&str] = &[
        " feat. ", " feat ", " ft. ", " ft ",
        " & ", " x ",
        ",", ";",
    ];

    let mut results = tokens;
    for delim in delimiters {
        results = results
            .into_iter()
            .flat_map(|s| {
                s.split(delim)
                    .map(|p| p.trim().to_string())
                    .collect::<Vec<_>>()
            })
            .collect();
    }

    results.into_iter().filter(|s| !s.is_empty()).collect()
}
```

**Step 3: Update `extract_metadata` to use `split_artists`**

Find where artist is extracted (around line 271):
```rust
let artist = tag.artist().map(|s| s.to_string());
```

Replace with:
```rust
let artists: Vec<String> = tag
    .artist()
    .map(|s| split_artists(&fix_mojibake(s.to_string())))
    .unwrap_or_default();
```

Then update the ExtractedMetadata construction:
```rust
// Change: artist: artist.map(fix_mojibake),
// To:
artists,
```

Also update the folder fallback that sets `artist` — it should set `artists: vec![artist_name]` if parsed from folder.

**Step 4: Write unit tests**

In `metadata.rs` tests module:

```rust
#[test]
fn test_split_artists_single() {
    assert_eq!(split_artists("Skinshape"), vec!["Skinshape"]);
}

#[test]
fn test_split_artists_feat() {
    assert_eq!(
        split_artists("Skinshape feat. Wu-Lu"),
        vec!["Skinshape", "Wu-Lu"]
    );
}

#[test]
fn test_split_artists_comma() {
    assert_eq!(
        split_artists("Artist A, Artist B, Artist C"),
        vec!["Artist A", "Artist B", "Artist C"]
    );
}

#[test]
fn test_split_artists_ampersand() {
    assert_eq!(
        split_artists("Bonobo & Erykah Badu"),
        vec!["Bonobo", "Erykah Badu"]
    );
}

#[test]
fn test_split_artists_mixed() {
    assert_eq!(
        split_artists("Madlib feat. Guilty Simpson & MED"),
        vec!["Madlib", "Guilty Simpson", "MED"]
    );
}

#[test]
fn test_split_artists_no_false_positive_hyphen() {
    // Hyphens within names must NOT be split
    assert_eq!(split_artists("Wu-Tang Clan"), vec!["Wu-Tang Clan"]);
}
```

**Step 5: Run tests**

```bash
cargo test -p soul-importer split_artists -- --nocapture
```

Expected: all 6 tests pass.

**Step 6: Fix all compile errors** caused by the `artist → artists` rename throughout soul-importer (check `metadata_extractor.rs`, `file_processor.rs`).

```bash
cargo build -p soul-importer 2>&1 | grep "^error"
```

**Step 7: Commit**

```bash
git add libraries/soul-importer/src/metadata.rs
git commit -m "feat: split multi-artist tags into Vec<String> with common delimiter support"
```

---

### Task 6: metadata_extractor.rs — resolve Vec<ArtistId> and persist to junction

**Files:**
- Modify: `libraries/soul-importer/src/metadata_extractor.rs`

**Step 1: Update `ProcessedMetadata`**

Replace:
```rust
pub artist_id: Option<ArtistId>,
```

With:
```rust
/// All resolved artist IDs (position = index in vec; [0] = primary)
pub artist_ids: Vec<ArtistId>,
```

**Step 2: Update `extract_and_match` to resolve all artists**

Replace the single-artist block:
```rust
// Fuzzy match artist
let artist_id = if let Some(ref artist_name) = raw.artist {
    ...
    Some(artist_match.entity.id)
} else {
    None
};
```

With:
```rust
// Resolve all artists
let mut artist_ids: Vec<ArtistId> = Vec::new();
for artist_name in &raw.artists {
    let artist_match = self
        .fuzzy_matcher
        .find_or_create_artist(pool, artist_name)
        .await?;
    artist_ids.push(artist_match.entity.id);
}
let artist_id = artist_ids.first().copied();  // primary artist for tracks.artist_id
```

Then update the `album_artist_id` block (it uses `raw.artist` → use `raw.artists.first()`):
```rust
let album_artist_id = if let Some(ref album_artist_name) = raw.album_artist {
    if raw.artists.first().map(String::as_str) != Some(album_artist_name.as_str()) {
        ...
    } else {
        artist_id
    }
} else {
    None
};
```

Update `ProcessedMetadata` construction:
```rust
Ok(ProcessedMetadata {
    raw,
    artist_ids,        // was: artist_id
    album_id,
    album_artist_id,
    genre_ids,
})
```

**Step 3: Add `add_artists_to_track` method** (mirrors `add_genres_to_track`):

```rust
pub async fn add_artists_to_track(
    &self,
    pool: &SqlitePool,
    track_id: TrackId,
    artist_ids: &[ArtistId],
) -> Result<()> {
    // Clear existing associations first (for re-import)
    soul_storage::track_artists::clear_for_track(pool, track_id.clone()).await?;
    for (position, &artist_id) in artist_ids.iter().enumerate() {
        soul_storage::track_artists::add_to_track(
            pool,
            track_id.clone(),
            artist_id,
            position as i64,
        )
        .await?;
    }
    Ok(())
}
```

**Step 4: Build**

```bash
cargo build -p soul-importer 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add libraries/soul-importer/src/metadata_extractor.rs
git commit -m "feat: resolve Vec<ArtistId> in metadata extractor and expose add_artists_to_track"
```

---

### Task 7: file_processor.rs — write artists to junction on import/update

**Files:**
- Modify: `libraries/soul-importer/src/file_processor.rs`

**Step 1: Update `import_new_file` to use `processed.artist_ids`**

Change:
```rust
artist_id: processed.artist_id,
```
To:
```rust
artist_id: processed.artist_ids.first().copied(),
```

After `add_genres_to_track`, add:
```rust
// Add all artists to track_artists junction
self.metadata_extractor
    .add_artists_to_track(self.pool, track_id_typed.clone(), &processed.artist_ids)
    .await?;
```

**Step 2: Update `update_track_metadata`** similarly:

After updating genres, add:
```rust
let track_id_typed = TrackId::new(track_id.to_string());
self.metadata_extractor
    .add_artists_to_track(self.pool, track_id_typed, &processed.artist_ids)
    .await?;
```

**Step 3: Build**

```bash
cargo build -p soul-importer 2>&1 | tail -5
```

**Step 4: Commit**

```bash
git add libraries/soul-importer/src/file_processor.rs
git commit -m "feat: write all artists to track_artists junction during import and update"
```

---

### Task 8: Populate Track.artists in track queries (storage layer)

**Files:**
- Modify: `libraries/soul-storage/src/tracks/mod.rs`

The goal is to populate `Track.artists` after fetching tracks. The simplest approach: create a helper that enriches a `Vec<Track>` with artist data.

**Step 1: Add `populate_artists` helper in `track_artists.rs`**

```rust
use soul_core::types::Track;

/// Populate the `artists` field on a collection of tracks from the junction table.
pub async fn populate_for_tracks(pool: &SqlitePool, tracks: &mut Vec<Track>) -> Result<()> {
    let ids: Vec<TrackId> = tracks.iter().map(|t| t.id.clone()).collect();
    let mut map = get_for_tracks(pool, &ids).await?;
    for track in tracks.iter_mut() {
        if let Some(artists) = map.remove(&track.id) {
            track.artists = artists;
        }
    }
    Ok(())
}
```

**Step 2: Call `populate_for_tracks` from `get_all` in tracks/mod.rs**

Locate `pub async fn get_all(...)` in `soul-storage/src/tracks/mod.rs`. After the fetch:

```rust
let mut tracks = sqlx::query_as!(...).fetch_all(pool).await?;
soul_storage::track_artists::populate_for_tracks(pool, &mut tracks).await?;
Ok(tracks)
```

Apply the same pattern to:
- `get_by_artist`
- `get_by_album`
- `get_by_playlist`
- `get_by_genre`
- Any other `fetch_all` returning `Vec<Track>`

Use `grep -n "fetch_all\|fetch_one\|fetch_optional" libraries/soul-storage/src/tracks/mod.rs` to find all call sites.

**Step 3: Run sqlx prepare**

```bash
cd libraries/soul-storage && cargo sqlx prepare -- --lib
```

**Step 4: Build**

```bash
cargo build -p soul-storage 2>&1 | tail -5
```

**Step 5: Run Rust tests**

```bash
cargo test -p soul-importer -- --test-threads=1 2>&1 | tail -20
```

**Step 6: Commit**

```bash
git add libraries/soul-storage/src/ libraries/soul-storage/.sqlx/
git commit -m "feat: populate Track.artists from junction table in all track queries"
```

---

### Task 9: TypeScript types — update Track interface and BackendContext

**Files:**
- Modify: `applications/shared/src/components/TrackList.tsx`
- Modify: `applications/shared/src/contexts/BackendContext.tsx`
- Modify: `applications/desktop/src/providers/TauriBackendProvider.tsx`

**Step 1: Add `TrackArtist` type in BackendContext or a shared types file**

In `BackendContext.tsx`, find the `Track` type definition and add:

```typescript
export interface TrackArtist {
  id: number
  name: string
}
```

Add to `Track` interface:
```typescript
artists?: TrackArtist[]  // All artists (populated when available)
```

**Step 2: Update `TrackList.tsx` `Track` interface**

Add to the local `Track` interface:
```typescript
artists?: Array<{ id: number; name: string }>
```

**Step 3: Update TauriBackendProvider mappings**

Find where tracks are mapped from Tauri responses. In the track mapping function, add:
```typescript
artists: t.artists ?? [],
```

**Step 4: TypeScript check**

```bash
yarn workspace @soul-player/desktop tsc --noEmit 2>&1 | tail -20
```

Fix any type errors.

**Step 5: Commit**

```bash
git add applications/shared/src/contexts/BackendContext.tsx applications/shared/src/components/TrackList.tsx applications/desktop/src/providers/TauriBackendProvider.tsx
git commit -m "feat: add TrackArtist type and artists[] field to frontend Track interfaces"
```

---

### Task 10: ArtistLinks component — render multiple clickable artists

**Files:**
- Modify: `applications/shared/src/components/ArtistLink.tsx`

**Step 1: Add `ArtistLinks` component** (handles zero, one, or many artists):

```tsx
interface ArtistLinksProps {
  /** Preferred: full artist list from Track.artists */
  artists?: Array<{ id: number; name: string }>
  /** Fallback: single artist (backward compat) */
  artistId?: number
  artistName?: string
  className?: string
}

export function ArtistLinks({
  artists,
  artistId,
  artistName,
  className = '',
}: ArtistLinksProps) {
  // Prefer junction-sourced artists; fall back to single artist
  const list: Array<{ id?: number; name: string }> =
    artists && artists.length > 0
      ? artists
      : artistId || artistName
        ? [{ id: artistId, name: artistName ?? '' }]
        : []

  if (list.length === 0) {
    return <span className={className}>Unknown Artist</span>
  }

  return (
    <span className={className}>
      {list.map((a, i) => (
        <span key={a.id ?? a.name}>
          <ArtistLink artistId={a.id} artistName={a.name} />
          {i < list.length - 1 && (
            <span className="text-muted-foreground">, </span>
          )}
        </span>
      ))}
    </span>
  )
}
```

**Step 2: Export `ArtistLinks` from the shared package index**

In `applications/shared/src/index.ts`, add:
```typescript
export { ArtistLinks } from './components/ArtistLink'
```

**Step 3: TypeScript check**

```bash
yarn workspace @soul-player/shared tsc --noEmit 2>&1 | tail -10
```

**Step 4: Commit**

```bash
git add applications/shared/src/components/ArtistLink.tsx applications/shared/src/index.ts
git commit -m "feat: add ArtistLinks component for displaying multiple clickable artist links"
```

---

### Task 11: TrackList — use ArtistLinks instead of ArtistLink

**Files:**
- Modify: `applications/shared/src/components/TrackList.tsx`

**Step 1: Import `ArtistLinks`**

```typescript
import { ArtistLinks } from './ArtistLink'
```

**Step 2: Find all `<ArtistLink` usages in TrackList.tsx**

```bash
grep -n "ArtistLink" applications/shared/src/components/TrackList.tsx
```

**Step 3: Replace each usage** (around line 438-446 per grep results):

Change:
```tsx
<ArtistLink
  artistId={activeVersion.artistId}
  artistName={activeVersion.artist}
/>
```

To:
```tsx
<ArtistLinks
  artists={activeVersion.artists}
  artistId={activeVersion.artistId}
  artistName={activeVersion.artist}
/>
```

**Step 4: TypeScript check**

```bash
yarn workspace @soul-player/shared tsc --noEmit 2>&1 | tail -10
```

**Step 5: Check other pages that display artist names** (AlbumPage, ArtistPage, NowPlayingPage, etc.) — replace `ArtistLink` with `ArtistLinks` where tracks are listed.

```bash
grep -rn "ArtistLink" applications/shared/src/pages/ applications/desktop/src/pages/
```

Update each found usage.

**Step 6: TypeScript check (full)**

```bash
yarn workspace @soul-player/desktop tsc --noEmit && yarn workspace @soul-player/shared tsc --noEmit
```

**Step 7: Commit**

```bash
git add applications/shared/src/components/TrackList.tsx applications/shared/src/pages/
git commit -m "feat: use ArtistLinks in TrackList and pages to display all artists"
```

---

### Task 12: Final — sqlx prepare, full build, and precommit

**Step 1: Regenerate sqlx query cache**

```bash
cd libraries/soul-storage && cargo sqlx prepare -- --lib
cd ../..
```

**Step 2: Full Rust build**

```bash
cargo build --workspace 2>&1 | tail -10
```

**Step 3: TypeScript build**

```bash
yarn build 2>&1 | tail -10
```

**Step 4: Run unit tests**

```bash
cargo test -p soul-importer -- --test-threads=1 2>&1 | tail -20
```

**Step 5: Commit any remaining sqlx files**

```bash
git add libraries/soul-storage/.sqlx/
git status
git commit -m "chore: regenerate sqlx query cache for multi-artist support" --allow-empty
```

**Step 6: Check TypeScript lint**

```bash
cd applications/shared && yarn lint && cd ../desktop && yarn lint
```

---

## Rollout Notes

- **Existing library data:** tracks imported before this change have `track_artists` empty. Users need to rescan/re-import their library to populate the junction table. No automatic backfill is done — the scanner's update path handles this on next scan.
- **Backward compat:** `tracks.artist_id` (primary artist) is unchanged, so all existing queries that read `artist_name` from the JOIN still work even before `track_artists` is populated.
- **Album grouping:** the existing `album_artist_id.or(artist_id)` logic for album matching is unchanged; only the first (primary) artist is used for album grouping, which is correct.
