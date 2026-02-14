/**
 * Progress bar with click-to-seek functionality and smooth 60fps interpolation.
 *
 * Features:
 * - Smooth progress interpolation (no more 500ms jumps from backend)
 * - Immediate click-to-seek (no drag/scrubbing)
 * - Race condition prevention with 100ms ignore window
 * - Seek verification after ignore window
 * - Visual seek handle on hover
 * - Automatic pause detection and track change handling
 * - Self-contained: connects directly to player store via hooks
 *
 * Use cases: Desktop app PlayerPanel, standalone player layouts
 * Status: Actively used in production (PlayerPanel sidebar)
 */

import React from 'react';
import { formatDuration } from '../../lib/utils';
import { useSeekBar } from '../../hooks/useSeekBar';
import { useInterpolatedProgress } from '../../hooks/useInterpolatedProgress';

export function ProgressBar() {
  // Use interpolated progress for smooth animation
  const interpolatedProgress = useInterpolatedProgress();

  // Use interpolated values by default
  const { progress, duration } = interpolatedProgress;
  const { handleSeek } = useSeekBar();

  // Calculate current time in seconds
  const currentTimeSeconds = duration > 0 ? (progress / 100) * duration : 0;

  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    e.stopPropagation();

    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const width = rect.width;
    const percentage = Math.max(0, Math.min(100, (clickX / width) * 100));
    // Clamp to prevent seeking beyond track duration (leave 0.1s buffer to avoid EOF)
    const newPosition = Math.min((percentage / 100) * duration, Math.max(0, duration - 0.1));

    handleSeek(newPosition);
  };

  return (
    <div className="flex items-center gap-3 w-full">
      {/* Current time */}
      <span className="text-xs text-muted-foreground font-mono min-w-[40px] text-right">
        {formatDuration(currentTimeSeconds)}
      </span>

      {/* Progress bar */}
      <div
        className="relative flex-1 h-2 bg-muted rounded-full cursor-pointer group overflow-hidden"
        onClick={handleClick}
      >
        {/* Filled progress */}
        <div
          className="absolute inset-y-0 left-0 bg-primary rounded-full transition-all duration-100"
          style={{
            width: `${Math.max(0, Math.min(100, progress))}%`,
            maxWidth: '100%'
          }}
        />

        {/* Seek handle */}
        <div
          className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full shadow-lg transition-opacity opacity-0 group-hover:opacity-100"
          style={{ left: `${Math.max(0, Math.min(100, progress))}%`, transform: 'translate(-50%, -50%)' }}
        />
      </div>

      {/* Total duration */}
      <span className="text-xs text-muted-foreground font-mono min-w-[40px]">
        {formatDuration(duration)}
      </span>
    </div>
  );
}
