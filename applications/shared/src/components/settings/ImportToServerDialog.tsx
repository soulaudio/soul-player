import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Upload, Server, Folder, Music, CheckCircle, AlertCircle, Loader2, X } from 'lucide-react';

interface ServerSource {
  id: number;
  name: string;
  url: string;
  isAuthenticated: boolean;
  isOnline: boolean;
}

interface UploadProgress {
  totalFiles: number;
  completedFiles: number;
  currentFile?: string;
  errors: string[];
}

interface ImportToServerDialogProps {
  isOpen: boolean;
  onClose: () => void;
  serverSources: ServerSource[];
  selectedTrackIds?: number[];
  onUploadTracks: (sourceId: number, trackIds: number[]) => Promise<void>;
  onUploadFolder?: (sourceId: number, folderPath: string) => Promise<void>;
  onSyncLibrary?: (sourceId: number) => Promise<void>;
  onSelectFolder?: () => Promise<string | null>;
}

type UploadMode = 'selected' | 'library' | 'folder';

export function ImportToServerDialog({
  isOpen,
  onClose,
  serverSources,
  selectedTrackIds = [],
  onUploadTracks,
  onUploadFolder,
  onSyncLibrary,
  onSelectFolder,
}: ImportToServerDialogProps) {
  const { t } = useTranslation();
  const [selectedSourceId, setSelectedSourceId] = useState<number | null>(null);
  const [uploadMode, setUploadMode] = useState<UploadMode>('selected');
  const [folderPath, setFolderPath] = useState<string>('');
  const [isUploading, setIsUploading] = useState(false);
  const [progress, setProgress] = useState<UploadProgress | null>(null);
  const [uploadComplete, setUploadComplete] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Filter to only authenticated and online servers
  const availableServers = serverSources.filter(s => s.isAuthenticated && s.isOnline);

  const handleSelectFolder = useCallback(async () => {
    if (onSelectFolder) {
      const path = await onSelectFolder();
      if (path) {
        setFolderPath(path);
      }
    }
  }, [onSelectFolder]);

  const handleUpload = async () => {
    if (!selectedSourceId) {
      setError(t('upload.selectServer', 'Please select a server'));
      return;
    }

    setIsUploading(true);
    setError(null);
    setProgress({ totalFiles: 0, completedFiles: 0, errors: [] });

    try {
      if (uploadMode === 'selected' && selectedTrackIds.length > 0) {
        await onUploadTracks(selectedSourceId, selectedTrackIds);
      } else if (uploadMode === 'library' && onSyncLibrary) {
        await onSyncLibrary(selectedSourceId);
      } else if (uploadMode === 'folder' && folderPath && onUploadFolder) {
        await onUploadFolder(selectedSourceId, folderPath);
      }
      setUploadComplete(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsUploading(false);
    }
  };

  const handleClose = () => {
    if (!isUploading) {
      setSelectedSourceId(null);
      setUploadMode('selected');
      setFolderPath('');
      setProgress(null);
      setUploadComplete(false);
      setError(null);
      onClose();
    }
  };

  const canUpload = () => {
    if (!selectedSourceId) return false;
    if (uploadMode === 'selected' && selectedTrackIds.length === 0) return false;
    if (uploadMode === 'folder' && !folderPath) return false;
    return true;
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="bg-background rounded-xl shadow-xl w-full max-w-lg mx-4 max-h-[90vh] overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-primary/10">
              <Upload className="w-5 h-5 text-primary" />
            </div>
            <div>
              <h2 className="text-lg font-semibold">{t('upload.title', 'Upload to Server')}</h2>
              <p className="text-sm text-muted-foreground">
                {t('upload.subtitle', 'Sync your music to a Soul Player server')}
              </p>
            </div>
          </div>
          <button
            onClick={handleClose}
            disabled={isUploading}
            className="p-2 rounded-lg hover:bg-muted transition-colors disabled:opacity-50"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-6 overflow-y-auto max-h-[60vh]">
          {/* Upload Complete State */}
          {uploadComplete && (
            <div className="text-center py-8">
              <CheckCircle className="w-16 h-16 text-green-500 mx-auto mb-4" />
              <h3 className="text-lg font-medium mb-2">
                {t('upload.complete', 'Upload Complete')}
              </h3>
              <p className="text-muted-foreground">
                {t('upload.completeDescription', 'Your music has been uploaded to the server')}
              </p>
              {progress && progress.errors.length > 0 && (
                <div className="mt-4 p-3 bg-yellow-500/10 rounded-lg text-left">
                  <p className="text-sm font-medium text-yellow-600 mb-2">
                    {t('upload.someErrors', '{{count}} files failed to upload', { count: progress.errors.length })}
                  </p>
                  <ul className="text-xs text-yellow-700 space-y-1">
                    {progress.errors.slice(0, 5).map((err, i) => (
                      <li key={i}>{err}</li>
                    ))}
                    {progress.errors.length > 5 && (
                      <li>...{t('common.andMore', 'and {{count}} more', { count: progress.errors.length - 5 })}</li>
                    )}
                  </ul>
                </div>
              )}
            </div>
          )}

          {/* Uploading State */}
          {isUploading && !uploadComplete && (
            <div className="text-center py-8">
              <Loader2 className="w-12 h-12 text-primary mx-auto mb-4 animate-spin" />
              <h3 className="text-lg font-medium mb-2">
                {t('upload.uploading', 'Uploading...')}
              </h3>
              {progress && (
                <div className="space-y-2">
                  <p className="text-muted-foreground">
                    {progress.completedFiles} / {progress.totalFiles} {t('common.files', 'files')}
                  </p>
                  {progress.currentFile && (
                    <p className="text-sm text-muted-foreground truncate px-8">
                      {progress.currentFile}
                    </p>
                  )}
                  <div className="w-full bg-muted rounded-full h-2 mt-4">
                    <div
                      className="bg-primary rounded-full h-2 transition-all duration-300"
                      style={{
                        width: `${progress.totalFiles > 0 ? (progress.completedFiles / progress.totalFiles) * 100 : 0}%`,
                      }}
                    />
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Configuration State */}
          {!isUploading && !uploadComplete && (
            <>
              {/* Error */}
              {error && (
                <div className="p-3 bg-destructive/10 border border-destructive/20 rounded-lg flex items-start gap-3">
                  <AlertCircle className="w-5 h-5 text-destructive flex-shrink-0 mt-0.5" />
                  <div>
                    <p className="text-sm font-medium text-destructive">
                      {t('upload.error', 'Upload Failed')}
                    </p>
                    <p className="text-sm text-destructive/80">{error}</p>
                  </div>
                </div>
              )}

              {/* Server Selection */}
              <div>
                <label className="block text-sm font-medium mb-2">
                  {t('upload.selectDestination', 'Select Destination Server')}
                </label>
                {availableServers.length === 0 ? (
                  <div className="p-4 bg-muted/50 rounded-lg text-center">
                    <Server className="w-8 h-8 text-muted-foreground mx-auto mb-2" />
                    <p className="text-sm text-muted-foreground">
                      {t('upload.noServers', 'No authenticated servers available')}
                    </p>
                    <p className="text-xs text-muted-foreground mt-1">
                      {t('upload.addServerHint', 'Add and authenticate a server in Settings > Sources')}
                    </p>
                  </div>
                ) : (
                  <div className="space-y-2">
                    {availableServers.map((server) => (
                      <button
                        key={server.id}
                        onClick={() => setSelectedSourceId(server.id)}
                        className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                          selectedSourceId === server.id
                            ? 'border-primary bg-primary/5'
                            : 'border-muted hover:border-primary/50'
                        }`}
                      >
                        <Server className={`w-5 h-5 ${selectedSourceId === server.id ? 'text-primary' : 'text-muted-foreground'}`} />
                        <div className="text-left flex-1">
                          <p className="font-medium">{server.name}</p>
                          <p className="text-xs text-muted-foreground">{server.url}</p>
                        </div>
                        {selectedSourceId === server.id && (
                          <CheckCircle className="w-5 h-5 text-primary" />
                        )}
                      </button>
                    ))}
                  </div>
                )}
              </div>

              {/* Upload Mode Selection */}
              {availableServers.length > 0 && (
                <div>
                  <label className="block text-sm font-medium mb-2">
                    {t('upload.whatToUpload', 'What to Upload')}
                  </label>
                  <div className="space-y-2">
                    {/* Selected Tracks */}
                    <button
                      onClick={() => setUploadMode('selected')}
                      disabled={selectedTrackIds.length === 0}
                      className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                        uploadMode === 'selected'
                          ? 'border-primary bg-primary/5'
                          : 'border-muted hover:border-primary/50'
                      } disabled:opacity-50 disabled:cursor-not-allowed`}
                    >
                      <Music className={`w-5 h-5 ${uploadMode === 'selected' ? 'text-primary' : 'text-muted-foreground'}`} />
                      <div className="text-left flex-1">
                        <p className="font-medium">{t('upload.selectedTracks', 'Selected Tracks')}</p>
                        <p className="text-xs text-muted-foreground">
                          {selectedTrackIds.length > 0
                            ? t('upload.tracksCount', '{{count}} tracks selected', { count: selectedTrackIds.length })
                            : t('upload.noTracksSelected', 'No tracks selected')}
                        </p>
                      </div>
                    </button>

                    {/* Entire Library */}
                    {onSyncLibrary && (
                      <button
                        onClick={() => setUploadMode('library')}
                        className={`w-full flex items-center gap-3 p-3 rounded-lg border transition-colors ${
                          uploadMode === 'library'
                            ? 'border-primary bg-primary/5'
                            : 'border-muted hover:border-primary/50'
                        }`}
                      >
                        <Music className={`w-5 h-5 ${uploadMode === 'library' ? 'text-primary' : 'text-muted-foreground'}`} />
                        <div className="text-left flex-1">
                          <p className="font-medium">{t('upload.entireLibrary', 'Entire Library')}</p>
                          <p className="text-xs text-muted-foreground">
                            {t('upload.syncAllTracks', 'Sync all tracks to the server')}
                          </p>
                        </div>
                      </button>
                    )}

                    {/* From Folder */}
                    {onSelectFolder && onUploadFolder && (
                      <div className={`rounded-lg border transition-colors ${
                        uploadMode === 'folder'
                          ? 'border-primary bg-primary/5'
                          : 'border-muted'
                      }`}>
                        <button
                          onClick={() => setUploadMode('folder')}
                          className="w-full flex items-center gap-3 p-3"
                        >
                          <Folder className={`w-5 h-5 ${uploadMode === 'folder' ? 'text-primary' : 'text-muted-foreground'}`} />
                          <div className="text-left flex-1">
                            <p className="font-medium">{t('upload.fromFolder', 'From Folder')}</p>
                            <p className="text-xs text-muted-foreground">
                              {t('upload.uploadFolder', 'Upload music from a folder')}
                            </p>
                          </div>
                        </button>
                        {uploadMode === 'folder' && (
                          <div className="px-3 pb-3">
                            <div className="flex gap-2">
                              <input
                                type="text"
                                value={folderPath}
                                readOnly
                                placeholder={t('upload.selectFolderPlaceholder', 'Select a folder...')}
                                className="flex-1 px-3 py-2 rounded-md bg-background border text-sm"
                              />
                              <button
                                onClick={handleSelectFolder}
                                className="px-3 py-2 bg-muted hover:bg-muted/80 rounded-md text-sm transition-colors"
                              >
                                {t('common.browse', 'Browse')}
                              </button>
                            </div>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                </div>
              )}
            </>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t flex justify-end gap-3">
          {uploadComplete ? (
            <button
              onClick={handleClose}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
            >
              {t('common.done', 'Done')}
            </button>
          ) : (
            <>
              <button
                onClick={handleClose}
                disabled={isUploading}
                className="px-4 py-2 text-muted-foreground hover:text-foreground transition-colors disabled:opacity-50"
              >
                {t('common.cancel', 'Cancel')}
              </button>
              <button
                onClick={handleUpload}
                disabled={!canUpload() || isUploading || availableServers.length === 0}
                className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 transition-colors flex items-center gap-2"
              >
                {isUploading ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    {t('upload.uploading', 'Uploading...')}
                  </>
                ) : (
                  <>
                    <Upload className="w-4 h-4" />
                    {t('upload.startUpload', 'Start Upload')}
                  </>
                )}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

export default ImportToServerDialog;
