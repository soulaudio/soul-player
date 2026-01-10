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

### Deliverables

#### 1.1: Project Structure ✅
- [x] **Monorepo Setup**
  - Workspace Cargo.toml with all crates
  - Yarn workspaces for frontend
  - Libraries vs Applications separation
  - CI/CD workflows (lint, test, audit, build)

- [x] **Tauri Desktop Scaffold**
  - React + TypeScript + Vite frontend
  - Tauri 2.0 backend skeleton
  - Shared component library
  - Basic UI with mock data

#### 1.2: Storage Foundation (2-3 weeks)
- [ ] **Multi-Source Storage** (`soul-storage`)
  - SQLite schema with multi-user support
  - **Source tracking** (local, server A, server B, etc.)
  - Track availability states (local_file, cached, stream_only)
  - Artists, Albums, Genres (separate tables)
  - Playlists with sharing support
  - Play history and statistics
  - SQLx migrations (embedded in binary)
  - Vertical slice structure (no repository pattern)

- [ ] **StorageContext Trait** (`soul-core`)
  - Trait defining all storage operations
  - LocalStorageContext (SQLite implementation)
  - User context (always carries user_id)
  - Foundation for future RemoteStorageContext

#### 1.3: Import & Metadata (1-2 weeks)
- [ ] **Intelligent Import** (`soul-import`)
  - File scanning and tag reading (Lofty crate)
  - Artist name normalization and capitalization
  - Fuzzy matching against existing entities
  - Proposal system (create artist? merge duplicates?)
  - Deduplication via MusicBrainz ID + fingerprinting
  - Batch import with progress tracking

- [ ] **Metadata Processing** (`soul-metadata`)
  - Tag reading/writing (ID3, Vorbis, etc.)
  - Library scanning and indexing
  - Album art extraction
  - Metadata enrichment (Phase 4)

#### 1.4: Audio Playback ✅ COMPLETE
- [x] **Audio Engine** (`soul-audio`)
  - Symphonia decoder (MP3, FLAC, OGG, WAV, AAC, OPUS)
  - CPAL output for desktop (`soul-audio-desktop`)
  - Effect chain architecture (trait-based)
  - 3-band parametric EQ
  - Dynamic range compressor
  - Brick-wall limiter
  - Volume control

- [x] **DSP Effect Chain**
  - Add/remove/reorder effects in realtime
  - Per-effect enable/disable toggle
  - Parameter adjustment UI with sliders
  - Effect presets (EQ, Compressor, Limiter)
  - Chain presets with database persistence
  - Built-in presets (Rock, Jazz, Classical, etc.)

- [x] **Testing & Validation**
  - 15 E2E tests for DSP verification
  - FFT-based frequency analysis
  - THD (Total Harmonic Distortion) measurement
  - Compression ratio verification
  - Test signal generation utilities

- [x] **Playback Integration**
  - Wire Tauri commands to audio engine
  - Play, pause, stop, seek
  - Queue management

#### 1.5: Advanced Audio Processing (6-9 weeks)

##### 1.5.1: High-Quality Resampling / Upsampling (1-2 weeks)
- [ ] **Resampling Engine**
  - r8brain algorithm (high-quality SRC)
  - Rubato fallback (portable)
  - Quality levels: Fast, Balanced, High, Maximum
  - Arbitrary sample rate conversion (44.1→96/192kHz)

- [ ] **Upsampling Pipeline**
  - Apply DSP at native rate → upsample → output
  - DSD conversion (PCM → DSD64/DSD128/DSD256)
  - Auto DAC capability detection
  - Manual target rate override

- [ ] **UI Integration**
  - Upsampling quality selector
  - Target sample rate dropdown
  - Real-time rate indicator

##### 1.5.2: Volume Leveling & Loudness Normalization (1-2 weeks)
- [ ] **ReplayGain Support**
  - Read ReplayGain tags (track/album gain)
  - Apply gain with clipping prevention
  - Mode: Track gain / Album gain / Disabled
  - Pre-amp adjustment

- [ ] **EBU R128 Loudness**
  - Real-time loudness analysis (`ebur128` crate)
  - Target LUFS: -23 (broadcast) or -18 (streaming)
  - True peak limiting
  - Background library analysis

- [ ] **Database & UI**
  - Store loudness metadata per track
  - Volume leveling settings page
  - Target loudness slider

##### 1.5.3: Advanced DSP Effects (2-3 weeks)
- [ ] **Crossfeed (Headphone Spatialization)**
  - Bauer stereophonic-to-binaural DSP
  - Adjustable crossfeed amount
  - Presets: Subtle, Moderate, Strong

