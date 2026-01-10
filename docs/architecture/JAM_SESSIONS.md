# Jam Sessions Architecture

## Overview

Jam Sessions enable collaborative listening - multiple people controlling a shared queue, similar to Spotify Jam. Key features:

- **Shareable links** - Anyone with the link can join
- **Anonymous guests** - No account required to participate
- **Collaborative queue** - Everyone can add songs
- **Real-time sync** - All participants see updates instantly
- **Permission system** - Host controls who can do what

---

## User Experience

### Starting a Jam

```
┌─────────────────────────────────────────────────┐
│  🎵 Now Playing                                 │
│                                                 │
│  ┌─────┐                                        │
│  │     │  Song Title                            │
│  │ Art │  Artist Name                           │
│  │     │                                        │
│  └─────┘  ▶ ▮▮ ⏭  ─────●───────── 2:34 / 4:12  │
│                                                 │
│  ┌─────────────────────────────────────────┐    │
│  │  👥 Start a Jam                         │    │
│  │                                         │    │
│  │  Listen together with friends!          │    │
│  │  Share a link and let them add songs.   │    │
│  │                                         │    │
│  │  [Start Jam Session]                    │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### Active Jam Session

```
┌─────────────────────────────────────────────────┐
│  🎵 Jam Session                    [End Jam]    │
│                                                 │
│  Share link: soul.example.com/jam/ABCD-1234    │
│  [Copy Link] [QR Code]                          │
│                                                 │
│  ─────────────────────────────────────────────  │
│                                                 │
│  👥 Participants (4)                            │
│  ┌─────────────────────────────────────────┐    │
│  │ 👑 You (Host)                           │    │
│  │ 🎵 Alex                                 │    │
│  │ 🎵 Jordan                               │    │
│  │ 🎵 Guest-7842                           │    │
│  └─────────────────────────────────────────┘    │
│                                                 │
│  ─────────────────────────────────────────────  │
│                                                 │
│  📋 Queue (6 songs)                             │
│  ┌─────────────────────────────────────────┐    │
│  │ ▶ Current: "Song A" - Artist            │    │
│  │   Added by You                          │    │
│  ├─────────────────────────────────────────┤    │
│  │ 2. "Song B" - Artist  (Alex) [🗑️]       │    │
│  │ 3. "Song C" - Artist  (Jordan)          │    │
│  │ 4. "Song D" - Artist  (Guest-7842)      │    │
│  │ 5. "Song E" - Artist  (You)             │    │
│  │ 6. "Song F" - Artist  (Alex) [🗑️]       │    │
│  └─────────────────────────────────────────┘    │
│                                                 │
│  [+ Add Song]                                   │
└─────────────────────────────────────────────────┘
```

### Guest Web View

```
┌─────────────────────────────────────────────────┐
│  🎵 You're in Alex's Jam                        │
│                                                 │
│  ┌─────┐                                        │
│  │     │  Currently Playing:                    │
│  │ Art │  "Song Title" - Artist                 │
│  │     │  ─────●───────── 2:34 / 4:12          │
│  └─────┘                                        │
│                                                 │
│  ─────────────────────────────────────────────  │
│                                                 │
│  📋 Up Next                                     │
│  1. "Song B" - Artist  (Alex)                   │
│  2. "Song C" - Artist  (You!) 🎉                │
│  3. "Song D" - Artist  (Jordan)                 │
│                                                 │
│  ─────────────────────────────────────────────  │
│                                                 │
│  🔍 Add a song                                  │
│  ┌─────────────────────────────────────────┐    │
│  │ Search songs...                         │    │
│  └─────────────────────────────────────────┘    │
│                                                 │
│  Search results...                              │
│  ┌─────────────────────────────────────────┐    │
│  │ "Found Song" - Artist       [+ Add]     │    │
│  │ "Another Song" - Artist     [+ Add]     │    │
│  └─────────────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

---

## Architecture

### System Overview

