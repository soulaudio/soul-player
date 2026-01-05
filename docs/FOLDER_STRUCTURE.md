# Soul Player - Folder Structure

This document describes the complete folder structure for the Soul Player monorepo.

## Overview

Soul Player uses a **libraries vs applications** separation:
- `libraries/` - Reusable Rust crates (audio, storage, metadata, etc.)
- `applications/` - User-facing applications (desktop, mobile, server)

This structure provides:
- Clear separation between reusable libraries and platform-specific apps
- Explicit shared frontend code location
- Easy scalability for future applications (CLI, web player, admin panel)
- CI/CD-friendly organization

---

## Complete Structure

```
soul-player/
├── .github/
│   └── workflows/                       # CI/CD pipelines
│       ├── ci-libraries.yml             # Test all Rust libraries
│       ├── ci-desktop.yml               # Desktop build (Windows, macOS, Linux)
│       ├── ci-mobile-ios.yml            # iOS build and tests
│       ├── ci-mobile-android.yml        # Android build and tests
│       ├── ci-server.yml                # Server build and Docker
│       ├── ci-frontend.yml              # Frontend tests (Vitest)
│       └── release.yml                  # Release automation
│
├── libraries/                           # Rust libraries
│   ├── soul-core/                       # Core traits & types
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── error.rs                 # Common error types
│   │   │   ├── audio/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── output.rs            # AudioOutput trait
│   │   │   │   ├── buffer.rs            # AudioBuffer types
│   │   │   │   └── effect.rs            # AudioEffect trait
│   │   │   ├── storage/
│   │   │   │   └── repository.rs        # Repository traits
│   │   │   └── types/
│   │   │       ├── track.rs
│   │   │       ├── artist.rs
│   │   │       ├── album.rs
│   │   │       └── playlist.rs
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── soul-storage/                    # SQLite storage layer
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── connection.rs
│   │   │   ├── repositories/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── track.rs
│   │   │   │   ├── playlist.rs
│   │   │   │   └── user.rs
│   │   │   └── migrations/
│   │   ├── tests/
│   │   │   └── integration_tests.rs     # Testcontainers tests
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── soul-audio/                      # Audio decoder + engine
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── decoder/
│   │   │   │   ├── mod.rs
│   │   │   │   └── symphonia.rs         # Symphonia decoder wrapper
│   │   │   ├── engine.rs                # Generic AudioEngine<O: AudioOutput>
│   │   │   ├── resampler.rs             # Sample rate conversion
│   │   │   └── effects/
│   │   │       ├── mod.rs
│   │   │       ├── chain.rs             # EffectChain
│   │   │       ├── eq.rs                # 3-band parametric EQ
│   │   │       └── compressor.rs        # Audio compressor
│   │   ├── tests/
│   │   ├── benches/                     # Criterion benchmarks
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── soul-audio-desktop/              # Desktop audio output (CPAL)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── cpal_output.rs           # impl AudioOutput for CpalOutput
│   │   ├── tests/
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── soul-audio-mobile/               # Mobile audio output
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ios.rs                   # iOS audio bridge
│   │   │   └── android.rs               # Android audio bridge
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── soul-audio-embedded/             # ESP32 audio output
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── esp32_output.rs          # awedio_esp32 wrapper
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── soul-metadata/                   # Tag reading & library scanning
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── scanner.rs               # Directory scanner
│   │   │   ├── extractor.rs             # Metadata extraction
│   │   │   └── tags/
│   │   ├── tests/
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   ├── soul-sync/                       # Client-server sync protocol
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── protocol.rs
│   │   │   ├── client.rs
│   │   │   └── conflict.rs              # Conflict resolution
│   │   ├── Cargo.toml
│   │   └── README.md
│   │
│   └── soul-discovery/                  # Music discovery (Phase 4)
│       ├── src/
│       ├── Cargo.toml
│       └── README.md
│
├── applications/                        # User-facing applications
│   ├── shared/                          # Shared React components
│   │   ├── src/
│   │   │   ├── components/
│   │   │   │   ├── ui/                  # shadcn/ui components
│   │   │   │   │   ├── button.tsx
│   │   │   │   │   ├── slider.tsx
│   │   │   │   │   ├── dialog.tsx
│   │   │   │   │   └── index.ts
│   │   │   │   ├── player/              # Player controls
│   │   │   │   │   ├── Controls.tsx
│   │   │   │   │   ├── ProgressBar.tsx
│   │   │   │   │   ├── VolumeControl.tsx
│   │   │   │   │   └── index.ts
│   │   │   │   ├── library/             # Library views
│   │   │   │   │   ├── TrackList.tsx
│   │   │   │   │   ├── AlbumGrid.tsx
│   │   │   │   │   ├── ArtistList.tsx
│   │   │   │   │   └── index.ts
│   │   │   │   └── playlists/           # Playlist UI
│   │   │   │       ├── PlaylistCard.tsx
│   │   │   │       ├── PlaylistDetail.tsx
│   │   │   │       └── index.ts
│   │   │   ├── stores/                  # Zustand state management
│   │   │   │   ├── player.ts            # Playback state
│   │   │   │   ├── library.ts           # Library state
│   │   │   │   ├── queue.ts             # Queue state
│   │   │   │   └── settings.ts          # Settings state
│   │   │   ├── hooks/                   # React hooks
│   │   │   │   ├── useAudioPlayer.ts
│   │   │   │   ├── useLibrary.ts
│   │   │   │   ├── usePlaylists.ts
│   │   │   │   └── usePlatform.ts       # Platform detection
│   │   │   ├── lib/                     # Utilities
│   │   │   │   ├── tauri.ts             # Tauri API wrapper
│   │   │   │   ├── format.ts            # Time/size formatting
│   │   │   │   ├── platform.ts          # Platform utilities
│   │   │   │   └── constants.ts
│   │   │   └── index.ts                 # Public exports
│   │   ├── tests/
│   │   │   ├── unit/
│   │   │   │   ├── components/
│   │   │   │   ├── stores/
│   │   │   │   └── hooks/
│   │   │   └── setup.ts                 # Vitest setup
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── vitest.config.ts
│   │   └── README.md
│   │
│   ├── desktop/                         # Desktop application
│   │   ├── src/
│   │   │   ├── main.tsx                 # Entry point
│   │   │   ├── App.tsx                  # Desktop app layout
│   │   │   ├── features/                # Desktop-specific features
│   │   │   │   ├── menubar/
│   │   │   │   │   ├── MenuBar.tsx
│   │   │   │   │   └── menu-config.ts
│   │   │   │   ├── titlebar/
│   │   │   │   │   └── CustomTitleBar.tsx
│   │   │   │   ├── keyboard-shortcuts/
│   │   │   │   │   └── ShortcutManager.tsx
│   │   │   │   ├── system-tray/
│   │   │   │   │   └── TrayMenu.tsx
│   │   │   │   └── settings/
│   │   │   │       └── SettingsDialog.tsx
│   │   │   ├── styles/
│   │   │   │   └── index.css            # Tailwind + design tokens
│   │   │   └── vite.config.ts
│   │   ├── src-tauri/
│   │   │   ├── src/
│   │   │   │   ├── main.rs
│   │   │   │   ├── commands/            # Tauri commands
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── player.rs        # Playback commands
│   │   │   │   │   ├── library.rs       # Library commands
│   │   │   │   │   ├── playlist.rs      # Playlist commands
│   │   │   │   │   └── settings.rs      # Settings commands
│   │   │   │   ├── menu.rs              # App menu definition
│   │   │   │   ├── tray.rs              # System tray
│   │   │   │   └── state.rs             # App state management
│   │   │   ├── icons/
│   │   │   ├── Cargo.toml
│   │   │   └── tauri.conf.json
│   │   ├── tests/
│   │   │   ├── integration/             # Rust integration tests
│   │   │   │   └── commands_tests.rs
│   │   │   └── e2e/                     # WebdriverIO E2E tests
│   │   │       ├── player.spec.ts
│   │   │       ├── library.spec.ts
│   │   │       └── wdio.conf.ts
│   │   ├── package.json
│   │   └── README.md
│   │
│   ├── mobile/                          # Mobile application
│   │   ├── src/
│   │   │   ├── main.tsx                 # Entry point
│   │   │   ├── App.tsx                  # Mobile app layout
│   │   │   ├── features/                # Mobile-specific features
│   │   │   │   ├── bottom-nav/
│   │   │   │   │   └── BottomNavigation.tsx
│   │   │   │   ├── gestures/
│   │   │   │   │   ├── SwipeControls.tsx
│   │   │   │   │   └── GestureHandler.tsx
│   │   │   │   ├── now-playing-sheet/
│   │   │   │   │   └── NowPlayingSheet.tsx
│   │   │   │   ├── background-audio/
│   │   │   │   │   └── AudioSession.tsx
│   │   │   │   └── notifications/
│   │   │   │       └── MediaNotification.tsx
│   │   │   ├── styles/
│   │   │   │   └── index.css
│   │   │   └── vite.config.ts
│   │   ├── src-tauri/
│   │   │   ├── src/
│   │   │   │   ├── lib.rs               # Mobile entry point
│   │   │   │   ├── commands/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── player.rs
│   │   │   │   │   ├── library.rs
│   │   │   │   │   └── sync.rs
│   │   │   │   └── mobile/              # Mobile-specific Rust
│   │   │   │       ├── mod.rs
│   │   │   │       ├── audio.rs         # Audio bridge coordinator
│   │   │   │       └── notifications.rs # Notification bridge
│   │   │   ├── gen/
│   │   │   │   ├── apple/               # iOS Xcode project
│   │   │   │   │   ├── Sources/
│   │   │   │   │   │   ├── AudioBridge.swift
│   │   │   │   │   │   ├── NowPlayingBridge.swift
│   │   │   │   │   │   └── NotificationBridge.swift
│   │   │   │   │   ├── Resources/
│   │   │   │   │   │   ├── Assets.xcassets/
│   │   │   │   │   │   └── LaunchScreen.storyboard
│   │   │   │   │   ├── soul-mobile.xcodeproj/
│   │   │   │   │   └── ExportOptions.plist
│   │   │   │   └── android/             # Android Studio project
│   │   │   │       ├── app/
│   │   │   │       │   ├── src/main/
│   │   │   │       │   │   ├── java/com/soulplayer/mobile/
│   │   │   │       │   │   │   ├── AudioBridge.kt
│   │   │   │       │   │   │   ├── MediaService.kt
│   │   │   │       │   │   │   └── NotificationManager.kt
│   │   │   │       │   │   ├── res/
│   │   │   │       │   │   │   ├── values/
│   │   │   │       │   │   │   ├── drawable/
│   │   │   │       │   │   │   └── layout/
│   │   │   │       │   │   └── AndroidManifest.xml
│   │   │   │       │   └── build.gradle
│   │   │   │       ├── gradle/
│   │   │   │       ├── build.gradle
│   │   │   │       └── settings.gradle
│   │   │   ├── Cargo.toml
│   │   │   └── tauri.conf.json
│   │   ├── tests/
│   │   │   ├── integration/
│   │   │   └── e2e/
│   │   ├── package.json
│   │   └── README.md
│   │
│   └── server/                          # Server application (Phase 2)
│       ├── src/
│       │   ├── main.rs
│       │   ├── api/
│       │   ├── auth/
│       │   └── streaming/
│       ├── Cargo.toml
│       ├── Dockerfile
│       └── README.md
│
├── docs/
│   ├── ARCHITECTURE.md                  # System architecture
│   ├── CONVENTIONS.md                   # Coding standards
│   ├── TESTING.md                       # Testing strategy
│   ├── FOLDER_STRUCTURE.md              # This file
│   │
│   ├── architecture/                    # Detailed architecture docs
│   │   ├── AUDIO_ABSTRACTION.md         # Audio DI pattern
│   │   ├── FRONTEND_ARCHITECTURE.md     # React architecture
│   │   ├── DEPENDENCY_INJECTION.md      # DI patterns
│   │   └── PLATFORM_SPECIFICS.md        # Platform differences
│   │
│   ├── development/                     # Development guides
│   │   ├── DESKTOP_SETUP.md             # Desktop dev environment
│   │   ├── MOBILE_SETUP.md              # Mobile dev environment
│   │   ├── TESTING_GUIDE.md             # How to test
│   │   └── DEBUGGING.md                 # Debugging tips
│   │
│   └── deployment/                      # Deployment guides
│       ├── CI_CD.md                     # CI/CD setup
│       ├── DESKTOP_DISTRIBUTION.md      # Desktop app distribution
│       ├── MOBILE_DISTRIBUTION.md       # App store submission
│       └── SERVER_DEPLOYMENT.md         # Server deployment
│
├── scripts/                             # Utility scripts
│   ├── setup-dev.sh                     # One-command dev setup
│   ├── test-all.sh                      # Run all tests
│   ├── build-all.sh                     # Build all targets
│   ├── format-all.sh                    # Format all code
│   └── ci/                              # CI-specific scripts
│       ├── install-deps.sh
│       └── build-matrix.sh
│
├── .github/
│   ├── workflows/                       # (see above)
│   ├── ISSUE_TEMPLATE/
│   └── PULL_REQUEST_TEMPLATE.md
│
├── Cargo.toml                           # Workspace root
├── package.json                         # npm workspace root
├── moon.yml                             # Moon task orchestration
├── .gitignore
├── .gitattributes
├── README.md
├── ROADMAP.md
├── CONTRIBUTING.md
├── LICENSE-MIT
└── LICENSE-APACHE
```

