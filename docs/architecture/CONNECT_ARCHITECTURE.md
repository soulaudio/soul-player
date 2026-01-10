# Soul Connect Architecture

## Overview

Soul Connect enables multi-device features for Soul Player:
- **Device Presence** - Track all devices logged into a user's account
- **Remote Playback Control** - Control playback on any device from any other device
- **Automatic Sync** - Keep playback state synchronized across all devices

This is similar to Spotify Connect but self-hosted.

---

## Architecture Diagram

```
                    ┌─────────────────────────────────────────────┐
                    │              soul-server                     │
                    │                                              │
                    │  ┌────────────────────────────────────────┐ │
                    │  │          WebSocket Hub                  │ │
                    │  │                                         │ │
                    │  │  - Connection management                │ │
                    │  │  - Message routing                      │ │
                    │  │  - Heartbeat monitoring                 │ │
                    │  └────────────────────────────────────────┘ │
                    │                    │                         │
                    │  ┌─────────────────┴─────────────────────┐  │
                    │  │                                        │  │
                    │  ▼                                        ▼  │
                    │  ┌──────────────┐    ┌──────────────────┐   │
                    │  │   Device     │    │    Playback      │   │
                    │  │   Registry   │    │    Router        │   │
                    │  │              │    │                  │   │
                    │  │ - Online     │    │ - State sync     │   │
                    │  │ - Offline    │    │ - Transfer       │   │
                    │  │ - Caps       │    │ - Commands       │   │
                    │  └──────────────┘    └──────────────────┘   │
                    │                                              │
                    └──────────────────────────────────────────────┘
                                         │
                                         │ WebSocket (TLS)
                 ┌───────────────────────┼───────────────────────┐
                 │                       │                       │
                 ▼                       ▼                       ▼
          ┌─────────────┐        ┌─────────────┐        ┌─────────────┐
          │   Desktop   │        │   Mobile    │        │    DAP      │
          │             │        │             │        │  (ESP32)    │
          │ ┌─────────┐ │        │ ┌─────────┐ │        │ ┌─────────┐ │
          │ │ Connect │ │        │ │ Connect │ │        │ │ Connect │ │
          │ │ Client  │ │        │ │ Client  │ │        │ │ Client  │ │
          │ └─────────┘ │        │ └─────────┘ │        │ └─────────┘ │
          │      │      │        │      │      │        │      │      │
          │      ▼      │        │      ▼      │        │      ▼      │
          │ ┌─────────┐ │        │ ┌─────────┐ │        │ ┌─────────┐ │
          │ │ Audio   │ │        │ │ Audio   │ │        │ │ Audio   │ │
          │ │ Engine  │ │        │ │ Engine  │ │        │ │ Engine  │ │
          │ └─────────┘ │        │ └─────────┘ │        │ └─────────┘ │
          └─────────────┘        └─────────────┘        └─────────────┘
           Can Render             Can Render             Can Render
           Can Control            Can Control            Render Only
```

---

## Core Concepts

### Device

A device is any client that can connect to Soul Player:

```rust
pub struct Device {
    pub id: Uuid,
    pub user_id: i64,
    pub name: String,
    pub device_type: DeviceType,
    pub capabilities: DeviceCapabilities,
    pub is_online: bool,
    pub last_seen: DateTime<Utc>,
}

pub enum DeviceType {
    Desktop,    // Windows/macOS/Linux app
    Mobile,     // iOS/Android app
    Dap,        // ESP32-S3 hardware player
    Web,        // Browser-based (jam guest)
}

pub struct DeviceCapabilities {
    pub can_render: bool,      // Can play audio
    pub can_control: bool,     // Can control other devices
    pub can_host_jam: bool,    // Can host jam sessions
    pub supported_formats: Vec<AudioFormat>,
    pub max_sample_rate: u32,
}
```

### Playback State

The canonical state of what's playing:

```rust
pub struct PlaybackState {
    pub device_id: Uuid,           // Which device is rendering
    pub track_id: Option<TrackId>,
    pub position_ms: u64,
    pub is_playing: bool,
    pub queue: Vec<TrackId>,
    pub queue_position: usize,
    pub volume: f32,               // 0.0 - 1.0
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub updated_at: DateTime<Utc>,
}
```

---

## Device Registry

### Registration Flow

```
┌──────────┐                          ┌──────────┐
│  Client  │                          │  Server  │
└────┬─────┘                          └────┬─────┘
     │                                     │
     │  1. WebSocket connect               │
     │ ──────────────────────────────────▶ │
     │                                     │
     │  2. Auth (JWT in header or first msg)
     │ ──────────────────────────────────▶ │
     │                                     │
     │  3. Register device                 │
     │  { type: "register",                │
     │    device: { name, type, caps } }   │
     │ ──────────────────────────────────▶ │
     │                                     │
     │  4. Registration ACK + device list  │
     │  { type: "registered",              │
     │    device_id: "uuid",               │
     │    devices: [...] }                 │
     │ ◀────────────────────────────────── │
     │                                     │
     │  5. Heartbeat (every 30s)           │
     │  { type: "ping" }                   │
     │ ──────────────────────────────────▶ │
     │                                     │
     │  { type: "pong" }                   │
     │ ◀────────────────────────────────── │
```

