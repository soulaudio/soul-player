# Soul Player - Architecture

## System Overview

Soul Player is a local-first, multi-platform music player with multiple operational modes:

1. **Desktop**: Standalone application (Tauri)
2. **Mobile**: iOS/Android mobile app (Tauri Mobile)
3. **Server**: Multi-user streaming server with sync

**Core Principle**: Shared Rust logic across all platforms, with platform-specific adapters for I/O.

---

## High-Level Architecture

The architecture consists of three layers: soul-core provides platform-agnostic traits and business logic at the base. The middle layer contains soul-storage (SQLite), soul-audio (Symphonia), and soul-metadata (tag I/O) which implement the core traits. The top layer has platform-specific adapters: Desktop and Mobile use CPAL for audio output, and Server uses Axum/Tokio for HTTP endpoints.

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
- Mobile: SQLite file in app data directory
- Server: Same SQLite schema (single DB, multi-user)

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
pub struct CpalOutput { /* Desktop/Mobile via CPAL */ }

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

## Frontend Architecture (Cross-Platform UI)

### **@soul-player/shared Package**

The shared package provides a **platform-agnostic React/TypeScript UI library** that works identically across desktop, web (server mode), web (demo mode), and marketing demo. All platforms use the **exact same** pages and components with zero duplication.

#### **Core Abstraction: Provider Pattern**

All platform-specific operations are abstracted behind context providers that implement standard interfaces:

```typescript
// Backend Interface - Data operations (read/write)
export interface BackendInterface {
  // Library data
  getAllTracks(): Promise<BackendTrack[]>
  getAllAlbums(): Promise<BackendAlbum[]>
  getAlbumTracks(albumId: number): Promise<BackendTrack[]>

  // Playlist operations
  createPlaylist(name: string): Promise<BackendPlaylist>
  addTrackToPlaylist(playlistId: string, trackId: number): Promise<void>

  // ... 30+ methods covering all data operations
}

// Player Commands Interface - Playback control only
export interface PlayerCommandsInterface {
  playQueue(queue: QueueTrack[], startIndex: number): Promise<void>
  pausePlayback(): Promise<void>
  skipNext(): Promise<void>
  setVolume(volume: number): Promise<void>
  setShuffle(enabled: boolean): Promise<void>
  setRepeatMode(mode: 'off' | 'all' | 'one'): Promise<void>

  // ... playback control methods only
}
```

**CRITICAL SEPARATION**: `BackendInterface` handles data fetching, `PlayerCommandsInterface` handles playback control. Never mix the two - this ensures clean separation of concerns and prevents duplicate code.

#### **Provider Implementations**

Each platform provides its own implementation:

| Provider | Platform | Data Source | Used By |
|----------|----------|-------------|---------|
| **TauriBackendProvider** | Desktop | Tauri `invoke()` commands | Desktop app |
| **MockBackendProvider** | Demo | In-memory `DemoStorage` | Marketing demo, Web demo mode |
| **ServerBackendProvider** | Web/Mobile | REST API to Soul Server | Web server mode, Mobile (future) |
| **AbstractBackendProvider** | Base class | Throws "not implemented" | Documentation, testing |

#### **Shared Components Structure**

