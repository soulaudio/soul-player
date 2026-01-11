import { useEffect, useState, useCallback, useMemo, useRef } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { TrackList, type Track, type QueueTrack, type SourceType } from '@soul-player/shared';
import { useSyncStore } from '@soul-player/shared/stores/sync';
import { AlbumCard, type Album } from '../components/AlbumCard';
import { TrackMenu } from '../components/TrackMenu';
import { ConfirmDialog } from '../components/ConfirmDialog';
import { Music, Disc3, ListMusic, Users, Filter, X, Plus, Play, Search } from 'lucide-react';
import { useVirtualizer } from '@tanstack/react-virtual';

type TabId = 'albums' | 'playlists' | 'artists' | 'tracks';

interface Artist {
  id: number;
  name: string;
  sort_name?: string;
  track_count: number;
  album_count: number;
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
  { id: 'albums', label: 'Albums', icon: <Disc3 className="w-4 h-4" /> },
  { id: 'playlists', label: 'Playlists', icon: <ListMusic className="w-4 h-4" /> },
  { id: 'artists', label: 'Artists', icon: <Users className="w-4 h-4" /> },
  { id: 'tracks', label: 'Tracks', icon: <Music className="w-4 h-4" /> },
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

// Virtualization constants
const VIRTUAL_ITEM_SIZE = 56; // px height for track rows
const VIRTUAL_GRID_COLUMNS = 6; // album/artist grid columns
const VIRTUAL_GRID_ROW_HEIGHT = 220; // px height for album card + text

// Virtualized Album Grid Component
function VirtualizedAlbumGrid({
  albums,
  searchQuery
}: {
  albums: Album[];
  searchQuery: string;
}) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);

  // Filter albums by search
  const filteredAlbums = useMemo(() => {
    if (!searchQuery.trim()) return albums;
    const query = searchQuery.toLowerCase();
    return albums.filter(
      album =>
        album.title.toLowerCase().includes(query) ||
        (album.artist_name?.toLowerCase().includes(query))
    );
  }, [albums, searchQuery]);

  // Calculate columns based on container width using ResizeObserver
  const [columns, setColumns] = useState(VIRTUAL_GRID_COLUMNS);

  useEffect(() => {
    if (!parentRef.current) return;

    const updateColumns = (width: number) => {
      // Responsive columns: xl=6, lg=5, md=4, sm=3, xs=2
      if (width >= 1280) setColumns(6);
      else if (width >= 1024) setColumns(5);
      else if (width >= 768) setColumns(4);
      else if (width >= 640) setColumns(3);
      else setColumns(2);
    };

    // Initial measurement
    updateColumns(parentRef.current.offsetWidth);

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        updateColumns(entry.contentRect.width);
      }
    });

    resizeObserver.observe(parentRef.current);
    return () => resizeObserver.disconnect();
  }, []);

  const rowCount = Math.ceil(filteredAlbums.length / columns);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => VIRTUAL_GRID_ROW_HEIGHT,
    overscan: 3,
  });

  if (albums.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Disc3 className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('library.noAlbums', 'No albums found')}</p>
        <p className="text-sm mt-1">{t('library.noAlbumsHint', 'Import music to see your albums')}</p>
      </div>
    );
  }

  if (filteredAlbums.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Search className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('library.noSearchResults', 'No results found')}</p>
        <p className="text-sm mt-1">{t('library.tryDifferentSearch', 'Try a different search term')}</p>
      </div>
    );
  }

  return (
    <div
      ref={parentRef}
      className="h-full overflow-auto"
    >
      <div
        style={{
          height: `${rowVirtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const startIndex = virtualRow.index * columns;
          const rowAlbums = filteredAlbums.slice(startIndex, startIndex + columns);

          return (
            <div
              key={virtualRow.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
                padding: '0 0 16px 0',
              }}
            >
              <div
                className="grid gap-4 h-full"
                style={{
                  gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                }}
              >
                {rowAlbums.map((album) => (
                  <AlbumCard key={album.id} album={album} className="w-full" />
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// Virtualized Artist Grid Component
function VirtualizedArtistGrid({
  artists,
  searchQuery,
  onNavigate
}: {
  artists: Artist[];
  searchQuery: string;
  onNavigate: (id: number) => void;
}) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);

  // Filter artists by search
  const filteredArtists = useMemo(() => {
    if (!searchQuery.trim()) return artists;
    const query = searchQuery.toLowerCase();
    return artists.filter(artist =>
      artist.name.toLowerCase().includes(query) ||
      (artist.sort_name?.toLowerCase().includes(query))
    );
  }, [artists, searchQuery]);

  // Calculate columns based on container width using ResizeObserver
  const [columns, setColumns] = useState(VIRTUAL_GRID_COLUMNS);

  useEffect(() => {
    if (!parentRef.current) return;

    const updateColumns = (width: number) => {
      if (width >= 1280) setColumns(6);
      else if (width >= 1024) setColumns(5);
      else if (width >= 768) setColumns(4);
      else if (width >= 640) setColumns(3);
      else setColumns(2);
    };

    updateColumns(parentRef.current.offsetWidth);

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        updateColumns(entry.contentRect.width);
      }
    });

    resizeObserver.observe(parentRef.current);
    return () => resizeObserver.disconnect();
  }, []);

  const rowCount = Math.ceil(filteredArtists.length / columns);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => VIRTUAL_GRID_ROW_HEIGHT,
    overscan: 3,
  });

  if (artists.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Users className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('artist.noArtists', 'No artists found')}</p>
        <p className="text-sm mt-1">{t('artist.noArtistsHint', 'Import music to see your artists')}</p>
      </div>
    );
  }

  if (filteredArtists.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Search className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('library.noSearchResults', 'No results found')}</p>
        <p className="text-sm mt-1">{t('library.tryDifferentSearch', 'Try a different search term')}</p>
      </div>
    );
  }

  return (
    <div
      ref={parentRef}
      className="h-full overflow-auto"
    >
      <div
        style={{
          height: `${rowVirtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const startIndex = virtualRow.index * columns;
          const rowArtists = filteredArtists.slice(startIndex, startIndex + columns);

          return (
            <div
              key={virtualRow.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
                padding: '0 0 16px 0',
              }}
            >
              <div
                className="grid gap-4 h-full"
                style={{
                  gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                }}
              >
                {rowArtists.map((artist) => (
                  <div
                    key={artist.id}
                    onClick={() => onNavigate(artist.id)}
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
            </div>
          );
        })}
      </div>
    </div>
  );
}

// Virtualized Playlist Grid Component
function VirtualizedPlaylistGrid({
  playlists,
  searchQuery,
  onNavigate,
  onCreatePlaylist
}: {
  playlists: Playlist[];
  searchQuery: string;
  onNavigate: (id: string) => void;
  onCreatePlaylist: () => void;
}) {
  const { t } = useTranslation();
  const parentRef = useRef<HTMLDivElement>(null);

  // Filter playlists by search
  const filteredPlaylists = useMemo(() => {
    if (!searchQuery.trim()) return playlists;
    const query = searchQuery.toLowerCase();
    return playlists.filter(playlist =>
      playlist.name.toLowerCase().includes(query) ||
      (playlist.description?.toLowerCase().includes(query))
    );
  }, [playlists, searchQuery]);

  // Calculate columns based on container width using ResizeObserver
  const [columns, setColumns] = useState(5);

  useEffect(() => {
    if (!parentRef.current) return;

    const updateColumns = (width: number) => {
      if (width >= 1024) setColumns(5);
      else if (width >= 768) setColumns(4);
      else if (width >= 640) setColumns(3);
      else setColumns(2);
    };

    updateColumns(parentRef.current.offsetWidth);

    const resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        updateColumns(entry.contentRect.width);
      }
    });

    resizeObserver.observe(parentRef.current);
    return () => resizeObserver.disconnect();
  }, []);

  const rowCount = Math.ceil(filteredPlaylists.length / columns);

  const rowVirtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 100, // Playlist card height
    overscan: 3,
  });

  if (playlists.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <ListMusic className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('playlist.noPlaylists', 'No playlists yet')}</p>
        <p className="text-sm mt-1">{t('playlist.createHint', 'Create a playlist to organize your music')}</p>
        <button
          onClick={onCreatePlaylist}
          className="mt-4 flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
        >
          <Plus className="w-4 h-4" />
          {t('playlist.create', 'Create Playlist')}
        </button>
      </div>
    );
  }

  if (filteredPlaylists.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Search className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('library.noSearchResults', 'No results found')}</p>
        <p className="text-sm mt-1">{t('library.tryDifferentSearch', 'Try a different search term')}</p>
      </div>
    );
  }

  return (
    <div
      ref={parentRef}
      className="h-full overflow-auto"
    >
      <div
        style={{
          height: `${rowVirtualizer.getTotalSize()}px`,
          width: '100%',
          position: 'relative',
        }}
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const startIndex = virtualRow.index * columns;
          const rowPlaylists = filteredPlaylists.slice(startIndex, startIndex + columns);

          return (
            <div
              key={virtualRow.key}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                height: `${virtualRow.size}px`,
                transform: `translateY(${virtualRow.start}px)`,
                padding: '0 0 16px 0',
              }}
            >
              <div
                className="grid gap-4 h-full"
                style={{
                  gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
                }}
              >
                {rowPlaylists.map((playlist) => (
                  <div
                    key={playlist.id}
                    onClick={() => onNavigate(playlist.id)}
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
            </div>
          );
        })}
      </div>
    </div>
  );
}

