// Crossfeed effect editor component
// Industry-standard UI inspired by Goodhertz CanOpener and 112dB Redline Monitor
//
// Crossfeed reduces the extreme stereo separation of headphones by adding
// subtle channel mixing, simulating how speakers sound in a room.

import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Headphones, ChevronDown, ChevronUp, Info, Check } from 'lucide-react';

export interface CrossfeedSettings {
  preset: string;
  levelDb: number;
  cutoffHz: number;
}

interface CrossfeedPreset {
  id: string;
  name: string;
  description: string;
  levelDb: number;
  cutoffHz: number;
  recommended?: boolean;
}

export interface CrossfeedEditorProps {
  settings: CrossfeedSettings;
  onSettingsChange: (settings: CrossfeedSettings) => void;
  slotIndex: number;
}

// Preset definitions matching backend CrossfeedPreset enum
const PRESETS: CrossfeedPreset[] = [
  {
    id: 'natural',
    name: 'crossfeed.presets.natural',
    description: 'crossfeed.presets.naturalDesc',
    levelDb: -4.5,
    cutoffHz: 700,
    recommended: true,
  },
  {
    id: 'relaxed',
    name: 'crossfeed.presets.relaxed',
    description: 'crossfeed.presets.relaxedDesc',
    levelDb: -6.0,
    cutoffHz: 650,
  },
  {
    id: 'meier',
    name: 'crossfeed.presets.meier',
    description: 'crossfeed.presets.meierDesc',
    levelDb: -9.0,
    cutoffHz: 550,
  },
  {
    id: 'custom',
    name: 'crossfeed.presets.custom',
    description: 'crossfeed.presets.customDesc',
    levelDb: -6.0,
    cutoffHz: 650,
  },
];