```
                           ┌───────────────────────────────────────┐
                           │            soul-server                 │
                           │                                        │
                           │  ┌──────────────────────────────────┐ │
                           │  │      Jam Session Manager          │ │
                           │  │                                   │ │
                           │  │  - Active sessions (in-memory)    │ │
                           │  │  - Participant tracking           │ │
                           │  │  - Queue management               │ │
                           │  │  - Permission enforcement         │ │
                           │  └──────────────────────────────────┘ │
                           │                 │                      │
                           │    ┌────────────┴────────────┐        │
                           │    │                         │        │
                           │    ▼                         ▼        │
                           │  ┌─────────────┐   ┌─────────────┐    │
                           │  │  WebSocket  │   │    REST     │    │
                           │  │  /ws/jam/*  │   │  /api/jam/* │    │
                           │  └─────────────┘   └─────────────┘    │
                           └───────────────────────────────────────┘
                                        │              │
           ┌────────────────────────────┼──────────────┼────────────────────────┐
           │                            │              │                        │
           ▼                            ▼              ▼                        ▼
    ┌─────────────┐             ┌─────────────┐ ┌─────────────┐         ┌─────────────┐
    │    Host     │             │   Member    │ │   Member    │         │    Guest    │
    │  (Desktop)  │             │  (Mobile)   │ │  (Desktop)  │         │   (Web)     │
    │             │             │             │ │             │         │             │
    │ - Plays     │             │ - Control   │ │ - Control   │         │ - View      │
    │   audio     │             │   queue     │ │   queue     │         │ - Add songs │
    │ - Full ctrl │             │ - Add songs │ │ - Add songs │         │             │
    └─────────────┘             └─────────────┘ └─────────────┘         └─────────────┘
```

### Data Model

```rust
/// A collaborative listening session
pub struct JamSession {
    pub id: Uuid,
    pub share_code: String,           // e.g., "ABCD-1234"
    pub host_user_id: i64,
    pub host_device_id: Uuid,
    pub settings: JamSettings,
    pub participants: Vec<JamParticipant>,
    pub queue: Vec<JamQueueItem>,
    pub current_index: usize,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

pub struct JamSettings {
    pub name: Option<String>,         // "Alex's Jam"
    pub allow_guests: bool,           // Anonymous users can join
    pub require_approval: bool,       // Host approves new joiners
    pub guest_can_add: bool,          // Guests can add to queue
    pub guest_can_skip_vote: bool,    // Guests can vote to skip
    pub max_queue_per_person: usize,  // Limit songs per person (0 = unlimited)
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct JamParticipant {
    pub id: Uuid,
    pub user_id: Option<i64>,         // None for anonymous guests
    pub display_name: String,
    pub permissions: JamPermissions,
    pub ws_connection_id: Option<Uuid>,
    pub joined_at: DateTime<Utc>,
}

pub struct JamPermissions {
    pub can_add_to_queue: bool,
    pub can_remove_own: bool,         // Remove songs they added
    pub can_remove_any: bool,         // Host only
    pub can_reorder: bool,            // Host only
    pub can_skip: bool,               // Host only (or vote)
    pub can_kick: bool,               // Host only
}

pub struct JamQueueItem {
    pub id: Uuid,
    pub track_id: TrackId,
    pub track_info: TrackInfo,        // Cached for display
    pub added_by: Uuid,               // Participant ID
    pub added_at: DateTime<Utc>,
    pub played_at: Option<DateTime<Utc>>,
}
```

### Database Schema

