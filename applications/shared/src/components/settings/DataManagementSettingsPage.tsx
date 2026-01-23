import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Trash2, AlertTriangle, RefreshCw } from 'lucide-react';
import { DesktopOnly } from '../../contexts/PlatformContext';

interface ResetDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

function ResetConfirmDialog({ isOpen, onClose, onConfirm }: ResetDialogProps) {
  const { t } = useTranslation();
  const [confirmText, setConfirmText] = useState('');
  const [isResetting, setIsResetting] = useState(false);
  const resetTimerRef = useRef<NodeJS.Timeout | null>(null);

  // Cleanup timer on unmount
  useEffect(() => {
    return () => {
      if (resetTimerRef.current) {
        clearTimeout(resetTimerRef.current);
      }
    };
  }, []);

  if (!isOpen) return null;

  const handleConfirm = async () => {
    setIsResetting(true);
    await onConfirm();
    // App will restart, but if it fails, re-enable the button after 5 seconds
    resetTimerRef.current = setTimeout(() => {
      setIsResetting(false);
      resetTimerRef.current = null;
    }, 5000);
  };

  const isConfirmed = confirmText.toLowerCase() === 'reset';

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background border border-border rounded-lg shadow-lg max-w-md w-full mx-4 p-6">
        <div className="flex items-start gap-3 mb-4">
          <AlertTriangle className="w-6 h-6 text-destructive flex-shrink-0 mt-1" />
          <div>
            <h2 className="text-lg font-semibold mb-2">
              {t('settings.dataManagement.resetDialog.title')}
            </h2>
            <p className="text-sm text-muted-foreground">
              {t('settings.dataManagement.resetDialog.description')}
            </p>
          </div>
        </div>

        <div className="bg-destructive/10 border border-destructive/20 rounded-lg p-4 mb-4">
          <p className="text-sm font-medium mb-2">
            {t('settings.dataManagement.resetDialog.willBeDeleted')}
          </p>
          <ul className="text-sm text-muted-foreground space-y-1 list-disc list-inside">
            <li>{t('settings.dataManagement.resetDialog.items.library')}</li>
            <li>{t('settings.dataManagement.resetDialog.items.playlists')}</li>
            <li>{t('settings.dataManagement.resetDialog.items.settings')}</li>
            <li>{t('settings.dataManagement.resetDialog.items.logs')}</li>
            <li>{t('settings.dataManagement.resetDialog.items.cache')}</li>
          </ul>
        </div>

        <div className="mb-4">
          <label className="block text-sm font-medium mb-2">
            {t('settings.dataManagement.resetDialog.confirmLabel')}
            <span className="text-destructive ml-1">*</span>
          </label>
          <input
            type="text"
            value={confirmText}
            onChange={(e) => setConfirmText(e.target.value)}
            placeholder={t('settings.dataManagement.resetDialog.confirmPlaceholder')}
            className="w-full px-3 py-2 bg-background border border-border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
            disabled={isResetting}
          />
          <p className="text-xs text-muted-foreground mt-1">
            {t('settings.dataManagement.resetDialog.confirmHint')}
          </p>
        </div>

        <div className="flex gap-2 justify-end">
          <button
            onClick={onClose}
            disabled={isResetting}
            className="px-4 py-2 text-sm rounded-lg border border-border hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)] disabled:opacity-[var(--disabled-opacity)]"
          >
            {t('settings.dataManagement.resetDialog.cancel')}
          </button>
          <button
            onClick={handleConfirm}
            disabled={!isConfirmed || isResetting}
            className="px-4 py-2 text-sm rounded-lg bg-destructive text-destructive-foreground hover:opacity-[var(--hover-button-opacity)] transition-all duration-[var(--transition-duration)] disabled:opacity-[var(--disabled-opacity)] flex items-center gap-2"
          >
            {isResetting && <RefreshCw className="w-4 h-4 animate-spin" />}
            {t('settings.dataManagement.resetDialog.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
}

export function DataManagementSettingsPage() {
  const { t } = useTranslation();
  const [isResetDialogOpen, setIsResetDialogOpen] = useState(false);
  const [resetError, setResetError] = useState<string | null>(null);

  const handleResetToFactory = async () => {
    try {
      setResetError(null);

      // Desktop-specific: Call Tauri command to reset
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('reset_to_factory_settings');

      // If we reach here, reset failed (app should have exited)
      setResetError(t('settings.dataManagement.resetError'));
    } catch (error) {
      console.error('Failed to reset to factory settings:', error);
      setResetError(
        error instanceof Error ? error.message : t('settings.dataManagement.resetError')
      );
    } finally {
      setIsResetDialogOpen(false);
    }
  };

  return (
    <div className="p-6 max-w-2xl">
      <h1 className="text-2xl font-bold mb-2">{t('settings.dataManagement.title')}</h1>
      <p className="text-muted-foreground mb-6">{t('settings.dataManagement.description')}</p>

      {/* Warning Banner */}
      <div className="bg-warning/10 border border-warning/20 rounded-lg p-4 mb-6 flex items-start gap-3">
        <AlertTriangle className="w-5 h-5 text-warning flex-shrink-0 mt-0.5" />
        <div className="text-sm">
          <p className="font-medium mb-1">{t('settings.dataManagement.warning.title')}</p>
          <p className="text-muted-foreground">{t('settings.dataManagement.warning.description')}</p>
        </div>
      </div>

      {/* Reset Section */}
      <DesktopOnly>
        <div className="border border-border rounded-lg p-6">
          <div className="flex items-start gap-4">
            <div className="p-3 bg-destructive/10 rounded-lg">
              <Trash2 className="w-6 h-6 text-destructive" />
            </div>
            <div className="flex-1">
              <h2 className="text-lg font-semibold mb-2">
                {t('settings.dataManagement.reset.title')}
              </h2>
              <p className="text-sm text-muted-foreground mb-4">
                {t('settings.dataManagement.reset.description')}
              </p>

              <ul className="text-sm text-muted-foreground space-y-1 mb-4 list-disc list-inside">
                <li>{t('settings.dataManagement.reset.effects.library')}</li>
                <li>{t('settings.dataManagement.reset.effects.playlists')}</li>
                <li>{t('settings.dataManagement.reset.effects.settings')}</li>
                <li>{t('settings.dataManagement.reset.effects.logs')}</li>
                <li>{t('settings.dataManagement.reset.effects.restart')}</li>
              </ul>

              {resetError && (
                <div className="bg-destructive/10 border border-destructive/20 rounded-lg p-3 mb-4">
                  <p className="text-sm text-destructive">{resetError}</p>
                </div>
              )}

              <button
                onClick={() => setIsResetDialogOpen(true)}
                className="px-4 py-2 text-sm rounded-lg bg-destructive text-destructive-foreground hover:opacity-90 transition-opacity flex items-center gap-2"
              >
                <Trash2 className="w-4 h-4" />
                {t('settings.dataManagement.reset.button')}
              </button>
            </div>
          </div>
        </div>
      </DesktopOnly>

      <ResetConfirmDialog
        isOpen={isResetDialogOpen}
        onClose={() => setIsResetDialogOpen(false)}
        onConfirm={handleResetToFactory}
      />
    </div>
  );
}
