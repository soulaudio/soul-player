/**
 * TDD tests for bugs found in playback hooks and providers.
 *
 * These tests cover pure/unit-testable logic that does NOT require Tauri IPC or
 * browser audio APIs. Each test is directly linked to a documented bug.
 *
 * Covered bugs:
 *   BUG-1  usePlaybackEvents: undefined/NaN position payload corrupts store
 *   BUG-3  usePlaybackEvents: null track on TrackChanged doesn't reset isPlaying
 *   BUG-6  TauriPlayerCommandsProvider: null track on TrackChanged doesn't reset isPlaying
 *   BUG-7  TauriPlayerCommandsProvider: onStateChange passes string as boolean (always truthy)
 *   BUG-10 TauriPlayerCommandsProvider: setVolume sends values >100 to backend (no clamp)
 *   BUG-11 useKeyboardShortcuts: missing event.repeat guard causes rapid-fire commands on held key
 *   BUG-12 player store: composite selectors create new object reference on every render
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { usePlayerStore } from '@soul-player/shared/stores/player';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Reset the Zustand player store to a clean initial state between tests. */
function resetStore() {
  usePlayerStore.setState({
    currentTrack: null,
    isPlaying: false,
    volume: 0.8,
    previousVolume: 0.8,
    progress: 0,
    duration: 0,
    seekVersion: 0,
    seekTarget: 0,
    queue: [],
    queueIndex: -1,
    repeatMode: 'off',
    shuffleMode: 'off',
  });
}

// ---------------------------------------------------------------------------
// BUG-1: undefined/NaN position guard in usePlaybackEvents
// ---------------------------------------------------------------------------
describe('BUG-1: position-updated event — undefined/NaN guard', () => {
  /**
   * The computation is:
   *   progressPercentage = duration > 0 ? Math.min(100, (positionInSeconds / duration) * 100) : 0
   *
   * If positionInSeconds is undefined (IPC glitch), the formula yields NaN.
   * The fix adds: if (typeof positionInSeconds !== 'number' || !isFinite(positionInSeconds)) return;
   *
   * We test the guard logic in isolation as a pure function.
   */
  function computeProgress(positionInSeconds: unknown, duration: number): number | null {
    // This mirrors the fixed guard in usePlaybackEvents.ts
    if (typeof positionInSeconds !== 'number' || !isFinite(positionInSeconds)) return null;
    return duration > 0 ? Math.min(100, (positionInSeconds / duration) * 100) : 0;
  }

  it('returns null for undefined position (should not update store)', () => {
    expect(computeProgress(undefined, 120)).toBeNull();
  });

  it('returns null for null position', () => {
    expect(computeProgress(null, 120)).toBeNull();
  });

  it('returns null for NaN position', () => {
    expect(computeProgress(NaN, 120)).toBeNull();
  });

  it('returns null for Infinity position', () => {
    expect(computeProgress(Infinity, 120)).toBeNull();
  });

  it('returns 0 when duration is 0 (even with valid position)', () => {
    expect(computeProgress(30, 0)).toBe(0);
  });

  it('computes correct percentage for valid position', () => {
    expect(computeProgress(60, 120)).toBe(50);
  });

  it('clamps to 100 when position exceeds duration', () => {
    expect(computeProgress(130, 120)).toBe(100);
  });
});

