# Demo Audio Files

Place your royalty-free music files here.

## Quick Setup

1. **Add your MP3/OGG files** to this directory:
   - `ambient-morning.mp3`
   - `electronic-dreams.mp3`
   - etc.

2. **Add album covers** to `covers/` subdirectory:
   - `peaceful-sounds.jpg`
   - `coffee-house.jpg`
   - etc.

3. **Update `../demo-data.json`** with your track info

## File Requirements

- **Audio formats**: MP3 (recommended), OGG, or WAV
- **Bitrate**: 128-192 kbps for demos
- **Duration**: 2-3 minutes ideal for demos
- **Covers**: 500x500px JPG/PNG (< 100 KB each)

## Finding Free Music

See `../DEMO_CONFIGURATION.md` for sources of royalty-free music.

## Example File Structure

```
demo-audio/
├── README.md (this file)
├── ambient-morning.mp3
├── electronic-dreams.mp3
├── jazz-cafe.mp3
├── acoustic-guitar.mp3
└── covers/
    ├── peaceful-sounds.jpg
    └── coffee-house.jpg
```

## Testing

1. Start the dev server: `npm run dev`
2. Navigate to the demo page
3. Click a track to test playback

## Troubleshooting

If audio doesn't play:
1. Check browser console for errors
2. Verify file paths in `demo-data.json`
3. Test files play in browser directly
4. Ensure files are in correct format
