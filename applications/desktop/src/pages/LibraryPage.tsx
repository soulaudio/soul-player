import { useEffect, useState, useCallback, useMemo } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { TrackList, type Track, type QueueTrack, type SourceType } from '@soul-player/shared';
import { useSyncStore } from '@soul-player/shared/stores/sync';
import { AlbumGrid, Album } from '../components/AlbumGrid';
import { TrackMenu } from '../components/TrackMenu';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { Music, Disc3, ListMusic, Users, Guitar, Filter, X, Plus, Play } from 'lucide-react';

type TabId = 'tracks' | 'albums' | 'playlists' | 'artists' | 'genres';

interface Artist {
  id: number;
  name: string;
  sort_name?: string;
  track_count: number;
  album_count: number;
}

interface Genre {
  id: number;
  name: string;
  track_count: number;
}

interface Playlist {
  id: string;
  name: string;
  description?: string;
  owner_id: number;
  is_public: boolean;
  is_favorite: boolean;
  track_count: number;
  created_at: string;
  updated_at: string;
}

interface Tab {
  id: TabId;
  label: string;
  icon: React.ReactNode;
}

const TABS: Tab[] = [
  { id: 'tracks', label: 'Tracks', icon: <Music className="w-4 h-4" /> },
  { id: 'albums', label: 'Albums', icon: <Disc3 className="w-4 h-4" /> },
  { id: 'playlists', label: 'Playlists', icon: <ListMusic className="w-4 h-4" /> },
  { id: 'artists', label: 'Artists', icon: <Users className="w-4 h-4" /> },
  { id: 'genres', label: 'Genres', icon: <Guitar className="w-4 h-4" /> },
];

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
  // Audio format info
  file_format?: string;
  bit_rate?: number;
  sample_rate?: number;
  channels?: number;
  // Source info
  source_id?: number;
  source_name?: string;
  source_type?: SourceType;
  source_online?: boolean;
}

// Filter state interface
interface LibraryFilters {
  sourceType: SourceType | 'all';
  format: string | 'all';
}

