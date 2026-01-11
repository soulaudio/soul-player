'use client';

import { useState } from 'react';
import { Play, Pause, Music, AlertTriangle } from 'lucide-react';
import { usePlayerStore } from '../stores/player';
import { usePlayerCommands } from '../contexts/PlayerCommandsContext';
import type { QueueTrack } from '../contexts/PlayerCommandsContext';
import { Tooltip } from './ui/Tooltip';
import { TrackQualityBadge } from './TrackQualityBadge';
import { SourceIndicator } from './SourceIndicator';

export type SourceType = 'local' | 'server' | 'cached';

export interface Track {
  id: number | string;
  title: string;
  artist?: string;
  album?: string;
  duration?: number;
  trackNumber?: number;
  /** Whether the track is available (file exists). Defaults to true. */
  isAvailable?: boolean;
  /** Audio file format (e.g., 'flac', 'mp3', 'aac') */
  format?: string;
  /** Bitrate in kbps (for lossy formats) */
  bitrate?: number;
  /** Sample rate in Hz */
  sampleRate?: number;
  /** Number of audio channels */
  channels?: number;
  /** Source type: local file, server stream, or cached from server */
  sourceType?: SourceType;
  /** Name of the source (e.g., server name or folder name) */
  sourceName?: string;
  /** Whether the source is currently online (for server sources) */
  sourceOnline?: boolean;
}

interface TrackListProps {
  tracks: Track[];
  /** Callback to build queue from tracks - platform-specific implementation */
  buildQueue: (tracks: Track[], clickedTrack: Track, clickedIndex: number) => QueueTrack[];
  onTrackAction?: (track: Track) => void;
  /** Optional menu component to render for each track */
  renderMenu?: (track: Track) => React.ReactNode;
}

function formatDuration(seconds?: number): string {
  if (!seconds) return '--:--';
  const minutes = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

export function TrackList({ tracks, buildQueue, onTrackAction, renderMenu }: TrackListProps) {
  const [hoveredTrackId, setHoveredTrackId] = useState<number | string | null>(null);
  const { currentTrack, isPlaying } = usePlayerStore();
  const commands = usePlayerCommands();

  const handlePlay = async (track: Track, index: number) => {
    // Use platform-specific queue building logic
    const queue = buildQueue(tracks, track, index);

    console.log('[TrackList] Playing queue with', queue.length, 'tracks');

    try {
      await commands.playQueue(queue, 0);
      onTrackAction?.(track);
    } catch (error) {
      console.error('[TrackList] Failed to play track:', error);
    }
  };

  const handlePause = async () => {
    try {
      await commands.pausePlayback();
    } catch (error) {
      console.error('[TrackList] Failed to pause:', error);
    }
  };

  if (tracks.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Music className="w-12 h-12 mb-4 opacity-50" />
        <p>No tracks found</p>
        <p className="text-sm mt-1">Add music to get started</p>
      </div>
    );
  }

  return (
    <div className="border rounded-lg overflow-hidden">
      <div className="bg-muted/50">
        <div className="grid grid-cols-[40px_minmax(200px,1fr)_minmax(120px,180px)_minmax(120px,180px)_90px_70px_80px_40px] gap-4 px-4 py-2 text-sm font-medium text-muted-foreground">
          <Tooltip content="Track number" position="top" delay={700}>
            <div>#</div>
          </Tooltip>
          <Tooltip content="Track title" position="top" delay={700}>
            <div>Title</div>
          </Tooltip>
          <Tooltip content="Artist name" position="top" delay={700}>
            <div>Artist</div>
          </Tooltip>
          <Tooltip content="Album title" position="top" delay={700}>
            <div>Album</div>
          </Tooltip>
          <Tooltip content="Audio format and quality" position="top" delay={700}>
            <div>Format</div>
          </Tooltip>
          <Tooltip content="Track source" position="top" delay={700}>
            <div>Source</div>
          </Tooltip>
          <Tooltip content="Track duration" position="top" delay={700}>
            <div className="text-right">Duration</div>
          </Tooltip>
          <div></div>
        </div>
      </div>
      <div>
        {tracks.map((track, index) => {
          const trackId = String(track.id);
          const isCurrentTrack = String(currentTrack?.id) === trackId;
          const showPauseButton = isCurrentTrack && isPlaying;
          const isUnavailable = track.isAvailable === false;

          return (
            <div
              key={trackId}
              className={`grid grid-cols-[40px_minmax(200px,1fr)_minmax(120px,180px)_minmax(120px,180px)_90px_70px_80px_40px] gap-4 px-4 py-3 hover:bg-accent/50 border-b last:border-b-0 transition-colors group ${
                isCurrentTrack ? 'bg-accent/30' : ''
              } ${isUnavailable ? 'opacity-60' : ''}`}
              onMouseEnter={() => setHoveredTrackId(trackId)}
              onMouseLeave={() => setHoveredTrackId(null)}
              onDoubleClick={() => !isUnavailable && handlePlay(track, index)}
            >
              <div className="flex items-center justify-center">
                {isUnavailable ? (
                  <Tooltip content="File not found" position="right">
                    <div className="w-8 h-8 flex items-center justify-center text-amber-500">
                      <AlertTriangle className="w-4 h-4" />
                    </div>
                  </Tooltip>
                ) : hoveredTrackId === trackId || isCurrentTrack ? (
                  <button
                    onClick={() => (showPauseButton ? handlePause() : handlePlay(track, index))}
                    className="w-8 h-8 flex items-center justify-center rounded hover:bg-primary/10 transition-colors"
                    aria-label={showPauseButton ? 'Pause' : 'Play'}
                  >
                    {showPauseButton ? (
                      <Pause className="w-4 h-4" fill="currentColor" />
                    ) : (
                      <Play className="w-4 h-4" fill="currentColor" />
                    )}
                  </button>
                ) : (
                  <div className="w-8 h-8 flex items-center justify-center text-muted-foreground text-sm">
                    {track.trackNumber || index + 1}
                  </div>
                )}
              </div>
              <div className="flex flex-col justify-center min-w-0">
                <div className={`truncate ${isCurrentTrack ? 'text-primary font-medium' : ''} ${isUnavailable ? 'line-through' : ''}`}>
                  {track.title}
                </div>
              </div>
              <div className="flex items-center text-sm text-muted-foreground truncate">
                {track.artist || 'Unknown Artist'}
              </div>
              <div className="flex items-center text-sm text-muted-foreground truncate">
                {track.album || '—'}
              </div>
              <div className="flex items-center">
                {track.format ? (
                  <TrackQualityBadge
                    format={track.format}
                    bitrate={track.bitrate}
                    sampleRate={track.sampleRate}
                    channels={track.channels}
                  />
                ) : (
                  <span className="text-xs text-muted-foreground">—</span>
                )}
              </div>
              <div className="flex items-center">
                {track.sourceType ? (
                  <SourceIndicator
                    sourceType={track.sourceType}
                    sourceName={track.sourceName}
                    isOnline={track.sourceOnline}
                    size="sm"
                  />
                ) : (
                  <span className="text-xs text-muted-foreground">—</span>
                )}
              </div>
              <div className="flex items-center justify-end text-sm text-muted-foreground font-mono">
                {formatDuration(track.duration)}
              </div>
              <div className="flex items-center justify-end">
                {renderMenu?.(track)}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
