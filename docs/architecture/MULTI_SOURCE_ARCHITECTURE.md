# Multi-Source Architecture

## Overview

Soul Player supports **multiple music sources** that can coexist in a unified library:
- **Local**: Files on the user's device
- **Servers**: Multiple remote Soul Player servers (home server, cloud server, friend's server, etc.)

Users can:
- Have multiple servers configured
- Choose one "active" server at a time
- Sync libraries from server to local
- Stream from server OR download for offline
- Automatically fallback to local when offline

---

## Core Concepts

### 1. Sources

A **source** is where music comes from:

```rust
pub enum SourceType {
    Local,           // Files on this device
    Server,          // Remote Soul Player server
}

pub struct Source {
    pub id: SourceId,
    pub name: String,              // "My Home Server", "Local Files"
    pub source_type: SourceType,
    pub config: SourceConfig,
    pub is_active: bool,           // Only one server can be active
    pub last_sync: Option<DateTime>,
    pub is_online: bool,           // Current connectivity status
}

pub enum SourceConfig {
    Local,
    Server {
        url: String,
        username: String,
        token: Option<String>,     // Cached auth token
    },
}
```

### 2. Track Sources

Each track knows **where it came from** and **where it's available**:

```rust
pub struct Track {
    pub id: TrackId,
    pub title: String,
    // ... metadata fields

    // Source tracking
    pub origin_source_id: SourceId,     // Where it was imported from
    pub availability: Vec<TrackAvailability>,
}

pub struct TrackAvailability {
    pub source_id: SourceId,
    pub status: AvailabilityStatus,
    pub file_path: Option<PathBuf>,     // For local/cached
    pub server_path: Option<String>,    // For streaming
}

pub enum AvailabilityStatus {
    LocalFile,           // File exists locally
    Cached,              // Downloaded from server, available offline
    StreamOnly,          // Must stream from server (requires connection)
    Unavailable,         // Source is offline, no local cache
}
```

### 3. Playback Strategy

When user plays a track, the player chooses the best source:

```
Priority (highest to lowest):
1. Local file (if available)
2. Cached file from server (if available)
3. Stream from active server (if online)
4. Stream from any online server (if active is offline)
5. Fail with "Offline" message
```

---

## Database Schema

### Source Management

```sql
-- Sources configuration
CREATE TABLE sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL,  -- 'local' or 'server'

    -- Server config (NULL for local)
    server_url TEXT,
    server_username TEXT,
    server_token TEXT,  -- Cached auth token

    is_active BOOLEAN NOT NULL DEFAULT 0,
    is_online BOOLEAN NOT NULL DEFAULT 1,
    last_sync_at TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Only one active server at a time
    CHECK (
        source_type = 'local' OR
        (source_type = 'server' AND server_url IS NOT NULL)
    )
);

-- Ensure only one active server
CREATE UNIQUE INDEX idx_active_server
ON sources(is_active)
WHERE source_type = 'server' AND is_active = 1;

-- Always have exactly one local source
INSERT INTO sources (id, name, source_type, is_active, is_online)
VALUES (1, 'Local Files', 'local', 1, 1);
```

### Track Source Tracking

```sql
-- Enhanced tracks table
CREATE TABLE tracks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- ... existing metadata fields

    -- Source tracking
    origin_source_id INTEGER NOT NULL,  -- Where it was imported from

    -- Deduplication (same track from multiple sources)
    musicbrainz_recording_id TEXT,      -- For matching across sources
    fingerprint TEXT,                   -- Acoustic fingerprint (Chromaprint)

    FOREIGN KEY (origin_source_id) REFERENCES sources(id) ON DELETE CASCADE
);

-- Track availability across sources
CREATE TABLE track_sources (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id INTEGER NOT NULL,
    source_id INTEGER NOT NULL,

    -- Availability
    status TEXT NOT NULL,  -- 'local_file', 'cached', 'stream_only', 'unavailable'

    -- Local storage
    local_file_path TEXT,              -- If downloaded/cached
    local_file_size INTEGER,
    downloaded_at TEXT,

    -- Server storage
    server_path TEXT,                  -- Path on server for streaming
    server_file_id TEXT,               -- Server's track ID

    -- Sync metadata
    last_verified_at TEXT,             -- Last time we checked availability

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (track_id) REFERENCES tracks(id) ON DELETE CASCADE,
    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE,

    UNIQUE(track_id, source_id)
);

-- Indexes
CREATE INDEX idx_track_sources_track ON track_sources(track_id);
CREATE INDEX idx_track_sources_source ON track_sources(source_id);
CREATE INDEX idx_track_sources_status ON track_sources(status);
CREATE INDEX idx_tracks_origin_source ON tracks(origin_source_id);
CREATE INDEX idx_tracks_musicbrainz ON tracks(musicbrainz_recording_id);
CREATE INDEX idx_tracks_fingerprint ON tracks(fingerprint);
```

### Sync State Tracking

```sql
-- Track what's been synced from servers
CREATE TABLE sync_state (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL,
    entity_type TEXT NOT NULL,  -- 'tracks', 'playlists', 'artists', etc.
    last_sync_at TEXT NOT NULL,
    last_sync_cursor TEXT,      -- For incremental sync
    total_items INTEGER,
    synced_items INTEGER,

    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE,
    UNIQUE(source_id, entity_type)
);

-- Failed sync operations (for retry)
CREATE TABLE sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id INTEGER NOT NULL,
    operation TEXT NOT NULL,    -- 'import_track', 'download_file', etc.
    payload TEXT NOT NULL,      -- JSON data
    retry_count INTEGER DEFAULT 0,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE
);
```

---

## Architecture Components

### 1. Source Manager

```rust
// soul-core/src/sources.rs

pub struct SourceManager {
    sources: HashMap<SourceId, Source>,
    active_server_id: Option<SourceId>,
    local_source_id: SourceId,  // Always ID = 1
}

impl SourceManager {
    /// Add a new server source
    pub async fn add_server(&mut self, name: String, url: String) -> Result<Source> {
        // Validate URL, test connection, save to DB
    }

    /// Set active server (only one at a time)
    pub async fn set_active_server(&mut self, source_id: SourceId) -> Result<()> {
        // Deactivate current, activate new
    }

    /// Check connectivity for all servers
    pub async fn check_connectivity(&mut self) -> Result<()> {
        for source in self.sources.values_mut() {
            if let SourceType::Server = source.source_type {
                source.is_online = self.ping_server(source).await;
            }
        }
    }

    /// Get best available source for a track
    pub fn best_source_for_track(&self, track: &Track) -> Option<&Source> {
        // Priority: local > cached > stream from active > stream from any
        track.availability.iter()
            .filter_map(|av| {
                let source = self.sources.get(&av.source_id)?;
                Some((av, source))
            })
            .max_by_key(|(av, source)| {
                match av.status {
                    AvailabilityStatus::LocalFile => 100,
                    AvailabilityStatus::Cached => 90,
                    AvailabilityStatus::StreamOnly if source.is_online && source.is_active => 50,
                    AvailabilityStatus::StreamOnly if source.is_online => 40,
                    _ => 0,
                }
            })
            .map(|(_, source)| source)
    }
}
```

### 2. Unified Storage Context

```rust
// soul-core/src/storage.rs

#[async_trait]
pub trait StorageContext: Send + Sync {
    /// Get current user ID
    fn user_id(&self) -> UserId;

    /// Get all configured sources
    async fn get_sources(&self) -> Result<Vec<Source>>;

    /// Get active server source
    async fn get_active_server(&self) -> Result<Option<Source>>;

    /// Get all tracks (from all sources)
    async fn get_all_tracks(&self) -> Result<Vec<Track>>;

    /// Get tracks from specific source
    async fn get_tracks_by_source(&self, source_id: SourceId) -> Result<Vec<Track>>;

    /// Get track availability
    async fn get_track_availability(&self, track_id: TrackId) -> Result<Vec<TrackAvailability>>;

    /// Download track from server to local cache
    async fn download_track(
        &self,
        track_id: TrackId,
        source_id: SourceId
    ) -> Result<PathBuf>;

    /// Sync library from server
    async fn sync_from_server(&self, source_id: SourceId) -> Result<SyncResult>;
}

// Local implementation queries local SQLite
pub struct LocalStorageContext {
    user_id: UserId,
    pool: SqlitePool,
    source_manager: Arc<RwLock<SourceManager>>,
}

// Hybrid implementation (local DB + server API)
pub struct HybridStorageContext {
    local: LocalStorageContext,
    server_clients: HashMap<SourceId, ServerClient>,
}

impl HybridStorageContext {
    /// Get tracks merges local + all servers
    async fn get_all_tracks(&self) -> Result<Vec<Track>> {
        // 1. Get all tracks from local DB (includes cached server tracks)
        let local_tracks = self.local.get_all_tracks().await?;

        // 2. If online, fetch fresh metadata from active server
        if let Some(active_server) = self.local.get_active_server().await? {
            if active_server.is_online {
                // Sync metadata (not files) from server
                self.sync_metadata_from_server(active_server.id).await?;
            }
        }

        Ok(local_tracks)
    }
}
```

### 3. Server Client

```rust
// soul-sync/src/server_client.rs

pub struct ServerClient {
    url: String,
    token: String,
    client: reqwest::Client,
}

impl ServerClient {
    /// Fetch track list from server
    pub async fn fetch_tracks(&self) -> Result<Vec<ServerTrack>> {
        self.client
            .get(&format!("{}/api/tracks", self.url))
            .bearer_auth(&self.token)
            .send()
            .await?
            .json()
            .await
    }

    /// Stream track (returns byte stream)
    pub async fn stream_track(&self, track_id: &str) -> Result<impl Stream<Item = Bytes>> {
        let response = self.client
            .get(&format!("{}/api/tracks/{}/stream", self.url, track_id))
            .bearer_auth(&self.token)
            .send()
            .await?;

        Ok(response.bytes_stream())
    }

    /// Download track to local file
    pub async fn download_track(&self, track_id: &str, dest: &Path) -> Result<()> {
        let mut stream = self.stream_track(track_id).await?;
        let mut file = tokio::fs::File::create(dest).await?;

        while let Some(chunk) = stream.next().await {
            file.write_all(&chunk?).await?;
        }

        Ok(())
    }

    /// Check server health
    pub async fn ping(&self) -> bool {
        self.client
            .get(&format!("{}/api/health", self.url))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .is_ok()
    }
}
```

---

## Playback Flow

### Scenario 1: Playing a Local Track

```
User clicks play on track
  → Check track.availability
  → Find LocalFile status
  → Play from local path
```

### Scenario 2: Playing a Server Track (Online)

```
User clicks play on track from server
  → Check track.availability
  → Find StreamOnly status for active server
  → Check server.is_online = true
  → Stream from server URL
```

### Scenario 3: Playing a Server Track (Offline)

```
User clicks play on track from server
  → Check track.availability
  → Find StreamOnly status
  → Check server.is_online = false
  → Check for Cached status
  → If cached: Play from cache
  → If not cached: Show "Offline - Track Unavailable"
```

### Scenario 4: Smart Download

```
User enables "Download for Offline"
  → Background task queues all StreamOnly tracks
  → Downloads to cache directory
  → Updates track_sources.status = 'cached'
  → Updates local_file_path
```

---

## Sync Strategy (Simplified)

### Full Sync (First Time)

```
1. Connect to server
2. Fetch all tracks metadata (not files)
3. Insert into local DB with origin_source_id = server
4. Create track_sources entries with status = 'stream_only'
5. User sees full server library (can stream)
```

### Incremental Sync

```
1. Get last_sync_cursor from sync_state
2. Fetch only new/updated tracks since cursor
3. Update local DB
4. Update last_sync_at and cursor
```

### Download for Offline

```
1. User marks tracks/playlists for offline
2. Background worker downloads files
3. Stores in cache directory with content-addressable names
4. Updates track_sources.status = 'cached'
5. Files available even when offline
```

---

## Cache Management

### Cache Directory Structure

```
~/.soul-player/cache/
  tracks/
    {source_id}/
      {track_hash}.mp3    # Content-addressable
      {track_hash}.flac
  covers/
    {album_hash}.jpg
  metadata/
    musicbrainz/          # Enrichment cache
```

### Cache Eviction Policy

- LRU (Least Recently Used)
- User can set max cache size (default 10GB)
- User can pin tracks (never evict)
- Auto-evict StreamOnly when space needed

---

## User Experience

### Settings UI

```
Sources:
  ☑ Local Files (always on)

  Servers:
    ⦿ Home Server (192.168.1.100) - Active, Online
      Last sync: 5 minutes ago
      [Sync Now] [Download All] [Remove]

    ○ Cloud Server (music.example.com) - Inactive, Online
      Last sync: 2 days ago
      [Set Active] [Sync Now] [Remove]

    [+ Add Server]

Offline Mode:
  Cache size: 4.2 GB / 10 GB
  Downloaded tracks: 342 / 1,243

  [Download All for Offline]
  [Clear Cache]
```

### Track List UI

```
Tracks show source badges:

🏠 Local       - Imported from local files
☁️ Server      - Available on server (can stream)
💾 Cached      - Downloaded, available offline
⚠️ Offline    - Server offline, not cached (grayed out)
```

---

## Security Considerations

### Server Authentication

- Use JWT tokens
- Refresh tokens for long-lived sessions
- Store encrypted in local DB

### Cache Encryption (Optional)

- Encrypt cached files with device key
- Protect user's downloaded music

---

## Future Enhancements

1. **Peer-to-Peer Sources**: Friend's library over local network
2. **Cloud Storage**: Google Drive, Dropbox as sources
3. **Smart Sync**: Only download high-rated tracks
4. **Bandwidth Management**: Stream quality based on connection
5. **Offline Playlists**: Auto-download playlist tracks

---

## Implementation Phases

See [SYNC_STRATEGY.md](./SYNC_STRATEGY.md) and [OFFLINE_MODE.md](./OFFLINE_MODE.md) for detailed strategies.
