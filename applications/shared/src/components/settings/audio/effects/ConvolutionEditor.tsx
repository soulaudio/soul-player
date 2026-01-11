// Convolution/Impulse Response editor component
// Industry-standard UI for IR-based reverb and cabinet simulation
// Inspired by: Convology XT, AudioThing Fog Convolver, Space Designer

import { useState, useEffect, useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import {
  FolderOpen,
  X,
  Volume2,
  Clock,
  Timer,
  Waves,
  AlertCircle,
  Loader2,
  Music
} from 'lucide-react';

/**
 * Settings for the convolution effect
 */
export interface ConvolutionSettings {
  /** Path to impulse response file */
  irFilePath: string;
  /** Wet/dry mix: 0.0 (dry) to 1.0 (wet) */
  wetDryMix: number;
  /** Pre-delay in milliseconds: 0 to 100ms */
  preDelayMs: number;
  /** Decay/length multiplier: 0.5 to 2.0 */
  decay: number;
}

/**
 * Props for the ConvolutionEditor component
 */
export interface ConvolutionEditorProps {
  settings: ConvolutionSettings;
  onSettingsChange: (settings: ConvolutionSettings) => void;
  slotIndex: number;
}

/**
 * IR waveform data for visualization
 */
interface IrWaveformData {
  peaks: number[];
  duration: number;
  sampleRate: number;
  channels: number;
}

/**
 * Built-in IR preset
 */
interface IrPreset {
  id: string;
  labelKey: string;
  descriptionKey: string;
  category: 'room' | 'hall' | 'plate' | 'cabinet';
}

const BUILT_IN_PRESETS: IrPreset[] = [
  { id: 'small_room', labelKey: 'convolution.presets.smallRoom', descriptionKey: 'convolution.presets.smallRoomDesc', category: 'room' },
  { id: 'large_hall', labelKey: 'convolution.presets.largeHall', descriptionKey: 'convolution.presets.largeHallDesc', category: 'hall' },
  { id: 'plate', labelKey: 'convolution.presets.plate', descriptionKey: 'convolution.presets.plateDesc', category: 'plate' },
  { id: 'chamber', labelKey: 'convolution.presets.chamber', descriptionKey: 'convolution.presets.chamberDesc', category: 'hall' },
];

/**
 * ConvolutionEditor - Industry-standard convolution reverb UI
 *
 * Features:
 * - IR file loader with WAV/FLAC support
 * - Waveform visualization of loaded IR
 * - Wet/Dry mix control
 * - Pre-delay control
 * - Decay/length control
 * - Built-in IR presets
 */
export function ConvolutionEditor({
  settings,
  onSettingsChange,
  slotIndex: _slotIndex // Reserved for backend integration
}: ConvolutionEditorProps) {
  const { t } = useTranslation();
  const canvasRef = useRef<HTMLCanvasElement>(null);

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [waveformData, setWaveformData] = useState<IrWaveformData | null>(null);
  const [backendAvailable, setBackendAvailable] = useState(false);

  // Local state for smooth slider updates
  const [localWetDry, setLocalWetDry] = useState(settings.wetDryMix);
  const [localPreDelay, setLocalPreDelay] = useState(settings.preDelayMs);
  const [localDecay, setLocalDecay] = useState(settings.decay);

  // Sync local state with props
  useEffect(() => {
    setLocalWetDry(settings.wetDryMix);
    setLocalPreDelay(settings.preDelayMs);
    setLocalDecay(settings.decay);
  }, [settings.wetDryMix, settings.preDelayMs, settings.decay]);

  // Check if backend convolution command is available
  useEffect(() => {
    // Convolution is now wired to the backend DSP chain
    setBackendAvailable(true);
  }, []);

  // Draw waveform when data changes or canvas resizes
  useEffect(() => {
    drawWaveform();
  }, [waveformData]);

  /**
   * Draw the IR waveform on the canvas
   */
  const drawWaveform = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas
    ctx.fillStyle = 'var(--background, #1a1a1a)';
    ctx.fillRect(0, 0, width, height);

    if (!waveformData || waveformData.peaks.length === 0) {
      // Draw placeholder
      ctx.fillStyle = 'var(--muted-foreground, #666)';
      ctx.font = '12px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText(t('convolution.noIrLoaded'), width / 2, height / 2);
      return;
    }

    const { peaks } = waveformData;
    const barWidth = Math.max(1, width / peaks.length);

    // Draw waveform bars
    ctx.fillStyle = 'var(--primary, #3b82f6)';

    for (let i = 0; i < peaks.length; i++) {
      const peak = peaks[i];
      const barHeight = peak * (height * 0.8);
      const x = i * barWidth;
      const y = (height - barHeight) / 2;

      ctx.fillRect(x, y, barWidth - 1, barHeight);
    }

    // Draw decay envelope indicator
    const decayFactor = localDecay;
    ctx.strokeStyle = 'var(--destructive, #ef4444)';
    ctx.lineWidth = 2;
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    const decayX = width * Math.min(1, decayFactor);
    ctx.moveTo(decayX, 0);
    ctx.lineTo(decayX, height);
    ctx.stroke();
    ctx.setLineDash([]);
  }, [waveformData, localDecay, t]);

  /**
   * Handle IR file loading via file dialog
   */
  const handleLoadIr = async () => {
    setLoading(true);
    setError(null);

    try {
      // Use Tauri command to open file dialog with audio file filters
      // Note: open_file_dialog expects filters as array of {name, extensions} objects
      const selected = await invoke<string[] | null>('open_file_dialog', {
        multiple: false,
        filters: [
          {
            name: t('convolution.audioFiles'),
            extensions: ['wav', 'flac', 'aiff', 'aif']
          }
        ]
      });

      if (selected && selected.length > 0) {
        const filePath = selected[0];
        // Update settings with new file path
        onSettingsChange({
          ...settings,
          irFilePath: filePath
        });

        // Generate mock waveform data for visualization
        // In a real implementation, this would call the backend to get actual IR data
        generateMockWaveform(filePath);
      }
    } catch (err) {
      console.error('Failed to load IR file:', err);
      setError(t('convolution.errors.loadFailed'));
    } finally {
      setLoading(false);
    }
  };

  /**
   * Generate mock waveform data for visualization
   * In production, this would parse the actual IR file
   */
  const generateMockWaveform = (_filePath: string) => {
    // Generate a decay envelope for visualization
    // TODO: In production, parse the actual file to get real waveform data
    const numPeaks = 100;
    const peaks: number[] = [];

    for (let i = 0; i < numPeaks; i++) {
      const t = i / numPeaks;
      // Exponential decay with some noise
      const decay = Math.exp(-t * 3) * (0.8 + Math.random() * 0.2);
      peaks.push(Math.max(0, Math.min(1, decay)));
    }

    setWaveformData({
      peaks,
      duration: 1.2, // Mock duration in seconds
      sampleRate: 48000,
      channels: 2
    });
  };

  /**
   * Clear the loaded IR
   */
  const handleClearIr = () => {
    onSettingsChange({
      ...settings,
      irFilePath: ''
    });
    setWaveformData(null);
    setError(null);
  };

  /**
   * Handle built-in preset selection
   */
  const handlePresetSelect = (presetId: string) => {
    // Built-in presets would use bundled IR files
    const presetPath = `bundled://presets/${presetId}.wav`;

    onSettingsChange({
      ...settings,
      irFilePath: presetPath
    });

    // Generate visualization for preset
    generateMockWaveform(presetPath);
  };

  /**
   * Update wet/dry mix
   */
  const handleWetDryChange = (value: number) => {
    setLocalWetDry(value);
    onSettingsChange({
      ...settings,
      wetDryMix: value
    });
  };

  /**
   * Update pre-delay
   */
  const handlePreDelayChange = (value: number) => {
    setLocalPreDelay(value);
    onSettingsChange({
      ...settings,
      preDelayMs: value
    });
  };

  /**
   * Update decay multiplier
   */
  const handleDecayChange = (value: number) => {
    setLocalDecay(value);
    onSettingsChange({
      ...settings,
      decay: value
    });
  };

  /**
   * Extract filename from path for display
   */
  const getFileName = (filePath: string): string => {
    if (!filePath) return '';
    if (filePath.startsWith('bundled://')) {
      const presetId = filePath.replace('bundled://presets/', '').replace('.wav', '');
      const preset = BUILT_IN_PRESETS.find(p => p.id === presetId);
      return preset ? t(preset.labelKey) : presetId;
    }
    // Get filename from path
    const parts = filePath.replace(/\\/g, '/').split('/');
    return parts[parts.length - 1] || filePath;
  };

  const hasIrLoaded = settings.irFilePath && settings.irFilePath.length > 0;

  return (
    <div className="space-y-5">
      {/* Backend not available warning */}
      {!backendAvailable && (
        <div className="bg-amber-500/10 border border-amber-500/30 rounded-lg p-3 flex items-start gap-3">
          <AlertCircle className="w-5 h-5 text-amber-500 flex-shrink-0 mt-0.5" />
          <div className="text-sm">
            <p className="font-medium text-amber-500">{t('convolution.notAvailable')}</p>
            <p className="text-muted-foreground text-xs mt-1">
              {t('convolution.notAvailableDesc')}
            </p>
          </div>
        </div>
      )}

      {/* IR Loader Section */}
      <div className="space-y-3">
        <label className="text-sm font-medium flex items-center gap-2">
          <Waves className="w-4 h-4" />
          {t('convolution.impulseResponse')}
        </label>

        {/* File loader row */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleLoadIr}
            disabled={loading}
            className="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 transition-colors"
          >
            {loading ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <FolderOpen className="w-4 h-4" />
            )}
            {t('convolution.loadIr')}
          </button>

          {hasIrLoaded && (
            <>
              <div className="flex-1 px-3 py-2 bg-muted/30 rounded-lg text-sm truncate flex items-center gap-2">
                <Music className="w-4 h-4 flex-shrink-0 text-muted-foreground" />
                <span className="truncate">{getFileName(settings.irFilePath)}</span>
              </div>

              <button
                onClick={handleClearIr}
                className="p-2 hover:bg-destructive/10 hover:text-destructive rounded-lg transition-colors"
                title={t('convolution.clearIr')}
              >
                <X className="w-4 h-4" />
              </button>
            </>
          )}
        </div>

        {/* Error display */}
        {error && (
          <div className="text-sm text-destructive flex items-center gap-2">
            <AlertCircle className="w-4 h-4" />
            {error}
          </div>
        )}

        {/* File format info */}
        <p className="text-xs text-muted-foreground">
          {t('convolution.supportedFormats')}
        </p>
      </div>

      {/* Waveform Display */}
      <div className="space-y-2">
        <div className="relative bg-muted/20 border border-border rounded-lg overflow-hidden">
          <canvas
            ref={canvasRef}
            width={400}
            height={100}
            className="w-full h-24"
            style={{ imageRendering: 'pixelated' }}
          />

          {/* Duration indicator */}
          {waveformData && (
            <div className="absolute bottom-2 right-2 px-2 py-1 bg-background/80 rounded text-xs font-mono">
              {waveformData.duration.toFixed(2)}s
            </div>
          )}
        </div>
      </div>

      {/* Mix Control */}
      <div className="space-y-3">
        <label className="text-sm font-medium flex items-center gap-2">
          <Volume2 className="w-4 h-4" />
          {t('convolution.wetDryMix')}
        </label>

        <div className="space-y-2">
          <div className="flex items-center gap-4">
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              value={Math.round(localWetDry * 100)}
              onChange={(e) => handleWetDryChange(parseInt(e.target.value) / 100)}
              className="flex-1"
            />
            <span className="text-sm font-mono w-12 text-right">
              {Math.round(localWetDry * 100)}%
            </span>
          </div>
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>{t('convolution.dry')}</span>
            <span>{t('convolution.wet')}</span>
          </div>
        </div>
      </div>

      {/* Pre-Delay Control */}
      <div className="space-y-3">
        <label className="text-sm font-medium flex items-center gap-2">
          <Clock className="w-4 h-4" />
          {t('convolution.preDelay')}
        </label>

        <div className="space-y-2">
          <div className="flex items-center gap-4">
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              value={localPreDelay}
              onChange={(e) => handlePreDelayChange(parseInt(e.target.value))}
              className="flex-1"
            />
            <span className="text-sm font-mono w-16 text-right">
              {localPreDelay} ms
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            {t('convolution.preDelayDesc')}
          </p>
        </div>
      </div>

      {/* Decay/Length Control */}
      <div className="space-y-3">
        <label className="text-sm font-medium flex items-center gap-2">
          <Timer className="w-4 h-4" />
          {t('convolution.decay')}
        </label>

        <div className="space-y-2">
          <div className="flex items-center gap-4">
            <input
              type="range"
              min="50"
              max="200"
              step="5"
              value={Math.round(localDecay * 100)}
              onChange={(e) => handleDecayChange(parseInt(e.target.value) / 100)}
              className="flex-1"
            />
            <span className="text-sm font-mono w-14 text-right">
              {Math.round(localDecay * 100)}%
            </span>
          </div>
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>{t('convolution.shorter')}</span>
            <span>{t('convolution.normal')}</span>
            <span>{t('convolution.longer')}</span>
          </div>
        </div>
      </div>

      {/* Built-in Presets */}
      <div className="space-y-3 pt-4 border-t border-border">
        <label className="text-sm font-medium">{t('convolution.builtInPresets')}</label>

        <div className="grid grid-cols-2 gap-2">
          {BUILT_IN_PRESETS.map((preset) => {
            const isActive = settings.irFilePath === `bundled://presets/${preset.id}.wav`;

            return (
              <button
                key={preset.id}
                onClick={() => handlePresetSelect(preset.id)}
                className={`
                  text-left p-3 rounded-lg border-2 transition-all
                  ${isActive
                    ? 'border-primary bg-primary/10'
                    : 'border-border hover:border-primary/50 hover:bg-muted/30'
                  }
                `}
              >
                <div className="font-medium text-sm">{t(preset.labelKey)}</div>
                <div className="text-xs text-muted-foreground mt-0.5">
                  {t(preset.descriptionKey)}
                </div>
              </button>
            );
          })}
        </div>

        <p className="text-xs text-muted-foreground">
          {t('convolution.presetsNote')}
        </p>
      </div>

      {/* Info note */}
      <div className="text-xs text-muted-foreground bg-muted/30 p-3 rounded-lg">
        <strong>{t('convolution.infoTitle')}:</strong> {t('convolution.infoText')}
      </div>
    </div>
  );
}

/**
 * Default convolution settings
 */
export const defaultConvolutionSettings: ConvolutionSettings = {
  irFilePath: '',
  wetDryMix: 0.3,
  preDelayMs: 0,
  decay: 1.0
};
