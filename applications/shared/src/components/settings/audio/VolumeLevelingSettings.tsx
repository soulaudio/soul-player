// Volume leveling (ReplayGain / EBU R128) settings component

import { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Info, Play, Square, RefreshCw, CheckCircle, Loader2 } from 'lucide-react';
import { debug } from '../../../utils/debug';
import { useBackend } from '../../../contexts/BackendContext';
import type { AnalysisQueueStats, AnalysisWorkerStatus } from '../../../contexts/BackendContext';

interface VolumeLevelingSettingsProps {
  mode: 'disabled' | 'replaygain_track' | 'replaygain_album' | 'ebu_r128';
  preampDb?: number;
  preventClipping?: boolean;
  onModeChange: (mode: 'disabled' | 'replaygain_track' | 'replaygain_album' | 'ebu_r128') => void;
  onPreampChange?: (preampDb: number) => void;
  onPreventClippingChange?: (prevent: boolean) => void;
}

const modes = [
  {
    value: 'disabled',
    label: 'Disabled',
    description: 'No volume normalization applied',
    targetLevel: null,
  },
  {
    value: 'replaygain_track',
    label: 'ReplayGain (Track)',
    description: 'Normalize each track independently',
    targetLevel: '-18 LUFS',
  },
  {
    value: 'replaygain_album',
    label: 'ReplayGain (Album)',
    description: 'Normalize albums while preserving relative track levels',
    targetLevel: '-18 LUFS',
  },
  {
    value: 'ebu_r128',
    label: 'EBU R128',
    description: 'European Broadcasting Union loudness standard',
    targetLevel: '-23 LUFS',
  },
] as const;

