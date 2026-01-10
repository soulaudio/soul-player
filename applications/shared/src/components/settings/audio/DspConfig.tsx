// Enhanced DSP effects chain configurator with backend integration

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Plus, X, Check, AlertCircle } from 'lucide-react';

// Types matching backend
export interface EffectSlot {
  index: number;
  effect: EffectType | null;
  enabled: boolean;
}

export type EffectType =
  | { type: 'eq'; bands: EqBand[] }
  | { type: 'compressor'; settings: CompressorSettings }
  | { type: 'limiter'; settings: LimiterSettings };

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

interface DspConfigProps {
  onChainChange?: () => void;
}

export function DspConfig({ onChainChange }: DspConfigProps) {
  const [chain, setChain] = useState<EffectSlot[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedSlot, setExpandedSlot] = useState<number | null>(null);
  const [notification, setNotification] = useState<{ type: 'success' | 'error'; message: string } | null>(null);

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
      showNotification('error', 'Failed to load DSP chain');
    } finally {
      setLoading(false);
    }
  };

  const showNotification = (type: 'success' | 'error', message: string) => {
    setNotification({ type, message });
  };

  const addEffect = async (slotIndex: number, effectType: 'eq' | 'compressor' | 'limiter') => {
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
      }

      await invoke('add_effect_to_chain', { slotIndex, effect });
      await loadChain();
      showNotification('success', `Added ${effectType} to slot ${slotIndex + 1}`);
      onChainChange?.();
      setExpandedSlot(null);
    } catch (error) {
      console.error('Failed to add effect:', error);
      showNotification('error', `Failed to add effect: ${error}`);
    }
  };

  const removeEffect = async (slotIndex: number) => {
    try {
      await invoke('remove_effect_from_chain', { slotIndex });
      await loadChain();
      showNotification('success', `Removed effect from slot ${slotIndex + 1}`);
      onChainChange?.();
      setExpandedSlot(null);
    } catch (error) {
      console.error('Failed to remove effect:', error);
      showNotification('error', `Failed to remove effect: ${error}`);
    }
  };

  const toggleEffect = async (slotIndex: number, enabled: boolean) => {
    try {
      await invoke('toggle_effect', { slotIndex, enabled });
      await loadChain();
      showNotification('success', `${enabled ? 'Enabled' : 'Disabled'} effect in slot ${slotIndex + 1}`);
      onChainChange?.();
    } catch (error) {
      console.error('Failed to toggle effect:', error);
      showNotification('error', `Failed to toggle effect: ${error}`);
    }
  };

  const clearChain = async () => {
    if (!confirm('Clear all effects from the DSP chain?')) return;

    try {
      await invoke('clear_dsp_chain');
      await loadChain();
      showNotification('success', 'Cleared DSP chain');
      onChainChange?.();
    } catch (error) {
      console.error('Failed to clear chain:', error);
      showNotification('error', `Failed to clear chain: ${error}`);
    }
  };

  if (loading) {
    return (
      <div className="bg-card border border-border rounded-lg p-6">
        <div className="text-center text-muted-foreground">Loading DSP chain...</div>
      </div>
    );
  }

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

      <div className="bg-card border border-border rounded-lg p-6 space-y-4">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-medium">DSP Effects Chain</h3>
            <p className="text-sm text-muted-foreground mt-0.5">
              Configure up to 4 effects processed in series
            </p>
          </div>
          {chain.some(slot => slot.effect !== null) && (
            <button
              onClick={clearChain}
              className="text-sm px-3 py-1.5 border border-border rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
            >
              Clear All
            </button>
          )}
        </div>

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
                  <div className="font-medium text-sm capitalize">
                    {slot.effect.type}
                    {!slot.enabled && <span className="text-muted-foreground ml-2">(Disabled)</span>}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {getEffectDescription(slot.effect)}
                  </div>
                </div>
              )}

              {/* Effect Picker Dropdown */}
              {expandedSlot === slot.index && (
                <div className="mt-3 border-t pt-3 space-y-2">
                  <div className="text-xs font-medium text-muted-foreground mb-2">
                    Select Effect:
                  </div>
                  <button
                    onClick={() => addEffect(slot.index, 'eq')}
                    className="w-full text-left p-3 rounded border hover:border-primary/50 hover:bg-primary/5 transition-colors"
                  >
                    <div className="text-sm font-medium">Parametric EQ</div>
                    <div className="text-xs text-muted-foreground">
                      3-5 band frequency equalizer
                    </div>
                  </button>
                  <button
                    onClick={() => addEffect(slot.index, 'compressor')}
                    className="w-full text-left p-3 rounded border hover:border-primary/50 hover:bg-primary/5 transition-colors"
                  >
                    <div className="text-sm font-medium">Compressor</div>
                    <div className="text-xs text-muted-foreground">
                      Dynamic range compression
                    </div>
                  </button>
                  <button
                    onClick={() => addEffect(slot.index, 'limiter')}
                    className="w-full text-left p-3 rounded border hover:border-primary/50 hover:bg-primary/5 transition-colors"
                  >
                    <div className="text-sm font-medium">Limiter</div>
                    <div className="text-xs text-muted-foreground">
                      Brick-wall peak limiting
                    </div>
                  </button>
                </div>
              )}

              {/* Effect Configuration UI - Hidden until parameter editing is implemented */}
            </div>
          ))}
        </div>

        {/* Info note */}
        <div className="text-xs text-muted-foreground bg-muted/30 p-3 rounded">
          <strong>Note:</strong> Effects are processed in order (Slot 1 → 2 → 3 → 4) before
          upsampling. Use the enable/disable checkbox to bypass effects without removing them.
        </div>
      </div>
    </div>
  );
}

function getEffectDescription(effect: EffectType): string {
  switch (effect.type) {
    case 'eq':
      return `${effect.bands.length} bands`;
    case 'compressor':
      return `${effect.settings.ratio}:1 ratio, ${effect.settings.thresholdDb}dB threshold`;
    case 'limiter':
      return `${effect.settings.thresholdDb}dB threshold, ${effect.settings.releaseMs}ms release`;
  }
}
