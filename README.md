# Soul Player

**Your music. Your way. Your privacy.**

Soul Player is a modern, open-source music player for desktop. Play your local music library with advanced audio features, privacy-first design, and no subscriptions required. Server and mobile apps coming soon.

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](LICENSE)
[![Discord](https://img.shields.io/discord/1461068250711461982?color=7289da&label=Discord&logo=discord&logoColor=white)](https://discord.gg/pCkTFbY9hC)
[![GitHub Stars](https://img.shields.io/github/stars/soulaudio/soul-player?style=social)](https://github.com/soulaudio/soul-player)

---

## Why Soul Player?

**Privacy First** - Your music stays on your devices. No cloud uploads, no tracking, no telemetry.

**Cross-Platform Desktop** - Windows, macOS, and Linux support. Mobile and server apps are planned.

**Advanced Audio Engine** - Gapless playback, crossfade, DSP effects, ReplayGain, high-quality resampling, DSD support, and bit-perfect output with WASAPI/ASIO exclusive mode.

**100% Free Core** - The music player is free and open source forever. Optional paid discovery features (Discogs, Bandcamp, metadata enrichment) will be available as a separate service to fund development.

**No Ads, No Subscriptions** - Play your music without interruption. Pay only if you want advanced discovery features.

---

## Get Started

### Download

**Desktop** - [Download for Windows, macOS, or Linux](https://github.com/soulaudio/soul-player/releases)

**Mobile** - iOS & Android apps are planned (see [ROADMAP.md](./ROADMAP.md))

**Server** - Multi-user streaming server is in development

### Installation Instructions

#### Windows
1. Download the `.exe` installer from [Releases](https://github.com/soulaudio/soul-player/releases)
2. Run the installer and follow the setup wizard
3. Launch Soul Player from the Start Menu

#### macOS
1. Download the `.dmg` file from [Releases](https://github.com/soulaudio/soul-player/releases)
2. Open the DMG and drag Soul Player to your Applications folder
3. **Important:** Due to macOS Gatekeeper security, you'll see a warning on first launch

**To open Soul Player:**
- **Method 1 (Recommended):** Right-click (or Control+click) the app → Select "Open" → Click "Open" in the dialog
- **Method 2 (Terminal):** Run this command:
  ```bash
  xattr -cr /Applications/Soul\ Player.app
  ```
  Then open the app normally

> **Note:** The app uses ad-hoc code signing (free, without Apple Developer account). This is safe - the warning is just macOS being cautious about apps not notarized by Apple. After the first open, the app will launch normally.

#### Linux
**Arch Linux:**
```bash
# Using an AUR helper (yay, paru, etc.)
yay -S soul-player
```

**Debian/Ubuntu:**
```bash
# Download the .deb file, then:
sudo dpkg -i soul_player_*_amd64.deb
sudo apt-get install -f  # Fix any dependency issues
```

**Fedora/RHEL:**
```bash
# Download the .rpm file, then:
sudo rpm -i soul_player-*.x86_64.rpm
```

**After installation:** Launch from your application menu or run `soul-player` in terminal

### Join the Community

- [Discord](https://discord.gg/pCkTFbY9hC) - Chat with the community, get help, share feedback
- [Report Issues](https://github.com/soulaudio/soul-player/issues) - Found a bug? Let us know
- [Feature Requests](https://github.com/soulaudio/soul-player/issues/new/choose) - Suggest new features

---

## Current Features (Desktop)

### Audio Playback
- **Gapless playback** and **crossfade** with configurable duration and fade curves
- **Queue management** with shuffle and repeat modes
- **Multiple audio formats** - MP3, FLAC, OGG, WAV, AAC, OPUS via Symphonia decoder

### Audio Quality
- **Bit-perfect output** with exclusive mode (WASAPI on Windows)
- **ASIO support** (Windows) and **JACK support** (Linux/macOS) via feature flags
- **High-quality resampling** with rubato and r8brain backends
- **DSD playback** - PCM to DSD64/DSD128/DSD256 conversion
- **ReplayGain normalization** - Track and album gain with EBU R128 loudness analysis
- **Low-latency monitoring** with real-time latency display

### DSP Effects
- **3-band parametric EQ** with fine-tune controls
- **10-band and 31-band graphic EQ** with presets
- **Dynamic range compressor** and **brick-wall limiter**
- **Crossfeed** (Bauer stereophonic-to-binaural DSP) with presets
- **Convolution engine** for room correction (IR file loading)
- **Stereo enhancement** - width control, mid/side processing, balance adjustment
- **Effect chain** with add/remove/reorder, per-effect enable/disable, and presets

### Library Management
- **Local music scanning** with metadata extraction (ID3, Vorbis, APE tags)
- **Multi-user database** with SQLite (designed for server use)
- **Artists, albums, genres** with separate tables
- **Playlists** with track management

### User Interface
- **Modern design** with dark mode and smooth animations
- **Customizable keyboard shortcuts** (app-level, not OS-level)
- **Album, artist, genre views** with navigation
- **Now Playing page** with playback controls
- **Settings page** for audio output, effects, and shortcuts

---

## Planned Features

See [ROADMAP.md](./ROADMAP.md) for detailed development timeline.

### Server (In Development)
- Multi-user authentication (JWT)
- Audio streaming with range requests
- REST API for library access
- Docker container support

### Mobile Apps (Planned)
- Native iOS and Android apps via Tauri Mobile
- Touch-optimized UI
- Background playback and notification controls
- Sync with server or local library

### Discovery & Metadata (Planned via Soul Services)
- **Discogs integration** - Browse and discover through real music communities
- **Bandcamp integration** - Support independent artists
- **AcoustID fingerprinting** - Identify unknown tracks automatically
- **MusicBrainz metadata** - Rich, community-curated music information
- **Lyrics support** - via Genius and LRCLIB
- **ListenBrainz scrobbling** - Track your listening history

**Note**: Discovery features will be offered as a paid subscription service (starting at $5-10/month) OR self-hosted for free if you provide your own API keys. The core music player remains 100% free and open source.

### Multi-Device Control (Planned)
- Soul Connect - Spotify Connect-style device control
- Transfer playback between devices seamlessly
- Jam Sessions - Collaborative listening with shareable links

---

## For Developers

Soul Player is built with Rust and TypeScript, designed for performance and cross-platform compatibility.

### Quick Start - Desktop Development

```bash
# Clone the repository
git clone https://github.com/soulaudio/soul-player.git
cd soul-player

# Enable Yarn 4.x (first time only)
corepack enable

# Install dependencies
yarn install

# Run the desktop app
yarn dev:desktop
```

**Requirements**: Rust 1.75+, Node 20+, system dependencies (see below)

### System Dependencies

#### Quick Install (Recommended)

```bash
# Unix/Linux/macOS (bash)
./scripts/install-deps.sh

# Windows (PowerShell as Administrator)
.\scripts\install-deps.ps1 -AutoInstall
```

<details>
<summary>Manual Installation</summary>

**Linux (Ubuntu/Debian)**
```bash
# Tauri / WebKit
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf pkg-config

# Audio (ALSA)
sudo apt install libasound2-dev

# Build tools (CMake for r8brain resampler)
sudo apt install cmake build-essential clang

# GTK
sudo apt install libglib2.0-dev libgtk-3-dev

# SQLite
sudo apt install sqlite3
```

**macOS**
```bash
xcode-select --install   # Xcode Command Line Tools
brew install cmake pkg-config sqlite
```

**Windows**

| Dependency | Install Command | Notes |
|------------|-----------------|-------|
| Visual Studio Build Tools | `winget install Microsoft.VisualStudio.2022.BuildTools` | C++ workload required |
| CMake | `winget install Kitware.CMake` | Required for r8brain resampler |
| LLVM/Clang | `winget install LLVM.LLVM` | Required for ASIO audio support |
| WebView2 | Usually pre-installed | [Download](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) |

Set the `LIBCLANG_PATH` environment variable:
```powershell
[System.Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")
```

**Cargo Tools (All Platforms)**
```bash
cargo install cargo-audit --locked      # Security auditing
cargo install sqlx-cli --no-default-features --features sqlite --locked  # Database migrations
cargo install wasm-pack --locked        # WASM builds (optional, for marketing demo)
```
</details>

### Database Setup

**First time only**:

```bash
# Unix/Linux/macOS
./scripts/setup-sqlx.sh

# Windows - see docs/SQLX_SETUP.md for PowerShell commands
```

See [docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md) for details.

---

## Build

```bash
# From repository root
yarn build:desktop     # Desktop app
yarn build:web         # Web player (for server)
yarn build:marketing   # Marketing site
```

---

## Project Structure

```
libraries/          # Rust libraries (audio, storage, metadata, sync)
applications/
  shared/          # React components (shared across desktop/mobile/web)
  desktop/         # Desktop Tauri app
  mobile/          # Mobile Tauri app (iOS/Android) - in development
  web/             # Web player (connects to server) - in development
  server/          # Multi-user streaming server - in development
  marketing/       # Marketing website with demo
docs/              # Architecture, testing, CI/CD guides
```

---

## Testing

### Running Tests

```bash
# Quick tests (no Docker needed)
cargo test --all

# Full tests with Docker/testcontainers
cargo test --all --features testcontainers

# Hardware-dependent tests (requires physical audio device)
cargo test --all -- --ignored
```

### Test Categories

| Category | Description | Docker Required |
|----------|-------------|-----------------|
| Unit tests | Fast, isolated tests for individual functions | No |
| Integration tests | Database and component interaction tests | No |
| Testcontainer tests | Audio backend tests with PulseAudio virtual device | Yes |
| Hardware tests | Physical audio device tests (skipped in CI) | No |

See [docs/TESTING.md](./docs/TESTING.md) for detailed testing strategy.

---

## Documentation

See [docs/README.md](./docs/README.md) for complete documentation index.

### Essential Docs

- [ARCHITECTURE.md](./docs/ARCHITECTURE.md) - System design and architecture
- [ROADMAP.md](./ROADMAP.md) - Development roadmap and planned features
- [SOUL_SERVICES_PLAN.md](./docs/SOUL_SERVICES_PLAN.md) - Discovery & metadata subscription service plan
- [TESTING.md](./docs/TESTING.md) - Testing strategy
- [SQLX_SETUP.md](./docs/SQLX_SETUP.md) - Database setup and troubleshooting
- [CLAUDE.md](./CLAUDE.md) - Codebase instructions for Claude Code
- [CONTRIBUTING.md](./CONTRIBUTING.md) - How to contribute

---

## Tech Stack

**Backend**: Rust (Symphonia, CPAL, SQLx, Axum)
**Frontend**: React, TypeScript, Tailwind CSS, Zustand
**Desktop/Mobile**: Tauri 2.0

---

## Contributing

We welcome contributions! Please read [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

- Pick an issue labeled `good first issue`
- Follow [CONVENTIONS.md](./docs/CONVENTIONS.md) for coding standards
- Join [Discord](https://discord.gg/pCkTFbY9hC) to discuss ideas

---

## Security

Found a security vulnerability? Please email **sebastian.stupak@pm.me** instead of opening a public issue.

---

## License

GNU AGPL-3.0 - see [LICENSE](./LICENSE) for details.

---

## Support the Project

Soul Player is free and open source. If you find it useful, consider:

- Starring the repo on GitHub
- Joining the [Discord](https://discord.gg/pCkTFbY9hC) community
- Reporting bugs or suggesting features
- Contributing code or documentation
- Spreading the word to fellow music lovers

---

<div align="center">

**[Website](https://player.soulaudio.co)** • **[Discord](https://discord.gg/pCkTFbY9hC)** • **[GitHub](https://github.com/soulaudio/soul-player)** • **[Download](https://github.com/soulaudio/soul-player/releases)**

Soul Audio

</div>
