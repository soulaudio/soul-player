'use client';

import { useTranslation } from 'react-i18next';
import { Music, Heart } from 'lucide-react';
import { ArtworkImage } from '../ArtworkImage';

export interface CurrentTrackInfo {
  id: string | number;
  title: string;
  artist: string;
  album: string;
  coverArtPath?: string;
}

export interface NowPlayingPanelProps {
  currentTrack: CurrentTrackInfo | null;
  isPlaying: boolean;
  canCreatePlaylists: boolean;
  onTrackClick?: () => void;
  onAddToPlaylist?: () => void;
}

export function NowPlayingPanel({
  currentTrack,
  canCreatePlaylists,
  onTrackClick,
  onAddToPlaylist,
}: NowPlayingPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="p-4" data-testid="now-playing-panel">
      <div className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
        {t('sidebar.nowPlaying')}
      </div>
      {currentTrack ? (
        <div
          className="flex items-center gap-2.5 min-w-0 rounded-lg px-1.5 py-1 -mx-1.5 hover:bg-foreground/[0.06] transition-colors cursor-pointer"
          onClick={onTrackClick}
          data-testid="now-playing-title"
        >
          <div className="w-10 h-10 rounded-md overflow-hidden flex-shrink-0">
            <ArtworkImage
              trackId={currentTrack.id}
              coverArtPath={currentTrack.coverArtPath}
              alt={currentTrack.title}
              className="w-full h-full object-cover"
              fallbackClassName="w-full h-full flex items-center justify-center bg-muted"
              fallbackIconSize="sm"
            />
          </div>
          <div className="flex flex-col min-w-0 flex-1">
            <span className="text-sm font-semibold truncate text-foreground hover:underline">
              {currentTrack.title}
            </span>
            <span className="text-xs text-muted-foreground truncate hover:underline">
              {currentTrack.artist || 'Unknown Artist'}
            </span>
          </div>
          {canCreatePlaylists && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onAddToPlaylist?.();
              }}
              className="ml-auto p-1.5 text-muted-foreground hover:text-foreground transition-colors flex-shrink-0"
              title={t('playlist.addToPlaylist', 'Add to Playlist')}
            >
              <Heart className="w-4 h-4" />
            </button>
          )}
        </div>
      ) : (
        <div className="flex items-center gap-3 text-muted-foreground h-12">
          <div className="w-12 h-12 bg-muted rounded flex items-center justify-center">
            <Music className="w-6 h-6 opacity-50" />
          </div>
          <span className="text-sm">{t('sidebar.noTrackPlaying')}</span>
        </div>
      )}
    </div>
  );
}
