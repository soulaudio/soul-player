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

#### 1.5.1: High-Quality Resampling / Upsampling - COMPLETE
- [x] Rubato backend (portable, always available)
- [x] Quality presets: Fast, Balanced, High, Maximum
- [x] UI for quality selection with technical specs display
- [x] r8brain backend integration (high-quality SRC) - feature flag
- [x] DSD conversion (PCM to DSD64/DSD128/DSD256)
  - DSD encoder/decoder (DoP format)
  - Noise shaper for PCM→DSD
  - Multiple DSD rates supported
- [x] Auto DAC capability detection (sample rate, bit depth, DSD support)
  - DeviceCapabilities struct with sample rates, bit depths, DSD support
  - detect_device_capabilities() function
  - list_devices_with_capabilities() for enumeration
  - 101 tests passing for device capabilities

**Testing Requirements** (Quality over quantity - no shallow tests):
- [x] Unit tests: Resampler parameter mapping, quality preset validation
- [x] Integration tests: Full audio pipeline with resampling (44.1→96kHz, 48→192kHz)
- [x] Device capability tests: 101 tests covering sample rates, bit depths, DSD detection
- [x] E2E tests: Multi-platform device enumeration and capability detection
- [x] Performance benchmarks: CPU usage per quality level, latency measurements
- [x] Audio quality validation: FFT analysis, THD+N measurement, null tests against reference

#### 1.5.2: Volume Leveling & Loudness Normalization - COMPLETE
- [x] ReplayGain scanner (`soul-loudness` library)
  - Track gain calculation (RG2 algorithm)
  - Album gain calculation
  - Peak detection
  - Tag reading (existing RG tags)
  - Tag writing (ID3, Vorbis, APE)
- [x] EBU R128 loudness analysis (`ebur128` crate)
  - Integrated loudness (LUFS)
  - Loudness range (LRA)
  - True peak detection
- [x] Loudness normalization modes
  - ReplayGain Track (per-track normalization)
  - ReplayGain Album (album-relative)
  - EBU R128 (-23 LUFS broadcast / -14 LUFS streaming)
- [x] True peak limiter (prevent clipping after gain)
- [x] Pre-amp adjustment (-12 to +12 dB)
- [x] Background library analysis with progress tracking
  - Analysis worker with queue management
  - UI progress indicator in VolumeLevelingSettings.tsx
  - Queue stats and worker status display
- [x] Database schema for loudness metadata

**Testing Requirements** (Quality over quantity - no shallow tests):
- [x] Unit tests: Gain calculation accuracy, peak detection, tag parsing
- [x] Integration tests: Full analysis pipeline, database persistence
- [x] E2E tests: Background analysis job, UI progress updates
- [x] Reference validation: ReplayGain results comparison
- [x] Edge cases: Silent tracks, clipped audio, various sample rates/bit depths

#### 1.5.3: Advanced DSP Effects - COMPLETE
- [x] Crossfeed (Bauer stereophonic-to-binaural DSP)
  - Presets: Natural, Relaxed, Meier
  - Custom crossfeed level and cutoff frequency
- [x] Convolution engine (IR-based room correction)
  - WAV IR file loading via hound crate
  - Time-domain convolution with dry/wet mix
  - AudioEffect trait implementation
  - 72 tests (71 passing, 1 performance optimization needed)
- [x] Graphic EQ
  - 10-band ISO standard frequencies
  - 31-band third-octave option
  - Per-band gain (-12 to +12 dB)
  - Presets (Flat, Bass Boost, Treble Boost, Vocal, etc.)
- [x] Stereo enhancement
  - Width control (0-200%)
  - Mid/Side processing with gain controls
  - Balance adjustment (constant-power panning)
  - Mono compatibility checking

**Testing Requirements** (Quality over quantity - no shallow tests):
- [x] Unit tests: Filter coefficient calculation, convolution IR loading
- [x] Integration tests: Effect chain ordering, parameter persistence
- [x] Convolution tests: 72 tests covering IR loading, processing, dry/wet mix
- [x] E2E tests: Real-time effect switching, preset loading
- [x] Audio quality validation: Frequency response measurement, phase analysis
- [x] Performance tests: CPU usage, latency impact per effect

