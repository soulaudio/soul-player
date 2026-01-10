# Debugging Sample Rate Issue

## User Report
"on 48000 Hz it still works correctly on my audient evo8 it still plays much faster"

## Diagnosis Steps

### 1. Check What Sample Rate Is Detected

When you start the app, check the console output for these lines:
```
[CPAL] Device default config:
  - Sample rate: XXXXX
  - Channels: X
```

**Expected**: Should show `96000` if your EVO8 is set to 96kHz in Windows settings
**Problem**: If it shows `48000` even though EVO8 is at 96kHz, that's the issue

### 2. Enable Debug Logging

Add this to verify sample rate at key points:

When playing a track, you should see:
```
[PlaybackCommand::Play] Target sample rate: XXXXX
[LocalAudioSource] Source sample rate: 44100 Hz
[LocalAudioSource] Target sample rate: XXXXX Hz
[LocalAudioSource] Needs resampling: true/false
```

### 3. Check Windows Audio Settings

1. Right-click speaker icon → **Sounds**
2. **Playback** tab → Select **Audient EVO8** → **Properties**
3. **Advanced** tab → Check **Default Format**
4. Should show "2 channel, 24 bit, 96000 Hz (Studio Quality)"

### 4. Verify WASAPI Detection

The sample rate comes from CPAL/WASAPI:
```rust
let config = device.default_output_config()?;
let sample_rate = config.sample_rate().0;  // This reads from Windows
```

**Possible Issue**: Windows might be reporting 48kHz even though device is at 96kHz

### 5. Test Device Switch

Try manually switching:
1. Start app
2. Play a 44.1kHz MP3
3. Open Settings → Audio → Device
4. Switch to Audient EVO8 (if not already)
5. Check console for:
```
[DesktopPlayback] Switching device
[DesktopPlayback] Reloading audio source for new sample rate
[LocalAudioSource] Target sample rate: 96000 Hz
```

## Possible Causes

### A. Windows Reports Wrong Sample Rate
**Symptom**: App shows 48kHz but device is actually 96kHz
**Fix**: Force sample rate detection from device properties
**Workaround**: Set Windows default format to 96kHz explicitly

### B. Exclusive Mode Issue
**Symptom**: Device is in shared mode at 48kHz
**Fix**: Enable exclusive mode in Windows
**Path**: Device Properties → Advanced → Check "Allow applications to take exclusive control"

### C. Sample Rate Not Updated on Device Change
**Symptom**: First device was 48kHz, switched to 96kHz device but sample rate not updated
**Fix**: Already implemented in `switch_device()` - should reload audio source

### D. Audio Source Not Using Target Sample Rate
**Symptom**: LocalAudioSource created with wrong target
**Check**: Console should show `Needs resampling: true` when rates differ

## Expected Console Output (Working)

```
[CPAL] Device default config:
  - Sample rate: 96000        ← Device is 96kHz
  - Channels: 2

[PlaybackCommand::Play] Playing track: song.mp3
[PlaybackCommand::Play] Target sample rate: 96000
[LocalAudioSource] File info:
  - Path: song.mp3
  - Source sample rate: 44100 Hz
  - Target sample rate: 96000 Hz     ← Matches device
  - Needs resampling: true           ← Resampling enabled
  - Speed ratio: 0.4594x
```

## Broken Console Output (Bug)

```
[CPAL] Device default config:
  - Sample rate: 48000        ← Wrong! Device is 96kHz but reports 48kHz
  - Channels: 2

[PlaybackCommand::Play] Target sample rate: 48000
[LocalAudioSource] Source sample rate: 44100 Hz
[LocalAudioSource] Target sample rate: 48000 Hz   ← Wrong target
[LocalAudioSource] Needs resampling: true
Result: 44.1kHz audio → 48kHz stream → 96kHz device = 2x fast
```

## Quick Test

Run this in PowerShell to check device sample rate:
```powershell
Get-AudioDevice -List | Where-Object {$_.Name -like "*Audient*"} | Select-Object Name, Format
```

Or use Windows Audio MIDI Setup to verify current sample rate.

## Next Steps

1. **Share console output** - Paste everything from app startup to when you play a track
2. **Check Windows settings** - Verify device is actually set to 96kHz
3. **Test device switch** - Try manually switching devices and check console
4. **Try exclusive mode** - Enable in Windows device properties

---

**Status**: Investigating
**User**: Audient EVO8 at 96kHz plays too fast
**Suspected**: Sample rate mismatch detection issue
