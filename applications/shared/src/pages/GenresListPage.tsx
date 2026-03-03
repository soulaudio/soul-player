import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router-dom'
import { Music } from 'lucide-react'
import { useBackend, type BackendGenre } from '../contexts/BackendContext'
import { cn } from '../lib/utils'

export function GenresListPage() {
  const { t } = useTranslation()
  const backend = useBackend()
  const navigate = useNavigate()
  const [genres, setGenres] = useState<BackendGenre[]>([])
  const [isLoading, setIsLoading] = useState(true)

  useEffect(() => {
    backend.getAllGenres()
      .then(setGenres)
      .catch(() => setGenres([]))
      .finally(() => setIsLoading(false))
  }, [backend])

  if (isLoading) {
    return (
      <div data-testid="genres-page" className="p-6">
        <div className="grid gap-3 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
          {Array.from({ length: 8 }).map((_, i) => (
            <div key={i} className="h-24 rounded-lg bg-muted animate-pulse" />
          ))}
        </div>
      </div>
    )
  }

  if (genres.length === 0) {
    return (
      <div data-testid="genres-page" className="flex flex-col items-center justify-center py-24 text-muted-foreground">
        <Music className="w-12 h-12 mb-4 opacity-50" />
        <p className="font-medium">{t('genre.noGenres')}</p>
      </div>
    )
  }

  return (
    <div data-testid="genres-page" className="p-6">
      <div className="grid gap-3 grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5">
        {genres.map((genre) => (
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
    </div>
  )
}
