# Soul Player - Development Roadmap

## Vision
A local-first, cross-platform music player with server streaming capabilities and embedded hardware support (ESP32-S3 DAP).

## Architecture Principles
- **Shared Core Logic**: Platform-agnostic Rust crates for all platforms
- **Vertical Slicing**: Feature-complete across all layers when possible
- **Quality Over Quantity**: Deep, meaningful tests - no shallow coverage
- **Embedded-First Thinking**: Core logic must work on resource-constrained devices

---

## Phase 1: Desktop Foundation (Local-First)
**Goal**: Functional local music player with library management

### 1.1: Project Structure - COMPLETE
- [x] Monorepo Setup (Cargo + Yarn workspaces)
- [x] Libraries vs Applications separation
- [x] CI/CD workflows (lint, test, audit, build)
- [x] Tauri 2.0 Desktop Scaffold (React + TypeScript + Vite)
- [x] Shared component library

### 1.2: Storage Foundation
- [ ] Multi-Source Storage (`soul-storage`)
  - SQLite schema with multi-user support
  - Source tracking (local, server A, server B, etc.)
  - Track availability states (local_file, cached, stream_only)
  - Artists, Albums, Genres (separate tables)
  - Playlists with sharing support
  - SQLx migrations (embedded in binary)
- [ ] StorageContext Trait (`soul-core`)
  - LocalStorageContext (SQLite implementation)
  - User context (always carries user_id)

### 1.3: Import & Metadata
- [ ] Intelligent Import (`soul-import`)
  - File scanning and tag reading (Lofty crate)
  - Artist name normalization
  - Fuzzy matching against existing entities
  - Deduplication via MusicBrainz ID + fingerprinting
- [ ] Metadata Processing (`soul-metadata`)
  - Tag reading/writing (ID3, Vorbis, etc.)
  - Library scanning and indexing
  - Album art extraction

### 1.4: Audio Playback - COMPLETE
- [x] Audio Engine (`soul-audio`)
  - Symphonia decoder (MP3, FLAC, OGG, WAV, AAC, OPUS)
  - CPAL output for desktop (`soul-audio-desktop`)
  - Effect chain architecture (trait-based)
  - 3-band parametric EQ
  - Dynamic range compressor
  - Brick-wall limiter
  - Volume control
- [x] DSP Effect Chain
  - Add/remove/reorder effects in realtime
  - Per-effect enable/disable toggle
  - Effect presets (EQ, Compressor, Limiter)
  - Chain presets with database persistence
- [x] Testing & Validation
  - E2E tests for DSP verification
  - FFT-based frequency analysis
  - THD measurement
- [x] Playback Integration
  - Tauri commands wired to audio engine
  - Play, pause, stop, seek
  - Queue management (two-tier system)
  - Shuffle (random + smart)
  - Repeat modes

### 1.5: Advanced Audio Processing

#### 1.5.1: High-Quality Resampling / Upsampling
- [x] Rubato backend (portable, always available)
- [x] Quality presets: Fast, Balanced, High, Maximum
- [x] UI for quality selection with technical specs display
- [ ] r8brain backend integration (high-quality SRC)
- [ ] DSD conversion (PCM to DSD64/DSD128/DSD256)
- [ ] Auto DAC capability detection (sample rate, bit depth, DSD support)

**Testing Requirements** (Quality over quantity - no shallow tests):
- Unit tests: Resampler parameter mapping, quality preset validation
- Integration tests: Full audio pipeline with resampling (44.1→96kHz, 48→192kHz)
- E2E tests with testcontainers: Multi-platform audio output verification
- Performance benchmarks: CPU usage per quality level, latency measurements
- Audio quality validation: FFT analysis, THD+N measurement, null tests against reference

#### 1.5.2: Volume Leveling & Loudness Normalization
- [ ] ReplayGain scanner (`soul-loudness` library)
  - Track gain calculation (RG2 algorithm)
  - Album gain calculation
  - Peak detection
  - Tag reading (existing RG tags)
  - Tag writing (ID3, Vorbis, APE)