- [ ] **Convolution (Impulse Response)**
  - Load custom IR files (WAV format)
  - Room correction
  - Virtual speaker emulation
  - Reverb effects

- [ ] **Graphic EQ**
  - 10-band or 31-band option
  - ISO standard frequencies
  - Visual frequency response curve

- [ ] **Stereo Enhancement**
  - Width control (0-200%)
  - M/S processing
  - Pseudo-surround

##### 1.5.4: Gapless Playback & Crossfade (1 week)
- [ ] **Gapless Playback**
  - Pre-decode next track
  - Eliminate silence between tracks
  - Handle sample rate changes

- [ ] **Crossfade**
  - Configurable duration (0-10s)
  - Fade curves: Linear, Logarithmic, S-curve
  - Skip crossfade for live albums

##### 1.5.5: Buffer & Latency Optimization (1 week)
- [ ] **Adaptive Buffering**
  - Auto-adjust based on performance
  - Monitor underruns
  - Low-latency mode

- [ ] **Exclusive Mode Support**
  - ASIO (Windows) - enable existing feature
  - JACK (Linux/macOS) - enable existing feature
  - Bit-perfect output

#### 1.6: Desktop UI (1-2 weeks)
- [ ] **Complete Desktop UI**
  - Library view (tracks, albums, artists, genres)
  - Playback controls and progress bar
  - Playlist management (create, edit, reorder)
  - Queue system with drag-and-drop
  - File picker for library scanning
  - Settings page (audio output, effects, theme)
  - Import wizard with proposal review

- [ ] **Testing**
  - Unit tests for core logic
  - Integration tests with testcontainers (SQLite)
  - Tauri command tests
  - Audio engine tests
  - Target: 50-60% coverage (meaningful tests only)

### Success Criteria
- Scan local music folders with smart deduplication
- Play all supported formats (MP3, FLAC, OGG, WAV, AAC, OPUS)
- Create and manage playlists
- ✅ Apply professional-grade DSP effects:
  - ✅ 3-band parametric EQ
  - ✅ Dynamic range compressor
  - ✅ Brick-wall limiter
  - ✅ Effect chain with presets
- High-quality upsampling/resampling (Phase 1.5)
- Volume leveling across tracks (Phase 1.5)
- Gapless playback and crossfade (Phase 1.5)
- Portable data folder (copy-paste to share)
- Beautiful, responsive UI
- 10,000+ track library performs well

---

## Phase 2: Multi-Source & Server Sync
**Goal**: Connect to multiple servers, sync libraries, offline support

**See**:
- [MULTI_SOURCE_ARCHITECTURE.md](docs/architecture/MULTI_SOURCE_ARCHITECTURE.md)
- [SYNC_STRATEGY.md](docs/architecture/SYNC_STRATEGY.md)
- [OFFLINE_MODE.md](docs/architecture/OFFLINE_MODE.md)

### Deliverables

#### 2.1: Source Management (1 week)
- [ ] **Source Configuration** (`soul-storage`)
  - Sources table (local + multiple servers)
  - Active server selection (only one active at a time)
  - Connection status tracking
  - Source authentication (JWT tokens)

- [ ] **Source Manager** (`soul-core`)
  - Add/remove server sources
  - Set active server
  - Connection monitoring and health checks
  - Auto-detect online/offline state

#### 2.2: Server Implementation (2-3 weeks)
- [ ] **Server Binary** (`soul-server`)
  - Multi-user authentication (JWT + local users)
  - REST API for library access
    - GET /api/sync/tracks (with cursor pagination)
    - GET /api/tracks/{id}/stream
    - GET /api/tracks/{id}/download
    - Playlist endpoints
    - User management
  - Audio streaming with range requests
  - Playlist sharing system
  - Health check endpoint
  - Docker container support
  - One-click setup script

#### 2.3: Sync Engine (2 weeks)
- [ ] **Sync Protocol** (`soul-sync`)
  - ServerClient (HTTP/REST client)
  - Initial sync (full metadata fetch)
  - Incremental sync (cursor-based delta)
  - Bidirectional sync (upload local changes)
  - Conflict resolution strategies
  - Deduplication (same track from multiple sources)
  - Sync state tracking

- [ ] **Offline Queue**
  - Queue local changes when offline
  - Auto-sync when connection restored
  - Retry logic with exponential backoff
  - Failed operation handling

#### 2.4: Download & Cache (1-2 weeks)
- [ ] **Download Manager**
  - Background download worker
  - Download queue with priorities
  - Progress tracking and resumable downloads
  - Bandwidth throttling
  - Smart download strategies:
    - Recently played tracks
    - High play count tracks
    - User playlists
    - User-selected tracks/albums

