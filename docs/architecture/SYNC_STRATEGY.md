# Sync Strategy

## Overview

Soul Player synchronizes music libraries between:
- **Local SQLite database** (always available)
- **Remote servers** (when online)

Sync is **bidirectional** with conflict resolution and works offline-first.

---

## Sync Types

### 1. Metadata Sync (Lightweight)

Syncs **track information only** (no files):
- Track metadata (title, artist, album, etc.)
- Playlists
- Play counts
- Favorites

**Use case**: Browse server library without downloading files.

**Data transfer**: ~1-5 KB per track (JSON metadata only)

### 2. Full Sync (Download Files)

Downloads **actual audio files** for offline use:
- All metadata (same as above)
- Audio files to local cache
- Album art

**Use case**: Make tracks available offline.

**Data transfer**: ~5-50 MB per track (depends on quality)

---

## Sync Modes

### Auto Sync (Default)

```
Triggers:
- App startup (if online)
- Every 30 minutes (if online)
- When server comes back online
- After user makes changes (playlist edits, etc.)

What syncs:
- Metadata only
- Recently played tracks (auto-cache)
- User's playlists
```

### Manual Sync

```
User triggers:
- Pull-to-refresh in UI
- "Sync Now" button in settings

What syncs:
- Full metadata refresh
- Optionally: Download new tracks
```

### Selective Download

```
User chooses:
- Individual tracks
- Entire playlists
- Entire albums
- Smart filters (e.g., "5-star rated tracks")

What syncs:
- Metadata + audio files
- Files stored in cache with 'cached' status
```

---

## Sync Protocol

### Initial Sync (First Time Connecting to Server)

```rust
async fn initial_sync(server: &Source) -> Result<SyncResult> {
    // 1. Authenticate
    let token = authenticate(server).await?;

    // 2. Fetch server info
    let server_info = fetch_server_info(server).await?;
    println!("Syncing from: {} (v{})", server_info.name, server_info.version);

    // 3. Fetch all entities in batches
    let mut cursor = None;
    let mut total_tracks = 0;

    loop {
        // Fetch batch of tracks (100 at a time)
        let response = fetch_tracks_batch(server, cursor).await?;

        // Insert into local DB
        for track in response.tracks {
            insert_track_metadata(track, server.id).await?;
        }

        total_tracks += response.tracks.len();

        // Check if more pages
        if let Some(next_cursor) = response.next_cursor {
            cursor = Some(next_cursor);
        } else {
            break;
        }
    }

    // 4. Save sync state
    save_sync_state(server.id, "tracks", total_tracks).await?;

    Ok(SyncResult {
        tracks_synced: total_tracks,
        playlists_synced: 0,
        downloaded_files: 0,
    })
}
```

### Incremental Sync (Subsequent Syncs)

```rust
async fn incremental_sync(server: &Source) -> Result<SyncResult> {
    // 1. Get last sync cursor
    let last_sync = get_sync_state(server.id, "tracks").await?;
    let cursor = last_sync.cursor;

    // 2. Fetch only new/updated tracks since last sync
    let response = fetch_tracks_since(server, cursor).await?;

    // 3. Update local DB
    for track in response.new_tracks {
        insert_track_metadata(track, server.id).await?;
    }

    for track in response.updated_tracks {
        update_track_metadata(track).await?;
    }

    for track_id in response.deleted_tracks {
        mark_track_deleted(track_id, server.id).await?;
    }

    // 4. Update sync state
    save_sync_state(server.id, "tracks", response.new_cursor).await?;

    Ok(SyncResult {
        tracks_synced: response.new_tracks.len(),
        tracks_updated: response.updated_tracks.len(),
        tracks_deleted: response.deleted_tracks.len(),
    })
}
```

---

## Server API Endpoints

### GET /api/sync/tracks

**Initial sync (no cursor):**
```http
GET /api/sync/tracks?limit=100
Authorization: Bearer <token>

Response:
{
  "tracks": [
    {
      "id": "track_123",
      "title": "Song Title",
      "artist": "Artist Name",
      "album": "Album Name",
      // ... full metadata
      "file_hash": "sha256:abc123...",  // For deduplication
      "updated_at": "2024-01-15T10:30:00Z"
    }
  ],
  "next_cursor": "eyJpZCI6MTIzLCJ0cyI6MTcwNTMxNjYwMH0=",
  "has_more": true
}
```

