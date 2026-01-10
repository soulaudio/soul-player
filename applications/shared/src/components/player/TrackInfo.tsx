import { useNavigate } from 'react-router-dom';
import { usePlayerStore } from '../../stores/player';
import { ArtworkImage } from '../ArtworkImage';
import { Music, ChevronUp } from 'lucide-react';

export function TrackInfo() {
  const navigate = useNavigate();
  const { currentTrack } = usePlayerStore();

  const handleClick = () => {
    if (currentTrack) {
      navigate('/now-playing');
    }
  };

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
    <button
      onClick={handleClick}
      className="flex items-center gap-3 min-w-0 group hover:bg-accent/50 rounded-lg p-1 -m-1 transition-colors"
    >
      {/* Album art */}
      <div className="flex-shrink-0 w-14 h-14 bg-gradient-player rounded overflow-hidden relative">
        <ArtworkImage
          trackId={currentTrack.id}
          coverArtPath={currentTrack.coverArtPath}
          alt={currentTrack.album || 'Album art'}
          className="w-full h-full object-cover"
          fallbackClassName="w-full h-full flex items-center justify-center"
        />
        <div className="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
          <ChevronUp className="w-6 h-6 text-white" />
        </div>
      </div>

      {/* Track info */}
      <div className="flex flex-col min-w-0 text-left">
        <div className="text-sm font-medium truncate group-hover:text-primary transition-colors" title={currentTrack.title}>
          {currentTrack.title}
        </div>
        <div className="text-xs text-muted-foreground truncate" title={currentTrack.artist}>
          {currentTrack.artist || 'Unknown Artist'}
        </div>
      </div>
    </button>
  );
}
