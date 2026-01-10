# Audient EVO8 ASIO Fix

## Problem
Audio plays too fast on Audient EVO8 when using WASAPI backend, even though resampling is configured correctly (44.1kHz → 96kHz).

## Root Cause
According to [Audient's official support documentation](https://support.audient.com/hc/en-us/articles/202335209-Optimising-Windows-Computers-For-Audio), **Audient highly recommends using ASIO drivers instead of WASAPI** for their audio interfaces.

WASAPI has known compatibility issues with Audient interfaces that can cause:
- Incorrect playback speed
- Sample rate mismatches
- Audio distortion

## Solution
Use the **ASIO backend** instead of WASAPI (default).

### Steps to Fix

1. **Rebuild the app** (ASIO support is now enabled):
   ```bash
   cd applications/desktop/src-tauri
   SQLX_OFFLINE=true cargo build
   ```

2. **Launch the app** and open Settings → Audio

3. **Switch to ASIO backend**:
   - In the "Audio Driver" section
   - Select "ASIO" from the backend dropdown
   - The device list will refresh automatically

4. **Select your Audient device**:
   - Choose one of:
     - `Main Output 1/2 (Audient EVO8)` - Main outputs (recommended)
     - `Line 3/4 (Audient EVO8)` - Alternate line outputs
     - `Loop-back 1/2 (Audient EVO8)` - Loopback for recording

5. **Test playback**:
   - Play a 44.1kHz track
   - Audio should now play at correct speed
   - Check console for: `[audio_settings] Switched to ASIO backend`

## Technical Details

### What Changed
- **Before**: WASAPI (default) → Incompatible with EVO8 → Wrong playback speed
- **After**: ASIO → Direct driver communication → Correct playback speed

### Why ASIO Works Better
- **Direct driver access**: ASIO communicates directly with the audio driver
- **Exclusive mode**: No Windows audio engine interference
- **Lower latency**: Bypasses Windows audio stack
- **Professional audio**: Industry standard for audio interfaces

### Resampling Still Works
The resampler still works correctly with ASIO:
```
Source: 44100 Hz (MP3 file)
↓ Rubato Sinc Resampler (0.4594x ratio)
Target: 96000 Hz (EVO8 device)
```

## Expected Console Output

```
[audio_settings] Setting audio device: backend=asio, device=Main Output 1/2 (Audient EVO8)
[audio_settings] Found device: Main Output 1/2 (Audient EVO8)
[PlaybackCommand::Play] Target sample rate: 96000
[LocalAudioSource] Source sample rate: 44100 Hz
[LocalAudioSource] Target sample rate: 96000 Hz
[LocalAudioSource] Needs resampling: true
[LocalAudioSource] Speed ratio: 0.4594x
```

## Additional Notes

### EVO Control App
For best results, set your preferred sample rate in the **EVO Control App** (Audient's official software):
- 44.1 kHz - CD quality
- 48 kHz - Video standard
- 96 kHz - High-resolution (recommended for EVO8)

### Windows Sample Rate
After setting the rate in EVO Control, verify in Windows:
1. Right-click speaker icon → **Sounds**
2. **Playback** tab → **Audient EVO8** → **Properties**
3. **Advanced** tab → Should show matching sample rate

### Fallback to WASAPI
If ASIO doesn't work:
- Check that EVO drivers are up to date
- Ensure no other app is using ASIO exclusively
- Restart the app to release the driver
- Switch back to "default" (WASAPI) if needed

## References
- [Audient - Optimising Windows Computers For Audio](https://support.audient.com/hc/en-us/articles/202335209-Optimising-Windows-Computers-For-Audio)
- [EVO Drivers - Change Log](https://support.audient.com/hc/en-us/articles/8317993699604-EVO-Drivers-Change-Log)

---

**Status**: ✅ ASIO support enabled (Windows only)
**Date**: 2026-01-10
