# Soul Player - Architecture

## System Overview

Soul Player is a local-first, multi-platform music player with three operational modes:

1. **Desktop**: Standalone application (Tauri)
2. **Server**: Multi-user streaming server with sync
3. **Embedded**: ESP32-S3 portable DAP (Digital Audio Player)

**Core Principle**: Shared Rust logic across all platforms, with platform-specific adapters for I/O.

---

## High-Level Architecture

The architecture consists of three layers: soul-core provides platform-agnostic traits and business logic at the base. The middle layer contains soul-storage (SQLite), soul-audio (Symphonia), and soul-metadata (tag I/O) which implement the core traits. The top layer has platform-specific adapters: Desktop uses CPAL for audio output, Server uses Axum/Tokio for HTTP endpoints, and ESP32-S3 uses awedio_esp32 for embedded audio.

---

## Crate Structure

### **soul-core**
**Purpose**: Platform-agnostic core types and traits

**Key Components**:
```rust
// Domain Types
pub struct Track { id, title, artist, album, duration, ... }
pub struct Playlist { id, name, tracks, owner, ... }
pub struct User { id, name, ... }

// Traits
pub trait AudioDecoder {
    fn decode(&mut self, data: &[u8]) -> Result<AudioBuffer>;
}

pub trait AudioOutput {
    fn play(&mut self, buffer: &AudioBuffer) -> Result<()>;
}

pub trait Storage {
    async fn get_track(&self, id: TrackId) -> Result<Track>;
    async fn create_playlist(&self, user_id: UserId, name: &str) -> Result<Playlist>;
    // ... multi-user from the start
}
```

**Dependencies**: Minimal (serde, thiserror only)

---

### **soul-storage**
**Purpose**: Database layer supporting multi-user scenarios

**Architecture**:
```rust
pub struct Database {
    sqlite: SqlitePool,      // Primary storage
    cache: Option<RedbCache>, // Optional performance layer
}

// Multi-user schema design
impl Database {
    // Users are first-class
    async fn create_user(&self, name: &str) -> Result<User>;

    // Tracks belong to library (shared)
    async fn add_track(&self, track: Track) -> Result<TrackId>;

    // Playlists belong to users
    async fn create_playlist(&self, user_id: UserId, name: &str) -> Result<Playlist>;

    // Sharing mechanism
    async fn share_playlist(&self, playlist_id: PlaylistId, with_user: UserId) -> Result<()>;
}
```

**Schema Design** (SQLite):
```sql
-- Users table
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Tracks table (shared library)
CREATE TABLE tracks (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT,
    album TEXT,
    duration_ms INTEGER,
    file_path TEXT NOT NULL,
    file_hash TEXT,
    added_at INTEGER NOT NULL
);

-- Playlists table (user-owned)
CREATE TABLE playlists (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id)
);

-- Playlist tracks (many-to-many)
CREATE TABLE playlist_tracks (
    playlist_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, track_id),
    FOREIGN KEY (playlist_id) REFERENCES playlists(id),
    FOREIGN KEY (track_id) REFERENCES tracks(id)
);

-- Shared playlists (collaboration)
CREATE TABLE playlist_shares (
    playlist_id TEXT NOT NULL,
    shared_with_user_id TEXT NOT NULL,
    permission TEXT NOT NULL, -- 'read' or 'write'
    shared_at INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, shared_with_user_id),
    FOREIGN KEY (playlist_id) REFERENCES playlists(id),
    FOREIGN KEY (shared_with_user_id) REFERENCES users(id)
);
```

**Data Portability**:
- Export entire database to JSON
- Import from JSON (for sharing with friends)
- Entire folder (`soul-data/`) is portable

**Platform Support**:
- Desktop: SQLite file on disk
- Server: Same SQLite schema (single DB, multi-user)
- ESP32-S3: SQLite on SD card (ESP-IDF provides filesystem)

---

### **soul-audio**
**Purpose**: Audio decoding and playback with effect chain

**Architecture**:
```rust
// Unified decoder (uses Symphonia)
pub struct AudioDecoder {
    // Supports: MP3, FLAC, OGG, WAV, AAC, OPUS
}

// Platform-specific output
#[cfg(not(target_os = "espidf"))]
pub struct CpalOutput { /* Desktop via CPAL */ }

#[cfg(target_os = "espidf")]
pub struct EspOutput { /* ESP32 via awedio_esp32 */ }

// Effect chain (trait-based)
pub trait AudioEffect: Send {
    fn process(&mut self, buffer: &mut [f32], sample_rate: u32);
}

pub struct EffectChain {
    effects: Vec<Box<dyn AudioEffect>>,
}

// Built-in effects (MVP)
pub struct ThreeBandEq { /* Parametric EQ */ }
pub struct Compressor { /* Dynamic range compression */ }
```

