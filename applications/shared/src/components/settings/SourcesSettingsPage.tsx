import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import {
  Cloud,
  Plus,
  Trash2,
  RefreshCw,
  ChevronDown,
  ChevronUp,
  AlertCircle,
  Check,
  Loader2,
  LogIn,
  LogOut,
  Server,
  Wifi,
  WifiOff,
  Upload,
  Download,
  Folder,
} from 'lucide-react';

// Types matching the Tauri backend
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

export function SourcesSettingsPage() {
  const { t } = useTranslation();

  // State
  const [sources, setSources] = useState<SourceInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // UI state
  const [expandedSection, setExpandedSection] = useState<string | null>('servers');
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

  // Load sources
  const loadSources = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);

      const sourcesData = await invoke<SourceInfo[]>('get_sources');
      setSources(sourcesData);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSources();
  }, [loadSources]);

  // Test server connection
  const handleTestConnection = async () => {
    if (!newServerUrl.trim()) return;

    try {
      setTestingConnection(true);
      setConnectionTestResult(null);

      const result = await invoke<ServerTestResult>('test_server_connection', {
        url: newServerUrl.trim(),
      });

      setConnectionTestResult(result);

      // Auto-fill name from server if successful and name is empty
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

  // Add server source
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
      await loadSources();
    } catch (err) {
      setError(String(err));
    }
  };

  // Remove source
  const handleRemoveSource = async (sourceId: number) => {
    try {
      await invoke('remove_source', { sourceId });
      await loadSources();
    } catch (err) {
      setError(String(err));
    }
  };

  // Authenticate with server
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
        await loadSources();
      } else {
        setError(result.error || t('sources.authFailed'));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setAuthenticating(false);
    }
  };

  // Logout from server
  const handleLogout = async (sourceId: number) => {
    try {
      await invoke('logout_source', { sourceId });
      await loadSources();
    } catch (err) {
      setError(String(err));
    }
  };

  // Set active source
  const handleSetActive = async (sourceId: number) => {
    try {
      await invoke('set_active_source', { sourceId });
      await loadSources();
    } catch (err) {
      setError(String(err));
    }
  };

  // Sync from server
  const handleSyncFromServer = async (sourceId: number) => {
    try {
      setSyncingSourceId(sourceId);
      setSyncStatus({ status: 'syncing', progress: 0, currentOperation: 'starting', currentItem: null, processedItems: 0, totalItems: 0, error: null });

      const result = await invoke<SyncResult>('sync_from_server', { sourceId });

      if (result.success) {
        setSyncStatus(null);
        await loadSources();
      } else {
        setError(result.errors.join(', ') || t('sources.syncFailed'));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setSyncingSourceId(null);
    }
  };

  // Cancel sync
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

  // Get server sources only
  const serverSources = sources.filter((s) => s.sourceType === 'server');
  const localSource = sources.find((s) => s.sourceType === 'local');

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

      {/* Local Source Info */}
      {localSource && (
        <section className="border border-border rounded-lg overflow-hidden">
          <div className="p-4 bg-muted/30">
            <div className="flex items-center gap-3">
              <Folder className="w-5 h-5 text-primary" />
              <div>
                <h3 className="font-medium">{t('sources.localSource')}</h3>
                <p className="text-sm text-muted-foreground">
                  {t('sources.localSourceDescription')}
                </p>
              </div>
            </div>
          </div>
        </section>
      )}

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
                        onClick={() => handleRemoveSource(source.id)}
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
