/**
 * AlbumsPage - displays all albums with search and grid scaling
 */

import { useState, useMemo, useDeferredValue } from 'react'
import { useTranslation } from 'react-i18next'
import { Disc3 } from 'lucide-react'
import { AlbumCard } from '../components/AlbumCard'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { SkeletonGrid } from '../components/SkeletonGrid'
import { VirtualizedGrid } from '../components/VirtualizedGrid'
import { useGridScale } from '../hooks/useGridScale'
import { useResponsiveColumns } from '../hooks/useResponsiveColumns'
import { useAlbums } from '../hooks/queries/useAlbumQueries'
import { useDatabaseHealth } from '../hooks/queries/useLibraryQueries'

export function AlbumsPage() {
  const { t } = useTranslation()
  const { scale } = useGridScale()
  const columnCount = useResponsiveColumns(scale)

  const [searchQuery, setSearchQuery] = useState('')
  const deferredSearchQuery = useDeferredValue(searchQuery)

  // Fetch data using React Query hooks
  const { data: albums = [], isLoading, isError, error } = useAlbums()
  const { data: health } = useDatabaseHealth()

  // Health warning from database health check
  const healthWarning = health?.issues.length ? health.issues.join(' ') : null

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
        return 'grid-cols-2 sm:grid-cols-3 md:grid-cols-5 lg:grid-cols-7 xl:grid-cols-8'
      case 1:
        return 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
      case 1.25:
        return 'grid-cols-2 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5'
      case 1.5:
        return 'grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-3 xl:grid-cols-4'
      default:
        return 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
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

  // Show error in LibraryPageLayout if present
  const errorContent = isError ? (
    <div className="flex items-center justify-center py-12">
      <div className="text-center text-destructive">
        <p className="font-medium mb-2">{t('library.loadFailed')}</p>
        <p className="text-sm">{error instanceof Error ? error.message : 'Failed to load albums'}</p>
      </div>
    </div>
  ) : null

  return (
    <LibraryPageLayout
      searchQuery={searchQuery}
      setSearchQuery={setSearchQuery}
      itemCount={albums.length}
      searchPlaceholderKey="library.search.albumsWithCount"
      healthWarning={healthWarning}
      isLoading={isLoading}
      itemType="album"
      gridClass={gridClass}
      cacheKey="library-albums-count"
    >
      {isLoading ? (
        <SkeletonGrid count={24} type="album" gridClass={gridClass} />
      ) : errorContent || (filteredAlbums.length > 0 ? (
        shouldVirtualize ? (
          <VirtualizedGrid
            items={filteredAlbums}
            totalCount={filteredAlbums.length}
            columnCount={columnCount}
            rowHeight={rowHeight}
            gridClass={gridClass}
            renderItem={(album, index) => (
              <AlbumCard
                album={{
                  id: album.id,
                  title: album.title,
                  artist_name: album.artist_name,
                  artist_id: album.artist_id,
                  year: album.year,
                  cover_art_path: album.cover_art_path,
                }}
                showArtist={true}
                className="w-full"
                priority={index < 24}
              />
            )}
          />
        ) : (
          <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
            {filteredAlbums.map((album, index) => (
              <AlbumCard
                key={album.id}
                album={{
                  id: album.id,
                  title: album.title,
                  artist_name: album.artist_name,
                  artist_id: album.artist_id,
                  year: album.year,
                  cover_art_path: album.cover_art_path,
                }}
                showArtist={true}
                className="w-full"
                priority={index < 24}
              />
            ))}
          </div>
        )
      ) : (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
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
  )
}
