/**
 * ArtistsPage - displays all artists with search and grid scaling
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Users, Search, X, Play } from 'lucide-react'
import { FeatureGate } from '../contexts/PlatformContext'
import { useBackend, type BackendArtist } from '../contexts/BackendContext'
import { useGridScale } from '../hooks/useGridScale'

// Artist Card Component
function ArtistCard({
  artist,
  onClick,
}: {
  artist: BackendArtist
  onClick: () => void
}) {
  const { t } = useTranslation()

  return (
    <div className="group cursor-pointer" onClick={onClick}>
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
          {artist.album_count} {t('library.albums')} • {artist.track_count} {t('library.tracks')}
        </p>
      </div>
    </div>
  )
}

export function ArtistsPage() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const backend = useBackend()
  const { scale, scaleUp, scaleDown } = useGridScale()

  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [artists, setArtists] = useState<BackendArtist[]>([])
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

  // Load artists
  const loadArtists = useCallback(async () => {
    setIsLoading(true)
    setError(null)
    setHealthWarning(null)
    try {
      const [artistsData, health] = await Promise.all([
        backend.getAllArtists(),
        backend.checkDatabaseHealth(),
      ])
      setArtists(artistsData)
      if (health.issues.length > 0) {
        setHealthWarning(health.issues.join(' '))
      }
    } catch (err) {
      console.error('Failed to load artists:', err)
      setError(err instanceof Error ? err.message : 'Failed to load artists')
    } finally {
      setIsLoading(false)
    }
  }, [backend])

  useEffect(() => {
    loadArtists()
  }, [loadArtists])

  // Filter artists by search
  const filteredArtists = useMemo(() => {
    if (!searchQuery.trim()) return artists
    const query = searchQuery.toLowerCase()
    return artists.filter(a => a.name.toLowerCase().includes(query))
  }, [artists, searchQuery])

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
            onClick={loadArtists}
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
        <h1 className="text-2xl sm:text-3xl font-bold">{t('library.tab.artists')}</h1>
        <p className="text-muted-foreground text-sm mt-1">
          {artists.length} {t('library.artists')}
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
            placeholder={t('library.search.artists')}
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
        {filteredArtists.length > 0 ? (
          <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
            {filteredArtists.map((artist) => (
              <ArtistCard
                key={artist.id}
                artist={artist}
                onClick={() => navigate(`/artists/${artist.id}`)}
              />
            ))}
          </div>
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
        )}
      </div>
    </div>
  )
}
