import { X, Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface UpdateInfo {
  version: string;
  date?: string;
  body?: string;
}

interface UpdateDialogProps {
  open: boolean;
  onClose: () => void;
  onInstall: () => void;
  updateInfo: UpdateInfo | null;
  isInstalling?: boolean;
  progress?: number;
}

export function UpdateDialog({
  open,
  onClose,
  onInstall,
  updateInfo,
  isInstalling = false,
  progress = 0,
}: UpdateDialogProps) {
  const { t } = useTranslation();

  if (!open || !updateInfo) return null;

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
            className="text-muted-foreground hover:text-foreground transition-colors"
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
            <p className="text-xl font-semibold text-primary">v{updateInfo.version}</p>
          </div>

          {updateInfo.body && (
            <div>
              <p className="text-sm font-medium mb-2">{t('updateDialog.releaseNotes')}</p>
              <div className="text-sm text-muted-foreground max-h-48 overflow-y-auto bg-accent/20 rounded p-3 whitespace-pre-wrap">
                {updateInfo.body}
              </div>
            </div>
          )}

          {isInstalling && (
            <div className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">{t('updateDialog.downloading')}</span>
                <span className="font-medium">{progress}%</span>
              </div>
              <div className="w-full bg-accent rounded-full h-2">
                <div
                  className="bg-primary h-2 rounded-full transition-all duration-300"
                  style={{ width: `${progress}%` }}
                />
              </div>
            </div>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t">
          <button
            onClick={onClose}
            className="px-4 py-2 rounded hover:bg-accent transition-colors"
            disabled={isInstalling}
          >
            {t('updateDialog.later')}
          </button>
          <button
            onClick={onInstall}
            disabled={isInstalling}
            className={`px-4 py-2 rounded transition-colors bg-primary hover:bg-primary/90 text-primary-foreground flex items-center gap-2 ${
              isInstalling ? 'opacity-50 cursor-not-allowed' : ''
            }`}
          >
            <Download className="w-4 h-4" />
            {isInstalling ? t('updateDialog.installing') : t('updateDialog.installNow')}
          </button>
        </div>
      </div>
    </div>
  );
}