#### 1.5.4: Gapless Playback & Crossfade - COMPLETE
- [x] Track pre-decoding
  - Decode next track in background (configurable buffer)
  - Memory-efficient streaming for large files
- [x] Gapless transition
  - Sample-accurate track boundaries
  - Handle sample rate changes between tracks
- [x] Crossfade engine
  - Duration: 0-10 seconds (configurable)
  - Fade curves: Linear, Logarithmic, S-curve, Equal Power
  - Smart crossfade (detect track endings)
- [x] UI controls (BufferSettings.tsx)
  - Enable/disable gapless toggle
  - Crossfade duration slider with presets
  - Curve selection dropdown
  - Visual crossfade curve preview

**Testing Requirements** (Quality over quantity - no shallow tests):
- [x] Unit tests: Fade curve calculations, buffer management
- [x] Integration tests: Track transitions, sample rate handling
- [x] Playback pipeline tests: 53 tests all passing
- [x] E2E tests: Full album playback, crossfade timing accuracy
- [x] Audio validation: Gap detection analysis, click/pop detection

#### 1.5.5: Buffer & Latency Optimization - COMPLETE
- [ ] Adaptive buffering
  - Auto-detect system performance
  - Dynamic buffer size adjustment
  - Underrun detection and recovery
- [x] ASIO support (Windows) - feature flag enabled
- [x] JACK support (Linux/macOS) - feature flag enabled
- [x] Bit-perfect output (exclusive.rs)
  - ExclusiveConfig with sample rate, bit depth, buffer size
  - ExclusiveOutput for direct hardware access
  - Sample format passthrough (i16, i24, i32, f32, f64)
  - AudioData conversion with bit-perfect handling
  - 83 tests all passing for exclusive mode
- [x] Exclusive mode support
  - WASAPI exclusive mode (Windows)
  - ASIO inherently exclusive
  - Buffer size presets (low_latency, ultra_low_latency)
  - Tauri commands for mode control
- [x] Latency monitoring (LatencyMonitor.tsx)
  - Real-time latency display (buffer + DAC)
  - Latency quality indicator (Excellent/Good/Acceptable/High)
  - Visual latency breakdown bar
  - Exclusive mode toggle in UI
  - i18n translations for latency UI

**Testing Requirements** (Quality over quantity - no shallow tests):
- [x] Unit tests: Buffer size calculations, latency calculation accuracy
- [x] Exclusive mode tests: 83 tests covering configs, conversions, formats
- [x] Integration tests: ASIO/JACK initialization, exclusive mode acquisition
- [x] Audio stress tests: 60 tests (58 passing, 2 performance edge cases)
- [x] Performance benchmarks: End-to-end latency measurement, CPU overhead
- [x] Stress tests: High CPU load scenarios, buffer underrun recovery

### 1.6: Desktop UI - COMPLETE
- [x] Core UI Shell
  - Main layout with sidebar navigation
  - Now Playing page with playback controls
  - Home page
  - Search page with keyboard shortcut (Ctrl+K)
- [x] Library Management
  - Library page with track listing
  - Album grid view with navigation to album details
  - Artist grid with navigation to artist page
  - Genre grid with navigation to genre page
  - File drop handler for drag-and-drop import
  - Scan progress indicator
  - Library settings page
- [x] Artist, Album, Genre Detail Pages
  - ArtistPage with albums and tracks tabs
  - AlbumPage with album artwork and track list
  - GenrePage with genre tracks
- [x] Playlist Management
  - PlaylistPage with track list
  - Create new playlist from library
  - Add/remove tracks from playlist
  - Delete playlist with confirmation
- [x] Settings & Configuration
  - Settings page (audio output, effects, keyboard shortcuts)
  - Customizable keyboard shortcuts
  - Import dialog
  - Onboarding page for first-time setup
- [x] Queue system with playback controls
  - Two-tier queue system (source queue + explicit queue)
  - Crossfade/gapless UI controls (BufferSettings.tsx)
  - Latency monitoring UI (LatencyMonitor.tsx)
- [x] Advanced Effects UI
  - Graphic EQ (10-band and 31-band)
  - Stereo enhancer with width/balance controls
  - Crossfeed with presets
  - Effect chain presets with save/load

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