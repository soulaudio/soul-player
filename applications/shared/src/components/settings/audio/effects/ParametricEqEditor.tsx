// Industry-standard Parametric EQ Editor inspired by FabFilter Pro-Q
// Features: Interactive frequency display with draggable nodes, Q visualization, band list with precise controls

import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Plus,
  Trash2,
  RotateCcw,
  ChevronDown,
  Power,
} from 'lucide-react';

// EQ Band interface matching backend
export interface EqBand {
  frequency: number;  // Hz (20-20000)
  gain: number;       // dB (-24 to +24)
  q: number;          // 0.1 to 10
  filterType?: FilterType;
  enabled?: boolean;
}

export type FilterType = 'bell' | 'lowShelf' | 'highShelf' | 'lowPass' | 'highPass';

export interface ParametricEqEditorProps {
  bands: EqBand[];
  onBandsChange: (bands: EqBand[]) => void;
  slotIndex: number;
}

// EQ Presets
interface EqPreset {
  name: string;
  bands: EqBand[];
}

const EQ_PRESETS: EqPreset[] = [
  {
    name: 'eqPresets.flat',
    bands: [
      { frequency: 100, gain: 0, q: 1.0, filterType: 'bell', enabled: true },
      { frequency: 1000, gain: 0, q: 1.0, filterType: 'bell', enabled: true },
      { frequency: 10000, gain: 0, q: 1.0, filterType: 'bell', enabled: true },
    ],
  },
  {
    name: 'eqPresets.bassBoost',
    bands: [
      { frequency: 60, gain: 6, q: 0.7, filterType: 'lowShelf', enabled: true },
      { frequency: 150, gain: 3, q: 1.0, filterType: 'bell', enabled: true },
      { frequency: 400, gain: -2, q: 1.4, filterType: 'bell', enabled: true },
    ],
  },
  {
    name: 'eqPresets.trebleBoost',
    bands: [
      { frequency: 3000, gain: 2, q: 1.0, filterType: 'bell', enabled: true },
      { frequency: 8000, gain: 4, q: 0.7, filterType: 'highShelf', enabled: true },
      { frequency: 12000, gain: 3, q: 1.0, filterType: 'bell', enabled: true },
    ],
  },
  {
    name: 'eqPresets.vocal',
    bands: [
      { frequency: 80, gain: -6, q: 0.7, filterType: 'highPass', enabled: true },
      { frequency: 250, gain: -3, q: 1.4, filterType: 'bell', enabled: true },
      { frequency: 3500, gain: 4, q: 1.2, filterType: 'bell', enabled: true },
      { frequency: 8000, gain: 2, q: 0.8, filterType: 'highShelf', enabled: true },
    ],
  },
  {
    name: 'eqPresets.loudness',
    bands: [
      { frequency: 60, gain: 6, q: 0.7, filterType: 'lowShelf', enabled: true },
      { frequency: 1000, gain: -2, q: 0.5, filterType: 'bell', enabled: true },
      { frequency: 12000, gain: 5, q: 0.7, filterType: 'highShelf', enabled: true },
    ],
  },
  {
    name: 'eqPresets.deEsser',
    bands: [
      { frequency: 5500, gain: -6, q: 4.0, filterType: 'bell', enabled: true },
      { frequency: 7500, gain: -4, q: 3.0, filterType: 'bell', enabled: true },
    ],
  },
];

// Constants for frequency display
const MIN_FREQ = 20;
const MAX_FREQ = 20000;
const MIN_GAIN = -24;
const MAX_GAIN = 24;
const MIN_Q = 0.1;
const MAX_Q = 10;
const MAX_BANDS = 8;

