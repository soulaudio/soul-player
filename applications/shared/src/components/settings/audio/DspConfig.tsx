// Enhanced DSP effects chain configurator with backend integration

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Plus, X, Check, AlertCircle, Headphones, SlidersHorizontal, Volume2, Activity, Gauge, Waves } from 'lucide-react';
import { ConfirmDialog } from '../../ui/Dialog';

// Types matching backend
export interface EffectSlot {
  index: number;
  effect: EffectType | null;
  enabled: boolean;
}

export type EffectType =
  | { type: 'eq'; bands: EqBand[] }
  | { type: 'compressor'; settings: CompressorSettings }
  | { type: 'limiter'; settings: LimiterSettings }
  | { type: 'crossfeed'; settings: CrossfeedSettings }
  | { type: 'stereo'; settings: StereoSettings }
  | { type: 'graphic_eq'; settings: GraphicEqSettings };

export interface EqBand {
  frequency: number;
  gain: number;
  q: number;
}

export interface CompressorSettings {
  thresholdDb: number;
  ratio: number;
  attackMs: number;
  releaseMs: number;
  kneeDb: number;
  makeupGainDb: number;
}

export interface LimiterSettings {
  thresholdDb: number;
  releaseMs: number;
}

export interface CrossfeedSettings {
  preset: string;
  levelDb: number;
  cutoffHz: number;
}

export interface StereoSettings {
  width: number;
  midGainDb: number;
  sideGainDb: number;
  balance: number;
}

export interface GraphicEqSettings {
  preset: string;
  bandCount: number;
  gains: number[];
}

interface DspConfigProps {
  onChainChange?: () => void;
}

type EffectTypeKey = 'eq' | 'graphic_eq' | 'compressor' | 'limiter' | 'crossfeed' | 'stereo';

interface EffectInfo {
  key: EffectTypeKey;
  name: string;
  description: string;
  icon: React.ReactNode;
  category: 'eq' | 'dynamics' | 'spatial';
}

const EFFECT_INFO: EffectInfo[] = [
  {
    key: 'eq',
    name: 'Parametric EQ',
    description: '3-band frequency equalizer with adjustable Q',
    icon: <SlidersHorizontal className="w-5 h-5" />,
    category: 'eq',
  },
  {
    key: 'graphic_eq',
    name: 'Graphic EQ',
    description: '10-band graphic equalizer with presets',
    icon: <Activity className="w-5 h-5" />,
    category: 'eq',
  },
  {
    key: 'compressor',
    name: 'Compressor',
    description: 'Dynamic range compression',
    icon: <Gauge className="w-5 h-5" />,
    category: 'dynamics',
  },
  {
    key: 'limiter',
    name: 'Limiter',
    description: 'Brick-wall peak limiting',
    icon: <Volume2 className="w-5 h-5" />,
    category: 'dynamics',
  },
  {
    key: 'crossfeed',
    name: 'Crossfeed',
    description: 'Headphone crossfeed for natural soundstage',
    icon: <Headphones className="w-5 h-5" />,
    category: 'spatial',
  },
  {
    key: 'stereo',
    name: 'Stereo Enhancer',
    description: 'Width control and mid/side processing',
    icon: <Waves className="w-5 h-5" />,
    category: 'spatial',
  },
];

