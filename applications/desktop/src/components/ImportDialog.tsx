import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, TauriEvent } from '@tauri-apps/api/event';

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
    console.log('Setting up import event listeners');

    // Listen for import progress
    const unlistenProgress = listen<ImportProgress>('import-progress', (event) => {
      console.log('Import progress event:', event.payload);
      setProgress(event.payload);
    });

    // Listen for import completion
    const unlistenComplete = listen<ImportSummary>('import-complete', (event) => {
      console.log('Import complete event:', event.payload);
      setSummary(event.payload);
      setImporting(false);
    });

    // Listen for import errors
    const unlistenError = listen<string>('import-error', (event) => {
      console.error('Import error event:', event.payload);
      setError(event.payload);
      setImporting(false);
    });

    return () => {
      console.log('Cleaning up import event listeners');
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  // Tauri file drop events (dragDropEnabled: true)
  useEffect(() => {
    if (!open) return;

    console.log('Setting up Tauri file drop listeners');

    let unlistenDrop: (() => void) | null = null;
    let unlistenHover: (() => void) | null = null;
    let unlistenCancel: (() => void) | null = null;

    const setupListeners = async () => {
      // Listen for file drop
      unlistenDrop = await listen(TauriEvent.DRAG_DROP, async (event) => {
        console.log('Tauri file drop event:', event);
        setIsDragging(false);

        // Normalize payload to always be an array of strings
        let paths: string[];
        const payload = event.payload;

        console.log('Payload type:', typeof payload);
        console.log('Payload value:', payload);

        if (typeof payload === 'string') {
          // Single file/folder path
          paths = [payload];
        } else if (Array.isArray(payload)) {
          // Array of paths
          paths = payload;
        } else if (payload && typeof payload === 'object' && 'paths' in payload) {
          // Object with paths property
          paths = Array.isArray((payload as any).paths) ? (payload as any).paths : [(payload as any).paths];
        } else {
          console.error('Unexpected payload format:', payload);
          setError('Unexpected drag and drop payload format');
          return;
        }

        console.log('Normalized paths:', paths);

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
          }
        } catch (err) {
          console.error('File drop error:', err);
          setError(err instanceof Error ? err.message : String(err));
          setImporting(false);
        }
      });

      // Listen for drag hover
      unlistenHover = await listen(TauriEvent.DRAG_ENTER, () => {
        console.log('Drag hover detected');
        if (!importing) {
          setIsDragging(true);
        }
      });

      // Listen for drag leave/cancel
      unlistenCancel = await listen(TauriEvent.DRAG_LEAVE, () => {
        console.log('Drag cancelled');
        setIsDragging(false);
      });
    };

    setupListeners();

    return () => {
      console.log('Cleaning up file drop listeners');
      if (unlistenDrop) unlistenDrop();
      if (unlistenHover) unlistenHover();
      if (unlistenCancel) unlistenCancel();
    };
  }, [open, importing]);

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
          // @ts-expect-error - Browser adds webkitRelativePath for directory uploads
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
        setError('No valid files or directories found. Please drop audio files or music folders.');
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
          <h2 className="text-xl font-semibold">Import Music</h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-accent rounded-full transition-colors"
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
                      Drop Here
                    </div>
                    <div className="text-muted-foreground">
                      Release to import your music files
                    </div>
                  </div>
                </div>
              ) : (
                /* Default State */
                <div className="flex flex-col items-center justify-center min-h-[400px] space-y-8">
                  <div className="text-center space-y-2">
                    <h3 className="text-xl font-semibold">Import Music</h3>
                    <p className="text-muted-foreground">
                      Add music files to your library
                    </p>
                  </div>

                  {/* File Management Strategy Selector */}
                  <div className="w-full max-w-md space-y-3">
                    <label className="text-sm font-medium">File Management Strategy</label>

                    <div className="space-y-2">
                      {/* Copy Option (Recommended) */}
                      <label className="flex items-start gap-3 p-3 border rounded-lg cursor-pointer hover:bg-accent transition-colors">
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
                            Copy files to library <span className="text-primary text-sm">(Recommended)</span>
                          </div>
                          <div className="text-sm text-muted-foreground mt-1">
                            Creates copies in managed library folder, preserves original files
                          </div>
                        </div>
                      </label>

                      {/* Move Option (Recommended) */}
                      <label className="flex items-start gap-3 p-3 border rounded-lg cursor-pointer hover:bg-accent transition-colors">
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
                            Move files to library <span className="text-primary text-sm">(Recommended)</span>
                          </div>
                          <div className="text-sm text-muted-foreground mt-1">
                            Moves files to managed library folder, saves disk space
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
                            Reference in current location
                            <svg className="w-4 h-4 text-yellow-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                            </svg>
                          </div>
                          <div className="text-sm text-yellow-600 dark:text-yellow-500 mt-1">
                            ⚠️ Warning: Library will break if files are moved or deleted
                          </div>
                        </div>
                      </label>
                    </div>
                  </div>

                  <div className="flex gap-4">
                    <button
                      onClick={handleImportFiles}
                      className="flex flex-col items-center gap-3 px-8 py-6 border-2 rounded-lg hover:border-primary hover:bg-accent transition-colors group min-w-[180px]"
                    >
                      <svg className="w-12 h-12 text-muted-foreground group-hover:text-primary transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 13h6m-3-3v6m5 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                      </svg>
                      <div className="font-medium">Select Files</div>
                    </button>

                    <button
                      onClick={handleImportFolder}
                      className="flex flex-col items-center gap-3 px-8 py-6 border-2 rounded-lg hover:border-primary hover:bg-accent transition-colors group min-w-[180px]"
                    >
                      <svg className="w-12 h-12 text-muted-foreground group-hover:text-primary transition-colors" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
                      </svg>
                      <div className="font-medium">Select Folder</div>
                    </button>
                  </div>

                  <div className="text-center text-sm text-muted-foreground">
                    or just drop files here
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
                  Processing {progress.processedFiles} of {progress.totalFiles} files
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
                  <div className="text-xs text-muted-foreground mt-1">Imported</div>
                </div>
                <div className="text-center p-3 bg-yellow-500/10 rounded-lg">
                  <div className="text-2xl font-bold text-yellow-500">{progress.skippedDuplicates}</div>
                  <div className="text-xs text-muted-foreground mt-1">Skipped</div>
                </div>
                <div className="text-center p-3 bg-red-500/10 rounded-lg">
                  <div className="text-2xl font-bold text-red-500">{progress.failedImports}</div>
                  <div className="text-xs text-muted-foreground mt-1">Failed</div>
                </div>
              </div>

              {/* Current File */}
              {progress.currentFile && (
                <div className="p-3 bg-muted/40 rounded-lg">
                  <div className="text-xs text-muted-foreground mb-1">Currently processing:</div>
                  <div className="text-sm font-mono truncate">{progress.currentFile}</div>
                </div>
              )}

              {/* Time Remaining */}
              {progress.estimatedSecondsRemaining !== null && (
                <div className="text-center text-sm text-muted-foreground">
                  Estimated time remaining: {Math.ceil(progress.estimatedSecondsRemaining / 60)} min
                </div>
              )}

              <button
                onClick={handleCancel}
                className="w-full px-4 py-2 border border-red-500 text-red-500 rounded-lg hover:bg-red-500/10 transition-colors"
              >
                Cancel Import
              </button>
            </div>
          )}

          {summary && (
            <div className="space-y-4">
              <div className="text-center">
                <div className="text-5xl mb-3">✓</div>
                <div className="text-xl font-semibold mb-2">Import Complete!</div>
                <div className="text-muted-foreground">
                  Processed {summary.totalProcessed} files in {summary.durationSeconds}s
                </div>
              </div>

              <div className="grid grid-cols-3 gap-4">
                <div className="text-center p-4 bg-green-500/10 rounded-lg">
                  <div className="text-3xl font-bold text-green-500">{summary.successful}</div>
                  <div className="text-sm text-muted-foreground mt-1">Imported</div>
                </div>
                <div className="text-center p-4 bg-yellow-500/10 rounded-lg">
                  <div className="text-3xl font-bold text-yellow-500">{summary.duplicatesSkipped}</div>
                  <div className="text-sm text-muted-foreground mt-1">Skipped</div>
                </div>
                <div className="text-center p-4 bg-red-500/10 rounded-lg">
                  <div className="text-3xl font-bold text-red-500">{summary.failed}</div>
                  <div className="text-sm text-muted-foreground mt-1">Failed</div>
                </div>
              </div>

              {summary.requireReviewCount > 0 && (
                <div className="p-4 bg-blue-500/10 border border-blue-500/20 rounded-lg">
                  <div className="flex items-center gap-2 text-blue-500 font-medium">
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    {summary.requireReviewCount} tracks require review
                  </div>
                  <div className="text-sm text-muted-foreground mt-1">
                    Some tracks had low confidence matches and need manual verification
                  </div>
                </div>
              )}

              {summary.errors.length > 0 && (
                <div className="max-h-40 overflow-auto">
                  <div className="text-sm font-medium mb-2">Errors:</div>
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
                className="w-full px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
              >
                Done
              </button>
            </div>
          )}

          {error && (
            <div className="text-center space-y-4">
              <div className="text-5xl mb-3">⚠️</div>
              <div className="text-xl font-semibold text-red-500">Import Failed</div>
              <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-sm">
                {error}
              </div>
              <button
                onClick={onClose}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
              >
                Close
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
