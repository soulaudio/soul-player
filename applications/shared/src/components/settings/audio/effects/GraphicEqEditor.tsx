// Industry-standard 10-band Graphic EQ Editor
// Based on ISO standard center frequencies with vertical sliders

import { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { RotateCcw, ChevronDown, Check } from 'lucide-react';

// ISO standard 10-band center frequencies (Hz)
const ISO_FREQUENCIES = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000] as const;

// Format frequency for display
const formatFrequency = (hz: number): string => {
  if (hz >= 1000) {
    return `${hz / 1000}k`;
  }
  return String(hz);
};

// Common EQ presets
const BUILTIN_PRESETS = [
  { id: 'Flat', gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] },
  { id: 'Bass Boost', gains: [6, 5, 4, 2, 0, 0, 0, 0, 0, 0] },
  { id: 'Treble Boost', gains: [0, 0, 0, 0, 0, 1, 2, 4, 5, 6] },
  { id: 'Vocal Boost', gains: [-2, -1, 0, 2, 4, 4, 2, 0, -1, -2] },
  { id: 'Rock', gains: [4, 3, 2, 0, -1, 0, 2, 3, 4, 4] },
  { id: 'Pop', gains: [-1, 0, 2, 4, 4, 2, 0, -1, -1, 0] },
  { id: 'Jazz', gains: [3, 2, 1, 2, -2, -2, 0, 1, 2, 3] },
  { id: 'Classical', gains: [3, 2, 1, 1, 0, 0, 0, 1, 2, 3] },
  { id: 'Loudness', gains: [4, 3, 0, 0, -1, 0, -1, 0, 3, 4] },
] as const;

export interface GraphicEqSettings {
  preset: string;
  bandCount: number;
  gains: number[];
}

export interface GraphicEqEditorProps {
  settings: GraphicEqSettings;
  onSettingsChange: (settings: GraphicEqSettings) => void;
  slotIndex: number;
}

interface BandSliderProps {
  frequency: number;
  gain: number;
  index: number;
  onGainChange: (index: number, gain: number) => void;
  isActive: boolean;
  onActiveChange: (index: number | null) => void;
  testId?: string;
}

