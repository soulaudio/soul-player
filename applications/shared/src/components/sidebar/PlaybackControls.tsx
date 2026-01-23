'use client';

import { Play, Pause, SkipBack, SkipForward, Shuffle, Repeat, Repeat1 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { cn } from '../../lib/utils';

export type ShuffleMode = 'off' | 'random' | 'smart';
export type RepeatMode = 'off' | 'all' | 'one';

export interface PlaybackControlsProps {
  isPlaying: boolean;
  hasCurrentTrack: boolean;
  shuffleMode: ShuffleMode;
  repeatMode: RepeatMode;
  onPlayPause: () => void;
  onPrevious: () => void;
  onNext: () => void;
  onShuffleToggle: () => void;
  onRepeatToggle: () => void;
}

export function PlaybackControls({
  isPlaying,
  hasCurrentTrack,
  shuffleMode,
  repeatMode,
  onPlayPause,
  onPrevious,
  onNext,
  onShuffleToggle,
  onRepeatToggle,
}: PlaybackControlsProps) {
  const { t } = useTranslation();

  const getShuffleTitle = () => {
    if (shuffleMode === 'off') return t('playback.shuffle.off', 'Shuffle: Off');
    if (shuffleMode === 'random') return t('playback.shuffle.random', 'Shuffle: Random');
    return t('playback.shuffle.smart', 'Shuffle: Smart');
  };

  return (
    <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-1">
      {/* Left group */}
      <div className="flex items-center justify-end gap-1">
        <button
          onClick={onShuffleToggle}
          disabled={!hasCurrentTrack}
          className={cn(
            'p-1.5 transition-opacity relative',
            !hasCurrentTrack && 'opacity-50 cursor-not-allowed',
            shuffleMode !== 'off'
              ? 'text-primary'
              : 'text-muted-foreground hover:opacity-[var(--hover-text-opacity)] disabled:hover:opacity-[var(--disabled-opacity)]'
          )}
          title={getShuffleTitle()}
        >
          <Shuffle className="w-3.5 h-3.5" />
          {shuffleMode === 'random' && (
            <span className="absolute -top-0.5 -right-0.5 text-[7px] font-bold text-primary">
              R
            </span>
          )}
          {shuffleMode === 'smart' && (
            <span className="absolute -top-0.5 -right-0.5 text-[7px] font-bold text-primary">
              S
            </span>
          )}
        </button>
        <button
          onClick={onPrevious}
          disabled={!hasCurrentTrack}
          className={cn(
            'p-1.5 text-muted-foreground transition-opacity',
            hasCurrentTrack
              ? 'hover:opacity-[var(--hover-text-opacity)]'
              : 'opacity-50 cursor-not-allowed'
          )}
        >
          <SkipBack className="w-4 h-4" />
        </button>
      </div>

      {/* Center - Play/Pause */}
      <button
        onClick={onPlayPause}
        disabled={!hasCurrentTrack}
        className={cn(
          'w-8 h-8 bg-primary text-primary-foreground rounded-full transition-all duration-[var(--transition-duration)] flex items-center justify-center',
          hasCurrentTrack
            ? 'hover:opacity-[var(--hover-button-opacity)]'
            : 'opacity-[var(--disabled-opacity)] cursor-not-allowed'
        )}
      >
        {isPlaying ? (
          <Pause className="w-4 h-4" />
        ) : (
          <Play className="w-4 h-4 translate-x-[1px]" />
        )}
      </button>

      {/* Right group */}
      <div className="flex items-center justify-start gap-1">
        <button
          onClick={onNext}
          disabled={!hasCurrentTrack}
          className={cn(
            'p-1.5 text-muted-foreground transition-opacity',
            hasCurrentTrack
              ? 'hover:opacity-[var(--hover-text-opacity)]'
              : 'opacity-50 cursor-not-allowed'
          )}
        >
          <SkipForward className="w-4 h-4" />
        </button>
        <button
          onClick={onRepeatToggle}
          disabled={!hasCurrentTrack}
          className={cn(
            'p-1.5 transition-opacity',
            !hasCurrentTrack && 'opacity-50 cursor-not-allowed',
            repeatMode !== 'off'
              ? 'text-primary'
              : 'text-muted-foreground hover:opacity-[var(--hover-text-opacity)] disabled:hover:opacity-[var(--disabled-opacity)]'
          )}
        >
          {repeatMode === 'one' ? (
            <Repeat1 className="w-3.5 h-3.5" />
          ) : (
            <Repeat className="w-3.5 h-3.5" />
          )}
        </button>
      </div>
    </div>
  );
}
