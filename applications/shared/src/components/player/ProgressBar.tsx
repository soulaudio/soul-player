/**
 * Progress bar with click-to-seek and drag-to-seek functionality.
 *
 * - Interpolated progress at 60fps between backend position updates (~250ms interval)
 * - Click-to-seek: Instant optimistic update + backend seek
 * - Drag-to-seek: Preview position while dragging, seek on release (no scrubbing)
 * - Visual feedback: hover and dragging states
 */

import React, { useState, useRef, useCallback, useEffect } from 'react';
import { formatDuration } from '../../lib/utils';
import { useSeekBar } from '../../hooks/useSeekBar';
import { useInterpolatedProgress } from '../../hooks/useInterpolatedProgress';

export function ProgressBar() {
  // Smooth 60fps animation between backend updates; snaps immediately on seek.
  const { progress, duration } = useInterpolatedProgress();
  const { handleSeek } = useSeekBar();

  // State for drag interactions
  const [isDragging, setIsDragging] = useState(false);
  const [dragPosition, setDragPosition] = useState<number | null>(null);
  const [isHovering, setIsHovering] = useState(false);
  const progressBarRef = useRef<HTMLDivElement>(null);

  // Deduplication: prevent click handler from firing after mouseUp completes
  const lastSeekTimeRef = useRef<number>(0);

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

    // Mark that we just seeked (prevents duplicate click handler from firing)
    lastSeekTimeRef.current = performance.now();

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

    // Prevent duplicate seek when click fires immediately after mouseUp (within 50ms)
    const timeSinceLastSeek = performance.now() - lastSeekTimeRef.current;
    if (timeSinceLastSeek < 50) {
      return;
    }

    e.stopPropagation();
    const position = calculatePosition(e.clientX);
    const seekTimeSeconds = Math.min((position / 100) * duration, Math.max(0, duration - 0.1));
    handleSeek(seekTimeSeconds);
    lastSeekTimeRef.current = performance.now();
  }, [isDragging, calculatePosition, duration, handleSeek]);

  return (
    <div className="flex items-center gap-3 w-full">
      {/* Current time */}
      <span className="text-xs text-muted-foreground font-mono min-w-[40px] text-right">
        {formatDuration(currentTimeSeconds)}
      </span>

      {/* Progress bar — outer div is the hit area (h-4), inner div is the visual track (h-[3px]) */}
      <div
        ref={progressBarRef}
        className="relative flex-1 h-4 flex items-center select-none"
        style={{
          cursor: isDragging ? 'grabbing' : 'pointer',
          userSelect: 'none'
        }}
        onClick={handleClick}
        onMouseDown={handleMouseDown}
        onMouseEnter={() => setIsHovering(true)}
        onMouseLeave={() => setIsHovering(false)}
      >
        {/* Visual track — thin line, overflow-hidden clips fill div only */}
        <div className="relative w-full h-1.5 bg-muted rounded-full overflow-hidden">
          {/* Filled progress */}
          <div
            className="absolute inset-y-0 left-0 bg-primary rounded-full"
            style={{
              width: `${Math.max(0, Math.min(100, displayProgress))}%`,
              maxWidth: '100%',
              transition: 'none',
              opacity: isDragging ? 0.8 : 1,
            }}
          />
        </div>

        {/* Ball — always in DOM, fades in/out via opacity to avoid pop-in */}
        <div
          className="absolute top-1/2 w-2.5 h-2.5 bg-primary rounded-full pointer-events-none"
          style={{
            left: `${Math.max(0, Math.min(100, displayProgress))}%`,
            transform: `translate(-50%, -50%)${isDragging ? ' scale(1.2)' : ''}`,
            opacity: isHovering || isDragging ? 1 : 0,
            transition: 'opacity 150ms ease, transform 100ms ease',
          }}
        />
      </div>

      {/* Total duration */}
      <span className="text-xs text-muted-foreground font-mono min-w-[40px]">
        {formatDuration(duration)}
      </span>
    </div>
  );
}
