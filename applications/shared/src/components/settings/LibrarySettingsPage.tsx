import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import {
  FolderOpen,
  Plus,
  Trash2,
  RefreshCw,
  FolderInput,
  FileAudio,
  ChevronDown,
  ChevronUp,
  AlertCircle,
  Check,
  Loader2,
  Cloud,
  Server,
  Wifi,
  WifiOff,
  LogIn,
  LogOut,
  Download,
} from 'lucide-react';

// Types matching the Tauri backend
interface LibrarySource {
  id: number;
  name: string;
  path: string;
  enabled: boolean;
  syncDeletes: boolean;
  lastScanAt: number | null;
  scanStatus: string | null;
  errorMessage: string | null;
}

interface ManagedLibrarySettings {
  libraryPath: string;
  pathTemplate: string;
  importAction: string;
}

interface ExternalFileSettings {
  defaultAction: string;
  importDestination: string;
  importToSourceId: number | null;
  showImportNotification: boolean;
}

interface PathTemplatePreset {
  id: string;
  template: string;
  example: string;
}

// Source sync types
interface SourceInfo {
  id: number;
  name: string;
  sourceType: string;
  serverUrl: string | null;
  isActive: boolean;
  isOnline: boolean;
  isAuthenticated: boolean;
  username: string | null;
  lastSyncAt: string | null;
}

interface ServerTestResult {
  success: boolean;
  name: string | null;
  version: string | null;
  requiresAuth: boolean;
  error: string | null;
}

interface AuthResult {
  success: boolean;
  username: string | null;
  error: string | null;
}

interface SyncStatus {
  status: string;
  progress: number;
  currentOperation: string | null;
  currentItem: string | null;
  processedItems: number;
  totalItems: number;
  error: string | null;
}

interface SyncResult {
  success: boolean;
  tracksUploaded: number;
  tracksDownloaded: number;
  tracksUpdated: number;
  tracksDeleted: number;
  errors: string[];
}

