/**
 * Shared AlbumCard component - wrapper around MediaCard
 * Maintains backward compatibility with existing usage
 */

import { memo } from 'react'
import { MediaCard } from './MediaCard'

export interface AlbumCardAlbum {
  id: number
  title: string
  artist_name?: string
  artist_id?: number
  year?: number
  cover_art_path?: string
  coverUrl?: string
}

interface AlbumCardProps {
  album: AlbumCardAlbum
  /** Card width class (default: w-full for responsive grid) */
  className?: string
  /** Show artist and year below title */
  showArtist?: boolean
  /** Priority: if true, loads artwork immediately without lazy loading. Use for above-the-fold items (first ~20-30 items) */
  priority?: boolean
}

const AlbumCardComponent = ({ album, className = 'w-full', showArtist = true, priority = false }: AlbumCardProps) => {
  return (
    <MediaCard
      type="album"
      id={album.id}
      title={album.title}
      subtitle={showArtist ? album.artist_name : undefined}
      artistId={showArtist ? album.artist_id : undefined}
      additionalInfo={showArtist && album.year ? String(album.year) : undefined}
      coverUrl={album.coverUrl || album.cover_art_path}
      className={className}
      priority={priority}
    />
  )
}

export const AlbumCard = memo(AlbumCardComponent)
AlbumCard.displayName = 'AlbumCard'
