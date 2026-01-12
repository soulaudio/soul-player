/**
 * PlaylistsPage - displays all playlists with search and grid scaling
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { ListMusic, Plus } from 'lucide-react'
import { PlaylistCard } from '../components/PlaylistCard'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { FeatureGate } from '../contexts/PlatformContext'
import { useBackend, type BackendPlaylist } from '../contexts/BackendContext'
import { useGridScale } from '../hooks/useGridScale'

export function PlaylistsPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const backend = useBackend()
  const { scale, scaleUp, scaleDown } = useGridScale()

  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [playlists, setPlaylists] = useState<BackendPlaylist[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [healthWarning, setHealthWarning] = useState<string | null>(null)

  // Keyboard shortcut for grid scaling
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return
      }

      if ((e.ctrlKey || e.metaKey) && (e.key === '=' || e.key === '+')) {
        e.preventDefault()
        scaleUp()
      } else if ((e.ctrlKey || e.metaKey) && e.key === '-') {
        e.preventDefault()
        scaleDown()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [scaleUp, scaleDown])

  // Load playlists
  const loadPlaylists = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    setHealthWarning(null)
    try {
      const [playlistsData, health] = await Promise.all([
        backend.getAllPlaylists(),
        backend.checkDatabaseHealth(),
      ])
      setPlaylists(playlistsData)
      if (health.issues.length > 0) {
        setHealthWarning(health.issues.join(' '))
      }
    } catch (err) {
      console.error('Failed to load playlists:', err)
      setError(err instanceof Error ? err.message : 'Failed to load playlists')
    } finally {
      setIsLoading(false)
    }
  }, [backend])

  useEffect(() => {
    loadPlaylists()
  }, [loadPlaylists])

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

  const handleCreatePlaylist = async () => {
    try {
      const playlist = await backend.createPlaylist(t('playlist.newPlaylistName', 'New Playlist'))
      navigate(`/playlists/${playlist.id}`)
    } catch (err) {
      console.error('Failed to create playlist:', err)
    }
  }

  // Show error in LibraryPageLayout if present
  const errorContent = error ? (
    <div className="flex items-center justify-center py-12">
      <div className="text-center text-destructive">
        <p className="font-medium mb-2">{t('library.loadFailed')}</p>
        <p className="text-sm">{error}</p>
        <button
          onClick={loadPlaylists}
          className="mt-4 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
        >
          {t('common.retry')}
        </button>
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
      {errorContent || (filteredPlaylists.length > 0 ? (
        <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
          {filteredPlaylists.map((playlist) => (
            <PlaylistCard
              key={playlist.id}
              playlist={playlist}
              className="w-full"
            />
          ))}
        </div>
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