```sql
-- Jam sessions (persisted for history/resume)
CREATE TABLE jam_sessions (
    id TEXT PRIMARY KEY,
    host_user_id INTEGER NOT NULL REFERENCES users(id),
    host_device_id TEXT NOT NULL,
    share_code TEXT UNIQUE NOT NULL,
    name TEXT,
    settings_json TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ended_at TEXT,

    -- Index for quick share code lookup
    UNIQUE(share_code)
);

CREATE INDEX idx_jam_sessions_host ON jam_sessions(host_user_id);
CREATE INDEX idx_jam_sessions_active ON jam_sessions(is_active) WHERE is_active = 1;

-- Jam participants
CREATE TABLE jam_participants (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES jam_sessions(id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users(id),  -- NULL for anonymous
    display_name TEXT NOT NULL,
    permissions_json TEXT NOT NULL,
    is_host INTEGER NOT NULL DEFAULT 0,
    joined_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    left_at TEXT,

    UNIQUE(session_id, user_id)  -- One entry per user per session
);

CREATE INDEX idx_jam_participants_session ON jam_participants(session_id);

-- Jam queue (songs added to session)
CREATE TABLE jam_queue (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES jam_sessions(id) ON DELETE CASCADE,
    track_id TEXT NOT NULL,
    track_info_json TEXT NOT NULL,  -- Cached TrackInfo
    added_by_participant_id TEXT NOT NULL REFERENCES jam_participants(id),
    position INTEGER NOT NULL,
    played_at TEXT,  -- NULL if not played yet
    removed_at TEXT, -- NULL if still in queue
    added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_jam_queue_session ON jam_queue(session_id);
CREATE INDEX idx_jam_queue_position ON jam_queue(session_id, position)
    WHERE removed_at IS NULL AND played_at IS NULL;
```

---

## WebSocket Protocol

### Joining a Session

```
┌──────────┐                        ┌──────────┐
│  Guest   │                        │  Server  │
└────┬─────┘                        └────┬─────┘
     │                                   │
     │  1. Connect to /ws/jam/ABCD-1234  │
     │ ────────────────────────────────▶ │
     │                                   │
     │  2. join_request                  │
     │  { display_name: "Alex" }         │
     │ ────────────────────────────────▶ │
     │                                   │
     │        (if require_approval)      │
     │                                   │
     │  3a. join_pending                 │
     │  { message: "Waiting for host" }  │
     │ ◀──────────────────────────────── │
     │                                   │
     │        (host approves)            │
     │                                   │
     │  3b. joined                       │
     │  { session: {...},                │
     │    participant_id: "uuid",        │
     │    permissions: {...} }           │
     │ ◀──────────────────────────────── │
```

### Message Types

```typescript
// Client -> Server
type JamClientMessage =
  | { type: "join_request"; display_name: string; user_token?: string }
  | { type: "leave" }
  | { type: "add_track"; track_id: string }
  | { type: "remove_track"; queue_item_id: string }
  | { type: "vote_skip" }
  | { type: "chat"; message: string }  // Optional feature
  // Host only:
  | { type: "approve_join"; participant_id: string }
  | { type: "deny_join"; participant_id: string }
  | { type: "kick"; participant_id: string }
  | { type: "skip" }
  | { type: "reorder"; queue_item_id: string; new_position: number }
  | { type: "update_settings"; settings: Partial<JamSettings> }
  | { type: "end_session" }

// Server -> Client
type JamServerMessage =
  | { type: "joined"; session: JamSession; participant_id: string }
  | { type: "join_pending"; message: string }
  | { type: "join_denied"; reason: string }
  | { type: "session_update"; session: JamSession }
  | { type: "participant_joined"; participant: JamParticipant }
  | { type: "participant_left"; participant_id: string }
  | { type: "queue_update"; queue: JamQueueItem[] }
  | { type: "track_added"; item: JamQueueItem; by: string }
  | { type: "track_removed"; item_id: string; by: string }
  | { type: "now_playing"; item: JamQueueItem }
  | { type: "notification"; message: string; type: "info" | "success" | "warning" }
  | { type: "chat_message"; from: string; message: string }  // Optional
  | { type: "kicked"; reason?: string }
  | { type: "session_ended" }
  | { type: "error"; code: string; message: string }
```

### Real-Time Updates

