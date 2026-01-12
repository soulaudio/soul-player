# Debug Pause Bug - Test Instructions

## What I Added

I've added comprehensive debug logging to trace the exact command and state flow:

1. **PlaybackCommand::Play** - logs state before and after
2. **PlaybackCommand::Pause** - logs state before and after
3. **poll_track_loader** - logs state BEFORE and AFTER set_audio_source()
4. **set_audio_source** - logs whether keeping Paused state or setting Playing

## How to Test

1. **Build the app:**
   ```bash
   cd applications/desktop/src-tauri
   cargo build --release
   ```

2. **Run with logs enabled:**
   ```bash
   yarn dev:desktop:logs
   ```

3. **Reproduce the bug:**
   - Click **Play** on any track/album
   - **Immediately** click **Pause** (or press Space)
   - Watch the terminal logs

4. **Check what actually happens:**
   Look for this sequence in the logs:
   ```
   [PlaybackCommand::Play] Received
   [PlaybackCommand::Play] State after play(): Loading
   [PlaybackCommand::Play] Requesting track load: <track name>
   [PlaybackCommand::Pause] Received, current state: Loading
   [PlaybackCommand::Pause] After pause(), state: Paused
   [poll_track_loader] Track loaded: <track name>
   [poll_track_loader] State BEFORE set_audio_source: Paused  <-- KEY
   [set_audio_source] Keeping state=Paused (user has paused/stopped)
   [poll_track_loader] State AFTER set_audio_source: Paused   <-- KEY
   ```

## Expected vs Actual

**Expected (if fix works):**
- State should stay `Paused` after set_audio_source
- Audio should be silent
- Progress bar should not move

**If bug persists:**
The logs will show us ONE of these scenarios:

### Scenario A: State reverts to Playing
```
[poll_track_loader] State BEFORE set_audio_source: Paused
[set_audio_source] Setting state to Playing  <-- BUG: shouldn't happen!
[poll_track_loader] State AFTER set_audio_source: Playing
```
**Meaning:** Our fix in set_audio_source() isn't working correctly

### Scenario B: Additional Play command sent
```
[poll_track_loader] State AFTER set_audio_source: Paused
[PlaybackCommand::Play] Received  <-- BUG: unexpected command!
```
**Meaning:** UI or event handler is sending extra Play command

### Scenario C: Audio plays despite Paused state
```
[poll_track_loader] State AFTER set_audio_source: Paused
<no more commands, but audio continues>
```
**Meaning:** process_audio() callback is not respecting Paused state

## What to Send Me

Copy the terminal output showing:
1. The sequence starting from when you click Play
2. Through when you click Pause
3. Until track finishes loading
4. And a few seconds after

This will tell us exactly where the bug is.
