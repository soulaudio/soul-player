# E2E Tests Quick Start

Quick reference for writing and running E2E playback tests.

## Running Tests

```bash
# Install dependencies (first time only)
yarn install

# Run all tests
yarn test

# Run E2E tests only
yarn test:e2e

# Watch mode (re-run on file changes)
yarn test:watch

# With coverage report
yarn test:coverage
```

## Writing a New Test

### 1. Basic Test Structure

```typescript
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { screen, waitFor } from '@testing-library/react';
import { renderDemoApp, setupUser } from './test-setup';

describe('My Feature Tests', () => {
  let cleanup: (() => void) | undefined;

  beforeEach(() => {
    // Reset between tests
  });

  afterEach(() => {
    if (cleanup) {
      cleanup();
      cleanup = undefined;
    }
  });

  it('should do something', async () => {
    const user = setupUser();
    const result = await renderDemoApp();
    cleanup = (result as any).cleanup;

    // Your test code here
    await user.click(screen.getByRole('button', { name: /play/i }));

    await waitFor(() => {
      expect(screen.getByText(/playing/i)).toBeInTheDocument();
    });
  });
});
```

### 2. Common Patterns

#### Render Demo App
```typescript
const result = await renderDemoApp();
cleanup = (result as any).cleanup;

// Or with custom data
const customData = createSampleDemoData();
const result = await renderDemoApp(customData);
```

#### Find and Click Elements
```typescript
const user = setupUser();

// Find button
const playButton = findButton(result.container, /play/i);
await user.click(playButton);

// Click album
await clickAlbumCard(user, result.container, 'Album Name');

// Click track
await clickPlayOnTrack(user, result.container, 'Track Name');
```

#### Verify Audio State
```typescript
const audio = getMostRecentAudioElement();

// Wait for playback
await waitForAudioPlaying(audio);

// Verify state
expect(audio.src).toContain('track1.mp3');
expect(audio.paused).toBe(false);
expect(audio.currentTime).toBeCloseTo(90, 1);
```

#### Simulate Audio Events
```typescript
// Simulate track ending
simulateAudioEnd(audio);

// Simulate time update
simulateTimeUpdate(audio, 90);
```

#### Assert Playback State
```typescript
assertPlaybackState(result.container, {
  isPlaying: true,
  currentTrack: 'Track Name',
  queueLength: 5,
});
```

### 3. Test Data

```typescript
// Sample data (5 tracks, 2 albums)
const data = createSampleDemoData();

// Large dataset (100 tracks)
const largeData = createLargeDemoData(100);

// Custom data
const customData: DemoData = {
  tracks: [
    {
      id: '1',
      title: 'My Track',
      artist: 'Artist',
      album: 'Album',
      duration: 180,
      path: '/audio/track.mp3',
    }
  ],
  albums: [...],
  playlists: [...],
};
```

## Helper Functions Cheat Sheet

### Finding Elements
```typescript
findButton(container, /label/i)              // Button by label/text
findAlbumCard(container, 'Album Name')       // Album card by title
findTrackRow(container, 'Track Name')        // Track row by title
```

### User Actions
```typescript
const user = setupUser();
await user.click(element)
await user.type(input, 'text')
await clickAlbumCard(user, container, 'Album')
await clickPlayOnTrack(user, container, 'Track')
```

### Audio Control
```typescript
const audio = getMostRecentAudioElement()
await audio.play()
audio.pause()
await waitForAudioPlaying(audio)
await waitForAudioPaused(audio)
simulateAudioEnd(audio)
simulateTimeUpdate(audio, seconds)
```

### Assertions
```typescript
// DOM assertions
expect(element).toBeInTheDocument()
expect(element).toHaveTextContent('text')
expect(element).toHaveClass('className')

// Playback state
assertPlaybackState(container, { ... })

// Wait for changes
await waitFor(() => {
  expect(condition).toBe(true)
}, { timeout: 3000 })
```

## Common Issues

### Test Timeout
Increase timeout in test:
```typescript
await waitFor(() => {
  expect(condition).toBe(true);
}, { timeout: 5000 }); // Default is 1000ms
```

### Element Not Found
Use `waitFor`:
```typescript
// ❌ Bad - may fail if element not ready
expect(screen.getByText('text')).toBeInTheDocument();

// ✅ Good - waits for element
await waitFor(() => {
  expect(screen.getByText('text')).toBeInTheDocument();
});
```

### Audio State Not Updating
Use helper functions:
```typescript
// ❌ Bad - race condition
expect(audio.paused).toBe(false);

// ✅ Good - waits for state
await waitForAudioPlaying(audio);
```

### Mock Not Working
Ensure cleanup is called:
```typescript
afterEach(() => {
  if (cleanup) {
    cleanup();
    cleanup = undefined;
  }
});
```

## Debugging

### Log Current State
```typescript
console.log('Audio:', {
  src: audio.src,
  paused: audio.paused,
  currentTime: audio.currentTime,
  duration: audio.duration,
});
```

### Debug DOM
```typescript
const { debug } = await renderDemoApp();
debug(); // Prints DOM tree
```

### Add Event Listeners
```typescript
audio.addEventListener('play', () => console.log('Playing'));
audio.addEventListener('pause', () => console.log('Paused'));
audio.addEventListener('timeupdate', () => {
  console.log('Time:', audio.currentTime);
});
```

## Example: Complete Test

```typescript
it('should play album and skip to next track', async () => {
  // Setup
  const user = setupUser();
  const result = await renderDemoApp();
  cleanup = (result as any).cleanup;

  // Navigate to album
  const albumsLink = screen.getByRole('link', { name: /albums/i });
  await user.click(albumsLink);

  // Click album card
  await clickAlbumCard(user, result.container, 'Test Album');

  // Play first track
  await clickPlayOnTrack(user, result.container, 'Sample Track 1');

  // Verify playback started
  const audio = getMostRecentAudioElement();
  await waitForAudioPlaying(audio);
  expect(audio.src).toContain('track1.mp3');

  // Skip to next track
  const nextButton = findButton(result.container, /next|skip/i);
  await user.click(nextButton);

  // Verify next track playing
  await waitFor(() => {
    assertPlaybackState(result.container, {
      currentTrack: 'Sample Track 2',
      isPlaying: true,
    });
  });
});
```

## Resources

- Full guide: [TESTING.md](../../../TESTING.md)
- Detailed docs: [README.md](./README.md)
- Project guidelines: [CLAUDE.md](../../../../../CLAUDE.md)
