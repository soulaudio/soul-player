import { useState, useEffect } from 'react';
import { usePlayerCommands, type Source } from '../contexts/PlayerCommandsContext';
import { debug } from '../utils/debug';
import { useTranslation } from 'react-i18next';

interface SourcesDialogProps {
  open: boolean;
  onClose: () => void;
}

export function SourcesDialog({ open, onClose }: SourcesDialogProps) {
  const { t } = useTranslation();
  const [sources, setSources] = useState<Source[]>([]);
  const [loading, setLoading] = useState(false);
  const commands = usePlayerCommands();

  useEffect(() => {
    if (open) {
      loadSources();
    }
  }, [open, commands]);

  const loadSources = async () => {
    try {
      setLoading(true);
      const result = await commands.getAllSources();
      setSources(result);
    } catch (err) {
      debug.error('[SourcesDialog] Failed to load sources:', err);
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={onClose}>
      <div className="bg-background border rounded-lg shadow-lg w-full max-w-2xl max-h-[80vh] overflow-hidden flex flex-col" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b">
          <h2 className="text-xl font-semibold">{t('sourcesDialog.title')}</h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-foreground/[var(--hover-bg-opacity)] rounded-full transition-colors"
            aria-label={t('common.close')}
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-auto p-6">
          {loading ? (
            <div className="text-center py-8">
              <div className="animate-spin w-8 h-8 border-4 border-primary border-t-transparent rounded-full mx-auto"></div>
              <div className="text-muted-foreground mt-4">{t('sourcesDialog.loadingSources')}</div>
            </div>
          ) : (
            <div className="space-y-4">
              <p className="text-muted-foreground text-sm">
                {t('sourcesDialog.description')}
              </p>

              <div className="space-y-3">
                {sources.map((source) => (
                  <div
                    key={source.id}
                    className="p-4 border rounded-lg flex items-center justify-between hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors"
                  >
                    <div className="flex items-center gap-4">
                      <div className={`w-2 h-2 rounded-full ${source.isOnline ? 'bg-green-500' : 'bg-gray-400'}`} />
                      <div>
                        <div className="font-medium">{source.name}</div>
                        <div className="text-sm text-muted-foreground">
                          {source.sourceType === 'local' ? t('sourcesDialog.localFiles') : t('sourcesDialog.remoteServer')}
                          {source.isActive && (
                            <span className="ml-2 text-xs bg-primary/20 text-primary px-2 py-0.5 rounded">
                              {t('sourcesDialog.active')}
                            </span>
                          )}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2">
                      <div className={`text-xs px-2 py-1 rounded ${
                        source.isOnline
                          ? 'bg-green-500/10 text-green-500'
                          : 'bg-gray-500/10 text-gray-500'
                      }`}>
                        {source.isOnline ? t('sourcesDialog.online') : t('sourcesDialog.offline')}
                      </div>
                    </div>
                  </div>
                ))}
              </div>

              <div className="mt-6 p-4 bg-muted/40 rounded-lg text-sm">
                <div className="font-medium mb-2">{t('sourcesDialog.aboutSources')}</div>
                <ul className="space-y-1 text-muted-foreground">
                  <li>• <strong>{t('sourcesDialog.localFiles')}</strong>: {t('sourcesDialog.aboutLocal')}</li>
                  <li>• <strong>{t('sourcesDialog.remoteServer')}</strong>: {t('sourcesDialog.aboutRemote')}</li>
                  <li>• {t('sourcesDialog.aboutMultiple')}</li>
                  <li>• {t('sourcesDialog.aboutCache')}</li>
                </ul>
              </div>

              <div className="flex gap-3 mt-6">
                <button
                  className="flex-1 px-4 py-2 border rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)] disabled:opacity-[var(--disabled-opacity)]"
                  disabled
                >
                  <div className="flex items-center justify-center gap-2">
                    <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                    </svg>
                    {t('sourcesDialog.addServer')}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">{t('sourcesDialog.addServerComingSoon')}</div>
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t flex justify-end">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-all duration-[var(--transition-duration)]"
          >
            {t('sourcesDialog.close')}
          </button>
        </div>
      </div>
    </div>
  );
}
