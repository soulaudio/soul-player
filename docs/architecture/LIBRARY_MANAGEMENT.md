# Library Management Architecture

> **Status**: Draft - Pending Review
> **Last Updated**: 2025-01-10
> **Author**: Soul Player Team
> **Reviewers**: Pending

This document defines how Soul Player handles audio file importing, library organization, and file tracking across all platforms.

---

## Table of Contents

1. [Overview](#overview)
2. [Import Strategies](#import-strategies)
3. [External File Handling](#external-file-handling)
4. [File Tracking](#file-tracking)
5. [Supported Formats](#supported-formats)
6. [Path Templates](#path-templates)
7. [Multi-Platform Architecture](#multi-platform-architecture)
8. [Database Schema](#database-schema)
9. [Testing Requirements](#testing-requirements)
10. [Implementation Roadmap](#implementation-roadmap)
11. [Research Sources](#research-sources)

---

## Overview

Soul Player supports **two import strategies** to accommodate different user workflows:

| Strategy | Target User | Behavior |
|----------|-------------|----------|
| **Watched Folders** | Audiophiles with organized collections | Monitor folders, never modify files |
| **Managed Library** | Casual users wanting organization help | Copy files to organized structure |

Both strategies are available from v1. Users can use them simultaneously (hybrid mode).

### Design Principles

1. **Non-destructive by default**: Watched folders never modify source files
2. **Efficient scanning**: Use mtime + size checks before expensive operations
3. **Resilient to moves**: Content hash enables file relocation detection
4. **Background-first**: Heavy operations (fingerprinting) happen in background
5. **Multi-user ready**: All data scoped by `user_id` from day 1
6. **Per-device sources**: Each device manages its own library sources

---

## Import Strategies

### Strategy 1: Watched Folders

Inspired by: [foobar2000](https://wiki.hydrogenaudio.org/index.php?title=Foobar2000:Preferences:Media_Library), [Roon](https://roon.app/en/music/organization), [Navidrome](https://www.navidrome.org/docs/faq/)

**Behavior**:
- User points to existing folders containing audio files
- Soul Player indexes files and monitors for changes
- **Never modifies, moves, or copies source files**
- Supports multiple watched folders per user
- Hidden files are excluded (following foobar2000 convention)

**Scan Triggers**:
1. **Startup scan**: Incremental scan on app launch
2. **Filesystem watcher**: Real-time updates while app is running
3. **Manual rescan**: User-triggered full rescan

**Soft Delete Behavior**:
- When a file disappears, mark as `is_available = false`
- Search for matching content hash in other watched folders
- If found elsewhere, silently update the path
- Keep metadata and playlist references intact
- User can filter to show/hide unavailable tracks

### Strategy 2: Managed Library

Inspired by: [JRiver Media Center](https://wiki.jriver.com/index.php/Rename,_Move,_and_Copy_Files), [MusicBee](https://musicbee.fandom.com/wiki/Library_Preferences), [beets](https://beets.readthedocs.io/en/stable/reference/pathformat.html)

**Behavior**:
- User designates a single destination folder
- Files can be imported via:
  - Drag & drop onto app
  - File picker dialog
  - "Add to library" context menu
- Files are **copied** (default) to organized structure
- Path template determines folder/file naming

**Default Action**: Copy (preserves originals)

**Duplicate Handling**:
- Compute SHA256 hash of incoming file
- If exact hash already exists in library → skip import
- Show notification: "X files skipped (already in library)"

**First-Run Setup**:
- Prompt user to choose managed library location
- Recommend platform music folder: `~/Music/Soul Player/`
- Allow changing later in settings (requires re-import)

---

## External File Handling

This section covers how Soul Player handles files that are opened or dropped into the player but are **not part of the library**.

### Drag & Drop onto Player

When a user drops audio file(s) anywhere in the Soul Player window:

```
┌─────────────────────────────────────────────────────────────────┐
│  Import or Play Dialog                                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  You dropped 3 audio files                                      │
│                                                                 │
│  What would you like to do?                                     │
│                                                                 │
│  ┌─────────────────────┐  ┌─────────────────────┐              │
│  │  ▶ Just Play        │  │  📥 Import to       │              │
│  │                     │  │     Library         │              │
│  │  Play now without   │  │                     │              │
│  │  adding to library  │  │  Copy to managed    │              │
│  │                     │  │  library folder     │              │
│  └─────────────────────┘  └─────────────────────┘              │
│                                                                 │
│  ☐ Remember my choice                                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### OS File Association

Soul Player registers as a handler for supported audio formats. When a user double-clicks an audio file in their file manager:

1. Soul Player opens (or focuses if already running)
2. Same "Import or Play" dialog appears
3. If "Remember my choice" was checked, uses saved preference

**Supported associations** (v1):
- `.flac`, `.mp3`, `.m4a`, `.aac`, `.ogg`, `.opus`, `.wav`, `.aif`, `.aiff`

### "Just Play" Behavior (In-Memory Only)

When user chooses "Just Play":

1. **Track metadata is read but NOT persisted to database**
2. Track appears in Now Playing / Queue
3. Playback works normally
4. **No database entry is created**
5. After playback ends (or track is removed from queue), all data is discarded
6. Track does NOT appear in library, search, or playlists

```
┌─────────────────────────────────────────────────────────────────┐
│  In-Memory Track Lifecycle                                      │
├─────────────────────────────────────────────────────────────────┤
│  1. User drops file → "Just Play"                               │
│  2. Read metadata into memory (TemporaryTrack struct)           │
│  3. Add to playback queue                                       │
│  4. Play audio from original file path                          │
│  5. When removed from queue → TemporaryTrack is dropped         │
│  6. No trace remains in database                                │
└─────────────────────────────────────────────────────────────────┘
```

**Rationale**: Clean separation between "playing a file" and "adding to library". Users who just want to preview a file shouldn't pollute their library.

### "Import" Behavior

When user chooses "Import to Library":

1. **Default destination**: Managed library folder (configurable in settings)
2. File is copied to organized structure using path template
3. Track is added to database with full metadata
4. **User notification**: "Imported X tracks to library"
5. Track now appears in library, is searchable, can be added to playlists

### Settings for External File Handling

Add to Settings → Library:

```
┌─────────────────────────────────────────────────────────────────┐
│  External Files                                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  When opening files not in library:                             │
│  ○ Always ask                                                   │
│  ○ Always play without importing                                │
│  ○ Always import to library                                     │
│                                                                 │
│  Default import destination:                                    │
│  ○ Managed library (recommended)                                │
│  ○ Add to watched folder: [Select folder...]                    │
│                                                                 │
│  Show notification after import: ☑                              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Edge Cases

| Scenario | Behavior |
|----------|----------|
| **File already in library (hash match)** | Skip dialog, just play from library |
| **File in watched folder but not yet scanned** | Trigger immediate scan of that file, then play |
| **Multiple files dropped, some in library** | Play library tracks, ask about unknown ones |
| **User drags folder** | Scan folder, show dialog with count |
| **File moved while playing** | Continue playing (file handle open), mark unavailable after |

### Implementation Notes

```rust
/// Represents a track that exists only in memory, not in database
pub struct TemporaryTrack {
    pub file_path: PathBuf,
    pub metadata: TrackMetadata,
    // No id field - not persisted
}

/// Check if dropped file is already in library
pub async fn find_track_by_path_or_hash(
    pool: &SqlitePool,
    user_id: i64,
    file_path: &Path,
) -> Result<Option<Track>> {
    // 1. Check by exact path
    // 2. If not found, compute hash and check by hash
    // 3. Return None if truly external
}
```

---

## File Tracking

### Change Detection Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│  Fast Path (mtime + size check)                                 │
├─────────────────────────────────────────────────────────────────┤
│  For each file in watched folder:                               │
│  1. Get current mtime + file_size from filesystem               │
│  2. Compare with stored values in database                      │
│  3. If unchanged → skip (no further processing)                 │
│  4. If changed → re-read metadata, update content_hash          │
│  5. If new file → full import (metadata + hash + add to DB)     │
└─────────────────────────────────────────────────────────────────┘
```

### File Relocation Detection

When a file is missing from its expected path:

1. **Search by content_hash**: Look for exact file match anywhere in sources
2. **If found**: Silently update `file_path` in database
3. **If not found**: Mark `is_available = false`, set `unavailable_since`

This handles common scenarios:
- User reorganized their folder structure
- File moved to different drive
- External drive temporarily disconnected

### Content Hash

- **Algorithm**: SHA256
- **Scope**: Entire file bytes (not just audio data)
- **Purpose**: Exact file deduplication, relocation detection

### Audio Fingerprint

- **Algorithm**: [Chromaprint](https://acoustid.org/chromaprint)
- **Scope**: First 2 minutes of decoded audio
- **Purpose**: Content-based matching (cross-format), discovery service integration

**Fingerprinting is a background task**:
- Does not block import
- Processed in low-priority queue
- UI indicator: "Processing audio fingerprints..."

---

## Supported Formats

### v1 Core Formats

All formats supported by Symphonia decoder:

| Format | Extensions | Container | Notes |
|--------|------------|-----------|-------|
| FLAC | `.flac` | Native | Lossless, most popular for audiophiles |
| MP3 | `.mp3` | Native | Most compatible format |
| AAC | `.m4a`, `.aac` | MP4/ADTS | iTunes default |
| OGG Vorbis | `.ogg` | Ogg | Spotify format |
| Opus | `.opus` | Ogg | Modern, efficient codec |
| WAV | `.wav` | RIFF | Uncompressed PCM |
| ALAC | `.m4a` | MP4 | Apple Lossless |
| AIFF | `.aif`, `.aiff` | AIFF | Apple uncompressed |

### Future Formats (v2+)

| Format | Extensions | Notes |
|--------|------------|-------|
| WavPack | `.wv` | Hybrid lossless, popular among audiophiles |
| APE | `.ape` | Monkey's Audio |
| Musepack | `.mpc` | High quality lossy |
| DSD | `.dsf`, `.dff` | High-res, requires special handling |
| MQA | `.mqa.flac` | Controversial, may skip |

---

## Path Templates

### Default Template

```
{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}
```

**Example**:
```
Pink Floyd/1977 - Animals/01 - Pigs on the Wing (Part 1).flac
```

### Preset Templates

| Name | Template | Example |
|------|----------|---------|
| **Audiophile** (default) | `{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}` | `Pink Floyd/1977 - Animals/01 - Pigs on the Wing.flac` |
| **Simple** | `{AlbumArtist}/{Album}/{TrackNo} - {Title}` | `Pink Floyd/Animals/01 - Pigs on the Wing.flac` |
| **Genre-first** | `{Genre}/{AlbumArtist}/{Album}/{TrackNo} - {Title}` | `Rock/Pink Floyd/Animals/01 - Pigs on the Wing.flac` |
| **Custom** | User-defined | Power users can define their own |

### Available Placeholders

| Placeholder | Description | Fallback |
|-------------|-------------|----------|
| `{Artist}` | Track artist | "Unknown Artist" |
| `{AlbumArtist}` | Album artist | Falls back to `{Artist}` |
| `{Album}` | Album title | "Unknown Album" |
| `{Title}` | Track title | Filename without extension |
| `{TrackNo}` | Track number (zero-padded) | "00" |
| `{DiscNo}` | Disc number | "1" |
| `{Year}` | Release year | "0000" |
| `{Genre}` | Primary genre | "Unknown" |
| `{Composer}` | Composer | Empty string |

### Multi-Disc Handling

When `DiscTotal > 1`, automatically insert disc subfolder:

```
{AlbumArtist}/{Year} - {Album}/Disc {DiscNo}/{TrackNo} - {Title}
```

**Example**:
```
The Beatles/1968 - The Beatles (White Album)/Disc 1/01 - Back in the U.S.S.R..flac
The Beatles/1968 - The Beatles (White Album)/Disc 2/01 - Birthday.flac
```

### Compilation/VA Album Handling

When `AlbumArtist` is empty or "Various Artists":
- Use "Various Artists" as the artist folder
- Include track artist in filename: `{TrackNo} - {Artist} - {Title}`

---

## Multi-Platform Architecture

### Per-Device Sources Model

Each device maintains its own sources and library. This allows:
- Desktop users to have local libraries
- Server to have its own sources
- No complex sync requirements between devices

```
┌─────────────────────────────────────────────────────────────────┐
│  Device A (Desktop - Windows)                                   │
│  ├── device_id: "abc-123-..."                                   │
│  ├── SQLite DB: %APPDATA%\Soul Player\library.db                │
│  └── Sources:                                                   │
│      ├── Watched: D:\Music\FLAC                                 │
│      ├── Watched: E:\Vinyl Rips                                 │
│      └── Managed: D:\Music\Soul Player                          │
├─────────────────────────────────────────────────────────────────┤
│  Device B (Desktop - macOS)                                     │
│  ├── device_id: "def-456-..."                                   │
│  ├── SQLite DB: ~/Library/Application Support/Soul Player/...  │
│  └── Sources:                                                   │
│      ├── Watched: ~/Music/Lossless                              │
│      └── Managed: ~/Music/Soul Player                           │
├─────────────────────────────────────────────────────────────────┤
│  Device C (soul-server)                                         │
│  ├── device_id: "server-789-..."                                │
│  ├── SQLite/PostgreSQL DB                                       │
│  └── Sources:                                                   │
│      ├── Watched: /mnt/nas/music                                │
│      └── Watched: /mnt/nas/vinyl-rips                           │
└─────────────────────────────────────────────────────────────────┘
```

### Device ID Generation

- Generated on first app launch
- UUID v4 format
- Stored in app configuration (not database)
- Used to scope sources and settings

### Desktop Standalone Mode

#### First-Run / Empty Library Detection

The onboarding flow is triggered when:
1. **First app launch** (no config file exists)
2. **Library is empty** (0 tracks in database for current user)

When triggered, show a **dedicated startup layout** (not the main app layout).

#### Onboarding Flow - Step 1: Welcome & Strategy Selection

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                    [🌐 Language: English ▼] │
│                                                                             │
│                        Welcome to Soul Player                               │
│                                                                             │
│              How would you like to manage your music library?               │
│                                                                             │
│  ┌─────────────────────────────────┐  ┌─────────────────────────────────┐  │
│  │  📁 Use My Existing Folders     │  │  📥 Let Soul Player Organize    │  │
│  │                                 │  │                                 │  │
│  │  Point Soul Player to folders   │  │  Import files into an organized │  │
│  │  where you already have music.  │  │  library managed by Soul Player │  │
│  │                                 │  │                                 │  │
│  │  ✓ Non-destructive              │  │  ✓ Auto-organized by artist/    │  │
│  │  ✓ Keeps your folder structure  │  │    album                        │  │
│  │  ✓ Best for audiophiles with    │  │  ✓ Best for casual listeners    │  │
│  │    organized collections        │  │  ✓ Easy drag & drop import      │  │
│  │                                 │  │                                 │  │
│  │         [ Select ]              │  │         [ Select ]              │  │
│  └─────────────────────────────────┘  └─────────────────────────────────┘  │
│                                                                             │
│                    ☐ Use both (I'll set this up myself)                     │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────── │
│  💡 You can always change this later in Settings → Library                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key elements**:
- **Language selector** in top-right corner (dropdown with all supported languages)
- **Two clear cards** with visual icons explaining each approach
- **Checkbox** for hybrid mode (uses both strategies)
- **Reassurance** that settings can be changed later

#### Onboarding Flow - Step 2a: Watched Folders Setup

If user selected "Use My Existing Folders":

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Back                                            [🌐 Language: English ▼] │
│                                                                             │
│                    Add Your Music Folders                                   │
│                                                                             │
│  Soul Player will scan these folders and keep them in sync.                 │
│  Your files will never be moved or modified.                                │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  📁 D:\Music\FLAC                                          [Remove] │   │
│  │  📁 E:\Vinyl Rips                                          [Remove] │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│                        [ + Add Folder ]                                     │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────── │
│  Found: 1,234 audio files in 2 folders                                      │
│                                                                             │
│                                              [ Skip for Now ]  [ Continue ] │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Onboarding Flow - Step 2b: Managed Library Setup

If user selected "Let Soul Player Organize":

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Back                                            [🌐 Language: English ▼] │
│                                                                             │
│                  Choose Your Library Location                               │
│                                                                             │
│  Soul Player will organize your music in this folder.                       │
│  Files you import will be copied here with a clean structure.               │
│                                                                             │
│  Library folder:                                                            │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  📁 D:\Music\Soul Player                                  [Browse]  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Organization style:                                                        │
│  ○ Artist / Year - Album / Track - Title  (Recommended)                     │
│  ○ Artist / Album / Track - Title                                           │
│  ○ Genre / Artist / Album / Track - Title                                   │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────── │
│  Ready to import? Drag files here or click below.                           │
│                                                                             │
│                                              [ Skip for Now ]  [ Continue ] │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Onboarding Flow - Step 3: Initial Import (Optional)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Back                                            [🌐 Language: English ▼] │
│                                                                             │
│                     Import Your First Music                                 │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                                                                     │   │
│  │              🎵  Drag & drop files or folders here                  │   │
│  │                                                                     │   │
│  │                    or click to browse                               │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────── │
│  Queued for import: 45 files (1.2 GB)                                       │
│                                                                             │
│                                              [ Skip for Now ]  [ Import & ] │
│                                                                [ Continue ] │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Onboarding Flow - Step 4: Scanning / Import Progress

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                       Setting Up Your Library                               │
│                                                                             │
│                    ████████████████░░░░░░░░░  65%                           │
│                                                                             │
│                    Scanning: 823 / 1,267 files                              │
│                    Found: 412 albums, 1,823 tracks                          │
│                                                                             │
│                         Processing metadata...                              │
│                                                                             │
│  ───────────────────────────────────────────────────────────────────────── │
│  💡 This runs in the background. You can start using Soul Player now.      │
│                                                                             │
│                                                   [ Continue to App → ]     │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### Implementation Notes

```typescript
// Check if onboarding should be shown
async function shouldShowOnboarding(): Promise<boolean> {
  const configExists = await configFileExists();
  if (!configExists) return true;

  const trackCount = await getTrackCount(userId);
  return trackCount === 0;
}

// Onboarding state machine
type OnboardingStep =
  | 'welcome'           // Step 1: Strategy selection
  | 'watched-folders'   // Step 2a: Add watched folders
  | 'managed-library'   // Step 2b: Choose managed library location
  | 'initial-import'    // Step 3: Drag & drop import
  | 'scanning'          // Step 4: Progress
  | 'complete';         // Done, go to main app
```

### Desktop Connected to Server

When desktop connects to a soul-server:
- Can stream server's library
- Can still have local sources for offline playback
- Local and server libraries appear separately in UI

---

## Database Schema

### New Tables

```sql
-- Sources configuration (per user, per device)
CREATE TABLE sources (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('watched', 'managed')),
    path TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    sync_deletes BOOLEAN NOT NULL DEFAULT true,
    last_scan_at TIMESTAMP,
    scan_status TEXT CHECK(scan_status IN ('idle', 'scanning', 'error')),
    error_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, device_id, path)
);

CREATE INDEX idx_sources_user_device ON sources(user_id, device_id);

-- Managed library settings (one per user per device)
CREATE TABLE managed_library_settings (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    library_path TEXT NOT NULL,
    path_template TEXT NOT NULL DEFAULT '{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}',
    import_action TEXT NOT NULL DEFAULT 'copy' CHECK(import_action IN ('copy', 'move')),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, device_id)
);

-- Scan progress tracking
CREATE TABLE scan_progress (
    id INTEGER PRIMARY KEY,
    source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    total_files INTEGER,
    processed_files INTEGER DEFAULT 0,
    new_files INTEGER DEFAULT 0,
    updated_files INTEGER DEFAULT 0,
    removed_files INTEGER DEFAULT 0,
    errors INTEGER DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed', 'cancelled'))
);

-- External file handling settings (per user per device)
CREATE TABLE external_file_settings (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id),
    device_id TEXT NOT NULL,
    -- 'ask' | 'play' | 'import'
    default_action TEXT NOT NULL DEFAULT 'ask' CHECK(default_action IN ('ask', 'play', 'import')),
    -- 'managed' | 'watched' (if watched, use import_to_source_id)
    import_destination TEXT NOT NULL DEFAULT 'managed' CHECK(import_destination IN ('managed', 'watched')),
    import_to_source_id INTEGER REFERENCES sources(id),
    show_import_notification BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, device_id)
);
```

### Track Table Extensions

```sql
-- Extend existing tracks table
ALTER TABLE tracks ADD COLUMN source_id INTEGER REFERENCES sources(id);
ALTER TABLE tracks ADD COLUMN file_path TEXT NOT NULL;
ALTER TABLE tracks ADD COLUMN file_size INTEGER NOT NULL;
ALTER TABLE tracks ADD COLUMN file_mtime INTEGER NOT NULL;
ALTER TABLE tracks ADD COLUMN content_hash TEXT;
ALTER TABLE tracks ADD COLUMN audio_fingerprint TEXT;
ALTER TABLE tracks ADD COLUMN format TEXT NOT NULL;
ALTER TABLE tracks ADD COLUMN codec_details TEXT;  -- JSON with bitrate, sample_rate, etc.
ALTER TABLE tracks ADD COLUMN is_available BOOLEAN NOT NULL DEFAULT true;
ALTER TABLE tracks ADD COLUMN unavailable_since TIMESTAMP;

-- Indexes for efficient lookups
CREATE INDEX idx_tracks_source ON tracks(source_id);
CREATE INDEX idx_tracks_file_path ON tracks(file_path);
CREATE INDEX idx_tracks_content_hash ON tracks(content_hash) WHERE content_hash IS NOT NULL;
CREATE INDEX idx_tracks_fingerprint ON tracks(audio_fingerprint) WHERE audio_fingerprint IS NOT NULL;
CREATE INDEX idx_tracks_available ON tracks(is_available);
```

### Fingerprint Queue

```sql
-- Background fingerprinting queue
CREATE TABLE fingerprint_queue (
    id INTEGER PRIMARY KEY,
    track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(track_id)
);

CREATE INDEX idx_fingerprint_queue_priority ON fingerprint_queue(priority DESC, created_at ASC);
```

---

## Testing Requirements

> **Quality over quantity. No shallow tests.**

Each phase of implementation MUST include comprehensive testing. Tests should cover real behavior, edge cases, and error paths—not just check that code runs.

### Testing Philosophy

Following project guidelines from [CLAUDE.md](../../CLAUDE.md):

- ✅ **DO test**: Business logic, edge cases, error paths, integration between components
- ❌ **DON'T test**: Getters, setters, trivial constructors, obvious pass-throughs
- 🎯 **Target**: 50-60% meaningful coverage (not vanity metrics)

### Test Types by Layer

#### Unit Tests (Rust Libraries)

**soul-storage**:
```rust
#[cfg(test)]
mod tests {
    // Test CRUD operations with real SQLite (not mocks)
    // Test query edge cases (empty results, duplicates, constraints)
    // Test migration rollback/forward
}
```

**soul-core**:
```rust
#[cfg(test)]
mod tests {
    // Test path template parsing with malformed input
    // Test hash computation correctness
    // Test metadata extraction from various formats
    // Test filesystem watcher event handling
}
```

#### Integration Tests (Cross-Crate)

Use **testcontainers** or real SQLite databases:

```rust
// tests/integration/library_scan_test.rs
#[tokio::test]
async fn test_full_scan_workflow() {
    // 1. Create temp directory with test audio files
    // 2. Configure source pointing to temp dir
    // 3. Run full scan
    // 4. Verify tracks in database match files
    // 5. Modify a file, run incremental scan
    // 6. Verify only modified file was re-processed
}

#[tokio::test]
async fn test_file_relocation_detection() {
    // 1. Import file, record hash
    // 2. Move file to different location
    // 3. Run scan
    // 4. Verify path updated, no duplicate created
}
```

#### E2E Tests (Desktop Application)

Use **Tauri's testing framework** or **Playwright/WebDriver**:

```typescript
// tests/e2e/library-import.spec.ts
test('drag and drop shows import dialog', async ({ app }) => {
  // 1. Launch app
  // 2. Simulate drag & drop of audio file
  // 3. Verify dialog appears with correct options
  // 4. Click "Import"
  // 5. Verify track appears in library
  // 6. Verify file exists in managed library folder
});

test('file association opens app and plays', async ({ app }) => {
  // 1. Ensure app is not running
  // 2. Open audio file via OS (simulate double-click)
  // 3. Verify app launches
  // 4. Verify dialog or playback starts (based on settings)
});
```

### Test Requirements per Phase

| Phase | Unit Tests | Integration Tests | E2E Tests |
|-------|------------|-------------------|-----------|
| **1. Core Infrastructure** | CRUD operations, constraints, migrations | Source + track lifecycle | - |
| **2. File Scanning** | Metadata extraction, hash computation | Full scan workflow, incremental scan | - |
| **3. Filesystem Watcher** | Event debouncing, path normalization | Watcher + scanner integration | - |
| **4. Managed Library** | Template parsing, path generation | Import workflow, duplicate detection | - |
| **5. Desktop UI** | - | - | First-run wizard, settings, drag & drop |
| **6. Fingerprinting** | Chromaprint correctness | Queue processing, retry logic | - |
| **7. Server Integration** | API request/response | Full API workflow | - |
| **8. Discovery Service** | AcoustID client | Metadata enrichment | - |

### Test Data

Create a test fixtures directory with:

```
tests/fixtures/
├── audio/
│   ├── valid/
│   │   ├── test-track.flac      # 5 seconds of silence, valid tags
│   │   ├── test-track.mp3       # Same audio, MP3 format
│   │   ├── test-track-notags.flac  # No metadata
│   │   └── multi-disc/          # Album with 2 discs
│   ├── invalid/
│   │   ├── corrupted.flac       # Truncated file
│   │   ├── wrong-extension.mp3  # Actually a FLAC
│   │   └── not-audio.flac       # Text file with .flac extension
│   └── edge-cases/
│       ├── unicode-タイトル.flac  # Unicode in filename
│       ├── spaces in name.flac  # Spaces
│       └── very-long-title-....flac  # 255 char filename
├── databases/
│   └── test-library.db          # Pre-populated test database
└── configs/
    └── test-config.json         # Test configuration
```

### Continuous Integration

All tests run on every PR:

```yaml
# .github/workflows/test.yml
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - name: Unit Tests
        run: cargo test --all --lib

      - name: Integration Tests
        run: cargo test --all --test '*'

      - name: E2E Tests (Desktop)
        if: matrix.os != 'ubuntu-latest'  # Needs display
        run: yarn test:e2e
```

### Test Coverage Goals

| Component | Target Coverage | Notes |
|-----------|-----------------|-------|
| soul-storage (queries) | 70%+ | Critical data layer |
| soul-core (scanner) | 60%+ | Complex logic |
| soul-core (templates) | 80%+ | Many edge cases |
| Desktop UI components | 40%+ | E2E covers integration |
| Server API | 60%+ | Contract testing |

### What NOT to Test

Per project guidelines, skip tests for:

- Simple struct constructors
- Getter/setter methods
- Direct FFI wrappers (Symphonia calls)
- UI layout (unless behavior-dependent)
- Configuration file parsing (covered by integration tests)

---

## Implementation Roadmap

### Phase 1: Core Infrastructure (Foundation)

**Goal**: Database schema and basic source management

| Task | Library | Priority |
|------|---------|----------|
| Add sources table migration | soul-storage | P0 |
| Add managed_library_settings table | soul-storage | P0 |
| Extend tracks table with file tracking columns | soul-storage | P0 |
| Add fingerprint_queue table | soul-storage | P0 |
| Add external_file_settings table | soul-storage | P0 |
| Create Source CRUD operations | soul-storage | P0 |
| Create ManagedLibrarySettings CRUD | soul-storage | P0 |
| Add device_id generation utility | soul-core | P0 |

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| Unit | All CRUD operations, constraint violations, default values |
| Integration | Source → Track relationship, cascade deletes |

**Deliverable**: Database ready for library management

---

### Phase 2: File Scanning (Watched Folders)

**Goal**: Import files from watched folders

| Task | Library | Priority |
|------|---------|----------|
| Directory walker with format filtering | soul-core | P0 |
| Metadata reader (using Symphonia) | soul-core | P0 |
| mtime/size change detection | soul-core | P0 |
| Content hash computation (SHA256) | soul-core | P0 |
| Batch track insertion | soul-storage | P0 |
| Scan progress tracking | soul-storage | P1 |
| Soft delete for missing files | soul-storage | P0 |
| File relocation detection (hash match) | soul-storage | P1 |

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| Unit | Hash correctness, metadata extraction per format, mtime comparison |
| Integration | Full scan with real files, incremental scan, relocation detection |
| Fixtures | 10+ audio files covering all supported formats |

**Deliverable**: Can scan folders and import tracks

---

### Phase 3: Filesystem Watcher

**Goal**: Real-time updates while app is running

| Task | Library | Priority |
|------|---------|----------|
| Cross-platform filesystem watcher | soul-core | P0 |
| Event debouncing (avoid duplicate events) | soul-core | P0 |
| Handle create/modify/delete/move events | soul-core | P0 |
| Integration with scan engine | soul-core | P0 |

**Platform notes**:
- Windows: `ReadDirectoryChangesW`
- macOS: `FSEvents`
- Linux: `inotify`

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| Unit | Event debouncing logic, path normalization |
| Integration | Create file → event fired → track added (per platform) |

**Deliverable**: Library updates in real-time

---

### Phase 4: Managed Library Import

**Goal**: Copy/organize files to managed library

| Task | Library | Priority |
|------|---------|----------|
| Path template parser | soul-core | P0 |
| Placeholder resolver (metadata → path) | soul-core | P0 |
| Safe file copy with verification | soul-core | P0 |
| Duplicate detection (hash check) | soul-core | P0 |
| Import progress tracking | soul-core | P1 |
| Multi-disc folder handling | soul-core | P1 |
| Compilation album handling | soul-core | P1 |

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| Unit | Template parsing (all placeholders, edge cases, malformed input) |
| Unit | Path sanitization (invalid chars, reserved names, length limits) |
| Integration | Full import workflow, duplicate skip, multi-disc organization |

**Deliverable**: Can import and organize files

---

### Phase 5: Desktop UI Integration

**Goal**: User-facing source management and external file handling

| Task | Application | Priority |
|------|-------------|----------|
| First-run wizard (onboarding flow) | desktop | P0 |
| Sources settings page | shared | P0 |
| Add watched folder dialog | shared | P0 |
| Managed library settings dialog | shared | P0 |
| **External file settings section** | shared | P0 |
| Scan progress indicator (status bar) | shared | P0 |
| **Drag & drop detection (whole window)** | desktop | P0 |
| **"Import or Play" dialog** | shared | P0 |
| **TemporaryTrack support in player** | shared | P0 |
| Import dialog (file picker) | desktop | P1 |
| Unavailable tracks filter/badge | shared | P1 |
| Path template editor (custom) | shared | P2 |
| **"Remember my choice" persistence** | desktop | P1 |

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| E2E | First-run wizard flow (all paths) |
| E2E | Drag & drop → dialog → import → verify in library |
| E2E | Drag & drop → dialog → play → verify NOT in library |
| E2E | Settings changes persist across restart |

**Deliverable**: Full UI for library management and external file handling

---

### Phase 5.5: OS File Association

**Goal**: Handle double-click to open audio files with Soul Player

| Task | Application | Priority |
|------|-------------|----------|
| Register file associations (installer/setup) | desktop | P1 |
| Handle file open arguments on launch | desktop | P1 |
| Single-instance check (focus existing window) | desktop | P1 |
| Pass file to running instance via IPC | desktop | P1 |

**Platform notes**:
- Windows: Registry entries + Tauri deep linking
- macOS: Info.plist CFBundleDocumentTypes
- Linux: .desktop file with MimeType

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| E2E | Double-click .flac → app opens → dialog appears |
| E2E | Double-click while app running → file passed to existing instance |

**Deliverable**: Soul Player opens when user double-clicks audio files

---

### Phase 6: Background Fingerprinting

**Goal**: Chromaprint integration for discovery

| Task | Library | Priority |
|------|---------|----------|
| Chromaprint Rust bindings or pure Rust impl | soul-core | P1 |
| Background fingerprinting worker | soul-core | P1 |
| Queue management (priority, retries) | soul-storage | P1 |
| UI indicator for fingerprinting progress | shared | P2 |

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| Unit | Fingerprint generation correctness (compare to reference impl) |
| Integration | Queue processing, retry on failure, priority ordering |
| Performance | Fingerprint 1000 tracks in < 10 minutes (benchmark) |

**Deliverable**: All tracks have audio fingerprints

---

### Phase 7: Server Integration

**Goal**: soul-server uses same library system

| Task | Application | Priority |
|------|-------------|----------|
| Source management API endpoints | server | P1 |
| Scan trigger API | server | P1 |
| Scan status SSE/WebSocket | server | P2 |
| Admin UI for source management | server | P2 |

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| Unit | API request/response serialization |
| Integration | Full API workflow with testcontainers |
| E2E | Admin UI source management flow |

**Deliverable**: Server can manage sources via API

---

### Phase 8: Discovery Service Integration (Future)

**Goal**: Use fingerprints for metadata lookup

| Task | Library | Priority |
|------|---------|----------|
| AcoustID API client | soul-core | P2 |
| MusicBrainz metadata enrichment | soul-core | P2 |
| User-triggered "Identify track" feature | shared | P2 |
| Batch identification for untagged files | soul-core | P2 |

**Testing Requirements**:
| Test Type | Coverage |
|-----------|----------|
| Unit | API response parsing, error handling |
| Integration | Mock AcoustID server, verify metadata enrichment |
| E2E | "Identify track" button → metadata updated in UI |

**Deliverable**: Can identify unknown tracks via fingerprint

---

## UI Indicators

### Scan Progress

```
┌─────────────────────────────────────────────────────────────────┐
│  Status Bar                                                     │
├─────────────────────────────────────────────────────────────────┤
│  [🔄] Scanning: 1,234 / 5,678 files                             │
│  └── Click to see details or cancel                             │
├─────────────────────────────────────────────────────────────────┤
│  Details Panel (expanded)                                       │
├─────────────────────────────────────────────────────────────────┤
│  Scanning: D:\Music\FLAC                                        │
│  ├── New files: 45                                              │
│  ├── Updated: 12                                                │
│  ├── Removed: 3                                                 │
│  └── [Cancel Scan]                                              │
└─────────────────────────────────────────────────────────────────┘
```

### Background Fingerprinting

```
┌─────────────────────────────────────────────────────────────────┐
│  [🎵] Processing audio fingerprints (234 remaining)             │
│  └── Low priority, runs in background                           │
└─────────────────────────────────────────────────────────────────┘
```

### Unavailable Tracks

```
┌─────────────────────────────────────────────────────────────────┐
│  Track List                                                     │
├─────────────────────────────────────────────────────────────────┤
│  1. Track One                                     3:45          │
│  2. Track Two                                     4:12          │
│  3. [⚠️] Track Three (unavailable)                 3:33          │
│     └── File not found. Last seen: 2 days ago                   │
│  4. Track Four                                    5:01          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Settings UI Integration

All library management settings appear in **Settings → Library**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Settings                                                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐                                                           │
│  │ General      │   LIBRARY SOURCES                                         │
│  │ Appearance   │   ─────────────────────────────────────────────────────── │
│  │ Audio        │                                                           │
│  │ ▶ Library    │   Watched Folders                                         │
│  │ Playback     │   Soul Player monitors these folders for music files.     │
│  │ Shortcuts    │                                                           │
│  └──────────────┘   ┌───────────────────────────────────────────────────┐   │
│                     │ 📁 D:\Music\FLAC                      [✓] [Remove]│   │
│                     │ 📁 E:\Vinyl Rips                      [✓] [Remove]│   │
│                     └───────────────────────────────────────────────────┘   │
│                                                     [ + Add Folder ]        │
│                                                                             │
│                     [ Rescan All ]  Last scan: 5 minutes ago                │
│                                                                             │
│                     ─────────────────────────────────────────────────────── │
│                                                                             │
│                     MANAGED LIBRARY                                         │
│                                                                             │
│                     Location:                                               │
│                     ┌───────────────────────────────────────────────────┐   │
│                     │ D:\Music\Soul Player                    [Browse] │   │
│                     └───────────────────────────────────────────────────┘   │
│                     ⚠️ Changing location requires re-importing files        │
│                                                                             │
│                     Organization template:                                  │
│                     ┌───────────────────────────────────────────────────┐   │
│                     │ {AlbumArtist}/{Year} - {Album}/...         [▼]   │   │
│                     └───────────────────────────────────────────────────┘   │
│                     Preview: Pink Floyd/1977 - Animals/01 - Pigs.flac      │
│                                                                             │
│                     ─────────────────────────────────────────────────────── │
│                                                                             │
│                     EXTERNAL FILES                                          │
│                     When opening files not in your library:                 │
│                                                                             │
│                     ○ Always ask what to do                                 │
│                     ○ Always play without importing                         │
│                     ○ Always import to library                              │
│                                                                             │
│                     Default import destination:                             │
│                     ○ Managed library (recommended)                         │
│                     ○ Add to watched folder: [Select folder...]             │
│                                                                             │
│                     ☑ Show notification after import                        │
│                                                                             │
│                     ─────────────────────────────────────────────────────── │
│                                                                             │
│                     LIBRARY MAINTENANCE                                     │
│                                                                             │
│                     Unavailable tracks: 3 tracks                            │
│                     [ Find Missing Files ]  [ Remove Unavailable ]          │
│                                                                             │
│                     [ Clear Library and Start Over ]                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Settings Sections Breakdown

| Section | Purpose |
|---------|---------|
| **Watched Folders** | Add/remove/enable folders, trigger rescan |
| **Managed Library** | Set location, choose organization template |
| **External Files** | Configure drag & drop / file open behavior |
| **Library Maintenance** | Handle unavailable tracks, reset library |

### Localization Keys Required

All strings in library settings must be localized:

```json
{
  "settings.library.title": "Library",
  "settings.library.sources.title": "Library Sources",
  "settings.library.sources.watched.title": "Watched Folders",
  "settings.library.sources.watched.description": "Soul Player monitors these folders for music files.",
  "settings.library.sources.watched.add": "Add Folder",
  "settings.library.sources.watched.remove": "Remove",
  "settings.library.sources.watched.rescan": "Rescan All",
  "settings.library.sources.watched.lastScan": "Last scan: {{time}}",
  "settings.library.managed.title": "Managed Library",
  "settings.library.managed.location": "Location",
  "settings.library.managed.locationWarning": "Changing location requires re-importing files",
  "settings.library.managed.template": "Organization template",
  "settings.library.managed.preview": "Preview: {{path}}",
  "settings.library.external.title": "External Files",
  "settings.library.external.description": "When opening files not in your library:",
  "settings.library.external.ask": "Always ask what to do",
  "settings.library.external.play": "Always play without importing",
  "settings.library.external.import": "Always import to library",
  "settings.library.external.destination": "Default import destination",
  "settings.library.external.destinationManaged": "Managed library (recommended)",
  "settings.library.external.destinationWatched": "Add to watched folder",
  "settings.library.external.notification": "Show notification after import",
  "settings.library.maintenance.title": "Library Maintenance",
  "settings.library.maintenance.unavailable": "Unavailable tracks: {{count}} tracks",
  "settings.library.maintenance.findMissing": "Find Missing Files",
  "settings.library.maintenance.removeUnavailable": "Remove Unavailable",
  "settings.library.maintenance.reset": "Clear Library and Start Over"
}
```

---

## Research Sources

This design was informed by research into how major music players handle library management:

### Watched Folder Approach
- [foobar2000 Media Library](https://wiki.hydrogenaudio.org/index.php?title=Foobar2000:Preferences:Media_Library) - Real-time folder monitoring
- [Roon Music Organization](https://roon.app/en/music/organization) - Never copies/moves files
- [Navidrome FAQ](https://www.navidrome.org/docs/faq/) - Server-side scanning, never writes to music folder
- [Plex Library Scanning](https://support.plex.tv/articles/200289306-scanning-vs-refreshing-a-library/) - Folder watching + scheduled scans

### Organize/Copy/Move Approach
- [JRiver Rename, Move, and Copy](https://wiki.jriver.com/index.php/Rename,_Move,_and_Copy_Files) - Template-based file organization
- [MusicBee Library Preferences](https://musicbee.fandom.com/wiki/Library_Preferences) - Auto-organize feature

### Path Templates
- [beets Path Formats](https://beets.readthedocs.io/en/stable/reference/pathformat.html) - `$albumartist/$album/$track $title`
- [dBpoweramp Naming](https://www.dbpoweramp.com/help/dmc/Naming) - Template syntax

### Audio Fingerprinting
- [Chromaprint](https://acoustid.org/chromaprint) - Audio fingerprint library
- [AcoustID](https://acoustid.org/) - Free fingerprint database with MusicBrainz integration

### Community Preferences
- [Audiophile Style Forums](https://audiophilestyle.com/forums/topic/34704-how-do-you-organize-and-manage-your-files-and-library/) - Folder organization discussions
- [AVS Forum Music Naming](https://www.avsforum.com/threads/music-files-directory-structure-and-naming-conventions-long.367503/) - Naming conventions
- [Hydrogenaudio Directory Structure](https://hydrogenaudio.org/index.php/topic,32726.0.html) - Audiophile preferences

---

## Appendix: Configuration Examples

### Desktop Config (JSON)

```json
{
  "device_id": "abc123-def456-...",
  "library": {
    "managed_library_path": "D:\\Music\\Soul Player",
    "path_template": "{AlbumArtist}/{Year} - {Album}/{TrackNo} - {Title}",
    "import_action": "copy"
  },
  "sources": [
    {
      "name": "FLAC Collection",
      "type": "watched",
      "path": "D:\\Music\\FLAC",
      "enabled": true
    },
    {
      "name": "Vinyl Rips",
      "type": "watched",
      "path": "E:\\Vinyl Rips",
      "enabled": true
    }
  ]
}
```

### Server Config (TOML)

```toml
[library]
device_id = "server-789-..."

[[sources]]
name = "NAS Music"
type = "watched"
path = "/mnt/nas/music"
enabled = true

[[sources]]
name = "Uploads"
type = "managed"
path = "/var/lib/soul-player/music"
enabled = true
```

---

## Open Questions

1. **Should we support network paths for watched folders?**
   - UNC paths on Windows, NFS/SMB mounts on Linux/macOS
   - Reliability concerns with network disconnections

2. **How to handle very large libraries (500k+ tracks)?**
   - Consider SQLite write-ahead logging
   - Potential PostgreSQL migration path for server

3. **Should we support multiple managed libraries per device?**
   - Use case: separate library for DSD vs PCM
   - Adds complexity to UI

4. **Conflict resolution for managed library imports?**
   - Same album from different sources with slightly different metadata
   - Currently: skip if hash matches, otherwise allow both