// Individual band slider component
function BandSlider({
  frequency,
  gain,
  index,
  onGainChange,
  isActive,
  onActiveChange,
  testId,
}: BandSliderProps) {
  const sliderRef = useRef<HTMLDivElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [localGain, setLocalGain] = useState(gain);

  // Sync local state with prop
  useEffect(() => {
    if (!isDragging) {
      setLocalGain(gain);
    }
  }, [gain, isDragging]);

  // Calculate position from gain (-12 to +12 dB)
  const gainToPercent = (g: number): number => {
    return ((g + 12) / 24) * 100;
  };

  // Calculate gain from position
  const percentToGain = (p: number): number => {
    const clamped = Math.max(0, Math.min(100, p));
    return (clamped / 100) * 24 - 12;
  };

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    onActiveChange(index);

    const updateFromEvent = (clientY: number) => {
      if (!sliderRef.current) return;
      const rect = sliderRef.current.getBoundingClientRect();
      const percent = 100 - ((clientY - rect.top) / rect.height) * 100;
      const newGain = Math.round(percentToGain(percent) * 2) / 2; // Round to 0.5 dB steps
      setLocalGain(newGain);
      onGainChange(index, newGain);
    };

    updateFromEvent(e.clientY);

    const handleMouseMove = (moveEvent: MouseEvent) => {
      updateFromEvent(moveEvent.clientY);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
      onActiveChange(null);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [index, onGainChange, onActiveChange]);

  // Touch support
  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    e.preventDefault();
    setIsDragging(true);
    onActiveChange(index);

    const updateFromTouch = (clientY: number) => {
      if (!sliderRef.current) return;
      const rect = sliderRef.current.getBoundingClientRect();
      const percent = 100 - ((clientY - rect.top) / rect.height) * 100;
      const newGain = Math.round(percentToGain(percent) * 2) / 2;
      setLocalGain(newGain);
      onGainChange(index, newGain);
    };

    updateFromTouch(e.touches[0].clientY);

    const handleTouchMove = (moveEvent: TouchEvent) => {
      moveEvent.preventDefault();
      updateFromTouch(moveEvent.touches[0].clientY);
    };

    const handleTouchEnd = () => {
      setIsDragging(false);
      onActiveChange(null);
      document.removeEventListener('touchmove', handleTouchMove);
      document.removeEventListener('touchend', handleTouchEnd);
    };

    document.addEventListener('touchmove', handleTouchMove, { passive: false });
    document.addEventListener('touchend', handleTouchEnd);
  }, [index, onGainChange, onActiveChange]);

  const fillPercent = gainToPercent(localGain);
  const isPositive = localGain > 0;
  const isNegative = localGain < 0;

  return (
    <div data-testid={testId} className="flex flex-col items-center gap-2 flex-1 min-w-0">
      {/* Gain value display (shown on hover/drag) */}
      <div
        className={`
          text-xs font-mono transition-opacity duration-150 h-5 flex items-center
          ${isActive || isDragging ? 'opacity-100' : 'opacity-0'}
        `}
      >
        <span className={isPositive ? 'text-green-500' : isNegative ? 'text-red-400' : 'text-muted-foreground'}>
          {localGain > 0 ? '+' : ''}{localGain.toFixed(1)}
        </span>
      </div>

      {/* Slider track */}
      <div
        ref={sliderRef}
        className="relative w-6 h-40 cursor-pointer select-none touch-none"
        onMouseDown={handleMouseDown}
        onTouchStart={handleTouchStart}
      >
        {/* Background track */}
        <div className="absolute inset-x-1 inset-y-0 bg-muted/50 rounded-full" />

        {/* Grid lines */}
        <div className="absolute inset-x-0 top-0 h-px bg-border/50" /> {/* +12dB */}
        <div className="absolute inset-x-0 top-1/4 h-px bg-border/30" /> {/* +6dB */}
        <div className="absolute inset-x-0 top-1/2 h-px bg-primary/60" /> {/* 0dB center */}
        <div className="absolute inset-x-0 top-3/4 h-px bg-border/30" /> {/* -6dB */}
        <div className="absolute inset-x-0 bottom-0 h-px bg-border/50" /> {/* -12dB */}

        {/* Fill gradient from 0dB center to current value */}
        <div
          className={`
            absolute inset-x-1 rounded-full transition-all duration-75
            ${isPositive ? 'bg-gradient-to-t from-primary/40 to-primary' : ''}
            ${isNegative ? 'bg-gradient-to-b from-primary/40 to-orange-500/80' : ''}
          `}
          style={{
            top: isPositive ? `${100 - fillPercent}%` : '50%',
            bottom: isNegative ? `${fillPercent}%` : '50%',
          }}
        />

        {/* Thumb */}
        <div
          className={`
            absolute left-1/2 -translate-x-1/2 w-5 h-3 rounded-sm
            transition-all duration-75 shadow-md
            ${isDragging ? 'scale-110 bg-primary ring-2 ring-primary/30' : 'bg-primary hover:scale-105'}
          `}
          style={{
            top: `calc(${100 - fillPercent}% - 6px)`,
          }}
        />
      </div>

      {/* Frequency label */}
      <div className="text-xs text-muted-foreground font-medium">
        {formatFrequency(frequency)}
      </div>
    </div>
  );
}