**Incremental sync (with cursor):**
```http
GET /api/sync/tracks?cursor=eyJpZCI6MTIzLCJ0cyI6MTcwNTMxNjYwMH0=&limit=100
Authorization: Bearer <token>

Response:
{
  "new_tracks": [...],      // Tracks created since cursor
  "updated_tracks": [...],  // Tracks modified since cursor
  "deleted_tracks": ["track_456", "track_789"],
  "next_cursor": "eyJpZCI6NDU2LCJ0cyI6MTcwNTQwMzAwMH0=",
  "has_more": false
}
```

### GET /api/tracks/{id}/stream

```http
GET /api/tracks/track_123/stream
Authorization: Bearer <token>
Range: bytes=0-1048576  // For resumable streaming

Response:
206 Partial Content
Content-Range: bytes 0-1048576/5242880
Content-Type: audio/mpeg

<binary audio data>
```

### GET /api/tracks/{id}/download

```http
GET /api/tracks/track_123/download
Authorization: Bearer <token>

Response:
200 OK
Content-Type: audio/mpeg
Content-Disposition: attachment; filename="artist_-_title.mp3"
Content-Length: 5242880

<full audio file>
```

---

## Conflict Resolution

### Scenario 1: Same Track Edited on Both Client and Server

```
Local:  Track "Song A" - Play Count: 5, Last Played: 2024-01-15
Server: Track "Song A" - Play Count: 3, Last Played: 2024-01-14

Resolution: Merge
- Use server's metadata (source of truth)
- Keep local play count (client-side stat)
- Sync local play count to server
```

### Scenario 2: Playlist Edited Offline

```
User adds tracks to playlist while offline
→ Queue changes in sync_queue table
→ When back online, push changes to server
→ Server validates and accepts
→ If conflict (playlist deleted on server), ask user
```

### Scenario 3: Track Deleted on Server

```
Server deletes track
→ Incremental sync detects deletion
→ Mark track_sources.status = 'unavailable' for that server
→ Keep local file if cached
→ If track only existed on that server, hide from library
```

---

## Deduplication Strategy

### Problem: Same Track from Multiple Sources

User imports "Song A.mp3" locally, then syncs from server that also has "Song A.mp3". We don't want duplicates in the UI.

### Solution: Fingerprinting + Metadata Matching

```rust
async fn deduplicate_track(new_track: &Track, source_id: SourceId) -> Result<()> {
    // 1. Check MusicBrainz ID (if available)
    if let Some(mb_id) = &new_track.musicbrainz_recording_id {
        if let Some(existing) = find_track_by_musicbrainz(mb_id).await? {
            // Same recording - add new source to existing track
            add_track_source(existing.id, source_id, new_track).await?;
            return Ok(());
        }
    }

    // 2. Check acoustic fingerprint (if available)
    if let Some(fingerprint) = &new_track.fingerprint {
        if let Some(existing) = find_track_by_fingerprint(fingerprint).await? {
            add_track_source(existing.id, source_id, new_track).await?;
            return Ok(());
        }
    }

    // 3. Fuzzy metadata match
    let similar = find_similar_tracks(
        &new_track.title,
        &new_track.artist,
        &new_track.album,
        new_track.duration_seconds
    ).await?;

    if let Some(existing) = similar.first() {
        if similarity_score(existing, new_track) > 0.9 {
            // Ask user to confirm merge
            let confirmed = ask_user_merge_tracks(existing, new_track).await?;
            if confirmed {
                add_track_source(existing.id, source_id, new_track).await?;
                return Ok(());
            }
        }
    }

    // 4. No match - create new track
    create_new_track(new_track, source_id).await?;
    Ok(())
}
```

**Result**: User sees one "Song A" in library with multiple sources:
```
Song A
  🏠 Local File: /music/song_a.mp3
  ☁️ Home Server: Available to stream
```

---

## Download Queue Management

### Background Download Worker

```rust
struct DownloadWorker {
    queue: Arc<Mutex<VecDeque<DownloadTask>>>,
    storage: Arc<dyn StorageContext>,
    max_concurrent: usize,
}

struct DownloadTask {
    track_id: TrackId,
    source_id: SourceId,
    priority: u8,  // User-requested = 100, auto-cache = 10
}

impl DownloadWorker {
    async fn run(&self) {
        loop {
            // Get next batch of tasks
            let tasks = self.queue.lock().await.drain(..self.max_concurrent).collect();

            // Download concurrently
            let handles: Vec<_> = tasks.iter().map(|task| {
                self.download_track(task)
            }).collect();

            futures::future::join_all(handles).await;

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn download_track(&self, task: &DownloadTask) -> Result<()> {
        let source = self.storage.get_source(task.source_id).await?;
        let client = ServerClient::new(&source);

        // Get track info
        let track = self.storage.get_track_by_id(task.track_id).await?;

        // Determine cache path
        let cache_path = get_cache_path(&track, task.source_id);

        // Download with progress tracking
        client.download_track(&track.server_path, &cache_path).await?;

        // Update DB
        update_track_source_status(
            task.track_id,
            task.source_id,
            AvailabilityStatus::Cached,
            cache_path
        ).await?;

        // Emit event for UI update
        emit_download_complete(task.track_id);

        Ok(())
    }
}
```

