import { usePlayerModes } from '../../stores/player';
import { usePlayerCommands } from '../../contexts/PlayerCommandsContext';
import { Shuffle, Repeat, Repeat1, Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { debug } from '../../utils/debug';

interface ShuffleRepeatControlsProps {
  /** Callback when the "Add to Playlist" button is clicked */
  onAddToPlaylist?: () => void;
}

export function ShuffleRepeatControls({ onAddToPlaylist }: ShuffleRepeatControlsProps) {
  const { t } = useTranslation();
  const { shuffleMode, repeatMode, setShuffleMode, setRepeatMode } = usePlayerModes();
  const commands = usePlayerCommands();

  const handleShuffleToggle = async () => {
    debug.log('[ShuffleRepeatControls] Current shuffle mode:', shuffleMode);
    try {
      const newMode = await commands.cycleShuffle();
      debug.log('[ShuffleRepeatControls] New shuffle mode from backend:', newMode);
      setShuffleMode(newMode);
    } catch (error) {
      debug.error('[ShuffleRepeatControls] Cycle shuffle failed:', error);
    }
  };

  const handleRepeatToggle = async () => {
    // Cycle through: off → all → one → off
    debug.log('[ShuffleRepeatControls] Current repeat mode:', repeatMode);
    const nextMode = repeatMode === 'off' ? 'all' : repeatMode === 'all' ? 'one' : 'off';
    debug.log('[ShuffleRepeatControls] Cycling to:', nextMode);
    setRepeatMode(nextMode);
    try {
      await commands.setRepeatMode(nextMode);
      debug.log('[ShuffleRepeatControls] Repeat mode set successfully');
    } catch (error) {
      debug.error('[ShuffleRepeatControls] Set repeat mode failed:', error);
      // Revert on error
      const prevMode = nextMode === 'off' ? 'one' : nextMode === 'all' ? 'off' : 'all';
      setRepeatMode(prevMode);
    }
  };

  const getShuffleTitle = () => {
    switch (shuffleMode) {
      case 'off':
        return t('playback.shuffleOff', 'Shuffle: Off');
      case 'random':
        return t('playback.shuffleRandom', 'Shuffle: Random');
      case 'smart':
        return t('playback.shuffleSmart', 'Shuffle: Smart');
    }
  };

  const getRepeatTitle = () => {
    switch (repeatMode) {
      case 'off':
        return t('playback.repeatOff');
      case 'all':
        return t('playback.repeatAll');
      case 'one':
        return t('playback.repeatOne');
      default:
        return t('playback.repeatOff');
    }
  };

  return (
    <div className="flex items-center gap-1">
      {/* Shuffle button */}
      <button
        onClick={handleShuffleToggle}
        className={`p-2 rounded-full transition-colors relative ${
          shuffleMode !== 'off'
            ? 'text-primary hover:bg-foreground/[var(--hover-bg-opacity)]'
            : 'text-muted-foreground hover:bg-foreground/[var(--hover-bg-opacity)] hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]'
        }`}
        aria-label={getShuffleTitle()}
        title={getShuffleTitle()}
      >
        <Shuffle className="w-4 h-4" />
        {shuffleMode === 'random' && (
          <span className="absolute -top-0.5 -right-0.5 text-[8px] font-bold text-primary">R</span>
        )}
        {shuffleMode === 'smart' && (
          <span className="absolute -top-0.5 -right-0.5 text-[8px] font-bold text-primary">S</span>
        )}
      </button>

      {/* Repeat button */}
      <button
        onClick={handleRepeatToggle}
        className={`p-2 rounded-full transition-colors ${
          repeatMode !== 'off'
            ? 'text-primary hover:bg-foreground/[var(--hover-bg-opacity)]'
            : 'text-muted-foreground hover:bg-foreground/[var(--hover-bg-opacity)] hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]'
        }`}
        aria-label={getRepeatTitle()}
        title={getRepeatTitle()}
      >
        {repeatMode === 'one' ? (
          <Repeat1 className="w-4 h-4" />
        ) : (
          <Repeat className="w-4 h-4" />
        )}
      </button>

      {/* Add to Playlist button */}
      {onAddToPlaylist && (
        <button
          onClick={onAddToPlaylist}
          className="p-2 rounded-full transition-colors text-muted-foreground hover:bg-foreground/[var(--hover-bg-opacity)] hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]"
          aria-label={t('playlist.addToPlaylist', 'Add to Playlist')}
          title={t('playlist.addToPlaylist', 'Add to Playlist')}
        >
          <Plus className="w-4 h-4" />
        </button>
      )}
    </div>
  );
}
