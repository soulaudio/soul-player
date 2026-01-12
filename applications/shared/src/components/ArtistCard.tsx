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
}

export function ArtistCard({ artist, className = 'w-full' }: ArtistCardProps) {
  const { t } = useTranslation()

  return (
    <MediaCard
      type="artist"
      id={artist.id}
      title={artist.name}
      subtitle={`${artist.album_count} ${t('library.albums')} • ${artist.track_count} ${t('library.tracks')}`}
      coverUrl={artist.cover_art_path ?? undefined}
      className={className}
    />
  )
}