---

## Key Directories Explained

### `libraries/`
Contains all reusable Rust libraries. These are platform-agnostic and should not depend on `applications/`.

**Naming Convention**: `soul-*` for core libraries, `soul-audio-*` for audio output implementations.

### `applications/shared/`
Shared React components, hooks, and stores. Used by both desktop and mobile apps.

**Import Path**: Applications import with `@shared/components`, `@shared/stores`, etc.

### `applications/desktop/`
Desktop-specific code. Uses `applications/shared/` for common UI components.

**Key Features**:
- Custom title bar
- Menu bar integration
- Keyboard shortcuts
- System tray

### `applications/mobile/`
Mobile-specific code. Uses `applications/shared/` for common UI components.

**Key Features**:
- Bottom navigation
- Gesture controls
- Native audio bridges (Swift/Kotlin)
- Background playback

### `docs/`
All documentation organized by category:
- Top-level: Main architecture docs
- `architecture/`: Deep dives into design patterns
- `development/`: Developer guides
- `deployment/`: CI/CD and distribution

---

## Configuration Files

### Cargo Workspace (`Cargo.toml`)
Defines all library crates in `libraries/` and apps in `applications/`.

### NPM Workspace (`package.json`)
Defines frontend workspaces: `applications/shared`, `applications/desktop`, `applications/mobile`.

