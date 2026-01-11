import { useState, useEffect, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { Search, ListMusic, Plus, Check, X } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogBody,
  DialogFooter,
} from '@soul-player/shared';

interface Playlist {
  id: string;
  name: string;
  description?: string;
  track_count: number;
  is_favorite: boolean;
}

interface AddToPlaylistDialogProps {
  open: boolean;
  onClose: () => void;
  trackId: number;
  trackTitle?: string;
}

export function AddToPlaylistDialog({
  open,
  onClose,
  trackId,
  trackTitle,
}: AddToPlaylistDialogProps) {
  const { t } = useTranslation();
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [containingPlaylistIds, setContainingPlaylistIds] = useState<Set<string>>(new Set());
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [searchQuery, setSearchQuery] = useState('');
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [newPlaylistName, setNewPlaylistName] = useState('');
  const [showNewPlaylistInput, setShowNewPlaylistInput] = useState(false);

  // Load playlists and which ones contain this track
  useEffect(() => {
    if (!open) return;

    const loadData = async () => {
      setIsLoading(true);
      try {
        const [playlistsResult, containingIds] = await Promise.all([
          invoke<Playlist[]>('get_all_playlists'),
          invoke<string[]>('get_playlists_containing_track', { trackId: String(trackId) }),
        ]);

        setPlaylists(playlistsResult);
        const containingSet = new Set(containingIds);
        setContainingPlaylistIds(containingSet);
        // Pre-select playlists that already contain the track
        setSelectedIds(new Set(containingSet));
      } catch (error) {
        console.error('Failed to load playlists:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadData();
    setSearchQuery('');
    setNewPlaylistName('');
    setShowNewPlaylistInput(false);
  }, [open, trackId]);

  // Filter playlists by search query
  const filteredPlaylists = useMemo(() => {
    if (!searchQuery.trim()) return playlists;
    const query = searchQuery.toLowerCase();
    return playlists.filter(
      (p) =>
        p.name.toLowerCase().includes(query) ||
        p.description?.toLowerCase().includes(query)
    );
  }, [playlists, searchQuery]);

  // Toggle playlist selection
  const togglePlaylist = useCallback((playlistId: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(playlistId)) {
        next.delete(playlistId);
      } else {
        next.add(playlistId);
      }
      return next;
    });
  }, []);

  // Create new playlist
  const handleCreatePlaylist = async () => {
    if (!newPlaylistName.trim()) return;

    try {
      const newPlaylist = await invoke<Playlist>('create_playlist', {
        name: newPlaylistName.trim(),
        description: null,
      });

      setPlaylists((prev) => [newPlaylist, ...prev]);
      setSelectedIds((prev) => new Set([...prev, newPlaylist.id]));
      setNewPlaylistName('');
      setShowNewPlaylistInput(false);
    } catch (error) {
      console.error('Failed to create playlist:', error);
    }
  };

  // Save changes (add/remove track from playlists)
  const handleSave = async () => {
    setIsSaving(true);
    try {
      const toAdd = Array.from(selectedIds).filter((id) => !containingPlaylistIds.has(id));
      const toRemove = Array.from(containingPlaylistIds).filter((id) => !selectedIds.has(id));

      // Add track to new playlists
      await Promise.all(
        toAdd.map((playlistId) =>
          invoke('add_track_to_playlist', {
            playlistId,
            trackId: String(trackId),
          })
        )
      );

      // Remove track from deselected playlists
      await Promise.all(
        toRemove.map((playlistId) =>
          invoke('remove_track_from_playlist', {
            playlistId,
            trackId: String(trackId),
          })
        )
      );

      onClose();
    } catch (error) {
      console.error('Failed to save playlist changes:', error);
    } finally {
      setIsSaving(false);
    }
  };

  // Check if there are any changes
  const hasChanges = useMemo(() => {
    if (selectedIds.size !== containingPlaylistIds.size) return true;
    const selectedArray = Array.from(selectedIds);
    for (let i = 0; i < selectedArray.length; i++) {
      if (!containingPlaylistIds.has(selectedArray[i])) return true;
    }
    return false;
  }, [selectedIds, containingPlaylistIds]);

  return (
    <Dialog open={open} onClose={onClose}>
      <DialogContent className="max-w-sm">
        <DialogHeader onClose={onClose}>
          {t('playlist.addToPlaylist', 'Add to Playlist')}
        </DialogHeader>

        <DialogBody>
          {/* Track info */}
          {trackTitle && (
            <div className="mb-4 p-3 rounded-lg bg-muted/50">
              <p className="text-sm text-muted-foreground">{t('playlist.addingTrack', 'Adding track')}</p>
              <p className="font-medium truncate">{trackTitle}</p>
            </div>
          )}

          {/* Search input */}
          <div className="relative mb-4">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('playlist.searchPlaylists', 'Search playlists...')}
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

          {/* Create new playlist */}
          {showNewPlaylistInput ? (
            <div className="flex items-center gap-2 mb-4">
              <input
                type="text"
                value={newPlaylistName}
                onChange={(e) => setNewPlaylistName(e.target.value)}
                placeholder={t('playlist.newPlaylistName', 'New Playlist')}
                className="flex-1 px-3 py-2 rounded-lg bg-muted border border-transparent focus:border-primary focus:outline-none text-sm"
                autoFocus
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleCreatePlaylist();
                  if (e.key === 'Escape') setShowNewPlaylistInput(false);
                }}
              />
              <button
                onClick={handleCreatePlaylist}
                disabled={!newPlaylistName.trim()}
                className="px-3 py-2 bg-primary text-primary-foreground rounded-lg text-sm font-medium hover:bg-primary/90 transition-colors disabled:opacity-50"
              >
                {t('common.save', 'Save')}
              </button>
              <button
                onClick={() => {
                  setShowNewPlaylistInput(false);
                  setNewPlaylistName('');
                }}
                className="p-2 hover:bg-muted rounded-lg transition-colors"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
          ) : (
            <button
              onClick={() => setShowNewPlaylistInput(true)}
              className="flex items-center gap-2 w-full px-3 py-2 mb-4 rounded-lg border border-dashed border-border hover:border-primary hover:bg-muted/50 transition-colors text-sm text-muted-foreground hover:text-foreground"
            >
              <Plus className="w-4 h-4" />
              {t('playlist.createNew', 'Create new playlist')}
            </button>
          )}

          {/* Playlist list */}
          <div className="max-h-64 overflow-y-auto -mx-6 px-6">
            {isLoading ? (
              <div className="flex items-center justify-center py-8 text-muted-foreground">
                <div className="animate-spin w-5 h-5 border-2 border-current border-t-transparent rounded-full" />
              </div>
            ) : filteredPlaylists.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                {searchQuery ? (
                  <p>{t('library.noSearchResults', 'No results found')}</p>
                ) : (
                  <>
                    <ListMusic className="w-8 h-8 mx-auto mb-2 opacity-50" />
                    <p>{t('playlist.noPlaylists', 'No playlists yet')}</p>
                  </>
                )}
              </div>
            ) : (
              <div className="space-y-1">
                {filteredPlaylists.map((playlist) => {
                  const isSelected = selectedIds.has(playlist.id);
                  const wasInPlaylist = containingPlaylistIds.has(playlist.id);

                  return (
                    <button
                      key={playlist.id}
                      onClick={() => togglePlaylist(playlist.id)}
                      className={`w-full flex items-center gap-3 p-3 rounded-lg transition-colors text-left ${
                        isSelected
                          ? 'bg-primary/10 border border-primary/30'
                          : 'hover:bg-muted border border-transparent'
                      }`}
                    >
                      {/* Playlist cover placeholder */}
                      <div className="w-12 h-12 rounded-md bg-muted flex items-center justify-center flex-shrink-0">
                        <ListMusic className="w-5 h-5 text-muted-foreground" />
                      </div>

                      {/* Playlist info */}
                      <div className="flex-1 min-w-0">
                        <p className="font-medium truncate">{playlist.name}</p>
                        <p className="text-sm text-muted-foreground">
                          {t('library.tracks', '{{count}} tracks', { count: playlist.track_count })}
                        </p>
                      </div>

                      {/* Selection indicator */}
                      <div
                        className={`w-6 h-6 rounded-full flex items-center justify-center flex-shrink-0 transition-colors ${
                          isSelected
                            ? 'bg-primary text-primary-foreground'
                            : 'border-2 border-muted-foreground/30'
                        }`}
                      >
                        {isSelected && <Check className="w-4 h-4" />}
                      </div>

                      {/* "Already added" indicator */}
                      {wasInPlaylist && !isSelected && (
                        <span className="text-xs text-muted-foreground ml-2">
                          {t('playlist.willRemove', 'will remove')}
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </DialogBody>

        <DialogFooter>
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-lg border border-border hover:bg-muted transition-colors"
          >
            {t('common.cancel', 'Cancel')}
          </button>
          <button
            onClick={handleSave}
            disabled={!hasChanges || isSaving}
            className="px-4 py-2 text-sm rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors disabled:opacity-50"
          >
            {isSaving ? t('common.saving', 'Saving...') : t('common.done', 'Done')}
          </button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