export function LibraryPage() {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const tabParam = searchParams.get('tab') as TabId | null;
  const [activeTab, setActiveTab] = useState<TabId>(tabParam || 'tracks');

  const navigate = useNavigate();
  const [tracks, setTracks] = useState<DesktopTrack[]>([]);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [artists, setArtists] = useState<Artist[]>([]);
  const [genres, setGenres] = useState<Genre[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [healthWarning, setHealthWarning] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<DesktopTrack | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const { setSyncRequired } = useSyncStore();

  // Filters state
  const [filters, setFilters] = useState<LibraryFilters>({
    sourceType: 'all',
    format: 'all',
  });
  const [showFilters, setShowFilters] = useState(false);

  // Compute unique formats and sources for filter dropdowns
  const availableFormats = useMemo(() => {
    const formats = new Set<string>();
    tracks.forEach(t => {
      if (t.file_format) formats.add(t.file_format.toUpperCase());
    });
    return Array.from(formats).sort();
  }, [tracks]);

  const availableSources = useMemo(() => {
    const sources = new Map<string, { type: SourceType; name: string }>();
    tracks.forEach(t => {
      if (t.source_type && t.source_name) {
        sources.set(t.source_name, { type: t.source_type, name: t.source_name });
      }
    });
    return Array.from(sources.values());
  }, [tracks]);

  // Filter tracks based on current filters
  const filteredTracks = useMemo(() => {
    return tracks.filter(track => {
      // Filter by source type
      if (filters.sourceType !== 'all' && track.source_type !== filters.sourceType) {
        return false;
      }
      // Filter by format
      if (filters.format !== 'all' && track.file_format?.toUpperCase() !== filters.format) {
        return false;
      }
      return true;
    });
  }, [tracks, filters]);

  // Check if any filters are active
  const hasActiveFilters = filters.sourceType !== 'all' || filters.format !== 'all';

  const clearFilters = () => {
    setFilters({ sourceType: 'all', format: 'all' });
  };

  // Update active tab when URL param changes
  useEffect(() => {
    if (tabParam && TABS.some(t => t.id === tabParam)) {
      setActiveTab(tabParam);
    }
  }, [tabParam]);

  // Update URL when tab changes
  const handleTabChange = (tabId: TabId) => {
    setActiveTab(tabId);
    if (tabId === 'tracks') {
      setSearchParams({});
    } else {
      setSearchParams({ tab: tabId });
    }
  };

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
      const [tracksData, albumsData, artistsData, genresData, playlistsData, health] = await Promise.all([
        invoke<DesktopTrack[]>('get_all_tracks'),
        invoke<Album[]>('get_all_albums'),
        invoke<Artist[]>('get_all_artists'),
        invoke<Genre[]>('get_all_genres'),
        invoke<Playlist[]>('get_all_playlists'),
        invoke<DatabaseHealth>('check_database_health'),
      ]);

      setTracks(tracksData);
      setAlbums(albumsData);
      setArtists(artistsData);
      setGenres(genresData);
      setPlaylists(playlistsData);

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
    console.log('[LibraryPage] buildQueue called:', {
      clickedTrack: clickedTrack.title,
      totalTracks: filteredTracks.length,
    });

    // Get desktop tracks to access file_path (use filtered tracks)
    const desktopTracks = filteredTracks;

    // Filter out tracks without file paths
    const validTracks = desktopTracks.filter((t) => t.file_path);
    console.log('[LibraryPage] Valid tracks with file_path:', validTracks.length);

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

    console.log('[LibraryPage] Built queue with', queue.length, 'tracks');
    return queue;
  }, [filteredTracks]);

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
              <span className="text-yellow-600 dark:text-yellow-400 text-sm">!</span>
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
          <h1 className="text-3xl font-bold">{t('nav.library')}</h1>
          <p className="text-muted-foreground mt-1">
            {hasActiveFilters ? (
              <>
                {filteredTracks.length} of {tracks.length} track{tracks.length !== 1 ? 's' : ''}
              </>
            ) : (
              <>
                {tracks.length} track{tracks.length !== 1 ? 's' : ''} • {albums.length} album
                {albums.length !== 1 ? 's' : ''} • {artists.length} artist{artists.length !== 1 ? 's' : ''}
              </>
            )}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowFilters(!showFilters)}
            className={`flex items-center gap-2 px-3 py-2 rounded-lg transition-colors ${
              showFilters || hasActiveFilters
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted hover:bg-muted/80'
            }`}
          >
            <Filter className="w-4 h-4" />
            <span className="text-sm">{t('common.filters', 'Filters')}</span>
            {hasActiveFilters && (
              <span className="ml-1 px-1.5 py-0.5 bg-primary-foreground/20 rounded text-xs">
                {(filters.sourceType !== 'all' ? 1 : 0) + (filters.format !== 'all' ? 1 : 0)}
              </span>
            )}
          </button>
          {hasActiveFilters && (
            <button
              onClick={clearFilters}
              className="p-2 rounded-lg hover:bg-muted transition-colors"
              title={t('common.clearFilters', 'Clear filters')}
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>

      {/* Filters Panel */}
      {showFilters && (
        <div className="mb-6 p-4 bg-muted/50 rounded-lg border">
          <div className="flex flex-wrap gap-4">
            {/* Source Filter */}
            <div className="flex flex-col gap-1">
              <label className="text-xs font-medium text-muted-foreground">
                {t('library.source', 'Source')}
              </label>
              <select
                value={filters.sourceType}
                onChange={(e) => setFilters({ ...filters, sourceType: e.target.value as SourceType | 'all' })}
                className="px-3 py-2 rounded-md bg-background border text-sm min-w-[140px]"
              >
                <option value="all">{t('common.all', 'All')}</option>
                <option value="local">{t('sources.localSource', 'Local Files')}</option>
                <option value="server">{t('sources.serverSources', 'Server')}</option>
                <option value="cached">{t('library.cached', 'Cached')}</option>
              </select>
            </div>

            {/* Format Filter */}
            <div className="flex flex-col gap-1">
              <label className="text-xs font-medium text-muted-foreground">
                {t('library.format', 'Format')}
              </label>
              <select
                value={filters.format}
                onChange={(e) => setFilters({ ...filters, format: e.target.value })}
                className="px-3 py-2 rounded-md bg-background border text-sm min-w-[140px]"
              >
                <option value="all">{t('common.all', 'All')}</option>
                {availableFormats.map((format) => (
                  <option key={format} value={format}>
                    {format}
                  </option>
                ))}
              </select>
            </div>

            {/* Available Sources as quick filters */}
            {availableSources.length > 1 && (
              <div className="flex flex-col gap-1">
                <label className="text-xs font-medium text-muted-foreground">
                  {t('library.quickFilter', 'Quick Filter')}
                </label>
                <div className="flex gap-2">
                  {availableSources.map((source) => (
                    <button
                      key={source.name}
                      onClick={() => setFilters({ ...filters, sourceType: source.type })}
                      className={`px-3 py-1.5 text-sm rounded-md transition-colors ${
                        filters.sourceType === source.type
                          ? 'bg-primary text-primary-foreground'
                          : 'bg-background border hover:bg-muted'
                      }`}
                    >
                      {source.name}
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Tab Navigation */}
      <div className="flex items-center gap-1 bg-muted rounded-lg p-1 mb-6 w-fit">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => handleTabChange(tab.id)}
            className={`px-4 py-2 rounded-md transition-colors flex items-center gap-2 ${
              activeTab === tab.id
                ? 'bg-background shadow-sm'
                : 'hover:bg-background/50'
            }`}
            aria-label={`View ${tab.label}`}
          >
            {tab.icon}
            <span className="text-sm font-medium">{tab.label}</span>
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {activeTab === 'tracks' && (
          <TrackList
            tracks={filteredTracks.map(t => ({
              id: t.id,
              title: String(t.title || 'Unknown'),
              artist: t.artist_name,
              album: t.album_title,
              duration: t.duration_seconds,
              trackNumber: t.trackNumber,
              isAvailable: !!t.file_path,
              // Format info
              format: t.file_format,
              bitrate: t.bit_rate,
              sampleRate: t.sample_rate,
              channels: t.channels,
              // Source info
              sourceType: t.source_type || 'local',
              sourceName: t.source_name,
              sourceOnline: t.source_online,
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
        )}

        {activeTab === 'albums' && (
          <AlbumGrid albums={albums} onPlay={handleAlbumPlay} />
        )}

        {activeTab === 'playlists' && (
          playlists.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
              {playlists.map((playlist) => (
                <div
                  key={playlist.id}
                  onClick={() => navigate(`/playlists/${playlist.id}`)}
                  className="p-4 rounded-xl bg-card border hover:bg-accent hover:border-primary transition-all cursor-pointer group"
                >
                  <div className="flex items-center gap-3">
                    <div className="p-3 rounded-lg bg-primary/10 text-primary group-hover:bg-primary group-hover:text-primary-foreground transition-colors">
                      <ListMusic className="w-8 h-8" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="font-medium truncate">{playlist.name}</h3>
                      <p className="text-xs text-muted-foreground">
                        {playlist.track_count} {t('library.tracks', 'tracks')}
                      </p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <ListMusic className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">{t('playlist.noPlaylists', 'No playlists yet')}</p>
              <p className="text-sm mt-1">{t('playlist.createHint', 'Create a playlist to organize your music')}</p>
              <button
                onClick={async () => {
                  try {
                    const newPlaylist = await invoke<Playlist>('create_playlist', {
                      name: t('playlist.newPlaylistName', 'New Playlist'),
                      description: null,
                    });
                    navigate(`/playlists/${newPlaylist.id}`);
                  } catch (err) {
                    console.error('Failed to create playlist:', err);
                  }
                }}
                className="mt-4 flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
              >
                <Plus className="w-4 h-4" />
                {t('playlist.create', 'Create Playlist')}
              </button>
            </div>
          )
        )}

        {activeTab === 'artists' && (
          artists.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
              {artists.map((artist) => (
                <div
                  key={artist.id}
                  onClick={() => navigate(`/artists/${artist.id}`)}
                  className="group cursor-pointer"
                >
                  <div className="relative aspect-square mb-3 bg-muted rounded-full overflow-hidden shadow-md hover:shadow-xl transition-shadow flex items-center justify-center group-hover:bg-primary/10">
                    <Users className="w-12 h-12 text-muted-foreground group-hover:text-primary transition-colors" />
                    <div className="absolute inset-0 bg-black/40 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity">
                      <div className="w-10 h-10 bg-primary rounded-full flex items-center justify-center">
                        <Play className="w-5 h-5 text-primary-foreground ml-0.5" fill="currentColor" />
                      </div>
                    </div>
                  </div>
                  <div className="text-center">
                    <h3 className="font-medium truncate" title={artist.name}>
                      {artist.name}
                    </h3>
                    <p className="text-sm text-muted-foreground">
                      {artist.album_count} {t('library.albums', 'albums')} • {artist.track_count} {t('library.tracks', 'tracks')}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Users className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">{t('artist.noArtists', 'No artists found')}</p>
              <p className="text-sm mt-1">{t('artist.noArtistsHint', 'Import music to see your artists')}</p>
            </div>
          )
        )}

        {activeTab === 'genres' && (
          genres.length > 0 ? (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-4">
              {genres.map((genre) => (
                <div
                  key={genre.id}
                  onClick={() => navigate(`/genres/${genre.id}`)}
                  className="p-4 rounded-xl bg-card border hover:bg-accent hover:border-primary transition-all cursor-pointer group"
                >
                  <div className="flex items-center gap-3">
                    <div className="p-2 rounded-lg bg-primary/10 text-primary group-hover:bg-primary group-hover:text-primary-foreground transition-colors">
                      <Guitar className="w-6 h-6" />
                    </div>
                    <div className="flex-1 min-w-0">
                      <h3 className="font-medium truncate">{genre.name}</h3>
                      <p className="text-xs text-muted-foreground">
                        {genre.track_count} {t('library.tracks', 'tracks')}
                      </p>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <Guitar className="w-12 h-12 mb-4 opacity-50" />
              <p className="font-medium">{t('genre.noGenres', 'No genres found')}</p>
              <p className="text-sm mt-1">{t('genre.noGenresHint', 'Genre information is extracted from your music files')}</p>
            </div>
          )
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