### Database Schema

```sql
-- User's registered devices
CREATE TABLE user_devices (
    id TEXT PRIMARY KEY,                    -- UUID
    user_id INTEGER NOT NULL REFERENCES users(id),
    device_name TEXT NOT NULL,
    device_type TEXT NOT NULL,              -- 'desktop', 'mobile', 'dap', 'web'
    capabilities_json TEXT NOT NULL,        -- DeviceCapabilities serialized
    last_seen_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, device_name)            -- No duplicate names per user
);

-- Active WebSocket sessions (in-memory, not persisted)
-- This is managed by the server's ConnectionManager

-- Last known playback state per device (for resume)
CREATE TABLE device_playback_state (
    device_id TEXT PRIMARY KEY REFERENCES user_devices(id),
    state_json TEXT NOT NULL,               -- PlaybackState serialized
    updated_at TEXT NOT NULL
);
```

---

## Remote Playback Control

### Transfer Playback

Moving playback from one device to another:

```
┌──────────┐            ┌──────────┐            ┌──────────┐
│ Phone    │            │  Server  │            │ Desktop  │
│(Control) │            │          │            │(Render)  │
└────┬─────┘            └────┬─────┘            └────┬─────┘
     │                       │                       │
     │  User taps            │                       │
     │  "Play on Desktop"    │                       │
     │                       │                       │
     │  1. transfer_playback │                       │
     │  { to: "desktop-id",  │                       │
     │    position: 45000 }  │                       │
     │ ─────────────────────▶│                       │
     │                       │                       │
     │                       │  2. start_playback    │
     │                       │  { track, pos, queue }│
     │                       │ ─────────────────────▶│
     │                       │                       │
     │                       │                       │ 3. Desktop
     │                       │                       │    starts
     │                       │                       │    playing
     │                       │                       │
     │                       │  4. playback_started  │
     │                       │  { state: {...} }     │
     │                       │ ◀─────────────────────│
     │                       │                       │
     │  5. state_update      │                       │
     │  (broadcast to all)   │                       │
     │ ◀─────────────────────│─────────────────────▶ │
```

### Command Routing

All playback commands go through the server:

```rust
pub enum PlaybackCommand {
    Play,
    Pause,
    Stop,
    Seek { position_ms: u64 },
    Next,
    Previous,
    SetQueue { tracks: Vec<TrackId>, start_index: usize },
    SetVolume { volume: f32 },
    SetShuffle { enabled: bool },
    SetRepeat { mode: RepeatMode },
}

// Client sends:
{
    "type": "playback_command",
    "target_device": "uuid-of-rendering-device",
    "command": { "type": "seek", "position_ms": 60000 }
}

// Server routes to target device:
{
    "type": "execute_command",
    "command": { "type": "seek", "position_ms": 60000 }
}

// Target device executes and broadcasts new state
```

---

## State Synchronization

### Real-Time Updates

The rendering device broadcasts state changes:

```
Rendering Device                Server              All Other Devices
       │                          │                        │
       │  Position update         │                        │
       │  (every 1s while playing)│                        │
       │  { type: "state",        │                        │
       │    position_ms: 61000 }  │                        │
       │ ────────────────────────▶│                        │
       │                          │                        │
       │                          │  Broadcast to all      │
       │                          │  user's devices        │
       │                          │ ──────────────────────▶│
       │                          │                        │
       │  Track change            │                        │
       │  (immediate)             │                        │
       │ ────────────────────────▶│──────────────────────▶ │
```

### Conflict Resolution

When multiple devices send conflicting commands:

1. **Timestamp wins** - Latest timestamp takes precedence
2. **Rendering device is authoritative** - Its state is the source of truth
3. **Optimistic UI** - Controllers update immediately, rollback if rejected

```rust
pub struct StateUpdate {
    pub state: PlaybackState,
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,  // Monotonic sequence number
}

// Server tracks per-user:
pub struct UserPlaybackSession {
    pub current_state: PlaybackState,
    pub last_sequence: u64,
    pub pending_commands: VecDeque<PendingCommand>,
}
```

---

## Reconnection Handling

### Graceful Disconnect

```rust
// Client sends before closing:
{ "type": "disconnect", "reason": "app_closed" }

// Server marks device offline but preserves state
// State can be resumed within 24 hours
```

### Connection Lost (Unexpected)

```
1. Server detects via heartbeat timeout (90s)
2. Server marks device offline
3. Server broadcasts: { "type": "device_offline", "device_id": "..." }
4. If this was the rendering device:
   - Playback pauses
   - Other devices see "Playback paused - [Device] went offline"
5. When device reconnects:
   - Can resume playback from saved state
   - Or transfer to another device
```

### Auto-Resume

```rust
// On reconnect, client sends:
{
    "type": "register",
    "device": { ... },
    "resume": true,
    "last_known_state": { ... }  // Optional
}

// Server responds with:
{
    "type": "registered",
    "device_id": "...",
    "current_state": { ... },  // From another device or saved
    "should_resume": true      // If this device was rendering
}
```

