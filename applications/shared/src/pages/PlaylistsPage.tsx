/**
 * PlaylistsPage - displays all playlists with search and grid scaling
 */

import { useState, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigateWithHistory } from '../hooks/useNavigateWithHistory'
import { ListMusic, Plus } from 'lucide-react'
import { PlaylistCard } from '../components/PlaylistCard'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { SkeletonGrid } from '../components/SkeletonGrid'
import { VirtualizedGrid } from '../components/VirtualizedGrid'
import { FeatureGate } from '../contexts/PlatformContext'
import { useGridScale } from '../hooks/useGridScale'
import { useResponsiveColumns } from '../hooks/useResponsiveColumns'
import { usePlaylists } from '../hooks/queries/useLibraryQueries'
import { useDatabaseHealth } from '../hooks/queries/useLibraryQueries'
import { useCreatePlaylist } from '../hooks/queries/usePlaylistMutations'

export function PlaylistsPage() {
  const { t } = useTranslation()
  const { navigate } = useNavigateWithHistory()
  const { scale } = useGridScale()
  const columnCount = useResponsiveColumns(scale)
  const createPlaylistMutation = useCreatePlaylist()

  const [searchQuery, setSearchQuery] = useState('')

  // Fetch data using React Query hooks
  const { data: playlists = [], isLoading, isError, error } = usePlaylists()
  const { data: health } = useDatabaseHealth()

  // Health warning from database health check
  const healthWarning = health?.issues.length ? health.issues.join(' ') : null

  // Filter playlists by search
  const filteredPlaylists = useMemo(() => {
    if (!searchQuery.trim()) return playlists
    const query = searchQuery.toLowerCase()
    return playlists.filter(p => p.name.toLowerCase().includes(query))
  }, [playlists, searchQuery])

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
  const shouldVirtualize = filteredPlaylists.length > 100

  const handleCreatePlaylist = () => {
    createPlaylistMutation.mutate(
      { name: t('playlist.newPlaylistName', 'New Playlist') },
      {
        onSuccess: (playlist) => {
          navigate(`/playlists/${playlist.id}`)
        },
        onError: (err) => {
          console.error('Failed to create playlist:', err)
        },
      }
    )
  }

  // Show error in LibraryPageLayout if present
  const errorContent = isError ? (
    <div className="flex items-center justify-center py-12">
      <div className="text-center text-destructive">
        <p className="font-medium mb-2">{t('library.loadFailed')}</p>
        <p className="text-sm">{error instanceof Error ? error.message : 'Failed to load playlists'}</p>
      </div>
    </div>
  ) : null

  return (
    <LibraryPageLayout
      searchQuery={searchQuery}
      setSearchQuery={setSearchQuery}
      itemCount={playlists.length}
      searchPlaceholderKey="library.search.playlistsWithCount"
      healthWarning={healthWarning}
      isLoading={isLoading}
      itemType="playlist"
      gridClass={gridClass}
      cacheKey="library-playlists-count"
      additionalButtons={
        <FeatureGate feature="canCreatePlaylists">
          <button
            onClick={handleCreatePlaylist}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            <Plus className="w-4 h-4" />
            <span className="hidden sm:inline">{t('playlist.create')}</span>
          </button>
        </FeatureGate>
      }
    >
      {isLoading ? (
        <SkeletonGrid count={24} type="playlist" gridClass={gridClass} />
      ) : errorContent || (filteredPlaylists.length > 0 ? (
        shouldVirtualize ? (
          <VirtualizedGrid
            items={filteredPlaylists}
            totalCount={filteredPlaylists.length}
            columnCount={columnCount}
            rowHeight={rowHeight}
            gridClass={gridClass}
            renderItem={(playlist, index) => (
              <PlaylistCard
                playlist={playlist}
                className="w-full"
                priority={index < 24}
              />
            )}
          />
        ) : (
          <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
            {filteredPlaylists.map((playlist, index) => (
              <PlaylistCard
                key={playlist.id}
                playlist={playlist}
                className="w-full"
                priority={index < 24}
              />
            ))}
          </div>
        )
      ) : (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <ListMusic className="w-12 h-12 mb-4 opacity-50" />
          <p className="font-medium">
            {searchQuery ? t('library.noSearchResults') : t('playlist.noPlaylists')}
          </p>
          <p className="text-sm mt-1">
            {searchQuery ? t('library.tryDifferentSearch') : t('playlist.createHint')}
          </p>
          <FeatureGate feature="canCreatePlaylists">
            <button
              onClick={handleCreatePlaylist}
              className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
            >
              {t('playlist.create')}
            </button>
          </FeatureGate>
        </div>
      ))}
    </LibraryPageLayout>
  )
}
