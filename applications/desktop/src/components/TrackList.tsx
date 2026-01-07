import { Play, Pause, Music } from 'lucide-react';
import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { TrackMenu } from './TrackMenu';
import { ConfirmDialog } from './ConfirmDialog';

export interface Track {
  id: number;
  title: string;
  artist_name?: string;
  album_title?: string;
  duration_seconds?: number;
  file_path?: string;
  track_number?: number;
  year?: number;
}

interface TrackListProps {
  tracks: Track[];
  currentTrackId?: number;
  isPlaying?: boolean;
  onPlay?: (track: Track) => void;
  onPause?: () => void;
  onTrackDeleted?: (trackId: number) => void;
}

function formatDuration(seconds?: number): string {
  if (!seconds) return '--:--';
  const minutes = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${minutes}:${secs.toString().padStart(2, '0')}`;
}

export function TrackList({
  tracks,
  currentTrackId,
  isPlaying = false,
  onPlay,
  onPause,
  onTrackDeleted,
}: TrackListProps) {
  const [hoveredTrackId, setHoveredTrackId] = useState<number | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<Track | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);

  const handlePlay = async (track: Track) => {
    if (!track.file_path) {
      console.error('[TrackList] Track has no file path:', track);
      alert('Cannot play track: File path not available');
      return;
    }

    console.log('[TrackList] Playing track:', track.title, 'path:', track.file_path);

    try {
      // Find the index of the clicked track
      const startIndex = tracks.findIndex((t) => t.id === track.id);
      if (startIndex === -1) {
        console.error('[TrackList] Track not found in list');
        return;
      }

      // Filter out tracks without file paths
      const validTracks = tracks.filter((t) => t.file_path);
      if (validTracks.length === 0) {
        console.error('[TrackList] No tracks have file paths');
        alert('Cannot play: No tracks have file paths');
        return;
      }

      // Prepare queue: all tracks starting from the clicked one, then wrap around
      const queue = [
        ...validTracks.slice(startIndex),
        ...validTracks.slice(0, startIndex),
      ].map((t) => ({
        trackId: t.id.toString(),
        title: t.title,
        artist: t.artist_name || 'Unknown Artist',
        album: t.album_title || null,
        filePath: t.file_path!,
        durationSeconds: t.duration_seconds || null,
        trackNumber: t.track_number || null,
      }));

      console.log('[TrackList] Playing queue with', queue.length, 'tracks');

      // Play the queue starting from index 0 (which is our clicked track)
      await invoke('play_queue', {
        queue,
        startIndex: 0,
      });

      console.log('[TrackList] play_queue command succeeded');
      onPlay?.(track);
    } catch (error) {
      console.error('[TrackList] Failed to play track:', error);
      alert(`Failed to play track: ${error}`);
    }
  };

  const handlePause = async () => {
    try {
      await invoke('pause_playback');
      onPause?.();
    } catch (error) {
      console.error('Failed to pause track:', error);
    }
  };

  const handleDeleteTrack = (trackId: number) => {
    const track = tracks.find((t) => t.id === trackId);
    if (track) {
      setConfirmDelete(track);
      setDeleteError(null);
    }
  };

  const handleConfirmDelete = async () => {
    if (!confirmDelete) return;

    setIsDeleting(true);
    setDeleteError(null);

    try {
      await invoke('delete_track', { id: confirmDelete.id });
      console.log('[TrackList] Track deleted successfully:', confirmDelete.id);

      // Notify parent to refresh
      onTrackDeleted?.(confirmDelete.id);

      setConfirmDelete(null);
    } catch (error) {
      console.error('[TrackList] Failed to delete track:', error);
      setDeleteError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsDeleting(false);
    }
  };

  if (tracks.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Music className="w-12 h-12 mb-4 opacity-50" />
        <p>No tracks found</p>
        <p className="text-sm mt-1">Import music to get started</p>
      </div>
    );
  }

  return (
    <div className="border rounded-lg overflow-hidden">
      <div className="bg-muted/50">
        <div className="grid grid-cols-[40px_minmax(200px,1fr)_minmax(150px,200px)_minmax(150px,200px)_80px_40px] gap-4 px-4 py-2 text-sm font-medium text-muted-foreground">
          <div>#</div>
          <div>Title</div>
          <div>Artist</div>
          <div>Album</div>
          <div className="text-right">Duration</div>
          <div></div>
        </div>
      </div>
      <div>
        {tracks.map((track, index) => {
          const isCurrentTrack = track.id === currentTrackId;
          const showPlayButton = isCurrentTrack && isPlaying;

          return (
            <div
              key={track.id}
              className={`grid grid-cols-[40px_minmax(200px,1fr)_minmax(150px,200px)_minmax(150px,200px)_80px_40px] gap-4 px-4 py-3 hover:bg-accent/50 border-b last:border-b-0 transition-colors group ${
                isCurrentTrack ? 'bg-accent/30' : ''
              }`}
              onMouseEnter={() => setHoveredTrackId(track.id)}
              onMouseLeave={() => setHoveredTrackId(null)}
              onDoubleClick={() => handlePlay(track)}
            >
              <div className="flex items-center justify-center">
                {hoveredTrackId === track.id || isCurrentTrack ? (
                  <button
                    onClick={() => (showPlayButton ? handlePause() : handlePlay(track))}
                    className="w-8 h-8 flex items-center justify-center rounded hover:bg-primary/10 transition-colors"
                    aria-label={showPlayButton ? 'Pause' : 'Play'}
                  >
                    {showPlayButton ? (
                      <Pause className="w-4 h-4" fill="currentColor" />
                    ) : (
                      <Play className="w-4 h-4" fill="currentColor" />
                    )}
                  </button>
                ) : (
                  <span className="text-muted-foreground text-sm">
                    {track.track_number || index + 1}
                  </span>
                )}
              </div>
              <div className="flex flex-col justify-center min-w-0">
                <div
                  className={`truncate ${isCurrentTrack ? 'text-primary font-medium' : ''}`}
                  title={track.title}
                >
                  {track.title}
                </div>
              </div>
              <div className="flex items-center text-muted-foreground truncate" title={track.artist_name}>
                {track.artist_name || 'Unknown Artist'}
              </div>
              <div className="flex items-center text-muted-foreground truncate" title={track.album_title}>
                {track.album_title || 'Unknown Album'}
              </div>
              <div className="flex items-center justify-end text-muted-foreground text-sm">
                {formatDuration(track.duration_seconds)}
              </div>
              <div className="flex items-center justify-center">
                <TrackMenu
                  trackId={track.id}
                  trackTitle={track.title}
                  onDelete={handleDeleteTrack}
                />
              </div>
            </div>
          );
        })}
      </div>
      <ConfirmDialog
        open={confirmDelete !== null}
        onClose={() => {
          setConfirmDelete(null);
          setDeleteError(null);
        }}
        onConfirm={handleConfirmDelete}
        title="Remove Track"
        message={
          deleteError
            ? deleteError
            : `Are you sure you want to remove "${confirmDelete?.title}" from your library?${
                confirmDelete?.file_path?.includes('/library/')
                  ? ' This will permanently delete the file.'
                  : ''
              }`
        }
        confirmText="Remove"
        confirmVariant="danger"
        isLoading={isDeleting}
      />
    </div>
  );
}
