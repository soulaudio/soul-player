import { X, Download, Terminal, AlertCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { debug } from '@soul-player/shared';

interface UpdateInfo {
  version: string;
  date?: string;
  body?: string;
}

interface InstallationInfo {
  method: {
    type: 'appimage' | 'deb' | 'rpm' | 'flatpak' | 'snap' | 'aur' | 'unknown';
  };
  update_command: string | null;
  supports_auto_update: boolean;
}

interface UpdateDialogProps {
  open: boolean;
  onClose: () => void;
  onInstall: () => void;
  updateInfo: UpdateInfo | null;
  isInstalling?: boolean;
  progress?: number;
  installError?: string | null;
}

export function UpdateDialog({
  open,
  onClose,
  onInstall,
  updateInfo,
  isInstalling = false,
  progress = 0,
  installError = null,
}: UpdateDialogProps) {
  const { t } = useTranslation();
  const [installationInfo, setInstallationInfo] = useState<InstallationInfo | null>(null);
  const [commandCopied, setCommandCopied] = useState(false);

  // Fetch installation info when dialog opens
  useEffect(() => {
    if (open) {
      invoke<InstallationInfo>('get_installation_info')
        .then(setInstallationInfo)
        .catch((err) => debug.error('Failed to get installation info:', err));
    }
  }, [open]);

  const handleCopyCommand = () => {
    if (installationInfo?.update_command) {
      navigator.clipboard.writeText(installationInfo.update_command);
      setCommandCopied(true);
      setTimeout(() => setCommandCopied(false), 2000);
    }
  };

  if (!open || !updateInfo) return null;

  const supportsAutoUpdate = installationInfo?.supports_auto_update ?? true;
  const updateCommand = installationInfo?.update_command;

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget && !isInstalling) {
      onClose();
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape' && !isInstalling) {
      onClose();
    }
  };

  return (
    <div
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
      data-testid="update-dialog"
    >
      <div className="bg-background border rounded-lg shadow-lg max-w-md w-full mx-4">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <div className="flex items-center gap-2">
            <Download className="w-5 h-5 text-primary" />
            <h2 className="text-lg font-semibold">{t('updateDialog.title')}</h2>
          </div>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]"
            disabled={isInstalling}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="px-6 py-4 space-y-4">
          <div>
            <p className="text-sm text-muted-foreground">
              {t('updateDialog.versionAvailable')}
            </p>
            <p className="text-xl font-semibold text-primary" data-testid="update-dialog-version">v{updateInfo.version}</p>
          </div>

          {updateInfo.body && (
            <div>
              <p className="text-sm font-medium mb-2">{t('updateDialog.releaseNotes')}</p>
              <div className="text-sm text-muted-foreground max-h-48 overflow-y-auto bg-accent/20 rounded p-3 whitespace-pre-wrap">
                {updateInfo.body}
              </div>
              {/* Extract GitHub release URL from notes if present */}
              {updateInfo.body.match(/https:\/\/github\.com\/[^/]+\/[^/]+\/releases\/tag\/[^\s)]+/) && (
                <a
                  href={updateInfo.body.match(/https:\/\/github\.com\/[^/]+\/[^/]+\/releases\/tag\/[^\s)]+/)?.[0]}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm text-primary hover:underline mt-2 inline-block"
                >
                  {t('updateDialog.viewFullReleaseNotes')}
                </a>
              )}
            </div>
          )}

          {/* Package manager update instructions */}
          {!supportsAutoUpdate && updateCommand && (
            <div className="p-4 bg-accent/30 rounded-lg border border-border space-y-3">
              <div className="flex items-start gap-2">
                <AlertCircle className="w-5 h-5 text-yellow-500 mt-0.5 flex-shrink-0" />
                <div className="flex-1 space-y-2">
                  <p className="text-sm font-medium text-foreground">
                    {t('updateDialog.packageManagerUpdateRequired')}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {t('updateDialog.packageManagerUpdateDescription')}
                  </p>
                  <div className="flex items-center gap-2 mt-2">
                    <code className="flex-1 px-3 py-2 bg-background border rounded text-xs font-mono">
                      {updateCommand}
                    </code>
                    <button
                      onClick={handleCopyCommand}
                      className="px-3 py-2 bg-primary/10 hover:bg-foreground/[var(--hover-bg-opacity)] text-primary rounded text-sm font-medium transition-colors duration-[var(--transition-duration)] flex items-center gap-1.5"
                    >
                      <Terminal className="w-3.5 h-3.5" />
                      {commandCopied ? t('updateDialog.copied') : t('updateDialog.copy')}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {isInstalling && (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">{t('updateDialog.downloading')}</span>
                <span className="font-medium">{progress}%</span>
              </div>
              <div className="w-full bg-accent rounded-full h-2" data-testid="update-progress-bar">
                <div
                  className="bg-primary h-2 rounded-full transition-all duration-300"
                  style={{ width: `${progress}%` }}
                />
              </div>
            </div>
          )}

          {installError && (
            <div className="flex items-start gap-2 p-3 bg-destructive/10 border border-destructive/30 rounded-lg" data-testid="update-install-error">
              <AlertCircle className="w-4 h-4 text-destructive mt-0.5 flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium text-destructive">{t('updateDialog.installFailed', 'Installation failed')}</p>
                <p className="text-xs text-destructive/80 mt-0.5 break-words">{installError}</p>
              </div>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]"
            disabled={isInstalling}
            data-testid="update-dialog-later"
          >
            {t('updateDialog.later')}
          </button>
          {supportsAutoUpdate ? (
            <button
              onClick={onInstall}
              disabled={isInstalling}
              className={`px-4 py-2 rounded transition-opacity duration-[var(--transition-duration)] bg-primary hover:opacity-[var(--hover-button-opacity)] text-primary-foreground flex items-center gap-2 ${
                isInstalling ? 'opacity-50 cursor-not-allowed' : ''
              }`}
              data-testid="update-dialog-install"
            >
              <Download className="w-4 h-4" />
              {isInstalling ? t('updateDialog.installing') : t('updateDialog.installNow')}
            </button>
          ) : (
            <a
              href={`https://github.com/soulaudio/soul-player/releases/tag/v${updateInfo.version}`}
              target="_blank"
              rel="noopener noreferrer"
              className="px-4 py-2 rounded bg-primary hover:opacity-[var(--hover-button-opacity)] text-primary-foreground flex items-center gap-2 transition-opacity duration-[var(--transition-duration)]"
            >
              <Download className="w-4 h-4" />
              {t('updateDialog.viewRelease')}
            </a>
          )}
        </div>
      </div>
    </div>
  );
}
