// Upsampling/Resampling settings component

import { Info, Check, Zap, Scale, Sparkles, Crown } from 'lucide-react';

export type ResamplingQuality = 'fast' | 'balanced' | 'high' | 'maximum';
export type ResamplingBackend = 'auto' | 'rubato' | 'r8brain';

interface UpsamplingSettingsProps {
  quality: ResamplingQuality;
  targetRate: 'auto' | number;
  backend?: ResamplingBackend;
  r8brainAvailable?: boolean;
  onQualityChange: (quality: ResamplingQuality) => void;
  onTargetRateChange: (rate: 'auto' | number) => void;
  onBackendChange?: (backend: ResamplingBackend) => void;
}

interface QualityPreset {
  id: ResamplingQuality;
  name: string;
  icon: React.ReactNode;
  description: string;
  specs: {
    filterLength: number;
    cutoff: string;
    stopband: string;
    cpuUsage: string;
  };
}

const qualityPresets: QualityPreset[] = [
  {
    id: 'fast',
    name: 'Fast',
    icon: <Zap className="w-4 h-4" />,
    description: 'Low CPU usage, good for older hardware or battery saving',
    specs: {
      filterLength: 64,
      cutoff: '0.90',
      stopband: '60 dB',
      cpuUsage: '~3%',
    },
  },
  {
    id: 'balanced',
    name: 'Balanced',
    icon: <Scale className="w-4 h-4" />,
    description: 'Good quality with moderate CPU usage',
    specs: {
      filterLength: 128,
      cutoff: '0.95',
      stopband: '100 dB',
      cpuUsage: '~6%',
    },
  },
  {
    id: 'high',
    name: 'High',
    icon: <Sparkles className="w-4 h-4" />,
    description: 'Excellent quality for critical listening',
    specs: {
      filterLength: 256,
      cutoff: '0.99',
      stopband: '140 dB',
      cpuUsage: '~10%',
    },
  },
  {
    id: 'maximum',
    name: 'Maximum',
    icon: <Crown className="w-4 h-4" />,
    description: 'Audiophile-grade, highest possible quality',
    specs: {
      filterLength: 512,
      cutoff: '0.995',
      stopband: '180 dB',
      cpuUsage: '~15%',
    },
  },
];

const targetRates = [
  { value: 'auto' as const, label: 'Auto (Match Device)' },
  { value: 44100, label: '44.1 kHz (CD Quality)' },
  { value: 48000, label: '48 kHz (DVD/DAT)' },
  { value: 88200, label: '88.2 kHz (2× CD)' },
  { value: 96000, label: '96 kHz (Hi-Res)' },
  { value: 176400, label: '176.4 kHz (4× CD)' },
  { value: 192000, label: '192 kHz (Studio)' },
];

