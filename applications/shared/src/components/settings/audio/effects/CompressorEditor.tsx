// Industry-standard Compressor UI component
// Inspired by Universal Audio, FabFilter Pro-C

import { useState, useEffect, useCallback, useMemo } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { ChevronDown, RotateCcw } from 'lucide-react';

// Types matching backend (from DspConfig.tsx)
export interface CompressorSettings {
  thresholdDb: number;    // -60 to 0 dB
  ratio: number;          // 1:1 to 20:1
  attackMs: number;       // 0.1 to 100 ms
  releaseMs: number;      // 10 to 1000 ms
  kneeDb: number;         // 0 to 12 dB (soft knee width)
  makeupGainDb: number;   // 0 to 24 dB
}

export interface CompressorEditorProps {
  settings: CompressorSettings;
  onSettingsChange: (settings: CompressorSettings) => void;
  slotIndex: number;
}

// Preset type from backend
interface CompressorPreset {
  name: string;
  settings: CompressorSettings;
}

// Built-in presets that extend backend presets
const EXTENDED_PRESETS: Record<string, CompressorSettings> = {
  transparent: {
    thresholdDb: -24,
    ratio: 2,
    attackMs: 20,
    releaseMs: 150,
    kneeDb: 10,
    makeupGainDb: 2,
  },
  punchy: {
    thresholdDb: -16,
    ratio: 6,
    attackMs: 5,
    releaseMs: 40,
    kneeDb: 3,
    makeupGainDb: 5,
  },
  vocal: {
    thresholdDb: -22,
    ratio: 3.5,
    attackMs: 8,
    releaseMs: 80,
    kneeDb: 6,
    makeupGainDb: 4,
  },
  limiting: {
    thresholdDb: -8,
    ratio: 20,
    attackMs: 0.5,
    releaseMs: 25,
    kneeDb: 0,
    makeupGainDb: 6,
  },
};

// Common ratio values for stepped control
const RATIO_STEPS = [1, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10, 12, 16, 20];

