import { usePlayerStore } from '../../stores/player';
import { Music } from 'lucide-react';

export function TrackInfo() {
  const { currentTrack } = usePlayerStore();

  if (!currentTrack) {
    return (
      <div className="flex items-center gap-3 min-w-0">
        <div className="flex-shrink-0 w-14 h-14 bg-muted rounded flex items-center justify-center">
          <Music className="w-6 h-6 text-muted-foreground" />
        </div>
        <div className="flex flex-col min-w-0">
          <div className="text-sm font-medium text-muted-foreground">No track playing</div>
          <div className="text-xs text-muted-foreground">Soul Player</div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-3 min-w-0">
      {/* Album art placeholder */}
      <div className="flex-shrink-0 w-14 h-14 bg-gradient-player rounded flex items-center justify-center">
        <Music className="w-6 h-6 text-primary-foreground" />
      </div>

      {/* Track info */}
      <div className="flex flex-col min-w-0">
        <div className="text-sm font-medium truncate" title={currentTrack.title}>
          {currentTrack.title}
        </div>
        <div className="text-xs text-muted-foreground truncate" title={currentTrack.artist}>
          {currentTrack.artist || 'Unknown Artist'}
        </div>
      </div>
    </div>
  );
}
