# E2E Playback Tests for Marketing Demo

Comprehensive end-to-end tests that verify actual user interactions with the WASM-powered playback system in the marketing demo.

## Overview

These tests simulate real user behavior by:
- Rendering the full `DemoApp` component with all providers
- Mocking `HTMLAudioElement` to simulate audio playback
- Using `@testing-library/user-event` for realistic user interactions
- Testing the complete flow from UI click to audio playback

## Test Files

### `mocks.ts`
Mock utilities for testing:
- `MockHTMLAudioElement` - Full audio element mock with event simulation
- `setupAudioMocks()` - Global audio mock setup
- Helper functions for waiting on audio state changes

### `test-setup.tsx`
Test setup helpers:
- `renderDemoApp()` - Renders full demo with pre-loaded data
- `createSampleDemoData()` - Creates test data fixtures
- `createLargeDemoData()` - Creates large datasets for performance tests
- UI query helpers (`findButton`, `findTrackRow`, etc.)
- Assertion helpers (`assertPlaybackState`)

### `playback.test.tsx`
Main E2E test suite covering:

#### Album Playback Flow (4 tests)
- Playing album from start
- Queue population with album tracks
- Current track highlighting in queue
- Progress bar updates

#### Queue Interaction Flow (3 tests)
- Jumping to tracks via queue clicks
- Queue position indicator updates
- Highlight removal when changing tracks

#### Playback Controls Flow (3 tests)
- Pause/resume functionality
- Play button state changes
- Position persistence across pause/resume

#### Navigation Flow (4 tests)
- Skip next/previous
- Queue position advancement
- Automatic next track on end
- Edge cases (end of queue)

#### Shuffle Flow (3 tests)
- Enable/disable shuffle
- Shuffle persistence across tracks
- Queue reordering

#### Repeat Flow (3 tests)
- Repeat mode cycling (off → all → one → off)
- Repeat all behavior (loop queue)
- Repeat one behavior (same track)

#### Volume Control Flow (3 tests)
- Volume slider changes
- Volume persistence across tracks
- Mute functionality

#### Seek Flow (2 tests)
- Progress bar seeking
- Playback continuation after seek

#### Error Scenarios (3 tests)
- Empty library handling
- End of queue edge case
- Missing audio file handling

#### Performance Tests (4 tests)
- Large playlist handling (100+ tracks)
- Rapid track skipping
- Multiple shuffle toggles
- Volume adjustment responsiveness

**Total: 32 test cases**

## Running Tests

```bash
# Run all E2E tests
yarn test:e2e

# Run all tests (including E2E)
yarn test

# Watch mode for development
yarn test:watch

# With coverage
yarn test:coverage
```

## Test Architecture

```
User Interaction (click, type, etc.)
  ↓
React Component Tree
  ↓
DemoPlayerCommandsProvider → WebPlaybackProvider
  ↓
WasmPlaybackAdapter (mocked HTMLAudioElement)
  ↓
Event Emission → Zustand Store
  ↓
UI Update (verified by assertions)
```

## Key Testing Patterns

### 1. Full App Rendering
```typescript
const result = await renderDemoApp();
// Wait for demo to load
await waitFor(() => {
  expect(screen.queryByText(/Loading demo/i)).not.toBeInTheDocument();
});
```

### 2. User Event Simulation
```typescript
const user = setupUser();
await user.click(playButton);
await user.type(volumeSlider, '50');
```

### 3. Audio State Verification
```typescript
const audio = getMostRecentAudioElement();
await waitForAudioPlaying(audio);
expect(audio.src).toContain('track1.mp3');
```

### 4. UI State Assertions
```typescript
assertPlaybackState(result.container, {
  isPlaying: true,
  currentTrack: 'Sample Track 1',
  queueLength: 3,
});
```

### 5. Event Simulation
```typescript
simulateAudioEnd(audio); // Trigger 'ended' event
simulateTimeUpdate(audio, 90); // Update currentTime
```

## Mock Data

Tests use sample data created via `createSampleDemoData()`:
- 5 tracks across 2 albums
- 1 playlist with 3 tracks
- Cover art URLs included
- Realistic durations (180-220 seconds)

For performance tests, `createLargeDemoData(100)` generates:
- 100 tracks
- 20 albums (5 tracks each)
- Proper track/album relationships

## Debugging Tips

### Enable Audio Mock Logging
```typescript
const audio = getMostRecentAudioElement();
audio.addEventListener('play', () => console.log('Audio playing'));
audio.addEventListener('pause', () => console.log('Audio paused'));
```

### Check Current State
```typescript
console.log('Audio state:', {
  paused: audio.paused,
  currentTime: audio.currentTime,
  duration: audio.duration,
  src: audio.src,
});
```

### Wait for Specific Conditions
```typescript
await waitFor(() => {
  expect(audio.paused).toBe(false);
}, { timeout: 3000 });
```

### Debug Render Output
```typescript
const { debug } = await renderDemoApp();
debug(); // Prints current DOM tree
```

## Known Limitations

1. **WASM Initialization**: Tests mock WASM module - actual WASM decoding not tested
2. **Audio Decoding**: Mock audio doesn't decode real audio files
3. **Timing**: Some tests use fixed delays - may need adjustment on slower systems
4. **Platform Differences**: Tests run in jsdom - browser-specific bugs may not be caught

## CI/CD Integration

Tests are designed to run in CI environments:
- No actual audio files required
- Deterministic timing with mocks
- Headless execution in jsdom
- Fast execution (~30 seconds for all tests)

## Future Enhancements

- [ ] Test keyboard shortcuts integration
- [ ] Test touch/gesture interactions (for mobile demo)
- [ ] Test theme switching during playback
- [ ] Test playlist creation/modification
- [ ] Add visual regression tests
- [ ] Test accessibility (screen reader compatibility)
- [ ] Test error recovery scenarios
- [ ] Test offline behavior
