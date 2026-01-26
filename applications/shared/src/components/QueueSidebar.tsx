import { useEffect, useState, useRef, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useVirtualizer } from '@tanstack/react-virtual';
import { usePlayerPlayback } from '../stores/player';
import { usePlayerCommands, usePlaybackEvents, type QueueTrack } from '../contexts/PlayerCommandsContext';
import { ArtworkImage } from './ArtworkImage';
import { X, Music } from 'lucide-react';
import { debug } from '../utils/debug';

interface QueueSidebarProps {
  isOpen: boolean;
  onClose: () => void;
}

const INITIAL_LOAD_COUNT = 100; // Load first 100 tracks
const LOAD_MORE_COUNT = 50; // Load 50 more when clicking "Load More"

export function QueueSidebar({ isOpen, onClose }: QueueSidebarProps) {
  const [fullQueue, setFullQueue] = useState<QueueTrack[]>([]);
  const [displayLimit, setDisplayLimit] = useState(INITIAL_LOAD_COUNT);
  const { currentTrack, isPlaying } = usePlayerPlayback();
  const commands = usePlayerCommands();
  const events = usePlaybackEvents();
  const parentRef = useRef<HTMLDivElement>(null);

  // Windowed queue - only display limited number of tracks
  const displayedQueue = useMemo(() => {
    return fullQueue.slice(0, displayLimit);
  }, [fullQueue, displayLimit]);

  const hasMore = useMemo(() => {
    return fullQueue.length > displayLimit;
  }, [fullQueue.length, displayLimit]);

  // Virtual list setup - reduced overscan for better performance
  const virtualizer = useVirtualizer({
    count: displayedQueue.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 56, // Estimated height of each queue item (py-2 + content)
    overscan: 2, // Reduced from 5 to 2 to minimize artwork loading
  });

  useEffect(() => {
    if (isOpen) {
      loadQueue();
      const unsubscribe = events.onQueueUpdate(() => {
        loadQueue();
      });
      return unsubscribe;
    }
  }, [isOpen, commands, events]);

  const loadQueue = async () => {
    try {
      const queueData = await commands.getQueue();
      setFullQueue(queueData);
      // Reset display limit when queue updates
      setDisplayLimit(INITIAL_LOAD_COUNT);
    } catch (error) {
      debug.error('[QueueSidebar] Failed to load queue:', error);
    }
  };

  const loadMore = () => {
    setDisplayLimit(prev => Math.min(prev + LOAD_MORE_COUNT, fullQueue.length));
  };

  const handleQueueItemClick = async (index: number) => {
    try {
      await commands.skipToQueueIndex(index);
    } catch (error) {
      debug.error('[QueueSidebar] Failed to skip to queue index:', error);
    }
  };

  const formatDuration = (seconds: number | null) => {
    if (!seconds) return '--:--';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  if (!isOpen) return null;

  return (
    <div className="w-80 border-l border-border bg-background flex flex-col h-full">
      {/* Header */}
      <div className="p-4 border-b border-border flex items-center justify-between">
        <h2 className="text-lg font-semibold">Queue</h2>
        <button
          onClick={onClose}
          className="p-1 rounded-md hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors"
          aria-label="Close queue"
        >
          <X className="w-5 h-5" />
        </button>
      </div>

      {/* Content */}
      <div ref={parentRef} className="flex-1 overflow-y-auto">
        {!currentTrack && fullQueue.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-8 text-center">
            <Music className="w-12 h-12 mb-4 opacity-50" />
            <p>No tracks in queue</p>
            <p className="text-sm mt-2">Play a track to start building your queue</p>
          </div>
        ) : (
          <div className="py-2">
            {/* Now Playing Section */}
            <AnimatePresence mode="popLayout">
              {currentTrack && (
                <motion.div
                  key={`now-playing-${currentTrack.id}`}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.2 }}
                >
                  <h3 className="px-4 pt-2 pb-1 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Now Playing
                  </h3>
                  <div className="px-4 py-2">
                    <div className="flex items-center gap-3">
                      <div className="flex-shrink-0">
                        <div className="w-12 h-12 bg-muted rounded overflow-hidden">
                          <ArtworkImage
                            trackId={currentTrack.id}
                            coverArtPath={currentTrack.coverArtPath}
                            alt={currentTrack.album || 'Album art'}
                            className="w-full h-full object-cover"
                            fallbackClassName="w-full h-full flex items-center justify-center"
                          />
                        </div>
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          {/* Now playing indicator - animated equalizer bars (only animate when playing) */}
                          <div className="flex items-end gap-[2px] h-3">
                            <span className={`w-[3px] bg-primary rounded-full origin-bottom ${isPlaying ? 'h-full animate-[equalize_0.8s_ease-in-out_infinite]' : 'h-1/2'}`} />
                            <span className={`w-[3px] bg-primary rounded-full origin-bottom ${isPlaying ? 'h-full animate-[equalize_0.8s_ease-in-out_infinite_0.2s]' : 'h-3/4'}`} />
                            <span className={`w-[3px] bg-primary rounded-full origin-bottom ${isPlaying ? 'h-full animate-[equalize_0.8s_ease-in-out_infinite_0.4s]' : 'h-1/3'}`} />
                          </div>
                          <div className="font-medium text-primary truncate">
                            {currentTrack.title}
                          </div>
                        </div>
                        <div className="text-sm text-muted-foreground truncate">
                          {currentTrack.artist}
                        </div>
                      </div>
                      <div className="text-xs text-muted-foreground font-mono flex-shrink-0">
                        {formatDuration(currentTrack.duration)}
                      </div>
                    </div>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>

            {/* Up Next Section - Virtualized */}
            {fullQueue.length > 0 && (
              <>
                <div className="px-4 pt-4 pb-1 flex items-center justify-between">
                  <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Up Next
                  </h3>
                  <span className="text-xs text-muted-foreground">
                    {displayedQueue.length} of {fullQueue.length}
                  </span>
                </div>
                <div
                  style={{
                    height: `${virtualizer.getTotalSize()}px`,
                    width: '100%',
                    position: 'relative',
                  }}
                >
                  {virtualizer.getVirtualItems().map((virtualItem) => {
                    const track = displayedQueue[virtualItem.index];
                    return (
                      <div
                        key={`track-${track.trackId}`}
                        data-index={virtualItem.index}
                        ref={virtualizer.measureElement}
                        style={{
                          position: 'absolute',
                          top: 0,
                          left: 0,
                          width: '100%',
                          transform: `translateY(${virtualItem.start}px)`,
                        }}
                        className="px-4 py-2 mx-2 hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors cursor-pointer rounded-md"
                        onClick={() => handleQueueItemClick(virtualItem.index)}
                      >
                        <div className="flex items-center gap-3">
                          <div className="flex-shrink-0">
                            <div className="w-10 h-10 bg-muted rounded overflow-hidden">
                              <ArtworkImage
                                trackId={track.trackId}
                                coverArtPath={track.coverArtPath}
                                alt={track.album || 'Album art'}
                                className="w-full h-full object-cover"
                                fallbackClassName="w-full h-full flex items-center justify-center"
                              />
                            </div>
                          </div>
                          <div className="flex-1 min-w-0">
                            <div className="font-medium truncate text-sm">{track.title}</div>
                            <div className="text-xs text-muted-foreground truncate">
                              {track.artist}
                            </div>
                          </div>
                          <div className="text-xs text-muted-foreground font-mono flex-shrink-0">
                            {formatDuration(track.durationSeconds)}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
                {/* Load More Button */}
                {hasMore && (
                  <div className="px-4 py-3 flex justify-center">
                    <button
                      onClick={loadMore}
                      className="px-4 py-2 text-sm bg-accent hover:opacity-[var(--hover-button-opacity)] rounded-md transition-opacity"
                    >
                      Load {Math.min(LOAD_MORE_COUNT, fullQueue.length - displayLimit)} more tracks
                    </button>
                  </div>
                )}
              </>
            )}
          </div>
        )}
      </div>

      {/* Footer stats */}
      {(currentTrack || fullQueue.length > 0) && (
        <div className="p-4 border-t border-border text-sm text-muted-foreground">
          {(currentTrack ? 1 : 0) + fullQueue.length} {(currentTrack ? 1 : 0) + fullQueue.length !== 1 ? 'tracks' : 'track'} total
        </div>
      )}
    </div>
  );
}
