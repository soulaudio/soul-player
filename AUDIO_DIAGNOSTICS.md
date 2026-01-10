# Audio Playback Speed Diagnostics

## Current Status
You're experiencing audio playing too fast. I've added comprehensive diagnostics to identify the issue.

## Run the App and Collect Diagnostic Output

1. **Run your desktop app:**
   ```powershell
   yarn dev:desktop
   ```

2. **Play a track** and watch the console output

3. **Copy and share** the diagnostic lines that start with:
   - `[CPAL] Device default config:`
   - `[LocalAudioSource] File info:`
   - `[process_audio] Call #`

## What the Diagnostics Show

### Expected Output Example (Correct):
```
[CPAL] Device default config:
  - Sample rate: 48000
  - Channels: 2
  - Buffer size: Unknown

[LocalAudioSource] File info:
  - Path: C:\Music\song.mp3
  - Source sample rate: 44100 Hz
  - Target sample rate: 48000 Hz
  - Channels: 2
  - Needs resampling: true
  - Speed ratio: 0.9188x  ← File slower than device (will be sped up to match)

[process_audio] Call #1
  - Output buffer size: 960 samples
  - Output channels: 2
  - Expected frames: 480
  - Sample rate: 48000 Hz
```

### Problem Indicators:

#### 1. **Sample Rate Mismatch (Most Common)**
```
Source sample rate: 48000 Hz
Target sample rate: 44100 Hz
Speed ratio: 1.0884x  ← 8.8% too fast!
```
**Cause**: File is 48kHz but device is 44.1kHz
**Solution**: Resampling should fix this automatically

#### 2. **Channel Mismatch**
```
Output channels: 1  ← Device is MONO
```
But file is stereo (2 channels)
**Cause**: Stereo audio on mono device = 2x speed if not converted
**Solution**: Automatic stereo→mono conversion should handle this

#### 3. **Wrong Buffer Calculation**
```
Output buffer size: 960 samples
Output channels: 2
Expected frames: 480  ← Should match buffer_size / channels
```
If this doesn't divide evenly, there's a problem.

## Common Issues by Speed

| Speed | Likely Cause | What to Look For |
|-------|--------------|------------------|
| **2x too fast** | Stereo treated as mono | `Output channels: 1` with stereo file |
| **~9% too fast** | 48kHz → 44.1kHz | `Speed ratio: 1.0884` |
| **~9% too slow** | 44.1kHz → 48kHz | `Speed ratio: 0.9188` |
| **1.5x too fast** | 48kHz → 32kHz | `Speed ratio: 1.5000` |

## What I Need from You

**Please run the app, play a track, and send me:**

1. The complete console output showing all three diagnostic sections
2. Approximately how much faster it's playing (2x? 1.5x? 10%?)
3. The audio file you're testing with (format and any info you have)

This will tell me exactly what's wrong!

## Quick Test

Try playing files with KNOWN durations:
- A 3:00 minute song
- Note how long it actually plays

If a 3:00 song finishes in:
- **1:30** → 2x too fast (channel issue)
- **2:45** → ~9% too fast (sample rate 48→44.1)
- **3:16** → ~9% too slow (sample rate 44.1→48)

---

**After you share the diagnostic output, I'll identify and fix the exact issue.**
