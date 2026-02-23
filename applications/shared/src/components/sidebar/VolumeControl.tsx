'use client';

import { Volume2, VolumeX } from 'lucide-react';
import { useId } from 'react';
import { useTranslation } from 'react-i18next';

export interface VolumeControlProps {
  volume: number;
  isMuted: boolean;
  onVolumeChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onMuteToggle: () => void;
  onWheel?: (e: React.WheelEvent) => void;
}

/**
 * NOTE: Logarithmic conversion functions removed.
 * The backend (soul-playback Volume module) already handles logarithmic scaling internally.
 * Volume is stored as a level (0-100) which the backend converts to logarithmic gain.
 * The UI just needs to pass through the level value without additional conversion.
 */

/**
 * VolumeControl Component
 *
 * Accessible volume slider with mute toggle following WCAG guidelines and music player best practices.
 *
 * Features:
 * - Full ARIA support (aria-label, aria-valuenow, aria-pressed)
 * - Keyboard accessible (arrow keys, Page Up/Down, Home/End)
 * - Logarithmic volume scaling (handled by backend)
 * - Mouse wheel support
 * - Visual feedback on hover and focus
 * - Touch-friendly 44px minimum touch target
 * - Percentage display
 *
 * Volume Flow:
 * - Backend stores volume as level (0-100) with internal logarithmic scaling
 * - Frontend receives level (0-100), stores as 0-1 for consistency
 * - Component displays level as-is on slider (0-1)
 * - User adjusts slider (0-1 level)
 * - Component passes level (0-1) to onChange
 * - Backend converts 0-1 → 0-100 and applies logarithmic gain internally
 *
 * @see https://wcag.dock.codes/documentation/wcag142/ - WCAG Audio Control
 * @see https://www.digitala11y.com/slider-role/ - ARIA Slider Best Practices
 * @see https://developer.mozilla.org/en-US/docs/Web/API/GainNode - Web Audio API GainNode
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

  // Volume is already a level (0-1), no conversion needed
  // Backend handles logarithmic scaling internally
  const displayVolume = isMuted ? 0 : volume;
  const volumePercent = Math.round(displayVolume * 100);

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

      {/* Volume Slider Container */}
      <div
        className="relative flex items-center group flex-1 min-w-0"
        style={{ height: '44px' }} // WCAG minimum touch target
      >
        {/* Native range input (provides keyboard + screen reader support) */}
        <input
          id={sliderId}
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={displayVolume}
          onChange={onVolumeChange}
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
          data-testid="volume-slider"
        />

        {/* Visual slider track (background) */}
        <div
          className="rounded-full bg-muted group-hover:bg-muted/80 transition-colors"
          style={{
            position: 'absolute',
            left: 0,
            right: 0,
            height: '4px',
            top: '50%',
            transform: 'translateY(-50%)'
          }}
          aria-hidden="true"
        />

        {/* Visual slider fill (primary color) */}
        <div
          className="rounded-full bg-primary transition-all duration-100 group-hover:bg-primary/90"
          style={{
            position: 'absolute',
            left: 0,
            height: '4px',
            width: `${displayVolume * 100}%`,
            top: '50%',
            transform: 'translateY(-50%)'
          }}
          aria-hidden="true"
        />

        {/* Visual slider thumb (appears on hover) */}
        <div
          className="absolute w-3 h-3 bg-primary rounded-full opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none"
          style={{
            left: `calc(${displayVolume * 100}% - 6px)`,
            top: '50%',
            transform: 'translateY(-50%)'
          }}
          aria-hidden="true"
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
