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
    const fetchScans = async () => {
      try {
        const runningScans = await invoke<ScanProgress[]>('get_running_scans');
        setScans(runningScans);

        // Check if all scans completed
        if (runningScans.length === 0 && scans.length > 0) {
          onComplete?.();
        }
      } catch (err) {
        console.error('Failed to fetch running scans:', err);
      }
    };

    // Initial fetch
    fetchScans();

    // Poll every 500ms while there are active scans
    const interval = setInterval(fetchScans, 500);

    return () => clearInterval(interval);
  }, [scans.length, onComplete]);

  // Listen for scan events
  useEffect(() => {
    let unlistenStart: (() => void) | null = null;
    let unlistenProgress: (() => void) | null = null;
    let unlistenComplete: (() => void) | null = null;

    const setupListeners = async () => {
      unlistenStart = await listen('scan-started', () => {
        // Refresh scans when a new one starts
        invoke<ScanProgress[]>('get_running_scans').then(setScans);
      });

      unlistenProgress = await listen<ScanProgress>('scan-progress', (event) => {
        setScans((prev) =>
          prev.map((s) => (s.id === event.payload.id ? event.payload : s))
        );
      });

      unlistenComplete = await listen<{ sourceId: number }>('scan-complete', () => {
        // Refresh scans when one completes
        invoke<ScanProgress[]>('get_running_scans').then(setScans);
      });
    };

    setupListeners();

    return () => {
      if (unlistenStart) unlistenStart();
      if (unlistenProgress) unlistenProgress();
      if (unlistenComplete) unlistenComplete();
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
            <div className="flex items-center gap-3 p-3 cursor-pointer hover:bg-muted/50 transition-colors">
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
                {totalProcessed} of {totalFiles} files
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
                {scans.reduce((sum, s) => sum + s.newFiles, 0)} new
              </span>
            )}
            {scans.reduce((sum, s) => sum + s.errors, 0) > 0 && (
              <span className="flex items-center gap-1">
                <AlertCircle className="w-3 h-3 text-red-500" />
                {scans.reduce((sum, s) => sum + s.errors, 0)} errors
              </span>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