export function CrossfeedEditor({
  settings,
  onSettingsChange,
}: CrossfeedEditorProps) {
  const { t } = useTranslation();
  const [showAdvanced, setShowAdvanced] = useState(settings.preset === 'custom');
  const [localSettings, setLocalSettings] = useState<CrossfeedSettings>(settings);

  // Sync local state when props change
  useEffect(() => {
    setLocalSettings(settings);
    setShowAdvanced(settings.preset === 'custom');
  }, [settings]);

  // Handle preset selection - parent handles backend via onSettingsChange
  const handlePresetSelect = useCallback((presetId: string) => {
    const preset = PRESETS.find((p) => p.id === presetId);
    if (!preset) return;

    const newSettings: CrossfeedSettings = {
      preset: presetId,
      levelDb: preset.levelDb,
      cutoffHz: preset.cutoffHz,
    };

    setLocalSettings(newSettings);
    setShowAdvanced(presetId === 'custom');
    onSettingsChange(newSettings);
  }, [onSettingsChange]);

  // Handle level slider change - update local state for responsive UI
  const handleLevelChange = useCallback((levelDb: number) => {
    const newSettings: CrossfeedSettings = {
      ...localSettings,
      preset: 'custom',
      levelDb,
    };
    setLocalSettings(newSettings);
  }, [localSettings]);

  // Handle level slider release - commit to parent
  const handleLevelCommit = useCallback(() => {
    const newSettings = { ...localSettings, preset: 'custom' };
    onSettingsChange(newSettings);
  }, [localSettings, onSettingsChange]);

  // Handle cutoff slider change - update local state for responsive UI
  const handleCutoffChange = useCallback((cutoffHz: number) => {
    const newSettings: CrossfeedSettings = {
      ...localSettings,
      preset: 'custom',
      cutoffHz,
    };
    setLocalSettings(newSettings);
  }, [localSettings]);

  // Handle cutoff slider release - commit to parent
  const handleCutoffCommit = useCallback(() => {
    const newSettings = { ...localSettings, preset: 'custom' };
    onSettingsChange(newSettings);
  }, [localSettings, onSettingsChange]);

  // Convert level dB to percentage for display (0 dB = 100%, -10 dB = 0%)
  const levelToPercent = (db: number) => Math.round(((db + 10) / 10) * 100);

  return (
    <div data-testid="crossfeed-editor" className="space-y-5">
      {/* Header with headphone icon */}
      <div className="flex items-center gap-3">
        <div className="p-2 bg-primary/10 rounded-lg">
          <Headphones className="w-5 h-5 text-primary" />
        </div>
        <div>
          <h3 className="text-sm font-semibold">{t('crossfeed.title')}</h3>
          <p className="text-xs text-muted-foreground">{t('crossfeed.subtitle')}</p>
        </div>
      </div>

      {/* Info box explaining crossfeed */}
      <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-3 flex gap-3">
        <Info className="w-4 h-4 text-blue-500 flex-shrink-0 mt-0.5" />
        <p className="text-xs text-muted-foreground">{t('crossfeed.explanation')}</p>
      </div>

      {/* Preset Cards */}
      <div className="space-y-2">
        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
          {t('crossfeed.selectPreset')}
        </label>
        <div className="grid grid-cols-2 gap-2">
          {PRESETS.map((preset) => {
            const isSelected = localSettings.preset === preset.id;
            const isCustomSelected = localSettings.preset === 'custom' && preset.id === 'custom';

            return (
              <button
                key={preset.id}
                data-testid={`crossfeed-preset-${preset.id}`}
                onClick={() => handlePresetSelect(preset.id)}
                className={`
                  relative text-left p-3 rounded-lg border-2 transition-all cursor-pointer
                  ${
                    isSelected || isCustomSelected
                      ? 'border-primary bg-primary/5 shadow-sm'
                      : 'border-border hover:border-primary/50 hover:bg-muted/30'
                  }
                `}
              >
                {/* Selected indicator */}
                {(isSelected || isCustomSelected) && (
                  <div className="absolute top-2 right-2">
                    <Check className="w-4 h-4 text-primary" />
                  </div>
                )}

                {/* Recommended badge */}
                {preset.recommended && (
                  <span className="absolute -top-2 -right-2 px-1.5 py-0.5 text-[10px] font-medium bg-green-500 text-white rounded">
                    {t('crossfeed.recommended')}
                  </span>
                )}

                <div className="pr-5">
                  <div className="font-medium text-sm">{t(preset.name)}</div>
                  <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
                    {t(preset.description)}
                  </p>
                  {preset.id !== 'custom' && (
                    <div className="text-[10px] text-muted-foreground mt-1.5 font-mono">
                      {preset.levelDb} dB / {preset.cutoffHz} Hz
                    </div>
                  )}
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Crossfeed Level Slider */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
            {t('crossfeed.level')}
          </label>
          <span className="text-sm font-mono">
            {localSettings.levelDb.toFixed(1)} dB ({levelToPercent(localSettings.levelDb)}%)
          </span>
        </div>

        {/* Stereo field visualization */}
        <StereoFieldVisualization levelDb={localSettings.levelDb} />

        <div className="space-y-1">
          <input
            data-testid="crossfeed-level"
            type="range"
            min="-10"
            max="0"
            step="0.5"
            value={localSettings.levelDb}
            onChange={(e) => handleLevelChange(parseFloat(e.target.value))}
            onMouseUp={handleLevelCommit}
            onTouchEnd={handleLevelCommit}
            className="w-full accent-primary"
          />
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>{t('crossfeed.subtle')}</span>
            <span>{t('crossfeed.strong')}</span>
          </div>
        </div>
      </div>

      {/* Advanced Controls Toggle */}
      <button
        onClick={() => setShowAdvanced(!showAdvanced)}
        className="flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
      >
        {showAdvanced ? (
          <ChevronUp className="w-4 h-4" />
        ) : (
          <ChevronDown className="w-4 h-4" />
        )}
        {t('crossfeed.advancedControls')}
      </button>

      {/* Advanced Controls */}
      {showAdvanced && (
        <div className="space-y-4 p-4 bg-muted/30 rounded-lg border border-border">
          {/* Cutoff Frequency */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium">
                {t('crossfeed.cutoffFrequency')}
              </label>
              <span className="text-sm font-mono">{Math.round(localSettings.cutoffHz)} Hz</span>
            </div>

            <input
              data-testid="crossfeed-cutoff"
              type="range"
              min="300"
              max="2000"
              step="10"
              value={localSettings.cutoffHz}
              onChange={(e) => handleCutoffChange(parseFloat(e.target.value))}
              onMouseUp={handleCutoffCommit}
              onTouchEnd={handleCutoffCommit}
              className="w-full accent-primary"
            />
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>300 Hz</span>
              <span>2000 Hz</span>
            </div>

            <p className="text-xs text-muted-foreground mt-2">
              {t('crossfeed.cutoffExplanation')}
            </p>
          </div>

          {/* Current Settings Summary */}
          <div className="pt-3 border-t border-border">
            <div className="text-xs font-medium text-muted-foreground mb-2">
              {t('crossfeed.currentSettings')}
            </div>
            <div className="grid grid-cols-2 gap-2 text-xs">
              <div className="bg-background/50 rounded p-2">
                <span className="text-muted-foreground">{t('crossfeed.preset')}: </span>
                <span className="font-medium">
                  {t(PRESETS.find((p) => p.id === localSettings.preset)?.name || 'crossfeed.presets.custom')}
                </span>
              </div>
              <div className="bg-background/50 rounded p-2">
                <span className="text-muted-foreground">{t('crossfeed.level')}: </span>
                <span className="font-medium font-mono">{localSettings.levelDb.toFixed(1)} dB</span>
              </div>
              <div className="bg-background/50 rounded p-2 col-span-2">
                <span className="text-muted-foreground">{t('crossfeed.cutoff')}: </span>
                <span className="font-medium font-mono">{Math.round(localSettings.cutoffHz)} Hz</span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// Stereo field visualization component
function StereoFieldVisualization({ levelDb }: { levelDb: number }) {
  // Convert dB to visual width (0 dB = narrow, -10 dB = wide)
  // At 0 dB (max crossfeed), stereo image is more mono-like (narrower)
  // At -10 dB (min crossfeed), stereo image is wider
  const widthPercent = Math.round(30 + ((-levelDb) / 10) * 50); // 30% to 80%
  const centerOffset = (100 - widthPercent) / 2;

  return (
    <div className="relative h-16 bg-muted/30 rounded-lg overflow-hidden">
      {/* Grid lines */}
      <div className="absolute inset-0 flex justify-between px-4 opacity-30">
        <div className="w-px h-full bg-current" />
        <div className="w-px h-full bg-current" />
        <div className="w-px h-full bg-current" />
        <div className="w-px h-full bg-current" />
        <div className="w-px h-full bg-current" />
      </div>

      {/* Center line */}
      <div className="absolute left-1/2 top-0 bottom-0 w-px bg-primary/50 transform -translate-x-1/2" />

      {/* Stereo field indicator */}
      <div
        className="absolute top-1/2 h-8 bg-gradient-to-r from-primary/20 via-primary/60 to-primary/20 rounded-full transform -translate-y-1/2 transition-all duration-300"
        style={{
          left: `${centerOffset}%`,
          width: `${widthPercent}%`,
        }}
      />

      {/* Left/Right labels */}
      <div className="absolute bottom-1 left-2 text-[10px] text-muted-foreground font-medium">
        L
      </div>
      <div className="absolute bottom-1 right-2 text-[10px] text-muted-foreground font-medium">
        R
      </div>

      {/* Headphone icons */}
      <div className="absolute top-1 left-1/2 transform -translate-x-1/2">
        <Headphones className="w-4 h-4 text-muted-foreground/50" />
      </div>
    </div>
  );
}

export default CrossfeedEditor;
