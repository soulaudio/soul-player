# Soul Player

Local-first, cross-platform music player. Desktop, mobile (iOS/Android), server, and ESP32-S3 DAP firmware.

---

## Quick Start

### Desktop

```bash
# From repository root
yarn                # Install all dependencies
yarn dev:desktop    # Run desktop app
```

**Requirements**: Rust 1.75+, Node 20+, system deps (see below)

---

## System Dependencies

### Linux (Ubuntu/Debian)
```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf pkg-config
```

### macOS
```bash
xcode-select --install
```

### Windows
- Install WebView2 (usually pre-installed)
- Install Visual Studio Build Tools

---

## Mobile

```bash
# From repository root, or cd applications/mobile

# iOS
yarn workspace soul-player-mobile tauri:ios init
yarn workspace soul-player-mobile tauri:ios dev

# Android
yarn workspace soul-player-mobile tauri:android init
yarn workspace soul-player-mobile tauri:android dev
```

See `docs/development/MOBILE_SETUP.md` for prerequisites.

---

## Build

```bash
# From repository root
yarn build:desktop
yarn build:mobile
```

---

## Structure

```
libraries/          # Rust libraries (audio, storage, metadata, sync)
applications/
  shared/          # React components (shared across desktop/mobile)
  desktop/         # Desktop Tauri app
  mobile/          # Mobile Tauri app (iOS/Android)
  server/          # Multi-user server
  firmware/        # ESP32-S3 DAP firmware
docs/              # Architecture, testing, CI/CD guides
```

---

## Development

```bash
# From repository root
yarn test          # Run all tests
yarn lint          # Lint all workspaces
yarn type-check    # TypeScript type checking
```

---

## Documentation

- `docs/ARCHITECTURE.md` - System design
- `docs/FOLDER_STRUCTURE.md` - Project layout
- `docs/TESTING.md` - Testing strategy
- `docs/architecture/AUDIO_ABSTRACTION.md` - Audio DI pattern
- `docs/deployment/CI_CD.md` - CI/CD setup

---

## Tech Stack

**Backend**: Rust (Symphonia, CPAL, SQLx, Axum)
**Frontend**: React, TypeScript, Tailwind CSS, Zustand
**Desktop/Mobile**: Tauri 2.0
**Firmware**: ESP32-S3 (Embassy, awedio_esp32)

---

## License

GNU AGPL-3.0