### Smart Download Strategies

**Strategy 1: Recently Played**
```sql
-- Auto-download tracks played in last 7 days
SELECT track_id
FROM play_history
WHERE user_id = ?
  AND played_at > datetime('now', '-7 days')
  AND track_id NOT IN (
      SELECT track_id FROM track_sources
      WHERE status IN ('local_file', 'cached')
  )
LIMIT 50;
```

**Strategy 2: High Play Count**
```sql
-- Auto-download frequently played tracks
SELECT track_id
FROM track_stats
WHERE play_count > 5
  AND track_id NOT IN (
      SELECT track_id FROM track_sources
      WHERE status IN ('local_file', 'cached')
  )
ORDER BY play_count DESC
LIMIT 100;
```

**Strategy 3: User Playlists**
```sql
-- Download all tracks in user's playlists
SELECT DISTINCT track_id
FROM playlist_tracks
WHERE playlist_id IN (
    SELECT id FROM playlists WHERE owner_id = ?
)
AND track_id NOT IN (
    SELECT track_id FROM track_sources
    WHERE status IN ('local_file', 'cached')
);
```

---

## Bandwidth & Performance

### Throttling

```rust
pub struct DownloadThrottler {
    max_bandwidth: u64,  // bytes per second
    tokens: Arc<Mutex<u64>>,
}

impl DownloadThrottler {
    async fn acquire(&self, bytes: u64) {
        let mut tokens = self.tokens.lock().await;

        while *tokens < bytes {
            // Refill tokens
            tokio::time::sleep(Duration::from_millis(100)).await;
            *tokens = (*tokens + self.max_bandwidth / 10).min(self.max_bandwidth);
        }

        *tokens -= bytes;
    }
}
```

### Adaptive Streaming Quality

```rust
// Measure connection speed
let speed = measure_download_speed().await?;

let quality = match speed {
    0..=500_000 => StreamQuality::Low,      // < 500 KB/s
    500_001..=2_000_000 => StreamQuality::Medium,  // 500 KB/s - 2 MB/s
    _ => StreamQuality::High,               // > 2 MB/s
};

stream_track(track_id, quality).await?;
```

---

## Sync UI Feedback

### Progress Indicators

```
Syncing from Home Server...
├─ Tracks: 1,234 / 5,000 (24%)
├─ Playlists: 12 / 12 (100%)
└─ Download Queue: 45 pending

[Cancel]
```

### Sync Status Badge

```
🔄 Syncing...
✅ Synced (2 mins ago)
⚠️ Sync Failed (Retry?)
🔌 Offline
```

---

## Error Handling

### Transient Errors (Retry)

```rust
#[derive(Debug)]
enum SyncError {
    NetworkTimeout,
    ServerUnavailable,
    RateLimited,
}

async fn sync_with_retry(server: &Source) -> Result<SyncResult> {
    let mut retries = 0;
    let max_retries = 3;

    loop {
        match sync(server).await {
            Ok(result) => return Ok(result),

            Err(SyncError::NetworkTimeout | SyncError::ServerUnavailable) => {
                if retries < max_retries {
                    retries += 1;
                    let delay = Duration::from_secs(2u64.pow(retries));  // Exponential backoff
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err("Max retries exceeded".into());
            }

            Err(e) => return Err(e.into()),
        }
    }
}
```

### Permanent Errors (Queue for Manual Resolution)

```rust
// Save failed operations for later
INSERT INTO sync_queue (source_id, operation, payload, last_error)
VALUES (?, 'sync_tracks', '{"cursor": "..."}', 'Authentication failed');
```

---

## Implementation Priority

### Phase 1: Basic Metadata Sync
- [x] Design architecture
- [ ] Implement server API endpoints
- [ ] Implement client sync logic
- [ ] UI: Sync button + progress

### Phase 2: Download for Offline
- [ ] Download queue
- [ ] Cache management
- [ ] Background worker
- [ ] UI: Download indicators

### Phase 3: Smart Sync
- [ ] Auto-download strategies
- [ ] Deduplication
- [ ] Conflict resolution
- [ ] Bandwidth throttling

### Phase 4: Real-time Sync
- [ ] WebSocket connection
- [ ] Live updates (no polling)
- [ ] Instant playlist changes
