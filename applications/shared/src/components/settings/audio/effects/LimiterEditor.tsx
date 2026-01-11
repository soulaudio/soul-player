// Industry-standard Limiter Editor component
// Inspired by FabFilter Pro-L and Waves L2 limiters

import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Info, AlertTriangle, ChevronDown } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface LimiterSettings {
  thresholdDb: number;
  releaseMs: number;
}

export interface LimiterEditorProps {
  settings: LimiterSettings;
  onSettingsChange: (settings: LimiterSettings) => void;
  slotIndex: number;
}

interface LimiterPreset {
  name: string;
  settings: LimiterSettings;
}

// Presets defined locally for UI display (actual backend presets loaded via API)
const UI_PRESETS: { key: string; releaseMs: number; description: string }[] = [
  { key: 'transparent', releaseMs: 100, description: 'limiter.preset.transparentDesc' },
  { key: 'punchy', releaseMs: 50, description: 'limiter.preset.punchyDesc' },
  { key: 'loud', releaseMs: 30, description: 'limiter.preset.loudDesc' },
  { key: 'mastering', releaseMs: 150, description: 'limiter.preset.masteringDesc' },
];

export function LimiterEditor({ settings, onSettingsChange, slotIndex: _slotIndex }: LimiterEditorProps) {
  const { t } = useTranslation();
  const [presets, setPresets] = useState<LimiterPreset[]>([]);
  const [selectedPreset, setSelectedPreset] = useState<string | null>(null);
  const [isPresetOpen, setIsPresetOpen] = useState(false);

  // Simulated gain reduction for visual feedback (in a real implementation,
  // this would come from the audio engine via events)
  const [gainReduction, setGainReduction] = useState(0);

  // Load presets from backend
  useEffect(() => {
    const loadPresets = async () => {
      try {
        const backendPresets = await invoke<[string, LimiterSettings][]>('get_limiter_presets');
        setPresets(backendPresets.map(([name, settings]) => ({ name, settings })));
      } catch (error) {
        console.error('Failed to load limiter presets:', error);
      }
    };
    loadPresets();
  }, []);

  // Simulate gain reduction animation based on threshold
  // In production, this would be driven by actual audio analysis
  useEffect(() => {
    const interval = setInterval(() => {
      // Simulate gain reduction: more reduction with lower thresholds
      const baseReduction = Math.max(0, -settings.thresholdDb * 0.8);
      const variance = Math.random() * 2 - 1;
      setGainReduction(Math.max(0, Math.min(12, baseReduction + variance)));
    }, 100);
    return () => clearInterval(interval);
  }, [settings.thresholdDb]);

  // Handle threshold change - parent handles backend via onSettingsChange
  const handleThresholdChange = useCallback((value: number) => {
    const newSettings = { ...settings, thresholdDb: value };
    onSettingsChange(newSettings);
    setSelectedPreset(null);
  }, [settings, onSettingsChange]);

  // Handle release change
  const handleReleaseChange = useCallback((value: number) => {
    const newSettings = { ...settings, releaseMs: value };
    onSettingsChange(newSettings);
    setSelectedPreset(null);
  }, [settings, onSettingsChange]);

  // Apply preset
  const applyPreset = useCallback((preset: LimiterPreset) => {
    onSettingsChange(preset.settings);
    setSelectedPreset(preset.name);
    setIsPresetOpen(false);
  }, [onSettingsChange]);

  // Apply UI preset (with default ceiling of -0.3dB)
  const applyUiPreset = useCallback((key: string, releaseMs: number) => {
    const newSettings = { thresholdDb: -0.3, releaseMs };
    onSettingsChange(newSettings);
    setSelectedPreset(key);
    setIsPresetOpen(false);
  }, [onSettingsChange]);

  // Get gain reduction color
  const getGainReductionColor = (db: number): string => {
    if (db < 3) return 'bg-green-500';
    if (db < 6) return 'bg-yellow-500';
    if (db < 9) return 'bg-orange-500';
    return 'bg-red-500';
  };

  // Format release time for display
  const formatRelease = (ms: number): string => {
    if (ms < 100) return `${ms.toFixed(0)} ms`;
    return `${(ms / 1000).toFixed(2)} s`;
  };

  return (
    <div data-testid="limiter-editor" className="space-y-6">
      {/* Header with Preset Dropdown */}
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold">{t('limiter.title')}</h3>

        {/* Preset Dropdown */}
        <div className="relative">
          <button
            data-testid="limiter-preset-select"
            onClick={() => setIsPresetOpen(!isPresetOpen)}
            className="flex items-center gap-2 px-3 py-1.5 text-sm border border-border rounded-lg hover:bg-muted/50 transition-colors"
          >
            <span>{selectedPreset ? t(`limiter.preset.${selectedPreset}`) : t('limiter.selectPreset')}</span>
            <ChevronDown className={`w-4 h-4 transition-transform ${isPresetOpen ? 'rotate-180' : ''}`} />
          </button>

          {isPresetOpen && (
            <div className="absolute right-0 mt-1 w-56 bg-popover border border-border rounded-lg shadow-lg z-10 py-1">
              {/* UI Presets */}
              <div className="px-2 py-1 text-xs font-medium text-muted-foreground">{t('limiter.presets')}</div>
              {UI_PRESETS.map((preset) => (
                <button
                  key={preset.key}
                  onClick={() => applyUiPreset(preset.key, preset.releaseMs)}
                  className={`w-full text-left px-3 py-2 text-sm hover:bg-muted/50 transition-colors ${
                    selectedPreset === preset.key ? 'bg-primary/10 text-primary' : ''
                  }`}
                >
                  <div className="font-medium">{t(`limiter.preset.${preset.key}`)}</div>
                  <div className="text-xs text-muted-foreground">{t(preset.description)}</div>
                </button>
              ))}

              {/* Backend Presets */}
              {presets.length > 0 && (
                <>
                  <div className="border-t border-border my-1" />
                  <div className="px-2 py-1 text-xs font-medium text-muted-foreground">{t('limiter.backendPresets')}</div>
                  {presets.map((preset) => (
                    <button
                      key={preset.name}
                      onClick={() => applyPreset(preset)}
                      className={`w-full text-left px-3 py-2 text-sm hover:bg-muted/50 transition-colors ${
                        selectedPreset === preset.name ? 'bg-primary/10 text-primary' : ''
                      }`}
                    >
                      <div className="font-medium">{preset.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {preset.settings.thresholdDb} dB, {formatRelease(preset.settings.releaseMs)}
                      </div>
                    </button>
                  ))}
                </>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Main Controls Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Ceiling/Threshold Control - Main focus */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium">{t('limiter.ceiling')}</label>
            <span className="text-lg font-mono font-bold tabular-nums">
              {settings.thresholdDb.toFixed(1)} dB
            </span>
          </div>

          {/* Large vertical slider representation */}
          <div className="relative h-48 bg-muted/30 rounded-lg p-4 flex flex-col justify-between">
            {/* dB scale markers */}
            <div className="absolute left-2 top-4 bottom-4 flex flex-col justify-between text-xs text-muted-foreground">
              <span>0</span>
              <span>-3</span>
              <span>-6</span>
              <span>-9</span>
              <span>-12</span>
            </div>

            {/* Threshold indicator bar */}
            <div className="ml-8 flex-1 relative">
              <div className="absolute inset-0 bg-gradient-to-t from-red-500/20 via-yellow-500/20 to-green-500/20 rounded" />
              <div
                className="absolute left-0 right-0 h-1 bg-primary rounded shadow-lg shadow-primary/50 transition-all"
                style={{
                  bottom: `${((settings.thresholdDb + 12) / 12) * 100}%`,
                  transform: 'translateY(50%)'
                }}
              />
              {/* Safe zone indicator at -0.3dB */}
              <div
                className="absolute left-0 right-0 h-px bg-green-500/50"
                style={{ bottom: `${((-0.3 + 12) / 12) * 100}%` }}
              >
                <span className="absolute -right-1 -top-3 text-[10px] text-green-500">{t('limiter.safe')}</span>
              </div>
            </div>

          </div>

          {/* Horizontal slider for ceiling control */}
          <div className="space-y-1">
            <input
              data-testid="limiter-ceiling"
              type="range"
              min="-12"
              max="0"
              step="0.1"
              value={settings.thresholdDb}
              onChange={(e) => handleThresholdChange(parseFloat(e.target.value))}
              className="w-full"
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>-12 dB</span>
              <span className="text-green-500 font-medium">-0.3 dB ({t('limiter.recommended')})</span>
              <span>0 dB</span>
            </div>
          </div>
        </div>

        {/* Right column: Release + Gain Reduction */}
        <div className="space-y-6">
          {/* Gain Reduction Meter */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="text-sm font-medium">{t('limiter.gainReduction')}</label>
              <span className="text-sm font-mono tabular-nums">
                -{gainReduction.toFixed(1)} dB
              </span>
            </div>

            {/* Horizontal meter */}
            <div className="h-6 bg-muted/30 rounded-lg overflow-hidden relative">
              <div
                className={`h-full transition-all duration-75 ${getGainReductionColor(gainReduction)}`}
                style={{ width: `${Math.min(100, (gainReduction / 12) * 100)}%` }}
              />
              {/* Scale markers */}
              <div className="absolute inset-0 flex justify-between items-center px-1 pointer-events-none">
                {[0, 3, 6, 9, 12].map((val) => (
                  <div
                    key={val}
                    className="h-full border-l border-background/50"
                    style={{ marginLeft: val === 0 ? 0 : undefined }}
                  />
                ))}
              </div>
            </div>
            <div className="flex justify-between text-[10px] text-muted-foreground">
              <span>0 dB</span>
              <span>-3</span>
              <span>-6</span>
              <span>-9</span>
              <span>-12 dB</span>
            </div>
          </div>

          {/* Release Control */}
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-sm font-medium">{t('limiter.release')}</label>
              <span className="text-sm font-mono tabular-nums">
                {formatRelease(settings.releaseMs)}
              </span>
            </div>

            <input
              data-testid="limiter-release"
              type="range"
              min="10"
              max="1000"
              step="1"
              value={settings.releaseMs}
              onChange={(e) => handleReleaseChange(parseFloat(e.target.value))}
              className="w-full"
            />

            <div className="flex justify-between text-xs text-muted-foreground">
              <span>{t('limiter.fast')} (10ms)</span>
              <span>{t('limiter.slow')} (1000ms)</span>
            </div>

            {/* Quick release buttons */}
            <div className="flex gap-2">
              {[10, 30, 50, 100, 250, 500].map((ms) => (
                <button
                  key={ms}
                  onClick={() => handleReleaseChange(ms)}
                  className={`flex-1 px-2 py-1 text-xs rounded border transition-colors ${
                    Math.abs(settings.releaseMs - ms) < 5
                      ? 'bg-primary text-primary-foreground border-primary'
                      : 'border-border hover:bg-muted/50'
                  }`}
                >
                  {ms}
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Info Section */}
      <div className="space-y-3">
        {/* Warning for 0dB ceiling */}
        {settings.thresholdDb >= -0.1 && (
          <div className="bg-amber-500/10 border border-amber-500/20 rounded-lg p-3 flex gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-500 flex-shrink-0" />
            <div className="text-sm">
              <p className="font-medium text-amber-600 dark:text-amber-400">
                {t('limiter.warning.title')}
              </p>
              <p className="text-xs text-muted-foreground mt-1">
                {t('limiter.warning.description')}
              </p>
            </div>
          </div>
        )}

        {/* Info box */}
        <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-3 flex gap-3">
          <Info className="w-5 h-5 text-blue-500 flex-shrink-0" />
          <div className="text-sm">
            <p className="font-medium">{t('limiter.info.title')}</p>
            <p className="text-xs text-muted-foreground mt-1">
              {t('limiter.info.description')}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

export default LimiterEditor;
