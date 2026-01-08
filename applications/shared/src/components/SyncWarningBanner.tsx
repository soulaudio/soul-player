import { useSyncStore } from '../stores/sync';
import { useTranslation } from 'react-i18next';

export function SyncWarningBanner() {
  const { isSyncing, progress } = useSyncStore();
  const { t } = useTranslation();

  if (!isSyncing) return null;

  return (
    <div className="bg-yellow-500/10 border-b border-yellow-500/20 px-4 py-2">
      <div className="flex items-center gap-3">
        <svg
          className="w-5 h-5 text-yellow-500 flex-shrink-0"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
          />
        </svg>

        <div className="flex-1">
          <p className="text-sm font-medium text-yellow-500">
            {t('sync.warning.title')}
          </p>
          <p className="text-xs text-yellow-500/80">
            {t('sync.warning.message')}
          </p>
        </div>

        {/* Progress bar */}
        {progress && (
          <>
            <div className="w-48 bg-yellow-500/20 rounded-full h-2 flex-shrink-0">
              <div
                className="bg-yellow-500 h-2 rounded-full transition-all duration-300"
                style={{ width: `${progress.percentage}%` }}
              />
            </div>

            <span className="text-xs text-yellow-500/80 flex-shrink-0">
              {progress.percentage.toFixed(0)}%
            </span>
          </>
        )}
      </div>
    </div>
  );
}
