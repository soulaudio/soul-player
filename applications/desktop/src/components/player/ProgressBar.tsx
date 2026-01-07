import { usePlayerStore } from '@soul-player/shared/stores/player';
import { formatDuration } from '@soul-player/shared/lib/utils';
import { useSeekBar } from '@soul-player/shared/hooks/useSeekBar';

export function ProgressBar() {
  const { progress, duration } = usePlayerStore();
  const { isDragging, seekPosition, handleSeekStart, handleSeekChange, handleSeekEnd } = useSeekBar();

  // Use seek position while dragging, otherwise use store progress
  const displayProgress = isDragging && seekPosition !== null
    ? (seekPosition / duration) * 100
    : progress;

  // Calculate current time in seconds
  const currentTimeSeconds = isDragging && seekPosition !== null
    ? seekPosition
    : (progress / 100) * duration;

  /**
   * Handle click on progress bar to seek
   */
  const handleProgressClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const width = rect.width;
    const percentage = (clickX / width) * 100;
    const newPosition = (percentage / 100) * duration;

    // Trigger immediate seek
    handleSeekStart(newPosition);
    handleSeekEnd();
  };

  /**
   * Handle mouse down to start dragging
   */
  const handleMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    e.preventDefault();
    const rect = e.currentTarget.getBoundingClientRect();
    const clickX = e.clientX - rect.left;
    const width = rect.width;
    const percentage = (clickX / width) * 100;
    const newPosition = (percentage / 100) * duration;

    handleSeekStart(newPosition);

    // Add global mouse move and mouse up listeners
    const handleMouseMove = (moveEvent: MouseEvent) => {
      const moveX = moveEvent.clientX - rect.left;
      const movePercentage = Math.max(0, Math.min(100, (moveX / width) * 100));
      const movePosition = (movePercentage / 100) * duration;
      handleSeekChange(movePosition);
    };

    const handleMouseUp = () => {
      handleSeekEnd();
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  };

  return (
    <div className="flex items-center gap-3 w-full">
      {/* Current time */}
      <span className="text-xs text-muted-foreground font-mono min-w-[40px] text-right">
        {formatDuration(currentTimeSeconds)}
      </span>

      {/* Progress bar */}
      <div
        className="relative flex-1 h-2 bg-muted rounded-full cursor-pointer group"
        onClick={handleProgressClick}
        onMouseDown={handleMouseDown}
      >
        {/* Filled progress */}
        <div
          className="absolute inset-y-0 left-0 bg-primary rounded-full transition-all duration-100"
          style={{ width: `${Math.max(0, Math.min(100, displayProgress))}%` }}
        />

        {/* Seek handle (visible on hover or while dragging) */}
        <div
          className={`absolute top-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full shadow-lg transition-opacity ${
            isDragging ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
          }`}
          style={{ left: `${Math.max(0, Math.min(100, displayProgress))}%`, transform: 'translate(-50%, -50%)' }}
        />
      </div>

      {/* Total duration */}
      <span className="text-xs text-muted-foreground font-mono min-w-[40px]">
        {formatDuration(duration)}
      </span>
    </div>
  );
}
