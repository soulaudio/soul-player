import { useState, useEffect } from 'react';
import { usePlayerStore } from '../../stores/player';
import { usePlayerCommands, usePlaybackEvents } from '../../contexts/PlayerCommandsContext';
import { SkipBack, Play, Pause, SkipForward } from 'lucide-react';

export function PlayerControls() {
  const { isPlaying, currentTrack } = usePlayerStore();
  const commands = usePlayerCommands();
  const events = usePlaybackEvents();
  const [hasNext, setHasNext] = useState(false);
  const [hasPrevious, setHasPrevious] = useState(false);

  // Load capabilities
  useEffect(() => {
    loadCapabilities();

    // Listen for queue and track changes
    // IMPORTANT: Defer capability check to avoid Rust borrow conflicts
    const unsubQueue = events.onQueueUpdate(() => {
      setTimeout(() => loadCapabilities(), 0);  // Defer to next tick
    });

    const unsubTrack = events.onTrackChange(() => {
      setTimeout(() => loadCapabilities(), 0);  // Defer to next tick
    });

    return () => {
      unsubQueue();
      unsubTrack();
    };
  }, [events]);

  const loadCapabilities = async () => {
    try {
      const caps = await commands.getPlaybackCapabilities();
      setHasNext(caps.hasNext);
      setHasPrevious(caps.hasPrevious);
    } catch (error) {
      console.error('[PlayerControls] Failed to load capabilities:', error);
    }
  };

  const handlePrevious = async () => {
    console.log('[PlayerControls] Previous clicked, hasPrevious:', hasPrevious);
    if (!hasPrevious) return;
    try {
      await commands.skipPrevious();
      console.log('[PlayerControls] Previous command succeeded');
    } catch (error) {
      console.error('[PlayerControls] Skip previous failed:', error);
    }
  };

  const handlePlayPause = async () => {
    try {
      if (isPlaying) {
        await commands.pausePlayback();
      } else {
        await commands.resumePlayback();
      }
    } catch (error) {
      console.error('[PlayerControls] Play/Pause failed:', error);
    }
  };

  const handleNext = async () => {
    console.log('[PlayerControls] Next clicked, hasNext:', hasNext);
    if (!hasNext) return;
    try {
      await commands.skipNext();
      console.log('[PlayerControls] Next command succeeded');
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
