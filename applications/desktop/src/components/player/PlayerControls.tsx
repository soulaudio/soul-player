import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { usePlayerStore } from '@soul-player/shared/stores/player';
import { playerCommands } from '@soul-player/shared/lib/tauri';
import { SkipBack, Play, Pause, SkipForward } from 'lucide-react';

export function PlayerControls() {
  const { isPlaying, currentTrack } = usePlayerStore();
  const [hasNext, setHasNext] = useState(false);
  const [hasPrevious, setHasPrevious] = useState(false);

  // Load capabilities
  useEffect(() => {
    loadCapabilities();

    // Listen for queue updates
    const unlisten = listen('playback:queue-updated', () => {
      loadCapabilities();
    });

    const unlistenTrackChange = listen('playback:track-changed', () => {
      loadCapabilities();
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenTrackChange.then((fn) => fn());
    };
  }, []);

  const loadCapabilities = async () => {
    try {
      const caps = await invoke<{ hasNext: boolean; hasPrevious: boolean }>(
        'get_playback_capabilities'
      );
      setHasNext(caps.hasNext);
      setHasPrevious(caps.hasPrevious);
    } catch (error) {
      console.error('[PlayerControls] Failed to load capabilities:', error);
    }
  };

  const handlePrevious = async () => {
    if (!hasPrevious) return;
    try {
      await playerCommands.skipPrevious();
    } catch (error) {
      console.error('[PlayerControls] Skip previous failed:', error);
    }
  };

  const handlePlayPause = async () => {
    try {
      if (isPlaying) {
        await playerCommands.pausePlayback();
      } else {
        await playerCommands.resumePlayback();
      }
    } catch (error) {
      console.error('[PlayerControls] Play/Pause failed:', error);
    }
  };

  const handleNext = async () => {
    if (!hasNext) return;
    try {
      await playerCommands.skipNext();
    } catch (error) {
      console.error('[PlayerControls] Skip next failed:', error);
    }
  };

  const isPlayDisabled = !currentTrack;
  const isPreviousDisabled = !currentTrack || !hasPrevious;
  const isNextDisabled = !currentTrack || !hasNext;

  return (
    <div className="flex items-center justify-center gap-2">
      {/* Previous button */}
      <button
        onClick={handlePrevious}
        disabled={isPreviousDisabled}
        className="p-2 rounded-full hover:bg-accent transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        aria-label="Previous track"
      >
        <SkipBack className="w-5 h-5" />
      </button>

      {/* Play/Pause button */}
      <button
        onClick={handlePlayPause}
        disabled={isPlayDisabled}
        className="p-3 rounded-full bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        aria-label={isPlaying ? 'Pause' : 'Play'}
      >
        {isPlaying ? (
          <Pause className="w-6 h-6" fill="currentColor" />
        ) : (
          <Play className="w-6 h-6" fill="currentColor" />
        )}
      </button>

      {/* Next button */}
      <button
        onClick={handleNext}
        disabled={isNextDisabled}
        className="p-2 rounded-full hover:bg-accent transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        aria-label="Next track"
      >
        <SkipForward className="w-5 h-5" />
      </button>
    </div>
  );
}
