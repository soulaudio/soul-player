/**
 * Volume control with slider and mute button
 */

import { useState, useRef, useEffect, useCallback } from 'react';
import { usePlayerStore, useVolume } from '../../stores/player';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { Volume2, VolumeX } from 'lucide-react';
import { debug } from '../../utils/debug';

const SCROLL_VOLUME_STEP = 0.05;

export function VolumeControl() {
  const volume = useVolume();
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
          debug.error('[VolumeControl] Set volume failed:', error);
        });
    }, 150);
  }, [commands, isMuted]);

  useEffect(() => {
    const container = sliderContainerRef.current;
    if (!container) return;

    const handleWheel = (e: WheelEvent) => {
      // Only handle wheel events if the target is within the volume control container
      // This prevents handling scroll events from other UI elements (e.g., dropdowns)
      const target = e.target as HTMLElement;
      if (!container.contains(target) || target.closest('[data-dropdown-menu]')) {
        return;
      }

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
      debug.error('[VolumeControl] Mute toggle failed:', error);
    }
  };

  const displayVolume = isMuted ? 0 : volume;

  return (
    <div className="flex items-center gap-2 shrink-0" style={{ border: '1px solid red', padding: '4px' }}>
      <button
        onClick={handleMuteToggle}
        className="p-2 rounded-full hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors shrink-0"
        aria-label={isMuted ? 'Unmute' : 'Mute'}
      >
        {isMuted || volume === 0 ? (
          <VolumeX className="w-5 h-5" />
        ) : (
          <Volume2 className="w-5 h-5" />
        )}
      </button>

      <div
        ref={sliderContainerRef}
        className="relative shrink-0"
        style={{
          width: '96px',
          minWidth: '96px',
          maxWidth: '96px',
          border: '2px solid blue',
          backgroundColor: 'rgba(255,0,0,0.1)'
        }}
      >
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={displayVolume}
          onChange={handleVolumeChange}
          className="absolute inset-0 w-full h-full opacity-0 cursor-pointer z-20"
          aria-label="Volume"
          style={{ width: '96px' }}
        />

        {/* Track container with solid background */}
        <div
          className="rounded-full overflow-hidden relative"
          style={{
            height: '8px',
            width: '96px',
            backgroundColor: '#374151',
            border: '1px solid yellow'
          }}
        >
          {/* Filled portion */}
          <div
            className="bg-primary transition-all duration-100"
            style={{
              width: `${displayVolume * 100}%`,
              height: '8px',
              backgroundColor: '#3b82f6'
            }}
          />
        </div>

        {/* Hover handle positioned on top */}
        <div
          className="absolute bg-primary rounded-full shadow-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"
          style={{
            left: `${displayVolume * 100}%`,
            top: '50%',
            transform: 'translate(-50%, -50%)',
            width: '12px',
            height: '12px'
          }}
        />
      </div>

      <span className="text-xs text-muted-foreground font-mono shrink-0" style={{ width: '32px', textAlign: 'right' }}>
        {Math.round(displayVolume * 100)}
      </span>
    </div>
  );
}
