import { useSyncStore } from '../stores/sync';
import { useTranslation } from 'react-i18next';

export function SyncAlert() {
  const { progress, isSyncing, syncRequired, startSync } = useSyncStore();
  const { t } = useTranslation();

  if (!isSyncing && !syncRequired) {
    return null;
  }

  const handleClick = () => {
    if (syncRequired && !isSyncing) {
      startSync('manual');
    }
  };

  const getPhaseLabel = (phase?: string): string => {
    if (!phase) return t('sync.status.idle');
    const phaseMap: Record<string, string> = {
      Scanning: t('sync.phase.scanning'),
      MetadataExtraction: t('sync.phase.extracting'),
      Validation: t('sync.phase.validating'),
      Cleanup: t('sync.phase.cleaning'),
    };
    return phaseMap[phase] || phase;
  };

  const getTooltipContent = () => {
    if (isSyncing && progress) {
      return t('sync.alert.syncing', {
        percentage: progress.percentage.toFixed(0),
        phase: getPhaseLabel(progress.phase),
      });
    }
    if (syncRequired) {
      return t('sync.alert.required');
    }
    return '';
  };

  return (
    <div className="relative group">
      <button
        onClick={handleClick}
        className="p-2 rounded-lg hover:bg-accent transition-colors"
        aria-label="Sync Status"
        title={getTooltipContent()}
      >
        {/* Alert Icon with animation */}
        <svg
          className={`w-5 h-5 ${isSyncing ? 'animate-spin' : 'animate-pulse text-yellow-500'}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          {isSyncing ? (
            // Spinning sync icon
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
            />
          ) : (
            // Alert/warning icon
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          )}
        </svg>

        {/* Progress indicator */}
        {isSyncing && progress && (
          <div className="absolute bottom-0 right-0 w-3 h-3 bg-blue-500 rounded-full border-2 border-background" />
        )}
      </button>

      {/* Tooltip */}
      <div className="absolute top-full left-1/2 -translate-x-1/2 mt-2 px-2 py-1 bg-popover text-popover-foreground text-xs rounded shadow-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50">
        {getTooltipContent()}
      </div>
    </div>
  );
}