### Moon Tasks (`moon.yml`)
Cross-language task orchestration for build/test/lint across Rust and TypeScript.

---

## Import Patterns

### Rust Libraries
```rust
// In applications/desktop/src-tauri/src/main.rs
use soul_core::audio::AudioOutput;
use soul_audio::AudioEngine;
use soul_audio_desktop::CpalOutput;
use soul_storage::repositories::TrackRepository;
```

### TypeScript Shared Components
```typescript
// In applications/desktop/src/App.tsx
import { PlayerControls } from '@shared/components/player';
import { TrackList } from '@shared/components/library';
import { usePlayerStore } from '@shared/stores/player';
import { usePlatform } from '@shared/hooks/usePlatform';
```

### Platform-Specific Code
```typescript
// In applications/mobile/src/features/gestures/SwipeControls.tsx
import { PlayerControls } from '@shared/components/player';
import { GestureHandler } from './GestureHandler';
```

---

## CI/CD Integration

Each platform has its own workflow:
- **libraries**: Test all Rust crates
- **desktop**: Build for Windows, macOS, Linux
- **mobile-ios**: Build and test iOS app
- **mobile-android**: Build and test Android app
- **frontend**: Test shared React components

See `docs/deployment/CI_CD.md` for details.

---

## Development Workflow

