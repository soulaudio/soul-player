/**
 * Progress bar with click-to-seek and drag-to-seek functionality.
 *
 * Features:
 * - Smooth progress interpolation (60fps, no 500ms jumps)
 * - Click-to-seek: Instant seek on click
 * - Drag-to-seek: Preview position while dragging, seek on release (no scrubbing)
 * - Race condition prevention with 50ms ignore window
 * - Visual feedback: hover, dragging, seeking states
 * - Self-contained: connects directly to player store via hooks
 *
 * Based on industry standard patterns from react-h5-audio-player, wavesurfer.js, Video.js
 */

import React, { useState, useRef, useCallback, useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { formatDuration } from '../../lib/utils';
import { useSeekBar } from '../../hooks/useSeekBar';
import { useInterpolatedProgress } from '../../hooks/useInterpolatedProgress';

export function ProgressBar() {
  // Use interpolated progress for smooth animation
  const interpolatedProgress = useInterpolatedProgress();

  // Use interpolated values by default
  const { progress, duration } = interpolatedProgress;
  const { handleSeek, isSeeking } = useSeekBar();

  // State for drag interactions
  const [isDragging, setIsDragging] = useState(false);
  const [dragPosition, setDragPosition] = useState<number | null>(null);
  const [isHovering, setIsHovering] = useState(false);
  const progressBarRef = useRef<HTMLDivElement>(null);

  // Display position: use drag preview if dragging, otherwise actual progress
  const displayProgress = isDragging && dragPosition !== null ? dragPosition : progress;

  // Calculate current time in seconds
  const currentTimeSeconds = duration > 0 ? (displayProgress / 100) * duration : 0;

  // Calculate position from mouse event
  const calculatePosition = useCallback((clientX: number): number => {
    if (!progressBarRef.current) return 0;
    const rect = progressBarRef.current.getBoundingClientRect();
    const percentage = Math.max(0, Math.min(100, ((clientX - rect.left) / rect.width) * 100));
    return percentage;
  }, []);

  // Handle mouse down - start drag
  const handleMouseDown = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragging(true);
    const position = calculatePosition(e.clientX);
    setDragPosition(position);
  }, [calculatePosition]);

  // Handle mouse move - update drag preview (only while dragging)
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isDragging) return;
    const position = calculatePosition(e.clientX);
    setDragPosition(position);
  }, [isDragging, calculatePosition]);

  // Handle mouse up - finalize seek
  const handleMouseUp = useCallback((e: MouseEvent) => {
    if (!isDragging) return;

    const finalPosition = calculatePosition(e.clientX);
    const seekTimeSeconds = Math.min((finalPosition / 100) * duration, Math.max(0, duration - 0.1));

    // Send seek command (no scrubbing - only on release)
    handleSeek(seekTimeSeconds);

    // Reset drag state
    setIsDragging(false);
    setDragPosition(null);
  }, [isDragging, calculatePosition, duration, handleSeek]);

  // Attach/detach window listeners for drag
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

  // Handle click (for quick click-to-seek without drag)
  const handleClick = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    // Don't seek on click if we just finished a drag
    if (isDragging) return;

    e.stopPropagation();
    const position = calculatePosition(e.clientX);
    const seekTimeSeconds = Math.min((position / 100) * duration, Math.max(0, duration - 0.1));
    handleSeek(seekTimeSeconds);
  }, [isDragging, calculatePosition, duration, handleSeek]);

  return (
    <div className="flex items-center gap-3 w-full">
      {/* Current time */}
      <span className="text-xs text-muted-foreground font-mono min-w-[40px] text-right">
        {formatDuration(currentTimeSeconds)}
      </span>

      {/* Progress bar */}
      <div
        ref={progressBarRef}
        className="relative flex-1 h-2 bg-muted rounded-full overflow-hidden select-none"
        style={{
          cursor: isDragging ? 'grabbing' : isSeeking ? 'wait' : 'pointer',
          userSelect: 'none'
        }}
        onClick={handleClick}
        onMouseDown={handleMouseDown}
        onMouseEnter={() => setIsHovering(true)}
        onMouseLeave={() => setIsHovering(false)}
      >
        {/* Filled progress */}
        <div
          className="absolute inset-y-0 left-0 bg-primary rounded-full"
          style={{
            width: `${Math.max(0, Math.min(100, displayProgress))}%`,
            maxWidth: '100%',
            transition: isDragging ? 'none' : 'width 200ms ease-out',
            opacity: isDragging ? 0.8 : isSeeking ? 0.9 : 1,
            boxShadow: (isDragging || isSeeking) ? '0 0 8px 2px rgba(var(--primary-rgb, 59, 130, 246), 0.5)' : 'none'
          }}
        />

        {/* Dragging handle (shown during drag) */}
        {isDragging && (
          <div
            className="absolute top-1/2 -translate-y-1/2 w-4 h-4 bg-primary rounded-full shadow-lg"
            style={{
              left: `${Math.max(0, Math.min(100, displayProgress))}%`,
              transform: 'translate(-50%, -50%) scale(1.2)',
              boxShadow: '0 0 12px 4px rgba(var(--primary-rgb, 59, 130, 246), 0.6)',
              pointerEvents: 'none'
            }}
          />
        )}

        {/* Seeking handle (shown after release, during backend seek) */}
        {isSeeking && !isDragging && (
          <div
            className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full shadow-lg transition-all"
            style={{
              left: `${Math.max(0, Math.min(100, displayProgress))}%`,
              transform: 'translate(-50%, -50%)',
              boxShadow: '0 0 12px 4px rgba(var(--primary-rgb, 59, 130, 246), 0.6)',
              pointerEvents: 'none'
            }}
          >
            {/* Loading spinner during seek */}
            <div className="absolute inset-0 flex items-center justify-center">
              <Loader2 className="w-3 h-3 animate-spin text-primary-foreground" />
            </div>
          </div>
        )}

        {/* Hover handle (only shown when hovering and not dragging/seeking) */}
        {isHovering && !isDragging && !isSeeking && (
          <div
            className="absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full shadow-lg transition-opacity"
            style={{
              left: `${Math.max(0, Math.min(100, progress))}%`,
              transform: 'translate(-50%, -50%)',
              pointerEvents: 'none'
            }}
          />
        )}
      </div>

      {/* Total duration */}
      <span className="text-xs text-muted-foreground font-mono min-w-[40px]">
        {formatDuration(duration)}
      </span>
    </div>
  );
}
