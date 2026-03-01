'use client';

import { Volume2, VolumeX } from 'lucide-react';
import { useState, useRef, useCallback, useEffect, useId } from 'react';
import { useTranslation } from 'react-i18next';

export interface VolumeControlProps {
  volume: number;
  isMuted: boolean;
  /** Called with a value in [0, 1] whenever volume changes (drag, click, or keyboard). */
  onVolumeChange: (volume: number) => void;
  onMuteToggle: () => void;
  onWheel?: (e: React.WheelEvent) => void;
}

/**
 * VolumeControl Component
 *
 * Volume slider with the same smooth drag behaviour as ProgressBar:
 * - Custom document-level mousemove/mouseup listeners during drag (no lag)
 * - No CSS transition on the fill during drag (matches ProgressBar)
 * - Native hidden <input type="range"> kept solely for keyboard accessibility
 *   (arrow keys, Tab focus) — it receives no pointer events
 *
 * Volume Flow:
 * - Backend stores volume as level (0-100) with internal logarithmic scaling
 * - Frontend receives level (0-100), stores as 0-1 for consistency
 * - Component receives level (0-1), calls onVolumeChange(0-1) on interaction
 * - Backend converts 0-1 → 0-100 and applies logarithmic gain internally
 */
export function VolumeControl({
  volume,
  isMuted,
  onVolumeChange,
  onMuteToggle,
  onWheel,
}: VolumeControlProps) {
  const { t } = useTranslation();
  const sliderId = useId();

  // Drag state — mirrors ProgressBar's isDragging / dragPosition pattern
  const [isDragging, setIsDragging] = useState(false);
  const [dragVolume, setDragVolume] = useState<number | null>(null);
  const [isHovering, setIsHovering] = useState(false);
  const trackRef = useRef<HTMLDivElement>(null);

  // Deduplication: prevent click handler from firing immediately after mouseUp
  const lastClickTimeRef = useRef<number>(0);

  // Committed volume (takes mute into account for visuals)
  const displayVolume = isMuted ? 0 : volume;

  // Visual volume: use drag preview when dragging, otherwise committed value
  const visualVolume = isDragging && dragVolume !== null ? dragVolume : displayVolume;
  const volumePercent = Math.round(visualVolume * 100);

  // Calculate volume in [0,1] from a clientX position over the track
  const calculateVolume = useCallback((clientX: number): number => {
    if (!trackRef.current) return 0;
    const rect = trackRef.current.getBoundingClientRect();
    return Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
  }, []);

  // Mouse down — start drag
  const handleMouseDown = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      const vol = calculateVolume(e.clientX);
      setIsDragging(true);
      setDragVolume(vol);
      onVolumeChange(vol);
    },
    [calculateVolume, onVolumeChange]
  );

  // Mouse move — update drag preview (document listener, only while dragging)
  const handleMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isDragging) return;
      const vol = calculateVolume(e.clientX);
      setDragVolume(vol);
      onVolumeChange(vol);
    },
    [isDragging, calculateVolume, onVolumeChange]
  );

  // Mouse up — finalise
  const handleMouseUp = useCallback(
    (e: MouseEvent) => {
      if (!isDragging) return;
      const vol = calculateVolume(e.clientX);
      onVolumeChange(vol);
      lastClickTimeRef.current = performance.now();
      setIsDragging(false);
      setDragVolume(null);
    },
    [isDragging, calculateVolume, onVolumeChange]
  );

  // Attach / detach document listeners while dragging
  useEffect(() => {
    if (isDragging) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
      };
    }
  }, [isDragging, handleMouseMove, handleMouseUp]);

  // Click — quick set without drag (deduplicated against mouseUp)
  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (isDragging) return;
      if (performance.now() - lastClickTimeRef.current < 50) return;
      e.stopPropagation();
      const vol = calculateVolume(e.clientX);
      onVolumeChange(vol);
      lastClickTimeRef.current = performance.now();
    },
    [isDragging, calculateVolume, onVolumeChange]
  );

  // Keyboard: native input fires onChange when arrow keys are used
  const handleKeyboardChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      onVolumeChange(parseFloat(e.target.value));
    },
    [onVolumeChange]
  );

  return (
    <div
      className="flex items-center gap-2 flex-1 min-w-0"
      onWheel={onWheel}
      data-testid="volume-control"
    >
      {/* Mute/Unmute Button */}
      <button
        onClick={onMuteToggle}
        aria-pressed={isMuted}
        aria-label={isMuted ? t('playback.unmuteAudio') : t('playback.muteAudio')}
        title={isMuted ? t('playback.unmuteWithKey') : t('playback.muteWithKey')}
        className="p-1 text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity shrink-0 rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2"
        data-testid="volume-mute-button"
      >
        {isMuted || volume === 0 ? (
          <VolumeX className="w-4 h-4" aria-hidden="true" />
        ) : (
          <Volume2 className="w-4 h-4" aria-hidden="true" />
        )}
      </button>

      {/* Volume Slider — same structure as ProgressBar */}
      <div
        ref={trackRef}
        className="relative flex-1 h-4 flex items-center select-none min-w-0"
        style={{
          cursor: isDragging ? 'grabbing' : 'pointer',
          userSelect: 'none',
        }}
        onClick={handleClick}
        onMouseDown={handleMouseDown}
        onMouseEnter={() => setIsHovering(true)}
        onMouseLeave={() => setIsHovering(false)}
      >
        {/* Visual track — thin line, overflow-hidden clips fill */}
        <div className="relative w-full h-1.5 bg-muted rounded-full overflow-hidden">
          {/* Filled volume */}
          <div
            className="absolute inset-y-0 left-0 bg-primary rounded-full"
            style={{
              width: `${Math.max(0, Math.min(100, visualVolume * 100))}%`,
              maxWidth: '100%',
              transition: 'none',
            }}
          />
        </div>

        {/* Thumb — fades in on hover/drag, scales up while dragging */}
        <div
          className="absolute top-1/2 w-2.5 h-2.5 bg-primary rounded-full pointer-events-none"
          style={{
            left: `${Math.max(0, Math.min(100, visualVolume * 100))}%`,
            transform: `translate(-50%, -50%)${isDragging ? ' scale(1.2)' : ''}`,
            opacity: isHovering || isDragging ? 1 : 0,
            transition: 'opacity 150ms ease, transform 100ms ease',
          }}
        />

        {/* Native input — hidden, used only for keyboard accessibility (Tab + arrow keys).
            pointer-events:none prevents it from intercepting mouse events so the
            custom handlers above control drag without any lag. */}
        <input
          id={sliderId}
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={displayVolume}
          onChange={handleKeyboardChange}
          aria-label={t('playback.volume')}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={volumePercent}
          aria-valuetext={t('playback.volumePercent', { percent: volumePercent })}
          title={t('playback.volumeLabel', { percent: volumePercent })}
          className="
            absolute inset-0 w-full h-full opacity-0 cursor-pointer z-10
            focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2
            focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:rounded
          "
          style={{ pointerEvents: 'none' }}
          data-testid="volume-slider"
        />
      </div>

      {/* Volume percentage display */}
      <span
        className="text-[10px] text-muted-foreground font-mono w-6 text-right shrink-0"
        aria-live="polite"
        aria-atomic="true"
        data-testid="volume-percentage"
      >
        {volumePercent}
      </span>
    </div>
  );
}