### Working on Libraries
```bash
cd libraries/soul-audio
cargo test
cargo clippy
```

### Working on Desktop
```bash
cd applications/desktop
npm install
npm run tauri dev
```

### Working on Mobile
```bash
cd applications/mobile
npm install
npm run tauri ios dev
# or
npm run tauri android dev
```

### Working on Shared Components
```bash
cd applications/shared
npm install
npm run test
npm run test:watch
```

---

## Migration Path

To migrate from the current structure to this new structure:

1. Create `libraries/` and move existing `crates/*` (except apps)
2. Create `applications/` directory
3. Move `crates/soul-player-desktop` → `applications/desktop`
4. Create `applications/shared` for shared frontend
5. Create `applications/mobile` for mobile app
6. Update all `Cargo.toml` path references
7. Update CI/CD workflows
8. Update documentation

See implementation plan in the next section.

---

## Benefits of This Structure

1. **Clarity**: Obvious separation between libraries and apps
2. **Scalability**: Easy to add new apps (CLI, web player, admin panel)
3. **Maintainability**: Shared code is explicit, not buried in app directories
4. **CI/CD**: Each app has clear boundaries for pipeline configuration
5. **Developer Experience**: New contributors immediately understand layout
6. **Vertical Slicing**: Features can span libraries → shared → apps
