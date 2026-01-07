import { usePlayerStore } from '../../stores/player';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { Shuffle, Repeat, Repeat1 } from 'lucide-react';

export function ShuffleRepeatControls() {
  const { shuffleEnabled, repeatMode, toggleShuffle, setRepeatMode } = usePlayerStore();
  const commands = usePlayerCommands();

  const handleShuffleToggle = async () => {
    const newValue = !shuffleEnabled;
    toggleShuffle();
    try {
      await commands.setShuffle(newValue);
    } catch (error) {
      console.error('[ShuffleRepeatControls] Set shuffle failed:', error);
      // Revert on error
      toggleShuffle();
    }
  };

  const handleRepeatToggle = async () => {
    // Cycle through: off → all → one → off
    const nextMode = repeatMode === 'off' ? 'all' : repeatMode === 'all' ? 'one' : 'off';
    setRepeatMode(nextMode);
    try {
      await commands.setRepeatMode(nextMode);
    } catch (error) {
      console.error('[ShuffleRepeatControls] Set repeat mode failed:', error);
      // Revert on error
      const prevMode = nextMode === 'off' ? 'one' : nextMode === 'all' ? 'off' : 'all';
      setRepeatMode(prevMode);
    }
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
