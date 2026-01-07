import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { usePlayerStore } from '@soul-player/shared/stores/player';
import { TrackList, Track } from '../components/TrackList';
import { AlbumGrid, Album } from '../components/AlbumGrid';
import { Music, Disc3 } from 'lucide-react';

type ViewMode = 'tracks' | 'albums';

interface DatabaseHealth {
  total_tracks: number;
  tracks_with_availability: number;
  tracks_with_local_files: number;
  issues: string[];
}

export function LibraryPage() {
  const [viewMode, setViewMode] = useState<ViewMode>('tracks');
  const [tracks, setTracks] = useState<Track[]>([]);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [healthWarning, setHealthWarning] = useState<string | null>(null);

  // Use global player store instead of local state
  const { currentTrack, isPlaying } = usePlayerStore();

  useEffect(() => {
    loadLibrary();

    // Listen for import completion to auto-refresh library
    const unlistenImport = listen('import-complete', () => {
      console.log('[LibraryPage] Import completed, reloading library...');
      loadLibrary();
    });

    return () => {
      unlistenImport.then((fn) => fn());
    };
  }, []);

  const loadLibrary = async () => {
    setLoading(true);
    setError(null);
    setHealthWarning(null);
    try {
      const [tracksData, albumsData, health] = await Promise.all([
        invoke<Track[]>('get_all_tracks'),
        invoke<Album[]>('get_all_albums'),
        invoke<DatabaseHealth>('check_database_health'),
      ]);

      setTracks(tracksData);
      setAlbums(albumsData);

      // Check for issues
      console.log('[LibraryPage] Database health:', health);
      if (health.issues.length > 0) {
        const warning = health.issues.join(' ');
        setHealthWarning(warning);
        console.warn('[LibraryPage] Health issues:', warning);
      }

      // Additional check: count tracks without file_path
      const tracksWithoutPaths = tracksData.filter(t => !t.file_path).length;
      if (tracksWithoutPaths > 0) {
        const msg = `${tracksWithoutPaths} out of ${tracksData.length} tracks are missing file paths and cannot be played.`;
        console.warn('[LibraryPage]', msg);
        if (!healthWarning) {
          setHealthWarning(msg + ' Try re-importing your library.');
        }
      }
    } catch (err) {
      console.error('Failed to load library:', err);
      setError(err instanceof Error ? err.message : 'Failed to load library');
    } finally {
      setLoading(false);
    }
  };

  const handleTrackPlay = (track: Track) => {
    // Playback state will be updated via backend events (usePlaybackEvents)
    console.log('[LibraryPage] Playing track:', track.title);
  };

  const handleTrackPause = () => {
    // Playback state will be updated via backend events (usePlaybackEvents)
    console.log('[LibraryPage] Pausing playback');
  };

  const handleAlbumPlay = async (album: Album) => {
    // TODO: Load album tracks and play first track
    console.log('Play album:', album);
  };

  const handleTrackDeleted = async (trackId: number) => {
    console.log('[LibraryPage] Track deleted, refreshing list:', trackId);
    await loadLibrary();
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4"></div>
          <p className="text-muted-foreground">Loading library...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">Failed to load library</p>
          <p className="text-sm">{error}</p>
          <button
            onClick={loadLibrary}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Health warning banner */}
      {healthWarning && (
        <div className="mb-4 p-4 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
          <div className="flex items-start gap-3">
            <div className="flex-shrink-0 w-5 h-5 rounded-full bg-yellow-500/20 flex items-center justify-center mt-0.5">
              <span className="text-yellow-600 dark:text-yellow-400 text-sm">⚠</span>
            </div>
            <div className="flex-1">
              <p className="text-sm text-yellow-800 dark:text-yellow-200 font-medium">
                Database Issue Detected
              </p>
              <p className="text-sm text-yellow-700 dark:text-yellow-300 mt-1">
                {healthWarning}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold">Library</h1>
          <p className="text-muted-foreground mt-1">
            {tracks.length} track{tracks.length !== 1 ? 's' : ''} • {albums.length} album
            {albums.length !== 1 ? 's' : ''}
          </p>
        </div>

        {/* View mode toggle */}
        <div className="flex items-center gap-2 bg-muted rounded-lg p-1">
          <button
            onClick={() => setViewMode('tracks')}
            className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
              viewMode === 'tracks'
                ? 'bg-background shadow-sm'
                : 'hover:bg-background/50'
            }`}
            aria-label="View tracks"
          >
            <Music className="w-4 h-4" />
            <span className="text-sm font-medium">Tracks</span>
          </button>
          <button
            onClick={() => setViewMode('albums')}
            className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
              viewMode === 'albums'
                ? 'bg-background shadow-sm'
                : 'hover:bg-background/50'
            }`}
            aria-label="View albums"
          >
            <Disc3 className="w-4 h-4" />
            <span className="text-sm font-medium">Albums</span>
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {viewMode === 'tracks' ? (
          <TrackList
            tracks={tracks}
            currentTrackId={currentTrack?.id}
            isPlaying={isPlaying}
            onPlay={handleTrackPlay}
            onPause={handleTrackPause}
            onTrackDeleted={handleTrackDeleted}
          />
        ) : (
          <AlbumGrid albums={albums} onPlay={handleAlbumPlay} />
        )}
      </div>
    </div>
  );
}
