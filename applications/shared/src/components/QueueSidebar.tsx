import { useEffect, useState } from 'react';
import { usePlayerStore } from '../stores/player';
import { usePlayerCommands, usePlaybackEvents, type QueueTrack } from '../contexts/PlayerCommandsContext';
import { X, Music } from 'lucide-react';

interface QueueSidebarProps {
  isOpen: boolean;
  onClose: () => void;
}

export function QueueSidebar({ isOpen, onClose }: QueueSidebarProps) {
  const [queue, setQueue] = useState<QueueTrack[]>([]);
  const { currentTrack } = usePlayerStore();
  const commands = usePlayerCommands();
  const events = usePlaybackEvents();

  useEffect(() => {
    if (isOpen) {
      // Load queue when opened
      loadQueue();

      // Listen for queue updates
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
          <>
            {/* Now Playing Section */}
            {currentTrack && (
              <div className="border-b border-border">
                <div className="px-4 py-2 bg-muted/30">
                  <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                    Now Playing
                  </h3>
                </div>
                <div className="px-4 py-3 bg-primary/5">
                  <div className="flex items-start gap-3">
                    {/* Playing indicator */}
                    <div className="flex-shrink-0 pt-1">
                      <div className="w-4 h-4 flex items-center justify-center">
                        <div className="w-3 h-3 bg-primary rounded-sm animate-pulse"></div>
                      </div>
                    </div>

                    {/* Track info */}
                    <div className="flex-1 min-w-0">
                      <div className="font-medium text-primary truncate">
                        {currentTrack.title}
                      </div>
                      <div className="text-sm text-muted-foreground truncate">
                        {currentTrack.artist}
                        {currentTrack.album && ` • ${currentTrack.album}`}
                      </div>
                    </div>

                    {/* Duration */}
                    <div className="text-xs text-muted-foreground font-mono flex-shrink-0 pt-1">
                      {formatDuration(currentTrack.duration)}
                    </div>
                  </div>
                </div>
              </div>
            )}

            {/* Up Next Section */}
            {queue.length > 0 && (
              <>
                <div className="px-4 py-2 bg-muted/30">
                  <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                    Up Next • {queue.length}
                  </h3>
                </div>
                <div className="py-1">
                  {queue.map((track, index) => (
                    <div
                      key={`${track.trackId}-${index}`}
                      className="px-4 py-3 hover:bg-accent transition-colors cursor-pointer"
                    >
                      <div className="flex items-start gap-3">
                        {/* Track number */}
                        <div className="text-xs text-muted-foreground font-mono w-6 flex-shrink-0 pt-1">
                          {index + 1}
                        </div>

                        {/* Track info */}
                        <div className="flex-1 min-w-0">
                          <div className="font-medium truncate">{track.title}</div>
                          <div className="text-sm text-muted-foreground truncate">
                            {track.artist}
                            {track.album && ` • ${track.album}`}
                          </div>
                        </div>

                        {/* Duration */}
                        <div className="text-xs text-muted-foreground font-mono flex-shrink-0 pt-1">
                          {formatDuration(track.durationSeconds)}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </>
            )}
          </>
        )}
      </div>

      {/* Footer stats */}
      {(currentTrack || queue.length > 0) && (
        <div className="p-4 border-t border-border text-sm text-muted-foreground">
          {currentTrack ? queue.length + 1 : queue.length} {currentTrack || queue.length !== 1 ? 'tracks' : 'track'} total
        </div>
      )}
    </div>
  );
}