- [ ] EBU R128 loudness analysis (`ebur128` crate)
  - Integrated loudness (LUFS)
  - Loudness range (LRA)
  - True peak detection
- [ ] Loudness normalization modes
  - ReplayGain Track (per-track normalization)
  - ReplayGain Album (album-relative)
  - EBU R128 (-23 LUFS broadcast / -14 LUFS streaming)
- [ ] True peak limiter (prevent clipping after gain)
- [ ] Pre-amp adjustment (-12 to +12 dB)
- [ ] Background library analysis with progress tracking
- [ ] Database schema for loudness metadata

**Testing Requirements** (Quality over quantity - no shallow tests):
- Unit tests: Gain calculation accuracy, peak detection, tag parsing
- Integration tests: Full analysis pipeline, database persistence
- E2E tests with testcontainers: Background analysis job, UI progress updates
- Reference validation: Compare against foobar2000/Audacity ReplayGain results
- Edge cases: Silent tracks, clipped audio, various sample rates/bit depths

#### 1.5.3: Advanced DSP Effects
- [ ] Crossfeed (Bauer stereophonic-to-binaural DSP)
  - Presets: Natural, Relaxed, Meier
  - Custom crossfeed level and cutoff frequency
- [ ] Convolution engine (IR-based room correction)
  - WAV IR file loading
  - Partitioned convolution for low latency
  - Dry/wet mix control
- [ ] Graphic EQ
  - 10-band ISO standard frequencies
  - 31-band third-octave option
  - Per-band gain (-12 to +12 dB)
  - Presets (Flat, Bass Boost, Treble Boost, etc.)
- [ ] Stereo enhancement
  - Width control (0-200%)
  - Mid/Side processing
  - Balance adjustment

**Testing Requirements** (Quality over quantity - no shallow tests):
- Unit tests: Filter coefficient calculation, convolution correctness
- Integration tests: Effect chain ordering, parameter persistence
- E2E tests with testcontainers: Real-time effect switching, preset loading
- Audio quality validation: Frequency response measurement, phase analysis
- Performance tests: CPU usage, latency impact per effect

#### 1.5.4: Gapless Playback & Crossfade
- [ ] Track pre-decoding
  - Decode next track in background (configurable buffer)
  - Memory-efficient streaming for large files
- [ ] Gapless transition
  - Sample-accurate track boundaries
  - Handle sample rate changes between tracks
- [ ] Crossfade engine
  - Duration: 0-10 seconds (configurable)
  - Fade curves: Linear, Logarithmic, S-curve, Equal Power
  - Smart crossfade (detect track endings)
- [ ] UI controls
  - Enable/disable gapless
  - Crossfade duration slider
  - Curve selection

**Testing Requirements** (Quality over quantity - no shallow tests):
- Unit tests: Fade curve calculations, buffer management
- Integration tests: Track transitions, sample rate handling
- E2E tests with testcontainers: Full album playback, crossfade timing accuracy
- Audio validation: Gap detection analysis, click/pop detection

#### 1.5.5: Buffer & Latency Optimization
- [ ] Adaptive buffering
  - Auto-detect system performance
  - Dynamic buffer size adjustment
  - Underrun detection and recovery
- [x] ASIO support (Windows) - feature flag enabled
- [x] JACK support (Linux/macOS) - feature flag enabled
- [ ] Bit-perfect output
  - Bypass OS mixer when possible
  - Sample format passthrough
  - Exclusive mode support (WASAPI, CoreAudio)
- [ ] Latency monitoring
  - Real-time latency display
  - Buffer fill level indicator

**Testing Requirements** (Quality over quantity - no shallow tests):
- Unit tests: Buffer size calculations, underrun detection logic
- Integration tests: ASIO/JACK initialization, exclusive mode acquisition
- E2E tests with testcontainers: Platform-specific audio stack testing
- Performance benchmarks: End-to-end latency measurement, CPU overhead
- Stress tests: High CPU load scenarios, buffer underrun recovery

