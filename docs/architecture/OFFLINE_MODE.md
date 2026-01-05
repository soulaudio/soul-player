# Offline Mode

## Overview

Soul Player is **offline-first**: the app works fully without internet connection by storing all data locally. Server sources enhance the experience but are not required.

---

## Offline Capabilities

### ✅ Full Functionality Offline

- Play all local tracks
- Play cached tracks (downloaded from server)
- Browse library
- Create/edit playlists
- View play history
- Adjust settings

### ⏸️ Limited Functionality Offline

- **Cannot stream** from servers (requires connection)
- **Cannot sync** new tracks from servers
- **Cannot download** new tracks for offline
- **Cannot share** playlists with other users (queued for later)

### 📝 Queued Operations

Actions performed offline are **queued** and sync when back online:
- Playlist edits (add/remove tracks)
- Play count updates
- New favorites
- Rating changes

---

## Offline Detection

### Connection Monitoring

```rust
pub struct ConnectionMonitor {
    status: Arc<RwLock<ConnectionStatus>>,
    sources: Arc<RwLock<HashMap<SourceId, bool>>>,  // source -> is_online
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Online,
    Offline,
    Limited,  // Connected but slow/unreliable
}

impl ConnectionMonitor {
    /// Start monitoring in background
    pub async fn start(&self) {
        tokio::spawn({
            let monitor = self.clone();
            async move {
                loop {
                    monitor.check_connectivity().await;
                    tokio::time::sleep(Duration::from_secs(30)).await;
                }
            }
        });
    }

    /// Check all sources
    async fn check_connectivity(&self) {
        // 1. Check general internet
        let has_internet = Self::ping_internet().await;

        // 2. Check each server source
        let mut sources = self.sources.write().await;

        for (source_id, is_online) in sources.iter_mut() {
            if !has_internet {
                *is_online = false;
                continue;
            }

            *is_online = Self::ping_source(source_id).await;
        }

        // 3. Update overall status
        let mut status = self.status.write().await;
        *status = if has_internet {
            ConnectionStatus::Online
        } else {
            ConnectionStatus::Offline
        };

        // 4. Emit status change event
        emit_connection_status_changed(*status);
    }

    /// Quick internet check (ping Cloudflare DNS)
    async fn ping_internet() -> bool {
        tokio::time::timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect("1.1.1.1:53")
        )
        .await
        .is_ok()
    }

    /// Check specific server health
    async fn ping_source(source_id: &SourceId) -> bool {
        let source = get_source(source_id).await.ok()?;

        if let SourceType::Server { url, .. } = source.source_type {
            let client = reqwest::Client::new();

            client
                .get(format!("{}/api/health", url))
                .timeout(Duration::from_secs(5))
                .send()
                .await
                .is_ok()
        } else {
            true  // Local source always "online"
        }
    }

    /// Subscribe to status changes
    pub fn subscribe(&self) -> ConnectionStatusReceiver {
        // Return channel receiver for UI updates
    }
}
```

---

## Automatic Fallback

### Playing Tracks

When user tries to play a track:

```rust
async fn play_track(track_id: TrackId, storage: &dyn StorageContext) -> Result<AudioSource> {
    let track = storage.get_track_by_id(track_id).await?;
    let availability = storage.get_track_availability(track_id).await?;

    // Priority: Local > Cached > Stream > Fail
    for av in availability {
        match av.status {
            // 1. Local file (always works offline)
            AvailabilityStatus::LocalFile => {
                if let Some(path) = &av.local_file_path {
                    return Ok(AudioSource::File(path.clone()));
                }
            }

            // 2. Cached from server (works offline)
            AvailabilityStatus::Cached => {
                if let Some(path) = &av.local_file_path {
                    return Ok(AudioSource::File(path.clone()));
                }
            }

            // 3. Stream from server (requires online)
            AvailabilityStatus::StreamOnly => {
                let source = storage.get_source(av.source_id).await?;

                if source.is_online {
                    return Ok(AudioSource::Stream {
                        url: format!("{}/api/tracks/{}/stream", source.url, track.server_id),
                        token: source.token.clone(),
                    });
                }
            }

            AvailabilityStatus::Unavailable => continue,
        }
    }

    // No available source
    Err(PlaybackError::Unavailable {
        reason: if is_offline() {
            "Track not available offline. Download for offline use?"
        } else {
            "Track unavailable from all sources"
        }
    })
}
```