```
@soul-player/shared/
├── pages/                  # 11 full pages (works on all platforms)
│   ├── HomePage.tsx
│   ├── LibraryPage.tsx
│   ├── AlbumsPage.tsx, ArtistsPage.tsx, PlaylistsPage.tsx
│   ├── AlbumPage.tsx, ArtistPage.tsx, PlaylistPage.tsx
│   ├── NowPlayingPage.tsx
│   └── SettingsPage.tsx
│
├── components/             # 75+ reusable components
│   ├── player/            # PlayerFooter, PlayerControls, VolumeControl
│   ├── library/           # AlbumCard, TrackList, ArtworkImage
│   ├── settings/          # Audio effects, EQ, compressor editors
│   └── ui/                # Shadcn-style primitives
│
├── contexts/              # Platform abstraction
│   ├── BackendContext.tsx         # Data operations interface
│   ├── PlayerCommandsContext.tsx  # Playback control interface
│   ├── PlatformContext.tsx        # Feature flags & platform detection
│   └── LibraryDataContext.tsx
│
├── providers/             # Reusable provider implementations
│   ├── AbstractBackendProvider.tsx  # Base class with defaults
│   ├── MockBackendProvider.tsx      # Demo/test data
│   └── ServerBackendProvider.tsx    # REST API client
│
├── stores/                # Zustand state management
│   ├── player.ts          # Current track, progress, volume
│   ├── library.ts         # Cached library data
│   └── sync.ts            # Multi-device sync state
│
├── lib/                   # Utilities
│   └── demo-storage.ts    # Demo data management
│
└── theme/                 # Theme system with built-in themes
    ├── ThemeManager.ts
    └── themes.ts          # dark, light, ocean, earth

**Exports:**
- 11 pages, 75+ components, 5 contexts, 3 providers
- Theme system, i18n system, utilities
- Types: Track, Album, Artist, Playlist, QueueTrack
- Total: ~12,000 LOC of shared UI code
```

#### **Platform-Specific Code**

Each platform only needs to implement 2-3 providers:

**Desktop (applications/desktop):**
```typescript
// Implements BackendInterface using Tauri commands
export function TauriBackendProvider({ children }) {
  const backend = useMemo(() => ({
    async getAllTracks() {
      return invoke<BackendTrack[]>('get_all_tracks')
    },
    async createPlaylist(name: string) {
      return invoke<BackendPlaylist>('create_playlist', { name })
    },
    // ... all methods map to Tauri commands
  }), [])

  return <BackendProvider value={backend}>{children}</BackendProvider>
}

// Usage:
<TauriBackendProvider>
  <TauriPlayerCommandsProvider>
    <AlbumsPage />  {/* Works identical to web! */}
  </TauriPlayerCommandsProvider>
</TauriBackendProvider>
```

**Marketing Demo (applications/marketing):**
```typescript
// Uses MockBackendProvider from shared + WASM playback
const demoStorage = new DemoStorage()
await demoStorage.loadFromJson('/demo-data.json')

<MockBackendProvider storage={demoStorage}>
  <DemoPlayerCommandsProvider storage={demoStorage}>
    <AlbumsPage />  {/* Same component as desktop! */}
  </DemoPlayerCommandsProvider>
</MockBackendProvider>
```

**Web App (applications/web):**
```typescript
// Supports both demo mode and server mode
function WebApp() {
  const [mode, setMode] = useState<'demo' | 'server'>('demo')

  if (mode === 'demo') {
    // Local demo with no authentication
    return (
      <MockBackendProvider storage={demoStorage}>
        <WebPlayerCommandsProvider>
          <AlbumsPage />  {/* Same as desktop & marketing! */}
        </WebPlayerCommandsProvider>
      </MockBackendProvider>
    )
  } else {
    // Server mode with authentication
    return (
      <ServerBackendProvider apiBase="/api" authToken={token}>
        <WebPlayerCommandsProvider>
          <AlbumsPage />  {/* Still the same component! */}
        </WebPlayerCommandsProvider>
      </ServerBackendProvider>
    )
  }
}
```

#### **Feature Gating**

Different platforms have different capabilities. The `PlatformContext` enables conditional rendering:

```typescript
<PlatformProvider
  platform="web"
  features={{
    canDeleteTracks: false,        // Read-only in demo mode
    canCreatePlaylists: serverMode, // Only in server mode
    hasAudioSettings: false,        // No EQ/effects on web
    hasKeyboardShortcuts: false,    // Desktop only
  }}
>
  {/* Components automatically hide unavailable features */}
  <FeatureGate feature="canCreatePlaylists">
    <CreatePlaylistButton />  {/* Only shows if enabled */}
  </FeatureGate>
</PlatformProvider>
```