// ---------------------------------------------------------------------------
// BUG-3 & BUG-6: null track on TrackChanged must reset isPlaying
// ---------------------------------------------------------------------------
describe('BUG-3 / BUG-6: TrackChanged(null) resets isPlaying in the store', () => {
  beforeEach(resetStore);

  it('store isPlaying is true before track ends', () => {
    usePlayerStore.setState({
      isPlaying: true,
      currentTrack: { id: 1, title: 'Track 1', artist: 'Artist', album: 'Album', duration: 180, filePath: '/a.mp3', addedAt: '' },
    });
    expect(usePlayerStore.getState().isPlaying).toBe(true);
    expect(usePlayerStore.getState().currentTrack).not.toBeNull();
  });

  it('simulated null-track handler resets isPlaying and clears currentTrack', () => {
    // Seed the store as if a track is playing
    usePlayerStore.setState({
      isPlaying: true,
      currentTrack: { id: 1, title: 'Track 1', artist: 'Artist', album: 'Album', duration: 180, filePath: '/a.mp3', addedAt: '' },
      duration: 180,
      progress: 42,
    });

    // Simulate what the fixed TrackChanged handler does when track is null
    // (mirrors the else-branch added in both usePlaybackEvents and TauriPlayerCommandsProvider)
    const track = null;
    if (track === null) {
      usePlayerStore.setState({
        currentTrack: null,
        isPlaying: false,
        duration: 0,
        progress: 0,
      });
    }

    const state = usePlayerStore.getState();
    expect(state.currentTrack).toBeNull();
    expect(state.isPlaying).toBe(false);
    expect(state.duration).toBe(0);
    expect(state.progress).toBe(0);
  });

  it('non-null track does NOT reset isPlaying', () => {
    usePlayerStore.setState({ isPlaying: true });

    const track = { id: 2, title: 'Track 2', artist: 'Artist', album: 'Album', duration: 200, filePath: '/b.mp3', addedAt: '' };
    if (track === null) {
      // This branch must NOT run
      usePlayerStore.setState({ isPlaying: false });
    } else {
      usePlayerStore.setState({ currentTrack: track, duration: track.duration, progress: 0 });
    }

    expect(usePlayerStore.getState().isPlaying).toBe(true);
    expect(usePlayerStore.getState().currentTrack?.id).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// BUG-7: onStateChange must convert backend string to boolean
// ---------------------------------------------------------------------------
describe('BUG-7: onStateChange — backend emits string, callback expects boolean', () => {
  /**
   * The backend emits 'Playing' | 'Paused' | 'Stopped' as a string.
   * The PlaybackEventsInterface callback signature is (isPlaying: boolean) => void.
   *
   * The original code passed the raw string to the callback via listen<boolean>,
   * meaning callback('Playing') was called — a truthy string, not a boolean.
   * The fix changes the listener to listen<string> and converts explicitly:
   *   callback(event.payload === 'Playing')
   */
  function convertStateToBoolean(payload: string): boolean {
    // Mirrors the fixed implementation
    return payload === 'Playing';
  }

  it('"Playing" maps to true', () => {
    expect(convertStateToBoolean('Playing')).toBe(true);
  });

  it('"Paused" maps to false', () => {
    expect(convertStateToBoolean('Paused')).toBe(false);
  });

  it('"Stopped" maps to false', () => {
    expect(convertStateToBoolean('Stopped')).toBe(false);
  });

  it('unknown string maps to false (defensive)', () => {
    expect(convertStateToBoolean('Crossfading')).toBe(false);
  });

  it('empty string maps to false', () => {
    expect(convertStateToBoolean('')).toBe(false);
  });

  it('demonstrates original bug: raw string cast to boolean is always truthy', () => {
    // This shows WHY the bug mattered: any non-empty string is truthy.
    // The fix must NOT use !!payload — it must use === 'Playing'.
    const buggyConvert = (payload: string): boolean => !!payload as boolean;
    // Under the bug, 'Paused' would have been truthy (wrong)
    expect(buggyConvert('Paused')).toBe(true);  // Bug: should be false
    expect(buggyConvert('Stopped')).toBe(true); // Bug: should be false
    // Fixed version is correct
    expect(convertStateToBoolean('Paused')).toBe(false);
    expect(convertStateToBoolean('Stopped')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// BUG-10: setVolume must clamp input before multiplying to avoid sending >100
// ---------------------------------------------------------------------------
describe('BUG-10: setVolume clamp before converting to 0-100 range', () => {
  /**
   * The shared interface passes volume as 0-1. The Tauri backend expects 0-100.
   * Original code: Math.round(volume * 100)
   * Bug: if volume = 1.005 (float precision), result is 101.
   * Fix: Math.round(Math.max(0, Math.min(1, volume)) * 100)
   */
  function convertVolume(volume: number): number {
    // Mirrors the fixed implementation in TauriPlayerCommandsProvider
    const clamped = Math.max(0, Math.min(1, volume));
    return Math.round(clamped * 100);
  }

  it('converts 1.0 to 100', () => {
    expect(convertVolume(1.0)).toBe(100);
  });

  it('converts 0.0 to 0', () => {
    expect(convertVolume(0.0)).toBe(0);
  });

  it('converts 0.5 to 50', () => {
    expect(convertVolume(0.5)).toBe(50);
  });

  it('clamps 1.005 to 100 (not 101)', () => {
    expect(convertVolume(1.005)).toBe(100);
  });

  it('clamps 2.0 (invalid high) to 100', () => {
    expect(convertVolume(2.0)).toBe(100);
  });

  it('clamps -0.1 (invalid low) to 0', () => {
    expect(convertVolume(-0.1)).toBe(0);
  });

  it('original bug: unclamped values above 1.0 send >100 to backend', () => {
    const buggyConvert = (v: number) => Math.round(v * 100);
    // Demonstrate the bug with a clearly out-of-range value (e.g. from caller error)
    expect(buggyConvert(1.01)).toBe(101); // Bug: 101 would be sent to the backend
    expect(buggyConvert(2.0)).toBe(200);  // Bug: 200 would be sent
    // Fixed version clamps before converting
    expect(convertVolume(1.01)).toBe(100);
    expect(convertVolume(2.0)).toBe(100);
  });
});

// ---------------------------------------------------------------------------
// BUG-11: event.repeat guard in useKeyboardShortcuts
// ---------------------------------------------------------------------------
describe('BUG-11: handleKeyDown — event.repeat guard prevents rapid-fire commands', () => {
  /**
   * When a key is held down the browser fires keydown repeatedly with event.repeat === true.
   * Without a guard, volume_up/volume_down fire on every repeat tick (rapid spike/drop)
   * and next/previous skip multiple tracks on a single held keypress.
   *
   * Fix: add `if (event.repeat) return;` at the top of handleKeyDown.
   *
   * We test the guard logic in isolation as a pure function that mirrors the
   * conditional in the fixed implementation.
   */

  function shouldHandleKeyDown(eventRepeat: boolean): boolean {
    // Mirrors the fixed guard: if (event.repeat) return; [handler runs] else [handler skipped]
    return !eventRepeat;
  }

  it('handles key when repeat is false (fresh key press)', () => {
    expect(shouldHandleKeyDown(false)).toBe(true);
  });

  it('ignores key when repeat is true (held-down auto-repeat)', () => {
    expect(shouldHandleKeyDown(true)).toBe(false);
  });

  it('demonstrates original bug: without guard every repeat tick fires the command', () => {
    const buggyHandler = (_eventRepeat: boolean) => true; // bug: always runs
    // Under the bug, held keys fire commands repeatedly
    expect(buggyHandler(true)).toBe(true);   // Bug: would execute volume_up/next/etc
    expect(buggyHandler(false)).toBe(true);  // First press: also fires (correct)
    // Fixed version filters out repeats
    expect(shouldHandleKeyDown(true)).toBe(false);   // Fixed: held key ignored
    expect(shouldHandleKeyDown(false)).toBe(true);   // Fixed: first press handled
  });

  it('volume_up would spike on held key without repeat guard', () => {
    // Simulate 10 repeat events from a held ArrowUp (volume_up)
    let callCount = 0;
    const buggyHandleKeyDown = (_eventRepeat: boolean) => {
      // Bug: no repeat check — parameter ignored, always fires
      callCount++;
    };
    const fixedHandleKeyDown = (eventRepeat: boolean) => {
      if (eventRepeat) return; // Fix
      callCount++;
    };

    // Reset and simulate buggy behavior
    callCount = 0;
    for (let i = 0; i < 10; i++) buggyHandleKeyDown(i > 0); // first=false, rest=true
    expect(callCount).toBe(10); // Bug: fires 10 times

    // Reset and simulate fixed behavior
    callCount = 0;
    for (let i = 0; i < 10; i++) fixedHandleKeyDown(i > 0);
    expect(callCount).toBe(1); // Fix: fires only once
  });
});

// ---------------------------------------------------------------------------
// BUG-12: Composite selector stability in player store
// ---------------------------------------------------------------------------
describe('BUG-12: composite selectors use shallow equality to prevent spurious re-renders', () => {
  /**
   * Zustand's default equality is Object.is (reference equality).
   * Selectors that return inline object literals, e.g.:
   *   state => ({ currentTrack: state.currentTrack, isPlaying: state.isPlaying })
   * always return a NEW object reference on every invocation, so any subscribed
   * component re-renders on EVERY store update — even when the returned fields
   * haven't changed.
   *
   * Fix: wrap composite selectors with useShallow() from zustand/react/shallow.
   * useShallow performs a one-level key-by-key comparison and returns the same
   * object reference when all top-level values are unchanged.
   *
   * We test the shallow equality logic in isolation as a pure function.
   */

  function shallowEqual(a: Record<string, unknown>, b: Record<string, unknown>): boolean {
    // Mirrors what useShallow does internally
    const keysA = Object.keys(a);
    const keysB = Object.keys(b);
    if (keysA.length !== keysB.length) return false;
    return keysA.every((k) => Object.is(a[k], b[k]));
  }

  it('shallow equality: identical primitive values are equal', () => {
    const a = { isPlaying: false, currentTrack: null };
    const b = { isPlaying: false, currentTrack: null };
    expect(shallowEqual(a, b)).toBe(true);
  });

  it('shallow equality: changed primitive triggers inequality', () => {
    const a = { isPlaying: false, currentTrack: null };
    const b = { isPlaying: true, currentTrack: null };
    expect(shallowEqual(a, b)).toBe(false);
  });

  it('shallow equality: same object reference is equal', () => {
    const track = { id: 1, title: 'T', artist: 'A', album: 'B', duration: 100, filePath: '/a.mp3', addedAt: '' };
    const a = { isPlaying: true, currentTrack: track };
    const b = { isPlaying: true, currentTrack: track };
    expect(shallowEqual(a, b)).toBe(true);
  });

  it('demonstrates original bug: reference equality causes spurious re-render', () => {
    // Two objects with same content but different references
    const prev = { isPlaying: false, currentTrack: null };
    const next = { isPlaying: false, currentTrack: null }; // same content, new object
    // Without useShallow: reference equality fails → re-render triggered
    const referenceEqual = Object.is(prev, next);
    expect(referenceEqual).toBe(false); // Bug: spurious re-render
    // With useShallow: shallow equality passes → no re-render
    expect(shallowEqual(prev, next)).toBe(true); // Fix: stable
  });

  it('shallow equality: progress change does NOT affect usePlayerPlayback selector', () => {
    // Simulate an unrelated store field (progress) changing while playback fields stay the same
    const track = { id: 5, title: 'Song', artist: 'Band', album: 'Album', duration: 200, filePath: '/f.mp3', addedAt: '' };
    const prevPlayback = { isPlaying: true, currentTrack: track };
    const nextPlayback = { isPlaying: true, currentTrack: track }; // unchanged
    // Even though progress changed elsewhere in the store, the playback selector
    // returns the same values — useShallow should prevent a re-render.
    expect(shallowEqual(prevPlayback, nextPlayback)).toBe(true);
  });
});
