# Demo Configuration Guide

The interactive demo on the marketing site uses **real playback functionality** with configurable demo data.

## Quick Start

### 1. Add Your Music Files

Place your royalty-free MP3/OGG files in the public directory:

```
applications/marketing/public/
├── demo-data.json          # Configuration file
└── demo-audio/             # Your audio files
    ├── ambient-morning.mp3
    ├── electronic-dreams.mp3
    ├── jazz-cafe.mp3
    └── covers/             # Album cover images
        ├── peaceful-sounds.jpg
        └── coffee-house.jpg
```

### 2. Edit demo-data.json

Update `public/demo-data.json` with your track information:

```json
{
  "tracks": [
    {
      "id": "1",
      "title": "Your Track Title",
      "artist": "Artist Name",
      "album": "Album Name",
      "duration": 180,
      "trackNumber": 1,
      "path": "/demo-audio/your-track.mp3",
      "coverUrl": "/demo-audio/covers/your-cover.jpg"
    }
  ],
  "albums": [
    {
      "id": "1",
      "title": "Your Album",
      "artist": "Artist Name",
      "year": 2024,
      "trackIds": ["1", "2"],
      "coverUrl": "/demo-audio/covers/your-album.jpg"
    }
  ]
}
```

### 3. That's It!

No code changes needed. The demo automatically loads your configured data.

---

## Finding Royalty-Free Music

### Recommended Sources

1. **Free Music Archive** (freemusicarchive.org)
   - Filter by CC0 or CC BY licenses
   - High-quality curated music
   - Clear licensing information

2. **Incompetech** (incompetech.com)
   - Created by Kevin MacLeod
   - CC BY 3.0 license (requires attribution)
   - Searchable by mood/genre

3. **YouTube Audio Library** (youtube.com/audiolibrary)
   - Completely free tracks
   - No attribution required for many tracks
   - Download as MP3

4. **ccMixter** (ccmixter.org)
   - Creative Commons remixes
   - Various CC licenses
   - Good for electronic/hip-hop

5. **Bensound** (bensound.com)
   - Free with attribution
   - Commercial use allowed
   - Professional quality

### License Requirements

For demo purposes, look for:
- **CC0** (Public Domain) - No attribution needed
- **CC BY** (Attribution) - Just add credit in your footer
- Avoid **CC BY-NC** (Non-Commercial) if demo will be on commercial site

---

## Audio Format Recommendations

### Formats
- **MP3**: Best browser compatibility (use 128-192 kbps for demos)
- **OGG**: Smaller file size, good quality
- **AAC**: Good quality, but MP3 more compatible

### Optimization
```bash
# Convert to optimized MP3 (192kbps)
ffmpeg -i input.wav -codec:a libmp3lame -b:a 192k output.mp3

# Convert to OGG (quality 5)
ffmpeg -i input.wav -codec:a libvorbis -q:a 5 output.ogg
```

### File Size Guidelines
- **Demo tracks**: 2-3 minutes (3-5 MB per track at 192kbps)
- **Album covers**: 500x500px JPG (< 100 KB each)
- **Total demo size**: Keep under 50 MB for fast loading

---

## Advanced Configuration

### Custom Track Metadata

All fields in demo-data.json:

```json
{
  "id": "unique-id",           // Required: Unique identifier
  "title": "Track Title",      // Required: Display name
  "artist": "Artist Name",     // Required: Artist name
  "album": "Album Name",       // Optional: Album name
  "duration": 180,             // Required: Length in seconds
  "trackNumber": 1,            // Optional: Track number in album
  "path": "/demo-audio/...",   // Required: Path to audio file
  "coverUrl": "/demo-audio/..." // Optional: Album art URL
}
```

### Album Configuration

```json
{
  "id": "album-id",           // Required: Unique identifier
  "title": "Album Title",     // Required: Display name
  "artist": "Artist Name",    // Required: Artist name
  "year": 2024,              // Required: Release year
  "trackIds": ["1", "2"],    // Required: Array of track IDs
  "coverUrl": "/path/to/cover.jpg" // Optional: Album art
}
```

### Multiple Artists/Albums

You can have as many tracks and albums as you want:

```json
{
  "tracks": [
    // ... up to 50 tracks for good demo performance
  ],
  "albums": [
    // ... up to 20 albums
  ]
}
```

---

## Features Included

The demo supports **ALL** desktop app features:

✅ **Playback Controls**
- Play/Pause/Stop
- Next/Previous track
- Seek (click progress bar)
- Volume control with mute

✅ **Queue Management**
- Play individual tracks
- Play full albums
- Queue tracks
- Two-tier queue (explicit + source)

✅ **Shuffle & Repeat**
- Shuffle: Off / Random / Smart
- Repeat: Off / All / One
- State persists during session

✅ **History**
- Track up to 50 previously played tracks
- Previous button behavior (restart vs. go back)

✅ **Visual Feedback**
- Currently playing indicator
- Hover effects
- Real-time position updates
- Responsive controls

---

## Architecture

The demo uses a **pure TypeScript implementation** that mirrors the Rust desktop app:

- **Storage**: JSON-based (replaces SQLite)
- **Audio**: Web Audio API (replaces CPAL + Symphonia)
- **Playback Logic**: Direct TypeScript port of soul-playback
- **Bundle Size**: ~50-80 KB JS (no WASM overhead)

### File Structure

```
src/
├── lib/demo/
│   ├── types.ts              # Type definitions (mirrors Rust)
│   ├── storage.ts            # JSON data loader
│   ├── audio-player.ts       # Web Audio wrapper
│   └── playback-manager.ts   # Queue/shuffle/repeat logic
├── hooks/
│   └── usePlayback.ts        # React integration
└── components/demo/
    ├── DemoApp.tsx           # Entry point
    ├── PlaybackControls.tsx  # Control bar
    └── LibraryPage.tsx       # Track/album browser
```

---

## Troubleshooting

### Audio Not Playing

1. **Check browser console** for errors
2. **Verify file paths** in demo-data.json match actual files
3. **Test audio files** can play directly in browser
4. **Check CORS** if files hosted separately

### Demo Not Loading

1. **Verify demo-data.json** is valid JSON (use jsonlint.com)
2. **Check file is in** `public/` directory
3. **Clear browser cache** and refresh

### Incorrect Durations

The `duration` field must be in **seconds** (not milliseconds):
- ✅ `"duration": 180` (3 minutes)
- ❌ `"duration": 180000` (wrong - 50 hours!)

---

## Example Attribution

If using CC BY music, add to your site footer:

```html
<!-- Demo music -->
<p class="text-sm text-muted-foreground">
  Demo music:
  <a href="https://incompetech.com">Kevin MacLeod (incompetech.com)</a>
  Licensed under CC BY 4.0
</p>
```

---

## Questions?

See the main Soul Player documentation or open an issue on GitHub.
