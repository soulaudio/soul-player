/**
 * EditArtworkDialog - Dialog for editing album, artist, or playlist artwork
 */

import { useState, useCallback, useRef, useEffect } from 'react';
import { Upload, Trash2, Image as ImageIcon, Loader2 } from 'lucide-react';
import { useTranslation } from '../i18n';
import { useBackend } from '../contexts/BackendContext';
import { Dialog, DialogContent, DialogHeader, DialogBody, DialogFooter } from './ui/Dialog';
import { Button } from './ui/button';
import { ImageCropper } from './ImageCropper';
import { clearArtworkCache } from './ArtworkImage';
import { debug } from '../utils/debug';

export type ArtworkEntityType = 'album' | 'artist' | 'playlist';

interface EditArtworkDialogProps {
  open: boolean;
  onClose: () => void;
  entityType: ArtworkEntityType;
  entityId: string;
  entityName: string;
  currentArtworkUrl?: string | null;
  onArtworkChanged?: () => void;
}

type DialogState = 'select' | 'crop' | 'preview' | 'saving';

export function EditArtworkDialog({
  open,
  onClose,
  entityType,
  entityId,
  entityName,
  currentArtworkUrl,
  onArtworkChanged,
}: EditArtworkDialogProps) {
  const { t } = useTranslation();
  const backend = useBackend();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const [state, setState] = useState<DialogState>('select');
  const [selectedImage, setSelectedImage] = useState<{ base64: string; mimeType: string } | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);
  const [rawImageUrl, setRawImageUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Loaded artwork from backend (data URL + source info)
  const [loadedArtworkUrl, setLoadedArtworkUrl] = useState<string | null>(null);
  const [isCustomArtwork, setIsCustomArtwork] = useState<boolean>(false);
  const [loadingArtwork, setLoadingArtwork] = useState(false);

  // Load artwork from backend when dialog opens
  useEffect(() => {
    if (!open) {
      // Reset loaded artwork when dialog closes
      setLoadedArtworkUrl(null);
      setIsCustomArtwork(false);
      return;
    }

    let cancelled = false;

    async function loadArtwork() {
      setLoadingArtwork(true);
      try {
        let artworkResponse: { dataUrl: string; isCustom: boolean } | null = null;

        // Check if we're in Tauri environment
        if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
          const { invoke } = await import('@tauri-apps/api/core');

          if (entityType === 'album') {
            artworkResponse = await invoke<{ dataUrl: string; isCustom: boolean } | null>('get_album_artwork_with_source', {
              albumId: parseInt(entityId, 10)
            });
          } else if (entityType === 'artist') {
            artworkResponse = await invoke<{ dataUrl: string; isCustom: boolean } | null>('get_artist_artwork_with_source', {
              artistId: parseInt(entityId, 10)
            });
          } else if (entityType === 'playlist') {
            artworkResponse = await invoke<{ dataUrl: string; isCustom: boolean } | null>('get_playlist_artwork_with_source', {
              playlistId: entityId
            });
          }
        }

        if (!cancelled) {
          setLoadedArtworkUrl(artworkResponse?.dataUrl || null);
          setIsCustomArtwork(artworkResponse?.isCustom || false);
        }
      } catch (err) {
        debug.error('[EditArtworkDialog] Failed to load artwork:', err);
      } finally {
        if (!cancelled) {
          setLoadingArtwork(false);
        }
      }
    }

    loadArtwork();

    return () => {
      cancelled = true;
    };
  }, [open, entityType, entityId]);

  // Use loaded artwork, or fall back to prop if it looks like a data URL
  const displayArtworkUrl = loadedArtworkUrl ||
    (currentArtworkUrl?.startsWith('data:') ? currentArtworkUrl : null);
  const hasArtwork = !!displayArtworkUrl;

  const handleFileSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    // Validate file type
    if (!file.type.startsWith('image/')) {
      setError(t('artwork.errors.invalidFileType'));
      return;
    }

    // Validate file size (max 10MB)
    if (file.size > 10 * 1024 * 1024) {
      setError(t('artwork.errors.fileTooLarge'));
      return;
    }

    setError(null);

    // Read file as data URL for cropper
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result as string;
      setRawImageUrl(result);
      setState('crop');
    };
    reader.onerror = () => {
      setError(t('artwork.errors.readFailed'));
    };
    reader.readAsDataURL(file);

    // Reset input so same file can be selected again
    e.target.value = '';
  }, [t]);

  const handleCropCancel = useCallback(() => {
    setRawImageUrl(null);
    setState('select');
  }, []);

  // Helper to reset dialog state and close
  const closeAndReset = useCallback(() => {
    setState('select');
    setSelectedImage(null);
    setPreviewUrl(null);
    setRawImageUrl(null);
    setError(null);
    onClose();
  }, [onClose]);

  const handleSave = useCallback(async (base64: string, mimeType: string, writeToFiles: boolean, useSoulStorage?: boolean) => {
    setSaving(true);
    setState('saving');

    try {
      await backend.setArtwork({
        entityType,
        entityId,
        artworkBase64: base64,
        mimeType,
        writeToFiles: entityType === 'album' ? writeToFiles : undefined,
        useSoulStorage: entityType === 'album' ? useSoulStorage : undefined,
      });

      // Clear the frontend artwork cache for this entity
      clearArtworkCache(entityType, entityId);

      onArtworkChanged?.();
      closeAndReset();
    } catch (err) {
      debug.error('[EditArtworkDialog] Failed to save artwork:', err);
      setError(t('artwork.errors.saveFailed'));
      setState('select');
    } finally {
      setSaving(false);
    }
  }, [backend, entityType, entityId, onArtworkChanged, t, closeAndReset]);

  // Handle cropped image from ImageCropper
  const handleCropComplete = useCallback((croppedBase64: string) => {
    // Extract base64 data (remove data:image/xxx;base64, prefix)
    const base64Match = croppedBase64.match(/^data:([^;]+);base64,(.+)$/);
    if (base64Match) {
      setSelectedImage({
        mimeType: base64Match[1],
        base64: base64Match[2],
      });
      setPreviewUrl(croppedBase64);

      // Show preview state with Save button
      setState('preview');
    }
  }, []);

  const handleAlbumChoice = useCallback((storageMode: 'folder' | 'soul_storage' | 'both') => {
    if (selectedImage) {
      // Convert storage mode to writeToFiles boolean for backend compatibility
      const writeToFiles = storageMode === 'both';
      const useSoulStorage = storageMode === 'soul_storage';
      handleSave(selectedImage.base64, selectedImage.mimeType, writeToFiles, useSoulStorage);
    }
  }, [selectedImage, handleSave]);

  const handleRemove = useCallback(async () => {
    setSaving(true);
    try {
      await backend.removeArtwork(entityType, entityId);

      // Clear the frontend artwork cache for this entity
      clearArtworkCache(entityType, entityId);

      onArtworkChanged?.();
      closeAndReset();
    } catch (err) {
      debug.error('[EditArtworkDialog] Failed to remove artwork:', err);
      setError(t('artwork.errors.removeFailed'));
    } finally {
      setSaving(false);
    }
  }, [backend, entityType, entityId, onArtworkChanged, t, closeAndReset]);

  // Alias for JSX usage
  const handleClose = closeAndReset;

  const handleBrowseClick = useCallback(() => {
    fileInputRef.current?.click();
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const file = e.dataTransfer.files[0];
    if (file && fileInputRef.current) {
      // Create a synthetic event to reuse the file select handler
      const dataTransfer = new DataTransfer();
      dataTransfer.items.add(file);
      fileInputRef.current.files = dataTransfer.files;
      fileInputRef.current.dispatchEvent(new Event('change', { bubbles: true }));
    }
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
  }, []);

  // Render crop dialog
  if (state === 'crop' && rawImageUrl) {
    return (
      <Dialog open={open} onClose={handleClose}>
        <DialogContent className="max-w-md">
          <DialogHeader onClose={handleClose}>
            {t('artwork.edit.cropTitle')}
          </DialogHeader>
          <DialogBody>
            <ImageCropper
              imageSrc={rawImageUrl}
              cropShape={entityType === 'artist' ? 'circle' : 'square'}
              onCrop={handleCropComplete}
              onCancel={handleCropCancel}
            />
          </DialogBody>
        </DialogContent>
      </Dialog>
    );
  }

  // Render preview dialog with save options
  if (state === 'preview' && previewUrl) {
    return (
      <Dialog open={open} onClose={handleClose}>
        <DialogContent className="max-w-lg">
          <DialogHeader onClose={handleClose}>
            {t('artwork.edit.title')}
          </DialogHeader>
          <DialogBody>
            <div className="text-center mb-4">
              <span className="text-muted-foreground">{entityName}</span>
            </div>

            {/* Preview artwork */}
            <div className="flex justify-center mb-6">
              <div className={`relative w-32 h-32 bg-muted overflow-hidden ${entityType === 'artist' ? 'rounded-full' : 'rounded-lg'} shadow-lg`}>
                <img
                  src={previewUrl}
                  alt={entityName}
                  className="w-full h-full object-cover"
                />
              </div>
            </div>

            {/* For albums, show storage options */}
            {entityType === 'album' && (
              <>
                <p className="text-sm text-muted-foreground mb-4 text-center">
                  {t('artwork.writeChoice.description')}
                </p>
                <div className="space-y-3">
                  {/* Album folder only option */}
                  <button
                    onClick={() => handleAlbumChoice('folder')}
                    disabled={saving}
                    className="w-full p-4 text-left rounded-lg border border-border hover:border-primary hover:bg-primary/5 transition-colors disabled:opacity-[var(--disabled-opacity)]"
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="font-medium">{t('artwork.writeChoice.albumFolderOnly')}</div>
                        <div className="text-sm text-muted-foreground mt-1">
                          {t('artwork.writeChoice.albumFolderOnlyDesc')}
                        </div>
                      </div>
                      <span className="text-xs text-primary font-medium ml-3 mt-0.5">
                        {t('artwork.priority.medium')}
                      </span>
                    </div>
                  </button>

                  {/* Album folder + Track metadata option */}
                  <button
                    onClick={() => handleAlbumChoice('both')}
                    disabled={saving}
                    className="w-full p-4 text-left rounded-lg border border-border hover:border-primary hover:bg-primary/5 transition-colors disabled:opacity-[var(--disabled-opacity)]"
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="font-medium">{t('artwork.writeChoice.albumFolderAndMetadata')}</div>
                        <div className="text-sm text-muted-foreground mt-1">
                          {t('artwork.writeChoice.albumFolderAndMetadataDesc')}
                        </div>
                      </div>
                      <span className="text-xs text-muted-foreground font-medium ml-3 mt-0.5">
                        {t('artwork.priority.lowest')}
                      </span>
                    </div>
                  </button>

                  {/* Soul Player storage only option */}
                  <button
                    onClick={() => handleAlbumChoice('soul_storage')}
                    disabled={saving}
                    className="w-full p-4 text-left rounded-lg border border-border hover:border-primary hover:bg-primary/5 transition-colors disabled:opacity-[var(--disabled-opacity)]"
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="font-medium">{t('artwork.writeChoice.soulPlayerOnly')}</div>
                        <div className="text-sm text-muted-foreground mt-1">
                          {t('artwork.writeChoice.soulPlayerOnlyDesc')}
                        </div>
                      </div>
                      <span className="text-xs text-primary font-medium ml-3 mt-0.5">
                        {t('artwork.priority.highest')}
                      </span>
                    </div>
                  </button>
                </div>
              </>
            )}

            {/* For artists/playlists, show simple save */}
            {entityType !== 'album' && (
              <p className="text-sm text-muted-foreground text-center">
                {t('artwork.edit.previewHint')}
              </p>
            )}
          </DialogBody>
          <DialogFooter>
            <Button variant="outline" onClick={handleCropCancel} disabled={saving}>
              {t('common.back')}
            </Button>
            {/* For artists/playlists, show Save button */}
            {entityType !== 'album' && (
              <Button onClick={() => handleAlbumChoice('folder')} disabled={saving}>
                {saving ? t('common.saving') : t('common.save')}
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }


  // Render main select dialog
  return (
    <Dialog open={open} onClose={handleClose}>
      <DialogContent className="max-w-md">
        <DialogHeader onClose={handleClose}>
          {t('artwork.edit.title')}
        </DialogHeader>
        <DialogBody>
          <div className="text-center mb-4">
            <span className="text-muted-foreground">{entityName}</span>
          </div>

          {/* Current/Preview artwork */}
          <div className="flex justify-center mb-6">
            <div className={`relative w-40 h-40 bg-muted overflow-hidden ${entityType === 'artist' ? 'rounded-full' : 'rounded-lg'}`}>
              {loadingArtwork ? (
                <div className="w-full h-full flex items-center justify-center">
                  <Loader2 className="w-8 h-8 text-muted-foreground animate-spin" />
                </div>
              ) : (previewUrl || displayArtworkUrl) ? (
                <img
                  src={previewUrl || displayArtworkUrl || ''}
                  alt={entityName}
                  className="w-full h-full object-cover"
                />
              ) : (
                <div className="w-full h-full flex items-center justify-center">
                  <ImageIcon className="w-12 h-12 text-muted-foreground" />
                </div>
              )}
            </div>
          </div>

          {/* Drop zone */}
          <div
            onDrop={handleDrop}
            onDragOver={handleDragOver}
            onClick={handleBrowseClick}
            className="border-2 border-dashed border-border rounded-lg p-6 text-center cursor-pointer hover:border-primary hover:bg-primary/5 transition-colors"
          >
            <Upload className="w-8 h-8 mx-auto mb-2 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">
              {t('artwork.edit.dropHint')}
            </p>
          </div>

          {/* Hidden file input */}
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            onChange={handleFileSelect}
            className="hidden"
          />

          {/* Error message */}
          {error && (
            <p className="text-destructive text-sm mt-3 text-center">{error}</p>
          )}

          {/* Saving indicator */}
          {saving && (
            <p className="text-muted-foreground text-sm mt-3 text-center">
              {t('common.saving')}
            </p>
          )}
        </DialogBody>
        <DialogFooter>
          {hasArtwork && isCustomArtwork && (
            <Button
              variant="ghost"
              onClick={handleRemove}
              disabled={saving || loadingArtwork}
              className="mr-auto text-destructive hover:text-destructive hover:bg-destructive/10"
            >
              <Trash2 className="w-4 h-4 mr-2" />
              {t('artwork.edit.remove')}
            </Button>
          )}
          <Button variant="outline" onClick={handleClose} disabled={saving}>
            {t('common.cancel')}
          </Button>
          <Button onClick={handleBrowseClick} disabled={saving}>
            {t('artwork.edit.selectImage')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
