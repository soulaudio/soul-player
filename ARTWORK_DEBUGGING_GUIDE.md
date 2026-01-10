# Artwork Display Debugging Guide

This guide will help you debug why album artwork isn't displaying in the desktop app.

## Step 1: Full Rebuild (CRITICAL!)

Hot-reload **does not** apply Rust changes or Tauri config changes. You MUST do a full rebuild:

```bash
# Stop the current dev server (Ctrl+C)
cd applications/desktop

# Clean previous builds (optional but recommended)
rm -rf dist
cd src-tauri && cargo clean && cd ..

# Full rebuild and run
yarn dev
```

**Important**: Wait for the Rust compilation to complete before the app launches. This can take 1-2 minutes.

## Step 2: Check Terminal Logs

Once the app is running and you play a track, you should see these logs in your **terminal** (not browser console):

```
[playback] Track changed: id=123, title=Song Name, coverArtPath=Some("artwork://track/123")
```

**If you DON'T see this log:**
- The FrontendTrackEvent changes haven't been compiled
- Do a `cargo clean` and rebuild

## Step 3: Check Browser Console

Open DevTools in the app (Right-click → Inspect or F12) and check the Console tab.

### When a track plays, you should see:

```
[TauriPlayerCommandsProvider] Track changed: {id: "123", title: "Song Name", ...}
[TauriPlayerCommandsProvider] coverArtPath: artwork://track/123
[TrackInfo] Current track: {id: "123", ...}
[TrackInfo] coverArtPath: artwork://track/123
```

**If `coverArtPath` is undefined or null:**
- The playback.rs changes haven't been applied
- Rebuild the Rust code

### When the image loads, you should see:

```
[TrackInfo] Image loaded successfully: artwork://track/123
```

**If you see "Image failed to load" instead:**
- Check the next section for artwork protocol logs

## Step 4: Check Artwork Protocol Logs

When an image tries to load, you should see these logs in your **terminal**:

```
[artwork protocol] Request: artwork://track/123
[artwork] Handling request: artwork://track/123
[artwork] Path after prefix: track/123
[artwork] Entity type: track, ID string: 123
[artwork] Parsed ID: 123
[artwork] Fetching artwork for track 123
[artwork] Track artwork result: found
[artwork] SUCCESS: Returning 45678 bytes of image/jpeg
```

**If you DON'T see these logs:**
- The CSP is blocking the protocol, OR
- The protocol handler isn't being called
- Check the browser console for CSP errors

**If you see "not found":**
- Your audio files don't have embedded artwork
- Test with the diagnostic command (see below)

## Step 5: Test with Diagnostic Command

Open the browser DevTools console (F12) and run:

```javascript
// Replace 1 with an actual track ID from your library
await window.__TAURI__.core.invoke('test_artwork_extraction', { trackId: 1 })
```

This will show detailed information about whether artwork can be extracted from that track.

**Example success output:**
```
"SUCCESS: Found artwork for 'Song Title'
File: /path/to/file.mp3
Size: 123456 bytes
Type: image/jpeg"
```

**Example failure output:**
```
"No artwork found in file: /path/to/file.mp3
The file may not have embedded artwork."
```

## Common Issues

### 1. CSP Blocking the Protocol

**Symptom**: Browser console shows CSP errors like:
```
Refused to load the image 'artwork://track/123' because it violates the following Content Security Policy directive: "img-src..."
```

**Fix**: Check that `tauri.conf.json` has `artwork:` in the CSP:
```json
"img-src 'self' data: https: artwork:;"
```

Then do a full rebuild.

### 2. Files Don't Have Embedded Artwork

**Symptom**: Diagnostic command returns "No artwork found"

**Fix**: Your audio files need embedded album art. You can:
1. Use a tool like Mp3tag, MusicBrainz Picard, or iTunes to embed artwork
2. Or download files that already have embedded artwork

### 3. Hot-Reload Not Applying Changes

**Symptom**: Logs show old behavior even after saving files

**Fix**: Hot-reload doesn't work for Rust changes. Always do a full rebuild:
```bash
cd applications/desktop
yarn dev
```

### 4. Track ID Issues

**Symptom**: Artwork requests fail with "Invalid ID" error

**Fix**: Check that track IDs in the database are valid integers. Run:
```javascript
await window.__TAURI__.core.invoke('get_all_tracks')
```

And check the `id` field of the first track.

## Verify File Has Artwork

To check if your audio file has embedded artwork (outside the app):

### Using ffprobe (part of ffmpeg):
```bash
ffprobe -v quiet -select_streams v:0 -show_entries stream=codec_name -of default=noprint_wrappers=1:nokey=1 /path/to/your/file.mp3
```

If it outputs `mjpeg`, `png`, or another image codec, the file has artwork.

### Using exiftool:
```bash
exiftool /path/to/your/file.mp3 | grep -i picture
```

Should show picture information if artwork exists.

## Still Not Working?

If you've followed all steps and images still aren't loading:

1. Check the **exact logs** you're seeing in both terminal and browser console
2. Run the diagnostic command and share the output
3. Verify your audio files actually have embedded artwork
4. Make sure you did a **full rebuild** (not just hot-reload)
5. Try opening DevTools **before** playing a track to catch all logs

## Quick Checklist

- [ ] Did a full rebuild with `yarn dev` (not just hot-reload)
- [ ] Terminal shows `[playback] Track changed:` with `coverArtPath`
- [ ] Browser console shows `[TrackInfo] coverArtPath: artwork://track/...`
- [ ] Terminal shows `[artwork protocol] Request:` when image loads
- [ ] Diagnostic command shows artwork can be extracted from test file
- [ ] Audio files have embedded artwork (verified with ffprobe or exiftool)

If all checkboxes are checked and it still doesn't work, there may be a deeper issue with the Tauri custom protocol system.