export function GraphicEqEditor({
  settings,
  onSettingsChange,
  slotIndex: _slotIndex, // Reserved for potential direct backend calls
}: GraphicEqEditorProps) {
  const { t } = useTranslation();
  const [presets, setPresets] = useState<{ id: string; gains: number[] }[]>(BUILTIN_PRESETS as unknown as { id: string; gains: number[] }[]);
  const [showPresetDropdown, setShowPresetDropdown] = useState(false);
  const [activeBand, setActiveBand] = useState<number | null>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Ensure gains array has correct length
  const gains = useMemo(() => {
    if (settings.gains.length === 10) {
      return settings.gains;
    }
    // Pad or trim to 10 bands
    const result = [...settings.gains];
    while (result.length < 10) {
      result.push(0);
    }
    return result.slice(0, 10);
  }, [settings.gains]);

  // Load presets from backend
  useEffect(() => {
    const loadPresets = async () => {
      try {
        // Backend returns tuples: [string, GraphicEqData][] where GraphicEqData = { preset, bandCount, gains }
        // We need to transform this to our expected format: { id: string; gains: number[] }[]
        const backendResponse = await invoke<unknown>('get_graphic_eq_presets');

        if (backendResponse && Array.isArray(backendResponse) && backendResponse.length > 0) {
          const transformedPresets: { id: string; gains: number[] }[] = [];

          for (const item of backendResponse) {
            // Skip null/undefined items
            if (item == null) continue;

            // Handle tuple format: [string, GraphicEqData]
            if (Array.isArray(item) && item.length >= 2) {
              const [name, data] = item;
              if (typeof name === 'string' && data && typeof data === 'object' && 'gains' in data && Array.isArray(data.gains)) {
                transformedPresets.push({
                  id: name,
                  gains: data.gains.map((g: unknown) => typeof g === 'number' ? g : 0),
                });
              }
            }
            // Handle object format: { id: string; gains: number[] }
            else if (typeof item === 'object' && 'id' in item && 'gains' in item) {
              const preset = item as { id: string; gains: number[] };
              if (typeof preset.id === 'string' && Array.isArray(preset.gains)) {
                transformedPresets.push({
                  id: preset.id,
                  gains: preset.gains.map(g => typeof g === 'number' ? g : 0),
                });
              }
            }
          }

          if (transformedPresets.length > 0) {
            setPresets(transformedPresets);
          }
        }
      } catch (error) {
        console.error('Failed to load presets, using defaults:', error);
        // Keep using builtin presets
      }
    };
    loadPresets();
  }, []);

  // Close dropdown on outside click
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowPresetDropdown(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Handle individual band change - parent handles backend via onSettingsChange
  const handleBandChange = useCallback((index: number, gain: number) => {
    const newGains = [...gains];
    newGains[index] = gain;

    // Check if it matches any preset (with null safety)
    const validPresets = presets.filter(
      (p): p is { id: string; gains: number[] } =>
        p != null && typeof p.id === 'string' && Array.isArray(p.gains)
    );
    const matchingPreset = validPresets.find(p =>
      p.gains.every((g, i) => Math.abs(g - newGains[i]) < 0.1)
    );

    const newSettings: GraphicEqSettings = {
      ...settings,
      preset: matchingPreset?.id || 'Custom',
      gains: newGains,
    };

    onSettingsChange(newSettings);
  }, [gains, presets, settings, onSettingsChange]);

  // Apply preset
  const applyPreset = useCallback((preset: { id: string; gains: number[] }) => {
    const newSettings: GraphicEqSettings = {
      ...settings,
      preset: preset.id,
      gains: [...preset.gains],
    };
    onSettingsChange(newSettings);
    setShowPresetDropdown(false);
  }, [settings, onSettingsChange]);

  // Reset to flat
  const handleReset = useCallback(() => {
    // Filter valid presets and find Flat (with null safety)
    const validPresets = presets.filter(
      (p): p is { id: string; gains: number[] } =>
        p != null && typeof p.id === 'string' && Array.isArray(p.gains)
    );
    const flatPreset = validPresets.find(p => p.id === 'Flat') || { id: 'Flat', gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] };
    applyPreset(flatPreset);
  }, [presets, applyPreset]);

  // Check if current settings are "flat"
  const isFlat = useMemo(() => {
    return gains.every(g => Math.abs(g) < 0.1);
  }, [gains]);

  return (
    <div data-testid="graphic-eq-editor" className="space-y-4">
      {/* Header: Preset selector and Reset button */}
      <div className="flex items-center justify-between gap-3">
        {/* Preset Dropdown */}
        <div className="relative flex-1" ref={dropdownRef}>
          <button
            data-testid="graphic-eq-preset-select"
            onClick={() => setShowPresetDropdown(!showPresetDropdown)}
            className={`
              w-full flex items-center justify-between gap-2 px-3 py-2
              border border-border rounded-lg bg-background
              hover:bg-muted/30 transition-colors text-sm
            `}
          >
            <span className="truncate">
              {t(`settings.audio.graphicEq.presets.${settings.preset.toLowerCase().replace(/\s+/g, '')}`, settings.preset)}
            </span>
            <ChevronDown className={`w-4 h-4 text-muted-foreground transition-transform ${showPresetDropdown ? 'rotate-180' : ''}`} />
          </button>

          {/* Dropdown menu */}
          {showPresetDropdown && (
            <div className="absolute z-50 w-full mt-1 py-1 bg-popover border border-border rounded-lg shadow-lg max-h-64 overflow-auto">
              {presets
                .filter((preset): preset is { id: string; gains: number[] } =>
                  preset != null && typeof preset.id === 'string' && Array.isArray(preset.gains)
                )
                .map((preset) => {
                  const isSelected = settings.preset === preset.id;
                  return (
                    <button
                      key={preset.id}
                      onClick={() => applyPreset(preset)}
                      className={`
                        w-full flex items-center justify-between px-3 py-2 text-sm text-left
                        transition-colors hover:bg-muted/50
                        ${isSelected ? 'bg-primary/10 text-primary' : ''}
                      `}
                    >
                      <span>
                        {t(`settings.audio.graphicEq.presets.${preset.id.toLowerCase().replace(/\s+/g, '')}`, preset.id)}
                      </span>
                      {isSelected && <Check className="w-4 h-4" />}
                    </button>
                  );
                })}
            </div>
          )}
        </div>

        {/* Reset Button */}
        <button
          data-testid="graphic-eq-reset-btn"
          onClick={handleReset}
          disabled={isFlat}
          className={`
            flex items-center gap-2 px-3 py-2 rounded-lg text-sm
            border border-border transition-colors
            ${isFlat
              ? 'opacity-50 cursor-not-allowed'
              : 'hover:bg-muted/50 hover:text-primary'
            }
          `}
          title={t('settings.audio.graphicEq.reset')}
        >
          <RotateCcw className="w-4 h-4" />
          <span className="hidden sm:inline">{t('settings.audio.graphicEq.reset')}</span>
        </button>
      </div>

      {/* EQ Sliders Container */}
      <div className="bg-muted/20 rounded-lg p-4 border border-border/50">
        {/* dB scale labels */}
        <div className="flex items-stretch gap-2 mb-2">
          <div className="w-8 flex flex-col justify-between text-xs text-muted-foreground text-right py-0">
            <span>+12</span>
            <span>+6</span>
            <span className="text-primary font-medium">0</span>
            <span>-6</span>
            <span>-12</span>
          </div>

          {/* Sliders */}
          <div className="flex-1 flex gap-1 justify-around">
            {ISO_FREQUENCIES.map((freq, index) => (
              <BandSlider
                key={freq}
                frequency={freq}
                gain={gains[index]}
                index={index}
                onGainChange={handleBandChange}
                isActive={activeBand === index}
                onActiveChange={setActiveBand}
                testId={`graphic-eq-band-${index}`}
              />
            ))}
          </div>

          {/* Right spacer for symmetry */}
          <div className="w-8" />
        </div>

        {/* Visual EQ curve preview */}
        <div className="mt-4 pt-3 border-t border-border/30">
          <EqCurvePreview gains={gains} />
        </div>
      </div>

      {/* Info text */}
      <p className="text-xs text-muted-foreground">
        {t('settings.audio.graphicEq.info')}
      </p>
    </div>
  );
}

