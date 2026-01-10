import { useEffect, useState } from 'react';
import { motion, AnimatePresence, LayoutGroup } from 'framer-motion';
import { usePlayerStore } from '../stores/player';
import { usePlayerCommands, usePlaybackEvents, type QueueTrack } from '../contexts/PlayerCommandsContext';
import { ArtworkImage } from './ArtworkImage';
import { X, Music } from 'lucide-react';

interface QueueSidebarProps {
  isOpen: boolean;
  onClose: () => void;
}

export function QueueSidebar({ isOpen, onClose }: QueueSidebarProps) {
  const [queue, setQueue] = useState<QueueTrack[]>([]);
  const { currentTrack, isPlaying } = usePlayerStore();
  const commands = usePlayerCommands();
  const events = usePlaybackEvents();

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
      setQueue(queueData);
    } catch (error) {
      console.error('[QueueSidebar] Failed to load queue:', error);
    }
  };

  const handleQueueItemClick = async (index: number) => {
    try {
      await commands.skipToQueueIndex(index);
    } catch (error) {
      console.error('[QueueSidebar] Failed to skip to queue index:', error);
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
          className="p-1 rounded-md hover:bg-accent transition-colors"
          aria-label="Close queue"
        >
          <X className="w-5 h-5" />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {!currentTrack && queue.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-8 text-center">
            <Music className="w-12 h-12 mb-4 opacity-50" />
            <p>No tracks in queue</p>
            <p className="text-sm mt-2">Play a track to start building your queue</p>
          </div>
        ) : (
          <LayoutGroup>
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
                    <motion.div
                      layoutId={`track-${currentTrack.id}`}
                      className="px-4 py-2"
                      transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
                    >
                      <div className="flex items-center gap-3">
                        <div className="flex-shrink-0">
                          <motion.div
                            layout
                            className="w-12 h-12 bg-muted rounded overflow-hidden"
                          >
                            <ArtworkImage
                              trackId={currentTrack.id}
                              coverArtPath={currentTrack.coverArtPath}
                              alt={currentTrack.album || 'Album art'}
                              className="w-full h-full object-cover"
                              fallbackClassName="w-full h-full flex items-center justify-center"
                            />
                          </motion.div>
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
                    </motion.div>
                  </motion.div>
                )}
              </AnimatePresence>

              {/* Up Next Section */}
              {queue.length > 0 && (
                <>
                  <h3 className="px-4 pt-4 pb-1 text-xs font-medium text-muted-foreground uppercase tracking-wider">
                    Up Next
                  </h3>
                  <AnimatePresence mode="popLayout" initial={false}>
                    {queue.map((track, index) => (
                      <motion.div
                        key={track.trackId}
                        layoutId={`track-${track.trackId}`}
                        layout
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
                        className="px-4 py-2 mx-2 hover:bg-accent/50 transition-colors cursor-pointer rounded-md"
                        onClick={() => handleQueueItemClick(index)}
                      >
                        <div className="flex items-center gap-3">
                          <div className="flex-shrink-0">
                            <motion.div
                              layout
                              className="w-10 h-10 bg-muted rounded overflow-hidden"
                            >
                              <ArtworkImage
                                trackId={track.trackId}
                                coverArtPath={track.coverArtPath}
                                alt={track.album || 'Album art'}
                                className="w-full h-full object-cover"
                                fallbackClassName="w-full h-full flex items-center justify-center"
                              />
                            </motion.div>
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
                      </motion.div>
                    ))}
                  </AnimatePresence>
                </>
              )}
            </div>
          </LayoutGroup>
        )}
      </div>

      {/* Footer stats */}
      {(currentTrack || queue.length > 0) && (
        <div className="p-4 border-t border-border text-sm text-muted-foreground">
          {(currentTrack ? 1 : 0) + queue.length} {(currentTrack ? 1 : 0) + queue.length !== 1 ? 'tracks' : 'track'} total
        </div>
      )}
    </div>
  );
}
