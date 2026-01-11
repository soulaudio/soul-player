import { useState, useEffect, useRef, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, TauriEvent } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { usePlayerCommands } from '@soul-player/shared';
import { Music, FolderPlus, Play, X, Loader2, Check } from 'lucide-react';

interface DroppedFile {
  path: string;
  isDirectory: boolean;
  name: string;
}

interface ExternalFileSettings {
  defaultAction: 'ask' | 'play' | 'import';
  importDestination: 'managed' | 'watched';
  importToSourceId: number | null;
  showImportNotification: boolean;
}

interface FileDropHandlerProps {
  children: ReactNode;
}

export function FileDropHandler({ children }: FileDropHandlerProps) {
  const { t } = useTranslation();
  const { playQueue } = usePlayerCommands();
  const [isDragging, setIsDragging] = useState(false);
  const [droppedFiles, setDroppedFiles] = useState<DroppedFile[]>([]);
  const [showDialog, setShowDialog] = useState(false);
  const [loading, setLoading] = useState(false);
  const [rememberChoice, setRememberChoice] = useState(false);
  const settingsRef = useRef<ExternalFileSettings | null>(null);

  // Load external file settings on mount
  useEffect(() => {
    invoke<ExternalFileSettings>('get_external_file_settings')
      .then((settings) => {
        settingsRef.current = settings;
      })
      .catch((err) => {
        console.error('Failed to load external file settings:', err);
      });
  }, []);

  // Helper to process file paths and show dialog (or auto-handle based on settings)
  const processFilePaths = async (paths: string[]) => {
    const audioExtensions = ['mp3', 'flac', 'wav', 'ogg', 'oga', 'm4a', 'mp4', 'aac', 'opus', 'wma', 'aiff', 'aif', 'ape', 'wv'];

    try {
      const files: DroppedFile[] = [];

      for (const path of paths) {
        const isDir = await invoke<boolean>('is_directory', { path });
        const parts = path.split(/[/\\]/);
        const name = parts[parts.length - 1] || 'Unknown';

        if (isDir) {
          files.push({ path, isDirectory: true, name });
        } else {
          const ext = name.split('.').pop()?.toLowerCase() || '';
          if (audioExtensions.includes(ext)) {
            files.push({ path, isDirectory: false, name });
          }
        }
      }

      if (files.length > 0) {
        // Check if we should auto-handle based on saved preference
        const settings = settingsRef.current;
        if (settings && settings.defaultAction !== 'ask') {
          setDroppedFiles(files);
          if (settings.defaultAction === 'play') {
            await handlePlayInternal(files);
          } else if (settings.defaultAction === 'import') {
            await handleImportInternal(files);
          }
        } else {
          // Show dialog
          setDroppedFiles(files);
          setRememberChoice(false);
          setShowDialog(true);
        }
      }
    } catch (err) {
      console.error('File processing error:', err);
    }
  };

  // Tauri file drop events (dragDropEnabled: true)
  useEffect(() => {
    let unlistenDrop: (() => void) | null = null;
    let unlistenHover: (() => void) | null = null;
    let unlistenCancel: (() => void) | null = null;

    const setupListeners = async () => {
      // Listen for file drop
      unlistenDrop = await listen(TauriEvent.DRAG_DROP, async (event) => {
        setIsDragging(false);

        // Normalize payload to always be an array of strings
        let paths: string[];
        const payload = event.payload;

        if (typeof payload === 'string') {
          paths = [payload];
        } else if (Array.isArray(payload)) {
          paths = payload;
        } else if (payload && typeof payload === 'object' && 'paths' in payload) {
          paths = Array.isArray((payload as { paths: string[] }).paths)
            ? (payload as { paths: string[] }).paths
            : [(payload as { paths: string }).paths];
        } else {
          console.error('Unexpected payload format:', payload);
          return;
        }

        await processFilePaths(paths);
      });

      // Listen for drag hover
      unlistenHover = await listen(TauriEvent.DRAG_ENTER, () => {
        setIsDragging(true);
      });

      // Listen for drag leave
      unlistenCancel = await listen(TauriEvent.DRAG_LEAVE, () => {
        setIsDragging(false);
      });
    };

    setupListeners();

    return () => {
      if (unlistenDrop) unlistenDrop();
      if (unlistenHover) unlistenHover();
      if (unlistenCancel) unlistenCancel();
    };
  }, []);

  // Listen for files opened via file association (double-click on audio files)
  useEffect(() => {
    let unlistenFilesOpened: (() => void) | null = null;

    const setupListener = async () => {
      unlistenFilesOpened = await listen<string[]>('files-opened', async (event) => {
        const paths = event.payload;
        if (paths && paths.length > 0) {
          await processFilePaths(paths);
        }
      });
    };

    setupListener();

    return () => {
      if (unlistenFilesOpened) unlistenFilesOpened();
    };
  }, []);

  // Internal play handler that takes files directly
  const handlePlayInternal = async (files: DroppedFile[]) => {
    try {
      const audioFiles: string[] = [];

      for (const file of files) {
        if (file.isDirectory) {
          const scanned = await invoke<string[]>('scan_directory_for_audio', { path: file.path });
          audioFiles.push(...scanned);
        } else {
          audioFiles.push(file.path);
        }
      }

      if (audioFiles.length > 0) {
        const tracks = audioFiles.map((filePath, index) => {
          const name = filePath.split(/[/\\]/).pop() || 'Unknown';
          const title = name.replace(/\.[^/.]+$/, '');

          return {
            trackId: `dropped-${index}-${Date.now()}`,
            title,
            artist: 'Unknown Artist',
            album: null as string | null,
            filePath,
            durationSeconds: null as number | null,
            trackNumber: null as number | null,
            coverArtPath: undefined,
          };
        });

        await playQueue(tracks, 0);
      }
    } catch (err) {
      console.error('Failed to play files:', err);
    }
  };

  // Internal import handler that takes files directly
  const handleImportInternal = async (files: DroppedFile[]) => {
    try {
      const directories = files.filter((f) => f.isDirectory).map((f) => f.path);
      const filePaths = files.filter((f) => !f.isDirectory).map((f) => f.path);

      if (directories.length > 0) {
        for (const dir of directories) {
          await invoke('import_directory', { directory: dir });
        }
      }

      if (filePaths.length > 0) {
        await invoke('import_files', { files: filePaths });
      }
    } catch (err) {
      console.error('Failed to import files:', err);
    }
  };

  // Save user preference
  const savePreference = async (action: 'play' | 'import') => {
    if (!rememberChoice) return;

    try {
      await invoke('set_external_file_settings', {
        settings: {
          defaultAction: action,
          importDestination: settingsRef.current?.importDestination || 'managed',
          importToSourceId: settingsRef.current?.importToSourceId || null,
          showImportNotification: settingsRef.current?.showImportNotification ?? true,
        },
      });
      // Update local ref
      if (settingsRef.current) {
        settingsRef.current.defaultAction = action;
      }
    } catch (err) {
      console.error('Failed to save preference:', err);
    }
  };

  const handlePlay = async () => {
    setLoading(true);
    try {
      await savePreference('play');
      await handlePlayInternal(droppedFiles);
    } finally {
      setLoading(false);
      setShowDialog(false);
      setDroppedFiles([]);
    }
  };

  const handleImport = async () => {
    setLoading(true);
    try {
      await savePreference('import');
      await handleImportInternal(droppedFiles);
    } finally {
      setLoading(false);
      setShowDialog(false);
      setDroppedFiles([]);
    }
  };

  const handleClose = () => {
    setShowDialog(false);
    setDroppedFiles([]);
  };

  const fileCount = droppedFiles.filter((f) => !f.isDirectory).length;
  const folderCount = droppedFiles.filter((f) => f.isDirectory).length;

  return (
    <>
      {children}

      {/* Global drag overlay */}
      {isDragging && (
        <div className="fixed inset-0 z-50 bg-primary/10 border-4 border-dashed border-primary pointer-events-none flex items-center justify-center">
          <div className="bg-background/95 backdrop-blur-sm rounded-2xl p-8 shadow-2xl text-center">
            <Music className="w-16 h-16 mx-auto mb-4 text-primary animate-bounce" />
            <p className="text-2xl font-bold">{t('import.dropToAdd')}</p>
            <p className="text-muted-foreground mt-2">{t('import.releaseToAdd')}</p>
          </div>
        </div>
      )}

      {/* Import or Play dialog */}
      {showDialog && (
        <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center" onClick={handleClose}>
          <div
            className="bg-background border rounded-xl shadow-2xl w-full max-w-md overflow-hidden"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b">
              <h2 className="text-lg font-semibold">{t('import.fileDropped')}</h2>
              <button
                onClick={handleClose}
                className="p-2 hover:bg-accent rounded-full transition-colors"
                disabled={loading}
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Content */}
            <div className="p-6">
              {/* File info */}
              <div className="mb-6 p-4 bg-muted/30 rounded-lg">
                <div className="flex items-center gap-3">
                  <div className="flex-shrink-0 w-10 h-10 bg-primary/10 rounded-lg flex items-center justify-center">
                    {folderCount > 0 ? (
                      <FolderPlus className="w-5 h-5 text-primary" />
                    ) : (
                      <Music className="w-5 h-5 text-primary" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium truncate">
                      {droppedFiles.length === 1
                        ? droppedFiles[0].name
                        : `${droppedFiles.length} ${t('common.items')}`}
                    </p>
                    <p className="text-sm text-muted-foreground">
                      {folderCount > 0 && fileCount > 0 ? (
                        <>
                          {folderCount} {folderCount === 1 ? t('common.folder') : t('common.folders')},{' '}
                          {fileCount} {fileCount === 1 ? t('common.file') : t('common.files')}
                        </>
                      ) : folderCount > 0 ? (
                        <>
                          {folderCount} {folderCount === 1 ? t('common.folder') : t('common.folders')}
                        </>
                      ) : (
                        <>
                          {fileCount} {fileCount === 1 ? t('common.file') : t('common.files')}
                        </>
                      )}
                    </p>
                  </div>
                </div>
              </div>

              {/* Actions */}
              <div className="space-y-3">
                <button
                  onClick={handlePlay}
                  disabled={loading}
                  className="w-full flex items-center gap-3 p-4 rounded-lg border-2 border-primary bg-primary/5 hover:bg-primary/10 transition-colors disabled:opacity-50"
                >
                  {loading ? (
                    <Loader2 className="w-6 h-6 text-primary animate-spin" />
                  ) : (
                    <Play className="w-6 h-6 text-primary" />
                  )}
                  <div className="text-left flex-1">
                    <p className="font-medium">{t('import.playNow')}</p>
                    <p className="text-sm text-muted-foreground">{t('import.playNowDescription')}</p>
                  </div>
                </button>

                <button
                  onClick={handleImport}
                  disabled={loading}
                  className="w-full flex items-center gap-3 p-4 rounded-lg border hover:border-primary hover:bg-primary/5 transition-colors disabled:opacity-50"
                >
                  {loading ? (
                    <Loader2 className="w-6 h-6 animate-spin" />
                  ) : (
                    <FolderPlus className="w-6 h-6" />
                  )}
                  <div className="text-left flex-1">
                    <p className="font-medium">{t('import.importToLibrary')}</p>
                    <p className="text-sm text-muted-foreground">{t('import.importToLibraryDescription')}</p>
                  </div>
                </button>
              </div>

              {/* Remember my choice */}
              <label className="flex items-center gap-2 mt-4 cursor-pointer select-none">
                <button
                  type="button"
                  onClick={() => setRememberChoice(!rememberChoice)}
                  className={`w-5 h-5 rounded border-2 flex items-center justify-center transition-colors ${
                    rememberChoice
                      ? 'bg-primary border-primary text-primary-foreground'
                      : 'border-muted-foreground/50 hover:border-primary'
                  }`}
                >
                  {rememberChoice && <Check className="w-3 h-3" />}
                </button>
                <span className="text-sm text-muted-foreground">{t('import.rememberChoice')}</span>
              </label>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