// Frequency grid lines (logarithmic)
const FREQ_GRID_LINES = [20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
const GAIN_GRID_LINES = [-24, -12, 0, 12, 24];

// Convert frequency to x position (logarithmic scale)
function freqToX(freq: number, width: number): number {
  const logMin = Math.log10(MIN_FREQ);
  const logMax = Math.log10(MAX_FREQ);
  const logFreq = Math.log10(Math.max(MIN_FREQ, Math.min(MAX_FREQ, freq)));
  return ((logFreq - logMin) / (logMax - logMin)) * width;
}

// Convert x position to frequency
function xToFreq(x: number, width: number): number {
  const logMin = Math.log10(MIN_FREQ);
  const logMax = Math.log10(MAX_FREQ);
  const logFreq = logMin + (x / width) * (logMax - logMin);
  return Math.pow(10, logFreq);
}

// Convert gain to y position
function gainToY(gain: number, height: number): number {
  return ((MAX_GAIN - gain) / (MAX_GAIN - MIN_GAIN)) * height;
}

// Convert y position to gain
function yToGain(y: number, height: number): number {
  return MAX_GAIN - (y / height) * (MAX_GAIN - MIN_GAIN);
}

// Format frequency for display
function formatFrequency(freq: number): string {
  if (freq >= 1000) {
    return `${(freq / 1000).toFixed(freq >= 10000 ? 0 : 1)}k`;
  }
  return `${Math.round(freq)}`;
}

// Get node color based on index
function getNodeColor(index: number): string {
  const colors = [
    '#f97316', // orange
    '#22c55e', // green
    '#3b82f6', // blue
    '#a855f7', // purple
    '#ef4444', // red
    '#eab308', // yellow
    '#06b6d4', // cyan
    '#ec4899', // pink
  ];
  return colors[index % colors.length];
}

// Filter type icons/labels
const FILTER_TYPE_LABELS: Record<FilterType, string> = {
  bell: 'filterTypes.bell',
  lowShelf: 'filterTypes.lowShelf',
  highShelf: 'filterTypes.highShelf',
  lowPass: 'filterTypes.lowPass',
  highPass: 'filterTypes.highPass',
};

// Calculate EQ curve points
function calculateEqCurve(
  bands: EqBand[],
  width: number,
  height: number,
  numPoints: number = 200
): string {
  const points: string[] = [];

  for (let i = 0; i <= numPoints; i++) {
    const x = (i / numPoints) * width;
    const freq = xToFreq(x, width);
    let totalGain = 0;

    // Sum contribution from all enabled bands
    for (const band of bands) {
      if (band.enabled === false) continue;

      const contribution = calculateBandContribution(
        freq,
        band.frequency,
        band.gain,
        band.q,
        band.filterType || 'bell'
      );
      totalGain += contribution;
    }

    // Clamp to display range
    totalGain = Math.max(MIN_GAIN, Math.min(MAX_GAIN, totalGain));
    const y = gainToY(totalGain, height);
    points.push(`${x},${y}`);
  }

  return `M${points.join(' L')}`;
}

// Calculate single band's contribution at a frequency
function calculateBandContribution(
  freq: number,
  centerFreq: number,
  gain: number,
  q: number,
  filterType: FilterType
): number {
  const freqRatio = freq / centerFreq;
  const logRatio = Math.log2(freqRatio);

  switch (filterType) {
    case 'bell': {
      // Bell curve approximation
      const width = 1 / q;
      const x = logRatio / width;
      return gain * Math.exp(-0.5 * x * x);
    }
    case 'lowShelf': {
      // Low shelf approximation
      const transition = 1 / (1 + Math.exp(q * 2 * logRatio));
      return gain * transition;
    }
    case 'highShelf': {
      // High shelf approximation
      const transition = 1 / (1 + Math.exp(-q * 2 * logRatio));
      return gain * transition;
    }
    case 'lowPass': {
      // Low pass approximation
      if (freq <= centerFreq) return 0;
      const rolloff = Math.pow(centerFreq / freq, q * 2);
      return -24 * (1 - rolloff);
    }
    case 'highPass': {
      // High pass approximation
      if (freq >= centerFreq) return 0;
      const rolloff = Math.pow(freq / centerFreq, q * 2);
      return -24 * (1 - rolloff);
    }
    default:
      return 0;
  }
}

export function ParametricEqEditor({
  bands,
  onBandsChange,
  slotIndex: _slotIndex, // Reserved for potential direct backend calls
}: ParametricEqEditorProps) {
  const { t } = useTranslation();
  const canvasRef = useRef<SVGSVGElement>(null);
  const [dimensions, setDimensions] = useState({ width: 600, height: 280 });
  const [selectedBand, setSelectedBand] = useState<number | null>(null);
  const [dragState, setDragState] = useState<{
    bandIndex: number;
    startX: number;
    startY: number;
    startFreq: number;
    startGain: number;
    isDragging: boolean;
  } | null>(null);
  const [showPresets, setShowPresets] = useState(false);

  // Ensure bands have all required properties
  const normalizedBands = useMemo(() => {
    return bands.map((band) => ({
      frequency: band.frequency,
      gain: band.gain,
      q: band.q,
      filterType: band.filterType || 'bell' as FilterType,
      enabled: band.enabled !== false,
    }));
  }, [bands]);

  // Update dimensions on resize
  useEffect(() => {
    const updateDimensions = () => {
      if (canvasRef.current) {
        const rect = canvasRef.current.getBoundingClientRect();
        setDimensions({
          width: rect.width || 600,
          height: 280,
        });
      }
    };

    updateDimensions();
    window.addEventListener('resize', updateDimensions);
    return () => window.removeEventListener('resize', updateDimensions);
  }, []);

  // Handle band changes - only update parent state, let parent handle backend
  // This prevents double backend calls and allows smooth dragging
  const handleBandsChange = useCallback((newBands: EqBand[]) => {
    onBandsChange(newBands);
  }, [onBandsChange]);

  // Mouse event handlers for dragging
  const handleMouseDown = useCallback((e: React.MouseEvent, bandIndex: number) => {
    e.preventDefault();
    const band = normalizedBands[bandIndex];
    setDragState({
      bandIndex,
      startX: e.clientX,
      startY: e.clientY,
      startFreq: band.frequency,
      startGain: band.gain,
      isDragging: false,
    });
    setSelectedBand(bandIndex);
  }, [normalizedBands]);

  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragState || !canvasRef.current) return;

    const rect = canvasRef.current.getBoundingClientRect();
    const deltaX = e.clientX - dragState.startX;
    const deltaY = e.clientY - dragState.startY;

    // Mark as dragging if moved significantly
    if (!dragState.isDragging && (Math.abs(deltaX) > 3 || Math.abs(deltaY) > 3)) {
      setDragState({ ...dragState, isDragging: true });
    }

    if (!dragState.isDragging && Math.abs(deltaX) <= 3 && Math.abs(deltaY) <= 3) {
      return;
    }

    // Calculate new frequency and gain
    const startX = freqToX(dragState.startFreq, rect.width);
    const startY = gainToY(dragState.startGain, 280);

    const newX = Math.max(0, Math.min(rect.width, startX + deltaX));
    const newY = Math.max(0, Math.min(280, startY + deltaY));

    const newFreq = xToFreq(newX, rect.width);
    const newGain = yToGain(newY, 280);

    // Update band
    const newBands = [...normalizedBands];
    newBands[dragState.bandIndex] = {
      ...newBands[dragState.bandIndex],
      frequency: Math.round(newFreq),
      gain: Math.round(newGain * 10) / 10,
    };
    onBandsChange(newBands);
  }, [dragState, normalizedBands, onBandsChange]);

  const handleMouseUp = useCallback(() => {
    // Just clear drag state - parent handles backend updates via onBandsChange
    setDragState(null);
  }, []);

  // Add/remove bands
  const addBand = useCallback(() => {
    if (normalizedBands.length >= MAX_BANDS) return;

    // Find a frequency that's not too close to existing bands
    let newFreq = 1000;
    const existingFreqs = normalizedBands.map((b) => b.frequency);
    const candidates = [100, 250, 500, 1000, 2000, 4000, 8000, 16000];
    for (const freq of candidates) {
      if (!existingFreqs.some((f) => Math.abs(Math.log10(f) - Math.log10(freq)) < 0.2)) {
        newFreq = freq;
        break;
      }
    }

    const newBand: EqBand = {
      frequency: newFreq,
      gain: 0,
      q: 1.0,
      filterType: 'bell',
      enabled: true,
    };

    handleBandsChange([...normalizedBands, newBand]);
    setSelectedBand(normalizedBands.length);
  }, [normalizedBands, handleBandsChange]);

  const removeBand = useCallback((index: number) => {
    if (normalizedBands.length <= 1) return;
    const newBands = normalizedBands.filter((_, i) => i !== index);
    handleBandsChange(newBands);
    if (selectedBand === index) {
      setSelectedBand(null);
    } else if (selectedBand !== null && selectedBand > index) {
      setSelectedBand(selectedBand - 1);
    }
  }, [normalizedBands, handleBandsChange, selectedBand]);

  const toggleBand = useCallback((index: number) => {
    const newBands = [...normalizedBands];
    newBands[index] = {
      ...newBands[index],
      enabled: !newBands[index].enabled,
    };
    handleBandsChange(newBands);
  }, [normalizedBands, handleBandsChange]);

  // Update band properties
  const updateBand = useCallback((index: number, updates: Partial<EqBand>) => {
    const newBands = [...normalizedBands];
    newBands[index] = { ...newBands[index], ...updates };
    handleBandsChange(newBands);
  }, [normalizedBands, handleBandsChange]);

  // Apply preset
  const applyPreset = useCallback((preset: EqPreset) => {
    handleBandsChange([...preset.bands]);
    setShowPresets(false);
    setSelectedBand(null);
  }, [handleBandsChange]);

  // Reset to flat
  const resetToFlat = useCallback(() => {
    const flatBands = normalizedBands.map((band) => ({
      ...band,
      gain: 0,
    }));
    handleBandsChange(flatBands);
  }, [normalizedBands, handleBandsChange]);

  // Calculate EQ curve path
  const curvePath = useMemo(() => {
    return calculateEqCurve(normalizedBands, dimensions.width, 280);
  }, [normalizedBands, dimensions.width]);

  return (
    <div data-testid="parametric-eq-editor" className="space-y-4">
      {/* Header with presets */}
      <div className="flex items-center justify-between">
        <div className="text-sm font-medium text-muted-foreground">
          {t('parametricEq.title', 'Parametric EQ')}
        </div>
        <div className="flex items-center gap-2">
          {/* Presets dropdown */}
          <div className="relative">
            <button
              data-testid="eq-preset-select"
              onClick={() => setShowPresets(!showPresets)}
              className="flex items-center gap-1 px-3 py-1.5 text-sm border border-border rounded-lg hover:bg-muted transition-colors"
            >
              {t('parametricEq.presets', 'Presets')}
              <ChevronDown className={`w-4 h-4 transition-transform ${showPresets ? 'rotate-180' : ''}`} />
            </button>
            {showPresets && (
              <div className="absolute right-0 top-full mt-1 w-48 bg-background border border-border rounded-lg shadow-lg z-20 py-1">
                {EQ_PRESETS.map((preset) => (
                  <button
                    key={preset.name}
                    onClick={() => applyPreset(preset)}
                    className="w-full text-left px-3 py-2 text-sm hover:bg-muted transition-colors"
                  >
                    {t(preset.name, preset.name.split('.')[1])}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Reset button */}
          <button
            onClick={resetToFlat}
            className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted rounded transition-colors"
            title={t('parametricEq.resetToFlat', 'Reset to flat')}
          >
            <RotateCcw className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Frequency Display Canvas */}
      <div
        className="relative rounded-lg overflow-hidden border border-border"
        style={{
          background: 'linear-gradient(180deg, hsl(var(--muted) / 0.3) 0%, hsl(var(--muted) / 0.1) 50%, hsl(var(--muted) / 0.3) 100%)',
        }}
      >
        <svg
          ref={canvasRef}
          className="w-full"
          viewBox={`0 0 ${dimensions.width} 280`}
          preserveAspectRatio="none"
          style={{ height: 280 }}
          onMouseMove={handleMouseMove}
          onMouseUp={handleMouseUp}
          onMouseLeave={handleMouseUp}
        >
          {/* Gradient definitions */}
          <defs>
            <linearGradient id="curveGradient" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="hsl(var(--primary))" stopOpacity="0.3" />
              <stop offset="50%" stopColor="hsl(var(--primary))" stopOpacity="0.1" />
              <stop offset="100%" stopColor="hsl(var(--primary))" stopOpacity="0.3" />
            </linearGradient>
            <filter id="glow">
              <feGaussianBlur stdDeviation="2" result="coloredBlur" />
              <feMerge>
                <feMergeNode in="coloredBlur" />
                <feMergeNode in="SourceGraphic" />
              </feMerge>
            </filter>
          </defs>

          {/* Gain grid lines */}
          {GAIN_GRID_LINES.map((gain) => {
            const y = gainToY(gain, 280);
            return (
              <g key={`gain-${gain}`}>
                <line
                  x1="0"
                  y1={y}
                  x2={dimensions.width}
                  y2={y}
                  stroke="hsl(var(--border))"
                  strokeWidth={gain === 0 ? 1.5 : 0.5}
                  strokeOpacity={gain === 0 ? 0.8 : 0.4}
                />
                <text
                  x="4"
                  y={y - 4}
                  className="text-xs fill-muted-foreground"
                  fontSize="10"
                >
                  {gain > 0 ? '+' : ''}{gain}dB
                </text>
              </g>
            );
          })}

          {/* Frequency grid lines */}
          {FREQ_GRID_LINES.map((freq) => {
            const x = freqToX(freq, dimensions.width);
            return (
              <g key={`freq-${freq}`}>
                <line
                  x1={x}
                  y1="0"
                  x2={x}
                  y2="280"
                  stroke="hsl(var(--border))"
                  strokeWidth={freq === 1000 ? 1 : 0.5}
                  strokeOpacity={freq === 1000 ? 0.6 : 0.3}
                />
                <text
                  x={x}
                  y="274"
                  className="text-xs fill-muted-foreground"
                  fontSize="10"
                  textAnchor="middle"
                >
                  {formatFrequency(freq)}
                </text>
              </g>
            );
          })}

          {/* EQ Curve */}
          <path
            d={curvePath}
            fill="none"
            stroke="hsl(var(--primary))"
            strokeWidth="2"
            filter="url(#glow)"
          />

          {/* Fill under curve */}
          <path
            d={`${curvePath} L${dimensions.width},${gainToY(0, 280)} L0,${gainToY(0, 280)} Z`}
            fill="url(#curveGradient)"
          />

          {/* Band nodes */}
          {normalizedBands.map((band, index) => {
            if (!band.enabled) return null;

            const x = freqToX(band.frequency, dimensions.width);
            const y = gainToY(band.gain, 280);
            const color = getNodeColor(index);
            const isSelected = selectedBand === index;

            // Q visualization radius
            const qRadius = Math.max(8, Math.min(40, 30 / band.q));

            return (
              <g key={index} style={{ cursor: 'grab' }}>
                {/* Q indicator circle */}
                <circle
                  cx={x}
                  cy={y}
                  r={qRadius}
                  fill={color}
                  fillOpacity={0.15}
                  stroke={color}
                  strokeWidth={1}
                  strokeOpacity={0.3}
                />

                {/* Node circle */}
                <circle
                  cx={x}
                  cy={y}
                  r={isSelected ? 10 : 8}
                  fill={color}
                  stroke="white"
                  strokeWidth={2}
                  style={{ cursor: 'grab', transition: 'r 0.1s' }}
                  onMouseDown={(e) => handleMouseDown(e, index)}
                />

                {/* Selection ring */}
                {isSelected && (
                  <circle
                    cx={x}
                    cy={y}
                    r={14}
                    fill="none"
                    stroke={color}
                    strokeWidth={2}
                    strokeOpacity={0.5}
                    strokeDasharray="4 2"
                  />
                )}

                {/* Band label */}
                <text
                  x={x}
                  y={y - 16}
                  textAnchor="middle"
                  className="text-xs font-medium pointer-events-none"
                  fill={color}
                  fontSize="11"
                >
                  {index + 1}
                </text>
              </g>
            );
          })}
        </svg>
      </div>

      {/* Band List Controls */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">
            {t('parametricEq.bands', 'Bands')} ({normalizedBands.length}/{MAX_BANDS})
          </span>
          <button
            data-testid="eq-add-band-btn"
            onClick={addBand}
            disabled={normalizedBands.length >= MAX_BANDS}
            className="flex items-center gap-1 px-2 py-1 text-sm border border-border rounded hover:bg-muted disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            <Plus className="w-4 h-4" />
            {t('parametricEq.addBand', 'Add Band')}
          </button>
        </div>

        {/* Band rows */}
        <div className="space-y-2">
          {normalizedBands.map((band, index) => (
            <div
              key={index}
              data-testid={`eq-band-${index}`}
              className={`
                p-3 border rounded-lg transition-colors
                ${selectedBand === index ? 'border-primary bg-primary/5' : 'border-border hover:border-primary/50'}
              `}
              onClick={() => setSelectedBand(index)}
            >
              <div className="flex items-center gap-3">
                {/* Band indicator */}
                <div
                  className="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold text-white"
                  style={{ backgroundColor: getNodeColor(index) }}
                >
                  {index + 1}
                </div>

                {/* Enable toggle */}
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleBand(index);
                  }}
                  className={`p-1.5 rounded transition-colors ${
                    band.enabled ? 'text-primary bg-primary/10' : 'text-muted-foreground hover:bg-muted'
                  }`}
                  title={band.enabled ? t('parametricEq.disable', 'Disable') : t('parametricEq.enable', 'Enable')}
                >
                  <Power className="w-4 h-4" />
                </button>

                {/* Filter type selector */}
                <select
                  value={band.filterType}
                  onChange={(e) => updateBand(index, { filterType: e.target.value as FilterType })}
                  onClick={(e) => e.stopPropagation()}
                  className="px-2 py-1 text-sm bg-background border border-border rounded focus:outline-none focus:ring-1 focus:ring-primary"
                >
                  {Object.entries(FILTER_TYPE_LABELS).map(([type, label]) => (
                    <option key={type} value={type}>
                      {t(label, type)}
                    </option>
                  ))}
                </select>

                {/* Frequency input */}
                <div className="flex items-center gap-1">
                  <input
                    data-testid={`eq-frequency-${index}`}
                    type="number"
                    value={Math.round(band.frequency)}
                    onChange={(e) => {
                      const value = Math.max(MIN_FREQ, Math.min(MAX_FREQ, Number(e.target.value)));
                      updateBand(index, { frequency: value });
                    }}
                    onClick={(e) => e.stopPropagation()}
                    className="w-20 px-2 py-1 text-sm bg-background border border-border rounded focus:outline-none focus:ring-1 focus:ring-primary text-right"
                    min={MIN_FREQ}
                    max={MAX_FREQ}
                  />
                  <span className="text-xs text-muted-foreground">Hz</span>
                </div>

                {/* Gain slider + value */}
                <div className="flex items-center gap-2 flex-1 max-w-48">
                  <input
                    data-testid={`eq-gain-${index}`}
                    type="range"
                    value={band.gain}
                    onChange={(e) => updateBand(index, { gain: Number(e.target.value) })}
                    onClick={(e) => e.stopPropagation()}
                    className="flex-1 accent-primary"
                    min={MIN_GAIN}
                    max={MAX_GAIN}
                    step={0.5}
                  />
                  <span className="w-14 text-sm text-right tabular-nums">
                    {band.gain > 0 ? '+' : ''}{band.gain.toFixed(1)}dB
                  </span>
                </div>

                {/* Q control */}
                <div className="flex items-center gap-1">
                  <span className="text-xs text-muted-foreground">Q</span>
                  <input
                    data-testid={`eq-q-${index}`}
                    type="number"
                    value={band.q.toFixed(1)}
                    onChange={(e) => {
                      const value = Math.max(MIN_Q, Math.min(MAX_Q, Number(e.target.value)));
                      updateBand(index, { q: value });
                    }}
                    onClick={(e) => e.stopPropagation()}
                    className="w-14 px-2 py-1 text-sm bg-background border border-border rounded focus:outline-none focus:ring-1 focus:ring-primary text-right"
                    min={MIN_Q}
                    max={MAX_Q}
                    step={0.1}
                  />
                </div>

                {/* Delete button */}
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    removeBand(index);
                  }}
                  disabled={normalizedBands.length <= 1}
                  className="p-1.5 text-muted-foreground hover:text-destructive hover:bg-destructive/10 rounded disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                  title={t('parametricEq.removeBand', 'Remove band')}
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Instructions */}
      <div className="text-xs text-muted-foreground bg-muted/30 p-3 rounded-lg space-y-1">
        <p>
          <strong>{t('parametricEq.tips', 'Tips')}:</strong>
        </p>
        <ul className="list-disc list-inside space-y-0.5 ml-2">
          <li>{t('parametricEq.tipDrag', 'Drag nodes to adjust frequency and gain')}</li>
          <li>{t('parametricEq.tipQ', 'Lower Q values create wider curves, higher Q creates narrower peaks')}</li>
          <li>{t('parametricEq.tipTypes', 'Use shelf filters for broad tonal adjustments, bell for surgical cuts')}</li>
        </ul>
      </div>
    </div>
  );
}
