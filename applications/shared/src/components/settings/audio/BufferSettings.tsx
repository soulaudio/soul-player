// Audio buffer, pre-loading, and crossfade settings component

import { Info, Shuffle } from 'lucide-react';

export type CrossfadeCurve = 'linear' | 'logarithmic' | 's_curve' | 'equal_power';

export interface CrossfadeSettings {
  enabled: boolean;
  durationMs: number;
  curve: CrossfadeCurve;
}

interface BufferSettingsProps {
  bufferSize: 'auto' | number;
  preloadEnabled: boolean;
  crossfade?: CrossfadeSettings;
  onBufferSizeChange: (size: 'auto' | number) => void;
  onPreloadChange: (enabled: boolean) => void;
  onCrossfadeChange?: (settings: CrossfadeSettings) => void;
}

const bufferSizes = [
  { value: 'auto', label: 'Auto', description: 'Let system decide (~50ms latency)', latency: '~50ms' },
  { value: 128, label: '128 samples', description: 'Ultra-low latency (may crackle)', latency: '~3ms @ 44.1kHz' },
  { value: 256, label: '256 samples', description: 'Low latency', latency: '~6ms @ 44.1kHz' },
  { value: 512, label: '512 samples', description: 'Balanced', latency: '~12ms @ 44.1kHz' },
  { value: 1024, label: '1024 samples', description: 'Safe (recommended)', latency: '~23ms @ 44.1kHz' },
  { value: 2048, label: '2048 samples', description: 'Maximum stability', latency: '~46ms @ 44.1kHz' },
];

const crossfadeDurations = [
  { value: 0, label: 'Gapless (No crossfade)', description: 'Seamless track transitions' },
  { value: 1000, label: '1 second', description: 'Quick fade' },
  { value: 2000, label: '2 seconds', description: 'Subtle blend' },
  { value: 3000, label: '3 seconds', description: 'Smooth transition' },
  { value: 5000, label: '5 seconds', description: 'Long blend' },
  { value: 8000, label: '8 seconds', description: 'DJ-style mix' },
  { value: 12000, label: '12 seconds', description: 'Full overlap' },
];

const crossfadeCurves: { value: CrossfadeCurve; label: string; description: string }[] = [
  { value: 'equal_power', label: 'Equal Power (Recommended)', description: 'Constant perceived loudness during crossfade' },
  { value: 's_curve', label: 'S-Curve', description: 'Smooth acceleration at start and end' },
  { value: 'linear', label: 'Linear', description: 'Simple straight-line fade' },
  { value: 'logarithmic', label: 'Logarithmic', description: 'Matches perceived loudness curve' },
];

