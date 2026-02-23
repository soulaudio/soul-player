import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useTranslation } from 'react-i18next';
import { Loader2, FolderSync, Check, AlertCircle } from 'lucide-react';

interface ScanProgress {
  id: number;
  librarySourceId: number;
  librarySourceName: string | null;
  startedAt: number;
  completedAt: number | null;
  totalFiles: number | null;
  processedFiles: number;
  newFiles: number;
  updatedFiles: number;
  removedFiles: number;
  errors: number;
  status: string;
  errorMessage: string | null;
  percentage: number;
}

interface ScanProgressIndicatorProps {
  /** Position of the indicator */
  position?: 'footer' | 'floating';
  /** Called when all scans complete */
  onComplete?: () => void;
}

export function ScanProgressIndicator({
  position = 'footer',
  onComplete,
}: ScanProgressIndicatorProps) {
  const { t } = useTranslation();
  const [scans, setScans] = useState<ScanProgress[]>([]);
  const [expanded, setExpanded] = useState(false);

  // Poll for running scans
  useEffect(() => {
    let previousScanCount = 0;
    let idleCount = 0; // Track consecutive idle polls
    let interval: NodeJS.Timeout | null = null;

    const fetchScans = async () => {
      try {
        const runningScans = await invoke<ScanProgress[]>('get_running_scans');
        setScans(runningScans);

        // Check if all scans completed (compare with previous count, not state)
        if (runningScans.length === 0 && previousScanCount > 0) {
          onComplete?.();
        }

        // Stop polling after 3 consecutive idle polls (1.5 seconds of no scans)
        if (runningScans.length === 0) {
          idleCount++;
          if (idleCount >= 3 && interval) {
            console.log('[ScanProgressIndicator] No scans for 1.5s, stopping poll interval');
            clearInterval(interval);
            interval = null;
          }
        } else {
          idleCount = 0; // Reset idle counter when scans are active
        }

        previousScanCount = runningScans.length;
      } catch (err) {
        console.error('[ScanProgressIndicator] Failed to fetch running scans:', err);
      }
    };

    // Initial fetch
    console.log('[ScanProgressIndicator] Starting scan polling');
    void fetchScans();

    // Poll every 500ms while there are active scans
    interval = setInterval(fetchScans, 500);

    return () => {
      if (interval) {
        console.log('[ScanProgressIndicator] Cleaning up poll interval');
        clearInterval(interval);
      }
    };
  }, [onComplete]); // Remove scans.length dependency to prevent interval leak

  // Listen for scan events
  useEffect(() => {
    const unlistenFunctions: (() => void)[] = [];
    let isMounted = true;

    const setupListeners = async () => {
      try {
        if (!isMounted) return;

        const unlistenStart = await listen('scan-started', () => {
          if (!isMounted) return;
          // Refresh scans when a new one starts
          invoke<ScanProgress[]>('get_running_scans').then(setScans).catch(console.error);
        });
        unlistenFunctions.push(unlistenStart);

        const unlistenProgress = await listen<ScanProgress>('scan-progress', (event) => {
          if (!isMounted) return;
          setScans((prev) =>
            prev.map((s) => (s.id === event.payload.id ? event.payload : s))
          );
        });
        unlistenFunctions.push(unlistenProgress);

        const unlistenComplete = await listen<{ sourceId: number }>('scan-complete', () => {
          if (!isMounted) return;
          // Refresh scans when one completes
          invoke<ScanProgress[]>('get_running_scans').then(setScans).catch(console.error);
        });
        unlistenFunctions.push(unlistenComplete);
      } catch (error) {
        console.error('[ScanProgressIndicator] Failed to set up listeners:', error);
      }
    };

    void setupListeners();

    return () => {
      isMounted = false;
      unlistenFunctions.forEach(fn => fn());
    };
  }, []);

  // Don't render if no active scans
  if (scans.length === 0) {
    return null;
  }

  const totalProgress =
    scans.length > 0
      ? scans.reduce((sum, s) => sum + s.percentage, 0) / scans.length
      : 0;

  const totalProcessed = scans.reduce((sum, s) => sum + s.processedFiles, 0);
  const totalFiles = scans.reduce((sum, s) => sum + (s.totalFiles || 0), 0);

  if (position === 'footer') {
    return (
      <div
        className="fixed bottom-16 left-0 right-0 z-40 px-4 pointer-events-none"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="max-w-xl mx-auto pointer-events-auto">
          <div className="bg-card border rounded-lg shadow-lg overflow-hidden">
            {/* Compact view */}
            <div className="flex items-center gap-3 p-3 cursor-pointer hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors duration-[var(--transition-duration)]">
              <Loader2 className="w-4 h-4 animate-spin text-primary flex-shrink-0" />
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between text-sm">
                  <span className="font-medium truncate">
                    {scans.length === 1
                      ? t('scan.scanningSource', { name: scans[0].librarySourceName || 'Library' })
                      : t('scan.scanningSources', { count: scans.length })}
                  </span>
                  <span className="text-muted-foreground ml-2">
                    {totalProcessed}/{totalFiles}
                  </span>
                </div>
                {/* Progress bar */}
                <div className="mt-2 h-1.5 bg-muted rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary transition-all duration-300"
                    style={{ width: `${totalProgress}%` }}
                  />
                </div>
              </div>
            </div>

            {/* Expanded view */}
            {expanded && scans.length > 1 && (
              <div className="border-t p-3 space-y-2">
                {scans.map((scan) => (
                  <div key={scan.id} className="flex items-center gap-2 text-sm">
                    <FolderSync className="w-4 h-4 text-muted-foreground" />
                    <span className="flex-1 truncate">
                      {scan.librarySourceName || 'Unknown'}
                    </span>
                    <span className="text-muted-foreground">
                      {Math.round(scan.percentage)}%
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  // Floating position (top-right notification style)
  return (
    <div className="fixed top-20 right-4 z-50 w-80">
      <div className="bg-card border rounded-lg shadow-lg overflow-hidden">
        <div className="p-4">
          <div className="flex items-center gap-3 mb-3">
            <Loader2 className="w-5 h-5 animate-spin text-primary" />
            <div>
              <p className="font-medium text-sm">
                {t('scan.scanning')}
              </p>
              <p className="text-xs text-muted-foreground">
                {t('scan.filesProgress', { processed: totalProcessed, total: totalFiles })}
              </p>
            </div>
          </div>

          {/* Progress bar */}
          <div className="h-2 bg-muted rounded-full overflow-hidden">
            <div
              className="h-full bg-primary transition-all duration-300"
              style={{ width: `${totalProgress}%` }}
            />
          </div>

          {/* Stats */}
          <div className="mt-3 flex items-center gap-4 text-xs text-muted-foreground">
            {scans.reduce((sum, s) => sum + s.newFiles, 0) > 0 && (
              <span className="flex items-center gap-1">
                <Check className="w-3 h-3 text-green-500" />
                {t('scan.newFiles', { count: scans.reduce((sum, s) => sum + s.newFiles, 0) })}
              </span>
            )}
            {scans.reduce((sum, s) => sum + s.errors, 0) > 0 && (
              <span className="flex items-center gap-1">
                <AlertCircle className="w-3 h-3 text-red-500" />
                {t('scan.errors', { count: scans.reduce((sum, s) => sum + s.errors, 0) })}
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
