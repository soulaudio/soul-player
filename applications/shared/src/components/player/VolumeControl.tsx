/**
 * Volume control with slider and mute button
 */

import { useState, useRef, useEffect, useCallback } from 'react';
import { usePlayerStore } from '../../stores/player';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { Volume2, VolumeX } from 'lucide-react';

const SCROLL_VOLUME_STEP = 0.05;

export function VolumeControl() {
  const { volume } = usePlayerStore();
  const commands = usePlayerCommands();
  const [isMuted, setIsMuted] = useState(false);
  const [volumeBeforeMute, setVolumeBeforeMute] = useState(volume);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const sliderContainerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (volume > 0 && !isMuted) {
      setVolumeBeforeMute(volume);
    }
  }, [volume, isMuted]);

  const applyVolumeChange = useCallback((newVolume: number) => {
    const clampedVolume = Math.max(0, Math.min(1, newVolume));

    usePlayerStore.getState().setVolume(clampedVolume);

    if (clampedVolume > 0 && isMuted) {
      setIsMuted(false);
    }

    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    debounceTimerRef.current = setTimeout(() => {
      commands.setVolume(clampedVolume)
        .catch((error) => {
          console.error('[VolumeControl] Set volume failed:', error);
        });
    }, 150);
  }, [commands, isMuted]);

  useEffect(() => {
    const container = sliderContainerRef.current;
    if (!container) return;

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      const currentVolume = usePlayerStore.getState().volume;
      const delta = e.deltaY < 0 ? SCROLL_VOLUME_STEP : -SCROLL_VOLUME_STEP;
      applyVolumeChange(currentVolume + delta);
    };

    container.addEventListener('wheel', handleWheel, { passive: false });
    return () => container.removeEventListener('wheel', handleWheel);
  }, [applyVolumeChange]);

  const handleVolumeChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    applyVolumeChange(parseFloat(e.target.value));
  };

  const handleMuteToggle = async () => {
    try {
      if (isMuted) {
        await commands.setVolume(volumeBeforeMute);
        usePlayerStore.getState().setVolume(volumeBeforeMute);
        setIsMuted(false);
      } else {
        setVolumeBeforeMute(volume);
        await commands.setVolume(0);
        usePlayerStore.getState().setVolume(0);
        setIsMuted(true);
      }
    } catch (error) {
      console.error('[VolumeControl] Mute toggle failed:', error);
    }
  };

  const displayVolume = isMuted ? 0 : volume;

  return (
    <div className="flex items-center gap-2">
      <button
        onClick={handleMuteToggle}
        className="p-2 rounded-full hover:bg-accent transition-colors"
        aria-label={isMuted ? 'Unmute' : 'Mute'}
      >
        {isMuted || volume === 0 ? (
          <VolumeX className="w-5 h-5" />
        ) : (
          <Volume2 className="w-5 h-5" />
        )}
      </button>

      <div ref={sliderContainerRef} className="relative w-24 h-2 group">
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={displayVolume}
          onChange={handleVolumeChange}
          className="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10"
          aria-label="Volume"
        />

        <div className="absolute inset-0 bg-muted rounded-full" />

        <div
          className="absolute inset-y-0 left-0 bg-primary rounded-full"
          style={{ width: `${displayVolume * 100}%` }}
        />

        <div
          className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full shadow-lg opacity-0 group-hover:opacity-100 transition-opacity"
          style={{ left: `${displayVolume * 100}%`, transform: 'translate(-50%, -50%)' }}
        />
      </div>

      <span className="text-xs text-muted-foreground font-mono w-8 text-right">
        {Math.round(displayVolume * 100)}
      </span>
    </div>
  );
}
