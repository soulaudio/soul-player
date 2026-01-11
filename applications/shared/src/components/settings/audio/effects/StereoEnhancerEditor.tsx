// Industry-standard Stereo Enhancer UI component
// Inspired by iZotope Ozone Imager, Waves S1 Stereo Imager

import { useState, useEffect, useMemo, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import {
  ChevronDown,
  ChevronUp,
  AlertTriangle,
  Info,
  RotateCcw
} from 'lucide-react';

export interface StereoSettings {
  width: number;      // 0.0 (mono) to 2.0 (extra wide), 1.0 = normal
  midGainDb: number;  // -12 to +12 dB
  sideGainDb: number; // -12 to +12 dB
  balance: number;    // -1.0 (full left) to +1.0 (full right)
}

export interface StereoEnhancerEditorProps {
  settings: StereoSettings;
  onSettingsChange: (settings: StereoSettings) => void;
  slotIndex: number;
}

interface StereoPreset {
  name: string;
  settings: StereoSettings;
}

// Default presets with camelCase keys matching backend
const DEFAULT_PRESETS: StereoPreset[] = [
  { name: 'Normal', settings: { width: 1.0, midGainDb: 0, sideGainDb: 0, balance: 0 } },
  { name: 'Mono', settings: { width: 0.0, midGainDb: 0, sideGainDb: 0, balance: 0 } },
  { name: 'Narrow', settings: { width: 0.5, midGainDb: 0, sideGainDb: 0, balance: 0 } },
  { name: 'Wide', settings: { width: 1.5, midGainDb: 0, sideGainDb: 0, balance: 0 } },
  { name: 'Extra Wide', settings: { width: 2.0, midGainDb: 0, sideGainDb: 0, balance: 0 } },
];

export function StereoEnhancerEditor({
  settings,
  onSettingsChange,
  slotIndex,
}: StereoEnhancerEditorProps) {
  const { t } = useTranslation();

  // Local state for responsive UI updates
  const [localSettings, setLocalSettings] = useState<StereoSettings>(settings);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [presets, setPresets] = useState<StereoPreset[]>(DEFAULT_PRESETS);
  const [isUpdating, setIsUpdating] = useState(false);

  // Sync local state when props change
  useEffect(() => {
    setLocalSettings(settings);
  }, [settings]);

  // Load presets from backend
  useEffect(() => {
    loadPresets();
  }, []);

  const loadPresets = async () => {
    try {
      const backendPresets = await invoke<[string, { width: number; midGainDb: number; sideGainDb: number; balance: number }][]>('get_stereo_presets');
      if (backendPresets && backendPresets.length > 0) {
        const formattedPresets: StereoPreset[] = backendPresets.map(([name, preset]) => ({
          name,
          settings: {
            width: preset.width,
            midGainDb: preset.midGainDb,
            sideGainDb: preset.sideGainDb,
            balance: preset.balance,
          },
        }));
        // Add Narrow preset if not present (backend might not have it)
        if (!formattedPresets.find(p => p.name === 'Narrow')) {
          formattedPresets.splice(2, 0, {
            name: 'Narrow',
            settings: { width: 0.5, midGainDb: 0, sideGainDb: 0, balance: 0 }
          });
        }
        setPresets(formattedPresets);
      }
    } catch (error) {
      console.error('Failed to load stereo presets:', error);
    }
  };

  // Update backend with debounce
  const updateBackend = useCallback(async (newSettings: StereoSettings) => {
    if (isUpdating) return;

    setIsUpdating(true);
    try {
      await invoke('update_effect_parameters', {
        slotIndex,
        effect: {
          type: 'stereo',
          settings: {
            width: newSettings.width,
            midGainDb: newSettings.midGainDb,
            sideGainDb: newSettings.sideGainDb,
            balance: newSettings.balance,
          }
        },
      });
      onSettingsChange(newSettings);
    } catch (error) {
      console.error('Failed to update stereo settings:', error);
    } finally {
      setIsUpdating(false);
    }
  }, [slotIndex, onSettingsChange, isUpdating]);

  // Handlers for individual controls
  const handleWidthChange = (width: number) => {
    const newSettings = { ...localSettings, width };
    setLocalSettings(newSettings);
    updateBackend(newSettings);
  };

  const handleMidGainChange = (midGainDb: number) => {
    const newSettings = { ...localSettings, midGainDb };
    setLocalSettings(newSettings);
    updateBackend(newSettings);
  };

  const handleSideGainChange = (sideGainDb: number) => {
    const newSettings = { ...localSettings, sideGainDb };
    setLocalSettings(newSettings);
    updateBackend(newSettings);
  };

  const handleBalanceChange = (balance: number) => {
    const newSettings = { ...localSettings, balance };
    setLocalSettings(newSettings);
    updateBackend(newSettings);
  };

  const handlePresetChange = (presetName: string) => {
    const preset = presets.find(p => p.name === presetName);
    if (preset) {
      setLocalSettings(preset.settings);
      updateBackend(preset.settings);
    }
  };

  const handleReset = () => {
    const defaultSettings: StereoSettings = { width: 1.0, midGainDb: 0, sideGainDb: 0, balance: 0 };
    setLocalSettings(defaultSettings);
    updateBackend(defaultSettings);
  };

  // Calculate display values
  const widthPercent = Math.round(localSettings.width * 100);
  const hasMonoCompatibilityWarning = localSettings.width > 1.5;

  // Determine current preset (if any matches)
  const currentPreset = useMemo(() => {
    return presets.find(p =>
      Math.abs(p.settings.width - localSettings.width) < 0.01 &&
      Math.abs(p.settings.midGainDb - localSettings.midGainDb) < 0.1 &&
      Math.abs(p.settings.sideGainDb - localSettings.sideGainDb) < 0.1 &&
      Math.abs(p.settings.balance - localSettings.balance) < 0.01
    )?.name || '';
  }, [localSettings, presets]);

  // Calculate balance display
  const balanceDisplay = useMemo(() => {
    const val = localSettings.balance;
    if (Math.abs(val) < 0.01) return t('dsp.stereo.center');
    const percent = Math.abs(Math.round(val * 100));
    return val < 0
      ? `${t('dsp.stereo.left')} ${percent}%`
      : `${t('dsp.stereo.right')} ${percent}%`;
  }, [localSettings.balance, t]);

  return (
    <div data-testid="stereo-editor" className="space-y-6">
      {/* Header with Preset and Reset */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex-1">
          <label className="text-sm font-medium mb-1 block">
            {t('dsp.stereo.preset')}
          </label>
          <select
            data-testid="stereo-preset-select"
            value={currentPreset}
            onChange={(e) => handlePresetChange(e.target.value)}
            className="w-full px-3 py-2 rounded-lg border border-border bg-background text-sm focus:outline-none focus:ring-2 focus:ring-primary/50"
          >
            <option value="">{t('dsp.stereo.customPreset')}</option>
            {presets.map((preset) => (
              <option key={preset.name} value={preset.name}>
                {t(`dsp.stereo.presets.${preset.name.toLowerCase().replace(' ', '')}`, preset.name)}
              </option>
            ))}
          </select>
        </div>
        <button
          onClick={handleReset}
          className="flex items-center gap-1.5 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-muted rounded-lg transition-colors"
          title={t('dsp.stereo.reset')}
        >
          <RotateCcw className="w-4 h-4" />
          {t('common.reset', 'Reset')}
        </button>
      </div>

      {/* Width Control (Main Feature) */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">
            {t('dsp.stereo.width')}
          </label>
          <span className="text-sm font-mono tabular-nums">
            {widthPercent}%
          </span>
        </div>

        {/* Stereo Field Visualization */}
        <div className="relative h-24 bg-muted/30 rounded-lg overflow-hidden border border-border">
          {/* Background grid lines */}
          <div className="absolute inset-0 flex justify-between px-4">
            <div className="w-px h-full bg-border/50" />
            <div className="w-px h-full bg-border/50" />
            <div className="w-px h-full bg-primary/30" /> {/* Center */}
            <div className="w-px h-full bg-border/50" />
            <div className="w-px h-full bg-border/50" />
          </div>

          {/* L and R labels */}
          <div className="absolute top-2 left-3 text-xs font-medium text-muted-foreground">L</div>
          <div className="absolute top-2 right-3 text-xs font-medium text-muted-foreground">R</div>

          {/* Stereo field indicator */}
          <div
            className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 h-12 bg-primary/20 border-2 border-primary rounded-lg transition-all duration-150"
            style={{
              width: `${Math.max(4, widthPercent * 0.4)}%`,
              marginLeft: `${localSettings.balance * 20}%`,
            }}
          />

          {/* Center dot */}
          <div
            className="absolute top-1/2 left-1/2 -translate-y-1/2 w-3 h-3 bg-primary rounded-full transition-all duration-150"
            style={{
              marginLeft: `${localSettings.balance * 20}%`,
            }}
          />

          {/* Width labels at bottom */}
          <div className="absolute bottom-2 left-0 right-0 flex justify-between px-3 text-[10px] text-muted-foreground">
            <span>{t('dsp.stereo.monoLabel')}</span>
            <span>{t('dsp.stereo.normalLabel')}</span>
            <span>{t('dsp.stereo.wideLabel')}</span>
          </div>
        </div>

        {/* Width Slider */}
        <div className="space-y-1">
          <input
            data-testid="stereo-width"
            type="range"
            min="0"
            max="200"
            step="1"
            value={widthPercent}
            onChange={(e) => handleWidthChange(parseFloat(e.target.value) / 100)}
            className="w-full accent-primary"
          />
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>0%</span>
            <span>100%</span>
            <span>200%</span>
          </div>
        </div>
      </div>

      {/* Balance Control */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium">
            {t('dsp.stereo.balance')}
          </label>
          <span className="text-sm font-mono tabular-nums">
            {balanceDisplay}
          </span>
        </div>
        <div className="relative">
          <input
            data-testid="stereo-balance"
            type="range"
            min="-100"
            max="100"
            step="1"
            value={Math.round(localSettings.balance * 100)}
            onChange={(e) => handleBalanceChange(parseFloat(e.target.value) / 100)}
            className="w-full accent-primary"
          />
          {/* Center detent indicator */}
          <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-0.5 h-3 bg-primary/50 pointer-events-none" />
        </div>
        <div className="flex justify-between text-xs text-muted-foreground">
          <span>L</span>
          <span>{t('dsp.stereo.center')}</span>
          <span>R</span>
        </div>
      </div>

      {/* Mono Compatibility Warning */}
      {hasMonoCompatibilityWarning && (
        <div className="flex items-start gap-3 p-3 rounded-lg bg-amber-500/10 border border-amber-500/20">
          <AlertTriangle className="w-5 h-5 text-amber-500 flex-shrink-0 mt-0.5" />
          <div className="text-sm">
            <p className="font-medium text-amber-700 dark:text-amber-400">
              {t('dsp.stereo.monoWarningTitle')}
            </p>
            <p className="text-muted-foreground text-xs mt-1">
              {t('dsp.stereo.monoWarningDescription')}
            </p>
          </div>
        </div>
      )}

      {/* Advanced Section (Collapsible) */}
      <div className="border-t pt-4">
        <button
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="flex items-center justify-between w-full text-sm font-medium hover:text-primary transition-colors"
        >
          <span className="flex items-center gap-2">
            {t('dsp.stereo.advanced')}
            <span className="text-xs text-muted-foreground font-normal">
              ({t('dsp.stereo.midSide')})
            </span>
          </span>
          {showAdvanced ? (
            <ChevronUp className="w-4 h-4" />
          ) : (
            <ChevronDown className="w-4 h-4" />
          )}
        </button>

        {showAdvanced && (
          <div className="mt-4 space-y-5">
            {/* Mid/Side Explanation */}
            <div className="flex items-start gap-3 p-3 rounded-lg bg-blue-500/10 border border-blue-500/20">
              <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
              <div className="text-xs text-muted-foreground">
                <p className="font-medium text-foreground mb-1">
                  {t('dsp.stereo.midSideExplanationTitle')}
                </p>
                <p>
                  {t('dsp.stereo.midSideExplanation')}
                </p>
              </div>
            </div>

            {/* Mid Gain */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-sm">
                  {t('dsp.stereo.midGain')}
                  <span className="text-xs text-muted-foreground ml-1">
                    ({t('dsp.stereo.centerContent')})
                  </span>
                </label>
                <span className="text-sm font-mono tabular-nums">
                  {localSettings.midGainDb >= 0 ? '+' : ''}{localSettings.midGainDb.toFixed(1)} dB
                </span>
              </div>
              <input
                data-testid="stereo-mid-gain"
                type="range"
                min="-12"
                max="12"
                step="0.5"
                value={localSettings.midGainDb}
                onChange={(e) => handleMidGainChange(parseFloat(e.target.value))}
                className="w-full accent-primary"
              />
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>-12 dB</span>
                <span>0 dB</span>
                <span>+12 dB</span>
              </div>
            </div>

            {/* Side Gain */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-sm">
                  {t('dsp.stereo.sideGain')}
                  <span className="text-xs text-muted-foreground ml-1">
                    ({t('dsp.stereo.stereoContent')})
                  </span>
                </label>
                <span className="text-sm font-mono tabular-nums">
                  {localSettings.sideGainDb >= 0 ? '+' : ''}{localSettings.sideGainDb.toFixed(1)} dB
                </span>
              </div>
              <input
                data-testid="stereo-side-gain"
                type="range"
                min="-12"
                max="12"
                step="0.5"
                value={localSettings.sideGainDb}
                onChange={(e) => handleSideGainChange(parseFloat(e.target.value))}
                className="w-full accent-primary"
              />
              <div className="flex justify-between text-xs text-muted-foreground">
                <span>-12 dB</span>
                <span>0 dB</span>
                <span>+12 dB</span>
              </div>
            </div>

            {/* Usage Tips */}
            <div className="text-xs text-muted-foreground space-y-1 pt-2 border-t">
              <p className="font-medium text-foreground">
                {t('dsp.stereo.tips')}
              </p>
              <ul className="list-disc list-inside space-y-0.5 ml-1">
                <li>{t('dsp.stereo.tip1')}</li>
                <li>{t('dsp.stereo.tip2')}</li>
                <li>{t('dsp.stereo.tip3')}</li>
              </ul>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default StereoEnhancerEditor;