export function CompressorEditor({
  settings,
  onSettingsChange,
  slotIndex: _slotIndex, // Reserved for potential direct backend calls
}: CompressorEditorProps) {
  const { t } = useTranslation();
  const [backendPresets, setBackendPresets] = useState<CompressorPreset[]>([]);
  const [isPresetOpen, setIsPresetOpen] = useState(false);

  // Load presets from backend
  useEffect(() => {
    const loadPresets = async () => {
      try {
        const presets = await invoke<[string, CompressorSettings][]>('get_compressor_presets');
        setBackendPresets(presets.map(([name, settings]) => ({ name, settings })));
      } catch (error) {
        console.error('Failed to load compressor presets:', error);
      }
    };
    loadPresets();
  }, []);

  // Combine backend and extended presets
  const allPresets = useMemo(() => {
    const presets: CompressorPreset[] = [...backendPresets];

    // Add extended presets if not already present
    Object.entries(EXTENDED_PRESETS).forEach(([name, settings]) => {
      const capitalizedName = name.charAt(0).toUpperCase() + name.slice(1);
      if (!presets.some(p => p.name.toLowerCase() === name)) {
        presets.push({ name: capitalizedName, settings });
      }
    });

    return presets;
  }, [backendPresets]);

  // Update a single parameter - parent handles backend via onSettingsChange
  const updateParam = useCallback(
    <K extends keyof CompressorSettings>(key: K, value: CompressorSettings[K]) => {
      const newSettings = { ...settings, [key]: value };
      onSettingsChange(newSettings);
    },
    [settings, onSettingsChange]
  );

  // Apply a preset
  const applyPreset = useCallback(
    (preset: CompressorPreset) => {
      onSettingsChange(preset.settings);
      setIsPresetOpen(false);
    },
    [onSettingsChange]
  );

  // Reset to default
  const resetToDefault = useCallback(() => {
    const defaultSettings: CompressorSettings = {
      thresholdDb: -20,
      ratio: 4,
      attackMs: 10,
      releaseMs: 100,
      kneeDb: 2,
      makeupGainDb: 0,
    };
    applyPreset({ name: 'Default', settings: defaultSettings });
  }, [applyPreset]);

  // Find closest ratio step
  const findClosestRatioIndex = (ratio: number): number => {
    let closestIndex = 0;
    let closestDiff = Math.abs(RATIO_STEPS[0] - ratio);

    for (let i = 1; i < RATIO_STEPS.length; i++) {
      const diff = Math.abs(RATIO_STEPS[i] - ratio);
      if (diff < closestDiff) {
        closestDiff = diff;
        closestIndex = i;
      }
    }

    return closestIndex;
  };

  return (
    <div data-testid="compressor-editor" className="space-y-6 p-4">
      {/* Header with preset dropdown and reset */}
      <div className="flex items-center justify-between">
        <div className="relative">
          <button
            data-testid="compressor-preset-select"
            onClick={() => setIsPresetOpen(!isPresetOpen)}
            className="flex items-center gap-2 px-3 py-2 bg-muted/50 rounded-lg border border-border hover:bg-muted transition-colors"
          >
            <span className="text-sm font-medium">
              {t('effects.compressor.presets', 'Presets')}
            </span>
            <ChevronDown className={`w-4 h-4 transition-transform ${isPresetOpen ? 'rotate-180' : ''}`} />
          </button>

          {isPresetOpen && (
            <>
              <div
                className="fixed inset-0 z-10"
                onClick={() => setIsPresetOpen(false)}
              />
              <div className="absolute top-full left-0 mt-1 z-20 w-48 bg-background border border-border rounded-lg shadow-lg overflow-hidden">
                {allPresets.map((preset) => (
                  <button
                    key={preset.name}
                    onClick={() => applyPreset(preset)}
                    className="w-full px-4 py-2 text-left text-sm hover:bg-muted transition-colors"
                  >
                    {preset.name}
                  </button>
                ))}
              </div>
            </>
          )}
        </div>

        <button
          onClick={resetToDefault}
          className="flex items-center gap-1.5 px-3 py-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors"
          title={t('effects.compressor.reset', 'Reset to Default')}
        >
          <RotateCcw className="w-4 h-4" />
          <span className="hidden sm:inline">{t('common.reset', 'Reset')}</span>
        </button>
      </div>

      {/* Transfer Curve Visualization */}
      <TransferCurve
        threshold={settings.thresholdDb}
        ratio={settings.ratio}
        knee={settings.kneeDb}
        makeupGain={settings.makeupGainDb}
      />

      {/* Main Controls Grid */}
      <div className="grid grid-cols-2 sm:grid-cols-3 gap-4">
        {/* Threshold */}
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
            {t('effects.compressor.threshold', 'Threshold')}
          </label>
          <div className="relative">
            <input
              data-testid="compressor-threshold"
              type="range"
              min="-60"
              max="0"
              step="0.5"
              value={settings.thresholdDb}
              onChange={(e) => updateParam('thresholdDb', parseFloat(e.target.value))}
              className="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>
          <div className="text-center">
            <span className="text-lg font-mono font-semibold">
              {settings.thresholdDb.toFixed(1)}
            </span>
            <span className="text-xs text-muted-foreground ml-1">dB</span>
          </div>
        </div>

        {/* Ratio */}
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
            {t('effects.compressor.ratio', 'Ratio')}
          </label>
          <div className="relative">
            <input
              data-testid="compressor-ratio"
              type="range"
              min="0"
              max={RATIO_STEPS.length - 1}
              step="1"
              value={findClosestRatioIndex(settings.ratio)}
              onChange={(e) => updateParam('ratio', RATIO_STEPS[parseInt(e.target.value)])}
              className="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>
          <div className="text-center">
            <span className="text-lg font-mono font-semibold">
              {settings.ratio >= 20 ? '\u221E' : settings.ratio.toFixed(1)}
            </span>
            <span className="text-xs text-muted-foreground ml-1">: 1</span>
          </div>
        </div>

        {/* Makeup Gain */}
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
            {t('effects.compressor.makeupGain', 'Makeup')}
          </label>
          <div className="relative">
            <input
              data-testid="compressor-makeup"
              type="range"
              min="0"
              max="24"
              step="0.5"
              value={settings.makeupGainDb}
              onChange={(e) => updateParam('makeupGainDb', parseFloat(e.target.value))}
              className="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>
          <div className="text-center">
            <span className="text-lg font-mono font-semibold">
              +{settings.makeupGainDb.toFixed(1)}
            </span>
            <span className="text-xs text-muted-foreground ml-1">dB</span>
          </div>
        </div>

        {/* Attack */}
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
            {t('effects.compressor.attack', 'Attack')}
          </label>
          <div className="relative">
            <input
              data-testid="compressor-attack"
              type="range"
              min="0"
              max="1"
              step="0.001"
              value={logScale(settings.attackMs, 0.1, 100)}
              onChange={(e) => updateParam('attackMs', expScale(parseFloat(e.target.value), 0.1, 100))}
              className="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>
          <div className="text-center">
            <span className="text-lg font-mono font-semibold">
              {formatTime(settings.attackMs)}
            </span>
            <span className="text-xs text-muted-foreground ml-1">ms</span>
          </div>
        </div>

        {/* Release */}
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
            {t('effects.compressor.release', 'Release')}
          </label>
          <div className="relative">
            <input
              data-testid="compressor-release"
              type="range"
              min="0"
              max="1"
              step="0.001"
              value={logScale(settings.releaseMs, 10, 1000)}
              onChange={(e) => updateParam('releaseMs', expScale(parseFloat(e.target.value), 10, 1000))}
              className="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>
          <div className="text-center">
            <span className="text-lg font-mono font-semibold">
              {formatTime(settings.releaseMs)}
            </span>
            <span className="text-xs text-muted-foreground ml-1">ms</span>
          </div>
        </div>

        {/* Knee */}
        <div className="space-y-2">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
            {t('effects.compressor.knee', 'Knee')}
          </label>
          <div className="relative">
            <input
              data-testid="compressor-knee"
              type="range"
              min="0"
              max="12"
              step="0.5"
              value={settings.kneeDb}
              onChange={(e) => updateParam('kneeDb', parseFloat(e.target.value))}
              className="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
            />
          </div>
          <div className="text-center">
            <span className="text-lg font-mono font-semibold">
              {settings.kneeDb.toFixed(1)}
            </span>
            <span className="text-xs text-muted-foreground ml-1">
              {settings.kneeDb === 0
                ? t('effects.compressor.hard', 'hard')
                : t('effects.compressor.soft', 'soft')}
            </span>
          </div>
        </div>
      </div>

      {/* Timing Visualization */}
      <TimingVisualization attack={settings.attackMs} release={settings.releaseMs} />

      {/* Info Section */}
      <div className="text-xs text-muted-foreground bg-muted/30 rounded-lg p-3 space-y-1">
        <p>
          <strong>{t('effects.compressor.thresholdDesc', 'Threshold')}:</strong>{' '}
          {t('effects.compressor.thresholdHelp', 'Signal level where compression starts')}
        </p>
        <p>
          <strong>{t('effects.compressor.ratioDesc', 'Ratio')}:</strong>{' '}
          {t('effects.compressor.ratioHelp', 'Amount of compression (4:1 = 4dB input becomes 1dB output above threshold)')}
        </p>
        <p>
          <strong>{t('effects.compressor.kneeDesc', 'Knee')}:</strong>{' '}
          {t('effects.compressor.kneeHelp', 'Softens the transition at threshold (0 = hard, higher = softer)')}
        </p>
      </div>
    </div>
  );
}

