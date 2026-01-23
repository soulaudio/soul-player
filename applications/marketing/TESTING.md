# Marketing Demo Testing Guide

Comprehensive testing setup for the Soul Player marketing demo, including end-to-end playback tests.

## Overview

The marketing demo uses a full E2E testing approach that simulates real user interactions with the WASM-powered playback system. Tests verify the complete flow from UI clicks to audio playback using mocked `HTMLAudioElement`.

## Test Structure

```
applications/marketing/
├── src/__tests__/
│   └── e2e/
│       ├── README.md              # Detailed E2E test documentation
│       ├── mocks.ts                # HTMLAudioElement mocks
│       ├── test-setup.tsx          # Test utilities and helpers
│       ├── setup.test.tsx          # Infrastructure verification tests
│       └── playback.test.tsx       # Main E2E playback tests (32 test cases)
├── tests/
│   └── setup.ts                    # Global test setup (Vitest)
├── vitest.config.ts                # Vitest configuration
└── vitest.d.ts                     # TypeScript type definitions
```

## Running Tests

### All Tests
```bash
yarn test                           # Run all tests once
yarn test:watch                     # Watch mode for development
yarn test:coverage                  # Run with coverage report
```

### E2E Tests Only
```bash
yarn test:e2e                       # Run E2E playback tests
```

### Specific Test Files
```bash
yarn test setup.test               # Test infrastructure
yarn test playback.test            # Playback E2E tests
```

## Test Categories

### 1. Infrastructure Tests (`setup.test.tsx`)
Verifies that the test environment is properly configured:
- Vitest globals available
- Testing library setup
- Audio mocking works
- Demo app renders

### 2. E2E Playback Tests (`playback.test.tsx`)
Comprehensive user interaction tests covering 32 scenarios:

#### Album Playback (4 tests)
- Playing album from start
- Queue population
- Current track highlighting
- Progress bar updates

#### Queue Interaction (3 tests)
- Jump to track via queue click
- Queue position updates
- Highlight removal

#### Playback Controls (3 tests)
- Pause/resume
- Play button state
- Position persistence

#### Navigation (4 tests)
- Skip next/previous
- Queue advancement
- Auto-play next track
- End of queue handling

#### Shuffle (3 tests)
- Enable/disable shuffle
- Shuffle persistence
- Queue reordering

#### Repeat (3 tests)
- Mode cycling (off → all → one)
- Repeat all behavior
- Repeat one behavior

#### Volume Control (3 tests)
- Slider changes
- Volume persistence
- Mute functionality

#### Seek (2 tests)
- Progress bar seeking
- Playback continuation

#### Error Scenarios (3 tests)
- Empty library
- End of queue edge cases
- Missing audio files

#### Performance (4 tests)
- Large playlists (100+ tracks)
- Rapid track skipping
- Multiple shuffle toggles
- Volume adjustment stress test

## Key Testing Patterns

### Full App Rendering
```typescript
const result = await renderDemoApp();
const cleanup = (result as any).cleanup;

// Wait for demo to load
await waitFor(() => {
  expect(screen.queryByText(/Loading demo/i)).not.toBeInTheDocument();
});

// Always cleanup
cleanup();
```

### User Interaction
```typescript
const user = setupUser();

// Click elements
await user.click(playButton);

// Type in inputs
await user.type(volumeSlider, '50');

// Complex interactions
await clickAlbumCard(user, container, 'Test Album');
await clickPlayOnTrack(user, container, 'Sample Track 1');
```

### Audio State Verification
```typescript
const audio = getMostRecentAudioElement();

// Wait for state changes
await waitForAudioPlaying(audio);
await waitForAudioPaused(audio);

// Verify state
expect(audio.src).toContain('track1.mp3');
expect(audio.paused).toBe(false);
expect(audio.currentTime).toBeCloseTo(90, 1);
```

### UI State Assertions
```typescript
assertPlaybackState(container, {
  isPlaying: true,
  currentTrack: 'Sample Track 1',
  queueLength: 3,
});
```

### Event Simulation
```typescript
// Simulate track ending
simulateAudioEnd(audio);

// Simulate time update
simulateTimeUpdate(audio, 90); // Jump to 90 seconds
```

## Mock Data

### Sample Data (Default)
```typescript
const data = createSampleDemoData();
// Creates:
// - 5 tracks across 2 albums
// - 1 playlist with 3 tracks
// - Realistic durations (180-220 seconds)
// - Cover art URLs
```

### Large Dataset (Performance)
```typescript
const data = createLargeDemoData(100);
// Creates:
// - 100 tracks
// - 20 albums (5 tracks each)
// - Proper relationships
```

### Custom Data
```typescript
const customData: DemoData = {
  tracks: [...],
  albums: [...],
  playlists: [...]
};
const result = await renderDemoApp(customData);
```

## Helper Functions

### UI Query Helpers
```typescript
findButton(container, /play/i)              // Find button by label/text
findAlbumCard(container, 'Test Album')      // Find album card
findTrackRow(container, 'Sample Track 1')   // Find track row
```

