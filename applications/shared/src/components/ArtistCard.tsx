/**
 * Shared ArtistCard component - wrapper around MediaCard
 * Maintains backward compatibility with existing usage
 */

import { useTranslation } from 'react-i18next'
import { MediaCard } from './MediaCard'
import { type BackendArtist } from '../contexts/BackendContext'

interface ArtistCardProps {
  artist: BackendArtist
  /** Card width class (default: w-full for responsive grid) */
  className?: string
  /** Priority: if true, loads artwork immediately without lazy loading. Use for above-the-fold items (first ~20-30 items) */
  priority?: boolean
  onAddToPlaylist?: () => void
}

export function ArtistCard({ artist, className = 'w-full', priority = false, onAddToPlaylist }: ArtistCardProps) {
  const { t } = useTranslation()

  return (
    <MediaCard
      type="artist"
      id={artist.id}
      title={artist.name}
      subtitle={`${artist.album_count} ${t('library.albums')} • ${artist.track_count} ${t('library.tracks')}`}
      coverUrl={artist.cover_art_path ?? undefined}
      className={className}
      priority={priority}
      onAddToPlaylist={onAddToPlaylist}
    />
  )
}