export function DspConfig({ onChainChange }: DspConfigProps) {
  const [chain, setChain] = useState<EffectSlot[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedSlot, setExpandedSlot] = useState<number | null>(null);
  const [notification, setNotification] = useState<{ type: 'success' | 'error'; message: string } | null>(null);
  const [showClearDialog, setShowClearDialog] = useState(false);

  useEffect(() => {
    loadChain();
  }, []);

  // Auto-hide notifications
  useEffect(() => {
    if (notification) {
      const timer = setTimeout(() => setNotification(null), 3000);
      return () => clearTimeout(timer);
    }
  }, [notification]);

  const loadChain = async () => {
    try {
      setLoading(true);
      const chainData = await invoke<EffectSlot[]>('get_dsp_chain');
      setChain(chainData);
    } catch (error) {
      console.error('Failed to load DSP chain:', error);
      showNotificationMsg('error', 'Failed to load DSP chain');
    } finally {
      setLoading(false);
    }
  };

  const showNotificationMsg = (type: 'success' | 'error', message: string) => {
    setNotification({ type, message });
  };

  const addEffect = async (slotIndex: number, effectType: EffectTypeKey) => {
    try {
      let effect: EffectType;

      // Create default effect based on type
      switch (effectType) {
        case 'eq':
          effect = {
            type: 'eq',
            bands: [
              { frequency: 100, gain: 0, q: 1.0 },
              { frequency: 1000, gain: 0, q: 1.0 },
              { frequency: 10000, gain: 0, q: 1.0 },
            ],
          };
          break;
        case 'graphic_eq':
          effect = {
            type: 'graphic_eq',
            settings: {
              preset: 'Flat',
              bandCount: 10,
              gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            },
          };
          break;
        case 'compressor':
          effect = {
            type: 'compressor',
            settings: {
              thresholdDb: -20,
              ratio: 4.0,
              attackMs: 10,
              releaseMs: 100,
              kneeDb: 2.0,
              makeupGainDb: 0,
            },
          };
          break;
        case 'limiter':
          effect = {
            type: 'limiter',
            settings: {
              thresholdDb: -0.3,
              releaseMs: 50,
            },
          };
          break;
        case 'crossfeed':
          effect = {
            type: 'crossfeed',
            settings: {
              preset: 'natural',
              levelDb: -4.5,
              cutoffHz: 700,
            },
          };
          break;
        case 'stereo':
          effect = {
            type: 'stereo',
            settings: {
              width: 1.0,
              midGainDb: 0,
              sideGainDb: 0,
              balance: 0,
            },
          };
          break;
      }

      await invoke('add_effect_to_chain', { slotIndex, effect });
      await loadChain();
      const effectInfo = EFFECT_INFO.find(e => e.key === effectType);
      showNotificationMsg('success', `Added ${effectInfo?.name || effectType} to slot ${slotIndex + 1}`);
      onChainChange?.();
      setExpandedSlot(null);
    } catch (error) {
      console.error('Failed to add effect:', error);
      showNotificationMsg('error', `Failed to add effect: ${error}`);
    }
  };

  const removeEffect = async (slotIndex: number) => {
    try {
      await invoke('remove_effect_from_chain', { slotIndex });
      await loadChain();
      showNotificationMsg('success', `Removed effect from slot ${slotIndex + 1}`);
      onChainChange?.();
      setExpandedSlot(null);
    } catch (error) {
      console.error('Failed to remove effect:', error);
      showNotificationMsg('error', `Failed to remove effect: ${error}`);
    }
  };

  const toggleEffect = async (slotIndex: number, enabled: boolean) => {
    try {
      await invoke('toggle_effect', { slotIndex, enabled });
      await loadChain();
      showNotificationMsg('success', `${enabled ? 'Enabled' : 'Disabled'} effect in slot ${slotIndex + 1}`);
      onChainChange?.();
    } catch (error) {
      console.error('Failed to toggle effect:', error);
      showNotificationMsg('error', `Failed to toggle effect: ${error}`);
    }
  };

  const clearChain = async () => {
    try {
      await invoke('clear_dsp_chain');
      await loadChain();
      showNotificationMsg('success', 'Cleared DSP chain');
      onChainChange?.();
    } catch (error) {
      console.error('Failed to clear chain:', error);
      showNotificationMsg('error', `Failed to clear chain: ${error}`);
    }
    setShowClearDialog(false);
  };

  if (loading) {
    return (
      <div className="text-center text-muted-foreground py-4">Loading DSP chain...</div>
    );
  }

  const groupedEffects = {
    eq: EFFECT_INFO.filter(e => e.category === 'eq'),
    dynamics: EFFECT_INFO.filter(e => e.category === 'dynamics'),
    spatial: EFFECT_INFO.filter(e => e.category === 'spatial'),
  };

  return (
    <div className="space-y-4">
      {/* Notification Toast */}
      {notification && (
        <div
          className={`
            fixed top-4 right-4 z-50 p-4 rounded-lg shadow-lg border flex items-center gap-3
            animate-in slide-in-from-top-2 duration-300
            ${notification.type === 'success'
              ? 'bg-green-50 border-green-200 text-green-900 dark:bg-green-950 dark:border-green-800 dark:text-green-100'
              : 'bg-red-50 border-red-200 text-red-900 dark:bg-red-950 dark:border-red-800 dark:text-red-100'
            }
          `}
        >
          {notification.type === 'success' ? (
            <Check className="w-5 h-5 flex-shrink-0" />
          ) : (
            <AlertCircle className="w-5 h-5 flex-shrink-0" />
          )}
          <span className="text-sm font-medium">{notification.message}</span>
        </div>
      )}

      {/* Header with Clear All button */}
      {chain.some(slot => slot.effect !== null) && (
        <div className="flex items-center justify-end">
          <button
            onClick={() => setShowClearDialog(true)}
            className="text-sm px-3 py-1.5 border border-border rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
          >
            Clear All
          </button>
        </div>
      )}

      {/* Clear Confirmation Dialog */}
      <ConfirmDialog
        open={showClearDialog}
        onClose={() => setShowClearDialog(false)}
        onConfirm={clearChain}
        title="Clear DSP Chain"
        message="Remove all effects from the DSP chain? This cannot be undone."
        confirmText="Clear All"
        variant="destructive"
      />

      {/* Effect Slots */}
      <div className="space-y-3">
        {chain.map((slot) => (
          <div key={slot.index} className="border border-border rounded-lg p-4">
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-3">
                <div className="text-sm font-medium">Slot {slot.index + 1}</div>
                {slot.effect && (
                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={slot.enabled}
                      onChange={(e) => toggleEffect(slot.index, e.target.checked)}
                      className="w-4 h-4"
                    />
                    <span className="text-xs text-muted-foreground">Enabled</span>
                  </label>
                )}
              </div>
              {slot.effect && (
                <button
                  onClick={() => removeEffect(slot.index)}
                  className="text-xs px-2 py-1 rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
                  title="Remove effect"
                >
                  <X className="w-4 h-4" />
                </button>
              )}
            </div>

            {slot.effect === null ? (
              <button
                onClick={() => setExpandedSlot(expandedSlot === slot.index ? null : slot.index)}
                className="w-full p-3 border border-dashed rounded-lg text-sm text-muted-foreground hover:bg-muted/30 hover:border-primary/50 transition-all flex items-center justify-center gap-2"
              >
                <Plus className="w-4 h-4" />
                Add Effect
              </button>
            ) : (
              <div className={`p-3 rounded ${slot.enabled ? 'bg-primary/10' : 'bg-muted/30'}`}>
                <div className="flex items-center gap-2">
                  {getEffectIcon(slot.effect.type)}
                  <div className="font-medium text-sm">
                    {getEffectName(slot.effect.type)}
                    {!slot.enabled && <span className="text-muted-foreground ml-2">(Disabled)</span>}
                  </div>
                </div>
                <div className="text-xs text-muted-foreground mt-1">
                  {getEffectDescription(slot.effect)}
                </div>
              </div>
            )}

            {/* Effect Picker Dropdown */}
            {expandedSlot === slot.index && (
              <div className="mt-3 border-t pt-3 space-y-4">
                <div className="text-xs font-medium text-muted-foreground">
                  Select Effect:
                </div>

                {/* EQ Effects */}
                <div>
                  <div className="text-xs font-semibold text-muted-foreground mb-2 flex items-center gap-1">
                    <SlidersHorizontal className="w-3 h-3" /> Equalization
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    {groupedEffects.eq.map((effect) => (
                      <button
                        key={effect.key}
                        onClick={() => addEffect(slot.index, effect.key)}
                        className="text-left p-3 rounded border hover:border-primary/50 hover:bg-primary/5 transition-colors"
                      >
                        <div className="flex items-center gap-2">
                          {effect.icon}
                          <div className="text-sm font-medium">{effect.name}</div>
                        </div>
                        <div className="text-xs text-muted-foreground mt-1">
                          {effect.description}
                        </div>
                      </button>
                    ))}
                  </div>
                </div>

                {/* Dynamics Effects */}
                <div>
                  <div className="text-xs font-semibold text-muted-foreground mb-2 flex items-center gap-1">
                    <Gauge className="w-3 h-3" /> Dynamics
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    {groupedEffects.dynamics.map((effect) => (
                      <button
                        key={effect.key}
                        onClick={() => addEffect(slot.index, effect.key)}
                        className="text-left p-3 rounded border hover:border-primary/50 hover:bg-primary/5 transition-colors"
                      >
                        <div className="flex items-center gap-2">
                          {effect.icon}
                          <div className="text-sm font-medium">{effect.name}</div>
                        </div>
                        <div className="text-xs text-muted-foreground mt-1">
                          {effect.description}
                        </div>
                      </button>
                    ))}
                  </div>
                </div>

                {/* Spatial Effects */}
                <div>
                  <div className="text-xs font-semibold text-muted-foreground mb-2 flex items-center gap-1">
                    <Headphones className="w-3 h-3" /> Spatial
                  </div>
                  <div className="grid grid-cols-2 gap-2">
                    {groupedEffects.spatial.map((effect) => (
                      <button
                        key={effect.key}
                        onClick={() => addEffect(slot.index, effect.key)}
                        className="text-left p-3 rounded border hover:border-primary/50 hover:bg-primary/5 transition-colors"
                      >
                        <div className="flex items-center gap-2">
                          {effect.icon}
                          <div className="text-sm font-medium">{effect.name}</div>
                        </div>
                        <div className="text-xs text-muted-foreground mt-1">
                          {effect.description}
                        </div>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Info note */}
      <div className="text-xs text-muted-foreground bg-muted/30 p-3 rounded">
        <strong>Note:</strong> Effects are processed in order (Slot 1 → 2 → 3 → 4) before
        upsampling. Use the enable/disable checkbox to bypass effects without removing them.
      </div>
    </div>
  );
}

function getEffectName(type: string): string {
  const info = EFFECT_INFO.find(e => e.key === type);
  return info?.name || type;
}

function getEffectIcon(type: string): React.ReactNode {
  const info = EFFECT_INFO.find(e => e.key === type);
  return info?.icon || null;
}

function getEffectDescription(effect: EffectType): string {
  switch (effect.type) {
    case 'eq':
      return `${effect.bands.length} bands`;
    case 'graphic_eq':
      return `${effect.settings.bandCount}-band, ${effect.settings.preset} preset`;
    case 'compressor':
      return `${effect.settings.ratio}:1 ratio, ${effect.settings.thresholdDb}dB threshold`;
    case 'limiter':
      return `${effect.settings.thresholdDb}dB threshold, ${effect.settings.releaseMs}ms release`;
    case 'crossfeed':
      return `${effect.settings.preset} preset, ${effect.settings.levelDb}dB level`;
    case 'stereo':
      return `${Math.round(effect.settings.width * 100)}% width, ${effect.settings.balance === 0 ? 'centered' : `${effect.settings.balance > 0 ? 'R' : 'L'} ${Math.abs(effect.settings.balance * 100).toFixed(0)}%`}`;
  }
}