### Interaction Helpers
```typescript
await clickAlbumCard(user, container, 'Album')
await clickPlayOnTrack(user, container, 'Track')
await waitForNavigation()
```

### Audio Helpers
```typescript
getMostRecentAudioElement()                 // Get latest Audio instance
await waitForAudioPlaying(audio)            // Wait for play state
await waitForAudioPaused(audio)             // Wait for pause state
simulateAudioEnd(audio)                     // Trigger 'ended' event
simulateTimeUpdate(audio, 90)               // Update currentTime
```

### Assertion Helpers
```typescript
assertPlaybackState(container, {
  isPlaying: true,
  currentTrack: 'Sample Track 1',
  queueLength: 3,
})
```

## Debugging Tips

### Enable Verbose Logging
```typescript
const audio = getMostRecentAudioElement();
audio.addEventListener('play', () => console.log('Audio playing'));
audio.addEventListener('pause', () => console.log('Audio paused'));
audio.addEventListener('timeupdate', () => console.log('Time:', audio.currentTime));
```

### Inspect Current State
```typescript
console.log('Audio state:', {
  paused: audio.paused,
  currentTime: audio.currentTime,
  duration: audio.duration,
  src: audio.src,
});
```

### Debug DOM Output
```typescript
const { debug } = await renderDemoApp();
debug(); // Prints current DOM tree
```

### Use Increased Timeouts
```typescript
await waitFor(() => {
  expect(condition).toBe(true);
}, { timeout: 5000 }); // Increase if needed
```

## CI/CD Integration

Tests are designed to run in CI environments:

- **No audio files required** - All audio is mocked
- **Deterministic timing** - Uses mocks instead of real audio
- **Headless execution** - Runs in jsdom environment
- **Fast execution** - ~30 seconds for full suite

### GitHub Actions Example
```yaml
- name: Run E2E Tests
  run: |
    cd applications/marketing
    yarn test:e2e
```

## Known Limitations

1. **WASM Decoding**: Tests mock WASM module - actual audio decoding not tested
2. **Real Audio**: Mock audio doesn't decode real audio files
3. **Timing**: Some tests use fixed delays - may need adjustment on slower systems
4. **Platform Differences**: Tests run in jsdom - browser-specific bugs may not be caught
5. **Visual**: No visual regression testing (consider adding Playwright/Cypress later)

## Best Practices

### 1. Always Cleanup
```typescript
let cleanup: (() => void) | undefined;

afterEach(() => {
  if (cleanup) {
    cleanup();
    cleanup = undefined;
  }
});
```

### 2. Wait for State Changes
```typescript
// ✅ Good - wait for state
await waitFor(() => {
  expect(audio.paused).toBe(false);
});

// ❌ Bad - race condition
expect(audio.paused).toBe(false);
```

### 3. Use Semantic Queries
```typescript
// ✅ Good - accessible query
screen.getByRole('button', { name: /play/i })

// ❌ Bad - implementation detail
container.querySelector('.play-button')
```

### 4. Test User Flows, Not Implementation
```typescript
// ✅ Good - tests user flow
await user.click(albumCard);
await user.click(playButton);
expect(audio.paused).toBe(false);

// ❌ Bad - tests internal state
expect(store.getState().isPlaying).toBe(true);
```

## Future Enhancements

- [ ] Test keyboard shortcuts
- [ ] Test touch/gesture interactions
- [ ] Test theme switching during playback
- [ ] Add visual regression tests (Playwright)
- [ ] Test accessibility (screen reader)
- [ ] Test error recovery scenarios
- [ ] Test offline behavior
- [ ] Add performance profiling
- [ ] Test concurrent playback scenarios
- [ ] Test memory leaks (long-running tests)

## Troubleshooting

### Tests Timing Out
Increase timeout in `vitest.config.ts`:
```typescript
test: {
  testTimeout: 15000,  // Increase if needed
}
```

### Mock Audio Not Working
Verify setup in `tests/setup.ts`:
```typescript
// Should be called before tests
setupAudioMocks();
```

### TypeScript Errors
Ensure `vitest.d.ts` is included in `tsconfig.json`:
```json
{
  "include": ["vitest.d.ts", ...]
}
```

### Tests Failing in CI
Check for timing issues and increase timeouts:
```typescript
await waitFor(() => {
  expect(condition).toBe(true);
}, { timeout: 5000 });
```

## Contributing

When adding new tests:

1. **Follow existing patterns** - Use helper functions
2. **Test user flows** - Not internal implementation
3. **Add comments** - Explain complex scenarios
4. **Keep tests focused** - One behavior per test
5. **Update README** - Document new test categories
6. **Run locally first** - Ensure tests pass before committing

## Resources

- [Vitest Documentation](https://vitest.dev/)
- [Testing Library](https://testing-library.com/docs/react-testing-library/intro/)
- [E2E Test README](./src/__tests__/e2e/README.md)
- [CLAUDE.md](../../CLAUDE.md) - Project guidelines
