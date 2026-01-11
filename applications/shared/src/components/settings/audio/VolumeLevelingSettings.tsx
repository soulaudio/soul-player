// Volume leveling (ReplayGain / EBU R128) settings component

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Info, Play, Square, RefreshCw, CheckCircle, AlertCircle, Loader2 } from 'lucide-react';

interface QueueStats {
  total: number;
  pending: number;
  processing: number;
  completed: number;
  failed: number;
}

interface WorkerStatus {
  isRunning: boolean;
  tracksAnalyzed: number;
}

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
  const [queueStats, setQueueStats] = useState<QueueStats | null>(null);
  const [workerStatus, setWorkerStatus] = useState<WorkerStatus>({ isRunning: false, tracksAnalyzed: 0 });
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
    } else {
      // Fallback: directly invoke Tauri command if no callback provided
      try {
        await invoke('set_volume_leveling_preamp', { preampDb: value });
      } catch (error) {
        console.error('Failed to set preamp:', error);
      }
    }
  };

  // Prevent clipping change handler
  const handlePreventClippingChange = async (checked: boolean) => {
    if (onPreventClippingChange) {
      onPreventClippingChange(checked);
    } else {
      // Fallback: directly invoke Tauri command if no callback provided
      try {
        await invoke('set_volume_leveling_prevent_clipping', { prevent: checked });
      } catch (error) {
        console.error('Failed to set prevent clipping:', error);
      }
    }
  };

  // Load initial stats
  useEffect(() => {
    loadQueueStats();
    loadWorkerStatus();
  }, []);

  // Listen for analysis events
  useEffect(() => {
    const unlistenProgress = listen<{ trackId: number; trackTitle: string }>('loudness-analysis-progress', (event) => {
      setLastAnalyzedTrack(event.payload.trackTitle);
      loadQueueStats();
      loadWorkerStatus();
    });

    const unlistenComplete = listen('analysis-worker-complete', () => {
      setWorkerStatus({ isRunning: false, tracksAnalyzed: workerStatus.tracksAnalyzed });
      loadQueueStats();
    });

    const unlistenStopped = listen('analysis-worker-stopped', () => {
      setWorkerStatus({ isRunning: false, tracksAnalyzed: workerStatus.tracksAnalyzed });
    });

    return () => {
      unlistenProgress.then(f => f());
      unlistenComplete.then(f => f());
      unlistenStopped.then(f => f());
    };
  }, [workerStatus.tracksAnalyzed]);

  const loadQueueStats = async () => {
    try {
      const stats = await invoke<QueueStats>('get_analysis_queue_stats');
      setQueueStats(stats);
    } catch (error) {
      console.error('Failed to load queue stats:', error);
    }
  };

  const loadWorkerStatus = async () => {
    try {
      const status = await invoke<WorkerStatus>('get_analysis_worker_status');
      setWorkerStatus(status);
    } catch (error) {
      console.error('Failed to load worker status:', error);
    }
  };

  const handleStartAnalysis = async () => {
    setIsLoading(true);
    try {
      await invoke('start_analysis_worker');
      setWorkerStatus({ ...workerStatus, isRunning: true });
    } catch (error) {
      console.error('Failed to start analysis:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleStopAnalysis = async () => {
    setIsLoading(true);
    try {
      await invoke('stop_analysis_worker');
    } catch (error) {
      console.error('Failed to stop analysis:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleQueueAllUnanalyzed = async () => {
    setIsLoading(true);
    try {
      const count = await invoke<number>('queue_all_unanalyzed');
      await loadQueueStats();
      if (count > 0) {
        // Auto-start worker if items were queued
        await handleStartAnalysis();
      }
    } catch (error) {
      console.error('Failed to queue tracks:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClearCompleted = async () => {
    try {
      await invoke('clear_completed_analysis');
      await loadQueueStats();
    } catch (error) {
      console.error('Failed to clear completed:', error);
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
                      : 'border-border hover:border-primary/50 hover:bg-muted/30'
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
            <Info className="w-3 h-3 text-muted-foreground" title="Additional gain applied after normalization" />
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
          <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-muted/30 transition-colors">
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
              className="p-1 hover:bg-muted rounded transition-colors"
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
                className="flex items-center gap-2 px-4 py-2 bg-red-500 text-white rounded-lg hover:bg-red-600 disabled:opacity-50 transition-colors"
              >
                <Square className="w-4 h-4" />
                Stop Analysis
              </button>
            ) : (
              <>
                <button
                  onClick={handleQueueAllUnanalyzed}
                  disabled={isLoading}
                  className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 transition-colors"
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
                    className="flex items-center gap-2 px-4 py-2 border border-border rounded-lg hover:bg-muted transition-colors"
                  >
                    Resume ({queueStats.pending} pending)
                  </button>
                )}
              </>
            )}
            {queueStats && queueStats.completed > 0 && (
              <button
                onClick={handleClearCompleted}
                className="flex items-center gap-2 px-4 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
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
