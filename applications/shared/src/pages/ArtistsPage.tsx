/**
 * ArtistsPage - displays all artists with search and grid scaling
 */

import { useState, useMemo, useDeferredValue } from 'react'
import { useTranslation } from 'react-i18next'
import { Users } from 'lucide-react'
import { ArtistCard } from '../components/ArtistCard'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { VirtualizedGrid } from '../components/VirtualizedGrid'
import { useGridScale } from '../hooks/useGridScale'
import { useResponsiveColumns } from '../hooks/useResponsiveColumns'
import { useArtists } from '../hooks/queries/useArtistQueries'
import { useDatabaseHealth } from '../hooks/queries/useLibraryQueries'

export function ArtistsPage() {
  const { t } = useTranslation()
  const { scale } = useGridScale()
  const columnCount = useResponsiveColumns(scale)

  const [searchQuery, setSearchQuery] = useState('')
  const deferredSearchQuery = useDeferredValue(searchQuery)

  // Fetch data using React Query hooks
  const { data: artists = [], isLoading, isError, error } = useArtists()
  const { data: health } = useDatabaseHealth()

  // Health warning from database health check
  const healthWarning = health?.issues.length ? health.issues.join(' ') : null

  // Filter artists by search
  const filteredArtists = useMemo(() => {
    if (!deferredSearchQuery.trim()) return artists
    const query = deferredSearchQuery.toLowerCase()
    return artists.filter(a => a.name.toLowerCase().includes(query))
  }, [artists, deferredSearchQuery])

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
  const shouldVirtualize = filteredArtists.length > 100

  // Show error if present
  const errorContent = isError ? (
    <div className="flex items-center justify-center py-12">
      <div className="text-center text-destructive">
        <p className="font-medium mb-2">{t('library.loadFailed')}</p>
        <p className="text-sm">{error instanceof Error ? error.message : 'Failed to load artists'}</p>
      </div>
    </div>
  ) : null

  return (
    <LibraryPageLayout
      searchQuery={searchQuery}
      setSearchQuery={setSearchQuery}
      itemCount={artists.length}
      searchPlaceholderKey="library.search.artistsWithCount"
      healthWarning={healthWarning}
      isLoading={isLoading}
      itemType="artist"
      gridClass={gridClass}
      cacheKey="library-artists-count"
    >
      {errorContent || (filteredArtists.length > 0 ? (
        shouldVirtualize ? (
          <VirtualizedGrid
            items={filteredArtists}
            totalCount={filteredArtists.length}
            columnCount={columnCount}
            rowHeight={rowHeight}
            gridClass={gridClass}
            renderItem={(artist, index) => (
              <ArtistCard
                artist={artist}
                priority={index < 24}
              />
            )}
          />
        ) : (
          <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
            {filteredArtists.map((artist, index) => (
              <ArtistCard
                key={artist.id}
                artist={artist}
                priority={index < 24}
              />
            ))}
          </div>
        )
      ) : (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Users className="w-12 h-12 mb-4 opacity-50" />
          <p className="font-medium">
            {searchQuery ? t('library.noSearchResults') : t('artist.noArtists')}
          </p>
          <p className="text-sm mt-1">
            {searchQuery ? t('library.tryDifferentSearch') : t('artist.noArtistsHint')}
          </p>
        </div>
      ))}
    </LibraryPageLayout>
  )
}
