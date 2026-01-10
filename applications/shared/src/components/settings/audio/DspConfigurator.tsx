// DSP effects chain configurator

import { useState } from 'react';
import { Settings, Plus } from 'lucide-react';

interface DspConfiguratorProps {
  enabled: boolean;
  slots: (string | null)[];
  onEnabledChange: (enabled: boolean) => void;
  onSlotsChange: (slots: (string | null)[]) => void;
}

const availableEffects = [
  { id: 'eq', name: 'Parametric EQ', description: '5-band equalizer' },
  { id: 'compressor', name: 'Compressor', description: 'Dynamic range control' },
  { id: 'crossfeed', name: 'Crossfeed', description: 'Headphone spatialization' },
  { id: 'convolution', name: 'Convolution', description: 'Room correction / reverb' },
];

export function DspConfigurator({
  enabled,
  slots,
  onEnabledChange,
  onSlotsChange,
}: DspConfiguratorProps) {
  const [expandedSlot, setExpandedSlot] = useState<number | null>(null);

  const handleSlotChange = (slotIndex: number, effectId: string | null) => {
    const newSlots = [...slots];
    newSlots[slotIndex] = effectId;
    onSlotsChange(newSlots);
    setExpandedSlot(null);
  };

  return (
    <div className="bg-card border border-border rounded-lg p-6 space-y-4">
      {/* Enable/Disable Toggle */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="font-medium">Enable DSP Effects</h3>
          <p className="text-sm text-muted-foreground mt-0.5">
            Effects are processed before upsampling
          </p>
        </div>
        <label className="relative inline-flex items-center cursor-pointer">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(e) => onEnabledChange(e.target.checked)}
            className="sr-only peer"
          />
          <div className="w-11 h-6 bg-muted rounded-full peer peer-checked:bg-primary transition-colors after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-transform peer-checked:after:translate-x-5"></div>
        </label>
      </div>

      {/* Warning when disabled */}
      {!enabled && (
        <div className="p-3 bg-muted/50 rounded-lg text-sm text-muted-foreground">
          DSP effects are currently disabled. Enable to configure effect slots.
        </div>
      )}

      {/* Effect Slots */}
      {enabled && (
        <div className="space-y-3">
          {slots.map((effect, index) => (
            <div key={index} className="border border-border rounded-lg p-4">
              <div className="flex items-center justify-between mb-2">
                <div className="text-sm font-medium">Slot {index + 1}</div>
                {effect && (
                  <button
                    onClick={() => setExpandedSlot(expandedSlot === index ? null : index)}
                    className="text-xs px-2 py-1 rounded hover:bg-muted transition-colors"
                  >
                    <Settings className="w-3 h-3" />
                  </button>
                )}
              </div>

              {effect === null ? (
                <button
                  onClick={() => setExpandedSlot(expandedSlot === index ? null : index)}
                  className="w-full p-3 border border-dashed rounded-lg text-sm text-muted-foreground hover:bg-muted/30 hover:border-primary/50 transition-all flex items-center justify-center gap-2"
                >
                  <Plus className="w-4 h-4" />
                  Add Effect
                </button>
              ) : (
                <div className="p-3 bg-muted/30 rounded">
                  <div className="font-medium text-sm">
                    {availableEffects.find(e => e.id === effect)?.name}
                  </div>
                  <div className="text-xs text-muted-foreground mt-1">
                    {availableEffects.find(e => e.id === effect)?.description}
                  </div>
                </div>
              )}

              {/* Effect Picker Dropdown */}
              {expandedSlot === index && (
                <div className="mt-3 border-t pt-3 space-y-2">
                  <div className="text-xs font-medium text-muted-foreground mb-2">
                    Select Effect:
                  </div>
                  {availableEffects.map((availEffect) => (
                    <button
                      key={availEffect.id}
                      onClick={() => handleSlotChange(index, availEffect.id)}
                      className="w-full text-left p-2 rounded hover:bg-muted/50 transition-colors"
                    >
                      <div className="text-sm font-medium">{availEffect.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {availEffect.description}
                      </div>
                    </button>
                  ))}
                  {effect !== null && (
                    <button
                      onClick={() => handleSlotChange(index, null)}
                      className="w-full text-left p-2 rounded hover:bg-destructive/10 text-destructive text-sm transition-colors"
                    >
                      Remove Effect
                    </button>
                  )}
                </div>
              )}
            </div>
          ))}

          {/* Info note */}
          <div className="text-xs text-muted-foreground bg-muted/30 p-3 rounded">
            <strong>Note:</strong> DSP is automatically bypassed when upsampling to DSD format.
            Effect parameters can be configured by clicking the settings icon.
          </div>
        </div>
      )}
    </div>
  );
}
