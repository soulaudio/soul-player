import { useEffect, useState } from 'react';
import { usePlayerCommands, usePlayerStore, TrackList, type Track } from '@soul-player/shared';
import { apiClient } from '../api/client';

interface TracksResponse {
  tracks: Track[];
  total: number;
}

export function LibraryPage() {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const commands = usePlayerCommands();
  const { currentTrack, isPlaying } = usePlayerStore();

  useEffect(() => {
    const loadTracks = async () => {
      try {
        setIsLoading(true);
        const response = await apiClient.get<TracksResponse>('/tracks');
        setTracks(response.tracks);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load tracks');
      } finally {
        setIsLoading(false);
      }
    };

    loadTracks();
  }, []);

  const handleTrackClick = async (track: Track, index: number) => {
    // Build queue from all tracks starting at clicked track
    const queue = tracks.slice(index).map((t) => ({
      trackId: String(t.id),
      title: t.title,
      artist: t.artist || 'Unknown Artist',
      album: t.album || null,
      filePath: t.filePath || '',
      durationSeconds: t.duration || null,
      trackNumber: t.trackNumber || null,
    }));

    // Add tracks before clicked index at the end for wrap-around
    const queueAfter = tracks.slice(0, index).map((t) => ({
      trackId: String(t.id),
      title: t.title,
      artist: t.artist || 'Unknown Artist',
      album: t.album || null,
      filePath: t.filePath || '',
      durationSeconds: t.duration || null,
      trackNumber: t.trackNumber || null,
    }));

    await commands.playQueue([...queue, ...queueAfter], 0);
  };

  if (isLoading) {
    return (
      <div className="p-6">
        <div className="text-muted-foreground">Loading library...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-6">
        <div className="text-destructive">Error: {error}</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-foreground">Library</h1>
        <p className="text-muted-foreground mt-1">{tracks.length} tracks</p>
      </div>

      {tracks.length === 0 ? (
        <div className="text-center py-12">
          <p className="text-muted-foreground">No tracks in library.</p>
          <p className="text-sm text-muted-foreground mt-2">
            Import tracks through the server to get started.
          </p>
        </div>
      ) : (
        <TrackList
          tracks={tracks}
          onTrackClick={handleTrackClick}
          currentTrackId={currentTrack?.id}
          isPlaying={isPlaying}
        />
      )}
    </div>
  );
}