// Visual representation of the EQ curve
function EqCurvePreview({ gains }: { gains: number[] }) {
  const points = useMemo(() => {
    // Create a smooth curve through the gain points
    const width = 100;
    const height = 40;
    const centerY = height / 2;
    const maxGain = 12;

    // Generate points with some interpolation for smoothness
    const pathPoints: string[] = [];

    for (let i = 0; i < gains.length; i++) {
      const x = (i / (gains.length - 1)) * width;
      const y = centerY - (gains[i] / maxGain) * (height / 2);
      pathPoints.push(`${x},${y}`);
    }

    // Create smooth bezier curve through points
    let d = `M ${pathPoints[0]}`;
    for (let i = 1; i < pathPoints.length; i++) {
      const [prevX, prevY] = pathPoints[i - 1].split(',').map(Number);
      const [currX, currY] = pathPoints[i].split(',').map(Number);
      const midX = (prevX + currX) / 2;
      d += ` Q ${prevX},${prevY} ${midX},${(prevY + currY) / 2}`;
    }
    // Final point
    const [lastX, lastY] = pathPoints[pathPoints.length - 1].split(',').map(Number);
    d += ` L ${lastX},${lastY}`;

    return d;
  }, [gains]);

  return (
    <svg
      viewBox="0 0 100 40"
      className="w-full h-10"
      preserveAspectRatio="none"
    >
      {/* Center line (0dB) */}
      <line
        x1="0"
        y1="20"
        x2="100"
        y2="20"
        stroke="currentColor"
        strokeOpacity="0.2"
        strokeWidth="0.5"
      />

      {/* Grid lines */}
      <line x1="0" y1="10" x2="100" y2="10" stroke="currentColor" strokeOpacity="0.1" strokeWidth="0.25" />
      <line x1="0" y1="30" x2="100" y2="30" stroke="currentColor" strokeOpacity="0.1" strokeWidth="0.25" />

      {/* Fill area */}
      <path
        d={`${points} L 100,20 L 0,20 Z`}
        fill="url(#eqGradient)"
        opacity="0.3"
      />

      {/* Curve line */}
      <path
        d={points}
        fill="none"
        stroke="hsl(var(--primary))"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />

      {/* Gradient definition */}
      <defs>
        <linearGradient id="eqGradient" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="hsl(var(--primary))" />
          <stop offset="50%" stopColor="hsl(var(--primary))" stopOpacity="0.2" />
          <stop offset="100%" stopColor="hsl(var(--primary))" />
        </linearGradient>
      </defs>
    </svg>
  );
}

export default GraphicEqEditor;