### Library View

Filter unavailable tracks when offline:

```rust
async fn get_all_tracks(storage: &dyn StorageContext) -> Result<Vec<Track>> {
    let is_offline = connection_monitor.is_offline().await;

    let mut tracks = storage.get_all_tracks().await?;

    if is_offline {
        // Filter out stream-only tracks
        tracks.retain(|track| {
            track.availability.iter().any(|av| matches!(
                av.status,
                AvailabilityStatus::LocalFile | AvailabilityStatus::Cached
            ))
        });
    }

    Ok(tracks)
}
```

---

## Offline Queue

### Queue Schema

```sql
CREATE TABLE offline_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    operation TEXT NOT NULL,    -- 'create_playlist', 'add_track', 'update_rating', etc.
    entity_type TEXT NOT NULL,  -- 'playlist', 'track', etc.
    entity_id TEXT NOT NULL,
    payload TEXT NOT NULL,      -- JSON data
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    retries INTEGER DEFAULT 0,
    last_error TEXT,
    status TEXT DEFAULT 'pending'  -- 'pending', 'syncing', 'failed', 'completed'
);

CREATE INDEX idx_offline_queue_status ON offline_queue(status);
```

### Queueing Operations

```rust
pub struct OfflineQueue {
    pool: SqlitePool,
}

impl OfflineQueue {
    /// Queue an operation for later sync
    pub async fn enqueue(&self, operation: OfflineOperation) -> Result<()> {
        sqlx::query!(
            "INSERT INTO offline_queue (operation, entity_type, entity_id, payload)
             VALUES (?, ?, ?, ?)",
            operation.operation,
            operation.entity_type,
            operation.entity_id,
            serde_json::to_string(&operation.payload)?
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Process queue when back online
    pub async fn process_queue(&self, server_client: &ServerClient) -> Result<QueueResult> {
        let pending = sqlx::query_as!(
            QueuedOperation,
            "SELECT * FROM offline_queue WHERE status = 'pending' ORDER BY created_at"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut succeeded = 0;
        let mut failed = 0;

        for op in pending {
            match self.execute_operation(&op, server_client).await {
                Ok(_) => {
                    // Mark as completed
                    sqlx::query!(
                        "UPDATE offline_queue SET status = 'completed' WHERE id = ?",
                        op.id
                    )
                    .execute(&self.pool)
                    .await?;
                    succeeded += 1;
                }

                Err(e) => {
                    // Retry or mark as failed
                    let retries = op.retries + 1;
                    let status = if retries > 3 { "failed" } else { "pending" };

                    sqlx::query!(
                        "UPDATE offline_queue SET retries = ?, last_error = ?, status = ?
                         WHERE id = ?",
                        retries,
                        e.to_string(),
                        status,
                        op.id
                    )
                    .execute(&self.pool)
                    .await?;

                    if retries > 3 {
                        failed += 1;
                    }
                }
            }
        }

        Ok(QueueResult { succeeded, failed })
    }

    async fn execute_operation(
        &self,
        op: &QueuedOperation,
        client: &ServerClient
    ) -> Result<()> {
        match op.operation.as_str() {
            "create_playlist" => {
                let data: CreatePlaylistData = serde_json::from_str(&op.payload)?;
                client.create_playlist(&data.name, data.description.as_deref()).await?;
            }

            "add_track_to_playlist" => {
                let data: AddTrackData = serde_json::from_str(&op.payload)?;
                client.add_track_to_playlist(&data.playlist_id, &data.track_id).await?;
            }

            "update_play_count" => {
                let data: PlayCountData = serde_json::from_str(&op.payload)?;
                client.update_play_count(&data.track_id, data.count).await?;
            }

            _ => return Err("Unknown operation".into()),
        }

        Ok(())
    }
}
```

### Example: Creating Playlist Offline

