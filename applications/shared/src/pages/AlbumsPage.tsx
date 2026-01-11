/**
 * AlbumsPage - displays all albums with search and grid scaling
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Disc3, Search, X } from 'lucide-react'
import { AlbumCard } from '../components/AlbumCard'
import { FeatureGate } from '../contexts/PlatformContext'
import { useBackend, type BackendAlbum } from '../contexts/BackendContext'
import { useGridScale } from '../hooks/useGridScale'

export function AlbumsPage() {
  const { t } = useTranslation()
  const backend = useBackend()
  const { scale, scaleUp, scaleDown } = useGridScale()

  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [albums, setAlbums] = useState<BackendAlbum[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [healthWarning, setHealthWarning] = useState<string | null>(null)

  // Keyboard shortcut for grid scaling
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ignore if in input field
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

  // Load albums
  const loadAlbums = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    setHealthWarning(null)
    try {
      const [albumsData, health] = await Promise.all([
        backend.getAllAlbums(),
        backend.checkDatabaseHealth(),
      ])
      setAlbums(albumsData)
      if (health.issues.length > 0) {
        setHealthWarning(health.issues.join(' '))
      }
    } catch (err) {
      console.error('Failed to load albums:', err)
      setError(err instanceof Error ? err.message : 'Failed to load albums')
    } finally {
      setIsLoading(false)
    }
  }, [backend])

  useEffect(() => {
    loadAlbums()
  }, [loadAlbums])

  // Filter albums by search
  const filteredAlbums = useMemo(() => {
    if (!searchQuery.trim()) return albums
    const query = searchQuery.toLowerCase()
    return albums.filter(
      a =>
        a.title.toLowerCase().includes(query) ||
        (a.artist_name || '').toLowerCase().includes(query)
    )
  }, [albums, searchQuery])

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
            onClick={loadAlbums}
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
      <div className="mb-4">
        <h1 className="text-2xl sm:text-3xl font-bold">{t('library.tab.albums')}</h1>
        <p className="text-muted-foreground text-sm mt-1">
          {albums.length} {t('library.albums')}
        </p>
      </div>

      {/* Search */}
      <div className="flex items-center gap-4 mb-4">
        <div className="relative flex-1 sm:max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t('library.search.albums')}
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
        {filteredAlbums.length > 0 ? (
          <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
            {filteredAlbums.map((album) => (
              <AlbumCard
                key={album.id}
                album={{
                  id: album.id,
                  title: album.title,
                  artist_name: album.artist_name,
                  year: album.year,
                  cover_art_path: album.cover_art_path,
                }}
                showArtist={true}
                className="w-full"
              />
            ))}
          </div>
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
        )}
      </div>
    </div>
  )
}
