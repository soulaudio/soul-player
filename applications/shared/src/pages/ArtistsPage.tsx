/**
 * ArtistsPage - displays all artists with search and grid scaling
 */

import { useState, useEffect, useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { Users } from 'lucide-react'
import { ArtistCard } from '../components/ArtistCard'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { useBackend, type BackendArtist } from '../contexts/BackendContext'
import { useGridScale } from '../hooks/useGridScale'

export function ArtistsPage() {
  const { t } = useTranslation()
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

  // Show error in LibraryPageLayout if present
  const errorContent = error ? (
    <div className="flex items-center justify-center py-12">
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
        <div className={`grid gap-3 sm:gap-4 ${gridClass}`}>
          {filteredArtists.map((artist, index) => (
            <ArtistCard
              key={artist.id}
              artist={artist}
              priority={index < 24}
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
      ))}
    </LibraryPageLayout>
  )
}