### 1.6: Desktop UI
- [ ] Complete Desktop UI
  - Library view (tracks, albums, artists, genres)
  - Playback controls and progress bar
  - Playlist management (create, edit, reorder)
  - Queue system with drag-and-drop
  - File picker for library scanning
  - Settings page (audio output, effects, theme)
  - Import wizard

### Success Criteria
- Scan local music folders with smart deduplication
- Play all supported formats (MP3, FLAC, OGG, WAV, AAC, OPUS)
- Create and manage playlists
- Apply professional-grade DSP effects
- Beautiful, responsive UI
- 10,000+ track library performs well

---

## Phase 2: Multi-Source & Server Sync
**Goal**: Connect to multiple servers, sync libraries, offline support

**See detailed architecture docs**:
- [MULTI_SOURCE_ARCHITECTURE.md](docs/architecture/MULTI_SOURCE_ARCHITECTURE.md)
- [SYNC_STRATEGY.md](docs/architecture/SYNC_STRATEGY.md)
- [OFFLINE_MODE.md](docs/architecture/OFFLINE_MODE.md)

### 2.1: Source Management
- [ ] Source Configuration (`soul-storage`)
  - Sources table (local + multiple servers)
  - Active server selection
  - Connection status tracking
- [ ] Source Manager (`soul-core`)
  - Add/remove server sources
  - Connection monitoring and health checks

### 2.2: Server Implementation
- [ ] Server Binary (`soul-server`)
  - Multi-user authentication (JWT)
  - REST API for library access
  - Audio streaming with range requests
  - Docker container support

### 2.3: Sync Engine
- [ ] Sync Protocol (`soul-sync`)
  - Initial sync (full metadata fetch)
  - Incremental sync (cursor-based delta)
  - Conflict resolution strategies
  - Deduplication
- [ ] Offline Queue
  - Queue local changes when offline
  - Auto-sync when connection restored

### 2.4: Download & Cache
- [ ] Download Manager
  - Background download worker
  - Progress tracking and resumable downloads
  - Smart download strategies
- [ ] Cache Management
  - Content-addressable file storage
  - LRU eviction policy
  - Pin tracks (never evict)

### Success Criteria
- Connect to multiple servers
- Library syncs from active server
- Stream tracks from server when online
- Download tracks for offline use
- Automatic fallback to local/cached when offline
- Easy Docker deployment for server

---

## Phase 2.5: Soul Connect (Multi-Device & Collaboration)
**Goal**: Spotify Connect-style device control, automatic cross-device sync, and collaborative listening (Jam sessions)

**See detailed architecture docs**:
- [CONNECT_ARCHITECTURE.md](docs/architecture/CONNECT_ARCHITECTURE.md)
- [JAM_SESSIONS.md](docs/architecture/JAM_SESSIONS.md)

### Overview

Soul Connect enables three key features:
1. **Device Presence** - See all your devices, know which is playing
2. **Remote Playback Control** - Control any device from any other device
3. **Jam Sessions** - Collaborative listening with shareable links

### Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         soul-server                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │
│  │  REST API    │  │  WebSocket   │  │   Session    │               │
│  │  /api/*      │  │  /ws/*       │  │   Manager    │               │
│  └──────────────┘  └──────────────┘  └──────────────┘               │
│                           │                                          │
│              ┌────────────┴────────────┐                            │
│              │    Device Registry      │                            │
│              │    Playback Router      │                            │
│              │    Jam Session Store    │                            │
│              └─────────────────────────┘                            │
└─────────────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┬──────────────────┐
        │                   │                   │                  │
        ▼                   ▼                   ▼                  ▼
   ┌─────────┐        ┌─────────┐        ┌─────────┐        ┌─────────┐
   │ Desktop │        │ Mobile  │        │  DAP    │        │  Web    │
   │ Render  │        │ Render  │        │ Render  │        │  Jam    │
   │ + Ctrl  │        │ + Ctrl  │        │  only   │        │  Guest  │
   └─────────┘        └─────────┘        └─────────┘        └─────────┘
```

### 2.5.1: Device Presence & Registry
- [ ] Device Registry (`soul-server`)
  - Device registration on WebSocket connect
  - Device metadata (name, type, capabilities)
  - Heartbeat/presence system (detect disconnects)
  - Per-user device list
  - Last active device tracking

- [ ] Database Schema (`soul-storage`)
  - `user_devices` table (id, user_id, device_name, device_type, capabilities, last_seen)
  - `device_sessions` table (active WebSocket sessions)
  - Device capability flags (can_render, can_control, can_host_jam)

- [ ] Client Integration (`soul-connect` library)
  - Auto-register on app startup
  - Reconnect with exponential backoff
  - Device naming (user-configurable)
  - Platform detection (desktop/mobile/dap/web)

- [ ] UI Components
  - Device selector dropdown (in player controls)
  - "Playing on [Device]" indicator
  - Device management in settings

### 2.5.2: Remote Playback Control (Soul Connect)
- [ ] Playback State Sync (`soul-server`)
  - Real-time state broadcast via WebSocket
  - State: track, position, playing/paused, queue, volume
  - Conflict resolution (latest timestamp wins)
  - State persistence (resume on reconnect)

- [ ] Transfer Playback (`soul-connect`)
  - Transfer command (from device A to device B)
  - Seamless handoff (continue from same position)
  - Queue transfer with playback
  - Handle offline target device gracefully

- [ ] Control Protocol
  - WebSocket messages: Play, Pause, Seek, Next, Previous, SetQueue
  - Server validates permissions, routes to target device
  - Acknowledgment system (confirm command received)
  - Optimistic UI updates with rollback

- [ ] Background Sync Enhancement
  - Automatic sync interval (configurable, default 30s)
  - Push-based updates via WebSocket (instant)
  - Sync queue, playlists, play history
  - Bandwidth-efficient delta sync

- [ ] UI Components
  - Device picker in player bar
  - "Available devices" panel
  - Transfer playback button
  - Remote volume control

### 2.5.3: Jam Sessions (Collaborative Listening)
- [ ] Session Management (`soul-server`)
  - Create session (returns session_id + share_code)
  - Join session (by share_code or direct link)
  - Leave session / end session
  - Session state: host, participants, queue, settings
  - Auto-cleanup inactive sessions

- [ ] Database Schema (`soul-storage`)
  - `jam_sessions` table (id, host_user_id, host_device_id, share_code, settings, is_active)
  - `jam_participants` table (session_id, user_id, display_name, permissions, joined_at)
  - `jam_queue` table (session_id, track_id, added_by, position, played_at)

- [ ] Shareable Links
  - Generate short codes (e.g., `ABCD-1234`)
  - Public URL: `https://server/jam/{code}`
  - QR code generation for in-person sharing
  - Link expiration settings

- [ ] Collaborative Queue
  - Add tracks to shared queue
  - Vote to skip (optional mode)
  - Remove own additions
  - Host can remove any / reorder
  - "Who added this" attribution

- [ ] Permissions System
  - Host: full control (skip, remove, end session, kick)
  - Member: add to queue, remove own tracks
  - Guest (anonymous): add to queue only (configurable)
  - Approval mode: host approves new joiners

- [ ] Guest Web UI (`applications/jam-web`)
  - Minimal React app for guests
  - No install required (works in browser)
  - Join by link, enter display name
  - View current track, queue
  - Add tracks (search server library)
  - Real-time updates via WebSocket

- [ ] Notifications
  - "[User] joined the session"
  - "[User] added [Track] to queue"
  - "You're up next!"
  - Host notifications for join requests

### 2.5.4: Testing & Documentation
- [ ] Integration Tests
  - Device registration/deregistration
  - Playback transfer between devices
  - State sync accuracy
  - Jam session lifecycle
  - Guest join flow
  - Permission enforcement
  - Reconnection handling

- [ ] Load Testing
  - Multiple concurrent devices per user
  - Large jam sessions (10+ participants)
  - High-frequency state updates
  - WebSocket connection limits

### New Library: `soul-connect`

Platform-agnostic library for multi-device features:

```rust
// libraries/soul-connect/src/lib.rs

pub struct ConnectClient {
    server_url: String,
    device_info: DeviceInfo,
    ws_connection: Option<WebSocketConnection>,
}

impl ConnectClient {
    /// Register this device with the server
    pub async fn register(&mut self) -> Result<DeviceId>;

    /// Get all devices for current user
    pub async fn list_devices(&self) -> Result<Vec<Device>>;

    /// Transfer playback to another device
    pub async fn transfer_playback(&self, target: DeviceId) -> Result<()>;

    /// Subscribe to playback state changes
    pub fn on_playback_state<F>(&mut self, callback: F)
    where F: Fn(PlaybackState) + Send + 'static;

    /// Create a new jam session
    pub async fn create_jam(&self, settings: JamSettings) -> Result<JamSession>;

    /// Join existing jam session
    pub async fn join_jam(&self, share_code: &str) -> Result<JamSession>;
}
```

### WebSocket Protocol

```
// Client -> Server
{ "type": "register", "device": { "name": "MacBook Pro", "type": "desktop" } }
{ "type": "playback_command", "target_device": "uuid", "command": "play" }
{ "type": "transfer_playback", "to_device": "uuid" }
{ "type": "jam_create", "settings": { ... } }
{ "type": "jam_join", "share_code": "ABCD-1234" }
{ "type": "jam_add_track", "session_id": "uuid", "track_id": "uuid" }

// Server -> Client
{ "type": "devices_update", "devices": [...] }
{ "type": "playback_state", "state": { "track_id": "...", "position_ms": 12345, ... } }
{ "type": "jam_update", "session": { "participants": [...], "queue": [...] } }
{ "type": "jam_notification", "message": "Alex joined the session" }
```

### Success Criteria
- All user devices visible in device selector
- Can transfer playback between devices seamlessly
- Playback state syncs within 500ms across devices
- Jam sessions work with shareable links
- Guests can join via web browser without account
- Collaborative queue updates in real-time
- Works reliably over internet (not just local network)
- Handles disconnections gracefully

---

## Phase 3: ESP32-S3 Embedded DAP
**Goal**: Portable hardware music player

### Deliverables
- [ ] ESP32-S3 Firmware (`soul-player-esp32`)
  - awedio_esp32 audio playback
  - Symphonia decoder (shared with desktop)
  - SD card filesystem integration
  - SQLite database (same schema)
  - WiFi server sync
  - Battery management
- [ ] E-ink Display Driver
  - Album art display
  - Track info
  - Menu system
- [ ] Hardware Integration
  - I2S DAC output
  - Button controls

### Success Criteria
- Plays music from SD card
- E-ink display shows track info
- Syncs library with server over WiFi
- 8+ hours battery life

---

## Phase 4: Discovery & Advanced Features
**Goal**: Music discovery and recommendations

### Deliverables
- [ ] Discovery Service (`soul-discovery`)
  - Bandcamp API integration
  - Discogs metadata enrichment
  - Similar track algorithm
  - Genre-based recommendations

- [ ] Mobile Support (`soul-player-mobile`)
  - Tauri Mobile implementation (iOS/Android)
  - Touch-optimized UI
  - Background playback
  - Notification controls

- [ ] Social Features
  - Playlist sharing links
  - Collaborative playlists
  - Listen history
  - Statistics dashboard

---

## Future Considerations (Post-MVP)

### Audio Formats
- [ ] DSD support (DSD-to-PCM conversion)
- [ ] High-res streaming (24-bit/192kHz)

### Advanced Effects
- [ ] Reverb
- [ ] Pitch shifting
- [ ] Time stretching
- [ ] Custom plugin system

---

## Release Strategy

### Desktop Releases
- GitHub Releases with binaries for:
  - Windows (x64, ARM64)
  - macOS (Intel, Apple Silicon)
  - Linux (x64, ARM64, AppImage/Flatpak)

### Server Releases
- Docker images (multi-arch)
- Standalone binaries

### Embedded Releases
- Firmware binaries (.bin)
- OTA update system
- Hardware documentation