// Transfer Curve SVG component
interface TransferCurveProps {
  threshold: number;
  ratio: number;
  knee: number;
  makeupGain: number;
}

function TransferCurve({ threshold, ratio, knee, makeupGain }: TransferCurveProps) {
  const width = 200;
  const height = 200;
  const padding = 24;
  const graphWidth = width - padding * 2;
  const graphHeight = height - padding * 2;

  // dB range
  const minDb = -60;
  const maxDb = 0;

  // Convert dB to pixel coordinates
  const dbToX = (db: number) => padding + ((db - minDb) / (maxDb - minDb)) * graphWidth;
  const dbToY = (db: number) => padding + graphHeight - ((db - minDb) / (maxDb - minDb)) * graphHeight;

  // Calculate output level for given input
  const computeOutput = (inputDb: number): number => {
    const halfKnee = knee / 2;
    const kneeStart = threshold - halfKnee;
    const kneeEnd = threshold + halfKnee;

    let outputDb: number;

    if (knee <= 0 || inputDb <= kneeStart) {
      // Below knee or hard knee - linear
      if (inputDb <= threshold) {
        outputDb = inputDb;
      } else {
        outputDb = threshold + (inputDb - threshold) / ratio;
      }
    } else if (inputDb >= kneeEnd) {
      // Above knee - full compression
      outputDb = threshold + (inputDb - threshold) / ratio;
    } else {
      // Within knee - smooth transition
      const x = inputDb - kneeStart;
      const slopeChange = (1 - 1 / ratio) / (2 * knee);
      outputDb = inputDb - slopeChange * x * x;
    }

    return Math.min(0, outputDb + makeupGain);
  };

  // Generate path
  const points: string[] = [];
  for (let db = minDb; db <= maxDb; db += 0.5) {
    const output = computeOutput(db);
    points.push(`${dbToX(db)},${dbToY(output)}`);
  }
  const pathD = `M ${points.join(' L ')}`;

  // Grid lines
  const gridLines = [-48, -36, -24, -12, 0];

  return (
    <div className="flex justify-center">
      <div className="relative bg-muted/30 rounded-lg overflow-hidden border border-border">
        <svg width={width} height={height} className="text-muted-foreground">
          {/* Grid */}
          {gridLines.map((db) => (
            <g key={db}>
              {/* Vertical grid line */}
              <line
                x1={dbToX(db)}
                y1={padding}
                x2={dbToX(db)}
                y2={height - padding}
                stroke="currentColor"
                strokeOpacity={0.1}
                strokeWidth={1}
              />
              {/* Horizontal grid line */}
              <line
                x1={padding}
                y1={dbToY(db)}
                x2={width - padding}
                y2={dbToY(db)}
                stroke="currentColor"
                strokeOpacity={0.1}
                strokeWidth={1}
              />
            </g>
          ))}

          {/* Unity line (1:1) */}
          <line
            x1={dbToX(minDb)}
            y1={dbToY(minDb)}
            x2={dbToX(maxDb)}
            y2={dbToY(maxDb)}
            stroke="currentColor"
            strokeOpacity={0.2}
            strokeWidth={1}
            strokeDasharray="4,4"
          />

          {/* Threshold line */}
          <line
            x1={dbToX(threshold)}
            y1={padding}
            x2={dbToX(threshold)}
            y2={height - padding}
            stroke="hsl(var(--primary))"
            strokeOpacity={0.5}
            strokeWidth={1}
            strokeDasharray="2,2"
          />

          {/* Transfer curve */}
          <path
            d={pathD}
            fill="none"
            stroke="hsl(var(--primary))"
            strokeWidth={2}
            strokeLinecap="round"
            strokeLinejoin="round"
          />

          {/* Knee region highlight */}
          {knee > 0 && (
            <rect
              x={dbToX(threshold - knee / 2)}
              y={padding}
              width={(knee / (maxDb - minDb)) * graphWidth}
              height={graphHeight}
              fill="hsl(var(--primary))"
              fillOpacity={0.05}
            />
          )}

          {/* Axis labels */}
          <text
            x={width / 2}
            y={height - 4}
            textAnchor="middle"
            className="text-[10px] fill-current opacity-50"
          >
            Input (dB)
          </text>
          <text
            x={8}
            y={height / 2}
            textAnchor="middle"
            transform={`rotate(-90, 8, ${height / 2})`}
            className="text-[10px] fill-current opacity-50"
          >
            Output (dB)
          </text>

          {/* Corner labels */}
          <text x={padding + 2} y={height - padding - 4} className="text-[9px] fill-current opacity-40">
            {minDb}
          </text>
          <text x={width - padding - 8} y={height - padding - 4} className="text-[9px] fill-current opacity-40">
            0
          </text>
        </svg>

        {/* Threshold indicator */}
        <div
          className="absolute bottom-1 text-[10px] font-mono text-primary/70"
          style={{ left: dbToX(threshold) - 12 }}
        >
          T
        </div>
      </div>
    </div>
  );
}

