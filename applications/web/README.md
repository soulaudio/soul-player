# Soul Player Web Client

Production-ready web player application that connects to a Soul Player server for streaming music.

## Features

- **Two Modes**:
  - **Demo Mode**: Explore the app with sample music (no server required)
  - **Server Mode**: Connect to your Soul Player server for full library access

- **Full Library Management**: Browse albums, artists, playlists, and tracks
- **Web Audio Playback**: Stream music directly from server
- **Authentication**: JWT-based login with token refresh
- **Responsive Design**: Works on desktop and mobile browsers
- **Shared Components**: Uses same UI as desktop app (zero parity)

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Web Client (React + Vite)                                 │
│  ├── Demo Mode: DemoStorage + MockBackendProvider          │
│  └── Server Mode: ServerBackendProvider + Auth             │
├─────────────────────────────────────────────────────────────┤
│  Backend Context (@soul-player/shared)                      │
│  └── Platform-agnostic data fetching interface              │
├─────────────────────────────────────────────────────────────┤
│  Player Commands (WebPlayerCommandsProvider)                │
│  └── HTMLAudioElement for streaming                         │
├─────────────────────────────────────────────────────────────┤
│  Soul Player Server (Rust + Axum)                          │
│  ├── REST API (/api/*)                                      │
│  ├── Audio Streaming (/api/stream/{track_id})              │
│  └── Authentication (JWT)                                   │
└─────────────────────────────────────────────────────────────┘
```

## Prerequisites

- Node.js 20+
- Yarn 4.x (via corepack)
- Soul Player server running (for server mode)

## Setup

```bash
# Install dependencies (from repository root)
yarn

# Create environment configuration
cd applications/web
cp .env.example .env

# Edit .env if needed (defaults to localhost:8080 in dev)
```

## Development

```bash
# Start dev server (from repository root)
yarn dev:web

# Or from applications/web directory
yarn dev

# Open browser to http://localhost:3000
```

**Dev Server Features:**
- Hot module replacement (HMR)
- API proxy to localhost:8080
- WebSocket proxy for future features
- TypeScript type checking

## Production Build

```bash
# Build for production
yarn build

# Preview production build
yarn preview
```

The build output is in `dist/` directory and can be served by any static file server or the Soul Player server itself.

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VITE_API_URL` | API base URL | `/api` (same origin) |
| `VITE_WS_URL` | WebSocket URL | `/ws` (future feature) |

### API Proxy (Development)

In development, Vite proxies API requests to avoid CORS issues:

```typescript
// vite.config.ts
server: {
  proxy: {
    '/api': {
      target: 'http://localhost:8080',
      changeOrigin: true,
    },
  },
}
```

## Authentication Flow

```
1. User enters credentials → POST /api/auth/login
2. Server returns JWT tokens → { access_token, refresh_token }
3. Tokens stored in localStorage
4. API client adds Authorization header to all requests
5. On 401 response → Try refresh token → POST /api/auth/refresh
6. If refresh fails → Redirect to login page
```

**Token Storage:**
- `access_token`: Short-lived JWT (e.g., 15 minutes)
- `refresh_token`: Long-lived token for obtaining new access tokens

## API Endpoints

The web client expects these endpoints from the server:

### Authentication
- `POST /api/auth/login` - Login with username/password
- `POST /api/auth/refresh` - Refresh access token

### Library
- `GET /api/tracks` - Get all tracks
- `GET /api/albums` - Get all albums
- `GET /api/artists` - Get all artists
- `GET /api/playlists` - Get all playlists
- `GET /api/albums/{id}` - Get album details
- `GET /api/albums/{id}/tracks` - Get album tracks

### Playback
- `GET /api/stream/{track_id}` - Stream audio file
- `POST /api/playback/play` - Update playback state
- `POST /api/playback/pause` - Pause playback

### Devices (Multi-device sync)
- `POST /api/devices` - Register device
- `PUT /api/devices/{id}/activate` - Set active device
- `DELETE /api/devices/{id}` - Unregister device

See `applications/web/src/api/client.ts` for full API client implementation.

## Deployment

### Option 1: Serve from Soul Player Server

The Soul Player server can serve both API and web UI from the same origin:

```rust
// Server serves static files from applications/web/dist/
axum::Router::new()
    .nest("/api", api_routes)
    .fallback_service(ServeDir::new("applications/web/dist"))
```

Benefits:
- No CORS issues
- Single deployment
- Simplified configuration

### Option 2: Separate Static Hosting

Deploy `dist/` to any static host (Netlify, Vercel, Cloudflare Pages):

```bash
# Build with production API URL
VITE_API_URL=https://api.example.com yarn build

# Deploy dist/ directory
```

**Requirements:**
- Configure CORS on server to allow web client origin
- Set `VITE_API_URL` to server API endpoint

## Mode Selector

On first visit, users choose between:

1. **Demo Mode**
   - Loads sample music from `/public/demo-data.json`
   - No server required
   - Limited features (read-only)
   - Good for testing UI

2. **Server Mode**
   - Requires authentication
   - Full library access
   - Playback syncs across devices
   - All features enabled

**Mode is saved in localStorage** (`soul-player-mode` key).

## Components

### Providers

- **AuthProvider**: JWT authentication, token refresh, logout
- **ServerBackendProvider**: REST API data fetching (from `@soul-player/shared`)
- **MockBackendProvider**: Demo data (from `@soul-player/shared`)
- **WebPlayerCommandsProvider**: HTMLAudioElement playback

### Pages

- **LoginPage**: Authentication form (custom)
- **All other pages**: Imported from `@soul-player/shared`
  - HomePage, LibraryPage, AlbumPage, ArtistPage, etc.

### API Client

- Located in `src/api/client.ts`
- Handles authentication headers
- Automatic token refresh
- Error handling with user-friendly messages

## Security

- **JWT Authentication**: Bearer token in Authorization header
- **Token Storage**: localStorage (not recommended for sensitive apps, consider httpOnly cookies in production)
- **HTTPS Required**: In production, always use HTTPS for API and web client
- **CORS**: Server must whitelist web client origin if deployed separately

## Browser Compatibility

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

**Required APIs:**
- Web Audio API (for playback)
- LocalStorage (for auth tokens)
- Fetch API (for API requests)

## Troubleshooting

### "Failed to fetch" errors

**Problem**: API requests fail with network errors.

**Solutions:**
1. Ensure Soul Player server is running on localhost:8080
2. Check Vite proxy configuration in `vite.config.ts`
3. In production, verify `VITE_API_URL` is set correctly

### "Unauthorized" errors

**Problem**: API returns 401 after login.

**Solutions:**
1. Check if tokens are stored in localStorage (DevTools → Application → Local Storage)
2. Verify server JWT secret matches
3. Check token expiration times
4. Clear localStorage and re-login

### Audio playback issues

**Problem**: Tracks don't play or have CORS errors.

**Solutions:**
1. Ensure `/api/stream/{track_id}` endpoint is accessible
2. Check server CORS headers for audio streaming
3. Verify audio file exists on server
4. Check browser console for media errors

### Mode selector stuck

**Problem**: Can't switch between demo and server mode.

**Solutions:**
1. Open DevTools → Application → Local Storage
2. Delete `soul-player-mode` key
3. Refresh page

## Development Guidelines

### Adding New API Endpoints

1. Add method to `ApiClient` class in `src/api/client.ts`
2. Use `ServerBackendProvider` from `@soul-player/shared` (no changes needed)
3. Test with server running

### Shared Components

**NEVER** create custom page components - always use `@soul-player/shared`:

```typescript
// ✅ CORRECT
import { AlbumPage } from '@soul-player/shared'

// ❌ WRONG
// Creating custom AlbumPage breaks desktop/web parity
```

### Authentication Context

The `AuthProvider` exposes:

```typescript
interface AuthContextValue {
  user: { id: string; username: string } | null
  isAuthenticated: boolean
  isLoading: boolean
  token: string | null  // Access token for API calls
  login: (username: string, password: string) => Promise<void>
  logout: () => void
}
```

## Future Enhancements

- [ ] WebSocket support for real-time sync
- [ ] Offline mode with service worker
- [ ] Progressive Web App (PWA) support
- [ ] Push notifications for updates
- [ ] Multi-device queue sync
- [ ] Collaborative playlists

## Related Documentation

- [Backend Context Architecture](../../CLAUDE.md#backend-abstraction-backendcontext)
- [Player Commands Context](../../CLAUDE.md#playback-architecture-critical)
- [Soul Player Server API](../server/README.md)

---

**Last Updated**: 2026-01-23
