'use client';

import { ArtworkImage } from '../ArtworkImage';
import { cn } from '../../lib/utils';

export interface TrackItemProps {
  trackId: string | number;
  title: string;
  artist: string;
  coverArtPath?: string;
  album?: string;
  isLarge?: boolean;
  isPlaying?: boolean;
  showEqualizer?: boolean;
  onClick?: () => void;
}

export function TrackItem({
  trackId,
  title,
  artist,
  coverArtPath,
  album,
  isLarge,
  isPlaying,
  showEqualizer,
  onClick,
}: TrackItemProps) {
  return (
    <div
      className={cn(
        'flex items-center group/track',
        isLarge ? 'gap-3' : 'gap-2',
        onClick && 'cursor-pointer'
      )}
      onClick={onClick}
    >
      <div
        className={cn(
          'bg-muted rounded overflow-hidden flex-shrink-0 relative',
          isLarge ? 'w-12 h-12' : 'w-8 h-8'
        )}
      >
        <ArtworkImage
          trackId={trackId}
          coverArtPath={coverArtPath}
          alt={album || 'Album art'}
          className="w-full h-full object-cover"
          fallbackClassName="w-full h-full flex items-center justify-center"
        />
        {showEqualizer && isPlaying && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/30">
            <div className="flex items-end gap-[2px] h-3">
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite]" />
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite_0.2s]" />
              <span className="w-[3px] bg-white rounded-full origin-bottom h-full animate-[equalize_0.8s_ease-in-out_infinite_0.4s]" />
            </div>
          </div>
        )}
      </div>
      <div
        className={cn(
          'flex-1 min-w-0',
          onClick && 'group-hover/track:text-foreground'
        )}
      >
        <div className="text-sm truncate">{title}</div>
        <div
          className={cn(
            'text-xs truncate transition-colors',
            onClick
              ? 'text-muted-foreground/70 group-hover/track:text-muted-foreground'
              : 'text-muted-foreground'
          )}
        >
          {artist}
        </div>
      </div>
    </div>
  );
}