// Timing visualization component
interface TimingVisualizationProps {
  attack: number;
  release: number;
}

function TimingVisualization({ attack, release }: TimingVisualizationProps) {
  const { t } = useTranslation();
  const width = 240;
  const height = 60;
  const padding = { left: 20, right: 20, top: 10, bottom: 20 };
  const graphWidth = width - padding.left - padding.right;
  const graphHeight = height - padding.top - padding.bottom;

  // Normalize times for visualization (0-1 range)
  const totalTime = attack + release;
  const attackRatio = attack / totalTime;

  // Attack curve point
  const attackEndX = padding.left + attackRatio * graphWidth * 0.5;
  const releaseEndX = width - padding.right;

  // Generate envelope path
  const pathD = `
    M ${padding.left},${height - padding.bottom}
    L ${padding.left},${height - padding.bottom}
    C ${padding.left + (attackEndX - padding.left) * 0.3},${height - padding.bottom}
      ${attackEndX - (attackEndX - padding.left) * 0.3},${padding.top}
      ${attackEndX},${padding.top}
    C ${attackEndX + (releaseEndX - attackEndX) * 0.1},${padding.top}
      ${releaseEndX - (releaseEndX - attackEndX) * 0.3},${height - padding.bottom}
      ${releaseEndX},${height - padding.bottom}
  `;

  return (
    <div className="flex flex-col items-center gap-2">
      <div className="text-xs text-muted-foreground uppercase tracking-wide">
        {t('effects.compressor.timing', 'Attack / Release Timing')}
      </div>
      <svg width={width} height={height} className="text-muted-foreground">
        {/* Background */}
        <rect
          x={padding.left}
          y={padding.top}
          width={graphWidth}
          height={graphHeight}
          fill="currentColor"
          fillOpacity={0.05}
          rx={2}
        />

        {/* Envelope curve */}
        <path
          d={pathD}
          fill="none"
          stroke="hsl(var(--primary))"
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
        />

        {/* Attack/Release labels */}
        <text
          x={padding.left + (attackEndX - padding.left) / 2}
          y={height - 4}
          textAnchor="middle"
          className="text-[9px] fill-current opacity-60"
        >
          {t('effects.compressor.attackLabel', 'ATK')}
        </text>
        <text
          x={attackEndX + (releaseEndX - attackEndX) / 2}
          y={height - 4}
          textAnchor="middle"
          className="text-[9px] fill-current opacity-60"
        >
          {t('effects.compressor.releaseLabel', 'REL')}
        </text>

        {/* Time markers */}
        <line
          x1={attackEndX}
          y1={padding.top}
          x2={attackEndX}
          y2={height - padding.bottom}
          stroke="currentColor"
          strokeOpacity={0.2}
          strokeDasharray="2,2"
        />
      </svg>

      {/* Time values */}
      <div className="flex justify-between w-full px-4 text-xs text-muted-foreground">
        <span>{formatTime(attack)} ms</span>
        <span>{formatTime(release)} ms</span>
      </div>
    </div>
  );
}

// Utility functions for logarithmic scaling
function logScale(value: number, min: number, max: number): number {
  const logMin = Math.log(min);
  const logMax = Math.log(max);
  const logValue = Math.log(Math.max(min, Math.min(max, value)));
  return (logValue - logMin) / (logMax - logMin);
}

function expScale(normalized: number, min: number, max: number): number {
  const logMin = Math.log(min);
  const logMax = Math.log(max);
  return Math.exp(logMin + normalized * (logMax - logMin));
}

// Format time value for display
function formatTime(ms: number): string {
  if (ms < 1) {
    return ms.toFixed(2);
  } else if (ms < 10) {
    return ms.toFixed(1);
  } else {
    return Math.round(ms).toString();
  }
}

export default CompressorEditor;
