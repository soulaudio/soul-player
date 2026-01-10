// Audio buffer and pre-loading settings component

import { Info } from 'lucide-react';

interface BufferSettingsProps {
  bufferSize: 'auto' | number;
  preloadEnabled: boolean;
  onBufferSizeChange: (size: 'auto' | number) => void;
  onPreloadChange: (enabled: boolean) => void;
}

const bufferSizes = [
  { value: 'auto', label: 'Auto', description: 'Let system decide (~50ms latency)', latency: '~50ms' },
  { value: 128, label: '128 samples', description: 'Ultra-low latency (may crackle)', latency: '~3ms @ 44.1kHz' },
  { value: 256, label: '256 samples', description: 'Low latency', latency: '~6ms @ 44.1kHz' },
  { value: 512, label: '512 samples', description: 'Balanced', latency: '~12ms @ 44.1kHz' },
  { value: 1024, label: '1024 samples', description: 'Safe (recommended)', latency: '~23ms @ 44.1kHz' },
  { value: 2048, label: '2048 samples', description: 'Maximum stability', latency: '~46ms @ 44.1kHz' },
];

export function BufferSettings({
  bufferSize,
  preloadEnabled,
  onBufferSizeChange,
  onPreloadChange,
}: BufferSettingsProps) {
  return (
    <div className="bg-card border border-border rounded-lg p-6 space-y-6">
      {/* Buffer Size */}
      <div className="space-y-3">
        <label className="text-sm font-medium flex items-center gap-2">
          Buffer Size
          <Info className="w-3 h-3 text-muted-foreground" title="Audio buffer size affects latency vs stability trade-off" />
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
            <Info className="w-3 h-3 text-muted-foreground" title="Load entire track to memory before playback" />
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