export function VolumeLevelingSettings({
  mode,
  preampDb = 0,
  preventClipping = true,
  onModeChange,
  onPreampChange,
  onPreventClippingChange,
}: VolumeLevelingSettingsProps) {
  const backend = useBackend();
  const [queueStats, setQueueStats] = useState<AnalysisQueueStats | null>(null);
  const [workerStatus, setWorkerStatus] = useState<AnalysisWorkerStatus>({ isRunning: false, tracksAnalyzed: 0 });
  const [isLoading, setIsLoading] = useState(false);
  const [lastAnalyzedTrack, setLastAnalyzedTrack] = useState<string | null>(null);

  // Local state for preamp slider to show current value during drag
  const [localPreampDb, setLocalPreampDb] = useState(preampDb);

  // Sync local state when prop changes
  useEffect(() => {
    setLocalPreampDb(preampDb);
  }, [preampDb]);

  // Debounced preamp change handler
  const handlePreampChange = async (value: number) => {
    setLocalPreampDb(value);
    if (onPreampChange) {
      onPreampChange(value);
    }
    // Note: Parent component (AudioSettingsPage) handles the backend call via onPreampChange
  };

  // Prevent clipping change handler
  const handlePreventClippingChange = async (checked: boolean) => {
    if (onPreventClippingChange) {
      onPreventClippingChange(checked);
    }
    // Note: Parent component (AudioSettingsPage) handles the backend call via onPreventClippingChange
  };

  // Load initial stats
  useEffect(() => {
    loadQueueStats();
    loadWorkerStatus();
  }, []);

  // Listen for analysis events
  useEffect(() => {
    const unlistenFunctions: (() => void)[] = [];
    let isMounted = true;

    const setupListeners = async () => {
      try {
        if (!isMounted) return;

        const unlistenProgress = await listen<{ trackId: number; trackTitle: string }>('loudness-analysis-progress', (event) => {
          if (!isMounted) return;
          setLastAnalyzedTrack(event.payload.trackTitle);
          loadQueueStats();
          loadWorkerStatus();
        });
        unlistenFunctions.push(unlistenProgress);

        const unlistenComplete = await listen('analysis-worker-complete', () => {
          if (!isMounted) return;
          setWorkerStatus((prev) => ({ isRunning: false, tracksAnalyzed: prev.tracksAnalyzed }));
          loadQueueStats();
        });
        unlistenFunctions.push(unlistenComplete);

        const unlistenStopped = await listen('analysis-worker-stopped', () => {
          if (!isMounted) return;
          setWorkerStatus((prev) => ({ isRunning: false, tracksAnalyzed: prev.tracksAnalyzed }));
        });
        unlistenFunctions.push(unlistenStopped);
      } catch (error) {
        console.error('[VolumeLevelingSettings] Failed to set up listeners:', error);
      }
    };

    void setupListeners();

    return () => {
      isMounted = false;
      unlistenFunctions.forEach(fn => fn());
    };
  }, []); // Remove workerStatus.tracksAnalyzed dependency - CRITICAL FIX for listener leak multiplier

  const loadQueueStats = async () => {
    try {
      const stats = await backend.getAnalysisQueueStats();
      setQueueStats(stats);
    } catch (error) {
      debug.error('Failed to load queue stats:', error);
    }
  };

  const loadWorkerStatus = async () => {
    try {
      const status = await backend.getAnalysisWorkerStatus();
      setWorkerStatus(status);
    } catch (error) {
      debug.error('Failed to load worker status:', error);
    }
  };

  const handleStartAnalysis = async () => {
    setIsLoading(true);
    try {
      await backend.startAnalysisWorker();
      setWorkerStatus({ ...workerStatus, isRunning: true });
    } catch (error) {
      debug.error('Failed to start analysis:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleStopAnalysis = async () => {
    setIsLoading(true);
    try {
      await backend.stopAnalysisWorker();
    } catch (error) {
      debug.error('Failed to stop analysis:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleQueueAllUnanalyzed = async () => {
    setIsLoading(true);
    try {
      const count = await backend.queueAllUnanalyzed();
      await loadQueueStats();
      if (count > 0) {
        // Auto-start worker if items were queued
        await handleStartAnalysis();
      }
    } catch (error) {
      debug.error('Failed to queue tracks:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClearCompleted = async () => {
    try {
      await backend.clearCompletedAnalysis();
      await loadQueueStats();
    } catch (error) {
      debug.error('Failed to clear completed:', error);
    }
  };

  return (
    <div className="space-y-6">
      {/* Mode Selection */}
      <div className="space-y-3">
        <label className="text-sm font-medium">Normalization Mode</label>

        <div className="space-y-2">
          {modes.map((option) => {
            const isSelected = option.value === mode;

            return (
              <button
                key={option.value}
                onClick={() => onModeChange(option.value)}
                className={`
                  w-full text-left p-4 rounded-lg border-2 transition-all
                  ${
                    isSelected
                      ? 'border-primary bg-primary/5 shadow-sm'
                      : 'border-border hover:border-primary/50 hover:bg-foreground/[var(--hover-bg-opacity)]'
                  }
                `}
              >
                <div className="flex items-start justify-between gap-4">
                  <div className="flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="font-semibold">{option.label}</span>
                      {option.targetLevel && (
                        <span className="text-xs px-2 py-0.5 bg-muted rounded text-muted-foreground">
                          {option.targetLevel}
                        </span>
                      )}
                    </div>
                    <p className="text-sm text-muted-foreground">{option.description}</p>
                  </div>

                  {isSelected && (
                    <div className="flex-shrink-0">
                      <input
                        type="radio"
                        checked={true}
                        onChange={() => {}}
                        className="w-4 h-4 text-primary"
                      />
                    </div>
                  )}
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Pre-amp (only when leveling is enabled) */}
      {mode !== 'disabled' && (
        <div className="space-y-3">
          <label className="text-sm font-medium flex items-center gap-2">
            Pre-amp Adjustment
            <Info className="w-3 h-3 text-muted-foreground" />
          </label>

          <div className="space-y-2">
            <div className="flex items-center gap-3">
              <input
                type="range"
                min="-12"
                max="12"
                step="0.5"
                value={localPreampDb}
                onChange={(e) => handlePreampChange(parseFloat(e.target.value))}
                className="w-full"
              />
              <span className="text-sm font-mono w-16 text-right">
                {localPreampDb >= 0 ? '+' : ''}{localPreampDb.toFixed(1)} dB
              </span>
            </div>
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>-12 dB</span>
              <span className="font-medium text-foreground">0 dB</span>
              <span>+12 dB</span>
            </div>
          </div>

          <p className="text-xs text-muted-foreground">
            Adjust overall volume after normalization. Use negative values if tracks are clipping.
          </p>
        </div>
      )}

      {/* Prevent Clipping */}
      {mode !== 'disabled' && (
        <div className="space-y-3">
          <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors">
            <input
              type="checkbox"
              checked={preventClipping}
              onChange={(e) => handlePreventClippingChange(e.target.checked)}
              className="w-4 h-4 mt-0.5"
            />
            <div className="flex-1">
              <div className="text-sm font-medium">Prevent Clipping</div>
              <p className="text-xs text-muted-foreground mt-1">
                Automatically reduce gain if normalized audio would clip. Preserves dynamic range.
              </p>
            </div>
          </label>
        </div>
      )}

      {/* Info Boxes */}
      <div className="space-y-3">
        {/* ReplayGain vs EBU R128 explanation */}
        {mode !== 'disabled' && (
          <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 flex gap-3">
            <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
            <div className="text-sm">
              <p className="font-medium mb-1">
                {mode.startsWith('replaygain') ? 'ReplayGain' : 'EBU R128'}
              </p>
              <p className="text-muted-foreground text-xs">
                {mode.startsWith('replaygain') ? (
                  <>
                    ReplayGain analyzes audio to determine perceived loudness and adjusts playback volume accordingly.
                    <strong> Track mode</strong> normalizes each song independently.
                    <strong> Album mode</strong> maintains relative volume differences within albums.
                  </>
                ) : (
                  <>
                    EBU R128 is a professional loudness standard used in broadcasting. It provides more accurate
                    perceptual loudness measurement than traditional ReplayGain. Target level is -23 LUFS.
                  </>
                )}
              </p>
            </div>
          </div>
        )}

        {/* Tag requirement */}
        {mode !== 'disabled' && (
          <div className="bg-amber-500/10 border border-amber-500/20 rounded-lg p-4 flex gap-3">
            <Info className="w-5 h-5 text-amber-500 flex-shrink-0 mt-0.5" />
            <div className="text-sm text-foreground">
              <p className="font-medium mb-1">Tag Requirement</p>
              <p className="text-muted-foreground text-xs">
                {mode.startsWith('replaygain') ? (
                  <>
                    ReplayGain requires audio files to have ReplayGain tags. Files without tags will play at original volume.
                    Use a tagging tool like foobar2000, Mp3tag, or Picard to analyze and tag your files.
                  </>
                ) : (
                  <>
                    EBU R128 analyzes tracks in real-time during first playback. Analysis results are cached for future playback.
                    First playback may have slight delay while analyzing.
                  </>
                )}
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Library Analysis Section */}
      {mode !== 'disabled' && (
        <div className="space-y-4 pt-4 border-t">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-medium">Library Analysis</h3>
            <button
              onClick={loadQueueStats}
              className="p-1 hover:bg-foreground/[var(--hover-bg-opacity)] rounded transition-colors"
              title="Refresh stats"
            >
              <RefreshCw className="w-4 h-4 text-muted-foreground" />
            </button>
          </div>

          {/* Queue Stats */}
          {queueStats && (
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
              <div className="bg-muted/30 rounded-lg p-3 text-center">
                <div className="text-2xl font-bold">{queueStats.pending}</div>
                <div className="text-xs text-muted-foreground">Pending</div>
              </div>
              <div className="bg-muted/30 rounded-lg p-3 text-center">
                <div className="text-2xl font-bold text-blue-500">{queueStats.processing}</div>
                <div className="text-xs text-muted-foreground">Processing</div>
              </div>
              <div className="bg-muted/30 rounded-lg p-3 text-center">
                <div className="text-2xl font-bold text-green-500">{queueStats.completed}</div>
                <div className="text-xs text-muted-foreground">Completed</div>
              </div>
              <div className="bg-muted/30 rounded-lg p-3 text-center">
                <div className="text-2xl font-bold text-red-500">{queueStats.failed}</div>
                <div className="text-xs text-muted-foreground">Failed</div>
              </div>
            </div>
          )}

          {/* Worker Status */}
          {workerStatus.isRunning && (
            <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-3">
              <div className="flex items-center gap-2">
                <Loader2 className="w-4 h-4 animate-spin text-blue-500" />
                <span className="text-sm font-medium">Analyzing library...</span>
              </div>
              {lastAnalyzedTrack && (
                <p className="text-xs text-muted-foreground mt-1 truncate">
                  Current: {lastAnalyzedTrack}
                </p>
              )}
              <p className="text-xs text-muted-foreground mt-1">
                {workerStatus.tracksAnalyzed} tracks analyzed this session
              </p>
            </div>
          )}

          {/* Action Buttons */}
          <div className="flex flex-wrap gap-2">
            {workerStatus.isRunning ? (
              <button
                onClick={handleStopAnalysis}
                disabled={isLoading}
                className="flex items-center gap-2 px-4 py-2 bg-red-500 text-white rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity duration-[var(--transition-duration)] disabled:opacity-[var(--disabled-opacity)]"
              >
                <Square className="w-4 h-4" />
                Stop Analysis
              </button>
            ) : (
              <>
                <button
                  onClick={handleQueueAllUnanalyzed}
                  disabled={isLoading}
                  className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:opacity-[var(--hover-button-opacity)] transition-opacity duration-[var(--transition-duration)] disabled:opacity-[var(--disabled-opacity)]"
                >
                  {isLoading ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Play className="w-4 h-4" />
                  )}
                  Analyze All Tracks
                </button>
                {queueStats && queueStats.pending > 0 && (
                  <button
                    onClick={handleStartAnalysis}
                    disabled={isLoading}
                    className="flex items-center gap-2 px-4 py-2 border border-border rounded-lg hover:bg-foreground/[var(--hover-bg-opacity)] transition-colors"
                  >
                    Resume ({queueStats.pending} pending)
                  </button>
                )}
              </>
            )}
            {queueStats && queueStats.completed > 0 && (
              <button
                onClick={handleClearCompleted}
                className="flex items-center gap-2 px-4 py-2 text-sm text-muted-foreground hover:opacity-[var(--hover-text-opacity)] transition-opacity duration-[var(--transition-duration)]"
              >
                <CheckCircle className="w-4 h-4" />
                Clear Completed
              </button>
            )}
          </div>

          <p className="text-xs text-muted-foreground">
            Analysis scans your library to calculate loudness values for volume normalization.
            This runs in the background and doesn't affect playback.
          </p>
        </div>
      )}
    </div>
  );
}