- [ ] **Cache Management**
  - Content-addressable file storage
  - LRU eviction policy
  - Pin tracks (never evict)
  - Cache size limits and monitoring
  - Track availability updates

#### 2.5: Desktop UI Updates (1 week)
- [ ] **Source Management UI**
  - Add/remove servers dialog
  - Server list with online status
  - Set active server
  - Sync status indicators

- [ ] **Offline Support UI**
  - Offline mode banner
  - Track availability badges (local, cached, stream-only)
  - "Available Offline" filter
  - Download for offline buttons
  - Cache settings page
  - Pending sync queue viewer

- [ ] **Playback Strategy**
  - Automatic source selection (local > cached > stream)
  - Fallback when server offline
  - "Unavailable offline" error messages
  - Stream quality selection

- [ ] **Testing**
  - Multi-user integration tests
  - Auth flow tests
  - Sync protocol tests (initial + incremental)
  - Offline queue tests
  - Connection loss/restore tests
  - Load testing (concurrent streams)
  - Cache eviction tests
  - Testcontainers for full server stack

### Success Criteria
- Can connect to multiple servers
- Library syncs from active server (metadata only)
- Can stream tracks from server when online
- Can download tracks for offline use
- Automatic fallback to local/cached when offline
- Offline changes sync when reconnected
- Deduplication works (same track from local + server shows once)
- Cache management prevents unlimited disk usage
- Easy Docker deployment for server

---

## Phase 3: ESP32-S3 Embedded DAP
**Goal**: Portable hardware music player

### Deliverables
- [ ] **ESP32-S3 Firmware** (`soul-player-esp32`)
  - awedio_esp32 audio playback
  - Symphonia decoder (shared with desktop!)
  - SD card filesystem integration
  - SQLite database (same schema!)
  - WiFi server sync
  - Battery management
  - Sleep modes

- [ ] **E-ink Display Driver**
  - Album art display
  - Track info (title, artist, album)
  - Playback controls
  - Battery indicator
  - Menu system

- [ ] **Hardware Integration**
  - I2S DAC output
  - Button controls
  - Rotary encoder (optional)
  - USB-C charging

- [ ] **Testing**
  - Hardware-in-loop tests (if available)
  - Simulator tests for core logic
  - Power consumption profiling

### Success Criteria
- Plays music from SD card
- E-ink display shows track info
- Syncs library with server over WiFi
- 8+ hours battery life

---

## Phase 4: Discovery & Advanced Features
**Goal**: Music discovery and recommendations

### Deliverables
- [ ] **Discovery Service** (`soul-discovery`)
  - Bandcamp API integration
  - Discogs metadata enrichment
  - Similar track algorithm (acoustic fingerprinting?)
  - Genre-based recommendations

- [ ] **Mobile Support** (`soul-player-mobile`)
  - Tauri Mobile implementation (iOS/Android)
  - Touch-optimized UI
  - Background playback
  - Notification controls

- [ ] **Advanced Audio**
  - Gapless playback
  - Crossfade
  - ReplayGain support
  - Audio visualization (optional)

- [ ] **Social Features**
  - Playlist sharing links
  - Collaborative playlists
  - Listen history
  - Statistics dashboard

### Success Criteria
- Discover new music via Bandcamp/Discogs
- Mobile app feature parity with desktop
- Rich metadata for entire library

---

## Future Considerations (Post-MVP)

### Audio Formats
- [ ] DSD support (requires DSD-to-PCM conversion)
- [ ] High-res streaming (24-bit/192kHz)

### Advanced Effects
- [ ] Reverb
- [ ] Pitch shifting
- [ ] Time stretching
- [ ] Custom plugin system

### Cloud Integration
- [ ] S3-compatible storage backend
- [ ] Encrypted cloud backups
- [ ] Multi-server federation

### Hardware Variants
- [ ] Alternative embedded platforms (RP2350?)
- [ ] Headless server appliance
- [ ] Car integration (Android Auto/CarPlay)

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
- Helm charts (Kubernetes)

### Embedded Releases
- Firmware binaries (.bin)
- OTA update system
- Hardware documentation

---

## Success Metrics

### Technical
- **Performance**: Library of 10,000+ tracks loads in <2s
- **Quality**: 50-60% test coverage, no shallow tests
- **Security**: Zero critical vulnerabilities (cargo audit)
- **Portability**: Same core code on desktop/server/embedded

### User Experience
- **Setup**: Server running in <5 minutes
- **Reliability**: 99.9% uptime for server
- **Battery**: 8+ hours on ESP32-S3
- **Sync**: <10s for metadata sync

