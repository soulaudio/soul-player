import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, TauriEvent } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';

interface ImportDialogProps {
  open: boolean;
  onClose: () => void;
}

interface ImportProgress {
  totalFiles: number;
  processedFiles: number;
  successfulImports: number;
  skippedDuplicates: number;
  failedImports: number;
  currentFile: string | null;
  estimatedSecondsRemaining: number | null;
  percentage: number;
}

interface ImportSummary {
  totalProcessed: number;
  successful: number;
  duplicatesSkipped: number;
  failed: number;
  requireReviewCount: number;
  errors: Array<[string, string]>;
  durationSeconds: number;
}

type FileManagementStrategy = 'move' | 'copy' | 'reference';

interface ImportConfig {
  libraryPath: string;
  fileStrategy: FileManagementStrategy;
  confidenceThreshold: number;
  fileNamingPattern: string;
  skipDuplicates: boolean;
}

export function ImportDialog({ open, onClose }: ImportDialogProps) {
  const { t } = useTranslation();
  const [importing, setImporting] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [summary, setSummary] = useState<ImportSummary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [fileStrategy, setFileStrategy] = useState<FileManagementStrategy>('copy');

  useEffect(() => {
    if (!open) {
      // Reset state when dialog closes
      setProgress(null);
      setSummary(null);
      setError(null);
      setImporting(false);
    } else {
      // Load current config when dialog opens
      loadImportConfig();
    }
  }, [open]);

  const loadImportConfig = async () => {
    try {
      const config = await invoke<ImportConfig>('get_import_config');
      setFileStrategy(config.fileStrategy);
    } catch (err) {
      console.error('Failed to load import config:', err);
    }
  };

  const handleStrategyChange = async (strategy: FileManagementStrategy) => {
    try {
      // Update local state
      setFileStrategy(strategy);

      // Get current config
      const config = await invoke<ImportConfig>('get_import_config');

      // Update with new strategy
      await invoke('update_import_config', {
        config: {
          ...config,
          fileStrategy: strategy,
        },
      });
    } catch (err) {
      console.error('Failed to update file strategy:', err);
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  useEffect(() => {
    console.log('[ImportDialog] Setting up import event listeners');
    const unlistenFunctions: (() => void)[] = [];
    let isMounted = true;

    const setupListeners = async () => {
      try {
        if (!isMounted) {
          console.log('[ImportDialog] Component unmounted before setup, aborting');
          return;
        }

        // Listen for import progress
        const unlistenProgress = await listen<ImportProgress>('import-progress', (event) => {
          if (!isMounted) return;
          console.log('[ImportDialog] Import progress event:', event.payload);
          setProgress(event.payload);
        });
        unlistenFunctions.push(unlistenProgress);
        console.log('[ImportDialog] import-progress listener registered');

        // Listen for import completion
        const unlistenComplete = await listen<ImportSummary>('import-complete', (event) => {
          if (!isMounted) return;
          console.log('[ImportDialog] Import complete event:', event.payload);
          setSummary(event.payload);
          setImporting(false);
        });
        unlistenFunctions.push(unlistenComplete);
        console.log('[ImportDialog] import-complete listener registered');

        // Listen for import errors
        const unlistenError = await listen<string>('import-error', (event) => {
          if (!isMounted) return;
          console.error('[ImportDialog] Import error event:', event.payload);
          setError(event.payload);
          setImporting(false);
        });
        unlistenFunctions.push(unlistenError);
        console.log('[ImportDialog] import-error listener registered');

        console.log('[ImportDialog] All import listeners registered successfully');
      } catch (error) {
        console.error('[ImportDialog] Failed to set up import listeners:', error);
      }
    };

    void setupListeners();

    return () => {
      console.log('[ImportDialog] Cleaning up import listeners, count:', unlistenFunctions.length);
      isMounted = false;
      unlistenFunctions.forEach(fn => fn());
    };
  }, []);

  // Tauri file drop events (dragDropEnabled: true)
  useEffect(() => {
    if (!open) {
      console.log('[ImportDialog] Dialog closed, skipping file drop setup');
      return;
    }

    console.log('[ImportDialog] Setting up Tauri file drop listeners');
    const unlistenFunctions: (() => void)[] = [];
    let isMounted = true;

    const setupListeners = async () => {
      try {
        if (!isMounted) {
          console.log('[ImportDialog] Component unmounted before drop setup, aborting');
          return;
        }

        // Listen for file drop
        const unlistenDrop = await listen(TauriEvent.DRAG_DROP, async (event) => {
          if (!isMounted) return;
          console.log('[ImportDialog] Tauri file drop event:', event);
          setIsDragging(false);

          // Normalize payload to always be an array of strings
          let paths: string[];
          const payload = event.payload;

          console.log('[ImportDialog] Payload type:', typeof payload);
          console.log('[ImportDialog] Payload value:', payload);

          if (typeof payload === 'string') {
            // Single file/folder path
            paths = [payload];
          } else if (Array.isArray(payload)) {
            // Array of paths
            paths = payload;
          } else if (payload && typeof payload === 'object' && 'paths' in payload) {
            // Object with paths property
            const payloadObj = payload as { paths: string | string[] };
            paths = Array.isArray(payloadObj.paths) ? payloadObj.paths : [payloadObj.paths];
          } else {
            console.error('[ImportDialog] Unexpected payload format:', payload);
            setError(t('import.unexpectedPayload'));
            return;
          }

          console.log('[ImportDialog] Normalized paths:', paths);

          try {
            const files: string[] = [];
            const directories: string[] = [];

            for (const path of paths) {
              const isDir = await invoke<boolean>('is_directory', { path });
              if (isDir) {
                directories.push(path);
              } else {
                files.push(path);
              }
            }

            if (directories.length > 0) {
              console.log('[ImportDialog] Importing directory:', directories[0]);
              setImporting(true);
              setError(null);
              setSummary(null);
              setProgress(null);
              await invoke('import_directory', { directory: directories[0] });
            } else if (files.length > 0) {
              console.log('[ImportDialog] Importing files:', files);
              setImporting(true);
              setError(null);
              setSummary(null);
              setProgress(null);
              await invoke('import_files', { files });
            }
          } catch (err) {
            console.error('[ImportDialog] File drop error:', err);
            setError(err instanceof Error ? err.message : String(err));
            setImporting(false);
          }
        });
        unlistenFunctions.push(unlistenDrop);
        console.log('[ImportDialog] DRAG_DROP listener registered');

        // Listen for drag hover
        const unlistenHover = await listen(TauriEvent.DRAG_ENTER, () => {
          if (!isMounted) return;
          console.log('[ImportDialog] Drag hover detected');
          if (!importing) {
            setIsDragging(true);
          }
        });
        unlistenFunctions.push(unlistenHover);
        console.log('[ImportDialog] DRAG_ENTER listener registered');

        // Listen for drag leave/cancel
        const unlistenCancel = await listen(TauriEvent.DRAG_LEAVE, () => {
          if (!isMounted) return;
          console.log('[ImportDialog] Drag cancelled');
          setIsDragging(false);
        });
        unlistenFunctions.push(unlistenCancel);
        console.log('[ImportDialog] DRAG_LEAVE listener registered');

        console.log('[ImportDialog] All file drop listeners registered successfully');
      } catch (error) {
        console.error('[ImportDialog] Failed to set up file drop listeners:', error);
      }
    };

    void setupListeners();

    return () => {
      console.log('[ImportDialog] Cleaning up file drop listeners, count:', unlistenFunctions.length);
      isMounted = false;
      unlistenFunctions.forEach(fn => fn());
    };
  }, [open]); // Remove importing dependency to prevent listener leak

  // HTML5 Drag and Drop handlers (fallback)
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    if (!importing && !summary && !error) {
      setIsDragging(true);
    }
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    // Only set to false if we're leaving the dialog entirely
    // Check if the related target is outside the dialog
    const currentTarget = e.currentTarget;
    const relatedTarget = e.relatedTarget as Node;
    if (!currentTarget.contains(relatedTarget)) {
      setIsDragging(false);
    }
  };

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    console.log('=== DROP EVENT ===');
    console.log('Files dropped:', e.dataTransfer.files.length);
    console.log('DataTransfer items:', e.dataTransfer.items.length);

    try {
      const files: string[] = [];
      const directories: string[] = [];

      // Process dropped files - Tauri adds a 'path' property to File objects
      for (let i = 0; i < e.dataTransfer.files.length; i++) {
        const file = e.dataTransfer.files[i];

        console.log(`File ${i}:`, {
          name: file.name,
          size: file.size,
          type: file.type,
          // @ts-expect-error - Tauri adds path property to File objects
          path: file.path,
          webkitRelativePath: file.webkitRelativePath,
        });

        // @ts-expect-error - Tauri adds 'path' property to File objects
        const path = file.path;

        if (!path) {
          console.warn('No path found for dropped file:', file.name, 'File object:', file);
          console.log('All file properties:', Object.keys(file));
          console.log('File prototype:', Object.getPrototypeOf(file));
          continue;
        }

        console.log('Processing dropped item with path:', path);

        // Check if it's a directory or file
        const isDir = await invoke<boolean>('is_directory', { path });
        console.log('Is directory:', isDir);

        if (isDir) {
          directories.push(path);
        } else {
          files.push(path);
        }
      }

      console.log('Results - Files:', files.length, 'Directories:', directories.length);

      // Import based on what was dropped
      if (directories.length > 0) {
        console.log('Importing directory:', directories[0]);
        setImporting(true);
        setError(null);
        setSummary(null);
        setProgress(null);
        await invoke('import_directory', { directory: directories[0] });
      } else if (files.length > 0) {
        console.log('Importing files:', files);
        setImporting(true);
        setError(null);
        setSummary(null);
        setProgress(null);
        await invoke('import_files', { files });
      } else {
        console.error('ERROR: No valid files or directories found');
        setError(t('import.noValidFiles'));
      }
    } catch (err) {
      console.error('Drop error:', err);
      setError(err instanceof Error ? err.message : String(err));
      setImporting(false);
    }
  };

  const handleImportFiles = async () => {
    try {
      console.log('Opening file dialog...');
      // Use Tauri command to open file dialog
      const files = await invoke<string[] | null>('open_file_dialog', {
        multiple: true,
        filters: [{ name: 'Audio Files', extensions: ['mp3', 'flac', 'ogg', 'wav', 'aac', 'm4a', 'opus'] }]
      });

      console.log('File dialog result:', files);

      if (files && files.length > 0) {
        console.log('Starting file import:', files.length, 'files');
        setImporting(true);
        setError(null);
        setSummary(null);
        setProgress(null);
        await invoke('import_files', { files });
        console.log('Import command sent');
      } else {
        console.log('No files selected');
      }
    } catch (err) {
      console.error('File import error:', err);
      setError(err instanceof Error ? err.message : String(err));
      setImporting(false);
    }
  };

  const handleImportFolder = async () => {
    try {
      console.log('Opening folder dialog...');
      // Use Tauri command to open folder dialog
      const folder = await invoke<string | null>('open_folder_dialog');

      console.log('Folder dialog result:', folder);

      if (folder) {
        console.log('Starting directory import:', folder);
        setImporting(true);
        setError(null);
        setSummary(null);
        setProgress(null);
        await invoke('import_directory', { directory: folder });
        console.log('Import command sent');
      } else {
        console.log('No folder selected');
      }
    } catch (err) {
      console.error('Folder import error:', err);
      setError(err instanceof Error ? err.message : String(err));
      setImporting(false);
    }
  };


  const handleCancel = async () => {
    try {
      await invoke('cancel_import');
      setImporting(false);
    } catch (err) {
      console.error('Failed to cancel import:', err);
    }
  };

  if (!open) return null;

  console.log('ImportDialog rendering:', { open, importing, hasProgress: !!progress, hasSummary: !!summary, hasError: !!error });

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-background border rounded-lg shadow-lg w-full max-w-2xl max-h-[80vh] overflow-hidden flex flex-col" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b">
          <h2 className="text-xl font-semibold">{t('import.title')}</h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-foreground/[var(--hover-bg-opacity)] rounded-full transition-colors duration-[var(--transition-duration)]"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div
          className="flex-1 overflow-auto p-6"
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
        >
          {!importing && !summary && !error && (
            <>
              {isDragging ? (
                /* Drop Zone Active State */
                <div className="flex items-center justify-center h-full min-h-[400px]">
                  <div className="text-center">
                    <svg className="w-24 h-24 mx-auto mb-6 text-primary animate-bounce" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                    </svg>
                    <div className="text-3xl font-bold text-primary mb-2">
                      {t('import.dropToAdd')}
                    </div>
                    <div className="text-muted-foreground">
                      {t('import.releaseToImport')}
                    </div>
                  </div>
                </div>
              ) : (
                /* Default State */
                <div className="flex flex-col items-center justify-center min-h-[400px] space-y-8">
                  <div className="text-center space-y-2">
                    <h3 className="text-xl font-semibold">{t('import.title')}</h3>
                    <p className="text-muted-foreground">
                      {t('import.addMusicToLibrary')}
                    </p>
                  </div>

                  {/* File Management Strategy Selector */}
                  <div className="w-full max-w-md space-y-3">
                    <label className="text-sm font-medium">{t('import.fileManagementStrategy')}</label>

                    <div className="space-y-2">
                      {/* Copy Option (Recommended) */}
                      <label className="flex items-start gap-3 p-3 border rounded-lg cursor-pointer hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]">
                        <input
                          type="radio"
                          name="fileStrategy"
                          value="copy"
                          checked={fileStrategy === 'copy'}
                          onChange={() => handleStrategyChange('copy')}
                          className="mt-1"
                        />
                        <div className="flex-1">
                          <div className="font-medium">
                            {t('import.copyFilesLabel')} <span className="text-primary text-sm">({t('import.recommended')})</span>
                          </div>
                          <div className="text-sm text-muted-foreground mt-1">
                            {t('import.copyFilesDesc')}
                          </div>
                        </div>
                      </label>

                      {/* Move Option */}
                      <label className="flex items-start gap-3 p-3 border rounded-lg cursor-pointer hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]">
                        <input
                          type="radio"
                          name="fileStrategy"
                          value="move"
                          checked={fileStrategy === 'move'}
                          onChange={() => handleStrategyChange('move')}
                          className="mt-1"
                        />
                        <div className="flex-1">
                          <div className="font-medium">
                            {t('import.moveFilesLabel')}
                          </div>
                          <div className="text-sm text-muted-foreground mt-1">
                            {t('import.moveFilesDesc')}
                          </div>
                        </div>
                      </label>

                      {/* Reference Option (Warning) */}
                      <label className="flex items-start gap-3 p-3 border border-yellow-500/50 rounded-lg cursor-pointer hover:bg-yellow-500/5 transition-colors">
                        <input
                          type="radio"
                          name="fileStrategy"
                          value="reference"
                          checked={fileStrategy === 'reference'}
                          onChange={() => handleStrategyChange('reference')}
                          className="mt-1"
                        />
                        <div className="flex-1">
                          <div className="font-medium flex items-center gap-2">
                            {t('import.referenceFilesLabel')}
                            <svg className="w-4 h-4 text-yellow-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                            </svg>
                          </div>
                          <div className="text-sm text-yellow-600 dark:text-yellow-500 mt-1">
                            {t('import.referenceFilesWarning')}
                          </div>
                        </div>
                      </label>
                    </div>
                  </div>

                  <div className="flex gap-4">
                    <button
                      onClick={handleImportFiles}
                      className="flex flex-col items-center gap-3 px-8 py-6 border-2 rounded-lg hover:border-primary hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)] group min-w-[180px]"
                    >
                      <svg className="w-12 h-12 text-muted-foreground group-hover:text-primary transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                      </svg>
                      <div className="font-medium">{t('import.selectFiles')}</div>
                    </button>

                    <button
                      onClick={handleImportFolder}
                      className="flex flex-col items-center gap-3 px-8 py-6 border-2 rounded-lg hover:border-primary hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)] group min-w-[180px]"
                    >
                      <svg className="w-12 h-12 text-muted-foreground group-hover:text-primary transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                      </svg>
                      <div className="font-medium">{t('import.selectFolder')}</div>
                    </button>
                  </div>

                  <div className="text-center text-sm text-muted-foreground">
                    {t('import.orDropFilesHere')}
                  </div>
                </div>
              )}
            </>
          )}

          {importing && progress && (
            <div className="space-y-6">
              <div className="text-center">
                <div className="text-4xl font-bold mb-2">{Math.round(progress.percentage)}%</div>
                <div className="text-muted-foreground">
                  {t('import.processingProgress', { processed: progress.processedFiles, total: progress.totalFiles })}
                </div>
              </div>

              {/* Progress Bar */}
              <div className="relative h-2 bg-muted rounded-full overflow-hidden">
                <div
                  className="absolute inset-y-0 left-0 bg-primary transition-all duration-300"
                  style={{ width: `${progress.percentage}%` }}
                />
              </div>

              {/* Stats */}
              <div className="grid grid-cols-3 gap-4">
                <div className="text-center p-3 bg-green-500/10 rounded-lg">
                  <div className="text-2xl font-bold text-green-500">{progress.successfulImports}</div>
                  <div className="text-xs text-muted-foreground mt-1">{t('import.statImported')}</div>
                </div>
                <div className="text-center p-3 bg-yellow-500/10 rounded-lg">
                  <div className="text-2xl font-bold text-yellow-500">{progress.skippedDuplicates}</div>
                  <div className="text-xs text-muted-foreground mt-1">{t('import.statSkipped')}</div>
                </div>
                <div className="text-center p-3 bg-red-500/10 rounded-lg">
                  <div className="text-2xl font-bold text-red-500">{progress.failedImports}</div>
                  <div className="text-xs text-muted-foreground mt-1">{t('import.statFailed')}</div>
                </div>
              </div>

              {/* Current File */}
              {progress.currentFile && (
                <div className="p-3 bg-muted/40 rounded-lg">
                  <div className="text-xs text-muted-foreground mb-1">{t('import.currentlyProcessing')}</div>
                  <div className="text-sm font-mono truncate">{progress.currentFile}</div>
                </div>
              )}

              {/* Time Remaining */}
              {progress.estimatedSecondsRemaining !== null && (
                <div className="text-center text-sm text-muted-foreground">
                  {t('import.estimatedTimeRemaining', { minutes: Math.ceil(progress.estimatedSecondsRemaining / 60) })}
                </div>
              )}

              <button
                onClick={handleCancel}
                className="w-full px-4 py-2 border border-red-500 text-red-500 rounded-lg hover:bg-red-500/10 transition-colors"
              >
                {t('import.cancel')}
              </button>
            </div>
          )}

          {summary && (
            <div className="space-y-4">
              <div className="text-center">
                <div className="text-5xl mb-3">✓</div>
                <div className="text-xl font-semibold mb-2">{t('import.complete')}</div>
                <div className="text-muted-foreground">
                  {t('import.processedSummary', { count: summary.totalProcessed, seconds: summary.durationSeconds })}
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div className="text-center p-4 bg-green-500/10 rounded-lg">
                  <div className="text-3xl font-bold text-green-500">{summary.successful}</div>
                  <div className="text-sm text-muted-foreground mt-1">{t('import.statImported')}</div>
                </div>
                <div className="text-center p-4 bg-yellow-500/10 rounded-lg">
                  <div className="text-3xl font-bold text-yellow-500">{summary.duplicatesSkipped}</div>
                  <div className="text-sm text-muted-foreground mt-1">{t('import.statSkipped')}</div>
                </div>
                <div className="text-center p-4 bg-red-500/10 rounded-lg">
                  <div className="text-3xl font-bold text-red-500">{summary.failed}</div>
                  <div className="text-sm text-muted-foreground mt-1">{t('import.statFailed')}</div>
                </div>
              </div>

              {summary.requireReviewCount > 0 && (
                <div className="p-4 bg-blue-500/10 border border-blue-500/20 rounded-lg">
                  <div className="flex items-center gap-2 text-blue-500 font-medium">
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    {t('import.requireReview', { count: summary.requireReviewCount })}
                  </div>
                  <div className="text-sm text-muted-foreground mt-1">
                    {t('import.requireReviewDesc')}
                  </div>
                </div>
              )}

              {summary.errors.length > 0 && (
                <div className="max-h-40 overflow-auto">
                  <div className="text-sm font-medium mb-2">{t('common.error')}:</div>
                  <div className="space-y-1">
                    {summary.errors.map(([path, errorMsg], index) => (
                      <div key={index} className="text-xs p-2 bg-red-500/10 rounded">
                        <div className="font-mono truncate text-red-500">{path}</div>
                        <div className="text-muted-foreground mt-1">{errorMsg}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              <button
                onClick={onClose}
                className="w-full px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity duration-[var(--transition-duration)]"
              >
                {t('common.done')}
              </button>
            </div>
          )}

          {error && (
            <div className="text-center space-y-4">
              <div className="text-5xl mb-3">⚠️</div>
              <div className="text-xl font-semibold text-red-500">{t('import.failed')}</div>
              <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-sm">
                {error}
              </div>
              <button
                onClick={onClose}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity duration-[var(--transition-duration)]"
              >
                {t('common.close')}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
