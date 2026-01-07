# Placeholder Audio Files

The demo is configured but needs actual audio files to play.

## Quick Test Without Audio

To test the UI interactions without audio files:
1. Click tracks - they'll try to load (but fail gracefully)
2. Use playback controls - they work but no sound
3. Test shuffle, repeat, volume - all functional

## Add Real Audio

Replace the paths in `../demo-data.json` with your actual audio files:

```json
{
  "tracks": [
    {
      "id": "1",
      "path": "/demo-audio/your-actual-file.mp3",
      ...
    }
  ]
}
```

## For Testing

You can use a silent audio file or any test MP3 from:
- https://file-examples.com/index.php/sample-audio-files/
- https://www.kozco.com/tech/soundtests.html

Or generate a test tone:
```bash
# Generate 30-second test tone (440 Hz)
ffmpeg -f lavfi -i "sine=frequency=440:duration=30" test-tone.mp3
```