```rust
#[tauri::command]
async fn create_playlist(
    name: String,
    description: Option<String>,
    storage: State<'_, Arc<dyn StorageContext>>,
    offline_queue: State<'_, Arc<OfflineQueue>>,
) -> Result<Playlist, String> {
    // 1. Create playlist locally
    let playlist = storage.create_playlist(name.clone(), description.clone()).await?;

    // 2. If we have an active server, queue for sync
    if let Some(active_server) = storage.get_active_server().await? {
        if !active_server.is_online {
            // Queue for later
            offline_queue.enqueue(OfflineOperation {
                operation: "create_playlist".to_string(),
                entity_type: "playlist".to_string(),
                entity_id: playlist.id.to_string(),
                payload: serde_json::json!({
                    "name": name,
                    "description": description,
                }),
            }).await?;
        } else {
            // Sync immediately
            let client = ServerClient::new(&active_server);
            client.create_playlist(&name, description.as_deref()).await?;
        }
    }

    Ok(playlist)
}
```

---

## UI Indicators

### Connection Status Banner

```
┌────────────────────────────────────────┐
│ 🔌 Offline - Some features unavailable │
│ [View Available Tracks]                │
└────────────────────────────────────────┘
```

### Track Availability Icons

```
Library View:

✓ Song A - Artist A (Local)             [Play]
✓ Song B - Artist B (Cached)            [Play]
⚠ Song C - Artist C (Requires WiFi)     [Download]
✗ Song D - Artist D (Unavailable)       [——]
```

### Offline Filter Toggle

```
[Filter: All Tracks ▾]
  • All Tracks
  • Available Offline Only  ← Auto-selected when offline
  • Cached Only
  • Server Only
```

### Queue Status

```
Settings > Offline Queue

Pending changes: 5
├─ Create playlist "Road Trip" (3 days ago)
├─ Add 12 tracks to "Favorites" (2 days ago)
└─ Update play counts (1 day ago)

[Sync Now] [Clear Queue]
```

---

## Smart Cache Management

### Auto-Eviction

```rust
pub struct CacheManager {
    max_cache_size: u64,
    current_size: Arc<RwLock<u64>>,
}

impl CacheManager {
    /// Evict least recently played tracks to free space
    pub async fn evict_lru(&self, space_needed: u64) -> Result<()> {
        let mut freed = 0u64;

        // Get cached tracks ordered by last played (oldest first)
        let tracks = sqlx::query!(
            "SELECT ts.track_id, ts.local_file_path, ts.local_file_size
             FROM track_sources ts
             LEFT JOIN play_history ph ON ts.track_id = ph.track_id
             WHERE ts.status = 'cached'
             AND ts.track_id NOT IN (
                 SELECT track_id FROM pinned_tracks
             )
             GROUP BY ts.track_id
             ORDER BY MAX(ph.played_at) ASC"
        )
        .fetch_all(&self.pool)
        .await?;

        for track in tracks {
            if freed >= space_needed {
                break;
            }

            // Delete file
            if let Some(path) = track.local_file_path {
                tokio::fs::remove_file(&path).await?;
            }

            // Update status
            sqlx::query!(
                "UPDATE track_sources
                 SET status = 'stream_only', local_file_path = NULL
                 WHERE track_id = ?",
                track.track_id
            )
            .execute(&self.pool)
            .await?;

            freed += track.local_file_size.unwrap_or(0) as u64;
        }

        Ok(())
    }

    /// Pin track (never evict)
    pub async fn pin_track(&self, track_id: TrackId) -> Result<()> {
        sqlx::query!(
            "INSERT INTO pinned_tracks (track_id) VALUES (?)
             ON CONFLICT DO NOTHING",
            track_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
```

### Pin Tracks for Offline

Users can **pin** tracks to prevent auto-eviction:

```
Song Details:
  [★ Pin for Offline]  ← Never auto-delete
```

---

## Reconnection Behavior

### Auto-Sync on Reconnect

