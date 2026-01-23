'use client';

import { cn } from '../../lib/utils';

export interface ProgressBarProps {
  progress: number;
  duration: number;
  hasCurrentTrack: boolean;
  onSeek?: (e: React.MouseEvent<HTMLDivElement>) => void;
}

export function ProgressBar({ progress, duration, hasCurrentTrack, onSeek }: ProgressBarProps) {
  const formatTime = (seconds: number) => {
    if (!seconds || !isFinite(seconds)) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  const currentPositionSeconds = duration > 0 ? (progress / 100) * duration : 0;

  return (
    <div>
      <div
        className={cn(
          'py-2 -my-2',
          hasCurrentTrack ? 'cursor-pointer' : 'cursor-default'
        )}
        onClick={hasCurrentTrack ? onSeek : undefined}
      >
        <div
          className={cn(
            'h-1.5 bg-muted rounded-full overflow-hidden',
            !hasCurrentTrack && 'opacity-50'
          )}
        >
          <div
            className="h-full bg-primary rounded-full transition-[width] duration-150"
            style={{ width: `${progress}%` }}
          />
        </div>
      </div>
      <div className="flex justify-between mt-1 text-[10px] text-muted-foreground font-mono">
        <span>{formatTime(currentPositionSeconds)}</span>
        <span>{formatTime(duration)}</span>
      </div>
    </div>
  );
}
