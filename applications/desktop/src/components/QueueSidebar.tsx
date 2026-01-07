import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { usePlayerStore } from '@soul-player/shared/stores/player';
import { X, Music } from 'lucide-react';

interface QueueTrack {
  trackId: string;
  title: string;
  artist: string;
  album: string | null;
  filePath: string;
  durationSeconds: number | null;
  trackNumber: number | null;
}

interface QueueSidebarProps {
  isOpen: boolean;
  onClose: () => void;
}

export function QueueSidebar({ isOpen, onClose }: QueueSidebarProps) {
  const [queue, setQueue] = useState<QueueTrack[]>([]);
  const { currentTrack } = usePlayerStore();

  useEffect(() => {
    // Load queue initially
    loadQueue();

    // Listen for queue updates
    const unlisten = listen('playback:queue-updated', () => {
      loadQueue();
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const loadQueue = async () => {
    try {
      const queueData = await invoke<QueueTrack[]>('get_queue');
      setQueue(queueData);
    } catch (error) {
      console.error('Failed to load queue:', error);
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

      {/* Queue List */}
      <div className="flex-1 overflow-y-auto">
        {queue.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-8 text-center">
            <Music className="w-12 h-12 mb-4 opacity-50" />
            <p>No tracks in queue</p>
            <p className="text-sm mt-2">Play a track to start building your queue</p>
          </div>
        ) : (
          <div className="py-2">
            {queue.map((track, index) => {
              const isCurrentTrack = currentTrack?.id.toString() === track.trackId;

              return (
                <div
                  key={`${track.trackId}-${index}`}
                  className={`px-4 py-3 hover:bg-accent transition-colors cursor-pointer ${
                    isCurrentTrack ? 'bg-accent/50' : ''
                  }`}
                >
                  <div className="flex items-start gap-3">
                    {/* Track number */}
                    <div className="text-xs text-muted-foreground font-mono w-6 flex-shrink-0 pt-1">
                      {index + 1}
                    </div>

                    {/* Track info */}
                    <div className="flex-1 min-w-0">
                      <div
                        className={`font-medium truncate ${
                          isCurrentTrack ? 'text-primary' : ''
                        }`}
                      >
                        {track.title}
                      </div>
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
              );
            })}
          </div>
        )}
      </div>

      {/* Footer stats */}
      {queue.length > 0 && (
        <div className="p-4 border-t border-border text-sm text-muted-foreground">
          {queue.length} {queue.length === 1 ? 'track' : 'tracks'} in queue
        </div>
      )}
    </div>
  );
}
