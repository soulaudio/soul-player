'use client';

import { useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import type { QueueTrack } from '../../contexts/PlayerCommandsContext';
import { TrackItem } from './TrackItem';

export interface QueueSectionProps {
  queue: QueueTrack[];
  currentTrackId?: string | number;
  scrollRef?: React.RefObject<HTMLDivElement>;
  onTrackClick: (originalIndex: number) => void;
}

export function QueueSection({ queue, currentTrackId, scrollRef, onTrackClick }: QueueSectionProps) {
  const { t } = useTranslation();
  const defaultScrollRef = useRef<HTMLDivElement>(null);
  const actualScrollRef = scrollRef || defaultScrollRef;

  // Filter out current track from queue, reverse so items closest to now playing are at bottom
  // Preserve original indices to handle duplicate tracks correctly
  const displayQueue = useMemo(() => {
    return queue
      .map((track, index) => ({ track, originalIndex: index }))
      .filter(({ track }) => String(track.trackId) !== String(currentTrackId))
      .reverse();
  }, [queue, currentTrackId]);

  if (displayQueue.length === 0) {
    return <div className="flex-1 min-h-0" />;
  }

  return (
    <div className="flex flex-col flex-1 min-h-0">
      <div className="px-4 py-2 text-xs font-medium text-muted-foreground uppercase tracking-wider flex-shrink-0">
        {t('sidebar.queue')}
      </div>
      <div
        ref={actualScrollRef}
        className="flex-1 overflow-y-auto px-4 pb-2 queue-scrollbar min-h-0"
      >
        <div className="flex flex-col justify-end min-h-full gap-1">
          {displayQueue.map(({ track, originalIndex }) => (
            <TrackItem
              key={`queue-${originalIndex}`}
              trackId={track.trackId}
              title={track.title}
              artist={track.artist}
              coverArtPath={track.coverArtPath}
              album={track.album ?? undefined}
              onClick={() => onTrackClick(originalIndex)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
