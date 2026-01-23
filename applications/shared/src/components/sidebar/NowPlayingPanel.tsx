'use client';

import { useTranslation } from 'react-i18next';
import { Music, Heart } from 'lucide-react';
import { TrackItem } from './TrackItem';
import { cn } from '../../lib/utils';

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
  isPlaying,
  canCreatePlaylists,
  onTrackClick,
  onAddToPlaylist,
}: NowPlayingPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="p-4">
      <div className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
        {t('sidebar.nowPlaying')}
      </div>

      <div className="h-12 flex items-center gap-2">
        <div className="flex-1 min-w-0">
          {currentTrack ? (
            <TrackItem
              key={String(currentTrack.id)}
              trackId={currentTrack.id}
              title={currentTrack.title}
              artist={currentTrack.artist}
              coverArtPath={currentTrack.coverArtPath}
              album={currentTrack.album}
              isLarge
              isPlaying={isPlaying}
              showEqualizer
              onClick={onTrackClick}
            />
          ) : (
            <div className="flex items-center gap-3 text-muted-foreground h-12">
              <div className="w-12 h-12 bg-muted rounded flex items-center justify-center">
                <Music className="w-6 h-6 opacity-50" />
              </div>
              <span className="text-sm">{t('sidebar.noTrackPlaying')}</span>
            </div>
          )}
        </div>
        <button
          onClick={(e) => {
            e.stopPropagation();
            onAddToPlaylist?.();
          }}
          disabled={!currentTrack || !canCreatePlaylists}
          className={cn(
            'p-2 transition-opacity text-muted-foreground flex-shrink-0 relative z-10',
            currentTrack && canCreatePlaylists
              ? 'hover:opacity-[var(--hover-text-opacity)] hover:bg-foreground/10 rounded-md'
              : 'opacity-[var(--disabled-opacity)] cursor-not-allowed'
          )}
          title={
            canCreatePlaylists
              ? t('playlist.addToPlaylist', 'Add to Playlist')
              : t('settings.demoDisabled', 'Available in desktop app')
          }
        >
          <Heart className="w-4 h-4" />
        </button>
      </div>
    </div>
  );
}
