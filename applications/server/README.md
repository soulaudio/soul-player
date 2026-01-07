# Soul Server

Multi-user music streaming server for Soul Player.

## Architecture

The server is built with:
- **Web Framework**: Axum + Tokio (async Rust)
- **Authentication**: JWT tokens + bcrypt password hashing
- **Storage**: Reuses `soul-storage` (SQLite multi-user database)
- **Transcoding**: FFmpeg-based format/quality conversion
- **File Storage**: Quality-based directory structure (original/high/medium/low)

## Structure

```
applications/server/
├── src/
│   ├── main.rs              # Server bootstrap + CLI
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error types
│   ├── api/                 # HTTP route handlers
│   │   ├── auth.rs          # Login, token refresh
│   │   ├── tracks.rs        # Track CRUD + search
│   │   ├── playlists.rs     # Playlist management + sharing
│   │   ├── stream.rs        # Audio streaming (range requests)
│   │   └── admin.rs         # User management, scanning
│   ├── services/
│   │   ├── auth.rs          # JWT + password service
│   │   ├── file_storage.rs  # File management
│   │   └── transcoding.rs   # FFmpeg wrapper
│   ├── middleware/
│   │   └── auth.rs          # JWT authentication middleware
│   └── jobs/
│       └── transcoder.rs    # Background transcoding queue
├── Cargo.toml
└── config.example.toml      # Configuration template
```

## API Endpoints

### Public (No Auth)
- `POST /api/auth/login` - Login with username/password
- `POST /api/auth/refresh` - Refresh access token

### Protected (Requires Bearer Token)

**Tracks:**
- `GET /api/tracks` - List all tracks (with pagination & search)
- `GET /api/tracks/:id` - Get track details
- `POST /api/tracks/import` - Upload a track file
- `DELETE /api/tracks/:id` - Delete a track

**Playlists:**
- `GET /api/playlists` - Get user's playlists
- `POST /api/playlists` - Create playlist
- `GET /api/playlists/:id` - Get playlist with tracks
- `PUT /api/playlists/:id` - Update playlist name
- `DELETE /api/playlists/:id` - Delete playlist
- `POST /api/playlists/:id/tracks` - Add track to playlist
- `DELETE /api/playlists/:id/tracks/:track_id` - Remove track
- `POST /api/playlists/:id/share` - Share playlist with user
- `DELETE /api/playlists/:id/share/:user_id` - Unshare playlist

**Streaming:**
- `GET /api/stream/:track_id?quality=high` - Stream audio file
  - Supports HTTP Range requests for seeking
  - Quality options: original, high, medium, low

**Admin:**
- `POST /api/admin/users` - Create user account
- `GET /api/admin/users` - List all users
- `DELETE /api/admin/users/:id` - Delete user
- `POST /api/admin/scan` - Trigger directory scan
- `GET /api/admin/scan/status` - Get scan status

## Configuration

Copy `config.example.toml` to `config.toml` and adjust:

```toml
[server]
host = "0.0.0.0"
port = 8080

[storage]
database_url = "sqlite://./data/soul.db"
music_storage_path = "./data/tracks"
scan_directories = ["/music"]

[auth]
jwt_secret = "your-secret-key"  # Or set SOUL_AUTH_JWT_SECRET env var
jwt_expiration_hours = 24
jwt_refresh_expiration_days = 30

[transcoding]
enabled = true
formats = ["mp3", "flac", "ogg"]  # Multiple format support
workers = 2
ffmpeg_path = "/usr/bin/ffmpeg"
```

## CLI Commands

```bash
# Start server
cargo run -p soul-server serve

# Create user
cargo run -p soul-server add-user --username admin --password secret

# List users
cargo run -p soul-server list-users

# Scan directory
cargo run -p soul-server scan /path/to/music
```

## Features Implemented

- ✅ JWT authentication with refresh tokens
- ✅ Multi-format transcoding (MP3, FLAC, OGG, WAV, Opus)
- ✅ Quality variants (original, high, medium, low)
- ✅ Audio streaming with HTTP range request support
- ✅ User-based playlist management
- ✅ Playlist sharing (read/write permissions)
- ✅ Background transcoding queue
- ✅ File upload/import from clients
- ✅ Configuration via file + environment variables
- ✅ Reuses existing multi-user database schema

## TODO (Database Migrations)

The server needs additional database tables:

1. **user_credentials** - Store password hashes
2. **Track quality variants tracking** - Add columns to track which variants exist
3. **Sync log** - For client sync (manual/cron)

These migrations need to be added to `soul-storage` or as server-specific migrations.

## Known Compilation Issues

Some Axum handler signatures need adjusting - extractors must be in correct order:
- Path parameters first
- State extractors
- Json/Query extractors
- Request must be last (if used)

Affected handlers:
- `add_track_to_playlist` - Path should come before State
- `remove_track_from_playlist` - Path tuple ordering
- `share_playlist` - Request/Json ordering
- `unshare_playlist` - Path tuple ordering
- `stream_track` - Too many extractors, simplify
- `create_user` - Request/Json ordering
- `delete_user` - Request/Path ordering
- `trigger_scan` - Request/Json ordering

## Development

```bash
# Build
cargo build -p soul-server

# Run with logging
RUST_LOG=soul_server=debug cargo run -p soul-server serve

# Format
cargo fmt -p soul-server

# Lint
cargo clippy -p soul-server
```

## Quick Start

### Using Docker (Recommended for Development)

From the project root:

```bash
# Start the server
yarn dev:server

# View logs
yarn dev:server:logs

# Stop the server
yarn dev:server:down

# Clean up (removes volumes/data)
yarn dev:server:clean
```

The server will be available at `http://localhost:8080`.

## Deployment

### Docker Compose (Recommended)

From the project root:

```bash
# Start services
docker compose up -d

# View logs
docker compose logs -f server

# Stop services
docker compose down
```

Configuration via environment variables (see `docker-compose.yml` and `.env.server`).

### Docker (Manual)

```bash
# Build from project root
docker build -f applications/server/Dockerfile -t soul-server .

# Run
docker run -d -p 8080:8080 \
  -v soul-data:/app/data \
  -e SOUL_AUTH_JWT_SECRET=your-secret \
  soul-server
```

### Environment Variables for Production

Create a `.env` file (see `.env.server` example):

```env
JWT_SECRET=$(openssl rand -hex 32)
SOUL_SERVER_HOST=0.0.0.0
SOUL_SERVER_PORT=8080
```

## Architecture Decisions

1. **Shared Library Model**: All tracks accessible to all users (like local mode)
2. **Quality Variants**: Configurable per-format quality levels
3. **Manual Sync**: No real-time WebSocket in MVP (manual client-initiated sync)
4. **Transcoding**: Background queue with configurable workers
5. **File Deduplication**: Handled by storage layer (file hash)
6. **Admin CLI**: Command-line user management (no web UI in MVP)