```
Host Device                Server              All Participants
    │                        │                        │
    │  Track ended           │                        │
    │  (auto-advance)        │                        │
    │                        │                        │
    │  state_update          │                        │
    │  { now_playing: ... }  │                        │
    │ ──────────────────────▶│                        │
    │                        │                        │
    │                        │  Broadcast             │
    │                        │  now_playing           │
    │                        │ ──────────────────────▶│
    │                        │                        │
    │                        │                        │
    │                        │◀─────────────────────  │
    │                        │  add_track             │
    │                        │  { track_id: "..." }   │
    │                        │                        │
    │                        │  Broadcast             │
    │  track_added           │  track_added           │
    │ ◀──────────────────────│──────────────────────▶ │
```

---

## Share Code Generation

### Format

Share codes are designed to be:
- Easy to type and share verbally
- Unambiguous (no 0/O, 1/I/L confusion)
- Short enough for casual sharing

```rust
const SHARE_CODE_CHARS: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
// Excludes: I, L, O, 0, 1

pub fn generate_share_code() -> String {
    // Format: XXXX-XXXX (8 chars, ~1.2 billion combinations)
    let mut rng = rand::thread_rng();
    let chars: String = (0..8)
        .map(|_| SHARE_CODE_CHARS[rng.gen_range(0..SHARE_CODE_CHARS.len())] as char)
        .collect();
    format!("{}-{}", &chars[0..4], &chars[4..8])
}

// Examples: "ABCD-1234", "XYZW-5678", "HQRS-2K4M"
```

### Shareable URL

```
https://your-server.com/jam/ABCD-1234
```

This URL serves:
1. **Web UI** for guests (if accessed in browser)
2. **Deep link** to native app (if installed)
3. **QR code** generation for in-person sharing

---

## REST API

### Endpoints

```
# Create a new jam session
POST /api/jam/sessions
Authorization: Bearer <token>
{
    "settings": {
        "name": "Friday Night Jam",
        "allow_guests": true,
        "require_approval": false
    }
}
Response: { "session_id": "uuid", "share_code": "ABCD-1234" }

# Get session info (public, for landing page)
GET /api/jam/sessions/{share_code}
Response: {
    "name": "Friday Night Jam",
    "host_name": "Alex",
    "participant_count": 4,
    "current_track": { "title": "...", "artist": "..." },
    "is_active": true
}

# Search tracks (for adding to queue)
GET /api/jam/sessions/{share_code}/search?q=song+name
Authorization: Bearer <token> (or guest session)
Response: { "tracks": [...] }

# Get session history (for host)
GET /api/jam/history
Authorization: Bearer <token>
Response: { "sessions": [...] }
```

---

## Guest Web Application

### Tech Stack

- **Framework**: React (shared with desktop)
- **Bundled in**: soul-server (served statically)
- **Size target**: < 100KB gzipped

### Routes

```
/jam/{code}           - Landing page / join
/jam/{code}/session   - Active session view
```

### Component Structure

```
applications/jam-web/
├── src/
│   ├── App.tsx
│   ├── pages/
│   │   ├── JoinPage.tsx       # Enter name, join session
│   │   ├── SessionPage.tsx    # Active session view
│   │   └── EndedPage.tsx      # Session ended message
│   ├── components/
│   │   ├── NowPlaying.tsx
│   │   ├── Queue.tsx
│   │   ├── SearchTracks.tsx
│   │   ├── Participants.tsx
│   │   └── Notifications.tsx
│   ├── hooks/
│   │   └── useJamSession.ts   # WebSocket connection
│   └── styles/
│       └── index.css
├── index.html
├── vite.config.ts
└── package.json
```

---

## Permission Matrix