export function LibrarySettingsPage() {
  const { t } = useTranslation();

  // State
  const [sources, setSources] = useState<LibrarySource[]>([]);
  const [managedSettings, setManagedSettings] = useState<ManagedLibrarySettings | null>(null);
  const [externalSettings, setExternalSettings] = useState<ExternalFileSettings | null>(null);
  const [presets, setPresets] = useState<PathTemplatePreset[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // UI state
  const [expandedSection, setExpandedSection] = useState<string | null>('sources');
  const [addingSource, setAddingSource] = useState(false);
  const [newSourceName, setNewSourceName] = useState('');
  const [newSourcePath, setNewSourcePath] = useState('');
  const [newSourceSyncDeletes, setNewSourceSyncDeletes] = useState(true);
  const [scanningSourceId, setScanningSourceId] = useState<number | null>(null);
  const [templatePreview, setTemplatePreview] = useState<string>('');

  // Server sources state
  const [serverSources, setServerSources] = useState<SourceInfo[]>([]);
  const [addingServer, setAddingServer] = useState(false);
  const [newServerName, setNewServerName] = useState('');
  const [newServerUrl, setNewServerUrl] = useState('');
  const [testingConnection, setTestingConnection] = useState(false);
  const [connectionTestResult, setConnectionTestResult] = useState<ServerTestResult | null>(null);

  // Auth state
  const [authSourceId, setAuthSourceId] = useState<number | null>(null);
  const [authUsername, setAuthUsername] = useState('');
  const [authPassword, setAuthPassword] = useState('');
  const [authenticating, setAuthenticating] = useState(false);

  // Sync state
  const [syncingSourceId, setSyncingSourceId] = useState<number | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);

  // Load all settings
  const loadSettings = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      const [sourcesData, managedData, externalData, presetsData, serverSourcesData] = await Promise.all([
        invoke<LibrarySource[]>('get_library_sources'),
        invoke<ManagedLibrarySettings | null>('get_managed_library_settings'),
        invoke<ExternalFileSettings>('get_external_file_settings'),
        invoke<[string, string, string][]>('get_path_template_presets'),
        invoke<SourceInfo[]>('get_sources').catch(() => [] as SourceInfo[]),
      ]);

      setSources(sourcesData);
      setManagedSettings(managedData);
      setExternalSettings(externalData);
      setPresets(
        presetsData.map(([id, template, example]) => ({ id, template, example }))
      );
      setServerSources(serverSourcesData.filter((s) => s.sourceType === 'server'));

      // Update template preview if we have managed settings
      if (managedData?.pathTemplate) {
        const preview = await invoke<string>('preview_path_template', {
          template: managedData.pathTemplate,
        });
        setTemplatePreview(preview);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  // Source management
  const handleAddSource = async () => {
    if (!newSourceName.trim() || !newSourcePath.trim()) return;

    try {
      await invoke('add_library_source', {
        name: newSourceName.trim(),
        path: newSourcePath.trim(),
        syncDeletes: newSourceSyncDeletes,
      });

      setNewSourceName('');
      setNewSourcePath('');
      setNewSourceSyncDeletes(true);
      setAddingSource(false);
      await loadSettings();
    } catch (err) {
      const errStr = String(err);
      if (errStr === 'DUPLICATE_PATH') {
        setError(t('librarySettings.errors.duplicatePath'));
      } else {
        setError(errStr);
      }
    }
  };

  const handleRemoveSource = async (sourceId: number) => {
    try {
      await invoke('remove_library_source', { sourceId });
      await loadSettings();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleToggleSource = async (sourceId: number, enabled: boolean) => {
    try {
      await invoke('toggle_library_source', { sourceId, enabled });
      await loadSettings();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleRescanSource = async (sourceId: number) => {
    try {
      setScanningSourceId(sourceId);
      await invoke('rescan_library_source', { sourceId });
      await loadSettings();
    } catch (err) {
      setError(String(err));
    } finally {
      setScanningSourceId(null);
    }
  };

  const handleRescanAll = async (forceRefresh = false) => {
    try {
      setScanningSourceId(-1); // Use -1 to indicate "all"
      await invoke('rescan_all_sources', { forceRefresh });
      await loadSettings();
    } catch (err) {
      setError(String(err));
    } finally {
      setScanningSourceId(null);
    }
  };

  // Managed library settings
  const handleManagedSettingsChange = async (
    field: keyof ManagedLibrarySettings,
    value: string
  ) => {
    if (!managedSettings) return;

    const updated = { ...managedSettings, [field]: value };
    setManagedSettings(updated);

    // Debounce save
    try {
      await invoke('set_managed_library_settings', {
        libraryPath: updated.libraryPath,
        pathTemplate: updated.pathTemplate,
        importAction: updated.importAction,
      });

      if (field === 'pathTemplate') {
        const preview = await invoke<string>('preview_path_template', {
          template: value,
        });
        setTemplatePreview(preview);
      }
    } catch (err) {
      setError(String(err));
    }
  };

  // External file settings
  const handleExternalSettingsChange = async (
    field: keyof ExternalFileSettings,
    value: string | boolean | number | null
  ) => {
    if (!externalSettings) return;

    const updated = { ...externalSettings, [field]: value };
    setExternalSettings(updated);

    try {
      await invoke('set_external_file_settings', {
        defaultAction: updated.defaultAction,
        importDestination: updated.importDestination,
        importToSourceId: updated.importToSourceId,
        showImportNotification: updated.showImportNotification,
      });
    } catch (err) {
      setError(String(err));
    }
  };

  // Server source handlers
  const handleTestConnection = async () => {
    if (!newServerUrl.trim()) return;

    try {
      setTestingConnection(true);
      setConnectionTestResult(null);

      const result = await invoke<ServerTestResult>('test_server_connection', {
        url: newServerUrl.trim(),
      });

      setConnectionTestResult(result);

      if (result.success && result.name && !newServerName.trim()) {
        setNewServerName(result.name);
      }
    } catch (err) {
      setConnectionTestResult({
        success: false,
        name: null,
        version: null,
        requiresAuth: false,
        error: String(err),
      });
    } finally {
      setTestingConnection(false);
    }
  };

  const handleAddServer = async () => {
    if (!newServerName.trim() || !newServerUrl.trim()) return;

    try {
      await invoke('add_server_source', {
        name: newServerName.trim(),
        url: newServerUrl.trim(),
      });

      setNewServerName('');
      setNewServerUrl('');
      setConnectionTestResult(null);
      setAddingServer(false);
      await loadSettings();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleRemoveServer = async (sourceId: number) => {
    try {
      await invoke('remove_source', { sourceId });
      await loadSettings();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleAuthenticate = async (sourceId: number) => {
    if (!authUsername.trim() || !authPassword.trim()) return;

    try {
      setAuthenticating(true);

      const result = await invoke<AuthResult>('authenticate_source', {
        sourceId,
        username: authUsername.trim(),
        password: authPassword,
      });

      if (result.success) {
        setAuthSourceId(null);
        setAuthUsername('');
        setAuthPassword('');
        await loadSettings();
      } else {
        setError(result.error || t('sources.authFailed'));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setAuthenticating(false);
    }
  };

  const handleLogout = async (sourceId: number) => {
    try {
      await invoke('logout_source', { sourceId });
      await loadSettings();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleSetActive = async (sourceId: number) => {
    try {
      await invoke('set_active_source', { sourceId });
      await loadSettings();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleSyncFromServer = async (sourceId: number) => {
    try {
      setSyncingSourceId(sourceId);
      setSyncStatus({ status: 'syncing', progress: 0, currentOperation: 'starting', currentItem: null, processedItems: 0, totalItems: 0, error: null });

      const result = await invoke<SyncResult>('sync_from_server', { sourceId });

      if (result.success) {
        setSyncStatus(null);
        await loadSettings();
      } else {
        setError(result.errors.join(', ') || t('sources.syncFailed'));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setSyncingSourceId(null);
    }
  };

  const handleCancelSync = async (sourceId: number) => {
    try {
      await invoke('cancel_sync', { sourceId });
      setSyncingSourceId(null);
      setSyncStatus(null);
    } catch (err) {
      setError(String(err));
    }
  };

  const toggleSection = (section: string) => {
    setExpandedSection(expandedSection === section ? null : section);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-6 h-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {error && (
        <div className="p-4 bg-destructive/10 border border-destructive/20 rounded-lg flex items-start gap-3">
          <AlertCircle className="w-5 h-5 text-destructive flex-shrink-0 mt-0.5" />
          <div>
            <p className="text-sm text-destructive">{error}</p>
            <button
              onClick={() => setError(null)}
              className="text-xs text-destructive/70 hover:text-destructive mt-1"
            >
              {t('common.close')}
            </button>
          </div>
        </div>
      )}

      {/* Library Sources Section */}
      <section className="border border-border rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('sources')}
          className="w-full flex items-center justify-between p-4 bg-muted/30 hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-3">
            <FolderOpen className="w-5 h-5 text-primary" />
            <div className="text-left">
              <h3 className="font-medium">{t('librarySettings.watchedFolders')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('librarySettings.watchedFoldersDescription')}
              </p>
            </div>
          </div>
          {expandedSection === 'sources' ? (
            <ChevronUp className="w-5 h-5 text-muted-foreground" />
          ) : (
            <ChevronDown className="w-5 h-5 text-muted-foreground" />
          )}
        </button>

        {expandedSection === 'sources' && (
          <div className="p-4 space-y-4">
            {/* Source list */}
            {sources.length > 0 ? (
              <div className="space-y-2">
                {sources.map((source) => (
                  <div
                    key={source.id}
                    className="flex items-center gap-3 p-3 bg-muted/20 rounded-lg"
                  >
                    <input
                      type="checkbox"
                      checked={source.enabled}
                      onChange={(e) => handleToggleSource(source.id, e.target.checked)}
                      className="w-4 h-4"
                    />
                    <div className="flex-1 min-w-0">
                      <p className="font-medium truncate">{source.name}</p>
                      <p className="text-sm text-muted-foreground truncate">{source.path}</p>
                      {source.lastScanAt && (
                        <p className="text-xs text-muted-foreground">
                          {t('librarySettings.lastScanned')}: {new Date(source.lastScanAt * 1000).toLocaleString()}
                        </p>
                      )}
                    </div>
                    <div className="flex items-center gap-2">
                      {source.scanStatus === 'scanning' || scanningSourceId === source.id ? (
                        <Loader2 className="w-4 h-4 animate-spin text-primary" />
                      ) : (
                        <button
                          onClick={() => handleRescanSource(source.id)}
                          className="p-2 hover:bg-muted rounded-lg transition-colors"
                          title={t('librarySettings.rescan')}
                        >
                          <RefreshCw className="w-4 h-4" />
                        </button>
                      )}
                      <button
                        onClick={() => handleRemoveSource(source.id)}
                        className="p-2 hover:bg-destructive/10 text-destructive rounded-lg transition-colors"
                        title={t('common.delete')}
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground text-center py-4">
                {t('librarySettings.noWatchedFolders')}
              </p>
            )}

            {/* Add source form */}
            {addingSource ? (
              <div className="space-y-3 p-4 bg-muted/20 rounded-lg">
                <input
                  type="text"
                  value={newSourceName}
                  onChange={(e) => setNewSourceName(e.target.value)}
                  placeholder={t('librarySettings.sourceName')}
                  className="w-full px-3 py-2 rounded-lg bg-background border border-border"
                />
                <div className="flex gap-2">
                  <div className="flex-1 px-3 py-2 rounded-lg bg-muted border border-border text-sm truncate">
                    {newSourcePath || <span className="text-muted-foreground">{t('librarySettings.sourcePath')}</span>}
                  </div>
                  <button
                    onClick={async () => {
                      const folder = await invoke<string | null>('open_folder_dialog');
                      if (folder) {
                        setNewSourcePath(folder);
                        if (!newSourceName.trim()) {
                          const folderName = folder.split(/[/\\]/).filter(Boolean).pop() || '';
                          setNewSourceName(folderName);
                        }
                      }
                    }}
                    className="flex items-center gap-2 px-4 py-2 bg-muted rounded-lg hover:bg-muted/80 whitespace-nowrap"
                  >
                    <FolderOpen className="w-4 h-4" />
                    {t('common.browse')}
                  </button>
                </div>
                <label className="flex items-center gap-2 text-sm">
                  <input
                    type="checkbox"
                    checked={newSourceSyncDeletes}
                    onChange={(e) => setNewSourceSyncDeletes(e.target.checked)}
                    className="w-4 h-4"
                  />
                  {t('librarySettings.syncDeletes')}
                </label>
                <div className="flex gap-2">
                  <button
                    onClick={handleAddSource}
                    disabled={!newSourceName.trim() || !newSourcePath.trim()}
                    className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
                  >
                    <Check className="w-4 h-4" />
                    {t('common.save')}
                  </button>
                  <button
                    onClick={() => setAddingSource(false)}
                    className="px-4 py-2 bg-muted rounded-lg hover:bg-muted/80"
                  >
                    {t('common.cancel')}
                  </button>
                </div>
              </div>
            ) : (
              <div className="flex gap-2">
                <button
                  onClick={() => setAddingSource(true)}
                  className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
                >
                  <Plus className="w-4 h-4" />
                  {t('librarySettings.addFolder')}
                </button>
                {sources.length > 0 && (
                  <>
                    <button
                      onClick={() => handleRescanAll(false)}
                      disabled={scanningSourceId !== null}
                      className="flex items-center gap-2 px-4 py-2 bg-muted rounded-lg hover:bg-muted/80 disabled:opacity-50"
                    >
                      {scanningSourceId === -1 ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <RefreshCw className="w-4 h-4" />
                      )}
                      {t('librarySettings.rescanAll')}
                    </button>
                    <button
                      onClick={() => handleRescanAll(true)}
                      disabled={scanningSourceId !== null}
                      className="flex items-center gap-2 px-4 py-2 bg-amber-500/20 text-amber-700 dark:text-amber-400 rounded-lg hover:bg-amber-500/30 disabled:opacity-50"
                      title={t('librarySettings.forceRefreshDescription')}
                    >
                      {scanningSourceId === -1 ? (
                        <Loader2 className="w-4 h-4 animate-spin" />
                      ) : (
                        <RefreshCw className="w-4 h-4" />
                      )}
                      {t('librarySettings.forceRefresh')}
                    </button>
                  </>
                )}
              </div>
            )}
          </div>
        )}
      </section>

      {/* Managed Library Section */}
      <section className="border border-border rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('managed')}
          className="w-full flex items-center justify-between p-4 bg-muted/30 hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-3">
            <FolderInput className="w-5 h-5 text-primary" />
            <div className="text-left">
              <h3 className="font-medium">{t('librarySettings.managedLibrary')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('librarySettings.managedLibraryDescription')}
              </p>
            </div>
          </div>
          {expandedSection === 'managed' ? (
            <ChevronUp className="w-5 h-5 text-muted-foreground" />
          ) : (
            <ChevronDown className="w-5 h-5 text-muted-foreground" />
          )}
        </button>

        {expandedSection === 'managed' && (
          <div className="p-4 space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">
                {t('librarySettings.libraryPath')}
              </label>
              <div className="flex gap-2">
                <div className="flex-1 px-3 py-2 rounded-lg bg-muted border border-border text-sm truncate">
                  {managedSettings?.libraryPath || <span className="text-muted-foreground">{t('librarySettings.libraryPathPlaceholder')}</span>}
                </div>
                <button
                  onClick={async () => {
                    const folder = await invoke<string | null>('open_folder_dialog');
                    if (folder) {
                      handleManagedSettingsChange('libraryPath', folder);
                    }
                  }}
                  className="flex items-center gap-2 px-4 py-2 bg-muted rounded-lg hover:bg-muted/80 whitespace-nowrap"
                >
                  <FolderOpen className="w-4 h-4" />
                  {t('common.browse')}
                </button>
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">
                {t('librarySettings.pathTemplate')}
              </label>
              <select
                value={presets.find(p => p.template === managedSettings?.pathTemplate)?.id ?? 'custom'}
                onChange={(e) => {
                  const preset = presets.find(p => p.id === e.target.value);
                  if (preset) {
                    handleManagedSettingsChange('pathTemplate', preset.template);
                  }
                }}
                className="w-full px-3 py-2 rounded-lg bg-muted border border-border mb-2"
              >
                {presets.map((preset) => (
                  <option key={preset.id} value={preset.id}>
                    {t(`librarySettings.preset.${preset.id}`)}
                  </option>
                ))}
                <option value="custom">{t('librarySettings.preset.custom')}</option>
              </select>
              <input
                type="text"
                value={managedSettings?.pathTemplate ?? ''}
                onChange={(e) => handleManagedSettingsChange('pathTemplate', e.target.value)}
                placeholder="{AlbumArtist}/{Album}/{TrackNo} - {Title}"
                className="w-full px-3 py-2 rounded-lg bg-muted border border-border font-mono text-sm"
              />
              {templatePreview && (
                <p className="mt-2 text-sm text-muted-foreground">
                  {t('librarySettings.preview')}: <span className="font-mono">{templatePreview}</span>
                </p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">
                {t('librarySettings.importAction')}
              </label>
              <select
                value={managedSettings?.importAction ?? 'copy'}
                onChange={(e) => handleManagedSettingsChange('importAction', e.target.value)}
                className="w-full max-w-xs px-3 py-2 rounded-lg bg-muted border border-border"
              >
                <option value="copy">{t('librarySettings.action.copy')}</option>
                <option value="move">{t('librarySettings.action.move')}</option>
              </select>
            </div>
          </div>
        )}
      </section>

      {/* External Files Section */}
      <section className="border border-border rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('external')}
          className="w-full flex items-center justify-between p-4 bg-muted/30 hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-3">
            <FileAudio className="w-5 h-5 text-primary" />
            <div className="text-left">
              <h3 className="font-medium">{t('librarySettings.externalFiles')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('librarySettings.externalFilesDescription')}
              </p>
            </div>
          </div>
          {expandedSection === 'external' ? (
            <ChevronUp className="w-5 h-5 text-muted-foreground" />
          ) : (
            <ChevronDown className="w-5 h-5 text-muted-foreground" />
          )}
        </button>

        {expandedSection === 'external' && (
          <div className="p-4 space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">
                {t('librarySettings.defaultAction')}
              </label>
              <select
                value={externalSettings?.defaultAction ?? 'ask'}
                onChange={(e) => handleExternalSettingsChange('defaultAction', e.target.value)}
                className="w-full max-w-xs px-3 py-2 rounded-lg bg-muted border border-border"
              >
                <option value="ask">{t('librarySettings.externalAction.ask')}</option>
                <option value="play">{t('librarySettings.externalAction.play')}</option>
                <option value="import">{t('librarySettings.externalAction.import')}</option>
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">
                {t('librarySettings.importDestination')}
              </label>
              <select
                value={externalSettings?.importDestination ?? 'managed'}
                onChange={(e) => handleExternalSettingsChange('importDestination', e.target.value)}
                className="w-full max-w-xs px-3 py-2 rounded-lg bg-muted border border-border"
              >
                <option value="managed">{t('librarySettings.destination.managed')}</option>
                <option value="watched">{t('librarySettings.destination.watched')}</option>
              </select>
            </div>

            {externalSettings?.importDestination === 'watched' && sources.length > 0 && (
              <div>
                <label className="block text-sm font-medium mb-2">
                  {t('librarySettings.targetFolder')}
                </label>
                <select
                  value={externalSettings?.importToSourceId ?? ''}
                  onChange={(e) =>
                    handleExternalSettingsChange(
                      'importToSourceId',
                      e.target.value ? Number(e.target.value) : null
                    )
                  }
                  className="w-full max-w-xs px-3 py-2 rounded-lg bg-muted border border-border"
                >
                  <option value="">{t('librarySettings.selectFolder')}</option>
                  {sources.map((source) => (
                    <option key={source.id} value={source.id}>
                      {source.name}
                    </option>
                  ))}
                </select>
              </div>
            )}

            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                checked={externalSettings?.showImportNotification ?? true}
                onChange={(e) =>
                  handleExternalSettingsChange('showImportNotification', e.target.checked)
                }
                className="w-4 h-4"
              />
              <span className="text-sm">{t('librarySettings.showImportNotification')}</span>
            </label>
          </div>
        )}
      </section>

      {/* Server Sources Section */}
      <section className="border border-border rounded-lg overflow-hidden">
        <button
          onClick={() => toggleSection('servers')}
          className="w-full flex items-center justify-between p-4 bg-muted/30 hover:bg-muted/50 transition-colors"
        >
          <div className="flex items-center gap-3">
            <Cloud className="w-5 h-5 text-primary" />
            <div className="text-left">
              <h3 className="font-medium">{t('sources.serverSources')}</h3>
              <p className="text-sm text-muted-foreground">
                {t('sources.serverSourcesDescription')}
              </p>
            </div>
          </div>
          {expandedSection === 'servers' ? (
            <ChevronUp className="w-5 h-5 text-muted-foreground" />
          ) : (
            <ChevronDown className="w-5 h-5 text-muted-foreground" />
          )}
        </button>

        {expandedSection === 'servers' && (
          <div className="p-4 space-y-4">
            {/* Server list */}
            {serverSources.length > 0 ? (
              <div className="space-y-3">
                {serverSources.map((source) => (
                  <div
                    key={source.id}
                    className="p-4 bg-muted/20 rounded-lg space-y-3"
                  >
                    {/* Server header */}
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <Server className="w-5 h-5 text-muted-foreground" />
                        <div>
                          <p className="font-medium">{source.name}</p>
                          <p className="text-sm text-muted-foreground">{source.serverUrl}</p>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        {source.isOnline ? (
                          <Wifi className="w-4 h-4 text-green-500" />
                        ) : (
                          <WifiOff className="w-4 h-4 text-muted-foreground" />
                        )}
                        {source.isActive && (
                          <span className="px-2 py-0.5 bg-primary/20 text-primary text-xs rounded-full">
                            {t('sources.active')}
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Auth status */}
                    <div className="flex items-center justify-between text-sm">
                      {source.isAuthenticated ? (
                        <span className="text-muted-foreground">
                          {t('sources.signedInAs', { username: source.username })}
                        </span>
                      ) : (
                        <span className="text-muted-foreground">
                          {t('sources.notSignedIn')}
                        </span>
                      )}
                      {source.lastSyncAt && (
                        <span className="text-muted-foreground">
                          {t('sources.lastSync')}: {new Date(source.lastSyncAt).toLocaleString()}
                        </span>
                      )}
                    </div>

                    {/* Auth form (if showing) */}
                    {authSourceId === source.id && (
                      <div className="space-y-3 p-3 bg-background rounded-lg">
                        <input
                          type="text"
                          value={authUsername}
                          onChange={(e) => setAuthUsername(e.target.value)}
                          placeholder={t('sources.username')}
                          className="w-full px-3 py-2 rounded-lg bg-muted border border-border"
                          autoComplete="username"
                        />
                        <input
                          type="password"
                          value={authPassword}
                          onChange={(e) => setAuthPassword(e.target.value)}
                          placeholder={t('sources.password')}
                          className="w-full px-3 py-2 rounded-lg bg-muted border border-border"
                          autoComplete="current-password"
                        />
                        <div className="flex gap-2">
                          <button
                            onClick={() => handleAuthenticate(source.id)}
                            disabled={authenticating || !authUsername.trim() || !authPassword.trim()}
                            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
                          >
                            {authenticating ? (
                              <Loader2 className="w-4 h-4 animate-spin" />
                            ) : (
                              <LogIn className="w-4 h-4" />
                            )}
                            {t('sources.signIn')}
                          </button>
                          <button
                            onClick={() => {
                              setAuthSourceId(null);
                              setAuthUsername('');
                              setAuthPassword('');
                            }}
                            className="px-4 py-2 bg-muted rounded-lg hover:bg-muted/80"
                          >
                            {t('common.cancel')}
                          </button>
                        </div>
                      </div>
                    )}

                    {/* Sync progress (if syncing) */}
                    {syncingSourceId === source.id && syncStatus && (
                      <div className="space-y-2 p-3 bg-background rounded-lg">
                        <div className="flex items-center justify-between text-sm">
                          <span>{syncStatus.currentOperation}</span>
                          <span>{Math.round(syncStatus.progress * 100)}%</span>
                        </div>
                        <div className="w-full h-2 bg-muted rounded-full overflow-hidden">
                          <div
                            className="h-full bg-primary transition-all duration-300"
                            style={{ width: `${syncStatus.progress * 100}%` }}
                          />
                        </div>
                        {syncStatus.currentItem && (
                          <p className="text-xs text-muted-foreground truncate">
                            {syncStatus.currentItem}
                          </p>
                        )}
                        <button
                          onClick={() => handleCancelSync(source.id)}
                          className="text-sm text-destructive hover:text-destructive/80"
                        >
                          {t('common.cancel')}
                        </button>
                      </div>
                    )}

                    {/* Actions */}
                    <div className="flex flex-wrap gap-2">
                      {!source.isAuthenticated ? (
                        <button
                          onClick={() => setAuthSourceId(source.id)}
                          className="flex items-center gap-2 px-3 py-1.5 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 text-sm"
                        >
                          <LogIn className="w-4 h-4" />
                          {t('sources.signIn')}
                        </button>
                      ) : (
                        <>
                          {!source.isActive && (
                            <button
                              onClick={() => handleSetActive(source.id)}
                              className="flex items-center gap-2 px-3 py-1.5 bg-muted rounded-lg hover:bg-muted/80 text-sm"
                            >
                              <Check className="w-4 h-4" />
                              {t('sources.setActive')}
                            </button>
                          )}
                          <button
                            onClick={() => handleSyncFromServer(source.id)}
                            disabled={syncingSourceId !== null}
                            className="flex items-center gap-2 px-3 py-1.5 bg-muted rounded-lg hover:bg-muted/80 text-sm disabled:opacity-50"
                          >
                            <Download className="w-4 h-4" />
                            {t('sources.syncFromServer')}
                          </button>
                          <button
                            onClick={() => handleLogout(source.id)}
                            className="flex items-center gap-2 px-3 py-1.5 bg-muted rounded-lg hover:bg-muted/80 text-sm"
                          >
                            <LogOut className="w-4 h-4" />
                            {t('sources.signOut')}
                          </button>
                        </>
                      )}
                      <button
                        onClick={() => handleRemoveServer(source.id)}
                        className="flex items-center gap-2 px-3 py-1.5 bg-destructive/10 text-destructive rounded-lg hover:bg-destructive/20 text-sm"
                      >
                        <Trash2 className="w-4 h-4" />
                        {t('common.delete')}
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground text-center py-4">
                {t('sources.noServers')}
              </p>
            )}

            {/* Add server form */}
            {addingServer ? (
              <div className="space-y-3 p-4 bg-muted/20 rounded-lg">
                <input
                  type="text"
                  value={newServerUrl}
                  onChange={(e) => {
                    setNewServerUrl(e.target.value);
                    setConnectionTestResult(null);
                  }}
                  placeholder={t('sources.serverUrlPlaceholder')}
                  className="w-full px-3 py-2 rounded-lg bg-background border border-border"
                />

                {/* Test connection button and result */}
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleTestConnection}
                    disabled={testingConnection || !newServerUrl.trim()}
                    className="flex items-center gap-2 px-3 py-1.5 bg-muted rounded-lg hover:bg-muted/80 text-sm disabled:opacity-50"
                  >
                    {testingConnection ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <RefreshCw className="w-4 h-4" />
                    )}
                    {t('sources.testConnection')}
                  </button>
                  {connectionTestResult && (
                    <span className={`text-sm ${connectionTestResult.success ? 'text-green-500' : 'text-destructive'}`}>
                      {connectionTestResult.success
                        ? `${connectionTestResult.name} v${connectionTestResult.version}`
                        : connectionTestResult.error}
                    </span>
                  )}
                </div>

                <input
                  type="text"
                  value={newServerName}
                  onChange={(e) => setNewServerName(e.target.value)}
                  placeholder={t('sources.serverName')}
                  className="w-full px-3 py-2 rounded-lg bg-background border border-border"
                />

                <div className="flex gap-2">
                  <button
                    onClick={handleAddServer}
                    disabled={!newServerName.trim() || !newServerUrl.trim()}
                    className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
                  >
                    <Check className="w-4 h-4" />
                    {t('common.save')}
                  </button>
                  <button
                    onClick={() => {
                      setAddingServer(false);
                      setNewServerName('');
                      setNewServerUrl('');
                      setConnectionTestResult(null);
                    }}
                    className="px-4 py-2 bg-muted rounded-lg hover:bg-muted/80"
                  >
                    {t('common.cancel')}
                  </button>
                </div>
              </div>
            ) : (
              <button
                onClick={() => setAddingServer(true)}
                className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90"
              >
                <Plus className="w-4 h-4" />
                {t('sources.addServer')}
              </button>
            )}
          </div>
        )}
      </section>
    </div>
  );
}