// Virtualized Track List Component
function VirtualizedTrackList({
  tracks,
  searchQuery,
  filters,
  buildQueue,
  onTrackAction,
  renderMenu,
}: {
  tracks: DesktopTrack[];
  searchQuery: string;
  filters: LibraryFilters;
  buildQueue: (tracks: Track[], clickedTrack: Track, clickedIndex: number) => QueueTrack[];
  onTrackAction: (track: Track) => void;
  renderMenu: (track: Track) => React.ReactNode;
}) {
  const { t } = useTranslation();

  // Filter tracks by search and filters
  const filteredTracks = useMemo(() => {
    let result = tracks;

    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(track =>
        track.title?.toLowerCase().includes(query) ||
        track.artist_name?.toLowerCase().includes(query) ||
        track.album_title?.toLowerCase().includes(query)
      );
    }

    // Apply source type filter
    if (filters.sourceType !== 'all') {
      result = result.filter(track => track.source_type === filters.sourceType);
    }

    // Apply format filter
    if (filters.format !== 'all') {
      result = result.filter(track => track.file_format?.toUpperCase() === filters.format);
    }

    return result;
  }, [tracks, searchQuery, filters]);

  const mappedTracks = useMemo(() => filteredTracks.map(t => ({
    id: t.id,
    title: String(t.title || 'Unknown'),
    artist: t.artist_name,
    album: t.album_title,
    duration: t.duration_seconds,
    trackNumber: t.trackNumber,
    isAvailable: !!t.file_path,
    format: t.file_format,
    bitrate: t.bit_rate,
    sampleRate: t.sample_rate,
    channels: t.channels,
    sourceType: t.source_type || 'local',
    sourceName: t.source_name,
    sourceOnline: t.source_online,
  })), [filteredTracks]);

  if (tracks.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Music className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('library.noTracks', 'No tracks in your library')}</p>
        <p className="text-sm mt-1">{t('library.addTracks', 'Add some music to get started')}</p>
      </div>
    );
  }

  if (filteredTracks.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
        <Search className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('library.noSearchResults', 'No results found')}</p>
        <p className="text-sm mt-1">{t('library.tryDifferentSearch', 'Try a different search term')}</p>
      </div>
    );
  }

  return (
    <TrackList
      tracks={mappedTracks}
      buildQueue={buildQueue}
      onTrackAction={onTrackAction}
      renderMenu={renderMenu}
      virtualized={true}
      virtualItemSize={VIRTUAL_ITEM_SIZE}
    />
  );
}

