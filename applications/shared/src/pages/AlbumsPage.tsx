/**
 * AlbumsPage - displays all albums with search and grid scaling
 */

import { useState, useMemo, useDeferredValue, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Disc3, SlidersHorizontal, X } from 'lucide-react'
import { AlbumCard } from '../components/AlbumCard'
import { AddToPlaylistDialog } from '../components/AddToPlaylistDialog'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { VirtualizedGrid } from '../components/VirtualizedGrid'
import { useGridScale } from '../hooks/useGridScale'
import { useResponsiveColumns } from '../hooks/useResponsiveColumns'
import { useAlbums } from '../hooks/queries/useAlbumQueries'
import { useDatabaseHealth } from '../hooks/queries/useLibraryQueries'
import { useBackend, type BackendGenre, type BackendAlbum } from '../contexts/BackendContext'
import { cn } from '../lib/utils'

export function AlbumsPage() {
  const { t } = useTranslation()
  const backend = useBackend()
  const { scale } = useGridScale()
  const columnCount = useResponsiveColumns(scale)

  const [searchQuery, setSearchQuery] = useState('')
  const deferredSearchQuery = useDeferredValue(searchQuery)

  const [entityForPlaylist, setEntityForPlaylist] = useState<{
    id: number | string
    name: string
  } | null>(null)

  const [genres, setGenres] = useState<BackendGenre[]>([])
  const [selectedGenreId, setSelectedGenreId] = useState<number | null>(null)
  const [showFilters, setShowFilters] = useState(false)

  useEffect(() => {
    backend.getAllGenres().then(setGenres).catch(() => {})
  }, [backend])

  // Fetch data using React Query hooks
  const { data: allAlbums = [], isLoading: albumsLoading, isError, error } = useAlbums()
  const [genreAlbums, setGenreAlbums] = useState<BackendAlbum[]>([])
  const [genreAlbumsLoading, setGenreAlbumsLoading] = useState(false)

  useEffect(() => {
    if (selectedGenreId === null) {
      setGenreAlbums([])
      return
    }
    setGenreAlbumsLoading(true)
    backend.getGenreAlbums(selectedGenreId, 1000)
      .then(setGenreAlbums)
      .catch(() => setGenreAlbums([]))
      .finally(() => setGenreAlbumsLoading(false))
  }, [selectedGenreId, backend])

  const albums = selectedGenreId !== null ? genreAlbums : allAlbums
  const isLoading = selectedGenreId !== null ? genreAlbumsLoading : albumsLoading

  const { data: health } = useDatabaseHealth()

  // Health warning from database health check
  const healthWarning = health?.issues.length ? health.issues.join(' ') : null

  const filterPanel = genres.length === 0 ? null : (
    <div className={`overflow-hidden transition-all duration-200 ${showFilters ? 'max-h-20 opacity-100' : 'max-h-0 opacity-0'}`}>
      <div className="flex flex-wrap gap-2 pt-2 pb-1">
        {genres.map((genre) => (
          <button
            key={genre.id}
            data-testid={`genre-chip-${genre.id}${selectedGenreId === genre.id ? '-active' : ''}`}
            onClick={() => setSelectedGenreId(selectedGenreId === genre.id ? null : genre.id)}
            className={cn(
              'px-3 py-1 rounded-full text-sm transition-all border',
              selectedGenreId === genre.id
                ? 'bg-primary text-primary-foreground border-primary'
                : 'bg-muted text-muted-foreground border-transparent hover:border-muted-foreground/30'
            )}
          >
            {genre.name}
          </button>
        ))}
      </div>
    </div>
  )

  const filtersButton = (
    <button
      data-testid="filter-toggle-button"
      onClick={() => {
        setShowFilters(v => !v)
        if (showFilters) setSelectedGenreId(null)
      }}
      className={cn(
        'flex items-center gap-1.5 px-3 py-2 rounded-lg text-sm transition-all border',
        showFilters || selectedGenreId !== null
          ? 'border-primary text-primary bg-primary/10'
          : 'border-transparent text-muted-foreground bg-muted hover:opacity-[var(--hover-text-opacity)]'
      )}
    >
      {selectedGenreId !== null ? <X className="w-3.5 h-3.5" /> : <SlidersHorizontal className="w-3.5 h-3.5" />}
      <span>Filters</span>
      {selectedGenreId !== null && (
        <span className="w-2 h-2 rounded-full bg-primary" />
      )}
    </button>
  )

  // Filter albums by search
  const filteredAlbums = useMemo(() => {
    if (!deferredSearchQuery.trim()) return albums
    const query = deferredSearchQuery.toLowerCase()
    return albums.filter(
      a =>
        a.title.toLowerCase().includes(query) ||
        (a.artist_name || '').toLowerCase().includes(query)
    )
  }, [albums, deferredSearchQuery])

  // Grid columns based on scale - responsive breakpoints to prevent overlap
  const gridClass = useMemo(() => {
    switch (scale) {
      case 0.75:
        return 'grid-cols-2 sm:grid-cols-3 md:grid-cols-5 lg:grid-cols-7 xl:grid-cols-8 2xl:grid-cols-10'
      case 1:
        return 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8'
      case 1.25:
        return 'grid-cols-2 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6'
      case 1.5:
        return 'grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5'
      default:
        return 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 2xl:grid-cols-8'
    }
  }, [scale])

  // Row height for virtualized grid (card height + gap, based on scale)
  const rowHeight = useMemo(() => {
    switch (scale) {
      case 0.75:
        return 220 // Smaller cards: ~200px card + 16px gap
      case 1:
        return 280 // Default: ~260px card + 16px gap
      case 1.25:
        return 340 // Medium: ~320px card + 16px gap
      case 1.5:
        return 400 // Larger: ~380px card + 16px gap
      default:
        return 280
    }
  }, [scale])

  // Use virtualization for large collections (>100 items)
  const shouldVirtualize = filteredAlbums.length > 100

  // Show error if present
  const errorContent = isError ? (
    <div className="flex items-center justify-center py-12">
      <div className="text-center text-destructive">
        <p className="font-medium mb-2">{t('library.loadFailed')}</p>
        <p className="text-sm">{error instanceof Error ? error.message : 'Failed to load albums'}</p>
      </div>
    </div>
  ) : null

  return (
    <>
    <LibraryPageLayout
      searchQuery={searchQuery}
      setSearchQuery={setSearchQuery}
      itemCount={albums.length}
      searchPlaceholderKey="library.search.albumsWithCount"
      healthWarning={healthWarning}
      additionalButtons={filtersButton}
      filterPanel={filterPanel}
      filterPanelVisible={showFilters}
      isLoading={isLoading}
      itemType="album"
      gridClass={gridClass}
      cacheKey="library-albums-count"
      pageTestId="albums-page"
    >
      {errorContent || (filteredAlbums.length > 0 ? (
        shouldVirtualize ? (
          <VirtualizedGrid
            items={filteredAlbums}
            totalCount={filteredAlbums.length}
            columnCount={columnCount}
            rowHeight={rowHeight}
            gridClass={gridClass}
            renderItem={(album, index) => (
              <AlbumCard
                album={album}
                showArtist={true}
                priority={index < 24}
                onAddToPlaylist={() => setEntityForPlaylist({ id: album.id, name: album.title })}
              />
            )}
          />
        ) : (
          <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
            {filteredAlbums.map((album, index) => (
              <AlbumCard
                key={album.id}
                album={album}
                showArtist={true}
                priority={index < 24}
                onAddToPlaylist={() => setEntityForPlaylist({ id: album.id, name: album.title })}
              />
            ))}
          </div>
        )
      ) : (
        <div data-testid="empty-state" className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Disc3 className="w-12 h-12 mb-4 opacity-50" />
          <p className="font-medium">
            {searchQuery ? t('library.noSearchResults') : t('library.noAlbums')}
          </p>
          <p className="text-sm mt-1">
            {searchQuery ? t('library.tryDifferentSearch') : t('library.noAlbumsHint')}
          </p>
        </div>
      ))}
    </LibraryPageLayout>
    {entityForPlaylist && (
      <AddToPlaylistDialog
        open={!!entityForPlaylist}
        onClose={() => setEntityForPlaylist(null)}
        mode="entity"
        entityType="album"
        entityId={entityForPlaylist.id}
        entityName={entityForPlaylist.name}
      />
    )}
    </>
  )
}
