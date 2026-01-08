import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { TrackList, type Track, type QueueTrack } from '@soul-player/shared';
import { useSyncStore } from '@soul-player/shared/stores/sync';
import { AlbumGrid, Album } from '../components/AlbumGrid';
import { TrackMenu } from '../components/TrackMenu';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { Music, Disc3 } from 'lucide-react';

type ViewMode = 'tracks' | 'albums';

interface DatabaseHealth {
  total_tracks: number;
  tracks_with_availability: number;
  tracks_with_local_files: number;
  issues: string[];
}

// Desktop-specific track interface
interface DesktopTrack extends Track {
  artist_name?: string;
  album_title?: string;
  duration_seconds?: number;
  file_path?: string;
  year?: number;
}

export function LibraryPage() {
  const [viewMode, setViewMode] = useState<ViewMode>('tracks');
  const [tracks, setTracks] = useState<DesktopTrack[]>([]);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [healthWarning, setHealthWarning] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<DesktopTrack | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const { setSyncRequired } = useSyncStore();

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
        invoke<DesktopTrack[]>('get_all_tracks'),
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
          setHealthWarning(msg + ' Sync required to fix - click the alert icon.');
        }

        // Automatically mark sync as required when database issues are detected
        console.log('[LibraryPage] Triggering automatic sync due to missing file paths');
        setSyncRequired(true);
      }
    } catch (err) {
      console.error('Failed to load library:', err);
      setError(err instanceof Error ? err.message : 'Failed to load library');
    } finally {
      setLoading(false);
    }
  };

  // Build queue callback - platform-specific logic
  const buildQueue = useCallback((_allTracks: Track[], clickedTrack: Track, _clickedIndex: number): QueueTrack[] => {
    // Get desktop tracks to access file_path
    const desktopTracks = tracks;

    // Filter out tracks without file paths
    const validTracks = desktopTracks.filter((t) => t.file_path);

    // Find the valid index of the clicked track in desktopTracks
    const validClickedIndex = validTracks.findIndex(t => t.id === clickedTrack.id);
    if (validClickedIndex === -1) {
      console.error('[LibraryPage] Clicked track not found in valid tracks');
      return [];
    }

    // Build queue: all valid tracks starting from clicked one, then wrap around
    const queue: QueueTrack[] = [
      ...validTracks.slice(validClickedIndex),
      ...validTracks.slice(0, validClickedIndex),
    ].map((t) => ({
      trackId: String(t.id),
      title: String(t.title ||'Unknown'),
      artist: t.artist_name || 'Unknown Artist',
      album: t.album_title || null,
      filePath: t.file_path!,
      durationSeconds: t.duration_seconds || null,
      trackNumber: t.trackNumber || null,
    }));

    return queue;
  }, [tracks]);

  const handleTrackPlay = (track: Track) => {
    // Playback state will be updated via backend events
    console.log('[LibraryPage] Playing track:', track.title);
  };

  const handleAlbumPlay = async (album: Album) => {
    // TODO: Load album tracks and play first track
    console.log('Play album:', album);
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
      console.log('[LibraryPage] Track deleted successfully:', confirmDelete.id);

      // Reload library
      await loadLibrary();
      setConfirmDelete(null);
    } catch (error) {
      console.error('[LibraryPage] Failed to delete track:', error);
      setDeleteError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsDeleting(false);
    }
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
            tracks={tracks.map(t => ({
              id: t.id,
              title: String(t.title || 'Unknown'),
              artist: t.artist_name,
              album: t.album_title,
              duration: t.duration_seconds,
              trackNumber: t.trackNumber,
            }))}
            buildQueue={buildQueue}
            onTrackAction={handleTrackPlay}
            renderMenu={(track) => (
              <TrackMenu
                trackId={Number(track.id)}
                trackTitle={track.title}
                onDelete={() => handleDeleteTrack(Number(track.id))}
              />
            )}
          />
        ) : (
          <AlbumGrid albums={albums} onPlay={handleAlbumPlay} />
        )}
      </div>

      {/* Delete confirmation dialog */}
      <ConfirmDialog
        open={!!confirmDelete}
        title="Delete Track"
        message={`Are you sure you want to delete "${confirmDelete?.title}"? This will remove the track from your library.${deleteError ? `\n\nError: ${deleteError}` : ''}`}
        confirmText="Delete"
        confirmVariant="danger"
        onConfirm={handleConfirmDelete}
        onClose={() => setConfirmDelete(null)}
        isLoading={isDeleting}
      />
    </div>
  );
}
