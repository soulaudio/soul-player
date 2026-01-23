# Web Playback Testing Checklist

This document provides a comprehensive manual testing checklist for the web playback library (`libraries/soul-playback-web`). These tests should be performed on the marketing demo application.

**Environment**: `yarn dev:marketing` (http://localhost:5174)

**Test Data**: Use the demo tracks provided by `DemoStorage`

---

## Pre-Testing Setup

- [ ] Confirm WASM module is built (`yarn build:wasm` in marketing app)
- [ ] Start marketing demo (`yarn dev:marketing`)
- [ ] Open browser DevTools console to monitor logs
- [ ] Clear browser cache if needed (Ctrl+Shift+Delete)

---

## 1. Basic Playback Control

### Play/Pause
- [ ] Click play button on a track - should start playback
- [ ] Click pause button - should pause immediately
- [ ] Click play again - should resume from same position
- [ ] Verify play/pause button icon toggles correctly
- [ ] Verify progress bar updates during playback

### Stop
- [ ] Play a track, then click stop - should return to start
- [ ] Verify progress bar resets to 0

### Track Loading
- [ ] Click on different tracks - should load and play new track
- [ ] Verify previous track stops when new track starts
- [ ] Check for smooth transitions (no audio glitches)

**Expected Behavior**: Audio starts/stops smoothly, UI reflects current state

---

## 2. Queue Management

### Load Playlist
- [ ] Click "Play All" on album page - should load all tracks
- [ ] Verify queue shows all tracks in correct order
- [ ] Verify first track starts playing
- [ ] Check queue length matches track count

### Add to Queue
- [ ] Play a track
- [ ] Click "Add to Queue" on another track
- [ ] Verify track appears at end of queue
- [ ] Verify current playback not interrupted

### Play Next
- [ ] Play a track
- [ ] Click "Play Next" on another track
- [ ] Verify track appears after current track
- [ ] Skip to next track - should play the "Play Next" track

### Queue Navigation
- [ ] Click on track in queue - should jump to that track
- [ ] Verify playback starts immediately
- [ ] Verify queue position updates

### Remove from Queue
- [ ] Click remove button on queue item
- [ ] Verify track removed from queue
- [ ] Verify queue length decreases
- [ ] Try removing currently playing track - should handle gracefully

### Clear Queue
- [ ] Click "Clear Queue" button
- [ ] Verify all tracks removed
- [ ] Verify playback stops

**Expected Behavior**: Queue operations work correctly, no crashes or freezes

---

## 3. Track Navigation

### Next Track
- [ ] Play a track with more tracks in queue
- [ ] Click next button - should advance to next track
- [ ] Verify next track starts playing immediately
- [ ] Try clicking next on last track - should handle gracefully

### Previous Track
- [ ] Play second track in queue
- [ ] Click previous button - should go back to first track
- [ ] Verify previous track starts playing
- [ ] Try clicking previous on first track - should handle gracefully

### Auto-Advance
- [ ] Play a short track (or seek to end)
- [ ] Wait for track to finish
- [ ] Verify automatically advances to next track
- [ ] Verify last track stops instead of looping (if repeat off)

### Keyboard Shortcuts (if implemented)
- [ ] Press Space - should play/pause
- [ ] Press Right Arrow - should skip next
- [ ] Press Left Arrow - should skip previous

**Expected Behavior**: Navigation works smoothly, no skipped tracks or errors

---

## 4. Shuffle Mode

### Enable Shuffle
- [ ] Load a playlist (5+ tracks)
- [ ] Enable shuffle mode
- [ ] Verify queue order changes
- [ ] Click next multiple times - verify random order

### Disable Shuffle
- [ ] With shuffle enabled, disable it
- [ ] Verify queue returns to original order
- [ ] Verify current track position maintained

### Shuffle Icon State
- [ ] Verify shuffle icon highlights when enabled
- [ ] Verify icon returns to normal when disabled

**Expected Behavior**: Shuffle randomizes playback order, can be toggled on/off

---

## 5. Repeat Mode

### Repeat Off
- [ ] Play playlist to end with repeat off
- [ ] Verify playback stops at last track
- [ ] Verify no auto-loop

### Repeat All
- [ ] Enable repeat all
- [ ] Play playlist to end
- [ ] Verify loops back to first track
- [ ] Verify can skip backward from first track to last

### Repeat One
- [ ] Enable repeat one
- [ ] Let track finish
- [ ] Verify same track repeats
- [ ] Verify next/previous buttons still work

### Mode Cycling
- [ ] Click repeat button multiple times
- [ ] Verify cycles: Off → All → One → Off
- [ ] Verify icon updates for each mode

**Expected Behavior**: Repeat modes work as expected, UI reflects current mode

---

## 6. Volume Control

### Volume Slider
- [ ] Drag volume slider to 50% - verify audio volume decreases
- [ ] Drag to 0% - verify audio muted
- [ ] Drag to 100% - verify audio at max
- [ ] Verify volume changes are smooth (no pops/clicks)

### Mute/Unmute
- [ ] Click mute button - should mute immediately
- [ ] Click again - should restore previous volume
- [ ] Verify mute icon changes

### Volume Persistence (if implemented)
- [ ] Change volume to 75%
- [ ] Reload page
- [ ] Verify volume is still 75%

**Expected Behavior**: Volume controls work smoothly, no audio distortion

---

## 7. Seek Functionality

### Seek Bar Click
- [ ] Play a track
- [ ] Click at different positions on seek bar
- [ ] Verify playback jumps to clicked position
- [ ] Verify audio resumes from new position

### Seek Bar Drag
- [ ] Drag seek bar handle backward
- [ ] Drag seek bar handle forward
- [ ] Verify playback position follows drag
- [ ] Verify smooth seeking (no stuttering)

### Time Display
- [ ] Verify current time updates during playback
- [ ] Verify total duration shows correct track length
- [ ] Verify format is MM:SS

### Edge Cases
- [ ] Seek to 0:00 - should restart track
- [ ] Seek to end of track - should auto-advance (or stop if last)
- [ ] Seek while paused - should update position without playing

**Expected Behavior**: Seeking works accurately, time display is correct

---

## 8. Error Handling

### Invalid Track
- [ ] Try to play track with invalid URL (modify demo data if needed)
- [ ] Verify error message displayed
- [ ] Verify app doesn't crash

### Empty Queue
- [ ] Clear queue
- [ ] Try to click next/previous
- [ ] Verify appropriate message shown
- [ ] Verify no console errors

### Network Issues (if streaming)
- [ ] Disconnect network
- [ ] Try to play track
- [ ] Verify error handling
- [ ] Reconnect and verify recovery

### WASM Initialization
- [ ] Refresh page multiple times
- [ ] Verify WASM loads correctly each time
- [ ] Check console for initialization errors

**Expected Behavior**: Errors are handled gracefully with user-friendly messages

---

## 9. UI State Synchronization

### Playback State
- [ ] Verify play/pause button matches actual playback state
- [ ] Verify progress bar updates in real-time
- [ ] Verify current track highlights in queue

### Queue Updates
- [ ] Add/remove tracks - verify queue UI updates immediately
- [ ] Verify queue length badge updates
- [ ] Verify queue order reflects shuffle/unshuffle

### Multiple UI Elements
- [ ] Verify Now Playing bar matches main player
- [ ] Verify queue panel matches playback state
- [ ] Verify all volume indicators sync

**Expected Behavior**: UI always reflects actual playback state

---

## 10. Performance

### Smooth Playback
- [ ] Play track for 1+ minute
- [ ] Verify no audio dropouts or stuttering
- [ ] Verify CPU usage reasonable (check Task Manager)

### Large Queue
- [ ] Load playlist with 50+ tracks
- [ ] Verify queue renders smoothly
- [ ] Try scrolling queue - should be smooth
- [ ] Try skipping through tracks - should be fast

### Memory Leaks
- [ ] Play multiple tracks in sequence
- [ ] Check browser memory usage (DevTools Memory tab)
- [ ] Verify memory doesn't grow indefinitely

### Multiple Rapid Actions
- [ ] Rapidly click next/previous
- [ ] Rapidly change volume
- [ ] Rapidly toggle shuffle/repeat
- [ ] Verify no crashes or freezes

**Expected Behavior**: Smooth performance, no memory leaks or lag

---

## 11. Event Emission

### State Change Events
- [ ] Open browser console
- [ ] Play/pause/stop - verify `stateChange` events logged
- [ ] Check event payload is correct

### Track Change Events
- [ ] Skip to next track - verify `trackChange` event
- [ ] Verify current track info in event payload

### Queue Change Events
- [ ] Add/remove track - verify `queueChange` event
- [ ] Shuffle - verify `queueChange` event

### Position Update Events
- [ ] Play track - verify `positionUpdate` events
- [ ] Verify events fire every ~100-250ms

**Expected Behavior**: All events emit correctly with proper data

---

## 12. Browser Compatibility

Test on multiple browsers:

### Chrome
- [ ] All features work
- [ ] No console errors
- [ ] Audio quality good

### Firefox
- [ ] All features work
- [ ] No console errors
- [ ] Audio quality good

### Safari (macOS)
- [ ] All features work
- [ ] Web Audio API compatibility
- [ ] Autoplay policy handling

### Edge
- [ ] All features work
- [ ] No console errors

**Expected Behavior**: Works consistently across browsers

---

## 13. Mobile Responsiveness (Optional)

### Mobile Browser
- [ ] Test on mobile Chrome/Safari
- [ ] Verify touch controls work
- [ ] Verify swipe gestures (if implemented)
- [ ] Check responsive layout

**Expected Behavior**: Works on mobile devices (though not primary target)

---

## Integration Tests (Future)

For full end-to-end testing, consider implementing:
- [ ] Playwright tests for user flows
- [ ] Cypress tests for UI interactions
- [ ] Automated visual regression tests

---

## Test Results Template

**Date**: YYYY-MM-DD
**Tester**: [Your Name]
**Environment**: Chrome/Firefox/Safari
**Version**: [App Version]

| Feature | Status | Notes |
|---------|--------|-------|
| Play/Pause | ✅/❌ | |
| Queue Management | ✅/❌ | |
| Track Navigation | ✅/❌ | |
| Shuffle | ✅/❌ | |
| Repeat | ✅/❌ | |
| Volume | ✅/❌ | |
| Seek | ✅/❌ | |
| Error Handling | ✅/❌ | |
| UI Sync | ✅/❌ | |
| Performance | ✅/❌ | |

**Overall Status**: ✅ Pass / ❌ Fail

**Issues Found**:
1. [Description of issue]
2. [Description of issue]

**Notes**:
- [Additional observations]

---

## Automated Test Coverage

**Unit Tests**: `yarn test` in `libraries/soul-playback-web`
- WasmPlaybackAdapter tests
- WebAudioPlayer tests

**Coverage Goal**: 50-60% meaningful coverage (no trivial getters/setters)

Run coverage report: `yarn test:coverage`

---

## Known Limitations

- WASM module requires browser environment (cannot test in Node.js)
- Web Audio API has browser-specific quirks
- Autoplay policy may block playback on some browsers
- Seeking accuracy depends on audio format

---

**Last Updated**: 2026-01-23