#### **Command Flow Example**

User clicks "Play Album" button:

1. **UI Component** (AlbumPage.tsx in shared):
   ```typescript
   const backend = useBackend()          // Get backend
   const commands = usePlayerCommands()   // Get player commands

   async function playAlbum(albumId: number) {
     // 1. Fetch data via BackendInterface
     const tracks = await backend.getAlbumTracks(albumId)

     // 2. Transform to queue format
     const queue = tracks.map(t => ({
       trackId: String(t.id),
       title: t.title,
       filePath: t.file_path!,
       // ...
     }))

     // 3. Play via PlayerCommandsInterface
     await commands.playQueue(queue, 0)
   }
   ```

2. **Desktop Path**:
   ```
   AlbumPage → TauriBackendProvider → invoke('get_album_tracks')
            → TauriPlayerCommandsProvider → invoke('play_queue')
            → Rust playback manager → Audio output
   ```

3. **Web Demo Path**:
   ```
   AlbumPage → MockBackendProvider → demoStorage.getAlbumTracks()
            → DemoPlayerCommandsProvider → WASM playback → Web Audio API
   ```

4. **Web Server Path**:
   ```
   AlbumPage → ServerBackendProvider → fetch('/api/albums/:id/tracks')
            → WebPlayerCommandsProvider → HTMLAudioElement + server sync
   ```

**Result:** Same UI code, different backend implementations, identical user experience.

#### **Code Reusability Metrics**

| Component | Desktop | Marketing | Web | Shared |
|-----------|---------|-----------|-----|--------|
| Pages | ✅ | ✅ | ✅ | 11 pages (100%) |
| Components | ✅ | ✅ | ✅ | 75+ components (100%) |
| Stores | ✅ | ✅ | ✅ | 3 Zustand stores (100%) |
| Theme System | ✅ | ✅ | ✅ | Full theme engine (100%) |
| i18n | ✅ | ✅ | ✅ | All translations (100%) |
| **Backend Logic** | TauriBackend (~160 LOC) | MockBackend (shared) | MockBackend + ServerBackend (shared) | 100% reusable |
| **Platform Code** | ~400 LOC | ~200 LOC | ~300 LOC | <5% unique per platform |

**Total Shared**: ~12,000 LOC
**Platform-Specific**: ~400 LOC per platform (3%)
**Reusability**: 97%

#### **Benefits of This Architecture**

1. **Zero UI Duplication**: All platforms use identical pages/components
2. **Easy to Add Platforms**: Implement 2 providers (~400 LOC) → get 11 pages + 75 components for free
3. **Consistent UX**: Users get identical experience across desktop, web, mobile
4. **Testability**: Mock providers enable easy testing without real backend
5. **Flexibility**: Platforms can mix providers (e.g., web demo mode + server mode)
6. **Type Safety**: TypeScript ensures provider implementations match interfaces

#### **Future Platforms**

Adding new platforms is trivial:

**Mobile (React Native):**
```typescript
class NativeBackendProvider extends AbstractBackendProvider {
  async getAllTracks() {
    return NativeModules.SoulPlayer.getAllTracks()
  }
  // ... implement methods using native modules
}

// Then use shared pages:
<NativeBackendProvider>
  <AlbumsPage />  {/* Works immediately! */}
</NativeBackendProvider>
```

**CLI (Ink.js):**
```typescript
// Terminal-based UI using same data layer
<TauriBackendProvider>
  <TerminalAlbumsList />  {/* Custom rendering, shared data */}
</TauriBackendProvider>
```

**Browser Extension:**
```typescript
<ServerBackendProvider apiBase="https://my-server.com/api">
  <MiniPlayer />  {/* Compact UI, shared logic */}
</ServerBackendProvider>
```

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
