// Latency monitoring display component
// Shows real-time audio latency information and exclusive mode status

import { useEffect, useState, useCallback } from 'react';
import { Activity, Zap, Lock, Unlock, RefreshCw, Settings2 } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';

export interface LatencyInfo {
  bufferSamples: number;
  bufferMs: number;
  totalMs: number;
  exclusive: boolean;
}

export interface ExclusiveConfig {
  sampleRate: number;
  bitDepth: string;
  bufferFrames: number | null;
  exclusiveMode: boolean;
  deviceName: string | null;
  backend: string;
}

export interface BufferSizeOption {
  frames: number;
  supported: boolean;
  latencyMs44100: number;
  latencyMs48000: number;
}

interface LatencyMonitorProps {
  sampleRate?: number;
  showExclusiveControls?: boolean;
  onExclusiveModeChange?: (enabled: boolean) => void;
}

export function LatencyMonitor({
  sampleRate = 44100,
  showExclusiveControls = true,
  onExclusiveModeChange,
}: LatencyMonitorProps) {
  const { t } = useTranslation();
  const [latencyInfo, setLatencyInfo] = useState<LatencyInfo | null>(null);
  const [isExclusive, setIsExclusive] = useState(false);
  const [bufferSizes, setBufferSizes] = useState<BufferSizeOption[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Fetch latency info
  const fetchLatencyInfo = useCallback(async () => {
    try {
      const info = await invoke<LatencyInfo>('get_latency_info');
      setLatencyInfo(info);
      setIsExclusive(info.exclusive);
      setError(null);
    } catch (e) {
      console.error('Failed to fetch latency info:', e);
      setError(String(e));
    }
  }, []);

  // Fetch exclusive mode status
  const fetchExclusiveStatus = useCallback(async () => {
    try {
      const exclusive = await invoke<boolean>('is_exclusive_mode');
      setIsExclusive(exclusive);
    } catch (e) {
      console.error('Failed to fetch exclusive mode status:', e);
    }
  }, []);

  // Initial fetch and periodic refresh
  useEffect(() => {
    const init = async () => {
      setIsLoading(true);
      await fetchLatencyInfo();
      await fetchExclusiveStatus();
      setIsLoading(false);
    };

    init();

    // Refresh every 5 seconds
    const interval = setInterval(() => {
      fetchLatencyInfo();
    }, 5000);

    return () => clearInterval(interval);
  }, [fetchLatencyInfo, fetchExclusiveStatus]);

  // Toggle exclusive mode
  const handleToggleExclusive = async () => {
    try {
      if (isExclusive) {
        await invoke('disable_exclusive_mode');
        setIsExclusive(false);
        onExclusiveModeChange?.(false);
      } else {
        const config: ExclusiveConfig = {
          sampleRate: 0, // Use device default
          bitDepth: 'float32',
          bufferFrames: 256,
          exclusiveMode: true,
          deviceName: null,
          backend: 'default',
        };
        const newLatency = await invoke<LatencyInfo>('set_exclusive_mode', { config });
        setLatencyInfo(newLatency);
        setIsExclusive(true);
        onExclusiveModeChange?.(true);
      }
    } catch (e) {
      console.error('Failed to toggle exclusive mode:', e);
      setError(String(e));
    }
  };

  // Get latency quality indicator
  const getLatencyQuality = (ms: number) => {
    if (ms <= 10) return { label: t('settings.audio.latency.excellent'), color: 'text-green-500' };
    if (ms <= 20) return { label: t('settings.audio.latency.good'), color: 'text-blue-500' };
    if (ms <= 50) return { label: t('settings.audio.latency.acceptable'), color: 'text-yellow-500' };
    return { label: t('settings.audio.latency.high'), color: 'text-red-500' };
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-4 text-muted-foreground">
        <RefreshCw className="w-4 h-4 animate-spin mr-2" />
        {t('settings.audio.latency.loading')}
      </div>
    );
  }

  const quality = latencyInfo ? getLatencyQuality(latencyInfo.totalMs) : null;

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Activity className="w-4 h-4 text-primary" />
          <span className="text-sm font-medium">{t('settings.audio.latency.title')}</span>
        </div>
        <button
          onClick={fetchLatencyInfo}
          className="p-1.5 rounded hover:bg-muted/50 transition-colors"
          title={t('settings.audio.latency.refresh')}
        >
          <RefreshCw className="w-3.5 h-3.5 text-muted-foreground" />
        </button>
      </div>

      {error && (
        <div className="text-xs text-red-500 bg-red-500/10 p-2 rounded">
          {error}
        </div>
      )}

      {latencyInfo && (
        <div className="grid grid-cols-2 gap-3">
          {/* Buffer Latency */}
          <div className="bg-muted/30 rounded-lg p-3">
            <div className="text-xs text-muted-foreground mb-1">
              {t('settings.audio.latency.buffer')}
            </div>
            <div className="flex items-baseline gap-1">
              <span className="text-lg font-semibold">{latencyInfo.bufferMs.toFixed(1)}</span>
              <span className="text-xs text-muted-foreground">ms</span>
            </div>
            <div className="text-xs text-muted-foreground mt-1">
              {latencyInfo.bufferSamples} {t('settings.audio.latency.samples')}
            </div>
          </div>

          {/* Total Latency */}
          <div className="bg-muted/30 rounded-lg p-3">
            <div className="text-xs text-muted-foreground mb-1">
              {t('settings.audio.latency.total')}
            </div>
            <div className="flex items-baseline gap-1">
              <span className="text-lg font-semibold">{latencyInfo.totalMs.toFixed(1)}</span>
              <span className="text-xs text-muted-foreground">ms</span>
            </div>
            {quality && (
              <div className={`text-xs mt-1 ${quality.color}`}>
                {quality.label}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Exclusive Mode Toggle */}
      {showExclusiveControls && (
        <div className="border-t pt-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              {isExclusive ? (
                <Lock className="w-4 h-4 text-primary" />
              ) : (
                <Unlock className="w-4 h-4 text-muted-foreground" />
              )}
              <div>
                <div className="text-sm font-medium">
                  {t('settings.audio.exclusive.title')}
                </div>
                <div className="text-xs text-muted-foreground">
                  {isExclusive
                    ? t('settings.audio.exclusive.enabledDesc')
                    : t('settings.audio.exclusive.disabledDesc')}
                </div>
              </div>
            </div>
            <button
              onClick={handleToggleExclusive}
              className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
                isExclusive
                  ? 'bg-primary text-primary-foreground hover:bg-primary/90'
                  : 'bg-muted hover:bg-muted/80'
              }`}
            >
              {isExclusive ? t('settings.audio.exclusive.disable') : t('settings.audio.exclusive.enable')}
            </button>
          </div>

          {/* Exclusive Mode Info */}
          <div className="mt-3 text-xs space-y-1 text-muted-foreground">
            <div className="flex items-start gap-2">
              <Zap className="w-3 h-3 mt-0.5 text-yellow-500" />
              <span>{t('settings.audio.exclusive.benefit1')}</span>
            </div>
            <div className="flex items-start gap-2">
              <Settings2 className="w-3 h-3 mt-0.5 text-blue-500" />
              <span>{t('settings.audio.exclusive.benefit2')}</span>
            </div>
          </div>
        </div>
      )}

      {/* Latency visualization */}
      {latencyInfo && (
        <div className="pt-3 border-t">
          <div className="text-xs text-muted-foreground mb-2">
            {t('settings.audio.latency.visualization')}
          </div>
          <LatencyBar
            bufferMs={latencyInfo.bufferMs}
            dacMs={latencyInfo.totalMs - latencyInfo.bufferMs}
          />
        </div>
      )}
    </div>
  );
}

// Latency bar visualization
function LatencyBar({ bufferMs, dacMs }: { bufferMs: number; dacMs: number }) {
  const totalMs = bufferMs + dacMs;
  const maxMs = Math.max(totalMs, 50); // Minimum scale of 50ms
  const bufferPercent = (bufferMs / maxMs) * 100;
  const dacPercent = (dacMs / maxMs) * 100;

  return (
    <div className="space-y-2">
      <div className="h-4 rounded-full overflow-hidden bg-muted flex">
        <div
          className="h-full bg-primary/80 transition-all duration-300"
          style={{ width: `${bufferPercent}%` }}
          title={`Buffer: ${bufferMs.toFixed(1)}ms`}
        />
        <div
          className="h-full bg-yellow-500/60 transition-all duration-300"
          style={{ width: `${dacPercent}%` }}
          title={`DAC: ${dacMs.toFixed(1)}ms`}
        />
      </div>
      <div className="flex justify-between text-xs text-muted-foreground">
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-primary/80" />
          <span>Buffer ({bufferMs.toFixed(1)}ms)</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 rounded-full bg-yellow-500/60" />
          <span>DAC ({dacMs.toFixed(1)}ms)</span>
        </div>
      </div>
    </div>
  );
}

export default LatencyMonitor;
