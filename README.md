# Soul Player

Local-first, cross-platform music player. Desktop, mobile (iOS/Android), server, and ESP32-S3 DAP firmware.

---

## Quick Start

### Desktop

```bash
# From repository root
corepack enable     # Enable Yarn 4.x via Corepack (first time only)
yarn install        # Install all dependencies
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

## Database Setup

**First time only**:

```bash
# Unix/Linux/macOS
./scripts/setup-sqlx.sh

# Windows - see docs/SQLX_SETUP.md for PowerShell commands
```

See [docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md) for details.

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

## Server

Run the multi-user streaming server with web interface.

### Local (No Docker)

```bash
# Server only (API on port 8080)
cargo run -p soul-server -- serve

# Server + Web UI (two terminals)
cargo run -p soul-server -- serve   # Terminal 1: API on :8080
yarn dev:web                         # Terminal 2: Web UI on :3000 (proxies to API)
```

Access:
- **Web UI**: http://localhost:3000
- **API**: http://localhost:8080/api

### Docker - Production

```bash
# Build and run (serves web UI + API on port 8080)
docker compose up -d

# Or build manually
docker build -f applications/server/Dockerfile -t soul-player .
docker run -p 8080:8080 -v soul-data:/app/data soul-player
```

Access:
- **Web UI**: http://localhost:8080
- **API**: http://localhost:8080/api

### Development (Hot Reload)

```bash
# Start server with cargo-watch + web dev server with HMR
docker compose -f docker-compose.dev.yml up

# Or use yarn scripts
yarn dev:all           # Start both server and web
yarn dev:all:down      # Stop
```

Access:
- **Web UI (Vite HMR)**: http://localhost:3000
- **API**: http://localhost:8080/api

### API-Only (No Web UI)

```bash
docker build -f applications/server/Dockerfile.server-only -t soul-server:api .
docker run -p 8080:8080 -v soul-data:/app/data soul-server:api
```

### Create User

```bash
# Inside container
docker compose exec soul-player soul-server add-user -u myuser -p mypassword

# Or with docker run
docker run --rm -v soul-data:/app/data soul-player soul-server add-user -u myuser -p mypassword
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SOUL_SERVER_HOST` | `0.0.0.0` | Server bind address |
| `SOUL_SERVER_PORT` | `8080` | Server port |
| `SOUL_AUTH_JWT_SECRET` | - | **Required**: JWT signing secret |
| `SOUL_STORAGE_DATABASE_URL` | `sqlite:///app/data/soul.db` | Database path |
| `SOUL_STORAGE_MUSIC_STORAGE_PATH` | `/app/data/tracks` | Music storage path |
| `SOUL_TRANSCODING_ENABLED` | `true` | Enable audio transcoding |
| `SOUL_WEB_DIR` | `/app/web` | Web UI static files path |

---

## Build

```bash
# From repository root
yarn build:desktop     # Desktop app
yarn build:mobile      # Mobile app
yarn build:web         # Web player
yarn build:marketing   # Marketing site
```

---

## Structure

```
libraries/          # Rust libraries (audio, storage, metadata, sync)
applications/
  shared/          # React components (shared across desktop/mobile/web)
  desktop/         # Desktop Tauri app
  mobile/          # Mobile Tauri app (iOS/Android)
  web/             # Web player (connects to server)
  server/          # Multi-user streaming server
  marketing/       # Marketing website with demo
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

See **[docs/README.md](./docs/README.md)** for complete documentation index.

### Essential Docs

- **[docs/SOUL_SERVICES_PLAN.md](./docs/SOUL_SERVICES_PLAN.md)** - Future: Subscription-based metadata & discovery platform
- **[docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md)** - System design and architecture
- **[docs/SQLX_SETUP.md](./docs/SQLX_SETUP.md)** - Database setup and troubleshooting
- **[docs/FOLDER_STRUCTURE.md](./docs/FOLDER_STRUCTURE.md)** - Project layout
- **[docs/TESTING.md](./docs/TESTING.md)** - Testing strategy
- **[CLAUDE.md](./CLAUDE.md)** - Codebase instructions for Claude Code

---

## Tech Stack

**Backend**: Rust (Symphonia, CPAL, SQLx, Axum)
**Frontend**: React, TypeScript, Tailwind CSS, Zustand
**Desktop/Mobile**: Tauri 2.0
**Firmware**: ESP32-S3 (Embassy, awedio_esp32)

---

## License

GNU AGPL-3.0
