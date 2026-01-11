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

  // Load all settings
  const loadSettings = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      const [sourcesData, managedData, externalData, presetsData] = await Promise.all([
        invoke<LibrarySource[]>('get_library_sources'),
        invoke<ManagedLibrarySettings | null>('get_managed_library_settings'),
        invoke<ExternalFileSettings>('get_external_file_settings'),
        invoke<[string, string, string][]>('get_path_template_presets'),
      ]);

      setSources(sourcesData);
      setManagedSettings(managedData);
      setExternalSettings(externalData);
      setPresets(
        presetsData.map(([id, template, example]) => ({ id, template, example }))
      );

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
      setError(String(err));
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

  const handleRescanAll = async () => {
    try {
      setScanningSourceId(-1); // Use -1 to indicate "all"
      await invoke('rescan_all_sources');
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
                <input
                  type="text"
                  value={newSourcePath}
                  onChange={(e) => setNewSourcePath(e.target.value)}
                  placeholder={t('librarySettings.sourcePath')}
                  className="w-full px-3 py-2 rounded-lg bg-background border border-border"
                />
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
                  <button
                    onClick={handleRescanAll}
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
              <input
                type="text"
                value={managedSettings?.libraryPath ?? ''}
                onChange={(e) => handleManagedSettingsChange('libraryPath', e.target.value)}
                placeholder={t('librarySettings.libraryPathPlaceholder')}
                className="w-full px-3 py-2 rounded-lg bg-muted border border-border"
              />
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
    </div>
  );
}