---

## soul-connect Library

### API Design

```rust
// libraries/soul-connect/src/lib.rs

pub struct ConnectClient {
    config: ConnectConfig,
    ws: Option<WebSocketConnection>,
    device_id: Option<Uuid>,
    state: Arc<RwLock<ConnectState>>,
}

pub struct ConnectConfig {
    pub server_url: String,
    pub device_name: String,
    pub device_type: DeviceType,
    pub auth_token: String,
    pub reconnect_policy: ReconnectPolicy,
}

impl ConnectClient {
    /// Create new client
    pub fn new(config: ConnectConfig) -> Self;

    /// Connect to server and register device
    pub async fn connect(&mut self) -> Result<()>;

    /// Disconnect gracefully
    pub async fn disconnect(&mut self) -> Result<()>;

    /// Get all devices for current user
    pub async fn list_devices(&self) -> Result<Vec<Device>>;

    /// Get current playback state
    pub fn playback_state(&self) -> Option<PlaybackState>;

    /// Transfer playback to another device
    pub async fn transfer_playback(
        &self,
        target_device: Uuid,
        options: TransferOptions,
    ) -> Result<()>;

    /// Send playback command to rendering device
    pub async fn send_command(&self, cmd: PlaybackCommand) -> Result<()>;

    /// Subscribe to state changes
    pub fn subscribe_state(&self) -> broadcast::Receiver<PlaybackState>;

    /// Subscribe to device list changes
    pub fn subscribe_devices(&self) -> broadcast::Receiver<Vec<Device>>;

    /// Report local playback state (when this device is rendering)
    pub async fn report_state(&self, state: PlaybackState) -> Result<()>;
}
```

### Usage Example

```rust
// In Tauri desktop app

let connect = ConnectClient::new(ConnectConfig {
    server_url: "wss://my-server.com/ws".into(),
    device_name: "MacBook Pro".into(),
    device_type: DeviceType::Desktop,
    auth_token: user_token.clone(),
    reconnect_policy: ReconnectPolicy::exponential_backoff(),
});

// Connect and register
connect.connect().await?;

// Listen for state changes
let mut state_rx = connect.subscribe_state();
tokio::spawn(async move {
    while let Ok(state) = state_rx.recv().await {
        // Update UI with new state
        update_now_playing(state);
    }
});

// Transfer playback from phone to this device
connect.transfer_playback(this_device_id, TransferOptions::default()).await?;

// When user presses play
connect.send_command(PlaybackCommand::Play).await?;
```

---

## WebSocket Protocol

### Message Types

```typescript
// TypeScript types for reference

// Client -> Server
type ClientMessage =
  | { type: "register"; device: DeviceInfo; resume?: boolean }
  | { type: "ping" }
  | { type: "disconnect"; reason: string }
  | { type: "playback_command"; target_device: string; command: PlaybackCommand }
  | { type: "transfer_playback"; to_device: string; options?: TransferOptions }
  | { type: "state_update"; state: PlaybackState }
  | { type: "request_devices" }

// Server -> Client
type ServerMessage =
  | { type: "registered"; device_id: string; devices: Device[] }
  | { type: "pong" }
  | { type: "devices_update"; devices: Device[] }
  | { type: "device_online"; device: Device }
  | { type: "device_offline"; device_id: string }
  | { type: "playback_state"; state: PlaybackState }
  | { type: "execute_command"; command: PlaybackCommand }
  | { type: "error"; code: string; message: string }
```

### Error Codes

| Code | Meaning |
|------|---------|
| `device_not_found` | Target device doesn't exist |
| `device_offline` | Target device is not connected |
| `not_authorized` | User doesn't own target device |
| `invalid_command` | Malformed command |
| `rate_limited` | Too many requests |

---

## Security Considerations

### Authentication

- JWT token required in WebSocket handshake
- Token validated on every connection
- Devices can only see/control same user's devices

### Rate Limiting

- Max 100 messages/minute per connection
- Max 10 state updates/second from rendering device
- Transfer playback limited to 1/second

### Data Privacy

- Playback state stored only in memory (not persisted long-term)
- Device history limited to 30 days
- Users can delete devices from their account

---

## Performance Targets

| Metric | Target |
|--------|--------|
| State sync latency | < 500ms |
| Command execution | < 200ms |
| Reconnection time | < 5s |
| Max devices per user | 10 |
| Max concurrent connections | 1000 per server |
| Memory per connection | < 10 KB |

---

## Implementation Phases

### Phase 1: Core Infrastructure
- [ ] WebSocket server setup in soul-server
- [ ] Device registration/deregistration
- [ ] Heartbeat and presence
- [ ] Basic message routing

### Phase 2: Playback Control
- [ ] State synchronization
- [ ] Command routing
- [ ] Transfer playback
- [ ] Conflict resolution

### Phase 3: Client Library
- [ ] soul-connect crate
- [ ] Tauri integration
- [ ] Reconnection handling
- [ ] Offline queueing

### Phase 4: UI Components
- [ ] Device selector
- [ ] "Playing on" indicator
- [ ] Device management page