export function BufferSettings({
  bufferSize,
  preloadEnabled,
  crossfade = { enabled: false, durationMs: 3000, curve: 'equal_power' },
  onBufferSizeChange,
  onPreloadChange,
  onCrossfadeChange,
}: BufferSettingsProps) {
  const handleCrossfadeEnabledChange = (enabled: boolean) => {
    onCrossfadeChange?.({ ...crossfade, enabled });
  };

  const handleCrossfadeDurationChange = (durationMs: number) => {
    onCrossfadeChange?.({ ...crossfade, durationMs, enabled: durationMs > 0 });
  };

  const handleCrossfadeCurveChange = (curve: CrossfadeCurve) => {
    onCrossfadeChange?.({ ...crossfade, curve });
  };

  return (
    <div className="space-y-6">
      {/* Buffer Size */}
      <div className="space-y-3">
        <label className="text-sm font-medium flex items-center gap-2">
          Buffer Size
          <span title="Audio buffer size affects latency vs stability trade-off">
            <Info className="w-3 h-3 text-muted-foreground" />
          </span>
        </label>

        <select
          value={bufferSize}
          onChange={(e) => onBufferSizeChange(e.target.value === 'auto' ? 'auto' : parseInt(e.target.value))}
          className="w-full px-3 py-2 border border-border rounded-lg bg-background focus:ring-2 focus:ring-primary/50 transition-all"
        >
          {bufferSizes.map((size) => (
            <option key={size.value} value={size.value}>
              {size.label} - {size.description} ({size.latency})
            </option>
          ))}
        </select>

        <p className="text-xs text-muted-foreground">
          Smaller buffers reduce latency but may cause audio glitches on slower systems.
          Larger buffers are more stable but have higher latency.
        </p>
      </div>

      {/* Pre-loading */}
      <div className="space-y-3">
        <div>
          <label className="text-sm font-medium flex items-center gap-2 mb-2">
            Track Pre-loading
            <span title="Load entire track to memory before playback">
              <Info className="w-3 h-3 text-muted-foreground" />
            </span>
          </label>

          <label className="flex items-start gap-3 cursor-pointer p-4 rounded-lg border hover:bg-muted/30 transition-colors">
            <input
              type="checkbox"
              checked={preloadEnabled}
              onChange={(e) => onPreloadChange(e.target.checked)}
              className="w-4 h-4 mt-0.5"
            />
            <div className="flex-1">
              <div className="font-medium text-sm">Enable pre-loading for local files</div>
              <p className="text-xs text-muted-foreground mt-1">
                Pre-load entire track into memory before playback starts.
                Reduces disk activity and eliminates jitter during playback.
              </p>
            </div>
          </label>
        </div>

        {/* Pre-loading details */}
        {preloadEnabled && (
          <div className="pl-7 space-y-2">
            <div className="text-xs text-muted-foreground space-y-1">
              <p><strong>Pros:</strong></p>
              <ul className="list-disc list-inside space-y-0.5 ml-2">
                <li>Eliminates disk I/O jitter during playback</li>
                <li>More stable playback timing</li>
                <li>Protects from interference (Audirvana-style)</li>
              </ul>
            </div>

            <div className="text-xs text-muted-foreground space-y-1">
              <p><strong>Cons:</strong></p>
              <ul className="list-disc list-inside space-y-0.5 ml-2">
                <li>Higher memory usage (~30-60 MB per track)</li>
                <li>Slight delay before playback starts (~100-500ms)</li>
                <li>Not suitable for streaming sources</li>
              </ul>
            </div>

            <div className="text-xs bg-muted/50 rounded p-2 mt-2">
              <strong>Memory estimate:</strong> A 3-minute 44.1kHz stereo FLAC file uses ~31 MB when pre-loaded
            </div>
          </div>
        )}
      </div>

      {/* Crossfade Settings */}
      {onCrossfadeChange && (
        <div className="space-y-4 pt-4 border-t">
          <div>
            <label className="text-sm font-medium flex items-center gap-2 mb-3">
              <Shuffle className="w-4 h-4" />
              Track Transitions
            </label>

            {/* Crossfade Enable */}
            <label className="flex items-start gap-3 cursor-pointer p-4 rounded-lg border hover:bg-muted/30 transition-colors">
              <input
                type="checkbox"
                checked={crossfade.enabled}
                onChange={(e) => handleCrossfadeEnabledChange(e.target.checked)}
                className="w-4 h-4 mt-0.5"
              />
              <div className="flex-1">
                <div className="font-medium text-sm">Enable Crossfade</div>
                <p className="text-xs text-muted-foreground mt-1">
                  Blend the end of one track with the beginning of the next for smoother transitions.
                  When disabled, gapless playback is used for seamless track changes.
                </p>
              </div>
            </label>
          </div>

          {/* Crossfade Duration */}
          {crossfade.enabled && (
            <div className="space-y-3 pl-4 border-l-2 border-primary/30">
              <div className="space-y-2">
                <label className="text-sm font-medium">Crossfade Duration</label>
                <select
                  value={crossfade.durationMs}
                  onChange={(e) => handleCrossfadeDurationChange(parseInt(e.target.value))}
                  className="w-full px-3 py-2 border border-border rounded-lg bg-background focus:ring-2 focus:ring-primary/50 transition-all"
                >
                  {crossfadeDurations.filter(d => d.value > 0).map((duration) => (
                    <option key={duration.value} value={duration.value}>
                      {duration.label} - {duration.description}
                    </option>
                  ))}
                </select>
              </div>

              {/* Crossfade Curve */}
              <div className="space-y-2">
                <label className="text-sm font-medium">Fade Curve</label>
                <select
                  value={crossfade.curve}
                  onChange={(e) => handleCrossfadeCurveChange(e.target.value as CrossfadeCurve)}
                  className="w-full px-3 py-2 border border-border rounded-lg bg-background focus:ring-2 focus:ring-primary/50 transition-all"
                >
                  {crossfadeCurves.map((curve) => (
                    <option key={curve.value} value={curve.value}>
                      {curve.label}
                    </option>
                  ))}
                </select>
                <p className="text-xs text-muted-foreground">
                  {crossfadeCurves.find(c => c.value === crossfade.curve)?.description}
                </p>
              </div>

              {/* Crossfade Visual Preview */}
              <div className="bg-muted/30 rounded-lg p-3">
                <div className="text-xs font-medium mb-2 text-muted-foreground">Preview</div>
                <div className="h-16 relative">
                  <CrossfadeVisualization curve={crossfade.curve} />
                </div>
                <div className="flex justify-between text-xs text-muted-foreground mt-1">
                  <span>Track A fades out</span>
                  <span>Track B fades in</span>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Performance Info */}
      <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 flex gap-3">
        <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
        <div className="text-sm">
          <p className="font-medium mb-1">Audiophile Playback</p>
          <p className="text-muted-foreground text-xs">
            For the highest quality playback, enable pre-loading and use a buffer size of 1024 or higher.
            This minimizes CPU activity during playback and ensures bit-perfect output.
          </p>
        </div>
      </div>

      {/* ASIO note */}
      <div className="text-xs text-muted-foreground bg-muted/30 p-3 rounded">
        <strong>Note:</strong> When using ASIO backend, buffer size may be controlled by your ASIO driver settings.
        Check your audio interface control panel for buffer configuration.
      </div>
    </div>
  );
}

// Simple crossfade curve visualization
function CrossfadeVisualization({ curve }: { curve: CrossfadeCurve }) {
  // Generate curve points
  const points = 50;
  const fadeOutPath: string[] = [];
  const fadeInPath: string[] = [];

  for (let i = 0; i <= points; i++) {
    const t = i / points;
    let fadeOutValue: number;
    let fadeInValue: number;

    switch (curve) {
      case 'linear':
        fadeOutValue = 1 - t;
        fadeInValue = t;
        break;
      case 'logarithmic':
        fadeOutValue = Math.pow(1 - t, 2);
        fadeInValue = Math.pow(t, 0.5);
        break;
      case 's_curve':
        // Smoothstep function
        const s = t * t * (3 - 2 * t);
        fadeOutValue = 1 - s;
        fadeInValue = s;
        break;
      case 'equal_power':
      default:
        // Equal power uses cosine/sine curves
        fadeOutValue = Math.cos(t * Math.PI / 2);
        fadeInValue = Math.sin(t * Math.PI / 2);
        break;
    }

    const x = t * 100;
    const yOut = (1 - fadeOutValue) * 100;
    const yIn = (1 - fadeInValue) * 100;

    fadeOutPath.push(`${x},${yOut}`);
    fadeInPath.push(`${x},${yIn}`);
  }

  return (
    <svg viewBox="0 0 100 100" className="w-full h-full" preserveAspectRatio="none">
      {/* Grid lines */}
      <line x1="0" y1="50" x2="100" y2="50" stroke="currentColor" strokeOpacity="0.1" strokeWidth="0.5" />
      <line x1="50" y1="0" x2="50" y2="100" stroke="currentColor" strokeOpacity="0.1" strokeWidth="0.5" />

      {/* Fade out curve (Track A) */}
      <polyline
        points={fadeOutPath.join(' ')}
        fill="none"
        stroke="hsl(var(--destructive))"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        opacity="0.7"
      />

      {/* Fade in curve (Track B) */}
      <polyline
        points={fadeInPath.join(' ')}
        fill="none"
        stroke="hsl(var(--primary))"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        opacity="0.7"
      />
    </svg>
  );
}
