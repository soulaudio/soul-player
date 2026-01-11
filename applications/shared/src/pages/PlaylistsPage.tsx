/**
 * PlaylistsPage - displays all playlists with search and grid scaling
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { ListMusic, Search, X, Plus } from 'lucide-react'
import { PlaylistCard } from '../components/PlaylistCard'
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

  // Grid columns based on scale
  const gridClass = useMemo(() => {
    switch (scale) {
      case 0.75:
        return 'grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-7 xl:grid-cols-8'
      case 1:
        return 'grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
      case 1.25:
        return 'grid-cols-2 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5'
      case 1.5:
        return 'grid-cols-1 sm:grid-cols-2 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4'
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

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto mb-4"></div>
          <p className="text-muted-foreground">{t('common.loading')}</p>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
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
    )
  }

  return (
    <div className="h-full flex flex-col">
      {/* Health warning */}
      <FeatureGate feature="hasHealthCheck">
        {healthWarning && (
          <div className="mb-4 p-4 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-5 h-5 rounded-full bg-yellow-500/20 flex items-center justify-center mt-0.5">
                <span className="text-yellow-600 dark:text-yellow-400 text-sm">!</span>
              </div>
              <div className="flex-1">
                <p className="text-sm text-yellow-800 dark:text-yellow-200 font-medium">
                  {t('library.databaseIssue')}
                </p>
                <p className="text-sm text-yellow-700 dark:text-yellow-300 mt-1">
                  {healthWarning}
                </p>
              </div>
            </div>
          </div>
        )}
      </FeatureGate>

      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-2xl sm:text-3xl font-bold">{t('library.tab.playlists')}</h1>
          <p className="text-muted-foreground text-sm mt-1">
            {playlists.length} {t('library.playlists')}
          </p>
        </div>
        <FeatureGate feature="canCreatePlaylists">
          <button
            onClick={handleCreatePlaylist}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
          >
            <Plus className="w-4 h-4" />
            <span className="hidden sm:inline">{t('playlist.create')}</span>
          </button>
        </FeatureGate>
      </div>

      {/* Search */}
      <div className="flex items-center gap-4 mb-4">
        <div className="relative flex-1 sm:max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t('library.search.playlists')}
            className="w-full pl-10 pr-4 py-2 rounded-lg bg-muted border border-transparent focus:border-primary focus:outline-none text-sm"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery('')}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
              <X className="w-4 h-4" />
            </button>
          )}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {filteredPlaylists.length > 0 ? (
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
        )}
      </div>
    </div>
  )
}
