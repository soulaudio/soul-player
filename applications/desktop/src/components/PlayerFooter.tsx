import { usePlaybackEvents } from '@soul-player/shared/hooks/usePlaybackEvents';
import { TrackInfo } from './player/TrackInfo';
import { PlayerControls } from './player/PlayerControls';
import { ProgressBar } from './player/ProgressBar';
import { VolumeControl } from './player/VolumeControl';
import { ShuffleRepeatControls } from './player/ShuffleRepeatControls';

/**
 * Main player footer component with playback controls and progress bar.
 * Subscribes to Tauri playback events and updates the player store.
 */
export function PlayerFooter() {
  // Subscribe to playback events from Tauri backend
  usePlaybackEvents();

  return (
    <div className="border-t bg-card">
      {/* Main controls row */}
      <div className="p-4">
        <div className="grid grid-cols-3 items-center gap-4">
          {/* Left: Track info */}
          <div className="flex items-center min-w-0">
            <TrackInfo />
          </div>

          {/* Center: Playback controls */}
          <div className="flex flex-col items-center gap-2">
            <PlayerControls />
          </div>

          {/* Right: Shuffle, Repeat, and Volume controls */}
          <div className="flex items-center justify-end gap-2">
            <ShuffleRepeatControls />
            <VolumeControl />
          </div>
        </div>
      </div>

      {/* Progress bar row */}
      <div className="px-4 pb-3">
        <ProgressBar />
      </div>
    </div>
  );
}