**Audio Pipeline**: Audio flows from file to decoder to effect chain (EQ and compressor) to output device.

**Threading Model**:
- Decoding: Background thread (CPU-bound)
- Effect processing: Audio callback thread (real-time)
- Output: Platform-specific (CPAL/I2S)

---

### **soul-metadata**
**Purpose**: Tag reading/writing and library scanning

**Components**:
```rust
// Tag reading (uses lofty or similar)
pub fn read_tags(path: &Path) -> Result<TrackMetadata>;
pub fn write_tags(path: &Path, metadata: &TrackMetadata) -> Result<()>;

// Library scanner
pub struct LibraryScanner {
    // Recursively scan directories
    // Extract metadata from files
    // Populate database
}

pub async fn scan_library(path: &Path, db: &Database) -> Result<ScanStats>;
```

**Supported Tags**:
- ID3v2 (MP3)
- Vorbis Comments (OGG, FLAC, OPUS)
- MP4/M4A atoms (AAC)

---

### **soul-discovery**
**Purpose**: Music discovery and metadata enrichment

**Services**:
```rust
// Bandcamp integration
pub struct BandcampClient {
    async fn search(&self, query: &str) -> Result<Vec<Album>>;
    async fn get_album(&self, url: &str) -> Result<AlbumDetails>;
}

// Discogs integration
pub struct DiscogsClient {
    async fn enrich_metadata(&self, track: &Track) -> Result<EnrichedMetadata>;
}

// Similar track algorithm (future)
pub struct SimilarityEngine {
    // Acoustic fingerprinting
    // Genre/mood analysis
}
```

**Phase**: Post-MVP (Phase 4)

---

### **soul-sync**
**Purpose**: Client-server synchronization protocol

**Protocol Design**:
```rust
// Sync operations
pub enum SyncOperation {
    // Metadata sync
    TrackAdded { track: Track },
    TrackUpdated { id: TrackId, metadata: Metadata },
    TrackDeleted { id: TrackId },

    // Playlist sync
    PlaylistCreated { playlist: Playlist },
    PlaylistUpdated { id: PlaylistId, changes: Vec<Change> },
    PlaylistShared { id: PlaylistId, with: UserId },

    // Play state sync
    NowPlaying { user_id: UserId, track_id: TrackId, position: Duration },
}

// Sync client
pub struct SyncClient {
    server_url: Url,
    auth_token: String,

    async fn push(&self, ops: Vec<SyncOperation>) -> Result<()>;
    async fn pull(&self) -> Result<Vec<SyncOperation>>;
}

// Conflict resolution
pub enum ConflictStrategy {
    ServerWins,
    ClientWins,
    LastWriteWins,
    Merge,
}
```

**Transport**:
- REST API for bulk operations
- WebSocket for real-time updates

---

### **soul-server**
**Purpose**: Multi-user streaming server

**Components**:
```rust
// HTTP server (Axum)
pub struct SoulServer {
    db: Database,          // Shared database (multi-user)
    auth: AuthService,
    storage: FileStorage,  // Audio file access
}

// Routes
// POST /api/auth/login
// POST /api/auth/refresh
// GET  /api/tracks
// POST /api/playlists
// GET  /api/playlists/:id
// POST /api/playlists/:id/share
// GET  /api/stream/:track_id
// WebSocket /api/ws (real-time sync)
```

**Authentication**:
```rust
pub struct AuthService {
    secret: String, // JWT secret

    fn create_token(&self, user: &User) -> Result<String>;
    fn verify_token(&self, token: &str) -> Result<UserId>;
}
```

**Streaming**:
- Range requests for seeking
- Transcoding (optional, future)
- Rate limiting per user

**Deployment**:
- Docker container
- Single binary with embedded migrations
- Environment-based config

---

### **soul-player-desktop**
**Purpose**: Tauri desktop application

**Architecture**: The frontend (React/Vue) provides library view, playback controls, playlist editor, and settings. It communicates via Tauri IPC commands to the Rust backend which handles the database (soul-storage), audio engine (soul-audio), sync client (soul-sync), and library scanner (soul-metadata).

**Tauri Commands**:
```rust
#[tauri::command]
async fn get_tracks(db: State<Database>) -> Result<Vec<Track>, Error>;

#[tauri::command]
async fn play_track(player: State<AudioPlayer>, track_id: TrackId) -> Result<(), Error>;

#[tauri::command]
async fn sync_with_server(sync: State<SyncClient>) -> Result<SyncStatus, Error>;
```

**State Management**:
- Backend: Tauri managed state
- Frontend: React Context / Vuex / Pinia