```rust
async fn on_connection_restored() {
    // 1. Update connection status
    connection_monitor.set_status(ConnectionStatus::Online).await;

    // 2. Show notification
    show_notification("Back Online", "Syncing library...");

    // 3. Process offline queue
    let result = offline_queue.process_queue(&server_client).await?;

    if result.failed > 0 {
        show_notification(
            "Sync Issues",
            &format!("{} operations failed to sync", result.failed)
        );
    } else {
        show_notification("Synced", "All changes uploaded");
    }

    // 4. Sync metadata from server
    sync_metadata_from_active_server().await?;

    // 5. Resume download queue
    download_worker.resume().await;
}
```

### Gradual Degradation

When connection becomes unstable:

```rust
match connection_quality {
    ConnectionQuality::Excellent => {
        // Full sync, high quality streaming
        stream_quality = StreamQuality::High;
        sync_interval = Duration::from_secs(60);
    }

    ConnectionQuality::Good => {
        // Medium quality, less frequent sync
        stream_quality = StreamQuality::Medium;
        sync_interval = Duration::from_secs(300);
    }

    ConnectionQuality::Poor => {
        // Low quality, disable auto-sync
        stream_quality = StreamQuality::Low;
        sync_interval = Duration::from_secs(0);  // Manual only
    }

    ConnectionQuality::Offline => {
        // Offline mode
        show_offline_banner();
    }
}
```

---

## Offline Settings

### User Preferences

```
Settings > Offline & Sync

Download Quality:
  ○ High (320kbps MP3 / FLAC)
  ⦿ Medium (192kbps MP3)
  ○ Low (128kbps MP3)

Cache Size:
  [=========>-----] 4.2 GB / 10 GB
  [Manage Cache]

Auto-Download:
  ☑ Recently Played (last 30 days)
  ☑ Favorite Playlists
  ☐ All Playlists
  ☐ Entire Library (⚠️ May use significant storage)

Network Usage:
  ☑ Only download on WiFi
  ☐ Allow mobile data (not recommended)
  ☑ Pause downloads while streaming

When Offline:
  ⦿ Show only available tracks
  ○ Show all tracks (grayed out if unavailable)
```

---

## Testing Offline Mode

### Manual Testing

```bash
# Simulate offline mode
# Option 1: Disable WiFi
# Option 2: Block server in hosts file
echo "127.0.0.1 music.example.com" >> /etc/hosts

# Test scenarios:
1. Play local track (should work)
2. Play cached track (should work)
3. Try to stream (should show "offline" error)
4. Create playlist (should queue)
5. Re-enable connection (should auto-sync)
```

### Automated Testing

```rust
#[tokio::test]
async fn test_offline_playback() {
    let storage = setup_test_storage().await;

    // Add local track
    let track = storage.create_track(CreateTrack {
        file_path: "/test/song.mp3",
        status: AvailabilityStatus::LocalFile,
        ...
    }).await.unwrap();

    // Simulate offline
    connection_monitor.set_offline().await;

    // Should still be able to play
    let audio_source = play_track(track.id, &storage).await.unwrap();
    assert!(matches!(audio_source, AudioSource::File(_)));
}

#[tokio::test]
async fn test_offline_queue() {
    let storage = setup_test_storage().await;
    let queue = OfflineQueue::new();

    // Go offline
    connection_monitor.set_offline().await;

    // Create playlist offline
    let playlist = create_playlist("Test", None, &storage, &queue).await.unwrap();

    // Verify queued
    let pending = queue.get_pending().await.unwrap();
    assert_eq!(pending.len(), 1);

    // Go back online
    connection_monitor.set_online().await;

    // Process queue
    let result = queue.process_queue(&server_client).await.unwrap();
    assert_eq!(result.succeeded, 1);
}
```

---

## Implementation Phases

### Phase 1: Basic Offline Support ✅
- [x] Local file playback
- [x] Connection monitoring
- [x] Filter unavailable tracks

### Phase 2: Offline Queue
- [ ] Queue schema
- [ ] Queue operations
- [ ] Auto-sync on reconnect
- [ ] UI: Pending changes indicator

### Phase 3: Cache Management
- [ ] LRU eviction
- [ ] Pin tracks
- [ ] Cache size limits
- [ ] UI: Cache settings

### Phase 4: Smart Offline
- [ ] Auto-download strategies
- [ ] Bandwidth awareness
- [ ] Gradual degradation
- [ ] Predictive caching (ML-based)