export function LibraryPage() {
  const { t } = useTranslation();
  const [searchParams, setSearchParams] = useSearchParams();
  const tabParam = searchParams.get('tab') as TabId | null;
  const [activeTab, setActiveTab] = useState<TabId>(tabParam || 'albums');

  const navigate = useNavigate();
  const [tracks, setTracks] = useState<DesktopTrack[]>([]);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [artists, setArtists] = useState<Artist[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [healthWarning, setHealthWarning] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<DesktopTrack | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const { setSyncRequired } = useSyncStore();

  // Per-tab search state
  const [tabSearchQueries, setTabSearchQueries] = useState<Record<TabId, string>>({
    albums: '',
    playlists: '',
    artists: '',
    tracks: '',
  });

  // Filters state (only for tracks tab)
  const [filters, setFilters] = useState<LibraryFilters>({
    sourceType: 'all',
    format: 'all',
  });
  const [showFilters, setShowFilters] = useState(false);

  // Get current tab's search query
  const currentSearchQuery = tabSearchQueries[activeTab];

  // Update search query for current tab
  const setCurrentSearchQuery = (query: string) => {
    setTabSearchQueries(prev => ({ ...prev, [activeTab]: query }));
  };

  // Compute unique formats and sources for filter dropdowns
  const availableFormats = useMemo(() => {
    const formats = new Set<string>();
    tracks.forEach(t => {
      if (t.file_format) formats.add(t.file_format.toUpperCase());
    });
    return Array.from(formats).sort();
  }, [tracks]);

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
    if (tabId === 'albums') {
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
      const [tracksData, albumsData, artistsData, playlistsData, health] = await Promise.all([
        invoke<DesktopTrack[]>('get_all_tracks'),
        invoke<Album[]>('get_all_albums'),
        invoke<Artist[]>('get_all_artists'),
        invoke<Playlist[]>('get_all_playlists'),
        invoke<DatabaseHealth>('check_database_health'),
      ]);

      setTracks(tracksData);
      setAlbums(albumsData);
      setArtists(artistsData);
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
    // Filter tracks based on current search and filters for tracks tab
    let filteredTracks = tracks;

    if (tabSearchQueries.tracks.trim()) {
      const query = tabSearchQueries.tracks.toLowerCase();
      filteredTracks = filteredTracks.filter(track =>
        track.title?.toLowerCase().includes(query) ||
        track.artist_name?.toLowerCase().includes(query) ||
        track.album_title?.toLowerCase().includes(query)
      );
    }

    if (filters.sourceType !== 'all') {
      filteredTracks = filteredTracks.filter(track => track.source_type === filters.sourceType);
    }

    if (filters.format !== 'all') {
      filteredTracks = filteredTracks.filter(track => track.file_format?.toUpperCase() === filters.format);
    }

    console.log('[LibraryPage] buildQueue called:', {
      clickedTrack: clickedTrack.title,
      totalTracks: filteredTracks.length,
    });

    // Filter out tracks without file paths
    const validTracks = filteredTracks.filter((t) => t.file_path);
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
  }, [tracks, tabSearchQueries.tracks, filters]);

  const handleTrackPlay = (track: Track) => {
    // Playback state will be updated via backend events
    console.log('[LibraryPage] Playing track:', track.title);
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

  const handleCreatePlaylist = async () => {
    try {
      const newPlaylist = await invoke<Playlist>('create_playlist', {
        name: t('playlist.newPlaylistName', 'New Playlist'),
        description: null,
      });
      navigate(`/playlists/${newPlaylist.id}`);
    } catch (err) {
      console.error('Failed to create playlist:', err);
    }
  };

  // Get tab counts for display
  const getTabCount = (tabId: TabId): number => {
    switch (tabId) {
      case 'albums': return albums.length;
      case 'playlists': return playlists.length;
      case 'artists': return artists.length;
      case 'tracks': return tracks.length;
      default: return 0;
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4"></div>
          <p className="text-muted-foreground">{t('common.loading', 'Loading...')}</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p className="font-medium mb-2">{t('library.loadFailed', 'Failed to load library')}</p>
          <p className="text-sm">{error}</p>
          <button
            onClick={loadLibrary}
            className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            {t('common.retry', 'Retry')}
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
                {t('library.databaseIssue', 'Database Issue Detected')}
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
            {albums.length} {t('library.albums', 'albums')} • {artists.length} {t('library.artists', 'artists')} • {tracks.length} {t('library.tracks', 'tracks')}
          </p>
        </div>
      </div>

      {/* Tab Navigation */}
      <div className="flex items-center gap-4 mb-6">
        <div className="flex items-center gap-1 bg-muted rounded-lg p-1 w-fit">
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
              <span className="text-sm font-medium">{t(`library.tab.${tab.id}`, tab.label)}</span>
              <span className="text-xs text-muted-foreground">({getTabCount(tab.id)})</span>
            </button>
          ))}
        </div>
      </div>

      {/* Search and Filters Bar */}
      <div className="flex items-center gap-4 mb-4">
        {/* Search input */}
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="text"
            value={currentSearchQuery}
            onChange={(e) => setCurrentSearchQuery(e.target.value)}
            placeholder={t(`library.search.${activeTab}`, `Search ${activeTab}...`)}
            className="w-full pl-10 pr-4 py-2 rounded-lg bg-muted border border-transparent focus:border-primary focus:outline-none text-sm"
          />
          {currentSearchQuery && (
            <button
              onClick={() => setCurrentSearchQuery('')}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>

        {/* Filters button (only for tracks tab) */}
        {activeTab === 'tracks' && (
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
        )}
      </div>

      {/* Filters Panel (only for tracks tab) */}
      {activeTab === 'tracks' && showFilters && (
        <div className="mb-4 p-4 bg-muted/50 rounded-lg border">
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
          </div>
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {activeTab === 'albums' && (
          <VirtualizedAlbumGrid albums={albums} searchQuery={tabSearchQueries.albums} />
        )}

        {activeTab === 'playlists' && (
          <VirtualizedPlaylistGrid
            playlists={playlists}
            searchQuery={tabSearchQueries.playlists}
            onNavigate={(id) => navigate(`/playlists/${id}`)}
            onCreatePlaylist={handleCreatePlaylist}
          />
        )}

        {activeTab === 'artists' && (
          <VirtualizedArtistGrid
            artists={artists}
            searchQuery={tabSearchQueries.artists}
            onNavigate={(id) => navigate(`/artists/${id}`)}
          />
        )}

        {activeTab === 'tracks' && (
          <VirtualizedTrackList
            tracks={tracks}
            searchQuery={tabSearchQueries.tracks}
            filters={filters}
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
      </div>

      {/* Delete confirmation dialog */}
      <ConfirmDialog
        open={!!confirmDelete}
        title={t('library.deleteTrack', 'Delete Track')}
        message={t('library.deleteTrackConfirm', `Are you sure you want to delete "${confirmDelete?.title}"? This will remove the track from your library.`) + (deleteError ? `\n\n${t('common.error', 'Error')}: ${deleteError}` : '')}
        confirmText={t('common.delete', 'Delete')}
        confirmVariant="danger"
        onConfirm={handleConfirmDelete}
        onClose={() => setConfirmDelete(null)}
        isLoading={isDeleting}
      />
    </div>
  );
}
