import { useState, useMemo, useDeferredValue, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { Music } from 'lucide-react'
import { useBackend, type BackendGenre } from '../contexts/BackendContext'
import { LibraryPageLayout } from '../components/LibraryPageLayout'
import { cn } from '../lib/utils'

export function GenresListPage() {
  const { t } = useTranslation()
  const backend = useBackend()
  const navigate = useNavigate()
  const [genres, setGenres] = useState<BackendGenre[]>([])
  const [isLoading, setIsLoading] = useState(true)

  const [searchQuery, setSearchQuery] = useState('')
  const deferredSearchQuery = useDeferredValue(searchQuery)

  useEffect(() => {
    backend.getAllGenres()
      .then(setGenres)
      .catch(() => setGenres([]))
      .finally(() => setIsLoading(false))
  }, [backend])

  const filteredGenres = useMemo(() => {
    if (!deferredSearchQuery.trim()) return genres
    const query = deferredSearchQuery.toLowerCase()
    return genres.filter(g => g.name.toLowerCase().includes(query))
  }, [genres, deferredSearchQuery])

  return (
    <LibraryPageLayout
      searchQuery={searchQuery}
      setSearchQuery={setSearchQuery}
      itemCount={genres.length}
      searchPlaceholderKey="library.search.genresWithCount"
      isLoading={isLoading}
      itemType="album"
      gridClass="grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5"
      cacheKey="library-genres-count"
      pageTestId="genres-page"
    >
      {filteredGenres.length > 0 ? (
        <div className="grid gap-3 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
          {filteredGenres.map((genre) => (
            <button
              key={genre.id}
              data-testid={`genre-card-${genre.id}`}
              onClick={() => navigate(`/genres/${genre.id}`)}
              className={cn(
                'text-left p-4 rounded-lg bg-muted hover:bg-muted/80 transition-opacity',
                'hover:opacity-[var(--hover-text-opacity)]'
              )}
            >
              <p className="font-semibold text-foreground truncate">{genre.name}</p>
              <p className="text-sm text-muted-foreground mt-1">
                {t('genre.trackCount', { count: genre.track_count })}
              </p>
            </button>
          ))}
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <Music className="w-12 h-12 mb-4 opacity-50" />
          <p className="font-medium">
            {searchQuery ? t('library.noSearchResults') : t('genre.noGenres')}
          </p>
          {searchQuery ? (
            <p className="text-sm mt-1">{t('library.tryDifferentSearch')}</p>
          ) : (
            <p className="text-sm mt-1">{t('genre.noGenresHint')}</p>
          )}
        </div>
      )}
    </LibraryPageLayout>
  )
}