export function UpsamplingSettings({
  quality,
  targetRate,
  backend = 'auto',
  r8brainAvailable = false,
  onQualityChange,
  onTargetRateChange,
  onBackendChange,
}: UpsamplingSettingsProps) {
  const selectedPreset = qualityPresets.find(p => p.id === quality) || qualityPresets[2];

  return (
    <div className="space-y-6">
      {/* Quality Presets */}
      <div>
        <label className="text-sm font-medium mb-3 block">Quality Preset</label>
        <div className="grid grid-cols-2 gap-3">
          {qualityPresets.map((preset) => {
            const isSelected = preset.id === quality;
            return (
              <button
                key={preset.id}
                onClick={() => onQualityChange(preset.id)}
                className={`
                  relative text-left p-4 rounded-lg border-2 transition-all
                  ${isSelected
                    ? 'border-primary bg-primary/5'
                    : 'border-border hover:border-primary/50 hover:bg-muted/30'
                  }
                `}
              >
                <div className="flex items-center gap-2 mb-1">
                  <span className={isSelected ? 'text-primary' : 'text-muted-foreground'}>
                    {preset.icon}
                  </span>
                  <span className="font-semibold">{preset.name}</span>
                  {isSelected && (
                    <Check className="w-4 h-4 text-primary ml-auto" />
                  )}
                </div>
                <p className="text-xs text-muted-foreground">{preset.description}</p>
              </button>
            );
          })}
        </div>
      </div>

      {/* Technical Specs for Selected Preset */}
      <div>
        <label className="text-sm font-medium mb-2 block">Technical Specifications</label>
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
            <span className="text-muted-foreground">Filter Length</span>
            <span className="font-medium">{selectedPreset.specs.filterLength} taps</span>
          </div>
          <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
            <span className="text-muted-foreground">Cutoff</span>
            <span className="font-medium">{selectedPreset.specs.cutoff} × Nyquist</span>
          </div>
          <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
            <span className="text-muted-foreground">Stopband</span>
            <span className="font-medium">{selectedPreset.specs.stopband}</span>
          </div>
          <div className="flex justify-between items-center p-3 bg-muted/30 rounded">
            <span className="text-muted-foreground">CPU Usage</span>
            <span className="font-medium">{selectedPreset.specs.cpuUsage}</span>
          </div>
        </div>
      </div>

      {/* Target Sample Rate */}
      <div>
        <label className="text-sm font-medium mb-2 block">Target Sample Rate</label>
        <select
          value={targetRate}
          onChange={(e) => {
            const val = e.target.value;
            onTargetRateChange(val === 'auto' ? 'auto' : parseInt(val, 10));
          }}
          className="w-full p-3 rounded-lg border border-border bg-background text-sm"
        >
          {targetRates.map((rate) => (
            <option key={rate.value} value={rate.value}>
              {rate.label}
            </option>
          ))}
        </select>
        <p className="text-xs text-muted-foreground mt-2">
          Auto mode matches your output device's native sample rate for optimal performance.
        </p>
      </div>

      {/* Backend Selection */}
      {onBackendChange && (
        <div>
          <label className="text-sm font-medium mb-2 block">Resampling Engine</label>
          <div className="space-y-2">
            <label className="flex items-center gap-3 p-3 rounded-lg border border-border hover:bg-muted/30 cursor-pointer">
              <input
                type="radio"
                name="backend"
                value="auto"
                checked={backend === 'auto'}
                onChange={() => onBackendChange('auto')}
                className="w-4 h-4"
              />
              <div className="flex-1">
                <div className="font-medium text-sm">Auto</div>
                <div className="text-xs text-muted-foreground">
                  Uses r8brain if available, otherwise Rubato
                </div>
              </div>
            </label>
            <label className="flex items-center gap-3 p-3 rounded-lg border border-border hover:bg-muted/30 cursor-pointer">
              <input
                type="radio"
                name="backend"
                value="rubato"
                checked={backend === 'rubato'}
                onChange={() => onBackendChange('rubato')}
                className="w-4 h-4"
              />
              <div className="flex-1">
                <div className="font-medium text-sm">Rubato</div>
                <div className="text-xs text-muted-foreground">
                  Fast, portable Sinc resampler (always available)
                </div>
              </div>
            </label>
            <label
              className={`flex items-center gap-3 p-3 rounded-lg border border-border cursor-pointer ${
                r8brainAvailable ? 'hover:bg-muted/30' : 'opacity-50 cursor-not-allowed'
              }`}
            >
              <input
                type="radio"
                name="backend"
                value="r8brain"
                checked={backend === 'r8brain'}
                onChange={() => r8brainAvailable && onBackendChange('r8brain')}
                disabled={!r8brainAvailable}
                className="w-4 h-4"
              />
              <div className="flex-1">
                <div className="font-medium text-sm flex items-center gap-2">
                  r8brain
                  {!r8brainAvailable && (
                    <span className="text-[10px] px-1.5 py-0.5 bg-muted rounded text-muted-foreground">
                      Not Available
                    </span>
                  )}
                </div>
                <div className="text-xs text-muted-foreground">
                  Audiophile-grade resampler by Aleksey Vaneev
                </div>
              </div>
            </label>
          </div>
        </div>
      )}

      {/* Info Box */}
      <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 flex gap-3">
        <Info className="w-5 h-5 text-blue-500 flex-shrink-0 mt-0.5" />
        <div className="text-sm text-foreground">
          <p className="font-medium mb-1">About Resampling</p>
          <p className="text-muted-foreground">
            Resampling converts audio between different sample rates. Higher quality settings preserve more
            high-frequency detail but use more CPU. For most listeners, <strong>High</strong> quality
            provides transparent results indistinguishable from the original.
          </p>
        </div>
      </div>
    </div>
  );
}
