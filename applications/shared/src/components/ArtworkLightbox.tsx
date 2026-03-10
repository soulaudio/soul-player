/**
 * ArtworkLightbox — shared modal for viewing (and optionally editing) artwork.
 *
 * Always renders the image in a 1:1 square that fits the viewport.
 * Pass `onEditArtwork` to show the "Edit Artwork" button below the image (desktop only).
 */

import { useState } from 'react'
import { Pencil } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Dialog } from './ui/Dialog'
import { ArtworkImage } from './ArtworkImage'
import { EditArtworkDialog } from './EditArtworkDialog'
import { usePlatform } from '../contexts/PlatformContext'

interface ArtworkLightboxProps {
  open: boolean
  onClose: () => void
  /** ArtworkImage entity props — pass whichever identifies the artwork */
  trackId?: string | number
  albumId?: number
  artistId?: number
  playlistId?: string
  /** Direct URL for non-Tauri environments (marketing demo) */
  coverArtPath?: string
  alt: string
  /** Increment to force a cache-bust after artwork is changed */
  cacheVersion?: number
  /** When provided, shows an "Edit Artwork" button that opens EditArtworkDialog */
  editArtwork?: {
    entityType: 'album' | 'artist' | 'playlist'
    entityId: string
    entityName: string
    currentArtworkUrl?: string | null
    onArtworkChanged?: () => void
  }
  'data-testid'?: string
}

export function ArtworkLightbox({
  open,
  onClose,
  trackId,
  albumId,
  artistId,
  playlistId,
  coverArtPath,
  alt,
  cacheVersion,
  editArtwork,
  'data-testid': testId,
}: ArtworkLightboxProps) {
  const { t } = useTranslation()
  const { isDesktop } = usePlatform()
  const [editOpen, setEditOpen] = useState(false)

  const handleEdit = () => {
    onClose()
    setEditOpen(true)
  }

  return (
    <>
      <Dialog open={open} onClose={onClose} data-testid={testId}>
        <div onClick={(e) => e.stopPropagation()} className="flex flex-col items-center gap-3">
          {/* Square artwork — min(85vh, 85vw) ensures it fits both axes */}
          <div
            className="rounded-xl overflow-hidden shadow-2xl bg-muted aspect-square"
            style={{ width: 'min(85vh, 85vw)' }}
          >
            <ArtworkImage
              key={cacheVersion}
              trackId={trackId}
              albumId={albumId}
              artistId={artistId}
              playlistId={playlistId}
              coverArtPath={coverArtPath}
              alt={alt}
              className="w-full h-full object-contain"
              fallbackClassName="w-full h-full flex items-center justify-center"
              priority
            />
          </div>

          {/* Edit button — desktop only, only when editArtwork prop is provided */}
          {isDesktop && editArtwork && (
            <button
              onClick={handleEdit}
              className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              <Pencil className="w-4 h-4" />
              <span>{t('artwork.editArtwork', 'Edit Artwork')}</span>
            </button>
          )}
        </div>
      </Dialog>

      {editArtwork && (
        <EditArtworkDialog
          open={editOpen}
          onClose={() => setEditOpen(false)}
          entityType={editArtwork.entityType}
          entityId={editArtwork.entityId}
          entityName={editArtwork.entityName}
          currentArtworkUrl={editArtwork.currentArtworkUrl}
          onArtworkChanged={editArtwork.onArtworkChanged}
        />
      )}
    </>
  )
}
