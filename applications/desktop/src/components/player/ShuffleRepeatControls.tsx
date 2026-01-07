import { usePlayerStore } from '@soul-player/shared/stores/player';
import { Shuffle, Repeat, Repeat1 } from 'lucide-react';

export function ShuffleRepeatControls() {
  const { shuffleEnabled, repeatMode, toggleShuffle, setRepeatMode } = usePlayerStore();

  const handleShuffleToggle = () => {
    toggleShuffle();
    // TODO: Call backend command when implemented
    // playerCommands.setShuffle(shuffleEnabled ? 'off' : 'random');
  };

  const handleRepeatToggle = () => {
    // Cycle through: off → all → one → off
    const nextMode = repeatMode === 'off' ? 'all' : repeatMode === 'all' ? 'one' : 'off';
    setRepeatMode(nextMode);
    // TODO: Call backend command when implemented
    // playerCommands.setRepeat(nextMode);
  };

  return (
    <div className="flex items-center gap-1">
      {/* Shuffle button */}
      <button
        onClick={handleShuffleToggle}
        className={`p-2 rounded-full transition-colors ${
          shuffleEnabled
            ? 'text-primary hover:bg-accent'
            : 'text-muted-foreground hover:bg-accent hover:text-foreground'
        }`}
        aria-label={shuffleEnabled ? 'Disable shuffle' : 'Enable shuffle'}
        title={shuffleEnabled ? 'Shuffle enabled' : 'Shuffle disabled'}
      >
        <Shuffle className="w-4 h-4" />
      </button>

      {/* Repeat button */}
      <button
        onClick={handleRepeatToggle}
        className={`p-2 rounded-full transition-colors ${
          repeatMode !== 'off'
            ? 'text-primary hover:bg-accent'
            : 'text-muted-foreground hover:bg-accent hover:text-foreground'
        }`}
        aria-label={`Repeat: ${repeatMode}`}
        title={`Repeat: ${repeatMode}`}
      >
        {repeatMode === 'one' ? (
          <Repeat1 className="w-4 h-4" />
        ) : (
          <Repeat className="w-4 h-4" />
        )}
      </button>
    </div>
  );
}
