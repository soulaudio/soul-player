// Headroom management settings component
// Prevents clipping by attenuating signal before DSP chain

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Info, AlertTriangle, Volume2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface HeadroomMode {
  mode: string; // 'auto' | 'manual' | 'disabled'
  manualDb: number | null;
}

interface HeadroomSettings {
  enabled: boolean;
  mode: HeadroomMode;
  totalGainDb: number;
  attenuationDb: number;
}

interface HeadroomSettingsProps {
  enabled?: boolean;
  mode?: string;
  manualDb?: number;
  onEnabledChange?: (enabled: boolean) => void;
  onModeChange?: (mode: string, manualDb?: number) => void;
}

const modes = [
  {
    value: 'auto',
    label: 'Auto',
    description: 'Automatically calculate headroom from ReplayGain + EQ boost',
  },
  {
    value: 'manual',
    label: 'Manual',
    description: 'Fixed headroom reserve (e.g., -6 dB)',
  },
  {
    value: 'disabled',
    label: 'Disabled',
    description: 'No headroom attenuation applied',
  },
] as const;

export function HeadroomSettings({
  enabled: propEnabled,
  mode: propMode,
  manualDb: propManualDb,
  onEnabledChange,
  onModeChange,
}: HeadroomSettingsProps) {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<HeadroomSettings | null>(null);
  const [localManualDb, setLocalManualDb] = useState(propManualDb ?? -6);
  const [isLoading, setIsLoading] = useState(false);

  // Load settings from backend
  const loadSettings = async () => {
    try {
      const result = await invoke<HeadroomSettings>('get_headroom_settings');
      setSettings(result);
      if (result.mode.manualDb !== null) {
        setLocalManualDb(result.mode.manualDb);
      }
    } catch (error) {
      console.error('Failed to load headroom settings:', error);
    }
  };

  useEffect(() => {
    loadSettings();
  }, []);

  // Sync local manual dB when prop changes
  useEffect(() => {
    if (propManualDb !== undefined) {
      setLocalManualDb(propManualDb);
    }
  }, [propManualDb]);

  const currentMode = propMode ?? settings?.mode.mode ?? 'auto';
  const currentEnabled = propEnabled ?? settings?.enabled ?? true;

  const handleModeChange = async (newMode: string) => {
    setIsLoading(true);
    try {
      const manualDb = newMode === 'manual' ? localManualDb : undefined;
      await invoke('set_headroom_mode', { mode: newMode, manualDb });

      if (onModeChange) {
        onModeChange(newMode, manualDb);
      }

      await loadSettings();
    } catch (error) {
      console.error('Failed to set headroom mode:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleManualDbChange = async (value: number) => {
    setLocalManualDb(value);

    if (currentMode === 'manual') {
      try {
        await invoke('set_headroom_mode', { mode: 'manual', manualDb: value });
        if (onModeChange) {
          onModeChange('manual', value);
        }
        await loadSettings();
      } catch (error) {
        console.error('Failed to set manual headroom:', error);
      }
    }
  };

  const handleEnabledChange = async (enabled: boolean) => {
    try {
      await invoke('set_headroom_enabled', { enabled });
      if (onEnabledChange) {
        onEnabledChange(enabled);
      }
      await loadSettings();
    } catch (error) {
      console.error('Failed to toggle headroom:', error);
    }
  };

  const totalGain = settings?.totalGainDb ?? 0;
  const attenuation = settings?.attenuationDb ?? 0;
  const isAttenuating = Math.abs(attenuation) > 0.1;

  return (
    <div className="space-y-6">
      {/* Enable/Disable Toggle */}
      <label className="flex items-start gap-3 cursor-pointer p-3 rounded-lg hover:bg-muted/30 transition-colors">
        <input
          type="checkbox"
          checked={currentEnabled}
          onChange={(e) => handleEnabledChange(e.target.checked)}
          className="w-4 h-4 mt-0.5"
          disabled={isLoading}
        />
        <div className="flex-1">
          <div className="text-sm font-medium">{t('settings.audio.headroom.enable', 'Enable Headroom Management')}</div>
          <p className="text-xs text-muted-foreground mt-1">
            {t('settings.audio.headroom.enableDescription', 'Automatically attenuate signal before DSP chain to prevent clipping')}
          </p>
        </div>
      </label>

      {currentEnabled && (
        <>
          {/* Mode Selection */}
          <div className="space-y-3">
            <label className="text-sm font-medium">{t('settings.audio.headroom.mode', 'Headroom Mode')}</label>

            <div className="space-y-2">
              {modes.map((option) => {
                const isSelected = option.value === currentMode;

                return (
                  <button
                    key={option.value}
                    onClick={() => handleModeChange(option.value)}
                    disabled={isLoading}
                    className={`
                      w-full text-left p-4 rounded-lg border-2 transition-all
                      ${
                        isSelected
                          ? 'border-primary bg-primary/5 shadow-sm'
                          : 'border-border hover:border-primary/50 hover:bg-muted/30'
                      }
                      disabled:opacity-50
                    `}
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1">
                          <span className="font-semibold">{t(`settings.audio.headroom.modes.${option.value}`, option.label)}</span>
                        </div>
                        <p className="text-sm text-muted-foreground">
                          {t(`settings.audio.headroom.modesDescription.${option.value}`, option.description)}
                        </p>
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

          {/* Manual dB slider (only when manual mode is selected) */}
          {currentMode === 'manual' && (
            <div className="space-y-3">
              <label className="text-sm font-medium flex items-center gap-2">
                {t('settings.audio.headroom.manualValue', 'Fixed Headroom Reserve')}
                <Info className="w-3 h-3 text-muted-foreground" title="Fixed attenuation applied to signal" />
              </label>

              <div className="space-y-2">
                <div className="flex items-center gap-3">
                  <input
                    type="range"
                    min="-24"
                    max="0"
                    step="0.5"
                    value={localManualDb}
                    onChange={(e) => handleManualDbChange(parseFloat(e.target.value))}
                    className="w-full"
                  />
                  <span className="text-sm font-mono w-16 text-right">
                    {localManualDb.toFixed(1)} dB
                  </span>
                </div>
                <div className="flex justify-between text-xs text-muted-foreground">
                  <span>-24 dB</span>
                  <span className="font-medium">-12 dB</span>
                  <span>0 dB</span>
                </div>
              </div>

              <p className="text-xs text-muted-foreground">
                {t('settings.audio.headroom.manualDescription', 'Lower values provide more headroom but reduce overall volume. -6 dB is a common choice.')}
              </p>
            </div>
          )}

          {/* Current Status */}
          {settings && currentMode !== 'disabled' && (
            <div className="space-y-3">
              <label className="text-sm font-medium">{t('settings.audio.headroom.currentStatus', 'Current Status')}</label>

              <div className="grid grid-cols-2 gap-3">
                <div className="bg-muted/30 rounded-lg p-3">
                  <div className="flex items-center gap-2 mb-1">
                    <Volume2 className="w-4 h-4 text-muted-foreground" />
                    <span className="text-xs text-muted-foreground">{t('settings.audio.headroom.totalGain', 'Total Potential Gain')}</span>
                  </div>
                  <div className={`text-lg font-mono ${totalGain > 0 ? 'text-amber-500' : 'text-green-500'}`}>
                    {totalGain >= 0 ? '+' : ''}{totalGain.toFixed(1)} dB
                  </div>
                </div>

                <div className="bg-muted/30 rounded-lg p-3">
                  <div className="flex items-center gap-2 mb-1">
                    <AlertTriangle className={`w-4 h-4 ${isAttenuating ? 'text-amber-500' : 'text-muted-foreground'}`} />
                    <span className="text-xs text-muted-foreground">{t('settings.audio.headroom.attenuation', 'Applied Attenuation')}</span>
                  </div>
                  <div className={`text-lg font-mono ${isAttenuating ? 'text-amber-500' : 'text-green-500'}`}>
                    {attenuation.toFixed(1)} dB
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Info Box */}
          <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 flex gap-3">
            <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
            <div className="text-sm">
              <p className="font-medium mb-1">{t('settings.audio.headroom.infoTitle', 'How Headroom Works')}</p>
              <p className="text-muted-foreground text-xs">
                {t('settings.audio.headroom.infoDescription',
                  'Headroom management prevents clipping by reducing the signal level before it enters the DSP chain. ' +
                  'In Auto mode, the attenuation is calculated from your ReplayGain, preamp, and EQ boost settings. ' +
                  'The output limiter still runs after the DSP chain as a safety net.'
                )}
              </p>
            </div>
          </div>

          {/* Signal Flow Diagram */}
          <div className="bg-muted/20 rounded-lg p-4">
            <p className="text-xs font-medium text-muted-foreground mb-3">{t('settings.audio.headroom.signalFlow', 'Signal Flow')}</p>
            <div className="flex items-center gap-2 text-xs overflow-x-auto pb-2">
              <span className="px-2 py-1 bg-muted rounded whitespace-nowrap">Source</span>
              <span className="text-muted-foreground">→</span>
              <span className="px-2 py-1 bg-muted rounded whitespace-nowrap">ReplayGain</span>
              <span className="text-muted-foreground">→</span>
              <span className={`px-2 py-1 rounded whitespace-nowrap ${isAttenuating ? 'bg-amber-500/20 text-amber-600' : 'bg-muted'}`}>
                Headroom {isAttenuating && `(${attenuation.toFixed(1)} dB)`}
              </span>
              <span className="text-muted-foreground">→</span>
              <span className="px-2 py-1 bg-muted rounded whitespace-nowrap">DSP Chain</span>
              <span className="text-muted-foreground">→</span>
              <span className="px-2 py-1 bg-muted rounded whitespace-nowrap">Volume</span>
              <span className="text-muted-foreground">→</span>
              <span className="px-2 py-1 bg-muted rounded whitespace-nowrap">Limiter</span>
              <span className="text-muted-foreground">→</span>
              <span className="px-2 py-1 bg-green-500/20 text-green-600 rounded whitespace-nowrap">Output</span>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