| Action | Host | Member | Guest |
|--------|------|--------|-------|
| View session | ✅ | ✅ | ✅ |
| Add to queue | ✅ | ✅ | ⚙️ (configurable) |
| Remove own songs | ✅ | ✅ | ⚙️ |
| Remove any song | ✅ | ❌ | ❌ |
| Reorder queue | ✅ | ❌ | ❌ |
| Skip current | ✅ | ❌ | ❌ |
| Vote to skip | ✅ | ✅ | ⚙️ |
| Kick participant | ✅ | ❌ | ❌ |
| End session | ✅ | ❌ | ❌ |
| Change settings | ✅ | ❌ | ❌ |

⚙️ = Configurable by host in session settings

---

## Vote to Skip

Optional feature where participants can vote to skip the current track:

```rust
pub struct SkipVote {
    pub session_id: Uuid,
    pub queue_item_id: Uuid,
    pub votes: HashSet<Uuid>,  // Participant IDs
    pub threshold: f32,         // e.g., 0.5 = 50% of participants
}

impl SkipVote {
    pub fn should_skip(&self, participant_count: usize) -> bool {
        let required = (participant_count as f32 * self.threshold).ceil() as usize;
        self.votes.len() >= required
    }
}
```

UI shows:
```
⏭️ Skip? (3/5 votes)  [Vote to Skip]
```

---

## Notifications

In-session notifications keep everyone informed:

```rust
pub enum JamNotification {
    ParticipantJoined { name: String },
    ParticipantLeft { name: String },
    TrackAdded { track: String, by: String },
    TrackRemoved { track: String, by: String },
    NowPlaying { track: String },
    SessionEnding { in_seconds: u32 },
    HostTransferred { new_host: String },
}
```

Display as toast notifications:

```
┌─────────────────────────────────────┐
│ 🎵 Alex added "Song Name"           │
└─────────────────────────────────────┘
```

---

## Error Handling

### Error Codes

| Code | Meaning | User Message |
|------|---------|--------------|
| `session_not_found` | Invalid share code | "This jam session doesn't exist" |
| `session_ended` | Session is no longer active | "This jam session has ended" |
| `session_full` | Max participants reached | "This jam is full" |
| `not_authorized` | Action not permitted | "You don't have permission to do that" |
| `already_joined` | User already in session | "You're already in this jam" |
| `track_not_found` | Track doesn't exist | "Song not found" |
| `queue_full` | Personal queue limit reached | "You've reached your song limit" |
| `rate_limited` | Too many actions | "Slow down! Try again in a moment" |

---

## Session Lifecycle

```
┌───────────┐     ┌───────────┐     ┌───────────┐     ┌───────────┐
│  Created  │────▶│  Active   │────▶│  Ending   │────▶│   Ended   │
└───────────┘     └───────────┘     └───────────┘     └───────────┘
                       │                  │
                       │ (host offline)   │
                       ▼                  │
                  ┌───────────┐           │
                  │  Paused   │───────────┘
                  └───────────┘
                       │
                       │ (host returns within 5min)
                       ▼
                  ┌───────────┐
                  │  Resumed  │
                  └───────────┘
```

### Auto-Cleanup

- Sessions with no activity for 30 minutes → End automatically
- Host disconnected for 5 minutes → Pause session
- Host disconnected for 30 minutes → End session
- All participants left → End session
- Max session duration: 24 hours

---

## Implementation Phases

### Phase 1: Core Session Management
- [ ] JamSession data model
- [ ] Database schema and migrations
- [ ] Create/end session API
- [ ] Share code generation

### Phase 2: WebSocket Integration
- [ ] WebSocket endpoint for jam sessions
- [ ] Join/leave flow
- [ ] Real-time state broadcast
- [ ] Participant tracking

### Phase 3: Queue Management
- [ ] Add/remove tracks
- [ ] Track search for guests
- [ ] Queue reordering (host)
- [ ] Now playing sync

### Phase 4: Guest Web UI
- [ ] Join page
- [ ] Session view
- [ ] Track search and add
- [ ] Real-time updates

### Phase 5: Advanced Features
- [ ] Vote to skip
- [ ] Chat (optional)
- [ ] Session history
- [ ] QR code generation