---

### **soul-player-esp32**
**Purpose**: ESP32-S3 portable music player

**Architecture**:
```rust
// Main task (Embassy)
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialize peripherals
    let i2s = I2s::new(/* ... */);
    let sd_card = SdCard::new(/* ... */);
    let display = EinkDisplay::new(/* ... */);

    // Spawn tasks
    spawner.spawn(audio_task(i2s)).unwrap();
    spawner.spawn(ui_task(display)).unwrap();
    spawner.spawn(sync_task()).unwrap();
}

// Audio task
#[embassy_executor::task]
async fn audio_task(i2s: I2s) {
    let decoder = AudioDecoder::new();
    let output = EspOutput::new(i2s);

    loop {
        let track = PLAYBACK_QUEUE.pop().await;
        let buffer = decoder.decode(&track.data)?;
        output.play(&buffer)?;
    }
}
```

**Storage**:
- SD card: Music files + SQLite database
- Same database schema as desktop/server!

**Sync**:
- WiFi client
- Push/pull changes when connected
- Offline-first

---

## Data Flow

### **Local Playback (Desktop/ESP32)**
When the user selects a track, the system performs a database lookup via soul-storage, reads the file from disk or SD card, decodes the audio using soul-audio, applies configured effects (EQ, compressor), and outputs to speakers through CPAL (desktop) or I2S (ESP32).

### **Server Streaming (Server to Desktop)**
Desktop requests a track from the server, which authenticates the user and checks permissions. The server streams the audio file to the desktop client, where it is decoded locally using the same audio pipeline and then played.

### **Synchronization**
Desktop scans new files and extracts metadata using soul-metadata. The data is added to the local database via soul-storage, and a sync operation is created through soul-sync. The operation is pushed to the server via HTTP, which updates the central database and broadcasts changes to other clients via WebSocket. Clients pull updates and refresh their local databases.

---

## Operational Modes

### **Mode 1: Desktop (Local Only)**
- Single user (default user auto-created)
- No server connection
- All data local

### **Mode 2: Desktop (Client Mode)**
- Connect to server
- Sync library metadata
- Stream tracks from server
- Local cache for offline

### **Mode 3: Server**
- Multi-user support
- Central library
- Authentication required
- Clients connect for streaming

### **Mode 4: ESP32 (Standalone)**
- Offline playback from SD card
- Local user profile
- WiFi sync when available

---

## Security Considerations

### **Authentication**
- Server: JWT-based (HS256)
- Token refresh mechanism
- No passwords stored in plaintext (bcrypt)

### **API Security**
- Rate limiting per user
- Input validation
- SQL injection prevention (parameterized queries)
- Path traversal prevention (file streaming)

### **Data Privacy**
- Local data not encrypted (user responsibility)
- Server data encrypted at rest (optional)
- TLS for network communication

---

## Performance Targets

### **Desktop**
- Library load (10k tracks): <2s
- Search latency: <100ms
- Audio latency: <50ms
- Memory usage: <200MB

### **Server**
- Concurrent streams: 100+ users
- API response time: <200ms (p95)
- Sync latency: <1s for metadata changes

### **ESP32-S3**
- Boot time: <5s
- Track start latency: <500ms
- Battery life: 8+ hours
- Sync time (100 tracks): <30s

---

## Scalability

### **Database**
- SQLite: Suitable for <100k tracks per instance
- For larger: Consider PostgreSQL (server only)
- Indexing strategy for fast lookups

### **Server**
- Horizontal scaling: Multiple instances + load balancer
- Shared storage: NFS / S3
- Caching: Redis for hot data

---

## Extensibility

### **Plugin System (Future)**
- Effect plugins (VST-like)
- Metadata provider plugins
- Output plugins (custom hardware)

### **Custom Audio Formats**
- Extend `AudioDecoder` trait
- Register new format handlers

---

## Technology Choices Summary

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **Audio Decoding** | Symphonia | Pure Rust, all formats, cross-platform |
| **Desktop Output** | CPAL | Cross-platform, low latency |
| **ESP32 Output** | awedio_esp32 | ESP-IDF integration, Symphonia compatible |
| **Database** | SQLite | Embedded, portable, multi-user capable |
| **Cache** | redb | Pure Rust, ACID, performant |
| **Server** | Axum + Tokio | Async, type-safe, fast |
| **Desktop UI** | Tauri v2 | Small binary, native performance |
| **ESP32 RTOS** | Embassy | Async embedded, excellent HAL |
| **Testing** | Testcontainers | Real database, realistic tests |

---

## References

- See `docs/CONVENTIONS.md` for coding standards
- See `docs/TESTING.md` for testing strategy
- See `ROADMAP.md` for implementation phases
