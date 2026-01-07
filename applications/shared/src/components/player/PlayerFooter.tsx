import { TrackInfo } from './TrackInfo';
import { PlayerControls } from './PlayerControls';
import { ProgressBar } from './ProgressBar';
import { VolumeControl } from './VolumeControl';
import { ShuffleRepeatControls } from './ShuffleRepeatControls';

/**
 * Main player footer component with playback controls and progress bar.
 * Note: Event subscriptions are handled by platform-specific providers
 * (TauriPlayerCommandsProvider for desktop, bridge.ts for demo)
 */
export function PlayerFooter() {

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
